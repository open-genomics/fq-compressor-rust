// =============================================================================
// fqc-rust - Global Analyzer (Minimizer Bucketing + Read Reordering)
// =============================================================================

use crate::algo::dna::BASE_TO_INDEX;
use crate::error::Result;
use crate::types::*;
use rayon::prelude::*;
use std::collections::{HashMap, VecDeque};

// =============================================================================
// Minimizer Extraction
// =============================================================================

#[derive(Debug, Clone)]
pub struct Minimizer {
    pub hash: u64,
}

const MAX_ROLLING_KMER: usize = 32;

fn base_value(byte: u8) -> u64 {
    // The minimizer hash uses a two-bit alphabet. Ambiguous/unknown symbols
    // are mapped deterministically to A instead of allowing the N marker
    // (index 4) to underflow the reverse-complement calculation.
    match BASE_TO_INDEX[byte as usize] {
        index @ 0..=3 => u64::from(index),
        _ => 0,
    }
}

fn compute_kmer_hash(seq: &[u8]) -> u64 {
    let k = seq.len();
    let mut hash: u64 = 0;
    let mut rc_hash: u64 = 0;
    for i in 0..k {
        let base = base_value(seq[i]);
        hash = (hash << 2) | base;
        let rc_base = 3 - base_value(seq[k - 1 - i]);
        rc_hash = (rc_hash << 2) | rc_base;
    }
    hash.min(rc_hash)
}

/// Old linear-window implementation, retained for exotic k values and as a
/// test oracle. The normal short-read path uses `extract_minimizers`.
fn extract_minimizers_legacy(seq: &[u8], k: usize, w: usize) -> Vec<Minimizer> {
    let num_kmers = seq.len() - k + 1;
    let hashes: Vec<u64> = (0..num_kmers).map(|i| compute_kmer_hash(&seq[i..i + k])).collect();

    let window_size = w.min(num_kmers);
    let mut prev_min_pos = usize::MAX;
    let mut minimizers = Vec::new();

    for window_start in 0..=(num_kmers.saturating_sub(window_size)) {
        let mut min_hash = u64::MAX;
        let mut min_pos = 0;

        for i in 0..window_size {
            let pos = window_start + i;
            if hashes[pos] < min_hash {
                min_hash = hashes[pos];
                min_pos = pos;
            }
        }

        if min_pos != prev_min_pos {
            minimizers.push(Minimizer { hash: min_hash });
            prev_min_pos = min_pos;
        }
    }

    minimizers
}

pub fn extract_minimizers(seq: &[u8], k: usize, w: usize) -> Vec<Minimizer> {
    if k == 0 || w == 0 || seq.len() < k {
        return Vec::new();
    }

    // k > 32 would make the 2-bit rolling state wider than u64. Keep the
    // historical behavior for those unusual parameters instead of silently
    // changing hash boundaries.
    if k > MAX_ROLLING_KMER {
        return extract_minimizers_legacy(seq, k, w);
    }

    let num_kmers = seq.len() - k + 1;
    let window_size = w.min(num_kmers);
    let mask = if k == MAX_ROLLING_KMER {
        u64::MAX
    } else {
        (1_u64 << (2 * k)) - 1
    };
    let high_shift = 2 * (k - 1);

    let mut forward: u64 = 0;
    let mut reverse_complement: u64 = 0;
    for &byte in &seq[..k] {
        forward = (forward << 2) | base_value(byte);
    }
    for &byte in seq[..k].iter().rev() {
        reverse_complement = (reverse_complement << 2) | (3 - base_value(byte));
    }

    let mut minimizers = Vec::new();
    let mut window_minima = VecDeque::with_capacity(window_size);
    let mut prev_min_pos = usize::MAX;

    for i in 0..num_kmers {
        let hash = forward.min(reverse_complement);

        // Strict `>` keeps the leftmost position when hashes tie, matching the
        // legacy scan's `<` comparison.
        while let Some(&(_, back_hash)) = window_minima.back() {
            if back_hash > hash {
                window_minima.pop_back();
            } else {
                break;
            }
        }
        window_minima.push_back((i, hash));

        while let Some(&(front_pos, _)) = window_minima.front() {
            if front_pos + window_size > i {
                break;
            }
            window_minima.pop_front();
        }

        if i + 1 >= window_size {
            let &(min_pos, min_hash) = window_minima.front().expect("a non-empty window must have a minimum");
            if min_pos != prev_min_pos {
                minimizers.push(Minimizer { hash: min_hash });
                prev_min_pos = min_pos;
            }
        }

        if i + 1 < num_kmers {
            let next_base = base_value(seq[i + k]);
            forward = ((forward << 2) & mask) | next_base;
            reverse_complement = (reverse_complement >> 2) | ((3 - next_base) << high_shift);
        }
    }

    minimizers
}

// =============================================================================
// GlobalAnalyzer Configuration
// =============================================================================

#[derive(Debug, Clone)]
pub struct GlobalAnalyzerConfig {
    pub reads_per_block: usize,
    pub minimizer_k: usize,
    pub minimizer_w: usize,
    pub enable_reorder: bool,
    pub max_search_reorder: usize,
    pub read_length_class: Option<ReadLengthClass>,
}

impl Default for GlobalAnalyzerConfig {
    fn default() -> Self {
        Self {
            reads_per_block: DEFAULT_BLOCK_SIZE_SHORT,
            minimizer_k: 15,
            minimizer_w: 10,
            enable_reorder: true,
            max_search_reorder: 64,
            read_length_class: None,
        }
    }
}

// =============================================================================
// Block Boundary
// =============================================================================

#[derive(Debug, Clone)]
pub struct BlockBoundary {
    pub block_id: BlockId,
    pub archive_id_start: ReadId,
    pub archive_id_end: ReadId,
}

// =============================================================================
// GlobalAnalysisResult
// =============================================================================

#[derive(Debug, Default)]
pub struct GlobalAnalysisResult {
    pub max_read_length: usize,
    pub length_class: ReadLengthClass,
    pub reordering_performed: bool,
    pub forward_map: Vec<ReadId>,
    pub reverse_map: Vec<ReadId>,
    pub block_boundaries: Vec<BlockBoundary>,
    pub num_blocks: usize,
}

// =============================================================================
// GlobalAnalyzer
// =============================================================================

pub struct GlobalAnalyzer {
    config: GlobalAnalyzerConfig,
}

impl GlobalAnalyzer {
    pub fn new(config: GlobalAnalyzerConfig) -> Self {
        Self { config }
    }

    pub fn analyze(&self, sequences: &[String]) -> Result<GlobalAnalysisResult> {
        let total_reads = sequences.len() as u64;

        let mut result = GlobalAnalysisResult::default();

        if sequences.is_empty() {
            return Ok(result);
        }

        // Compute length statistics
        let lengths: Vec<usize> = sequences.iter().map(|s| s.len()).collect();
        let stats = LengthStats::from_lengths(&lengths);

        result.max_read_length = stats.max_length;
        result.length_class = if let Some(lc) = self.config.read_length_class {
            lc
        } else {
            classify_read_length(stats.median_length, stats.max_length)
        };

        let should_reorder = self.config.enable_reorder && result.length_class == ReadLengthClass::Short;

        if should_reorder {
            log::info!("Performing global reordering on {} reads", total_reads);
            let reorder_map = self.perform_reordering(sequences)?;
            result.reverse_map.clone_from(&reorder_map);
            result.forward_map = self.build_forward_map(&reorder_map);
            result.reordering_performed = true;
        } else {
            log::info!("Skipping reordering (class: {})", result.length_class.as_str());
            result.reordering_performed = false;
        }

        // Compute block boundaries
        let effective_block_size = self.config.reads_per_block.max(1);
        result.block_boundaries = self.compute_block_boundaries(total_reads, effective_block_size);
        result.num_blocks = result.block_boundaries.len();

        log::info!(
            "Created {} blocks with {} reads per block",
            result.num_blocks,
            effective_block_size
        );

        Ok(result)
    }

    fn build_forward_map(&self, reverse_map: &[ReadId]) -> Vec<ReadId> {
        let n = reverse_map.len();
        let mut forward = vec![0u64; n];
        for (archive_id, &orig_id) in reverse_map.iter().enumerate() {
            if (orig_id as usize) < n {
                forward[orig_id as usize] = archive_id as ReadId;
            }
        }
        forward
    }

    fn perform_reordering(&self, sequences: &[String]) -> Result<Vec<ReadId>> {
        let total_reads = sequences.len();

        // Step 1: Extract minimizers in parallel
        let all_buckets: Vec<Vec<(u64, u64)>> = sequences
            .par_iter()
            .enumerate()
            .map(|(i, seq)| {
                let mins = extract_minimizers(seq.as_bytes(), self.config.minimizer_k, self.config.minimizer_w);
                mins.into_iter().map(|m| (m.hash, i as u64)).collect()
            })
            .collect();

        // Build minimizer -> read ID index
        let mut bucket_map: HashMap<u64, Vec<u64>> = HashMap::new();
        for entries in &all_buckets {
            for &(hash, read_id) in entries {
                bucket_map.entry(hash).or_default().push(read_id);
            }
        }

        // Step 2: Greedy reordering (approximate Hamiltonian path).
        // Reuse the minimizers from step 1; recomputing them for every visited
        // read doubles the scalar extraction work in the reorder hot path.
        let mut used = vec![false; total_reads];
        let mut ordered: Vec<ReadId> = Vec::with_capacity(total_reads);

        ordered.push(0);
        used[0] = true;

        while ordered.len() < total_reads {
            let last_read = *ordered.last().expect("ordered is never empty - initialized with 0") as usize;
            let last_len = sequences[last_read].len();

            let mut best_match: Option<u64> = None;
            let mut best_score = usize::MAX;

            let mut searched = 0;

            'outer: for &(hash, _) in &all_buckets[last_read] {
                if let Some(bucket) = bucket_map.get(&hash) {
                    for &candidate_id in bucket {
                        let cid = candidate_id as usize;
                        if used[cid] {
                            continue;
                        }

                        let clen = sequences[cid].len();
                        let len_diff = last_len.abs_diff(clen);

                        if len_diff < best_score {
                            best_score = len_diff;
                            best_match = Some(candidate_id);
                        }

                        searched += 1;
                        if searched >= self.config.max_search_reorder {
                            break 'outer;
                        }
                    }
                }
            }

            let next = if let Some(m) = best_match {
                m
            } else {
                // Find first unused
                (0..total_reads as u64).find(|&i| !used[i as usize]).unwrap_or(0)
            };

            ordered.push(next);
            used[next as usize] = true;
        }

        Ok(ordered)
    }

    fn compute_block_boundaries(&self, total_reads: u64, reads_per_block: usize) -> Vec<BlockBoundary> {
        if total_reads == 0 {
            return Vec::new();
        }

        let num_blocks = (total_reads as usize).div_ceil(reads_per_block);
        let mut boundaries = Vec::with_capacity(num_blocks);

        for block_id in 0..num_blocks {
            let start = block_id as u64 * reads_per_block as u64;
            let end = (start + reads_per_block as u64).min(total_reads);
            boundaries.push(BlockBoundary {
                block_id: block_id as BlockId,
                archive_id_start: start,
                archive_id_end: end,
            });
        }

        boundaries
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_minimizers, extract_minimizers_legacy};

    fn sample_sequence(len: usize, seed: u64) -> Vec<u8> {
        let alphabet = b"ACGTNacgt.-";
        let mut state = seed | 1;
        (0..len)
            .map(|_| {
                state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                alphabet[((state >> 33) as usize) % alphabet.len()]
            })
            .collect()
    }

    fn hashes(seq: &[u8], k: usize, w: usize) -> Vec<u64> {
        extract_minimizers(seq, k, w)
            .into_iter()
            .map(|minimizer| minimizer.hash)
            .collect()
    }

    fn legacy_hashes(seq: &[u8], k: usize, w: usize) -> Vec<u64> {
        extract_minimizers_legacy(seq, k, w)
            .into_iter()
            .map(|minimizer| minimizer.hash)
            .collect()
    }

    #[test]
    fn rolling_minimizers_match_legacy_scan() {
        for k in 1..=40 {
            for len in [k, k + 1, k + 8, 64, 257] {
                let seq = sample_sequence(len, k as u64 * 1_000 + len as u64);
                for w in [1, 2, 5, 17, 64, 257] {
                    assert_eq!(
                        hashes(&seq, k, w),
                        legacy_hashes(&seq, k, w),
                        "fast minimizer scan diverged for k={k}, len={len}, w={w}"
                    );
                }
            }
        }
    }

    #[test]
    fn rolling_minimizers_preserve_ambiguous_base_mapping() {
        let seq = b"ACGTNNacgt..ACGT".repeat(5);
        for k in [1, 3, 15, 32, 33] {
            for w in [1, 2, 10] {
                assert_eq!(
                    hashes(&seq, k, w),
                    legacy_hashes(&seq, k, w),
                    "ambiguous-base minimizers diverged for k={k}, w={w}"
                );
            }
        }
    }
}
