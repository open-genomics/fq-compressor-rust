# Database Schema Specifications

This directory would contain database schema definitions if fqc required persistent storage.

## Current Status

**fqc is a CLI tool** and does not use databases. All data is stored in FQC archive files (binary format).

## Data Persistence

fqc uses a custom binary format for data storage:

- **File Format**: Block-indexed binary format
- **Specification**: [../product/file-format.md](../product/file-format.md)
- **Implementation**: `src/format.rs`, `src/fqc_reader.rs`, `src/fqc_writer.rs`

## Future Considerations

If database support is added in the future (e.g., for compression metadata indexing), schema definitions will be added here using:

- **DBML** (Database Markup Language) for schema visualization
- **Migration files** for version control
- **Index specifications** for query optimization

## Related Directories

- [../product/file-format.md](../product/file-format.md) - Binary format specification
- [../rfc/0001-core-architecture.md](../rfc/0001-core-architecture.md) - Architecture overview
