// =============================================================================
// fqc-rust - Compressed Input Format Tests
// =============================================================================
// The gz/bz2/xz/zstd input decoders had no real coverage: only extension
// detection was tested. These tests build actual compressed FASTQ files and
// push them through the full compress command, plus corrupt/truncated
// streams must fail cleanly.
// =============================================================================

use std::io::Write as _;

use fqc::commands::compress::{CompressCommand, CompressOptions};
use fqc::types::*;

// =============================================================================
// Helpers
// =============================================================================

fn make_records(n: usize, len: usize) -> Vec<ReadRecord> {
    let bases = b"ACGT";
    (0..n)
        .map(|i| {
            let seq: String = (0..len).map(|j| bases[(i + j) % 4] as char).collect();
            let qual = "I".repeat(len);
            ReadRecord::new(format!("read_{}", i), seq, qual)
        })
        .collect()
}

fn write_fastq_bytes(path: &std::path::Path, records: &[ReadRecord]) {
    let mut out = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    for record in records {
        fqc::fastq::parser::write_record(&mut out, record).unwrap();
    }
    out.flush().unwrap();
}

fn read_fastq_records(path: &std::path::Path) -> Vec<ReadRecord> {
    let file = std::fs::File::open(path).unwrap();
    let mut parser = fqc::fastq::parser::FastqParser::new(std::io::BufReader::new(file));
    parser.collect_all().unwrap()
}

/// Compress a (possibly compressed) FASTQ to an archive and decompress back,
/// asserting per-read equality.
fn compressed_input_roundtrip(compressed_input: &std::path::Path, expected: &[ReadRecord]) {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("out.fqc");
    let restored = dir.path().join("restored.fastq");

    let opts = CompressOptions {
        input_path: compressed_input.to_string_lossy().to_string(),
        output_path: archive.to_string_lossy().to_string(),
        show_progress: false,
        force_overwrite: true,
        ..Default::default()
    };
    let code = CompressCommand::new(opts).execute();
    assert_eq!(code, 0, "compressing {} should succeed", compressed_input.display());

    // Decompress via reader API.
    let mut reader = fqc::archive::reader::FqcReader::open(archive.to_string_lossy().as_ref()).unwrap();
    let flags = reader.global_header.flags;
    let config = fqc::algo::block_compressor::BlockCompressorConfig {
        read_length_class: fqc::archive::format::get_read_length_class(flags),
        quality_mode: fqc::archive::format::get_quality_mode(flags),
        id_mode: fqc::archive::format::get_id_mode(flags),
        ..Default::default()
    };
    let mut compressor = fqc::algo::block_compressor::BlockCompressor::new(config);

    let mut out = std::io::BufWriter::new(std::fs::File::create(&restored).unwrap());
    for block_id in 0..reader.block_count() {
        let bd = reader.read_block(block_id as u32).unwrap();
        let bh = &bd.header;
        let dec = compressor
            .decompress_raw(
                bh.block_id,
                bh.uncompressed_count,
                bh.uniform_read_length,
                bh.codec_ids,
                bh.codec_seq,
                bh.codec_qual,
                bh.codec_aux,
                &bd.ids_data,
                &bd.seq_data,
                &bd.qual_data,
                &bd.aux_data,
            )
            .unwrap();
        for read in &dec.reads {
            fqc::fastq::parser::write_record(&mut out, read).unwrap();
        }
    }
    drop(out);

    let restored_records = read_fastq_records(&restored);
    assert_eq!(restored_records.len(), expected.len());
    for (i, (orig, rest)) in expected.iter().zip(restored_records.iter()).enumerate() {
        assert_eq!(orig.id, rest.id, "ID mismatch at read {i}");
        assert_eq!(orig.sequence, rest.sequence, "Sequence mismatch at read {i}");
        assert_eq!(orig.quality, rest.quality, "Quality mismatch at read {i}");
    }
}

// =============================================================================
// Real compressed inputs
// =============================================================================

#[test]
fn gzip_input_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("in.fastq");
    let gz = dir.path().join("in.fastq.gz");
    let records = make_records(20, 150);
    write_fastq_bytes(&plain, &records);

    let file = std::fs::File::create(&gz).unwrap();
    let mut enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    enc.write_all(&std::fs::read(&plain).unwrap()).unwrap();
    enc.finish().unwrap();

    compressed_input_roundtrip(&gz, &records);
}

#[test]
fn bzip2_input_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("in.fastq");
    let bz2 = dir.path().join("in.fastq.bz2");
    let records = make_records(20, 150);
    write_fastq_bytes(&plain, &records);

    let file = std::fs::File::create(&bz2).unwrap();
    let mut enc = bzip2::write::BzEncoder::new(file, bzip2::Compression::best());
    enc.write_all(&std::fs::read(&plain).unwrap()).unwrap();
    enc.finish().unwrap();

    compressed_input_roundtrip(&bz2, &records);
}

#[test]
fn xz_input_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("in.fastq");
    let xz = dir.path().join("in.fastq.xz");
    let records = make_records(20, 150);
    write_fastq_bytes(&plain, &records);

    let file = std::fs::File::create(&xz).unwrap();
    let mut enc = xz2::write::XzEncoder::new(file, 6);
    enc.write_all(&std::fs::read(&plain).unwrap()).unwrap();
    enc.finish().unwrap();

    compressed_input_roundtrip(&xz, &records);
}

#[test]
fn zstd_input_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("in.fastq");
    let zst = dir.path().join("in.fastq.zst");
    let records = make_records(20, 150);
    write_fastq_bytes(&plain, &records);

    let file = std::fs::File::create(&zst).unwrap();
    let mut enc = zstd::Encoder::new(file, 3).unwrap();
    enc.write_all(&std::fs::read(&plain).unwrap()).unwrap();
    enc.finish().unwrap();

    compressed_input_roundtrip(&zst, &records);
}

// =============================================================================
// Corrupt / truncated compressed streams
// =============================================================================

#[test]
fn truncated_gzip_input_fails_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let gz = dir.path().join("truncated.fastq.gz");
    let records = make_records(10, 100);
    let plain = dir.path().join("in.fastq");
    write_fastq_bytes(&plain, &records);
    let file = std::fs::File::create(&gz).unwrap();
    let mut enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    enc.write_all(&std::fs::read(&plain).unwrap()).unwrap();
    enc.finish().unwrap();

    // Truncate to half: the gzip stream is now corrupt.
    let bytes = std::fs::read(&gz).unwrap();
    std::fs::write(&gz, &bytes[..bytes.len() / 2]).unwrap();

    let opts = CompressOptions {
        input_path: gz.to_string_lossy().to_string(),
        output_path: dir.path().join("out.fqc").to_string_lossy().to_string(),
        show_progress: false,
        force_overwrite: true,
        ..Default::default()
    };
    let code = CompressCommand::new(opts).execute();
    assert_ne!(code, 0, "truncated gzip input must fail");
}

#[test]
fn tiny_garbage_gzip_header_fails_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let gz = dir.path().join("garbage.fastq.gz");
    // 4 bytes that look like a gzip header but are not a valid stream.
    std::fs::write(&gz, [0x1f, 0x8b, 0x08, 0x00]).unwrap();

    let opts = CompressOptions {
        input_path: gz.to_string_lossy().to_string(),
        output_path: dir.path().join("out.fqc").to_string_lossy().to_string(),
        show_progress: false,
        force_overwrite: true,
        ..Default::default()
    };
    let code = CompressCommand::new(opts).execute();
    assert_ne!(code, 0, "garbage gzip stream must fail");
}

#[test]
fn detect_format_from_bytes_magic_detection() {
    use fqc::io::compressed_stream::{detect_format_from_bytes, CompressionFormat};
    assert_eq!(detect_format_from_bytes(&[0x1f, 0x8b]), CompressionFormat::Gzip);
    assert_eq!(detect_format_from_bytes(b"BZh"), CompressionFormat::Bzip2);
    assert_eq!(
        detect_format_from_bytes(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]),
        CompressionFormat::Xz
    );
    assert_eq!(
        detect_format_from_bytes(&[0x28, 0xb5, 0x2f, 0xfd]),
        CompressionFormat::Zstd
    );
    assert_eq!(detect_format_from_bytes(b"@read"), CompressionFormat::Plain);
    // Short buffers must not panic and must not misdetect.
    assert_eq!(detect_format_from_bytes(&[0x1f]), CompressionFormat::Plain);
    assert_eq!(detect_format_from_bytes(b""), CompressionFormat::Plain);
}
