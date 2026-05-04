// =============================================================================
// fqc-rust - Verify Command
// =============================================================================

use crate::algo::block_compressor::BlockCompressor;
use crate::error::{FqcError, Result};
use crate::format::{get_id_mode, get_quality_mode, get_read_length_class};
use crate::fqc_reader::FqcReader;
use xxhash_rust::xxh64::Xxh64;

// =============================================================================
// VerifyOptions
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct VerifyOptions {
    pub input_path: String,
    pub fail_fast: bool,
    pub verbose: bool,
    /// Quick verification: only check magic header + footer (skip block decompression)
    pub quick_mode: bool,
}

// =============================================================================
// VerifyStats
// =============================================================================

#[derive(Debug, Default)]
struct VerifyStats {
    blocks_checked: u64,
    blocks_ok: u64,
    blocks_failed: u64,
    reads_verified: u64,
}

// =============================================================================
// VerifyCommand
// =============================================================================

pub struct VerifyCommand {
    opts: VerifyOptions,
    stats: VerifyStats,
}

impl VerifyCommand {
    pub fn new(opts: VerifyOptions) -> Self {
        Self {
            opts,
            stats: VerifyStats::default(),
        }
    }

    pub fn execute(mut self) -> i32 {
        match self.run() {
            Ok(passed) => {
                if passed {
                    println!(
                        "Verification PASSED: {} blocks checked, {} reads verified",
                        self.stats.blocks_checked, self.stats.reads_verified
                    );
                    0
                } else {
                    eprintln!(
                        "Verification FAILED: {}/{} blocks had errors",
                        self.stats.blocks_failed, self.stats.blocks_checked
                    );
                    1
                }
            }
            Err(e) => {
                eprintln!("Verify failed: {e}");
                e.exit_code_num()
            }
        }
    }

    fn run(&mut self) -> Result<bool> {
        if !std::path::Path::new(&self.opts.input_path).exists() {
            return Err(FqcError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found: {}", self.opts.input_path),
            )));
        }

        let mut reader = FqcReader::open(&self.opts.input_path)?;

        // Verify footer magic
        if !reader.footer.is_valid() {
            if self.opts.verbose {
                println!("Footer magic: FAILED");
            }
            return Ok(false);
        }
        if self.opts.verbose {
            println!("Footer magic: OK");
        }

        // Verify global checksum (if non-zero)
        if reader.footer.global_checksum != 0 {
            if self.opts.verbose {
                print!("Global checksum: ");
            }

            // Recompute: the writer hashes flags + all block compressed streams
            let mut global_hasher = Xxh64::new(0);
            global_hasher.update(&reader.global_header.flags.to_le_bytes());

            for bid in 0..reader.block_count() {
                let block_data = reader.read_block(bid as u32)?;
                global_hasher.update(&block_data.ids_data);
                global_hasher.update(&block_data.seq_data);
                global_hasher.update(&block_data.qual_data);
                global_hasher.update(&block_data.aux_data);
            }

            let computed = global_hasher.digest();
            if computed != reader.footer.global_checksum {
                if self.opts.verbose {
                    println!(
                        "FAILED (expected=0x{:016x}, computed=0x{:016x})",
                        reader.footer.global_checksum, computed
                    );
                }
                return Ok(false);
            }
            if self.opts.verbose {
                println!("OK (0x{:016x})", computed);
            }
        }

        // In quick mode, we only verify magic + footer + global checksum
        if self.opts.quick_mode {
            if self.opts.verbose {
                println!("Quick mode: skipping block-level verification");
            }
            return Ok(true);
        }

        let flags = reader.global_header.flags;

        let block_config = crate::algo::block_compressor::BlockCompressorConfig {
            read_length_class: get_read_length_class(flags),
            quality_mode: get_quality_mode(flags),
            id_mode: get_id_mode(flags),
            ..Default::default()
        };

        let mut compressor = BlockCompressor::new(block_config);
        let block_count = reader.block_count();
        let mut all_ok = true;

        for block_id in 0..block_count {
            self.stats.blocks_checked += 1;

            if self.opts.verbose {
                print!("Block {}/{}... ", block_id + 1, block_count);
            }

            match self.verify_block(&mut reader, &mut compressor, block_id as u32) {
                Ok(reads_in_block) => {
                    self.stats.blocks_ok += 1;
                    self.stats.reads_verified += reads_in_block;
                    if self.opts.verbose {
                        println!("OK ({} reads)", reads_in_block);
                    }
                }
                Err(e) => {
                    self.stats.blocks_failed += 1;
                    all_ok = false;
                    if self.opts.verbose {
                        println!("FAILED: {}", e);
                    } else {
                        eprintln!("Block {} failed: {}", block_id, e);
                    }
                    if self.opts.fail_fast {
                        return Ok(false);
                    }
                }
            }
        }

        Ok(all_ok)
    }

    fn verify_block(&self, reader: &mut FqcReader, compressor: &mut BlockCompressor, block_id: u32) -> Result<u64> {
        let block_data = reader.read_block(block_id)?;
        let bh = &block_data.header;

        // Decompress the block first to verify data integrity
        let decompressed = compressor.decompress_block(&block_data)?;

        // Verify checksum against decompressed reads
        if bh.block_xxhash64 != 0 {
            let computed = crate::algo::block_compressor::compute_block_checksum(&decompressed.reads);
            if computed != bh.block_xxhash64 {
                return Err(FqcError::ChecksumMismatch {
                    expected: bh.block_xxhash64,
                    actual: computed,
                });
            }
        }

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
}
