// =============================================================================
// fqc-rust - Format Contract / Characterization Tests
// =============================================================================
// These tests freeze the fqc-indexed/v2 wire contract: magic, version,
// codec/checksum identifier encoding, version compatibility, frozen decoder
// fixture, and unknown-identifier rejection.
//
// The frozen fixture's SHA-256 hashes are documented in
// tests/fixtures/indexed-v2/MANIFEST.md for human verification. These tests
// verify structural properties (magic, version, file size, round-trip).
// =============================================================================

use fqc::archive::format::*;
use fqc::archive::reader::FqcReader;
use fqc::types::*;
use std::io::Cursor;

// =============================================================================
// Magic and version contract
// =============================================================================

#[test]
fn test_indexed_magic_exact_bytes() {
    assert_eq!(MAGIC_BYTES, [0x89, b'F', b'Q', b'C', 0x0D, 0x0A, 0x1A, 0x0A]);
}

#[test]
fn test_current_version_byte_exact() {
    assert_eq!(CURRENT_VERSION, 0x20);
    assert_eq!(FORMAT_VERSION_MAJOR, 2);
    assert_eq!(FORMAT_VERSION_MINOR, 0);
}

#[test]
fn test_version_compatible_only_major_2() {
    assert!(is_version_compatible(0x20));
    assert!(is_version_compatible(0x2F));
    assert!(!is_version_compatible(0x10));
    assert!(!is_version_compatible(0x30));
    assert!(!is_version_compatible(0x00));
}

// =============================================================================
// Codec identifier encoding: (family << 4) | version
// =============================================================================

#[test]
fn test_codec_encoding_is_high_nibble_family_low_nibble_version() {
    let families = [
        CodecFamily::Raw,
        CodecFamily::AbcV1,
        CodecFamily::ScmV1,
        CodecFamily::DeltaLzma,
        CodecFamily::DeltaZstd,
        CodecFamily::DeltaVarint,
        CodecFamily::OverlapV1,
        CodecFamily::ZstdPlain,
        CodecFamily::ScmOrder1,
        CodecFamily::External,
        CodecFamily::Reserved,
    ];

    for family in families {
        let byte = encode_codec(family, 0);
        assert_eq!(
            byte,
            (family as u8) << 4,
            "codec byte for {family:?} at version 0 should be family << 4"
        );
        assert_eq!(
            decode_codec_family(byte),
            family,
            "decode_codec_family should recover {family:?}"
        );
    }
}

#[test]
fn test_codec_encoding_preserves_version_in_low_nibble() {
    let byte = encode_codec(CodecFamily::AbcV1, 3);
    assert_eq!(byte, 0x13);
    assert_eq!(decode_codec_family(byte), CodecFamily::AbcV1);
    assert_eq!(byte & 0x0F, 3);
}

#[test]
fn test_specific_codec_bytes_match_implementation() {
    assert_eq!(encode_codec(CodecFamily::Raw, 0), 0x00);
    assert_eq!(encode_codec(CodecFamily::AbcV1, 0), 0x10);
    assert_eq!(encode_codec(CodecFamily::ScmV1, 0), 0x20);
    assert_eq!(encode_codec(CodecFamily::DeltaZstd, 0), 0x40);
    assert_eq!(encode_codec(CodecFamily::DeltaVarint, 0), 0x50);
    assert_eq!(encode_codec(CodecFamily::ZstdPlain, 0), 0x70);
    assert_eq!(encode_codec(CodecFamily::ScmOrder1, 0), 0x80);
}

// =============================================================================
// Checksum identifier contract: ID 0 = XxHash64 (not "none")
// =============================================================================

#[test]
fn test_checksum_id_zero_means_xxhash64() {
    assert_eq!(ChecksumType::XxHash64 as u8, 0);

    let gh = GlobalHeader::new(0, 0, "test", 0);
    assert_eq!(gh.checksum_type, 0);

    let mut buf = Vec::new();
    gh.write(&mut buf).unwrap();
    let gh2 = GlobalHeader::read(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(gh2.checksum_type, 0);
}

// =============================================================================
// Block header stores encoded codec bytes
// =============================================================================

#[test]
fn test_block_header_stores_encoded_codec_bytes() {
    let bh = BlockHeader {
        codec_ids: encode_codec(CodecFamily::DeltaZstd, 0),
        codec_seq: encode_codec(CodecFamily::AbcV1, 0),
        codec_qual: encode_codec(CodecFamily::ScmV1, 0),
        codec_aux: encode_codec(CodecFamily::DeltaVarint, 0),
        ..Default::default()
    };

    let mut buf = Vec::new();
    bh.write(&mut buf).unwrap();
    let bh2 = BlockHeader::read(&mut Cursor::new(&buf)).unwrap();

    assert_eq!(bh2.codec_ids, 0x40);
    assert_eq!(bh2.codec_seq, 0x10);
    assert_eq!(bh2.codec_qual, 0x20);
    assert_eq!(bh2.codec_aux, 0x50);
}

// =============================================================================
// Frozen decoder fixture
// =============================================================================

fn fixture_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("indexed-v2")
}

#[test]
fn test_frozen_archive_magic_and_version() {
    let data = std::fs::read(fixture_dir().join("frozen.fqc")).expect("frozen.fqc must exist");

    assert!(data.len() >= MAGIC_HEADER_SIZE);
    assert_eq!(&data[..8], &MAGIC_BYTES);
    assert_eq!(data[8], CURRENT_VERSION);
}

#[test]
fn test_frozen_archive_size_matches_manifest() {
    let metadata = std::fs::metadata(fixture_dir().join("frozen.fqc")).expect("frozen.fqc must exist");
    assert_eq!(
        metadata.len(),
        448,
        "frozen archive must be 448 bytes as recorded in MANIFEST.md"
    );
}

#[test]
fn test_frozen_archive_reader_opens_and_info_matches() {
    let archive_path = fixture_dir().join("frozen.fqc");
    let reader = FqcReader::open(archive_path.to_str().unwrap()).expect("frozen archive must open");
    let info = reader.info();

    assert_eq!(info.total_reads, 3);
    assert_eq!(info.num_blocks, 1);
    assert!(info.has_reorder_map);
    assert_eq!(info.quality_mode, QualityMode::Lossless);
    assert_eq!(info.id_mode, IdMode::Exact);
    assert_eq!(info.read_length_class, ReadLengthClass::Short);
}

#[test]
fn test_frozen_archive_block_uses_abc_codec() {
    let archive_path = fixture_dir().join("frozen.fqc");
    let mut reader = FqcReader::open(archive_path.to_str().unwrap()).expect("frozen archive must open");

    // Checksum type in global header must be 0 (XxHash64).
    assert_eq!(
        reader.global_header.checksum_type, 0,
        "checksum type 0 means XxHash64, not 'none'"
    );

    let block = reader.read_block(0).expect("block 0 must read");

    // Short reads should use ABC for sequences.
    assert_eq!(decode_codec_family(block.header.codec_seq), CodecFamily::AbcV1);
}

// =============================================================================
// Unknown identifier / bad input rejection
// =============================================================================

#[test]
fn test_reader_rejects_unsupported_major_version() {
    let mut data = std::fs::read(fixture_dir().join("frozen.fqc")).unwrap();
    data[8] = 0x30; // major=3

    let temp = std::env::temp_dir().join("fqc_test_bad_version.fqc");
    std::fs::write(&temp, &data).unwrap();

    let result = FqcReader::open(temp.to_str().unwrap());
    match result {
        Err(fqc::error::FqcError::UnsupportedVersion { major: 3 }) => {}
        Err(e) => panic!("expected UnsupportedVersion {{ major: 3 }}, got {e}"),
        Ok(_) => panic!("reader should reject unsupported major version"),
    }

    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_reader_rejects_bad_magic() {
    let mut data = std::fs::read(fixture_dir().join("frozen.fqc")).unwrap();
    data[0] = 0xFF;

    let temp = std::env::temp_dir().join("fqc_test_bad_magic.fqc");
    std::fs::write(&temp, &data).unwrap();

    let result = FqcReader::open(temp.to_str().unwrap());
    assert!(result.is_err());

    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_block_header_rejects_nonzero_reserved() {
    let bh = BlockHeader {
        reserved1: 1,
        ..Default::default()
    };
    let mut buf = Vec::new();
    bh.write(&mut buf).unwrap();

    let result = BlockHeader::read(&mut Cursor::new(&buf));
    assert!(result.is_err());
}

#[test]
fn test_global_header_rejects_nonzero_reserved() {
    let mut gh = GlobalHeader::new(0, 0, "test", 0);
    gh.reserved = 1;

    let mut buf = Vec::new();
    gh.write(&mut buf).unwrap();

    let result = GlobalHeader::read(&mut Cursor::new(&buf));
    assert!(result.is_err());
}
