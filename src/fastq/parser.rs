// =============================================================================
// fqc-rust - FASTQ Parser
// =============================================================================

use crate::error::{FqcError, Result};
use crate::types::ReadRecord;
use std::io::{BufRead, BufReader, Read};

// =============================================================================
// Parser Options
// =============================================================================

/// Configuration for FASTQ parsing behavior
#[derive(Debug, Clone)]
pub struct ParserOptions {
    /// Validate DNA sequence characters (A/C/G/T/N only)
    pub validate_sequence: bool,
    /// Validate quality value range (Phred+33: '!' to '~')
    pub validate_quality: bool,
    /// Collect statistics while parsing
    pub collect_stats: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            validate_sequence: false,
            validate_quality: false,
            collect_stats: false,
        }
    }
}

// =============================================================================
// Parser Statistics
// =============================================================================

/// Statistics collected during FASTQ parsing
#[derive(Debug, Clone, Default)]
pub struct ParserStats {
    pub total_records: u64,
    pub total_bases: u64,
    pub min_length: usize,
    pub max_length: usize,
    pub total_n_count: u64,
    pub total_bytes_read: u64,
}

impl ParserStats {
    pub fn avg_length(&self) -> f64 {
        if self.total_records == 0 { 0.0 }
        else { self.total_bases as f64 / self.total_records as f64 }
    }

    pub fn n_fraction(&self) -> f64 {
        if self.total_bases == 0 { 0.0 }
        else { self.total_n_count as f64 / self.total_bases as f64 }
    }

    fn update(&mut self, record: &ReadRecord, raw_bytes: usize) {
        self.total_records += 1;
        let len = record.sequence.len();
        self.total_bases += len as u64;
        if self.total_records == 1 {
            self.min_length = len;
            self.max_length = len;
        } else {
            self.min_length = self.min_length.min(len);
            self.max_length = self.max_length.max(len);
        }
        self.total_n_count += record.sequence.bytes().filter(|&b| b == b'N' || b == b'n').count() as u64;
        self.total_bytes_read += raw_bytes as u64;
    }
}

// =============================================================================
// Sequence / Quality Validation
// =============================================================================

/// Check if a byte is a valid DNA base
fn is_valid_base(b: u8) -> bool {
    matches!(b, b'A' | b'C' | b'G' | b'T' | b'N' | b'a' | b'c' | b'g' | b't' | b'n')
}

/// Validate DNA sequence
pub fn validate_sequence(seq: &str) -> std::result::Result<(), String> {
    for (i, b) in seq.bytes().enumerate() {
        if !is_valid_base(b) {
            return Err(format!("Invalid base '{}' at position {}", b as char, i));
        }
    }
    Ok(())
}

/// Validate quality string (Phred+33: ASCII 33-126)
pub fn validate_quality_string(qual: &str) -> std::result::Result<(), String> {
    for (i, b) in qual.bytes().enumerate() {
        if b < 33 || b > 126 {
            return Err(format!("Invalid quality value {} at position {} (expected 33-126)", b, i));
        }
    }
    Ok(())
}

// =============================================================================
// FastqParser
// =============================================================================

/// FASTQ file parser with optional validation and statistics
pub struct FastqParser<R: BufRead> {
    reader: R,
    line_buf: String,
    options: ParserOptions,
    stats: ParserStats,
    line_number: u64,
    record_number: u64,
}

impl<R: BufRead> FastqParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(512),
            options: ParserOptions::default(),
            stats: ParserStats::default(),
            line_number: 0,
            record_number: 0,
        }
    }

    pub fn with_options(reader: R, options: ParserOptions) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(512),
            options,
            stats: ParserStats::default(),
            line_number: 0,
            record_number: 0,
        }
    }

    /// Get current parser statistics
    pub fn stats(&self) -> &ParserStats {
        &self.stats
    }

    /// Get current line number (1-based)
    pub fn line_number(&self) -> u64 {
        self.line_number
    }

    /// Get current record number (1-based)
    pub fn record_number(&self) -> u64 {
        self.record_number
    }

    fn read_line_tracked(&mut self) -> Result<usize> {
        self.line_buf.clear();
        let bytes = self.reader.read_line(&mut self.line_buf)?;
        if bytes > 0 {
            self.line_number += 1;
        }
        Ok(bytes)
    }

    /// Read the next FASTQ record.
    /// Returns None on EOF.
    pub fn next_record(&mut self) -> Result<Option<ReadRecord>> {
        // Read ID line
        let bytes1 = self.read_line_tracked()?;
        if bytes1 == 0 {
            return Ok(None);
        }

        let id_line = self.line_buf.trim_end();
        if id_line.is_empty() {
            return Ok(None);
        }
        if !id_line.starts_with('@') {
            return Err(FqcError::Parse(format!(
                "Line {}: Expected '@' at start of FASTQ record, got: {}",
                self.line_number, id_line
            )));
        }
        let id = id_line[1..].to_string();
        let mut raw_bytes = bytes1;

        // Read sequence line
        let bytes2 = self.read_line_tracked()?;
        if bytes2 == 0 {
            return Err(FqcError::Parse(format!(
                "Line {}: Unexpected EOF reading sequence for record '{}'",
                self.line_number, id
            )));
        }
        let sequence = self.line_buf.trim_end().to_string();
        raw_bytes += bytes2;

        // Read plus line
        let bytes3 = self.read_line_tracked()?;
        if bytes3 == 0 {
            return Err(FqcError::Parse(format!(
                "Line {}: Unexpected EOF reading plus line for record '{}'",
                self.line_number, id
            )));
        }
        let plus_line = self.line_buf.trim_end();
        if !plus_line.starts_with('+') {
            return Err(FqcError::Parse(format!(
                "Line {}: Expected '+' line in FASTQ record, got: {}",
                self.line_number, plus_line
            )));
        }
        raw_bytes += bytes3;

        // Read quality line
        let bytes4 = self.read_line_tracked()?;
        if bytes4 == 0 {
            return Err(FqcError::Parse(format!(
                "Line {}: Unexpected EOF reading quality for record '{}'",
                self.line_number, id
            )));
        }
        let quality = self.line_buf.trim_end().to_string();
        raw_bytes += bytes4;

        if sequence.len() != quality.len() {
            return Err(FqcError::Parse(format!(
                "Record {} ('{}'): sequence length {} != quality length {}",
                self.record_number + 1, id, sequence.len(), quality.len()
            )));
        }

        // Optional validation
        if self.options.validate_sequence {
            if let Err(msg) = validate_sequence(&sequence) {
                return Err(FqcError::Parse(format!(
                    "Record {} ('{}'): {}", self.record_number + 1, id, msg
                )));
            }
        }
        if self.options.validate_quality {
            if let Err(msg) = validate_quality_string(&quality) {
                return Err(FqcError::Parse(format!(
                    "Record {} ('{}'): {}", self.record_number + 1, id, msg
                )));
            }
        }

        let record = ReadRecord::new(id, sequence, quality);
        self.record_number += 1;

        if self.options.collect_stats {
            self.stats.update(&record, raw_bytes);
        }

        Ok(Some(record))
    }

    /// Collect all records into a Vec
    pub fn collect_all(&mut self) -> Result<Vec<ReadRecord>> {
        let mut records = Vec::new();
        while let Some(record) = self.next_record()? {
            records.push(record);
        }
        Ok(records)
    }

    /// Read a chunk of up to `max_records` records
    pub fn read_chunk(&mut self, max_records: usize) -> Result<Vec<ReadRecord>> {
        let mut records = Vec::with_capacity(max_records);
        for _ in 0..max_records {
            match self.next_record()? {
                Some(r) => records.push(r),
                None => break,
            }
        }
        Ok(records)
    }

    /// Sample up to `n` records evenly spaced from the input.
    /// Reads all records but only keeps sampled ones.
    pub fn sample_records(&mut self, n: usize) -> Result<Vec<ReadRecord>> {
        let all = self.collect_all()?;
        if all.len() <= n {
            return Ok(all);
        }
        let step = all.len() as f64 / n as f64;
        let sampled: Vec<ReadRecord> = (0..n)
            .map(|i| all[(i as f64 * step) as usize].clone())
            .collect();
        Ok(sampled)
    }

    /// Apply a callback to each record (avoids collecting all into memory)
    pub fn for_each<F>(&mut self, mut f: F) -> Result<u64>
    where
        F: FnMut(&ReadRecord) -> Result<()>,
    {
        let mut count = 0u64;
        while let Some(record) = self.next_record()? {
            f(&record)?;
            count += 1;
        }
        Ok(count)
    }
}

// =============================================================================
// File Opening (delegates to io::compressed_stream)
// =============================================================================

/// Open a FASTQ file (plain or compressed) for reading.
/// Auto-detects gzip, bzip2, xz, and zstd by magic bytes or extension.
pub fn open_fastq(path: &str) -> Result<FastqParser<BufReader<Box<dyn Read + Send>>>> {
    let reader = crate::io::compressed_stream::open_buffered_reader(path)?;
    Ok(FastqParser::new(reader))
}

/// Open stdin for FASTQ reading (plain text only)
pub fn open_fastq_stdin() -> FastqParser<BufReader<Box<dyn Read + Send>>> {
    let reader = crate::io::compressed_stream::open_stdin_reader();
    FastqParser::new(BufReader::new(reader))
}

/// Paired-end interleaved reader: alternates R1/R2 records
pub struct PairedFastqReader<R1: BufRead, R2: BufRead> {
    r1: FastqParser<R1>,
    r2: FastqParser<R2>,
}

impl<R1: BufRead, R2: BufRead> PairedFastqReader<R1, R2> {
    pub fn new(r1: FastqParser<R1>, r2: FastqParser<R2>) -> Self {
        Self { r1, r2 }
    }

    pub fn next_pair(&mut self) -> Result<Option<(ReadRecord, ReadRecord)>> {
        match (self.r1.next_record()?, self.r2.next_record()?) {
            (Some(a), Some(b)) => Ok(Some((a, b))),
            (Some(_), None) => {
                log::warn!("R1 has more reads than R2, truncating");
                Ok(None)
            }
            (None, Some(_)) => {
                log::warn!("R2 has more reads than R1, truncating");
                Ok(None)
            }
            (None, None) => Ok(None),
        }
    }

    /// Collect all records interleaved: R1_0, R2_0, R1_1, R2_1, ...
    pub fn collect_all_interleaved(&mut self) -> Result<Vec<ReadRecord>> {
        let mut records = Vec::new();
        while let Some((a, b)) = self.next_pair()? {
            records.push(a);
            records.push(b);
        }
        Ok(records)
    }

    /// Collect all records in consecutive PE layout: all R1 reads, then all R2 reads.
    pub fn collect_all_consecutive(&mut self) -> Result<Vec<ReadRecord>> {
        let mut r1_reads = Vec::new();
        let mut r2_reads = Vec::new();

        while let Some((a, b)) = self.next_pair()? {
            r1_reads.push(a);
            r2_reads.push(b);
        }

        r1_reads.extend(r2_reads);
        Ok(r1_reads)
    }
}

/// Open paired-end FASTQ files for reading
pub fn open_fastq_paired(
    path1: &str,
    path2: &str,
) -> Result<PairedFastqReader<BufReader<Box<dyn Read + Send>>, BufReader<Box<dyn Read + Send>>>> {
    let r1 = open_fastq(path1)?;
    let r2 = open_fastq(path2)?;
    Ok(PairedFastqReader::new(r1, r2))
}

// =============================================================================
// Interleaved PE Parser (single file, alternating R1/R2)
// =============================================================================

/// Parser for interleaved paired-end FASTQ (R1, R2, R1, R2, ... in one file)
pub struct InterleavedPeParser<R: BufRead> {
    parser: FastqParser<R>,
}

impl<R: BufRead> InterleavedPeParser<R> {
    pub fn new(parser: FastqParser<R>) -> Self {
        Self { parser }
    }

    /// Read the next R1/R2 pair from the interleaved stream
    pub fn next_pair(&mut self) -> Result<Option<(ReadRecord, ReadRecord)>> {
        let r1 = match self.parser.next_record()? {
            Some(r) => r,
            None => return Ok(None),
        };
        let r2 = match self.parser.next_record()? {
            Some(r) => r,
            None => return Err(FqcError::Parse(
                "Interleaved PE file has odd number of records (missing R2 mate)".to_string()
            )),
        };
        Ok(Some((r1, r2)))
    }

    pub fn collect_all_interleaved(&mut self) -> Result<Vec<ReadRecord>> {
        let mut records = Vec::new();
        while let Some((r1, r2)) = self.next_pair()? {
            records.push(r1);
            records.push(r2);
        }
        Ok(records)
    }

    pub fn collect_all_consecutive(&mut self) -> Result<Vec<ReadRecord>> {
        let mut r1_reads = Vec::new();
        let mut r2_reads = Vec::new();
        while let Some((r1, r2)) = self.next_pair()? {
            r1_reads.push(r1);
            r2_reads.push(r2);
        }
        r1_reads.extend(r2_reads);
        Ok(r1_reads)
    }
}

/// Open an interleaved paired-end FASTQ file
pub fn open_fastq_interleaved(
    path: &str,
) -> Result<InterleavedPeParser<BufReader<Box<dyn Read + Send>>>> {
    let parser = open_fastq(path)?;
    Ok(InterleavedPeParser::new(parser))
}

// =============================================================================
// PE ID Validation
// =============================================================================

/// Check if two read IDs form a valid paired-end pair.
/// Common conventions: "read/1" + "read/2", "read 1:..." + "read 2:...", identical IDs
pub fn validate_pe_pair_ids(id1: &str, id2: &str) -> bool {
    if id1 == id2 {
        return true;
    }
    // Try /1 /2 suffix convention
    if id1.ends_with("/1") && id2.ends_with("/2") {
        return id1[..id1.len()-2] == id2[..id2.len()-2];
    }
    // Try space-separated comment with 1:/2: prefix
    if let (Some(p1), Some(p2)) = (id1.find(' '), id2.find(' ')) {
        let base1 = &id1[..p1];
        let base2 = &id2[..p2];
        if base1 == base2 {
            let suffix1 = &id1[p1+1..];
            let suffix2 = &id2[p2+1..];
            if suffix1.starts_with("1:") && suffix2.starts_with("2:") {
                return true;
            }
        }
    }
    false
}

/// Detect if a FASTQ file is in interleaved paired-end format by checking the first few records.
pub fn detect_interleaved_format(path: &str) -> Result<bool> {
    let mut parser = open_fastq(path)?;
    let mut pairs_checked = 0;
    for _ in 0..4 {
        let r1 = match parser.next_record()? {
            Some(r) => r,
            None => break,
        };
        let r2 = match parser.next_record()? {
            Some(r) => r,
            None => return Ok(false), // Odd number of reads
        };
        if !validate_pe_pair_ids(&r1.id, &r2.id) {
            return Ok(false);
        }
        pairs_checked += 1;
    }
    Ok(pairs_checked > 0)
}

/// Write a single FASTQ record to a writer
pub fn write_record<W: std::io::Write + ?Sized>(w: &mut W, record: &ReadRecord) -> Result<()> {
    w.write_all(b"@")?;
    w.write_all(record.id.as_bytes())?;
    w.write_all(b"\n")?;
    w.write_all(record.sequence.as_bytes())?;
    w.write_all(b"\n+\n")?;
    w.write_all(record.quality.as_bytes())?;
    w.write_all(b"\n")?;
    Ok(())
}
