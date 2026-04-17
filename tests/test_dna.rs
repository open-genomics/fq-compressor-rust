// =============================================================================
// fqc-rust - DNA Utility Module Tests
// =============================================================================

use fqc::algo::dna::*;

// =============================================================================
// BASE_TO_INDEX Table
// =============================================================================

#[test]
fn test_base_to_index_upper() {
    assert_eq!(BASE_TO_INDEX[b'A' as usize], 0);
    assert_eq!(BASE_TO_INDEX[b'C' as usize], 1);
    assert_eq!(BASE_TO_INDEX[b'G' as usize], 2);
    assert_eq!(BASE_TO_INDEX[b'T' as usize], 3);
}

#[test]
fn test_base_to_index_lower() {
    assert_eq!(BASE_TO_INDEX[b'a' as usize], 0);
    assert_eq!(BASE_TO_INDEX[b'c' as usize], 1);
    assert_eq!(BASE_TO_INDEX[b'g' as usize], 2);
    assert_eq!(BASE_TO_INDEX[b't' as usize], 3);
}

#[test]
fn test_base_to_index_non_base() {
    // N/n now map to 4 (special marker for N)
    assert_eq!(BASE_TO_INDEX[b'N' as usize], 4);
    assert_eq!(BASE_TO_INDEX[b'n' as usize], 4);
    // Unknown bases (like X) map to 0 (treated as A, but should be validated earlier)
    assert_eq!(BASE_TO_INDEX[b'X' as usize], 0);
    assert_eq!(BASE_TO_INDEX[0], 0);
}

#[test]
fn test_is_valid_base() {
    assert!(is_valid_base(b'A'));
    assert!(is_valid_base(b'C'));
    assert!(is_valid_base(b'G'));
    assert!(is_valid_base(b'T'));
    assert!(is_valid_base(b'N'));
    assert!(is_valid_base(b'a'));
    assert!(is_valid_base(b'c'));
    assert!(is_valid_base(b'g'));
    assert!(is_valid_base(b't'));
    assert!(is_valid_base(b'n'));
    assert!(!is_valid_base(b'X'));
    assert!(!is_valid_base(b'?'));
    assert!(!is_valid_base(0));
}

#[test]
fn test_is_valid_base_strict() {
    // Strict validation excludes N
    assert!(is_valid_base_strict(b'A'));
    assert!(is_valid_base_strict(b'C'));
    assert!(is_valid_base_strict(b'G'));
    assert!(is_valid_base_strict(b'T'));
    assert!(!is_valid_base_strict(b'N'));
    assert!(!is_valid_base_strict(b'X'));
}

#[test]
fn test_count_invalid_bases() {
    assert_eq!(count_invalid_bases(b"ACGT"), 0);
    assert_eq!(count_invalid_bases(b"ACGN"), 0); // N is valid
    assert_eq!(count_invalid_bases(b"ACGX"), 1);
    assert_eq!(count_invalid_bases(b"ACGXYZ"), 3);
}

#[test]
fn test_validate_sequence() {
    assert!(validate_sequence(b"ACGTN").is_ok());
    assert!(validate_sequence(b"ACGX").is_err());
    let err = validate_sequence(b"ACGX");
    assert_eq!(err, Err((3, b'X')));
}

// =============================================================================
// INDEX_TO_BASE Table
// =============================================================================

#[test]
fn test_index_to_base() {
    assert_eq!(INDEX_TO_BASE[0], b'A');
    assert_eq!(INDEX_TO_BASE[1], b'C');
    assert_eq!(INDEX_TO_BASE[2], b'G');
    assert_eq!(INDEX_TO_BASE[3], b'T');
    assert_eq!(INDEX_TO_BASE[4], b'N'); // N is now at index 4
}

// =============================================================================
// COMPLEMENT Table
// =============================================================================

#[test]
fn test_complement_upper() {
    assert_eq!(COMPLEMENT[b'A' as usize], b'T');
    assert_eq!(COMPLEMENT[b'C' as usize], b'G');
    assert_eq!(COMPLEMENT[b'G' as usize], b'C');
    assert_eq!(COMPLEMENT[b'T' as usize], b'A');
    assert_eq!(COMPLEMENT[b'N' as usize], b'N');
}

#[test]
fn test_complement_lower() {
    assert_eq!(COMPLEMENT[b'a' as usize], b't');
    assert_eq!(COMPLEMENT[b'c' as usize], b'g');
    assert_eq!(COMPLEMENT[b'g' as usize], b'c');
    assert_eq!(COMPLEMENT[b't' as usize], b'a');
    assert_eq!(COMPLEMENT[b'n' as usize], b'n');
}

#[test]
fn test_complement_unknown_base() {
    assert_eq!(COMPLEMENT[b'X' as usize], 0);
    assert_eq!(COMPLEMENT[0], 0);
}

// =============================================================================
// reverse_complement
// =============================================================================

#[test]
fn test_reverse_complement_basic() {
    assert_eq!(reverse_complement(b"ACGT"), b"ACGT");
    assert_eq!(reverse_complement(b"AAAA"), b"TTTT");
    assert_eq!(reverse_complement(b"CCCC"), b"GGGG");
}

#[test]
fn test_reverse_complement_asymmetric() {
    assert_eq!(reverse_complement(b"AACG"), b"CGTT");
    assert_eq!(reverse_complement(b"TGCA"), b"TGCA");
}

#[test]
fn test_reverse_complement_with_n() {
    assert_eq!(reverse_complement(b"ANCG"), b"CGNT");
    assert_eq!(reverse_complement(b"NNN"), b"NNN");
}

#[test]
fn test_reverse_complement_unknown_mapped_to_n() {
    let result = reverse_complement(b"AXG");
    assert_eq!(result, b"CNT");
}

#[test]
fn test_reverse_complement_empty() {
    assert_eq!(reverse_complement(b""), b"");
}

#[test]
fn test_reverse_complement_single() {
    assert_eq!(reverse_complement(b"A"), b"T");
    assert_eq!(reverse_complement(b"C"), b"G");
    assert_eq!(reverse_complement(b"G"), b"C");
    assert_eq!(reverse_complement(b"T"), b"A");
}

#[test]
fn test_reverse_complement_involution() {
    // RC(RC(seq)) == seq for valid DNA
    let seq = b"ACGTACGT";
    let rc = reverse_complement(seq);
    let rc_rc = reverse_complement(&rc);
    assert_eq!(&rc_rc, seq);
}

#[test]
fn test_reverse_complement_long_sequence() {
    let seq = b"ACGTACGTACGTACGTACGTACGTACGTACGT";
    let rc = reverse_complement(seq);
    assert_eq!(rc.len(), seq.len());
    // RC(RC(seq)) == seq
    let rc_rc = reverse_complement(&rc);
    assert_eq!(&rc_rc[..], &seq[..]);
}
