//! Error types for Fortress Rollback.
//!
//! This module provides structured error types for the rollback networking library.
//! The error types are designed to be:
//!
//! - **Zero-allocation on hot paths**: Errors store numeric data directly instead
//!   of formatting strings, enabling allocation-free error construction.
//! - **Programmatically inspectable**: Using enums and structured fields instead
//!   of string messages allows callers to match on specific error cases.
//! - **Self-documenting**: Each error variant and field is documented.
//!
//! # Error Type Design
//!
//! ## Dual Variant Pattern
//!
//! Some error variants exist in both unstructured (legacy) and structured forms:
//!
//! - `InvalidFrame` (unstructured) vs `InvalidFrameStructured` (structured)
//! - `InternalError` (unstructured) vs `InternalErrorStructured` (structured)
//!
//! This pattern exists for backward compatibility. The unstructured variants
//! accept string messages and are used by legacy code. The structured variants
//! use zero-allocation enums and provide better error inspection.
//!
//! **Migration path:** New code should use structured variants. Unstructured
//! variants are deprecated but retained for API stability. A future major version
//! may remove the unstructured variants.
//!
//! ## Structured Error Types
//!
//! - [`IndexOutOfBounds`]: Index/bounds error with collection name and indices.
//! - [`InvalidFrameReason`]: Why a frame was invalid (null, negative, out of window).
//! - [`InternalErrorKind`]: Specific internal error types with structured context.
//! - [`RleDecodeReason`]: Why RLE decoding failed.
//!
//! ## Module-Specific Error Types
//!
//! Other modules provide their own structured error types:
//!
//! - [`crate::network::compression::CompressionError`]: RLE and delta decode errors.
//! - [`crate::network::codec::CodecError`]: Serialization/deserialization errors.
//! - [`crate::checksum::ChecksumError`]: Checksum computation errors.
//!
//! # Usage Examples
//!
//! ## Creating structured errors (preferred)
//!
//! ```
//! use fortress_rollback::{FortressError, InternalErrorKind, IndexOutOfBounds};
//!
//! // Create a structured index out of bounds error
//! let error = FortressError::InternalErrorStructured {
//!     kind: InternalErrorKind::IndexOutOfBounds(IndexOutOfBounds {
//!         name: "inputs",
//!         index: 10,
//!         length: 5,
//!     }),
//! };
//! ```
//!
//! ## Matching on error variants
//!
//! ```
//! use fortress_rollback::{FortressError, InvalidFrameReason};
//!
//! fn handle_error(err: FortressError) {
//!     match err {
//!         FortressError::InvalidFrameStructured { frame, reason } => {
//!             match reason {
//!                 InvalidFrameReason::NullFrame => {
//!                     println!("Frame {} is NULL", frame.as_i32());
//!                 }
//!                 InvalidFrameReason::OutsidePredictionWindow { max_prediction, .. } => {
//!                     println!("Frame {} outside {} frame prediction window",
//!                              frame.as_i32(), max_prediction);
//!                 }
//!                 _ => {}
//!             }
//!         }
//!         _ => {}
//!     }
//! }
//! ```

use std::error::Error;
use std::fmt;
use std::fmt::Display;

use crate::{Frame, PlayerHandle};

// =============================================================================
// Structured Error Types for Hot Path
// =============================================================================
// These types store debugging data as fields (cheap - no allocation) and format
// lazily in Display impl (only when error is displayed - cold path).

/// Represents an index out of bounds error with full context.
///
/// This structured type stores all debugging information without allocation,
/// and formats the message lazily in the `Display` implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IndexOutOfBounds {
    /// The name of the collection that was accessed.
    pub name: &'static str,
    /// The index that was attempted.
    pub index: usize,
    /// The length of the collection.
    pub length: usize,
}

impl Display for IndexOutOfBounds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} index {} out of bounds (length: {})",
            self.name, self.index, self.length
        )
    }
}

/// Represents why a frame was invalid.
///
/// Using an enum instead of String allows for zero-allocation error construction
/// on hot paths while still providing detailed error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InvalidFrameReason {
    /// Frame is NULL_FRAME (-1).
    NullFrame,
    /// Frame is negative (other than NULL_FRAME).
    Negative,
    /// Frame must be non-negative.
    MustBeNonNegative,
    /// Frame is not in the past (must load a frame before current).
    NotInPast {
        /// The current frame.
        current_frame: Frame,
    },
    /// Frame is outside the prediction window.
    OutsidePredictionWindow {
        /// The current frame.
        current_frame: Frame,
        /// The maximum prediction depth.
        max_prediction: usize,
    },
    /// The saved state for this frame has the wrong frame number.
    WrongSavedFrame {
        /// The frame number in the saved state.
        saved_frame: Frame,
    },
    /// Frame is not confirmed yet.
    NotConfirmed {
        /// The highest confirmed frame.
        confirmed_frame: Frame,
    },
    /// Frame is NULL or negative (general validation).
    NullOrNegative,
    /// Custom reason (fallback for API compatibility).
    Custom(&'static str),
}

impl Display for InvalidFrameReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullFrame => write!(f, "cannot load NULL_FRAME"),
            Self::Negative => write!(f, "frame is negative"),
            Self::MustBeNonNegative => write!(f, "frame must be non-negative"),
            Self::NotInPast { current_frame } => {
                write!(
                    f,
                    "must load frame in the past (current: {})",
                    current_frame
                )
            },
            Self::OutsidePredictionWindow {
                current_frame,
                max_prediction,
            } => {
                write!(
                    f,
                    "cannot load frame outside of prediction window (current: {}, max_prediction: {})",
                    current_frame, max_prediction
                )
            },
            Self::WrongSavedFrame { saved_frame } => {
                write!(f, "saved state has wrong frame (found: {})", saved_frame)
            },
            Self::NotConfirmed { confirmed_frame } => {
                write!(
                    f,
                    "frame is not confirmed yet (confirmed_frame: {})",
                    confirmed_frame
                )
            },
            Self::NullOrNegative => write!(f, "frame is NULL or negative"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Represents why an RLE decode operation failed.
///
/// Using an enum instead of String allows for zero-allocation error construction
/// on hot paths while still providing detailed error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RleDecodeReason {
    /// The bitfield index was out of bounds during decode.
    BitfieldIndexOutOfBounds,
    /// The destination slice was out of bounds during decode.
    DestinationSliceOutOfBounds,
    /// The source slice was out of bounds during decode.
    SourceSliceOutOfBounds,
    /// The encoded data was truncated (offset exceeded buffer length).
    TruncatedData {
        /// The offset that was reached.
        offset: usize,
        /// The buffer length.
        buffer_len: usize,
    },
}

impl Display for RleDecodeReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BitfieldIndexOutOfBounds => {
                write!(f, "bitfield index out of bounds")
            },
            Self::DestinationSliceOutOfBounds => {
                write!(f, "destination slice out of bounds")
            },
            Self::SourceSliceOutOfBounds => {
                write!(f, "source slice out of bounds")
            },
            Self::TruncatedData { offset, buffer_len } => {
                write!(
                    f,
                    "truncated data: offset {} exceeds buffer length {}",
                    offset, buffer_len
                )
            },
        }
    }
}

/// Represents why a delta decode operation failed.
///
/// Using an enum instead of String allows for zero-allocation error construction
/// and programmatic error inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeltaDecodeReason {
    /// The reference bytes were empty.
    EmptyReference,
    /// The data length is not a multiple of the reference length.
    DataLengthMismatch {
        /// The length of the data buffer.
        data_len: usize,
        /// The length of the reference buffer.
        reference_len: usize,
    },
    /// The reference bytes index was out of bounds.
    ReferenceIndexOutOfBounds {
        /// The index that was out of bounds.
        index: usize,
        /// The length of the reference buffer.
        length: usize,
    },
    /// The data index was out of bounds.
    DataIndexOutOfBounds {
        /// The index that was out of bounds.
        index: usize,
        /// The length of the data buffer.
        length: usize,
    },
}

impl Display for DeltaDecodeReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyReference => write!(f, "reference bytes is empty"),
            Self::DataLengthMismatch {
                data_len,
                reference_len,
            } => {
                write!(
                    f,
                    "data length {} is not a multiple of reference length {}",
                    data_len, reference_len
                )
            },
            Self::ReferenceIndexOutOfBounds { index, length } => {
                write!(
                    f,
                    "reference bytes index {} out of bounds (length: {})",
                    index, length
                )
            },
            Self::DataIndexOutOfBounds { index, length } => {
                write!(f, "data index {} out of bounds (length: {})", index, length)
            },
        }
    }
}

/// Specific internal error kinds with structured data.
///
/// Using an enum instead of String allows for zero-allocation error construction
/// on hot paths while preserving full debugging context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InternalErrorKind {
    /// An index was out of bounds.
    IndexOutOfBounds(IndexOutOfBounds),
    /// Failed to get synchronized inputs.
    SynchronizedInputsFailed {
        /// The frame at which inputs were requested.
        frame: Frame,
    },
    /// Player inputs vector is empty.
    EmptyPlayerInputs,
    /// Buffer index out of bounds (generic).
    BufferIndexOutOfBounds,
    /// Player handle not found when checking disconnect status.
    DisconnectStatusNotFound {
        /// The player handle that was not found.
        player_handle: PlayerHandle,
    },
    /// Endpoint not found for a registered remote player.
    EndpointNotFoundForRemote {
        /// The player handle for which the endpoint was not found.
        player_handle: PlayerHandle,
    },
    /// Endpoint not found for a registered spectator.
    EndpointNotFoundForSpectator {
        /// The player handle for which the endpoint was not found.
        player_handle: PlayerHandle,
    },
    /// Connection status index out of bounds when updating local connection status.
    ConnectionStatusIndexOutOfBounds {
        /// The player handle that was out of bounds.
        player_handle: PlayerHandle,
    },
    /// RLE decode operation failed.
    RleDecodeError {
        /// The specific reason for the RLE decode failure.
        reason: RleDecodeReason,
    },
    /// Delta decode operation failed.
    DeltaDecodeError {
        /// The specific reason for the delta decode failure.
        reason: DeltaDecodeReason,
    },
    /// Custom error (fallback for API compatibility).
    Custom(&'static str),
}

impl Display for InternalErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndexOutOfBounds(iob) => write!(f, "{}", iob),
            Self::SynchronizedInputsFailed { frame } => {
                write!(f, "failed to get synchronized inputs for frame {}", frame)
            },
            Self::EmptyPlayerInputs => write!(f, "player inputs vector is empty"),
            Self::BufferIndexOutOfBounds => write!(f, "buffer index out of bounds"),
            Self::DisconnectStatusNotFound { player_handle } => {
                write!(
                    f,
                    "disconnect status not found for player handle {}",
                    player_handle
                )
            },
            Self::EndpointNotFoundForRemote { player_handle } => {
                write!(
                    f,
                    "endpoint not found for registered remote player {}",
                    player_handle
                )
            },
            Self::EndpointNotFoundForSpectator { player_handle } => {
                write!(
                    f,
                    "endpoint not found for registered spectator {}",
                    player_handle
                )
            },
            Self::ConnectionStatusIndexOutOfBounds { player_handle } => {
                write!(
                    f,
                    "connection status index out of bounds for player handle {}",
                    player_handle
                )
            },
            Self::RleDecodeError { reason } => {
                write!(f, "RLE decode failed: {}", reason)
            },
            Self::DeltaDecodeError { reason } => {
                write!(f, "delta decode failed: {}", reason)
            },
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Represents why a request was invalid.
///
/// Using an enum instead of String allows for zero-allocation error construction
/// and programmatic error inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InvalidRequestKind {
    // Player handle errors
    /// The player handle is already in use.
    PlayerHandleInUse {
        /// The handle that is already in use.
        handle: PlayerHandle,
    },
    /// The player handle does not refer to a local player.
    NotLocalPlayer {
        /// The handle that is not a local player.
        handle: PlayerHandle,
    },
    /// The player handle does not refer to a remote player or spectator.
    NotRemotePlayerOrSpectator {
        /// The handle that is not a remote player or spectator.
        handle: PlayerHandle,
    },
    /// Invalid handle for a local player.
    InvalidLocalPlayerHandle {
        /// The invalid player handle.
        handle: PlayerHandle,
        /// The number of players in the session.
        num_players: usize,
    },
    /// Invalid handle for a remote player.
    InvalidRemotePlayerHandle {
        /// The invalid player handle.
        handle: PlayerHandle,
        /// The number of players in the session.
        num_players: usize,
    },
    /// Invalid handle for a spectator.
    InvalidSpectatorHandle {
        /// The invalid player handle.
        handle: PlayerHandle,
        /// The number of players in the session.
        num_players: usize,
    },

    // Input errors
    /// Missing local input for one or more players.
    MissingLocalInput,
    /// No confirmed input available for the requested frame.
    NoConfirmedInput {
        /// The frame for which no confirmed input was available.
        frame: Frame,
    },

    // Configuration errors
    /// A configuration value is outside the allowed range.
    ConfigValueOutOfRange {
        /// The name of the configuration field.
        field: &'static str,
        /// The minimum allowed value.
        min: u64,
        /// The maximum allowed value.
        max: u64,
        /// The actual value that was provided.
        actual: u64,
    },
    /// A Duration configuration value is outside the allowed range.
    DurationConfigOutOfRange {
        /// The name of the configuration field.
        field: &'static str,
        /// The minimum allowed value in milliseconds.
        min_ms: u64,
        /// The maximum allowed value in milliseconds.
        max_ms: u64,
        /// The actual value provided in milliseconds.
        actual_ms: u64,
    },
    /// Frame delay exceeds the maximum allowed for the queue length.
    FrameDelayTooLarge {
        /// The requested delay.
        delay: usize,
        /// The maximum allowed delay.
        max_delay: usize,
    },
    /// Input delay exceeds the maximum allowed for the given FPS.
    InputDelayTooLarge {
        /// The requested input delay in frames.
        delay_frames: usize,
        /// The frames per second (for computing actual delay in seconds).
        fps: usize,
        /// The maximum allowed seconds (e.g., 10 for "10 seconds").
        max_seconds_limit: usize,
    },
    /// Input queue length is too small (minimum is 2).
    QueueLengthTooSmall {
        /// The requested length.
        length: usize,
    },
    /// Event queue size is too small (minimum is 10).
    EventQueueSizeTooSmall {
        /// The requested size.
        size: usize,
    },

    // Session building errors
    /// Number of players must be greater than 0.
    ZeroPlayers,
    /// FPS must be greater than 0.
    ZeroFps,
    /// Not enough players have been registered.
    NotEnoughPlayers {
        /// The expected number of players.
        expected: usize,
        /// The actual number of players registered.
        actual: usize,
    },
    /// Check distance is too large for the prediction window.
    CheckDistanceTooLarge {
        /// The requested check distance.
        check_dist: usize,
        /// The maximum prediction window.
        max_prediction: usize,
    },
    /// Max frames behind is invalid.
    MaxFramesBehindInvalid {
        /// The requested value.
        value: usize,
        /// The buffer size.
        buffer_size: usize,
    },
    /// Catchup speed is invalid.
    CatchupSpeedInvalid {
        /// The requested catchup speed.
        speed: usize,
        /// The maximum frames behind value.
        max_frames_behind: usize,
    },

    // Disconnect errors
    /// Cannot disconnect: player handle is invalid.
    DisconnectInvalidHandle {
        /// The invalid handle.
        handle: PlayerHandle,
    },
    /// Cannot disconnect a local player.
    DisconnectLocalPlayer {
        /// The local player handle.
        handle: PlayerHandle,
    },
    /// Player is already disconnected.
    AlreadyDisconnected {
        /// The already disconnected handle.
        handle: PlayerHandle,
    },

    // Protocol errors
    /// Operation called in wrong protocol state.
    WrongProtocolState {
        /// The current state name.
        current_state: &'static str,
        /// The expected state name.
        expected_state: &'static str,
    },

    /// Custom error (fallback for API compatibility).
    Custom(&'static str),
}

impl Display for InvalidRequestKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlayerHandleInUse { handle } => {
                write!(f, "player handle {} is already in use", handle)
            },
            Self::NotLocalPlayer { handle } => {
                write!(
                    f,
                    "player handle {} does not refer to a local player",
                    handle
                )
            },
            Self::NotRemotePlayerOrSpectator { handle } => {
                write!(
                    f,
                    "player handle {} does not refer to a remote player or spectator",
                    handle
                )
            },
            Self::InvalidLocalPlayerHandle {
                handle,
                num_players,
            } => {
                write!(
                    f,
                    "invalid local player handle {}: num_players is {}",
                    handle, num_players
                )
            },
            Self::InvalidRemotePlayerHandle {
                handle,
                num_players,
            } => {
                write!(
                    f,
                    "invalid remote player handle {}: num_players is {}",
                    handle, num_players
                )
            },
            Self::InvalidSpectatorHandle {
                handle,
                num_players,
            } => {
                write!(
                    f,
                    "invalid spectator handle {}: num_players is {}",
                    handle, num_players
                )
            },
            Self::MissingLocalInput => write!(f, "missing local input for one or more players"),
            Self::NoConfirmedInput { frame } => {
                write!(f, "no confirmed input available for frame {}", frame)
            },
            Self::ConfigValueOutOfRange {
                field,
                min,
                max,
                actual,
            } => {
                write!(
                    f,
                    "configuration value '{}' is out of range: {} not in [{}, {}]",
                    field, actual, min, max
                )
            },
            Self::DurationConfigOutOfRange {
                field,
                min_ms,
                max_ms,
                actual_ms,
            } => {
                write!(
                    f,
                    "duration configuration '{}' is out of range: {}ms not in [{}ms, {}ms]",
                    field, actual_ms, min_ms, max_ms
                )
            },
            Self::FrameDelayTooLarge { delay, max_delay } => {
                write!(
                    f,
                    "frame delay {} exceeds maximum allowed delay {}",
                    delay, max_delay
                )
            },
            Self::InputDelayTooLarge {
                delay_frames,
                fps,
                max_seconds_limit,
            } => {
                // Defensive: if fps is zero, report infinite delay rather than panic
                let actual_seconds = if *fps > 0 {
                    *delay_frames as f64 / *fps as f64
                } else {
                    f64::INFINITY
                };
                write!(
                    f,
                    "input delay {} frames ({:.2}s) exceeds maximum allowed {}s",
                    delay_frames, actual_seconds, max_seconds_limit
                )
            },
            Self::QueueLengthTooSmall { length } => {
                write!(
                    f,
                    "input queue length {} is too small (minimum is 2)",
                    length
                )
            },
            Self::EventQueueSizeTooSmall { size } => {
                write!(f, "event queue size {} is too small (minimum is 10)", size)
            },
            Self::ZeroPlayers => write!(f, "number of players must be greater than 0"),
            Self::ZeroFps => write!(f, "FPS must be greater than 0"),
            Self::NotEnoughPlayers { expected, actual } => {
                write!(
                    f,
                    "not enough players registered: expected {}, got {}",
                    expected, actual
                )
            },
            Self::CheckDistanceTooLarge {
                check_dist,
                max_prediction,
            } => {
                write!(
                    f,
                    "check distance {} is too large for prediction window {}",
                    check_dist, max_prediction
                )
            },
            Self::MaxFramesBehindInvalid { value, buffer_size } => {
                write!(
                    f,
                    "max frames behind {} is invalid for buffer size {}",
                    value, buffer_size
                )
            },
            Self::CatchupSpeedInvalid {
                speed,
                max_frames_behind,
            } => {
                write!(
                    f,
                    "catchup speed {} is invalid for max frames behind {}",
                    speed, max_frames_behind
                )
            },
            Self::DisconnectInvalidHandle { handle } => {
                write!(f, "cannot disconnect: player handle {} is invalid", handle)
            },
            Self::DisconnectLocalPlayer { handle } => {
                write!(f, "cannot disconnect local player {}", handle)
            },
            Self::AlreadyDisconnected { handle } => {
                write!(f, "player {} is already disconnected", handle)
            },
            Self::WrongProtocolState {
                current_state,
                expected_state,
            } => {
                write!(
                    f,
                    "operation called in wrong protocol state: current '{}', expected '{}'",
                    current_state, expected_state
                )
            },
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Represents why serialization failed.
///
/// Using an enum instead of String allows for zero-allocation error construction
/// and programmatic error inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SerializationErrorKind {
    /// Failed to create a protocol endpoint for remote players.
    EndpointCreationFailed,
    /// Failed to create a protocol endpoint for spectators.
    SpectatorEndpointCreationFailed,
    /// Custom error (fallback for API compatibility).
    Custom(&'static str),
}

impl Display for SerializationErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndpointCreationFailed => {
                write!(f, "failed to create protocol endpoint for remote players")
            },
            Self::SpectatorEndpointCreationFailed => {
                write!(f, "failed to create protocol endpoint for spectators")
            },
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Represents why a socket operation failed.
///
/// Using an enum instead of String allows for zero-allocation error construction
/// and programmatic error inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SocketErrorKind {
    /// Failed to bind socket to the specified port.
    BindFailed {
        /// The port that failed to bind.
        port: u16,
    },
    /// Failed to bind after multiple retry attempts.
    BindFailedAfterRetries {
        /// The port that failed to bind.
        port: u16,
        /// The number of attempts made.
        attempts: u8,
    },
    /// Custom error (fallback for API compatibility).
    Custom(&'static str),
}

impl Display for SocketErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BindFailed { port } => {
                write!(f, "failed to bind socket to port {}", port)
            },
            Self::BindFailedAfterRetries { port, attempts } => {
                write!(
                    f,
                    "failed to bind socket to port {} after {} attempts",
                    port, attempts
                )
            },
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

// =============================================================================
// Main Error Enum
// =============================================================================

/// This enum contains all error messages this library can return. Most API functions will generally return a [`Result<(), FortressError>`].
///
/// # Forward Compatibility
///
/// This enum is marked `#[non_exhaustive]` because new error variants may be
/// added in future versions. Always include a wildcard arm when matching:
///
/// ```ignore
/// match error {
///     FortressError::NotSynchronized => { /* handle */ }
///     FortressError::PredictionThreshold => { /* handle */ }
///     _ => { /* handle unknown errors */ }
/// }
/// ```
///
/// [`Result<(), FortressError>`]: std::result::Result
#[derive(Debug, Clone, PartialEq, Hash)]
#[non_exhaustive]
pub enum FortressError {
    /// When the prediction threshold has been reached, we cannot accept more inputs from the local player.
    PredictionThreshold,
    /// You made an invalid request, usually by using wrong parameters for function calls.
    ///
    /// **Note**: For new code, prefer using [`FortressError::InvalidRequestStructured`] which
    /// provides zero-allocation error construction and programmatic error inspection.
    InvalidRequest {
        /// Further specifies why the request was invalid.
        info: String,
    },
    /// In a [`SyncTestSession`], this error is returned if checksums of resimulated frames do not match up with the original checksum.
    ///
    /// [`SyncTestSession`]: crate::SyncTestSession
    MismatchedChecksum {
        /// The frame at which the mismatch occurred.
        current_frame: Frame,
        /// The frames with mismatched checksums (one or more)
        mismatched_frames: Vec<Frame>,
    },
    /// The Session is not synchronized yet. Please start the session and wait a few ms to let the clients synchronize.
    NotSynchronized,
    /// The spectator got so far behind the host that catching up is impossible.
    SpectatorTooFarBehind,
    /// An invalid frame number was provided. Frames must be non-negative and within valid ranges.
    InvalidFrame {
        /// The frame that was invalid.
        frame: Frame,
        /// A description of why the frame was invalid (legacy String variant).
        reason: String,
    },
    /// An invalid frame number was provided, with structured reason (zero-allocation on hot path).
    ///
    /// This variant is preferred over `InvalidFrame` on hot paths as it avoids
    /// allocating a String for the reason.
    InvalidFrameStructured {
        /// The frame that was invalid.
        frame: Frame,
        /// The structured reason why the frame was invalid.
        reason: InvalidFrameReason,
    },
    /// An invalid player handle was provided. Player handles must be less than the number of players.
    InvalidPlayerHandle {
        /// The player handle that was invalid.
        handle: PlayerHandle,
        /// The maximum valid player handle (num_players - 1).
        max_handle: PlayerHandle,
    },
    /// A required input was missing for the specified frame.
    MissingInput {
        /// The player handle whose input was missing.
        player_handle: PlayerHandle,
        /// The frame for which input was missing.
        frame: Frame,
    },
    /// Serialization or deserialization of data failed.
    ///
    /// **Note**: For new code, prefer using [`FortressError::SerializationErrorStructured`] which
    /// provides zero-allocation error construction and programmatic error inspection.
    SerializationError {
        /// A description of what failed to serialize/deserialize.
        context: String,
    },
    /// An internal error occurred that should not happen under normal operation.
    /// If you encounter this error, please report it as a bug.
    InternalError {
        /// A description of the internal error.
        context: String,
    },
    /// An internal error with structured data (zero-allocation on hot path).
    ///
    /// This variant is preferred over `InternalError` on hot paths as it avoids
    /// allocating a String for the context.
    InternalErrorStructured {
        /// The structured kind of internal error.
        kind: InternalErrorKind,
    },
    /// A network socket operation failed.
    ///
    /// **Note**: For new code, prefer using [`FortressError::SocketErrorStructured`] which
    /// provides zero-allocation error construction and programmatic error inspection.
    SocketError {
        /// A description of the socket error.
        context: String,
    },
    /// An invalid request with structured reason (zero-allocation on hot path).
    ///
    /// This variant is preferred over `InvalidRequest` as it avoids
    /// allocating a String for the info.
    InvalidRequestStructured {
        /// The structured kind of invalid request.
        kind: InvalidRequestKind,
    },
    /// Serialization error with structured reason (zero-allocation on hot path).
    ///
    /// This variant is preferred over `SerializationError` as it avoids
    /// allocating a String for the context.
    SerializationErrorStructured {
        /// The structured kind of serialization error.
        kind: SerializationErrorKind,
    },
    /// Socket error with structured reason (zero-allocation on hot path).
    ///
    /// This variant is preferred over `SocketError` as it avoids
    /// allocating a String for the context.
    SocketErrorStructured {
        /// The structured kind of socket error.
        kind: SocketErrorKind,
    },
}

impl Display for FortressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PredictionThreshold => {
                write!(
                    f,
                    "Prediction threshold is reached, cannot proceed without catching up."
                )
            },
            Self::InvalidRequest { info } => {
                write!(f, "Invalid Request: {}", info)
            },
            Self::NotSynchronized => {
                write!(
                    f,
                    "The session is not yet synchronized with all remote sessions."
                )
            },
            Self::MismatchedChecksum {
                current_frame,
                mismatched_frames,
            } => {
                write!(
                    f,
                    "Detected checksum mismatch during rollback on frame {}, mismatched frames: {:?}",
                    current_frame, mismatched_frames
                )
            },
            Self::SpectatorTooFarBehind => {
                write!(
                    f,
                    "The spectator got so far behind the host that catching up is impossible."
                )
            },
            Self::InvalidFrame { frame, reason } => {
                write!(f, "Invalid frame {}: {}", frame, reason)
            },
            Self::InvalidFrameStructured { frame, reason } => {
                write!(f, "Invalid frame {}: {}", frame, reason)
            },
            Self::InvalidPlayerHandle { handle, max_handle } => {
                write!(
                    f,
                    "Invalid player handle {}: must be less than or equal to {}",
                    handle, max_handle
                )
            },
            Self::MissingInput {
                player_handle,
                frame,
            } => {
                write!(
                    f,
                    "Missing input for player {} at frame {}",
                    player_handle, frame
                )
            },
            Self::SerializationError { context } => {
                write!(f, "Serialization error: {}", context)
            },
            Self::InternalError { context } => {
                write!(f, "Internal error (please report as bug): {}", context)
            },
            Self::InternalErrorStructured { kind } => {
                write!(f, "Internal error (please report as bug): {}", kind)
            },
            Self::SocketError { context } => {
                write!(f, "Socket error: {}", context)
            },
            Self::InvalidRequestStructured { kind } => {
                write!(f, "Invalid Request: {}", kind)
            },
            Self::SerializationErrorStructured { kind } => {
                write!(f, "Serialization error: {}", kind)
            },
            Self::SocketErrorStructured { kind } => {
                write!(f, "Socket error: {}", kind)
            },
        }
    }
}

impl Error for FortressError {
    /// Returns the lower-level source of this error, if any.
    ///
    /// Currently, `FortressError` variants store error context as strings rather than
    /// wrapping underlying error types. This design choice:
    /// - Keeps the error type `Clone` and `PartialEq` (which `dyn Error` cannot be)
    /// - Avoids complexity in serializing errors across network boundaries
    /// - Maintains a simple, stable API
    ///
    /// If you need to preserve the original error, consider logging it before
    /// converting to `FortressError`.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // Error context is stored as strings, not wrapped errors.
        // This is intentional - see documentation above.
        None
    }
}

impl From<InvalidRequestKind> for FortressError {
    fn from(kind: InvalidRequestKind) -> Self {
        Self::InvalidRequestStructured { kind }
    }
}

impl From<SerializationErrorKind> for FortressError {
    fn from(kind: SerializationErrorKind) -> Self {
        Self::SerializationErrorStructured { kind }
    }
}

impl From<SocketErrorKind> for FortressError {
    fn from(kind: SocketErrorKind) -> Self {
        Self::SocketErrorStructured { kind }
    }
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

    #[test]
    fn test_prediction_threshold_display() {
        let err = FortressError::PredictionThreshold;
        let display = format!("{}", err);
        assert!(display.contains("Prediction threshold"));
        assert!(display.contains("cannot proceed"));
    }

    #[test]
    fn test_invalid_request_display() {
        let err = FortressError::InvalidRequest {
            info: "test error info".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Invalid Request"));
        assert!(display.contains("test error info"));
    }

    #[test]
    fn test_not_synchronized_display() {
        let err = FortressError::NotSynchronized;
        let display = format!("{}", err);
        assert!(display.contains("not yet synchronized"));
    }

    #[test]
    fn test_mismatched_checksum_display() {
        let err = FortressError::MismatchedChecksum {
            current_frame: Frame::new(100),
            mismatched_frames: vec![Frame::new(95), Frame::new(96)],
        };
        let display = format!("{}", err);
        assert!(display.contains("checksum mismatch"));
        assert!(display.contains("100"));
    }

    #[test]
    fn test_spectator_too_far_behind_display() {
        let err = FortressError::SpectatorTooFarBehind;
        let display = format!("{}", err);
        assert!(display.contains("spectator"));
        assert!(display.contains("behind"));
    }

    #[test]
    fn test_invalid_frame_display() {
        let err = FortressError::InvalidFrame {
            frame: Frame::new(42),
            reason: "frame is in the past".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Invalid frame"));
        assert!(display.contains("42"));
        assert!(display.contains("frame is in the past"));
    }

    #[test]
    fn test_invalid_player_handle_display() {
        let err = FortressError::InvalidPlayerHandle {
            handle: PlayerHandle(5),
            max_handle: PlayerHandle(3),
        };
        let display = format!("{}", err);
        assert!(display.contains("Invalid player handle"));
        assert!(display.contains('5'));
        assert!(display.contains('3'));
    }

    #[test]
    fn test_missing_input_display() {
        let err = FortressError::MissingInput {
            player_handle: PlayerHandle(1),
            frame: Frame::new(50),
        };
        let display = format!("{}", err);
        assert!(display.contains("Missing input"));
        assert!(display.contains("player 1"));
        assert!(display.contains("frame 50"));
    }

    #[test]
    fn test_serialization_error_display() {
        let err = FortressError::SerializationError {
            context: "failed to serialize game state".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Serialization error"));
        assert!(display.contains("failed to serialize game state"));
    }

    #[test]
    fn test_internal_error_display() {
        let err = FortressError::InternalError {
            context: "unexpected state transition".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Internal error"));
        assert!(display.contains("please report as bug"));
        assert!(display.contains("unexpected state transition"));
    }

    #[test]
    fn test_socket_error_display() {
        let err = FortressError::SocketError {
            context: "connection refused".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Socket error"));
        assert!(display.contains("connection refused"));
    }

    #[test]
    fn test_error_debug() {
        let err = FortressError::PredictionThreshold;
        let debug = format!("{:?}", err);
        assert!(debug.contains("PredictionThreshold"));
    }

    #[test]
    #[allow(clippy::redundant_clone)] // Testing Clone trait implementation
    fn test_error_clone() {
        let err = FortressError::InvalidRequest {
            info: "test".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_error_partial_eq() {
        let err1 = FortressError::NotSynchronized;
        let err2 = FortressError::NotSynchronized;
        let err3 = FortressError::PredictionThreshold;
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_error_implements_std_error() {
        let err: Box<dyn Error> = Box::new(FortressError::NotSynchronized);
        // This test verifies that FortressError implements std::error::Error
        assert!(err.source().is_none());
    }

    // =========================================================================
    // Structured Error Type Tests
    // =========================================================================

    #[test]
    fn test_index_out_of_bounds_display() {
        let err = IndexOutOfBounds {
            name: "input_queues",
            index: 5,
            length: 3,
        };
        let display = format!("{}", err);
        assert!(display.contains("input_queues"));
        assert!(display.contains('5'));
        assert!(display.contains('3'));
        assert!(display.contains("out of bounds"));
    }

    #[test]
    fn test_invalid_frame_reason_null_frame() {
        let reason = InvalidFrameReason::NullFrame;
        let display = format!("{}", reason);
        assert!(display.contains("NULL_FRAME"));
    }

    #[test]
    fn test_invalid_frame_reason_not_in_past() {
        let reason = InvalidFrameReason::NotInPast {
            current_frame: Frame::new(10),
        };
        let display = format!("{}", reason);
        assert!(display.contains("past"));
        assert!(display.contains("10"));
    }

    #[test]
    fn test_invalid_frame_reason_outside_prediction_window() {
        let reason = InvalidFrameReason::OutsidePredictionWindow {
            current_frame: Frame::new(100),
            max_prediction: 8,
        };
        let display = format!("{}", reason);
        assert!(display.contains("prediction window"));
        assert!(display.contains("100"));
        assert!(display.contains('8'));
    }

    #[test]
    fn test_invalid_frame_reason_wrong_saved_frame() {
        let reason = InvalidFrameReason::WrongSavedFrame {
            saved_frame: Frame::new(42),
        };
        let display = format!("{}", reason);
        assert!(display.contains("wrong frame"));
        assert!(display.contains("42"));
    }

    #[test]
    fn test_invalid_frame_reason_not_confirmed() {
        let reason = InvalidFrameReason::NotConfirmed {
            confirmed_frame: Frame::new(50),
        };
        let display = format!("{}", reason);
        assert!(display.contains("not confirmed"));
        assert!(display.contains("50"));
    }

    #[test]
    fn test_internal_error_kind_index_out_of_bounds() {
        let kind = InternalErrorKind::IndexOutOfBounds(IndexOutOfBounds {
            name: "states",
            index: 10,
            length: 5,
        });
        let display = format!("{}", kind);
        assert!(display.contains("states"));
        assert!(display.contains("10"));
        assert!(display.contains('5'));
    }

    #[test]
    fn test_internal_error_kind_synchronized_inputs_failed() {
        let kind = InternalErrorKind::SynchronizedInputsFailed {
            frame: Frame::new(25),
        };
        let display = format!("{}", kind);
        assert!(display.contains("synchronized inputs"));
        assert!(display.contains("25"));
    }

    #[test]
    fn test_internal_error_kind_empty_player_inputs() {
        let kind = InternalErrorKind::EmptyPlayerInputs;
        let display = format!("{}", kind);
        assert!(display.contains("empty"));
    }

    #[test]
    fn test_invalid_frame_structured_display() {
        let err = FortressError::InvalidFrameStructured {
            frame: Frame::new(42),
            reason: InvalidFrameReason::NullFrame,
        };
        let display = format!("{}", err);
        assert!(display.contains("Invalid frame"));
        assert!(display.contains("42"));
        assert!(display.contains("NULL_FRAME"));
    }

    #[test]
    fn test_internal_error_structured_display() {
        let err = FortressError::InternalErrorStructured {
            kind: InternalErrorKind::BufferIndexOutOfBounds,
        };
        let display = format!("{}", err);
        assert!(display.contains("Internal error"));
        assert!(display.contains("buffer index out of bounds"));
    }

    #[test]
    fn test_internal_error_kind_disconnect_status_not_found() {
        let kind = InternalErrorKind::DisconnectStatusNotFound {
            player_handle: PlayerHandle(3),
        };
        let display = format!("{}", kind);
        assert!(display.contains("disconnect status"));
        assert!(display.contains("player handle 3"));
    }

    #[test]
    fn test_internal_error_kind_endpoint_not_found_for_remote() {
        let kind = InternalErrorKind::EndpointNotFoundForRemote {
            player_handle: PlayerHandle(5),
        };
        let display = format!("{}", kind);
        assert!(display.contains("endpoint not found"));
        assert!(display.contains("remote player 5"));
    }

    #[test]
    fn test_internal_error_kind_endpoint_not_found_for_spectator() {
        let kind = InternalErrorKind::EndpointNotFoundForSpectator {
            player_handle: PlayerHandle(7),
        };
        let display = format!("{}", kind);
        assert!(display.contains("endpoint not found"));
        assert!(display.contains("spectator 7"));
    }

    #[test]
    fn test_structured_errors_are_copy() {
        // Verify that structured error types are Copy (important for hot path)
        let iob = IndexOutOfBounds {
            name: "test",
            index: 1,
            length: 2,
        };
        let iob2 = iob; // Copy
        assert_eq!(iob, iob2);

        let reason = InvalidFrameReason::NullFrame;
        let reason2 = reason; // Copy
        assert_eq!(reason, reason2);

        let kind = InternalErrorKind::EmptyPlayerInputs;
        let kind2 = kind; // Copy
        assert_eq!(kind, kind2);
    }

    // =========================================================================
    // RLE Decode Reason Tests
    // =========================================================================

    #[test]
    fn test_rle_decode_reason_bitfield_index_out_of_bounds() {
        let reason = RleDecodeReason::BitfieldIndexOutOfBounds;
        let display = format!("{}", reason);
        assert!(display.contains("bitfield index out of bounds"));
    }

    #[test]
    fn test_rle_decode_reason_destination_slice_out_of_bounds() {
        let reason = RleDecodeReason::DestinationSliceOutOfBounds;
        let display = format!("{}", reason);
        assert!(display.contains("destination slice out of bounds"));
    }

    #[test]
    fn test_rle_decode_reason_source_slice_out_of_bounds() {
        let reason = RleDecodeReason::SourceSliceOutOfBounds;
        let display = format!("{}", reason);
        assert!(display.contains("source slice out of bounds"));
    }

    #[test]
    fn test_rle_decode_reason_truncated_data() {
        let reason = RleDecodeReason::TruncatedData {
            offset: 100,
            buffer_len: 50,
        };
        let display = format!("{}", reason);
        assert!(display.contains("truncated data"));
        assert!(display.contains("100"));
        assert!(display.contains("50"));
    }

    #[test]
    fn test_internal_error_kind_rle_decode_error() {
        let kind = InternalErrorKind::RleDecodeError {
            reason: RleDecodeReason::BitfieldIndexOutOfBounds,
        };
        let display = format!("{}", kind);
        assert!(display.contains("RLE decode failed"));
        assert!(display.contains("bitfield index out of bounds"));
    }

    #[test]
    fn test_rle_decode_reason_is_copy() {
        // Verify RleDecodeReason is Copy (important for hot path)
        let reason = RleDecodeReason::BitfieldIndexOutOfBounds;
        let reason2 = reason; // Copy
        assert_eq!(reason, reason2);

        let reason_with_data = RleDecodeReason::TruncatedData {
            offset: 10,
            buffer_len: 5,
        };
        let reason_with_data2 = reason_with_data; // Copy
        assert_eq!(reason_with_data, reason_with_data2);
    }

    // =========================================================================
    // InvalidRequestKind Tests
    // =========================================================================

    #[test]
    fn test_invalid_request_kind_player_handle_in_use() {
        let kind = InvalidRequestKind::PlayerHandleInUse {
            handle: PlayerHandle(3),
        };
        let display = format!("{}", kind);
        assert!(display.contains("player handle 3"));
        assert!(display.contains("already in use"));
    }

    #[test]
    fn test_invalid_request_kind_not_local_player() {
        let kind = InvalidRequestKind::NotLocalPlayer {
            handle: PlayerHandle(2),
        };
        let display = format!("{}", kind);
        assert!(display.contains("player handle 2"));
        assert!(display.contains("local player"));
    }

    #[test]
    fn test_invalid_request_kind_not_remote_player_or_spectator() {
        let kind = InvalidRequestKind::NotRemotePlayerOrSpectator {
            handle: PlayerHandle(1),
        };
        let display = format!("{}", kind);
        assert!(display.contains("player handle 1"));
        assert!(display.contains("remote player or spectator"));
    }

    #[test]
    fn test_invalid_request_kind_missing_local_input() {
        let kind = InvalidRequestKind::MissingLocalInput;
        let display = format!("{}", kind);
        assert!(display.contains("missing local input"));
    }

    #[test]
    fn test_invalid_request_kind_no_confirmed_input() {
        let kind = InvalidRequestKind::NoConfirmedInput {
            frame: Frame::new(42),
        };
        let display = format!("{}", kind);
        assert!(display.contains("no confirmed input"));
        assert!(display.contains("42"));
    }

    #[test]
    fn test_invalid_request_kind_config_value_out_of_range() {
        let kind = InvalidRequestKind::ConfigValueOutOfRange {
            field: "fps",
            min: 1,
            max: 120,
            actual: 0,
        };
        let display = format!("{}", kind);
        assert!(display.contains("fps"));
        assert!(display.contains("out of range"));
        assert!(display.contains('0'));
        assert!(display.contains('1'));
        assert!(display.contains("120"));
    }

    #[test]
    fn test_invalid_request_kind_frame_delay_too_large() {
        let kind = InvalidRequestKind::FrameDelayTooLarge {
            delay: 10,
            max_delay: 5,
        };
        let display = format!("{}", kind);
        assert!(display.contains("frame delay"));
        assert!(display.contains("10"));
        assert!(display.contains('5'));
    }

    #[test]
    fn test_invalid_request_kind_queue_length_too_small() {
        let kind = InvalidRequestKind::QueueLengthTooSmall { length: 1 };
        let display = format!("{}", kind);
        assert!(display.contains("queue length"));
        assert!(display.contains('1'));
        assert!(display.contains("minimum is 2"));
    }

    #[test]
    fn test_invalid_request_kind_event_queue_size_too_small() {
        let kind = InvalidRequestKind::EventQueueSizeTooSmall { size: 5 };
        let display = format!("{}", kind);
        assert!(display.contains("event queue size"));
        assert!(display.contains('5'));
        assert!(display.contains("minimum is 10"));
    }

    #[test]
    fn test_invalid_request_kind_zero_players() {
        let kind = InvalidRequestKind::ZeroPlayers;
        let display = format!("{}", kind);
        assert!(display.contains("players"));
        assert!(display.contains("greater than 0"));
    }

    #[test]
    fn test_invalid_request_kind_zero_fps() {
        let kind = InvalidRequestKind::ZeroFps;
        let display = format!("{}", kind);
        assert!(display.contains("FPS"));
        assert!(display.contains("greater than 0"));
    }

    #[test]
    fn test_invalid_request_kind_not_enough_players() {
        let kind = InvalidRequestKind::NotEnoughPlayers {
            expected: 4,
            actual: 2,
        };
        let display = format!("{}", kind);
        assert!(display.contains("not enough players"));
        assert!(display.contains('4'));
        assert!(display.contains('2'));
    }

    #[test]
    fn test_invalid_request_kind_check_distance_too_large() {
        let kind = InvalidRequestKind::CheckDistanceTooLarge {
            check_dist: 20,
            max_prediction: 10,
        };
        let display = format!("{}", kind);
        assert!(display.contains("check distance"));
        assert!(display.contains("20"));
        assert!(display.contains("10"));
    }

    #[test]
    fn test_invalid_request_kind_max_frames_behind_invalid() {
        let kind = InvalidRequestKind::MaxFramesBehindInvalid {
            value: 100,
            buffer_size: 50,
        };
        let display = format!("{}", kind);
        assert!(display.contains("max frames behind"));
        assert!(display.contains("100"));
        assert!(display.contains("50"));
    }

    #[test]
    fn test_invalid_request_kind_catchup_speed_invalid() {
        let kind = InvalidRequestKind::CatchupSpeedInvalid {
            speed: 5,
            max_frames_behind: 2,
        };
        let display = format!("{}", kind);
        assert!(display.contains("catchup speed"));
        assert!(display.contains('5'));
        assert!(display.contains('2'));
    }

    #[test]
    fn test_invalid_request_kind_disconnect_invalid_handle() {
        let kind = InvalidRequestKind::DisconnectInvalidHandle {
            handle: PlayerHandle(99),
        };
        let display = format!("{}", kind);
        assert!(display.contains("disconnect"));
        assert!(display.contains("99"));
        assert!(display.contains("invalid"));
    }

    #[test]
    fn test_invalid_request_kind_disconnect_local_player() {
        let kind = InvalidRequestKind::DisconnectLocalPlayer {
            handle: PlayerHandle(0),
        };
        let display = format!("{}", kind);
        assert!(display.contains("disconnect"));
        assert!(display.contains("local player"));
        assert!(display.contains('0'));
    }

    #[test]
    fn test_invalid_request_kind_already_disconnected() {
        let kind = InvalidRequestKind::AlreadyDisconnected {
            handle: PlayerHandle(2),
        };
        let display = format!("{}", kind);
        assert!(display.contains("already disconnected"));
        assert!(display.contains('2'));
    }

    #[test]
    fn test_invalid_request_kind_wrong_protocol_state() {
        let kind = InvalidRequestKind::WrongProtocolState {
            current_state: "Running",
            expected_state: "Synchronizing",
        };
        let display = format!("{}", kind);
        assert!(display.contains("wrong protocol state"));
        assert!(display.contains("Running"));
        assert!(display.contains("Synchronizing"));
    }

    #[test]
    fn test_invalid_request_kind_custom() {
        let kind = InvalidRequestKind::Custom("custom error message");
        let display = format!("{}", kind);
        assert!(display.contains("custom error message"));
    }

    #[test]
    fn test_invalid_request_kind_is_copy() {
        // Verify InvalidRequestKind is Copy (important for hot path)
        let kind = InvalidRequestKind::ZeroPlayers;
        let kind2 = kind; // Copy
        assert_eq!(kind, kind2);

        let kind_with_data = InvalidRequestKind::PlayerHandleInUse {
            handle: PlayerHandle(1),
        };
        let kind_with_data2 = kind_with_data; // Copy
        assert_eq!(kind_with_data, kind_with_data2);
    }

    // =========================================================================
    // SerializationErrorKind Tests
    // =========================================================================

    #[test]
    fn test_serialization_error_kind_endpoint_creation_failed() {
        let kind = SerializationErrorKind::EndpointCreationFailed;
        let display = format!("{}", kind);
        assert!(display.contains("failed to create"));
        assert!(display.contains("endpoint"));
        assert!(display.contains("remote players"));
    }

    #[test]
    fn test_serialization_error_kind_spectator_endpoint_creation_failed() {
        let kind = SerializationErrorKind::SpectatorEndpointCreationFailed;
        let display = format!("{}", kind);
        assert!(display.contains("failed to create"));
        assert!(display.contains("endpoint"));
        assert!(display.contains("spectators"));
    }

    #[test]
    fn test_serialization_error_kind_custom() {
        let kind = SerializationErrorKind::Custom("custom serialization error");
        let display = format!("{}", kind);
        assert!(display.contains("custom serialization error"));
    }

    #[test]
    fn test_serialization_error_kind_is_copy() {
        // Verify SerializationErrorKind is Copy (important for hot path)
        let kind = SerializationErrorKind::EndpointCreationFailed;
        let kind2 = kind; // Copy
        assert_eq!(kind, kind2);
    }

    // =========================================================================
    // SocketErrorKind Tests
    // =========================================================================

    #[test]
    fn test_socket_error_kind_bind_failed() {
        let kind = SocketErrorKind::BindFailed { port: 8080 };
        let display = format!("{}", kind);
        assert!(display.contains("failed to bind"));
        assert!(display.contains("8080"));
    }

    #[test]
    fn test_socket_error_kind_bind_failed_after_retries() {
        let kind = SocketErrorKind::BindFailedAfterRetries {
            port: 9000,
            attempts: 5,
        };
        let display = format!("{}", kind);
        assert!(display.contains("failed to bind"));
        assert!(display.contains("9000"));
        assert!(display.contains('5'));
        assert!(display.contains("attempts"));
    }

    #[test]
    fn test_socket_error_kind_custom() {
        let kind = SocketErrorKind::Custom("custom socket error");
        let display = format!("{}", kind);
        assert!(display.contains("custom socket error"));
    }

    #[test]
    fn test_socket_error_kind_is_copy() {
        // Verify SocketErrorKind is Copy (important for hot path)
        let kind = SocketErrorKind::BindFailed { port: 8080 };
        let kind2 = kind; // Copy
        assert_eq!(kind, kind2);
    }

    // =========================================================================
    // FortressError Structured Variant Tests
    // =========================================================================

    #[test]
    fn test_invalid_request_structured_display() {
        let err = FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::ZeroPlayers,
        };
        let display = format!("{}", err);
        assert!(display.contains("Invalid Request"));
        assert!(display.contains("players"));
    }

    #[test]
    fn test_serialization_error_structured_display() {
        let err = FortressError::SerializationErrorStructured {
            kind: SerializationErrorKind::EndpointCreationFailed,
        };
        let display = format!("{}", err);
        assert!(display.contains("Serialization error"));
        assert!(display.contains("endpoint"));
    }

    #[test]
    fn test_socket_error_structured_display() {
        let err = FortressError::SocketErrorStructured {
            kind: SocketErrorKind::BindFailed { port: 8080 },
        };
        let display = format!("{}", err);
        assert!(display.contains("Socket error"));
        assert!(display.contains("8080"));
    }

    // =========================================================================
    // From Implementations Tests
    // =========================================================================

    #[test]
    fn test_from_invalid_request_kind() {
        let kind = InvalidRequestKind::ZeroPlayers;
        let err: FortressError = kind.into();
        assert_eq!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ZeroPlayers
            }
        );
    }

    #[test]
    fn test_from_serialization_error_kind() {
        let kind = SerializationErrorKind::EndpointCreationFailed;
        let err: FortressError = kind.into();
        assert_eq!(
            err,
            FortressError::SerializationErrorStructured {
                kind: SerializationErrorKind::EndpointCreationFailed
            }
        );
    }

    #[test]
    fn test_from_socket_error_kind() {
        let kind = SocketErrorKind::BindFailed { port: 8080 };
        let err: FortressError = kind.into();
        assert_eq!(
            err,
            FortressError::SocketErrorStructured {
                kind: SocketErrorKind::BindFailed { port: 8080 }
            }
        );
    }

    // =========================================================================
    // New Variant Tests (Review Feedback)
    // =========================================================================

    #[test]
    fn test_invalid_request_kind_duration_config_out_of_range() {
        let kind = InvalidRequestKind::DurationConfigOutOfRange {
            field: "disconnect_timeout",
            min_ms: 100,
            max_ms: 60000,
            actual_ms: 50,
        };
        let display = format!("{}", kind);
        assert!(display.contains("duration configuration"));
        assert!(display.contains("disconnect_timeout"));
        assert!(display.contains("50ms"));
        assert!(display.contains("100ms"));
        assert!(display.contains("60000ms"));
    }

    #[test]
    fn test_invalid_request_kind_invalid_local_player_handle() {
        let kind = InvalidRequestKind::InvalidLocalPlayerHandle {
            handle: PlayerHandle(5),
            num_players: 4,
        };
        let display = format!("{}", kind);
        assert!(display.contains("invalid local player handle"));
        assert!(display.contains('5'));
        assert!(display.contains("num_players is 4"));
    }

    #[test]
    fn test_invalid_request_kind_invalid_remote_player_handle() {
        let kind = InvalidRequestKind::InvalidRemotePlayerHandle {
            handle: PlayerHandle(10),
            num_players: 2,
        };
        let display = format!("{}", kind);
        assert!(display.contains("invalid remote player handle"));
        assert!(display.contains("10"));
        assert!(display.contains("num_players is 2"));
    }

    #[test]
    fn test_invalid_request_kind_invalid_spectator_handle() {
        let kind = InvalidRequestKind::InvalidSpectatorHandle {
            handle: PlayerHandle(3),
            num_players: 2,
        };
        let display = format!("{}", kind);
        assert!(display.contains("invalid spectator handle"));
        assert!(display.contains('3'));
        assert!(display.contains("num_players is 2"));
    }

    #[test]
    fn test_invalid_request_kind_input_delay_too_large() {
        let kind = InvalidRequestKind::InputDelayTooLarge {
            delay_frames: 660,
            fps: 60,
            max_seconds_limit: 10,
        };
        let display = format!("{}", kind);
        assert!(display.contains("input delay"));
        assert!(display.contains("660"));
        assert!(display.contains("11.00s")); // 660 / 60 = 11.00
        assert!(display.contains("10s")); // max_seconds_limit
    }

    #[test]
    fn test_invalid_request_kind_input_delay_too_large_with_zero_fps() {
        // Test the defensive branch that handles fps=0 without panicking
        let kind = InvalidRequestKind::InputDelayTooLarge {
            delay_frames: 100,
            fps: 0,
            max_seconds_limit: 10,
        };
        let display = format!("{}", kind);
        assert!(display.contains("input delay"));
        assert!(display.contains("100"));
        assert!(
            display.contains("inf"),
            "Should display infinity for zero fps: {display}"
        );
    }

    #[test]
    fn test_new_variants_are_copy() {
        // Verify new variants remain Copy (important for hot path)
        let kind1 = InvalidRequestKind::DurationConfigOutOfRange {
            field: "test",
            min_ms: 0,
            max_ms: 100,
            actual_ms: 50,
        };
        let kind1_copy = kind1; // Copy
        assert_eq!(kind1, kind1_copy);

        let kind2 = InvalidRequestKind::InvalidLocalPlayerHandle {
            handle: PlayerHandle(1),
            num_players: 2,
        };
        let kind2_copy = kind2; // Copy
        assert_eq!(kind2, kind2_copy);

        let kind3 = InvalidRequestKind::InputDelayTooLarge {
            delay_frames: 10,
            fps: 60,
            max_seconds_limit: 1,
        };
        let kind3_copy = kind3; // Copy
        assert_eq!(kind3, kind3_copy);
    }
}
