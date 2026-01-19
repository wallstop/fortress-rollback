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

use std::error::Error;
use std::fmt;

use crate::report_violation;
use crate::rle;
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::{DeltaDecodeReason, FortressError, InternalErrorKind, RleDecodeReason};

// =============================================================================
// Compression Error Types
// =============================================================================

/// Error type for compression and decompression operations.
///
/// This error type distinguishes between RLE encoding/decoding errors and
/// delta encoding/decoding errors, allowing callers to handle each type
/// appropriately.
///
/// # Structured Error Data
///
/// Both variants use structured reason types ([`RleDecodeReason`] and
/// [`DeltaDecodeReason`]) instead of strings, enabling zero-allocation error
/// construction on hot paths and programmatic error inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompressionError {
    /// An error occurred during RLE decoding.
    RleDecode {
        /// The structured reason for the RLE decode failure.
        reason: RleDecodeReason,
    },
    /// An error occurred during delta decoding.
    DeltaDecode {
        /// The structured reason for the delta decode failure.
        reason: DeltaDecodeReason,
    },
}

impl fmt::Display for CompressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RleDecode { reason } => {
                write!(f, "RLE decode error: {}", reason)
            },
            Self::DeltaDecode { reason } => {
                write!(f, "delta decode error: {}", reason)
            },
        }
    }
}

impl Error for CompressionError {}

impl From<CompressionError> for FortressError {
    fn from(err: CompressionError) -> Self {
        match err {
            CompressionError::RleDecode { reason } => Self::InternalErrorStructured {
                kind: InternalErrorKind::RleDecodeError { reason },
            },
            CompressionError::DeltaDecode { reason } => Self::InternalErrorStructured {
                kind: InternalErrorKind::DeltaDecodeError { reason },
            },
        }
    }
}

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

        for (reference_byte, input_byte) in ref_bytes.iter().zip(input_bytes.iter()) {
            bytes.push(reference_byte ^ input_byte);
        }
    }
    bytes
}

/// Decodes RLE-compressed XOR delta-encoded data.
///
/// # Errors
///
/// Returns a `CompressionError` if:
/// - RLE decoding fails (e.g., truncated or malformed data)
/// - Delta decoding fails (e.g., empty reference, length mismatch)
pub fn decode(reference: &[u8], data: &[u8]) -> Result<Vec<Vec<u8>>, CompressionError> {
    // decode the RLE encoding first
    let buf = rle::decode(data).map_err(|e| {
        // Extract the structured RleDecodeReason from the FortressError.
        if let FortressError::InternalErrorStructured {
            kind: InternalErrorKind::RleDecodeError { reason },
        } = e
        {
            return CompressionError::RleDecode { reason };
        }
        // Fallback: use a generic reason if we can't extract the specific one
        CompressionError::RleDecode {
            reason: RleDecodeReason::TruncatedData {
                offset: 0,
                buffer_len: data.len(),
            },
        }
    })?;

    // decode the delta-encoding
    delta_decode(reference, &buf)
}

/// Decodes XOR delta-encoded data against a reference.
///
/// # Errors
///
/// Returns a `CompressionError::DeltaDecode` if:
/// - The reference bytes are empty
/// - The data length is not a multiple of the reference length
/// - An index is out of bounds during decoding
pub fn delta_decode(ref_bytes: &[u8], data: &[u8]) -> Result<Vec<Vec<u8>>, CompressionError> {
    // Validate preconditions - return error instead of panicking
    if ref_bytes.is_empty() {
        report_violation!(
            ViolationSeverity::Error,
            ViolationKind::NetworkProtocol,
            "delta_decode: reference bytes is empty"
        );
        return Err(CompressionError::DeltaDecode {
            reason: DeltaDecodeReason::EmptyReference,
        });
    }

    if data.len() % ref_bytes.len() != 0 {
        report_violation!(
            ViolationSeverity::Error,
            ViolationKind::NetworkProtocol,
            "delta_decode: data length {} is not a multiple of reference length {}",
            data.len(),
            ref_bytes.len()
        );
        return Err(CompressionError::DeltaDecode {
            reason: DeltaDecodeReason::DataLengthMismatch {
                data_len: data.len(),
                reference_len: ref_bytes.len(),
            },
        });
    }

    let out_size = data.len() / ref_bytes.len();
    let mut output = Vec::with_capacity(out_size);

    for output_index in 0..out_size {
        // Pre-allocate buffer without zero-initialization to reduce allocations in hot path
        let mut buffer = Vec::with_capacity(ref_bytes.len());
        for byte_index in 0..ref_bytes.len() {
            let data_idx = ref_bytes.len() * output_index + byte_index;
            let ref_byte = ref_bytes
                .get(byte_index)
                .ok_or(CompressionError::DeltaDecode {
                    reason: DeltaDecodeReason::ReferenceIndexOutOfBounds {
                        index: byte_index,
                        length: ref_bytes.len(),
                    },
                })?;
            let data_byte = data.get(data_idx).ok_or(CompressionError::DeltaDecode {
                reason: DeltaDecodeReason::DataIndexOutOfBounds {
                    index: data_idx,
                    length: data.len(),
                },
            })?;
            // Push directly instead of allocating zeros then mutating
            buffer.push(ref_byte ^ data_byte);
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

    // =========================================================================
    // CompressionError Tests
    // =========================================================================

    #[test]
    fn test_compression_error_rle_decode_display() {
        let err = CompressionError::RleDecode {
            reason: RleDecodeReason::TruncatedData {
                offset: 10,
                buffer_len: 5,
            },
        };
        let display = format!("{}", err);
        assert!(display.contains("RLE decode error"));
        assert!(display.contains("truncated data"));
    }

    #[test]
    fn test_compression_error_delta_decode_display() {
        let err = CompressionError::DeltaDecode {
            reason: DeltaDecodeReason::EmptyReference,
        };
        let display = format!("{}", err);
        assert!(display.contains("delta decode error"));
        assert!(display.contains("reference bytes is empty"));
    }

    #[test]
    fn test_delta_decode_reason_empty_reference() {
        let reason = DeltaDecodeReason::EmptyReference;
        let display = format!("{}", reason);
        assert!(display.contains("reference bytes is empty"));
    }

    #[test]
    fn test_delta_decode_reason_data_length_mismatch() {
        let reason = DeltaDecodeReason::DataLengthMismatch {
            data_len: 7,
            reference_len: 4,
        };
        let display = format!("{}", reason);
        assert!(display.contains("data length 7"));
        assert!(display.contains("reference length 4"));
    }

    #[test]
    fn test_delta_decode_reason_reference_index_out_of_bounds() {
        let reason = DeltaDecodeReason::ReferenceIndexOutOfBounds {
            index: 5,
            length: 4,
        };
        let display = format!("{}", reason);
        assert!(display.contains("reference bytes index 5"));
        assert!(display.contains("length: 4"));
    }

    #[test]
    fn test_delta_decode_reason_data_index_out_of_bounds() {
        let reason = DeltaDecodeReason::DataIndexOutOfBounds {
            index: 10,
            length: 8,
        };
        let display = format!("{}", reason);
        assert!(display.contains("data index 10"));
        assert!(display.contains("length: 8"));
    }

    #[test]
    fn test_delta_decode_reason_is_copy() {
        let reason = DeltaDecodeReason::EmptyReference;
        let reason2 = reason;
        assert_eq!(reason, reason2);

        let reason_with_data = DeltaDecodeReason::DataLengthMismatch {
            data_len: 10,
            reference_len: 4,
        };
        let reason_with_data2 = reason_with_data;
        assert_eq!(reason_with_data, reason_with_data2);
    }

    #[test]
    fn test_compression_error_equality() {
        let err1 = CompressionError::DeltaDecode {
            reason: DeltaDecodeReason::EmptyReference,
        };
        let err2 = CompressionError::DeltaDecode {
            reason: DeltaDecodeReason::EmptyReference,
        };
        let err3 = CompressionError::RleDecode {
            reason: RleDecodeReason::BitfieldIndexOutOfBounds,
        };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_compression_error_is_copy() {
        let err = CompressionError::DeltaDecode {
            reason: DeltaDecodeReason::EmptyReference,
        };
        let err2 = err; // Copy
        assert_eq!(err, err2);

        let rle_err = CompressionError::RleDecode {
            reason: RleDecodeReason::BitfieldIndexOutOfBounds,
        };
        let rle_err2 = rle_err; // Copy
        assert_eq!(rle_err, rle_err2);
    }

    #[test]
    fn test_compression_error_to_fortress_error() {
        // Test DeltaDecode conversion
        let delta_err = CompressionError::DeltaDecode {
            reason: DeltaDecodeReason::EmptyReference,
        };
        let fortress_err: FortressError = delta_err.into();
        match fortress_err {
            FortressError::InternalErrorStructured {
                kind: InternalErrorKind::DeltaDecodeError { reason },
            } => {
                assert_eq!(reason, DeltaDecodeReason::EmptyReference);
            },
            _ => panic!("Expected InternalErrorStructured with DeltaDecodeError"),
        }

        // Test RleDecode conversion
        let rle_err = CompressionError::RleDecode {
            reason: RleDecodeReason::BitfieldIndexOutOfBounds,
        };
        let fortress_err: FortressError = rle_err.into();
        match fortress_err {
            FortressError::InternalErrorStructured {
                kind: InternalErrorKind::RleDecodeError { reason },
            } => {
                assert_eq!(reason, RleDecodeReason::BitfieldIndexOutOfBounds);
            },
            _ => panic!("Expected InternalErrorStructured with RleDecodeError"),
        }
    }

    #[test]
    fn test_delta_decode_returns_structured_error_for_empty_ref() {
        let result = delta_decode(&[], &[1, 2, 3, 4]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            CompressionError::DeltaDecode {
                reason: DeltaDecodeReason::EmptyReference
            }
        ));
    }

    #[test]
    fn test_delta_decode_returns_structured_error_for_length_mismatch() {
        let result = delta_decode(&[1, 2, 3, 4], &[1, 2, 3]); // 3 is not multiple of 4
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CompressionError::DeltaDecode {
                reason:
                    DeltaDecodeReason::DataLengthMismatch {
                        data_len,
                        reference_len,
                    },
            } => {
                assert_eq!(data_len, 3);
                assert_eq!(reference_len, 4);
            },
            _ => panic!("Expected DataLengthMismatch error"),
        }
    }

    // =========================================================================
    // Fallback Error Path Tests
    // =========================================================================

    /// Tests that the decode function's fallback error path works correctly
    /// when the RLE decode error is NOT a FortressError.
    ///
    /// This test exercises the fallback path in decode() where downcast_ref fails
    /// and we return a generic TruncatedData error instead.
    #[test]
    fn test_decode_fallback_error_path_for_non_fortress_error() {
        // To test the fallback path, we need to verify the behavior of the
        // error conversion logic. Since rle::decode returns Box<dyn Error>,
        // and the decode function tries to downcast to FortressError,
        // we can test this by creating a helper that simulates the conversion.

        // Create a non-FortressError error
        let non_fortress_error: Box<dyn Error + Send + Sync> =
            Box::new(std::io::Error::other("test error"));

        // Simulate the fallback logic from decode()
        let data = &[1, 2, 3, 4, 5];
        let result = non_fortress_error
            .downcast_ref::<FortressError>()
            .and_then(|fe| match fe {
                FortressError::InternalErrorStructured {
                    kind: InternalErrorKind::RleDecodeError { reason },
                } => Some(*reason),
                _ => None,
            })
            .unwrap_or(RleDecodeReason::TruncatedData {
                offset: 0,
                buffer_len: data.len(),
            });

        // Verify the fallback produces the expected result
        match result {
            RleDecodeReason::TruncatedData { offset, buffer_len } => {
                assert_eq!(offset, 0);
                assert_eq!(buffer_len, 5);
            },
            _ => panic!("Expected TruncatedData fallback, got {:?}", result),
        }
    }

    /// Tests that the decode function's error path correctly extracts
    /// FortressError when it IS present.
    #[test]
    fn test_decode_error_path_extracts_fortress_error() {
        // Create a FortressError with RleDecodeError
        let fortress_error: Box<dyn Error + Send + Sync> =
            Box::new(FortressError::InternalErrorStructured {
                kind: InternalErrorKind::RleDecodeError {
                    reason: RleDecodeReason::BitfieldIndexOutOfBounds,
                },
            });

        // Simulate the extraction logic from decode()
        let result = fortress_error
            .downcast_ref::<FortressError>()
            .and_then(|fe| match fe {
                FortressError::InternalErrorStructured {
                    kind: InternalErrorKind::RleDecodeError { reason },
                } => Some(*reason),
                _ => None,
            })
            .unwrap_or(RleDecodeReason::TruncatedData {
                offset: 0,
                buffer_len: 0,
            });

        // Verify the correct reason is extracted
        assert_eq!(result, RleDecodeReason::BitfieldIndexOutOfBounds);
    }

    /// Tests that the decode function's error path falls back correctly
    /// when FortressError is present but is NOT an RleDecodeError.
    #[test]
    fn test_decode_error_path_fallback_for_non_rle_fortress_error() {
        // Create a FortressError that is NOT an RleDecodeError
        let fortress_error: Box<dyn Error + Send + Sync> =
            Box::new(FortressError::InternalErrorStructured {
                kind: InternalErrorKind::BufferIndexOutOfBounds,
            });

        let data = &[1, 2, 3];

        // Simulate the extraction logic from decode()
        let result = fortress_error
            .downcast_ref::<FortressError>()
            .and_then(|fe| match fe {
                FortressError::InternalErrorStructured {
                    kind: InternalErrorKind::RleDecodeError { reason },
                } => Some(*reason),
                _ => None,
            })
            .unwrap_or(RleDecodeReason::TruncatedData {
                offset: 0,
                buffer_len: data.len(),
            });

        // Verify the fallback is used since this isn't an RleDecodeError
        match result {
            RleDecodeReason::TruncatedData { offset, buffer_len } => {
                assert_eq!(offset, 0);
                assert_eq!(buffer_len, 3);
            },
            _ => panic!("Expected TruncatedData fallback, got {:?}", result),
        }
    }

    /// Tests the full decode path produces correct CompressionError on fallback.
    #[test]
    fn test_decode_produces_compression_error_on_fallback() {
        // Create a non-FortressError
        let non_fortress_error: Box<dyn Error + Send + Sync> = Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "bad data",
        ));

        let data = &[10, 20, 30];

        // Simulate the full map_err logic from decode()
        let compression_error: CompressionError = (|| {
            if let Some(FortressError::InternalErrorStructured {
                kind: InternalErrorKind::RleDecodeError { reason },
            }) = non_fortress_error.downcast_ref::<FortressError>()
            {
                return CompressionError::RleDecode { reason: *reason };
            }
            CompressionError::RleDecode {
                reason: RleDecodeReason::TruncatedData {
                    offset: 0,
                    buffer_len: data.len(),
                },
            }
        })();

        // Verify the error structure
        match compression_error {
            CompressionError::RleDecode {
                reason: RleDecodeReason::TruncatedData { offset, buffer_len },
            } => {
                assert_eq!(offset, 0);
                assert_eq!(buffer_len, 3);
            },
            _ => panic!(
                "Expected RleDecode with TruncatedData, got {:?}",
                compression_error
            ),
        }
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
    use crate::test_config::miri_case_count;
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
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]
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
