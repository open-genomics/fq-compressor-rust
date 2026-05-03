# Logging Guidelines

> How logging is done in this project.

---

## Overview

This project uses the `log` crate facade with `env_logger` as the backend. Logging is for CLI status output, not for application telemetry.

---

## Log Levels

| Level | When to Use | Example |
|-------|-------------|---------|
| `error` | Unrecoverable errors, operation failures | Compression failed, file corruption detected |
| `warn` | Recoverable issues, unexpected states | Odd number of reads in paired-end file |
| `info` | Progress updates, operation milestones | "Compression complete! 42 blocks written" |
| `debug` | Detailed progress, internal state | Per-block compression details |

### CLI Verbosity Mapping

```rust
// main.rs
match cli.verbose {
    0 => env_logger::Builder::from_default_env().filter_level(log::LevelFilter::Warn),
    1 => env_logger::Builder::from_default_env().filter_level(log::LevelFilter::Info),
    _ => env_logger::Builder::from_default_env().filter_level(log::LevelFilter::Debug),
}
```

- Default: `WARN` (only warnings and errors)
- `-v`: `INFO` (progress updates)
- `-vv`: `DEBUG` (detailed internals)

---

## What to Log

### Always Log (info level)

- Operation start: "Compressing input.fastq → archive.fqc"
- Key parameters: block size, compression level, thread count
- Operation completion: "Compression complete! X blocks written"
- Significant milestones: "Loaded X reads (Y bases)"

### Log on Conditions (debug level)

- Per-block progress in streaming mode
- Algorithm selection decisions
- Memory allocation decisions

### Log on Issues (warn/error)

- Non-fatal anomalies: "Odd number of reads in interleaved file"
- Recoverable degradation: "Falling back to single-threaded mode"

---

## What NOT to Log

- File contents (FASTQ data, sequences)
- Checksums or hash values (except on mismatch)
- Internal buffer addresses or pointers
- User paths beyond the filename for operation status

---

## Logging Patterns

### Basic logging

```rust
use log::{info, warn, debug};

info!("Compression complete! {} blocks written.", stats.blocks_written);
warn!("Odd number of reads in interleaved file, last read treated as unpaired");
debug!("Block {} compressed: {} → {} bytes", block_id, original, compressed);
```

### Conditional detailed logging

```rust
if log::log_enabled!(log::Level::Debug) {
    debug!("Read length class: {}", length_class.as_str());
    debug!("Block size: {}", block_size);
}
```

---

## Examples from Codebase

### Command progress

```rust
// src/commands/compress.rs
log::info!("Reading input file: {}", self.opts.input_path);
log::info!("Loaded {} reads ({} bases)", records.len(), total_bases);
log::info!("Compression complete! {} blocks written.", self.stats.blocks_written);
```

### Mode announcements

```rust
log::info!("Streaming compression mode");
log::info!("Streaming compression mode (interleaved single-file PE)");
log::info!("Streaming compression mode (paired-end)");
```

### Warnings for edge cases

```rust
log::warn!("Odd number of reads in interleaved file, last read treated as unpaired");
```
