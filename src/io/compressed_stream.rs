// =============================================================================
// fqc-rust - Compressed Stream Module
// =============================================================================
// Independent module for transparent compression/decompression of file streams.
// Supports gzip, bzip2, xz, and zstd formats with magic-byte detection.
// =============================================================================

use crate::error::{FqcError, Result};
use std::io::{BufReader, Read};

// =============================================================================
// CompressionFormat
// =============================================================================

/// Detected compression format for input files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionFormat {
    Plain,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
}

impl CompressionFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Gzip => "gzip",
            Self::Bzip2 => "bzip2",
            Self::Xz => "xz",
            Self::Zstd => "zstd",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Plain => "",
            Self::Gzip => ".gz",
            Self::Bzip2 => ".bz2",
            Self::Xz => ".xz",
            Self::Zstd => ".zst",
        }
    }
}

// =============================================================================
// Format Detection
// =============================================================================

/// Magic byte signatures for compression formats
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
const BZIP2_MAGIC: [u8; 3] = [b'B', b'Z', b'h'];
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];
const XZ_MAGIC: [u8; 6] = [0xfd, b'7', b'z', b'X', b'Z', 0x00];

/// Detect compression format from magic bytes in a buffer
pub fn detect_format_from_bytes(magic: &[u8]) -> CompressionFormat {
    if magic.len() >= 2 && magic[..2] == GZIP_MAGIC {
        return CompressionFormat::Gzip;
    }
    if magic.len() >= 3 && magic[..3] == BZIP2_MAGIC {
        return CompressionFormat::Bzip2;
    }
    if magic.len() >= 4 && magic[..4] == ZSTD_MAGIC {
        return CompressionFormat::Zstd;
    }
    if magic.len() >= 6 && magic[..6] == XZ_MAGIC {
        return CompressionFormat::Xz;
    }
    CompressionFormat::Plain
}

/// Detect compression format from file extension
pub fn detect_format_from_extension(path: &str) -> CompressionFormat {
    let path = std::path::Path::new(path);
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("gz" | "gzip") => CompressionFormat::Gzip,
        Some("bz2") => CompressionFormat::Bzip2,
        Some("xz") => CompressionFormat::Xz,
        Some("zst" | "zstd") => CompressionFormat::Zstd,
        _ => CompressionFormat::Plain,
    }
}

/// Detect compression format by magic bytes first, falling back to extension
pub fn detect_compression_format(path: &str) -> CompressionFormat {
    if let Ok(mut f) = std::fs::File::open(path) {
        let mut magic = [0u8; 6];
        match std::io::Read::read(&mut f, &mut magic) {
            Ok(n) if n >= 2 => {
                let fmt = detect_format_from_bytes(&magic);
                if fmt != CompressionFormat::Plain {
                    return fmt;
                }
            }
            Ok(_) => {
                // File too small for magic detection, fall back to extension
                log::debug!("File too small for magic detection: {}", path);
            }
            Err(e) => {
                // Read error, fall back to extension
                log::debug!("Failed to read magic bytes from {}: {}", path, e);
            }
        }
    }
    detect_format_from_extension(path)
}

// =============================================================================
// Format Support Queries
// =============================================================================

/// Check if a specific compression format is supported
pub fn is_compression_supported(format: CompressionFormat) -> bool {
    match format {
        CompressionFormat::Plain | CompressionFormat::Zstd => true,
        #[cfg(feature = "gz")]
        CompressionFormat::Gzip => true,
        #[cfg(not(feature = "gz"))]
        CompressionFormat::Gzip => false,
        #[cfg(feature = "bz2")]
        CompressionFormat::Bzip2 => true,
        #[cfg(not(feature = "bz2"))]
        CompressionFormat::Bzip2 => false,
        #[cfg(feature = "xz")]
        CompressionFormat::Xz => true,
        #[cfg(not(feature = "xz"))]
        CompressionFormat::Xz => false,
    }
}

/// Get list of supported compression formats (filtered by enabled feature flags)
pub fn supported_formats() -> Vec<CompressionFormat> {
    let mut formats = vec![CompressionFormat::Plain];
    #[cfg(feature = "gz")]
    formats.push(CompressionFormat::Gzip);
    #[cfg(feature = "bz2")]
    formats.push(CompressionFormat::Bzip2);
    #[cfg(feature = "xz")]
    formats.push(CompressionFormat::Xz);
    formats.push(CompressionFormat::Zstd);
    formats
}

/// Get list of supported file extensions
pub fn supported_extensions() -> Vec<&'static str> {
    vec![".fastq", ".fq", ".gz", ".gzip", ".bz2", ".xz", ".zst", ".zstd"]
}

// =============================================================================
// Stream Opening
// =============================================================================

/// Open a file as a decompressed reader, auto-detecting compression format.
/// Returns a boxed reader that transparently decompresses.
pub fn open_compressed_reader(path: &str) -> Result<Box<dyn Read + Send>> {
    let format = detect_compression_format(path);
    let file = std::fs::File::open(path).map_err(|e| {
        FqcError::Io(std::io::Error::new(
            e.kind(),
            format!("Cannot open file '{}': {}", path, e),
        ))
    })?;

    log::debug!("Opening {} (format: {})", path, format.as_str());

    let reader: Box<dyn Read + Send> = match format {
        #[cfg(feature = "gz")]
        CompressionFormat::Gzip => Box::new(flate2::read::GzDecoder::new(file)),
        #[cfg(feature = "bz2")]
        CompressionFormat::Bzip2 => Box::new(bzip2::read::BzDecoder::new(file)),
        #[cfg(feature = "xz")]
        CompressionFormat::Xz => Box::new(xz2::read::XzDecoder::new(file)),
        CompressionFormat::Zstd => Box::new(
            zstd::Decoder::new(file)
                .map_err(|e| FqcError::Io(std::io::Error::other(format!("Zstd decoder init failed: {e}"))))?,
        ),
        CompressionFormat::Plain => Box::new(file),
        #[allow(unreachable_patterns)]
        other => {
            return Err(FqcError::UnsupportedFormat(format!(
                "Compression format {} not enabled (missing feature flag)",
                other.as_str()
            )));
        }
    };

    Ok(reader)
}

/// Open a file as a buffered decompressed reader
pub fn open_buffered_reader(path: &str) -> Result<BufReader<Box<dyn Read + Send>>> {
    let reader = open_compressed_reader(path)?;
    Ok(BufReader::new(reader))
}

/// Open stdin as a reader (plain text only)
pub fn open_stdin_reader() -> Box<dyn Read + Send> {
    Box::new(std::io::stdin())
}

/// Strip compression extension from a filename to get the base name
pub fn strip_compression_extension(path: &str) -> &str {
    let lower = path.to_lowercase();
    for ext in &[".gz", ".gzip", ".bz2", ".xz", ".zst", ".zstd"] {
        if lower.ends_with(ext) {
            return &path[..path.len() - ext.len()];
        }
    }
    path
}
