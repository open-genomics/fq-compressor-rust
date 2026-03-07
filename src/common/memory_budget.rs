// =============================================================================
// fqc-rust - Memory Budget Management
// =============================================================================
// Memory budget calculation, monitoring, and chunking strategy for large files.
// =============================================================================

// =============================================================================
// Constants
// =============================================================================

const MIN_MEMORY_LIMIT_MB: usize = 256;
const DEFAULT_PHASE1_RESERVE_MB: usize = 2048;
const DEFAULT_BLOCK_BUFFER_MB: usize = 512;
const DEFAULT_WORKER_STACK_MB: usize = 64;
const MEMORY_PER_READ_PHASE1: usize = 24;
const MEMORY_PER_READ_PHASE2: usize = 50;
const MEMORY_SAFETY_MARGIN: f64 = 1.2;

// =============================================================================
// MemoryBudget
// =============================================================================

#[derive(Debug, Clone)]
pub struct MemoryBudget {
    pub max_total_mb: usize,
    pub phase1_reserve_mb: usize,
    pub block_buffer_mb: usize,
    pub worker_stack_mb: usize,
}

impl MemoryBudget {
    pub fn from_memory_limit(memory_limit_mb: usize) -> Self {
        let total_mb = memory_limit_mb.max(MIN_MEMORY_LIMIT_MB);
        let phase1_mb = (total_mb / 4).min(DEFAULT_PHASE1_RESERVE_MB);
        let block_mb = (total_mb / 16).min(DEFAULT_BLOCK_BUFFER_MB);
        let worker_mb = DEFAULT_WORKER_STACK_MB;

        Self {
            max_total_mb: total_mb,
            phase1_reserve_mb: phase1_mb,
            block_buffer_mb: block_mb,
            worker_stack_mb: worker_mb,
        }
    }

    pub fn phase1_reserve_bytes(&self) -> usize {
        self.phase1_reserve_mb * 1024 * 1024
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

    pub fn validate(&self) -> Result<(), String> {
        if self.max_total_mb < MIN_MEMORY_LIMIT_MB {
            return Err(format!("Memory limit must be at least {} MB", MIN_MEMORY_LIMIT_MB));
        }
        if self.phase1_reserve_mb >= self.max_total_mb {
            return Err("Phase 1 reserve must be less than total limit".to_string());
        }
        Ok(())
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
pub struct MemoryEstimate {
    pub phase1_bytes: usize,
    pub phase2_bytes_per_block: usize,
    pub peak_bytes: usize,
    pub max_reads_phase1: usize,
    pub reads_per_block: usize,
    pub requires_chunking: bool,
    pub recommended_chunks: usize,
}

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

    pub fn estimate(&self, total_reads: usize, reads_per_block: usize, num_threads: usize) -> MemoryEstimate {
        let phase1_bytes = self.estimate_phase1(total_reads);
        let phase2_per_block = self.estimate_phase2(reads_per_block, 1);
        let phase2_total = self.estimate_phase2(reads_per_block, num_threads);

        let peak = ((phase1_bytes.max(phase2_total) as f64) * MEMORY_SAFETY_MARGIN) as usize
            + self.budget.block_buffer_bytes();

        let max_reads = self.max_reads_for_phase1();
        let requires_chunking = total_reads > max_reads;

        let recommended_chunks = if requires_chunking {
            total_reads.div_ceil(max_reads).max(2)
        } else {
            1
        };

        MemoryEstimate {
            phase1_bytes,
            phase2_bytes_per_block: phase2_per_block,
            peak_bytes: peak,
            max_reads_phase1: max_reads,
            reads_per_block,
            requires_chunking,
            recommended_chunks,
        }
    }

    fn estimate_phase1(&self, total_reads: usize) -> usize {
        ((total_reads as f64) * (MEMORY_PER_READ_PHASE1 as f64) * MEMORY_SAFETY_MARGIN) as usize
    }

    fn estimate_phase2(&self, reads_per_block: usize, num_threads: usize) -> usize {
        let per_block = ((reads_per_block as f64) * (MEMORY_PER_READ_PHASE2 as f64) * MEMORY_SAFETY_MARGIN) as usize;
        per_block * num_threads
    }

    pub fn max_reads_for_phase1(&self) -> usize {
        let available = self.budget.phase1_reserve_bytes();
        let effective = (available as f64 / MEMORY_SAFETY_MARGIN) as usize;
        effective / MEMORY_PER_READ_PHASE1
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

/// Get current process memory usage in MB.
pub fn get_current_memory_usage_mb() -> usize {
    #[cfg(target_os = "windows")]
    {
        get_process_memory_windows()
    }
    #[cfg(target_os = "linux")]
    {
        get_process_memory_linux()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        0
    }
}

#[cfg(target_os = "windows")]
fn get_process_memory_windows() -> usize {
    use std::mem;

    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(
            process: isize,
            ppsmemCounters: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
    }

    unsafe {
        let mut counters: ProcessMemoryCounters = mem::zeroed();
        counters.cb = mem::size_of::<ProcessMemoryCounters>() as u32;
        let handle = GetCurrentProcess();
        if K32GetProcessMemoryInfo(handle, &mut counters, counters.cb) != 0 {
            counters.working_set_size / (1024 * 1024)
        } else {
            0
        }
    }
}

#[cfg(target_os = "linux")]
fn get_process_memory_linux() -> usize {
    if let Ok(contents) = std::fs::read_to_string("/proc/self/status") {
        for line in contents.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<usize>() {
                        return kb / 1024;
                    }
                }
            }
        }
    }
    0
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
        let estimated_peak_mb = chunk_phase1_mb.max(phase2_mb) +
            DEFAULT_BLOCK_BUFFER_MB + DEFAULT_WORKER_STACK_MB * num_threads;

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
                self.num_chunks, self.reads_per_chunk,
                self.blocks_per_chunk, self.estimated_peak_mb
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
