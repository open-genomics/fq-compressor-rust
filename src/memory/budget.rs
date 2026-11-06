// =============================================================================
// fqc - Coarse memory budget and chunking strategy
// =============================================================================

use super::system::get_available_memory_mb;

const MIN_MEMORY_LIMIT_MB: usize = 256;
const DEFAULT_BLOCK_BUFFER_MB: usize = 512;
const DEFAULT_WORKER_STACK_MB: usize = 64;
const MEMORY_PER_READ_PHASE1: usize = 24;
const MEMORY_PER_READ_PHASE2: usize = 50;

// =============================================================================
// MemoryBudget
// =============================================================================

#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct MemoryBudget {
    pub max_total_mb: usize,
    pub block_buffer_mb: usize,
    pub worker_stack_mb: usize,
}

impl MemoryBudget {
    pub fn from_memory_limit(memory_limit_mb: usize) -> Self {
        let total_mb = memory_limit_mb.max(MIN_MEMORY_LIMIT_MB);
        let block_mb = (total_mb / 16).min(DEFAULT_BLOCK_BUFFER_MB);

        Self {
            max_total_mb: total_mb,
            block_buffer_mb: block_mb,
            worker_stack_mb: DEFAULT_WORKER_STACK_MB,
        }
    }

    pub fn block_buffer_bytes(&self) -> usize {
        self.block_buffer_mb * 1024 * 1024
    }

    pub fn phase2_available_bytes(&self) -> usize {
        let used = self.block_buffer_mb + self.worker_stack_mb;
        if self.max_total_mb > used {
            (self.max_total_mb - used) * 1024 * 1024
        } else {
            MIN_MEMORY_LIMIT_MB * 1024 * 1024
        }
    }
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self::from_memory_limit(8192)
    }
}

// =============================================================================
// Dynamic Chunking Strategy
// =============================================================================

/// Strategy for divide-and-conquer chunking when data exceeds memory limits
#[derive(Debug, Clone)]
pub struct ChunkingStrategy {
    pub num_chunks: usize,
    pub reads_per_chunk: usize,
    pub block_size: usize,
    pub blocks_per_chunk: usize,
    pub estimated_peak_mb: usize,
}

impl ChunkingStrategy {
    /// Compute an optimal chunking strategy given dataset and system constraints
    pub fn compute(
        total_reads: usize,
        avg_read_length: usize,
        block_size: usize,
        num_threads: usize,
        memory_limit_mb: usize,
    ) -> Self {
        let block_size = block_size.max(1);
        let num_threads = num_threads.max(1);
        let effective_limit = if memory_limit_mb == 0 {
            let system_mb = get_available_memory_mb();
            // Use 75% of available memory
            (system_mb as f64 * 0.75) as usize
        } else {
            memory_limit_mb
        };

        let bytes_per_read = avg_read_length.saturating_mul(3).saturating_add(80); // seq + qual + id + overhead
        let phase1_per_read = MEMORY_PER_READ_PHASE1.saturating_add(bytes_per_read);
        let phase1_total_mb = total_reads.saturating_mul(phase1_per_read) / (1024 * 1024);

        let available_mb = effective_limit.max(MIN_MEMORY_LIMIT_MB);

        let num_chunks = if phase1_total_mb <= available_mb {
            1
        } else {
            phase1_total_mb.div_ceil(available_mb).max(2)
        };

        let reads_per_chunk = total_reads.div_ceil(num_chunks);
        let blocks_per_chunk = reads_per_chunk.div_ceil(block_size);

        let chunk_phase1_mb = reads_per_chunk.saturating_mul(phase1_per_read) / (1024 * 1024);
        let phase2_per_block_mb = block_size.saturating_mul(MEMORY_PER_READ_PHASE2) / (1024 * 1024);
        let phase2_mb = phase2_per_block_mb.saturating_mul(num_threads.min(blocks_per_chunk));
        let estimated_peak_mb = chunk_phase1_mb
            .max(phase2_mb)
            .saturating_add(DEFAULT_BLOCK_BUFFER_MB)
            .saturating_add(DEFAULT_WORKER_STACK_MB.saturating_mul(num_threads));

        Self {
            num_chunks,
            reads_per_chunk,
            block_size,
            blocks_per_chunk,
            estimated_peak_mb,
        }
    }

    pub fn requires_chunking(&self) -> bool {
        self.num_chunks > 1
    }

    /// Summary string for logging
    pub fn summary(&self) -> String {
        if self.requires_chunking() {
            format!(
                "{} chunks × {} reads/chunk ({} blocks/chunk), est. peak {} MB",
                self.num_chunks, self.reads_per_chunk, self.blocks_per_chunk, self.estimated_peak_mb
            )
        } else {
            format!(
                "single pass, {} reads, est. peak {} MB",
                self.reads_per_chunk, self.estimated_peak_mb
            )
        }
    }
}

/// Auto-configure memory budget from system state
pub fn auto_memory_budget(user_limit_mb: usize) -> MemoryBudget {
    let limit = if user_limit_mb == 0 {
        let avail = get_available_memory_mb();
        (avail as f64 * 0.75) as usize
    } else {
        user_limit_mb
    };
    MemoryBudget::from_memory_limit(limit)
}
