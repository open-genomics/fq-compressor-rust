# Directory Structure

> How backend code is organized in this project.

---

## Overview

`fqc` is a Rust CLI application for FASTQ compression. The codebase follows a modular architecture with clear separation between algorithms, I/O, commands, and pipeline logic.

---

## Directory Layout

```
src/
├── main.rs              # CLI entry point, clap definitions
├── lib.rs               # Library root, module declarations
├── error.rs             # Error types and exit codes
├── types.rs             # Shared type definitions (constants, structs)
├── format.rs            # File format definitions (header, block structures)
├── fqc_reader.rs        # Archive reader implementation
├── fqc_writer.rs        # Archive writer implementation
├── archive_traits.rs    # Traits for archive operations
├── algo/                # Compression algorithms
│   ├── mod.rs
│   ├── abc.rs           # ABC algorithm (consensus-based)
│   ├── block_compressor.rs
│   ├── dna.rs           # DNA encoding utilities
│   ├── global_analyzer.rs
│   ├── id_compressor.rs
│   └── quality_compressor.rs
├── commands/            # CLI command implementations
│   ├── mod.rs
│   ├── compress.rs
│   ├── decompress.rs
│   ├── info.rs
│   └── verify.rs
├── common/              # Shared utilities
│   ├── mod.rs
│   └── memory_budget.rs
├── fastq/               # FASTQ parsing
│   ├── mod.rs
│   └── parser.rs
├── io/                  # I/O abstractions
│   ├── mod.rs
│   ├── compressed_stream.rs
│   └── async_io.rs
└── pipeline/            # Processing pipelines
    ├── mod.rs
    ├── compression.rs
    └── decompression.rs
```

---

## Module Organization

### When adding new features:

1. **New CLI command**: Add to `src/commands/`
   - Create `src/commands/<name>.rs`
   - Export from `src/commands/mod.rs`
   - Add variant to `Commands` enum in `main.rs`

2. **New compression algorithm**: Add to `src/algo/`
   - Create `src/algo/<name>.rs`
   - Export from `src/algo/mod.rs`
   - Integrate via `block_compressor.rs` if needed

3. **New I/O handling**: Add to `src/io/`
   - Create `src/io/<name>.rs`
   - Export from `src/io/mod.rs`

4. **Shared utilities**: Add to `src/common/`

5. **New types/constants**: Add to `src/types.rs`

---

## Naming Conventions

| Category | Convention | Example |
|----------|------------|---------|
| Files | snake_case | `block_compressor.rs` |
| Modules | snake_case | `pub mod block_compressor;` |
| Structs | PascalCase | `struct FqcReader` |
| Enums | PascalCase | `enum FqcError` |
| Functions | snake_case | `fn compress_block()` |
| Constants | SCREAMING_SNAKE | `const MAGIC_BYTES: &[u8]` |
| Type aliases | PascalCase | `type Result<T> = std::result::Result<T, FqcError>;` |

---

## Examples

### Well-organized modules:

- **`src/commands/`**: Each command has its own file with a clear `Command` struct and `Options` struct
- **`src/algo/`**: Each algorithm is self-contained with focused responsibility
- **`src/error.rs`**: Centralized error handling with clear variants

---

## Anti-patterns to Avoid

- Do not add business logic to `main.rs` — delegate to command modules
- Do not create circular dependencies between modules
- Do not put multiple unrelated concerns in a single file
