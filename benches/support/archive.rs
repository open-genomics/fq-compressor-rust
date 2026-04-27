use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use fqc::algo::block_compressor::{compute_block_checksum, BlockCompressor, BlockCompressorConfig};
use fqc::fastq::parser::FastqParser;
use fqc::format::{build_flags, get_id_mode, get_quality_mode, get_read_length_class, GlobalHeader};
use fqc::fqc_reader::FqcReader;
use fqc::fqc_writer::FqcWriter;
use fqc::types::{recommended_block_size, IdMode, PeLayout, QualityMode, ReadLengthClass, ReadRecord};
use xxhash_rust::xxh64::Xxh64;

const INPUT_REPEAT_COUNT: usize = 8;
const INPUT_FIXTURE_NAME: &str = "benchmark-input.fastq";
const ARCHIVE_FIXTURE_NAME: &str = "benchmark-input.fqc";

static INPUT_RECORDS: OnceLock<Vec<ReadRecord>> = OnceLock::new();
static ARCHIVE_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn input_size_bytes() -> u64 {
    fs::metadata(bench_input_path()).unwrap().len()
}

pub fn archive_size_bytes() -> u64 {
    fs::metadata(bench_archive_path()).unwrap().len()
}

pub fn bench_compress_roundtrip(output_name: &str) -> u64 {
    let records = input_records();
    let output_path = bench_data_dir().join(output_name);
    write_archive(records, &output_path, INPUT_FIXTURE_NAME);
    let restored = decompress_archive(&output_path);

    assert_eq!(records.len(), restored.len());
    assert_eq!(compute_block_checksum(records), compute_block_checksum(&restored));

    restored.len() as u64
}

pub fn bench_verify_archive() -> u64 {
    verify_archive(bench_archive_path())
}

fn input_records() -> &'static [ReadRecord] {
    INPUT_RECORDS
        .get_or_init(|| {
            let file = File::open(bench_input_path()).unwrap();
            let reader = BufReader::new(file);
            let mut parser = FastqParser::new(reader);
            parser.collect_all().unwrap()
        })
        .as_slice()
}

fn bench_input_path() -> PathBuf {
    let path = bench_data_dir().join(INPUT_FIXTURE_NAME);
    if path.exists() {
        return path;
    }

    let seed = fs::read(test_data_dir().join("test_se.fastq")).unwrap();
    let mut expanded = Vec::with_capacity(seed.len() * INPUT_REPEAT_COUNT);
    for _ in 0..INPUT_REPEAT_COUNT {
        expanded.extend_from_slice(&seed);
    }
    fs::write(&path, expanded).unwrap();
    path
}

fn bench_archive_path() -> &'static Path {
    ARCHIVE_PATH
        .get_or_init(|| {
            let archive_path = bench_data_dir().join(ARCHIVE_FIXTURE_NAME);
            if !archive_path.exists() {
                write_archive(input_records(), &archive_path, INPUT_FIXTURE_NAME);
            }
            archive_path
        })
        .as_path()
}

fn bench_data_dir() -> PathBuf {
    let dir = repo_root().join("target").join("bench-data");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn test_data_dir() -> PathBuf {
    repo_root().join("tests").join("data")
}

fn write_archive(records: &[ReadRecord], output_path: &Path, original_filename: &str) {
    let length_class = ReadLengthClass::Short;
    let flags = build_flags(
        false,
        true,
        QualityMode::Lossless,
        IdMode::Exact,
        false,
        PeLayout::Interleaved,
        length_class,
        false,
    );

    let mut writer = FqcWriter::create(output_path.to_str().unwrap()).unwrap();
    let header = GlobalHeader::new(flags, records.len() as u64, original_filename, 0);
    writer.write_global_header(&header).unwrap();

    let compressor = BlockCompressor::new(BlockCompressorConfig {
        read_length_class: length_class,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        zstd_level: 3,
        ..Default::default()
    });

    for (block_id, chunk) in records.chunks(recommended_block_size(length_class)).enumerate() {
        let compressed = compressor.compress(chunk, block_id as u32).unwrap();
        writer.write_block(&compressed).unwrap();
    }

    writer.finalize().unwrap();
}

fn decompress_archive(path: &Path) -> Vec<ReadRecord> {
    let mut reader = FqcReader::open(path.to_str().unwrap()).unwrap();
    let flags = reader.global_header.flags;
    let compressor = BlockCompressor::new(BlockCompressorConfig {
        read_length_class: get_read_length_class(flags),
        quality_mode: get_quality_mode(flags),
        id_mode: get_id_mode(flags),
        ..Default::default()
    });

    let mut restored = Vec::new();
    for block_id in 0..reader.block_count() {
        let block = reader.read_block(block_id as u32).unwrap();
        let header = &block.header;
        let decompressed = compressor
            .decompress_raw(
                header.block_id,
                header.uncompressed_count,
                header.uniform_read_length,
                header.codec_seq,
                header.codec_qual,
                &block.ids_data,
                &block.seq_data,
                &block.qual_data,
                &block.aux_data,
            )
            .unwrap();
        restored.extend(decompressed.reads);
    }

    restored
}

fn verify_archive(path: &Path) -> u64 {
    let mut reader = FqcReader::open(path.to_str().unwrap()).unwrap();
    assert!(reader.footer.is_valid());

    if reader.footer.global_checksum != 0 {
        let mut global_hasher = Xxh64::new(0);
        global_hasher.update(&reader.global_header.flags.to_le_bytes());
        for block_id in 0..reader.block_count() {
            let block = reader.read_block(block_id as u32).unwrap();
            global_hasher.update(&block.ids_data);
            global_hasher.update(&block.seq_data);
            global_hasher.update(&block.qual_data);
            global_hasher.update(&block.aux_data);
        }
        assert_eq!(global_hasher.digest(), reader.footer.global_checksum);
    }

    let flags = reader.global_header.flags;
    let compressor = BlockCompressor::new(BlockCompressorConfig {
        read_length_class: get_read_length_class(flags),
        quality_mode: get_quality_mode(flags),
        id_mode: get_id_mode(flags),
        ..Default::default()
    });

    let mut verified_reads = 0u64;
    for block_id in 0..reader.block_count() {
        let block = reader.read_block(block_id as u32).unwrap();
        let header = &block.header;
        let decompressed = compressor
            .decompress_raw(
                header.block_id,
                header.uncompressed_count,
                header.uniform_read_length,
                header.codec_seq,
                header.codec_qual,
                &block.ids_data,
                &block.seq_data,
                &block.qual_data,
                &block.aux_data,
            )
            .unwrap();

        if header.block_xxhash64 != 0 {
            assert_eq!(compute_block_checksum(&decompressed.reads), header.block_xxhash64);
        }
        for read in &decompressed.reads {
            assert!(read.is_valid());
        }
        verified_reads += decompressed.reads.len() as u64;
    }

    assert_eq!(verified_reads, reader.total_read_count());
    verified_reads
}
