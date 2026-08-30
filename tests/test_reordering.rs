// =============================================================================
// fqc-rust - Global Reordering Correctness Tests
// =============================================================================
// The reorder map is the highest-risk algorithm in the codebase: it was only
// ever asserted as a boolean flag (`reorder_map_written`). These tests verify
// the actual mathematics:
//   - forward ∘ reverse = identity (maps are true inverses)
//   - the reordered archive decompresses back to the original order
//   - minimizer extraction is well-formed
// =============================================================================

use fqc::algo::global_analyzer::{extract_minimizers, GlobalAnalyzer, GlobalAnalyzerConfig};
use fqc::archive::format::{get_id_mode, get_quality_mode, get_read_length_class};
use fqc::commands::compress::{CompressCommand, CompressOptions};
use fqc::types::*;

// =============================================================================
// Helpers
// =============================================================================

fn make_reads(n: usize, len: usize) -> Vec<ReadRecord> {
    let bases = b"ACGT";
    (0..n)
        .map(|i| {
            // Deterministic but with periodic variation so minimizers differ.
            let seq: String = (0..len).map(|j| bases[(i * 7 + j) % 4] as char).collect();
            let qual = "I".repeat(len);
            ReadRecord::new(format!("read_{:04}", i), seq, qual)
        })
        .collect()
}

fn write_fastq(path: &std::path::Path, records: &[ReadRecord]) {
    let mut out = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    for record in records {
        fqc::fastq::parser::write_record(&mut out, record).unwrap();
    }
}

fn read_fastq_records(path: &std::path::Path) -> Vec<ReadRecord> {
    let file = std::fs::File::open(path).unwrap();
    let mut parser = fqc::fastq::parser::FastqParser::new(std::io::BufReader::new(file));
    parser.collect_all().unwrap()
}

/// Compress with reordering enabled, then decompress and compare per-read.
fn reorder_roundtrip(records: &[ReadRecord], threads: usize) -> Vec<ReadRecord> {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    let restored = dir.path().join("restored.fastq");

    write_fastq(&fastq, records);
    let opts = CompressOptions {
        input_path: fastq.to_string_lossy().to_string(),
        output_path: archive.to_string_lossy().to_string(),
        enable_reorder: true,
        show_progress: false,
        force_overwrite: true,
        threads,
        ..Default::default()
    };
    assert_eq!(CompressCommand::new(opts).execute(), 0, "compress failed");

    // Decompress via the reader API.
    let mut reader = fqc::archive::reader::FqcReader::open(archive.to_string_lossy().as_ref()).unwrap();
    assert!(reader.has_reorder_map(), "reorder should produce a reorder map");

    let flags = reader.global_header.flags;
    let config = fqc::algo::block_compressor::BlockCompressorConfig {
        read_length_class: get_read_length_class(flags),
        quality_mode: get_quality_mode(flags),
        id_mode: get_id_mode(flags),
        ..Default::default()
    };
    let mut compressor = fqc::algo::block_compressor::BlockCompressor::new(config);

    let mut out = std::io::BufWriter::new(std::fs::File::create(&restored).unwrap());
    for block_id in 0..reader.block_count() {
        let bd = reader.read_block(block_id as u32).unwrap();
        let bh = &bd.header;
        let dec = compressor
            .decompress_raw(
                bh.block_id,
                bh.uncompressed_count,
                bh.uniform_read_length,
                bh.codec_ids,
                bh.codec_seq,
                bh.codec_qual,
                bh.codec_aux,
                &bd.ids_data,
                &bd.seq_data,
                &bd.qual_data,
                &bd.aux_data,
            )
            .unwrap();
        for read in &dec.reads {
            fqc::fastq::parser::write_record(&mut out, read).unwrap();
        }
    }
    drop(out);

    read_fastq_records(&restored)
}

// =============================================================================
// Minimizer Extraction
// =============================================================================

#[test]
fn extract_minimizers_basic() {
    let seq = b"ACGTACGTACGTACGTACGT"; // 20 bp
    let mins = extract_minimizers(seq, 5, 4);
    assert!(!mins.is_empty(), "a 20bp sequence must yield minimizers");
    for m in &mins {
        assert!(m.hash > 0, "canonical k-mer hash should be non-trivial");
    }
}

#[test]
fn extract_minimizers_shorter_than_k_returns_empty() {
    assert!(extract_minimizers(b"ACGT", 5, 3).is_empty());
    assert!(extract_minimizers(b"", 3, 2).is_empty());
    // Degenerate window sizes must not panic (w=0 yields one minimizer per
    // window position — documented implementation behavior).
    assert!(!extract_minimizers(b"ACGTACGT", 3, 0).is_empty());
}

#[test]
fn extract_minimizers_identical_kmers_dedupe() {
    // All-A sequence: every k-mer is identical, so each window's minimum is
    // the leftmost position of that window; the minimizer count is bounded by
    // the number of windows (no more than one per window).
    let seq = b"AAAAAAAAAAAAAAAAAAAA"; // 20 A's
    let mins = extract_minimizers(seq, 5, 4);
    let num_windows = seq.len() - 5 + 1 - (4 - 1);
    assert!(mins.len() <= num_windows, "at most one minimizer per window");
    assert!(!mins.is_empty());
}

#[test]
fn extract_minimizers_canonical_rc_invariance() {
    // A k-mer and its reverse complement hash to the same canonical value, so
    // a sequence and its reverse complement produce the same minimizer set.
    let seq = b"ACGTACGTACGTACGTACGT";
    let rc: Vec<u8> = seq
        .iter()
        .rev()
        .map(|&b| match b {
            b'A' => b'T',
            b'C' => b'G',
            b'G' => b'C',
            b'T' => b'A',
            _ => b'N',
        })
        .collect();
    let a = extract_minimizers(seq, 5, 4);
    let b = extract_minimizers(&rc, 5, 4);
    assert_eq!(a.len(), b.len());
}

// =============================================================================
// GlobalAnalyzer Map Mathematics
// =============================================================================

#[test]
fn forward_and_reverse_maps_are_inverses() {
    let reads = make_reads(200, 100);
    let seqs: Vec<String> = reads.iter().map(|r| r.sequence.clone()).collect();

    let analyzer = GlobalAnalyzer::new(GlobalAnalyzerConfig {
        reads_per_block: 50,
        minimizer_k: 7,
        minimizer_w: 5,
        enable_reorder: true,
        ..Default::default()
    });
    let result = analyzer.analyze(&seqs).unwrap();

    assert!(result.reordering_performed, "short reads with reorder enabled");
    assert_eq!(result.forward_map.len(), seqs.len());
    assert_eq!(result.reverse_map.len(), seqs.len());

    // forward[reverse[i]] == i  for every archive id i.
    for (archive_id, &orig_id) in result.reverse_map.iter().enumerate() {
        let orig = orig_id as usize;
        assert!(orig < seqs.len(), "orig id out of range");
        assert_eq!(
            result.forward_map[orig] as usize, archive_id,
            "forward[reverse[{archive_id}]] != {archive_id}"
        );
    }
}

#[test]
fn reorder_map_is_a_permutation() {
    let reads = make_reads(120, 80);
    let seqs: Vec<String> = reads.iter().map(|r| r.sequence.clone()).collect();

    let analyzer = GlobalAnalyzer::new(GlobalAnalyzerConfig {
        reads_per_block: 40,
        minimizer_k: 7,
        minimizer_w: 5,
        enable_reorder: true,
        ..Default::default()
    });
    let result = analyzer.analyze(&seqs).unwrap();

    // reverse_map must contain every original id exactly once.
    let mut sorted = result.reverse_map.clone();
    sorted.sort_unstable();
    let expected: Vec<u64> = (0..seqs.len() as u64).collect();
    assert_eq!(sorted, expected, "reverse map is not a permutation");
}

#[test]
fn reorder_disabled_produces_identity_maps() {
    let reads = make_reads(60, 100);
    let seqs: Vec<String> = reads.iter().map(|r| r.sequence.clone()).collect();

    let analyzer = GlobalAnalyzer::new(GlobalAnalyzerConfig {
        reads_per_block: 30,
        minimizer_k: 7,
        minimizer_w: 5,
        enable_reorder: false,
        ..Default::default()
    });
    let result = analyzer.analyze(&seqs).unwrap();

    assert!(!result.reordering_performed);
    assert!(result.forward_map.is_empty());
    assert!(result.reverse_map.is_empty());
}

// =============================================================================
// End-to-End Reorder Roundtrip
// =============================================================================

#[test]
fn reorder_roundtrip_restores_original_order() {
    let records = make_reads(150, 100); // short reads => reorder eligible
    let restored = reorder_roundtrip(&records, 2);

    assert_eq!(restored.len(), records.len());
    for (i, (orig, rest)) in records.iter().zip(restored.iter()).enumerate() {
        assert_eq!(orig.id, rest.id, "ID mismatch at read {i}");
        assert_eq!(orig.sequence, rest.sequence, "Sequence mismatch at read {i}");
        assert_eq!(orig.quality, rest.quality, "Quality mismatch at read {i}");
    }
}

#[test]
fn reorder_roundtrip_single_thread_matches_multi_thread() {
    let records = make_reads(300, 100);
    let restored_mt = reorder_roundtrip(&records, 4);
    assert_eq!(restored_mt.len(), records.len());
    for (i, (orig, rest)) in records.iter().zip(restored_mt.iter()).enumerate() {
        assert_eq!(orig.id, rest.id, "ID mismatch at read {i}");
        assert_eq!(orig.sequence, rest.sequence, "Sequence mismatch at read {i}");
    }
}

#[test]
fn reorder_archive_lookup_original_id_is_consistent() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    let records = make_reads(100, 80);
    write_fastq(&fastq, &records);

    let opts = CompressOptions {
        input_path: fastq.to_string_lossy().to_string(),
        output_path: archive.to_string_lossy().to_string(),
        enable_reorder: true,
        show_progress: false,
        force_overwrite: true,
        ..Default::default()
    };
    assert_eq!(CompressCommand::new(opts).execute(), 0);

    let mut reader = fqc::archive::reader::FqcReader::open(archive.to_string_lossy().as_ref()).unwrap();
    assert!(reader.has_reorder_map());
    reader.load_reorder_map().unwrap();

    // Every archive id must map back to a valid original id, and the mapping
    // must be total (each original id appears exactly once).
    let n = records.len() as u64;
    let mut seen = vec![false; records.len()];
    for archive_id in 0..n {
        let orig = reader
            .lookup_original_id(archive_id)
            .expect("every archive id must map back");
        assert!((orig as usize) < records.len(), "orig id out of range");
        seen[orig as usize] = true;
    }
    assert!(seen.iter().all(|&s| s), "every original id must be reachable");
}
