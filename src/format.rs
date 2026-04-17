// =============================================================================
// fqc-rust - Binary Format Definitions (.fqc format)
// =============================================================================
//
// File Layout:
// +----------------+
// |  Magic Header  |  (9 bytes: 8 magic + 1 version)
// +----------------+
// | Global Header  |  (Variable Length, min 34 bytes)
// +----------------+
// |    Block 0     |
// +----------------+
// |    Block 1     |
// +----------------+
// |      ...       |
// +----------------+
// |    Block N     |
// +----------------+
// | Reorder Map    |  (Optional, Variable Length)
// +----------------+
// |   Block Index  |  (Variable Length)
// +----------------+
// |  File Footer   |  (32 bytes)
// +----------------+

use crate::error::{FqcError, Result};
use crate::types::*;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

// =============================================================================
// Magic Constants
// =============================================================================

pub const MAGIC_BYTES: [u8; 8] = [0x89, b'F', b'Q', b'C', 0x0D, 0x0A, 0x1A, 0x0A];
pub const MAGIC_END: [u8; 8] = [b'F', b'Q', b'C', b'_', b'E', b'O', b'F', 0x00];

pub const FORMAT_VERSION_MAJOR: u8 = 2;
pub const FORMAT_VERSION_MINOR: u8 = 0;
pub const CURRENT_VERSION: u8 = (FORMAT_VERSION_MAJOR << 4) | FORMAT_VERSION_MINOR;

pub const MAGIC_HEADER_SIZE: usize = 9;
pub const FILE_FOOTER_SIZE: usize = 32;

// =============================================================================
// Flag Bit Definitions
// =============================================================================

pub mod flags {
    pub const IS_PAIRED: u64 = 1 << 0;
    pub const PRESERVE_ORDER: u64 = 1 << 1;
    pub const LEGACY_LONG_READ_MODE: u64 = 1 << 2;
    pub const QUALITY_MODE_MASK: u64 = 0x3 << 3;
    pub const QUALITY_MODE_SHIFT: u8 = 3;
    pub const ID_MODE_MASK: u64 = 0x3 << 5;
    pub const ID_MODE_SHIFT: u8 = 5;
    pub const HAS_REORDER_MAP: u64 = 1 << 7;
    pub const PE_LAYOUT_MASK: u64 = 0x3 << 8;
    pub const PE_LAYOUT_SHIFT: u8 = 8;
    pub const READ_LENGTH_CLASS_MASK: u64 = 0x3 << 10;
    pub const READ_LENGTH_CLASS_SHIFT: u8 = 10;
    pub const STREAMING_MODE: u64 = 1 << 12;
}

#[allow(clippy::too_many_arguments)]
pub fn build_flags(
    is_paired: bool,
    preserve_order: bool,
    quality_mode: QualityMode,
    id_mode: IdMode,
    has_reorder_map: bool,
    pe_layout: PeLayout,
    read_length_class: ReadLengthClass,
    streaming_mode: bool,
) -> u64 {
    let mut f: u64 = 0;
    if is_paired {
        f |= flags::IS_PAIRED;
    }
    if preserve_order {
        f |= flags::PRESERVE_ORDER;
    }
    f = (f & !flags::QUALITY_MODE_MASK) | ((quality_mode as u64) << flags::QUALITY_MODE_SHIFT);
    f = (f & !flags::ID_MODE_MASK) | ((id_mode as u64) << flags::ID_MODE_SHIFT);
    if has_reorder_map {
        f |= flags::HAS_REORDER_MAP;
    }
    f = (f & !flags::PE_LAYOUT_MASK) | ((pe_layout as u64) << flags::PE_LAYOUT_SHIFT);
    f = (f & !flags::READ_LENGTH_CLASS_MASK) | ((read_length_class as u64) << flags::READ_LENGTH_CLASS_SHIFT);
    if streaming_mode {
        f |= flags::STREAMING_MODE;
    }
    f
}

pub fn get_quality_mode(f: u64) -> QualityMode {
    QualityMode::from_u8(((f & flags::QUALITY_MODE_MASK) >> flags::QUALITY_MODE_SHIFT) as u8)
}

pub fn get_id_mode(f: u64) -> IdMode {
    IdMode::from_u8(((f & flags::ID_MODE_MASK) >> flags::ID_MODE_SHIFT) as u8)
}

pub fn get_pe_layout(f: u64) -> PeLayout {
    PeLayout::from_u8(((f & flags::PE_LAYOUT_MASK) >> flags::PE_LAYOUT_SHIFT) as u8)
}

pub fn get_read_length_class(f: u64) -> ReadLengthClass {
    ReadLengthClass::from_u8(((f & flags::READ_LENGTH_CLASS_MASK) >> flags::READ_LENGTH_CLASS_SHIFT) as u8)
}

// =============================================================================
// GlobalHeader
// =============================================================================

/// Minimum size: 34 bytes
pub const GLOBAL_HEADER_MIN_SIZE: usize = 34;

#[derive(Debug, Clone, Default)]
pub struct GlobalHeader {
    pub header_size: u32,
    pub flags: u64,
    pub compression_algo: u8,
    pub checksum_type: u8,
    pub reserved: u16,
    pub total_read_count: u64,
    pub original_filename: String,
    pub timestamp: u64,
}

impl GlobalHeader {
    pub fn new(flags: u64, total_read_count: u64, original_filename: &str, timestamp: u64) -> Self {
        let fname_bytes = original_filename.len();
        let header_size = (GLOBAL_HEADER_MIN_SIZE + fname_bytes) as u32;
        Self {
            header_size,
            flags,
            compression_algo: 0,
            checksum_type: ChecksumType::XxHash64 as u8,
            reserved: 0,
            total_read_count,
            original_filename: original_filename.to_string(),
            timestamp,
        }
    }

    pub fn write<W: Write>(&self, w: &mut W) -> Result<usize> {
        let fname_bytes = self.original_filename.as_bytes();
        let fname_len = fname_bytes.len() as u16;
        let actual_size = GLOBAL_HEADER_MIN_SIZE + fname_bytes.len();

        w.write_u32::<LittleEndian>(actual_size as u32)?;
        w.write_u64::<LittleEndian>(self.flags)?;
        w.write_u8(self.compression_algo)?;
        w.write_u8(self.checksum_type)?;
        w.write_u16::<LittleEndian>(self.reserved)?;
        w.write_u64::<LittleEndian>(self.total_read_count)?;
        w.write_u16::<LittleEndian>(fname_len)?;
        w.write_all(fname_bytes)?;
        w.write_u64::<LittleEndian>(self.timestamp)?;
        Ok(actual_size)
    }

    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let header_size = r.read_u32::<LittleEndian>()?;
        let flags = r.read_u64::<LittleEndian>()?;
        let compression_algo = r.read_u8()?;
        let checksum_type = r.read_u8()?;
        let reserved = r.read_u16::<LittleEndian>()?;
        let total_read_count = r.read_u64::<LittleEndian>()?;
        let fname_len = r.read_u16::<LittleEndian>()? as usize;
        let mut fname_buf = vec![0u8; fname_len];
        r.read_exact(&mut fname_buf)?;
        let original_filename =
            String::from_utf8(fname_buf).map_err(|e| FqcError::Format(format!("Invalid filename: {e}")))?;
        let timestamp = r.read_u64::<LittleEndian>()?;

        // Skip any extra bytes in the header
        let read_so_far = GLOBAL_HEADER_MIN_SIZE + fname_len;
        if header_size as usize > read_so_far {
            let extra = header_size as usize - read_so_far;
            let mut skip = vec![0u8; extra];
            r.read_exact(&mut skip)?;
        }

        if reserved != 0 {
            return Err(FqcError::Format("Reserved field in GlobalHeader must be 0".to_string()));
        }

        Ok(Self {
            header_size,
            flags,
            compression_algo,
            checksum_type,
            reserved,
            total_read_count,
            original_filename,
            timestamp,
        })
    }
}

// =============================================================================
// BlockHeader (104 bytes fixed)
// =============================================================================

pub const BLOCK_HEADER_SIZE: usize = 104;

#[derive(Debug, Clone, Default)]
pub struct BlockHeader {
    pub header_size: u32,
    pub block_id: u32,
    pub checksum_type: u8,
    pub codec_ids: u8,
    pub codec_seq: u8,
    pub codec_qual: u8,
    pub codec_aux: u8,
    pub reserved1: u8,
    pub reserved2: u16,
    pub block_xxhash64: u64,
    pub uncompressed_count: u32,
    pub uniform_read_length: u32,
    pub compressed_size: u64,
    pub offset_ids: u64,
    pub offset_seq: u64,
    pub offset_qual: u64,
    pub offset_aux: u64,
    pub size_ids: u64,
    pub size_seq: u64,
    pub size_qual: u64,
    pub size_aux: u64,
}

impl BlockHeader {
    pub fn write<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_u32::<LittleEndian>(BLOCK_HEADER_SIZE as u32)?; // header_size
        w.write_u32::<LittleEndian>(self.block_id)?;
        w.write_u8(self.checksum_type)?;
        w.write_u8(self.codec_ids)?;
        w.write_u8(self.codec_seq)?;
        w.write_u8(self.codec_qual)?;
        w.write_u8(self.codec_aux)?;
        w.write_u8(self.reserved1)?;
        w.write_u16::<LittleEndian>(self.reserved2)?;
        w.write_u64::<LittleEndian>(self.block_xxhash64)?;
        w.write_u32::<LittleEndian>(self.uncompressed_count)?;
        w.write_u32::<LittleEndian>(self.uniform_read_length)?;
        w.write_u64::<LittleEndian>(self.compressed_size)?;
        w.write_u64::<LittleEndian>(self.offset_ids)?;
        w.write_u64::<LittleEndian>(self.offset_seq)?;
        w.write_u64::<LittleEndian>(self.offset_qual)?;
        w.write_u64::<LittleEndian>(self.offset_aux)?;
        w.write_u64::<LittleEndian>(self.size_ids)?;
        w.write_u64::<LittleEndian>(self.size_seq)?;
        w.write_u64::<LittleEndian>(self.size_qual)?;
        w.write_u64::<LittleEndian>(self.size_aux)?;
        Ok(())
    }

    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let header_size = r.read_u32::<LittleEndian>()?;
        let block_id = r.read_u32::<LittleEndian>()?;
        let checksum_type = r.read_u8()?;
        let codec_ids = r.read_u8()?;
        let codec_seq = r.read_u8()?;
        let codec_qual = r.read_u8()?;
        let codec_aux = r.read_u8()?;
        let reserved1 = r.read_u8()?;
        let reserved2 = r.read_u16::<LittleEndian>()?;
        let block_xxhash64 = r.read_u64::<LittleEndian>()?;
        let uncompressed_count = r.read_u32::<LittleEndian>()?;
        let uniform_read_length = r.read_u32::<LittleEndian>()?;
        let compressed_size = r.read_u64::<LittleEndian>()?;
        let offset_ids = r.read_u64::<LittleEndian>()?;
        let offset_seq = r.read_u64::<LittleEndian>()?;
        let offset_qual = r.read_u64::<LittleEndian>()?;
        let offset_aux = r.read_u64::<LittleEndian>()?;
        let size_ids = r.read_u64::<LittleEndian>()?;
        let size_seq = r.read_u64::<LittleEndian>()?;
        let size_qual = r.read_u64::<LittleEndian>()?;
        let size_aux = r.read_u64::<LittleEndian>()?;

        // Skip extra bytes if header is larger
        if header_size as usize > BLOCK_HEADER_SIZE {
            let extra = header_size as usize - BLOCK_HEADER_SIZE;
            let mut skip = vec![0u8; extra];
            r.read_exact(&mut skip)?;
        }

        if reserved1 != 0 || reserved2 != 0 {
            return Err(FqcError::Format("Reserved fields in BlockHeader must be 0".to_string()));
        }

        Ok(Self {
            header_size,
            block_id,
            checksum_type,
            codec_ids,
            codec_seq,
            codec_qual,
            codec_aux,
            reserved1,
            reserved2,
            block_xxhash64,
            uncompressed_count,
            uniform_read_length,
            compressed_size,
            offset_ids,
            offset_seq,
            offset_qual,
            offset_aux,
            size_ids,
            size_seq,
            size_qual,
            size_aux,
        })
    }

    pub fn has_uniform_length(&self) -> bool {
        self.uniform_read_length > 0 && self.size_aux == 0
    }

    pub fn is_quality_discarded(&self) -> bool {
        self.size_qual == 0 && decode_codec_family(self.codec_qual) == CodecFamily::Raw
    }
}

// =============================================================================
// IndexEntry (28 bytes)
// =============================================================================

pub const INDEX_ENTRY_SIZE: usize = 28;

#[derive(Debug, Clone, Default)]
pub struct IndexEntry {
    pub offset: u64,
    pub compressed_size: u64,
    pub archive_id_start: u64,
    pub read_count: u32,
}

impl IndexEntry {
    pub fn archive_id_end(&self) -> u64 {
        self.archive_id_start + self.read_count as u64
    }

    pub fn contains_read(&self, archive_id: u64) -> bool {
        archive_id >= self.archive_id_start && archive_id < self.archive_id_end()
    }

    pub fn write<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_u64::<LittleEndian>(self.offset)?;
        w.write_u64::<LittleEndian>(self.compressed_size)?;
        w.write_u64::<LittleEndian>(self.archive_id_start)?;
        w.write_u32::<LittleEndian>(self.read_count)?;
        Ok(())
    }

    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let offset = r.read_u64::<LittleEndian>()?;
        let compressed_size = r.read_u64::<LittleEndian>()?;
        let archive_id_start = r.read_u64::<LittleEndian>()?;
        let read_count = r.read_u32::<LittleEndian>()?;
        Ok(Self {
            offset,
            compressed_size,
            archive_id_start,
            read_count,
        })
    }
}

// =============================================================================
// BlockIndex
// =============================================================================

pub const BLOCK_INDEX_HEADER_SIZE: usize = 16;

#[derive(Debug, Clone, Default)]
pub struct BlockIndex {
    pub num_blocks: u64,
    pub entries: Vec<IndexEntry>,
}

impl BlockIndex {
    pub fn write<W: Write>(&self, w: &mut W) -> Result<usize> {
        w.write_u32::<LittleEndian>(BLOCK_INDEX_HEADER_SIZE as u32)?;
        w.write_u32::<LittleEndian>(INDEX_ENTRY_SIZE as u32)?;
        w.write_u64::<LittleEndian>(self.num_blocks)?;
        for entry in &self.entries {
            entry.write(w)?;
        }
        Ok(BLOCK_INDEX_HEADER_SIZE + self.entries.len() * INDEX_ENTRY_SIZE)
    }

    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let header_size = r.read_u32::<LittleEndian>()? as usize;
        let entry_size = r.read_u32::<LittleEndian>()? as usize;
        let num_blocks = r.read_u64::<LittleEndian>()?;

        if entry_size < INDEX_ENTRY_SIZE {
            return Err(FqcError::Format(format!(
                "BlockIndex entry size {entry_size} < required {INDEX_ENTRY_SIZE}"
            )));
        }

        // Skip extra header bytes
        if header_size > BLOCK_INDEX_HEADER_SIZE {
            let extra = header_size - BLOCK_INDEX_HEADER_SIZE;
            let mut skip = vec![0u8; extra];
            r.read_exact(&mut skip)?;
        }

        let mut entries = Vec::with_capacity(num_blocks as usize);
        for _ in 0..num_blocks {
            let entry = IndexEntry::read(r)?;
            // Skip extra entry bytes for forward compatibility
            if entry_size > INDEX_ENTRY_SIZE {
                let extra = entry_size - INDEX_ENTRY_SIZE;
                let mut skip = vec![0u8; extra];
                r.read_exact(&mut skip)?;
            }
            entries.push(entry);
        }

        Ok(Self { num_blocks, entries })
    }
}

// =============================================================================
// ReorderMapHeader
// =============================================================================

pub const REORDER_MAP_HEADER_SIZE: usize = 32;

#[derive(Debug, Clone, Default)]
pub struct ReorderMapHeader {
    pub version: u32,
    pub total_reads: u64,
    pub forward_map_size: u64,
    pub reverse_map_size: u64,
}

impl ReorderMapHeader {
    pub fn write<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_u32::<LittleEndian>(REORDER_MAP_HEADER_SIZE as u32)?;
        w.write_u32::<LittleEndian>(self.version)?;
        w.write_u64::<LittleEndian>(self.total_reads)?;
        w.write_u64::<LittleEndian>(self.forward_map_size)?;
        w.write_u64::<LittleEndian>(self.reverse_map_size)?;
        Ok(())
    }

    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let header_size = r.read_u32::<LittleEndian>()? as usize;
        let version = r.read_u32::<LittleEndian>()?;
        let total_reads = r.read_u64::<LittleEndian>()?;
        let forward_map_size = r.read_u64::<LittleEndian>()?;
        let reverse_map_size = r.read_u64::<LittleEndian>()?;

        if header_size > REORDER_MAP_HEADER_SIZE {
            let extra = header_size - REORDER_MAP_HEADER_SIZE;
            let mut skip = vec![0u8; extra];
            r.read_exact(&mut skip)?;
        }

        Ok(Self {
            version,
            total_reads,
            forward_map_size,
            reverse_map_size,
        })
    }
}

// =============================================================================
// FileFooter (32 bytes)
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct FileFooter {
    pub index_offset: u64,
    pub reorder_map_offset: u64,
    pub global_checksum: u64,
    pub magic_end: [u8; 8],
}

impl FileFooter {
    pub fn new(index_offset: u64, reorder_map_offset: u64, global_checksum: u64) -> Self {
        Self {
            index_offset,
            reorder_map_offset,
            global_checksum,
            magic_end: MAGIC_END,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic_end == MAGIC_END
    }

    pub fn has_reorder_map(&self) -> bool {
        self.reorder_map_offset != 0
    }

    pub fn write<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_u64::<LittleEndian>(self.index_offset)?;
        w.write_u64::<LittleEndian>(self.reorder_map_offset)?;
        w.write_u64::<LittleEndian>(self.global_checksum)?;
        w.write_all(&self.magic_end)?;
        Ok(())
    }

    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let index_offset = r.read_u64::<LittleEndian>()?;
        let reorder_map_offset = r.read_u64::<LittleEndian>()?;
        let global_checksum = r.read_u64::<LittleEndian>()?;
        let mut magic_end = [0u8; 8];
        r.read_exact(&mut magic_end)?;

        let footer = Self {
            index_offset,
            reorder_map_offset,
            global_checksum,
            magic_end,
        };
        if !footer.is_valid() {
            return Err(FqcError::Format("Invalid file footer magic".to_string()));
        }
        Ok(footer)
    }
}

// =============================================================================
// Validation
// =============================================================================

pub fn validate_magic(data: &[u8]) -> bool {
    data.len() >= 8 && data[..8] == MAGIC_BYTES
}

pub fn is_version_compatible(version: u8) -> bool {
    let major = version >> 4;
    major == FORMAT_VERSION_MAJOR
}
