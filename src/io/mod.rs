// =============================================================================
// fqc-rust - I/O Module
// =============================================================================

pub mod async_io;
pub mod compressed_stream;
pub mod output_transaction;

pub use output_transaction::{begin_fqc_writer, commit_split, OutputTransaction};

/// Print a line to stdout, treating a closed pipe (`BrokenPipe`, e.g.
/// `fqc info | head`) as a normal end of output. Replaces `println!`, which
/// panics (exit 101) when the downstream consumer closes the pipe early.
#[macro_export]
macro_rules! println_stdout {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        if let Err(e) = writeln!(lock, $($arg)*) {
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                panic!("failed printing to stdout: {e}");
            }
        }
    }};
}

/// Print to stdout without a trailing newline, with the same closed-pipe
/// tolerance as [`println_stdout`].
#[macro_export]
macro_rules! print_stdout {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        if let Err(e) = write!(lock, $($arg)*) {
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                panic!("failed printing to stdout: {e}");
            }
        }
    }};
}
