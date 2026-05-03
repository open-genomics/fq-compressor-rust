// =============================================================================
// fqc-rust - Global Analyzer (Minimizer Bucketing + Read Reordering)
// =============================================================================

use crate::algo::dna::BASE_TO_INDEX;
use crate::error::Result;
use crate::types::*;
use rayon::prelude::*;
use std::collections::HashMap;

// =============================================================================
// Minimizer Extraction
// =============================================================================

#[derive(Debug, Clone)]
pub struct Minimizer {
    pub hash: u64,
    pub position: u32,
    pub is_rc: bool,
}

fn compute_kmer_hash(seq: &[u8]) -> u64 {
    let k = seq.len();
    let mut hash: u64 = 0;
    let mut rc_hash: u64 = 0;
    for i in 0..k {
        let base = BASE_TO_INDEX[seq[i] as usize] as u64;
        hash = (hash << 2) | base;
        let rc_base = 3 - BASE_TO_INDEX[seq[k - 1 - i] as usize] as u64;
        rc_hash = (rc_hash << 2) | rc_base;
    }
    hash.min(rc_hash)
}

pub fn extract_minimizers(seq: &[u8], k: usize, w: usize) -> Vec<Minimizer> {
    let mut minimizers = Vec::new();
    if seq.len() < k {
        return minimizers;
    }

    let num_kmers = seq.len() - k + 1;
    let hashes: Vec<u64> = (0..num_kmers).map(|i| compute_kmer_hash(&seq[i..i + k])).collect();

    let window_size = w.min(num_kmers);
    let mut prev_min_pos = usize::MAX;

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
            let mut fwd_hash: u64 = 0;
            for i in 0..k {
                let base = BASE_TO_INDEX[seq[min_pos + i] as usize] as u64;
                fwd_hash = (fwd_hash << 2) | base;
            }
            let is_rc = min_hash != fwd_hash;
            minimizers.push(Minimizer {
                hash: min_hash,
                position: min_pos as u32,
                is_rc,
            });
            prev_min_pos = min_pos;
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
    pub memory_limit: usize,
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
            memory_limit: 0,
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
    pub total_reads: u64,
    pub max_read_length: usize,
    pub length_class: ReadLengthClass,
    pub reordering_performed: bool,
    pub forward_map: Vec<ReadId>,
    pub reverse_map: Vec<ReadId>,
    pub block_boundaries: Vec<BlockBoundary>,
    pub num_blocks: usize,
}

impl GlobalAnalysisResult {
    pub fn find_block(&self, archive_id: ReadId) -> Option<BlockId> {
        let idx = self
            .block_boundaries
            .partition_point(|b| b.archive_id_start <= archive_id);
        if idx == 0 {
            return None;
        }
        let b = &self.block_boundaries[idx - 1];
        if archive_id >= b.archive_id_start && archive_id < b.archive_id_end {
            Some(b.block_id)
        } else {
            None
        }
    }
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

        let mut result = GlobalAnalysisResult {
            total_reads,
            ..Default::default()
        };

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

        // Step 2: Greedy reordering (approximate Hamiltonian path)
        let mut used = vec![false; total_reads];
        let mut ordered: Vec<ReadId> = Vec::with_capacity(total_reads);

        ordered.push(0);
        used[0] = true;

        while ordered.len() < total_reads {
            let last_read = *ordered.last().expect("ordered is never empty - initialized with 0") as usize;
            let last_seq = sequences[last_read].as_bytes();

            let last_mins = extract_minimizers(last_seq, self.config.minimizer_k, self.config.minimizer_w);

            let mut best_match: Option<u64> = None;
            let mut best_score = usize::MAX;

            let last_len = last_seq.len();
            let mut searched = 0;

            'outer: for m in &last_mins {
                if let Some(bucket) = bucket_map.get(&m.hash) {
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
