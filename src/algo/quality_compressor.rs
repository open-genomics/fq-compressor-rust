// =============================================================================
// fqc-rust - Quality Compressor (SCM + Arithmetic Coding)
// =============================================================================

use crate::algo::compressor_traits::QualityCompressor as QualityCompressorTrait;
use crate::error::{FqcError, Result};
use crate::types::{encode_codec, CodecFamily, QualityMode, ReadRecord};

pub const NUM_QUALITY_SYMBOLS: usize = 94; // Phred 0-93 (ASCII 33-126)
pub const NUM_POSITION_BINS: usize = 8;
const MAX_FREQUENCY: u32 = 16383;
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

const QUALITY_TREE_STRIDE: usize = NUM_QUALITY_SYMBOLS + 1;

/// Adaptive Fenwick models for every quality context.
///
/// The previous shape owned one `Vec<u16>` per context, so order-2 quality
/// compression paid roughly 70k allocations and an indirection per symbol.
/// Trees and totals now live in two contiguous allocations; each context is a
/// fixed stride inside them.
struct QualityContextModel {
    order: ContextOrder,
    num_position_bins: usize,
    trees: Vec<u16>,
    totals: Vec<u32>,
}

impl QualityContextModel {
    fn new(order: ContextOrder, num_position_bins: usize) -> Self {
        let num_contexts = Self::compute_num_contexts(order, num_position_bins);
        let mut trees = vec![0u16; num_contexts * QUALITY_TREE_STRIDE];
        for tree in trees.chunks_exact_mut(QUALITY_TREE_STRIDE) {
            Self::reset_tree(tree);
        }
        Self {
            order,
            num_position_bins,
            trees,
            totals: vec![NUM_QUALITY_SYMBOLS as u32; num_contexts],
        }
    }

    fn compute_num_contexts(order: ContextOrder, num_position_bins: usize) -> usize {
        let qual_contexts = match order {
            ContextOrder::Order1 => NUM_QUALITY_SYMBOLS,
            ContextOrder::Order2 => NUM_QUALITY_SYMBOLS * NUM_QUALITY_SYMBOLS,
        };
        qual_contexts * num_position_bins
    }

    fn context_index(&self, prev1: usize, prev2: usize, pos_bin: usize) -> usize {
        let qual_ctx = match self.order {
            ContextOrder::Order1 => prev1,
            ContextOrder::Order2 => prev1 * NUM_QUALITY_SYMBOLS + prev2,
        };
        qual_ctx * self.num_position_bins + pos_bin
    }

    fn tree(&self, context: usize) -> &[u16] {
        let base = context * QUALITY_TREE_STRIDE;
        &self.trees[base..base + QUALITY_TREE_STRIDE]
    }

    fn tree_mut(&mut self, context: usize) -> &mut [u16] {
        let base = context * QUALITY_TREE_STRIDE;
        &mut self.trees[base..base + QUALITY_TREE_STRIDE]
    }

    fn get_total(&self, context: usize) -> u32 {
        self.totals[context]
    }

    fn get_cumulative(&self, context: usize, symbol: usize) -> u32 {
        let tree = self.tree(context);
        let mut index = symbol;
        let mut sum = 0u32;
        while index > 0 {
            sum += u32::from(tree[index]);
            index -= index & index.wrapping_neg();
        }
        sum
    }

    fn find_symbol(&self, context: usize, cum_freq: u32) -> usize {
        let tree = self.tree(context);
        let mut index = 0usize;
        let mut sum = 0u32;
        let mut bit = 1usize << (usize::BITS - 1 - NUM_QUALITY_SYMBOLS.leading_zeros());
        while bit != 0 {
            let next = index + bit;
            if next <= NUM_QUALITY_SYMBOLS {
                let candidate = sum + u32::from(tree[next]);
                if candidate <= cum_freq {
                    index = next;
                    sum = candidate;
                }
            }
            bit >>= 1;
        }
        index.min(NUM_QUALITY_SYMBOLS - 1)
    }

    fn update(&mut self, context: usize, symbol: usize) {
        let previous_total = self.totals[context];
        self.add(context, symbol, ADAPT_INCREMENT);
        self.totals[context] += ADAPT_INCREMENT;
        if previous_total + ADAPT_INCREMENT > MAX_FREQUENCY {
            self.rescale(context);
        }
    }

    fn add(&mut self, context: usize, symbol: usize, delta: u32) {
        let tree = self.tree_mut(context);
        let mut index = symbol + 1;
        while index <= NUM_QUALITY_SYMBOLS {
            tree[index] += delta as u16;
            index += index & index.wrapping_neg();
        }
    }

    fn rescale(&mut self, context: usize) {
        let mut frequencies = [0u16; NUM_QUALITY_SYMBOLS];
        let mut previous = 0u32;
        for (offset, frequency) in frequencies.iter_mut().enumerate() {
            let current = self.get_cumulative(context, offset + 1);
            *frequency = (current - previous).div_ceil(2).max(1) as u16;
            previous = current;
        }

        let tree = self.tree_mut(context);
        tree.fill(0);
        for (index, frequency) in frequencies.iter().enumerate() {
            tree[index + 1] = *frequency;
        }
        for index in 1..=NUM_QUALITY_SYMBOLS {
            let parent = index + (index & index.wrapping_neg());
            if parent <= NUM_QUALITY_SYMBOLS {
                tree[parent] += tree[index];
            }
        }
        self.totals[context] = frequencies.iter().map(|&frequency| u32::from(frequency)).sum();
    }

    fn reset_tree(tree: &mut [u16]) {
        tree.fill(0);
        for (index, slot) in tree.iter_mut().enumerate().skip(1) {
            *slot = (index & index.wrapping_neg()) as u16;
        }
    }

    fn reset(&mut self) {
        for tree in self.trees.chunks_exact_mut(QUALITY_TREE_STRIDE) {
            Self::reset_tree(tree);
        }
        self.totals.fill(NUM_QUALITY_SYMBOLS as u32);
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

    fn encode(&mut self, symbol: usize, model: &QualityContextModel, context: usize) {
        let range = self.high - self.low + 1;
        let total = model.get_total(context) as u64;
        let cum_low = model.get_cumulative(context, symbol) as u64;
        let cum_high = model.get_cumulative(context, symbol + 1) as u64;

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

    fn decode(&mut self, model: &QualityContextModel, context: usize) -> usize {
        let range = self.high - self.low + 1;
        let total = model.get_total(context) as u64;
        let cum_freq = (((self.value - self.low + 1) * total - 1) / range) as u32;

        let symbol = model.find_symbol(context, cum_freq);

        let cum_low = model.get_cumulative(context, symbol) as u64;
        let cum_high = model.get_cumulative(context, symbol + 1) as u64;

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
    Order1,
    Order2,
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

/// QVZ-style reconstruction codebook (8 Phred levels, nearest-neighbor).
pub const QVZ_CODEBOOK: [u8; 8] = [7, 15, 20, 25, 30, 35, 40, 41];

fn qvz_to_bin(q: u8) -> u8 {
    let mut best = 0u8;
    let mut best_dist = u8::MAX;
    for (i, &rep) in QVZ_CODEBOOK.iter().enumerate() {
        let dist = q.abs_diff(rep);
        if dist < best_dist {
            best_dist = dist;
            best = i as u8;
        }
    }
    best
}

fn qvz_from_bin(bin: u8) -> u8 {
    QVZ_CODEBOOK[bin.min(7) as usize]
}

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

impl QualityCompressorConfig {
    /// Conservative resident estimate for the adaptive context tables.
    ///
    /// Trees and totals are two flat allocations now, so the dominant term is
    /// one fixed-stride tree and one total per context. This is intentionally
    /// approximate: it bounds worker counts, not exact allocation sizes.
    pub fn estimated_model_bytes(&self) -> usize {
        let contexts = match self.context_order {
            ContextOrder::Order1 => NUM_QUALITY_SYMBOLS,
            ContextOrder::Order2 => NUM_QUALITY_SYMBOLS * NUM_QUALITY_SYMBOLS,
        } * self.num_position_bins.max(1);
        let per_context = QUALITY_TREE_STRIDE * std::mem::size_of::<u16>() + std::mem::size_of::<u32>();
        contexts.saturating_mul(per_context).saturating_add(4 * 1024)
    }
}

pub struct QualityCompressor {
    config: QualityCompressorConfig,
    ctx_model: QualityContextModel,
}

impl QualityCompressor {
    pub fn new(config: QualityCompressorConfig) -> Self {
        let mut config = config;
        config.num_position_bins = config.num_position_bins.max(1);
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

        for quality in qualities {
            if let Some((position, byte)) = quality
                .bytes()
                .enumerate()
                .find(|&(_, byte)| !(33..=126).contains(&byte))
            {
                return Err(FqcError::InvalidArgument(format!(
                    "Quality value {} at position {} is outside Phred+33 range",
                    byte, position
                )));
            }
            if quality.len() != quality.chars().count() {
                return Err(FqcError::InvalidArgument(
                    "Quality strings must contain only single-byte Phred+33 characters".to_string(),
                ));
            }
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
                } else if self.config.quality_mode == QualityMode::Qvz {
                    qv = qvz_to_bin(qv as u8) as usize;
                }

                qv = qv.min(NUM_QUALITY_SYMBOLS - 1);

                let pos_bin = compute_position_bin(pos, read_len, self.config.num_position_bins);
                let context = self.ctx_model.context_index(prev1, prev2, pos_bin);
                encoder.encode(qv, &self.ctx_model, context);
                self.ctx_model.update(context, qv);

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
        if lengths.is_empty() {
            return if data.is_empty() {
                Ok(Vec::new())
            } else {
                Err(FqcError::Format("Quality stream has data for zero reads".to_string()))
            };
        }

        if data.is_empty() {
            if self.config.quality_mode == QualityMode::Discard {
                return Ok(lengths.iter().map(|&l| "!".repeat(l as usize)).collect());
            }
            return Err(FqcError::Format(
                "Quality stream is empty for a non-discard block".to_string(),
            ));
        }

        if self.config.quality_mode == QualityMode::Discard {
            return Err(FqcError::Format(
                "Discarded quality stream must not contain payload bytes".to_string(),
            ));
        }

        // Arithmetic-coded stream after zstd; bound by total quality bytes * 2 + overhead.
        let total_q: usize = lengths.iter().map(|&l| l as usize).fold(0usize, usize::saturating_add);
        if total_q == 0 {
            let decoded = crate::memory::zstd_decompress_bounded(data, 64, "quality")?;
            // `ArithmeticEncoder::finish` emits the two bits `01` when no
            // symbols were encoded, padded to one byte (0x40).  Keep this
            // canonical marker strict so a random one-byte payload cannot be
            // mistaken for a valid empty stream.
            if decoded != [0x40] {
                return Err(FqcError::Format("invalid empty arithmetic quality stream".to_string()));
            }
            return Ok(vec![String::new(); lengths.len()]);
        }
        let max_out = total_q.saturating_mul(2).saturating_add(1024).max(64);
        let decoded = crate::memory::zstd_decompress_bounded(data, max_out, "quality")?;
        // The decoder intentionally pads missing tail bits with zero while
        // priming its 32-bit state.  Short but valid streams (for example a
        // single quality symbol) therefore contain fewer than four bytes;
        // only an entirely empty decoded payload is unambiguously truncated.
        if decoded.is_empty() {
            return Err(FqcError::Format("Truncated arithmetic quality stream".to_string()));
        }

        self.ctx_model.reset();
        let mut decoder = ArithmeticDecoder::new(&decoded);

        let mut qualities = Vec::with_capacity(lengths.len());

        for &len in lengths {
            let mut quality = String::with_capacity(len as usize);
            let mut prev1 = 0usize;
            let mut prev2 = 0usize;

            for pos in 0..len as usize {
                let pos_bin = compute_position_bin(pos, len as usize, self.config.num_position_bins);
                let context = self.ctx_model.context_index(prev1, prev2, pos_bin);
                let qv = decoder.decode(&self.ctx_model, context);
                self.ctx_model.update(context, qv);

                let out_qv = match self.config.quality_mode {
                    QualityMode::Illumina8 => illumina8_from_bin(qv as u8) as usize,
                    QualityMode::Qvz => qvz_from_bin(qv as u8) as usize,
                    _ => qv,
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
                ContextOrder::Order1 => encode_codec(CodecFamily::ScmOrder1, 0),
                ContextOrder::Order2 => encode_codec(CodecFamily::ScmV1, 0),
            },
        }
    }
}
