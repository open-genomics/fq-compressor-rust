// =============================================================================
// fqc-rust - FASTQ Parser
// =============================================================================

use crate::error::{FqcError, Result};
use crate::types::ReadRecord;
use std::io::{BufRead, BufReader, Read};

/// FASTQ file parser (supports plain and gzip-compressed files)
pub struct FastqParser<R: BufRead> {
    reader: R,
    line_buf: String,
}

impl<R: BufRead> FastqParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(512),
        }
    }

    /// Read the next FASTQ record.
    /// Returns None on EOF.
    pub fn next_record(&mut self) -> Result<Option<ReadRecord>> {
        // Read ID line
        self.line_buf.clear();
        let bytes = self.reader.read_line(&mut self.line_buf)?;
        if bytes == 0 {
            return Ok(None);
        }

        let id_line = self.line_buf.trim_end();
        if id_line.is_empty() {
            return Ok(None);
        }
        if !id_line.starts_with('@') {
            return Err(FqcError::Parse(format!(
                "Expected '@' at start of FASTQ record, got: {id_line}"
            )));
        }
        let id = id_line[1..].to_string();

        // Read sequence line
        self.line_buf.clear();
        let bytes = self.reader.read_line(&mut self.line_buf)?;
        if bytes == 0 {
            return Err(FqcError::Parse("Unexpected EOF reading sequence".to_string()));
        }
        let sequence = self.line_buf.trim_end().to_string();

        // Read plus line
        self.line_buf.clear();
        let bytes = self.reader.read_line(&mut self.line_buf)?;
        if bytes == 0 {
            return Err(FqcError::Parse("Unexpected EOF reading plus line".to_string()));
        }
        let plus_line = self.line_buf.trim_end();
        if !plus_line.starts_with('+') {
            return Err(FqcError::Parse(format!(
                "Expected '+' line in FASTQ record, got: {plus_line}"
            )));
        }

        // Read quality line
        self.line_buf.clear();
        let bytes = self.reader.read_line(&mut self.line_buf)?;
        if bytes == 0 {
            return Err(FqcError::Parse("Unexpected EOF reading quality".to_string()));
        }
        let quality = self.line_buf.trim_end().to_string();

        if sequence.len() != quality.len() {
            return Err(FqcError::Parse(format!(
                "Sequence length {} != quality length {} for read '{}'",
                sequence.len(),
                quality.len(),
                id
            )));
        }

        Ok(Some(ReadRecord::new(id, sequence, quality)))
    }

    /// Collect all records into a Vec
    pub fn collect_all(&mut self) -> Result<Vec<ReadRecord>> {
        let mut records = Vec::new();
        while let Some(record) = self.next_record()? {
            records.push(record);
        }
        Ok(records)
    }
}

/// Open a FASTQ file (plain or gzip compressed) for reading.
/// Detects gzip by magic bytes (0x1f 0x8b) or `.gz` extension.
pub fn open_fastq(path: &str) -> Result<FastqParser<BufReader<Box<dyn Read + Send>>>> {
    use flate2::read::GzDecoder;
    use std::fs::File;

    let is_gz = path.ends_with(".gz") || path.ends_with(".gzip") || {
        // Peek at first 2 bytes for gzip magic
        if let Ok(mut f) = File::open(path) {
            let mut magic = [0u8; 2];
            let _ = std::io::Read::read_exact(&mut f, &mut magic);
            magic == [0x1f, 0x8b]
        } else {
            false
        }
    };

    let file = File::open(path)?;

    if is_gz {
        let decoder = GzDecoder::new(file);
        let reader: Box<dyn Read + Send> = Box::new(decoder);
        Ok(FastqParser::new(BufReader::new(reader)))
    } else {
        let reader: Box<dyn Read + Send> = Box::new(file);
        Ok(FastqParser::new(BufReader::new(reader)))
    }
}

/// Open stdin for FASTQ reading (plain text only)
pub fn open_fastq_stdin() -> FastqParser<BufReader<Box<dyn Read + Send>>> {
    let reader: Box<dyn Read + Send> = Box::new(std::io::stdin());
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

/// Open paired-end FASTQ files for interleaved reading
pub fn open_fastq_paired(
    path1: &str,
    path2: &str,
) -> Result<PairedFastqReader<BufReader<Box<dyn Read + Send>>, BufReader<Box<dyn Read + Send>>>> {
    let r1 = open_fastq(path1)?;
    let r2 = open_fastq(path2)?;
    Ok(PairedFastqReader::new(r1, r2))
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
