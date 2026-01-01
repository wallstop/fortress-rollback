//! Binary codec for network message serialization.
//!
//! This module provides a centralized, optimized interface for encoding and decoding
//! network messages using bincode. It encapsulates the bincode configuration to ensure
//! consistent, deterministic serialization across the codebase.
//!
//! # Design Rationale
//!
//! - **Centralized Configuration**: The bincode config is defined once, avoiding
//!   repeated `bincode::config::standard().with_fixed_int_encoding()` calls.
//! - **Buffer Reuse**: Provides `encode_into` variants that write into existing
//!   buffers, reducing allocations in hot paths.
//! - **Clear Error Handling**: All functions return `Result` types with descriptive
//!   error variants.
//! - **Type Safety**: Generic over serde types, but specialized for our `Message` type.
//!
//! # Examples
//!
//! ```
//! use fortress_rollback::network::codec::{encode, decode, encode_into};
//!
//! // Encode any serializable type
//! let data: u32 = 42;
//! let bytes = encode(&data).expect("encoding should succeed");
//!
//! // Decode from bytes
//! let (decoded, _bytes_read): (u32, _) = decode(&bytes).expect("decoding should succeed");
//! assert_eq!(data, decoded);
//!
//! // Encode into a pre-allocated buffer (zero allocation)
//! let mut buffer = [0u8; 256];
//! let len = encode_into(&data, &mut buffer).expect("encoding should succeed");
//! let encoded_slice = &buffer[..len];
//! ```

use serde::{de::DeserializeOwned, Serialize};
use std::fmt;

// The bincode configuration used throughout Fortress Rollback.
//
// We use `standard()` with `fixed_int_encoding()` for several reasons:
// - Fixed-size integers ensure deterministic message sizes (important for rollback)
// - Standard config is compatible with most platforms
// - No variable-length encoding overhead for small integers
//
// This is a zero-cost abstraction - the config is computed at compile time.
fn config() -> impl bincode::config::Config {
    bincode::config::standard().with_fixed_int_encoding()
}

/// Represents what operation was being performed when a codec error occurred.
///
/// This helps with debugging by indicating what we were trying to encode or decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CodecOperation {
    /// Encoding a network message.
    EncodeMessage,
    /// Decoding a network message.
    DecodeMessage,
    /// Encoding into a buffer.
    EncodeIntoBuffer,
    /// Appending to a buffer.
    AppendToBuffer,
    /// A generic encoding operation.
    Encode,
    /// A generic decoding operation.
    Decode,
}

impl fmt::Display for CodecOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EncodeMessage => write!(f, "encoding network message"),
            Self::DecodeMessage => write!(f, "decoding network message"),
            Self::EncodeIntoBuffer => write!(f, "encoding into buffer"),
            Self::AppendToBuffer => write!(f, "appending to buffer"),
            Self::Encode => write!(f, "encoding"),
            Self::Decode => write!(f, "decoding"),
        }
    }
}

/// Errors that can occur during encoding or decoding.
///
/// # Why String for Error Messages?
///
/// Unlike other error types in this crate that use structured enums for zero-allocation
/// error construction, `CodecError` stores error messages as `String`. This design choice
/// is intentional:
///
/// 1. **Bincode errors are opaque**: The underlying `bincode::error::EncodeError` and
///    `bincode::error::DecodeError` types don't expose structured information about
///    failure reasons. They only provide a `Display` implementation for human-readable
///    messages.
///
/// 2. **Error source preservation**: Converting bincode errors to strings preserves
///    the diagnostic information that would otherwise be lost. The bincode library
///    may report issues like "unexpected end of input", "invalid enum variant", or
///    "sequence too long" - all as formatted strings.
///
/// 3. **Not on the hot path**: Codec errors occur during message deserialization
///    failures, which are exceptional conditions (corrupted data, protocol mismatch).
///    These are not hot-path operations where zero-allocation matters.
///
/// 4. **Simpler API**: Since bincode doesn't provide a structured error API, creating
///    our own structured error types would require pattern-matching on error message
///    strings, which would be fragile and could break with bincode updates.
///
/// For hot-path error handling (like RLE decode errors in compression), we use
/// structured enums. See [`CompressionError`](crate::network::compression::CompressionError)
/// and [`RleDecodeReason`](crate::RleDecodeReason) for examples.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodecError {
    /// The encoding operation failed.
    EncodeError {
        /// The underlying bincode error message.
        ///
        /// This is a `String` because bincode errors are opaque - they don't expose
        /// structured failure reasons, only human-readable messages via `Display`.
        message: String,
        /// The operation that was being performed.
        operation: CodecOperation,
    },
    /// The decoding operation failed.
    DecodeError {
        /// The underlying bincode error message.
        ///
        /// This is a `String` because bincode errors are opaque - they don't expose
        /// structured failure reasons, only human-readable messages via `Display`.
        message: String,
        /// The operation that was being performed.
        operation: CodecOperation,
    },
    /// The provided buffer was too small for encoding.
    BufferTooSmall {
        /// The required buffer size (0 if unknown).
        required: usize,
        /// The actual buffer size provided.
        provided: usize,
    },
}

impl CodecError {
    /// Creates a new encode error with the given message and operation.
    pub fn encode(message: impl Into<String>, operation: CodecOperation) -> Self {
        Self::EncodeError {
            message: message.into(),
            operation,
        }
    }

    /// Creates a new decode error with the given message and operation.
    pub fn decode(message: impl Into<String>, operation: CodecOperation) -> Self {
        Self::DecodeError {
            message: message.into(),
            operation,
        }
    }
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EncodeError { message, operation } => {
                write!(f, "encoding failed while {operation}: {message}")
            },
            Self::DecodeError { message, operation } => {
                write!(f, "decoding failed while {operation}: {message}")
            },
            Self::BufferTooSmall { required, provided } => {
                if *required > 0 {
                    write!(
                        f,
                        "buffer too small: needed {required} bytes, but only {provided} provided"
                    )
                } else {
                    write!(f, "buffer too small: only {provided} bytes provided")
                }
            },
        }
    }
}

impl std::error::Error for CodecError {}

/// Result type for codec operations.
pub type CodecResult<T> = Result<T, CodecError>;

/// Encodes a value into a new `Vec<u8>`.
///
/// This is the simplest encoding function but allocates a new vector.
/// For hot paths where you have a reusable buffer, prefer [`encode_into`].
///
/// # Examples
///
/// ```
/// use fortress_rollback::network::codec::encode;
///
/// let data: u32 = 42;
/// let bytes = encode(&data).expect("encoding should succeed");
/// assert!(!bytes.is_empty());
/// ```
pub fn encode<T: Serialize>(value: &T) -> CodecResult<Vec<u8>> {
    bincode::serde::encode_to_vec(value, config())
        .map_err(|e| CodecError::encode(e.to_string(), CodecOperation::Encode))
}

/// Encodes a value into an existing byte slice.
///
/// Returns the number of bytes written. This is more efficient than [`encode`]
/// when you have a pre-allocated buffer, as it avoids allocation.
///
/// # Errors
///
/// Returns [`CodecError::BufferTooSmall`] if the buffer is not large enough.
///
/// # Examples
///
/// ```
/// use fortress_rollback::network::codec::encode_into;
///
/// let data: u32 = 42;
/// let mut buffer = [0u8; 64];
/// let len = encode_into(&data, &mut buffer).expect("encoding should succeed");
/// assert!(len > 0);
/// assert!(len <= buffer.len());
/// ```
pub fn encode_into<T: Serialize>(value: &T, buffer: &mut [u8]) -> CodecResult<usize> {
    bincode::serde::encode_into_slice(value, buffer, config()).map_err(|e| {
        // Check if this is a buffer-too-small error
        let msg = e.to_string();
        if msg.contains("UnexpectedEnd") || msg.contains("not enough") {
            CodecError::BufferTooSmall {
                required: 0, // bincode doesn't tell us the required size
                provided: buffer.len(),
            }
        } else {
            CodecError::encode(msg, CodecOperation::EncodeIntoBuffer)
        }
    })
}

/// Encodes a value by appending to an existing `Vec<u8>`.
///
/// This is useful when building up a message incrementally. The vector
/// will be extended as needed.
///
/// # Examples
///
/// ```
/// use fortress_rollback::network::codec::encode_append;
///
/// let mut buffer = Vec::new();
/// encode_append(&42u32, &mut buffer).expect("encoding should succeed");
/// encode_append(&"hello", &mut buffer).expect("encoding should succeed");
/// assert!(!buffer.is_empty());
/// ```
pub fn encode_append<T: Serialize>(value: &T, buffer: &mut Vec<u8>) -> CodecResult<usize> {
    let start_len = buffer.len();
    bincode::serde::encode_into_std_write(value, buffer, config())
        .map(|_| buffer.len() - start_len)
        .map_err(|e| CodecError::encode(e.to_string(), CodecOperation::AppendToBuffer))
}

/// Decodes a value from a byte slice.
///
/// Returns the decoded value and the number of bytes consumed.
///
/// # Examples
///
/// ```
/// use fortress_rollback::network::codec::{encode, decode};
///
/// let original: u32 = 42;
/// let bytes = encode(&original).expect("encoding should succeed");
/// let (decoded, bytes_read): (u32, _) = decode(&bytes).expect("decoding should succeed");
/// assert_eq!(original, decoded);
/// assert_eq!(bytes_read, bytes.len());
/// ```
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> CodecResult<(T, usize)> {
    bincode::serde::decode_from_slice(bytes, config())
        .map_err(|e| CodecError::decode(e.to_string(), CodecOperation::Decode))
}

/// Decodes a value from a byte slice, ignoring the bytes consumed.
///
/// This is a convenience function when you don't care about how many bytes were read.
///
/// # Examples
///
/// ```
/// use fortress_rollback::network::codec::{encode, decode_value};
///
/// let original: u32 = 42;
/// let bytes = encode(&original).expect("encoding should succeed");
/// let decoded: u32 = decode_value(&bytes).expect("decoding should succeed");
/// assert_eq!(original, decoded);
/// ```
pub fn decode_value<T: DeserializeOwned>(bytes: &[u8]) -> CodecResult<T> {
    decode(bytes).map(|(value, _)| value)
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::network::messages::{Message, MessageBody, MessageHeader, SyncRequest};

    #[test]
    fn test_encode_decode_roundtrip_primitive() {
        let original: u32 = 12345;
        let bytes = encode(&original).unwrap();
        let (decoded, len): (u32, _) = decode(&bytes).unwrap();
        assert_eq!(original, decoded);
        assert_eq!(len, bytes.len());
    }

    #[test]
    fn test_encode_decode_roundtrip_message() {
        let original = Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::SyncRequest(SyncRequest {
                random_request: 999,
            }),
        };
        let bytes = encode(&original).unwrap();
        let (decoded, _): (Message, _) = decode(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_into_buffer() {
        let value: u32 = 42;
        let mut buffer = [0u8; 64];
        let len = encode_into(&value, &mut buffer).unwrap();
        assert!(len > 0);
        assert!(len <= 64);

        // Verify we can decode from the same buffer
        let (decoded, _): (u32, _) = decode(&buffer[..len]).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_encode_into_buffer_too_small() {
        let value: u64 = 0x1234_5678_9ABC_DEF0;
        let mut buffer = [0u8; 1]; // Too small for a u64
        let result = encode_into(&value, &mut buffer);
        assert!(matches!(
            result,
            Err(CodecError::BufferTooSmall { .. }) | Err(CodecError::EncodeError { .. })
        ));
    }

    #[test]
    fn test_encode_append() {
        let mut buffer = Vec::new();
        let len1 = encode_append(&42u32, &mut buffer).unwrap();
        let len2 = encode_append(&"test", &mut buffer).unwrap();
        assert_eq!(buffer.len(), len1 + len2);
    }

    #[test]
    fn test_decode_value_convenience() {
        let original: u32 = 42;
        let bytes = encode(&original).unwrap();
        let decoded: u32 = decode_value(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_decode_invalid_data() {
        let invalid_bytes = [0xFF, 0xFF, 0xFF];
        let result: CodecResult<(u64, _)> = decode(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_codec_error_display() {
        let err = CodecError::EncodeError {
            message: "test error".to_string(),
            operation: CodecOperation::Encode,
        };
        assert!(err.to_string().contains("encoding failed"));
        assert!(err.to_string().contains("encoding"));

        let err = CodecError::DecodeError {
            message: "test error".to_string(),
            operation: CodecOperation::Decode,
        };
        assert!(err.to_string().contains("decoding failed"));
        assert!(err.to_string().contains("decoding"));

        let err = CodecError::BufferTooSmall {
            required: 100,
            provided: 10,
        };
        let msg = err.to_string();
        assert!(msg.contains("buffer too small"));
        assert!(msg.contains("100"));
        assert!(msg.contains("10"));
    }

    #[test]
    fn test_codec_operation_display() {
        assert!(format!("{}", CodecOperation::Encode).contains("encoding"));
        assert!(format!("{}", CodecOperation::Decode).contains("decoding"));
        assert!(format!("{}", CodecOperation::EncodeMessage).contains("network message"));
        assert!(format!("{}", CodecOperation::DecodeMessage).contains("network message"));
        assert!(format!("{}", CodecOperation::EncodeIntoBuffer).contains("buffer"));
        assert!(format!("{}", CodecOperation::AppendToBuffer).contains("buffer"));
    }

    #[test]
    fn test_codec_error_helper_methods() {
        let encode_err = CodecError::encode("test", CodecOperation::Encode);
        assert!(matches!(encode_err, CodecError::EncodeError { .. }));

        let decode_err = CodecError::decode("test", CodecOperation::Decode);
        assert!(matches!(decode_err, CodecError::DecodeError { .. }));
    }

    #[test]
    fn test_codec_error_equality() {
        let err1 = CodecError::encode("test", CodecOperation::Encode);
        let err2 = CodecError::encode("test", CodecOperation::Encode);
        let err3 = CodecError::encode("different", CodecOperation::Encode);
        let err4 = CodecError::encode("test", CodecOperation::EncodeMessage);

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
        assert_ne!(err1, err4);
    }

    #[test]
    fn test_codec_operation_is_copy() {
        let op = CodecOperation::Encode;
        let op2 = op;
        assert_eq!(op, op2);
    }

    #[test]
    fn test_encoding_is_deterministic() {
        let msg = Message {
            header: MessageHeader { magic: 0x1234 },
            body: MessageBody::KeepAlive,
        };
        let bytes1 = encode(&msg).unwrap();
        let bytes2 = encode(&msg).unwrap();
        assert_eq!(
            bytes1, bytes2,
            "Encoding must be deterministic for rollback networking"
        );
    }

    #[test]
    fn test_encode_into_message() {
        let msg = Message {
            header: MessageHeader { magic: 0x1234 },
            body: MessageBody::KeepAlive,
        };
        let mut buffer = [0u8; 256];
        let len = encode_into(&msg, &mut buffer).unwrap();

        let (decoded, _): (Message, _) = decode(&buffer[..len]).unwrap();
        assert_eq!(msg, decoded);
    }
}
