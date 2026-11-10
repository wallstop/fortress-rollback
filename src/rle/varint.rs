//! LEB128 varint encoding/decoding, isolated for source-linked verification.
//!
//! This module deliberately has **no dependencies outside `core`**: it does not
//! reference `crate::*`, `super::*`, allocation, or the RLE error type. The
//! production source can therefore be extracted unchanged by Hax/Aeneas (see
//! `.agents/skills/kani/references/kani-obligations.md`), while `rle.rs` maps
//! [`VarintError`] into the public [`FortressError`] at the seam.
//!
//! Keep it dependency-free: adding a `use crate::...`, a `Vec`, or a trait
//! object to this file breaks the extraction smoke package.
//!
//! [`FortressError`]: crate::FortressError

/// Error raised by [`decode`].
///
/// The variants carry exactly the data the RLE layer needs to reproduce its
/// public `RleDecodeReason` payload: the truncation point or the malformed
/// varint's starting offset.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum VarintError {
    /// The buffer ended before the varint terminator.
    Truncated {
        /// Offset at which the missing byte was expected.
        offset: usize,
        /// Length of the buffer that was too short.
        buffer_len: usize,
    },
    /// A continuation would overflow the `u64` payload.
    Malformed {
        /// Starting offset of the malformed varint.
        offset: usize,
    },
}

/// Result type for varint operations.
pub(crate) type VarintResult<T> = Result<T, VarintError>;

/// Returns the number of bytes needed to encode a value.
#[inline]
pub(crate) fn encoded_len(value: u64) -> usize {
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
pub(crate) fn encode(mut value: u64, buf: &mut [u8]) -> usize {
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

/// Decodes a varint from the buffer starting at offset.
/// Returns (decoded_value, bytes_consumed).
#[inline]
#[allow(clippy::while_let_loop)] // Multiple break conditions make while-let less clear
pub(crate) fn decode(buf: &[u8], offset: usize) -> VarintResult<(u64, usize)> {
    let mut value: u64 = 0;
    let mut shift = 0;
    let mut i = offset;

    loop {
        let byte = match buf.get(i) {
            Some(&b) => b,
            None => {
                return Err(VarintError::Truncated {
                    offset: i,
                    buffer_len: buf.len(),
                });
            },
        };
        if shift == 63 && byte > 1 {
            return Err(VarintError::Malformed { offset });
        }
        value |= ((byte & 0x7F) as u64) << shift;
        i += 1;

        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return Err(VarintError::Malformed { offset });
        }
    }

    Ok((value, i - offset))
}

#[cfg(test)]
#[allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Decodes a valid varint without requiring `Debug` on the error type
    /// (the `Debug` derive is deliberately absent so the production module
    /// extracts through Aeneas).
    fn decode_ok(buf: &[u8], offset: usize) -> (u64, usize) {
        match decode(buf, offset) {
            Ok(decoded) => decoded,
            Err(VarintError::Truncated { .. } | VarintError::Malformed { .. }) => {
                panic!("expected a valid varint")
            },
        }
    }

    /// Pre-seam reference decoder: the exact algorithm the production decoder
    /// replaced, used to prove the refactor is behaviour-preserving.
    fn reference_decode(buf: &[u8], offset: usize) -> Result<(u64, usize), (&'static str, usize)> {
        let mut value: u64 = 0;
        let mut shift = 0;
        let mut i = offset;
        loop {
            let Some(&byte) = buf.get(i) else {
                return Err(("truncated", i));
            };
            if shift == 63 && byte > 1 {
                return Err(("malformed", offset));
            }
            value |= ((byte & 0x7F) as u64) << shift;
            i += 1;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift >= 64 {
                return Err(("malformed", offset));
            }
        }
        Ok((value, i - offset))
    }

    fn production_decode(buf: &[u8], offset: usize) -> Result<(u64, usize), (&'static str, usize)> {
        decode(buf, offset).map_err(|error| match error {
            VarintError::Truncated { offset, .. } => ("truncated", offset),
            VarintError::Malformed { offset } => ("malformed", offset),
        })
    }

    #[test]
    fn decode_matches_the_pre_seam_reference_for_every_prefix_and_offset() {
        let corpus = [
            0_u64,
            1,
            127,
            128,
            16_383,
            16_384,
            (1_u64 << 32) - 1,
            1_u64 << 32,
            u64::MAX - 1,
            u64::MAX,
        ];
        for value in corpus {
            let mut encoded = [0_u8; 10];
            let written = encode(value, &mut encoded);
            assert_eq!(written, encoded_len(value));
            let bytes = &encoded[..written];
            for offset in 0..=bytes.len() {
                for len in offset..=bytes.len() {
                    let slice = &bytes[offset..len];
                    assert_eq!(
                        production_decode(slice, 0),
                        reference_decode(slice, 0),
                        "decode at offset 0 diverged for {slice:?}"
                    );
                    // Offsets into a buffer: compare at every legal offset.
                    for inner in 0..=slice.len() {
                        assert_eq!(
                            production_decode(slice, inner),
                            reference_decode(slice, inner),
                            "decode at offset {inner} diverged for {slice:?}"
                        );
                    }
                }
            }
            // Truncated prefixes are classified with the exact missing offset.
            for prefix in 0..written {
                assert_eq!(
                    production_decode(&bytes[..prefix], 0),
                    Err(("truncated", prefix)),
                    "prefix {prefix} of {value} must be truncated at its end"
                );
            }
        }
    }

    #[test]
    fn malformed_continuations_match_the_reference() {
        let cases: [&[u8]; 5] = [
            &[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80],
            &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02],
            &[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01],
            &[0x80],
            &[0x80, 0x80],
        ];
        for buf in cases {
            for offset in 0..buf.len() {
                assert_eq!(
                    production_decode(buf, offset),
                    reference_decode(buf, offset),
                    "malformed decode diverged at offset {offset} for {buf:?}"
                );
            }
        }
    }

    #[test]
    fn short_buffers_write_the_prefix_of_the_full_encoding() {
        for value in [0_u64, 1, 127, 128, 16_383, 16_384, u64::MAX] {
            let mut full = [0_u8; 10];
            let needed = encode(value, &mut full);
            for capacity in 0..=10 {
                let mut partial = [0_u8; 10];
                let written = encode(value, &mut partial[..capacity]);
                assert_eq!(
                    written,
                    needed.min(capacity),
                    "short-buffer write count diverged for {value} in {capacity} bytes"
                );
                assert_eq!(&partial[..written], &full[..written]);
            }
        }
    }

    #[test]
    fn full_u64_roundtrip_covers_every_boundary() {
        for value in [
            0_u64,
            1,
            127,
            128,
            16_383,
            16_384,
            u64::from(u32::MAX),
            u64::from(u32::MAX) + 1,
            u64::MAX / 2,
            u64::MAX - 1,
            u64::MAX,
        ] {
            let mut buf = [0_u8; 10];
            let written = encode(value, &mut buf);
            let (decoded, consumed) = decode_ok(&buf, 0);
            assert_eq!(decoded, value);
            assert_eq!(consumed, written);
        }
    }
}
