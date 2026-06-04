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
//! #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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

use crate::error::{allocation_failed, InvalidRequestKind};
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
/// [`Config::Input`]: `Copy + Clone + PartialEq + Eq + Default + Serialize + DeserializeOwned`.
/// When the `sync-send` feature is enabled, `Send + Sync` bounds are additionally required.
///
/// # Example
///
/// ```
/// use fortress_rollback::replay::{Replay, ReplayMetadata};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)] // derive-bounds:ok(Eq via Config::Input)
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

/// Caller-configurable replay decode options.
///
/// The default configuration imposes no replay-size cap beyond the byte slice
/// provided by the caller. Applications that load replay files from untrusted
/// locations can set [`max_bytes`](Self::max_bytes) to their own policy limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayDecodeConfig {
    /// Maximum encoded replay size accepted by the decoder.
    ///
    /// `None` means no caller policy limit. Allocation sizes are still bounded:
    /// every length-prefixed vector (frames, per-frame inputs, checksums) is
    /// checked against the remaining byte slice before any memory is reserved,
    /// so a malformed replay cannot drive an out-of-proportion allocation
    /// regardless of this setting.
    pub max_bytes: Option<usize>,
    /// Whether to run [`Replay::validate`] after decoding.
    pub validate: bool,
}

impl Default for ReplayDecodeConfig {
    fn default() -> Self {
        Self {
            max_bytes: None,
            validate: true,
        }
    }
}

impl ReplayDecodeConfig {
    /// Creates the default replay decode configuration.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_bytes: None,
            validate: true,
        }
    }

    /// Sets a caller-defined encoded byte limit.
    #[must_use]
    pub const fn max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = Some(max_bytes);
        self
    }

    /// Clears any caller-defined encoded byte limit.
    #[must_use]
    pub const fn without_byte_limit(mut self) -> Self {
        self.max_bytes = None;
        self
    }

    /// Enables or disables post-decode replay validation.
    #[must_use]
    pub const fn validate(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }
}

impl<I> Replay<I>
where
    I: Serialize,
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
}

impl<I> Replay<I>
where
    I: Copy + DeserializeOwned,
{
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
        Self::from_bytes_with_config(bytes, ReplayDecodeConfig::default())
    }

    /// Deserializes a replay with caller-configured decode options.
    ///
    /// # Errors
    ///
    /// Returns a [`CodecError`] if the byte stream is malformed, fails the
    /// caller's configured policy, or fails replay validation.
    ///
    /// [`CodecError`]: crate::network::codec::CodecError
    pub fn from_bytes_with_config(bytes: &[u8], config: ReplayDecodeConfig) -> CodecResult<Self> {
        decode_replay(bytes, config)
    }
}

fn replay_decode_error(message: impl Into<String>) -> codec::CodecError {
    codec::CodecError::decode(message, codec::CodecOperation::Decode)
}

fn read_replay_array<const N: usize>(
    bytes: &[u8],
    cursor: &mut usize,
    field: &'static str,
) -> CodecResult<[u8; N]> {
    let end = cursor
        .checked_add(N)
        .ok_or_else(|| replay_decode_error(format!("replay {field} offset overflow")))?;
    let slice = bytes
        .get(*cursor..end)
        .ok_or_else(|| replay_decode_error(format!("truncated replay {field}")))?;
    let mut out = [0_u8; N];
    out.copy_from_slice(slice);
    *cursor = end;
    Ok(out)
}

fn read_replay_u8(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<u8> {
    Ok(read_replay_array::<1>(bytes, cursor, field)?[0])
}

fn read_replay_u128(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<u128> {
    Ok(u128::from_le_bytes(read_replay_array(
        bytes, cursor, field,
    )?))
}

fn read_replay_usize(bytes: &[u8], cursor: &mut usize, field: &'static str) -> CodecResult<usize> {
    let value = u64::from_le_bytes(read_replay_array(bytes, cursor, field)?);
    usize::try_from(value)
        .map_err(|_err| replay_decode_error(format!("replay {field} length exceeds usize")))
}

fn take_replay_bytes<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    len: usize,
    field: &'static str,
) -> CodecResult<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| replay_decode_error(format!("replay {field} offset overflow")))?;
    let slice = bytes
        .get(*cursor..end)
        .ok_or_else(|| replay_decode_error(format!("truncated replay {field}")))?;
    *cursor = end;
    Ok(slice)
}

fn read_replay_string(
    bytes: &[u8],
    cursor: &mut usize,
    field: &'static str,
) -> CodecResult<String> {
    let len = read_replay_usize(bytes, cursor, field)?;
    let raw = take_replay_bytes(bytes, cursor, len, field)?;
    let utf8 = std::str::from_utf8(raw)
        .map_err(|err| replay_decode_error(format!("invalid UTF-8 in replay {field}: {err}")))?;
    let mut out = String::new();
    out.try_reserve_exact(utf8.len())
        .map_err(|_err| replay_decode_error(format!("failed to reserve replay {field}")))?;
    out.push_str(utf8);
    Ok(out)
}

/// Minimum wire footprint of one encoded frame: each frame begins with its
/// own inputs.len u64 length prefix (8 bytes), so a frame can never encode in
/// fewer bytes than that.
const MIN_FRAME_ENCODED_LEN: usize = core::mem::size_of::<u64>();

/// Minimum wire footprint of one encoded checksum entry: each is at least its
/// 1-byte Option tag.
const MIN_CHECKSUM_ENCODED_LEN: usize = 1;

/// Minimum wire footprint of one encoded input: under fixed-int bincode every
/// non-zero-sized input occupies at least 1 byte. (Every derived `Serialize`
/// satisfies this; a hand-rolled impl that writes nothing for a non-zero-sized
/// input would be lossy and already breaks the determinism contract required of
/// `Config::Input`, and the decode loop's forward-progress guard rejects it.)
const MIN_INPUT_ENCODED_LEN: usize = 1;

/// Reserves space in a replay vector after first bounding the element count
/// against the unread input bytes.
///
/// This mirrors the network codec decode discipline: a hostile length prefix
/// cannot drive a large speculative allocation because the count is first
/// bounded by `remaining_bytes / min_encoded_len` via
/// [`codec::ensure_length_within_remaining`]. Folding the bound into the
/// reservation makes an unbounded allocation impossible for every element type
/// that occupies wire space. `cursor` is the current local cursor value (a
/// `Copy` `usize`) taken after reading the length prefix, so `remaining` is
/// measured correctly.
///
/// The byte bound is deliberately *skipped* for a zero-sized `T`. Such an
/// element occupies no wire bytes and never allocates (`try_reserve_exact` is a
/// no-op for it), so there is nothing to bound; and its count is intrinsically
/// independent of the encoded size -- a valid replay of zero-sized inputs can
/// declare any per-frame count within a fixed number of bytes. Applying a byte
/// bound to it would therefore reject a replay that [`Replay::validate`] accepts
/// and [`Replay::to_bytes`] produced, breaking round-trip self-consistency. The
/// only residual cost is that a crafted huge per-frame count for a zero-sized
/// input type spins the decode loop; that requires a zero-sized `Config::Input`
/// (a degenerate game with no per-player input that no real configuration uses),
/// allocates nothing, and cannot occur for any input that carries data -- every
/// byte-carrying input is bounded here, and the decode loop additionally guards
/// forward progress for non-zero-sized inputs.
///
/// [`Config::Input`]: crate::Config::Input
fn reserve_replay_vec<T>(
    vec: &mut Vec<T>,
    bytes: &[u8],
    cursor: usize,
    len: usize,
    min_encoded_len: usize,
    field: &'static str,
) -> CodecResult<()> {
    // Skip the byte bound for a zero-sized `T`: it allocates nothing and its
    // count has no byte-proportional bound, so bounding it would reject valid
    // (`validate`-passing) zero-sized replays. See this fn's docs.
    if core::mem::size_of::<T>() != 0 {
        codec::ensure_length_within_remaining(bytes, cursor, len, min_encoded_len, field)?;
    }
    vec.try_reserve_exact(len).map_err(|_err| {
        replay_decode_error(format!(
            "failed to reserve replay {field} with {len} element(s)"
        ))
    })
}

fn decode_replay<I>(bytes: &[u8], config: ReplayDecodeConfig) -> CodecResult<Replay<I>>
where
    I: Copy + DeserializeOwned,
{
    if let Some(max_bytes) = config.max_bytes {
        if bytes.len() > max_bytes {
            return Err(replay_decode_error(format!(
                "encoded replay length {} exceeds configured limit {}",
                bytes.len(),
                max_bytes
            )));
        }
    }

    let mut cursor = 0;
    let num_players = read_replay_usize(bytes, &mut cursor, "num_players")?;
    let frame_count = read_replay_usize(bytes, &mut cursor, "frames.len")?;

    let mut frames = Vec::new();
    reserve_replay_vec(
        &mut frames,
        bytes,
        cursor,
        frame_count,
        MIN_FRAME_ENCODED_LEN,
        "frames",
    )?;
    for frame_index in 0..frame_count {
        let frame_inputs_len = read_replay_usize(bytes, &mut cursor, "frame.inputs.len")?;
        if frame_inputs_len != num_players {
            return Err(replay_decode_error(format!(
                "replay frame {frame_index} has {frame_inputs_len} input(s), expected {num_players}"
            )));
        }

        let mut frame = Vec::new();
        reserve_replay_vec(
            &mut frame,
            bytes,
            cursor,
            frame_inputs_len,
            MIN_INPUT_ENCODED_LEN,
            "frame.inputs",
        )?;
        for player_index in 0..frame_inputs_len {
            let remaining = bytes
                .get(cursor..)
                .ok_or_else(|| replay_decode_error("replay input cursor out of bounds"))?;
            let (input, consumed) = codec::decode::<I>(remaining).map_err(|err| {
                replay_decode_error(format!(
                    "failed to decode replay input at frame {frame_index}, player {player_index}: {err}"
                ))
            })?;
            // Forward-progress guard: a non-zero-sized input must consume at
            // least one wire byte, so the loop can run at most `remaining` times
            // regardless of the declared count. A zero-sized input legitimately
            // consumes nothing; its iteration count is bounded instead by the
            // `reserve_replay_vec` byte check above.
            if core::mem::size_of::<I>() != 0 && consumed == 0 {
                return Err(replay_decode_error(format!(
                    "replay input at frame {frame_index}, player {player_index} decoded zero bytes"
                )));
            }
            cursor = cursor
                .checked_add(consumed)
                .ok_or_else(|| replay_decode_error("replay input cursor overflow"))?;
            frame.push(input);
        }
        frames.push(frame);
    }

    let checksum_count = read_replay_usize(bytes, &mut cursor, "checksums.len")?;
    if checksum_count != frame_count {
        return Err(replay_decode_error(format!(
            "replay has {checksum_count} checksum(s), expected {frame_count}"
        )));
    }

    let mut checksums = Vec::new();
    reserve_replay_vec(
        &mut checksums,
        bytes,
        cursor,
        checksum_count,
        MIN_CHECKSUM_ENCODED_LEN,
        "checksums",
    )?;
    for checksum_index in 0..checksum_count {
        let tag = read_replay_u8(bytes, &mut cursor, "checksum.option")?;
        let checksum = match tag {
            0 => None,
            1 => Some(read_replay_u128(bytes, &mut cursor, "checksum.value")?),
            other => {
                return Err(replay_decode_error(format!(
                    "invalid replay checksum option tag {other} at index {checksum_index}"
                )));
            },
        };
        checksums.push(checksum);
    }

    let metadata_library_version =
        read_replay_string(bytes, &mut cursor, "metadata.library_version")?;
    let metadata_num_players = read_replay_usize(bytes, &mut cursor, "metadata.num_players")?;
    let metadata_total_frames = read_replay_usize(bytes, &mut cursor, "metadata.total_frames")?;
    let metadata_skipped_frames = if cursor == bytes.len() {
        0
    } else {
        read_replay_usize(bytes, &mut cursor, "metadata.skipped_frames")?
    };
    let metadata = ReplayMetadata {
        library_version: metadata_library_version,
        num_players: metadata_num_players,
        total_frames: metadata_total_frames,
        skipped_frames: metadata_skipped_frames,
    };

    if cursor != bytes.len() {
        return Err(replay_decode_error(format!(
            "replay has {} trailing byte(s)",
            bytes.len() - cursor
        )));
    }

    let replay = Replay {
        num_players,
        frames,
        checksums,
        metadata,
    };
    if config.validate {
        replay.validate().map_err(|err| {
            replay_decode_error(format!("decoded replay failed validation: {err}"))
        })?;
    }
    Ok(replay)
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
    pub(crate) fn record_skipped_frame(&mut self) -> FortressResult<()>
    where
        I: Default + Clone,
    {
        let mut inputs = Vec::new();
        inputs
            .try_reserve_exact(self.num_players)
            .map_err(|_err| allocation_failed("replay.skipped_frame_inputs", self.num_players))?;
        for _ in 0..self.num_players {
            inputs.push(I::default());
        }
        self.frames.push(inputs);
        self.checksums.push(None);
        self.skipped_frames = self.skipped_frames.saturating_add(1);
        Ok(())
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
    use serde::{Deserialize, Serialize};

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
    fn replay_from_bytes_rejects_configured_byte_limit() {
        let replay = Replay::<u8> {
            num_players: 1,
            frames: vec![vec![7]],
            checksums: vec![None],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 1,
                total_frames: 1,
                skipped_frames: 0,
            },
        };
        let bytes = replay.to_bytes().unwrap();

        let result = Replay::<u8>::from_bytes_with_config(
            &bytes,
            ReplayDecodeConfig::new().max_bytes(bytes.len() - 1),
        );

        assert!(result.is_err());
        let restored = Replay::<u8>::from_bytes_with_config(
            &bytes,
            ReplayDecodeConfig::new().max_bytes(bytes.len()),
        )
        .unwrap();
        assert_eq!(restored, replay);
    }

    #[test]
    fn replay_from_bytes_rejects_pathological_frame_count_without_allocating() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // frames.len

        let result = Replay::<u8>::from_bytes(&bytes);

        assert!(result.is_err());
    }

    #[test]
    fn replay_from_bytes_rejects_huge_frame_count_via_byte_bound() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&1_000_000_000_u64.to_le_bytes()); // frames.len

        let err = match Replay::<u8>::from_bytes(&bytes) {
            Ok(_) => panic!("huge frame count must be rejected before allocating"),
            Err(err) => err.to_string(),
        };

        // The byte bound fired before any large reservation was attempted.
        assert!(err.contains("frames"), "message was: {err}");
        assert!(err.contains("exceeds"), "message was: {err}");
    }

    #[test]
    fn replay_from_bytes_rejects_huge_frame_inputs_len_via_byte_bound() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1_000_000_000_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&1_u64.to_le_bytes()); // frames.len
        bytes.extend_from_slice(&1_000_000_000_u64.to_le_bytes()); // frame0.inputs.len

        let err = match Replay::<u8>::from_bytes(&bytes) {
            Ok(_) => panic!("huge per-frame input count must be rejected before allocating"),
            Err(err) => err.to_string(),
        };

        assert!(err.contains("frame.inputs"), "message was: {err}");
        assert!(err.contains("exceeds"), "message was: {err}");
    }

    #[test]
    fn replay_from_bytes_rejects_truncated_checksums_via_byte_bound() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // frames.len
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // frame0.inputs.len
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // frame1.inputs.len
        bytes.extend_from_slice(&2_u64.to_le_bytes()); // checksums.len (no payload follows)

        let err = match Replay::<u8>::from_bytes(&bytes) {
            Ok(_) => panic!("truncated checksums must be rejected before allocating"),
            Err(err) => err.to_string(),
        };

        assert!(err.contains("checksums"), "message was: {err}");
        assert!(err.contains("exceeds"), "message was: {err}");
    }

    #[test]
    fn replay_from_bytes_accepts_zero_sized_inputs() {
        // A zero-sized input occupies no wire bytes and `try_reserve_exact` is a
        // no-op for it, so the byte bound is skipped for it (see
        // `reserve_replay_vec`). A valid zero-sized replay round-trips unchanged.
        #[derive(Copy, Clone, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
        struct Zst;

        let replay = Replay::<Zst> {
            num_players: 2,
            frames: vec![vec![Zst, Zst]; 5],
            checksums: vec![None; 5],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 2,
                total_frames: 5,
                skipped_frames: 0,
            },
        };

        let bytes = replay.to_bytes().unwrap();
        let restored = Replay::<Zst>::from_bytes(&bytes).unwrap();
        assert_eq!(restored, replay);
    }

    #[test]
    fn replay_roundtrips_zero_sized_inputs_with_many_players() {
        // Regression guard for the zero-sized-input bound exemption. A replay of
        // zero-sized inputs encodes its per-frame input count in a fixed number
        // of bytes no matter how large that count is, so the count can legitimately
        // exceed the bytes that follow it. Here num_players (64) far exceeds the
        // few tens of trailing bytes: applying the byte bound to a zero-sized
        // element (as a byte-carrying one is bounded) would reject this replay
        // even though `validate` accepts it and `to_bytes` produced it -- a
        // round-trip/self-consistency break. The bound is skipped for zero-sized
        // elements, so the round-trip holds.
        #[derive(Copy, Clone, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
        struct Zst;

        let replay = Replay::<Zst> {
            num_players: 64,
            frames: vec![vec![Zst; 64]; 1],
            checksums: vec![None; 1],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 64,
                total_frames: 1,
                skipped_frames: 0,
            },
        };

        replay.validate().unwrap();
        let bytes = replay.to_bytes().unwrap();
        let restored = Replay::<Zst>::from_bytes(&bytes).unwrap();
        assert_eq!(restored, replay);
    }

    #[test]
    fn replay_from_bytes_validates_decoded_replay_by_default() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // frames.len
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // checksums.len
        bytes.extend_from_slice(&4_u64.to_le_bytes()); // metadata.library_version.len
        bytes.extend_from_slice(b"test");
        bytes.extend_from_slice(&1_u64.to_le_bytes()); // metadata.num_players
        bytes.extend_from_slice(&99_u64.to_le_bytes()); // metadata.total_frames mismatch
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // metadata.skipped_frames

        let result = Replay::<u8>::from_bytes(&bytes);

        assert!(result.is_err());
        let decoded =
            Replay::<u8>::from_bytes_with_config(&bytes, ReplayDecodeConfig::new().validate(false))
                .unwrap();
        assert_eq!(decoded.metadata.total_frames, 99);
    }

    #[test]
    fn replay_from_bytes_supports_pre_skipped_frames_metadata() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1_u64.to_le_bytes()); // num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // frames.len
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // checksums.len
        bytes.extend_from_slice(&4_u64.to_le_bytes()); // metadata.library_version.len
        bytes.extend_from_slice(b"test");
        bytes.extend_from_slice(&1_u64.to_le_bytes()); // metadata.num_players
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // metadata.total_frames

        let decoded = Replay::<u8>::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.metadata.skipped_frames, 0);
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
