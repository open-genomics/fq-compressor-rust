//! Decode / verify resource budget tests (FQCR-LIMIT-001).

use fqc::archive::format::{BLOCK_INDEX_HEADER_SIZE, INDEX_ENTRY_SIZE};
use fqc::archive::reader::FqcReader;
use fqc::commands::compress::{CompressCommand, CompressOptions};
use fqc::commands::decompress::{DecompressCommand, DecompressOptions};
use fqc::commands::verify::{VerifyCommand, VerifyOptions};
use fqc::error::FqcError;
use fqc::memory_budget::{zstd_decompress_bounded, DecodeBudget, HARD_MAX_INDEX_ENTRIES, MIN_DECODE_MEMORY_MB};
use std::fs;
use std::path::PathBuf;

fn write_tiny_fastq(path: &PathBuf, n_reads: usize) {
    use std::fmt::Write as _;
    let mut out = String::new();
    for i in 0..n_reads {
        write!(
            out,
            "@r{i}\nACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTAC\n+\nIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIII\n"
        )
        .unwrap();
    }
    fs::write(path, out).unwrap();
}

fn compress_tiny(dir: &std::path::Path, n_reads: usize, reorder: bool) -> (PathBuf, PathBuf) {
    let input = dir.join("in.fastq");
    let archive = dir.join("out.fqc");
    write_tiny_fastq(&input, n_reads);
    let code = CompressCommand::new(CompressOptions {
        input_path: input.to_string_lossy().into_owned(),
        output_path: archive.to_string_lossy().into_owned(),
        force_overwrite: true,
        enable_reorder: reorder,
        show_progress: false,
        ..Default::default()
    })
    .execute();
    assert_eq!(code, 0, "compress failed");
    (input, archive)
}

fn assert_resource_limit(err: FqcError) {
    match err {
        FqcError::ResourceLimit {
            location,
            declared,
            allowed,
        } => {
            assert!(!location.is_empty());
            assert!(declared > allowed || allowed == 0);
        }
        other => panic!("expected ResourceLimit, got {other}"),
    }
}

#[test]
fn tiny_archive_opens_and_verifies_under_min_budget() {
    let dir = tempfile::tempdir().unwrap();
    let (_input, archive) = compress_tiny(dir.path(), 4, false);

    let budget = DecodeBudget::resolve(MIN_DECODE_MEMORY_MB);
    FqcReader::open_with_budget(archive.to_str().unwrap(), budget).unwrap();

    let code = VerifyCommand::new(VerifyOptions {
        input_path: archive.to_string_lossy().into_owned(),
        memory_limit_mb: MIN_DECODE_MEMORY_MB,
        ..Default::default()
    })
    .execute();
    assert_eq!(code, 0);

    let out = dir.path().join("out.fastq");
    let code = DecompressCommand::new(DecompressOptions {
        input_path: archive.to_string_lossy().into_owned(),
        output_path: out.to_string_lossy().into_owned(),
        force_overwrite: true,
        memory_limit_mb: MIN_DECODE_MEMORY_MB,
        show_progress: false,
        ..Default::default()
    })
    .execute();
    assert_eq!(code, 0);
    assert!(out.exists());
}

#[test]
fn forged_huge_num_blocks_rejected_before_alloc() {
    let dir = tempfile::tempdir().unwrap();
    let (_input, archive) = compress_tiny(dir.path(), 4, false);

    let reader = FqcReader::open(archive.to_str().unwrap()).unwrap();
    let index_offset = reader.footer.index_offset as usize;
    drop(reader);

    // num_blocks sits at index_offset + 8 (after header_size + entry_size).
    let mut bytes = fs::read(&archive).unwrap();
    let forged = HARD_MAX_INDEX_ENTRIES + 1;
    bytes[index_offset + 8..index_offset + 16].copy_from_slice(&forged.to_le_bytes());
    fs::write(&archive, &bytes).unwrap();

    let budget = DecodeBudget::resolve(MIN_DECODE_MEMORY_MB);
    let err = match FqcReader::open_with_budget(archive.to_str().unwrap(), budget) {
        Ok(_) => panic!("expected forged num_blocks to be rejected"),
        Err(e) => e,
    };
    assert_resource_limit(err);
}

#[test]
fn forged_num_blocks_exceeding_file_region_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let (_input, archive) = compress_tiny(dir.path(), 4, false);

    let reader = FqcReader::open(archive.to_str().unwrap()).unwrap();
    let index_offset = reader.footer.index_offset as usize;
    let file_size = reader.file_size;
    drop(reader);

    let index_region = file_size.saturating_sub(index_offset as u64).saturating_sub(32);
    let max_by_file = index_region / INDEX_ENTRY_SIZE as u64;
    let forged = (max_by_file + 100).max(10_000);

    let mut bytes = fs::read(&archive).unwrap();
    bytes[index_offset + 8..index_offset + 16].copy_from_slice(&forged.to_le_bytes());
    // Keep header sizes valid so we hit the budget check.
    assert_eq!(
        u32::from_le_bytes(bytes[index_offset..index_offset + 4].try_into().unwrap()) as usize,
        BLOCK_INDEX_HEADER_SIZE
    );
    fs::write(&archive, &bytes).unwrap();

    let budget = DecodeBudget::resolve(64);
    let err = match FqcReader::open_with_budget(archive.to_str().unwrap(), budget) {
        Ok(_) => panic!("expected forged num_blocks vs file region to be rejected"),
        Err(e) => e,
    };
    match err {
        FqcError::ResourceLimit { location, .. } => {
            assert!(
                location.contains("block_index") || location.contains("num_blocks"),
                "unexpected location: {location}"
            );
        }
        other => panic!("expected ResourceLimit, got {other}"),
    }
}

#[test]
fn zstd_bounded_rejects_expansion_past_ceiling() {
    let payload = vec![b'A'; 256 * 1024];
    let compressed = zstd::bulk::compress(&payload, 3).unwrap();
    let err = zstd_decompress_bounded(&compressed, 1024, "zstd-bomb-test").unwrap_err();
    assert_resource_limit(err);
}

#[test]
fn original_order_peak_rejected_before_output() {
    let dir = tempfile::tempdir().unwrap();
    // Enough reads that peak estimate with avg_bases_hint=256 exceeds 16 MB.
    let (_input, archive) = compress_tiny(dir.path(), 40_000, true);

    let out = dir.path().join("restored.fastq");
    assert!(!out.exists());

    let code = DecompressCommand::new(DecompressOptions {
        input_path: archive.to_string_lossy().into_owned(),
        output_path: out.to_string_lossy().into_owned(),
        original_order: true,
        force_overwrite: true,
        memory_limit_mb: MIN_DECODE_MEMORY_MB,
        show_progress: false,
        ..Default::default()
    })
    .execute();

    assert_ne!(code, 0);
    assert!(!out.exists(), "output must not be created when peak exceeds budget");
}

#[test]
fn automatic_budget_is_finite() {
    let b = DecodeBudget::resolve(0);
    assert!(b.automatic);
    assert!(b.limit_bytes > 0);
    assert!(b.max_alloc_bytes > 0);
    assert!(b.max_index_entries > 0);
    assert!(b.max_index_entries <= HARD_MAX_INDEX_ENTRIES);
}

#[test]
fn parallel_batch_never_zero_and_rejects_oversized_block() {
    let budget = DecodeBudget::resolve(MIN_DECODE_MEMORY_MB);
    let n = budget.parallel_batch_size(4, 64 * 1024).unwrap();
    assert!(n >= 1);

    let huge = budget.limit_bytes; // compressed hint → peak = *3 > limit
    let err = budget.parallel_batch_size(4, huge).unwrap_err();
    assert_resource_limit(err);
}
