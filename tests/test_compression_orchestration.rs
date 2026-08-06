use std::io::BufWriter;
use std::path::{Path, PathBuf};

use fqc::archive::reader::FqcReader;
use fqc::commands::compress::{CompressCommand, CompressOptions};
use fqc::fastq::parser::write_record;
use fqc::types::{PeLayout, ReadRecord};
use tempfile::tempdir;

fn test_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("data")
}

fn paired_inputs() -> (String, String) {
    let dir = test_data_dir();
    (
        dir.join("test_R1.fastq").to_string_lossy().to_string(),
        dir.join("test_R2.fastq").to_string_lossy().to_string(),
    )
}

fn interleaved_input() -> String {
    test_data_dir()
        .join("test_interleaved.fastq")
        .to_string_lossy()
        .to_string()
}

fn base_paired_options(output_path: &Path) -> CompressOptions {
    let (r1, r2) = paired_inputs();
    CompressOptions {
        input_path: r1,
        input2_path: Some(r2),
        output_path: output_path.to_string_lossy().to_string(),
        pe_layout: PeLayout::Consecutive,
        force_overwrite: true,
        show_progress: false,
        threads: 1,
        ..Default::default()
    }
}

fn run_and_read_info(opts: &CompressOptions) -> fqc::archive::reader::ArchiveInfo {
    let exit_code = CompressCommand::new(opts.clone()).execute();
    assert_eq!(exit_code, 0, "compression should succeed");
    FqcReader::open(&opts.output_path).unwrap().info()
}

fn write_fastq(path: &Path, records: &[ReadRecord]) {
    let file = std::fs::File::create(path).unwrap();
    let mut writer = BufWriter::new(file);
    for record in records {
        write_record(&mut writer, record).unwrap();
    }
}

#[test]
fn test_archive_mode_preserves_requested_paired_layout_for_paired_files() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("archive-layout.fqc");

    let opts = base_paired_options(&output);
    let info = run_and_read_info(&opts);

    assert!(info.is_paired);
    assert_eq!(info.pe_layout, PeLayout::Consecutive);
}

#[test]
fn test_streaming_mode_preserves_requested_paired_layout_for_paired_files() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("streaming-layout.fqc");
    let mut opts = base_paired_options(&output);
    opts.streaming_mode = true;

    let info = run_and_read_info(&opts);

    assert!(info.is_paired);
    assert!(info.streaming_mode);
    assert_eq!(info.pe_layout, PeLayout::Consecutive);
}

#[test]
fn test_pipeline_mode_preserves_requested_paired_layout_for_paired_files() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("pipeline-layout.fqc");
    let mut opts = base_paired_options(&output);
    opts.use_pipeline = true;

    let info = run_and_read_info(&opts);

    assert!(info.is_paired);
    assert_eq!(info.pe_layout, PeLayout::Consecutive);
}

#[test]
fn test_pipeline_mode_marks_interleaved_input_as_paired() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("pipeline-interleaved.fqc");
    let opts = CompressOptions {
        input_path: interleaved_input(),
        output_path: output.to_string_lossy().to_string(),
        interleaved: true,
        use_pipeline: true,
        pe_layout: PeLayout::Consecutive,
        force_overwrite: true,
        show_progress: false,
        threads: 1,
        ..Default::default()
    };

    let info = run_and_read_info(&opts);

    assert!(info.is_paired);
    assert_eq!(info.pe_layout, PeLayout::Consecutive);
}

#[test]
fn test_archive_mode_rejects_mismatched_paired_files() {
    let dir = tempdir().unwrap();
    let r1 = dir.path().join("mismatch_R1.fastq");
    let r2 = dir.path().join("mismatch_R2.fastq");
    let output = dir.path().join("mismatch.fqc");

    write_fastq(
        &r1,
        &[
            ReadRecord::new("pair1/1".into(), "ACGT".into(), "IIII".into()),
            ReadRecord::new("pair2/1".into(), "TGCA".into(), "JJJJ".into()),
        ],
    );
    write_fastq(&r2, &[ReadRecord::new("pair1/2".into(), "ACGT".into(), "IIII".into())]);

    let opts = CompressOptions {
        input_path: r1.to_string_lossy().to_string(),
        input2_path: Some(r2.to_string_lossy().to_string()),
        output_path: output.to_string_lossy().to_string(),
        force_overwrite: true,
        show_progress: false,
        threads: 1,
        ..Default::default()
    };

    let exit_code = CompressCommand::new(opts).execute();
    assert_ne!(exit_code, 0, "mismatched paired inputs must fail");
}

#[test]
fn test_streaming_mode_rejects_odd_interleaved_input() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("odd-interleaved.fastq");
    let output = dir.path().join("odd-interleaved.fqc");

    write_fastq(
        &input,
        &[
            ReadRecord::new("pair1/1".into(), "ACGT".into(), "IIII".into()),
            ReadRecord::new("pair1/2".into(), "TGCA".into(), "JJJJ".into()),
            ReadRecord::new("pair2/1".into(), "GGGG".into(), "HHHH".into()),
        ],
    );

    let opts = CompressOptions {
        input_path: input.to_string_lossy().to_string(),
        output_path: output.to_string_lossy().to_string(),
        interleaved: true,
        streaming_mode: true,
        force_overwrite: true,
        show_progress: false,
        threads: 1,
        ..Default::default()
    };

    let exit_code = CompressCommand::new(opts).execute();
    assert_ne!(exit_code, 0, "odd interleaved paired input must fail");
    assert!(
        !output.exists(),
        "streaming interleaved validation should fail before writing an archive"
    );
}
