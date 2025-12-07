use std::error::Error;
use std::fmt;
use std::fmt::Display;

use crate::{Frame, PlayerHandle};

/// This enum contains all error messages this library can return. Most API functions will generally return a [`Result<(), FortressError>`].
///
/// [`Result<(), FortressError>`]: std::result::Result
#[derive(Debug, Clone, PartialEq, Hash)]
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
            FortressError::PredictionThreshold => {
                write!(
                    f,
                    "Prediction threshold is reached, cannot proceed without catching up."
                )
            }
            FortressError::InvalidRequest { info } => {
                write!(f, "Invalid Request: {}", info)
            }
            FortressError::NotSynchronized => {
                write!(
                    f,
                    "The session is not yet synchronized with all remote sessions."
                )
            }
            FortressError::MismatchedChecksum {
                current_frame,
                mismatched_frames,
            } => {
                write!(
                    f,
                    "Detected checksum mismatch during rollback on frame {}, mismatched frames: {:?}",
                    current_frame, mismatched_frames
                )
            }
            FortressError::SpectatorTooFarBehind => {
                write!(
                    f,
                    "The spectator got so far behind the host that catching up is impossible."
                )
            }
            FortressError::InvalidFrame { frame, reason } => {
                write!(f, "Invalid frame {}: {}", frame, reason)
            }
            FortressError::InvalidPlayerHandle { handle, max_handle } => {
                write!(
                    f,
                    "Invalid player handle {}: must be less than or equal to {}",
                    handle, max_handle
                )
            }
            FortressError::MissingInput {
                player_handle,
                frame,
            } => {
                write!(
                    f,
                    "Missing input for player {} at frame {}",
                    player_handle, frame
                )
            }
            FortressError::SerializationError { context } => {
                write!(f, "Serialization error: {}", context)
            }
            FortressError::InternalError { context } => {
                write!(f, "Internal error (please report as bug): {}", context)
            }
            FortressError::SocketError { context } => {
                write!(f, "Socket error: {}", context)
            }
        }
    }
}

impl Error for FortressError {}
