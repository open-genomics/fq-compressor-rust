// =============================================================================
// fqc-rust - Malformed Input Defense Tests
// =============================================================================
// The decoder is documented to fail closed on forged/truncated input, and
// tests/test_decode_budget.rs covers hand-crafted hostile headers. These
// tests add breadth: every truncation offset, single-bit corruption at every
// byte, and deterministic pseudo-random garbage must never panic — returning
// an error is always acceptable.
// =============================================================================

use std::io::Cursor;
use std::path::PathBuf;

use fqc::archive::reader::FqcReader;
use fqc::fastq::parser::FastqParser;

fn repo_file(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn frozen_archive_bytes() -> Vec<u8> {
    std::fs::read(repo_file("tests/fixtures/indexed-v2/frozen.fqc")).unwrap()
}

/// Opens the archive and drains every block. Any `Err` is fine — a panic is
/// not.
fn decode_archive(path: &str) -> fqc::error::Result<()> {
    let mut reader = FqcReader::open(path)?;
    if reader.has_reorder_map() {
        reader.load_reorder_map()?;
    }
    for block_id in 0..reader.block_count() as u32 {
        let _ = reader.read_block(block_id)?;
    }
    Ok(())
}

fn assert_decode_never_panics(bytes: &[u8], label: &str) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("case.fqc");
    std::fs::write(&path, bytes).unwrap();

    let displayed = path.to_string_lossy().into_owned();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = decode_archive(&displayed);
    }));
    assert!(outcome.is_ok(), "decoder panicked on {label}");
}

fn parse_all_without_panicking(bytes: &[u8], label: &str) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut parser = FastqParser::new(Cursor::new(bytes.to_vec()));
        loop {
            match parser.next_record() {
                Ok(Some(_)) => continue,
                Ok(None) | Err(_) => break,
            }
        }
    }));
    assert!(outcome.is_ok(), "parser panicked on {label}");
}

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

#[test]
fn intact_frozen_fixture_still_decodes() {
    // Guard for the sweeps below: the untouched fixture must decode cleanly,
    // otherwise the sweeps are vacuous.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("intact.fqc");
    std::fs::write(&path, frozen_archive_bytes()).unwrap();
    decode_archive(path.to_string_lossy().as_ref()).expect("intact fixture must decode");
}

#[test]
fn every_truncation_offset_is_rejected_without_panic() {
    let full = frozen_archive_bytes();
    for len in 0..full.len() {
        assert_decode_never_panics(&full[..len], &format!("truncation to {len} bytes"));
    }
}

#[test]
fn single_bit_flips_at_every_byte_are_rejected_without_panic() {
    let full = frozen_archive_bytes();
    for pos in 0..full.len() {
        for mask in [0x01u8, 0x80u8] {
            let mut mutated = full.clone();
            mutated[pos] ^= mask;
            assert_decode_never_panics(&mutated, &format!("bit flip at byte {pos} mask {mask}"));
        }
    }
}

#[test]
fn random_garbage_never_panics() {
    let mut rng = XorShift(0x0F_9C_E5_EE_11_u64);
    for case in 0..300u32 {
        let len = (rng.next() % 129) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        assert_decode_never_panics(&bytes, &format!("random archive case {case}"));
        parse_all_without_panicking(&bytes, &format!("random fastq case {case}"));
    }
}

#[test]
fn mutated_valid_fastq_never_panics() {
    let valid = std::fs::read(repo_file("tests/data/test_se.fastq")).unwrap();
    let mut rng = XorShift(0x5EED);
    for case in 0..200u32 {
        let mut mutated = valid.clone();
        let mutations = 1 + (rng.next() % 8) as usize;
        for _ in 0..mutations {
            let pos = (rng.next() as usize) % mutated.len();
            mutated[pos] = (rng.next() & 0xFF) as u8;
        }
        parse_all_without_panicking(&mutated, &format!("mutated fastq case {case}"));
    }
}
