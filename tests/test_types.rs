// =============================================================================
// fqc-rust - Types Unit Tests
// =============================================================================

use fqc::algo::block_compressor::BlockCompressorConfig;
use fqc::plan::{plan_compression_layout, CompressionPlanInput};
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

    // Empty records are structurally valid (seq/qual lengths match at 0):
    // the parser accepts them and verify must not flag archives containing
    // them. Strict content checks belong to the parser, not is_valid.
    let empty = ReadRecord::default();
    assert!(empty.is_valid());
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

#[test]
fn memory_estimates_use_class_upper_bounds() {
    // Decode falls back to class ceilings when the archive has no uniform
    // length. These are upper bounds for Short/Medium and a practical fallback
    // for Long, not the old fixed 256 bp short-read constant.
    assert_eq!(
        ReadLengthClass::Short.memory_estimate_read_length(),
        SPRING_MAX_READ_LENGTH
    );
    assert_eq!(
        ReadLengthClass::Medium.memory_estimate_read_length(),
        LONG_READ_THRESHOLD
    );
    assert_eq!(
        ReadLengthClass::Long.memory_estimate_read_length(),
        LONG_READ_MEMORY_ESTIMATE_BP
    );
}

#[test]
fn compression_plan_sizes_blocks_from_memory_limited_workers() {
    let config = BlockCompressorConfig::default();

    // Plenty of budget: the auto worker cap (4) drives the target block count,
    // so 100k short reads become eight 12.5k blocks rather than ten 10k blocks
    // derived from a larger request-thread count.
    let roomy = plan_compression_layout(CompressionPlanInput {
        requested_block_size: 0,
        avg_length: 150,
        max_block_bases: 0,
        thread_limit: 4,
        worker_budget_bytes: 8 * 1024 * 1024 * 1024,
        total_reads: Some(100_000),
        total_bases: Some(15_000_000),
        read_length_class: config.read_length_class,
        codec_model_bytes: config.to_quality_config().estimated_model_bytes(),
    });
    assert_eq!(roomy.workers, 4);
    assert_eq!(roomy.block_size, 12_500);

    // A 96 MB worker budget cannot sustain four 12.5k-read order-2 workers.
    // The planner must lower the worker count and enlarge blocks to match, not
    // keep ten blocks while only running the memory-limited subset.
    let constrained = plan_compression_layout(CompressionPlanInput {
        requested_block_size: 0,
        avg_length: 150,
        max_block_bases: 0,
        thread_limit: 8,
        worker_budget_bytes: 96 * 1024 * 1024,
        total_reads: Some(100_000),
        total_bases: Some(15_000_000),
        read_length_class: config.read_length_class,
        codec_model_bytes: config.to_quality_config().estimated_model_bytes(),
    });
    assert!(constrained.workers <= 2, "{constrained:?}");
    assert!(constrained.block_size > 10_000, "{constrained:?}");
    let constrained_blocks = 100_000usize.div_ceil(constrained.block_size);
    assert!(
        constrained_blocks >= constrained.workers * 2,
        "planner should retain at least two blocks per worker: {constrained:?}, blocks={constrained_blocks}"
    );
}

#[test]
fn decompression_estimate_does_not_treat_long_reads_as_short() {
    let long = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Long,
        ..Default::default()
    };
    let reads = 500;
    let by_class = long.estimated_worker_bytes(reads, ReadLengthClass::Long.memory_estimate_read_length());
    let old_short_constant = long.estimated_worker_bytes(reads, 256);
    assert!(
        by_class > old_short_constant,
        "long-read estimate {by_class} must exceed the old 256 bp estimate {old_short_constant}"
    );
}

#[test]
fn adaptive_block_size_balances_workers_and_base_ceiling() {
    // 100k short reads, 8 workers: two blocks per worker would be ~6.25k
    // reads, but the short-read floor keeps blocks at 10k reads.
    assert_eq!(
        adaptive_block_size(0, ReadLengthClass::Short, 150, 0, 8, Some(100_000), Some(15_000_000)),
        10_000
    );
    assert_eq!(
        adaptive_block_size(0, ReadLengthClass::Short, 150, 0, 8, Some(200_000), Some(30_000_000)),
        12_500
    );

    // 4k long reads at 16 kb each must become multiple blocks instead of one
    // serial 65 Mbp block. Archive/pipeline use the 5 Mbp long-read ceiling.
    assert_eq!(
        adaptive_block_size(0, ReadLengthClass::Long, 16_000, 0, 8, Some(4_000), Some(64_000_000)),
        312
    );

    // Streaming intentionally keeps its historical read-count sizing: the
    // archive/pipeline long-read auto-split ceiling must not leak into it.
    assert_eq!(
        adaptive_block_size_with_policy(
            0,
            ReadLengthClass::Long,
            16_000,
            0,
            8,
            Some(4_000),
            Some(64_000_000),
            false
        ),
        500
    );

    // Exact totals override a stale sampled average when sizing the base cap.
    assert_eq!(
        adaptive_block_size(
            0,
            ReadLengthClass::Short,
            1_000,
            300_000,
            8,
            Some(100_000),
            Some(15_000_000)
        ),
        2_000
    );

    // Explicit read count wins; the automatic base cap only applies in auto mode.
    assert_eq!(
        adaptive_block_size(
            7_000,
            ReadLengthClass::Short,
            150,
            0,
            8,
            Some(100_000),
            Some(15_000_000)
        ),
        7_000
    );
    assert_eq!(
        adaptive_block_size(
            0,
            ReadLengthClass::Short,
            150,
            300_000,
            8,
            Some(100_000),
            Some(15_000_000)
        ),
        2_000
    );
}

#[test]
fn length_stats_slice_and_histogram_agree() {
    let cases: &[&[usize]] = &[
        &[],
        &[100],
        &[100, 200],
        &[100, 100, 200],
        &[100, 200, 200, 200, 5_000],
        &[0, 1, 1, 2, 2, 2],
    ];
    for lengths in cases {
        let slice = LengthStats::from_lengths(lengths);
        let histogram = LengthStats::from_lengths_iter(lengths.iter().copied());
        assert_eq!(slice.sample_size, histogram.sample_size, "lengths={lengths:?}");
        assert_eq!(slice.avg_length, histogram.avg_length, "lengths={lengths:?}");
        assert_eq!(slice.median_length, histogram.median_length, "lengths={lengths:?}");
        assert_eq!(slice.max_length, histogram.max_length, "lengths={lengths:?}");
    }
}

#[test]
fn length_stats_histogram_median_uses_lower_middle_for_even_counts() {
    // Sorted [10, 20, 30, 40]: the slice implementation picks index 2 -> 30.
    let stats = LengthStats::from_lengths_iter([10, 20, 30, 40]);
    assert_eq!(stats.median_length, 30);
    assert_eq!(stats.max_length, 40);
    assert_eq!(stats.avg_length, 25);
    assert_eq!(stats.sample_size, 4);
}
