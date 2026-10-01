// =============================================================================
// fqc-rust - Compression/Decompression Mode Matrix Tests
// =============================================================================
// One explicit cell per {single, paired, interleaved} × {archive, streaming,
// pipeline}: compress, verify, decompress (classic path), and assert content
// integrity plus info totals. These pin the observable semantics of every
// execution path before/while the code paths are being consolidated.
// =============================================================================

use std::path::Path;

use fqc::commands::compress::{CompressCommand, CompressOptions};
use fqc::commands::decompress::{DecompressCommand, DecompressOptions};
use fqc::commands::verify::{VerifyCommand, VerifyOptions};
use fqc::fastq::parser::FastqParser;
use fqc::types::ReadRecord;

// =============================================================================
// Data generation
// =============================================================================

fn write_fastq(path: &Path, records: &[ReadRecord]) {
    use std::fmt::Write as _;
    let mut out = String::new();
    for r in records {
        let _ = writeln!(out, "@{}\n{}\n+\n{}", r.id, r.sequence, r.quality);
    }
    std::fs::write(path, out).unwrap();
}

fn make_records(prefix: &str, n: usize, len: usize) -> Vec<ReadRecord> {
    (0..n)
        .map(|i| {
            let seq: String = (0..len).map(|j| b"ACGT"[(i * 7 + j) % 4] as char).collect();
            ReadRecord::new(format!("{prefix}{i:04}"), seq, "I".repeat(len))
        })
        .collect()
}

#[derive(Debug)]
enum Topology {
    Single,
    Paired,
    Interleaved,
}

#[derive(Debug)]
enum Mode {
    Archive,
    Streaming,
    Pipeline,
}

struct Case {
    dir: tempfile::TempDir,
    reads: Vec<ReadRecord>,
    opts: CompressOptions,
}

fn build_case(topology: &Topology, mode: &Mode) -> Case {
    let dir = tempfile::tempdir().unwrap();
    let (reads, opts) = match topology {
        Topology::Single => {
            let reads = make_records("s", 20, 120);
            let input = dir.path().join("in.fastq");
            write_fastq(&input, &reads);
            let opts = CompressOptions {
                input_path: input.to_string_lossy().into_owned(),
                output_path: dir.path().join("out.fqc").to_string_lossy().into_owned(),
                force_overwrite: true,
                show_progress: false,
                threads: 2,
                ..Default::default()
            };
            (reads, opts)
        }
        Topology::Paired => {
            let r1 = make_records("a", 12, 120);
            let r2 = make_records("b", 12, 120);
            let p1 = dir.path().join("r1.fastq");
            let p2 = dir.path().join("r2.fastq");
            write_fastq(&p1, &r1);
            write_fastq(&p2, &r2);
            let mut reads = r1;
            reads.extend(r2);
            let opts = CompressOptions {
                input_path: p1.to_string_lossy().into_owned(),
                input2_path: Some(p2.to_string_lossy().into_owned()),
                output_path: dir.path().join("out.fqc").to_string_lossy().into_owned(),
                force_overwrite: true,
                show_progress: false,
                threads: 2,
                ..Default::default()
            };
            (reads, opts)
        }
        Topology::Interleaved => {
            let r1 = make_records("a", 12, 120);
            let r2 = make_records("b", 12, 120);
            let interleaved: Vec<ReadRecord> = r1.into_iter().zip(r2).flat_map(|(x, y)| [x, y]).collect();
            let input = dir.path().join("inter.fastq");
            write_fastq(&input, &interleaved);
            let opts = CompressOptions {
                input_path: input.to_string_lossy().into_owned(),
                output_path: dir.path().join("out.fqc").to_string_lossy().into_owned(),
                interleaved: true,
                force_overwrite: true,
                show_progress: false,
                threads: 2,
                ..Default::default()
            };
            (interleaved, opts)
        }
    };

    let mut opts = opts;
    match mode {
        Mode::Archive => {}
        Mode::Streaming => opts.streaming_mode = true,
        Mode::Pipeline => opts.use_pipeline = true,
    }

    Case { dir, reads, opts }
}

fn parse_fastq(path: &Path) -> Vec<ReadRecord> {
    let file = std::fs::File::open(path).unwrap();
    let mut parser = FastqParser::new(std::io::BufReader::new(file));
    parser.collect_all().unwrap()
}

fn sorted_content(reads: &[ReadRecord]) -> Vec<(String, String, String)> {
    let mut v: Vec<_> = reads
        .iter()
        .map(|r| (r.id.clone(), r.sequence.clone(), r.quality.clone()))
        .collect();
    v.sort();
    v
}

fn run_matrix_cell(topology: &Topology, mode: &Mode) {
    let case = build_case(topology, mode);
    let archive = case.opts.output_path.clone();

    let code = CompressCommand::new(case.opts.clone()).execute();
    assert_eq!(code, 0, "compress failed for {topology:?} × {mode:?}");

    let verify_code = VerifyCommand::new(VerifyOptions {
        input_path: archive.clone(),
        ..Default::default()
    })
    .execute();
    assert_eq!(verify_code, 0, "verify failed for {topology:?} × {mode:?}");

    // Info totals via the structured reader API.
    let reader = fqc::archive::reader::FqcReader::open(&archive).unwrap();
    let info = reader.info();
    assert_eq!(
        info.total_reads,
        case.reads.len() as u64,
        "total_reads mismatch for {topology:?} × {mode:?}"
    );
    assert!(info.num_blocks >= 1);
    let has_reorder_map = reader.has_reorder_map();
    drop(reader);

    // Classic decompress path: content must round-trip as a multiset.
    let restored = case.dir.path().join("restored.fastq");
    let dcode = DecompressCommand::new(DecompressOptions {
        input_path: archive.clone(),
        output_path: restored.to_string_lossy().into_owned(),
        force_overwrite: true,
        ..Default::default()
    })
    .execute();
    assert_eq!(dcode, 0, "decompress failed for {topology:?} × {mode:?}");
    let got = parse_fastq(&restored);
    assert_eq!(
        got.len() as u64,
        info.total_reads,
        "restored count mismatch for {topology:?} × {mode:?}"
    );
    assert_eq!(
        sorted_content(&got),
        sorted_content(&case.reads),
        "content mismatch for {topology:?} × {mode:?}"
    );

    // Original-order restore keeps content intact (exact order pinned for
    // single-end only; paired ingest order is layout-dependent). Streaming
    // and reorder-skipping archives carry no reorder map, where
    // --original-order is documented to fail — skip those cells.
    if has_reorder_map {
        let ordered = case.dir.path().join("ordered.fastq");
        let ocode = DecompressCommand::new(DecompressOptions {
            input_path: archive.clone(),
            output_path: ordered.to_string_lossy().into_owned(),
            original_order: true,
            force_overwrite: true,
            ..Default::default()
        })
        .execute();
        assert_eq!(ocode, 0, "original-order decompress failed for {topology:?} × {mode:?}");
        let ordered_reads = parse_fastq(&ordered);
        assert_eq!(
            sorted_content(&ordered_reads),
            sorted_content(&case.reads),
            "original-order content mismatch for {topology:?} × {mode:?}"
        );
        if matches!(topology, Topology::Single) {
            assert_eq!(
                ordered_reads.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
                case.reads.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
                "single-end original order must equal input order for {mode:?}"
            );
        }
    }
}

macro_rules! matrix_test {
    ($fn_name:ident, $topology:expr, $mode:expr) => {
        #[test]
        fn $fn_name() {
            run_matrix_cell(&$topology, &$mode);
        }
    };
}

matrix_test!(single_archive, Topology::Single, Mode::Archive);
matrix_test!(single_streaming, Topology::Single, Mode::Streaming);
matrix_test!(single_pipeline, Topology::Single, Mode::Pipeline);
matrix_test!(paired_archive, Topology::Paired, Mode::Archive);
matrix_test!(paired_streaming, Topology::Paired, Mode::Streaming);
matrix_test!(paired_pipeline, Topology::Paired, Mode::Pipeline);
matrix_test!(interleaved_archive, Topology::Interleaved, Mode::Archive);
matrix_test!(interleaved_streaming, Topology::Interleaved, Mode::Streaming);
matrix_test!(interleaved_pipeline, Topology::Interleaved, Mode::Pipeline);

// =============================================================================
// Cross-path equivalence: classic vs pipeline decompression must agree
// =============================================================================

fn decompress_once(archive: &str, out: &Path, use_pipeline: bool) {
    let code = DecompressCommand::new(DecompressOptions {
        input_path: archive.to_string(),
        output_path: out.to_string_lossy().into_owned(),
        use_pipeline,
        force_overwrite: true,
        ..Default::default()
    })
    .execute();
    assert_eq!(code, 0);
}

#[test]
fn range_extraction_agrees_between_classic_and_pipeline_paths() {
    let case = build_case(&Topology::Single, &Mode::Archive);
    let code = CompressCommand::new(case.opts.clone()).execute();
    assert_eq!(code, 0);
    let archive = case.opts.output_path.clone();

    for (start, end) in [(2u64, 4u64), (1, 1), (19, 25), (5, 0)] {
        let classic = case.dir.path().join(format!("classic_{start}_{end}.fastq"));
        let piped = case.dir.path().join(format!("pipe_{start}_{end}.fastq"));

        let c1 = DecompressCommand::new(DecompressOptions {
            input_path: archive.clone(),
            output_path: classic.to_string_lossy().into_owned(),
            range_start: start,
            range_end: end,
            force_overwrite: true,
            ..Default::default()
        })
        .execute();
        let c2 = DecompressCommand::new(DecompressOptions {
            input_path: archive.clone(),
            output_path: piped.to_string_lossy().into_owned(),
            range_start: start,
            range_end: end,
            use_pipeline: true,
            force_overwrite: true,
            ..Default::default()
        })
        .execute();
        assert_eq!((c1, c2), (0, 0), "range {start}:{end} exit codes diverge");

        let a = parse_fastq(&classic);
        let b = parse_fastq(&piped);
        assert_eq!(
            a.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
            b.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
            "range {start}:{end} selected different reads"
        );

        // Expected selection per 1-based inclusive semantics.
        let expected: Vec<String> = case
            .reads
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let id1 = *i as u64 + 1;
                id1 >= start && (end == 0 || id1 <= end)
            })
            .map(|(_, r)| r.id.clone())
            .collect();
        assert_eq!(
            a.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
            expected,
            "range {start}:{end} selection deviates from documented semantics"
        );
    }
}

#[test]
fn pipeline_decompress_matches_classic_on_archive_and_streaming_inputs() {
    for mode in [Mode::Archive, Mode::Streaming] {
        let case = build_case(&Topology::Single, &mode);
        let code = CompressCommand::new(case.opts.clone()).execute();
        assert_eq!(code, 0);
        let archive = case.opts.output_path.clone();

        let a_path = case.dir.path().join("classic.fastq");
        let b_path = case.dir.path().join("piped.fastq");
        decompress_once(&archive, &a_path, false);
        decompress_once(&archive, &b_path, true);

        assert_eq!(
            sorted_content(&parse_fastq(&a_path)),
            sorted_content(&parse_fastq(&b_path)),
            "classic and pipeline decompression disagree for {mode:?}"
        );
    }
}
