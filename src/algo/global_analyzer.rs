// =============================================================================
// fqc-rust - Global Analyzer (Minimizer Bucketing + Read Reordering)
// =============================================================================

use crate::dna::BASE_TO_INDEX;
use crate::error::{FqcError, Result};
use crate::plan::ReorderMode;
use crate::types::*;
use rayon::prelude::*;
use std::collections::{BTreeSet, HashMap, VecDeque};

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

const ORDER_DETECTION_WINDOWS: usize = 8;
const ORDER_DETECTION_READS_PER_WINDOW: usize = 256;
const ORDER_DETECTION_KMER: usize = 15;
const ORDER_DETECTION_ADJACENT_FRACTION: f64 = 0.25;
/// A low adjacent-overlap score alone does not make reordering worthwhile.
/// Random reads with no shared sequence structure also have low adjacency.
/// Require a minimum fraction of sampled reads to share at least one k-mer
/// before paying for the global reorder pass.
const ORDER_DETECTION_GLOBAL_FRACTION: f64 = 0.25;

fn sorted_kmers(seq: &[u8], k: usize) -> Vec<u64> {
    if seq.len() < k {
        return Vec::new();
    }
    let mut hashes: Vec<u64> = (0..=seq.len() - k)
        .map(|start| compute_kmer_hash(&seq[start..start + k]))
        .collect();
    hashes.sort_unstable();
    hashes.dedup();
    hashes
}

fn has_shared_kmer(left: &[u64], right: &[u64]) -> bool {
    let (mut i, mut j) = (0usize, 0usize);
    while i < left.len() && j < right.len() {
        match left[i].cmp(&right[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => return true,
        }
    }
    false
}

/// Return the contiguous windows used by order detection.
fn sample_windows(total: usize) -> Vec<(usize, usize)> {
    if total < 4 {
        return Vec::new();
    }

    let windows = ORDER_DETECTION_WINDOWS.min(total / 2).max(1);
    let window_len = ORDER_DETECTION_READS_PER_WINDOW.min(total / windows).max(2);
    (0..windows)
        .map(|window| {
            let max_start = total.saturating_sub(window_len);
            let start = if windows == 1 {
                0
            } else {
                window * max_start / (windows - 1)
            };
            (start, window_len)
        })
        .collect()
}

fn sampled_indices(total: usize) -> Vec<usize> {
    let mut indices: Vec<usize> = sample_windows(total)
        .into_iter()
        .flat_map(|(start, len)| start..start + len)
        .collect();
    indices.sort_unstable();
    indices.dedup();
    indices
}

/// Estimate whether consecutive reads already share sequence locality.
///
/// Coordinate-sorted FASTQ from a sequencing pipeline has many adjacent
/// overlapping reads. Sampling a few contiguous windows separates that case
/// from a shuffled stream by orders of magnitude without a full minimizer
/// index. Returns `None` when the input is too small to judge.
fn adjacent_shared_fraction(reads: &[ReadRecord]) -> Option<f64> {
    let mut pairs = 0usize;
    let mut shared_pairs = 0usize;

    for (start, window_len) in sample_windows(reads.len()) {
        let mut previous = sorted_kmers(reads[start].sequence.as_bytes(), ORDER_DETECTION_KMER);
        for offset in 1..window_len {
            let current = sorted_kmers(reads[start + offset].sequence.as_bytes(), ORDER_DETECTION_KMER);
            if has_shared_kmer(&previous, &current) {
                shared_pairs += 1;
            }
            pairs += 1;
            previous = current;
        }
    }

    if pairs == 0 {
        None
    } else {
        Some(shared_pairs as f64 / pairs as f64)
    }
}

/// Estimate whether the sample has enough shared k-mer structure for
/// reordering to have any chance of helping.
///
/// This is deliberately separate from the adjacent-overlap test: a shuffled
/// WXS stream has low adjacent overlap but high global sharing, while a stream
/// of independent random reads has both scores near zero. Only the former
/// should pay for global reordering.
fn global_shared_fraction(reads: &[ReadRecord]) -> Option<f64> {
    let indices = sampled_indices(reads.len());
    if indices.len() < 2 {
        return None;
    }

    let mut first_owner: HashMap<u64, u32> = HashMap::new();
    let mut linked = vec![false; indices.len()];
    for (local_index, &read_index) in indices.iter().enumerate() {
        for hash in sorted_kmers(reads[read_index].sequence.as_bytes(), ORDER_DETECTION_KMER) {
            match first_owner.get(&hash).copied() {
                Some(previous) => {
                    let previous = previous as usize;
                    if previous != local_index {
                        linked[previous] = true;
                        linked[local_index] = true;
                    }
                }
                None => {
                    first_owner.insert(hash, local_index as u32);
                }
            }
        }
    }

    let linked_count = linked.into_iter().filter(|linked| *linked).count();
    Some(linked_count as f64 / indices.len() as f64)
}

// =============================================================================
// GlobalAnalyzer Configuration
// =============================================================================

#[derive(Debug, Clone)]
pub struct GlobalAnalyzerConfig {
    pub reads_per_block: usize,
    pub minimizer_k: usize,
    pub minimizer_w: usize,
    /// Reordering policy. `Always` skips the auto-detection check.
    pub reorder: ReorderMode,
    pub max_search_reorder: usize,
    pub read_length_class: Option<ReadLengthClass>,
}

impl Default for GlobalAnalyzerConfig {
    fn default() -> Self {
        Self {
            reads_per_block: DEFAULT_BLOCK_SIZE_SHORT,
            minimizer_k: 15,
            minimizer_w: 10,
            reorder: ReorderMode::Always,
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

    pub fn analyze(&self, reads: &[ReadRecord]) -> Result<GlobalAnalysisResult> {
        let total_reads = reads.len() as u64;

        let mut result = GlobalAnalysisResult::default();

        if reads.is_empty() {
            return Ok(result);
        }

        // When the caller already classified the input, avoid a full lengths
        // vector and its sort on the hot archive/pipeline path.
        result.length_class = if let Some(length_class) = self.config.read_length_class {
            length_class
        } else {
            let lengths: Vec<usize> = reads.iter().map(|read| read.sequence.len()).collect();
            let stats = LengthStats::from_lengths(&lengths);
            classify_read_length(stats.median_length, stats.max_length)
        };

        let mut should_reorder = self.config.reorder.enabled() && result.length_class == ReadLengthClass::Short;

        if should_reorder && self.config.reorder.auto() {
            if let Some(adjacent_fraction) = adjacent_shared_fraction(reads) {
                if adjacent_fraction >= ORDER_DETECTION_ADJACENT_FRACTION {
                    log::info!(
                        "Reorder auto: adjacent overlap {:.1}%; input already spatially ordered, skipping reorder",
                        adjacent_fraction * 100.0
                    );
                    should_reorder = false;
                } else if let Some(global_fraction) = global_shared_fraction(reads) {
                    if global_fraction >= ORDER_DETECTION_GLOBAL_FRACTION {
                        log::info!(
                            "Reorder auto: adjacent overlap {:.3}%, global shared reads {:.1}%; input looks unordered with reorderable redundancy, enabling reorder",
                            adjacent_fraction * 100.0,
                            global_fraction * 100.0
                        );
                    } else {
                        log::info!(
                            "Reorder auto: adjacent overlap {:.3}%, global shared reads {:.1}%; too little shared sequence structure, skipping reorder",
                            adjacent_fraction * 100.0,
                            global_fraction * 100.0
                        );
                        should_reorder = false;
                    }
                }
            }
        }

        if should_reorder {
            log::info!("Performing global reordering on {} reads", total_reads);
            let reorder_map = self.perform_reordering(reads)?;
            result.forward_map = self.build_forward_map(&reorder_map);
            result.reverse_map = reorder_map;
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

    fn perform_reordering(&self, reads: &[ReadRecord]) -> Result<Vec<ReadId>> {
        let total_reads = reads.len();

        // Step 1: Extract minimizers in parallel
        let all_buckets: Vec<Vec<(u64, u64)>> = reads
            .par_iter()
            .enumerate()
            .map(|(i, read)| {
                let mins = extract_minimizers(
                    read.sequence.as_bytes(),
                    self.config.minimizer_k,
                    self.config.minimizer_w,
                );
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
        //
        // `unused` makes the no-candidate fallback O(log n) instead of scanning
        // the whole read set for every emitted read. The old `(0..n).find`
        // fallback was quadratic on independent reads with no shared k-mers.
        let mut used = vec![false; total_reads];
        let mut unused: BTreeSet<usize> = (0..total_reads).collect();
        let mut ordered: Vec<ReadId> = Vec::with_capacity(total_reads);

        ordered.push(0);
        used[0] = true;
        unused.remove(&0);

        while ordered.len() < total_reads {
            let last_read = *ordered.last().expect("ordered is never empty - initialized with 0") as usize;
            let last_len = reads[last_read].sequence.len();

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

                        let clen = reads[cid].sequence.len();
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
                let id = m as usize;
                unused.remove(&id);
                id
            } else {
                unused.pop_first().ok_or_else(|| {
                    FqcError::Compression("reorder planning exhausted unused reads unexpectedly".to_string())
                })?
            };

            ordered.push(next as u64);
            used[next] = true;
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
