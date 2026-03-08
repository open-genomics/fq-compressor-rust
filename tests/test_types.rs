// =============================================================================
// fqc-rust - Types Unit Tests
// =============================================================================

use fqc::types::*;

#[test]
fn test_quality_mode_from_u8() {
    assert_eq!(QualityMode::from_u8(0), QualityMode::Lossless);
    assert_eq!(QualityMode::from_u8(1), QualityMode::Illumina8);
    assert_eq!(QualityMode::from_u8(2), QualityMode::Qvz);
    assert_eq!(QualityMode::from_u8(3), QualityMode::Discard);
    assert_eq!(QualityMode::from_u8(255), QualityMode::Lossless); // fallback
}

#[test]
fn test_quality_mode_as_str() {
    assert_eq!(QualityMode::Lossless.as_str(), "lossless");
    assert_eq!(QualityMode::Illumina8.as_str(), "illumina8");
    assert_eq!(QualityMode::Qvz.as_str(), "qvz");
    assert_eq!(QualityMode::Discard.as_str(), "discard");
}

#[test]
fn test_id_mode_from_u8() {
    assert_eq!(IdMode::from_u8(0), IdMode::Exact);
    assert_eq!(IdMode::from_u8(1), IdMode::Tokenize);
    assert_eq!(IdMode::from_u8(2), IdMode::Discard);
    assert_eq!(IdMode::from_u8(99), IdMode::Exact); // fallback
}

#[test]
fn test_read_length_class() {
    assert_eq!(ReadLengthClass::from_u8(0), ReadLengthClass::Short);
    assert_eq!(ReadLengthClass::from_u8(1), ReadLengthClass::Medium);
    assert_eq!(ReadLengthClass::from_u8(2), ReadLengthClass::Long);
    assert_eq!(ReadLengthClass::from_u8(99), ReadLengthClass::Short);
}

#[test]
fn test_classify_read_length() {
    // Ultra-long
    assert_eq!(classify_read_length(50000, 200_000), ReadLengthClass::Long);
    // Long
    assert_eq!(classify_read_length(5000, 15_000), ReadLengthClass::Long);
    // Medium (max > 511)
    assert_eq!(classify_read_length(300, 600), ReadLengthClass::Medium);
    // Medium (median >= 1KB)
    assert_eq!(classify_read_length(1500, 400), ReadLengthClass::Medium);
    // Short
    assert_eq!(classify_read_length(150, 200), ReadLengthClass::Short);
}

#[test]
fn test_recommended_block_size() {
    assert_eq!(recommended_block_size(ReadLengthClass::Short), DEFAULT_BLOCK_SIZE_SHORT);
    assert_eq!(
        recommended_block_size(ReadLengthClass::Medium),
        DEFAULT_BLOCK_SIZE_MEDIUM
    );
    assert_eq!(recommended_block_size(ReadLengthClass::Long), DEFAULT_BLOCK_SIZE_LONG);
}

#[test]
fn test_pe_layout() {
    assert_eq!(PeLayout::from_u8(0), PeLayout::Interleaved);
    assert_eq!(PeLayout::from_u8(1), PeLayout::Consecutive);
    assert_eq!(PeLayout::from_u8(99), PeLayout::Interleaved);
}

#[test]
fn test_codec_family() {
    assert_eq!(CodecFamily::from_u8(0x0), CodecFamily::Raw);
    assert_eq!(CodecFamily::from_u8(0x1), CodecFamily::AbcV1);
    assert_eq!(CodecFamily::from_u8(0x7), CodecFamily::ZstdPlain);
    assert_eq!(CodecFamily::from_u8(0xE), CodecFamily::External);
    assert_eq!(CodecFamily::from_u8(0xF), CodecFamily::Reserved);
}

#[test]
fn test_encode_decode_codec() {
    let coded = encode_codec(CodecFamily::AbcV1, 3);
    assert_eq!(decode_codec_family(coded), CodecFamily::AbcV1);
    assert_eq!(coded & 0x0F, 3); // version

    let coded2 = encode_codec(CodecFamily::ZstdPlain, 0);
    assert_eq!(decode_codec_family(coded2), CodecFamily::ZstdPlain);
    assert_eq!(coded2 & 0x0F, 0);
}

#[test]
fn test_read_record() {
    let r = ReadRecord::new("id1".to_string(), "ACGT".to_string(), "IIII".to_string());
    assert!(r.is_valid());
    assert_eq!(r.len(), 4);
    assert!(!r.is_empty());

    let empty = ReadRecord::default();
    assert!(!empty.is_valid());
    assert!(empty.is_empty());

    // Mismatched lengths
    let bad = ReadRecord::new("id".to_string(), "ACGT".to_string(), "II".to_string());
    assert!(!bad.is_valid());
}

#[test]
fn test_constants() {
    assert_eq!(INVALID_BLOCK_ID, u32::MAX);
    assert_eq!(INVALID_READ_ID, u64::MAX);
    assert_eq!(DEFAULT_COMPRESSION_LEVEL, 5);
    assert_eq!(MIN_COMPRESSION_LEVEL, 1);
    assert_eq!(MAX_COMPRESSION_LEVEL, 9);
    assert_eq!(SPRING_MAX_READ_LENGTH, 511);
}
