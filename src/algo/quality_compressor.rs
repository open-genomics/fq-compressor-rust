// =============================================================================
// fqc-rust - Quality Compressor (SCM + Arithmetic Coding)
// =============================================================================

use crate::algo::compressor_traits::QualityCompressor as QualityCompressorTrait;
use crate::error::{FqcError, Result};
use crate::types::{encode_codec, CodecFamily, QualityMode, ReadRecord};

pub const NUM_QUALITY_SYMBOLS: usize = 94; // Phred 0-93 (ASCII 33-126)
pub const NUM_POSITION_BINS: usize = 8;
const MAX_FREQUENCY: u32 = 16383;
const INITIAL_FREQUENCY: u32 = 1;
const ADAPT_INCREMENT: u32 = 8;

// Arithmetic coding constants
const CODE_BITS: u32 = 32;
const TOP_VALUE: u64 = (1u64 << CODE_BITS) - 1;
const FIRST_QUARTER: u64 = TOP_VALUE / 4 + 1;
const HALF: u64 = 2 * FIRST_QUARTER;
const THIRD_QUARTER: u64 = 3 * FIRST_QUARTER;

// =============================================================================
// Adaptive Frequency Model
// =============================================================================

struct AdaptiveModel {
    num_symbols: usize,
    frequencies: Vec<u32>,
    cumulative: Vec<u32>,
}

impl AdaptiveModel {
    fn new(num_symbols: usize) -> Self {
        let mut m = Self {
            num_symbols,
            frequencies: vec![INITIAL_FREQUENCY; num_symbols],
            cumulative: vec![0; num_symbols + 1],
        };
        m.update_cumulative();
        m
    }

    fn get_cumulative(&self, symbol: usize) -> u32 {
        self.cumulative[symbol]
    }

    fn get_total(&self) -> u32 {
        self.cumulative[self.num_symbols]
    }

    fn find_symbol(&self, cum_freq: u32) -> usize {
        let mut low = 0usize;
        let mut high = self.num_symbols;
        while low < high {
            let mid = (low + high) / 2;
            if self.cumulative[mid + 1] <= cum_freq {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low
    }

    fn update(&mut self, symbol: usize) {
        self.frequencies[symbol] += ADAPT_INCREMENT;
        if self.get_total() + ADAPT_INCREMENT > MAX_FREQUENCY {
            self.rescale();
        }
        self.update_cumulative();
    }

    fn reset(&mut self) {
        for f in &mut self.frequencies {
            *f = INITIAL_FREQUENCY;
        }
        self.update_cumulative();
    }

    fn update_cumulative(&mut self) {
        self.cumulative[0] = 0;
        for i in 0..self.num_symbols {
            self.cumulative[i + 1] = self.cumulative[i] + self.frequencies[i];
        }
    }

    fn rescale(&mut self) {
        for f in &mut self.frequencies {
            *f = (*f).div_ceil(2).max(1);
        }
    }
}

// =============================================================================
// Arithmetic Encoder
// =============================================================================

struct ArithmeticEncoder {
    output: Vec<u8>,
    low: u64,
    high: u64,
    bits_to_follow: u32,
    bit_buffer: u8,
    bit_count: u8,
}

impl ArithmeticEncoder {
    fn new() -> Self {
        Self {
            output: Vec::new(),
            low: 0,
            high: TOP_VALUE,
            bits_to_follow: 0,
            bit_buffer: 0,
            bit_count: 0,
        }
    }

    fn encode(&mut self, symbol: usize, model: &AdaptiveModel) {
        let range = self.high - self.low + 1;
        let total = model.get_total() as u64;
        let cum_low = model.get_cumulative(symbol) as u64;
        let cum_high = model.get_cumulative(symbol + 1) as u64;

        self.high = self.low + (range * cum_high) / total - 1;
        self.low += (range * cum_low) / total;

        self.normalize();
    }

    fn finish(&mut self) {
        self.bits_to_follow += 1;
        if self.low < FIRST_QUARTER {
            self.output_bit(0);
        } else {
            self.output_bit(1);
        }
        // Flush remaining bits
        if self.bit_count > 0 {
            self.output.push(self.bit_buffer << (8 - self.bit_count));
        }
    }

    fn get_data(&self) -> &[u8] {
        &self.output
    }

    fn normalize(&mut self) {
        loop {
            if self.high < HALF {
                self.output_bit(0);
            } else if self.low >= HALF {
                self.output_bit(1);
                self.low -= HALF;
                self.high -= HALF;
            } else if self.low >= FIRST_QUARTER && self.high < THIRD_QUARTER {
                self.bits_to_follow += 1;
                self.low -= FIRST_QUARTER;
                self.high -= FIRST_QUARTER;
            } else {
                break;
            }
            self.low *= 2;
            self.high = 2 * self.high + 1;
        }
    }

    fn output_bit(&mut self, bit: u8) {
        self.write_bit(bit);
        while self.bits_to_follow > 0 {
            self.write_bit(1 - bit);
            self.bits_to_follow -= 1;
        }
    }

    fn write_bit(&mut self, bit: u8) {
        self.bit_buffer = (self.bit_buffer << 1) | bit;
        self.bit_count += 1;
        if self.bit_count == 8 {
            self.output.push(self.bit_buffer);
            self.bit_buffer = 0;
            self.bit_count = 0;
        }
    }
}

// =============================================================================
// Arithmetic Decoder
// =============================================================================

struct ArithmeticDecoder<'a> {
    data: &'a [u8],
    low: u64,
    high: u64,
    value: u64,
    bit_pos: usize,
    byte_pos: usize,
}

impl<'a> ArithmeticDecoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut dec = Self {
            data,
            low: 0,
            high: TOP_VALUE,
            value: 0,
            bit_pos: 0,
            byte_pos: 0,
        };
        for _ in 0..CODE_BITS {
            dec.value = (dec.value << 1) | dec.read_bit() as u64;
        }
        dec
    }

    fn decode(&mut self, model: &mut AdaptiveModel) -> usize {
        let range = self.high - self.low + 1;
        let total = model.get_total() as u64;
        let cum_freq = (((self.value - self.low + 1) * total - 1) / range) as u32;

        let symbol = model.find_symbol(cum_freq);

        let cum_low = model.get_cumulative(symbol) as u64;
        let cum_high = model.get_cumulative(symbol + 1) as u64;

        self.high = self.low + (range * cum_high) / total - 1;
        self.low += (range * cum_low) / total;

        self.normalize();
        symbol
    }

    fn normalize(&mut self) {
        loop {
            if self.high < HALF {
                // do nothing
            } else if self.low >= HALF {
                self.value -= HALF;
                self.low -= HALF;
                self.high -= HALF;
            } else if self.low >= FIRST_QUARTER && self.high < THIRD_QUARTER {
                self.value -= FIRST_QUARTER;
                self.low -= FIRST_QUARTER;
                self.high -= FIRST_QUARTER;
            } else {
                break;
            }
            self.low *= 2;
            self.high = 2 * self.high + 1;
            self.value = 2 * self.value + self.read_bit() as u64;
        }
    }

    fn read_bit(&mut self) -> u8 {
        if self.byte_pos >= self.data.len() {
            return 0;
        }
        let bit = (self.data[self.byte_pos] >> (7 - self.bit_pos)) & 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        bit
    }
}

// =============================================================================
// Quality Context Model
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextOrder {
    Order0,
    Order1,
    Order2,
}

struct QualityContextModel {
    order: ContextOrder,
    num_position_bins: usize,
    models: Vec<AdaptiveModel>,
}

impl QualityContextModel {
    fn new(order: ContextOrder, num_position_bins: usize) -> Self {
        let num_contexts = Self::compute_num_contexts(order, num_position_bins);
        let models = (0..num_contexts)
            .map(|_| AdaptiveModel::new(NUM_QUALITY_SYMBOLS))
            .collect();
        Self {
            order,
            num_position_bins,
            models,
        }
    }

    fn compute_num_contexts(order: ContextOrder, num_position_bins: usize) -> usize {
        let qual_contexts = match order {
            ContextOrder::Order0 => 1,
            ContextOrder::Order1 => NUM_QUALITY_SYMBOLS,
            ContextOrder::Order2 => NUM_QUALITY_SYMBOLS * NUM_QUALITY_SYMBOLS,
        };
        qual_contexts * num_position_bins
    }

    fn context_index(&self, prev1: usize, prev2: usize, pos_bin: usize) -> usize {
        let qual_ctx = match self.order {
            ContextOrder::Order0 => 0,
            ContextOrder::Order1 => prev1,
            ContextOrder::Order2 => prev1 * NUM_QUALITY_SYMBOLS + prev2,
        };
        qual_ctx * self.num_position_bins + pos_bin
    }

    fn get_model(&mut self, prev1: usize, prev2: usize, pos_bin: usize) -> &mut AdaptiveModel {
        let idx = self.context_index(prev1, prev2, pos_bin);
        &mut self.models[idx]
    }

    fn reset(&mut self) {
        for m in &mut self.models {
            m.reset();
        }
    }
}

fn compute_position_bin(pos: usize, read_len: usize, num_bins: usize) -> usize {
    if num_bins == 0 || read_len == 0 {
        return 0;
    }
    (pos * num_bins / read_len).min(num_bins - 1)
}

fn qual_char_to_value(c: char) -> usize {
    let v = c as usize;
    if v < 33 {
        0
    } else {
        (v - 33).min(NUM_QUALITY_SYMBOLS - 1)
    }
}

fn qual_value_to_char(v: usize) -> char {
    (33 + v.min(NUM_QUALITY_SYMBOLS - 1)) as u8 as char
}

// =============================================================================
// Illumina 8-bin Mapper
// =============================================================================

const BIN_BOUNDARIES: [u8; 8] = [2, 10, 20, 25, 30, 35, 40, 255];
const BIN_REPRESENTATIVES: [u8; 8] = [2, 6, 15, 22, 27, 33, 37, 40];

fn illumina8_to_bin(q: u8) -> u8 {
    for (i, &b) in BIN_BOUNDARIES.iter().enumerate() {
        if q < b {
            return i as u8;
        }
    }
    7
}

fn illumina8_from_bin(bin: u8) -> u8 {
    BIN_REPRESENTATIVES[bin.min(7) as usize]
}

// =============================================================================
// QualityCompressor
// =============================================================================

#[derive(Clone)]
pub struct QualityCompressorConfig {
    pub quality_mode: QualityMode,
    pub context_order: ContextOrder,
    pub num_position_bins: usize,
}

impl Default for QualityCompressorConfig {
    fn default() -> Self {
        Self {
            quality_mode: QualityMode::Lossless,
            context_order: ContextOrder::Order2,
            num_position_bins: NUM_POSITION_BINS,
        }
    }
}

pub struct QualityCompressor {
    config: QualityCompressorConfig,
    ctx_model: QualityContextModel,
}

impl QualityCompressor {
    pub fn new(config: QualityCompressorConfig) -> Self {
        let ctx_model = QualityContextModel::new(config.context_order, config.num_position_bins);
        Self { config, ctx_model }
    }

    /// Compress quality strings using SCM + arithmetic coding + Zstd
    pub fn compress(&mut self, qualities: &[&str]) -> Result<Vec<u8>> {
        if qualities.is_empty() {
            return Ok(Vec::new());
        }

        if self.config.quality_mode == QualityMode::Discard {
            return Ok(Vec::new());
        }

        self.ctx_model.reset();

        let mut encoder = ArithmeticEncoder::new();

        for quality in qualities {
            let read_len = quality.len();
            let mut prev1 = 0usize;
            let mut prev2 = 0usize;

            for (pos, c) in quality.chars().enumerate() {
                let mut qv = qual_char_to_value(c);

                if self.config.quality_mode == QualityMode::Illumina8 {
                    qv = illumina8_to_bin(qv as u8) as usize;
                }

                qv = qv.min(NUM_QUALITY_SYMBOLS - 1);

                let pos_bin = compute_position_bin(pos, read_len, self.config.num_position_bins);
                let model = self.ctx_model.get_model(prev1, prev2, pos_bin);
                encoder.encode(qv, model);
                model.update(qv);

                prev2 = prev1;
                prev1 = qv;
            }
        }

        encoder.finish();
        let encoded = encoder.get_data().to_vec();

        // Apply Zstd on top for better compression
        let compressed = zstd::bulk::compress(&encoded, 3)
            .map_err(|e| FqcError::Compression(format!("Zstd compression failed: {e}")))?;
        Ok(compressed)
    }

    /// Decompress quality data given per-read lengths
    pub fn decompress(&mut self, data: &[u8], lengths: &[u32]) -> Result<Vec<String>> {
        if data.is_empty() || lengths.is_empty() {
            if self.config.quality_mode == QualityMode::Discard {
                return Ok(lengths.iter().map(|&l| "!".repeat(l as usize)).collect());
            }
            return Ok(lengths.iter().map(|_| String::new()).collect());
        }

        if self.config.quality_mode == QualityMode::Discard {
            return Ok(lengths.iter().map(|&l| "!".repeat(l as usize)).collect());
        }

        // Decompress Zstd layer first
        let decoded = zstd::stream::decode_all(data)
            .map_err(|e| FqcError::Decompression(format!("Zstd decompression failed: {e}")))?;

        self.ctx_model.reset();
        let mut decoder = ArithmeticDecoder::new(&decoded);

        let mut qualities = Vec::with_capacity(lengths.len());

        for &len in lengths {
            let mut quality = String::with_capacity(len as usize);
            let mut prev1 = 0usize;
            let mut prev2 = 0usize;

            for pos in 0..len as usize {
                let pos_bin = compute_position_bin(pos, len as usize, self.config.num_position_bins);
                let model = self.ctx_model.get_model(prev1, prev2, pos_bin);
                let qv = decoder.decode(model);
                model.update(qv);

                let out_qv = if self.config.quality_mode == QualityMode::Illumina8 {
                    illumina8_from_bin(qv as u8) as usize
                } else {
                    qv
                };

                quality.push(qual_value_to_char(out_qv));

                prev2 = prev1;
                prev1 = qv;
            }

            qualities.push(quality);
        }

        Ok(qualities)
    }
}

// =============================================================================
// QualityCompressor Trait Implementation
// =============================================================================

impl QualityCompressorTrait for QualityCompressor {
    fn compress(&mut self, reads: &[ReadRecord]) -> Result<Vec<u8>> {
        let qualities: Vec<&str> = reads.iter().map(|r| r.quality.as_str()).collect();
        QualityCompressor::compress(self, &qualities)
    }

    fn decompress(
        &mut self,
        data: &[u8],
        read_count: u32,
        uniform_length: u32,
        lengths: &[u32],
    ) -> Result<Vec<String>> {
        // Use uniform_length if available, otherwise use individual lengths
        let length_vec: Vec<u32> = if uniform_length > 0 {
            vec![uniform_length; read_count as usize]
        } else if !lengths.is_empty() {
            lengths.to_vec()
        } else {
            return Ok(vec![String::new(); read_count as usize]);
        };

        QualityCompressor::decompress(self, data, &length_vec)
    }

    fn codec_id(&self) -> u8 {
        match self.config.quality_mode {
            QualityMode::Discard => encode_codec(CodecFamily::Raw, 0),
            QualityMode::Lossless | QualityMode::Illumina8 | QualityMode::Qvz => match self.config.context_order {
                ContextOrder::Order0 | ContextOrder::Order1 => encode_codec(CodecFamily::ScmOrder1, 0),
                ContextOrder::Order2 => encode_codec(CodecFamily::ScmV1, 0),
            },
        }
    }
}
