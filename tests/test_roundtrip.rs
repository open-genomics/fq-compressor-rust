// =============================================================================
// fqc-rust - Round-trip Integration Tests (Compress → Decompress → Verify)
// =============================================================================

use fqc::algo::block_compressor::*;
use fqc::format::*;
use fqc::fqc_reader::FqcReader;
use fqc::fqc_writer::FqcWriter;
use fqc::types::*;

fn make_reads(n: usize, length: usize) -> Vec<ReadRecord> {
    let bases = b"ACGT";
    (0..n)
        .map(|i| {
            let seq: String = (0..length).map(|j| bases[(i + j) % 4] as char).collect();
            let qual: String = (0..length).map(|j| (b'!' + ((i + j) % 40) as u8) as char).collect();
            ReadRecord::new(format!("read_{}", i), seq, qual)
        })
        .collect()
}

fn decompress_block(compressor: &BlockCompressor, compressed: &CompressedBlockData) -> DecompressedBlockData {
    compressor
        .decompress_raw(
            0,
            compressed.read_count,
            compressed.uniform_read_length,
            compressed.codec_seq,
            compressed.codec_qual,
            &compressed.id_stream,
            &compressed.seq_stream,
            &compressed.qual_stream,
            &compressed.aux_stream,
        )
        .unwrap()
}

fn assert_reads_match(original: &[ReadRecord], restored: &[ReadRecord]) {
    assert_eq!(original.len(), restored.len(), "Read count mismatch");
    for (i, (orig, dec)) in original.iter().zip(restored.iter()).enumerate() {
        assert_eq!(orig.id, dec.id, "ID mismatch at read {i}");
        assert_eq!(orig.sequence, dec.sequence, "Sequence mismatch at read {i}");
        assert_eq!(orig.quality, dec.quality, "Quality mismatch at read {i}");
    }
}

fn make_variable_length_reads(n: usize) -> Vec<ReadRecord> {
    let bases = b"ACGT";
    (0..n)
        .map(|i| {
            let length = 100 + (i % 50);
            let seq: String = (0..length).map(|j| bases[(i + j) % 4] as char).collect();
            let qual: String = (0..length).map(|j| (b'!' + ((i + j) % 40) as u8) as char).collect();
            ReadRecord::new(format!("read_{}", i), seq, qual)
        })
        .collect()
}

// =============================================================================
// Block Compressor Round-Trip (Short reads, ABC)
// =============================================================================

#[test]
fn test_block_compress_decompress_short_reads() {
    let reads = make_reads(20, 150);
    let config = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        ..Default::default()
    };
    let compressor = BlockCompressor::new(config);

    let compressed = compressor.compress(&reads, 0).unwrap();
    assert_eq!(compressed.read_count, 20);
    assert!(compressed.total_compressed_size() > 0);
    assert_eq!(compressed.uniform_read_length, 150);

    let decompressed = decompress_block(&compressor, &compressed);
    assert_reads_match(&reads, &decompressed.reads);
}

#[test]
fn test_block_compress_decompress_large_short_block_falls_back_to_zstd() {
    let reads = make_reads(5_000, 150);
    let config = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        ..Default::default()
    };
    let compressor = BlockCompressor::new(config);

    let compressed = compressor.compress(&reads, 0).unwrap();
    assert_eq!(decode_codec_family(compressed.codec_seq), CodecFamily::ZstdPlain);

    let decompressed = decompress_block(&compressor, &compressed);
    assert_reads_match(&reads, &decompressed.reads);
}

// =============================================================================
// Block Compressor Round-Trip (Medium reads, Zstd)
// =============================================================================

#[test]
fn test_block_compress_decompress_medium_reads() {
    let reads = make_reads(10, 600);
    let config = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Medium,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        ..Default::default()
    };
    let compressor = BlockCompressor::new(config);

    let compressed = compressor.compress(&reads, 1).unwrap();
    assert_eq!(compressed.read_count, 10);
    assert_eq!(compressed.uniform_read_length, 600);

    let decompressed = decompress_block(&compressor, &compressed);
    assert_reads_match(&reads, &decompressed.reads);
}

// =============================================================================
// Block Compressor Round-Trip (Variable length reads)
// =============================================================================

#[test]
fn test_block_compress_decompress_variable_length() {
    let reads = make_variable_length_reads(15);
    let config = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Medium,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        ..Default::default()
    };
    let compressor = BlockCompressor::new(config);

    let compressed = compressor.compress(&reads, 2).unwrap();
    assert_eq!(compressed.read_count, 15);
    assert_eq!(compressed.uniform_read_length, 0); // variable
    assert!(!compressed.aux_stream.is_empty()); // aux needed for lengths

    let decompressed = decompress_block(&compressor, &compressed);
    for (orig, dec) in reads.iter().zip(decompressed.reads.iter()) {
        assert_eq!(orig.sequence.len(), dec.sequence.len(), "Length mismatch");
        assert_eq!(orig.sequence, dec.sequence);
        assert_eq!(orig.quality, dec.quality);
    }
}

// =============================================================================
// Quality Discard Mode
// =============================================================================

#[test]
fn test_block_compress_quality_discard() {
    let reads = make_reads(10, 150);
    let config = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Discard,
        id_mode: IdMode::Exact,
        ..Default::default()
    };
    let compressor = BlockCompressor::new(config);

    let compressed = compressor.compress(&reads, 0).unwrap();
    assert!(compressed.qual_stream.is_empty());

    let decompressed = decompress_block(&compressor, &compressed);
    for (orig, dec) in reads.iter().zip(decompressed.reads.iter()) {
        assert_eq!(orig.sequence, dec.sequence);
        assert!(dec.quality.chars().all(|c| c == '!'), "Quality should be placeholder");
    }
}

// =============================================================================
// ID Discard Mode
// =============================================================================

#[test]
fn test_block_compress_id_discard() {
    let reads = make_reads(10, 150);
    let config = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Discard,
        ..Default::default()
    };
    let compressor = BlockCompressor::new(config);

    let compressed = compressor.compress(&reads, 0).unwrap();
    let decompressed = decompress_block(&compressor, &compressed);

    // IDs should be sequential placeholders
    for (i, dec) in decompressed.reads.iter().enumerate() {
        assert!(!dec.id.is_empty(), "ID should not be empty");
        assert_eq!(dec.sequence, reads[i].sequence);
    }
}

// =============================================================================
// Block Checksum
// =============================================================================

#[test]
fn test_block_checksum_deterministic() {
    let reads = make_reads(5, 100);
    let c1 = compute_block_checksum(&reads);
    let c2 = compute_block_checksum(&reads);
    assert_eq!(c1, c2);
    assert_ne!(c1, 0);
}

#[test]
fn test_block_checksum_differs_on_change() {
    let reads1 = make_reads(5, 100);
    let mut reads2 = reads1.clone();
    reads2[0].sequence = "A".repeat(100);

    let c1 = compute_block_checksum(&reads1);
    let c2 = compute_block_checksum(&reads2);
    assert_ne!(c1, c2);
}

// =============================================================================
// Delta Encode/Decode IDs (Varint)
// =============================================================================

#[test]
fn test_delta_encode_decode_ids() {
    let ids: Vec<u64> = vec![0, 1, 2, 3, 100, 200, 500, 1000, 10000];
    let encoded = delta_encode_ids(&ids);
    let decoded = delta_decode_ids(&encoded, ids.len() as u64).unwrap();
    assert_eq!(ids, decoded);
}

#[test]
fn test_delta_encode_decode_ids_reverse_order() {
    let ids: Vec<u64> = vec![10000, 5000, 3000, 1000, 500, 100, 0];
    let encoded = delta_encode_ids(&ids);
    let decoded = delta_decode_ids(&encoded, ids.len() as u64).unwrap();
    assert_eq!(ids, decoded);
}

#[test]
fn test_delta_encode_decode_ids_single() {
    let ids: Vec<u64> = vec![42];
    let encoded = delta_encode_ids(&ids);
    let decoded = delta_decode_ids(&encoded, 1).unwrap();
    assert_eq!(ids, decoded);
}

#[test]
fn test_delta_encode_decode_ids_empty() {
    let ids: Vec<u64> = vec![];
    let encoded = delta_encode_ids(&ids);
    let decoded = delta_decode_ids(&encoded, 0).unwrap();
    assert_eq!(ids, decoded);
}

// =============================================================================
// Empty block
// =============================================================================

#[test]
fn test_block_compress_decompress_empty() {
    let reads: Vec<ReadRecord> = vec![];
    let config = BlockCompressorConfig::default();
    let compressor = BlockCompressor::new(config);

    let compressed = compressor.compress(&reads, 0).unwrap();
    assert_eq!(compressed.read_count, 0);

    let decompressed = compressor
        .decompress_raw(
            0,
            0,
            0,
            compressed.codec_seq,
            compressed.codec_qual,
            &compressed.id_stream,
            &compressed.seq_stream,
            &compressed.qual_stream,
            &compressed.aux_stream,
        )
        .unwrap();
    assert!(decompressed.reads.is_empty());
}

// =============================================================================
// Full archive round-trip (Compress command → FQC file → Decompress command)
// =============================================================================

#[test]
fn test_full_archive_roundtrip() {
    let dir = std::env::temp_dir().join("fqc_test_roundtrip");
    let _ = std::fs::create_dir_all(&dir);
    let fqc_path = dir.join("test.fqc");
    let fqc_path_str = fqc_path.to_str().unwrap();

    // Create test reads
    let reads = make_reads(50, 150);

    let config = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        ..Default::default()
    };
    let compressor = BlockCompressor::new(config);

    // Write archive
    {
        let mut writer = FqcWriter::create(fqc_path_str).unwrap();

        let f = build_flags(
            false,
            false,
            QualityMode::Lossless,
            IdMode::Exact,
            false,
            PeLayout::Interleaved,
            ReadLengthClass::Short,
            false,
        );
        let gh = GlobalHeader::new(f, reads.len() as u64, "test.fastq", 0);
        writer.write_global_header(&gh).unwrap();

        // Compress in one block
        let compressed = compressor.compress(&reads, 0).unwrap();
        writer.write_block(&compressed).unwrap();

        writer.finalize().unwrap();
    }

    // Read archive
    {
        let mut reader = FqcReader::open(fqc_path_str).unwrap();
        assert_eq!(reader.total_read_count(), 50);
        assert_eq!(reader.block_count(), 1);

        let block_data = reader.read_block(0).unwrap();
        let bh = &block_data.header;

        let decompressed = compressor
            .decompress_raw(
                bh.block_id,
                bh.uncompressed_count,
                bh.uniform_read_length,
                bh.codec_seq,
                bh.codec_qual,
                &block_data.ids_data,
                &block_data.seq_data,
                &block_data.qual_data,
                &block_data.aux_data,
            )
            .unwrap();

        assert_reads_match(&reads, &decompressed.reads);
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

// =============================================================================
// Archive with reorder map
// =============================================================================

#[test]
fn test_archive_with_reorder_map() {
    let dir = std::env::temp_dir().join("fqc_test_reorder");
    let _ = std::fs::create_dir_all(&dir);
    let fqc_path = dir.join("test_reorder.fqc");
    let fqc_path_str = fqc_path.to_str().unwrap();

    let reads = make_reads(20, 150);

    let config = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        ..Default::default()
    };
    let compressor = BlockCompressor::new(config);

    // Create a simple reorder map (identity)
    let forward_map: Vec<u64> = (0..20).collect();
    let reverse_map: Vec<u64> = (0..20).collect();

    {
        let mut writer = FqcWriter::create(fqc_path_str).unwrap();

        let f = build_flags(
            false,
            false,
            QualityMode::Lossless,
            IdMode::Exact,
            true,
            PeLayout::Interleaved,
            ReadLengthClass::Short,
            false,
        );
        let gh = GlobalHeader::new(f, 20, "test.fastq", 0);
        writer.write_global_header(&gh).unwrap();

        let compressed = compressor.compress(&reads, 0).unwrap();
        writer.write_block(&compressed).unwrap();

        writer.write_reorder_map(&forward_map, &reverse_map).unwrap();
        writer.finalize().unwrap();
    }

    {
        let mut reader = FqcReader::open(fqc_path_str).unwrap();
        assert!(reader.has_reorder_map());

        reader.load_reorder_map().unwrap();

        for i in 0..20u64 {
            let orig = reader.lookup_original_id(i);
            assert_eq!(orig, Some(i));
        }
    }

    let _ = std::fs::remove_dir_all(&dir);
}
