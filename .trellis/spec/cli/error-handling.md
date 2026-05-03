# Error Handling

> How errors are handled in this project.

---

## Overview

This project uses `thiserror` for error definitions and follows a centralized error handling pattern. All errors are defined in `src/error.rs` and propagated via `Result<T, FqcError>`.

---

## Error Types

Defined in `src/error.rs`:

```rust
#[derive(Debug, Error)]
pub enum FqcError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Format error: {0}")]
    Format(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Checksum mismatch: expected {expected:#x}, got {actual:#x}")]
    ChecksumMismatch { expected: u64, actual: u64 },

    #[error("Corrupted block {block_id}: {reason}")]
    CorruptedBlock { block_id: u32, reason: String },

    #[error("Unsupported format version: major={major}")]
    UnsupportedVersion { major: u8 },

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Out of range: {0}")]
    OutOfRange(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}
```

### Type Alias

```rust
pub type Result<T> = std::result::Result<T, FqcError>;
```

---

## Error Context

Use `ErrorContext` to add additional context:

```rust
let ctx = ErrorContext::new()
    .with_file("archive.fqc")
    .with_block(42)
    .with_offset(0x1000);

// Wrap error with context
return Err(err.with_context(&ctx));
```

---

## Exit Codes

Defined in `src/error.rs`:

```rust
pub enum ExitCode {
    Success = 0,
    Usage = 1,          // Invalid arguments, missing files
    IoError = 2,        // File not found, permission denied
    FormatError = 3,    // Invalid magic, bad header
    ChecksumError = 4,  // Integrity check failed
    UnsupportedError = 5, // Unsupported codec/version
}
```

Use `FqcError::exit_code()` to map errors to CLI exit codes.

---

## Error Handling Patterns

### Propagation

Use `?` operator for propagation:

```rust
fn process_file(path: &str) -> Result<()> {
    let mut file = File::open(path)?;
    let data = read_header(&mut file)?;
    // ...
    Ok(())
}
```

### Wrapping with Context

```rust
let reader = FqcReader::open(path)
    .map_err(|e| FqcError::Format(format!("Failed to open archive: {}", e)))?;
```

### CLI Error Handling

In `main.rs`:

```rust
fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        return e.exit_code();
    }
    ExitCode::Success
}
```

---

## Common Mistakes

1. **Using `unwrap()` in library code** — Always propagate errors with `?` or handle explicitly
2. **Losing error context** — Use `.with_context()` before returning
3. **Panic for expected failures** — Use `FqcError` variants instead
4. **Ignoring `Result`** — Always handle or propagate

---

## Examples from Codebase

### Block validation error

```rust
// src/commands/verify.rs
if calculated != stored {
    return Err(FqcError::ChecksumMismatch {
        expected: stored,
        actual: calculated,
    });
}
```

### Corrupted data error

```rust
// src/fqc_reader.rs
return Err(FqcError::CorruptedBlock {
    block_id,
    reason: "Invalid block header".to_string(),
});
```
