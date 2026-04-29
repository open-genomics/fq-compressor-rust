use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use fqc::algo::block_compressor::{compute_block_checksum, BlockCompressor, BlockCompressorConfig};
use fqc::fastq::parser::FastqParser;
use fqc::format::{build_flags, get_id_mode, get_quality_mode, get_read_length_class, GlobalHeader};
use fqc::fqc_reader::FqcReader;
use fqc::fqc_writer::FqcWriter;
use fqc::types::{recommended_block_size, IdMode, PeLayout, QualityMode, ReadLengthClass, ReadRecord};
use xxhash_rust::xxh64::Xxh64;

pub fn ensure_repeated_fixture(output_path: &Path, repeat_count: usize) -> PathBuf {
    if output_path.exists() {
        return output_path.to_path_buf();
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let seed = fs::read(test_data_dir().join("test_se.fastq")).unwrap();
    fs::write(output_path, seed.repeat(repeat_count)).unwrap();
    output_path.to_path_buf()
}

#[allow(dead_code)]
pub fn load_fastq_records(path: &Path) -> Vec<ReadRecord> {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let mut parser = FastqParser::new(reader);
    parser.collect_all().unwrap()
}

#[allow(dead_code)]
pub fn write_archive(records: &[ReadRecord], output_path: &Path, original_filename: &str) {
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

#[allow(dead_code)]
pub fn decompress_archive(path: &Path) -> Vec<ReadRecord> {
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

#[allow(dead_code)]
pub fn verify_archive(path: &Path) -> u64 {
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

pub fn bench_data_dir() -> PathBuf {
    let dir = repo_root().join("target").join("bench-data");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn test_data_dir() -> PathBuf {
    repo_root().join("tests").join("data")
}
