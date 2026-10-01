// =============================================================================
// fqc-rust - Reader Info Field Tests
// =============================================================================
// `fqc info` output numbers were previously only hand-checked against the
// fixture MANIFEST. This pins the structured data that feeds the info report
// to the documented values.
// =============================================================================

use std::path::PathBuf;

use fqc::archive::reader::FqcReader;
use fqc::types::{IdMode, QualityMode, ReadLengthClass};

fn frozen_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/indexed-v2/frozen.fqc")
}

#[test]
fn frozen_fixture_info_matches_manifest() {
    let path = frozen_fixture();
    let reader = FqcReader::open(path.to_string_lossy().as_ref()).unwrap();
    let info = reader.info();

    // Values documented in tests/fixtures/indexed-v2/MANIFEST.md
    // ("Archive structure (from `fqc info`)").
    assert_eq!(info.file_size, 448);
    assert_eq!(info.total_reads, 3);
    assert_eq!(info.num_blocks, 1);
    assert!(info.has_reorder_map, "reorder map: present");
    assert_eq!(info.quality_mode, QualityMode::Lossless);
    assert_eq!(info.id_mode, IdMode::Exact);
    assert_eq!(info.read_length_class, ReadLengthClass::Short);
    assert!(!info.is_paired);
    assert!(!info.streaming_mode);

    assert_eq!(reader.block_count(), 1);
    assert_eq!(reader.total_read_count(), 3);
}

#[test]
fn info_rejects_sequential_family_via_reader() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/foreign-sequential-v2/frozen_se.fqc");

    let err = FqcReader::open(path.to_string_lossy().as_ref())
        .map(|_| ())
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unsupported") && msg.contains("fqc-sequential/v2"),
        "unexpected rejection message: {msg}"
    );
}
