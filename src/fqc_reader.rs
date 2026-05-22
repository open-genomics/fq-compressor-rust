// =============================================================================
// fqc-rust - FQC Archive Reader
// =============================================================================

use crate::algo::block_compressor::delta_decode_ids;
use crate::archive_traits::{ArchiveReader, BlockData};
use crate::error::{FqcError, Result};
use crate::format::*;
use crate::types::{IdMode, PeLayout, QualityMode, ReadLengthClass};
use byteorder::ReadBytesExt;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

// =============================================================================
// ArchiveInfo
// =============================================================================

/// Structured information about an archive.
#[derive(Debug, Clone)]
pub struct ArchiveInfo {
    pub file_path: String,
    pub file_size: u64,
    pub total_reads: u64,
    pub num_blocks: usize,
    pub original_filename: String,
    pub timestamp: u64,
    pub is_paired: bool,
    pub has_reorder_map: bool,
    pub preserve_order: bool,
    pub streaming_mode: bool,
    pub quality_mode: QualityMode,
    pub id_mode: IdMode,
    pub pe_layout: PeLayout,
    pub read_length_class: ReadLengthClass,
}

// =============================================================================
// FqcReader
// =============================================================================

pub struct FqcReader {
    path: String,
    reader: BufReader<File>,
    pub global_header: GlobalHeader,
    pub footer: FileFooter,
    pub block_index: BlockIndex,
    pub file_size: u64,
    pub reorder_forward: Option<Vec<u64>>,
    pub reorder_reverse: Option<Vec<u64>>,
}

impl FqcReader {
    pub fn open(path: &str) -> Result<Self> {
        let file = File::open(path).map_err(|e| FqcError::Io(e))?;
        let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
        let mut reader = BufReader::new(file);

        // Read and validate magic + version
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if !validate_magic(&magic) {
            return Err(FqcError::Format("Invalid .fqc magic header".to_string()));
        }

        let version = reader.read_u8()?;
        if !is_version_compatible(version) {
            return Err(FqcError::UnsupportedVersion { major: version >> 4 });
        }

        // Read footer (seek to end - 32 bytes)
        if file_size < FILE_FOOTER_SIZE as u64 + MAGIC_HEADER_SIZE as u64 {
            return Err(FqcError::Format(
                "File too small to be a valid .fqc archive".to_string(),
            ));
        }
        let footer_pos = file_size - FILE_FOOTER_SIZE as u64;
        reader.seek(SeekFrom::Start(footer_pos))?;
        let footer = FileFooter::read(&mut reader)?;

        // Read global header (after magic)
        reader.seek(SeekFrom::Start(MAGIC_HEADER_SIZE as u64))?;
        let global_header = GlobalHeader::read(&mut reader)?;
        let header_end = reader.stream_position()?;

        Self::validate_footer_offsets(&footer, header_end, footer_pos)?;

        // Read block index
        reader.seek(SeekFrom::Start(footer.index_offset))?;
        let block_index = BlockIndex::read(&mut reader)?;
        Self::validate_block_index(&block_index, header_end, &footer, global_header.total_read_count)?;
        Self::validate_block_headers(&mut reader, &block_index, &footer)?;

        Ok(Self {
            path: path.to_string(),
            reader,
            global_header,
            footer,
            block_index,
            file_size,
            reorder_forward: None,
            reorder_reverse: None,
        })
    }

    fn validate_footer_offsets(footer: &FileFooter, header_end: u64, footer_pos: u64) -> Result<()> {
        if footer.index_offset < header_end || footer.index_offset >= footer_pos {
            return Err(FqcError::Format(format!(
                "Block index offset {} is outside archive data region",
                footer.index_offset
            )));
        }

        if footer.has_reorder_map()
            && (footer.reorder_map_offset < header_end || footer.reorder_map_offset >= footer.index_offset)
        {
            return Err(FqcError::Format(format!(
                "Reorder map offset {} is outside archive data region",
                footer.reorder_map_offset
            )));
        }

        Ok(())
    }

    fn validate_block_index(
        block_index: &BlockIndex,
        header_end: u64,
        footer: &FileFooter,
        total_read_count: u64,
    ) -> Result<()> {
        let data_end = if footer.has_reorder_map() {
            footer.reorder_map_offset
        } else {
            footer.index_offset
        };
        let mut previous_end = header_end;
        let mut expected_archive_id_start = 0u64;

        for (idx, entry) in block_index.entries.iter().enumerate() {
            if entry.compressed_size < BLOCK_HEADER_SIZE as u64 {
                return Err(FqcError::Format(format!(
                    "Block index entry {idx} compressed size {} is smaller than block header",
                    entry.compressed_size
                )));
            }
            if entry.offset < header_end {
                return Err(FqcError::Format(format!(
                    "Block index entry {idx} offset {} precedes block region",
                    entry.offset
                )));
            }
            if entry.offset < previous_end {
                return Err(FqcError::Format(format!(
                    "Block index entry {idx} overlaps or reorders block data"
                )));
            }
            if entry.archive_id_start != expected_archive_id_start {
                return Err(FqcError::Format(format!(
                    "Block index entry {idx} archive start {} does not match expected {}",
                    entry.archive_id_start, expected_archive_id_start
                )));
            }

            let entry_end = entry
                .offset
                .checked_add(entry.compressed_size)
                .ok_or_else(|| FqcError::Format(format!("Block index entry {idx} overflows archive offsets")))?;
            if entry_end > data_end {
                return Err(FqcError::Format(format!(
                    "Block index entry {idx} exceeds archive data region"
                )));
            }

            previous_end = entry_end;
            expected_archive_id_start = expected_archive_id_start
                .checked_add(entry.read_count as u64)
                .ok_or_else(|| FqcError::Format("Block index read counts overflow total read count".to_string()))?;
        }

        if expected_archive_id_start != total_read_count {
            return Err(FqcError::Format(format!(
                "Block index total read count {} does not match global header {}",
                expected_archive_id_start, total_read_count
            )));
        }

        Ok(())
    }

    fn validate_block_headers(
        reader: &mut BufReader<File>,
        block_index: &BlockIndex,
        footer: &FileFooter,
    ) -> Result<()> {
        let block_region_end = if footer.has_reorder_map() {
            footer.reorder_map_offset
        } else {
            footer.index_offset
        };

        for (idx, entry) in block_index.entries.iter().enumerate() {
            reader.seek(SeekFrom::Start(entry.offset))?;
            let header = BlockHeader::read(reader)?;

            if header.block_id != idx as u32 {
                return Err(FqcError::Format(format!(
                    "Block header id {} does not match block index position {}",
                    header.block_id, idx
                )));
            }
            if header.uncompressed_count != entry.read_count {
                return Err(FqcError::Format(format!(
                    "Block header read count {} does not match block index {} for block {}",
                    header.uncompressed_count, entry.read_count, idx
                )));
            }

            let block_end = entry
                .offset
                .checked_add(entry.compressed_size)
                .ok_or_else(|| FqcError::Format(format!("Block {} overflows archive offsets", idx)))?;
            let payload_start = entry
                .offset
                .checked_add(header.header_size as u64)
                .ok_or_else(|| FqcError::Format(format!("Block {} header overflows archive offsets", idx)))?;
            if payload_start > block_end {
                return Err(FqcError::Format(format!(
                    "Block {} stream extent exceeds block payload bounds",
                    idx
                )));
            }

            let declared_stream_end = [
                header.offset_ids.checked_add(header.size_ids),
                header.offset_seq.checked_add(header.size_seq),
                header.offset_qual.checked_add(header.size_qual),
                header.offset_aux.checked_add(header.size_aux),
            ]
            .into_iter()
            .flatten()
            .max()
            .ok_or_else(|| FqcError::Format(format!("Block {} stream extent overflows block payload", idx)))?;

            if declared_stream_end > header.compressed_size {
                return Err(FqcError::Format(format!(
                    "Block {} stream extent exceeds declared block payload",
                    idx
                )));
            }

            let declared_block_end = payload_start
                .checked_add(header.compressed_size)
                .ok_or_else(|| FqcError::Format(format!("Block {} payload overflows archive offsets", idx)))?;
            if declared_block_end > block_end || declared_block_end > block_region_end {
                return Err(FqcError::Format(format!(
                    "Block {} stream extent exceeds block payload bounds",
                    idx
                )));
            }
        }

        Ok(())
    }

    pub fn block_count(&self) -> usize {
        self.block_index.entries.len()
    }

    pub fn total_read_count(&self) -> u64 {
        self.global_header.total_read_count
    }

    /// Get structured information about this archive.
    pub fn info(&self) -> ArchiveInfo {
        let flags = self.global_header.flags;

        ArchiveInfo {
            file_path: self.path.clone(),
            file_size: self.file_size,
            total_reads: self.global_header.total_read_count,
            num_blocks: self.block_count(),
            original_filename: self.global_header.original_filename.clone(),
            timestamp: self.global_header.timestamp,
            is_paired: (flags & flags::IS_PAIRED) != 0,
            has_reorder_map: (flags & flags::HAS_REORDER_MAP) != 0,
            preserve_order: (flags & flags::PRESERVE_ORDER) != 0,
            streaming_mode: (flags & flags::STREAMING_MODE) != 0,
            quality_mode: get_quality_mode(flags),
            id_mode: get_id_mode(flags),
            pe_layout: get_pe_layout(flags),
            read_length_class: get_read_length_class(flags),
        }
    }

    pub fn has_reorder_map(&self) -> bool {
        self.footer.has_reorder_map()
    }

    /// Load the reorder map if present.
    pub fn load_reorder_map(&mut self) -> Result<()> {
        if !self.has_reorder_map() {
            return Err(FqcError::Format("No reorder map in this archive".to_string()));
        }

        self.reader.seek(SeekFrom::Start(self.footer.reorder_map_offset))?;
        let rmh = ReorderMapHeader::read(&mut self.reader)?;

        let mut forward_compressed = vec![0u8; rmh.forward_map_size as usize];
        self.reader.read_exact(&mut forward_compressed)?;

        let mut reverse_compressed = vec![0u8; rmh.reverse_map_size as usize];
        self.reader.read_exact(&mut reverse_compressed)?;

        // Decompress and decode
        let forward_raw = zstd::stream::decode_all(forward_compressed.as_slice())
            .map_err(|e| FqcError::Decompression(format!("Reorder map decompress: {e}")))?;
        let reverse_raw = zstd::stream::decode_all(reverse_compressed.as_slice())
            .map_err(|e| FqcError::Decompression(format!("Reorder map decompress: {e}")))?;

        self.reorder_forward = Some(delta_decode_ids(&forward_raw, rmh.total_reads)?);
        self.reorder_reverse = Some(delta_decode_ids(&reverse_raw, rmh.total_reads)?);

        Ok(())
    }

    /// Read a block by its block_id. Loads all streams.
    pub fn read_block(&mut self, block_id: u32) -> Result<BlockData> {
        let entry = self
            .block_index
            .entries
            .get(block_id as usize)
            .ok_or_else(|| FqcError::Format(format!("Block {block_id} not in index")))?
            .clone();

        // Seek to block start
        self.reader.seek(SeekFrom::Start(entry.offset))?;

        // Read block header
        let bh = BlockHeader::read(&mut self.reader)?;

        // Payload starts right after the block header (use actual header_size for forward compat)
        let payload_start = entry.offset + bh.header_size as u64;

        let mut block_data = BlockData {
            header: bh.clone(),
            ..Default::default()
        };

        // Read IDs stream
        if bh.size_ids > 0 {
            self.reader.seek(SeekFrom::Start(payload_start + bh.offset_ids))?;
            block_data.ids_data = vec![0u8; bh.size_ids as usize];
            self.reader.read_exact(&mut block_data.ids_data)?;
        }

        // Read sequence stream
        if bh.size_seq > 0 {
            self.reader.seek(SeekFrom::Start(payload_start + bh.offset_seq))?;
            block_data.seq_data = vec![0u8; bh.size_seq as usize];
            self.reader.read_exact(&mut block_data.seq_data)?;
        }

        // Read quality stream
        if bh.size_qual > 0 {
            self.reader.seek(SeekFrom::Start(payload_start + bh.offset_qual))?;
            block_data.qual_data = vec![0u8; bh.size_qual as usize];
            self.reader.read_exact(&mut block_data.qual_data)?;
        }

        // Read aux stream
        if bh.size_aux > 0 {
            self.reader.seek(SeekFrom::Start(payload_start + bh.offset_aux))?;
            block_data.aux_data = vec![0u8; bh.size_aux as usize];
            self.reader.read_exact(&mut block_data.aux_data)?;
        }

        Ok(block_data)
    }

    /// Read only the block header for a given block_id (no stream data).
    pub fn read_block_header(&mut self, block_id: u32) -> Result<BlockHeader> {
        let entry = self
            .block_index
            .entries
            .get(block_id as usize)
            .ok_or_else(|| FqcError::Format(format!("Block {block_id} not in index")))?;
        self.reader.seek(SeekFrom::Start(entry.offset))?;
        BlockHeader::read(&mut self.reader)
    }

    /// Look up original read ID from archive ID using the reorder map.
    pub fn lookup_original_id(&self, archive_id: u64) -> Option<u64> {
        self.reorder_reverse
            .as_ref()
            .and_then(|m| m.get(archive_id as usize).copied())
    }
}

// =============================================================================
// ArchiveReader Implementation
// =============================================================================

impl ArchiveReader for FqcReader {
    fn global_header(&self) -> &GlobalHeader {
        &self.global_header
    }

    fn block_count(&self) -> usize {
        self.block_index.entries.len()
    }

    fn total_read_count(&self) -> u64 {
        self.global_header.total_read_count
    }

    fn has_reorder_map(&self) -> bool {
        self.footer.has_reorder_map()
    }

    fn load_reorder_map(&mut self) -> Result<()> {
        self.load_reorder_map()
    }

    fn lookup_original_id(&self, archive_id: u64) -> Option<u64> {
        self.lookup_original_id(archive_id)
    }

    fn read_block(&mut self, block_id: u32) -> Result<BlockData> {
        self.read_block(block_id)
    }

    fn read_block_header(&mut self, block_id: u32) -> Result<BlockHeader> {
        self.read_block_header(block_id)
    }
}
