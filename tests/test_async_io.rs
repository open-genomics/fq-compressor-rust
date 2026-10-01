// =============================================================================
// fqc-rust - AsyncWriter Direct Tests
// =============================================================================
// The async write-behind layer (background thread + channel + Drop-join)
// previously had no direct coverage despite being the easiest place to
// introduce deadlocks, reordered bytes, or silently lost data. These tests
// pin ordering, flush acknowledgement, and error propagation.
// =============================================================================

use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use fqc::io::async_io::AsyncWriter;

/// Inner writer shared with the test thread so contents can be observed
/// after the background thread consumes them.
#[derive(Clone)]
struct SharedSink(Arc<Mutex<Vec<u8>>>);

impl SharedSink {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    fn contents(&self) -> Vec<u8> {
        self.0.lock().unwrap().clone()
    }
}

impl Write for SharedSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Inner writer whose `write` starts failing after `fail_after` calls.
struct FailingWriter {
    fail_after: usize,
    calls: AtomicUsize,
}

impl Write for FailingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if call >= self.fail_after {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "inner writer failed"));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn dropped_writer_flushes_all_buffered_bytes() {
    let sink = SharedSink::new();
    {
        let mut w = AsyncWriter::new(sink.clone(), 4, 64);
        w.write_all(b"hello world").unwrap();
        // Deliberately below buffer_size: nothing was flushed yet, Drop must
        // send the remainder and join the thread without losing it.
    }
    assert_eq!(sink.contents(), b"hello world");
}

#[test]
fn large_stream_preserves_order_and_content() {
    let sink = SharedSink::new();
    let mut w = AsyncWriter::new(sink.clone(), 4, 256);

    // Many writes crossing the buffer threshold force repeated handoffs to
    // the background thread; byte identity proves ordering is preserved.
    let mut expected = Vec::new();
    for i in 0..2_000u32 {
        let chunk = format!("record-{i:06}\n");
        w.write_all(chunk.as_bytes()).unwrap();
        expected.extend_from_slice(chunk.as_bytes());
    }
    w.flush().unwrap();
    assert_eq!(sink.contents(), expected);
}

#[test]
fn flush_is_acknowledged_after_data_is_visible() {
    let sink = SharedSink::new();
    let mut w = AsyncWriter::new(sink.clone(), 2, 16);
    w.write_all(b"acked").unwrap();
    w.flush().unwrap();
    // Flush acks only after the background thread processed the queued data.
    assert_eq!(sink.contents(), b"acked");
}

#[test]
fn flush_returns_error_when_background_thread_dies() {
    // First background write fails; flush must surface an error instead of
    // pretending the data landed.
    let mut w = AsyncWriter::new(
        FailingWriter {
            fail_after: 0,
            calls: AtomicUsize::new(0),
        },
        2,
        4,
    );
    w.write_all(b"12345").unwrap();
    let err = w.flush().unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
}

#[test]
fn successful_writes_are_fully_delegated() {
    let sink = SharedSink::new();
    let mut w = AsyncWriter::new(sink.clone(), 8, 8);
    // Exactly at the threshold: first write triggers a send, second stays
    // buffered until flush.
    w.write_all(b"12345678").unwrap();
    w.write_all(b"9").unwrap();
    w.flush().unwrap();
    assert_eq!(sink.contents(), b"123456789");
}
