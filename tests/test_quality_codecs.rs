// =============================================================================
// fqc-rust - Quality Codec Tests
// =============================================================================
// The arithmetic encoder / adaptive model and the Illumina8 quantizer had no
// direct tests. These tests exercise the full quality-value range (Phred 0-93
// i.e. ASCII 33-126), the Illumina8 mode roundtrip (first ever), and assert
// discard placeholder content (not just length).
// =============================================================================

use fqc::algo::quality_compressor::{QualityCompressor, QualityCompressorConfig};
use fqc::types::QualityMode;

// =============================================================================
// Helpers
// =============================================================================

fn roundtrip(qualities: &[&str], mode: QualityMode) -> Vec<String> {
    let config = QualityCompressorConfig {
        quality_mode: mode,
        ..Default::default()
    };
    let mut compressor = QualityCompressor::new(config.clone());
    let compressed = compressor.compress(qualities).unwrap();
    let lengths: Vec<u32> = qualities.iter().map(|q| q.len() as u32).collect();
    let mut decompressor = QualityCompressor::new(config);
    decompressor.decompress(&compressed, &lengths).unwrap()
}

// =============================================================================
// Lossless: full Phred range
// =============================================================================

#[test]
fn quality_lossless_phred_full_range_roundtrip() {
    // Every printable ASCII quality value from '!' (Phred 0) to '~' (Phred 93).
    let full: String = (33u8..=126).map(|c| c as char).collect();
    let qualities = vec![
        full.as_str(),
        "!",          // single, Phred 0
        "~",          // single, Phred 93
        "!!!!IIIII!", // mix of extremes
        "~~~~!!!!II", // high-low mix
        "",           // empty quality string
    ];
    let restored = roundtrip(&qualities, QualityMode::Lossless);
    assert_eq!(restored.len(), qualities.len());
    for (orig, dec) in qualities.iter().zip(restored.iter()) {
        assert_eq!(*orig, dec, "lossless roundtrip must preserve quality exactly");
    }
}

#[test]
fn quality_lossless_varied_lengths_and_values() {
    // Long, varied values to stress the arithmetic coder's frequency model.
    let qualities: Vec<String> = (0..50)
        .map(|i| {
            let len = 1 + (i * 7) % 200;
            (0..len).map(|j| (33 + ((i * 13 + j) % 94)) as u8 as char).collect()
        })
        .collect();
    let refs: Vec<&str> = qualities.iter().map(|s| s.as_str()).collect();
    let restored = roundtrip(&refs, QualityMode::Lossless);
    assert_eq!(restored.len(), qualities.len());
    for (orig, dec) in qualities.iter().zip(restored.iter()) {
        assert_eq!(orig, dec, "varied-length lossless roundtrip mismatch");
    }
}

// =============================================================================
// Illumina8 (first-ever roundtrip)
// =============================================================================
// Illumina8 is a LOSSY 8-bin quantizer: values map to bin representatives
// [2,6,15,22,27,33,37,40] (Phred), i.e. ASCII '#'(35) '\''(39) '0'(48) '7'(55)
// '<'(60) 'B'(66) 'F'(70) 'I'(73). Roundtrip must be stable and the output
// must consist only of bin representatives.

fn illumina8_valid_chars() -> Vec<char> {
    [2u8, 6, 15, 22, 27, 33, 37, 40]
        .iter()
        .map(|&phred| (33 + phred) as char)
        .collect()
}

fn assert_only_valid_chars(s: &str, valid: &[char]) {
    for c in s.chars() {
        assert!(
            valid.contains(&c),
            "unexpected char {:?} in quantized output {:?}",
            c,
            s
        );
    }
}

#[test]
fn quality_illumina8_roundtrip() {
    let qualities = vec!["IIIIIIIIII", "!!!!!!", "~~~~~~~~~~", "IIII!!!!~~~~"];
    let restored = roundtrip(&qualities, QualityMode::Illumina8);
    assert_eq!(restored.len(), qualities.len());
    let valid = illumina8_valid_chars();
    for (orig, dec) in qualities.iter().zip(restored.iter()) {
        assert_eq!(orig.len(), dec.len(), "quantization preserves length");
        assert_only_valid_chars(dec, &valid);
    }
}

#[test]
fn quality_illumina8_full_range_roundtrip() {
    let full: String = (33u8..=126).map(|c| c as char).collect();
    let qualities = vec![full.as_str(), "!", "~"];
    let restored = roundtrip(&qualities, QualityMode::Illumina8);
    let valid = illumina8_valid_chars();
    for dec in &restored {
        assert_only_valid_chars(dec, &valid);
    }
}

#[test]
fn quality_illumina8_is_idempotent_for_stable_values() {
    // KNOWN ISSUE: the bin-0 representative is NOT idempotent. Phred 0-1
    // quantize to bin 0 whose representative is Phred 2 ('#'), but
    // re-quantizing Phred 2 falls into bin 1 (representative Phred 6, "'").
    // So '#' can never be a fixed point. All other bins are stable.
    // This test pins idempotency for bins 1-7; the bin-0 defect is a real bug
    // reported separately (do not fix production code here).
    let stable = vec![
        "''''''''''",
        "0000000000",
        "7777777777",
        "<<<<<<<<<<",
        "BBBBBBBBBB",
        "FFFFFFFFF",
        "IIIIIIIIII",
    ];
    let once = roundtrip(&stable, QualityMode::Illumina8);
    let refs: Vec<&str> = once.iter().map(|s| s.as_str()).collect();
    let twice = roundtrip(&refs, QualityMode::Illumina8);
    assert_eq!(once, twice, "deep-bin illumina8 quantization must be idempotent");
}

// =============================================================================
// Discard: placeholder content
// =============================================================================

#[test]
fn quality_discard_placeholder_is_ascii33() {
    let qualities = ["IIIII", "!!!!!", "~~"]; // varied values
    let config = QualityCompressorConfig {
        quality_mode: QualityMode::Discard,
        ..Default::default()
    };
    let mut compressor = QualityCompressor::new(config.clone());
    let compressed = compressor.compress(&qualities).unwrap();
    // Discard mode produces no quality payload.
    assert!(compressed.is_empty());

    let lengths = vec![5u32, 5, 2];
    let mut decompressor = QualityCompressor::new(config);
    let decompressed = decompressor.decompress(&compressed, &lengths).unwrap();

    assert_eq!(decompressed.len(), 3);
    for (i, dec) in decompressed.iter().enumerate() {
        assert_eq!(dec.len(), lengths[i] as usize);
        assert!(
            dec.chars().all(|c| c == '!'),
            "discard placeholder must be '!' (Phred 0), got {:?}",
            dec
        );
    }
}

// =============================================================================
// QVZ: quantization stability
// =============================================================================
// QVZ is a LOSSY nearest-neighbor quantizer over the codebook
// [7,15,20,25,30,35,40,41] (Phred) → ASCII '('(40) '0'(48) '5'(53) ':'(58)
// '?'(63) 'D'(68) 'I'(73) 'J'(74). Roundtrip must be stable and output must
// consist only of codebook representatives.

fn qvz_valid_chars() -> Vec<char> {
    [7u8, 15, 20, 25, 30, 35, 40, 41]
        .iter()
        .map(|&phred| (33 + phred) as char)
        .collect()
}

#[test]
fn quality_qvz_roundtrip() {
    let qualities = vec!["IIIIIIIIII", "!!!!!!!!!!", "I!I!I!I!I!", "~~~~!!!!II"];
    let restored = roundtrip(&qualities, QualityMode::Qvz);
    assert_eq!(restored.len(), qualities.len());
    let valid = qvz_valid_chars();
    for dec in &restored {
        assert_only_valid_chars(dec, &valid);
    }
}

#[test]
fn quality_qvz_is_idempotent() {
    let qualities = vec!["IIIIIIIIII", "!!!!!!!!!!", "I!I!I!I!I!", "~~~~!!!!II"];
    let once = roundtrip(&qualities, QualityMode::Qvz);
    let refs: Vec<&str> = once.iter().map(|s| s.as_str()).collect();
    let twice = roundtrip(&refs, QualityMode::Qvz);
    assert_eq!(once, twice, "qvz quantization must be idempotent");
}

#[test]
fn quality_qvz_compresses_high_redundancy_qualities() {
    // All-identical qualities should compress to something small.
    let qualities: Vec<String> = (0..200).map(|_| "IIIIIIIIII".to_string()).collect();
    let refs: Vec<&str> = qualities.iter().map(|s| s.as_str()).collect();

    let config = QualityCompressorConfig {
        quality_mode: QualityMode::Qvz,
        ..Default::default()
    };
    let mut compressor = QualityCompressor::new(config.clone());
    let compressed = compressor.compress(&refs).unwrap();
    let lengths: Vec<u32> = refs.iter().map(|q| q.len() as u32).collect();
    let mut decompressor = QualityCompressor::new(config);
    let decompressed = decompressor.decompress(&compressed, &lengths).unwrap();

    assert_eq!(decompressed.len(), 200);
    for dec in &decompressed {
        assert_eq!(dec, "IIIIIIIIII");
    }
    // 200 * 10 = 2000 raw bytes; QVZ should be substantially smaller.
    assert!(
        compressed.len() < 2000,
        "200 identical quality strings should compress well ({} bytes)",
        compressed.len()
    );
}
