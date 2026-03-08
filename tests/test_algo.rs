// =============================================================================
// fqc-rust - Algorithm Module Tests (ID Compressor, Quality Compressor, PE Optimizer)
// =============================================================================

use fqc::algo::id_compressor::{compress_ids, decompress_ids};
use fqc::algo::pe_optimizer::*;
use fqc::algo::quality_compressor::{QualityCompressor, QualityCompressorConfig};
use fqc::types::*;

// =============================================================================
// ID Compressor
// =============================================================================

#[test]
fn test_id_compress_decompress_exact() {
    let ids = vec!["read_0", "read_1", "read_2", "read_3", "read_4"];
    let compressed = compress_ids(&ids, 3, false).unwrap();
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
    let compressed = compress_ids(&ids, 3, true).unwrap();

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
    let compressed = compress_ids(&ids, 3, false).unwrap();
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
    let compressed = compress_ids(&ids, 3, false).unwrap();
    let decompressed = decompress_ids(&compressed, 4, "read").unwrap();
    assert_eq!(decompressed.len(), 4);
    for (orig, dec) in ids.iter().zip(decompressed.iter()) {
        assert_eq!(*orig, dec);
    }
}

#[test]
fn test_id_compress_single() {
    let ids = vec!["single_read"];
    let compressed = compress_ids(&ids, 3, false).unwrap();
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

// =============================================================================
// PE Optimizer
// =============================================================================

#[test]
fn test_pe_optimizer_no_complementarity() {
    let config = PEOptimizerConfig {
        enable_complementarity: false,
        ..Default::default()
    };
    let mut optimizer = PEOptimizer::new(config);

    let r1 = ReadRecord::new("r1".into(), "ACGTACGT".into(), "IIIIIIII".into());
    let r2 = ReadRecord::new("r2".into(), "TTTTTTTTT".into(), "!!!!!!!!!".into());

    let encoded = optimizer.encode_pair(&r1, &r2);
    assert!(!encoded.use_complementarity);

    let (dec_r1, dec_r2) = optimizer.decode_pair(&encoded);
    assert_eq!(dec_r1.sequence, r1.sequence);
    assert_eq!(dec_r2.sequence, r2.sequence);
    assert_eq!(dec_r1.quality, r1.quality);
    assert_eq!(dec_r2.quality, r2.quality);
}

#[test]
fn test_pe_optimizer_with_complementarity() {
    let config = PEOptimizerConfig {
        enable_complementarity: true,
        complementarity_threshold: 5,
        min_overlap: 4,
    };
    let mut optimizer = PEOptimizer::new(config);

    // R2 is exact reverse complement of R1
    let r1 = ReadRecord::new("r1".into(), "ACGTACGTACGT".into(), "IIIIIIIIIIII".into());
    let r2_seq = String::from_utf8(fqc::algo::dna::reverse_complement(b"ACGTACGTACGT")).unwrap();
    let r2 = ReadRecord::new("r2".into(), r2_seq.clone(), "IIIIIIIIIIII".into());

    let encoded = optimizer.encode_pair(&r1, &r2);
    assert!(encoded.use_complementarity);

    let (dec_r1, dec_r2) = optimizer.decode_pair(&encoded);
    assert_eq!(dec_r1.sequence, r1.sequence);
    assert_eq!(dec_r2.sequence, r2_seq);
}

#[test]
fn test_pe_check_complementarity_dissimilar() {
    let config = PEOptimizerConfig {
        enable_complementarity: true,
        complementarity_threshold: 5,
        min_overlap: 4,
    };
    let optimizer = PEOptimizer::new(config);

    // Completely unrelated sequences: not complementary
    let (beneficial, _) = optimizer.check_complementarity(b"AAAAAAAA", b"AAAAAAAA");
    assert!(!beneficial);
}

#[test]
fn test_pe_check_complementarity_rc() {
    let config = PEOptimizerConfig {
        enable_complementarity: true,
        complementarity_threshold: 5,
        min_overlap: 4,
    };
    let optimizer = PEOptimizer::new(config);

    let r1 = b"ACGTACGTACGT";
    let r2 = fqc::algo::dna::reverse_complement(r1);
    let (beneficial, diff_count) = optimizer.check_complementarity(r1, &r2);
    assert!(beneficial);
    assert_eq!(diff_count, 0);
}

#[test]
fn test_pe_generate_r2_id_slash() {
    assert_eq!(generate_r2_id("read1/1"), "read1/2");
}

#[test]
fn test_pe_generate_r2_id_illumina() {
    assert_eq!(
        generate_r2_id("HWUSI:1:1101:1234:5678 1:N:0:ATCACG"),
        "HWUSI:1:1101:1234:5678 2:N:0:ATCACG"
    );
}

#[test]
fn test_pe_generate_r2_id_fallback() {
    assert_eq!(generate_r2_id("simple_read"), "simple_read/2");
}

#[test]
fn test_pe_serialize_deserialize_roundtrip() {
    let pair = PEEncodedPair {
        id1: "read1".into(),
        seq1: "ACGT".into(),
        qual1: "IIII".into(),
        id2: "read2".into(),
        seq2: "TGCA".into(),
        qual2: "!!!!".into(),
        use_complementarity: false,
        diff_positions: vec![],
        diff_bases: vec![],
        qual_delta: vec![],
    };

    let serialized = serialize_encoded_pair(&pair);
    assert!(!serialized.is_empty());

    let mut pos = 0;
    let deserialized = deserialize_encoded_pair(&serialized, &mut pos).unwrap();
    // serialize only stores id2, seq2, qual2 (not id1/seq1/qual1)
    assert_eq!(deserialized.id2, pair.id2);
    assert_eq!(deserialized.seq2, pair.seq2);
    assert_eq!(deserialized.qual2, pair.qual2);
    assert!(!deserialized.use_complementarity);
}

#[test]
fn test_pe_serialize_deserialize_with_complementarity() {
    let pair = PEEncodedPair {
        id1: "r1".into(),
        seq1: "ACGTACGT".into(),
        qual1: "IIIIIIII".into(),
        id2: "r2".into(),
        seq2: String::new(),
        qual2: String::new(),
        use_complementarity: true,
        diff_positions: vec![1, 3],
        diff_bases: vec![b'A', b'C'],
        // qual_delta length must match diff_count for serialization roundtrip
        qual_delta: vec![2, -1],
    };

    let serialized = serialize_encoded_pair(&pair);
    let mut pos = 0;
    let deserialized = deserialize_encoded_pair(&serialized, &mut pos).unwrap();
    assert!(deserialized.use_complementarity);
    assert_eq!(deserialized.diff_positions, vec![1, 3]);
    assert_eq!(deserialized.diff_bases, vec![b'A', b'C']);
    assert_eq!(deserialized.qual_delta, vec![2, -1]);
}

#[test]
fn test_pe_optimizer_encode_decode_roundtrip_no_comp() {
    // Use dissimilar sequences that won't trigger complementarity
    let config = PEOptimizerConfig {
        enable_complementarity: true,
        complementarity_threshold: 2,
        min_overlap: 8,
    };
    let mut optimizer = PEOptimizer::new(config);

    let r1 = ReadRecord::new("read1/1".into(), "AAAAAAAAAAAAAAAA".into(), "IIIIIIIIIIIIIIII".into());
    let r2 = ReadRecord::new("read1/2".into(), "CCCCCCCCCCCCCCCC".into(), "!!!!IIII~~~~!!!!".into());

    let encoded = optimizer.encode_pair(&r1, &r2);
    // These are too different for complementarity
    assert!(!encoded.use_complementarity);

    let (dec_r1, dec_r2) = optimizer.decode_pair(&encoded);
    assert_eq!(dec_r1.sequence, r1.sequence);
    assert_eq!(dec_r1.quality, r1.quality);
    assert_eq!(dec_r2.sequence, r2.sequence);
    assert_eq!(dec_r2.quality, r2.quality);
}
