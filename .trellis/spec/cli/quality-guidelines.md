# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

This project enforces strict quality standards via CI. All code must pass `cargo fmt`, `cargo clippy`, and `cargo test` before merge.

---

## Validation Commands

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
```

Or use the validation script:

```bash
bash scripts/validate.sh full
```

---

## Forbidden Patterns

| Pattern | Reason |
|---------|--------|
| `unsafe` code | Safety rule: no new unsafe blocks |
| `unwrap()` in library code | Use `?` or proper error handling |
| `expect()` in library code | Use `?` with context instead |
| `panic!()` for expected failures | Return `FqcError` instead |
| `.clone()` on large data | Prefer references or ownership transfer |
| `String::from("")` for empty strings | Use `String::new()` or `Default::default()` |

---

## Required Patterns

### Error Handling

```rust
// Good: propagate errors
fn process(path: &str) -> Result<()> {
    let data = fs::read(path)?;
    Ok(())
}

// Bad: panic on error
fn process(path: &str) {
    let data = fs::read(path).unwrap(); // FORBIDDEN
}
```

### Documentation

```rust
/// Brief description.
///
/// # Arguments
/// * `path` - File path to read
///
/// # Returns
/// The parsed data or an error.
///
/// # Errors
/// Returns `FqcError::Io` if the file cannot be read.
pub fn parse_file(path: &str) -> Result<Data> { ... }
```

### Constants

```rust
// Good: defined in types.rs or at module level
const DEFAULT_BLOCK_SIZE: usize = 65536;
const MAGIC_BYTES: &[u8] = b"FQC1";

// Bad: magic numbers inline
if data.len() > 65536 { ... }  // Use constant instead
```

---

## Testing Requirements

### Unit Tests

- Place tests in the same file as the code: `#[cfg(test)] mod tests { ... }`
- Test edge cases: empty input, maximum sizes, error conditions
- Use `#[test]` attribute

### Integration Tests

- Place in `tests/` directory
- Test CLI commands via `std::process::Command` or test fixtures

### Example Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress_roundtrip() {
        let input = b"@read1\nACGT\n+\nIIII\n";
        let compressed = compress(input).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(input.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_empty_input() {
        let result = compress(&[]);
        assert!(result.is_err());
    }
}
```

---

## Code Review Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes
- [ ] No new `unsafe` code
- [ ] Errors are propagated with `?`, not `unwrap`
- [ ] Public functions have documentation comments
- [ ] No magic numbers — use named constants
- [ ] Log messages use appropriate levels

---

## CI Requirements

All PRs must pass:

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --lib --tests`
4. `cargo doc --no-deps`
5. `npm run docs:build` (VitePress docs)

---

## Examples from Codebase

### Error handling with context

```rust
// src/fqc_reader.rs
let header = Header::read(&mut reader)
    .map_err(|e| FqcError::Format(format!("Invalid header: {}", e)))?;
```

### Proper test structure

```rust
// In-file unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_validation() {
        // Test implementation
    }
}
```
