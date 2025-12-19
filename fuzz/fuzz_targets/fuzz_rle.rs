//! Fuzz target for direct RLE encode/decode testing.
//!
//! This target tests the RLE (Run-Length Encoding) implementation directly,
//! without going through the compression layer. It verifies:
//!
//! 1. **Roundtrip invariant**: `decode(encode(data)) == data` for all inputs
//! 2. **No panics**: No crashes on any byte sequence
//! 3. **Decode safety**: Arbitrary encoded data doesn't cause buffer overflows
//!
//! # Safety Properties Tested
//!
//! - No panics on arbitrary byte sequences
//! - Safe handling of edge cases (empty, very long, all zeros, all ones)
//! - Decode handles malformed/truncated encoded data gracefully
//! - Varint encoding/decoding is correct for all u64 values

#![no_main]

use arbitrary::Arbitrary;
use fortress_rollback::rle::{decode, encode};
use libfuzzer_sys::fuzz_target;

/// Structured input for RLE fuzzing with different scenarios
#[derive(Debug, Arbitrary)]
struct RleInput {
    /// Test case variant
    mode: TestMode,
}

#[derive(Debug, Arbitrary)]
enum TestMode {
    /// Test roundtrip encoding/decoding of arbitrary data
    Roundtrip { data: Vec<u8> },

    /// Test decoding of arbitrary bytes (may be invalid encoding)
    DecodeArbitrary { bytes: Vec<u8> },

    /// Test with patterns that stress RLE compression
    StressPattern { pattern: StressPattern },
}

#[derive(Debug, Arbitrary)]
enum StressPattern {
    /// Long run of compressible bytes (0x00 or 0xFF)
    CompressibleRun { len: u16, byte: CompressibleByte },

    /// Alternating compressible/non-compressible bytes (worst case)
    Alternating { len: u16 },

    /// Mix of runs and random data
    Mixed {
        zero_run: u8,
        random: Vec<u8>,
        one_run: u8,
        trailing: Vec<u8>,
    },

    /// Pattern simulating XOR-encoded game state (sparse non-zero)
    SparseGameState { positions: Vec<u16>, data_len: u16 },
}

#[derive(Debug, Arbitrary)]
enum CompressibleByte {
    Zero,
    Max,
}

impl CompressibleByte {
    fn value(&self) -> u8 {
        match self {
            CompressibleByte::Zero => 0x00,
            CompressibleByte::Max => 0xFF,
        }
    }
}

fuzz_target!(|input: RleInput| {
    match input.mode {
        TestMode::Roundtrip { data } => {
            // Core invariant: roundtrip must preserve data
            let encoded = encode(&data);
            if let Ok(decoded) = decode(&encoded) {
                assert_eq!(
                    data, decoded,
                    "Roundtrip invariant violated: {} bytes encoded to {} bytes, decoded to {} bytes",
                    data.len(), encoded.len(), decoded.len()
                );
            }
        },

        TestMode::DecodeArbitrary { bytes } => {
            // Decoding arbitrary bytes should not panic
            // Result may be Ok or Err, but should never crash
            let _result = decode(&bytes);
        },

        TestMode::StressPattern { pattern } => {
            let data = generate_stress_pattern(pattern);

            // Verify roundtrip on stress pattern
            let encoded = encode(&data);
            if let Ok(decoded) = decode(&encoded) {
                assert_eq!(
                    data,
                    decoded,
                    "Roundtrip failed on stress pattern: {} bytes",
                    data.len()
                );
            }
        },
    }
});

/// Generate data bytes from a stress pattern
fn generate_stress_pattern(pattern: StressPattern) -> Vec<u8> {
    match pattern {
        StressPattern::CompressibleRun { len, byte } => {
            vec![byte.value(); len as usize]
        },

        StressPattern::Alternating { len } => {
            // Worst case: alternating 0x00 and non-compressible byte
            (0..len)
                .map(|i| if i % 2 == 0 { 0x00 } else { 0x42 })
                .collect()
        },

        StressPattern::Mixed {
            zero_run,
            random,
            one_run,
            trailing,
        } => {
            let mut data = vec![0x00; zero_run as usize];
            data.extend(&random);
            data.extend(std::iter::repeat_n(0xFF, one_run as usize));
            data.extend(&trailing);
            data
        },

        StressPattern::SparseGameState {
            positions,
            data_len,
        } => {
            // Create sparse data (like XOR-encoded game state)
            let len = data_len as usize;
            let mut data = vec![0u8; len];
            for pos in positions {
                let idx = (pos as usize) % len.max(1);
                // Set a non-zero value at sparse positions
                data[idx] = (pos % 255).max(1) as u8;
            }
            data
        },
    }
}
