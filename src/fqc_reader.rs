// =============================================================================
// fqc-rust - FQC Archive Reader
// =============================================================================

use crate::algo::block_compressor::delta_decode_ids;
use crate::error::{FqcError, Result};
use crate::format::*;
use byteorder::ReadBytesExt;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

// =============================================================================
// BlockData - raw decompressed streams for a block
// =============================================================================

#[derive(Debug, Default)]
pub struct BlockData {
    pub header: BlockHeader,
    pub ids_data: Vec<u8>,
    pub seq_data: Vec<u8>,
    pub qual_data: Vec<u8>,
    pub aux_data: Vec<u8>,
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

        // Read block index
        reader.seek(SeekFrom::Start(footer.index_offset))?;
        let block_index = BlockIndex::read(&mut reader)?;

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

    pub fn block_count(&self) -> usize {
        self.block_index.entries.len()
    }

    pub fn total_read_count(&self) -> u64 {
        self.global_header.total_read_count
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
