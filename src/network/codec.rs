//! Binary codec for network message serialization.
//!
//! This module provides a centralized, optimized interface for encoding and decoding
//! network messages using bincode. It encapsulates the bincode configuration to ensure
//! consistent, deterministic serialization across the codebase.
//!
//! # Design Rationale
//!
//! - **Centralized Configuration**: The bincode config is defined once, avoiding
//!   repeated `bincode::config::standard().with_little_endian().with_fixed_int_encoding()`
//!   calls.
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
    ChecksumReport, ConnectionStatus, DropAbort, DropAbortReason, DropBackfill, DropCommit,
    DropOperationId, DropPrepare, DropReceipt, DropReport, DropReportStage, DropTarget, FloorReply,
    FloorRequest, Goodbye, Input, InputAck, Message, MessageBody, MessageHeader, QualityReply,
    QualityReport, SessionConfigBlock, SyncReply, SyncRequest,
};
#[cfg(feature = "hot-join")]
use crate::network::messages::{
    JoinAborted, JoinCommitted, JoinRequest, ReactivateSlot, ReactivateSlotAck, StateSnapshot,
    StateSnapshotAck,
};
use crate::Frame;

/// Best-effort classification of a rejected protocol datagram.
///
/// Variants carrying a value are rate-limited by variant family at socket
/// boundaries: multiple unsupported versions in one poll count as one class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WireRejectKind {
    /// The bytes resemble the legacy, unversioned protocol header.
    LegacyUnversionedSuspected,
    /// The packet declares a protocol version this build does not accept.
    UnsupportedVersion {
        /// Version byte observed on the wire.
        seen: u8,
    },
    /// The packet sets header flags this protocol version does not define.
    UnknownFlags {
        /// Flags byte observed on the wire.
        seen: u8,
    },
    /// The two-byte protocol sentinel does not match.
    BadSentinel,
    /// The packet is truncated or otherwise malformed.
    Malformed,
}

impl WireRejectKind {
    pub(super) const fn rate_limit_bit(self) -> u8 {
        match self {
            Self::LegacyUnversionedSuspected => 1 << 0,
            Self::UnsupportedVersion { .. } => 1 << 1,
            Self::UnknownFlags { .. } => 1 << 2,
            Self::BadSentinel => 1 << 3,
            Self::Malformed => 1 << 4,
        }
    }
}

impl fmt::Display for WireRejectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LegacyUnversionedSuspected => f.write_str("suspected legacy unversioned packet"),
            Self::UnsupportedVersion { seen } => {
                write!(f, "unsupported protocol version {seen}")
            },
            Self::UnknownFlags { seen } => write!(f, "unknown protocol flags 0x{seen:02x}"),
            Self::BadSentinel => f.write_str("bad protocol sentinel"),
            Self::Malformed => f.write_str("malformed protocol packet"),
        }
    }
}

/// Classifies bytes that [`decode_message`] rejected.
///
/// This is a diagnostic helper, not a validator: because [`WireRejectKind`] has
/// no accepted variant, valid v1 bytes also fall through to
/// [`WireRejectKind::Malformed`]. The legacy test is intentionally heuristic and
/// may classify a malformed v1 packet as legacy; valid v1 connection IDs make
/// the layouts unambiguous.
#[must_use]
pub fn classify_wire_bytes(bytes: &[u8]) -> WireRejectKind {
    let legacy_discriminant = bytes.get(2).copied();
    let legacy_tail = bytes.get(3..6);
    if bytes.len() >= 6
        && legacy_discriminant.is_some_and(|variant| variant <= 16)
        && legacy_tail == Some(&[0, 0, 0])
    {
        return WireRejectKind::LegacyUnversionedSuspected;
    }

    let Some(sentinel) = bytes.get(..2) else {
        return WireRejectKind::Malformed;
    };
    if sentinel != super::WIRE_SENTINEL {
        return WireRejectKind::BadSentinel;
    }

    let Some(version) = bytes.get(2).copied() else {
        return WireRejectKind::Malformed;
    };
    if version != crate::PROTOCOL_VERSION {
        return WireRejectKind::UnsupportedVersion { seen: version };
    }

    let Some(flags) = bytes.get(3).copied() else {
        return WireRejectKind::Malformed;
    };
    if flags != 0 {
        return WireRejectKind::UnknownFlags { seen: flags };
    }

    WireRejectKind::Malformed
}

// The bincode configuration used throughout Fortress Rollback.
//
// We use `standard()` with `fixed_int_encoding()` for several reasons:
// - Fixed-size integers ensure deterministic message sizes (important for rollback)
// - Standard config is compatible with most platforms
// - No variable-length encoding overhead for small integers
//
// This is a zero-cost abstraction - the config is computed at compile time.
fn config() -> impl bincode::config::Config {
    bincode::config::standard()
        .with_little_endian()
        .with_fixed_int_encoding()
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

/// Hard payload-byte ceiling for one length-prefixed stream frame.
///
/// A stream frame is a four-byte little-endian payload length followed by one
/// encoded [`Message`]. The prefix itself is not included in this limit. The
/// ceiling matches the crate's existing 64 MiB hostile-decode ceiling, keeping
/// a peer-controlled stream length from driving an unbounded allocation.
pub const DEFAULT_MAX_FRAME_LEN: usize = crate::rle::DEFAULT_MAX_DECODED_LEN;

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

fn read_u64(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<u64> {
    Ok(u64::from_le_bytes(read_array(bytes, cursor, field)?))
}

fn read_i16(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<i16> {
    Ok(i16::from_le_bytes(read_array(bytes, cursor, field)?))
}

fn read_i32(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<i32> {
    Ok(i32::from_le_bytes(read_array(bytes, cursor, field)?))
}

fn read_frame(
    bytes: &[u8],
    cursor: &mut usize,
    field: &'static str,
    allow_null: bool,
) -> CodecResult<Frame> {
    let value = read_i32(bytes, cursor, field)?;
    if value < 0 && !(allow_null && value == Frame::NULL.as_i32()) {
        return Err(decode_message_error(format!(
            "invalid negative frame {value} for {field}"
        )));
    }
    Ok(Frame::new(value))
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
    // Field order MUST match the `ConnectionStatus` declaration (serde/bincode
    // serializes struct fields in declaration order): `disconnected`,
    // `last_frame`, then `epoch`.
    Ok(ConnectionStatus {
        disconnected: read_bool(bytes, cursor, "connection_status.disconnected")?,
        last_frame: read_frame(bytes, cursor, "connection_status.last_frame", true)?,
        epoch: read_u16(bytes, cursor, "connection_status.epoch")?,
    })
}

/// The fixed wire footprint, in bytes, of one encoded [`ConnectionStatus`].
//
// 1-byte bool + 4-byte fixed-int i32 + 2-byte fixed-int u16.
const CONNECTION_STATUS_WIRE_LEN: usize = 7;

/// The fixed wire footprint, in bytes, of one encoded [`Frame`] (a fixed-int
/// `i32`). Used to bound length-prefixed `Vec<Frame>` decodes (e.g. a
/// [`FloorReply`]'s `floors`).
const FRAME_WIRE_LEN: usize = 4;

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

fn decode_session_config(
    bytes: &[u8],
    cursor: &mut usize,
    fields: [&'static str; 5],
) -> CodecResult<SessionConfigBlock> {
    let [num_players, input_bytes_per_player, fps, max_prediction, desync_interval] = fields;

    Ok(SessionConfigBlock {
        num_players: read_u16(bytes, cursor, num_players)?,
        input_bytes_per_player: read_u16(bytes, cursor, input_bytes_per_player)?,
        fps: read_u32(bytes, cursor, fps)?,
        max_prediction: read_u16(bytes, cursor, max_prediction)?,
        desync_interval: read_u32(bytes, cursor, desync_interval)?,
    })
}

fn decode_sync_request(bytes: &[u8], cursor: &mut usize) -> CodecResult<SyncRequest> {
    Ok(SyncRequest {
        random_request: read_u32(bytes, cursor, "sync_request.random_request")?,
        min_compat_version: read_array::<1>(bytes, cursor, "sync_request.min_compat_version")?[0],
        features: read_u32(bytes, cursor, "sync_request.features")?,
        config: decode_session_config(
            bytes,
            cursor,
            [
                "sync_request.config.num_players",
                "sync_request.config.input_bytes_per_player",
                "sync_request.config.fps",
                "sync_request.config.max_prediction",
                "sync_request.config.desync_interval",
            ],
        )?,
        config_digest: read_u64(bytes, cursor, "sync_request.config_digest")?,
    })
}

fn decode_sync_reply(bytes: &[u8], cursor: &mut usize) -> CodecResult<SyncReply> {
    Ok(SyncReply {
        random_reply: read_u32(bytes, cursor, "sync_reply.random_reply")?,
        min_compat_version: read_array::<1>(bytes, cursor, "sync_reply.min_compat_version")?[0],
        features: read_u32(bytes, cursor, "sync_reply.features")?,
        config: decode_session_config(
            bytes,
            cursor,
            [
                "sync_reply.config.num_players",
                "sync_reply.config.input_bytes_per_player",
                "sync_reply.config.fps",
                "sync_reply.config.max_prediction",
                "sync_reply.config.desync_interval",
            ],
        )?,
        config_digest: read_u64(bytes, cursor, "sync_reply.config_digest")?,
    })
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
        start_frame,
        ack_frame,
        bytes: input_bytes,
    })
}

/// Decodes a [`FloorReply`] body: a `u32` `round_seq` followed by a
/// length-prefixed `Vec<Frame>` of per-slot pessimistic floors (the
/// double-failure-relay connected-relay reorder fix). The floor vector is
/// bounded like the message's other length-prefixed vectors — the length prefix
/// is validated against the remaining packet bytes (`FRAME_WIRE_LEN` per
/// element) before `try_reserve_exact`, so a hostile/garbled length cannot
/// over-reserve.
/// The field is advisory: a receiver reads it via `.get(slot)` and falls back
/// to `last_frame` for a missing/`Frame::NULL` slot.
fn decode_floor_reply(bytes: &[u8], cursor: &mut usize) -> CodecResult<FloorReply> {
    let round_seq = read_u32(bytes, cursor, "floor_reply.round_seq")?;
    let floor_len = read_usize(bytes, cursor, "floor_reply.floors.len")?;
    ensure_length_within_remaining(
        bytes,
        *cursor,
        floor_len,
        FRAME_WIRE_LEN,
        "floor_reply.floors",
    )?;
    let mut floors = Vec::new();
    floors.try_reserve_exact(floor_len).map_err(|_err| {
        decode_message_error(format!(
            "failed to reserve {} floor reply entries",
            floor_len
        ))
    })?;
    for _ in 0..floor_len {
        floors.push(read_frame(bytes, cursor, "floor_reply.floors", true)?);
    }
    Ok(FloorReply { round_seq, floors })
}

fn decode_drop_operation_id(
    bytes: &[u8],
    cursor: &mut usize,
    prefix: &'static str,
) -> CodecResult<DropOperationId> {
    let coordinator = match prefix {
        "drop_prepare" => "drop_prepare.operation.coordinator",
        "drop_report" => "drop_report.operation.coordinator",
        "drop_backfill" => "drop_backfill.operation.coordinator",
        "drop_commit" => "drop_commit.operation.coordinator",
        "drop_abort" => "drop_abort.operation.coordinator",
        _ => "drop.operation.coordinator",
    };
    let coordinator_generation = match prefix {
        "drop_prepare" => "drop_prepare.operation.coordinator_generation",
        "drop_report" => "drop_report.operation.coordinator_generation",
        "drop_backfill" => "drop_backfill.operation.coordinator_generation",
        "drop_commit" => "drop_commit.operation.coordinator_generation",
        "drop_abort" => "drop_abort.operation.coordinator_generation",
        _ => "drop.operation.coordinator_generation",
    };
    let sequence = match prefix {
        "drop_prepare" => "drop_prepare.operation.sequence",
        "drop_report" => "drop_report.operation.sequence",
        "drop_backfill" => "drop_backfill.operation.sequence",
        "drop_commit" => "drop_commit.operation.sequence",
        "drop_abort" => "drop_abort.operation.sequence",
        _ => "drop.operation.sequence",
    };
    let target_set_digest = match prefix {
        "drop_prepare" => "drop_prepare.operation.target_set_digest",
        "drop_report" => "drop_report.operation.target_set_digest",
        "drop_backfill" => "drop_backfill.operation.target_set_digest",
        "drop_commit" => "drop_commit.operation.target_set_digest",
        "drop_abort" => "drop_abort.operation.target_set_digest",
        _ => "drop.operation.target_set_digest",
    };
    Ok(DropOperationId {
        coordinator: read_u16(bytes, cursor, coordinator)?,
        coordinator_generation: read_u16(bytes, cursor, coordinator_generation)?,
        sequence: read_u32(bytes, cursor, sequence)?,
        target_set_digest: read_u64(bytes, cursor, target_set_digest)?,
    })
}

fn decode_drop_prepare(bytes: &[u8], cursor: &mut usize) -> CodecResult<DropPrepare> {
    let operation = decode_drop_operation_id(bytes, cursor, "drop_prepare")?;
    let target_len = read_usize(bytes, cursor, "drop_prepare.targets.len")?;
    ensure_length_within_remaining(bytes, *cursor, target_len, 4, "drop_prepare.targets")?;
    // alloc-bound: the peer-controlled count must fit as fixed 4-byte records
    // in this already-bounded packet before any reservation is attempted.
    let mut targets = Vec::new();
    targets.try_reserve_exact(target_len).map_err(|_err| {
        decode_message_error(format!("failed to reserve {target_len} drop targets"))
    })?;
    for _ in 0..target_len {
        targets.push(DropTarget {
            handle: read_u16(bytes, cursor, "drop_prepare.targets.handle")?,
            generation: read_u16(bytes, cursor, "drop_prepare.targets.generation")?,
        });
    }

    let participant_len = read_usize(bytes, cursor, "drop_prepare.participants.len")?;
    ensure_length_within_remaining(
        bytes,
        *cursor,
        participant_len,
        2,
        "drop_prepare.participants",
    )?;
    // alloc-bound: the count is bounded by fixed u16 records remaining in the packet.
    let mut participants = Vec::new();
    participants
        .try_reserve_exact(participant_len)
        .map_err(|_err| {
            decode_message_error(format!(
                "failed to reserve {participant_len} drop participants"
            ))
        })?;
    for _ in 0..participant_len {
        participants.push(read_u16(bytes, cursor, "drop_prepare.participants.handle")?);
    }
    Ok(DropPrepare {
        operation,
        targets,
        participants,
    })
}

fn decode_drop_report_stage(bytes: &[u8], cursor: &mut usize) -> CodecResult<DropReportStage> {
    match read_u32(bytes, cursor, "drop_report.stage")? {
        0 => Ok(DropReportStage::Inventory),
        1 => Ok(DropReportStage::Ready),
        2 => Ok(DropReportStage::Committed),
        other => Err(decode_message_error(format!(
            "invalid drop report stage {other}"
        ))),
    }
}

fn decode_drop_report(bytes: &[u8], cursor: &mut usize) -> CodecResult<DropReport> {
    let operation = decode_drop_operation_id(bytes, cursor, "drop_report")?;
    let participant = read_u16(bytes, cursor, "drop_report.participant")?;
    let stage = decode_drop_report_stage(bytes, cursor)?;
    let exposed_confirmed = read_frame(bytes, cursor, "drop_report.exposed_confirmed", true)?;
    let cut = read_frame(bytes, cursor, "drop_report.cut", true)?;
    let cut_digest = read_u64(bytes, cursor, "drop_report.cut_digest")?;
    let receipt_len = read_usize(bytes, cursor, "drop_report.receipts.len")?;
    ensure_length_within_remaining(bytes, *cursor, receipt_len, 10, "drop_report.receipts")?;
    // alloc-bound: each peer-controlled entry has a checked fixed 10-byte footprint.
    let mut receipts = Vec::new();
    receipts.try_reserve_exact(receipt_len).map_err(|_err| {
        decode_message_error(format!("failed to reserve {receipt_len} drop receipts"))
    })?;
    for _ in 0..receipt_len {
        receipts.push(DropReceipt {
            target: read_u16(bytes, cursor, "drop_report.receipts.target")?,
            available_from: read_frame(bytes, cursor, "drop_report.receipts.available_from", true)?,
            contiguous_through: read_frame(
                bytes,
                cursor,
                "drop_report.receipts.contiguous_through",
                true,
            )?,
        });
    }
    Ok(DropReport {
        operation,
        participant,
        stage,
        exposed_confirmed,
        cut,
        cut_digest,
        receipts,
    })
}

fn decode_drop_backfill(bytes: &[u8], cursor: &mut usize) -> CodecResult<DropBackfill> {
    let operation = decode_drop_operation_id(bytes, cursor, "drop_backfill")?;
    let chunk_index = read_u16(bytes, cursor, "drop_backfill.chunk_index")?;
    let chunk_count = read_u16(bytes, cursor, "drop_backfill.chunk_count")?;
    let start_frame = read_frame(bytes, cursor, "drop_backfill.start_frame", true)?;
    let frame_count = read_u16(bytes, cursor, "drop_backfill.frame_count")?;
    let byte_len = read_usize(bytes, cursor, "drop_backfill.bytes.len")?;
    let source = take_bytes(bytes, cursor, byte_len, "drop_backfill.bytes")?;
    // alloc-bound: `byte_len` has already been proven to fit in the unread packet.
    let mut payload = Vec::new();
    payload.try_reserve_exact(byte_len).map_err(|_err| {
        decode_message_error(format!("failed to reserve {byte_len} drop backfill bytes"))
    })?;
    payload.extend_from_slice(source);
    Ok(DropBackfill {
        operation,
        chunk_index,
        chunk_count,
        start_frame,
        frame_count,
        bytes: payload,
    })
}

fn decode_drop_abort_reason(bytes: &[u8], cursor: &mut usize) -> CodecResult<DropAbortReason> {
    match read_u32(bytes, cursor, "drop_abort.reason")? {
        0 => Ok(DropAbortReason::Superseded),
        1 => Ok(DropAbortReason::MissingHistory),
        2 => Ok(DropAbortReason::ConflictingHistory),
        3 => Ok(DropAbortReason::ParticipantLost),
        4 => Ok(DropAbortReason::Timeout),
        5 => Ok(DropAbortReason::GenerationChanged),
        6 => Ok(DropAbortReason::ResourceLimit),
        other => Err(decode_message_error(format!(
            "invalid drop abort reason {other}"
        ))),
    }
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

/// Encodes one network [`Message`] for a byte-stream transport.
///
/// The returned bytes contain a four-byte little-endian `u32` payload length,
/// followed by exactly the bytes produced by [`encode`] for `message`. The
/// length excludes the four-byte prefix. Datagram and message-oriented
/// transports should continue to use [`encode`] directly.
///
/// # Errors
///
/// Returns [`CodecError::EncodeError`] if the message cannot be encoded, its
/// payload exceeds [`DEFAULT_MAX_FRAME_LEN`], its length cannot fit in `u32`,
/// framed-length arithmetic overflows, or the output allocation cannot be
/// reserved.
///
/// # Examples
///
/// ```
/// use fortress_rollback::{network::codec::encode_framed, Message};
///
/// fn write_frame(message: &Message) -> Result<Vec<u8>, fortress_rollback::network::codec::CodecError> {
///     encode_framed(message)
/// }
/// # let _ = write_frame;
/// ```
pub fn encode_framed(message: &Message) -> CodecResult<Vec<u8>> {
    encode_framed_with_max_len(message, DEFAULT_MAX_FRAME_LEN)
}

fn encode_framed_with_max_len(message: &Message, max_frame_len: usize) -> CodecResult<Vec<u8>> {
    const PREFIX_LEN: usize = std::mem::size_of::<u32>();

    let payload_len = encoded_len(message)?;
    if payload_len > max_frame_len {
        return Err(CodecError::encode(
            format!("frame payload length {payload_len} exceeds maximum {max_frame_len}"),
            CodecOperation::EncodeMessage,
        ));
    }
    let wire_payload_len = u32::try_from(payload_len).map_err(|_err| {
        CodecError::encode(
            format!("frame payload length {payload_len} does not fit in u32"),
            CodecOperation::EncodeMessage,
        )
    })?;
    let framed_len = PREFIX_LEN.checked_add(payload_len).ok_or_else(|| {
        CodecError::encode(
            "framed message length overflow",
            CodecOperation::EncodeMessage,
        )
    })?;
    let mut framed = Vec::new();
    framed.try_reserve_exact(framed_len).map_err(|_err| {
        CodecError::encode(
            format!("failed to reserve {framed_len} framed message bytes"),
            CodecOperation::EncodeMessage,
        )
    })?;
    framed.extend_from_slice(&wire_payload_len.to_le_bytes());
    let written = encode_append(message, &mut framed)?;
    if written != payload_len {
        return Err(CodecError::encode(
            format!(
                "framed message length changed while encoding: counted {payload_len}, wrote {written}"
            ),
            CodecOperation::EncodeMessage,
        ));
    }
    Ok(framed)
}

/// Incremental decoder for length-prefixed network messages on byte streams.
///
/// Each frame starts with a four-byte little-endian `u32` payload length that
/// excludes the prefix, followed by one encoded [`Message`]. A decoder buffers
/// only an incomplete prefix or payload and yields at most one message from each
/// [`push`](Self::push) call. The returned consumed-byte count lets callers feed
/// the unconsumed suffix back immediately without an unbounded internal message
/// queue.
///
/// A malformed frame poisons the decoder because a corrupt length may make later
/// byte boundaries untrustworthy. Call [`reset`](Self::reset) only after the
/// underlying stream has been discarded or re-established; it is not a safe way
/// to resynchronize within a corrupt stream.
///
/// # Examples
///
/// ```
/// use fortress_rollback::{network::codec::FrameDecoder, Message};
/// # use fortress_rollback::network::codec::CodecError;
///
/// fn read_chunk<'a>(
///     decoder: &mut FrameDecoder,
///     mut bytes: &'a [u8],
///     mut on_message: impl FnMut(Message),
/// ) -> Result<&'a [u8], fortress_rollback::network::codec::CodecError> {
///     while !bytes.is_empty() {
///         let (message, consumed) = decoder.push(bytes)?;
///         bytes = bytes.get(consumed..).unwrap_or_default();
///         if let Some(message) = message {
///             on_message(message);
///         }
///         if consumed == 0 {
///             break;
///         }
///     }
///     Ok(bytes)
/// }
/// # fn main() -> Result<(), CodecError> {
/// # let mut decoder = FrameDecoder::new();
/// # let _suffix = read_chunk(&mut decoder, &[], |_message| {})?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct FrameDecoder {
    max_frame_len: usize,
    prefix: [u8; std::mem::size_of::<u32>()],
    prefix_len: usize,
    expected_payload_len: Option<usize>,
    payload: Vec<u8>,
    poisoned: bool,
}

impl FrameDecoder {
    const PREFIX_LEN: usize = std::mem::size_of::<u32>();

    /// Creates a decoder with the [`DEFAULT_MAX_FRAME_LEN`] payload ceiling.
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_frame_len: DEFAULT_MAX_FRAME_LEN,
            prefix: [0; Self::PREFIX_LEN],
            prefix_len: 0,
            expected_payload_len: None,
            payload: Vec::new(),
            poisoned: false,
        }
    }

    /// Creates a decoder with a smaller, caller-selected payload ceiling.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::DecodeError`] when `max_frame_len` is zero or
    /// exceeds the hard [`DEFAULT_MAX_FRAME_LEN`] ceiling.
    pub fn try_with_max_frame_len(max_frame_len: usize) -> CodecResult<Self> {
        if max_frame_len == 0 || max_frame_len > DEFAULT_MAX_FRAME_LEN {
            return Err(CodecError::decode(
                format!(
                    "frame decoder maximum must be in 1..={DEFAULT_MAX_FRAME_LEN}, got {max_frame_len}"
                ),
                CodecOperation::DecodeMessage,
            ));
        }
        Ok(Self {
            max_frame_len,
            ..Self::new()
        })
    }

    /// Feeds stream bytes into the decoder and yields at most one message.
    ///
    /// The returned `usize` is the exact number of bytes consumed from `input`.
    /// When `input` contains more than one frame, this method stops after the
    /// first and leaves the suffix to the caller. Partial prefixes and payloads
    /// are buffered and return `Ok((None, input.len()))`. Empty input is inert.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::DecodeError`] if a declared payload is zero or
    /// exceeds this decoder's limit, allocation fails, the completed payload is
    /// not exactly one valid [`Message`], or the decoder was already poisoned by
    /// an earlier error. Every error poisons the decoder until [`reset`](Self::reset).
    pub fn push(&mut self, input: &[u8]) -> CodecResult<(Option<Message>, usize)> {
        if self.poisoned {
            return Err(Self::poisoned_error());
        }
        if input.is_empty() {
            return Ok((None, 0));
        }

        let mut consumed = 0;
        if self.expected_payload_len.is_none() {
            let prefix_needed = Self::PREFIX_LEN.saturating_sub(self.prefix_len);
            let take = prefix_needed.min(input.len());
            let Some(source) = input.get(..take) else {
                return self.poison("frame prefix slice out of bounds");
            };
            let Some(prefix_end) = self.prefix_len.checked_add(take) else {
                return self.poison("frame prefix offset overflow");
            };
            let Some(destination) = self.prefix.get_mut(self.prefix_len..prefix_end) else {
                return self.poison("frame prefix destination out of bounds");
            };
            destination.copy_from_slice(source);
            self.prefix_len = prefix_end;
            consumed = take;

            if self.prefix_len < Self::PREFIX_LEN {
                return Ok((None, consumed));
            }

            let declared = usize::try_from(u32::from_le_bytes(self.prefix)).map_err(|_err| {
                self.poisoned = true;
                CodecError::decode(
                    "frame payload length does not fit in usize",
                    CodecOperation::DecodeMessage,
                )
            })?;
            if declared == 0 {
                return self.poison("frame payload length must be non-zero");
            }
            if declared > self.max_frame_len {
                return self.poison(format!(
                    "frame payload length {declared} exceeds maximum {}",
                    self.max_frame_len
                ));
            }
            if let Err(_err) = self.payload.try_reserve_exact(declared) {
                return self.poison(format!("failed to reserve {declared} frame payload bytes"));
            }
            self.expected_payload_len = Some(declared);
        }

        let Some(expected) = self.expected_payload_len else {
            return self.poison("frame decoder lost its expected payload length");
        };
        let payload_needed = expected.saturating_sub(self.payload.len());
        let available = input.len().saturating_sub(consumed);
        let take = payload_needed.min(available);
        let Some(input_end) = consumed.checked_add(take) else {
            return self.poison("frame input offset overflow");
        };
        let Some(source) = input.get(consumed..input_end) else {
            return self.poison("frame payload slice out of bounds");
        };
        // alloc-bound: the full peer-declared payload length was bounded by
        // `max_frame_len` and reserved fallibly before any payload byte is copied.
        self.payload.extend_from_slice(source);
        consumed = input_end;

        if self.payload.len() < expected {
            return Ok((None, consumed));
        }

        match decode_message(&self.payload) {
            Ok((message, decoded_len)) if decoded_len == expected => {
                self.prefix_len = 0;
                self.expected_payload_len = None;
                self.payload.clear();
                Ok((Some(message), consumed))
            },
            Ok((_message, decoded_len)) => self.poison(format!(
                "decoded frame consumed {decoded_len} bytes, expected {expected}"
            )),
            Err(error) => {
                self.poisoned = true;
                Err(error)
            },
        }
    }

    /// Verifies that the stream ended exactly between frames.
    /// # Errors
    ///
    /// Returns [`CodecError::DecodeError`] if the decoder is poisoned or holds an
    /// incomplete length prefix or payload. An incomplete end poisons the decoder.
    pub fn finish(&mut self) -> CodecResult<()> {
        if self.poisoned {
            return Err(Self::poisoned_error());
        }
        if self.prefix_len == 0 && self.expected_payload_len.is_none() {
            return Ok(());
        }
        if let Some(expected) = self.expected_payload_len {
            return self.poison(format!(
                "incomplete frame payload: buffered {} of {expected} bytes",
                self.payload.len()
            ));
        }
        self.poison(format!(
            "incomplete length prefix: buffered {} of {} bytes",
            self.prefix_len,
            Self::PREFIX_LEN
        ))
    }

    /// Clears all buffered state and releases its payload allocation.
    ///
    /// Use this only for a new or otherwise independently re-established stream;
    /// reset cannot locate a trustworthy boundary in the stream that failed.
    pub fn reset(&mut self) {
        self.prefix = [0; Self::PREFIX_LEN];
        self.prefix_len = 0;
        self.expected_payload_len = None;
        self.payload = Vec::new();
        self.poisoned = false;
    }

    /// Returns the configured maximum payload size, excluding the prefix.
    #[must_use]
    pub const fn max_frame_len(&self) -> usize {
        self.max_frame_len
    }

    /// Returns the number of prefix and payload bytes currently buffered.
    #[must_use]
    pub fn buffered_len(&self) -> usize {
        self.prefix_len.saturating_add(self.payload.len())
    }

    fn poison<T>(&mut self, message: impl Into<String>) -> CodecResult<T> {
        self.poisoned = true;
        Err(CodecError::decode(message, CodecOperation::DecodeMessage))
    }

    fn poisoned_error() -> CodecError {
        CodecError::decode(
            "frame decoder is poisoned; reset it only for a new stream",
            CodecOperation::DecodeMessage,
        )
    }
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
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
/// This uses the crate's generic bincode configuration and is not
/// allocation-bounded. Do not use it for peer-controlled bytes whose decoded
/// type can contain length-prefixed containers such as `Vec`, `String`, or maps.
/// For received network [`Message`] bytes, use [`decode_message`] instead.
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
/// invalid variant or boolean, contains an invalid connection-status,
/// floor-gossip, or checksum-report frame value, has trailing bytes, or declares
/// a length that cannot fit in the remaining packet.
pub fn decode_message(bytes: &[u8]) -> CodecResult<(Message, usize)> {
    let mut cursor = 0;
    let sentinel = read_array(bytes, &mut cursor, "message.header.sentinel")?;
    if sentinel != super::WIRE_SENTINEL {
        return Err(decode_message_error("invalid message header sentinel"));
    }
    let protocol_version =
        read_array::<1>(bytes, &mut cursor, "message.header.protocol_version")?[0];
    if protocol_version < super::MIN_SUPPORTED_PROTOCOL_VERSION
        || protocol_version > crate::PROTOCOL_VERSION
    {
        return Err(decode_message_error(format!(
            "unsupported protocol version {protocol_version}"
        )));
    }
    let flags = read_array::<1>(bytes, &mut cursor, "message.header.flags")?[0];
    if flags != 0 {
        return Err(decode_message_error(format!(
            "unknown protocol flags 0x{flags:02x}"
        )));
    }
    let conn_id = read_u32(bytes, &mut cursor, "message.header.conn_id")?;
    if !super::is_valid_conn_id(conn_id) {
        return Err(decode_message_error(format!(
            "invalid connection ID 0x{conn_id:08x}"
        )));
    }
    let header = MessageHeader {
        sentinel,
        protocol_version,
        flags,
        conn_id,
    };
    let variant = read_u32(bytes, &mut cursor, "message.body.variant")?;
    let body = match variant {
        0 => MessageBody::SyncRequest(decode_sync_request(bytes, &mut cursor)?),
        1 => MessageBody::SyncReply(decode_sync_reply(bytes, &mut cursor)?),
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
            frame: read_frame(bytes, &mut cursor, "checksum_report.frame", false)?,
        }),
        7 => MessageBody::KeepAlive,
        // Floor-round variants (double-failure-relay connected-relay reorder fix,
        // S55), appended after the original core block — see the `MessageBody`
        // enum comment. Hot-join variants occupy discriminants 10..=16 in every
        // build; builds without the feature recognize and reject them below.
        8 => MessageBody::FloorRequest(FloorRequest {
            round_seq: read_u32(bytes, &mut cursor, "floor_request.round_seq")?,
        }),
        9 => MessageBody::FloorReply(decode_floor_reply(bytes, &mut cursor)?),
        #[cfg(feature = "hot-join")]
        10 => MessageBody::JoinRequest(JoinRequest {
            player_handle: read_usize(bytes, &mut cursor, "join_request.player_handle")?,
        }),
        #[cfg(feature = "hot-join")]
        11 => MessageBody::StateSnapshot(decode_state_snapshot(bytes, &mut cursor)?),
        #[cfg(feature = "hot-join")]
        12 => MessageBody::StateSnapshotAck(StateSnapshotAck {
            frame: Frame::new(read_i32(bytes, &mut cursor, "state_snapshot_ack.frame")?),
        }),
        #[cfg(feature = "hot-join")]
        13 => MessageBody::ReactivateSlot(ReactivateSlot {
            handle: read_usize(bytes, &mut cursor, "reactivate_slot.handle")?,
            frame: Frame::new(read_i32(bytes, &mut cursor, "reactivate_slot.frame")?),
        }),
        #[cfg(feature = "hot-join")]
        14 => MessageBody::ReactivateSlotAck(ReactivateSlotAck {
            handle: read_usize(bytes, &mut cursor, "reactivate_slot_ack.handle")?,
            frame: Frame::new(read_i32(bytes, &mut cursor, "reactivate_slot_ack.frame")?),
        }),
        #[cfg(feature = "hot-join")]
        15 => MessageBody::JoinCommitted(JoinCommitted {
            handle: read_usize(bytes, &mut cursor, "join_committed.handle")?,
            frame: Frame::new(read_i32(bytes, &mut cursor, "join_committed.frame")?),
        }),
        #[cfg(feature = "hot-join")]
        16 => MessageBody::JoinAborted(JoinAborted {
            handle: read_usize(bytes, &mut cursor, "join_aborted.handle")?,
            frame: Frame::new(read_i32(bytes, &mut cursor, "join_aborted.frame")?),
        }),
        #[cfg(not(feature = "hot-join"))]
        10..=16 => {
            return Err(decode_message_error(format!(
                "message body variant {variant} requires the disabled hot-join feature"
            )))
        },
        17 => MessageBody::Goodbye(Goodbye {
            reason: read_array::<1>(bytes, &mut cursor, "goodbye.reason")?[0],
        }),
        18 => MessageBody::DropPrepare(decode_drop_prepare(bytes, &mut cursor)?),
        19 => MessageBody::DropReport(decode_drop_report(bytes, &mut cursor)?),
        20 => MessageBody::DropBackfill(decode_drop_backfill(bytes, &mut cursor)?),
        21 => MessageBody::DropCommit(DropCommit {
            operation: decode_drop_operation_id(bytes, &mut cursor, "drop_commit")?,
            cut: read_frame(bytes, &mut cursor, "drop_commit.cut", true)?,
            cut_digest: read_u64(bytes, &mut cursor, "drop_commit.cut_digest")?,
        }),
        22 => MessageBody::DropAbort(DropAbort {
            operation: decode_drop_operation_id(bytes, &mut cursor, "drop_abort")?,
            reason: decode_drop_abort_reason(bytes, &mut cursor)?,
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
/// Like [`decode`], this is not allocation-bounded; use [`decode_message`] for
/// received peer [`Message`] bytes instead of generic bincode decoding.
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
/// # Bounds both *allocation* and *recursion depth*
///
/// The byte limit above caps total bytes allocated; on its own it does **not**
/// cap the decode's call stack. bincode decodes a recursive type (one
/// transitively containing `Box<Self>`, `Vec<Self>`, etc.) by recursing once per
/// level of nesting, and a deeply-nested value can be encoded in far fewer bytes
/// than [`MAX_BOUNDED_DECODE_LEN`] (each level adds only a tag/length byte or
/// two), so a malicious blob could stay under the byte cap yet overflow the
/// stack mid-decode — an uncatchable abort, not a recoverable `Err`. Because
/// `Config::State` is only `DeserializeOwned` (it may legitimately be
/// recursive), this function decodes through
/// [`deserialize_depth_limited`](super::codec_depth::deserialize_depth_limited),
/// which **rejects** nesting deeper than
/// [`MAX_DECODE_DEPTH`](super::codec_depth::MAX_DECODE_DEPTH) with a recoverable
/// `Err` before the stack can overflow. A `Send` bound (which a thread-stack
/// trick would require) is therefore not needed, and no value shallower than the
/// limit is affected.
///
/// # Errors
///
/// Returns [`CodecError::DecodeError`] when `bytes` exceeds the byte cap, when a
/// declared container length would exceed the cap, when container nesting exceeds
/// [`MAX_DECODE_DEPTH`](super::codec_depth::MAX_DECODE_DEPTH), or when bincode
/// otherwise fails to decode (truncated input; trailing bytes are *not* rejected
/// here — use [`decode_bounded_with_consumed`] if you need the consumed length).
#[cfg(feature = "hot-join")]
pub(crate) fn decode_bounded<T: DeserializeOwned>(bytes: &[u8]) -> CodecResult<T> {
    // alloc-bound: the same MAX_BOUNDED_DECODE_LEN byte cap as
    // `decode_bounded_with_consumed` (pre-reject + bincode `Limit`); see
    // `bounded_decode_config` / `reject_over_cap` for the full allocation
    // analysis. This path additionally bounds recursion DEPTH via the
    // depth-limited deserializer below.
    reject_over_cap(bytes.len())?;
    let config = bounded_decode_config();
    // Wrap bincode's deserializer so a deeply-nested (recursive) `Config::State`
    // blob is rejected with a recoverable error instead of overflowing the stack
    // (B-codec). `BorrowedSerdeDecoder` exposes the serde deserializer; the
    // borrowed reader is correct for a `DeserializeOwned` type (no field actually
    // borrows from the slice).
    let mut decoder = bincode::serde::BorrowedSerdeDecoder::from_slice(bytes, config, ());
    super::codec_depth::deserialize_depth_limited::<T, _>(
        decoder.as_deserializer(),
        super::codec_depth::MAX_DECODE_DEPTH,
    )
    .map_err(|e| CodecError::decode(e.to_string(), CodecOperation::Decode))
}

/// [`decode_bounded`] variant that also returns the number of bytes consumed,
/// for callers that decode several bounded values back-to-back from one slice
/// (e.g. the hot-join bridge-input blob: `num_players` fixed-width inputs
/// concatenated) and must verify exact consumption.
///
/// Unlike [`decode_bounded`], this path is **not** recursion-depth-limited, and
/// it does not need to be: its only callers decode `Config::Input`, which is
/// bound `Copy`, and a `Copy + Sized` type is provably non-recursive (recursion
/// requires `Box`/`Vec`/heap indirection, none of which are `Copy`; a direct
/// `enum E { N([E; 2]) }` is infinite-size and rejected by the compiler). So
/// malicious bytes cannot drive this decode into unbounded recursion, and the
/// hot input-decode path stays on the direct bincode call.
pub(crate) fn decode_bounded_with_consumed<T: DeserializeOwned>(
    bytes: &[u8],
) -> CodecResult<(T, usize)> {
    // alloc-bound: this decode is bounded to MAX_BOUNDED_DECODE_LEN (64 MiB) two
    // ways (see `reject_over_cap` + `bounded_decode_config`). (1) `bytes` is
    // pre-rejected when longer than the cap. (2) The bincode
    // `Limit<MAX_BOUNDED_DECODE_LEN>` config makes every container decoder claim
    // its `len * size_of::<element>()` byte footprint against a running total
    // *before* allocating and error once it would exceed the cap, so a malicious
    // length prefix (e.g. a `Vec<u8>` claiming u64::MAX) yields a decode error,
    // never a giant `vec![0u8; len]`. No input this function accepts can drive an
    // allocation past the cap. Empirically verified for both the per-element
    // (`Vec<T>`) and native (`serde_bytes`) decode paths.
    reject_over_cap(bytes.len())?;
    let config = bounded_decode_config();
    bincode::serde::decode_from_slice(bytes, config)
        .map_err(|e| CodecError::decode(e.to_string(), CodecOperation::Decode))
}

/// Rejects an input slice longer than [`MAX_BOUNDED_DECODE_LEN`] before any
/// decode begins — the first of the two allocation bounds shared by
/// [`decode_bounded`] and [`decode_bounded_with_consumed`].
fn reject_over_cap(len: usize) -> CodecResult<()> {
    if len > MAX_BOUNDED_DECODE_LEN {
        return Err(CodecError::decode(
            format!("input length {len} exceeds bounded-decode cap {MAX_BOUNDED_DECODE_LEN}"),
            CodecOperation::Decode,
        ));
    }
    Ok(())
}

/// The bincode config for bounded peer-decodes: the shared codec config
/// ([`config`]'s `standard().with_little_endian().with_fixed_int_encoding()`,
/// kept byte-for-byte identical) plus a [`MAX_BOUNDED_DECODE_LEN`] byte limit.
/// `with_limit` is an inherent method on the concrete `Configuration`, so this
/// is built from the concrete base rather than the `impl Config` return of
/// [`config`].
fn bounded_decode_config() -> impl bincode::config::Config {
    bincode::config::standard()
        .with_little_endian()
        .with_fixed_int_encoding()
        .with_limit::<MAX_BOUNDED_DECODE_LEN>()
}

#[cfg(test)]
#[path = "wire_golden_v1.rs"]
mod wire_golden_v1;

#[cfg(test)]
#[path = "wire_golden_legacy_0_9.rs"]
mod wire_golden_legacy_0_9;

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
        ChecksumReport, ConnectionStatus, FloorReply, FloorRequest, Input, InputAck, Message,
        MessageBody, MessageHeader, QualityReply, QualityReport, SessionConfigBlock, SyncReply,
        SyncRequest,
    };

    fn wire_prefix(conn_id: u32, variant: u32) -> Vec<u8> {
        let mut bytes = encode(&MessageHeader::new(conn_id)).unwrap();
        bytes.extend_from_slice(&variant.to_le_bytes());
        bytes
    }

    fn keep_alive(conn_id: u32) -> Message {
        Message {
            header: MessageHeader::new(conn_id),
            body: MessageBody::KeepAlive,
        }
    }

    fn drain_framed(decoder: &mut FrameDecoder, mut bytes: &[u8]) -> CodecResult<Vec<Message>> {
        let mut messages = Vec::new();
        while !bytes.is_empty() {
            let (message, consumed) = decoder.push(bytes)?;
            assert!(consumed > 0, "non-empty input must make progress");
            bytes = bytes
                .get(consumed..)
                .expect("decoder consumption is in bounds");
            if let Some(message) = message {
                messages.push(message);
            }
        }
        Ok(messages)
    }

    #[test]
    fn encode_framed_prefixes_exact_payload_length_in_little_endian() {
        let message = keep_alive(0xABCD);
        let payload = encode(&message).unwrap();

        let framed = encode_framed(&message).unwrap();

        assert_eq!(
            framed.get(..4),
            Some((payload.len() as u32).to_le_bytes().as_slice())
        );
        assert_eq!(framed.get(4..), Some(payload.as_slice()));
        assert_eq!(framed.len(), payload.len() + 4);
    }

    #[test]
    fn frame_decoder_accepts_every_single_split_of_prefix_and_payload() {
        let message = keep_alive(0xABCD);
        let framed = encode_framed(&message).unwrap();

        for split in 0..=framed.len() {
            let mut decoder = FrameDecoder::new();
            let (first, consumed) = decoder.push(&framed[..split]).unwrap();
            assert_eq!(consumed, split, "split {split}");
            assert_eq!(first, (split == framed.len()).then(|| message.clone()));
            let (second, consumed) = decoder.push(&framed[split..]).unwrap();
            assert_eq!(consumed, framed.len() - split, "split {split}");
            assert_eq!(second, (split != framed.len()).then(|| message.clone()));
            assert_eq!(decoder.buffered_len(), 0);
            assert_eq!(decoder.finish(), Ok(()));
        }
    }

    #[test]
    fn frame_decoder_accepts_one_byte_chunks() {
        let message = keep_alive(0xABCD);
        let framed = encode_framed(&message).unwrap();
        let mut decoder = FrameDecoder::new();
        let mut decoded = None;

        for byte in &framed {
            let (next, consumed) = decoder.push(std::slice::from_ref(byte)).unwrap();
            assert_eq!(consumed, 1);
            assert!(decoded.is_none());
            if next.is_some() {
                decoded = next;
            }
        }

        assert_eq!(decoded, Some(message));
        assert_eq!(decoder.finish(), Ok(()));
    }

    #[test]
    fn frame_decoder_consumes_only_one_of_multiple_frames_per_call() {
        let messages = [keep_alive(1), keep_alive(2), keep_alive(3)];
        let mut stream = Vec::new();
        for message in &messages {
            stream.extend_from_slice(&encode_framed(message).unwrap());
        }
        let first_len = encode_framed(&messages[0]).unwrap().len();
        let mut decoder = FrameDecoder::new();

        let (first, consumed) = decoder.push(&stream).unwrap();

        assert_eq!(first, Some(messages[0].clone()));
        assert_eq!(consumed, first_len);
        assert_eq!(
            drain_framed(&mut decoder, &stream[consumed..]).unwrap(),
            messages[1..]
        );
    }

    #[test]
    fn frame_decoder_empty_input_is_inert() {
        let mut decoder = FrameDecoder::new();
        assert_eq!(decoder.push(&[]), Ok((None, 0)));
        assert_eq!(decoder.buffered_len(), 0);
        assert_eq!(decoder.finish(), Ok(()));
    }

    #[test]
    fn frame_decoder_limit_validation_is_fail_closed() {
        assert!(FrameDecoder::try_with_max_frame_len(0).is_err());
        assert!(FrameDecoder::try_with_max_frame_len(DEFAULT_MAX_FRAME_LEN + 1).is_err());
        assert!(FrameDecoder::try_with_max_frame_len(usize::MAX).is_err());

        let decoder = FrameDecoder::try_with_max_frame_len(DEFAULT_MAX_FRAME_LEN).unwrap();
        assert_eq!(decoder.max_frame_len(), DEFAULT_MAX_FRAME_LEN);
        assert_eq!(FrameDecoder::new().max_frame_len(), DEFAULT_MAX_FRAME_LEN);
    }

    #[test]
    fn frame_decoder_accepts_exact_limit_and_rejects_one_byte_over_before_payload() {
        let message = keep_alive(1);
        let payload = encode(&message).unwrap();
        let mut exact = FrameDecoder::try_with_max_frame_len(payload.len()).unwrap();
        assert_eq!(
            exact.push(&encode_framed(&message).unwrap()).unwrap(),
            (Some(message), payload.len() + 4)
        );

        let mut over = FrameDecoder::try_with_max_frame_len(payload.len() - 1).unwrap();
        let prefix = u32::try_from(payload.len()).unwrap().to_le_bytes();
        let error = over.push(&prefix).expect_err("over-limit prefix must fail");
        assert!(error.to_string().contains("exceeds maximum"));
        assert!(over.push(&[]).unwrap_err().to_string().contains("poisoned"));
    }

    #[test]
    fn frame_decoder_rejects_zero_length_and_remains_poisoned_until_reset() {
        let mut decoder = FrameDecoder::new();
        let error = decoder.push(&0_u32.to_le_bytes()).unwrap_err();
        assert!(error.to_string().contains("must be non-zero"));
        assert!(decoder
            .finish()
            .unwrap_err()
            .to_string()
            .contains("poisoned"));
        assert!(decoder
            .push(&encode_framed(&keep_alive(1)).unwrap())
            .unwrap_err()
            .to_string()
            .contains("poisoned"));

        decoder.reset();

        assert_eq!(
            drain_framed(&mut decoder, &encode_framed(&keep_alive(1)).unwrap()).unwrap(),
            vec![keep_alive(1)]
        );
    }

    #[test]
    fn frame_decoder_malformed_or_trailing_payload_poison_decoder() {
        let valid_payload = encode(&keep_alive(1)).unwrap();
        let cases = [vec![0xFF], {
            let mut trailing = valid_payload;
            trailing.push(0);
            trailing
        }];

        for payload in cases {
            let mut framed = u32::try_from(payload.len()).unwrap().to_le_bytes().to_vec();
            framed.extend_from_slice(&payload);
            let mut decoder = FrameDecoder::new();

            assert!(decoder.push(&framed).is_err());
            assert!(decoder
                .push(&[])
                .unwrap_err()
                .to_string()
                .contains("poisoned"));
        }
    }

    #[test]
    fn frame_decoder_finish_distinguishes_partial_prefix_and_payload() {
        let framed = encode_framed(&keep_alive(1)).unwrap();
        for prefix_len in 1..4 {
            let mut decoder = FrameDecoder::new();
            assert_eq!(decoder.push(&framed[..prefix_len]), Ok((None, prefix_len)));
            let error = decoder.finish().unwrap_err();
            assert!(error.to_string().contains("incomplete length prefix"));
        }

        let mut decoder = FrameDecoder::new();
        let partial_len = framed.len() - 1;
        assert_eq!(
            decoder.push(&framed[..partial_len]),
            Ok((None, partial_len))
        );
        let error = decoder.finish().unwrap_err();
        assert!(error.to_string().contains("incomplete frame payload"));
        assert!(error.to_string().contains("11 of 12"));
    }

    #[test]
    fn encode_framed_limit_helper_accepts_exact_and_rejects_over_limit() {
        let message = keep_alive(1);
        let payload_len = encode(&message).unwrap().len();

        assert!(encode_framed_with_max_len(&message, payload_len).is_ok());
        let error = encode_framed_with_max_len(&message, payload_len - 1).unwrap_err();
        assert!(error.to_string().contains("exceeds maximum"));
    }

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
            header: MessageHeader::new(0xABCD),
            body: MessageBody::SyncRequest(SyncRequest {
                random_request: 999,
                ..SyncRequest::default()
            }),
        };
        let bytes = encode(&original).unwrap();
        let (decoded, _): (Message, _) = decode(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn codec_wire_format_uses_fixed_little_endian_bytes() {
        assert_eq!(
            crate::PROTOCOL_VERSION,
            1,
            "wire bytes changed without a version bump"
        );
        let cases = [
            (
                "sync_request",
                Message {
                    header: MessageHeader::new(0xABCD),
                    body: MessageBody::SyncRequest(SyncRequest {
                        random_request: 999,
                        min_compat_version: 1,
                        features: 1,
                        config: SessionConfigBlock {
                            num_players: 2,
                            input_bytes_per_player: 4,
                            fps: 60,
                            max_prediction: 8,
                            desync_interval: 60,
                        },
                        config_digest: 0x5082_C060_858A_E1C8,
                    }),
                },
                vec![
                    0xF5, 0x52, 0x01, 0x00, // sentinel, version, flags
                    0xCD, 0xAB, 0x00, 0x00, // conn_id
                    0x00, 0x00, 0x00, 0x00, // MessageBody::SyncRequest tag
                    0xE7, 0x03, 0x00, 0x00, // random_request
                    0x01, // min_compat_version
                    0x01, 0x00, 0x00, 0x00, // features
                    0x02, 0x00, // config.num_players
                    0x04, 0x00, // config.input_bytes_per_player
                    0x3C, 0x00, 0x00, 0x00, // config.fps
                    0x08, 0x00, // config.max_prediction
                    0x3C, 0x00, 0x00, 0x00, // config.desync_interval
                    0xC8, 0xE1, 0x8A, 0x85, 0x60, 0xC0, 0x82, 0x50, // config_digest
                ],
            ),
            (
                "quality_report",
                Message {
                    header: MessageHeader::new(0x1234),
                    body: MessageBody::QualityReport(QualityReport {
                        frame_advantage: -2,
                        ping: 0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10,
                    }),
                },
                vec![
                    0xF5, 0x52, 0x01, 0x00, // sentinel, version, flags
                    0x34, 0x12, 0x00, 0x00, // MessageHeader::conn_id
                    0x04, 0x00, 0x00, 0x00, // MessageBody::QualityReport tag
                    0xFE, 0xFF, // frame_advantage: i16 -2
                    0x10, 0x0F, 0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04,
                    0x03, 0x02, 0x01, // ping: u128
                ],
            ),
            (
                "goodbye",
                Message {
                    header: MessageHeader::new(0x1234),
                    body: MessageBody::Goodbye(Goodbye { reason: 7 }),
                },
                vec![
                    0xF5, 0x52, 0x01, 0x00, // sentinel, version, flags
                    0x34, 0x12, 0x00, 0x00, // MessageHeader::conn_id
                    0x11, 0x00, 0x00, 0x00, // MessageBody::Goodbye tag 17
                    0x07, // reason
                ],
            ),
        ];

        for (name, original, expected) in cases {
            let bytes = encode(&original).unwrap();
            assert_eq!(bytes, expected, "encoded bytes for {name}");

            let generic: Message = decode_value(&bytes).unwrap();
            let (manual, consumed) = decode_message(&bytes).unwrap();
            assert_eq!(generic, original, "generic decode for {name}");
            assert_eq!(manual, original, "manual decode for {name}");
            assert_eq!(consumed, bytes.len(), "consumed bytes for {name}");
        }
    }

    #[test]
    fn decode_message_rejects_every_truncated_handshake_field() {
        let message = Message {
            header: MessageHeader::new(1),
            body: MessageBody::SyncRequest(SyncRequest {
                random_request: 7,
                min_compat_version: 1,
                features: 1,
                config: SessionConfigBlock {
                    num_players: 2,
                    input_bytes_per_player: 4,
                    fps: 60,
                    max_prediction: 8,
                    desync_interval: 60,
                },
                config_digest: 0x5082_C060_858A_E1C8,
            }),
        };
        let bytes = encode(&message).unwrap();
        assert_eq!(bytes.len(), 43);

        for len in 0..bytes.len() {
            assert!(
                decode_message(&bytes[..len]).is_err(),
                "truncated handshake prefix of {len} bytes must be rejected"
            );
        }
        assert_eq!(decode_message(&bytes).unwrap(), (message, bytes.len()));
    }

    #[test]
    fn classify_wire_bytes_uses_stable_reject_precedence() {
        let legacy = [0x34, 0x12, 16, 0, 0, 0];
        let mut bad_sentinel = wire_prefix(1, 7);
        bad_sentinel[0] = 0;
        let mut unsupported = wire_prefix(1, 7);
        unsupported[2] = crate::PROTOCOL_VERSION.saturating_add(1);
        let mut unknown_flags = wire_prefix(1, 7);
        unknown_flags[3] = 0x80;
        let valid = wire_prefix(1, 7);

        let cases = [
            (
                legacy.as_slice(),
                WireRejectKind::LegacyUnversionedSuspected,
            ),
            (bad_sentinel.as_slice(), WireRejectKind::BadSentinel),
            (
                unsupported.as_slice(),
                WireRejectKind::UnsupportedVersion {
                    seen: crate::PROTOCOL_VERSION.saturating_add(1),
                },
            ),
            (
                unknown_flags.as_slice(),
                WireRejectKind::UnknownFlags { seen: 0x80 },
            ),
            (&[0xF5][..], WireRejectKind::Malformed),
            (valid.as_slice(), WireRejectKind::Malformed),
        ];

        for (bytes, expected) in cases {
            assert_eq!(classify_wire_bytes(bytes), expected, "bytes={bytes:02x?}");
        }

        let sentinel_collision_legacy = [0xF5, 0x52, 1, 0, 0, 0];
        assert_eq!(
            classify_wire_bytes(&sentinel_collision_legacy),
            WireRejectKind::LegacyUnversionedSuspected,
            "the documented best-effort legacy heuristic takes precedence"
        );
    }

    #[test]
    fn decode_message_rejects_every_invalid_v1_header_before_body_decode() {
        let valid = wire_prefix(1, 7);
        for len in 0..valid.len() {
            assert!(
                decode_message(&valid[..len]).is_err(),
                "truncated prefix of {len} bytes must be rejected"
            );
        }

        let mut invalid_headers = Vec::new();
        let mut bad_sentinel = valid.clone();
        bad_sentinel[1] ^= 1;
        invalid_headers.push(bad_sentinel);
        let mut unsupported = valid.clone();
        unsupported[2] = crate::PROTOCOL_VERSION.saturating_add(1);
        invalid_headers.push(unsupported);
        let mut flags = valid;
        flags[3] = 1;
        invalid_headers.push(flags);
        invalid_headers.push(wire_prefix(0, 7));
        invalid_headers.push(wire_prefix(0x1234_0000, 7));

        for bytes in invalid_headers {
            assert!(
                decode_message(&bytes).is_err(),
                "invalid header must be rejected: {bytes:02x?}"
            );
        }

        for conn_id in [1, u32::MAX] {
            let bytes = wire_prefix(conn_id, 7);
            assert_eq!(
                decode_message(&bytes),
                Ok((
                    Message {
                        header: MessageHeader::new(conn_id),
                        body: MessageBody::KeepAlive,
                    },
                    bytes.len(),
                ))
            );
        }
    }

    #[cfg(not(feature = "hot-join"))]
    #[test]
    fn decode_message_recognizes_disabled_hot_join_discriminants() {
        for variant in 10..=16 {
            let bytes = wire_prefix(1, variant);
            let error = decode_message(&bytes).expect_err("disabled body must be rejected");
            assert!(
                error
                    .to_string()
                    .contains("requires the disabled hot-join feature"),
                "variant {variant} should be recognized, not reported as unknown: {error}"
            );
            assert_eq!(classify_wire_bytes(&bytes), WireRejectKind::Malformed);
        }
    }

    #[test]
    fn decode_message_roundtrips_every_body_variant() {
        let messages = [
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::SyncRequest(SyncRequest {
                    random_request: 999,
                    ..SyncRequest::default()
                }),
            },
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::SyncReply(SyncReply {
                    random_reply: 123,
                    ..SyncReply::default()
                }),
            },
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::Input(Input {
                    peer_connect_status: vec![
                        ConnectionStatus {
                            disconnected: false,
                            last_frame: Frame::new(10),
                            // Non-zero epoch spanning both u16 bytes (> 255) pins
                            // the connect-status `epoch` wire round-trip.
                            epoch: 513,
                        },
                        ConnectionStatus {
                            disconnected: true,
                            last_frame: Frame::new(20),
                            epoch: 7,
                        },
                    ],
                    start_frame: Frame::new(100),
                    ack_frame: Frame::new(50),
                    bytes: vec![1, 2, 3, 4, 5],
                }),
            },
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::InputAck(InputAck {
                    ack_frame: Frame::new(77),
                }),
            },
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::QualityReport(QualityReport {
                    frame_advantage: -2,
                    ping: 1_000,
                }),
            },
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::QualityReply(QualityReply { pong: 2_000 }),
            },
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::ChecksumReport(ChecksumReport {
                    checksum: 0xDEAD_BEEF,
                    frame: Frame::new(88),
                }),
            },
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::KeepAlive,
            },
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::FloorRequest(FloorRequest { round_seq: 42 }),
            },
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::FloorReply(FloorReply {
                    round_seq: 42,
                    floors: vec![Frame::new(4), Frame::new(-1), Frame::new(10)],
                }),
            },
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::Goodbye(Goodbye { reason: 3 }),
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

    fn drop_operation() -> DropOperationId {
        DropOperationId {
            coordinator: 2,
            coordinator_generation: 7,
            sequence: 0x1020_3040,
            target_set_digest: 0x0102_0304_0506_0708,
        }
    }

    fn drop_bodies() -> Vec<(u32, MessageBody)> {
        vec![
            (
                18,
                MessageBody::DropPrepare(DropPrepare {
                    operation: drop_operation(),
                    targets: vec![
                        DropTarget {
                            handle: 4,
                            generation: 9,
                        },
                        DropTarget {
                            handle: 5,
                            generation: 9,
                        },
                    ],
                    participants: vec![0, 1, 2, 3],
                }),
            ),
            (
                19,
                MessageBody::DropReport(DropReport {
                    operation: drop_operation(),
                    participant: 1,
                    stage: DropReportStage::Inventory,
                    exposed_confirmed: Frame::new(30),
                    cut: Frame::NULL,
                    cut_digest: 0,
                    receipts: vec![
                        DropReceipt {
                            target: 4,
                            available_from: Frame::new(10),
                            contiguous_through: Frame::new(31),
                        },
                        DropReceipt {
                            target: 5,
                            available_from: Frame::new(11),
                            contiguous_through: Frame::new(31),
                        },
                    ],
                }),
            ),
            (
                20,
                MessageBody::DropBackfill(DropBackfill {
                    operation: drop_operation(),
                    chunk_index: 1,
                    chunk_count: 3,
                    start_frame: Frame::new(24),
                    frame_count: 2,
                    bytes: vec![0xAA, 0xBB, 0xCC, 0xDD],
                }),
            ),
            (
                21,
                MessageBody::DropCommit(DropCommit {
                    operation: drop_operation(),
                    cut: Frame::new(31),
                    cut_digest: 0x1112_1314_1516_1718,
                }),
            ),
            (
                22,
                MessageBody::DropAbort(DropAbort {
                    operation: drop_operation(),
                    reason: DropAbortReason::ConflictingHistory,
                }),
            ),
        ]
    }

    #[test]
    fn coordinated_drop_v1_goldens_roundtrip_with_manual_generic_parity() {
        for (tag, body) in drop_bodies() {
            let original = Message {
                header: MessageHeader::new(0x1234),
                body,
            };
            let bytes = encode(&original).unwrap();
            let expected: &[u8] = match tag {
                18 => &[
                    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x02,
                    0x00, 0x07, 0x00, 0x40, 0x30, 0x20, 0x10, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03,
                    0x02, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x09,
                    0x00, 0x05, 0x00, 0x09, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00,
                ],
                19 => &[
                    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x02,
                    0x00, 0x07, 0x00, 0x40, 0x30, 0x20, 0x10, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03,
                    0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1E, 0x00, 0x00, 0x00, 0xFF,
                    0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x1F,
                    0x00, 0x00, 0x00, 0x05, 0x00, 0x0B, 0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00,
                ],
                20 => &[
                    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x02,
                    0x00, 0x07, 0x00, 0x40, 0x30, 0x20, 0x10, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03,
                    0x02, 0x01, 0x01, 0x00, 0x03, 0x00, 0x18, 0x00, 0x00, 0x00, 0x02, 0x00, 0x04,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD,
                ],
                21 => &[
                    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x15, 0x00, 0x00, 0x00, 0x02,
                    0x00, 0x07, 0x00, 0x40, 0x30, 0x20, 0x10, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03,
                    0x02, 0x01, 0x1F, 0x00, 0x00, 0x00, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12,
                    0x11,
                ],
                22 => &[
                    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x02,
                    0x00, 0x07, 0x00, 0x40, 0x30, 0x20, 0x10, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03,
                    0x02, 0x01, 0x02, 0x00, 0x00, 0x00,
                ],
                other => panic!("missing coordinated-drop golden for tag {other}"),
            };
            assert_eq!(
                bytes, expected,
                "immutable protocol-v1 golden for tag {tag}"
            );
            assert_eq!(bytes.get(8..12), Some(tag.to_le_bytes().as_slice()));
            assert_eq!(original.encoded_len(), bytes.len());

            let generic: Message = decode_value(&bytes).unwrap();
            let (manual, consumed) = decode_message(&bytes).unwrap();
            assert_eq!(generic, original, "generic decode for drop tag {tag}");
            assert_eq!(manual, original, "manual decode for drop tag {tag}");
            assert_eq!(consumed, bytes.len());
        }
    }

    #[test]
    fn coordinated_drop_decoder_rejects_invalid_typed_discriminants() {
        let report = Message {
            header: MessageHeader::new(1),
            body: drop_bodies().remove(1).1,
        };
        let mut report_bytes = encode(&report).unwrap();
        report_bytes[30..34].copy_from_slice(&3_u32.to_le_bytes());
        let report_error = decode_message(&report_bytes).unwrap_err();
        assert!(report_error
            .to_string()
            .contains("invalid drop report stage 3"));

        let abort = Message {
            header: MessageHeader::new(1),
            body: MessageBody::DropAbort(DropAbort {
                operation: drop_operation(),
                reason: DropAbortReason::Superseded,
            }),
        };
        let mut abort_bytes = encode(&abort).unwrap();
        abort_bytes[28..32].copy_from_slice(&7_u32.to_le_bytes());
        let abort_error = decode_message(&abort_bytes).unwrap_err();
        assert!(abort_error
            .to_string()
            .contains("invalid drop abort reason 7"));
    }

    #[test]
    fn coordinated_drop_report_stages_and_abort_reasons_roundtrip_exhaustively() {
        for stage in [
            DropReportStage::Inventory,
            DropReportStage::Ready,
            DropReportStage::Committed,
        ] {
            let message = Message {
                header: MessageHeader::new(1),
                body: MessageBody::DropReport(DropReport {
                    operation: drop_operation(),
                    participant: 1,
                    stage,
                    exposed_confirmed: Frame::new(2),
                    cut: Frame::new(3),
                    cut_digest: 4,
                    receipts: Vec::new(),
                }),
            };
            let bytes = encode(&message).unwrap();
            assert_eq!(decode_message(&bytes).unwrap().0, message);
        }

        for reason in [
            DropAbortReason::Superseded,
            DropAbortReason::MissingHistory,
            DropAbortReason::ConflictingHistory,
            DropAbortReason::ParticipantLost,
            DropAbortReason::Timeout,
            DropAbortReason::GenerationChanged,
            DropAbortReason::ResourceLimit,
        ] {
            let message = Message {
                header: MessageHeader::new(1),
                body: MessageBody::DropAbort(DropAbort {
                    operation: drop_operation(),
                    reason,
                }),
            };
            let bytes = encode(&message).unwrap();
            assert_eq!(decode_message(&bytes).unwrap().0, message);
        }
    }

    #[test]
    fn coordinated_drop_decoder_rejects_unrepresentable_vector_lengths_before_allocating() {
        let prepare = Message {
            header: MessageHeader::new(1),
            body: MessageBody::DropPrepare(DropPrepare {
                operation: drop_operation(),
                targets: Vec::new(),
                participants: Vec::new(),
            }),
        };
        let mut bytes = encode(&prepare).unwrap();
        bytes[28..36].copy_from_slice(&u64::MAX.to_le_bytes());
        let error = decode_message(&bytes).unwrap_err();
        assert!(error.to_string().contains("drop_prepare.targets"));
        assert!(
            error.to_string().contains("exceeds") || error.to_string().contains("overflows"),
            "unexpected length-bound error: {error}"
        );
    }

    /// A `ConnectionStatus` with arbitrary field values (used by the wire-size
    /// property strategies for both `Input` and `StateSnapshot`).
    fn arb_connection_status() -> impl proptest::strategy::Strategy<Value = ConnectionStatus> {
        use proptest::prelude::*;
        (any::<bool>(), any::<i32>(), any::<u16>()).prop_map(|(disconnected, frame, epoch)| {
            ConnectionStatus {
                disconnected,
                last_frame: Frame::new(frame),
                epoch,
            }
        })
    }

    /// A strategy producing an arbitrary [`Message`] of any body variant with
    /// arbitrary field values and (bounded) collection lengths, covering the
    /// hot-join variants when the feature is enabled.
    fn arb_message() -> impl proptest::strategy::Strategy<Value = Message> {
        use proptest::collection::vec as pvec;
        use proptest::prelude::*;
        use proptest::strategy::{BoxedStrategy, Union};

        // `bodies` is only pushed to under the hot-join feature; without it the
        // `vec!` literal is the complete set.
        #[cfg_attr(not(feature = "hot-join"), allow(unused_mut))]
        let mut bodies: Vec<BoxedStrategy<MessageBody>> = vec![
            (
                any::<u32>(),
                any::<u8>(),
                any::<u32>(),
                any::<u16>(),
                any::<u16>(),
                any::<u32>(),
                any::<u16>(),
                any::<u32>(),
                any::<u64>(),
            )
                .prop_map(
                    |(
                        random_request,
                        min_compat_version,
                        features,
                        num_players,
                        input_bytes_per_player,
                        fps,
                        max_prediction,
                        desync_interval,
                        config_digest,
                    )| {
                        MessageBody::SyncRequest(SyncRequest {
                            random_request,
                            min_compat_version,
                            features,
                            config: SessionConfigBlock {
                                num_players,
                                input_bytes_per_player,
                                fps,
                                max_prediction,
                                desync_interval,
                            },
                            config_digest,
                        })
                    },
                )
                .boxed(),
            (
                any::<u32>(),
                any::<u8>(),
                any::<u32>(),
                any::<u16>(),
                any::<u16>(),
                any::<u32>(),
                any::<u16>(),
                any::<u32>(),
                any::<u64>(),
            )
                .prop_map(
                    |(
                        random_reply,
                        min_compat_version,
                        features,
                        num_players,
                        input_bytes_per_player,
                        fps,
                        max_prediction,
                        desync_interval,
                        config_digest,
                    )| {
                        MessageBody::SyncReply(SyncReply {
                            random_reply,
                            min_compat_version,
                            features,
                            config: SessionConfigBlock {
                                num_players,
                                input_bytes_per_player,
                                fps,
                                max_prediction,
                                desync_interval,
                            },
                            config_digest,
                        })
                    },
                )
                .boxed(),
            (
                pvec(arb_connection_status(), 0..8),
                any::<i32>(),
                any::<i32>(),
                pvec(any::<u8>(), 0..64),
            )
                .prop_map(|(peer_connect_status, start, ack, bytes)| {
                    MessageBody::Input(Input {
                        peer_connect_status,
                        start_frame: Frame::new(start),
                        ack_frame: Frame::new(ack),
                        bytes,
                    })
                })
                .boxed(),
            any::<i32>()
                .prop_map(|f| {
                    MessageBody::InputAck(InputAck {
                        ack_frame: Frame::new(f),
                    })
                })
                .boxed(),
            (any::<i16>(), any::<u128>())
                .prop_map(|(frame_advantage, ping)| {
                    MessageBody::QualityReport(QualityReport {
                        frame_advantage,
                        ping,
                    })
                })
                .boxed(),
            any::<u128>()
                .prop_map(|pong| MessageBody::QualityReply(QualityReply { pong }))
                .boxed(),
            (any::<u128>(), any::<i32>())
                .prop_map(|(checksum, f)| {
                    MessageBody::ChecksumReport(ChecksumReport {
                        checksum,
                        frame: Frame::new(f),
                    })
                })
                .boxed(),
            Just(MessageBody::KeepAlive).boxed(),
            any::<u32>()
                .prop_map(|round_seq| MessageBody::FloorRequest(FloorRequest { round_seq }))
                .boxed(),
            (any::<u32>(), pvec(any::<i32>().prop_map(Frame::new), 0..8))
                .prop_map(|(round_seq, floors)| {
                    MessageBody::FloorReply(FloorReply { round_seq, floors })
                })
                .boxed(),
            any::<u8>()
                .prop_map(|reason| MessageBody::Goodbye(Goodbye { reason }))
                .boxed(),
            (
                pvec(
                    (any::<u16>(), any::<u16>())
                        .prop_map(|(handle, generation)| DropTarget { handle, generation }),
                    0..4,
                ),
                pvec(any::<u16>(), 0..4),
            )
                .prop_map(|(targets, participants)| {
                    MessageBody::DropPrepare(DropPrepare {
                        operation: drop_operation(),
                        targets,
                        participants,
                    })
                })
                .boxed(),
            (
                any::<u16>(),
                0_u8..3,
                any::<i32>(),
                any::<i32>(),
                any::<u64>(),
                pvec(
                    (any::<u16>(), any::<i32>(), any::<i32>()).prop_map(
                        |(target, available, through)| DropReceipt {
                            target,
                            available_from: Frame::new(available),
                            contiguous_through: Frame::new(through),
                        },
                    ),
                    0..4,
                ),
            )
                .prop_map(|(participant, stage, exposed, cut, cut_digest, receipts)| {
                    let stage = match stage {
                        0 => DropReportStage::Inventory,
                        1 => DropReportStage::Ready,
                        _ => DropReportStage::Committed,
                    };
                    MessageBody::DropReport(DropReport {
                        operation: drop_operation(),
                        participant,
                        stage,
                        exposed_confirmed: Frame::new(exposed),
                        cut: Frame::new(cut),
                        cut_digest,
                        receipts,
                    })
                })
                .boxed(),
            (
                any::<u16>(),
                any::<u16>(),
                any::<i32>(),
                any::<u16>(),
                pvec(any::<u8>(), 0..64),
            )
                .prop_map(|(chunk_index, chunk_count, start, frame_count, bytes)| {
                    MessageBody::DropBackfill(DropBackfill {
                        operation: drop_operation(),
                        chunk_index,
                        chunk_count,
                        start_frame: Frame::new(start),
                        frame_count,
                        bytes,
                    })
                })
                .boxed(),
            (any::<i32>(), any::<u64>())
                .prop_map(|(cut, cut_digest)| {
                    MessageBody::DropCommit(DropCommit {
                        operation: drop_operation(),
                        cut: Frame::new(cut),
                        cut_digest,
                    })
                })
                .boxed(),
            (0_u8..7)
                .prop_map(|reason| {
                    let reason = match reason {
                        0 => DropAbortReason::Superseded,
                        1 => DropAbortReason::MissingHistory,
                        2 => DropAbortReason::ConflictingHistory,
                        3 => DropAbortReason::ParticipantLost,
                        4 => DropAbortReason::Timeout,
                        5 => DropAbortReason::GenerationChanged,
                        _ => DropAbortReason::ResourceLimit,
                    };
                    MessageBody::DropAbort(DropAbort {
                        operation: drop_operation(),
                        reason,
                    })
                })
                .boxed(),
        ];

        #[cfg(feature = "hot-join")]
        {
            use crate::network::messages::{
                JoinAborted, JoinCommitted, JoinRequest, ReactivateSlot, ReactivateSlotAck,
                StateSnapshot, StateSnapshotAck,
            };
            bodies.push(
                any::<usize>()
                    .prop_map(|player_handle| {
                        MessageBody::JoinRequest(JoinRequest { player_handle })
                    })
                    .boxed(),
            );
            bodies.push(
                (
                    any::<i32>(),
                    any::<usize>(),
                    pvec(any::<u8>(), 0..64),
                    pvec(any::<u8>(), 0..32),
                    pvec(arb_connection_status(), 0..8),
                    proptest::option::of(any::<u128>()),
                )
                    .prop_map(
                        |(
                            f,
                            num_players,
                            state_bytes,
                            bridge_inputs,
                            bridge_statuses,
                            checksum,
                        )| {
                            MessageBody::StateSnapshot(StateSnapshot {
                                frame: Frame::new(f),
                                num_players,
                                state_bytes,
                                bridge_inputs,
                                bridge_statuses,
                                checksum,
                            })
                        },
                    )
                    .boxed(),
            );
            bodies.push(
                any::<i32>()
                    .prop_map(|f| {
                        MessageBody::StateSnapshotAck(StateSnapshotAck {
                            frame: Frame::new(f),
                        })
                    })
                    .boxed(),
            );
            bodies.push(
                (any::<usize>(), any::<i32>())
                    .prop_map(|(handle, f)| {
                        MessageBody::ReactivateSlot(ReactivateSlot {
                            handle,
                            frame: Frame::new(f),
                        })
                    })
                    .boxed(),
            );
            bodies.push(
                (any::<usize>(), any::<i32>())
                    .prop_map(|(handle, f)| {
                        MessageBody::ReactivateSlotAck(ReactivateSlotAck {
                            handle,
                            frame: Frame::new(f),
                        })
                    })
                    .boxed(),
            );
            bodies.push(
                (any::<usize>(), any::<i32>())
                    .prop_map(|(handle, f)| {
                        MessageBody::JoinCommitted(JoinCommitted {
                            handle,
                            frame: Frame::new(f),
                        })
                    })
                    .boxed(),
            );
            bodies.push(
                (any::<usize>(), any::<i32>())
                    .prop_map(|(handle, f)| {
                        MessageBody::JoinAborted(JoinAborted {
                            handle,
                            frame: Frame::new(f),
                        })
                    })
                    .boxed(),
            );
        }

        (
            any::<u32>().prop_filter("valid connection ID", |id| {
                super::super::is_valid_conn_id(*id)
            }),
            Union::new(bodies),
        )
            .prop_map(|(conn_id, body)| Message {
                header: MessageHeader::new(conn_id),
                body,
            })
    }

    proptest::proptest! {
        /// The D1 regression artifact: `Message::encoded_len` is arithmetic and
        /// alloc-free, so it can silently drift from the real wire format if a
        /// field width or the codec configuration ever changes. This asserts it
        /// equals the exact serialized byte count for arbitrary messages of every
        /// variant — the property that makes it a trustworthy bandwidth meter.
        #[test]
        fn encoded_len_matches_exact_wire_bytes(msg in arb_message()) {
            let encoded = encode(&msg).expect("arbitrary message must encode");
            proptest::prop_assert_eq!(
                msg.encoded_len(),
                encoded.len(),
                "encoded_len diverged from wire bytes for {:?}",
                msg
            );
        }

        /// Stream framing is an envelope only: it must preserve the exact
        /// protocol-v1 bytes for every body variant.
        #[cfg_attr(miri, ignore)] // arbitrary-message proptest takes ~8 minutes on Windows Miri
        #[test]
        fn encode_framed_wraps_exact_arbitrary_message_bytes(msg in arb_message()) {
            let payload = encode(&msg).expect("arbitrary message must encode");
            let framed = encode_framed(&msg).expect("bounded arbitrary message must frame");
            let prefix = u32::try_from(payload.len())
                .expect("property payload is tiny")
                .to_le_bytes();

            proptest::prop_assert_eq!(framed.get(..4), Some(prefix.as_slice()));
            proptest::prop_assert_eq!(framed.get(4..), Some(payload.as_slice()));
        }

        /// Arbitrary fragmentation of both the length prefix and payload cannot
        /// change the decoded message or byte-consumption accounting.
        #[cfg_attr(miri, ignore)] // proptest takes ~2 minutes on Windows Miri; every split is unit-tested
        #[test]
        fn frame_decoder_roundtrips_keep_alive_at_arbitrary_split(
            conn_low in 1_u32..=u32::from(u16::MAX),
            split_seed in proptest::prelude::any::<usize>(),
        ) {
            let message = keep_alive(conn_low);
            let framed = encode_framed(&message).expect("keep-alive must frame");
            let split = split_seed % (framed.len() + 1);
            let mut decoder = FrameDecoder::new();

            let (first, first_consumed) = decoder
                .push(&framed[..split])
                .expect("first fragment must decode or buffer");
            let (second, second_consumed) = decoder
                .push(&framed[split..])
                .expect("second fragment must complete or be empty");

            proptest::prop_assert_eq!(first_consumed, split);
            proptest::prop_assert_eq!(second_consumed, framed.len() - split);
            proptest::prop_assert_eq!(first.or(second), Some(message));
            proptest::prop_assert_eq!(decoder.finish(), Ok(()));
        }

        /// A single read may contain many complete frames, but each `push`
        /// yields exactly one and reports the suffix boundary without buffering
        /// an unbounded decoded-message queue.
        #[cfg_attr(miri, ignore)] // proptest takes ~4 minutes on Windows Miri; deterministic coverage remains
        #[test]
        fn frame_decoder_preserves_arbitrary_concatenated_frame_order(
            conn_lows in proptest::collection::vec(1_u32..=u32::from(u16::MAX), 0..32),
        ) {
            let messages: Vec<_> = conn_lows.into_iter().map(keep_alive).collect();
            let mut stream = Vec::new();
            for message in &messages {
                stream.extend_from_slice(
                    &encode_framed(message).expect("keep-alive must frame")
                );
            }
            let mut decoder = FrameDecoder::new();

            let decoded = drain_framed(&mut decoder, &stream)
                .expect("concatenated valid frames must decode");

            proptest::prop_assert_eq!(decoded, messages);
            proptest::prop_assert_eq!(decoder.finish(), Ok(()));
        }
    }

    /// Documents why D1 was a real defect: the old accounting charged
    /// `std::mem::size_of_val(&msg)` — the constant in-memory `Message` size,
    /// which is identical for a bare `KeepAlive` and a fully-loaded `Input`
    /// because `Vec` payloads live on the heap. Wire size differs by hundreds of
    /// bytes; `encoded_len` reports the truth.
    #[test]
    fn size_of_val_is_constant_while_wire_size_is_not_d1() {
        let tiny = Message {
            header: MessageHeader::new(0),
            body: MessageBody::KeepAlive,
        };
        let heavy = Message {
            header: MessageHeader::new(0),
            body: MessageBody::Input(Input {
                peer_connect_status: vec![ConnectionStatus::default(); 16],
                start_frame: Frame::new(10_000),
                ack_frame: Frame::new(9_999),
                bytes: vec![0xAB; 128],
            }),
        };

        // The old metric: identical for both, regardless of payload.
        assert_eq!(
            std::mem::size_of_val(&tiny),
            std::mem::size_of_val(&heavy),
            "in-memory size is payload-independent — the D1 fiction"
        );

        // The truth: wire footprints differ by hundreds of bytes, and
        // `encoded_len` matches the serialized length exactly for each.
        assert_eq!(tiny.encoded_len(), encode(&tiny).unwrap().len());
        assert_eq!(heavy.encoded_len(), encode(&heavy).unwrap().len());
        assert!(
            heavy.encoded_len() > tiny.encoded_len() + 200,
            "loaded Input must dwarf KeepAlive on the wire ({} vs {})",
            heavy.encoded_len(),
            tiny.encoded_len()
        );
    }

    #[test]
    fn decode_message_roundtrips_input_without_generic_vec_decode() {
        let original = Message {
            header: MessageHeader::new(0xABCD),
            body: MessageBody::Input(Input {
                peer_connect_status: vec![
                    ConnectionStatus {
                        disconnected: false,
                        last_frame: Frame::new(10),
                        epoch: 0,
                    },
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: Frame::new(20),
                        epoch: 0,
                    },
                ],
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
        let mut bytes = wire_prefix(0xABCD, 2);
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // peer_connect_status len

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    #[test]
    fn decode_message_rejects_input_bytes_length_that_exceeds_packet_bytes() {
        let mut bytes = wire_prefix(0xABCD, 2);
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // peer_connect_status len
        bytes.extend_from_slice(&100_i32.to_le_bytes()); // start_frame
        bytes.extend_from_slice(&50_i32.to_le_bytes()); // ack_frame
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // input.bytes len

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    #[test]
    fn decode_message_rejects_negative_connection_status_frame() {
        let message = Message {
            header: MessageHeader::new(0xABCD),
            body: MessageBody::Input(Input {
                peer_connect_status: vec![ConnectionStatus::default()],
                ..Input::default()
            }),
        };
        let mut bytes = encode(&message).unwrap();
        // header (8) + variant (4) + status length (8) + disconnected (1)
        bytes[21..25].copy_from_slice(&(-2_i32).to_le_bytes());

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    #[test]
    fn decode_message_allows_null_connection_status_and_floor_frames() {
        let messages = [
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::Input(Input {
                    peer_connect_status: vec![ConnectionStatus::default()],
                    ..Input::default()
                }),
            },
            Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::FloorReply(FloorReply {
                    round_seq: 1,
                    floors: vec![Frame::NULL],
                }),
            },
        ];

        for message in messages {
            let bytes = encode(&message).unwrap();
            let result = decode_message(&bytes);
            assert_eq!(result, Ok((message, bytes.len())));
        }
    }

    #[test]
    fn decode_message_rejects_negative_floor_reply_frame() {
        let message = Message {
            header: MessageHeader::new(0xABCD),
            body: MessageBody::FloorReply(FloorReply {
                round_seq: 1,
                floors: vec![Frame::new(5)],
            }),
        };
        let mut bytes = encode(&message).unwrap();
        // header (8) + variant (4) + round sequence (4) + floors length (8)
        bytes[24..28].copy_from_slice(&(-2_i32).to_le_bytes());

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    #[test]
    fn decode_message_rejects_negative_checksum_frame() {
        for frame in [Frame::NULL, Frame::new(-2)] {
            let message = Message {
                header: MessageHeader::new(0xABCD),
                body: MessageBody::ChecksumReport(ChecksumReport { checksum: 7, frame }),
            };
            let bytes = encode(&message).unwrap();

            let result = decode_message(&bytes);

            assert!(matches!(result, Err(CodecError::DecodeError { .. })));
        }
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
            header: MessageHeader::new(0xABCD),
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
            header: MessageHeader::new(0x1234),
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
            header: MessageHeader::new(0x1234),
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

    fn wire_prefix(conn_id: u32, variant: u32) -> Vec<u8> {
        let mut bytes = encode(&MessageHeader::new(conn_id)).unwrap();
        bytes.extend_from_slice(&variant.to_le_bytes());
        bytes
    }

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
            header: MessageHeader::new(0xABCD),
            body: MessageBody::JoinRequest(JoinRequest { player_handle: 3 }),
        });
    }

    #[test]
    fn decode_message_roundtrips_state_snapshot_with_checksum() {
        roundtrip(Message {
            header: MessageHeader::new(0xABCD),
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
            header: MessageHeader::new(0xABCD),
            body: MessageBody::StateSnapshot(StateSnapshot {
                frame: Frame::new(42),
                num_players: 3,
                state_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
                bridge_inputs: vec![0x11, 0x22, 0x33],
                bridge_statuses: vec![
                    ConnectionStatus {
                        disconnected: false,
                        last_frame: Frame::new(42),
                        epoch: 0,
                    },
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: Frame::new(17),
                        epoch: 0,
                    },
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: Frame::NULL,
                        epoch: 0,
                    },
                ],
                checksum: Some(0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10),
            }),
        });
    }

    #[test]
    fn decode_message_roundtrips_state_snapshot_without_checksum() {
        roundtrip(Message {
            header: MessageHeader::new(0xABCD),
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
            header: MessageHeader::new(0xABCD),
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
            header: MessageHeader::new(0xABCD),
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
        let mut bytes = wire_prefix(0xABCD, 11);
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
        let mut bytes = wire_prefix(0xABCD, 11);
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
        let mut bytes = wire_prefix(0xABCD, 11);
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
        let mut bytes = wire_prefix(0xABCD, 11);
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
        let mut bytes = wire_prefix(0xABCD, 11);
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
        let mut bytes = wire_prefix(0xABCD, 11);
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
        let mut bytes = wire_prefix(0xABCD, 11);
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // state_bytes len = 0
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // bridge_inputs len = 0
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // bridge_statuses len (absurd)

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A `bridge_statuses` length that fits in `usize` but whose minimum wire
    /// footprint (`CONNECTION_STATUS_WIRE_LEN` = 7 bytes per status) exceeds the
    /// remaining bytes must also be rejected before reserve.
    #[test]
    fn decode_message_rejects_bridge_statuses_length_larger_than_remaining() {
        let mut bytes = wire_prefix(0xABCD, 11);
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
        let mut bytes = wire_prefix(0xABCD, 11);
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // state_bytes len = 0
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // bridge_inputs len = 0
                                                       // bridge_statuses length prefix omitted entirely.

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A snapshot buffer truncated mid-`bridge_statuses` payload (the prefix
    /// claims one status but fewer than its `CONNECTION_STATUS_WIRE_LEN` = 7
    /// record bytes are present) must be rejected, never read out of bounds.
    /// Because the record is fixed-width, this is caught by the pre-reserve
    /// length bound (`1 * 7 > remaining`), before any element is decoded.
    #[test]
    fn decode_message_rejects_snapshot_truncated_inside_bridge_statuses() {
        let mut bytes = wire_prefix(0xABCD, 11);
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // frame
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // state_bytes len = 0
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // bridge_inputs len = 0
        bytes.extend_from_slice(&1_u64.to_le_bytes()); // bridge_statuses len = 1
        bytes.extend_from_slice(&[0, 0xAA, 0xBB]); // 3 of the 7 status bytes

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A valid snapshot buffer with extra trailing bytes must be rejected.
    #[test]
    fn decode_message_rejects_trailing_bytes_after_snapshot() {
        let original = Message {
            header: MessageHeader::new(0xABCD),
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
            header: MessageHeader::new(0xABCD),
            body: MessageBody::ReactivateSlot(ReactivateSlot {
                handle: 2,
                frame: Frame::new(42),
            }),
        });
    }

    #[test]
    fn decode_message_roundtrips_reactivate_slot_ack() {
        roundtrip(Message {
            header: MessageHeader::new(0xABCD),
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
        let mut bytes = wire_prefix(0xABCD, 13);
        bytes.extend_from_slice(&3_u64.to_le_bytes()); // handle
                                                       // frame (i32) omitted entirely.

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A `ReactivateSlotAck` buffer truncated mid-`frame` must likewise be rejected.
    #[test]
    fn decode_message_rejects_truncated_reactivate_slot_ack() {
        let mut bytes = wire_prefix(0xABCD, 14);
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
            header: MessageHeader::new(0xABCD),
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
            header: MessageHeader::new(0xABCD),
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
            header: MessageHeader::new(0xABCD),
            body: MessageBody::JoinCommitted(JoinCommitted {
                handle: 2,
                frame: Frame::new(42),
            }),
        });
    }

    #[test]
    fn decode_message_roundtrips_join_aborted() {
        roundtrip(Message {
            header: MessageHeader::new(0xABCD),
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
        let mut bytes = wire_prefix(0xABCD, 15);
        bytes.extend_from_slice(&3_u64.to_le_bytes()); // handle
                                                       // frame (i32) omitted entirely.

        let result = decode_message(&bytes);

        assert!(matches!(result, Err(CodecError::DecodeError { .. })));
    }

    /// A `JoinAborted` buffer truncated mid-`frame` must likewise be rejected.
    #[test]
    fn decode_message_rejects_truncated_join_aborted() {
        let mut bytes = wire_prefix(0xABCD, 16);
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
            header: MessageHeader::new(0xABCD),
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
            header: MessageHeader::new(0xABCD),
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
