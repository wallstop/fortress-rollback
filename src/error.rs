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
    SocketError {
        /// A description of the socket error.
        context: String,
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
}
