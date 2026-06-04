//! Fuzz target for compression via network message sequences.
//!
//! This target indirectly tests the compression layer by creating Message
//! types with varying input byte patterns and serializing them.
//! The compression is exercised during protocol message handling.
//!
//! # Safety Properties Tested
//! - No panics on arbitrary byte sequences in Input messages
//! - Safe handling of edge cases (empty, very long, all zeros, all ones)
//! - The decode path (including RLE byte caps and delta frame-count caps)
//!   never panics or over-allocates on arbitrary attacker-controlled bytes

#![no_main]

use arbitrary::Arbitrary;
use fortress_rollback::network::{codec, compression};
use fortress_rollback::rle::DEFAULT_MAX_DECODED_LEN;
use libfuzzer_sys::fuzz_target;

/// Structured input for compression fuzzing
#[derive(Debug, Arbitrary)]
struct CompressionInput {
    /// The reference bytes (used as XOR key)
    reference: Vec<u8>,
    /// Multiple input sequences to encode
    inputs: Vec<Vec<u8>>,
    /// Arbitrary (attacker-controlled) bytes fed straight into the decode path
    /// to exercise the RLE decompression-bomb guard.
    decode_data: Vec<u8>,
}

fuzz_target!(|input: CompressionInput| {
    // Skip if reference is empty (edge case handled elsewhere)
    if input.reference.is_empty() {
        return;
    }

    // Create arbitrary byte sequences and ensure they serialize/deserialize safely
    // The Message type internally uses compression for Input variants

    // Test that creating Input-type messages doesn't panic
    // Even with mismatched or extreme byte sequences

    // Simulate the kind of data that would go through compression:
    // 1. XOR encoding with reference
    // 2. RLE encoding of the result

    // Test various edge cases through serialization
    for bytes in &input.inputs {
        // Create bytes that would be delta-encoded
        let xor_result: Vec<u8> = input
            .reference
            .iter()
            .zip(bytes.iter().cycle())
            .map(|(a, b)| a ^ b)
            .take(input.reference.len())
            .collect();

        // Ensure operations don't panic
        let _ = codec::encode(&xor_result);
    }

    // Test boundary conditions:
    // - All zeros
    let zeros = vec![0u8; input.reference.len()];
    let _ = codec::encode(&zeros);

    // - All ones
    let ones = vec![0xFFu8; input.reference.len()];
    let _ = codec::encode(&ones);

    // - Alternating pattern
    let alternating: Vec<u8> = (0..input.reference.len())
        .map(|i| if i % 2 == 0 { 0x55 } else { 0xAA })
        .collect();
    let _ = codec::encode(&alternating);

    // Fuzz the convenience DECODE path on arbitrary attacker-controlled bytes.
    // It must never panic or over-allocate; if it returns Ok, the total decoded
    // byte count is bounded by the default RLE safety cap.
    if let Ok(decoded) = compression::decode(&input.reference, &input.decode_data) {
        let total: usize = decoded.iter().map(Vec::len).sum();
        assert!(
            total <= DEFAULT_MAX_DECODED_LEN,
            "decoded {} bytes exceeds DEFAULT_MAX_DECODED_LEN {}",
            total,
            DEFAULT_MAX_DECODED_LEN
        );
        assert!(
            decoded.len() <= compression::MAX_DELTA_DECODED_FRAMES,
            "decoded {} frames exceeds MAX_DELTA_DECODED_FRAMES {}",
            decoded.len(),
            compression::MAX_DELTA_DECODED_FRAMES
        );
    }

    // Protocol receive paths use an explicit cap derived from configuration and
    // reference input size. Exercise that stricter API too.
    let configured_limit = input.reference.len().saturating_mul(4).max(1);
    if let Ok(decoded) =
        compression::decode_with_max_len(&input.reference, &input.decode_data, configured_limit)
    {
        let total: usize = decoded.iter().map(Vec::len).sum();
        assert!(
            total <= configured_limit,
            "decoded {} bytes exceeds configured limit {}",
            total,
            configured_limit
        );
        assert!(
            decoded.len() <= compression::MAX_DELTA_DECODED_FRAMES,
            "decoded {} frames exceeds MAX_DELTA_DECODED_FRAMES {}",
            decoded.len(),
            compression::MAX_DELTA_DECODED_FRAMES
        );
    }
});
