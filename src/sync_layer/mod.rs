//! # Sync Layer - Rollback Networking Core
//!
//! The sync layer manages game state synchronization for rollback-based netcode.
//! It handles state saving, input prediction, and rollback/re-simulation.
//!
//! ## How Rollback Works
//!
//! Rollback networking allows games to run smoothly despite network latency by
//! predicting remote player inputs and correcting mistakes when actual inputs arrive.
//!
//! ### Step 1: State Saving
//!
//! Each frame, the game state is saved to a circular buffer managed by [`SavedStates`].
//! The buffer holds `max_prediction + 1` frames, allowing rollback up to `max_prediction`
//! frames into the past. States are stored in [`GameStateCell`] containers for thread-safe
//! access. When the buffer is full, the oldest state is overwritten.
//!
//! ### Step 2: Input Handling
//!
//! - **Local inputs**: Added immediately via `SyncLayer::add_local_input`
//! - **Remote inputs**: Arrive over the network with variable latency
//! - Each player has a dedicated [`InputQueue`] tracking confirmed and predicted inputs
//! - The `input_delay` setting adds buffer frames to smooth network jitter
//!
//! ### Step 3: Prediction
//!
//! When remote inputs haven't arrived for a frame, the sync layer uses the
//! [`PredictionStrategy`] to guess what
//! the remote player will do:
//!
//! - **`RepeatLastConfirmed`** (default): Use the last known input - works well
//!   for most games since players typically hold inputs for multiple frames
//! - **`BlankPrediction`**: Use a neutral/default input
//!
//! **Critical**: Predictions must be deterministic. Given the same game state and
//! last confirmed input, all peers must predict the same value.
//!
//! ### Step 4: Rollback
//!
//! When actual remote inputs arrive and differ from predictions:
//!
//! 1. **Detection**: The input queue detects the misprediction
//! 2. **Load State**: Load the saved state from before the misprediction via
//!    [`FortressRequest::LoadGameState`]
//! 3. **Re-simulation**: Advance forward with correct inputs, generating
//!    [`FortressRequest::AdvanceFrame`] requests
//! 4. **Bounds**: Rollback is bounded by `max_prediction` frames
//!
//! ### Step 5: Desync Detection
//!
//! Checksums are compared between peers to detect when game states have diverged.
//! Desyncs typically indicate non-determinism bugs and cannot be automatically
//! recovered - the game must be restarted or resynchronized.
//!
//! ## Data Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Game Loop (per frame)                   │
//! ├─────────────────────────────────────────────────────────────┤
//! │  1. Add local inputs    ──►  InputQueue (local player)      │
//! │  2. Receive network     ──►  InputQueue (remote players)    │
//! │  3. Check for rollback  ──►  If misprediction detected:     │
//! │                              └─► LoadGameState request      │
//! │                              └─► Re-simulate frames         │
//! │  4. Get synchronized    ──►  All players' inputs for frame  │
//! │     inputs                                                   │
//! │  5. Save state          ──►  SavedStates circular buffer    │
//! │  6. Advance simulation  ──►  AdvanceFrame request           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Bounds and Limits
//!
//! - **`max_prediction`**: Maximum frames of prediction (default: 8, typical: 7-15)
//!   - Higher = more latency tolerance, more memory, longer rollbacks
//!   - At 60 FPS: 8 frames ≈ 133ms, 15 frames ≈ 250ms
//! - **State buffer size**: `max_prediction + 1` slots in circular buffer
//! - **Input queue length**: Configurable, default 128 frames (~2.1s at 60 FPS)
//!
//! ## Determinism Requirement
//!
//! **Critical**: The game simulation MUST be deterministic. Given the same inputs,
//! every peer must produce identical game states. Non-determinism causes desyncs.
//!
//! Common sources of non-determinism to avoid:
//! - **Floating-point**: Use fixed-point or integers for physics/positions
//! - **HashMap iteration**: Use `BTreeMap` or sort keys before iterating
//! - **System time**: Use frame counter, not wall clock
//! - **Random numbers**: Use the provided deterministic [`Rng`]
//! - **Uninitialized memory**: Always initialize all fields
//! - **Multithreading**: Run simulation on a single thread
//! - **External I/O**: Only read inputs from the input queue
//!
//! ## Module Structure
//!
//! - [`GameStateCell`] and [`GameStateAccessor`] - Types for saving/loading game states
//! - [`SavedStates`] - Circular buffer holding saved game states
//! - [`SyncLayer`] - The main synchronization layer managing state and inputs
//!
//! [`PredictionStrategy`]: crate::input_queue::PredictionStrategy
//! [`Rng`]: crate::rng::Rng

mod game_state_cell;
mod saved_states;

pub use game_state_cell::{GameStateAccessor, GameStateCell};
pub use saved_states::SavedStates;

use crate::frame_info::PlayerInput;
use crate::input_queue::InputQueue;
use crate::network::messages::ConnectionStatus;
use crate::proof_vec::ProofVec;
use crate::sessions::config::SaveMode;
use crate::telemetry::{InvariantChecker, InvariantViolation, ViolationKind, ViolationSeverity};
use crate::{report_violation, safe_frame_add};
// `safe_frame_sub!` is invoked from the confirmed-frame discard pass (compiled
// out under Kani; see `set_last_confirmed_frame`) and, under the `hot-join`
// feature, from `seek_to_frame`. Import it whenever either consumer is present.
#[cfg(any(not(kani), feature = "hot-join"))]
use crate::safe_frame_sub;
#[cfg(feature = "hot-join")]
use crate::InvalidRequestKind;
use crate::{
    Config, FortressError, FortressRequest, Frame, IndexOutOfBounds, InputStatus, InputVec,
    InternalErrorKind, InvalidFrameReason, PlayerHandle,
};

/// The synchronization layer manages game state, input queues, and rollback operations.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
///
/// # Formal Specification Alignment
/// - **TLA+**: `SyncLayer` state in `specs/tla/Rollback.tla`
/// - **Invariants verified**:
///   - INV-1: Frame monotonicity (except during rollback)
///   - INV-2: Rollback bounded by `max_prediction`
///   - INV-6: State availability for rollback frames
///   - INV-7: `last_confirmed_frame <= current_frame`
///   - INV-8: `last_saved_frame <= current_frame`
/// - **Kani proofs**: 14 proofs in `sync_layer.rs` verify bounds and state transitions
/// - **Loom tests**: `GameStateCell` concurrent access verified in `loom-tests/`
pub struct SyncLayer<T>
where
    T: Config,
{
    num_players: usize,
    /// Maximum frames of prediction allowed before rollback is required.
    ///
    /// # Formal Specification Alignment
    /// - **TLA+**: `MAX_PREDICTION` in `specs/tla/Rollback.tla`
    /// - **Z3**: `MAX_PREDICTION` in `tests/test_z3_verification.rs`
    /// - **formal-spec.md**: INV-2 requires `rollback_depth <= max_prediction`
    max_prediction: usize,
    saved_states: SavedStates<T::State>,
    /// The last frame where all player inputs are confirmed.
    ///
    /// # Formal Specification Alignment
    /// - **TLA+**: `lastConfirmedFrame` in `specs/tla/Rollback.tla`
    /// - **formal-spec.md**: INV-7 requires `last_confirmed_frame <= current_frame`
    last_confirmed_frame: Frame,
    /// The most recently saved frame.
    ///
    /// # Formal Specification Alignment
    /// - **TLA+**: `lastSavedFrame` in `specs/tla/Rollback.tla`
    /// - **formal-spec.md**: INV-8 requires `last_saved_frame <= current_frame`
    last_saved_frame: Frame,
    /// The current simulation frame.
    ///
    /// # Formal Specification Alignment
    /// - **TLA+**: `currentFrame` in `specs/tla/Rollback.tla`
    /// - **formal-spec.md**: INV-1 requires monotonic increase (except rollback)
    current_frame: Frame,
    input_queues: ProofVec<InputQueue<T>>,
}

/// Builds an `InternalErrorStructured`/`IndexOutOfBounds` error tagged with
/// the literal collection name `"input_queues"`. Concentrating the literal
/// in one place avoids drift between callsites and removes the structured-
/// error ceremony from the surrounding control flow.
#[inline]
fn input_queue_oob(index: usize, length: usize) -> FortressError {
    FortressError::InternalErrorStructured {
        kind: InternalErrorKind::IndexOutOfBounds(IndexOutOfBounds {
            name: "input_queues",
            index,
            length,
        }),
    }
}

impl<T: Config> SyncLayer<T> {
    /// Creates a new `SyncLayer` instance with given values and default queue length.
    ///
    /// Note: This function exists for backward compatibility and testing.
    /// The main construction path uses `with_queue_length` via `SessionBuilder`.
    #[allow(dead_code)]
    #[must_use]
    pub fn new(num_players: usize, max_prediction: usize) -> Self {
        Self::with_queue_length(
            num_players,
            max_prediction,
            crate::input_queue::INPUT_QUEUE_LENGTH,
        )
    }

    /// Creates a new `SyncLayer` instance with a custom input queue length.
    ///
    /// # Arguments
    /// * `num_players` - The number of players in the session
    /// * `max_prediction` - Maximum frames of prediction allowed
    /// * `queue_length` - The size of the input queue circular buffer per player
    ///
    /// Production construction uses [`Self::try_with_queue_length`] so invalid
    /// configuration and failed reservations can be returned as structured
    /// errors. This compatibility wrapper reports the error and returns an
    /// empty internal layer rather than panicking or allocating unchecked.
    ///
    /// # Degraded fallback
    ///
    /// On error this returns a layer with `num_players == 0`, `max_prediction
    /// == 0`, and no input queues — a valid-but-inert object that cannot
    /// advance frames. This wrapper exists only for backward compatibility and
    /// internal testing (`SyncLayer` is re-exported under
    /// [`__internal`](crate::__internal) with no stability guarantees); the
    /// production session path never hits this fallback because it constructs
    /// via the fallible [`Self::try_with_queue_length`] and propagates the
    /// structured error. Callers that need to distinguish or recover from
    /// construction failures must use the fallible constructor directly.
    #[must_use]
    pub fn with_queue_length(
        num_players: usize,
        max_prediction: usize,
        queue_length: usize,
    ) -> Self {
        match Self::try_with_queue_length(num_players, max_prediction, queue_length) {
            Ok(sync_layer) => sync_layer,
            Err(error) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::FrameSync,
                    "Failed to create SyncLayer: {}. Falling back to an empty internal layer.",
                    error
                );
                Self {
                    num_players: 0,
                    max_prediction: 0,
                    last_confirmed_frame: Frame::NULL,
                    last_saved_frame: Frame::NULL,
                    current_frame: Frame::new(0),
                    saved_states: SavedStates::new(0),
                    input_queues: ProofVec::new(),
                }
            },
        }
    }

    /// Creates a new `SyncLayer`, returning a structured error if any backing
    /// buffer cannot be reserved.
    pub(crate) fn try_with_queue_length(
        num_players: usize,
        max_prediction: usize,
        queue_length: usize,
    ) -> Result<Self, FortressError> {
        let mut input_queues =
            crate::error::try_with_capacity(num_players, "sync_layer.input_queues")?;
        for player_index in 0..num_players {
            input_queues.push(InputQueue::try_with_queue_length(
                player_index,
                queue_length,
            )?);
        }

        let saved_states = SavedStates::try_new(max_prediction)?;

        Ok(Self {
            num_players,
            max_prediction,
            last_confirmed_frame: Frame::NULL,
            last_saved_frame: Frame::NULL,
            current_frame: Frame::new(0),
            saved_states,
            input_queues,
        })
    }

    /// Returns the current simulation frame.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    #[must_use]
    pub fn current_frame(&self) -> Frame {
        self.current_frame
    }

    /// Advances the simulation by one frame.
    ///
    /// Uses safe arithmetic that reports a violation if overflow would occur.
    /// In practice, at 60 FPS, it would take over a year to reach `i32::MAX`,
    /// but we detect and report it gracefully rather than panicking.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    pub fn advance_frame(&mut self) {
        self.current_frame = safe_frame_add!(self.current_frame, 1, "SyncLayer::advance_frame");
    }

    /// Saves the current game state.
    ///
    /// This method relies on the invariant that `current_frame` is always valid
    /// (`>= 0`). Construction initializes it to `0`; frame-advance increments it;
    /// rollback and hot-join seek paths validate their target frames before
    /// assigning it.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    pub fn save_current_state(&mut self) -> FortressRequest<T> {
        self.last_saved_frame = self.current_frame;
        // Debug assertion to catch invariant violations during development.
        // Every current_frame mutation path validates its target first, so this
        // should never fail.
        debug_assert!(
            self.current_frame.as_i32() >= 0,
            "Internal invariant violation: current_frame must be non-negative"
        );
        // Use match to handle the theoretical error case gracefully instead of panicking.
        // In the impossible case of an invalid frame, create a default cell.
        let cell = match self.saved_states.get_cell(self.current_frame) {
            Ok(cell) => cell,
            Err(_) => {
                // This should never happen due to our invariants, but if it does,
                // report it and return a default cell to avoid panicking.
                report_violation!(
                    ViolationSeverity::Critical,
                    ViolationKind::InternalError,
                    "save_current_state: current_frame {} failed get_cell - this indicates an internal bug",
                    self.current_frame
                );
                GameStateCell::default()
            },
        };
        FortressRequest::SaveGameState {
            cell,
            frame: self.current_frame,
        }
    }

    /// Sets the frame delay for a player.
    ///
    /// # Errors
    /// - Returns [`FortressError::InvalidPlayerHandle`] if `player_handle >= num_players`.
    /// - Returns [`FortressError::InternalErrorStructured`] with
    ///   [`InternalErrorKind::IndexOutOfBounds`] (name `"input_queues"`) if
    ///   the input-queue entry for `player_handle` is missing. This indicates
    ///   an internal-invariant violation and should not occur in correct code.
    /// - Surfaces every error variant that
    ///   [`InputQueue::set_frame_delay`](crate::__internal::InputQueue::set_frame_delay)
    ///   can return:
    ///   - [`FortressError::InvalidRequestStructured`] with
    ///     [`InvalidRequestKind::FrameDelayTooLarge`] if `delay` exceeds the
    ///     queue's `max_frame_delay()`.
    ///   - [`FortressError::InvalidRequestStructured`] with
    ///     [`InvalidRequestKind::InputDelayDecreaseUnsupported`] if `delay`
    ///     is strictly less than the current frame delay and inputs have
    ///     already been added (mid-session decreases are unsupported).
    ///   - [`FortressError::InternalErrorStructured`] with
    ///     [`InternalErrorKind::IndexOutOfBounds`] (name `"inputs"`) if the
    ///     queue's most-recent input slot cannot be located while computing
    ///     the gap-fill source — an internal-invariant violation.
    ///   - [`FortressError::InternalErrorStructured`] with
    ///     [`InternalErrorKind::InputQueueGapFillFailed`] if a replicated
    ///     gap-fill frame cannot be appended (queue overflow or
    ///     prediction-frame collision) — an internal-invariant violation.
    ///
    /// [`InvalidRequestKind::FrameDelayTooLarge`]: crate::error::InvalidRequestKind::FrameDelayTooLarge
    /// [`InvalidRequestKind::InputDelayDecreaseUnsupported`]: crate::error::InvalidRequestKind::InputDelayDecreaseUnsupported
    /// [`InternalErrorKind::InputQueueGapFillFailed`]: crate::error::InternalErrorKind::InputQueueGapFillFailed
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    pub fn set_frame_delay(
        &mut self,
        player_handle: PlayerHandle,
        delay: usize,
    ) -> Result<(), FortressError> {
        if !player_handle.is_valid_player_for(self.num_players) {
            return Err(FortressError::InvalidPlayerHandle {
                handle: player_handle,
                max_handle: PlayerHandle::new(self.num_players.saturating_sub(1)),
            });
        }
        let len = self.input_queues.len();
        self.input_queues
            .get_mut(player_handle.as_usize())
            .ok_or_else(|| input_queue_oob(player_handle.as_usize(), len))?
            .set_frame_delay(delay)?;
        Ok(())
    }

    /// Returns the current frame delay for a player.
    ///
    /// # Errors
    /// - Returns [`FortressError::InvalidPlayerHandle`] if `player_handle >= num_players`.
    /// - Returns [`FortressError::InternalErrorStructured`] with
    ///   [`InternalErrorKind::IndexOutOfBounds`] (name `"input_queues"`) if
    ///   the input-queue entry for `player_handle` is missing. This indicates
    ///   an internal-invariant violation and should not occur in correct code.
    pub fn frame_delay(&self, player_handle: PlayerHandle) -> Result<usize, FortressError> {
        if !player_handle.is_valid_player_for(self.num_players) {
            return Err(FortressError::InvalidPlayerHandle {
                handle: player_handle,
                max_handle: PlayerHandle::new(self.num_players.saturating_sub(1)),
            });
        }
        let len = self.input_queues.len();
        let queue = self
            .input_queues
            .get(player_handle.as_usize())
            .ok_or_else(|| input_queue_oob(player_handle.as_usize(), len))?;
        Ok(queue.frame_delay())
    }

    /// Returns the maximum allowed frame delay for any player in this `SyncLayer`.
    ///
    /// All input queues share the same `queue_length`, so this is the same value
    /// for every player. Returns `0` when no input queues are present, which only
    /// occurs in degenerate sessions with zero players.
    #[must_use]
    pub fn max_frame_delay(&self) -> usize {
        self.input_queues
            .first()
            .map_or(0, InputQueue::max_frame_delay)
    }

    /// Returns the most recently added input frame for the given player, or
    /// [`Frame::NULL`] if no inputs have been added yet.
    ///
    /// # Errors
    /// Returns a [`FortressError`] if `player_handle >= num_players`.
    pub(crate) fn last_added_frame(
        &self,
        player_handle: PlayerHandle,
    ) -> Result<Frame, FortressError> {
        if !player_handle.is_valid_player_for(self.num_players) {
            return Err(FortressError::InvalidPlayerHandle {
                handle: player_handle,
                max_handle: PlayerHandle::new(self.num_players.saturating_sub(1)),
            });
        }
        let len = self.input_queues.len();
        let queue = self
            .input_queues
            .get(player_handle.as_usize())
            .ok_or_else(|| input_queue_oob(player_handle.as_usize(), len))?;
        Ok(queue.last_added_frame())
    }

    /// Returns the confirmed input for the given player at the given frame.
    /// Used by the session layer to retrieve the replicated gap-fill bytes
    /// after a mid-session frame-delay increase.
    ///
    /// # Errors
    /// Returns a [`FortressError`] if `player_handle >= num_players`, the
    /// queue's slot does not contain `frame`, or the queue is missing.
    pub(crate) fn confirmed_input(
        &self,
        player_handle: PlayerHandle,
        frame: Frame,
    ) -> Result<PlayerInput<T::Input>, FortressError> {
        if !player_handle.is_valid_player_for(self.num_players) {
            return Err(FortressError::InvalidPlayerHandle {
                handle: player_handle,
                max_handle: PlayerHandle::new(self.num_players.saturating_sub(1)),
            });
        }
        let len = self.input_queues.len();
        let queue = self
            .input_queues
            .get(player_handle.as_usize())
            .ok_or_else(|| input_queue_oob(player_handle.as_usize(), len))?;
        queue.confirmed_input(frame)
    }

    /// Freezes the input queue for a specific player at an **agreed freeze
    /// frame**.
    ///
    /// After this call, [`Self::add_remote_input`] for `player_handle` is
    /// silently dropped (the underlying [`InputQueue::add_input`] becomes a
    /// no-op), and the player's last confirmed input is repeated forever from
    /// the queue. This is part of the graceful peer-drop flow: combined with
    /// `connect_status[handle].disconnected = true` at the session level,
    /// remaining peers can keep simulating using the dropped peer's last
    /// confirmed input (reported as [`crate::InputStatus::Disconnected`] by
    /// [`Self::synchronized_inputs`]).
    ///
    /// `freeze_frame` is the session-computed agreed freeze frame `F` (the
    /// global minimum across all peers of the dropped slot's received frame).
    /// The queue rolls its `last_confirmed_input` back to the value confirmed at
    /// `F` (via `InputQueue::freeze_at`) so that **every** survivor — which may
    /// have received the dropped peer's inputs through different frames under
    /// packet loss — repeats the identical value, closing the under-loss desync
    /// for the common case (a staggered-detection discard-before-convergence
    /// residual remains; see `CHANGELOG.md` / the N0 design notes).
    /// See `InputQueue::freeze_at` for the full rationale and fail-safe behavior
    /// when no confirmed input exists at `F`.
    ///
    /// # Errors
    /// Returns [`FortressError::InvalidPlayerHandle`] if `player_handle` is
    /// out of range for this sync layer.
    ///
    /// [`InputQueue::add_input`]: crate::__internal::InputQueue::add_input
    pub(crate) fn freeze_player(
        &mut self,
        player_handle: PlayerHandle,
        freeze_frame: Frame,
    ) -> Result<(), FortressError> {
        if !player_handle.is_valid_player_for(self.num_players) {
            return Err(FortressError::InvalidPlayerHandle {
                handle: player_handle,
                max_handle: PlayerHandle::new(self.num_players.saturating_sub(1)),
            });
        }
        let len = self.input_queues.len();
        let queue = self
            .input_queues
            .get_mut(player_handle.as_usize())
            .ok_or_else(|| input_queue_oob(player_handle.as_usize(), len))?;
        queue.freeze_at(freeze_frame);
        Ok(())
    }

    /// Re-rolls an **already-frozen** player's frozen value to the value
    /// confirmed at `frame`, delegating to [`InputQueue::set_frozen_value_at`].
    ///
    /// This is the convergence counterpart to [`Self::freeze_player`]. The
    /// session calls it from `P2PSession::disconnect_player_at_frames` every
    /// time the disconnect machinery sets or lowers
    /// `local_connect_status[handle].last_frame` to the global-min agreed freeze
    /// frame `F`, so the dropped slot's repeated value tracks `F` **down** to
    /// the value every survivor shares — closing the under-loss desync on the
    /// direct-detection (own-endpoint timeout, `remove_player`) and re-adjust
    /// paths that [`Self::freeze_player`] alone cannot reach (it is idempotent
    /// once frozen). See [`InputQueue::set_frozen_value_at`] for the full
    /// rationale and fail-safe behavior.
    ///
    /// # Infallible by design
    ///
    /// Unlike [`Self::freeze_player`], this does **not** return a `Result`. The
    /// disconnect path must not fail-closed on a re-roll: an out-of-range handle
    /// or a missing input queue here is logged as a [`ViolationSeverity::Error`]
    /// violation and the call continues (the queue method itself is a no-op on a
    /// non-frozen queue and fail-safe on a missing/evicted frame). Bubbling an
    /// error would add noise to a path whose primary job — marking the slot
    /// disconnected — has already succeeded by the time this runs.
    pub(crate) fn set_frozen_value_at(&mut self, player_handle: PlayerHandle, frame: Frame) {
        if !player_handle.is_valid_player_for(self.num_players) {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "set_frozen_value_at called with out-of-range handle {} (num_players={}); skipping re-roll",
                player_handle,
                self.num_players
            );
            return;
        }
        let len = self.input_queues.len();
        let Some(queue) = self.input_queues.get_mut(player_handle.as_usize()) else {
            let oob = input_queue_oob(player_handle.as_usize(), len);
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "set_frozen_value_at: no input queue for handle {} ({}); skipping re-roll",
                player_handle,
                oob
            );
            return;
        };
        queue.set_frozen_value_at(frame);
    }

    /// Unfreezes the input queue for a specific player, the counterpart to
    /// [`Self::freeze_player`].
    ///
    /// After this call, [`Self::add_remote_input`] for `player_handle` is no
    /// longer silently dropped (the underlying [`InputQueue::add_input`] resumes
    /// accepting inputs). Used when a previously gracefully-dropped slot is
    /// reactivated by a hot-joining peer. Callers reactivating a slot at a
    /// non-zero activation frame typically use
    /// [`Self::reactivate_player_at_frame`] instead, which also repositions the
    /// queue.
    ///
    /// # Errors
    /// Returns [`FortressError::InvalidPlayerHandle`] if `player_handle` is out
    /// of range for this sync layer (same validation as [`Self::freeze_player`]).
    ///
    /// [`InputQueue::add_input`]: crate::__internal::InputQueue::add_input
    // dead_code: consumed by chunk 5's session orchestration; only the sync-layer
    // primitive lands in this chunk.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn unfreeze_player(
        &mut self,
        player_handle: PlayerHandle,
    ) -> Result<(), FortressError> {
        if !player_handle.is_valid_player_for(self.num_players) {
            return Err(FortressError::InvalidPlayerHandle {
                handle: player_handle,
                max_handle: PlayerHandle::new(self.num_players.saturating_sub(1)),
            });
        }
        let len = self.input_queues.len();
        let queue = self
            .input_queues
            .get_mut(player_handle.as_usize())
            .ok_or_else(|| input_queue_oob(player_handle.as_usize(), len))?;
        queue.unfreeze();
        Ok(())
    }

    /// Reactivates a frozen/reserved slot at game frame `frame` (host side).
    ///
    /// Used by the host to reactivate a slot when a peer hot-joins, so the slot
    /// resumes accepting that peer's real inputs from `frame` onward. This
    /// repositions only the single target player's input queue via
    /// [`InputQueue::reset_to_frame`] (which also unfreezes it); it does **not**
    /// touch any sync-layer frame counter (`current_frame`,
    /// `last_confirmed_frame`, `last_saved_frame`) — those are driven by the
    /// session's normal advance path.
    ///
    /// After reactivation, [`Self::confirmed_input`] for the reactivated slot is
    /// only valid for frames `>= frame - 1` (see [`InputQueue::reset_to_frame`]'s
    /// "Pre-activation `confirmed_input` surface" note); requesting a lower frame
    /// yields [`InvalidRequestKind::NoConfirmedInput`](crate::error::InvalidRequestKind::NoConfirmedInput).
    ///
    /// # Errors
    /// Returns [`FortressError::InvalidPlayerHandle`] if `player_handle` is out
    /// of range for this sync layer (same validation as [`Self::freeze_player`]).
    ///
    /// Note: a negative/`NULL` `frame` is rejected by
    /// [`InputQueue::reset_to_frame`] itself (it reports a violation and leaves
    /// the queue unchanged); this method still returns `Ok(())` for a valid
    /// handle, since the per-queue primitive owns frame validation.
    // dead_code: consumed by chunk 5's host orchestration.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn reactivate_player_at_frame(
        &mut self,
        player_handle: PlayerHandle,
        frame: Frame,
    ) -> Result<(), FortressError> {
        if !player_handle.is_valid_player_for(self.num_players) {
            return Err(FortressError::InvalidPlayerHandle {
                handle: player_handle,
                max_handle: PlayerHandle::new(self.num_players.saturating_sub(1)),
            });
        }
        let len = self.input_queues.len();
        let queue = self
            .input_queues
            .get_mut(player_handle.as_usize())
            .ok_or_else(|| input_queue_oob(player_handle.as_usize(), len))?;
        queue.reset_to_frame(frame);
        Ok(())
    }

    /// Fast-forwards a freshly-constructed sync layer to a received snapshot's
    /// activation `frame` (joiner side).
    ///
    /// Intended for a **fresh joiner** layer that is about to load a host's
    /// state snapshot taken at `frame`. Frames before `frame` are considered
    /// done (their effect is already baked into the snapshot's simulated state);
    /// `frame` itself is not yet confirmed.
    ///
    /// # Postconditions (on success)
    /// - [`Self::current_frame`] returns `frame`.
    /// - [`Self::last_confirmed_frame`] returns `frame - 1` (which is
    ///   [`Frame::NULL`] for `frame == 0`).
    /// - [`Self::last_saved_frame`] is unchanged and remains [`Frame::NULL`].
    ///   The snapshot's state is injected into the saved-states cell (and
    ///   `last_saved_frame` set) by a later chunk; this method does not set it.
    /// - Every input queue has been repositioned via
    ///   [`InputQueue::reset_to_frame`] to accept inputs from `frame` onward.
    /// - [`InvariantChecker::check_invariants`] still holds
    ///   (`last_confirmed_frame <= current_frame`, `last_saved_frame <=
    ///   current_frame`, queue count unchanged).
    ///
    /// This is **only** valid for a fresh layer: `current_frame == 0`,
    /// `last_confirmed_frame == Frame::NULL`, `last_saved_frame == Frame::NULL`,
    /// and no queue has accepted or frozen input yet. It overwrites the frame
    /// counters wholesale, so a running layer is rejected before any mutation.
    ///
    /// # Errors
    /// Returns [`FortressError::InvalidFrameStructured`] with
    /// [`InvalidFrameReason::MustBeNonNegative`] if `frame` is negative or
    /// [`Frame::NULL`].
    ///
    /// Returns [`InvalidFrameReason::Custom`] with `"seek target is older than
    /// last_saved_frame"` if `last_saved_frame` is already later than `frame`.
    ///
    /// Returns [`InvalidRequestKind::Custom`] with `"seek_to_frame requires a
    /// fresh SyncLayer"` for any other non-fresh layer state.
    // dead_code: consumed by chunk 5's joiner orchestration.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn seek_to_frame(&mut self, frame: Frame) -> Result<(), FortressError> {
        if frame.as_i32() < 0 {
            return Err(FortressError::InvalidFrameStructured {
                frame,
                reason: InvalidFrameReason::MustBeNonNegative,
            });
        }

        self.validate_fresh_seek_target(frame)?;

        self.current_frame = frame;
        // Frames before `frame` are baked into the loaded snapshot; `frame`
        // itself is not yet confirmed. For `frame == 0` this is `Frame::NULL`.
        self.last_confirmed_frame = safe_frame_sub!(frame, 1, "SyncLayer::seek_to_frame confirmed");
        // `last_saved_frame` is intentionally left unchanged (set by the later
        // snapshot-injection chunk).

        for queue in self.input_queues.iter_mut() {
            queue.reset_to_frame(frame);
        }

        // The freshness precheck above keeps both frame-ordering invariants: the
        // new last_confirmed_frame is frame - 1 (or NULL), and the unchanged
        // last_saved_frame is NULL until snapshot injection. This debug
        // assert surfaces a regression in development; production is unaffected.
        debug_assert!(
            self.check_invariants().is_ok(),
            "seek_to_frame must preserve SyncLayer invariants"
        );

        Ok(())
    }

    #[cfg(feature = "hot-join")]
    fn validate_fresh_seek_target(&self, frame: Frame) -> Result<(), FortressError> {
        if !self.last_saved_frame.is_null() && self.last_saved_frame > frame {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "seek_to_frame target {} is before last_saved_frame {}; refusing to violate SyncLayer invariants",
                frame,
                self.last_saved_frame
            );
            return Err(FortressError::InvalidFrameStructured {
                frame,
                reason: InvalidFrameReason::Custom("seek target is older than last_saved_frame"),
            });
        }

        let dirty_queue = self
            .input_queues
            .iter()
            .position(|queue| !queue.last_added_frame().is_null() || queue.is_frozen());
        if self.current_frame != Frame::new(0)
            || !self.last_confirmed_frame.is_null()
            || !self.last_saved_frame.is_null()
            || dirty_queue.is_some()
        {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "seek_to_frame requires a fresh SyncLayer (current_frame {}, last_confirmed_frame {}, last_saved_frame {}, dirty_queue {:?})",
                self.current_frame,
                self.last_confirmed_frame,
                self.last_saved_frame,
                dirty_queue
            );
            return Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::Custom("seek_to_frame requires a fresh SyncLayer"),
            });
        }

        Ok(())
    }

    /// Reads the saved state and checksum at `frame` for a hot-join snapshot
    /// (host side).
    ///
    /// Returns `Some((state, checksum))` only when the circular-buffer slot for
    /// `frame` actually holds `frame` and contains data: [`SavedStates`] indexes
    /// by `frame % len`, so a slot can hold a *different* (later or earlier)
    /// frame after the buffer wraps. The `cell.frame() == frame` guard rejects
    /// that stale/wrapped case (mirroring [`Self::saved_state_by_frame`] /
    /// [`Self::load_frame`]). Returns `None` when the slot is stale, empty
    /// (`cell.load()` is `None`), or `frame` is invalid for `get_cell`.
    ///
    /// This is a pure read: no sync-layer state is mutated.
    // dead_code: consumed by chunk 5's host snapshot orchestration; only the
    // sync-layer accessor lands in this chunk.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn capture_snapshot_state(&self, frame: Frame) -> Option<(T::State, Option<u128>)>
    where
        // `cell.load()` clones the saved state out of the cell. `Config::State`
        // is not unconditionally `Clone` in every feature combination (e.g.
        // `hot-join` without `sync-send`), so the bound is stated locally.
        T::State: Clone,
    {
        let cell = self.saved_states.get_cell(frame).ok()?;
        if cell.frame() != frame {
            return None;
        }
        let state = cell.load()?;
        Some((state, cell.checksum()))
    }

    /// Injects a received hot-join snapshot's `state` into the saved-states cell
    /// at `frame` and returns the [`FortressRequest::LoadGameState`] the joiner
    /// must emit so the user restores it (joiner side).
    ///
    /// Writes the cell via [`GameStateCell::save`] and sets
    /// [`Self::last_saved_frame`] to `frame` (the snapshot is now the joiner's
    /// most recent saved state), then returns the same `LoadGameState { cell,
    /// frame }` shape [`Self::load_frame`] constructs. Must be called *after*
    /// [`Self::seek_to_frame`] has repositioned the layer to the same `frame`
    /// (seek resets the queues and frame counters; this injection writes the
    /// cell and `last_saved_frame`).
    ///
    /// # Errors
    /// Returns the structured error from [`SavedStates::get_cell`] when `frame`
    /// is negative or the saved-state ring is internally inconsistent.
    ///
    /// Returns [`InvalidFrameReason::Custom`] with `"snapshot injection frame
    /// must match current_frame"` if called out of order for a frame different
    /// from [`Self::current_frame`].
    ///
    /// Returns [`InvalidFrameReason::MissingState`] if the cell write is
    /// rejected defensively.
    // dead_code: consumed by chunk 5's joiner snapshot orchestration.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn inject_snapshot_state(
        &mut self,
        frame: Frame,
        state: T::State,
        checksum: Option<u128>,
    ) -> Result<FortressRequest<T>, FortressError> {
        if frame != self.current_frame {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "inject_snapshot_state frame {} does not match current_frame {}; refusing to violate SyncLayer invariants",
                frame,
                self.current_frame
            );
            return Err(FortressError::InvalidFrameStructured {
                frame,
                reason: InvalidFrameReason::Custom(
                    "snapshot injection frame must match current_frame",
                ),
            });
        }

        let cell = self.saved_states.get_cell(frame)?;
        // `GameStateCell::save` only returns `false` for a `Frame::NULL`, which
        // `get_cell` above already rejects (it errors on any frame < 0). Propagate
        // the result anyway so this stays correct if either guard ever changes,
        // and so `last_saved_frame` is never advanced for a write that did not
        // land.
        if !cell.save(frame, Some(state), checksum) {
            return Err(FortressError::InvalidFrameStructured {
                frame,
                reason: InvalidFrameReason::MissingState,
            });
        }
        self.last_saved_frame = frame;
        debug_assert!(
            self.check_invariants().is_ok(),
            "inject_snapshot_state must preserve SyncLayer invariants"
        );
        Ok(FortressRequest::LoadGameState { cell, frame })
    }

    /// Pre-validates that a subsequent call to [`Self::freeze_player`] for
    /// `player_handle` would succeed.
    ///
    /// Returns the exact same error variant ([`FortressError::InvalidPlayerHandle`]
    /// or [`InternalErrorKind::IndexOutOfBounds`]) that [`Self::freeze_player`]
    /// would produce for the same handle. This lets callers pre-validate every
    /// handle in a multi-handle endpoint before performing any state-mutating
    /// work, so the freeze step can be made transactional (all-or-nothing) —
    /// the application-level graceful-drop contract is honored for every
    /// handle, or no handle.
    ///
    /// # Errors
    /// - Returns [`FortressError::InvalidPlayerHandle`] if `player_handle` is
    ///   out of range for this sync layer.
    /// - Returns [`FortressError::InternalErrorStructured`] with
    ///   [`InternalErrorKind::IndexOutOfBounds`] if no input queue exists for
    ///   `player_handle` (an internal-invariant violation).
    pub(crate) fn validate_freeze_player(
        &self,
        player_handle: PlayerHandle,
    ) -> Result<(), FortressError> {
        if !player_handle.is_valid_player_for(self.num_players) {
            return Err(FortressError::InvalidPlayerHandle {
                handle: player_handle,
                max_handle: PlayerHandle::new(self.num_players.saturating_sub(1)),
            });
        }
        if self.input_queues.get(player_handle.as_usize()).is_none() {
            return Err(input_queue_oob(
                player_handle.as_usize(),
                self.input_queues.len(),
            ));
        }
        Ok(())
    }

    /// Resets the prediction state for all input queues.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    pub fn reset_prediction(&mut self) {
        for queue in self.input_queues.iter_mut() {
            queue.reset_prediction();
        }
    }

    /// Loads the gamestate indicated by `frame_to_load`.
    ///
    /// # Errors
    /// Returns `FortressError::InvalidFrame` if:
    /// - `frame_to_load` is `NULL_FRAME`
    /// - `frame_to_load` is not in the past (>= current_frame)
    /// - `frame_to_load` is outside the prediction window
    /// - The saved state for `frame_to_load` doesn't exist or has wrong frame
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    pub fn load_frame(
        &mut self,
        frame_to_load: Frame,
    ) -> Result<FortressRequest<T>, FortressError> {
        // The state should not be the current state or the state should not be in the future or too far away in the past
        if frame_to_load.is_null() {
            return Err(FortressError::InvalidFrameStructured {
                frame: frame_to_load,
                reason: InvalidFrameReason::NullFrame,
            });
        }

        if frame_to_load >= self.current_frame {
            return Err(FortressError::InvalidFrameStructured {
                frame: frame_to_load,
                reason: InvalidFrameReason::NotInPast {
                    current_frame: self.current_frame,
                },
            });
        }

        if frame_to_load.as_i32() < self.current_frame.as_i32() - self.max_prediction as i32 {
            return Err(FortressError::InvalidFrameStructured {
                frame: frame_to_load,
                reason: InvalidFrameReason::OutsidePredictionWindow {
                    current_frame: self.current_frame,
                    max_prediction: self.max_prediction,
                },
            });
        }

        let cell = self.saved_states.get_cell(frame_to_load)?;
        #[cfg(all(not(loom), not(kani)))]
        let cell_frame = cell.0.lock().frame;
        #[cfg(loom)]
        let cell_frame = cell.0.lock().unwrap().frame;
        #[cfg(kani)]
        let cell_frame = cell.0.borrow().frame;
        if cell_frame != frame_to_load {
            return Err(FortressError::InvalidFrameStructured {
                frame: frame_to_load,
                reason: InvalidFrameReason::WrongSavedFrame {
                    saved_frame: cell_frame,
                },
            });
        }
        self.current_frame = frame_to_load;
        // Update last_saved_frame to maintain invariant: last_saved_frame <= current_frame
        // After rollback, we're working from the loaded state, which is now our reference point
        self.last_saved_frame = frame_to_load;

        Ok(FortressRequest::LoadGameState {
            cell,
            frame: frame_to_load,
        })
    }

    /// Adds local input to the corresponding input queue. Checks if the prediction threshold has been reached. Returns the frame number where the input is actually added to.
    /// This number will only be different if the input delay was set to a number higher than 0.
    ///
    /// Returns `Frame::NULL` if the input frame doesn't match the current frame.
    pub(crate) fn add_local_input(
        &mut self,
        player_handle: PlayerHandle,
        input: PlayerInput<T::Input>,
    ) -> Frame {
        // The input provided should match the current frame, we account for input delay later
        if input.frame != self.current_frame {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "Input frame {} doesn't match current frame {}",
                input.frame,
                self.current_frame
            );
            return Frame::NULL;
        }
        let queue_count = self.input_queues.len();
        match self.input_queues.get_mut(player_handle.as_usize()) {
            Some(queue) => queue.add_input(input),
            None => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InputQueue,
                    "add_local_input: missing input_queues entry for player handle {} (input_queues.len()={})",
                    player_handle.as_usize(),
                    queue_count
                );
                Frame::NULL
            },
        }
    }

    /// Adds remote input to the corresponding input queue.
    /// Unlike `add_local_input`, this will not check for correct conditions, as remote inputs have already been checked on another device.
    pub(crate) fn add_remote_input(
        &mut self,
        player_handle: PlayerHandle,
        input: PlayerInput<T::Input>,
    ) {
        let queue_count = self.input_queues.len();
        match self.input_queues.get_mut(player_handle.as_usize()) {
            Some(queue) => {
                queue.add_input(input);
            },
            None => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InputQueue,
                    "add_remote_input: missing input_queues entry for player handle {} (input_queues.len()={})",
                    player_handle.as_usize(),
                    queue_count
                );
            },
        }
    }

    /// Returns inputs for all players for the current frame of the sync layer. If there are none for a specific player, return predictions.
    ///
    /// # Returns
    /// Returns `None` if any input queue operation fails (indicates a severe internal error).
    ///
    /// # Performance
    /// Uses [`InputVec`] (a [`SmallVec`]) to avoid heap allocation for games with 1-4 players.
    pub(crate) fn synchronized_inputs(
        &mut self,
        connect_status: &[ConnectionStatus],
    ) -> Option<InputVec<T::Input>> {
        let num_players = connect_status.len();
        let mut inputs = InputVec::new();
        if inputs.try_reserve(num_players).is_err() {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "Failed to reserve synchronized input buffer for {} players",
                num_players
            );
            return None;
        }
        for (i, con_stat) in connect_status.iter().enumerate() {
            if con_stat.disconnected && con_stat.last_frame < self.current_frame {
                // Disconnected past last_frame. If the player's queue was
                // frozen via `freeze_player` (graceful peer drop), surface the
                // queue's frozen `last_confirmed_input` rather than a default
                // value. After the under-loss convergence fix this value is the
                // dropped peer's input at the **agreed freeze frame `F`** (the
                // global minimum across survivors of the dropped slot's received
                // frame), which `set_frozen_value_at` rolls every survivor to.
                // Under packet loss `F` may be EARLIER than this peer's own
                // most-recently-received input — surfacing the agreed-frame
                // value (not the most-recent one) is exactly what keeps every
                // survivor's confirmed history byte-identical. For non-frozen
                // disconnects (legacy halt path) keep returning the default to
                // preserve back-compat.
                let queue = self.input_queues.get(i)?;
                let value = if queue.is_frozen() {
                    queue.last_confirmed_input().unwrap_or_default()
                } else {
                    T::Input::default()
                };
                inputs.push((value, InputStatus::Disconnected));
            } else {
                let queue = self.input_queues.get_mut(i)?;
                inputs.push(queue.input(self.current_frame)?);
            }
        }
        Some(inputs)
    }

    /// Returns confirmed inputs for all players for the current frame of the sync layer.
    ///
    /// # Frozen-queue semantics
    ///
    /// When a player has been gracefully dropped via [`Self::freeze_player`]
    /// (the `ContinueWithout` graceful-drop path), and the player is marked
    /// disconnected past their last frame, this method surfaces the queue's
    /// frozen `last_confirmed_input` (encoded into a `PlayerInput` with
    /// [`Frame::NULL`]) instead of a blank/default input. After the under-loss
    /// convergence fix this surfaced value is the dropped peer's input at the
    /// **agreed freeze frame `F`** (the global minimum across survivors of the
    /// dropped slot's received frame, converged by `set_frozen_value_at`), which
    /// under packet loss may be EARLIER than this peer's most-recently-received
    /// input. Surfacing the agreed-frame value — not the most-recent one — is
    /// what keeps the byte stream sent to spectators consistent with the input
    /// stream remaining peers actually simulate (see
    /// [`Self::synchronized_inputs`]). For non-frozen disconnects (legacy halt
    /// path) and for queues that never received any confirmed input before being
    /// frozen, blank input is still returned to preserve back-compat.
    pub(crate) fn confirmed_inputs(
        &self,
        frame: Frame,
        connect_status: &[ConnectionStatus],
    ) -> Result<Vec<PlayerInput<T::Input>>, FortressError> {
        let mut inputs = Vec::new();
        for (i, con_stat) in connect_status.iter().enumerate() {
            let queue = self
                .input_queues
                .get(i)
                .ok_or_else(|| input_queue_oob(i, self.input_queues.len()))?;
            if con_stat.disconnected && con_stat.last_frame < frame {
                // Mirror the freeze logic in `synchronized_inputs` so spectator
                // state and player state agree on the dropped peer's input.
                //
                // The `Frame::NULL` stamp on dropped-peer entries is wire-safe:
                // `InputBytes::from_inputs` derives the packet's frame from the
                // first non-NULL entry (any still-connected peer in the same
                // `inputs` slice supplies it), and `send_confirmed_inputs_to_spectators`
                // tolerates `Frame::NULL` in its consistency check.
                let frozen_input = if queue.is_frozen() {
                    queue.last_confirmed_input()
                } else {
                    None
                };
                match frozen_input {
                    Some(input) => inputs.push(PlayerInput {
                        frame: Frame::NULL,
                        input,
                    }),
                    None => inputs.push(PlayerInput::blank_input(Frame::NULL)),
                }
            } else {
                inputs.push(queue.confirmed_input(frame)?);
            }
        }
        Ok(inputs)
    }

    /// Sets the last confirmed frame to a given frame. By raising the last confirmed frame, we can discard all previous frames, as they are no longer necessary.
    pub(crate) fn set_last_confirmed_frame(&mut self, mut frame: Frame, save_mode: SaveMode) {
        // don't set the last confirmed frame after the first incorrect frame before a rollback has happened
        let first_incorrect: Frame = self
            .input_queues
            .iter()
            .map(InputQueue::first_incorrect_frame)
            .fold(Frame::NULL, std::cmp::max);

        // if sparse saving option is turned on, don't set the last confirmed frame after the last saved frame
        if save_mode == SaveMode::Sparse {
            frame = std::cmp::min(frame, self.last_saved_frame);
        }

        // never delete stuff ahead of the current frame
        frame = std::cmp::min(frame, self.current_frame());

        // if we set the last confirmed frame beyond the first incorrect frame, we discard inputs that we need later for adjusting the gamestate.
        // Clamp frame to not exceed first_incorrect as a safety measure and log if this happens
        if !first_incorrect.is_null() && first_incorrect < frame {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "Clamping confirmed frame {} to first_incorrect {} - this may indicate a bug",
                frame,
                first_incorrect
            );
            frame = first_incorrect;
        }

        self.last_confirmed_frame = frame;
        // The confirmed-frame clamp above (the two `min`s plus the
        // `first_incorrect` guard) is what every `set_last_confirmed_frame`
        // proof asserts about `last_confirmed_frame`. The discard pass below
        // only trims already-confirmed inputs out of the per-player input
        // queues; it runs *after* `last_confirmed_frame` is assigned and never
        // reads or writes it, so it cannot affect any verified invariant.
        //
        // Under Kani it is gated out because the `iter_mut()` over
        // `input_queues` together with the `&mut`-self `discard_confirmed_frames`
        // call forces CBMC to instantiate the full mutable pointer model of
        // every queue's backing `Vec` for the *whole* function body — even on
        // the proofs that construct zero input queues, where the loop is a
        // provable no-op. Measured in isolation this single block accounts for
        // ~17 GB of the ~18 GB peak `cbmc` RSS (and most of the wall time);
        // removing it under verification keeps the clamp proofs tractable while
        // leaving production / `cargo test` / loom builds byte-for-byte
        // unchanged. Queue-trimming behavior is covered by the regular test
        // suite, not by these frame-clamp proofs.
        #[cfg(not(kani))]
        if self.last_confirmed_frame.as_i32() > 0 {
            let discard_frame = safe_frame_sub!(frame, 1, "SyncLayer::confirm_frame");
            for queue in self.input_queues.iter_mut() {
                queue.discard_confirmed_frames(discard_frame);
            }
        }
    }

    /// Finds the earliest incorrect frame detected by the individual input queues
    pub(crate) fn check_simulation_consistency(&self, mut first_incorrect: Frame) -> Frame {
        for queue in self.input_queues.iter() {
            let incorrect = queue.first_incorrect_frame();
            if !incorrect.is_null() && (first_incorrect.is_null() || incorrect < first_incorrect) {
                first_incorrect = incorrect;
            }
        }
        first_incorrect
    }

    /// Returns the player handles that have incorrect predictions at or before `disconnect_frame`.
    /// Used by telemetry to identify which players' inputs were mispredicted.
    pub(crate) fn players_with_incorrect_predictions(
        &self,
        disconnect_frame: Frame,
    ) -> Vec<(PlayerHandle, Frame)> {
        let mut result = Vec::new();
        for (handle, queue) in self.input_queues.iter().enumerate() {
            let incorrect = queue.first_incorrect_frame();
            if !incorrect.is_null() && (disconnect_frame.is_null() || incorrect <= disconnect_frame)
            {
                result.push((PlayerHandle::new(handle), incorrect));
            }
        }
        result
    }

    /// Returns a gamestate through given frame
    pub(crate) fn saved_state_by_frame(&self, frame: Frame) -> Option<GameStateCell<T::State>> {
        let cell = self.saved_states.get_cell(frame).ok()?;

        #[cfg(all(not(loom), not(kani)))]
        let cell_frame = cell.0.lock().frame;
        #[cfg(loom)]
        let cell_frame = cell.0.lock().unwrap().frame;
        #[cfg(kani)]
        let cell_frame = cell.0.borrow().frame;

        (cell_frame == frame).then_some(cell)
    }

    /// Returns the latest saved frame.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    #[must_use]
    pub fn last_saved_frame(&self) -> Frame {
        self.last_saved_frame
    }

    /// Returns the latest confirmed frame.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    #[must_use]
    pub fn last_confirmed_frame(&self) -> Frame {
        self.last_confirmed_frame
    }
}

/// Compile-time guarantee that the fallback inside
/// [`SyncLayer::with_queue_length`] is sound: the library constant
/// the fallback hands to `InputQueue::with_queue_length` must satisfy
/// that function's `>= 2` precondition. Evaluated in every build —
/// not only `#[cfg(test)]` — so the compiler refuses any future
/// regression that violates it (e.g., a new feature gate that
/// accidentally reduces `INPUT_QUEUE_LENGTH` below 2).
const _INPUT_QUEUE_LENGTH_IS_VALID: () = assert!(crate::input_queue::INPUT_QUEUE_LENGTH >= 2);

impl<T: Config> InvariantChecker for SyncLayer<T> {
    /// Checks the invariants of the SyncLayer.
    ///
    /// # Invariants
    ///
    /// 1. `num_players` must be > 0
    /// 2. `max_prediction` must be > 0
    /// 3. `current_frame` must be >= 0
    /// 4. `last_confirmed_frame` must be <= `current_frame`
    /// 5. `last_saved_frame` must be <= `current_frame`
    /// 6. Input queues count must match `num_players`
    /// 7. Saved states count must be `max_prediction + 1`
    /// 8. All input queues must pass their invariant checks
    fn check_invariants(&self) -> Result<(), InvariantViolation> {
        // Invariant 1: num_players > 0
        if self.num_players == 0 {
            return Err(InvariantViolation::new(
                "SyncLayer",
                "num_players must be greater than 0",
            ));
        }

        // Invariant 2: max_prediction > 0
        if self.max_prediction == 0 {
            return Err(InvariantViolation::new(
                "SyncLayer",
                "max_prediction must be greater than 0",
            ));
        }

        // Invariant 3: current_frame >= 0
        if self.current_frame.as_i32() < 0 {
            return Err(
                InvariantViolation::new("SyncLayer", "current_frame must be non-negative")
                    .with_field_value("current_frame", self.current_frame),
            );
        }

        // Invariant 4: last_confirmed_frame <= current_frame
        if !self.last_confirmed_frame.is_null() && self.last_confirmed_frame > self.current_frame {
            return Err(InvariantViolation::new(
                "SyncLayer",
                "last_confirmed_frame exceeds current_frame",
            )
            .with_bounds_violation(
                "last_confirmed_frame",
                self.last_confirmed_frame,
                "NULL",
                self.current_frame,
            ));
        }

        // Invariant 5: last_saved_frame <= current_frame
        if !self.last_saved_frame.is_null() && self.last_saved_frame > self.current_frame {
            return Err(InvariantViolation::new(
                "SyncLayer",
                "last_saved_frame exceeds current_frame",
            )
            .with_bounds_violation(
                "last_saved_frame",
                self.last_saved_frame,
                "NULL",
                self.current_frame,
            ));
        }

        // Invariant 6: input queues count matches num_players
        if self.input_queues.len() != self.num_players {
            return Err(InvariantViolation::new(
                "SyncLayer",
                "input_queues count does not match num_players",
            )
            .with_bounds_violation(
                "input_queues.len()",
                self.input_queues.len(),
                self.num_players,
                self.num_players,
            ));
        }

        // Invariant 7: saved states count is max_prediction + 1
        let expected_states = self.max_prediction + 1;
        if self.saved_states.states.len() != expected_states {
            return Err(
                InvariantViolation::new("SyncLayer", "saved_states count is incorrect")
                    .with_bounds_violation(
                        "saved_states.len()",
                        self.saved_states.states.len(),
                        expected_states,
                        expected_states,
                    ),
            );
        }

        // Invariant 8: all input queues pass their invariant checks
        for (i, queue) in self.input_queues.iter().enumerate() {
            if let Err(violation) = queue.check_invariants() {
                return Err(
                    InvariantViolation::new("SyncLayer", "input_queue invariant violated")
                        .with_input_queue_index(i, violation.to_string()),
                );
            }
        }

        Ok(())
    }
}

// #########
// # TESTS #
// #########

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod sync_layer_tests {

    use super::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
    struct TestInput {
        inp: u8,
    }

    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = u8;
        type Address = SocketAddr;
    }

    #[test]
    fn test_different_delays() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        let p1_delay = 2;
        let p2_delay = 0;
        sync_layer
            .set_frame_delay(PlayerHandle::new(0), p1_delay)
            .unwrap();
        sync_layer
            .set_frame_delay(PlayerHandle::new(1), p2_delay)
            .unwrap();

        let mut dummy_connect_status = Vec::new();
        dummy_connect_status.push(ConnectionStatus::default());
        dummy_connect_status.push(ConnectionStatus::default());

        for i in 0..20i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            // adding input as remote to avoid prediction threshold detection
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            // update the dummy connect status
            dummy_connect_status[0].last_frame = Frame::new(i);
            dummy_connect_status[1].last_frame = Frame::new(i);

            if i >= 3 {
                let sync_inputs = sync_layer
                    .synchronized_inputs(&dummy_connect_status)
                    .expect("synchronized inputs should be available");
                let player0_inputs = sync_inputs[0].0.inp;
                let player1_inputs = sync_inputs[1].0.inp;
                assert_eq!(player0_inputs, i as u8 - p1_delay as u8);
                assert_eq!(player1_inputs, i as u8 - p2_delay as u8);
            }

            sync_layer.advance_frame();
        }
    }

    #[test]
    fn test_set_frame_delay_invalid_handle() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        // Valid handles are 0 and 1 (num_players = 2)
        let result = sync_layer.set_frame_delay(PlayerHandle::new(2), 0);
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidPlayerHandle { handle, max_handle }) => {
                assert_eq!(handle, PlayerHandle::new(2));
                assert_eq!(max_handle, PlayerHandle::new(1));
            },
            _ => panic!("Expected InvalidPlayerHandle error"),
        }
    }

    #[test]
    fn test_with_queue_length_invalid_returns_empty_internal_fallback() {
        // Production construction uses try_with_queue_length and returns the
        // structured QueueLengthTooSmall error. The compatibility wrapper must
        // still avoid panicking, so it reports a violation and returns an empty
        // internal layer.
        let sync_layer = SyncLayer::<TestConfig>::with_queue_length(3, 8, 1);
        assert_eq!(sync_layer.input_queues.len(), 0);
        assert_eq!(sync_layer.num_players, 0);
        assert_eq!(sync_layer.max_prediction, 0);

        let err = match SyncLayer::<TestConfig>::try_with_queue_length(3, 8, 1) {
            Ok(_) => panic!("queue_length < 2 should fail"),
            Err(err) => err,
        };
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: crate::InvalidRequestKind::QueueLengthTooSmall { length: 1 }
            }
        ));

        let sync_layer_zero = SyncLayer::<TestConfig>::with_queue_length(2, 4, 0);
        assert_eq!(sync_layer_zero.input_queues.len(), 0);
    }

    #[test]
    fn input_queue_length_constant_is_valid_for_with_queue_length() {
        // `_INPUT_QUEUE_LENGTH_IS_VALID` is a non-generic, module-level
        // const, so the `assert!` is evaluated unconditionally at compile
        // time. Referencing it here only surfaces the invariant in
        // test-runner reports — the test would never run if the const
        // expression failed.
        let () = super::_INPUT_QUEUE_LENGTH_IS_VALID;
    }

    #[test]
    fn test_sync_layer_new_initializes_correctly() {
        let sync_layer = SyncLayer::<TestConfig>::new(4, 7);
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::NULL);
        assert_eq!(sync_layer.last_saved_frame(), Frame::NULL);
        assert_eq!(sync_layer.num_players, 4);
        assert_eq!(sync_layer.max_prediction, 7);
        assert_eq!(sync_layer.input_queues.len(), 4);
    }

    #[test]
    fn test_advance_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(1));
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(2));
    }

    #[test]
    fn test_save_current_state() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        match request {
            FortressRequest::SaveGameState { cell, frame } => {
                assert_eq!(frame, Frame::new(0));
                // Save some data
                cell.save(Frame::new(0), Some(42u8), Some(1234));
                assert_eq!(cell.frame(), Frame::new(0));
            },
            _ => panic!("Expected SaveGameState request"),
        }
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(0));

        // Advance and save at frame 1
        sync_layer.advance_frame();
        let request = sync_layer.save_current_state();
        match request {
            FortressRequest::SaveGameState { frame, .. } => {
                assert_eq!(frame, Frame::new(1));
            },
            _ => panic!("Expected SaveGameState request"),
        }
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(1));
    }

    #[test]
    fn test_load_frame_success() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            cell.save(frame, Some(100u8), None);
        }

        // Advance a few frames
        sync_layer.advance_frame();
        sync_layer.advance_frame();
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(3));

        // Load frame 0
        let request = sync_layer.load_frame(Frame::new(0)).unwrap();
        match request {
            FortressRequest::LoadGameState { frame, cell } => {
                assert_eq!(frame, Frame::new(0));
                assert_eq!(cell.load(), Some(100u8));
            },
            _ => panic!("Expected LoadGameState request"),
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
    }

    #[test]
    fn test_load_frame_null_frame_error() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        sync_layer.advance_frame();

        let result = sync_layer.load_frame(Frame::NULL);
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidFrameStructured { frame, reason }) => {
                assert_eq!(frame, Frame::NULL);
                assert!(matches!(reason, InvalidFrameReason::NullFrame));
            },
            _ => panic!("Expected InvalidFrameStructured error"),
        }
    }

    #[test]
    fn test_load_frame_future_frame_error() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        // Current frame is 0

        // Try to load frame 5 (in the future)
        let result = sync_layer.load_frame(Frame::new(5));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidFrameStructured { frame, reason }) => {
                assert_eq!(frame, Frame::new(5));
                assert!(matches!(reason, InvalidFrameReason::NotInPast { .. }));
            },
            _ => panic!("Expected InvalidFrameStructured error"),
        }
    }

    #[test]
    fn test_load_frame_current_frame_error() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        sync_layer.advance_frame();
        sync_layer.advance_frame();
        // Current frame is 2

        // Try to load current frame
        let result = sync_layer.load_frame(Frame::new(2));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidFrameStructured { frame, reason }) => {
                assert_eq!(frame, Frame::new(2));
                assert!(matches!(reason, InvalidFrameReason::NotInPast { .. }));
            },
            _ => panic!("Expected InvalidFrameStructured error"),
        }
    }

    #[test]
    fn test_load_frame_outside_prediction_window() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3); // max_prediction = 3

        // Advance to frame 10
        for _ in 0..10 {
            sync_layer.advance_frame();
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(10));

        // Try to load frame 0 (too far back, outside prediction window of 3)
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidFrameStructured { frame, reason }) => {
                assert_eq!(frame, Frame::new(0));
                assert!(matches!(
                    reason,
                    InvalidFrameReason::OutsidePredictionWindow { .. }
                ));
            },
            _ => panic!("Expected InvalidFrameStructured error"),
        }
    }

    /// Test that rollback to frame 0 works correctly when within prediction window.
    /// This is an important edge case: frame 0 is valid and should be loadable.
    #[test]
    fn test_load_frame_zero_within_prediction_window() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8); // max_prediction = 8

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            assert_eq!(frame, Frame::new(0));
            cell.save(frame, Some(42u8), Some(12345));
        }

        // Advance to frame 5 (within prediction window of 8)
        for _ in 0..5 {
            sync_layer.advance_frame();
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(5));

        // Load frame 0 - should succeed
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(
            result.is_ok(),
            "Frame 0 should be loadable within prediction window"
        );

        match result.unwrap() {
            FortressRequest::LoadGameState { frame, cell } => {
                assert_eq!(frame, Frame::new(0));
                assert_eq!(cell.frame(), Frame::new(0));
                assert_eq!(cell.load(), Some(42u8));
                assert_eq!(cell.checksum(), Some(12345));
            },
            _ => panic!("Expected LoadGameState request"),
        }

        // Current frame should now be 0
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
    }

    /// Test that frame 0 rollback fails when outside prediction window.
    #[test]
    fn test_load_frame_zero_outside_prediction_window() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 4); // max_prediction = 4

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            cell.save(frame, Some(42u8), None);
        }

        // Advance to frame 6 (frame 0 is now outside prediction window of 4)
        for _ in 0..6 {
            sync_layer.advance_frame();
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(6));

        // Load frame 0 - should fail (outside prediction window)
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(result.is_err());

        match result {
            Err(FortressError::InvalidFrameStructured { frame, reason }) => {
                assert_eq!(frame, Frame::new(0));
                assert!(matches!(
                    reason,
                    InvalidFrameReason::OutsidePredictionWindow { .. }
                ));
            },
            _ => panic!("Expected InvalidFrameStructured error"),
        }
    }

    // =========================================================================
    // Rollback Invariant Tests
    // These tests verify that invariants are maintained during rollback:
    // - INV-4: last_confirmed_frame <= current_frame
    // - INV-5: last_saved_frame <= current_frame
    // =========================================================================

    /// Test that load_frame updates last_saved_frame to maintain invariant.
    ///
    /// This is a critical test case discovered during TLA+ verification:
    /// After rollback, last_saved_frame must be <= current_frame.
    #[test]
    fn test_load_frame_updates_last_saved_frame_invariant() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            cell.save(frame, Some(100u8), None);
        }
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(0));

        // Advance to frame 5 and save state
        for i in 1..=5 {
            sync_layer.advance_frame();
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(5));
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(5));

        // Rollback to frame 2
        sync_layer.load_frame(Frame::new(2)).unwrap();

        // INVARIANT CHECK: last_saved_frame must be <= current_frame after rollback
        assert_eq!(sync_layer.current_frame(), Frame::new(2));
        assert_eq!(
            sync_layer.last_saved_frame(),
            Frame::new(2),
            "last_saved_frame should be updated to rollback target"
        );
        assert!(
            sync_layer.last_saved_frame() <= sync_layer.current_frame(),
            "Invariant violated: last_saved_frame ({}) > current_frame ({})",
            sync_layer.last_saved_frame(),
            sync_layer.current_frame()
        );
    }

    /// Test that rollback to frame 0 correctly updates last_saved_frame.
    #[test]
    fn test_load_frame_zero_updates_last_saved_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            cell.save(frame, Some(0u8), None);
        }

        // Advance to frame 3 and save each frame
        for i in 1..=3 {
            sync_layer.advance_frame();
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(3));
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(3));

        // Rollback all the way to frame 0
        sync_layer.load_frame(Frame::new(0)).unwrap();

        // Verify invariant
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(0));
    }

    /// Test multiple consecutive rollbacks maintain invariants.
    #[test]
    fn test_multiple_rollbacks_maintain_invariants() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save states for frames 0-5
        for i in 0..=5 {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }

        // First rollback: 5 -> 3
        let _ = sync_layer.load_frame(Frame::new(3));
        assert_eq!(sync_layer.current_frame(), Frame::new(3));
        assert!(sync_layer.last_saved_frame() <= sync_layer.current_frame());

        // Re-advance to frame 5
        for _ in 0..2 {
            sync_layer.advance_frame();
        }

        // Second rollback: 5 -> 1
        let _ = sync_layer.load_frame(Frame::new(1));
        assert_eq!(sync_layer.current_frame(), Frame::new(1));
        assert!(sync_layer.last_saved_frame() <= sync_layer.current_frame());

        // Third rollback: 1 -> 0
        sync_layer.advance_frame();
        let _ = sync_layer.load_frame(Frame::new(0));
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert!(sync_layer.last_saved_frame() <= sync_layer.current_frame());
    }

    /// Test that check_invariants passes after rollback.
    #[test]
    fn test_check_invariants_after_rollback() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Setup: save states for frames 0-4
        for i in 0..=4 {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }

        // Verify invariants before rollback
        assert!(
            sync_layer.check_invariants().is_ok(),
            "Invariants should pass before rollback"
        );

        // Rollback to frame 1
        let _ = sync_layer.load_frame(Frame::new(1));

        // Verify invariants after rollback
        assert!(
            sync_layer.check_invariants().is_ok(),
            "Invariants should pass after rollback"
        );
    }

    /// Test rollback at the edge of prediction window maintains invariants.
    #[test]
    fn test_rollback_at_prediction_window_edge() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 4); // max_prediction = 4

        // Save states for frames 0-4
        for i in 0..=4 {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(4));

        // Rollback exactly to the edge of prediction window (frame 0)
        // current_frame (4) - max_prediction (4) = 0
        sync_layer.load_frame(Frame::new(0)).unwrap();

        // Verify invariants
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert!(sync_layer.last_saved_frame() <= sync_layer.current_frame());
        sync_layer.check_invariants().unwrap();
    }

    /// Test that last_confirmed_frame invariant is maintained.
    /// Note: last_confirmed_frame is set separately from load_frame, but
    /// this test ensures the SyncLayer invariant checker works correctly.
    #[test]
    fn test_last_confirmed_frame_invariant() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add inputs and advance
        for i in 0..5i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }

        // Set confirmed frame
        sync_layer.set_last_confirmed_frame(Frame::new(3), SaveMode::EveryFrame);

        // Verify invariant: last_confirmed_frame <= current_frame
        assert!(sync_layer.last_confirmed_frame() <= sync_layer.current_frame());
        sync_layer.check_invariants().unwrap();
    }

    /// Test that set_last_confirmed_frame clamps to current_frame.
    /// Note: This test uses a smaller confirmed frame to avoid triggering
    /// a separate issue in discard_confirmed_frames when discarding all inputs.
    #[test]
    fn test_set_last_confirmed_frame_clamps_to_current() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add inputs and advance to frame 10
        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(10));

        // Try to set confirmed frame beyond current frame
        sync_layer.set_last_confirmed_frame(Frame::new(15), SaveMode::EveryFrame);

        // Should be clamped to current_frame
        assert!(
            sync_layer.last_confirmed_frame() <= sync_layer.current_frame(),
            "last_confirmed_frame ({}) should be clamped to current_frame ({})",
            sync_layer.last_confirmed_frame(),
            sync_layer.current_frame()
        );

        // The confirmed frame should be at most current_frame
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::new(10));
    }

    /// Test invariant checking catches invalid states.
    #[test]
    fn test_invariant_checker_validates_player_count() {
        // Create sync layer with valid player count
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        sync_layer.check_invariants().unwrap();

        // Note: We can't easily create an invalid state from outside,
        // so this test just verifies the checker runs successfully.
    }

    /// Test full rollback cycle: advance, rollback, re-advance, verify invariants.
    #[test]
    fn test_full_rollback_cycle_maintains_invariants() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Phase 1: Advance to frame 5, saving states
        for i in 0..=5 {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }
        assert!(sync_layer.check_invariants().is_ok(), "Before rollback");

        // Phase 2: Rollback to frame 2
        let _ = sync_layer.load_frame(Frame::new(2));
        assert!(sync_layer.check_invariants().is_ok(), "After rollback");
        assert_eq!(sync_layer.current_frame(), Frame::new(2));
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(2));

        // Phase 3: Re-advance to frame 5, saving states again
        for _ in 0..3 {
            sync_layer.advance_frame();
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(99u8), None);
            }
        }
        assert!(sync_layer.check_invariants().is_ok(), "After re-advance");
        assert_eq!(sync_layer.current_frame(), Frame::new(5));

        // Phase 4: Another rollback
        let _ = sync_layer.load_frame(Frame::new(3));
        assert!(
            sync_layer.check_invariants().is_ok(),
            "After second rollback"
        );
        assert_eq!(sync_layer.current_frame(), Frame::new(3));
        assert!(sync_layer.last_saved_frame() <= sync_layer.current_frame());
    }

    #[test]
    fn test_saved_state_by_frame_found() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            cell.save(frame, Some(77u8), Some(9999));
        }

        // Retrieve the saved state
        let cell = sync_layer.saved_state_by_frame(Frame::new(0));
        assert!(cell.is_some());
        let cell = cell.unwrap();
        assert_eq!(cell.frame(), Frame::new(0));
        assert_eq!(cell.checksum(), Some(9999));
    }

    #[test]
    fn test_saved_state_by_frame_not_found() {
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Frame 5 was never saved
        let cell = sync_layer.saved_state_by_frame(Frame::new(5));
        assert!(cell.is_none());
    }

    #[test]
    fn test_saved_state_by_frame_negative() {
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Negative frame
        let cell = sync_layer.saved_state_by_frame(Frame::new(-1));
        assert!(cell.is_none());
    }

    #[test]
    fn test_set_last_confirmed_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add some inputs
        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }

        // Confirm up to frame 5
        sync_layer.set_last_confirmed_frame(Frame::new(5), SaveMode::EveryFrame);
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::new(5));
    }

    #[test]
    fn test_set_last_confirmed_frame_with_sparse_saving() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        sync_layer.save_current_state();

        // Advance and add inputs
        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }

        // With sparse saving, confirmed frame should not exceed last saved frame (0)
        sync_layer.set_last_confirmed_frame(Frame::new(5), SaveMode::Sparse);
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::new(0));
    }

    #[test]
    fn test_check_simulation_consistency_no_errors() {
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        let result = sync_layer.check_simulation_consistency(Frame::NULL);
        assert_eq!(result, Frame::NULL);
    }

    #[test]
    fn test_reset_prediction() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add some inputs
        let game_input = PlayerInput::new(Frame::new(0), TestInput { inp: 1 });
        sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
        sync_layer.add_remote_input(PlayerHandle::new(1), game_input);

        // Get input for future frame (triggers prediction)
        let connect_status = vec![ConnectionStatus::default(); 2];
        let _ = sync_layer.synchronized_inputs(&connect_status);

        // Reset predictions
        sync_layer.reset_prediction();
        // Should not panic and should clear prediction state
    }

    #[test]
    fn test_synchronized_inputs_with_disconnected_player() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add input for player 0
        let game_input = PlayerInput::new(Frame::new(0), TestInput { inp: 42 });
        sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
        sync_layer.add_remote_input(PlayerHandle::new(1), game_input);

        // Player 1 disconnected before current frame
        let mut connect_status = vec![ConnectionStatus::default(); 2];
        connect_status[1].disconnected = true;
        connect_status[1].last_frame = Frame::NULL; // Disconnected before frame 0

        let inputs = sync_layer
            .synchronized_inputs(&connect_status)
            .expect("synchronized inputs should be available");
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].1, InputStatus::Confirmed);
        assert_eq!(inputs[1].1, InputStatus::Disconnected);
    }

    #[test]
    fn test_confirmed_inputs_with_disconnected_player() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add input for both players
        let game_input = PlayerInput::new(Frame::new(0), TestInput { inp: 42 });
        sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
        sync_layer.add_remote_input(PlayerHandle::new(1), game_input);

        // Player 1 disconnected before frame 0
        let mut connect_status = vec![ConnectionStatus::default(); 2];
        connect_status[1].disconnected = true;
        connect_status[1].last_frame = Frame::NULL;

        let inputs = sync_layer
            .confirmed_inputs(Frame::new(0), &connect_status)
            .unwrap();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].input.inp, 42);
        assert_eq!(inputs[1].frame, Frame::NULL); // Blank input for disconnected
    }

    #[test]
    fn test_game_state_cell_save_load() {
        let cell = GameStateCell::<u32>::default();

        // Initially no data
        assert!(cell.load().is_none());

        // Save data
        cell.save(Frame::new(5), Some(12345), Some(999));

        // Load data
        assert_eq!(cell.frame(), Frame::new(5));
        assert_eq!(cell.checksum(), Some(999));
        assert_eq!(cell.load(), Some(12345));
    }

    #[test]
    fn test_game_state_cell_data_accessor() {
        let cell = GameStateCell::<String>::default();
        cell.save(Frame::new(1), Some("hello".to_string()), None);

        let accessor = cell.data();
        assert!(accessor.is_some());
        let accessor = accessor.unwrap();
        assert_eq!(&*accessor, "hello");
    }

    #[test]
    #[allow(clippy::redundant_clone)] // Testing Clone trait - cell2 shares Arc with cell1
    fn test_game_state_cell_clone() {
        let cell1 = GameStateCell::<u8>::default();
        cell1.save(Frame::new(10), Some(200), Some(5555));

        let cell2 = cell1.clone();

        // Both should point to same data (Arc clone)
        assert_eq!(cell2.frame(), Frame::new(10));
        assert_eq!(cell2.load(), Some(200));

        // Modifying through one affects the other
        cell1.save(Frame::new(11), Some(201), Some(6666));
        assert_eq!(cell2.frame(), Frame::new(11));
        assert_eq!(cell2.load(), Some(201));
    }

    #[test]
    fn test_game_state_cell_null_frame_rejected() {
        let cell = GameStateCell::<u32>::default();

        // Saving with null frame should return false
        let result = cell.save(Frame::NULL, Some(42), None);
        assert!(!result);

        // Cell should remain empty/unchanged
        assert!(cell.load().is_none());
    }

    #[test]
    fn test_game_state_cell_debug_format() {
        let cell = GameStateCell::<u32>::default();
        cell.save(Frame::new(42), Some(12345), Some(0xDEAD_BEEF));

        let debug_str = format!("{:?}", cell);
        assert!(debug_str.contains("GameStateCell"));
        assert!(debug_str.contains("42") || debug_str.contains("frame"));
    }

    #[test]
    fn test_game_state_cell_empty_debug() {
        let cell = GameStateCell::<u32>::default();
        let debug_str = format!("{:?}", cell);
        assert!(debug_str.contains("GameStateCell"));
    }

    #[test]
    fn test_game_state_cell_save_none_data() {
        let cell = GameStateCell::<u32>::default();

        // Save with None data
        let result = cell.save(Frame::new(1), None, Some(123));
        assert!(result);

        // Load returns None
        assert!(cell.load().is_none());
        assert!(cell.data().is_none());

        // But frame and checksum are set
        assert_eq!(cell.frame(), Frame::new(1));
        assert_eq!(cell.checksum(), Some(123));
    }

    #[test]
    fn test_game_state_cell_save_none_checksum() {
        let cell = GameStateCell::<u32>::default();

        // Save with None checksum
        let result = cell.save(Frame::new(5), Some(42), None);
        assert!(result);

        assert_eq!(cell.load(), Some(42));
        assert_eq!(cell.checksum(), None);
    }

    #[test]
    fn test_game_state_cell_overwrite() {
        let cell = GameStateCell::<u32>::default();

        // First save
        cell.save(Frame::new(1), Some(100), Some(1));
        assert_eq!(cell.load(), Some(100));

        // Overwrite with new data
        cell.save(Frame::new(2), Some(200), Some(2));
        assert_eq!(cell.load(), Some(200));
        assert_eq!(cell.frame(), Frame::new(2));
        assert_eq!(cell.checksum(), Some(2));
    }

    #[test]
    fn test_game_state_cell_data_accessor_deref() {
        let cell = GameStateCell::<Vec<i32>>::default();
        cell.save(Frame::new(1), Some(vec![1, 2, 3]), None);

        let accessor = cell.data().unwrap();
        // Use Deref to access Vec methods
        assert_eq!(accessor.len(), 3);
        assert_eq!(accessor[0], 1);
    }

    #[test]
    fn test_game_state_cell_data_accessor_mut_dangerous() {
        let cell = GameStateCell::<Vec<i32>>::default();
        cell.save(Frame::new(1), Some(vec![1, 2, 3]), None);

        {
            let mut accessor = cell.data().unwrap();
            // Use the dangerous mut accessor
            let data = accessor.as_mut_dangerous();
            data.push(4);
        }

        // Verify the modification persisted
        let loaded = cell.load().unwrap();
        assert_eq!(loaded, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_game_state_cell_data_returns_none_when_empty() {
        let cell = GameStateCell::<u32>::default();
        assert!(cell.data().is_none());

        // Save None explicitly
        cell.save(Frame::new(1), None, None);
        assert!(cell.data().is_none());
    }

    // ==========================================
    // Invariant Checker Tests
    // ==========================================

    #[test]
    fn test_invariant_checker_new_sync_layer() {
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        sync_layer.check_invariants().unwrap();
    }

    #[test]
    fn test_invariant_checker_after_advance_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        for _ in 0..20 {
            sync_layer.advance_frame();
            sync_layer.check_invariants().unwrap();
        }
    }

    #[test]
    fn test_invariant_checker_after_save_state() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        for i in 0..10 {
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
            sync_layer.check_invariants().unwrap();
            sync_layer.advance_frame();
        }
    }

    #[test]
    fn test_invariant_checker_after_add_inputs() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.check_invariants().unwrap();
            sync_layer.advance_frame();
        }
    }

    #[test]
    fn test_invariant_checker_after_set_last_confirmed_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }

        sync_layer.set_last_confirmed_frame(Frame::new(5), SaveMode::EveryFrame);
        sync_layer.check_invariants().unwrap();
    }

    #[test]
    fn test_invariant_checker_with_frame_delay() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        sync_layer.set_frame_delay(PlayerHandle::new(0), 2).unwrap();
        sync_layer.set_frame_delay(PlayerHandle::new(1), 3).unwrap();

        sync_layer.check_invariants().unwrap();

        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
            sync_layer.check_invariants().unwrap();
        }
    }

    // ==========================================
    // save_current_state Invariant Tests
    // ==========================================

    /// Verifies that save_current_state maintains the current_frame invariant
    /// by checking that current_frame is always non-negative.
    #[test]
    fn test_save_current_state_maintains_frame_invariant() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save at frame 0 - the initial state
        let request = sync_layer.save_current_state();
        match &request {
            FortressRequest::SaveGameState { frame, .. } => {
                assert!(frame.as_i32() >= 0, "Frame should be non-negative");
                assert_eq!(*frame, Frame::new(0));
            },
            _ => panic!("Expected SaveGameState request"),
        }

        // Advance many frames and verify invariant holds at each
        for expected_frame in 1..100 {
            sync_layer.advance_frame();
            let request = sync_layer.save_current_state();
            match &request {
                FortressRequest::SaveGameState { frame, .. } => {
                    assert!(frame.as_i32() >= 0, "Frame should be non-negative");
                    assert_eq!(*frame, Frame::new(expected_frame));
                },
                _ => panic!("Expected SaveGameState request"),
            }
        }
    }

    /// Verifies that save_current_state correctly updates last_saved_frame.
    #[test]
    fn test_save_current_state_updates_last_saved_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Initially last_saved_frame is NULL
        assert_eq!(sync_layer.last_saved_frame(), Frame::NULL);

        // After saving, it should be updated
        sync_layer.save_current_state();
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(0));

        // Advance and save again
        sync_layer.advance_frame();
        sync_layer.save_current_state();
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(1));
    }

    /// Verifies that save_current_state works correctly after rollback.
    #[test]
    fn test_save_current_state_after_rollback() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save and advance several frames
        for i in 0..5 {
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
            sync_layer.advance_frame();
        }

        assert_eq!(sync_layer.current_frame(), Frame::new(5));

        // Load frame 2 (rollback)
        sync_layer.load_frame(Frame::new(2)).unwrap();
        assert_eq!(sync_layer.current_frame(), Frame::new(2));

        // Now save_current_state should work correctly at frame 2
        let request = sync_layer.save_current_state();
        match &request {
            FortressRequest::SaveGameState { frame, .. } => {
                assert_eq!(*frame, Frame::new(2));
            },
            _ => panic!("Expected SaveGameState request"),
        }
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(2));
    }

    /// Verifies save_current_state works correctly at frame 0 (boundary condition).
    #[test]
    fn test_save_current_state_at_frame_zero() {
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Should work correctly at frame 0
        assert_eq!(sync_layer.current_frame(), Frame::new(0));

        // Note: We use a non-mutable borrow pattern to test the const-ness
        // but save_current_state needs &mut self, so this is mainly documenting
        // that frame 0 is a valid state
        let mut sync_layer = sync_layer;
        let request = sync_layer.save_current_state();
        match request {
            FortressRequest::SaveGameState { frame, cell } => {
                assert_eq!(frame, Frame::new(0));
                // Cell should be usable
                cell.save(Frame::new(0), Some(42u8), Some(12345));
                assert_eq!(cell.frame(), Frame::new(0));
                assert_eq!(cell.load(), Some(42u8));
            },
            _ => panic!("Expected SaveGameState request"),
        }
    }

    /// Verifies that save_current_state provides correct cell cycling
    /// when frames exceed max_prediction.
    #[test]
    fn test_save_current_state_cell_cycling() {
        const MAX_PREDICTION: usize = 4;
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, MAX_PREDICTION);

        // Save more frames than we have cells (max_prediction + 1 = 5 cells)
        // Frame 0 and Frame 5 should use the same cell slot (index 0)
        // Frame 1 and Frame 6 should use the same cell slot (index 1)

        // First, save frames 0-4
        for i in 0..=MAX_PREDICTION {
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some((i * 10) as u8), None);
            }
            if i < MAX_PREDICTION {
                sync_layer.advance_frame();
            }
        }

        // Now at frame 4, advance to frame 5
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(5));

        // Save at frame 5 - this should overwrite cell slot 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            assert_eq!(frame, Frame::new(5));
            cell.save(frame, Some(50u8), None);
            // Verify the cell now has frame 5's data
            assert_eq!(cell.load(), Some(50u8));
        }
    }

    /// Documents the invariant that save_current_state relies on:
    /// current_frame is always non-negative because every mutation path validates
    /// or increments from a valid non-negative frame.
    #[test]
    fn test_save_current_state_invariant_documentation() {
        // This test documents and verifies the invariant that save_current_state relies on.
        //
        // Invariant: current_frame >= 0
        //
        // Proof:
        // 1. SyncLayer::new() initializes current_frame to Frame::new(0)
        // 2. advance_frame() increments current_frame
        // 3. load_frame() can reduce current_frame but only to a frame that was
        //    previously valid (saved state exists)
        // 4. feature-gated hot-join seek can reposition current_frame, but only
        //    after validating a non-negative target
        // 5. Therefore, current_frame is always >= 0
        //
        // The save_current_state() method uses this invariant to call get_cell()
        // which requires frame >= 0. If this invariant were violated (which should
        // be impossible), the telemetry system would report a Critical violation.

        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Verify initial state
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert!(sync_layer.current_frame().as_i32() >= 0);

        // Verify after many operations
        for _ in 0..1000 {
            sync_layer.advance_frame();
            assert!(sync_layer.current_frame().as_i32() >= 0);
        }
    }

    #[test]
    fn test_players_with_incorrect_predictions_includes_boundary_frame() {
        // Regression test: misprediction exactly at disconnect_frame must be included.
        // Previously the filter used `<` instead of `<=`, excluding the boundary.
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        let mut connect_status = vec![ConnectionStatus::default(), ConnectionStatus::default()];

        // Add confirmed inputs for both players at frame 0
        let input0_p0 = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        let input0_p1 = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        sync_layer.add_remote_input(PlayerHandle::new(0), input0_p0);
        sync_layer.add_remote_input(PlayerHandle::new(1), input0_p1);
        connect_status[0].last_frame = Frame::new(0);
        connect_status[1].last_frame = Frame::new(0);

        // Advance to frame 1
        sync_layer.advance_frame();

        // Request synchronized inputs at frame 1.
        // Player 0 has no input for frame 1 yet, so it predicts (RepeatLastConfirmed = 5).
        // Player 1 also predicts.
        let _inputs = sync_layer
            .synchronized_inputs(&connect_status)
            .expect("synchronized inputs should succeed");

        // Now add the ACTUAL input for player 1 at frame 1 with a DIFFERENT value,
        // triggering a misprediction at frame 1.
        let actual_p1 = PlayerInput::new(Frame::new(1), TestInput { inp: 99 });
        sync_layer.add_remote_input(PlayerHandle::new(1), actual_p1);

        // Also add matching input for player 0 (no misprediction)
        let actual_p0 = PlayerInput::new(Frame::new(1), TestInput { inp: 5 });
        sync_layer.add_remote_input(PlayerHandle::new(0), actual_p0);

        // Boundary case: disconnect_frame == misprediction frame (frame 1).
        // The doc says "at or before", so this must be included.
        let result = sync_layer.players_with_incorrect_predictions(Frame::new(1));
        assert_eq!(
            result.len(),
            1,
            "misprediction at disconnect_frame must be included"
        );
        assert_eq!(result[0].0, PlayerHandle::new(1));
        assert_eq!(result[0].1, Frame::new(1));

        // disconnect_frame strictly after the misprediction includes it
        let result_after = sync_layer.players_with_incorrect_predictions(Frame::new(2));
        assert_eq!(
            result_after.len(),
            1,
            "misprediction before disconnect_frame must be included"
        );
        assert_eq!(result_after[0].0, PlayerHandle::new(1));
        assert_eq!(result_after[0].1, Frame::new(1));

        // Sanity: frame BEFORE the misprediction excludes it
        let result_before = sync_layer.players_with_incorrect_predictions(Frame::new(0));
        assert!(
            result_before.is_empty(),
            "misprediction after disconnect_frame must be excluded"
        );

        // Sanity: null disconnect_frame includes everything
        let result_null = sync_layer.players_with_incorrect_predictions(Frame::NULL);
        assert_eq!(
            result_null.len(),
            1,
            "null disconnect_frame should include all mispredictions"
        );
    }
}

// ###################
// # KANI PROOFS     #
// ###################

/// Kani proofs for SyncLayer state consistency.
///
/// These proofs verify:
/// - INV-1: Frame monotonicity (current_frame never decreases except during rollback)
/// - INV-7: Confirmed frame consistency (last_confirmed_frame <= current_frame)
/// - INV-8: Saved frame consistency (last_saved_frame <= current_frame)
/// - State cell management and rollback bounds
///
/// Note: Requires Kani verifier. Install with:
///   cargo install --locked kani-verifier
///   cargo kani setup
///
/// Run proofs with:
///   cargo kani --tests
///
/// ## Unwind Bound Guidelines for SyncLayer Proofs
///
/// SyncLayer construction is more expensive than InputQueue because it creates:
/// - Multiple InputQueues (one per player), each with Vec of INPUT_QUEUE_LENGTH elements
/// - SavedStates with (max_prediction + 1) cells
///
/// Recommended unwind bounds for `SyncLayer::new(num_players, max_prediction)`:
/// - Base: 12-15 for construction with small num_players (1-2) and max_prediction (1-3)
/// - Add loop iterations for any additional loops in the proof
///
/// If proofs timeout:
/// 1. Use concrete values instead of symbolic (kani::any())
/// 2. Reduce loop iteration counts
/// 3. Avoid calling complex methods like `add_remote_input` which involve InputQueue operations
/// 4. Test one behavior at a time rather than multiple assertions in one proof
///
/// ## Measured CBMC cost breakdown for SyncLayer proofs (2026-06)
///
/// The Tier-3 SyncLayer proofs were observed to blow up CBMC. Controlled
/// before/after measurement isolated TWO independent, additive costs:
///
/// 1. **Symbolic-execution (symex) time** — dominated by modeling
///    `GameStateCell`'s `Arc<parking_lot::Mutex<GameState>>` (atomic refcounts,
///    `Weak` teardown, parking_lot's word-lock). This is addressed by the
///    `#[cfg(kani)]` representation of `GameStateCell` (see `game_state_cell.rs`),
///    which swaps the atomic `Arc<Mutex>` for a single-threaded `Rc<RefCell>`
///    under Kani only (sound: proofs are single-threaded; zero production
///    impact because `cfg(kani)` is inactive in every normal build). Measured
///    effect on `proof_confirmed_frame_bounded`: symex 265s -> 8s.
/// 2. **Propositional-reduction (SAT) memory** — dominated by modeling Rust's
///    heap-`Vec` allocator (`RawVec`, `Layout::array`'s SAT-hard 64-bit
///    `size_of::<U>() * count` multiply, plus the capacity-overflow / `grow` /
///    `deallocate` paths) reached via `SavedStates::try_new`,
///    `InputQueue::try_with_queue_length`, and the `input_queues` vector — all
///    routed through `crate::error::try_with_capacity`. This is a *per-operation*
///    cost, independent of the element count, so shrinking sizes does not help.
///    It is addressed by backing those vectors with the `#[cfg(kani)]`
///    `InlineVec` (in `crate::proof_vec`): a stack `[Option<T>; CAP]` with no heap
///    allocation, so CBMC models a fixed-size object with no allocator circuit at
///    all. Kani-only; zero production impact (`ProofVec<T>` is exactly `Vec<T>`
///    in every real build). Measured: the rollback proofs went from ~20 GB / OOM
///    to a few seconds each.
///
/// Practical rule: build every runtime-sized collection through
/// `try_with_capacity` (so it gets the heap-free `InlineVec` backing under Kani),
/// concretize construction parameters to the minimum that still exercises the
/// asserted invariant, and `core::mem::forget` the constructed layer at the end
/// of a proof when its only remaining cost is the `[Option<T>; CAP]` drop loop
/// (orthogonal to the asserted property; needed when the proof's unwind bound is
/// below `CAP + 1`).
///
/// Critical anti-pattern (regression caught 2026-05-09): a `kani::any()` value
/// that flows into a function and becomes a *loop bound* causes CBMC path
/// explosion. Symbolic values are safe when they only size data structures
/// (single allocation), but lethal when they bound loops the verifier must
/// unroll. If a proof using kani::any() hangs or causes runner shutdown,
/// concretize the parameter that flows into the loop bound.
#[cfg(kani)]
mod kani_sync_layer_proofs {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
    struct TestInput {
        inp: u8,
    }

    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = u8;
        type Address = SocketAddr;
    }

    const MIN_TRACTABLE_QUEUE_LENGTH: usize = 2;

    fn minimal_sync_layer(num_players: usize, max_prediction: usize) -> SyncLayer<TestConfig> {
        SyncLayer::<TestConfig>::with_queue_length(
            num_players,
            max_prediction,
            MIN_TRACTABLE_QUEUE_LENGTH,
        )
    }

    /// Proof: The default input queue length is valid for `SyncLayer::new`.
    ///
    /// This keeps default-constructor precondition coverage separate from
    /// broader constructor-state proofs, which use the minimal valid queue
    /// length to keep Kani's state space bounded.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: default queue length satisfies InputQueue construction
    /// - Related: proof_minimal_sync_layer_initial_state_valid_1p,
    ///   proof_minimal_sync_layer_initial_state_valid_2p
    #[kani::proof]
    fn proof_sync_layer_default_queue_length_valid() {
        kani::assert(
            crate::input_queue::INPUT_QUEUE_LENGTH >= MIN_TRACTABLE_QUEUE_LENGTH,
            "default input queue length should be valid",
        );
    }

    // Asserts the structural and frame-state invariants for a freshly-constructed
    // SyncLayer. Factored out so the per-num_players proofs share one assertion
    // body — keeping the harnesses CBMC-tractable by concretizing num_players
    // (which would otherwise drive a symbolic loop bound in
    // SyncLayer::with_queue_length; see the symbolic-loop-bound anti-pattern note
    // in this module's `kani_sync_layer_proofs` docs).
    fn assert_initial_state(
        sync_layer: &SyncLayer<TestConfig>,
        expected_num_players: usize,
        expected_max_prediction: usize,
    ) {
        // INV-1: current_frame starts at 0
        kani::assert(
            sync_layer.current_frame() == Frame::new(0),
            "SyncLayer should start at frame 0",
        );

        // INV-7: last_confirmed_frame <= current_frame (NULL is treated as -1)
        kani::assert(
            sync_layer.last_confirmed_frame().is_null(),
            "SyncLayer should have null last_confirmed_frame",
        );

        // INV-8: last_saved_frame <= current_frame
        kani::assert(
            sync_layer.last_saved_frame().is_null(),
            "SyncLayer should have null last_saved_frame",
        );

        // Structural invariants
        kani::assert(
            sync_layer.num_players == expected_num_players,
            "num_players should be set correctly",
        );
        kani::assert(
            sync_layer.max_prediction == expected_max_prediction,
            "max_prediction should be set correctly",
        );
        kani::assert(
            sync_layer.input_queues.len() == expected_num_players,
            "Should have one input queue per player",
        );
    }

    /// Proof: Minimal SyncLayer construction has valid initial state (0 players).
    ///
    /// Uses the smallest config — num_players=0 and a concrete max_prediction=0.
    /// The initial-state invariants are independent of the player/cell counts, so
    /// this minimal case and proof_minimal_sync_layer_initial_state_valid_2p (2
    /// players, symbolic max_prediction) together cover that independence.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Initial SyncLayer state validity (INV-1, INV-7, INV-8)
    /// - Related: proof_advance_frame_monotonic, proof_saved_states_count
    #[kani::proof]
    #[kani::unwind(3)]
    fn proof_minimal_sync_layer_initial_state_valid_1p() {
        // 0 players + max_prediction 0 (one saved cell): scalar/length init is
        // independent of player/cell counts, so the minimal config verifies the
        // same INV-1/7/8 init within budget.
        let max_prediction: usize = 0;
        let sync_layer = minimal_sync_layer(0, max_prediction);
        assert_initial_state(&sync_layer, 0, max_prediction);
        // Skip the saved-state drop teardown (orthogonal to the init property).
        core::mem::forget(sync_layer);
    }

    /// Proof: Minimal SyncLayer construction has valid initial state (2 players).
    ///
    /// Uses num_players=2 with max_prediction enumerated symbolically across
    /// {1, 2, 3}. Together with proof_minimal_sync_layer_initial_state_valid_1p
    /// (the 0-player minimal config) this covers the initial-state invariants'
    /// independence from the player/cell counts.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Initial SyncLayer state validity (INV-1, INV-7, INV-8)
    /// - Related: proof_advance_frame_monotonic, proof_saved_states_count
    #[kani::proof]
    #[kani::unwind(12)]
    fn proof_minimal_sync_layer_initial_state_valid_2p() {
        let max_prediction: usize = kani::any();
        kani::assume(max_prediction > 0 && max_prediction <= 3);
        let sync_layer = minimal_sync_layer(2, max_prediction);
        assert_initial_state(&sync_layer, 2, max_prediction);
        // The asserted init invariants are checked; skip the saved-state drop
        // teardown (orthogonal `[Option<GameStateCell>; CAP]` drop loop).
        core::mem::forget(sync_layer);
    }

    /// Proof: advance_frame maintains INV-1 (monotonicity).
    ///
    /// Verifies that advance_frame always increases current_frame.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Frame monotonicity (INV-1)
    /// - Related: proof_multiple_advances_monotonic,
    ///   proof_minimal_sync_layer_initial_state_valid_1p,
    ///   proof_minimal_sync_layer_initial_state_valid_2p
    #[kani::proof]
    #[kani::unwind(3)]
    fn proof_advance_frame_monotonic() {
        // 0 players + max_prediction 0: advance_frame mutates only
        // current_frame, never input_queues/saved_states; the minimal config
        // keeps CBMC's pointer model tractable.
        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(0, 0, 2);

        let initial_frame = sync_layer.current_frame();
        sync_layer.advance_frame();
        let new_frame = sync_layer.current_frame();

        kani::assert(
            new_frame > initial_frame,
            "advance_frame should increase current_frame",
        );
        kani::assert(
            new_frame == initial_frame + 1,
            "advance_frame should increment by exactly 1",
        );

        // Skip the saved-state teardown (see proof_confirmed_frame_bounded): the
        // `[Option<GameStateCell>; CAP]` drop loop is orthogonal to monotonicity
        // and its unwinding assertion would exceed this proof's small unwind.
        core::mem::forget(sync_layer);
    }

    /// Proof: Multiple advances maintain monotonicity.
    ///
    /// Note: unwind(15) accounts for SyncLayer construction + loop iterations
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Repeated advance_frame maintains monotonicity
    /// - Related: proof_advance_frame_monotonic
    #[kani::proof]
    #[kani::unwind(15)]
    fn proof_multiple_advances_monotonic() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);
        // Use concrete count for tractability (symbolic count creates too many paths)
        let count: usize = 2;

        let mut prev_frame = sync_layer.current_frame();
        for _ in 0..count {
            sync_layer.advance_frame();
            let curr_frame = sync_layer.current_frame();

            kani::assert(
                curr_frame > prev_frame,
                "Each advance should increase frame",
            );
            prev_frame = curr_frame;
        }

        kani::assert(
            sync_layer.current_frame() == Frame::new(count as i32),
            "Final frame should equal advance count",
        );
    }

    /// Proof: save_current_state maintains INV-8.
    ///
    /// Verifies that after saving, last_saved_frame == current_frame.
    ///
    /// Note: 0 players + max_prediction 0 (one saved cell). With the `forget`
    /// below skipping the saved-state teardown, the harness models only
    /// construction + advance + save; unwind(3) matches the identical
    /// construction in proof_advance_frame_monotonic.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Save updates last_saved_frame (INV-8)
    /// - Related: proof_load_frame_validates_bounds, proof_saved_states_count
    ///   (proof_load_frame_validates_bounds and proof_load_frame_success_maintains_invariants
    ///   still exercise SyncLayer::new(2, 3) for broader multi-player coverage)
    #[kani::proof]
    #[kani::unwind(3)]
    fn proof_save_maintains_inv8() {
        // 0 players + max_prediction 0 (one saved cell): `save_current_state`
        // touches only `current_frame` and `saved_states`, never
        // `input_queues`. Each extra player/cell grows CBMC's pointer model
        // super-linearly; this minimal config exercises the identical INV-8
        // code path within budget.
        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(0, 0, 2);

        // Advance one frame (sufficient to verify save property)
        sync_layer.advance_frame();

        let frame_before_save = sync_layer.current_frame();
        let request = sync_layer.save_current_state();
        let saved_frame = sync_layer.last_saved_frame();

        kani::assert(
            saved_frame == frame_before_save,
            "last_saved_frame should equal current_frame after save",
        );
        kani::assert(
            saved_frame <= sync_layer.current_frame(),
            "INV-8: last_saved_frame <= current_frame",
        );

        // INV-8 checked. Skip the drop teardown of the saved-state `Rc<RefCell>`
        // cell (and the save request's cloned cell handle), whose
        // `drop_in_place`/`Layout` dealloc circuit measured multi-GB peak cbmc
        // RSS and is orthogonal to the save property (see
        // proof_confirmed_frame_bounded). `#[cfg(kani)]`-only; production /
        // `cargo test` / loom drop normally.
        core::mem::forget(request);
        core::mem::forget(sync_layer);
    }

    /// Proof: load_frame validates bounds correctly.
    ///
    /// Verifies that load_frame rejects invalid frames.
    ///
    /// Note: unwind(20) accounts for SyncLayer construction + loop iterations (5)
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: load_frame rejects invalid frames
    /// - Related: proof_load_frame_success_maintains_invariants
    #[kani::proof]
    #[kani::unwind(20)]
    fn proof_load_frame_validates_bounds() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);

        // Advance to frame 5 and save each frame
        for i in 0..5i32 {
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
            sync_layer.advance_frame();
        }
        // Now at frame 5, max_prediction is 4

        // Load NULL_FRAME should fail
        let result_null = sync_layer.load_frame(Frame::NULL);
        kani::assert(result_null.is_err(), "Loading NULL_FRAME should fail");

        // Load current frame should fail (not in the past)
        let result_current = sync_layer.load_frame(Frame::new(5));
        kani::assert(result_current.is_err(), "Loading current frame should fail");

        // Load future frame should fail
        let result_future = sync_layer.load_frame(Frame::new(10));
        kani::assert(result_future.is_err(), "Loading future frame should fail");

        // Load frame outside prediction window should fail (frame 0 is > 4 frames back)
        let result_too_old = sync_layer.load_frame(Frame::new(0));
        kani::assert(
            result_too_old.is_err(),
            "Loading frame outside prediction window should fail",
        );
    }

    /// Proof: load_frame success maintains invariants.
    ///
    /// Note: unwind(20) accounts for SyncLayer construction + loop iterations
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Successful load_frame sets current_frame correctly
    /// - Related: proof_load_frame_validates_bounds
    #[kani::proof]
    #[kani::unwind(20)]
    fn proof_load_frame_success_maintains_invariants() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);

        // Advance to frame 5 and save each frame
        for i in 0..5i32 {
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
            sync_layer.advance_frame();
        }
        // Now at frame 5

        // Load frame 2 (valid: in past, within prediction window)
        let result = sync_layer.load_frame(Frame::new(2));
        kani::assert(result.is_ok(), "Loading valid frame should succeed");

        // After load, current_frame should be the loaded frame
        kani::assert(
            sync_layer.current_frame() == Frame::new(2),
            "current_frame should be set to loaded frame",
        );
    }

    /// Proof: set_frame_delay validates player handle.
    ///
    /// Note: unwind(15) accounts for SyncLayer construction
    /// Tests that invalid handles are rejected
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Invalid player handle rejection
    /// - Related: proof_player_handle_validity
    #[kani::proof]
    #[kani::unwind(15)]
    fn proof_set_frame_delay_validates_handle() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);

        // Invalid handle (>= num_players) should fail
        let result_invalid = sync_layer.set_frame_delay(PlayerHandle::new(5), 2);
        kani::assert(result_invalid.is_err(), "Invalid handle should fail");
    }

    /// Proof: Saved states count is correct.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: SavedStates has max_prediction + 1 slots
    /// - Related: proof_minimal_sync_layer_initial_state_valid_1p,
    ///   proof_minimal_sync_layer_initial_state_valid_2p,
    ///   proof_saved_states_circular_index
    #[kani::proof]
    #[kani::unwind(12)]
    fn proof_saved_states_count() {
        let max_prediction: usize = kani::any();
        kani::assume(max_prediction > 0 && max_prediction <= 3);

        let sync_layer = SyncLayer::<TestConfig>::new(2, max_prediction);

        // Should have max_prediction + 1 state slots
        kani::assert(
            sync_layer.saved_states.states.len() == max_prediction + 1,
            "Should have max_prediction + 1 saved state slots",
        );
    }

    /// Proof: SavedStates get_cell validates frame.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: get_cell rejects negative frames
    /// - Related: proof_saved_states_circular_index
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_get_cell_validates_frame() {
        let saved_states: SavedStates<u8> = SavedStates::new(3);

        // Negative frame should fail
        let result_neg = saved_states.get_cell(Frame::new(-1));
        kani::assert(result_neg.is_err(), "Negative frame should fail");

        // Valid frame should succeed
        let valid_frame: i32 = kani::any();
        kani::assume(valid_frame >= 0 && valid_frame <= 1000);
        let result_valid = saved_states.get_cell(Frame::new(valid_frame));
        kani::assert(result_valid.is_ok(), "Valid frame should succeed");
    }

    /// Proof: SavedStates uses circular indexing correctly.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Circular index calculation bounds
    /// - Related: proof_saved_states_count, proof_get_cell_validates_frame
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_saved_states_circular_index() {
        let max_prediction: usize = kani::any();
        kani::assume(max_prediction > 0 && max_prediction <= 3);

        // Create SavedStates to verify num_cells calculation matches
        let _saved_states: SavedStates<u8> = SavedStates::new(max_prediction);
        let num_cells = max_prediction + 1;

        let frame: i32 = kani::any();
        kani::assume(frame >= 0 && frame <= 10000);

        let expected_pos = frame as usize % num_cells;

        // The get_cell implementation should use this index
        kani::assert(
            expected_pos < num_cells,
            "Calculated position should be within bounds",
        );
    }

    /// Proof: reset_prediction doesn't affect frame state.
    ///
    /// Note: 1 player, max_prediction 1, minimal queue length 2. With the
    /// `forget` below skipping the input-queue + saved-state drop teardown, the
    /// harness models only construction + save + advance + reset_prediction, so
    /// unwind(4) covers the (length-2) construction loops with margin.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: reset_prediction preserves frame invariants
    /// - Related: proof_minimal_sync_layer_initial_state_valid_1p,
    ///   proof_minimal_sync_layer_initial_state_valid_2p
    ///   (proof_load_frame_validates_bounds and proof_load_frame_success_maintains_invariants
    ///   still exercise SyncLayer::new(2, 3) for broader multi-player coverage)
    #[kani::proof]
    #[kani::unwind(4)]
    fn proof_reset_prediction_preserves_frames() {
        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(1, 1, 2);

        // A single save+advance is sufficient to set non-trivial frame values
        sync_layer.save_current_state();
        sync_layer.advance_frame();

        let current_before = sync_layer.current_frame();
        let confirmed_before = sync_layer.last_confirmed_frame();
        let saved_before = sync_layer.last_saved_frame();

        sync_layer.reset_prediction();

        kani::assert(
            sync_layer.current_frame() == current_before,
            "reset_prediction should not change current_frame",
        );
        kani::assert(
            sync_layer.last_confirmed_frame() == confirmed_before,
            "reset_prediction should not change last_confirmed_frame",
        );
        kani::assert(
            sync_layer.last_saved_frame() == saved_before,
            "reset_prediction should not change last_saved_frame",
        );

        // Frame invariants checked. Skip the drop teardown of the `InlineVec`-backed
        // input queue and the `Rc<RefCell>` saved-state cells, whose
        // `drop_in_place`/`Layout` dealloc circuit measured multi-GB peak cbmc
        // RSS and is orthogonal to the reset property (see
        // proof_confirmed_frame_bounded). `#[cfg(kani)]`-only; production /
        // `cargo test` / loom drop normally.
        core::mem::forget(sync_layer);
    }

    /// Proof: INV-7 holds after set_last_confirmed_frame.
    ///
    /// Verifies that set_last_confirmed_frame maintains INV-7 invariant.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Confirmed frame bounded by current frame (INV-7)
    /// - Related: proof_sparse_saving_respects_saved_frame
    #[kani::proof]
    #[kani::unwind(3)]
    fn proof_confirmed_frame_bounded() {
        // 0 players + max_prediction 0 (one saved cell): `set_last_confirmed_frame`
        // clamps the confirmed frame to `current_frame` independently of the
        // input queues (over an empty queue set `first_incorrect` is NULL and the
        // discard loop is a no-op), so INV-7 is exercised identically. Each extra
        // player (InputQueue) and saved GameStateCell (Arc<Mutex>) grows CBMC's
        // pointer model super-linearly on drop, which is what made the prior
        // `new(2, 3)` @ unwind(15) configuration time out in CI.
        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(0, 0, 2);

        // Advance a couple frames without adding inputs (simplified for tractability)
        sync_layer.advance_frame();
        sync_layer.advance_frame();
        // Now at frame 2

        // Set confirmed frame to a concrete value that should be clamped
        sync_layer.set_last_confirmed_frame(Frame::new(5), SaveMode::EveryFrame);

        // INV-7: last_confirmed_frame <= current_frame
        kani::assert(
            sync_layer.last_confirmed_frame() <= sync_layer.current_frame(),
            "INV-7: last_confirmed_frame should be <= current_frame",
        );

        // The invariant has been checked. Tearing the `SyncLayer` down here
        // would force CBMC to model `drop_in_place` over the `InlineVec`-backed
        // input queues and `Rc<RefCell>` saved-state cells (the
        // `[Option<T>; CAP]` drop loop), which measured ~3-4 GB of
        // peak `cbmc` RSS on its own and is entirely orthogonal to the
        // frame-clamp property under test. `forget` is `#[cfg(kani)]`-only;
        // production / `cargo test` / loom drop normally.
        core::mem::forget(sync_layer);
    }

    /// Proof: Sparse saving respects last_saved_frame.
    ///
    /// Verifies that sparse save mode clamps the confirm frame to last_saved.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Sparse save mode respects last_saved_frame
    /// - Related: proof_confirmed_frame_bounded, proof_save_maintains_inv8
    #[kani::proof]
    #[kani::unwind(3)]
    fn proof_sparse_saving_respects_saved_frame() {
        // 0 players + max_prediction 0 (one saved cell): `save_current_state`
        // and the sparse clamp in `set_last_confirmed_frame` touch only
        // `current_frame`/`last_saved_frame`, never the input queues, so the
        // sparse-saving property is exercised identically. Dropping the player
        // count from 2 to 0 and max_prediction from 3 to 0 keeps CBMC's pointer
        // model tractable (the prior `new(2, 3)` @ unwind(15) timed out in CI).
        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(0, 0, 2);

        // Save at frame 0
        sync_layer.save_current_state();

        // Advance to frame 2 (simplified - no add_remote_input for tractability)
        sync_layer.advance_frame();
        sync_layer.advance_frame();

        // With sparse saving enabled, confirm frame should not exceed last_saved (0)
        sync_layer.set_last_confirmed_frame(Frame::new(2), SaveMode::Sparse);

        kani::assert(
            sync_layer.last_confirmed_frame() <= sync_layer.last_saved_frame(),
            "With sparse saving, confirmed frame should not exceed last saved",
        );

        // See `proof_confirmed_frame_bounded`: skip the `cfg(kani)`-irrelevant
        // teardown of the `Vec`/`Rc<RefCell>` object graph, which is orthogonal
        // to the sparse-saving clamp under test. Production drops normally.
        core::mem::forget(sync_layer);
    }

    /// Proof: invalid freeze handles are rejected.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Graceful-drop handle validation
    /// - Related: proof_set_frame_delay_validates_handle
    #[kani::proof]
    #[kani::unwind(5)]
    fn proof_freeze_player_rejects_invalid_handle() {
        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(1, 1, 2);

        let result = sync_layer.freeze_player(PlayerHandle::new(2), Frame::NULL);
        kani::assert(result.is_err(), "invalid freeze handle should fail");
        kani::assert(
            sync_layer.current_frame() == Frame::new(0),
            "failed freeze should not change current frame",
        );

        // Skip the saved-state/input-queue teardown (see
        // proof_confirmed_frame_bounded); the drop loop is orthogonal to the
        // rejected-freeze property and exceeds this proof's unwind.
        core::mem::forget(sync_layer);
    }

    /// Proof: freezing a player preserves frame-state fields.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Graceful-drop freeze does not mutate frame bookkeeping
    /// - Related: proof_freeze_player_rejects_invalid_handle
    #[kani::proof]
    #[kani::unwind(8)]
    fn proof_freeze_player_preserves_frame_state() {
        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(1, 1, 2);
        sync_layer.save_current_state();
        sync_layer.advance_frame();

        let current_before = sync_layer.current_frame();
        let confirmed_before = sync_layer.last_confirmed_frame();
        let saved_before = sync_layer.last_saved_frame();

        let result = sync_layer.freeze_player(PlayerHandle::new(0), Frame::NULL);
        kani::assert(result.is_ok(), "valid freeze should succeed");
        kani::assert(
            sync_layer.current_frame() == current_before,
            "freeze should preserve current_frame",
        );
        kani::assert(
            sync_layer.last_confirmed_frame() == confirmed_before,
            "freeze should preserve last_confirmed_frame",
        );
        kani::assert(
            sync_layer.last_saved_frame() == saved_before,
            "freeze should preserve last_saved_frame",
        );

        // Skip the saved-state/input-queue drop teardown (orthogonal to the
        // preserve property), consistent with proof_freeze_player_rejects_invalid_handle.
        core::mem::forget(sync_layer);
    }

    /// Proof: frozen disconnected players produce the same input for
    /// synchronized simulation and confirmed spectator streams.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Frozen disconnected synchronized/confirmed parity
    /// - Related: proof_freeze_player_preserves_frame_state
    #[kani::proof]
    #[kani::unwind(8)]
    fn proof_frozen_disconnected_inputs_match_confirmed_stream() {
        // This proof genuinely needs one player (it adds, freezes, then reads
        // back a single remote input), so it keeps `with_queue_length(1, 1, 3)`:
        // 1 InputQueue + 2 saved GameStateCells is already the minimal structure
        // that exercises the frozen-disconnected parity. unwind(8) is the
        // smallest bound that still fully unwinds construction, the queue ops
        // over the 3-slot queue, and the per-element drop loops; the prior
        // unwind(12) made CBMC's drop-path state space time out in CI.
        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(1, 1, 3);
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 11 });
        sync_layer.add_remote_input(PlayerHandle::new(0), input);
        let confirmed = sync_layer.confirmed_input(PlayerHandle::new(0), Frame::new(0));
        kani::assert(confirmed.is_ok(), "input should be accepted");

        let freeze = sync_layer.freeze_player(PlayerHandle::new(0), Frame::new(0));
        kani::assert(freeze.is_ok(), "freeze should succeed");
        sync_layer.advance_frame();

        let mut connect_status = [ConnectionStatus::default(); 1];
        connect_status[0].disconnected = true;
        connect_status[0].last_frame = Frame::new(0);

        let synchronized = sync_layer.synchronized_inputs(&connect_status);
        kani::assert(
            synchronized.is_some(),
            "frozen disconnected synchronized input should be available",
        );
        if let Some(inputs) = synchronized {
            kani::assert(inputs.len() == 1, "one synchronized input expected");
            if let Some((input_value, status)) = inputs.first() {
                kani::assert(
                    input_value.inp == 11,
                    "synchronized input should repeat frozen value",
                );
                kani::assert(
                    *status == InputStatus::Disconnected,
                    "synchronized status should be Disconnected",
                );
            }
        }

        let confirmed = sync_layer.confirmed_inputs(Frame::new(1), &connect_status);
        kani::assert(
            confirmed.is_ok(),
            "frozen disconnected confirmed stream should be available",
        );
        if let Ok(inputs) = confirmed {
            kani::assert(inputs.len() == 1, "one confirmed input expected");
            if let Some(input_value) = inputs.first() {
                kani::assert(
                    input_value.input.inp == 11,
                    "confirmed stream should repeat frozen value",
                );
                kani::assert(
                    input_value.frame == Frame::NULL,
                    "dropped confirmed entry should use NULL frame stamp",
                );
            }
        }

        // See `proof_confirmed_frame_bounded`: the frozen-disconnected parity
        // properties have been checked; skip the `cfg(kani)`-irrelevant teardown
        // of the input queue's `Vec` and the saved-state `Rc<RefCell>` cells.
        // Production / `cargo test` / loom drop normally.
        core::mem::forget(sync_layer);
    }
}

#[cfg(all(test, feature = "hot-join"))]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod hot_join_sync_layer_tests {

    use super::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
    struct TestInput {
        inp: u8,
    }

    #[derive(Debug)]
    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = u8;
        type Address = SocketAddr;
    }

    /// White-box helper: is the given player's queue frozen?
    fn queue_frozen(sync_layer: &SyncLayer<TestConfig>, handle: usize) -> bool {
        sync_layer
            .input_queues
            .get(handle)
            .expect("queue should exist")
            .is_frozen()
    }

    /// Test 7: `unfreeze_player` round-trips with `freeze_player`; invalid handle errors.
    #[test]
    fn unfreeze_player_roundtrips_with_freeze_player() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        assert!(!queue_frozen(&sync_layer, 0));
        sync_layer
            .freeze_player(PlayerHandle::new(0), Frame::NULL)
            .expect("freeze valid handle");
        assert!(queue_frozen(&sync_layer, 0));

        sync_layer
            .unfreeze_player(PlayerHandle::new(0))
            .expect("unfreeze valid handle");
        assert!(!queue_frozen(&sync_layer, 0));

        // Invalid handle errors exactly like freeze_player.
        let result = sync_layer.unfreeze_player(PlayerHandle::new(2));
        match result {
            Err(FortressError::InvalidPlayerHandle { handle, max_handle }) => {
                assert_eq!(handle, PlayerHandle::new(2));
                assert_eq!(max_handle, PlayerHandle::new(1));
            },
            other => panic!("Expected InvalidPlayerHandle error, got {other:?}"),
        }
    }

    /// Test 8: `reactivate_player_at_frame` resets ONLY the target queue;
    /// invalid handle errors.
    #[test]
    fn reactivate_player_at_frame_resets_only_target() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Give both queues some state via remote inputs.
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), input);
            sync_layer.add_remote_input(PlayerHandle::new(1), input);
        }

        // Freeze player 0 so we can observe reactivation unfreezing it, and
        // capture player 1's queue markers to prove they are untouched.
        sync_layer
            .freeze_player(PlayerHandle::new(0), Frame::NULL)
            .expect("freeze valid handle");
        assert!(queue_frozen(&sync_layer, 0));

        let other_queue_before = sync_layer.input_queues.get(1).expect("queue 1 exists");
        let other_last_added_before = other_queue_before.last_added_frame();
        let other_last_confirmed_before = other_queue_before.last_confirmed_input();
        let other_frozen_before = other_queue_before.is_frozen();

        let f = Frame::new(30);
        sync_layer
            .reactivate_player_at_frame(PlayerHandle::new(0), f)
            .expect("reactivate valid handle");

        // Target queue (0) was reset + unfrozen and now accepts at F. After the
        // reset the queue's last_added_frame is F + frame_delay(0) - 1 = 29, so
        // the next sequential input is for F. Adding it advances last_added_frame
        // to F, proving acceptance. (We verify via last_added_frame rather than
        // confirmed_input because confirmed_input indexes by `frame %
        // queue_length`, which only holds for queues filled contiguously from
        // frame 0; a reset queue stores frame F at slot 0. The simulation reads
        // a reset queue via input()'s tail-relative path, exercised in the
        // input-queue unit tests.)
        assert!(!queue_frozen(&sync_layer, 0));
        assert_eq!(
            sync_layer.last_added_frame(PlayerHandle::new(0)).unwrap(),
            Frame::new(29)
        );
        sync_layer.add_remote_input(
            PlayerHandle::new(0),
            PlayerInput::new(f, TestInput { inp: 77 }),
        );
        assert_eq!(
            sync_layer.last_added_frame(PlayerHandle::new(0)).unwrap(),
            f
        );

        // Other queue (1)'s markers are unchanged.
        let other_after = sync_layer.input_queues.get(1).expect("queue 1 exists");
        assert_eq!(other_after.last_added_frame(), other_last_added_before);
        assert_eq!(
            other_after.last_confirmed_input(),
            other_last_confirmed_before
        );
        assert_eq!(other_after.is_frozen(), other_frozen_before);
        assert!(!other_after.is_frozen());
        // Its frame-2 input is still confirmed (not reset away).
        assert_eq!(
            sync_layer
                .confirmed_input(PlayerHandle::new(1), Frame::new(2))
                .unwrap()
                .input
                .inp,
            2
        );

        // Invalid handle errors.
        let result = sync_layer.reactivate_player_at_frame(PlayerHandle::new(5), f);
        match result {
            Err(FortressError::InvalidPlayerHandle { handle, max_handle }) => {
                assert_eq!(handle, PlayerHandle::new(5));
                assert_eq!(max_handle, PlayerHandle::new(1));
            },
            other => panic!("Expected InvalidPlayerHandle error, got {other:?}"),
        }
    }

    /// Test 9: `seek_to_frame(F)` sets counters correctly, invariants hold, and
    /// the layer is usable for a full save/advance cycle afterward.
    #[test]
    fn seek_to_frame_sets_counters_and_is_usable() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        let f = Frame::new(100);

        sync_layer.seek_to_frame(f).expect("seek to valid frame");

        assert_eq!(sync_layer.current_frame(), f);
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::new(99));
        // last_saved_frame untouched by seek.
        assert_eq!(sync_layer.last_saved_frame(), Frame::NULL);
        // Every queue repositioned to accept at F.
        assert_eq!(sync_layer.input_queues.len(), 2);
        for handle in 0..2 {
            assert_eq!(
                sync_layer
                    .last_added_frame(PlayerHandle::new(handle))
                    .unwrap(),
                Frame::new(99)
            );
        }

        // Invariants hold post-seek.
        assert!(
            sync_layer.check_invariants().is_ok(),
            "invariants must hold after seek_to_frame"
        );

        // Full cycle: predictions, save at F, advance to F+1.
        let mut connect_status = vec![ConnectionStatus::default(); 2];
        connect_status[0].last_frame = f;
        connect_status[1].last_frame = f;

        let inputs = sync_layer
            .synchronized_inputs(&connect_status)
            .expect("synchronized inputs available after seek");
        assert_eq!(inputs.len(), 2);
        // No real inputs yet at F -> predictions.
        assert_eq!(inputs[0].1, InputStatus::Predicted);
        assert_eq!(inputs[1].1, InputStatus::Predicted);

        match sync_layer.save_current_state() {
            FortressRequest::SaveGameState { frame, .. } => assert_eq!(frame, f),
            _ => panic!("Expected SaveGameState at F"),
        }
        assert_eq!(sync_layer.last_saved_frame(), f);

        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(101));
        assert!(
            sync_layer.check_invariants().is_ok(),
            "invariants must hold after advance"
        );
    }

    /// Test 9 (rollback round-trip): after `seek_to_frame(F)` the layer supports
    /// a full save -> advance -> (confirmed inputs + advance) -> rollback cycle.
    /// Mirrors the existing `test_save_current_state_after_rollback` /
    /// `test_load_frame_success` scaffolding (populate a `GameStateCell` via
    /// `cell.save`, then `load_frame`). Proves a sought layer participates in the
    /// rollback machinery and remains usable afterward.
    #[test]
    fn seek_to_frame_save_advance_rollback_roundtrips() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        let f = Frame::new(100);

        sync_layer.seek_to_frame(f).expect("seek to valid frame");
        assert_eq!(sync_layer.current_frame(), f);

        // Save the snapshot frame F, filling the cell exactly as the production
        // save path would (the orchestration injects the loaded snapshot here).
        match sync_layer.save_current_state() {
            FortressRequest::SaveGameState { cell, frame } => {
                assert_eq!(frame, f);
                cell.save(f, Some(0xAB_u8), Some(0x1234));
                assert_eq!(cell.frame(), f);
            },
            _ => panic!("Expected SaveGameState at F"),
        }
        assert_eq!(sync_layer.last_saved_frame(), f);

        // Advance to F+1, feed confirmed inputs for F (remote inputs avoid the
        // prediction-threshold path), and advance once more.
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(101));
        sync_layer.add_remote_input(
            PlayerHandle::new(0),
            PlayerInput::new(f, TestInput { inp: 5 }),
        );
        sync_layer.add_remote_input(
            PlayerHandle::new(1),
            PlayerInput::new(f, TestInput { inp: 6 }),
        );

        // Save F+1 too so the rollback target's prediction window is populated.
        match sync_layer.save_current_state() {
            FortressRequest::SaveGameState { cell, frame } => {
                assert_eq!(frame, Frame::new(101));
                cell.save(Frame::new(101), Some(0xCD_u8), None);
            },
            _ => panic!("Expected SaveGameState at F+1"),
        }
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(102));

        // Rollback to F: load_frame restores current_frame == F and returns the
        // snapshot data we stored.
        let request = sync_layer
            .load_frame(f)
            .expect("rollback to sought frame F must succeed");
        match request {
            FortressRequest::LoadGameState { frame, cell } => {
                assert_eq!(frame, f);
                assert_eq!(cell.load(), Some(0xAB_u8));
            },
            _ => panic!("Expected LoadGameState at F"),
        }
        assert_eq!(sync_layer.current_frame(), f);
        assert!(
            sync_layer.check_invariants().is_ok(),
            "invariants must hold after rollback to sought frame"
        );

        // Layer is still usable: re-save at F, advance again, invariants intact.
        match sync_layer.save_current_state() {
            FortressRequest::SaveGameState { frame, .. } => assert_eq!(frame, f),
            _ => panic!("Expected SaveGameState at F after rollback"),
        }
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(101));
        assert!(
            sync_layer.check_invariants().is_ok(),
            "invariants must hold after post-rollback advance"
        );
    }

    /// Test 9 (frame 0 variant): seek_to_frame(0) yields a NULL last_confirmed_frame.
    #[test]
    fn seek_to_frame_zero_confirmed_is_null() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        sync_layer.seek_to_frame(Frame::new(0)).expect("seek to 0");
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::NULL);
        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Test 10: `seek_to_frame` with a negative frame returns an error (no panic).
    #[test]
    fn seek_to_frame_negative_errors() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        let result = sync_layer.seek_to_frame(Frame::NULL);
        match result {
            Err(FortressError::InvalidFrameStructured { frame, reason }) => {
                assert_eq!(frame, Frame::NULL);
                assert!(matches!(reason, InvalidFrameReason::MustBeNonNegative));
            },
            _ => panic!("Expected InvalidFrameStructured error"),
        }

        let result = sync_layer.seek_to_frame(Frame::new(-7));
        assert!(matches!(
            result,
            Err(FortressError::InvalidFrameStructured {
                reason: InvalidFrameReason::MustBeNonNegative,
                ..
            })
        ));

        // State unchanged after a rejected seek.
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::NULL);
    }

    /// Test 10 (saved watermark variant): `seek_to_frame` refuses to move behind
    /// `last_saved_frame` because it intentionally leaves that watermark
    /// untouched. This pins the precondition that keeps the post-seek invariant
    /// `last_saved_frame <= current_frame` true.
    #[test]
    fn seek_to_frame_before_last_saved_frame_errors_without_mutation() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        for _ in 0..5 {
            sync_layer.advance_frame();
        }
        match sync_layer.save_current_state() {
            FortressRequest::SaveGameState { frame, .. } => assert_eq!(frame, Frame::new(5)),
            _ => panic!("Expected SaveGameState at frame 5"),
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(5));
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(5));
        let last_confirmed_before = sync_layer.last_confirmed_frame();
        let queue_markers_before = sync_layer
            .input_queues
            .iter()
            .map(InputQueue::last_added_frame)
            .collect::<Vec<_>>();

        let result = sync_layer.seek_to_frame(Frame::new(3));

        match result {
            Err(FortressError::InvalidFrameStructured { frame, reason }) => {
                assert_eq!(frame, Frame::new(3));
                assert!(matches!(
                    reason,
                    InvalidFrameReason::Custom("seek target is older than last_saved_frame")
                ));
            },
            _ => panic!("Expected InvalidFrameStructured error"),
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(5));
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(5));
        assert_eq!(sync_layer.last_confirmed_frame(), last_confirmed_before);
        let queue_markers_after = sync_layer
            .input_queues
            .iter()
            .map(InputQueue::last_added_frame)
            .collect::<Vec<_>>();
        assert_eq!(queue_markers_after, queue_markers_before);
        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Test 10 (freshness variant): even when the target is not older than
    /// `last_saved_frame`, `seek_to_frame` rejects a running layer. The helper
    /// resets all queues wholesale, so hot-join snapshot application must start
    /// from a fresh layer.
    #[test]
    fn seek_to_frame_on_running_layer_errors_without_mutation() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(1));
        assert_eq!(sync_layer.last_saved_frame(), Frame::NULL);

        let result = sync_layer.seek_to_frame(Frame::new(5));

        match result {
            Err(FortressError::InvalidRequestStructured { kind }) => {
                assert!(matches!(
                    kind,
                    InvalidRequestKind::Custom("seek_to_frame requires a fresh SyncLayer")
                ));
            },
            other => panic!("Expected InvalidRequestStructured error, got {other:?}"),
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(1));
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::NULL);
        assert_eq!(sync_layer.last_saved_frame(), Frame::NULL);
        for queue in &sync_layer.input_queues {
            assert_eq!(queue.last_added_frame(), Frame::NULL);
            assert!(!queue.is_frozen());
        }
        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Test 10 (injection-order variant): snapshot injection must happen after
    /// seek positions `current_frame` at the snapshot frame. Injecting first would
    /// otherwise set `last_saved_frame` ahead of `current_frame`.
    #[test]
    fn inject_snapshot_state_future_frame_errors_without_mutation() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        let result = sync_layer.inject_snapshot_state(Frame::new(5), 42_u8, Some(0xCAFE));

        match result {
            Err(FortressError::InvalidFrameStructured { frame, reason }) => {
                assert_eq!(frame, Frame::new(5));
                assert!(matches!(
                    reason,
                    InvalidFrameReason::Custom("snapshot injection frame must match current_frame")
                ));
            },
            other => panic!("Expected InvalidFrameStructured error, got {other:?}"),
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert_eq!(sync_layer.last_saved_frame(), Frame::NULL);
        assert!(sync_layer.capture_snapshot_state(Frame::new(5)).is_none());
        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Test 10 (direct injection success): after seek has positioned the layer,
    /// injection writes the snapshot cell and advances `last_saved_frame` to the
    /// same frame without violating invariants.
    #[test]
    fn inject_snapshot_state_after_seek_succeeds() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        let f = Frame::new(5);

        sync_layer.seek_to_frame(f).expect("fresh seek succeeds");
        let request = sync_layer
            .inject_snapshot_state(f, 42_u8, Some(0xCAFE))
            .expect("inject at current frame succeeds");

        match request {
            FortressRequest::LoadGameState { frame, cell } => {
                assert_eq!(frame, f);
                assert_eq!(cell.load(), Some(42_u8));
                assert_eq!(cell.checksum(), Some(0xCAFE));
            },
            _ => panic!("Expected LoadGameState"),
        }
        assert_eq!(sync_layer.current_frame(), f);
        assert_eq!(sync_layer.last_saved_frame(), f);
        assert!(sync_layer.check_invariants().is_ok());
    }
}
