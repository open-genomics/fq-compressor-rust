// =============================================================================
// fqc-rust - Parser Unit Tests
// =============================================================================

use fqc::fastq::parser::*;
use std::io::BufReader;

fn make_fastq_data(records: &[(&str, &str, &str)]) -> Vec<u8> {
    let mut data = Vec::new();
    for (id, seq, qual) in records {
        data.extend_from_slice(format!("@{}\n{}\n+\n{}\n", id, seq, qual).as_bytes());
    }
    data
}

// =============================================================================
// Basic Parsing
// =============================================================================

#[test]
fn test_parse_single_record() {
    let data = make_fastq_data(&[("read1", "ACGT", "IIII")]);
    let reader = BufReader::new(data.as_slice());
    let mut parser = FastqParser::new(reader);

    let rec = parser.next_record().unwrap().unwrap();
    assert_eq!(rec.id, "read1");
    assert_eq!(rec.sequence, "ACGT");
    assert_eq!(rec.quality, "IIII");

    assert!(parser.next_record().unwrap().is_none());
}

#[test]
fn test_parse_multiple_records() {
    let data = make_fastq_data(&[("r1", "AAAA", "!!!!"), ("r2", "CCCC", "IIII"), ("r3", "GGGG", "~~~~")]);
    let reader = BufReader::new(data.as_slice());
    let mut parser = FastqParser::new(reader);

    let all = parser.collect_all().unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].id, "r1");
    assert_eq!(all[2].sequence, "GGGG");
}

#[test]
fn test_parse_empty_input() {
    let data = Vec::new();
    let reader = BufReader::new(data.as_slice());
    let mut parser = FastqParser::new(reader);
    assert!(parser.next_record().unwrap().is_none());
}

// =============================================================================
// Chunk Reading
// =============================================================================

#[test]
fn test_read_chunk() {
    let data = make_fastq_data(&[
        ("r1", "AAAA", "!!!!"),
        ("r2", "CCCC", "IIII"),
        ("r3", "GGGG", "~~~~"),
        ("r4", "TTTT", "JJJJ"),
        ("r5", "ACGT", "ABCD"),
    ]);
    let reader = BufReader::new(data.as_slice());
    let mut parser = FastqParser::new(reader);

    let chunk1 = parser.read_chunk(2).unwrap();
    assert_eq!(chunk1.len(), 2);
    assert_eq!(chunk1[0].id, "r1");
    assert_eq!(chunk1[1].id, "r2");

    let chunk2 = parser.read_chunk(2).unwrap();
    assert_eq!(chunk2.len(), 2);
    assert_eq!(chunk2[0].id, "r3");

    let chunk3 = parser.read_chunk(2).unwrap();
    assert_eq!(chunk3.len(), 1);
    assert_eq!(chunk3[0].id, "r5");

    let chunk4 = parser.read_chunk(2).unwrap();
    assert!(chunk4.is_empty());
}

// =============================================================================
// Line Number & Record Number Tracking
// =============================================================================

#[test]
fn test_line_and_record_tracking() {
    let data = make_fastq_data(&[("r1", "AAAA", "!!!!"), ("r2", "CCCC", "IIII")]);
    let reader = BufReader::new(data.as_slice());
    let mut parser = FastqParser::new(reader);

    assert_eq!(parser.line_number(), 0);
    assert_eq!(parser.record_number(), 0);

    parser.next_record().unwrap();
    assert_eq!(parser.line_number(), 4);
    assert_eq!(parser.record_number(), 1);

    parser.next_record().unwrap();
    assert_eq!(parser.line_number(), 8);
    assert_eq!(parser.record_number(), 2);
}

// =============================================================================
// Statistics Collection
// =============================================================================

#[test]
fn test_parser_stats() {
    let data = make_fastq_data(&[
        ("r1", "ACGTN", "!!!!!"),
        ("r2", "NNNN", "!!!!"),
        ("r3", "ACGTACGT", "!!!!!!!!"),
    ]);
    let reader = BufReader::new(data.as_slice());
    let opts = ParserOptions {
        collect_stats: true,
        ..Default::default()
    };
    let mut parser = FastqParser::with_options(reader, opts);
    parser.collect_all().unwrap();

    let stats = parser.stats();
    assert_eq!(stats.total_records, 3);
    assert_eq!(stats.total_bases, 17); // 5+4+8
    assert_eq!(stats.min_length, 4);
    assert_eq!(stats.max_length, 8);
    assert_eq!(stats.total_n_count, 5); // 1 in r1 + 4 in r2
    assert!(stats.total_bytes_read > 0);
    assert!((stats.avg_length() - 5.666).abs() < 0.01);
}

// =============================================================================
// Validation
// =============================================================================

#[test]
fn test_sequence_validation_pass() {
    assert!(validate_sequence("ACGTNacgtn").is_ok());
}

#[test]
fn test_sequence_validation_fail() {
    assert!(validate_sequence("ACGTX").is_err());
    assert!(validate_sequence("ACGT1").is_err());
}

#[test]
fn test_quality_validation_pass() {
    assert!(validate_quality_string("!!IIIJ~~~~").is_ok());
}

#[test]
fn test_quality_validation_fail() {
    let mut bad = String::new();
    bad.push(20 as char); // below 33
    assert!(validate_quality_string(&bad).is_err());
}

#[test]
fn test_parser_with_validation() {
    let data = make_fastq_data(&[("r1", "ACXGT", "!!!!!")]);
    let reader = BufReader::new(data.as_slice());
    let opts = ParserOptions {
        validate_sequence: true,
        ..Default::default()
    };
    let mut parser = FastqParser::with_options(reader, opts);
    let result = parser.next_record();
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Invalid base"));
}

// =============================================================================
// Error Cases
// =============================================================================

#[test]
fn test_parse_missing_at_sign() {
    let data = b"read1\nACGT\n+\nIIII\n";
    let reader = BufReader::new(data.as_slice());
    let mut parser = FastqParser::new(reader);
    assert!(parser.next_record().is_err());
}

#[test]
fn test_parse_truncated_record() {
    let data = b"@read1\nACGT\n";
    let reader = BufReader::new(data.as_slice());
    let mut parser = FastqParser::new(reader);
    assert!(parser.next_record().is_err());
}

#[test]
fn test_parse_length_mismatch() {
    let data = b"@read1\nACGT\n+\nII\n";
    let reader = BufReader::new(data.as_slice());
    let mut parser = FastqParser::new(reader);
    assert!(parser.next_record().is_err());
}

// =============================================================================
// for_each callback
// =============================================================================

#[test]
fn test_for_each() {
    let data = make_fastq_data(&[("r1", "AAAA", "!!!!"), ("r2", "CCCC", "IIII")]);
    let reader = BufReader::new(data.as_slice());
    let mut parser = FastqParser::new(reader);

    let mut ids = Vec::new();
    let count = parser
        .for_each(|r| {
            ids.push(r.id.clone());
            Ok(())
        })
        .unwrap();

    assert_eq!(count, 2);
    assert_eq!(ids, vec!["r1", "r2"]);
}

// =============================================================================
// PE ID Validation
// =============================================================================

#[test]
fn test_pe_id_validation_identical() {
    assert!(validate_pe_pair_ids("read1", "read1"));
}

#[test]
fn test_pe_id_validation_slash_convention() {
    assert!(validate_pe_pair_ids("read1/1", "read1/2"));
    assert!(!validate_pe_pair_ids("read1/1", "read2/2"));
    assert!(!validate_pe_pair_ids("read1/2", "read1/1")); // wrong order
}

#[test]
fn test_pe_id_validation_illumina_convention() {
    assert!(validate_pe_pair_ids(
        "HWUSI:1:1101:1234:5678 1:N:0:ATCACG",
        "HWUSI:1:1101:1234:5678 2:N:0:ATCACG"
    ));
    assert!(!validate_pe_pair_ids(
        "HWUSI:1:1101:1234:5678 1:N:0:ATCACG",
        "HWUSI:1:1101:9999:5678 2:N:0:ATCACG"
    ));
}

#[test]
fn test_pe_id_validation_different() {
    assert!(!validate_pe_pair_ids("read1", "read2"));
}
