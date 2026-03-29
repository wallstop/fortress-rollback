//! Replay recording and playback for deterministic match replays.
//!
//! This module provides types for recording game inputs during a P2P session
//! and playing them back deterministically. A [`Replay`] captures all confirmed
//! inputs per frame, enabling exact reproduction of a match.
//!
//! # Recording
//!
//! Enable recording on a [`SessionBuilder`] with [`with_recording`], then
//! extract the replay after the session ends with [`P2PSession::into_replay`].
//!
//! # Playback
//!
//! Create a [`ReplaySession`] from a [`Replay`] to play back the recorded
//! inputs frame by frame.
//!
//! # Serialization
//!
//! Replays can be serialized to and from bytes using [`Replay::to_bytes`] and
//! [`Replay::from_bytes`], which use the same deterministic bincode codec as
//! network messages.
//!
//! # Example
//!
//! ```
//! use fortress_rollback::replay::{Replay, ReplayMetadata};
//! use serde::{Deserialize, Serialize};
//!
//! // Replays are parameterized on your Config's Input type
//! #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
//! struct MyInput { buttons: u8 }
//!
//! let replay = Replay::<MyInput> {
//!     num_players: 2,
//!     frames: vec![vec![MyInput { buttons: 0 }; 2]; 10],
//!     checksums: vec![None; 10],
//!     metadata: ReplayMetadata {
//!         library_version: env!("CARGO_PKG_VERSION").to_string(),
//!         num_players: 2,
//!         total_frames: 10,
//!         skipped_frames: 0,
//!     },
//! };
//!
//! // Serialize roundtrip
//! let bytes = replay.to_bytes()?;
//! let restored = Replay::<MyInput>::from_bytes(&bytes)?;
//! assert_eq!(restored.num_players, 2);
//! assert_eq!(restored.frames.len(), 10);
//! # Ok::<(), fortress_rollback::network::codec::CodecError>(())
//! ```
//!
//! [`SessionBuilder`]: crate::SessionBuilder
//! [`with_recording`]: crate::SessionBuilder::with_recording
//! [`P2PSession::into_replay`]: crate::P2PSession::into_replay
//! [`ReplaySession`]: crate::sessions::replay_session::ReplaySession

use std::fmt;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::InvalidRequestKind;
use crate::network::codec::{self, CodecResult};
use crate::FortressResult;

/// A recorded match that can be played back deterministically.
///
/// Contains all confirmed inputs per frame along with optional checksums
/// for validation. The inner `Vec` of each frame entry contains one input
/// per player, ordered by player handle index.
///
/// # Type Parameter
///
/// `I` is the input type, which must satisfy the same bounds as
/// [`Config::Input`]: `Copy + Clone + PartialEq + Default + Serialize + DeserializeOwned`.
/// When the `sync-send` feature is enabled, `Send + Sync` bounds are additionally required.
///
/// # Example
///
/// ```
/// use fortress_rollback::replay::{Replay, ReplayMetadata};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
/// struct GameInput { direction: u8 }
///
/// let replay = Replay::<GameInput> {
///     num_players: 2,
///     frames: vec![vec![GameInput::default(); 2]; 60],
///     checksums: vec![None; 60],
///     metadata: ReplayMetadata {
///         library_version: env!("CARGO_PKG_VERSION").to_string(),
///         num_players: 2,
///         total_frames: 60,
///         skipped_frames: 0,
///     },
/// };
/// assert_eq!(replay.total_frames(), 60);
/// ```
///
/// [`Config::Input`]: crate::Config::Input
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay<I> {
    /// The number of players in this recorded match.
    pub num_players: usize,
    /// Confirmed inputs per frame. Each inner `Vec` has one entry per player.
    pub frames: Vec<Vec<I>>,
    /// Optional per-frame checksums for desync detection during playback.
    ///
    /// When recording is enabled on a P2P session, checksums are captured
    /// from the saved game state at each confirmed frame. During validation
    /// playback (via [`ReplaySession::new_with_validation`]), these checksums
    /// are compared against freshly computed checksums to detect
    /// non-determinism.
    ///
    /// [`ReplaySession::new_with_validation`]: crate::sessions::replay_session::ReplaySession::new_with_validation
    pub checksums: Vec<Option<u128>>,
    /// Metadata about the replay.
    pub metadata: ReplayMetadata,
}

impl<I> Replay<I>
where
    I: Serialize + DeserializeOwned,
{
    /// Serializes this replay to bytes using the deterministic bincode codec.
    ///
    /// # Errors
    ///
    /// Returns a [`CodecError`] if serialization fails.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::replay::{Replay, ReplayMetadata};
    ///
    /// let replay = Replay::<u8> {
    ///     num_players: 2,
    ///     frames: vec![vec![0u8; 2]; 5],
    ///     checksums: vec![None; 5],
    ///     metadata: ReplayMetadata {
    ///         library_version: env!("CARGO_PKG_VERSION").to_string(),
    ///         num_players: 2,
    ///         total_frames: 5,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// let bytes = replay.to_bytes()?;
    /// assert!(!bytes.is_empty());
    /// # Ok::<(), fortress_rollback::network::codec::CodecError>(())
    /// ```
    ///
    /// [`CodecError`]: crate::network::codec::CodecError
    pub fn to_bytes(&self) -> CodecResult<Vec<u8>> {
        codec::encode(self)
    }

    /// Deserializes a replay from bytes using the deterministic bincode codec.
    ///
    /// # Errors
    ///
    /// Returns a [`CodecError`] if deserialization fails.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::replay::{Replay, ReplayMetadata};
    ///
    /// let replay = Replay::<u8> {
    ///     num_players: 2,
    ///     frames: vec![vec![0u8; 2]; 5],
    ///     checksums: vec![None; 5],
    ///     metadata: ReplayMetadata {
    ///         library_version: env!("CARGO_PKG_VERSION").to_string(),
    ///         num_players: 2,
    ///         total_frames: 5,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// let bytes = replay.to_bytes()?;
    /// let restored = Replay::<u8>::from_bytes(&bytes)?;
    /// assert_eq!(restored.num_players, 2);
    /// # Ok::<(), fortress_rollback::network::codec::CodecError>(())
    /// ```
    ///
    /// [`CodecError`]: crate::network::codec::CodecError
    pub fn from_bytes(bytes: &[u8]) -> CodecResult<Self> {
        codec::decode_value(bytes)
    }
}

impl<I> Replay<I> {
    /// Returns the total number of recorded frames.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::replay::{Replay, ReplayMetadata};
    ///
    /// let replay = Replay::<u8> {
    ///     num_players: 1,
    ///     frames: vec![vec![0u8]; 42],
    ///     checksums: vec![None; 42],
    ///     metadata: ReplayMetadata {
    ///         library_version: String::new(),
    ///         num_players: 1,
    ///         total_frames: 42,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// assert_eq!(replay.total_frames(), 42);
    /// ```
    #[must_use]
    pub fn total_frames(&self) -> usize {
        self.frames.len()
    }

    /// Validates the internal consistency of this replay.
    ///
    /// Checks that:
    /// - `frames.len() == checksums.len()`
    /// - All frames have exactly `num_players` inputs
    /// - `metadata.num_players == num_players`
    /// - `metadata.total_frames == frames.len()`
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRequestKind::Custom`] if any consistency check fails.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::replay::{Replay, ReplayMetadata};
    ///
    /// let replay = Replay::<u8> {
    ///     num_players: 2,
    ///     frames: vec![vec![0u8, 1u8]; 5],
    ///     checksums: vec![None; 5],
    ///     metadata: ReplayMetadata {
    ///         library_version: String::new(),
    ///         num_players: 2,
    ///         total_frames: 5,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// replay.validate()?;
    /// # Ok::<(), fortress_rollback::FortressError>(())
    /// ```
    pub fn validate(&self) -> FortressResult<()> {
        if self.frames.len() != self.checksums.len() {
            return Err(InvalidRequestKind::Custom(
                "replay validation failed: frames.len() != checksums.len()",
            )
            .into());
        }
        for frame in &self.frames {
            if frame.len() != self.num_players {
                return Err(InvalidRequestKind::Custom(
                    "replay validation failed: frame does not have exactly num_players inputs",
                )
                .into());
            }
        }
        if self.metadata.num_players != self.num_players {
            return Err(InvalidRequestKind::Custom(
                "replay validation failed: metadata.num_players != num_players",
            )
            .into());
        }
        if self.metadata.total_frames != self.frames.len() {
            return Err(InvalidRequestKind::Custom(
                "replay validation failed: metadata.total_frames != frames.len()",
            )
            .into());
        }
        Ok(())
    }
}

impl<I> fmt::Display for Replay<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Replay({} players, {} frames, v{})",
            self.num_players,
            self.frames.len(),
            self.metadata.library_version,
        )
    }
}

impl fmt::Display for ReplayMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.skipped_frames > 0 {
            write!(
                f,
                "ReplayMetadata({} players, {} frames, {} skipped, v{})",
                self.num_players, self.total_frames, self.skipped_frames, self.library_version,
            )
        } else {
            write!(
                f,
                "ReplayMetadata({} players, {} frames, v{})",
                self.num_players, self.total_frames, self.library_version,
            )
        }
    }
}

/// Metadata about a recorded replay.
///
/// Contains information about the library version, player count, and
/// total frames for compatibility checks and display purposes.
///
/// # Example
///
/// ```
/// use fortress_rollback::replay::ReplayMetadata;
///
/// let meta = ReplayMetadata {
///     library_version: env!("CARGO_PKG_VERSION").to_string(),
///     num_players: 2,
///     total_frames: 300,
///     skipped_frames: 0,
/// };
/// assert_eq!(meta.num_players, 2);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayMetadata {
    /// The version of the fortress-rollback library used to record this replay.
    pub library_version: String,
    /// The number of players in the recorded match.
    ///
    /// **Note:** This must be consistent with [`Replay::num_players`]. The
    /// [`Replay::validate`] method checks this invariant.
    pub num_players: usize,
    /// The total number of frames in the replay.
    ///
    /// **Note:** This must be consistent with `Replay::frames.len()`. The
    /// [`Replay::validate`] method checks this invariant. This field is
    /// useful when metadata is serialized independently without loading
    /// all frame data.
    pub total_frames: usize,
    /// The number of frames that were skipped during recording due to
    /// input retrieval failures.
    ///
    /// When a frame's confirmed inputs cannot be retrieved (e.g., because
    /// the frame was already discarded from the input queue), default
    /// placeholder inputs are recorded instead and this counter is
    /// incremented. A non-zero value indicates the replay has gaps where
    /// the real inputs were unavailable.
    ///
    /// Defaults to `0` when deserializing replays that were recorded before
    /// this field was added.
    #[serde(default)]
    pub skipped_frames: usize,
}

/// Accumulates confirmed inputs during a P2P session for replay recording.
///
/// This is an internal type used by [`P2PSession`] when recording is enabled.
/// It tracks confirmed inputs frame by frame and can produce a [`Replay`]
/// when the session ends.
///
/// [`P2PSession`]: crate::P2PSession
#[derive(Clone, Debug)]
pub(crate) struct ReplayRecorder<I> {
    num_players: usize,
    frames: Vec<Vec<I>>,
    checksums: Vec<Option<u128>>,
    skipped_frames: usize,
}

impl<I> ReplayRecorder<I> {
    /// Creates a new recorder for the given number of players.
    pub(crate) fn new(num_players: usize) -> Self {
        Self {
            num_players,
            frames: Vec::new(),
            checksums: Vec::new(),
            skipped_frames: 0,
        }
    }

    /// Records a single frame's confirmed inputs.
    pub(crate) fn record_frame(&mut self, inputs: Vec<I>, checksum: Option<u128>) {
        self.frames.push(inputs);
        self.checksums.push(checksum);
    }

    /// Records a skipped frame with default placeholder inputs and no checksum.
    ///
    /// This maintains frame index alignment in the replay when the real
    /// inputs for a frame could not be retrieved. The `skipped_frames`
    /// counter is incremented so consumers can detect recording gaps.
    pub(crate) fn record_skipped_frame(&mut self)
    where
        I: Default + Clone,
    {
        self.frames.push(vec![I::default(); self.num_players]);
        self.checksums.push(None);
        self.skipped_frames = self.skipped_frames.saturating_add(1);
    }

    /// Returns the number of frames recorded so far.
    #[cfg(test)]
    pub(crate) fn recorded_frames(&self) -> usize {
        self.frames.len()
    }

    /// Returns the number of skipped frames recorded so far.
    #[cfg(test)]
    pub(crate) fn skipped_frames(&self) -> usize {
        self.skipped_frames
    }

    /// Consumes this recorder and produces a [`Replay`].
    pub(crate) fn into_replay(self) -> Replay<I> {
        let total_frames = self.frames.len();
        Replay {
            num_players: self.num_players,
            frames: self.frames,
            checksums: self.checksums,
            metadata: ReplayMetadata {
                library_version: env!("CARGO_PKG_VERSION").to_string(),
                num_players: self.num_players,
                total_frames,
                skipped_frames: self.skipped_frames,
            },
        }
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
    fn replay_construction_basic() {
        let replay = Replay::<u8> {
            num_players: 2,
            frames: vec![vec![1, 2], vec![3, 4]],
            checksums: vec![None, Some(42)],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 2,
                total_frames: 2,
                skipped_frames: 0,
            },
        };
        assert_eq!(replay.num_players, 2);
        assert_eq!(replay.total_frames(), 2);
        assert_eq!(replay.frames[0], vec![1, 2]);
        assert_eq!(replay.checksums[1], Some(42));
    }

    #[test]
    fn replay_serialization_roundtrip() {
        let replay = Replay::<u8> {
            num_players: 2,
            frames: vec![vec![10, 20], vec![30, 40], vec![50, 60]],
            checksums: vec![None, Some(123), None],
            metadata: ReplayMetadata {
                library_version: "0.7.0".to_string(),
                num_players: 2,
                total_frames: 3,
                skipped_frames: 0,
            },
        };

        let bytes = replay.to_bytes().unwrap();
        let restored = Replay::<u8>::from_bytes(&bytes).unwrap();

        assert_eq!(restored.num_players, replay.num_players);
        assert_eq!(restored.frames, replay.frames);
        assert_eq!(restored.checksums, replay.checksums);
        assert_eq!(
            restored.metadata.library_version,
            replay.metadata.library_version
        );
        assert_eq!(restored.metadata.total_frames, replay.metadata.total_frames);
    }

    #[test]
    fn replay_serialization_empty() {
        let replay = Replay::<u8> {
            num_players: 1,
            frames: vec![],
            checksums: vec![],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 1,
                total_frames: 0,
                skipped_frames: 0,
            },
        };

        let bytes = replay.to_bytes().unwrap();
        let restored = Replay::<u8>::from_bytes(&bytes).unwrap();
        assert_eq!(restored.total_frames(), 0);
        assert!(restored.frames.is_empty());
    }

    #[test]
    fn replay_from_invalid_bytes_fails() {
        let result = Replay::<u8>::from_bytes(&[0xFF, 0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn replay_metadata_fields() {
        let meta = ReplayMetadata {
            library_version: "1.2.3".to_string(),
            num_players: 4,
            total_frames: 1000,
            skipped_frames: 0,
        };
        assert_eq!(meta.library_version, "1.2.3");
        assert_eq!(meta.num_players, 4);
        assert_eq!(meta.total_frames, 1000);
    }

    #[test]
    fn replay_recorder_basic() {
        let mut recorder = ReplayRecorder::<u8>::new(2);
        assert_eq!(recorder.recorded_frames(), 0);

        recorder.record_frame(vec![1, 2], None);
        recorder.record_frame(vec![3, 4], Some(99));
        assert_eq!(recorder.recorded_frames(), 2);

        let replay = recorder.into_replay();
        assert_eq!(replay.num_players, 2);
        assert_eq!(replay.total_frames(), 2);
        assert_eq!(replay.frames[0], vec![1, 2]);
        assert_eq!(replay.frames[1], vec![3, 4]);
        assert_eq!(replay.checksums[0], None);
        assert_eq!(replay.checksums[1], Some(99));
        assert_eq!(replay.metadata.num_players, 2);
        assert_eq!(replay.metadata.total_frames, 2);
    }

    #[test]
    fn replay_recorder_empty_into_replay() {
        let recorder = ReplayRecorder::<u8>::new(3);
        let replay = recorder.into_replay();
        assert_eq!(replay.num_players, 3);
        assert_eq!(replay.total_frames(), 0);
        assert!(replay.frames.is_empty());
    }

    #[test]
    fn replay_clone() {
        let replay = Replay::<u8> {
            num_players: 2,
            frames: vec![vec![1, 2]],
            checksums: vec![None],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 2,
                total_frames: 1,
                skipped_frames: 0,
            },
        };
        let cloned = replay.clone();
        assert_eq!(cloned.num_players, replay.num_players);
        assert_eq!(cloned.frames, replay.frames);
    }

    #[test]
    fn replay_serialization_deterministic() {
        let replay = Replay::<u8> {
            num_players: 2,
            frames: vec![vec![10, 20], vec![30, 40]],
            checksums: vec![None, Some(42)],
            metadata: ReplayMetadata {
                library_version: "0.7.0".to_string(),
                num_players: 2,
                total_frames: 2,
                skipped_frames: 0,
            },
        };

        let bytes1 = replay.to_bytes().unwrap();
        let bytes2 = replay.to_bytes().unwrap();
        assert_eq!(bytes1, bytes2, "Replay serialization must be deterministic");
    }

    #[test]
    fn validate_valid_replay_succeeds() {
        let replay = Replay::<u8> {
            num_players: 2,
            frames: vec![vec![1, 2], vec![3, 4]],
            checksums: vec![None, None],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 2,
                total_frames: 2,
                skipped_frames: 0,
            },
        };
        replay.validate().unwrap();
    }

    #[test]
    fn validate_empty_replay_succeeds() {
        let replay = Replay::<u8> {
            num_players: 1,
            frames: vec![],
            checksums: vec![],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 1,
                total_frames: 0,
                skipped_frames: 0,
            },
        };
        replay.validate().unwrap();
    }

    #[test]
    fn validate_frames_checksums_mismatch_fails() {
        let replay = Replay::<u8> {
            num_players: 1,
            frames: vec![vec![1], vec![2]],
            checksums: vec![None],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 1,
                total_frames: 2,
                skipped_frames: 0,
            },
        };
        assert!(replay.validate().is_err());
    }

    #[test]
    fn validate_wrong_num_inputs_per_frame_fails() {
        let replay = Replay::<u8> {
            num_players: 2,
            frames: vec![vec![1, 2], vec![3]], // second frame has only 1 input
            checksums: vec![None, None],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 2,
                total_frames: 2,
                skipped_frames: 0,
            },
        };
        assert!(replay.validate().is_err());
    }

    #[test]
    fn validate_metadata_num_players_mismatch_fails() {
        let replay = Replay::<u8> {
            num_players: 2,
            frames: vec![vec![1, 2]],
            checksums: vec![None],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 3, // mismatch
                total_frames: 1,
                skipped_frames: 0,
            },
        };
        assert!(replay.validate().is_err());
    }

    #[test]
    fn validate_metadata_total_frames_mismatch_fails() {
        let replay = Replay::<u8> {
            num_players: 1,
            frames: vec![vec![1]],
            checksums: vec![None],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 1,
                total_frames: 99, // mismatch
                skipped_frames: 0,
            },
        };
        assert!(replay.validate().is_err());
    }

    #[test]
    fn replay_partial_eq() {
        let replay1 = Replay::<u8> {
            num_players: 2,
            frames: vec![vec![1, 2]],
            checksums: vec![None],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 2,
                total_frames: 1,
                skipped_frames: 0,
            },
        };
        let replay2 = replay1.clone();
        assert_eq!(replay1, replay2);
    }

    #[test]
    fn replay_display() {
        let replay = Replay::<u8> {
            num_players: 2,
            frames: vec![vec![1, 2]; 10],
            checksums: vec![None; 10],
            metadata: ReplayMetadata {
                library_version: "0.7.0".to_string(),
                num_players: 2,
                total_frames: 10,
                skipped_frames: 0,
            },
        };
        let display = format!("{}", replay);
        assert!(display.contains("2 players"));
        assert!(display.contains("10 frames"));
        assert!(display.contains("0.7.0"));
    }

    #[test]
    fn replay_metadata_display() {
        let meta = ReplayMetadata {
            library_version: "1.0.0".to_string(),
            num_players: 4,
            total_frames: 100,
            skipped_frames: 0,
        };
        let display = format!("{}", meta);
        assert!(display.contains("4 players"));
        assert!(display.contains("100 frames"));
        assert!(display.contains("1.0.0"));
    }

    #[test]
    fn replay_metadata_partial_eq() {
        let meta1 = ReplayMetadata {
            library_version: "test".to_string(),
            num_players: 2,
            total_frames: 10,
            skipped_frames: 0,
        };
        let meta2 = meta1.clone();
        assert_eq!(meta1, meta2);
    }
}
