// =============================================================================
// fqc-rust - FQC Archive Reader
// =============================================================================

use crate::algo::block_compressor::delta_decode_ids;
use crate::archive::format::*;
use crate::archive::traits::BlockData;
use crate::error::{FqcError, Result};
use crate::memory_budget::{zstd_decompress_bounded, DecodeBudget};
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
    budget: DecodeBudget,
}

impl FqcReader {
    pub fn open(path: &str) -> Result<Self> {
        Self::open_with_budget(path, DecodeBudget::automatic())
    }

    pub fn open_with_budget(path: &str, budget: DecodeBudget) -> Result<Self> {
        let file = File::open(path).map_err(FqcError::Io)?;
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
        budget.check_total_reads(global_header.total_read_count, "global_header.total_read_count")?;

        // Read block index with budget + file-region caps
        let index_region = footer_pos.saturating_sub(footer.index_offset);
        reader.seek(SeekFrom::Start(footer.index_offset))?;
        let block_index = BlockIndex::read_with_budget(&mut reader, &budget, index_region)?;
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
            budget,
        })
    }

    pub fn budget(&self) -> &DecodeBudget {
        &self.budget
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
                .ok_or_else(|| FqcError::Format(format!("Block {idx} overflows archive offsets")))?;
            let payload_start = entry
                .offset
                .checked_add(header.header_size as u64)
                .ok_or_else(|| FqcError::Format(format!("Block {idx} header overflows archive offsets")))?;
            if payload_start > block_end {
                return Err(FqcError::Format(format!(
                    "Block {idx} stream extent exceeds block payload bounds"
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
            .ok_or_else(|| FqcError::Format(format!("Block {idx} stream extent overflows block payload")))?;

            if declared_stream_end > header.compressed_size {
                return Err(FqcError::Format(format!(
                    "Block {idx} stream extent exceeds declared block payload"
                )));
            }

            let declared_block_end = payload_start
                .checked_add(header.compressed_size)
                .ok_or_else(|| FqcError::Format(format!("Block {idx} payload overflows archive offsets")))?;
            if declared_block_end > block_end || declared_block_end > block_region_end {
                return Err(FqcError::Format(format!(
                    "Block {idx} stream extent exceeds block payload bounds"
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

    /// Largest compressed block size in the index (for peak estimates).
    pub fn max_block_compressed_size(&self) -> u64 {
        self.block_index
            .entries
            .iter()
            .map(|e| e.compressed_size)
            .max()
            .unwrap_or(0)
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

        if rmh.total_reads != self.global_header.total_read_count {
            return Err(FqcError::Format(format!(
                "reorder map total_reads {} != global header {}",
                rmh.total_reads, self.global_header.total_read_count
            )));
        }
        self.budget
            .check_total_reads(rmh.total_reads, "reorder_map.total_reads")?;
        self.budget
            .check_alloc(rmh.forward_map_size, "reorder_map.forward_compressed")?;
        self.budget
            .check_alloc(rmh.reverse_map_size, "reorder_map.reverse_compressed")?;

        let fwd_len = self
            .budget
            .checked_usize(rmh.forward_map_size, "reorder_map.forward_compressed")?;
        let rev_len = self
            .budget
            .checked_usize(rmh.reverse_map_size, "reorder_map.reverse_compressed")?;

        let mut forward_compressed = vec![0u8; fwd_len];
        self.reader.read_exact(&mut forward_compressed)?;

        let mut reverse_compressed = vec![0u8; rev_len];
        self.reader.read_exact(&mut reverse_compressed)?;

        // Bound zstd output by total_reads * 16 (generous varint/delta room).
        let max_raw = self
            .budget
            .checked_usize(rmh.total_reads.saturating_mul(16).max(64), "reorder_map.decompressed")?
            .min(self.budget.max_alloc_bytes as usize);

        let forward_raw = zstd_decompress_bounded(&forward_compressed, max_raw, "reorder_map.forward")?;
        let reverse_raw = zstd_decompress_bounded(&reverse_compressed, max_raw, "reorder_map.reverse")?;

        self.reorder_forward = Some(delta_decode_ids(&forward_raw, rmh.total_reads)?);
        self.reorder_reverse = Some(delta_decode_ids(&reverse_raw, rmh.total_reads)?);

        Ok(())
    }

    fn alloc_stream(&self, size: u64, location: &str, payload_extent: u64) -> Result<Vec<u8>> {
        if size > payload_extent {
            return Err(FqcError::Format(format!(
                "{location}: stream size {size} exceeds block payload extent {payload_extent}"
            )));
        }
        self.budget.check_alloc(size, location)?;
        let n = self.budget.checked_usize(size, location)?;
        Ok(vec![0u8; n])
    }

    /// Read a block by its block_id. Loads all streams.
    pub fn read_block(&mut self, block_id: u32) -> Result<BlockData> {
        let entry = self
            .block_index
            .entries
            .get(block_id as usize)
            .ok_or_else(|| FqcError::Format(format!("Block {block_id} not in index")))?
            .clone();

        // Peak for this block (compressed streams + rough decode room) must fit budget.
        let peak = entry.compressed_size.saturating_mul(3);
        if peak > self.budget.limit_bytes {
            return Err(FqcError::ResourceLimit {
                location: format!("block {block_id} decode peak"),
                declared: peak,
                allowed: self.budget.limit_bytes,
            });
        }

        self.reader.seek(SeekFrom::Start(entry.offset))?;
        let bh = BlockHeader::read(&mut self.reader)?;
        let payload_start = entry.offset + bh.header_size as u64;
        let payload_extent = bh.compressed_size;

        let mut block_data = BlockData {
            header: bh.clone(),
            ..Default::default()
        };

        if bh.size_ids > 0 {
            self.reader.seek(SeekFrom::Start(payload_start + bh.offset_ids))?;
            block_data.ids_data = self.alloc_stream(bh.size_ids, &format!("block {block_id} ids"), payload_extent)?;
            self.reader.read_exact(&mut block_data.ids_data)?;
        }

        if bh.size_seq > 0 {
            self.reader.seek(SeekFrom::Start(payload_start + bh.offset_seq))?;
            block_data.seq_data = self.alloc_stream(bh.size_seq, &format!("block {block_id} seq"), payload_extent)?;
            self.reader.read_exact(&mut block_data.seq_data)?;
        }

        if bh.size_qual > 0 {
            self.reader.seek(SeekFrom::Start(payload_start + bh.offset_qual))?;
            block_data.qual_data =
                self.alloc_stream(bh.size_qual, &format!("block {block_id} qual"), payload_extent)?;
            self.reader.read_exact(&mut block_data.qual_data)?;
        }

        if bh.size_aux > 0 {
            self.reader.seek(SeekFrom::Start(payload_start + bh.offset_aux))?;
            block_data.aux_data = self.alloc_stream(bh.size_aux, &format!("block {block_id} aux"), payload_extent)?;
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
