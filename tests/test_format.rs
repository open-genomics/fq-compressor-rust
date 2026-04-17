// =============================================================================
// fqc-rust - Format Unit Tests
// =============================================================================

use fqc::format::*;
use fqc::types::*;
use std::io::Cursor;

#[test]
fn test_magic_validation() {
    assert!(validate_magic(&MAGIC_BYTES));
    assert!(!validate_magic(&[0u8; 8]));
    assert!(!validate_magic(&[0x89, b'X', b'Q', b'C', 0x0D, 0x0A, 0x1A, 0x0A]));
}

#[test]
fn test_version_compatibility() {
    assert!(is_version_compatible(CURRENT_VERSION));
    // Same major, different minor
    assert!(is_version_compatible((FORMAT_VERSION_MAJOR << 4) | 0x0F));
    // Different major
    assert!(!is_version_compatible(0x30)); // major=3
    assert!(!is_version_compatible(0x10)); // major=1 (old version)
    assert!(!is_version_compatible(0x00)); // major=0
}

#[test]
fn test_build_and_extract_flags() {
    let f = build_flags(
        true,                    // is_paired
        false,                   // preserve_order
        QualityMode::Illumina8,  // quality_mode
        IdMode::Tokenize,        // id_mode
        true,                    // has_reorder_map
        PeLayout::Consecutive,   // pe_layout
        ReadLengthClass::Medium, // read_length_class
        false,                   // streaming_mode
    );

    assert_ne!(f & flags::IS_PAIRED, 0);
    assert_eq!(f & flags::PRESERVE_ORDER, 0);
    assert_eq!(get_quality_mode(f), QualityMode::Illumina8);
    assert_eq!(get_id_mode(f), IdMode::Tokenize);
    assert_ne!(f & flags::HAS_REORDER_MAP, 0);
    assert_eq!(get_pe_layout(f), PeLayout::Consecutive);
    assert_eq!(get_read_length_class(f), ReadLengthClass::Medium);
    assert_eq!(f & flags::STREAMING_MODE, 0);
}

#[test]
fn test_build_flags_streaming() {
    let f = build_flags(
        false,
        true,
        QualityMode::Lossless,
        IdMode::Exact,
        false,
        PeLayout::Interleaved,
        ReadLengthClass::Short,
        true,
    );
    assert_ne!(f & flags::STREAMING_MODE, 0);
    assert_ne!(f & flags::PRESERVE_ORDER, 0);
    assert_eq!(get_quality_mode(f), QualityMode::Lossless);
}

#[test]
fn test_global_header_roundtrip() {
    let gh = GlobalHeader::new(
        build_flags(
            false,
            false,
            QualityMode::Lossless,
            IdMode::Exact,
            true,
            PeLayout::Interleaved,
            ReadLengthClass::Short,
            false,
        ),
        1000,
        "test.fastq",
        1_700_000_000,
    );

    let mut buf = Vec::new();
    gh.write(&mut buf).unwrap();

    let mut cursor = Cursor::new(&buf);
    let gh2 = GlobalHeader::read(&mut cursor).unwrap();

    assert_eq!(gh2.flags, gh.flags);
    assert_eq!(gh2.total_read_count, 1000);
    assert_eq!(gh2.original_filename, "test.fastq");
    assert_eq!(gh2.timestamp, 1_700_000_000);
    assert_eq!(gh2.checksum_type, ChecksumType::XxHash64 as u8);
    assert_eq!(gh2.reserved, 0);
}

#[test]
fn test_global_header_empty_filename() {
    let gh = GlobalHeader::new(0, 0, "", 0);
    let mut buf = Vec::new();
    gh.write(&mut buf).unwrap();

    let mut cursor = Cursor::new(&buf);
    let gh2 = GlobalHeader::read(&mut cursor).unwrap();
    assert_eq!(gh2.original_filename, "");
    assert_eq!(gh2.total_read_count, 0);
}

#[test]
fn test_block_header_roundtrip() {
    let bh = BlockHeader {
        block_id: 42,
        uncompressed_count: 1000,
        uniform_read_length: 150,
        block_xxhash64: 0xDEAD_BEEF,
        codec_ids: encode_codec(CodecFamily::DeltaZstd, 0),
        codec_seq: encode_codec(CodecFamily::AbcV1, 0),
        codec_qual: encode_codec(CodecFamily::ScmV1, 0),
        codec_aux: encode_codec(CodecFamily::DeltaVarint, 0),
        compressed_size: 5000,
        offset_ids: 0,
        size_ids: 1000,
        offset_seq: 1000,
        size_seq: 2000,
        offset_qual: 3000,
        size_qual: 1500,
        offset_aux: 4500,
        size_aux: 500,
        ..Default::default()
    };

    let mut buf = Vec::new();
    bh.write(&mut buf).unwrap();
    assert_eq!(buf.len(), BLOCK_HEADER_SIZE);

    let mut cursor = Cursor::new(&buf);
    let bh2 = BlockHeader::read(&mut cursor).unwrap();

    assert_eq!(bh2.block_id, 42);
    assert_eq!(bh2.uncompressed_count, 1000);
    assert_eq!(bh2.uniform_read_length, 150);
    assert_eq!(bh2.block_xxhash64, 0xDEAD_BEEF);
    assert_eq!(bh2.codec_ids, bh.codec_ids);
    assert_eq!(bh2.codec_seq, bh.codec_seq);
    assert_eq!(bh2.codec_qual, bh.codec_qual);
    assert_eq!(bh2.codec_aux, bh.codec_aux);
    assert_eq!(bh2.compressed_size, 5000);
    assert_eq!(bh2.size_ids, 1000);
    assert_eq!(bh2.size_seq, 2000);
    assert_eq!(bh2.size_qual, 1500);
    assert_eq!(bh2.size_aux, 500);
}

#[test]
fn test_block_header_uniform_length() {
    let bh = BlockHeader {
        uniform_read_length: 150,
        size_aux: 0,
        ..Default::default()
    };
    assert!(bh.has_uniform_length());

    let bh2 = BlockHeader {
        uniform_read_length: 0,
        ..Default::default()
    };
    assert!(!bh2.has_uniform_length());

    let bh3 = BlockHeader {
        uniform_read_length: 150,
        size_aux: 100,
        ..Default::default()
    };
    assert!(!bh3.has_uniform_length());
}

#[test]
fn test_block_header_quality_discarded() {
    let bh = BlockHeader {
        size_qual: 0,
        codec_qual: encode_codec(CodecFamily::Raw, 0),
        ..Default::default()
    };
    assert!(bh.is_quality_discarded());

    let bh2 = BlockHeader {
        size_qual: 0,
        codec_qual: encode_codec(CodecFamily::ScmV1, 0),
        ..Default::default()
    };
    assert!(!bh2.is_quality_discarded());
}

#[test]
fn test_index_entry_roundtrip() {
    let entry = IndexEntry {
        offset: 1024,
        compressed_size: 50000,
        archive_id_start: 100,
        read_count: 1000,
    };

    assert_eq!(entry.archive_id_end(), 1100);
    assert!(entry.contains_read(100));
    assert!(entry.contains_read(1099));
    assert!(!entry.contains_read(1100));
    assert!(!entry.contains_read(99));

    let mut buf = Vec::new();
    entry.write(&mut buf).unwrap();
    assert_eq!(buf.len(), INDEX_ENTRY_SIZE);

    let mut cursor = Cursor::new(&buf);
    let entry2 = IndexEntry::read(&mut cursor).unwrap();
    assert_eq!(entry2.offset, 1024);
    assert_eq!(entry2.compressed_size, 50000);
    assert_eq!(entry2.archive_id_start, 100);
    assert_eq!(entry2.read_count, 1000);
}

#[test]
fn test_block_index_roundtrip() {
    let index = BlockIndex {
        num_blocks: 3,
        entries: vec![
            IndexEntry {
                offset: 100,
                compressed_size: 500,
                archive_id_start: 0,
                read_count: 100,
            },
            IndexEntry {
                offset: 600,
                compressed_size: 400,
                archive_id_start: 100,
                read_count: 100,
            },
            IndexEntry {
                offset: 1000,
                compressed_size: 300,
                archive_id_start: 200,
                read_count: 50,
            },
        ],
    };

    let mut buf = Vec::new();
    let written = index.write(&mut buf).unwrap();
    assert_eq!(written, BLOCK_INDEX_HEADER_SIZE + 3 * INDEX_ENTRY_SIZE);

    let mut cursor = Cursor::new(&buf);
    let index2 = BlockIndex::read(&mut cursor).unwrap();
    assert_eq!(index2.num_blocks, 3);
    assert_eq!(index2.entries.len(), 3);
    assert_eq!(index2.entries[0].offset, 100);
    assert_eq!(index2.entries[2].read_count, 50);
}

#[test]
fn test_reorder_map_header_roundtrip() {
    let rmh = ReorderMapHeader {
        version: 1,
        total_reads: 100_000,
        forward_map_size: 5000,
        reverse_map_size: 5000,
    };

    let mut buf = Vec::new();
    rmh.write(&mut buf).unwrap();
    assert_eq!(buf.len(), REORDER_MAP_HEADER_SIZE);

    let mut cursor = Cursor::new(&buf);
    let rmh2 = ReorderMapHeader::read(&mut cursor).unwrap();
    assert_eq!(rmh2.version, 1);
    assert_eq!(rmh2.total_reads, 100_000);
    assert_eq!(rmh2.forward_map_size, 5000);
    assert_eq!(rmh2.reverse_map_size, 5000);
}

#[test]
fn test_file_footer_roundtrip() {
    let footer = FileFooter::new(12345, 6789, 0xCAFE_BABE);
    assert!(footer.is_valid());
    assert!(footer.has_reorder_map());

    let mut buf = Vec::new();
    footer.write(&mut buf).unwrap();
    assert_eq!(buf.len(), FILE_FOOTER_SIZE);

    let mut cursor = Cursor::new(&buf);
    let footer2 = FileFooter::read(&mut cursor).unwrap();
    assert_eq!(footer2.index_offset, 12345);
    assert_eq!(footer2.reorder_map_offset, 6789);
    assert_eq!(footer2.global_checksum, 0xCAFE_BABE);
    assert!(footer2.is_valid());
}

#[test]
fn test_file_footer_no_reorder_map() {
    let footer = FileFooter::new(1000, 0, 0);
    assert!(!footer.has_reorder_map());
}

#[test]
fn test_file_footer_invalid_magic() {
    let mut buf = Vec::new();
    let footer = FileFooter::new(0, 0, 0);
    footer.write(&mut buf).unwrap();
    // Corrupt the magic end
    buf[24] = 0xFF;

    let mut cursor = Cursor::new(&buf);
    let result = FileFooter::read(&mut cursor);
    assert!(result.is_err());
}
