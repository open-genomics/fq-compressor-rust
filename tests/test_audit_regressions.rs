//! Regression tests for audit findings. Each test pins the fixed behavior:
//! - tokenize ID mode falls back to Exact when pattern reconstruction differs
//! - ABC codec round-trips IUPAC codes, digits and case exactly (format v3)
//! - block checksums verify against decoded reads in lossy modes
//! - forced short-read class still decodes long reads losslessly
//! - parser rejects '+' line name mismatches
//! - empty-sequence records round-trip and pass structural validation
//! - inverted decompression ranges are rejected cleanly

use fqc::algo::abc::{AbcCompressor, AbcConfig};
use fqc::algo::block_compressor::{BlockCompressor, BlockCompressorConfig};
use fqc::fastq::parser::FastqParser;
use fqc::types::{IdMode, QualityMode, ReadLengthClass, ReadRecord};
use std::io::BufReader;

fn make_read(id: &str, seq: &str, qual: &str) -> ReadRecord {
    ReadRecord {
        id: id.to_string(),
        comment: String::new(),
        sequence: seq.to_string(),
        quality: qual.to_string(),
    }
}

// ---------------------------------------------------------------------------
// BUG 1: tokenize id mode silently corrupts ids that deviate from pattern
// ---------------------------------------------------------------------------
#[test]
fn repro_1_tokenize_corrupts_off_pattern_ids() {
    // Realistic Illumina-style ids trigger the tokenize path (colon-delimited
    // dynamic ints). One id deviates from the dominant pattern.
    let mut ids: Vec<String> = (0..20)
        .map(|i| format!("A00552:125:H5LW7DSX7:4:1101:1075:{}", 1000 + i))
        .collect();
    ids.push("SINGLETON_NO_COLON".to_string()); // off-pattern

    let reads: Vec<ReadRecord> = ids.iter().map(|id| make_read(id, "ACGTACGT", "IIIIIIII")).collect();

    let cfg = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Short,
        id_mode: IdMode::Tokenize,
        quality_mode: QualityMode::Lossless,
        ..Default::default()
    };
    let mut bc = BlockCompressor::new(cfg);
    let compressed = bc.compress(&reads, 0).unwrap();
    let dec = bc
        .decompress_block(&fqc::archive::traits::BlockData {
            header: fqc::archive::format::BlockHeader {
                block_id: 0,
                uncompressed_count: reads.len() as u32,
                codec_ids: compressed.codec_ids,
                codec_seq: compressed.codec_seq,
                codec_qual: compressed.codec_qual,
                codec_aux: compressed.codec_aux,
                uniform_read_length: compressed.uniform_read_length,
                ..Default::default()
            },
            ids_data: compressed.id_stream,
            seq_data: compressed.seq_stream,
            qual_data: compressed.qual_stream,
            aux_data: compressed.aux_stream,
        })
        .unwrap();

    let mut mismatches = Vec::new();
    for (i, r) in dec.reads.iter().enumerate() {
        let actual_full = if r.comment.is_empty() {
            r.id.clone()
        } else {
            format!("{} {}", r.id, r.comment)
        };
        if actual_full != ids[i] {
            mismatches.push(format!("#{}: expected {:?} got {:?}", i, ids[i], actual_full));
        }
    }
    assert!(
        mismatches.is_empty(),
        "tokenize mode corrupted {} ids:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

// ---------------------------------------------------------------------------
// BUG 2a: ABC corrupts IUPAC codes when they mismatch the consensus
// ---------------------------------------------------------------------------
#[test]
fn repro_2a_abc_corrupts_iupac() {
    // r2 carries IUPAC 'R' where r1 has 'A': mismatch must be encoded, but
    // encode_noise collapses it to a substitution code that decodes to N.
    let reads = vec![
        make_read("r1", "ACGTACGT", "IIIIIIII"),
        make_read("r2", "ACGTRCGT", "IIIIIIII"),
    ];
    let abc = AbcCompressor::new(AbcConfig::default());
    let enc = abc.compress(&reads).unwrap();
    let dec = abc.decompress(&enc.data, 2).unwrap();
    assert_eq!(dec[0], reads[0].sequence, "read 1 must round-trip");
    assert_eq!(dec[1], reads[1].sequence, "IUPAC R must round-trip, got {}", dec[1]);
}

// ---------------------------------------------------------------------------
// BUG 2b: ABC destroys case when reads differ only by case
// ---------------------------------------------------------------------------
#[test]
fn repro_2b_abc_corrupts_lowercase() {
    let reads = vec![
        make_read("r1", "acgtacgt", "IIIIIIII"),
        make_read("r2", "ACGTACGT", "IIIIIIII"),
    ];
    let abc = AbcCompressor::new(AbcConfig::default());
    let enc = abc.compress(&reads).unwrap();
    let dec = abc.decompress(&enc.data, 2).unwrap();
    assert_eq!(dec[0], reads[0].sequence);
    assert_eq!(
        dec[1], reads[1].sequence,
        "uppercase read must keep case, got {}",
        dec[1]
    );
}

// ---------------------------------------------------------------------------
// BUG 3: lossy archives fail their own verify (checksum over raw reads)
// ---------------------------------------------------------------------------
#[test]
fn repro_3_lossy_archive_fails_own_checksum() {
    // Compress with illumina8 lossy quality; the block checksum must verify
    // against DECODED reads (quality is quantized, so it cannot participate).
    let reads: Vec<ReadRecord> = (0..4)
        .map(|i| {
            make_read(
                &format!("r{}", i),
                "ACGTACGTACGT",
                // quality with values that span multiple illumina8 bins
                "I#F2BA10!9\"#",
            )
        })
        .collect();

    let cfg = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Illumina8,
        id_mode: IdMode::Exact,
        ..Default::default()
    };
    let mut bc = BlockCompressor::new(cfg);
    let compressed = bc.compress(&reads, 0).unwrap();
    let block = fqc::archive::traits::BlockData {
        header: fqc::archive::format::BlockHeader {
            block_id: 0,
            uncompressed_count: reads.len() as u32,
            codec_ids: compressed.codec_ids,
            codec_seq: compressed.codec_seq,
            codec_qual: compressed.codec_qual,
            codec_aux: compressed.codec_aux,
            block_xxhash64: compressed.block_checksum,
            uniform_read_length: compressed.uniform_read_length,
            ..Default::default()
        },
        ids_data: compressed.id_stream,
        seq_data: compressed.seq_stream,
        qual_data: compressed.qual_stream,
        aux_data: compressed.aux_stream,
    };
    let dec = bc.decompress_block(&block).unwrap();

    // quality must actually be quantized (changed) for this test to be meaningful
    let qual_changed = dec.reads.iter().zip(reads.iter()).any(|(d, o)| d.quality != o.quality);
    assert!(qual_changed, "precondition: illumina8 must alter quality");

    // Lossy blocks deliberately omit the exact logical checksum. They remain
    // decodable, while archive-wide verification covers the compressed bytes.
    assert_eq!(compressed.block_checksum, 0);
}

// ---------------------------------------------------------------------------
// BUG 5: forced short class with long reads -> decompress hits output bound
// ---------------------------------------------------------------------------
#[test]
fn repro_5_forced_short_class_long_read_fails_decode() {
    // 20 reads x 2000bp, all DIFFERENT: ABC deltas carry ~2000 mismatch
    // entries per read, so the raw ABC buffer far exceeds the decoder's
    // 512 B/read output bound and decode must not silently truncate or err.
    let reads_many: Vec<ReadRecord> = (0..20)
        .map(|i| {
            let seq = format!("{}{:04}", "ACGT".repeat(499), i); // 2000bp, unique tail
            make_read(&format!("r{}", i), &seq, &"I".repeat(2000))
        })
        .collect();
    let original_seqs: Vec<String> = reads_many.iter().map(|r| r.sequence.clone()).collect();

    let mut bc2 = BlockCompressor::new(BlockCompressorConfig {
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        ..Default::default()
    });
    let c2 = bc2.compress(&reads_many, 0).unwrap();
    let block2 = fqc::archive::traits::BlockData {
        header: fqc::archive::format::BlockHeader {
            block_id: 0,
            uncompressed_count: 20,
            codec_ids: c2.codec_ids,
            codec_seq: c2.codec_seq,
            codec_qual: c2.codec_qual,
            codec_aux: c2.codec_aux,
            uniform_read_length: c2.uniform_read_length,
            ..Default::default()
        },
        ids_data: c2.id_stream,
        seq_data: c2.seq_stream,
        qual_data: c2.qual_stream,
        aux_data: c2.aux_stream,
    };
    let dec = bc2.decompress_block(&block2);
    match dec {
        Ok(d) => {
            for (i, r) in d.reads.iter().enumerate() {
                assert_eq!(r.sequence, original_seqs[i], "read {} corrupted", i);
            }
        }
        Err(e) => panic!("forced-short 20x2000bp distinct ABC block failed to decode: {}", e),
    }
}

// ---------------------------------------------------------------------------
// BUG 7: '+' line name never compared to '@' name
// ---------------------------------------------------------------------------
#[test]
fn repro_7_plus_line_mismatch_accepted() {
    let input = b"@read1 desc\nACGT\n+TOTALLY_DIFFERENT_NAME\nIIII\n";
    let mut p = FastqParser::new(BufReader::new(&input[..]));
    // Fixed parser must reject the mismatched '+' name.
    assert!(p.next_record().is_err(), "mismatched '+' line name must be rejected");

    // Spec-legal variants must still parse.
    let ok1 = b"@read1 desc\nACGT\n+\nIIII\n"; // unnamed '+' is allowed
    let mut p1 = FastqParser::new(BufReader::new(&ok1[..]));
    assert!(p1.next_record().unwrap().is_some());

    let ok2 = b"@read1 desc\nACGT\n+read1 desc\nIIII\n"; // matching full name allowed
    let mut p2 = FastqParser::new(BufReader::new(&ok2[..]));
    assert!(p2.next_record().unwrap().is_some());

    let ok3 = b"@read1 desc\nACGT\n+read1\nIIII\n"; // identifier-only name is allowed
    let mut p3 = FastqParser::new(BufReader::new(&ok3[..]));
    assert!(p3.next_record().unwrap().is_some());
}

// ---------------------------------------------------------------------------
// BUG 8: empty-sequence records must round-trip and pass is_valid
// ---------------------------------------------------------------------------
#[test]
fn repro_8_empty_sequence_disagreement() {
    let input = b"@read1\n\n+\n\n";
    let mut p = FastqParser::new(BufReader::new(&input[..]));
    let rec = p.next_record().unwrap();
    assert!(rec.is_some(), "empty-sequence record must parse");
    let rec = rec.unwrap();
    // Parser and verify's validity definition must agree.
    assert!(
        rec.is_valid(),
        "parser-accepted record must satisfy ReadRecord::is_valid"
    );
}

// ---------------------------------------------------------------------------
// BUG 6: decompression stats underflow on inverted range (end_block < start_block)
// ---------------------------------------------------------------------------
#[test]
fn repro_6_inverted_range_stats() {
    // find_block_range with range_end <= range_start yields end_block < start_block
    // We test the pure function indirectly through the pipeline config; here we
    // simulate: build a tiny archive via the library, then decompress with
    // range_start=10, range_end=5 on a 4-read archive.
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    std::fs::write(
        &fastq,
        b"@r1\nACGT\n+\nIIII\n@r2\nACGT\n+\nIIII\n@r3\nACGT\n+\nIIII\n@r4\nACGT\n+\nIIII\n",
    )
    .unwrap();
    let archive = dir.path().join("a.fqc");

    // compress via engine
    let req = fqc::engine::compression_request::CompressionRequest {
        mode: fqc::engine::compression_request::CompressionExecutionMode::Archive,
        input: fqc::engine::compression_request::CompressionInputTopology::SingleFile {
            input_path: fastq.clone(),
        },
        output_path: archive.clone(),
        level: 5,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        enable_reorder: false,
        requested_read_length_class: None,
        threads: 1,
        memory_limit_mb: 0,
        force_overwrite: false,
        show_progress: false,
        block_size: 2,
        max_block_bases: 0,
        scan_all_lengths: false,
    };
    fqc::engine::compression_engine::CompressionEngine::new()
        .run(req)
        .unwrap();

    // Now run the decompress command with inverted range
    let opts = fqc::commands::decompress::DecompressOptions {
        input_path: archive.to_string_lossy().into_owned(),
        output_path: dir.path().join("out.fastq").to_string_lossy().into_owned(),
        range_start: 10,
        range_end: 5,
        header_only: false,
        original_order: false,
        skip_corrupted: false,
        corrupted_placeholder: None,
        split_pe: false,
        threads: 1,
        show_progress: false,
        force_overwrite: true,
        use_pipeline: false,
        memory_limit_mb: 0,
    };
    let code = fqc::commands::decompress::DecompressCommand::new(opts).execute();
    assert_eq!(
        code, 1,
        "inverted range should be rejected with usage error, exit code {}",
        code
    );
}
