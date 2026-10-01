// =============================================================================
// fqc-rust - Zstd Sequence Codec Direct Tests
// =============================================================================
// ZstdSequenceCompressor previously had zero direct test coverage; its
// defensive branches (empty stream, count/length disagreement, trailing
// bytes) were only reachable through higher-level roundtrips. These tests
// pin each branch at the codec boundary.
// =============================================================================

use fqc::algo::zstd_sequence::ZstdSequenceCompressor;
use fqc::types::ReadRecord;
use fqc::SequenceCompressor;

fn make_reads(lens: &[usize]) -> Vec<ReadRecord> {
    lens.iter()
        .enumerate()
        .map(|(i, &len)| {
            let seq: String = (0..len).map(|j| b"ACGT"[(i + j) % 4] as char).collect();
            ReadRecord::new(format!("read_{i}"), seq, "I".repeat(len))
        })
        .collect()
}

fn codec() -> ZstdSequenceCompressor {
    ZstdSequenceCompressor::new(5)
}

#[test]
fn roundtrip_with_explicit_lengths() {
    let reads = make_reads(&[600, 612, 588]);
    let data = codec().compress(&reads).unwrap();

    let lens: Vec<u32> = reads.iter().map(|r| r.sequence.len() as u32).collect();
    let out = codec().decompress(&data, 3, 0, &lens).unwrap();
    let expected: Vec<String> = reads.iter().map(|r| r.sequence.clone()).collect();
    assert_eq!(out, expected);
}

#[test]
fn roundtrip_with_uniform_length() {
    let reads = make_reads(&[600, 600, 600]);
    let data = codec().compress(&reads).unwrap();

    let out = codec().decompress(&data, 3, 600, &[]).unwrap();
    let expected: Vec<String> = reads.iter().map(|r| r.sequence.clone()).collect();
    assert_eq!(out, expected);
}

#[test]
fn empty_stream_with_zero_reads_is_ok() {
    let out = codec().decompress(&[], 0, 0, &[]).unwrap();
    assert!(out.is_empty());
}

#[test]
fn empty_stream_with_nonzero_reads_is_rejected() {
    let err = codec().decompress(&[], 2, 0, &[10, 10]).unwrap_err();
    assert!(err.to_string().contains("empty for a non-empty block"));
}

#[test]
fn lengths_count_mismatch_is_rejected() {
    let reads = make_reads(&[600, 600]);
    let data = codec().compress(&reads).unwrap();

    // uniform_length = 0 requires one length per read; one is missing.
    let err = codec().decompress(&data, 2, 0, &[600]).unwrap_err();
    assert!(err.to_string().contains("lengths for 2 reads"));
}

#[test]
fn aux_length_disagreement_is_rejected() {
    let reads = make_reads(&[600]);
    let data = codec().compress(&reads).unwrap();

    let err = codec().decompress(&data, 1, 0, &[599]).unwrap_err();
    assert!(err.to_string().contains("disagrees with aux length"));
}

#[test]
fn trailing_bytes_are_rejected() {
    // Craft a payload of one record plus trailing bytes that stays inside
    // the bounded output cap (600 + 4 + 64): the decode loop consumes the
    // record prefix and the remainder must be reported, not ignored.
    let mut payload = Vec::new();
    payload.extend_from_slice(&600u32.to_le_bytes());
    payload.extend(std::iter::repeat(b'A').take(600));
    payload.extend_from_slice(b"TRAILING");
    let data = zstd::bulk::compress(&payload, 5).unwrap();

    let err = codec().decompress(&data, 1, 0, &[600]).unwrap_err();
    assert!(err.to_string().contains("trailing bytes"));
}

#[test]
fn truncated_payload_is_rejected() {
    let reads = make_reads(&[600, 600]);
    let data = codec().compress(&reads).unwrap();

    let cut = data.len() / 2;
    let err = codec().decompress(&data[..cut], 2, 0, &[600, 600]).unwrap_err();
    assert!(
        err.to_string().contains("zstd sequence") || err.to_string().contains("Truncated"),
        "unexpected error: {err}"
    );
}
