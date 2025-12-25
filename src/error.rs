use std::error::Error;
use std::fmt;
use std::fmt::Display;

use crate::{Frame, PlayerHandle};

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
        /// A description of why the frame was invalid.
        reason: String,
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
            Self::SocketError { context } => {
                write!(f, "Socket error: {}", context)
            },
        }
    }
}

impl Error for FortressError {}

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
}
