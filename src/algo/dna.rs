// =============================================================================
// fqc-rust - DNA Utility Module
// =============================================================================
// Shared DNA encoding tables and reverse complement function.
// Used by block_compressor, global_analyzer, and pe_optimizer.
// =============================================================================

// =============================================================================
// DNA Encoding Tables
// =============================================================================

pub const BASE_TO_INDEX: [u8; 256] = {
    let mut t = [0u8; 256];
    t[b'A' as usize] = 0; t[b'a' as usize] = 0;
    t[b'C' as usize] = 1; t[b'c' as usize] = 1;
    t[b'G' as usize] = 2; t[b'g' as usize] = 2;
    t[b'T' as usize] = 3; t[b't' as usize] = 3;
    t
};

pub const INDEX_TO_BASE: [u8; 4] = [b'A', b'C', b'G', b'T'];

pub const COMPLEMENT: [u8; 256] = {
    let mut t = [0u8; 256];
    t[b'A' as usize] = b'T'; t[b'a' as usize] = b't';
    t[b'C' as usize] = b'G'; t[b'c' as usize] = b'g';
    t[b'G' as usize] = b'C'; t[b'g' as usize] = b'c';
    t[b'T' as usize] = b'A'; t[b't' as usize] = b'a';
    t[b'N' as usize] = b'N'; t[b'n' as usize] = b'n';
    t
};

/// Compute the reverse complement of a DNA sequence.
pub fn reverse_complement(seq: &[u8]) -> Vec<u8> {
    seq.iter().rev().map(|&c| {
        let rc = COMPLEMENT[c as usize];
        if rc != 0 { rc } else { b'N' }
    }).collect()
}
