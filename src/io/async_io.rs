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

use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
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
    pub stalls: AtomicU64,
}

impl AsyncIOStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_transfer(&self, bytes: u64) {
        self.bytes_transferred.fetch_add(bytes, Ordering::Relaxed);
        self.operations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_stall(&self) {
        self.stalls.fetch_add(1, Ordering::Relaxed);
    }

    pub fn bytes(&self) -> u64 {
        self.bytes_transferred.load(Ordering::Relaxed)
    }

    pub fn ops(&self) -> u64 {
        self.operations.load(Ordering::Relaxed)
    }

    pub fn stalls(&self) -> u64 {
        self.stalls.load(Ordering::Relaxed)
    }
}

// =============================================================================
// BufferPool
// =============================================================================

/// Thread-safe pool of reusable byte buffers
pub struct BufferPool {
    pool: Mutex<Vec<Vec<u8>>>,
    buffer_size: usize,
}

impl BufferPool {
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        let mut pool = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            pool.push(vec![0u8; buffer_size]);
        }
        Self {
            pool: Mutex::new(pool),
            buffer_size,
        }
    }

    /// Acquire a buffer from the pool, or allocate a new one if empty
    pub fn acquire(&self) -> Vec<u8> {
        let mut pool = self.pool.lock().unwrap();
        pool.pop().unwrap_or_else(|| vec![0u8; self.buffer_size])
    }

    /// Return a buffer to the pool
    pub fn release(&self, mut buf: Vec<u8>) {
        buf.clear();
        let mut pool = self.pool.lock().unwrap();
        pool.push(buf);
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

// =============================================================================
// AsyncReader
// =============================================================================

/// Filled buffer with actual data length
struct FilledBuffer {
    data: Vec<u8>,
    len: usize,
}

/// Asynchronous reader with background prefetch thread.
/// Reads data ahead into a bounded queue of buffers.
pub struct AsyncReader {
    receiver: Receiver<FilledBuffer>,
    _handle: Option<thread::JoinHandle<io::Result<()>>>,
    current_buf: Vec<u8>,
    current_pos: usize,
    current_len: usize,
    eof: bool,
    stats: Arc<AsyncIOStats>,
}

impl AsyncReader {
    /// Create an AsyncReader wrapping the given reader.
    /// - `prefetch_depth`: number of buffers to read ahead (bounded channel size)
    /// - `buffer_size`: size of each read buffer in bytes
    pub fn new<R: Read + Send + 'static>(mut reader: R, prefetch_depth: usize, buffer_size: usize) -> Self {
        let (tx, rx): (Sender<FilledBuffer>, Receiver<FilledBuffer>) = bounded(prefetch_depth);
        let stats = Arc::new(AsyncIOStats::new());
        let bg_stats = stats.clone();

        let handle = thread::spawn(move || -> io::Result<()> {
            loop {
                let mut buf = vec![0u8; buffer_size];
                let mut total_read = 0;

                // Fill the buffer as much as possible
                while total_read < buffer_size {
                    match reader.read(&mut buf[total_read..]) {
                        Ok(0) => break, // EOF
                        Ok(n) => total_read += n,
                        Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(e) => return Err(e),
                    }
                }

                if total_read == 0 {
                    // EOF: send an empty buffer to signal completion
                    let _ = tx.send(FilledBuffer { data: buf, len: 0 });
                    break;
                }

                bg_stats.add_transfer(total_read as u64);

                if tx
                    .send(FilledBuffer {
                        data: buf,
                        len: total_read,
                    })
                    .is_err()
                {
                    break; // Receiver dropped
                }
            }
            Ok(())
        });

        Self {
            receiver: rx,
            _handle: Some(handle),
            current_buf: Vec::new(),
            current_pos: 0,
            current_len: 0,
            eof: false,
            stats,
        }
    }

    pub fn stats(&self) -> &Arc<AsyncIOStats> {
        &self.stats
    }

    fn fill_buffer(&mut self) -> io::Result<bool> {
        if self.eof {
            return Ok(false);
        }

        match self.receiver.recv() {
            Ok(filled) => {
                if filled.len == 0 {
                    self.eof = true;
                    return Ok(false);
                }
                self.current_buf = filled.data;
                self.current_pos = 0;
                self.current_len = filled.len;
                Ok(true)
            }
            Err(_) => {
                self.eof = true;
                Ok(false)
            }
        }
    }
}

impl Read for AsyncReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.current_pos >= self.current_len {
            if !self.fill_buffer()? {
                return Ok(0);
            }
        }

        let available = self.current_len - self.current_pos;
        let to_copy = available.min(buf.len());
        buf[..to_copy].copy_from_slice(&self.current_buf[self.current_pos..self.current_pos + to_copy]);
        self.current_pos += to_copy;
        Ok(to_copy)
    }
}

// =============================================================================
// AsyncWriter
// =============================================================================

/// Data buffer to be written
struct WriteBuffer {
    data: Vec<u8>,
}

/// Asynchronous writer with background write-behind thread.
/// Buffers data and writes it in a background thread.
pub struct AsyncWriter {
    sender: Option<Sender<WriteBuffer>>,
    handle: Option<thread::JoinHandle<io::Result<()>>>,
    buffer: Vec<u8>,
    buffer_size: usize,
    stats: Arc<AsyncIOStats>,
}

impl AsyncWriter {
    /// Create an AsyncWriter wrapping the given writer.
    /// - `queue_depth`: number of write buffers in flight
    /// - `buffer_size`: size threshold to trigger a flush to the background thread
    pub fn new<W: Write + Send + 'static>(mut writer: W, queue_depth: usize, buffer_size: usize) -> Self {
        let (tx, rx): (Sender<WriteBuffer>, Receiver<WriteBuffer>) = bounded(queue_depth);
        let stats = Arc::new(AsyncIOStats::new());
        let bg_stats = stats.clone();

        let handle = thread::spawn(move || -> io::Result<()> {
            for wb in rx.iter() {
                writer.write_all(&wb.data)?;
                bg_stats.add_transfer(wb.data.len() as u64);
            }
            writer.flush()?;
            Ok(())
        });

        Self {
            sender: Some(tx),
            handle: Some(handle),
            buffer: Vec::with_capacity(buffer_size),
            buffer_size,
            stats,
        }
    }

    pub fn stats(&self) -> &Arc<AsyncIOStats> {
        &self.stats
    }

    fn send_buffer(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let data = std::mem::replace(&mut self.buffer, Vec::with_capacity(self.buffer_size));
        if let Some(ref tx) = self.sender {
            tx.send(WriteBuffer { data })
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "AsyncWriter: background thread gone"))?;
        }
        Ok(())
    }

    /// Finalize: flush remaining data and wait for the background thread to finish
    pub fn finalize(mut self) -> io::Result<()> {
        self.send_buffer()?;
        // Drop sender to signal background thread to stop
        self.sender.take();
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| io::Error::other("AsyncWriter: background thread panicked"))??;
        }
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
        // Drop sender to signal background thread to finish
        self.sender.take();
        // Wait for background thread and propagate errors
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| io::Error::other("AsyncWriter: background thread panicked"))??;
        }
        Ok(())
    }
}

impl Drop for AsyncWriter {
    fn drop(&mut self) {
        if let Err(e) = self.send_buffer() {
            log::error!("AsyncWriter drop: failed to send buffer: {e}");
        }
        self.sender.take();
        if let Some(handle) = self.handle.take() {
            if let Err(e) = handle.join() {
                log::error!("AsyncWriter drop: background thread panicked: {e:?}");
            }
        }
    }
}

// =============================================================================
// DoubleBuffer
// =============================================================================

/// Double-buffered (ping-pong) container for zero-copy swapping
pub struct DoubleBuffer<T: Default> {
    front: T,
    back: T,
}

impl<T: Default> DoubleBuffer<T> {
    pub fn new() -> Self {
        Self {
            front: T::default(),
            back: T::default(),
        }
    }

    pub fn front(&self) -> &T {
        &self.front
    }

    pub fn front_mut(&mut self) -> &mut T {
        &mut self.front
    }

    pub fn back(&self) -> &T {
        &self.back
    }

    pub fn back_mut(&mut self) -> &mut T {
        &mut self.back
    }

    /// Swap front and back buffers
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.front, &mut self.back);
    }
}

impl<T: Default> Default for DoubleBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}
