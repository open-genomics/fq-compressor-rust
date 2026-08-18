// =============================================================================
// fqc-rust - Algorithm Module Tests (ID Compressor, Quality Compressor, PE Optimizer)
// =============================================================================

use fqc::algo::global_analyzer::{GlobalAnalyzer, GlobalAnalyzerConfig};
use fqc::algo::id_compressor::{compress_ids, decompress_ids, ID_MAGIC_EXACT, ID_MAGIC_TOKENIZE};
use fqc::algo::quality_compressor::{QualityCompressor, QualityCompressorConfig};
use fqc::types::*;

// =============================================================================
// ID Compressor
// =============================================================================

#[test]
fn test_id_compress_decompress_exact() {
    let ids = vec!["read_0", "read_1", "read_2", "read_3", "read_4"];
    let compressed = compress_ids(&ids, 3, IdMode::Tokenize).unwrap();
    assert!(!compressed.is_empty());

    let decompressed = decompress_ids(&compressed, 5, "read").unwrap();
    assert_eq!(decompressed.len(), 5);
    for (orig, dec) in ids.iter().zip(decompressed.iter()) {
        assert_eq!(*orig, dec);
    }
}

#[test]
fn test_id_compress_decompress_discard() {
    let ids = vec!["read_0", "read_1", "read_2"];
    let compressed = compress_ids(&ids, 3, IdMode::Discard).unwrap();

    let decompressed = decompress_ids(&compressed, 3, "block0").unwrap();
    assert_eq!(decompressed.len(), 3);
    // Discarded IDs should be synthetic placeholders
    for id in &decompressed {
        assert!(!id.is_empty());
    }
}

#[test]
fn test_id_compress_decompress_empty() {
    let ids: Vec<&str> = vec![];
    let compressed = compress_ids(&ids, 3, IdMode::Tokenize).unwrap();
    let decompressed = decompress_ids(&compressed, 0, "").unwrap();
    assert!(decompressed.is_empty());
}

#[test]
fn test_id_compress_decompress_illumina_style() {
    let ids = vec![
        "HWUSI:1:1101:1234:5678",
        "HWUSI:1:1101:1234:5679",
        "HWUSI:1:1101:1234:5680",
        "HWUSI:1:1101:1235:5678",
    ];
    let compressed = compress_ids(&ids, 3, IdMode::Tokenize).unwrap();
    let decompressed = decompress_ids(&compressed, 4, "read").unwrap();
    assert_eq!(decompressed.len(), 4);
    for (orig, dec) in ids.iter().zip(decompressed.iter()) {
        assert_eq!(*orig, dec);
    }
}

#[test]
fn test_id_exact_mode_does_not_tokenize_patterned_ids() {
    let ids = vec![
        "HWUSI:1:1101:1234:5678",
        "HWUSI:1:1101:1234:5679",
        "HWUSI:1:1101:1234:5680",
        "HWUSI:1:1101:1235:5678",
    ];
    let exact = compress_ids(&ids, 3, IdMode::Exact).unwrap();
    let tokenized = compress_ids(&ids, 3, IdMode::Tokenize).unwrap();
    assert_eq!(exact[0], ID_MAGIC_EXACT);
    assert_eq!(tokenized[0], ID_MAGIC_TOKENIZE);
    assert_eq!(decompress_ids(&exact, 4, "read").unwrap(), ids);
    assert_eq!(decompress_ids(&tokenized, 4, "read").unwrap(), ids);
}

#[test]
fn test_id_compress_single() {
    let ids = vec!["single_read"];
    let compressed = compress_ids(&ids, 3, IdMode::Tokenize).unwrap();
    let decompressed = decompress_ids(&compressed, 1, "read").unwrap();
    assert_eq!(decompressed, vec!["single_read"]);
}

// =============================================================================
// Quality Compressor
// =============================================================================

#[test]
fn test_quality_compress_decompress_lossless() {
    let qualities = ["IIIIIIIII!", "!!!!IIIII!", "~~~~!!!!II"];
    let lengths: Vec<u32> = qualities.iter().map(|q| q.len() as u32).collect();
    let refs: Vec<&str> = qualities.to_vec();

    let config = QualityCompressorConfig {
        quality_mode: QualityMode::Lossless,
        ..Default::default()
    };
    let mut compressor = QualityCompressor::new(config.clone());
    let compressed = compressor.compress(&refs).unwrap();
    assert!(!compressed.is_empty());

    let mut decompressor = QualityCompressor::new(config);
    let decompressed = decompressor.decompress(&compressed, &lengths).unwrap();
    assert_eq!(decompressed.len(), 3);
    for (orig, dec) in qualities.iter().zip(decompressed.iter()) {
        assert_eq!(*orig, dec);
    }
}

#[test]
fn test_quality_compress_decompress_discard() {
    let qualities = ["IIIII", "!!!!!"];
    let refs: Vec<&str> = qualities.to_vec();

    let config = QualityCompressorConfig {
        quality_mode: QualityMode::Discard,
        ..Default::default()
    };
    let mut compressor = QualityCompressor::new(config.clone());
    let compressed = compressor.compress(&refs).unwrap();
    assert!(compressed.is_empty());

    let lengths = vec![5u32, 5];
    let mut decompressor = QualityCompressor::new(config);
    let decompressed = decompressor.decompress(&compressed, &lengths).unwrap();
    assert_eq!(decompressed.len(), 2);
    // Discarded quality should be uniform placeholder
    for dec in &decompressed {
        assert_eq!(dec.len(), 5);
    }
}

#[test]
fn test_quality_compress_decompress_empty() {
    let qualities: Vec<&str> = vec![];
    let config = QualityCompressorConfig {
        quality_mode: QualityMode::Lossless,
        ..Default::default()
    };
    let mut compressor = QualityCompressor::new(config.clone());
    let compressed = compressor.compress(&qualities).unwrap();

    let mut decompressor = QualityCompressor::new(config);
    let decompressed = decompressor.decompress(&compressed, &[]).unwrap();
    assert!(decompressed.is_empty());
}

#[test]
fn test_quality_compress_decompress_varied_lengths() {
    let qualities = ["III", "!!!!!!", "~~~~!"];
    let lengths: Vec<u32> = qualities.iter().map(|q| q.len() as u32).collect();
    let refs: Vec<&str> = qualities.to_vec();

    let config = QualityCompressorConfig {
        quality_mode: QualityMode::Lossless,
        ..Default::default()
    };
    let mut compressor = QualityCompressor::new(config.clone());
    let compressed = compressor.compress(&refs).unwrap();

    let mut decompressor = QualityCompressor::new(config);
    let decompressed = decompressor.decompress(&compressed, &lengths).unwrap();
    for (orig, dec) in qualities.iter().zip(decompressed.iter()) {
        assert_eq!(*orig, dec);
    }
}

#[test]
fn test_quality_qvz_quantizes_to_codebook() {
    let qualities = ["!#I~"];
    let refs: Vec<&str> = qualities.to_vec();
    let config = QualityCompressorConfig {
        quality_mode: QualityMode::Qvz,
        ..Default::default()
    };
    let mut compressor = QualityCompressor::new(config.clone());
    let compressed = compressor.compress(&refs).unwrap();
    let mut decompressor = QualityCompressor::new(config);
    let out = decompressor.decompress(&compressed, &[4]).unwrap();
    assert_eq!(out.len(), 1);
    assert_ne!(out[0], qualities[0]);
    for c in out[0].chars() {
        let q = (c as u8).saturating_sub(33);
        assert!(
            fqc::algo::quality_compressor::QVZ_CODEBOOK.contains(&q),
            "reconstructed q={q} not in QVZ codebook"
        );
    }
}

#[test]
fn test_global_analyzer_respects_requested_block_size() {
    let sequences: Vec<String> = (0..10).map(|_| "ACGTACGT".to_string()).collect();
    let analyzer = GlobalAnalyzer::new(GlobalAnalyzerConfig {
        reads_per_block: 3,
        enable_reorder: false,
        read_length_class: Some(ReadLengthClass::Short),
        ..Default::default()
    });

    let result = analyzer.analyze(&sequences).unwrap();

    assert_eq!(result.num_blocks, 4);
    assert_eq!(result.block_boundaries[0].archive_id_start, 0);
    assert_eq!(result.block_boundaries[0].archive_id_end, 3);
    assert_eq!(result.block_boundaries[3].archive_id_start, 9);
    assert_eq!(result.block_boundaries[3].archive_id_end, 10);
}
