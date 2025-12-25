// special thanks to james7132
//!
//! # Compression Module
//!
//! This module provides XOR delta encoding and RLE compression for network messages.
//!
//! # Note
//!
//! These functions are re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
//! They are not part of the stable public API.

use crate::report_violation;
use crate::rle;
use crate::telemetry::{ViolationKind, ViolationSeverity};

/// Encodes input bytes using XOR delta encoding followed by RLE compression.
pub fn encode<'a>(reference: &[u8], pending_input: impl Iterator<Item = &'a Vec<u8>>) -> Vec<u8> {
    // first, do a XOR encoding to the reference input (will probably lead to a lot of same bits in sequence)
    let buf = delta_encode(reference, pending_input);
    // then, RLE encode the buffer (making use of the property mentioned above)
    rle::encode(buf)
}

/// Performs XOR delta encoding against a reference.
pub fn delta_encode<'a>(
    ref_bytes: &[u8],
    pending_input: impl Iterator<Item = &'a Vec<u8>>,
) -> Vec<u8> {
    let (lower, upper) = pending_input.size_hint();
    let capacity = upper.unwrap_or(lower) * ref_bytes.len();
    let mut bytes = Vec::with_capacity(capacity);

    for input in pending_input {
        let input_bytes = input;
        // Validate input length matches reference - skip mismatched inputs with warning
        if input_bytes.len() != ref_bytes.len() {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "delta_encode: input length {} doesn't match reference length {} - skipping",
                input_bytes.len(),
                ref_bytes.len()
            );
            continue;
        }

        for (b1, b2) in ref_bytes.iter().zip(input_bytes.iter()) {
            bytes.push(b1 ^ b2);
        }
    }
    bytes
}

/// Decodes RLE-compressed XOR delta-encoded data.
pub fn decode(
    reference: &[u8],
    data: &[u8],
) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
    // decode the RLE encoding first
    let buf = rle::decode(data)?;

    // decode the delta-encoding
    delta_decode(reference, &buf)
}

/// Decodes XOR delta-encoded data against a reference.
pub fn delta_decode(
    ref_bytes: &[u8],
    data: &[u8],
) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
    // Validate preconditions - return error instead of panicking
    if ref_bytes.is_empty() {
        report_violation!(
            ViolationSeverity::Error,
            ViolationKind::NetworkProtocol,
            "delta_decode: reference bytes is empty"
        );
        return Err("delta_decode: reference bytes is empty".into());
    }

    if data.len() % ref_bytes.len() != 0 {
        report_violation!(
            ViolationSeverity::Error,
            ViolationKind::NetworkProtocol,
            "delta_decode: data length {} is not a multiple of reference length {}",
            data.len(),
            ref_bytes.len()
        );
        return Err(format!(
            "delta_decode: data length {} is not a multiple of reference length {}",
            data.len(),
            ref_bytes.len()
        )
        .into());
    }

    let out_size = data.len() / ref_bytes.len();
    let mut output = Vec::with_capacity(out_size);

    for inp in 0..out_size {
        let mut buffer = vec![0u8; ref_bytes.len()];
        for i in 0..ref_bytes.len() {
            let data_idx = ref_bytes.len() * inp + i;
            let ref_byte = ref_bytes
                .get(i)
                .ok_or_else(|| format!("delta_decode: ref_bytes index {} out of bounds", i))?;
            let data_byte = data
                .get(data_idx)
                .ok_or_else(|| format!("delta_decode: data index {} out of bounds", data_idx))?;
            let buffer_slot = buffer
                .get_mut(i)
                .ok_or_else(|| format!("delta_decode: buffer index {} out of bounds", i))?;
            *buffer_slot = ref_byte ^ data_byte;
        }
        output.push(buffer);
    }

    Ok(output)
} // #########
  // # TESTS #
  // #########

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod compression_tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let ref_input = vec![0, 0, 0, 1];
        let inp0: Vec<u8> = vec![0, 0, 1, 0];
        let inp1: Vec<u8> = vec![0, 0, 1, 1];
        let inp2: Vec<u8> = vec![0, 1, 0, 0];
        let inp3: Vec<u8> = vec![0, 1, 0, 1];
        let inp4: Vec<u8> = vec![0, 1, 1, 0];

        let pend_inp = vec![inp0, inp1, inp2, inp3, inp4];

        let encoded = encode(&ref_input, pend_inp.iter());
        let decoded = decode(&ref_input, &encoded).unwrap();

        assert!(pend_inp == decoded);
    }

    #[test]
    fn test_encode_decode_empty() {
        let ref_input = vec![0, 0, 0, 0];
        let pend_inp: Vec<Vec<u8>> = vec![];

        let encoded = encode(&ref_input, pend_inp.iter());
        let decoded = decode(&ref_input, &encoded).unwrap();

        assert!(pend_inp == decoded);
    }

    #[test]
    fn test_encode_decode_identical_inputs() {
        let ref_input = vec![1, 2, 3, 4];
        let inp0: Vec<u8> = vec![1, 2, 3, 4]; // Same as reference
        let inp1: Vec<u8> = vec![1, 2, 3, 4];
        let inp2: Vec<u8> = vec![1, 2, 3, 4];

        let pend_inp = vec![inp0, inp1, inp2];

        let encoded = encode(&ref_input, pend_inp.iter());
        let decoded = decode(&ref_input, &encoded).unwrap();

        assert!(pend_inp == decoded);
    }

    #[test]
    fn test_delta_encode_xor_property() {
        // XOR property: a ^ a = 0, so identical bytes should produce zeros
        let ref_bytes = vec![0xFF, 0xAA, 0x55];
        let inputs = [vec![0xFF, 0xAA, 0x55]]; // identical to reference

        let encoded = delta_encode(&ref_bytes, inputs.iter());

        // All bytes should be zero due to XOR with itself
        assert!(encoded.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_delta_encode_inverse_property() {
        // XOR is its own inverse: (a ^ b) ^ b = a
        let ref_bytes = vec![0x12, 0x34, 0x56, 0x78];
        let input = vec![0xAB, 0xCD, 0xEF, 0x01];
        let inputs = vec![input];

        let encoded = delta_encode(&ref_bytes, inputs.iter());
        let decoded = delta_decode(&ref_bytes, &encoded).unwrap();

        assert_eq!(decoded, inputs);
    }

    #[test]
    fn test_delta_decode_empty_reference_returns_error() {
        let ref_bytes: Vec<u8> = vec![];
        let data = vec![1, 2, 3];

        let result = delta_decode(&ref_bytes, &data);
        assert!(result.is_err());
    }

    #[test]
    fn test_delta_decode_misaligned_data_returns_error() {
        let ref_bytes = vec![1, 2, 3, 4];
        let data = vec![1, 2, 3]; // Not a multiple of 4

        let result = delta_decode(&ref_bytes, &data);
        assert!(result.is_err());
    }

    #[test]
    fn test_delta_encode_skips_mismatched_inputs() {
        let ref_bytes = vec![1, 2, 3, 4];
        let good_input = vec![5, 6, 7, 8];
        let bad_input = vec![1, 2]; // Wrong length
        let inputs = [good_input.clone(), bad_input, good_input];

        let encoded = delta_encode(&ref_bytes, inputs.iter());

        // Should only have encoded the two good inputs (8 bytes total)
        // Each good input XORs with ref to produce 4 bytes
        assert_eq!(encoded.len(), 8);
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating valid input sizes (1-32 bytes)
    fn input_size() -> impl Strategy<Value = usize> {
        1usize..=32
    }

    // Strategy for generating a reference buffer of a given size
    fn reference_buffer(size: usize) -> impl Strategy<Value = Vec<u8>> {
        proptest::collection::vec(any::<u8>(), size)
    }

    // Strategy for generating pending inputs (1-16 inputs, each matching the reference size)
    fn pending_inputs(size: usize, count: usize) -> impl Strategy<Value = Vec<Vec<u8>>> {
        proptest::collection::vec(proptest::collection::vec(any::<u8>(), size), count)
    }

    proptest! {
        /// Property: encode followed by decode is identity
        #[test]
        fn prop_encode_decode_roundtrip(
            size in input_size(),
            count in 1usize..=16,
        ) {
            let ref_strategy = reference_buffer(size);
            let pending_strategy = pending_inputs(size, count);

            // Use prop_flat_map to chain dependent strategies
            let combined = (ref_strategy, pending_strategy);
            proptest::test_runner::TestRunner::default()
                .run(&combined, |(ref_input, pend_inp)| {
                    let encoded = encode(&ref_input, pend_inp.iter());
                    let decoded = decode(&ref_input, &encoded).expect("decode should succeed");
                    prop_assert_eq!(decoded, pend_inp);
                    Ok(())
                })?;
        }

        /// Property: delta encoding XOR is self-inverse
        #[test]
        fn prop_delta_encode_inverse(
            size in input_size(),
            count in 1usize..=16,
        ) {
            let ref_strategy = reference_buffer(size);
            let pending_strategy = pending_inputs(size, count);

            let combined = (ref_strategy, pending_strategy);
            proptest::test_runner::TestRunner::default()
                .run(&combined, |(ref_bytes, inputs)| {
                    let encoded = delta_encode(&ref_bytes, inputs.iter());
                    let decoded = delta_decode(&ref_bytes, &encoded).expect("decode should succeed");
                    prop_assert_eq!(decoded, inputs);
                    Ok(())
                })?;
        }

        /// Property: identical inputs produce zero delta
        #[test]
        fn prop_identical_inputs_zero_delta(
            size in input_size(),
        ) {
            let ref_strategy = reference_buffer(size);

            proptest::test_runner::TestRunner::default()
                .run(&ref_strategy, |ref_bytes| {
                    let inputs = [ref_bytes.clone()];
                    let encoded = delta_encode(&ref_bytes, inputs.iter());
                    prop_assert!(encoded.iter().all(|&b| b == 0));
                    Ok(())
                })?;
        }

        /// Property: encoded size is deterministic
        #[test]
        fn prop_encoding_deterministic(
            size in input_size(),
            count in 1usize..=8,
        ) {
            let ref_strategy = reference_buffer(size);
            let pending_strategy = pending_inputs(size, count);

            let combined = (ref_strategy, pending_strategy);
            proptest::test_runner::TestRunner::default()
                .run(&combined, |(ref_input, pend_inp)| {
                    let encoded1 = encode(&ref_input, pend_inp.iter());
                    let encoded2 = encode(&ref_input, pend_inp.iter());
                    prop_assert_eq!(encoded1, encoded2);
                    Ok(())
                })?;
        }

        /// Property: empty input list produces empty output
        #[test]
        fn prop_empty_inputs(
            size in input_size(),
        ) {
            let ref_strategy = reference_buffer(size);

            proptest::test_runner::TestRunner::default()
                .run(&ref_strategy, |ref_input| {
                    let pend_inp: Vec<Vec<u8>> = vec![];
                    let encoded = encode(&ref_input, pend_inp.iter());
                    let decoded = decode(&ref_input, &encoded).expect("decode should succeed");
                    prop_assert!(decoded.is_empty());
                    Ok(())
                })?;
        }
    }
}
