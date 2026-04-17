# SCM Quality Compression (Statistical Context Model)

This document describes the Statistical Context Model (SCM) with arithmetic coding used by fqc for compressing FASTQ quality scores.

## Overview

Quality scores in FASTQ files represent per-base confidence estimates from the sequencer. They exhibit strong contextual patterns:
- **Positional correlation**: Quality tends to decrease toward the 3' end of reads
- **Sequential correlation**: Adjacent quality scores within a read are correlated
- **Distribution patterns**: Quality values follow platform-specific distributions

SCM exploits these patterns using an **adaptive arithmetic coding** approach with context models that condition on both position and neighboring quality values.

## Algorithm Pipeline

```
Quality Strings → Context Selection → Adaptive Model → Arithmetic Encode → Zstd
```

## Context Model Architecture

### Context Definition

A context is defined by three components:

| Component | Description | Values |
|-----------|-------------|--------|
| **Previous quality (prev1)** | Quality value at the previous position | 0-93 (94 values) |
| **Second previous quality (prev2)** | Quality value two positions back | 0-93 (94 values) |
| **Position bin** | Normalized position within the read | 0-7 (8 bins) |

### Context Orders

fqc supports three context orders, trading compression ratio for memory usage:

| Order | Context | Number of Models | Memory |
|-------|---------|------------------|--------|
| **Order-0** | position bin only | 8 | Minimal |
| **Order-1** | prev1 × position bin | 94 × 8 = 752 | Moderate |
| **Order-2** | prev2 × prev1 × position bin | 94² × 8 = 70,688 | Highest |

```rust
fn compute_num_contexts(order: ContextOrder, num_position_bins: usize) -> usize {
    let qual_contexts = match order {
        ContextOrder::Order0 => 1,
        ContextOrder::Order1 => NUM_QUALITY_SYMBOLS,          // 94
        ContextOrder::Order2 => NUM_QUALITY_SYMBOLS * NUM_QUALITY_SYMBOLS, // 94² = 8,836
    };
    qual_contexts * num_position_bins
}
```

### Context Index Calculation

The model index for a given context is computed as:

```rust
fn context_index(&self, prev1: usize, prev2: usize, pos_bin: usize) -> usize {
    let qual_ctx = match self.order {
        ContextOrder::Order0 => 0,
        ContextOrder::Order1 => prev1,
        ContextOrder::Order2 => prev1 * NUM_QUALITY_SYMBOLS + prev2,
    };
    qual_ctx * self.num_position_bins + pos_bin
}
```

### Position Binning

Rather than using absolute positions, quality positions are normalized into bins:

```rust
fn compute_position_bin(pos: usize, read_len: usize, num_bins: usize) -> usize {
    if num_bins == 0 || read_len == 0 { return 0; }
    (pos * num_bins / read_len).min(num_bins - 1)
}
```

This maps positions proportionally:
- For a 150bp read with 8 bins: positions 0-18 → bin 0, 19-37 → bin 1, ..., 132-149 → bin 7
- For a 300bp read with 8 bins: positions 0-37 → bin 0, 38-74 → bin 1, ..., 263-299 → bin 7

This normalization allows the model to generalize the positional quality patterns across reads of different lengths.

## Adaptive Frequency Model

Each context has an independent **adaptive frequency model** that tracks symbol frequencies:

```rust
struct AdaptiveModel {
    num_symbols: usize,           // 94
    frequencies: Vec<u32>,        // Frequency count per symbol
    cumulative: Vec<u32>,         // Cumulative frequencies for arithmetic coding
}
```

### Initialization

All symbols start with a small initial frequency to avoid zero-probability issues:

```rust
const INITIAL_FREQUENCY: u32 = 1;
// All 94 symbols start with frequency 1
```

### Update Rule

After encoding/decoding each symbol, the model is updated:

```rust
fn update(&mut self, symbol: usize) {
    self.frequencies[symbol] += ADAPT_INCREMENT;  // ADAPT_INCREMENT = 8
    if self.get_total() + ADAPT_INCREMENT > MAX_FREQUENCY {
        self.rescale();
    }
    self.update_cumulative();
}
```

### Rescaling

When the total frequency approaches the maximum (16,383), all frequencies are halved to prevent overflow while maintaining relative proportions:

```rust
fn rescale(&mut self) {
    for f in &mut self.frequencies {
        *f = (*f).div_ceil(2).max(1);  // Never go below 1
    }
}
```

This is a standard technique in adaptive arithmetic coding that ensures:
1. Frequencies never overflow the `MAX_FREQUENCY` limit
2. Recent symbols have higher relative weights (forgetting older statistics)
3. No symbol ever reaches zero frequency (avoids coding failure)

### Cumulative Frequencies

Cumulative frequencies are maintained for arithmetic coding range calculations:

```rust
fn update_cumulative(&mut self) {
    self.cumulative[0] = 0;
    for i in 0..self.num_symbols {
        self.cumulative[i + 1] = self.cumulative[i] + self.frequencies[i];
    }
}
```

The cumulative frequency `cumulative[s]` represents the sum of frequencies for all symbols `< s`, which defines the range boundaries for arithmetic coding.

## Arithmetic Coding

### Encoder

The arithmetic encoder maintains a current range `[low, high]`:

```rust
struct ArithmeticEncoder {
    output: Vec<u8>,      // Output bitstream
    low: u64,             // Current range low
    high: u64,            // Current range high
    bits_to_follow: u32,  // Pending bits to output
    bit_buffer: u8,       // Bit accumulator
    bit_count: u8,        // Bits in buffer
}
```

Constants for 32-bit precision:

```rust
const CODE_BITS: u32 = 32;
const TOP_VALUE: u64 = (1u64 << CODE_BITS) - 1;  // 2³² - 1
const FIRST_QUARTER: u64 = TOP_VALUE / 4 + 1;
const HALF: u64 = 2 * FIRST_QUARTER;
const THIRD_QUARTER: u64 = 3 * FIRST_QUARTER;
```

Encoding a symbol:

```rust
fn encode(&mut self, symbol: usize, model: &AdaptiveModel) {
    let range = self.high - self.low + 1;
    let total = model.get_total() as u64;
    let cum_low = model.get_cumulative(symbol) as u64;
    let cum_high = model.get_cumulative(symbol + 1) as u64;

    self.high = self.low + (range * cum_high) / total - 1;
    self.low += (range * cum_low) / total;

    self.normalize();
}
```

### Normalization

When the range becomes too small, the encoder outputs bits and rescales:

```rust
fn normalize(&mut self) {
    loop {
        if self.high < HALF {
            self.output_bit(0);          // Both bounds in lower half
        } else if self.low >= HALF {
            self.output_bit(1);           // Both bounds in upper half
            self.low -= HALF;
            self.high -= HALF;
        } else if self.low >= FIRST_QUARTER && self.high < THIRD_QUARTER {
            self.bits_to_follow += 1;     // Bounds straddle middle
            self.low -= FIRST_QUARTER;
            self.high -= FIRST_QUARTER;
        } else {
            break;                         // Range large enough
        }
        self.low *= 2;
        self.high = 2 * self.high + 1;
    }
}
```

The `bits_to_follow` mechanism handles the case where bounds are in the middle range: pending bits are output with the opposite value once the range resolves to a definite half.

### Decoder

The decoder mirrors the encoder:

```rust
struct ArithmeticDecoder<'a> {
    data: &'a [u8],    // Input bitstream
    low: u64,
    high: u64,
    value: u64,        // Current value in range
    bit_pos: usize,
    byte_pos: usize,
}
```

Decoding a symbol:

```rust
fn decode(&mut self, model: &mut AdaptiveModel) -> usize {
    let range = self.high - self.low + 1;
    let total = model.get_total() as u64;
    // Compute cumulative frequency corresponding to current value
    let cum_freq = (((self.value - self.low + 1) * total - 1) / range) as u32;

    // Find symbol with matching cumulative frequency
    let symbol = model.find_symbol(cum_freq);

    // Narrow range (same as encoder)
    let cum_low = model.get_cumulative(symbol) as u64;
    let cum_high = model.get_cumulative(symbol + 1) as u64;
    self.high = self.low + (range * cum_high) / total - 1;
    self.low += (range * cum_low) / total;

    self.normalize();
    symbol
}
```

Symbol lookup uses binary search:

```rust
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
```

## Quality Value Encoding

### Value Range

Quality scores are Phred+33 encoded, mapping ASCII characters 33-126 to quality values 0-93:

```rust
fn qual_char_to_value(c: char) -> usize {
    let v = c as usize;
    if v < 33 { 0 } else { (v - 33).min(NUM_QUALITY_SYMBOLS - 1) }
}

fn qual_value_to_char(v: usize) -> char {
    (33 + v.min(NUM_QUALITY_SYMBOLS - 1)) as u8 as char
}
```

The alphabet size is 94 symbols (Phred Q0 through Q93).

### Illumina 8-Bin Mode

In Illumina 8 mode, quality values are quantized before encoding:

```rust
const BIN_BOUNDARIES: [u8; 8] = [2, 10, 20, 25, 30, 35, 40, 255];
const BIN_REPRESENTATIVES: [u8; 8] = [2, 6, 15, 22, 27, 33, 37, 40];

fn illumina8_to_bin(q: u8) -> u8 {
    for (i, &b) in BIN_BOUNDARIES.iter().enumerate() {
        if q < b { return i as u8; }
    }
    7
}

fn illumina8_from_bin(bin: u8) -> u8 {
    BIN_REPRESENTATIVES[bin.min(7) as usize]
}
```

| Bin | Q Range | Representative Q | ASCII |
|-----|---------|------------------|-------|
| 0 | Q < 2 | 2 | `#` |
| 1 | 2 ≤ Q < 10 | 6 | `'` |
| 2 | 10 ≤ Q < 20 | 15 | `0` |
| 3 | 20 ≤ Q < 25 | 22 | `7` |
| 4 | 25 ≤ Q < 30 | 27 | `<` |
| 5 | 30 ≤ Q < 35 | 33 | `B` |
| 6 | 35 ≤ Q < 40 | 37 | `F` |
| 7 | Q ≥ 40 | 40 | `I` |

This reduces the effective alphabet from 94 to 8 symbols, significantly lowering the entropy and improving compression.

### Discard Mode

When quality mode is `Discard`:
- Compress returns an empty vector (`Vec::new()`)
- Block header sets `size_qual = 0` and `codec_qual = Raw`
- Decompress generates `"!"` (Q0) repeated to each read's length

## Compression Process

```rust
pub fn compress(&mut self, qualities: &[&str]) -> Result<Vec<u8>> {
    // Reset model for each block
    self.ctx_model.reset();

    let mut encoder = ArithmeticEncoder::new();

    for quality in qualities {
        let read_len = quality.len();
        let mut prev1 = 0usize;
        let mut prev2 = 0usize;

        for (pos, c) in quality.chars().enumerate() {
            let mut qv = qual_char_to_value(c);

            // Optional Illumina 8 quantization
            if self.config.quality_mode == QualityMode::Illumina8 {
                qv = illumina8_to_bin(qv as u8) as usize;
            }

            qv = qv.min(NUM_QUALITY_SYMBOLS - 1);

            // Select model based on context
            let pos_bin = compute_position_bin(pos, read_len, self.config.num_position_bins);
            let model = self.ctx_model.get_model(prev1, prev2, pos_bin);

            // Encode and update
            encoder.encode(qv, model);
            model.update(qv);

            prev2 = prev1;
            prev1 = qv;
        }
    }

    encoder.finish();
    let encoded = encoder.get_data().to_vec();

    // Apply Zstd on top for additional compression
    let compressed = zstd::bulk::compress(&encoded, 3)?;
    Ok(compressed)
}
```

## Zstd Post-Processing

After arithmetic coding, the output is further compressed with zstd (level 3):

```rust
let compressed = zstd::bulk::compress(&encoded, 3)?;
```

This two-stage approach (arithmetic coding + zstd) captures patterns that arithmetic coding alone might miss, such as:
- Repeated byte patterns in the arithmetic output
- Cross-read correlations not captured by the per-block model reset

## Decompression Process

Decompression reverses the encoding process:

```rust
pub fn decompress(&mut self, data: &[u8], lengths: &[u32]) -> Result<Vec<String>> {
    // Decompress Zstd layer
    let decoded = zstd::stream::decode_all(data)?;

    // Reset model
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

            // Decode symbol and update model
            let qv = decoder.decode(model);
            model.update(qv);

            // Reverse Illumina 8 quantization if applicable
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
```

## Model Reset Strategy

The context model is **reset at the beginning of each block**:

```rust
self.ctx_model.reset();
```

This ensures:
1. Models adapt to the specific quality distribution of each block
2. No model staleness across blocks with different quality characteristics
3. Block independence for random-access decompression

## Codec Selection by Read Length Class

| Read Length Class | SCM Variant | Context Order | Rationale |
|-------------------|-------------|---------------|-----------|
| Short | `ScmV1` (0x20) | Order-2 | Maximum compression, memory feasible |
| Medium | `ScmV1` (0x20) | Order-2 | Same as short |
| Long | `ScmOrder1` (0x80) | Order-1 | Lower memory for long reads |

Order-1 is used for long reads because:
- Order-2 would require 70,688 models × 94 symbols × 4 bytes ≈ 26 MB
- Long reads typically have fewer reads per block, reducing the benefit of higher-order contexts
- Order-1 (752 models) provides good compression with much less memory

## Performance Characteristics

### Compression Ratio

| Mode | Typical Ratio | Notes |
|------|---------------|-------|
| Lossless | 1.5-3x | Full quality preservation |
| Illumina8 | 3-6x | With quantization loss |
| Discard | ∞ | Quality removed |

### Memory Usage

| Context Order | Models | Memory per Model | Total |
|---------------|--------|------------------|-------|
| Order-0 | 8 | 94 × 4 × 2 ≈ 752 B | ~6 KB |
| Order-1 | 752 | 94 × 4 × 2 ≈ 752 B | ~566 KB |
| Order-2 | 70,688 | 94 × 4 × 2 ≈ 752 B | ~53 MB |

### Time Complexity

Per quality character:
- Model lookup: O(1) (direct index)
- Arithmetic encode: O(1) amortized (range narrowing)
- Model update: O(1) (single frequency increment)
- Cumulative update: O(94) worst case (full recalculation)

Overall: O(total quality characters) for encoding/decoding.

## Constants Summary

| Constant | Value | Purpose |
|----------|-------|---------|
| `NUM_QUALITY_SYMBOLS` | 94 | Phred Q0-Q93 |
| `NUM_POSITION_BINS` | 8 | Normalized position bins |
| `MAX_FREQUENCY` | 16,383 | Max total before rescale |
| `INITIAL_FREQUENCY` | 1 | Starting frequency |
| `ADAPT_INCREMENT` | 8 | Update increment |
| `CODE_BITS` | 32 | Arithmetic coder precision |

## Related Documents

- [Strategy Selection](./strategy-selection.md)
- [Source Module Overview](../architecture/modules.md)
- [Block Format](../architecture/block-format.md)
- [Compression Algorithms RFC](../../specs/rfc/0002-compression-algorithms.md)
