// =============================================================================
// fqc-rust - FQC Archive Writer
// =============================================================================

use crate::error::{FqcError, Result};
use crate::format::*;
use crate::algo::block_compressor::{CompressedBlockData, delta_encode_ids};
use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use xxhash_rust::xxh64::Xxh64;

// =============================================================================
// FqcWriter
// =============================================================================

pub struct FqcWriter {
    writer: BufWriter<File>,
    current_offset: u64,
    index_entries: Vec<IndexEntry>,
    reorder_map_offset: u64,
    global_hasher: Xxh64,
    block_count: u64,
}

impl FqcWriter {
    pub fn create(path: &str) -> Result<Self> {
        let file = File::create(path)
            .map_err(|e| FqcError::Io(e))?;
        let mut writer = BufWriter::new(file);

        // Write magic header (8 bytes) + version byte
        writer.write_all(&MAGIC_BYTES)?;
        writer.write_u8(CURRENT_VERSION)?;

        let current_offset = MAGIC_HEADER_SIZE as u64;

        Ok(Self {
            writer,
            current_offset,
            index_entries: Vec::new(),
            reorder_map_offset: 0,
            global_hasher: Xxh64::new(0),
            block_count: 0,
        })
    }

    pub fn write_global_header(&mut self, header: &GlobalHeader) -> Result<()> {
        let written = header.write(&mut self.writer)?;
        self.global_hasher.update(&header.flags.to_le_bytes());
        self.current_offset += written as u64;
        Ok(())
    }

    pub fn patch_total_read_count(&mut self, total_read_count: u64) -> Result<()> {
        const TOTAL_READ_COUNT_OFFSET: u64 = 4 + 8 + 1 + 1 + 2;

        self.writer.flush()?;
        self.writer.seek(SeekFrom::Start(MAGIC_HEADER_SIZE as u64 + TOTAL_READ_COUNT_OFFSET))?;
        self.writer.write_u64::<LittleEndian>(total_read_count)?;
        self.writer.seek(SeekFrom::Start(self.current_offset))?;
        Ok(())
    }

    /// Write a compressed block. Returns the block's starting offset.
    pub fn write_block(&mut self, compressed: &CompressedBlockData) -> Result<u64> {
        let archive_id_start = self.index_entries
            .last()
            .map(|entry| entry.archive_id_end())
            .unwrap_or(0);
        self.write_block_with_id(compressed, archive_id_start)
    }

    /// Write a block with explicit archive_id_start
    pub fn write_block_with_id(&mut self, compressed: &CompressedBlockData, archive_id_start: u64) -> Result<u64> {
        let block_start = self.current_offset;

        let mut bh = BlockHeader::default();
        bh.block_id = compressed.block_id;
        bh.uncompressed_count = compressed.read_count;
        bh.uniform_read_length = compressed.uniform_read_length;
        bh.block_xxhash64 = compressed.block_checksum;
        bh.codec_ids = compressed.codec_ids;
        bh.codec_seq = compressed.codec_seq;
        bh.codec_qual = compressed.codec_qual;
        bh.codec_aux = compressed.codec_aux;
        bh.checksum_type = 0;

        bh.offset_ids = 0;
        bh.size_ids = compressed.id_stream.len() as u64;
        bh.offset_seq = bh.size_ids;
        bh.size_seq = compressed.seq_stream.len() as u64;
        bh.offset_qual = bh.offset_seq + bh.size_seq;
        bh.size_qual = compressed.qual_stream.len() as u64;
        bh.offset_aux = bh.offset_qual + bh.size_qual;
        bh.size_aux = compressed.aux_stream.len() as u64;

        let total_payload = compressed.id_stream.len()
            + compressed.seq_stream.len()
            + compressed.qual_stream.len()
            + compressed.aux_stream.len();
        bh.compressed_size = total_payload as u64;

        bh.write(&mut self.writer)?;

        self.writer.write_all(&compressed.id_stream)?;
        self.writer.write_all(&compressed.seq_stream)?;
        self.writer.write_all(&compressed.qual_stream)?;
        self.writer.write_all(&compressed.aux_stream)?;

        let total_block_bytes = BLOCK_HEADER_SIZE as u64 + total_payload as u64;

        self.global_hasher.update(&compressed.id_stream);
        self.global_hasher.update(&compressed.seq_stream);
        self.global_hasher.update(&compressed.qual_stream);
        self.global_hasher.update(&compressed.aux_stream);

        self.index_entries.push(IndexEntry {
            offset: block_start,
            compressed_size: total_block_bytes,
            archive_id_start,
            read_count: compressed.read_count,
        });

        self.current_offset += total_block_bytes;
        self.block_count += 1;

        Ok(block_start)
    }

    /// Write reorder map. Returns the offset where it was written.
    pub fn write_reorder_map(&mut self, forward_map: &[u64], reverse_map: &[u64]) -> Result<u64> {
        let map_offset = self.current_offset;
        self.reorder_map_offset = map_offset;

        // Delta encode the maps
        let forward_encoded = delta_encode_ids(forward_map);
        let reverse_encoded = delta_encode_ids(reverse_map);

        // Compress with zstd
        let forward_compressed = zstd::bulk::compress(&forward_encoded, 3)
            .map_err(|e| FqcError::Compression(format!("Reorder map compress failed: {e}")))?;
        let reverse_compressed = zstd::bulk::compress(&reverse_encoded, 3)
            .map_err(|e| FqcError::Compression(format!("Reorder map compress failed: {e}")))?;

        // Write reorder map header
        let rmh = ReorderMapHeader {
            version: 1,
            total_reads: forward_map.len() as u64,
            forward_map_size: forward_compressed.len() as u64,
            reverse_map_size: reverse_compressed.len() as u64,
        };
        rmh.write(&mut self.writer)?;

        // Write compressed maps
        self.writer.write_all(&forward_compressed)?;
        self.writer.write_all(&reverse_compressed)?;

        let total = REORDER_MAP_HEADER_SIZE + forward_compressed.len() + reverse_compressed.len();
        self.current_offset += total as u64;

        Ok(map_offset)
    }

    /// Finalize the archive: write block index and footer.
    pub fn finalize(mut self) -> Result<()> {
        let index_offset = self.current_offset;

        // Write block index
        let block_index = BlockIndex {
            num_blocks: self.index_entries.len() as u64,
            entries: self.index_entries.clone(),
        };
        let index_size = block_index.write(&mut self.writer)?;
        self.current_offset += index_size as u64;

        // Compute global checksum
        let global_checksum = self.global_hasher.digest();

        // Write footer
        let footer = FileFooter::new(index_offset, self.reorder_map_offset, global_checksum);
        footer.write(&mut self.writer)?;

        self.writer.flush()?;
        Ok(())
    }
}

