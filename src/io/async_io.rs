// =============================================================================
// fqc-rust - Async I/O Module
// =============================================================================
// Provides asynchronous read-ahead and write-behind I/O for improved throughput.
//
// - AsyncReader: Background thread prefetches data into a buffer queue
// - AsyncWriter: Background thread writes data from a buffer queue
// - BufferPool: Thread-safe pool of reusable buffers
// - AsyncIOStats: I/O performance statistics
// =============================================================================

use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{bounded, Receiver, Sender};

// =============================================================================
// AsyncIOStats
// =============================================================================

/// I/O performance statistics
#[derive(Debug, Default)]
pub struct AsyncIOStats {
    pub bytes_transferred: AtomicU64,
    pub operations: AtomicU64,
}

impl AsyncIOStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_transfer(&self, bytes: u64) {
        self.bytes_transferred.fetch_add(bytes, Ordering::Relaxed);
        self.operations.fetch_add(1, Ordering::Relaxed);
    }
}

// =============================================================================
// AsyncReader
// =============================================================================

// =============================================================================
// AsyncWriter
// =============================================================================

/// Data buffer to be written
struct WriteBuffer {
    data: Vec<u8>,
}

enum WriteMessage {
    Data(WriteBuffer),
    Flush(Sender<io::Result<()>>),
}

/// Asynchronous writer with background write-behind thread.
/// Buffers data and writes it in a background thread.
pub struct AsyncWriter {
    sender: Option<Sender<WriteMessage>>,
    handle: Option<thread::JoinHandle<io::Result<()>>>,
    buffer: Vec<u8>,
    buffer_size: usize,
}

impl AsyncWriter {
    /// Create an AsyncWriter wrapping the given writer.
    /// - `queue_depth`: number of write buffers in flight
    /// - `buffer_size`: size threshold to trigger a flush to the background thread
    pub fn new<W: Write + Send + 'static>(mut writer: W, queue_depth: usize, buffer_size: usize) -> Self {
        let (tx, rx): (Sender<WriteMessage>, Receiver<WriteMessage>) = bounded(queue_depth);
        let stats = Arc::new(AsyncIOStats::new());
        let bg_stats = stats.clone();

        let handle = thread::spawn(move || -> io::Result<()> {
            for msg in rx.iter() {
                match msg {
                    WriteMessage::Data(wb) => {
                        writer.write_all(&wb.data)?;
                        bg_stats.add_transfer(wb.data.len() as u64);
                    }
                    WriteMessage::Flush(ack_tx) => match writer.flush() {
                        Ok(()) => {
                            let _ = ack_tx.send(Ok(()));
                        }
                        Err(err) => {
                            let _ = ack_tx.send(Err(io::Error::new(err.kind(), err.to_string())));
                            return Err(err);
                        }
                    },
                }
            }
            writer.flush()?;
            Ok(())
        });

        Self {
            sender: Some(tx),
            handle: Some(handle),
            buffer: Vec::with_capacity(buffer_size),
            buffer_size,
        }
    }

    fn send_buffer(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let tx = self
            .sender
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "AsyncWriter: writer already finalized"))?;

        let data = std::mem::replace(&mut self.buffer, Vec::with_capacity(self.buffer_size));
        tx.send(WriteMessage::Data(WriteBuffer { data }))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "AsyncWriter: background thread gone"))?;
        Ok(())
    }
}

impl Write for AsyncWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        if self.buffer.len() >= self.buffer_size {
            self.send_buffer()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.send_buffer()?;

        let tx = self
            .sender
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "AsyncWriter: writer already finalized"))?;

        let (ack_tx, ack_rx) = bounded(1);
        tx.send(WriteMessage::Flush(ack_tx))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "AsyncWriter: background thread gone"))?;

        ack_rx
            .recv()
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "AsyncWriter: flush acknowledgement failed"))??;

        Ok(())
    }
}

impl Drop for AsyncWriter {
    fn drop(&mut self) {
        if self.handle.is_none() {
            return;
        }

        if let Err(e) = self.send_buffer() {
            log::error!("AsyncWriter drop: failed to send buffer: {e}");
        }

        self.sender.take();
        if let Some(handle) = self.handle.take() {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => log::error!("AsyncWriter drop: background thread failed: {e}"),
                Err(e) => log::error!("AsyncWriter drop: background thread panicked: {e:?}"),
            }
        }
    }
}
