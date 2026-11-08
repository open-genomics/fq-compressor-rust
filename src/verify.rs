// =============================================================================
// fqc-rust - Archive verification core
// =============================================================================

use crate::algo::block_compressor::BlockCompressor;
use crate::archive::format::{get_id_mode, get_quality_mode, get_read_length_class};
use crate::archive::reader::FqcReader;
use crate::error::{FqcError, Result};
use crate::memory::DecodeBudget;
use crate::println_stdout;
use std::path::PathBuf;
use xxhash_rust::xxh64::Xxh64;

// =============================================================================
// VerifyOptions
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct VerifyOptions {
    pub input_path: PathBuf,
    pub fail_fast: bool,
    pub verbose: bool,
    /// Quick verification: only check magic header + footer (skip block decompression)
    pub quick_mode: bool,
    /// Memory limit in MB (`0` = automatic, still finite). Full verify shares decode limits.
    pub memory_limit_mb: usize,
}

// =============================================================================
// VerifyReport
// =============================================================================

/// Result of one archive verification run.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VerifyReport {
    pub passed: bool,
    pub blocks_checked: u64,
    pub blocks_ok: u64,
    pub blocks_failed: u64,
    pub reads_verified: u64,
}

/// Verify an archive without printing the top-level summary.
pub fn verify_archive(opts: &VerifyOptions) -> Result<VerifyReport> {
    if !opts.input_path.exists() {
        return Err(FqcError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file not found: {}", opts.input_path.display()),
        )));
    }

    let budget = DecodeBudget::resolve(opts.memory_limit_mb);
    let mut reader = FqcReader::open_with_budget(&opts.input_path, budget)?;
    let mut report = VerifyReport::default();

    // Verify footer magic
    if !reader.footer.is_valid() {
        if opts.verbose {
            println_stdout!("Footer magic: FAILED");
        }
        return Ok(report);
    }
    if opts.verbose {
        println_stdout!("Footer magic: OK");
    }

    // Verify global checksum (if non-zero)
    if reader.footer.global_checksum != 0 {
        // Recompute: the writer hashes flags + all block compressed streams
        let mut global_hasher = Xxh64::new(0);
        global_hasher.update(&reader.global_header.flags.to_le_bytes());

        for bid in 0..reader.block_count() {
            let block_data = reader.read_block(bid as u32)?;
            global_hasher.update(&block_data.id_stream);
            global_hasher.update(&block_data.seq_stream);
            global_hasher.update(&block_data.qual_stream);
            global_hasher.update(&block_data.aux_stream);
        }

        let computed = global_hasher.digest();
        if computed != reader.footer.global_checksum {
            if opts.verbose {
                println_stdout!(
                    "Global checksum: FAILED (expected=0x{:016x}, computed=0x{:016x})",
                    reader.footer.global_checksum,
                    computed
                );
            }
            return Ok(report);
        }
        if opts.verbose {
            println_stdout!("Global checksum: OK (0x{computed:016x})");
        }
    }

    // A reorder map is part of the archive's integrity contract even when
    // the caller only asks for a quick verification. Decode and validate
    // both permutations so a corrupt map cannot be reported as healthy.
    if reader.has_reorder_map() {
        reader.load_reorder_map()?;
        if opts.verbose {
            println_stdout!("Reorder map: OK");
        }
    }

    // In quick mode, we only verify magic + footer + global checksum
    if opts.quick_mode {
        if opts.verbose {
            println_stdout!("Quick mode: skipping block-level verification");
        }
        report.passed = true;
        return Ok(report);
    }

    let flags = reader.global_header.flags;

    let block_config = crate::algo::block_compressor::BlockCompressorConfig {
        read_length_class: get_read_length_class(flags),
        quality_mode: get_quality_mode(flags),
        id_mode: get_id_mode(flags),
        // 与 decompress 相同：单块 ReadRecord 骨架受单次分配预算约束。
        max_block_decoded_bytes: reader.budget().max_alloc_bytes,
        ..Default::default()
    };

    let mut compressor = BlockCompressor::new(block_config);
    let block_count = reader.block_count();
    let mut all_ok = true;

    for block_id in 0..block_count {
        report.blocks_checked += 1;

        match verify_block(&mut reader, &mut compressor, block_id as u32) {
            Ok(reads_in_block) => {
                report.blocks_ok += 1;
                report.reads_verified += reads_in_block;
                if opts.verbose {
                    println_stdout!(
                        "Block {}/{}... OK ({} reads)",
                        block_id + 1,
                        block_count,
                        reads_in_block
                    );
                }
            }
            Err(e) => {
                report.blocks_failed += 1;
                all_ok = false;
                if opts.verbose {
                    println_stdout!("Block {}/{}... FAILED: {e}", block_id + 1, block_count);
                } else {
                    eprintln!("Block {block_id} failed: {e}");
                }
                if opts.fail_fast {
                    return Ok(report);
                }
            }
        }
    }

    report.passed = all_ok;
    Ok(report)
}

fn verify_block(reader: &mut FqcReader, compressor: &mut BlockCompressor, block_id: u32) -> Result<u64> {
    let block_data = reader.read_block(block_id)?;
    // Decompress the block first to verify data integrity
    let decompressed = compressor.decompress_block(&block_data)?;

    // Validate each read
    for read in &decompressed.reads {
        if !read.is_valid() {
            return Err(FqcError::CorruptedBlock {
                block_id,
                reason: format!(
                    "Invalid read: seq len={}, qual len={}",
                    read.sequence.len(),
                    read.quality.len()
                ),
            });
        }
    }

    Ok(decompressed.reads.len() as u64)
}
