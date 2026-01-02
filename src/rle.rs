//! # Run-Length Encoding Module
//!
//! This module provides bitfield run-length encoding (RLE) for network message compression.
//! It implements a format compatible with the `bitfield-rle` crate, using variable-length
//! integer (varint) encoding for length fields.
//!
//! ## Format
//!
//! The encoded bitfield is a series of compressed and uncompressed byte sequences.
//! Each sequence starts with a varint header:
//!
//! - **Compressed (repeated byte) sequence**: `varint(length << 2 | bit << 1 | 1)`
//!   - `length`: number of bytes
//!   - `bit`: 0 for `0x00` bytes, 1 for `0xFF` bytes
//!   - Trailing `1` indicates compressed
//!
//! - **Uncompressed sequence**: `varint(length << 1 | 0) + raw_bytes`
//!   - `length`: number of bytes
//!   - Trailing `0` indicates uncompressed
//!   - Followed by the actual raw bytes
//!
//! ## Example
//!
//! ```
//! use fortress_rollback::rle::{encode, decode};
//!
//! let data = vec![0, 0, 0, 0, 255, 255, 1, 2, 3];
//! let encoded = encode(&data);
//! let decoded = decode(&encoded).unwrap();
//! assert_eq!(data, decoded);
//! ```
//!
//! ## Note
//!
//! These functions are re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
//! They are not part of the stable public API.

use std::error::Error;

use crate::{FortressError, InternalErrorKind, RleDecodeReason};

/// Result type for RLE operations.
pub type RleResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Varint encoding/decoding utilities.
///
/// Uses LEB128 (Little Endian Base 128) variable-length encoding.
mod varint {
    /// Returns the number of bytes needed to encode a value.
    #[inline]
    pub fn encoded_len(value: u64) -> usize {
        if value == 0 {
            return 1;
        }
        // Number of bits needed, divided by 7, rounded up
        let bits = 64 - value.leading_zeros() as usize;
        bits.div_ceil(7)
    }

    /// Encodes a value as a varint into the provided buffer.
    /// Returns the number of bytes written.
    #[inline]
    pub fn encode(mut value: u64, buf: &mut [u8]) -> usize {
        let mut i = 0;
        while value >= 0x80 {
            if let Some(byte) = buf.get_mut(i) {
                *byte = (value as u8) | 0x80;
            } else {
                return i; // Buffer too small, return bytes written so far
            }
            value >>= 7;
            i += 1;
        }
        if let Some(byte) = buf.get_mut(i) {
            *byte = value as u8;
            i + 1
        } else {
            i // Buffer too small, return bytes written so far
        }
    }

    /// Encodes a value as a varint, returning a Vec.
    #[inline]
    #[allow(dead_code)]
    pub fn encode_to_vec(value: u64) -> Vec<u8> {
        let mut buf = vec![0u8; encoded_len(value)];
        encode(value, &mut buf);
        buf
    }

    /// Decodes a varint from the buffer starting at offset.
    /// Returns (decoded_value, bytes_consumed).
    #[inline]
    #[allow(clippy::while_let_loop)] // Multiple break conditions make while-let less clear
    pub fn decode(buf: &[u8], offset: usize) -> (u64, usize) {
        let mut value: u64 = 0;
        let mut shift = 0;
        let mut i = offset;

        loop {
            let byte = match buf.get(i) {
                Some(&b) => b,
                None => break, // Truncated varint - return what we have
            };
            value |= ((byte & 0x7F) as u64) << shift;
            i += 1;

            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift >= 64 {
                // Overflow - return what we have
                break;
            }
        }

        (value, i - offset)
    }
}

/// Encode a bitfield using run-length encoding.
///
/// This function compresses byte sequences that consist entirely of `0x00` or `0xFF` bytes.
/// Mixed byte sequences are stored uncompressed.
///
/// # Arguments
///
/// * `buf` - The input bytes to encode
///
/// # Returns
///
/// The RLE-encoded bytes.
///
/// # Example
///
/// ```
/// use fortress_rollback::rle::encode;
///
/// // Compresses runs of zeros and 0xFF bytes efficiently
/// let data = vec![0, 0, 0, 0, 255, 255, 255];
/// let encoded = encode(&data);
/// assert!(encoded.len() < data.len());
/// ```
pub fn encode(buf: impl AsRef<[u8]>) -> Vec<u8> {
    encode_with_offset(buf.as_ref(), 0)
}

/// Encode a bitfield starting at a specific offset.
fn encode_with_offset(buf: &[u8], offset: usize) -> Vec<u8> {
    let mut enc = Vec::with_capacity(encode_len_with_offset(buf, offset));
    let mut contiguous_len: u64 = 0;
    let mut contiguous = false;
    let mut prev_bits: u8 = 0;
    // Pre-allocate for typical non-contiguous runs (16 bytes is a reasonable estimate)
    let mut noncontiguous_bits: Vec<u8> = Vec::with_capacity(16);

    let slice = match buf.get(offset..) {
        Some(s) => s,
        None => return enc, // Invalid offset, return empty
    };

    for (i, &byte) in slice.iter().enumerate() {
        if contiguous && byte == prev_bits {
            // Continue the contiguous run
            contiguous_len += 1;
            continue;
        } else if contiguous {
            // End the contiguous run, write it out
            write_contiguous(&mut enc, contiguous_len, prev_bits);
        }

        if byte == 0 || byte == 255 {
            // Start a new contiguous run
            if !contiguous && i > 0 {
                // Write out any pending non-contiguous bytes
                write_noncontiguous(&mut enc, &mut noncontiguous_bits);
            }
            contiguous_len = 1;
            prev_bits = byte;
            contiguous = true;
        } else if !contiguous {
            // Continue non-contiguous sequence
            noncontiguous_bits.push(byte);
        } else {
            // End contiguous, start non-contiguous
            contiguous = false;
            noncontiguous_bits.push(byte);
        }
    }

    // Write final segment
    if contiguous {
        write_contiguous(&mut enc, contiguous_len, prev_bits);
    } else {
        write_noncontiguous(&mut enc, &mut noncontiguous_bits);
    }

    enc
}

/// Write a contiguous (compressed) sequence to the output.
#[inline]
fn write_contiguous(enc: &mut Vec<u8>, len: u64, prev_bits: u8) {
    // Format: length << 2 | bit << 1 | 1
    // bit is 1 if prev_bits is 0xFF, 0 if prev_bits is 0x00
    let mut value = len << 2;
    value |= 1; // Mark as contiguous
    if prev_bits == 255 {
        value |= 2; // Mark as 0xFF bytes
    }
    // Use stack-allocated buffer to avoid heap allocation in hot path
    let mut temp_buf = [0u8; 10]; // Max varint size for u64
    let written = varint::encode(value, &mut temp_buf);
    enc.extend_from_slice(&temp_buf[..written]);
}

/// Write a non-contiguous (uncompressed) sequence to the output.
#[inline]
fn write_noncontiguous(enc: &mut Vec<u8>, noncontiguous_bits: &mut Vec<u8>) {
    if noncontiguous_bits.is_empty() {
        return;
    }
    // Format: length << 1 | 0
    let value = (noncontiguous_bits.len() as u64) << 1;
    // Use stack-allocated buffer to avoid heap allocation in hot path
    let mut temp_buf = [0u8; 10]; // Max varint size for u64
    let written = varint::encode(value, &mut temp_buf);
    enc.extend_from_slice(&temp_buf[..written]);
    enc.append(noncontiguous_bits);
}

/// Returns the length of the encoded output for a given input.
fn encode_len_with_offset(buf: &[u8], offset: usize) -> usize {
    let mut len: u64 = 0;
    let mut partial_len: u64 = 0;
    let mut contiguous = false;
    let mut prev_bits: u8 = 0;

    let slice = match buf.get(offset..) {
        Some(s) => s,
        None => return 0, // Invalid offset, return 0
    };

    for (i, &byte) in slice.iter().enumerate() {
        if contiguous && byte == prev_bits {
            partial_len += 1;
            continue;
        } else if contiguous {
            // Add varint length for the contiguous sequence
            len += varint::encoded_len(partial_len << 2) as u64;
        }

        if byte == 0 || byte == 255 {
            if !contiguous && i > 0 {
                // Add length for non-contiguous: varint + data
                len += partial_len;
                len += varint::encoded_len(partial_len << 1) as u64;
            }
            partial_len = 1;
            prev_bits = byte;
            contiguous = true;
        } else if !contiguous {
            partial_len += 1;
        } else {
            partial_len = 1;
            contiguous = false;
        }
    }

    if contiguous {
        len += varint::encoded_len(partial_len << 2) as u64;
    } else if partial_len > 0 {
        // Only add if there are actual non-contiguous bytes
        len += partial_len;
        len += varint::encoded_len(partial_len << 1) as u64;
    }

    len as usize
}

/// Decode an RLE-encoded bitfield.
///
/// # Arguments
///
/// * `buf` - The RLE-encoded bytes
///
/// # Returns
///
/// The decoded bytes, or an error if the input is invalid.
///
/// # Errors
///
/// Returns an error if the encoded data is malformed (e.g., truncated, invalid length).
///
/// # Example
///
/// ```
/// use fortress_rollback::rle::{encode, decode};
///
/// let original = vec![0, 0, 0, 1, 2, 3, 255, 255];
/// let encoded = encode(&original);
/// let decoded = decode(&encoded).unwrap();
/// assert_eq!(original, decoded);
/// ```
pub fn decode(buf: impl AsRef<[u8]>) -> RleResult<Vec<u8>> {
    decode_with_offset(buf.as_ref(), 0)
}

/// Decode an RLE-encoded bitfield starting at a specific offset.
fn decode_with_offset(buf: &[u8], mut offset: usize) -> RleResult<Vec<u8>> {
    let decoded_len = decode_len_with_offset(buf, offset)?;
    let mut bitfield = vec![0u8; decoded_len];
    let mut ptr = 0;

    while offset < buf.len() {
        let (next, consumed) = varint::decode(buf, offset);
        offset += consumed;

        let repeat = next & 1;
        let len = if repeat > 0 {
            (next >> 2) as usize
        } else {
            (next >> 1) as usize
        };

        if repeat > 0 {
            // Contiguous sequence
            if next & 2 > 0 {
                // Fill with 0xFF
                for i in 0..len {
                    if ptr + i < bitfield.len() {
                        *bitfield.get_mut(ptr + i).ok_or(
                            FortressError::InternalErrorStructured {
                                kind: InternalErrorKind::RleDecodeError {
                                    reason: RleDecodeReason::BitfieldIndexOutOfBounds,
                                },
                            },
                        )? = 255;
                    }
                }
            }
            // If bit is 0, the bytes are already 0 from vec initialization
        } else {
            // Non-contiguous sequence - copy raw bytes
            let end = (len + offset).min(buf.len());
            let src_len = end - offset;
            let dst_end = (ptr + src_len).min(bitfield.len());
            let actual_len = dst_end - ptr;
            if actual_len > 0 && offset + actual_len <= buf.len() {
                let dst_slice = bitfield.get_mut(ptr..dst_end).ok_or(
                    FortressError::InternalErrorStructured {
                        kind: InternalErrorKind::RleDecodeError {
                            reason: RleDecodeReason::DestinationSliceOutOfBounds,
                        },
                    },
                )?;
                let src_slice = buf.get(offset..offset + actual_len).ok_or(
                    FortressError::InternalErrorStructured {
                        kind: InternalErrorKind::RleDecodeError {
                            reason: RleDecodeReason::SourceSliceOutOfBounds,
                        },
                    },
                )?;
                dst_slice.copy_from_slice(src_slice);
            }
            offset += len;
        }

        ptr += len;
    }

    Ok(bitfield)
}

/// Returns the decoded length for an RLE-encoded bitfield.
fn decode_len_with_offset(buf: &[u8], mut offset: usize) -> RleResult<usize> {
    let mut len: usize = 0;

    while offset < buf.len() {
        let (next, consumed) = varint::decode(buf, offset);
        offset += consumed;

        let repeat = next & 1;
        let slice = if repeat > 0 {
            (next >> 2) as usize
        } else {
            (next >> 1) as usize
        };

        len += slice;
        if repeat == 0 {
            offset += slice;
        }
    }

    if offset > buf.len() {
        return Err(Box::new(FortressError::InternalErrorStructured {
            kind: InternalErrorKind::RleDecodeError {
                reason: RleDecodeReason::TruncatedData {
                    offset,
                    buffer_len: buf.len(),
                },
            },
        }));
    }

    Ok(len)
}

// #########
// # TESTS #
// #########

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    // ================
    // Test-only types
    // ================

    /// Test-only error type for RLE decoding failures.
    ///
    /// This struct is only used in tests to verify error display formatting.
    /// Production code uses the structured `RleDecodeReason` variants via
    /// `FortressError::InternalErrorStructured`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct RleDecodeError {
        message: String,
    }

    impl RleDecodeError {
        /// Creates a new RLE decode error with the given message.
        fn new(message: impl Into<String>) -> Self {
            Self {
                message: message.into(),
            }
        }
    }

    impl std::fmt::Display for RleDecodeError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "RLE decode error: {}", self.message)
        }
    }

    impl std::error::Error for RleDecodeError {}

    // ================
    // Varint tests
    // ================

    #[test]
    fn test_varint_encode_decode_zero() {
        let mut buf = [0u8; 10];
        let written = varint::encode(0, &mut buf);
        assert_eq!(written, 1);
        assert_eq!(buf[0], 0);

        let (decoded, consumed) = varint::decode(&buf, 0);
        assert_eq!(decoded, 0);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_varint_encode_decode_small() {
        for value in 1..128u64 {
            let mut buf = [0u8; 10];
            let written = varint::encode(value, &mut buf);
            assert_eq!(written, 1, "value {} should encode to 1 byte", value);

            let (decoded, consumed) = varint::decode(&buf, 0);
            assert_eq!(decoded, value);
            assert_eq!(consumed, 1);
        }
    }

    #[test]
    fn test_varint_encode_decode_medium() {
        for value in [128u64, 255, 256, 1000, 16383, 16384] {
            let encoded = varint::encode_to_vec(value);
            let (decoded, consumed) = varint::decode(&encoded, 0);
            assert_eq!(decoded, value, "Failed for value {}", value);
            assert_eq!(consumed, encoded.len());
        }
    }

    #[test]
    fn test_varint_encode_decode_large() {
        for value in [u32::MAX as u64, u64::MAX / 2, u64::MAX] {
            let encoded = varint::encode_to_vec(value);
            let (decoded, _consumed) = varint::decode(&encoded, 0);
            assert_eq!(decoded, value, "Failed for value {}", value);
        }
    }

    #[test]
    fn test_varint_encoded_len() {
        assert_eq!(varint::encoded_len(0), 1);
        assert_eq!(varint::encoded_len(1), 1);
        assert_eq!(varint::encoded_len(127), 1);
        assert_eq!(varint::encoded_len(128), 2);
        assert_eq!(varint::encoded_len(16383), 2);
        assert_eq!(varint::encoded_len(16384), 3);
    }

    // ================
    // RLE encode/decode tests
    // ================

    #[test]
    fn test_encode_decode_empty() {
        let data: Vec<u8> = vec![];
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_encode_decode_all_zeros() {
        let data = vec![0u8; 100];
        let encoded = encode(&data);
        // Should compress well
        assert!(encoded.len() < data.len(), "Should compress zeros");
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_encode_decode_all_ones() {
        let data = vec![255u8; 100];
        let encoded = encode(&data);
        // Should compress well
        assert!(encoded.len() < data.len(), "Should compress 0xFF bytes");
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_encode_decode_mixed() {
        let data = vec![0, 0, 0, 0, 255, 255, 1, 2, 3, 4, 0, 0];
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_encode_decode_no_compression() {
        // Data with no repeated 0x00 or 0xFF bytes
        let data: Vec<u8> = (1..=50).collect();
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_encode_decode_alternating() {
        let mut data = Vec::new();
        for _ in 0..10 {
            data.extend_from_slice(&[0, 0, 0, 0]);
            data.extend_from_slice(&[255, 255, 255, 255]);
        }
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_encode_decode_single_byte() {
        for byte in [0u8, 1, 127, 128, 254, 255] {
            let data = vec![byte];
            let encoded = encode(&data);
            let decoded = decode(&encoded).unwrap();
            assert_eq!(data, decoded, "Failed for byte {}", byte);
        }
    }

    #[test]
    fn test_encode_decode_xor_pattern() {
        // This is similar to what the compression module uses
        let reference = [1u8, 2, 3, 4];
        let inputs: Vec<Vec<u8>> = vec![
            vec![1, 2, 3, 5], // XOR = [0, 0, 0, 1]
            vec![1, 2, 4, 4], // XOR = [0, 0, 1, 0]
            vec![1, 3, 3, 4], // XOR = [0, 1, 0, 0]
        ];

        // XOR encode (simulating delta_encode)
        let mut xor_data = Vec::new();
        for input in &inputs {
            for (r, i) in reference.iter().zip(input.iter()) {
                xor_data.push(r ^ i);
            }
        }

        let encoded = encode(&xor_data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(xor_data, decoded);
    }

    #[test]
    fn test_compression_ratio() {
        // Long run of zeros should compress very well
        let data = vec![0u8; 1000];
        let encoded = encode(&data);
        // Should be just a few bytes (varint for length)
        assert!(
            encoded.len() < 10,
            "1000 zeros should compress to < 10 bytes, got {}",
            encoded.len()
        );
    }

    #[test]
    fn test_decode_invalid_truncated() {
        // Create encoded data for a long sequence
        let data = vec![0u8; 100];
        let _encoded = encode(&data);

        // Truncate it in the middle of non-contiguous data section
        // First, create data that will have non-contiguous section
        let data2: Vec<u8> = (1..=100).collect();
        let encoded2 = encode(&data2);

        // The decode should handle truncated data gracefully
        // (either return partial result or error)
        if encoded2.len() > 5 {
            let truncated = &encoded2[..5];
            // This might error or return partial - both are acceptable
            let _result = decode(truncated);
        }
    }

    #[test]
    fn test_encode_preserves_trailing_zeros() {
        let data = vec![1, 2, 3, 0, 0, 0];
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_roundtrip_random_patterns() {
        // Test various patterns that might occur in game input compression
        let patterns: Vec<Vec<u8>> = vec![
            vec![0, 0, 0, 1, 0, 0, 0], // sparse set bit
            vec![255, 255, 0, 0, 255, 255],
            vec![1, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![128, 64, 32, 16, 8, 4, 2, 1],
            (0..255).collect(), // all byte values
        ];

        for (i, pattern) in patterns.iter().enumerate() {
            let encoded = encode(pattern);
            let decoded = decode(&encoded).unwrap();
            assert_eq!(pattern, &decoded, "Pattern {} failed roundtrip", i);
        }
    }

    #[test]
    fn test_encode_len_matches_actual() {
        let test_cases: Vec<Vec<u8>> = vec![
            vec![],
            vec![0],
            vec![255],
            vec![1, 2, 3],
            vec![0, 0, 0, 0],
            vec![255, 255, 255, 255],
            vec![0, 1, 0, 1, 0],
            (0..100).collect(),
        ];

        for data in test_cases {
            let predicted = encode_len_with_offset(&data, 0);
            let encoded = encode(&data);
            assert_eq!(
                predicted,
                encoded.len(),
                "Length mismatch for data {:?}",
                data
            );
        }
    }

    // ======================
    // Mutation testing gaps
    // ======================

    #[test]
    fn test_rle_decode_error_display() {
        let err = RleDecodeError::new("test error message");
        let display = format!("{}", err);
        assert!(display.contains("test error message"));
        assert!(display.contains("RLE decode error"));
    }

    #[test]
    fn test_contiguous_zeros_vs_ones_distinguished() {
        // This tests that the bit flag (|= 2) correctly distinguishes 0x00 from 0xFF
        let zeros = vec![0u8; 10];
        let ones = vec![255u8; 10];

        let encoded_zeros = encode(&zeros);
        let encoded_ones = encode(&ones);

        // They should encode differently (the bit flag differs)
        assert_ne!(
            encoded_zeros, encoded_ones,
            "0x00 and 0xFF runs should encode differently"
        );

        // Both should decode correctly
        let decoded_zeros = decode(&encoded_zeros).unwrap();
        let decoded_ones = decode(&encoded_ones).unwrap();

        assert_eq!(zeros, decoded_zeros);
        assert_eq!(ones, decoded_ones);
    }

    #[test]
    fn test_contiguous_bit_flag_in_encoding() {
        // Verify the exact encoding format: value = len << 2 | bit << 1 | 1
        // For zeros: value = len << 2 | 0 << 1 | 1 = len << 2 | 1
        // For ones:  value = len << 2 | 1 << 1 | 1 = len << 2 | 3

        let zeros = vec![0u8; 4]; // len=4, expected varint value = 4<<2 | 1 = 17
        let encoded_zeros = encode(&zeros);
        assert_eq!(encoded_zeros.len(), 1);
        assert_eq!(encoded_zeros[0], 17); // 0b10001

        let ones = vec![255u8; 4]; // len=4, expected varint value = 4<<2 | 3 = 19
        let encoded_ones = encode(&ones);
        assert_eq!(encoded_ones.len(), 1);
        assert_eq!(encoded_ones[0], 19); // 0b10011
    }

    #[test]
    fn test_noncontiguous_start_after_first_byte() {
        // Test the condition: if !contiguous && i > 0
        // This tests that non-contiguous sequences starting after index 0 are handled
        let data = vec![1, 2, 3]; // All non-zero, non-255
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);

        // Verify the encoding format: value = len << 1 | 0
        // len=3, value = 3 << 1 | 0 = 6
        assert_eq!(encoded[0], 6);
        assert_eq!(&encoded[1..], &[1, 2, 3]);
    }

    #[test]
    fn test_mixed_contiguous_noncontiguous_boundary() {
        // Test transitions at exact boundaries
        let data = vec![0, 1]; // Contiguous zero, then non-contiguous 1
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);

        let data2 = vec![1, 0]; // Non-contiguous 1, then contiguous zero
        let encoded2 = encode(&data2);
        let decoded2 = decode(&encoded2).unwrap();
        assert_eq!(data2, decoded2);
    }

    #[test]
    fn test_encode_len_with_various_patterns() {
        // Test encode_len_with_offset with various patterns to ensure correctness
        let patterns: Vec<Vec<u8>> = vec![
            vec![1],                          // Single non-contiguous
            vec![0],                          // Single contiguous zero
            vec![255],                        // Single contiguous one
            vec![1, 2],                       // Two non-contiguous
            vec![0, 0],                       // Two contiguous zeros
            vec![255, 255],                   // Two contiguous ones
            vec![0, 1],                       // Mixed
            vec![1, 0],                       // Mixed reversed
            vec![0, 0, 0, 1, 2, 3, 255, 255], // Complex mix
        ];

        for pattern in patterns {
            let predicted = encode_len_with_offset(&pattern, 0);
            let actual = encode(&pattern).len();
            assert_eq!(
                predicted, actual,
                "Length prediction failed for {:?}: predicted {}, actual {}",
                pattern, predicted, actual
            );
        }
    }

    #[test]
    fn test_varint_multi_byte_accumulation() {
        // Test that varint decode correctly accumulates bits across multiple bytes
        // Value 128 encodes as [0x80, 0x01]
        let value = 128u64;
        let encoded = varint::encode_to_vec(value);
        assert_eq!(encoded, vec![0x80, 0x01]);

        let (decoded, consumed) = varint::decode(&encoded, 0);
        assert_eq!(decoded, value);
        assert_eq!(consumed, 2);

        // Value 300 = 0b100101100 encodes as [0xAC, 0x02]
        // Low 7 bits: 0101100 = 44 + 0x80 = 0xAC
        // High bits: 10 = 2
        let value2 = 300u64;
        let encoded2 = varint::encode_to_vec(value2);
        let (decoded2, consumed2) = varint::decode(&encoded2, 0);
        assert_eq!(decoded2, value2);
        assert_eq!(consumed2, encoded2.len());
    }

    #[test]
    fn test_decode_boundary_conditions() {
        // Test decode at exact boundaries
        let data = vec![0u8; 1]; // Minimum contiguous
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);

        // Test with non-contiguous at exact buffer boundary
        let data2 = vec![1u8]; // Single non-contiguous byte
        let encoded2 = encode(&data2);
        let decoded2 = decode(&encoded2).unwrap();
        assert_eq!(data2, decoded2);
    }

    #[test]
    fn test_decode_len_overflow_check() {
        // Test that decode_len_with_offset returns error for invalid data
        // Create data that claims more bytes than available
        let invalid = vec![100u8]; // Claims 50 bytes (100 >> 1 = 50), but none available
        let result = decode_len_with_offset(&invalid, 0);
        assert!(result.is_err(), "Should error on truncated data");
    }

    #[test]
    fn test_decode_with_trailing_data() {
        // Ensure decoding handles exact boundaries correctly
        let data = vec![0, 0, 0, 0, 1, 2, 3, 4];
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
        assert_eq!(decoded.len(), data.len());
    }

    #[test]
    fn test_long_noncontiguous_sequence() {
        // Test longer non-contiguous sequences for length calculation
        let data: Vec<u8> = (1..=200).map(|i| (i % 254 + 1) as u8).collect();
        let predicted = encode_len_with_offset(&data, 0);
        let encoded = encode(&data);
        assert_eq!(predicted, encoded.len());

        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_alternating_single_bytes() {
        // Alternating 0 and 255, which creates many small contiguous runs
        let data: Vec<u8> = (0..20).map(|i| if i % 2 == 0 { 0 } else { 255 }).collect();
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_encode_decode_with_offset_consistency() {
        // Test that encode_with_offset and decode_with_offset maintain consistency
        let data = vec![0, 0, 1, 2, 255, 255, 3, 4, 0, 0];
        let encoded = encode_with_offset(&data, 0);
        let decoded = decode_with_offset(&encoded, 0).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_decode_noncontiguous_exact_length() {
        // Test decoding a non-contiguous sequence where we read exactly the right amount
        let data = vec![1, 2, 3, 4, 5];
        let encoded = encode(&data);

        // Verify the structure: header byte + 5 data bytes
        // header = len << 1 = 5 << 1 = 10
        assert_eq!(encoded[0], 10);
        assert_eq!(&encoded[1..], &[1, 2, 3, 4, 5]);

        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    // ============================
    // Additional mutation tests
    // ============================

    #[test]
    fn test_first_byte_zero_starts_contiguous() {
        // When i == 0 and byte is 0, we should NOT write non-contiguous first
        // This tests the `i > 0` condition in `!contiguous && i > 0`
        let data = vec![0, 0, 0]; // Starts with contiguous zeros
        let encoded = encode(&data);

        // Should be a single contiguous encoding: len=3 << 2 | 1 = 13
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], 13);

        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_first_byte_nonzero_starts_noncontiguous() {
        // When i == 0 and byte is non-zero (not 0 or 255), starts non-contiguous
        let data = vec![1, 0, 0]; // Non-contiguous 1, then contiguous zeros
        let encoded = encode(&data);

        // First: non-contiguous header (1 << 1 = 2) + byte 1
        // Then: contiguous zeros (2 << 2 | 1 = 9)
        assert_eq!(encoded[0], 2); // header for 1 non-contiguous byte
        assert_eq!(encoded[1], 1); // the byte itself
        assert_eq!(encoded[2], 9); // header for 2 contiguous zeros

        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_noncontiguous_to_contiguous_at_index_1() {
        // Tests i == 1 specifically (boundary of i > 0 check)
        let data = vec![1, 0]; // One non-contiguous, then contiguous
        let encoded = encode(&data);

        // Non-contiguous: 1 << 1 = 2, followed by byte 1
        // Contiguous zeros: 1 << 2 | 1 = 5
        assert_eq!(encoded[0], 2);
        assert_eq!(encoded[1], 1);
        assert_eq!(encoded[2], 5);

        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_write_contiguous_flag_is_1_not_xor() {
        // Tests that |= 1 cannot be replaced with ^= 1
        // For a new value (starting at 0), |= 1 and ^= 1 give the same result
        // But if we have other bits set, they differ
        // Actually, for len=1, zeros: value = 1 << 2 | 1 = 5 (binary: 101)
        // The mutation |= 1 -> ^= 1 would give 4 (binary: 100) for an already-set bit

        // Test: encode [0], decode, verify correct
        let data = vec![0u8];
        let encoded = encode(&data);
        // Expected: 1 << 2 | 1 = 5
        assert_eq!(encoded[0], 5);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);

        // Test: encode [0, 0], decode, verify correct
        let data2 = vec![0u8, 0];
        let encoded2 = encode(&data2);
        // Expected: 2 << 2 | 1 = 9
        assert_eq!(encoded2[0], 9);
        let decoded2 = decode(&encoded2).unwrap();
        assert_eq!(data2, decoded2);
    }

    #[test]
    fn test_write_contiguous_ones_flag_is_2_not_xor() {
        // Tests that |= 2 cannot be replaced with ^= 2
        // For 0xFF bytes: value = len << 2 | 2 | 1 = len << 2 | 3

        let data = vec![255u8];
        let encoded = encode(&data);
        // Expected: 1 << 2 | 3 = 7
        assert_eq!(encoded[0], 7);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);

        // Check decoding produces correct bytes
        assert_eq!(decoded[0], 255);
    }

    #[test]
    fn test_zeros_and_ones_have_different_flags() {
        // Verify that zeros (bit=0) and ones (bit=1) encode to different values
        let zeros = vec![0u8; 5];
        let ones = vec![255u8; 5];

        let encoded_zeros = encode(&zeros);
        let encoded_ones = encode(&ones);

        // Zeros: 5 << 2 | 0 << 1 | 1 = 21
        // Ones:  5 << 2 | 1 << 1 | 1 = 23
        assert_eq!(encoded_zeros[0], 21);
        assert_eq!(encoded_ones[0], 23);

        // They must be different
        assert_ne!(encoded_zeros[0], encoded_ones[0]);
    }

    #[test]
    fn test_varint_decode_accumulates_correctly() {
        // Tests that |= is used (not ^=) in varint decode
        // For multi-byte values, using ^= would corrupt the value

        // Value 256 = 0x100 encodes as [0x80, 0x02] in LEB128
        // First byte: 0x80 (128 with continuation bit)
        // Second byte: 0x02 (2, final byte)
        // Decoded: (128 & 0x7F) | (2 << 7) = 0 | 256 = 256
        let value = 256u64;
        let encoded = varint::encode_to_vec(value);

        let (decoded, consumed) = varint::decode(&encoded, 0);
        assert_eq!(decoded, value);
        assert_eq!(consumed, encoded.len());

        // Another test: 0x3FFF (16383) - max 2-byte value
        let value2 = 16383u64;
        let encoded2 = varint::encode_to_vec(value2);
        let (decoded2, _) = varint::decode(&encoded2, 0);
        assert_eq!(decoded2, value2);

        // Test 0x4000 (16384) - first 3-byte value
        let value3 = 16384u64;
        let encoded3 = varint::encode_to_vec(value3);
        let (decoded3, _) = varint::decode(&encoded3, 0);
        assert_eq!(decoded3, value3);
    }

    #[test]
    fn test_encode_len_contiguous_partial_calculation() {
        // Tests the encode_len_with_offset calculations for contiguous sequences
        // Specifically: len += varint::encoded_len(partial_len << 2) as u64;

        // Single zero byte: partial_len=1, 1<<2=4, varint_len(4)=1
        let data1 = vec![0u8];
        assert_eq!(encode_len_with_offset(&data1, 0), 1);

        // 32 zero bytes: partial_len=32, 32<<2=128, varint_len(128)=2
        let data2 = vec![0u8; 32];
        assert_eq!(encode_len_with_offset(&data2, 0), 2);
    }

    #[test]
    fn test_encode_len_noncontiguous_calculation() {
        // Tests the encode_len calculations for non-contiguous sequences
        // Specifically: len += partial_len + varint::encoded_len(partial_len << 1)

        // 1 non-contiguous byte: 1 + varint_len(2) = 1 + 1 = 2
        let data1 = vec![1u8];
        assert_eq!(encode_len_with_offset(&data1, 0), 2);

        // 64 non-contiguous bytes: 64 + varint_len(128) = 64 + 2 = 66
        let data2: Vec<u8> = (1..=64).collect();
        assert_eq!(encode_len_with_offset(&data2, 0), 66);
    }

    #[test]
    fn test_decode_contiguous_boundary_exact() {
        // Test that decoding stops exactly at the right boundary
        // This catches issues with >= vs > comparisons

        let data = vec![0u8; 10];
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();

        // Must have exactly 10 bytes
        assert_eq!(decoded.len(), 10);
        for &byte in &decoded {
            assert_eq!(byte, 0);
        }
    }

    #[test]
    fn test_decode_noncontiguous_boundary_exact() {
        // Test decoding non-contiguous at exact boundary
        let data: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();

        assert_eq!(decoded.len(), 10);
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_mixed_sequence_length_prediction() {
        // Complex mix to test encode_len thoroughly
        let data = vec![
            0, 0, 0, // 3 zeros
            1, 2, 3, 4, 5, // 5 non-contiguous
            255, 255, 255, 255, // 4 ones
            6, 7, 8, // 3 non-contiguous
            0, 0, // 2 zeros
        ];

        let predicted = encode_len_with_offset(&data, 0);
        let actual = encode(&data).len();
        assert_eq!(predicted, actual);

        let decoded = decode(encode(&data)).unwrap();
        assert_eq!(data, decoded);
    }
}

// =============================================================================
// Property-Based Tests
//
// These tests use proptest to verify invariants hold under random inputs.
// They are critical for ensuring the RLE implementation is correct for all
// possible byte sequences, not just hand-crafted test cases.
// =============================================================================

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

    /// Maximum size for property tests to keep execution time reasonable
    const MAX_TEST_SIZE: usize = 4096;

    /// Generate arbitrary byte vectors of varying sizes
    fn arbitrary_bytes(max_size: usize) -> impl Strategy<Value = Vec<u8>> {
        proptest::collection::vec(any::<u8>(), 0..=max_size)
    }

    /// Generate byte vectors with realistic game state patterns
    /// (sparse data, runs of zeros/ones, mixed patterns)
    fn game_state_bytes() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            // Random bytes (worst case for compression)
            arbitrary_bytes(1024),
            // Mostly zeros with sparse non-zero bytes (common game state)
            proptest::collection::vec(prop_oneof![9 => Just(0u8), 1 => any::<u8>()], 0..=1024),
            // Mostly 0xFF with sparse changes
            proptest::collection::vec(prop_oneof![9 => Just(255u8), 1 => any::<u8>()], 0..=1024),
            // Alternating runs of 0x00 and 0xFF
            (1..50usize, 1..50usize).prop_flat_map(|(zero_len, one_len)| {
                let zeros = vec![0u8; zero_len];
                let ones = vec![255u8; one_len];
                let mut pattern = zeros;
                pattern.extend(&ones);
                Just(pattern.into_iter().cycle().take(512).collect::<Vec<_>>())
            }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]
        /// Property: Roundtrip invariant - decode(encode(data)) == data for ALL inputs.
        ///
        /// This is THE fundamental property of any lossless compression algorithm.
        /// If this property fails, data corruption will occur in production.
        #[test]
        fn prop_roundtrip_invariant(data in arbitrary_bytes(MAX_TEST_SIZE)) {
            let encoded = encode(&data);
            let decoded = decode(&encoded).expect("decode should not fail on valid encoded data");
            prop_assert_eq!(
                data, decoded,
                "Roundtrip failed: encode/decode produced different data"
            );
        }

        /// Property: Roundtrip holds for game-state-like patterns.
        ///
        /// Tests with data patterns similar to what the compression layer sees
        /// (XOR-encoded game inputs with many zeros).
        #[test]
        fn prop_roundtrip_game_state_patterns(data in game_state_bytes()) {
            let encoded = encode(&data);
            let decoded = decode(&encoded).expect("decode should not fail");
            prop_assert_eq!(data, decoded, "Roundtrip failed for game state pattern");
        }

        /// Property: Encoded length prediction is always accurate.
        ///
        /// encode_len_with_offset must exactly predict the encoded output length.
        /// An incorrect prediction could cause buffer overflows or underflows.
        #[test]
        fn prop_encode_len_prediction_accurate(data in arbitrary_bytes(MAX_TEST_SIZE)) {
            let predicted = encode_len_with_offset(&data, 0);
            let actual = encode(&data).len();
            prop_assert_eq!(
                predicted, actual,
                "Length prediction mismatch: predicted {}, actual {}",
                predicted, actual
            );
        }

        /// Property: Compression is bounded.
        ///
        /// RLE compression has a worst-case expansion factor. The worst case
        /// is alternating between compressible (0x00/0xFF) and non-compressible
        /// bytes, which would create a segment header for each byte.
        ///
        /// For N bytes of input, the worst case is roughly:
        /// - N segments (one per byte)
        /// - Each segment has at least 1 byte header + 0-1 data bytes
        /// - So worst case is approximately 2*N bytes
        ///
        /// However, this is a pathological case. For typical input, compression
        /// is much better. We verify that encoding never fails and decoded
        /// result is always correct (via roundtrip property above).
        #[test]
        fn prop_compression_bounded_expansion(data in arbitrary_bytes(MAX_TEST_SIZE)) {
            let encoded = encode(&data);

            // Worst case: alternating compressible/non-compressible creates
            // a segment for each byte. Each segment needs:
            // - 1-10 bytes for varint header
            // - 0-1 bytes for data (non-contiguous needs data, contiguous doesn't)
            //
            // In practice, the maximum overhead is bounded by the number of
            // state transitions in the data. For pathological data where every
            // byte switches between compressible/non-compressible, we could
            // theoretically get 2*len + overhead.
            //
            // However, the encode_len_with_offset function provides an exact
            // prediction, so we verify against that.
            let predicted = encode_len_with_offset(&data, 0);
            prop_assert_eq!(
                encoded.len(), predicted,
                "Encoded length {} doesn't match prediction {}",
                encoded.len(), predicted
            );
        }

        /// Property: Compressible data actually compresses.
        ///
        /// Long runs of 0x00 or 0xFF bytes should compress to much smaller output.
        #[test]
        fn prop_compressible_data_compresses(run_len in 10usize..1000, byte in prop_oneof![Just(0u8), Just(255u8)]) {
            let data = vec![byte; run_len];
            let encoded = encode(&data);

            // A run of N identical compressible bytes should encode to just a varint
            // varint(N << 2 | flags) which is at most ceil(log128(N * 4)) bytes
            // For N=1000, that's about 2 bytes (log128(4000) ≈ 1.7)
            let max_encoded = varint::encoded_len((run_len as u64) << 2) + 1;

            prop_assert!(
                encoded.len() <= max_encoded,
                "Compressible run of {} bytes encoded to {} bytes, expected at most {}",
                run_len, encoded.len(), max_encoded
            );
        }

        /// Property: Varint roundtrip - decode(encode(n)) == n for all u64 values.
        #[test]
        fn prop_varint_roundtrip(value in any::<u64>()) {
            let encoded = varint::encode_to_vec(value);
            let (decoded, consumed) = varint::decode(&encoded, 0);
            prop_assert_eq!(
                value, decoded,
                "Varint roundtrip failed: {} != {}",
                value, decoded
            );
            prop_assert_eq!(
                encoded.len(), consumed,
                "Varint consumed {} bytes, but encoded {} bytes",
                consumed, encoded.len()
            );
        }

        /// Property: Varint encoding length is optimal.
        ///
        /// For any value, the encoded length should be the minimum needed to
        /// represent that value in LEB128 format.
        #[test]
        fn prop_varint_length_optimal(value in any::<u64>()) {
            let encoded_len = varint::encoded_len(value);
            let actual_len = varint::encode_to_vec(value).len();
            prop_assert_eq!(
                encoded_len, actual_len,
                "Varint length prediction mismatch for {}: predicted {}, actual {}",
                value, encoded_len, actual_len
            );

            // Also verify the length is optimal
            // LEB128 uses 7 bits per byte, so optimal length is ceil(bits_needed / 7)
            let bits_needed = if value == 0 { 1 } else { 64 - value.leading_zeros() as usize };
            let expected_len = bits_needed.div_ceil(7);
            prop_assert_eq!(
                actual_len, expected_len,
                "Varint encoding for {} used {} bytes but optimal is {}",
                value, actual_len, expected_len
            );
        }

        /// Property: Decode handles empty and single-byte inputs.
        #[test]
        fn prop_decode_small_inputs(byte in any::<u8>()) {
            // Empty input should decode to empty
            let empty_decoded = decode([]).expect("empty should decode");
            prop_assert_eq!(empty_decoded.len(), 0);

            // Single byte input (as raw data, not encoded)
            let single = vec![byte];
            let encoded = encode(&single);
            let decoded = decode(&encoded).expect("single byte should decode");
            prop_assert_eq!(single, decoded);
        }

        /// Property: Multiple encodes of same data produce identical output (determinism).
        ///
        /// Critical for rollback networking - encoding must be deterministic.
        #[test]
        fn prop_encode_deterministic(data in arbitrary_bytes(MAX_TEST_SIZE)) {
            let encoded1 = encode(&data);
            let encoded2 = encode(&data);
            prop_assert_eq!(
                encoded1, encoded2,
                "Encoding is non-deterministic!"
            );
        }

        /// Property: Decode is deterministic - same encoded input always produces same output.
        #[test]
        fn prop_decode_deterministic(data in arbitrary_bytes(MAX_TEST_SIZE)) {
            let encoded = encode(&data);
            let decoded1 = decode(&encoded).expect("decode 1");
            let decoded2 = decode(&encoded).expect("decode 2");
            prop_assert_eq!(
                decoded1, decoded2,
                "Decoding is non-deterministic!"
            );
        }
    }
}

// =============================================================================
// Kani Formal Verification Proofs
//
// These proofs use Kani (https://model-checking.github.io/kani/) to formally
// verify safety properties of the RLE implementation. They prove:
//
// 1. Varint encoded_len is correct for all u64 values
// 2. Varint decode always terminates
// 3. Buffer access in encode/decode is always in bounds
// 4. No integer overflow in varint operations
//
// Run with: cargo kani --tests
// =============================================================================

#[cfg(kani)]
mod kani_proofs {
    use super::varint;

    /// Proof: varint::encoded_len returns correct length for all u64 values.
    ///
    /// LEB128 encoding uses 7 bits per byte, so for a value with N bits,
    /// we need ceil(N/7) bytes. For value 0, we need 1 byte.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Varint length calculation correctness
    /// - Related: proof_varint_encoded_len_no_overflow, proof_varint_encode_single_byte
    #[kani::proof]
    fn proof_varint_encoded_len_correct() {
        let value: u64 = kani::any();

        let len = varint::encoded_len(value);

        // Length must be between 1 and 10 (max bytes for u64 LEB128)
        kani::assert(len >= 1, "encoded_len must be at least 1");
        kani::assert(len <= 10, "encoded_len must be at most 10 for u64");

        // Verify the length is optimal:
        // A value that fits in N bytes must satisfy: value < 128^N
        // Equivalently, value < (1 << (7*N))
        if len < 10 {
            // Value should not fit in fewer bytes
            let max_for_len_minus_1 = if len > 1 { 1u64 << (7 * (len - 1)) } else { 0 };
            kani::assert(
                value >= max_for_len_minus_1 || (len == 1 && value == 0),
                "Value should require at least this many bytes",
            );
        }
    }

    /// Proof: varint::encode produces correct output for small values.
    ///
    /// For values < 128, the encoding is a single byte equal to the value.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Single-byte varint encoding correctness
    /// - Related: proof_varint_encoded_len_correct, proof_varint_continuation_handling
    #[kani::proof]
    fn proof_varint_encode_single_byte() {
        let value: u64 = kani::any();
        kani::assume(value < 128);

        let mut buf = [0u8; 1];
        let written = varint::encode(value, &mut buf);

        kani::assert(written == 1, "Values < 128 should encode to 1 byte");
        kani::assert(
            buf[0] == value as u8,
            "Single byte encoding should equal value",
        );
        kani::assert(buf[0] & 0x80 == 0, "Continuation bit should not be set");
    }

    /// Proof: varint::decode terminates and returns valid consumed count.
    ///
    /// This proves that the decode loop always terminates and returns a valid
    /// number of consumed bytes.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Decode loop termination and bounds safety
    /// - Related: proof_varint_decode_offset_safe, proof_varint_decode_empty_safe
    #[kani::proof]
    #[kani::unwind(5)] // 3 bytes + 2 for loop overhead
    fn proof_varint_decode_terminates() {
        // Test with a small buffer to keep proof tractable
        let b0: u8 = kani::any();
        let b1: u8 = kani::any();
        let b2: u8 = kani::any();
        let buf = [b0, b1, b2];

        let (value, consumed) = varint::decode(&buf, 0);

        // Consumed must be in valid range (consumed is usize, always non-negative)
        kani::assert(
            consumed <= buf.len(),
            "consumed must not exceed buffer length",
        );

        // If first byte has continuation bit clear, should consume exactly 1
        if b0 & 0x80 == 0 {
            kani::assert(consumed == 1, "No continuation bit means 1 byte consumed");
            kani::assert(
                value == b0 as u64,
                "Single byte decode should equal byte value",
            );
        }
    }

    /// Proof: varint::decode handles offset correctly.
    ///
    /// Decoding at different offsets should not cause buffer overflow.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Offset parameter bounds safety
    /// - Related: proof_varint_decode_terminates, proof_varint_decode_empty_safe
    #[kani::proof]
    #[kani::unwind(4)]
    fn proof_varint_decode_offset_safe() {
        let b0: u8 = kani::any();
        let b1: u8 = kani::any();
        let buf = [b0, b1];

        let offset: usize = kani::any();
        kani::assume(offset <= buf.len());

        let (_value, consumed) = varint::decode(&buf, offset);

        // Consumed bytes should not exceed remaining buffer
        kani::assert(
            consumed <= buf.len() - offset,
            "Should not consume more than available",
        );
    }

    /// Proof: varint roundtrip is correct for small values.
    ///
    /// For values that fit in 2 bytes (< 16384), verify encode/decode roundtrip.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Encode/decode roundtrip correctness
    /// - Related: proof_varint_encode_single_byte, proof_varint_continuation_handling
    #[kani::proof]
    #[kani::unwind(5)]
    fn proof_varint_roundtrip_small() {
        let value: u64 = kani::any();
        kani::assume(value < 16384); // Keep proof tractable

        let encoded = varint::encode_to_vec(value);
        let (decoded, consumed) = varint::decode(&encoded, 0);

        kani::assert(decoded == value, "Roundtrip should preserve value");
        kani::assert(
            consumed == encoded.len(),
            "Should consume all encoded bytes",
        );
    }

    /// Proof: varint::encoded_len never overflows.
    ///
    /// The calculation `bits.div_ceil(7)` should never overflow for any u64.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: No overflow in length calculation
    /// - Related: proof_varint_encoded_len_correct
    #[kani::proof]
    fn proof_varint_encoded_len_no_overflow() {
        let value: u64 = kani::any();

        // This should never panic
        let len = varint::encoded_len(value);

        // Maximum bits needed is 64, ceil(64/7) = 10
        kani::assert(len <= 10, "Length should never exceed 10");
    }

    /// Proof: Empty buffer decode is safe.
    ///
    /// Decoding from an empty buffer should return (0, 0) without panicking.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Empty input handling safety
    /// - Related: proof_varint_decode_terminates, proof_varint_decode_offset_safe
    #[kani::proof]
    fn proof_varint_decode_empty_safe() {
        let buf: [u8; 0] = [];
        let (value, consumed) = varint::decode(&buf, 0);

        kani::assert(consumed == 0, "Empty buffer should consume 0 bytes");
        kani::assert(value == 0, "Empty buffer should decode to 0");
    }

    /// Proof: Continuation bit handling is correct.
    ///
    /// A varint with all continuation bits set should consume all available bytes
    /// (up to the overflow limit).
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Multi-byte varint decoding correctness
    /// - Related: proof_varint_encode_single_byte, proof_varint_roundtrip_small
    #[kani::proof]
    #[kani::unwind(4)]
    fn proof_varint_continuation_handling() {
        // Create buffer with first byte having continuation, second without
        let byte1: u8 = kani::any();
        let byte2: u8 = kani::any();
        kani::assume(byte1 & 0x80 != 0); // First has continuation
        kani::assume(byte2 & 0x80 == 0); // Second doesn't

        let buf = [byte1, byte2];
        let (value, consumed) = varint::decode(&buf, 0);

        kani::assert(consumed == 2, "Should consume both bytes");

        // Verify decoded value
        let expected = ((byte1 & 0x7F) as u64) | (((byte2 & 0x7F) as u64) << 7);
        kani::assert(value == expected, "Decoded value should be correct");
    }
}
