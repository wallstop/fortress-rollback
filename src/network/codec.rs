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
use std::io::{self, Write};

use crate::network::messages::{
    ChecksumReport, ConnectionStatus, Input, InputAck, Message, MessageBody, MessageHeader,
    QualityReply, QualityReport, SyncReply, SyncRequest,
};
use crate::Frame;

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

struct FallibleVecWriter<'a> {
    buffer: &'a mut Vec<u8>,
}

impl Write for FallibleVecWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer
            .try_reserve(buf.len())
            .map_err(|_err| io::Error::other("failed to reserve output buffer"))?;
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct CountingWriter {
    len: usize,
}

impl Write for CountingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.len = self
            .len
            .checked_add(buf.len())
            .ok_or_else(|| io::Error::other("encoded length overflow"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Represents what operation was being performed when a codec error occurred.
///
/// This helps with debugging by indicating what we were trying to encode or decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
/// structured enums. See [`CompressionError`]
/// and [`RleDecodeReason`] for examples.
///
/// [`CompressionError`]: crate::network::compression::CompressionError
/// [`RleDecodeReason`]: crate::RleDecodeReason
#[derive(Debug, Clone, PartialEq, Eq)]
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

fn decode_message_error(message: impl Into<String>) -> CodecError {
    CodecError::decode(message, CodecOperation::DecodeMessage)
}

fn read_array<const N: usize>(
    bytes: &[u8],
    cursor: &mut usize,
    field: &'static str,
) -> CodecResult<[u8; N]> {
    let end = cursor
        .checked_add(N)
        .ok_or_else(|| decode_message_error(format!("{} offset overflow", field)))?;
    let slice = bytes
        .get(*cursor..end)
        .ok_or_else(|| decode_message_error(format!("truncated {}", field)))?;
    let mut out = [0_u8; N];
    out.copy_from_slice(slice);
    *cursor = end;
    Ok(out)
}

fn read_u16(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<u16> {
    Ok(u16::from_le_bytes(read_array(bytes, cursor, field)?))
}

fn read_u32(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<u32> {
    Ok(u32::from_le_bytes(read_array(bytes, cursor, field)?))
}

fn read_i16(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<i16> {
    Ok(i16::from_le_bytes(read_array(bytes, cursor, field)?))
}

fn read_i32(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<i32> {
    Ok(i32::from_le_bytes(read_array(bytes, cursor, field)?))
}

fn read_u128(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<u128> {
    Ok(u128::from_le_bytes(read_array(bytes, cursor, field)?))
}

fn read_bool(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<bool> {
    let value = read_array::<1>(bytes, cursor, field)?[0];
    match value {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(decode_message_error(format!(
            "invalid boolean value {} for {}",
            other, field
        ))),
    }
}

fn read_usize(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<usize> {
    let value = u64::from_le_bytes(read_array(bytes, cursor, field)?);
    usize::try_from(value)
        .map_err(|_err| decode_message_error(format!("{} length exceeds usize", field)))
}

fn decode_connection_status(bytes: &[u8], cursor: &mut usize) -> CodecResult<ConnectionStatus> {
    Ok(ConnectionStatus {
        disconnected: read_bool(bytes, cursor, "connection_status.disconnected")?,
        last_frame: Frame::new(read_i32(bytes, cursor, "connection_status.last_frame")?),
    })
}

fn decode_input(bytes: &[u8], cursor: &mut usize) -> CodecResult<Input> {
    let status_len = read_usize(bytes, cursor, "input.peer_connect_status.len")?;
    let status_bytes = status_len
        .checked_mul(5)
        .ok_or_else(|| decode_message_error("input.peer_connect_status byte length overflow"))?;
    let remaining = bytes.len().saturating_sub(*cursor);
    if status_bytes > remaining {
        return Err(decode_message_error(format!(
            "input.peer_connect_status length {} exceeds remaining packet bytes {}",
            status_len, remaining
        )));
    }

    let mut peer_connect_status = Vec::new();
    peer_connect_status
        .try_reserve_exact(status_len)
        .map_err(|_err| {
            decode_message_error(format!(
                "failed to reserve {} connection status entries",
                status_len
            ))
        })?;
    for _ in 0..status_len {
        peer_connect_status.push(decode_connection_status(bytes, cursor)?);
    }

    let disconnect_requested = read_bool(bytes, cursor, "input.disconnect_requested")?;
    let start_frame = Frame::new(read_i32(bytes, cursor, "input.start_frame")?);
    let ack_frame = Frame::new(read_i32(bytes, cursor, "input.ack_frame")?);

    let byte_len = read_usize(bytes, cursor, "input.bytes.len")?;
    let remaining = bytes.len().saturating_sub(*cursor);
    if byte_len > remaining {
        return Err(decode_message_error(format!(
            "input.bytes length {} exceeds remaining packet bytes {}",
            byte_len, remaining
        )));
    }
    let byte_slice = bytes
        .get(*cursor..*cursor + byte_len)
        .ok_or_else(|| decode_message_error("input.bytes slice out of bounds"))?;
    let mut input_bytes = Vec::new();
    input_bytes.try_reserve_exact(byte_len).map_err(|_err| {
        decode_message_error(format!("failed to reserve {} input bytes", byte_len))
    })?;
    input_bytes.extend_from_slice(byte_slice);
    *cursor += byte_len;

    Ok(Input {
        peer_connect_status,
        disconnect_requested,
        start_frame,
        ack_frame,
        bytes: input_bytes,
    })
}

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
/// let bytes = encode(&data)?;
/// assert!(!bytes.is_empty());
/// # Ok::<(), fortress_rollback::network::codec::CodecError>(())
/// ```
pub fn encode<T: Serialize>(value: &T) -> CodecResult<Vec<u8>> {
    let len = encoded_len(value)?;
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(len).map_err(|_err| {
        CodecError::encode("failed to reserve output buffer", CodecOperation::Encode)
    })?;
    encode_append(value, &mut buffer)?;
    Ok(buffer)
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
/// let len = encode_into(&data, &mut buffer)?;
/// assert!(len > 0);
/// assert!(len <= buffer.len());
/// # Ok::<(), fortress_rollback::network::codec::CodecError>(())
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
/// encode_append(&42u32, &mut buffer)?;
/// encode_append(&"hello", &mut buffer)?;
/// assert!(!buffer.is_empty());
/// # Ok::<(), fortress_rollback::network::codec::CodecError>(())
/// ```
pub fn encode_append<T: Serialize>(value: &T, buffer: &mut Vec<u8>) -> CodecResult<usize> {
    let start_len = buffer.len();
    let mut writer = FallibleVecWriter { buffer };
    bincode::serde::encode_into_std_write(value, &mut writer, config())
        .map(|_| buffer.len() - start_len)
        .map_err(|e| CodecError::encode(e.to_string(), CodecOperation::AppendToBuffer))
}

/// Computes the encoded length without allocating an output buffer.
pub(crate) fn encoded_len<T: Serialize>(value: &T) -> CodecResult<usize> {
    let mut writer = CountingWriter { len: 0 };
    bincode::serde::encode_into_std_write(value, &mut writer, config())
        .map(|_| writer.len)
        .map_err(|e| CodecError::encode(e.to_string(), CodecOperation::Encode))
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
/// let bytes = encode(&original)?;
/// let (decoded, bytes_read): (u32, _) = decode(&bytes)?;
/// assert_eq!(original, decoded);
/// assert_eq!(bytes_read, bytes.len());
/// # Ok::<(), fortress_rollback::network::codec::CodecError>(())
/// ```
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> CodecResult<(T, usize)> {
    bincode::serde::decode_from_slice(bytes, config())
        .map_err(|e| CodecError::decode(e.to_string(), CodecOperation::Decode))
}

/// Decodes a network [`Message`] without allocating from untrusted length prefixes.
///
/// This mirrors the crate's bincode configuration for the fixed network message
/// schema, but checks every variable-length field against the remaining packet
/// bytes before reserving memory.
pub(crate) fn decode_message(bytes: &[u8]) -> CodecResult<(Message, usize)> {
    let mut cursor = 0;
    let header = MessageHeader {
        magic: read_u16(bytes, &mut cursor, "message.header.magic")?,
    };
    let variant = read_u32(bytes, &mut cursor, "message.body.variant")?;
    let body = match variant {
        0 => MessageBody::SyncRequest(SyncRequest {
            random_request: read_u32(bytes, &mut cursor, "sync_request.random_request")?,
        }),
        1 => MessageBody::SyncReply(SyncReply {
            random_reply: read_u32(bytes, &mut cursor, "sync_reply.random_reply")?,
        }),
        2 => MessageBody::Input(decode_input(bytes, &mut cursor)?),
        3 => MessageBody::InputAck(InputAck {
            ack_frame: Frame::new(read_i32(bytes, &mut cursor, "input_ack.ack_frame")?),
        }),
        4 => MessageBody::QualityReport(QualityReport {
            frame_advantage: read_i16(bytes, &mut cursor, "quality_report.frame_advantage")?,
            ping: read_u128(bytes, &mut cursor, "quality_report.ping")?,
        }),
        5 => MessageBody::QualityReply(QualityReply {
            pong: read_u128(bytes, &mut cursor, "quality_reply.pong")?,
        }),
        6 => MessageBody::ChecksumReport(ChecksumReport {
            checksum: read_u128(bytes, &mut cursor, "checksum_report.checksum")?,
            frame: Frame::new(read_i32(bytes, &mut cursor, "checksum_report.frame")?),
        }),
        7 => MessageBody::KeepAlive,
        other => {
            return Err(decode_message_error(format!(
                "unknown message body variant {}",
                other
            )))
        },
    };

    if cursor != bytes.len() {
        return Err(decode_message_error(format!(
            "message has {} trailing byte(s)",
            bytes.len() - cursor
        )));
    }

    Ok((Message { header, body }, cursor))
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
/// let bytes = encode(&original)?;
/// let decoded: u32 = decode_value(&bytes)?;
/// assert_eq!(original, decoded);
/// # Ok::<(), fortress_rollback::network::codec::CodecError>(())
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
    use crate::network::messages::{
        ChecksumReport, ConnectionStatus, Input, InputAck, Message, MessageBody, MessageHeader,
        QualityReply, QualityReport, SyncReply, SyncRequest,
    };

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
    fn decode_message_roundtrips_every_body_variant() {
        let messages = [
            Message {
                header: MessageHeader { magic: 0xABCD },
                body: MessageBody::SyncRequest(SyncRequest {
                    random_request: 999,
                }),
            },
            Message {
                header: MessageHeader { magic: 0xABCD },
                body: MessageBody::SyncReply(SyncReply { random_reply: 123 }),
            },
            Message {
                header: MessageHeader { magic: 0xABCD },
                body: MessageBody::Input(Input {
                    peer_connect_status: vec![
                        ConnectionStatus {
                            disconnected: false,
                            last_frame: Frame::new(10),
                        },
                        ConnectionStatus {
                            disconnected: true,
                            last_frame: Frame::new(20),
                        },
                    ],
                    disconnect_requested: false,
                    start_frame: Frame::new(100),
                    ack_frame: Frame::new(50),
                    bytes: vec![1, 2, 3, 4, 5],
                }),
            },
            Message {
                header: MessageHeader { magic: 0xABCD },
                body: MessageBody::InputAck(InputAck {
                    ack_frame: Frame::new(77),
                }),
            },
            Message {
                header: MessageHeader { magic: 0xABCD },
                body: MessageBody::QualityReport(QualityReport {
                    frame_advantage: -2,
                    ping: 1_000,
                }),
            },
            Message {
                header: MessageHeader { magic: 0xABCD },
                body: MessageBody::QualityReply(QualityReply { pong: 2_000 }),
            },
            Message {
                header: MessageHeader { magic: 0xABCD },
                body: MessageBody::ChecksumReport(ChecksumReport {
                    checksum: 0xDEAD_BEEF,
                    frame: Frame::new(88),
                }),
            },
            Message {
                header: MessageHeader { magic: 0xABCD },
                body: MessageBody::KeepAlive,
            },
        ];

        for original in messages {
            let bytes = encode(&original).unwrap();
            let generic: Message = decode_value(&bytes).unwrap();
            let (manual, consumed) = decode_message(&bytes).unwrap();

            assert_eq!(generic, original);
            assert_eq!(manual, original);
            assert_eq!(consumed, bytes.len());
        }
    }

    #[test]
    fn decode_message_roundtrips_input_without_generic_vec_decode() {
        let original = Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::Input(Input {
                peer_connect_status: vec![
                    ConnectionStatus {
                        disconnected: false,
                        last_frame: Frame::new(10),
                    },
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: Frame::new(20),
                    },
                ],
                disconnect_requested: false,
                start_frame: Frame::new(100),
                ack_frame: Frame::new(50),
                bytes: vec![1, 2, 3, 4, 5],
            }),
        };
        let bytes = encode(&original).unwrap();

        let (decoded, consumed) = decode_message(&bytes).unwrap();

        assert_eq!(decoded, original);
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn decode_message_rejects_vec_length_that_exceeds_packet_bytes() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes());
        bytes.extend_from_slice(&2_u32.to_le_bytes()); // MessageBody::Input
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // peer_connect_status len

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    #[test]
    fn decode_message_rejects_input_bytes_length_that_exceeds_packet_bytes() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes());
        bytes.extend_from_slice(&2_u32.to_le_bytes()); // MessageBody::Input
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // peer_connect_status len
        bytes.push(0); // disconnect_requested
        bytes.extend_from_slice(&100_i32.to_le_bytes()); // start_frame
        bytes.extend_from_slice(&50_i32.to_le_bytes()); // ack_frame
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // input.bytes len

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    #[test]
    fn decode_message_rejects_trailing_bytes() {
        let original = Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::KeepAlive,
        };
        let mut bytes = encode(&original).unwrap();
        bytes.push(0);

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
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
