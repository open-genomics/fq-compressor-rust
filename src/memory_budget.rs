// =============================================================================
// fqc-rust - Memory Budget Management
// =============================================================================
// Memory budget calculation, monitoring, and chunking strategy for large files.
// =============================================================================

// =============================================================================
// Constants
// =============================================================================

const MIN_MEMORY_LIMIT_MB: usize = 256;
const DEFAULT_BLOCK_BUFFER_MB: usize = 512;
const DEFAULT_WORKER_STACK_MB: usize = 64;
const MEMORY_PER_READ_PHASE1: usize = 24;
const MEMORY_PER_READ_PHASE2: usize = 50;
const MEMORY_SAFETY_MARGIN: f64 = 1.2;

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
// MemoryEstimate
// =============================================================================

#[derive(Debug, Clone, Default)]

// =============================================================================
// MemoryEstimator
// =============================================================================

pub struct MemoryEstimator {
    budget: MemoryBudget,
}

impl MemoryEstimator {
    pub fn new(budget: MemoryBudget) -> Self {
        Self { budget }
    }

    pub fn optimal_block_size(&self, num_threads: usize) -> usize {
        let available = self.budget.phase2_available_bytes();
        let per_thread = (available as f64 / (num_threads as f64 * MEMORY_SAFETY_MARGIN)) as usize;
        let block_size = per_thread / MEMORY_PER_READ_PHASE2;
        block_size.clamp(1000, 500_000)
    }
}

// =============================================================================
// System Memory Detection
// =============================================================================

/// Get available system memory in MB.
pub fn get_available_memory_mb() -> usize {
    #[cfg(target_os = "windows")]
    {
        get_available_memory_windows()
    }
    #[cfg(target_os = "linux")]
    {
        get_available_memory_linux()
    }
    #[cfg(target_os = "macos")]
    {
        get_available_memory_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        8192 // Default fallback
    }
}

#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
fn get_available_memory_windows() -> usize {
    use std::mem;

    #[repr(C)]
    struct MemoryStatusEx {
        dw_length: u32,
        dw_memory_load: u32,
        ull_total_phys: u64,
        ull_avail_phys: u64,
        ull_total_page_file: u64,
        ull_avail_page_file: u64,
        ull_total_virtual: u64,
        ull_avail_virtual: u64,
        ull_avail_extended_virtual: u64,
    }

    extern "system" {
        fn GlobalMemoryStatusEx(lpBuffer: *mut MemoryStatusEx) -> i32;
    }

    unsafe {
        let mut status: MemoryStatusEx = mem::zeroed();
        status.dw_length = mem::size_of::<MemoryStatusEx>() as u32;
        if GlobalMemoryStatusEx(&mut status) != 0 {
            (status.ull_avail_phys / (1024 * 1024)) as usize
        } else {
            8192
        }
    }
}

#[cfg(target_os = "linux")]
fn get_available_memory_linux() -> usize {
    if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
        for line in contents.lines() {
            if line.starts_with("MemAvailable:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<usize>() {
                        return kb / 1024;
                    }
                }
            }
        }
    }
    8192
}

#[cfg(target_os = "macos")]
fn get_available_memory_macos() -> usize {
    // Use sysctl to get physical memory
    use std::process::Command;
    if let Ok(output) = Command::new("sysctl").arg("-n").arg("hw.memsize").output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
            if let Ok(bytes) = s.trim().parse::<u64>() {
                return (bytes / (1024 * 1024)) as usize;
            }
        }
    }
    8192
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
        let effective_limit = if memory_limit_mb == 0 {
            let system_mb = get_available_memory_mb();
            // Use 75% of available memory
            (system_mb as f64 * 0.75) as usize
        } else {
            memory_limit_mb
        };

        let bytes_per_read = avg_read_length * 3 + 80; // seq + qual + id + overhead
        let phase1_per_read = MEMORY_PER_READ_PHASE1 + bytes_per_read;
        let phase1_total_mb = (total_reads * phase1_per_read) / (1024 * 1024);

        let available_mb = effective_limit.max(MIN_MEMORY_LIMIT_MB);

        let num_chunks = if phase1_total_mb <= available_mb {
            1
        } else {
            phase1_total_mb.div_ceil(available_mb).max(2)
        };

        let reads_per_chunk = total_reads.div_ceil(num_chunks);
        let blocks_per_chunk = reads_per_chunk.div_ceil(block_size);

        let chunk_phase1_mb = (reads_per_chunk * phase1_per_read) / (1024 * 1024);
        let phase2_per_block_mb = (block_size * MEMORY_PER_READ_PHASE2) / (1024 * 1024);
        let phase2_mb = phase2_per_block_mb * num_threads.min(blocks_per_chunk);
        let estimated_peak_mb =
            chunk_phase1_mb.max(phase2_mb) + DEFAULT_BLOCK_BUFFER_MB + DEFAULT_WORKER_STACK_MB * num_threads;

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
