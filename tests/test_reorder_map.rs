// =============================================================================
// fqc-rust - ReorderMap Unit Tests
// =============================================================================

use fqc::reorder_map::*;

// =============================================================================
// ZigZag Varint
// =============================================================================

#[test]
fn test_signed_varint_positive() {
    let encoded = encode_signed_varint(42);
    let (decoded, consumed) = decode_signed_varint(&encoded).unwrap();
    assert_eq!(decoded, 42);
    assert!(consumed > 0);
}

#[test]
fn test_signed_varint_negative() {
    let encoded = encode_signed_varint(-100);
    let (decoded, _) = decode_signed_varint(&encoded).unwrap();
    assert_eq!(decoded, -100);
}

#[test]
fn test_signed_varint_zero() {
    let encoded = encode_signed_varint(0);
    let (decoded, consumed) = decode_signed_varint(&encoded).unwrap();
    assert_eq!(decoded, 0);
    assert_eq!(consumed, 1);
}

#[test]
fn test_signed_varint_large() {
    for val in [i64::MAX, i64::MIN, 1_000_000, -1_000_000, 1, -1] {
        let encoded = encode_signed_varint(val);
        let (decoded, _) = decode_signed_varint(&encoded).unwrap();
        assert_eq!(decoded, val, "Failed for value {}", val);
    }
}

// =============================================================================
// ReorderMapData Construction
// =============================================================================

#[test]
fn test_identity_map() {
    let map = ReorderMapData::identity(10);
    assert_eq!(map.total_reads(), 10);
    assert!(map.is_valid());
    for i in 0..10u64 {
        assert_eq!(map.get_archive_id(i), i);
        assert_eq!(map.get_original_id(i), i);
    }
}

#[test]
fn test_from_reverse_map() {
    // Reverse order: archive[0]=orig[4], archive[1]=orig[3], ...
    let reverse: Vec<u64> = vec![4, 3, 2, 1, 0];
    let map = ReorderMapData::from_reverse_map(reverse);
    assert_eq!(map.total_reads(), 5);
    assert!(map.is_valid());
    assert_eq!(map.get_archive_id(0), 4); // orig 0 -> archive 4
    assert_eq!(map.get_archive_id(4), 0); // orig 4 -> archive 0
    assert_eq!(map.get_original_id(0), 4); // archive 0 -> orig 4
}

#[test]
fn test_custom_permutation() {
    let forward: Vec<u64> = vec![2, 0, 1]; // orig[0]->arch[2], orig[1]->arch[0], orig[2]->arch[1]
    let reverse: Vec<u64> = vec![1, 2, 0]; // arch[0]->orig[1], arch[1]->orig[2], arch[2]->orig[0]
    let map = ReorderMapData::new(forward, reverse);
    assert!(map.is_valid());
    assert_eq!(map.get_archive_id(0), 2);
    assert_eq!(map.get_original_id(0), 1);
}

// =============================================================================
// Validation
// =============================================================================

#[test]
fn test_valid_map() {
    let map = ReorderMapData::identity(100);
    assert!(map.is_valid());
}

#[test]
fn test_invalid_map_size_mismatch() {
    let map = ReorderMapData::new(vec![0, 1], vec![0]);
    assert!(!map.is_valid());
}

#[test]
fn test_invalid_map_broken_inverse() {
    let map = ReorderMapData::new(vec![0, 1, 2], vec![1, 0, 2]);
    // forward says orig[0]->arch[0], but reverse says arch[0]->orig[1]
    assert!(!map.is_valid());
}

#[test]
fn test_verify_map_consistency_ok() {
    let fwd: Vec<u64> = vec![2, 0, 1];
    let rev: Vec<u64> = vec![1, 2, 0];
    assert!(verify_map_consistency(&fwd, &rev).is_ok());
}

#[test]
fn test_verify_map_consistency_fail() {
    let fwd: Vec<u64> = vec![0, 1, 2];
    let rev: Vec<u64> = vec![1, 0, 2]; // inconsistent
    assert!(verify_map_consistency(&fwd, &rev).is_err());
}

#[test]
fn test_validate_permutation_ok() {
    let map: Vec<u64> = vec![3, 1, 0, 2];
    assert!(validate_permutation(&map).is_ok());
}

#[test]
fn test_validate_permutation_duplicate() {
    let map: Vec<u64> = vec![0, 1, 1, 2];
    assert!(validate_permutation(&map).is_err());
}

#[test]
fn test_validate_permutation_out_of_range() {
    let map: Vec<u64> = vec![0, 1, 99];
    assert!(validate_permutation(&map).is_err());
}

// =============================================================================
// Serialization / Deserialization
// =============================================================================

#[test]
fn test_serialize_deserialize_identity() {
    let map = ReorderMapData::identity(100);
    let bytes = map.serialize().unwrap();
    let restored = ReorderMapData::deserialize(&bytes).unwrap();

    assert_eq!(restored.total_reads(), 100);
    assert!(restored.is_valid());
    for i in 0..100u64 {
        assert_eq!(restored.get_archive_id(i), i);
        assert_eq!(restored.get_original_id(i), i);
    }
}

#[test]
fn test_serialize_deserialize_permutation() {
    let reverse: Vec<u64> = vec![9, 7, 5, 3, 1, 0, 2, 4, 6, 8];
    let map = ReorderMapData::from_reverse_map(reverse.clone());
    assert!(map.is_valid());

    let bytes = map.serialize().unwrap();
    let restored = ReorderMapData::deserialize(&bytes).unwrap();

    assert!(restored.is_valid());
    assert_eq!(restored.total_reads(), 10);
    for i in 0..10u64 {
        assert_eq!(restored.get_archive_id(i), map.get_archive_id(i));
        assert_eq!(restored.get_original_id(i), map.get_original_id(i));
    }
}

#[test]
fn test_serialize_deserialize_large() {
    let n = 10_000;
    let reverse: Vec<u64> = (0..n as u64).rev().collect();
    let map = ReorderMapData::from_reverse_map(reverse);
    assert!(map.is_valid());

    let bytes = map.serialize().unwrap();
    let restored = ReorderMapData::deserialize(&bytes).unwrap();
    assert!(restored.is_valid());
    assert_eq!(restored.total_reads(), n as u64);

    for i in 0..n as u64 {
        assert_eq!(restored.get_archive_id(i), map.get_archive_id(i));
    }
}

// =============================================================================
// Chunk Concatenation
// =============================================================================

#[test]
fn test_append_chunk() {
    let mut combined = ReorderMapData::identity(5);
    let chunk2 = ReorderMapData::identity(3);

    combined.append_chunk(&chunk2, 5, 5);
    assert_eq!(combined.forward_map().len(), 8);
    assert_eq!(combined.reverse_map().len(), 8);

    // First 5 entries are identity
    for i in 0..5u64 {
        assert_eq!(combined.get_archive_id(i), i);
    }
    // Next 3 entries are offset by 5
    assert_eq!(combined.forward_map()[5], 5);
    assert_eq!(combined.forward_map()[6], 6);
    assert_eq!(combined.forward_map()[7], 7);
}

#[test]
fn test_combine_chunks() {
    let chunk1 = ReorderMapData::identity(3);
    let chunk2 = ReorderMapData::identity(4);
    let chunk3 = ReorderMapData::identity(2);

    let combined = ReorderMapData::combine_chunks(&[chunk1, chunk2, chunk3], &[3, 4, 2]);
    assert_eq!(combined.forward_map().len(), 9);
    assert_eq!(combined.reverse_map().len(), 9);
}

// =============================================================================
// Compression Stats
// =============================================================================

#[test]
fn test_compression_stats() {
    let map = ReorderMapData::identity(1000);
    let stats = map.compression_stats().unwrap();
    assert_eq!(stats.total_reads, 1000);
    assert!(stats.total_compressed_size > 0);
    assert!(stats.bytes_per_read > 0.0);
    assert!(stats.compression_ratio > 0.0);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_map() {
    let map = ReorderMapData::identity(0);
    assert!(map.is_empty());
    assert!(map.is_valid());

    let bytes = map.serialize().unwrap();
    let restored = ReorderMapData::deserialize(&bytes).unwrap();
    assert!(restored.is_empty());
}

#[test]
fn test_single_element_map() {
    let map = ReorderMapData::identity(1);
    assert!(map.is_valid());
    assert_eq!(map.get_archive_id(0), 0);

    let bytes = map.serialize().unwrap();
    let restored = ReorderMapData::deserialize(&bytes).unwrap();
    assert!(restored.is_valid());
    assert_eq!(restored.total_reads(), 1);
}
