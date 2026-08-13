//! Per-stream codec dispatch tests (FQCR-CODEC-001).

use fqc::algo::block_compressor::{BlockCompressor, BlockCompressorConfig};
use fqc::error::FqcError;
use fqc::types::{encode_codec, CodecFamily, IdMode, QualityMode, ReadLengthClass, ReadRecord};

fn sample_reads() -> Vec<ReadRecord> {
    (0..3)
        .map(|i| ReadRecord {
            id: format!("r{i}"),
            comment: String::new(),
            sequence: "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTAC".to_string(),
            quality: "I".repeat(50),
        })
        .collect()
}

fn compressor() -> BlockCompressor {
    BlockCompressor::new(BlockCompressorConfig {
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        ..Default::default()
    })
}

fn assert_unsupported(err: FqcError, stream: &str) {
    match err {
        FqcError::UnsupportedFormat(msg) => {
            assert!(
                msg.contains(stream) && msg.contains("codec 0x"),
                "expected stream={stream} in error, got: {msg}"
            );
        }
        other => panic!("expected UnsupportedFormat, got {other}"),
    }
}

#[test]
fn round_trip_uses_all_four_header_codecs() {
    let reads = sample_reads();
    let mut c = compressor();
    let compressed = c.compress(&reads, 0).unwrap();
    let decoded = c
        .decompress_raw(
            0,
            compressed.read_count,
            compressed.uniform_read_length,
            compressed.codec_ids,
            compressed.codec_seq,
            compressed.codec_qual,
            compressed.codec_aux,
            &compressed.id_stream,
            &compressed.seq_stream,
            &compressed.qual_stream,
            &compressed.aux_stream,
        )
        .unwrap();
    assert_eq!(decoded.reads.len(), reads.len());
    assert_eq!(decoded.reads[0].sequence, reads[0].sequence);
}

#[test]
fn rejects_inapplicable_family_on_ids_stream() {
    let reads = sample_reads();
    let mut c = compressor();
    let compressed = c.compress(&reads, 0).unwrap();
    let bad = encode_codec(CodecFamily::AbcV1, 0); // valid family, wrong stream
    let err = c
        .decompress_raw(
            7,
            compressed.read_count,
            compressed.uniform_read_length,
            bad,
            compressed.codec_seq,
            compressed.codec_qual,
            compressed.codec_aux,
            &compressed.id_stream,
            &compressed.seq_stream,
            &compressed.qual_stream,
            &compressed.aux_stream,
        )
        .unwrap_err();
    assert_unsupported(err, "ids");
}

#[test]
fn rejects_inapplicable_family_on_seq_stream() {
    let reads = sample_reads();
    let mut c = compressor();
    let compressed = c.compress(&reads, 0).unwrap();
    let bad = encode_codec(CodecFamily::DeltaZstd, 0);
    let err = c
        .decompress_raw(
            7,
            compressed.read_count,
            compressed.uniform_read_length,
            compressed.codec_ids,
            bad,
            compressed.codec_qual,
            compressed.codec_aux,
            &compressed.id_stream,
            &compressed.seq_stream,
            &compressed.qual_stream,
            &compressed.aux_stream,
        )
        .unwrap_err();
    assert_unsupported(err, "seq");
}

#[test]
fn rejects_inapplicable_family_on_qual_stream() {
    let reads = sample_reads();
    let mut c = compressor();
    let compressed = c.compress(&reads, 0).unwrap();
    let bad = encode_codec(CodecFamily::ZstdPlain, 0);
    let err = c
        .decompress_raw(
            7,
            compressed.read_count,
            compressed.uniform_read_length,
            compressed.codec_ids,
            compressed.codec_seq,
            bad,
            compressed.codec_aux,
            &compressed.id_stream,
            &compressed.seq_stream,
            &compressed.qual_stream,
            &compressed.aux_stream,
        )
        .unwrap_err();
    assert_unsupported(err, "qual");
}

#[test]
fn rejects_inapplicable_family_on_aux_stream() {
    let reads = sample_reads();
    let mut c = compressor();
    let compressed = c.compress(&reads, 0).unwrap();
    let bad = encode_codec(CodecFamily::Raw, 0);
    let err = c
        .decompress_raw(
            7,
            compressed.read_count,
            compressed.uniform_read_length,
            compressed.codec_ids,
            compressed.codec_seq,
            compressed.codec_qual,
            bad,
            &compressed.id_stream,
            &compressed.seq_stream,
            &compressed.qual_stream,
            &compressed.aux_stream,
        )
        .unwrap_err();
    assert_unsupported(err, "aux");
}

#[test]
fn rejects_unsupported_codec_version() {
    let reads = sample_reads();
    let mut c = compressor();
    let compressed = c.compress(&reads, 0).unwrap();
    let bad = encode_codec(CodecFamily::AbcV1, 1); // version 1 unimplemented
    let err = c
        .decompress_raw(
            3,
            compressed.read_count,
            compressed.uniform_read_length,
            compressed.codec_ids,
            bad,
            compressed.codec_qual,
            compressed.codec_aux,
            &compressed.id_stream,
            &compressed.seq_stream,
            &compressed.qual_stream,
            &compressed.aux_stream,
        )
        .unwrap_err();
    match err {
        FqcError::UnsupportedFormat(msg) => {
            assert!(msg.contains("seq") && msg.contains("version"), "{msg}");
        }
        other => panic!("expected UnsupportedFormat, got {other}"),
    }
}

#[test]
fn rejects_quality_codec_contradicting_global_flags() {
    let reads = sample_reads();
    let mut c = compressor(); // Lossless quality
    let compressed = c.compress(&reads, 0).unwrap();
    let discard = encode_codec(CodecFamily::Raw, 0);
    let err = c
        .decompress_raw(
            1,
            compressed.read_count,
            compressed.uniform_read_length,
            compressed.codec_ids,
            compressed.codec_seq,
            discard,
            compressed.codec_aux,
            &compressed.id_stream,
            &compressed.seq_stream,
            &[], // unused; fails on consistency before decode
            &compressed.aux_stream,
        )
        .unwrap_err();
    match err {
        FqcError::Format(msg) => assert!(msg.contains("qual") && msg.contains("quality_mode"), "{msg}"),
        other => panic!("expected Format contradiction, got {other}"),
    }
}
