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
//! - **Type Safety**: Generic over serde types, with
//!   [`decode_message()`](crate::network::codec::decode_message) for bounded
//!   decoding of peer-controlled [`Message`](crate::Message) bytes.
//!
//! # Examples
//!
//! ```
//! use fortress_rollback::network::codec::{encode, decode, encode_into};
//!
//! // Encode any serializable type
//! let data: u32 = 42;
//! let bytes = encode(&data)?;
//!
//! // Decode from bytes
//! let (decoded, _bytes_read): (u32, _) = decode(&bytes)?;
//! assert_eq!(data, decoded);
//!
//! // Encode into a pre-allocated buffer (zero allocation)
//! let mut buffer = [0u8; 256];
//! let len = encode_into(&data, &mut buffer)?;
//! assert!(len <= buffer.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use serde::{de::DeserializeOwned, Serialize};
use std::fmt;
use std::io::{self, Write};

use crate::network::messages::{
    ChecksumReport, ConnectionStatus, Input, InputAck, Message, MessageBody, MessageHeader,
    QualityReply, QualityReport, SyncReply, SyncRequest,
};
#[cfg(feature = "hot-join")]
use crate::network::messages::{
    JoinAborted, JoinCommitted, JoinRequest, ReactivateSlot, ReactivateSlotAck, StateSnapshot,
    StateSnapshotAck,
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

fn take_bytes<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    len: usize,
    field: &'static str,
) -> CodecResult<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| decode_message_error(format!("{} offset overflow", field)))?;
    let slice = bytes
        .get(*cursor..end)
        .ok_or_else(|| decode_message_error(format!("truncated {}", field)))?;
    *cursor = end;
    Ok(slice)
}

fn read_array<const N: usize>(
    bytes: &[u8],
    cursor: &mut usize,
    field: &'static str,
) -> CodecResult<[u8; N]> {
    let slice = take_bytes(bytes, cursor, N, field)?;
    let mut out = [0_u8; N];
    out.copy_from_slice(slice);
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

/// The fixed wire footprint, in bytes, of one encoded [`ConnectionStatus`].
//
// 1-byte bool + 4-byte fixed-int i32.
const CONNECTION_STATUS_WIRE_LEN: usize = 5;

/// Rejects a length prefix that cannot possibly fit in the unread input bytes,
/// *before* any memory is reserved for it.
///
/// Decoders read an element count from an untrusted `&[u8]`, then reserve a
/// `Vec` sized by that count. Even fallible reservation (`try_reserve_exact`)
/// only prevents an allocator *abort*; it can still *succeed* at a huge
/// speculative allocation when the count is attacker-chosen, which is the
/// memory-exhaustion DoS of the RUSTSEC-2022-0035 class. Calling this first
/// makes that impossible: a count whose minimum wire footprint exceeds the
/// remaining bytes is rejected outright.
///
/// `min_encoded_len` is the smallest wire footprint of a single element -- a
/// length prefix, an option/tag byte, or a fixed-size record -- so
/// `len * min_encoded_len` is a lower bound on the bytes the count *claims* to
/// describe. The multiplication is `checked_mul`, so the bound itself is
/// overflow-safe and a pathological count is rejected rather than wrapping.
pub(crate) fn ensure_length_within_remaining(
    bytes: &[u8],
    cursor: usize,
    len: usize,
    min_encoded_len: usize,
    field: &'static str,
) -> CodecResult<()> {
    let remaining = bytes.len().saturating_sub(cursor);
    let min_bytes = len.checked_mul(min_encoded_len).ok_or_else(|| {
        decode_message_error(format!("{field} length {len} overflows the byte bound"))
    })?;
    if min_bytes > remaining {
        return Err(decode_message_error(format!(
            "{field} length {len} exceeds the {remaining} remaining byte(s)"
        )));
    }
    Ok(())
}

fn decode_input(bytes: &[u8], cursor: &mut usize) -> CodecResult<Input> {
    let status_len = read_usize(bytes, cursor, "input.peer_connect_status.len")?;
    ensure_length_within_remaining(
        bytes,
        *cursor,
        status_len,
        CONNECTION_STATUS_WIRE_LEN,
        "input.peer_connect_status",
    )?;

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
    let byte_slice = take_bytes(bytes, cursor, byte_len, "input.bytes")?;
    let mut input_bytes = Vec::new();
    input_bytes.try_reserve_exact(byte_len).map_err(|_err| {
        decode_message_error(format!("failed to reserve {} input bytes", byte_len))
    })?;
    input_bytes.extend_from_slice(byte_slice);

    Ok(Input {
        peer_connect_status,
        disconnect_requested,
        start_frame,
        ack_frame,
        bytes: input_bytes,
    })
}

/// Reads a bincode `Option<u128>` encoded under fixed-int config: a one-byte
/// tag (0 = `None`, 1 = `Some`) followed by a 16-byte little-endian `u128` when
/// the tag is 1.
#[cfg(feature = "hot-join")]
fn read_option_u128(
    bytes: &[u8],
    cursor: &mut usize,
    field: &'static str,
) -> CodecResult<Option<u128>> {
    let tag = read_array::<1>(bytes, cursor, field)?[0];
    match tag {
        0 => Ok(None),
        1 => Ok(Some(read_u128(bytes, cursor, field)?)),
        other => Err(decode_message_error(format!(
            "invalid option tag {} for {}",
            other, field
        ))),
    }
}

#[cfg(feature = "hot-join")]
fn decode_state_snapshot(bytes: &[u8], cursor: &mut usize) -> CodecResult<StateSnapshot> {
    let frame = Frame::new(read_i32(bytes, cursor, "state_snapshot.frame")?);
    let num_players = read_usize(bytes, cursor, "state_snapshot.num_players")?;

    let state_len = read_usize(bytes, cursor, "state_snapshot.state_bytes.len")?;
    // alloc-bound: `state_len` is a peer-controlled length prefix. Before reserving,
    // it is validated against the bytes still remaining in this packet
    // (`ensure_length_within_remaining`, min element footprint 1 byte/`u8`), so a
    // count larger than the buffer can describe is rejected outright. Only then do we
    // `try_reserve_exact` and copy exactly `state_len` bytes via `take_bytes`, which
    // bounds-checks the slice. This is the same pattern as `decode_input`'s `bytes`.
    ensure_length_within_remaining(bytes, *cursor, state_len, 1, "state_snapshot.state_bytes")?;
    let state_slice = take_bytes(bytes, cursor, state_len, "state_snapshot.state_bytes")?;
    let mut state_bytes = Vec::new();
    state_bytes.try_reserve_exact(state_len).map_err(|_err| {
        decode_message_error(format!("failed to reserve {} state bytes", state_len))
    })?;
    state_bytes.extend_from_slice(state_slice);

    let bridge_len = read_usize(bytes, cursor, "state_snapshot.bridge_inputs.len")?;
    // alloc-bound: `bridge_len` is a peer-controlled length prefix, validated
    // against the bytes still remaining in this packet before reserving (same
    // pattern as `state_bytes` above). The session layer additionally bounds
    // the field semantically when consuming it: the blob must decode to exactly
    // the receiver's already-validated `num_players` fixed-width inputs.
    ensure_length_within_remaining(
        bytes,
        *cursor,
        bridge_len,
        1,
        "state_snapshot.bridge_inputs",
    )?;
    let bridge_slice = take_bytes(bytes, cursor, bridge_len, "state_snapshot.bridge_inputs")?;
    let mut bridge_inputs = Vec::new();
    bridge_inputs
        .try_reserve_exact(bridge_len)
        .map_err(|_err| {
            decode_message_error(format!(
                "failed to reserve {} bridge input bytes",
                bridge_len
            ))
        })?;
    bridge_inputs.extend_from_slice(bridge_slice);

    let statuses_len = read_usize(bytes, cursor, "state_snapshot.bridge_statuses.len")?;
    // alloc-bound: `statuses_len` is a peer-controlled length prefix, validated
    // against the bytes still remaining in this packet before reserving (the
    // exact `decode_input` peer_connect_status pattern, min element footprint
    // CONNECTION_STATUS_WIRE_LEN). The session layer additionally bounds the
    // field semantically when consuming it: it must hold exactly the
    // receiver's already-validated `num_players` entries.
    ensure_length_within_remaining(
        bytes,
        *cursor,
        statuses_len,
        CONNECTION_STATUS_WIRE_LEN,
        "state_snapshot.bridge_statuses",
    )?;
    let mut bridge_statuses = Vec::new();
    bridge_statuses
        .try_reserve_exact(statuses_len)
        .map_err(|_err| {
            decode_message_error(format!(
                "failed to reserve {} bridge status entries",
                statuses_len
            ))
        })?;
    for _ in 0..statuses_len {
        bridge_statuses.push(decode_connection_status(bytes, cursor)?);
    }

    let checksum = read_option_u128(bytes, cursor, "state_snapshot.checksum")?;

    Ok(StateSnapshot {
        frame,
        num_players,
        state_bytes,
        bridge_inputs,
        bridge_statuses,
        checksum,
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
///
/// Custom [`NonBlockingSocket`](crate::NonBlockingSocket) implementations should
/// use this for received peer bytes instead of generic bincode decoding. Generic
/// serde decoding cannot validate the `Message` schema's length-prefixed fields
/// before allocating for them.
///
/// # Errors
///
/// Returns [`CodecError::DecodeError`] when the message is truncated, contains an
/// invalid variant or boolean, has trailing bytes, or declares a length that
/// cannot fit in the remaining packet.
pub fn decode_message(bytes: &[u8]) -> CodecResult<(Message, usize)> {
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
        #[cfg(feature = "hot-join")]
        8 => MessageBody::JoinRequest(JoinRequest {
            player_handle: read_usize(bytes, &mut cursor, "join_request.player_handle")?,
        }),
        #[cfg(feature = "hot-join")]
        9 => MessageBody::StateSnapshot(decode_state_snapshot(bytes, &mut cursor)?),
        #[cfg(feature = "hot-join")]
        10 => MessageBody::StateSnapshotAck(StateSnapshotAck {
            frame: Frame::new(read_i32(bytes, &mut cursor, "state_snapshot_ack.frame")?),
        }),
        #[cfg(feature = "hot-join")]
        11 => MessageBody::ReactivateSlot(ReactivateSlot {
            handle: read_usize(bytes, &mut cursor, "reactivate_slot.handle")?,
            frame: Frame::new(read_i32(bytes, &mut cursor, "reactivate_slot.frame")?),
        }),
        #[cfg(feature = "hot-join")]
        12 => MessageBody::ReactivateSlotAck(ReactivateSlotAck {
            handle: read_usize(bytes, &mut cursor, "reactivate_slot_ack.handle")?,
            frame: Frame::new(read_i32(bytes, &mut cursor, "reactivate_slot_ack.frame")?),
        }),
        #[cfg(feature = "hot-join")]
        13 => MessageBody::JoinCommitted(JoinCommitted {
            handle: read_usize(bytes, &mut cursor, "join_committed.handle")?,
            frame: Frame::new(read_i32(bytes, &mut cursor, "join_committed.frame")?),
        }),
        #[cfg(feature = "hot-join")]
        14 => MessageBody::JoinAborted(JoinAborted {
            handle: read_usize(bytes, &mut cursor, "join_aborted.handle")?,
            frame: Frame::new(read_i32(bytes, &mut cursor, "join_aborted.frame")?),
        }),
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

/// Compile-time byte cap applied by [`decode_bounded`] to every container a
/// decoded value declares.
///
/// bincode's container decoders (`Vec`, byte buffers, etc.) only validate a
/// declared element/byte count against the input when the bincode config carries
/// a `Limit` — with the default no-limit config a `Vec<u8>`/`serde_bytes` field
/// whose length prefix claims `u64::MAX` is allocated as `vec![0u8; u64::MAX]`
/// *before* any data is read (an allocator-abort / OOM DoS, the
/// RUSTSEC-2022-0035 class). Decoding peer-controlled bytes into a user
/// `Config::State` must therefore be bounded.
///
/// This mirrors [`crate::rle::DEFAULT_MAX_DECODED_LEN`] (64 MiB): a single
/// rollback state snapshot far below it, far above any plausible decode buffer.
#[cfg(feature = "hot-join")]
pub(crate) const MAX_BOUNDED_DECODE_LEN: usize = crate::rle::DEFAULT_MAX_DECODED_LEN;

/// Decodes a value from a byte slice with a fixed per-decode byte limit, so a
/// corrupt or malicious length prefix cannot trigger an oversized allocation.
///
/// Identical to [`decode_value`] except the bincode config carries a
/// [`Limit`](bincode::config::Configuration::with_limit) of
/// [`MAX_BOUNDED_DECODE_LEN`] bytes. Under that config every container decoder
/// claims `len * size_of::<element>()` bytes against the running total *before*
/// allocating and fails with a decode error once the total would exceed the
/// limit — so even a `Vec<u8>`/`serde_bytes` field claiming `u64::MAX` is
/// rejected without allocating. `bytes` is additionally pre-rejected when it is
/// itself longer than the cap, so no input this function accepts can drive an
/// allocation past [`MAX_BOUNDED_DECODE_LEN`].
///
/// Use this (not [`decode_value`]) for any value reconstructed from
/// peer-controlled bytes whose type can contain a length-prefixed container.
///
/// # Bounds *allocation*, not *recursion depth*
///
/// The limit caps total bytes allocated; it does **not** cap the decode's call
/// stack. bincode decodes a recursive type (one transitively containing
/// `Box<Self>`, `Vec<Self>`, etc.) by recursing once per level of nesting, and a
/// deeply-nested value can be encoded in far fewer bytes than
/// [`MAX_BOUNDED_DECODE_LEN`] (each nesting level adds only a tag/length byte or
/// two). Such input therefore stays under the byte cap yet can still overflow
/// the stack mid-decode — an uncatchable abort, not a recoverable `Err`. This
/// limit cannot prevent that; callers must keep peer-decoded types non-recursive
/// / shallow. (Thread-based stack bounding is intentionally avoided: it would
/// require `T: Send`, which `T` here is not guaranteed to be.)
///
/// # Errors
///
/// Returns [`CodecError::DecodeError`] when `bytes` exceeds the cap, when a
/// declared container length would exceed the cap, or when bincode otherwise
/// fails to decode (truncated input, trailing bytes are *not* rejected here —
/// use [`decode`] if you need the consumed length).
#[cfg(feature = "hot-join")]
pub(crate) fn decode_bounded<T: DeserializeOwned>(bytes: &[u8]) -> CodecResult<T> {
    decode_bounded_with_consumed(bytes).map(|(value, _consumed)| value)
}

/// [`decode_bounded`] variant that also returns the number of bytes consumed,
/// for callers that decode several bounded values back-to-back from one slice
/// (e.g. the hot-join bridge-input blob: `num_players` fixed-width inputs
/// concatenated) and must verify exact consumption.
#[cfg(feature = "hot-join")]
pub(crate) fn decode_bounded_with_consumed<T: DeserializeOwned>(
    bytes: &[u8],
) -> CodecResult<(T, usize)> {
    // alloc-bound: this decode is bounded to MAX_BOUNDED_DECODE_LEN (64 MiB) two
    // ways. (1) `bytes` is pre-rejected when longer than the cap. (2) The bincode
    // `Limit<MAX_BOUNDED_DECODE_LEN>` config makes every container decoder claim
    // its `len * size_of::<element>()` byte footprint against a running total
    // *before* allocating and error once it would exceed the cap, so a malicious
    // length prefix (e.g. a `Vec<u8>` claiming u64::MAX) yields a decode error,
    // never a giant `vec![0u8; len]`. No input this function accepts can drive an
    // allocation past the cap. Empirically verified for both the per-element
    // (`Vec<T>`) and native (`serde_bytes`) decode paths.
    if bytes.len() > MAX_BOUNDED_DECODE_LEN {
        return Err(CodecError::decode(
            format!(
                "input length {} exceeds bounded-decode cap {}",
                bytes.len(),
                MAX_BOUNDED_DECODE_LEN
            ),
            CodecOperation::Decode,
        ));
    }
    // `with_limit` is an inherent method on the concrete `Configuration`, so the
    // limited config is built from the same `standard().with_fixed_int_encoding()`
    // base as `config()` (kept byte-for-byte identical to the shared codec config,
    // only adding the limit) rather than from the `impl Config` return of
    // `config()`.
    let config = bincode::config::standard()
        .with_fixed_int_encoding()
        .with_limit::<MAX_BOUNDED_DECODE_LEN>();
    bincode::serde::decode_from_slice(bytes, config)
        .map_err(|e| CodecError::decode(e.to_string(), CodecOperation::Decode))
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
    fn take_bytes_rejects_offset_overflow_and_preserves_cursor() {
        let bytes = [0_u8];
        let mut cursor = usize::MAX;

        let result = take_bytes(&bytes, &mut cursor, 1, "overflowing.field");

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
        assert_eq!(cursor, usize::MAX);
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

#[cfg(all(test, feature = "hot-join"))]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod hot_join_tests {
    use super::*;
    use crate::network::messages::{
        JoinAborted, JoinCommitted, JoinRequest, Message, MessageBody, MessageHeader,
        ReactivateSlot, ReactivateSlotAck, StateSnapshot, StateSnapshotAck,
    };

    fn roundtrip(original: Message) {
        let bytes = encode(&original).unwrap();
        // The generic bincode decode is the authority for the wire format; the manual
        // bounded decoder must agree with it byte-for-byte.
        let generic: Message = decode_value(&bytes).unwrap();
        let (manual, consumed) = decode_message(&bytes).unwrap();

        assert_eq!(generic, original, "generic bincode decode must roundtrip");
        assert_eq!(manual, original, "manual bounded decode must roundtrip");
        assert_eq!(
            consumed,
            bytes.len(),
            "manual decode must consume all bytes"
        );
    }

    #[test]
    fn decode_message_roundtrips_join_request() {
        roundtrip(Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::JoinRequest(JoinRequest { player_handle: 3 }),
        });
    }

    #[test]
    fn decode_message_roundtrips_state_snapshot_with_checksum() {
        roundtrip(Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::StateSnapshot(StateSnapshot {
                frame: Frame::new(42),
                num_players: 4,
                state_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02],
                bridge_inputs: Vec::new(),
                bridge_statuses: Vec::new(),
                checksum: Some(0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10),
            }),
        });
    }

    /// The N-peer serve shape: a snapshot carrying non-empty bridge inputs
    /// (the confirmed inputs at `S` for all slots) AND per-slot connection
    /// statuses must roundtrip through the manual bounded decoder
    /// byte-for-byte.
    #[test]
    fn decode_message_roundtrips_state_snapshot_with_bridge_inputs() {
        roundtrip(Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::StateSnapshot(StateSnapshot {
                frame: Frame::new(42),
                num_players: 3,
                state_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
                bridge_inputs: vec![0x11, 0x22, 0x33],
                bridge_statuses: vec![
                    ConnectionStatus {
                        disconnected: false,
                        last_frame: Frame::new(42),
                    },
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: Frame::new(17),
                    },
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: Frame::NULL,
                    },
                ],
                checksum: Some(0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10),
            }),
        });
    }

    #[test]
    fn decode_message_roundtrips_state_snapshot_without_checksum() {
        roundtrip(Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::StateSnapshot(StateSnapshot {
                frame: Frame::new(7),
                num_players: 2,
                state_bytes: vec![1, 2, 3, 4, 5],
                bridge_inputs: Vec::new(),
                bridge_statuses: Vec::new(),
                checksum: None,
            }),
        });
    }

    #[test]
    fn decode_message_roundtrips_state_snapshot_empty_state_bytes() {
        roundtrip(Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::StateSnapshot(StateSnapshot {
                frame: Frame::new(0),
                num_players: 1,
                state_bytes: Vec::new(),
                bridge_inputs: Vec::new(),
                bridge_statuses: Vec::new(),
                checksum: None,
            }),
        });
    }

    #[test]
    fn decode_message_roundtrips_state_snapshot_ack() {
        roundtrip(Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::StateSnapshotAck(StateSnapshotAck {
                frame: Frame::new(99),
            }),
        });
    }

    /// Hand-crafts a `StateSnapshot` wire buffer whose `state_bytes` length prefix
    /// claims `u64::MAX` while the buffer holds no payload. The bounded decoder must
    /// reject this via `ensure_length_within_remaining` *before* reserving, never
    /// panicking or attempting a giant allocation. Mirrors
    /// `decode_message_rejects_input_bytes_length_that_exceeds_packet_bytes`.
    #[test]
    fn decode_message_rejects_state_bytes_length_that_exceeds_packet_bytes() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&9_u32.to_le_bytes()); // MessageBody::StateSnapshot
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // state_bytes len (absurd)

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A `state_bytes` length that fits in `usize` but exceeds the remaining bytes
    /// (claims 100 bytes when only a few remain) must also be rejected before reserve.
    #[test]
    fn decode_message_rejects_state_bytes_length_larger_than_remaining() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&9_u32.to_le_bytes()); // MessageBody::StateSnapshot
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&100_u64.to_le_bytes()); // state_bytes len
        bytes.extend_from_slice(&[0xAA, 0xBB]); // only 2 bytes of payload present

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A `checksum` option tag other than 0/1 is invalid under bincode's encoding.
    #[test]
    fn decode_message_rejects_invalid_checksum_option_tag() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&9_u32.to_le_bytes()); // MessageBody::StateSnapshot
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&1_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // state_bytes len = 0
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // bridge_inputs len = 0
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // bridge_statuses len = 0
        bytes.push(2); // invalid option tag (only 0 or 1 valid)

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// Hand-crafts a `StateSnapshot` wire buffer whose `bridge_inputs` length
    /// prefix claims `u64::MAX` while the buffer holds no payload. The bounded
    /// decoder must reject this via `ensure_length_within_remaining` *before*
    /// reserving, never panicking or attempting a giant allocation. Mirrors
    /// `decode_message_rejects_state_bytes_length_that_exceeds_packet_bytes`.
    #[test]
    fn decode_message_rejects_bridge_inputs_length_that_exceeds_packet_bytes() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&9_u32.to_le_bytes()); // MessageBody::StateSnapshot
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // state_bytes len = 0
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // bridge_inputs len (absurd)

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A `bridge_inputs` length that fits in `usize` but exceeds the remaining
    /// bytes (claims 100 bytes when only a few remain) must also be rejected
    /// before reserve.
    #[test]
    fn decode_message_rejects_bridge_inputs_length_larger_than_remaining() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&9_u32.to_le_bytes()); // MessageBody::StateSnapshot
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // state_bytes len = 0
        bytes.extend_from_slice(&100_u64.to_le_bytes()); // bridge_inputs len
        bytes.extend_from_slice(&[0xAA, 0xBB]); // only 2 bytes of payload present

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A snapshot buffer truncated mid-`bridge_inputs` payload (the length
    /// prefix itself missing) must be rejected, never read out of bounds.
    #[test]
    fn decode_message_rejects_snapshot_truncated_before_bridge_inputs() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&9_u32.to_le_bytes()); // MessageBody::StateSnapshot
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // state_bytes len = 0
                                                       // bridge_inputs length prefix omitted entirely.

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// Hand-crafts a `StateSnapshot` wire buffer whose `bridge_statuses`
    /// length prefix claims `u64::MAX` while the buffer holds no payload. The
    /// bounded decoder must reject this via `ensure_length_within_remaining`
    /// (min element footprint `CONNECTION_STATUS_WIRE_LEN`) *before*
    /// reserving — mirrors the `bridge_inputs` battery.
    #[test]
    fn decode_message_rejects_bridge_statuses_length_that_exceeds_packet_bytes() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&9_u32.to_le_bytes()); // MessageBody::StateSnapshot
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // state_bytes len = 0
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // bridge_inputs len = 0
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // bridge_statuses len (absurd)

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A `bridge_statuses` length that fits in `usize` but whose minimum wire
    /// footprint (5 bytes per status) exceeds the remaining bytes must also
    /// be rejected before reserve.
    #[test]
    fn decode_message_rejects_bridge_statuses_length_larger_than_remaining() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&9_u32.to_le_bytes()); // MessageBody::StateSnapshot
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // state_bytes len = 0
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // bridge_inputs len = 0
        bytes.extend_from_slice(&100_u64.to_le_bytes()); // bridge_statuses len
        bytes.extend_from_slice(&[0xAA, 0xBB, 0xCC]); // only 3 payload bytes present

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A snapshot buffer truncated before the `bridge_statuses` length prefix
    /// must be rejected, never read out of bounds.
    #[test]
    fn decode_message_rejects_snapshot_truncated_before_bridge_statuses() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&9_u32.to_le_bytes()); // MessageBody::StateSnapshot
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // state_bytes len = 0
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // bridge_inputs len = 0
                                                       // bridge_statuses length prefix omitted entirely.

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A snapshot buffer truncated mid-`bridge_statuses` payload (the prefix
    /// claims one status, only part of its 5-byte record present) must be
    /// rejected, never read out of bounds.
    #[test]
    fn decode_message_rejects_snapshot_truncated_inside_bridge_statuses() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&9_u32.to_le_bytes()); // MessageBody::StateSnapshot
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // state_bytes len = 0
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // bridge_inputs len = 0
        bytes.extend_from_slice(&1_u64.to_le_bytes()); // bridge_statuses len = 1
        bytes.extend_from_slice(&[0, 0xAA, 0xBB]); // 3 of the 5 status bytes

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A valid snapshot buffer with extra trailing bytes must be rejected.
    #[test]
    fn decode_message_rejects_trailing_bytes_after_snapshot() {
        let original = Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::StateSnapshot(StateSnapshot {
                frame: Frame::new(5),
                num_players: 2,
                state_bytes: vec![7, 8, 9],
                bridge_inputs: vec![4, 5],
                bridge_statuses: Vec::new(),
                checksum: Some(123),
            }),
        };
        let mut bytes = encode(&original).unwrap();
        bytes.push(0); // trailing byte

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    #[test]
    fn decode_message_roundtrips_reactivate_slot() {
        roundtrip(Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::ReactivateSlot(ReactivateSlot {
                handle: 2,
                frame: Frame::new(42),
            }),
        });
    }

    #[test]
    fn decode_message_roundtrips_reactivate_slot_ack() {
        roundtrip(Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::ReactivateSlotAck(ReactivateSlotAck {
                handle: 2,
                frame: Frame::new(42),
            }),
        });
    }

    /// A `ReactivateSlot` buffer truncated mid-`frame` (the `handle` is present but
    /// the trailing `i32` is missing) must be rejected, never panicking or reading
    /// out of bounds.
    #[test]
    fn decode_message_rejects_truncated_reactivate_slot() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&11_u32.to_le_bytes()); // MessageBody::ReactivateSlot
        bytes.extend_from_slice(&3_u64.to_le_bytes()); // handle
                                                       // frame (i32) omitted entirely.

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A `ReactivateSlotAck` buffer truncated mid-`frame` must likewise be rejected.
    #[test]
    fn decode_message_rejects_truncated_reactivate_slot_ack() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&12_u32.to_le_bytes()); // MessageBody::ReactivateSlotAck
        bytes.extend_from_slice(&3_u64.to_le_bytes()); // handle
                                                       // frame (i32) omitted entirely.

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A valid `ReactivateSlot` buffer with an extra trailing byte must be rejected
    /// by the trailing-bytes check.
    #[test]
    fn decode_message_rejects_trailing_bytes_after_reactivate_slot() {
        let original = Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::ReactivateSlot(ReactivateSlot {
                handle: 1,
                frame: Frame::new(7),
            }),
        };
        let mut bytes = encode(&original).unwrap();
        bytes.push(0); // trailing byte

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A valid `ReactivateSlotAck` buffer with an extra trailing byte must be
    /// rejected by the trailing-bytes check.
    #[test]
    fn decode_message_rejects_trailing_bytes_after_reactivate_slot_ack() {
        let original = Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::ReactivateSlotAck(ReactivateSlotAck {
                handle: 1,
                frame: Frame::new(7),
            }),
        };
        let mut bytes = encode(&original).unwrap();
        bytes.push(0); // trailing byte

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    #[test]
    fn decode_message_roundtrips_join_committed() {
        roundtrip(Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::JoinCommitted(JoinCommitted {
                handle: 2,
                frame: Frame::new(42),
            }),
        });
    }

    #[test]
    fn decode_message_roundtrips_join_aborted() {
        roundtrip(Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::JoinAborted(JoinAborted {
                handle: 2,
                frame: Frame::new(42),
            }),
        });
    }

    /// A `JoinCommitted` buffer truncated mid-`frame` (the `handle` is present but
    /// the trailing `i32` is missing) must be rejected, never panicking or reading
    /// out of bounds. Mirrors `decode_message_rejects_truncated_reactivate_slot`.
    #[test]
    fn decode_message_rejects_truncated_join_committed() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&13_u32.to_le_bytes()); // MessageBody::JoinCommitted
        bytes.extend_from_slice(&3_u64.to_le_bytes()); // handle
                                                       // frame (i32) omitted entirely.

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A `JoinAborted` buffer truncated mid-`frame` must likewise be rejected.
    #[test]
    fn decode_message_rejects_truncated_join_aborted() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xABCD_u16.to_le_bytes()); // header.magic
        bytes.extend_from_slice(&14_u32.to_le_bytes()); // MessageBody::JoinAborted
        bytes.extend_from_slice(&3_u64.to_le_bytes()); // handle
                                                       // frame (i32) omitted entirely.

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A valid `JoinCommitted` buffer with an extra trailing byte must be rejected
    /// by the trailing-bytes check.
    #[test]
    fn decode_message_rejects_trailing_bytes_after_join_committed() {
        let original = Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::JoinCommitted(JoinCommitted {
                handle: 1,
                frame: Frame::new(7),
            }),
        };
        let mut bytes = encode(&original).unwrap();
        bytes.push(0); // trailing byte

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A valid `JoinAborted` buffer with an extra trailing byte must be rejected
    /// by the trailing-bytes check.
    #[test]
    fn decode_message_rejects_trailing_bytes_after_join_aborted() {
        let original = Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::JoinAborted(JoinAborted {
                handle: 1,
                frame: Frame::new(7),
            }),
        };
        let mut bytes = encode(&original).unwrap();
        bytes.push(0); // trailing byte

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }
}
