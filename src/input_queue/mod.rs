//! Input queue management for rollback networking.
//!
//! This module provides [`InputQueue`] for managing player inputs in a circular buffer,
//! along with prediction strategies for handling missing inputs during rollback.
//!
//! # Prediction Strategies
//!
//! Prediction strategies determine what input to use when a player's confirmed input
//! hasn't arrived yet. The following strategies are available:
//!
//! - [`PredictionStrategy`] - Trait for custom prediction strategies
//! - [`RepeatLastConfirmed`] - Default strategy (repeats last confirmed input)
//! - [`BlankPrediction`] - Strategy that returns default input

mod prediction;

pub use prediction::{BlankPrediction, PredictionStrategy, RepeatLastConfirmed};

use crate::frame_info::PlayerInput;
use crate::proof_vec::ProofVec;
use crate::telemetry::{InvariantChecker, InvariantViolation, ViolationKind, ViolationSeverity};
use crate::{report_violation, safe_frame_add, safe_frame_sub};
use crate::{
    Config, FortressError, Frame, IndexOutOfBounds, InputStatus, InternalErrorKind,
    InvalidRequestKind,
};
use std::cmp;

/// The length of the input queue. This describes the number of inputs Fortress Rollback can hold at the same time per player.
///
/// For Kani verification, we use a smaller queue (7 elements) to keep verification tractable.
/// This is sufficient to prove the invariants hold for the circular buffer logic.
///
/// Why 7 (not 8): this constant also caps the Kani-only
/// `InlineVec` (defined under `#[cfg(kani)]` in `crate::proof_vec`) that backs the
/// queue under verification. CBMC
/// unwinds the `CAP`-element loops that initialize, and (for non-`Copy` elements
/// such as `SavedStates`' cells) drop, a `[Option<T>; CAP]`; their unwinding
/// assertion needs an unwind bound of `CAP + 1`. A proof carrying no explicit
/// `#[kani::unwind]` runs at CI's `--default-unwind 8`, so `CAP <= 7` keeps those
/// loops tractable. (Proofs whose explicit unwind is lower instead
/// `core::mem::forget` the layer to skip the drop entirely.) The circular-buffer
/// invariants are length-independent for any value `>= 2`, so 7 proves exactly
/// what 8 would.
///
/// # Note
///
/// This constant is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
///
/// # Formal Specification Alignment
/// - **TLA+**: `QUEUE_LENGTH` in `specs/tla/InputQueue.tla` (set to 3 for model checking)
/// - **Z3**: `INPUT_QUEUE_LENGTH` in `tests/test_z3_verification.rs` (128)
/// - **Kani**: Uses 7 for tractable verification, production uses 128
/// - **formal-spec.md**: INV-4 (queue length bounds), INV-5 (index validity)
#[cfg(kani)]
pub const INPUT_QUEUE_LENGTH: usize = 7;

/// The length of the input queue. This describes the number of inputs Fortress Rollback can hold at the same time per player.
/// At 60fps, 128 frames = ~2.1 seconds of input history.
///
/// # Note
///
/// This constant is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
///
/// # Formal Specification Alignment
/// - **TLA+**: `QUEUE_LENGTH` in `specs/tla/InputQueue.tla` (set to 3 for model checking)
/// - **Z3**: `INPUT_QUEUE_LENGTH` in `tests/test_z3_verification.rs` (128)
/// - **formal-spec.md**: INV-4 requires `0 ≤ q.length ≤ INPUT_QUEUE_LENGTH`
#[cfg(not(kani))]
pub const INPUT_QUEUE_LENGTH: usize = 128;

/// The maximum allowed frame delay. Must be less than [`INPUT_QUEUE_LENGTH`] to ensure
/// the circular buffer doesn't overflow when advancing the queue head.
///
/// This constraint was discovered through Kani formal verification of the `add_input` function.
///
/// # Note
///
/// This constant is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
///
/// # Formal Specification Alignment
/// - **Kani**: `kani_add_input_no_overflow` proof verifies this constraint
/// - **Z3**: `z3_proof_frame_delay_prevents_overflow` in `tests/test_z3_verification.rs`
/// - **formal-spec.md**: Ensures SAFE-1 (no buffer overflow) holds
///
/// Note: This constant is primarily used for testing. Production code uses
/// the configurable `max_frame_delay()` method on `InputQueueConfig` or `InputQueue`.
#[allow(dead_code)]
pub const MAX_FRAME_DELAY: usize = INPUT_QUEUE_LENGTH - 1;

fn circular_index_add(index: usize, offset: usize, modulus: usize) -> Option<usize> {
    if modulus == 0 || index >= modulus {
        return None;
    }

    let offset = offset % modulus;
    let distance_to_wrap = modulus - index;
    Some(if offset >= distance_to_wrap {
        offset - distance_to_wrap
    } else {
        index + offset
    })
}

fn frame_distance_usize(from: Frame, to: Frame) -> Option<usize> {
    usize::try_from(from.distance_to(to)?).ok()
}

/// `InputQueue` handles inputs for a single player and saves them in a circular array.
/// Valid inputs are between `head` and `tail`.
///
/// This is the core circular buffer for managing player inputs in rollback networking.
/// It handles:
/// - Input storage with configurable queue length
/// - Frame delay support
/// - Input prediction when actual inputs haven't arrived
/// - Tracking of incorrect predictions for rollback
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
///
/// # Formal Specification
///
/// - **TLA+**: `specs/tla/InputQueue.tla`
/// - **Kani**: `kani_input_queue_proofs` module (invariant verification)
/// - **Z3**: `tests/test_z3_verification.rs` (circular buffer arithmetic)
#[derive(Debug, Clone)]
pub struct InputQueue<T>
where
    T: Config,
{
    /// The head of the queue. The newest `PlayerInput` is saved here
    head: usize,
    /// The tail of the queue. The oldest `PlayerInput` still valid is saved here.
    tail: usize,
    /// The current length of the queue.
    length: usize,
    /// Denotes if we still are in the first frame, an edge case to be considered by some methods.
    first_frame: bool,

    /// The last frame added by the user
    last_added_frame: Frame,
    /// The first frame in the queue that is known to be an incorrect prediction
    first_incorrect_frame: Frame,
    /// The last frame that has been requested. We make sure to never delete anything after this, as we would throw away important data.
    last_requested_frame: Frame,

    /// The delay in frames by which inputs are sent back to the user. This can be set during initialization.
    frame_delay: usize,

    /// The player index this queue is for (used for prediction strategy)
    player_index: usize,

    /// The length of the input queue (circular buffer).
    /// This is configurable at construction time. Default is INPUT_QUEUE_LENGTH (128).
    queue_length: usize,

    /// Our cyclic input queue
    inputs: ProofVec<PlayerInput<T::Input>>,
    /// A pre-allocated prediction we are going to use to return predictions from.
    prediction: PlayerInput<T::Input>,

    /// The last confirmed input for this player. This is deterministic across all peers
    /// because confirmed inputs are synchronized via the network protocol.
    /// Used as the basis for predictions to ensure determinism.
    last_confirmed_input: Option<T::Input>,

    /// Whether this queue is frozen. A frozen queue silently ignores
    /// [`Self::add_input`] calls, leaving the most recently added input as the
    /// final confirmed value forever. Used by the session layer to support
    /// graceful peer drop: when a remote peer disconnects with the
    /// `ContinueWithout` policy, the dropped peer's queue is frozen so
    /// remaining peers can keep simulating using the dropped peer's last
    /// confirmed input.
    frozen: bool,
}

impl<T: Config> InputQueue<T> {
    /// Creates a new input queue with the default queue length (INPUT_QUEUE_LENGTH).
    ///
    /// Note: This function exists for backward compatibility and testing.
    /// The main construction path uses `with_queue_length` via `SyncLayer::with_queue_length`.
    ///
    /// # Returns
    /// Returns `None` if the default queue length is invalid (should never happen).
    #[allow(dead_code)]
    #[must_use]
    pub fn new(player_index: usize) -> Option<Self> {
        Self::with_queue_length(player_index, INPUT_QUEUE_LENGTH)
    }

    /// Creates a new input queue with a custom queue length.
    ///
    /// # Arguments
    /// * `player_index` - The index of the player this queue is for
    /// * `queue_length` - The size of the circular buffer. Must be at least 2.
    ///
    /// # Returns
    /// Returns `None` if `queue_length < 2` or the requested buffer cannot be reserved.
    #[must_use]
    pub fn with_queue_length(player_index: usize, queue_length: usize) -> Option<Self> {
        match Self::try_with_queue_length(player_index, queue_length) {
            Ok(queue) => Some(queue),
            Err(error) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InputQueue,
                    "Failed to create input queue for player {}: {}",
                    player_index,
                    error
                );
                None
            },
        }
    }

    pub(crate) fn try_with_queue_length(
        player_index: usize,
        queue_length: usize,
    ) -> Result<Self, FortressError> {
        // Return the structured error WITHOUT logging: this fallible constructor
        // is wrapped by the infallible `with_queue_length`, which reports the
        // violation. Logging here too would emit duplicate telemetry for the
        // same invalid input. (Matches `SavedStates::try_new`,
        // `TimeSync::try_with_config`, and `SyncLayer::try_with_queue_length`,
        // which all stay silent and let their wrapper report.)
        if queue_length < 2 {
            return Err(InvalidRequestKind::QueueLengthTooSmall {
                length: queue_length,
            }
            .into());
        }

        let mut inputs = crate::error::try_with_capacity(queue_length, "input_queue.inputs")?;
        for _ in 0..queue_length {
            inputs.push(PlayerInput::blank_input(Frame::NULL));
        }

        Ok(Self {
            head: 0,
            tail: 0,
            length: 0,
            frame_delay: 0,
            first_frame: true,
            last_added_frame: Frame::NULL,
            first_incorrect_frame: Frame::NULL,
            last_requested_frame: Frame::NULL,
            prediction: PlayerInput::blank_input(Frame::NULL),
            inputs,
            player_index,
            queue_length,
            last_confirmed_input: None,
            frozen: false,
        })
    }

    /// Returns the queue length (size of the circular buffer).
    pub fn queue_length(&self) -> usize {
        self.queue_length
    }

    /// Returns the maximum allowed frame delay for this queue.
    /// This is always `queue_length - 1`.
    pub fn max_frame_delay(&self) -> usize {
        self.queue_length.saturating_sub(1)
    }

    /// Returns the first frame in the queue that is known to be an incorrect prediction.
    pub fn first_incorrect_frame(&self) -> Frame {
        self.first_incorrect_frame
    }

    /// Sets the frame delay for this input queue.
    ///
    /// # Behavior
    ///
    /// - **No-op:** If `delay` equals the current frame delay, no change is made.
    /// - **Initial setup:** If no inputs have been added yet, the delay is updated directly.
    /// - **Mid-session increase:** If `delay` is larger than the current delay and
    ///   inputs have already been added, the gap created by the larger delay is filled
    ///   by replicating the most recent input. This preserves the sequential invariant
    ///   `input.frame + frame_delay == last_added_frame + 1` for the next user input.
    /// - **Mid-session decrease:** Decreasing the delay mid-session is **not
    ///   supported**: it would require dropping already-queued (and potentially
    ///   already-sent) inputs. Returns
    ///   [`InvalidRequestKind::InputDelayDecreaseUnsupported`] in that case.
    ///
    /// # Mid-session delay change
    ///
    /// Increasing the delay replicates the most recently added input across the new
    /// gap. This matches the strategy used by [`advance_queue_head`] for initial
    /// delay setup, and is consistent with what the network protocol expects (the
    /// remote peer must observe the same input sequence on both sides). The
    /// trade-off is that "held" inputs (e.g., an attack button) will continue for
    /// the gap frames; applications that need different gap-fill semantics should
    /// call this method only when no input is held.
    ///
    /// Decreasing the delay mid-session is rejected because it would require
    /// discarding already-queued inputs. Set the desired delay before adding any
    /// inputs (typically via the session builder).
    ///
    /// # Errors
    /// - Returns [`InvalidRequestKind::FrameDelayTooLarge`] if `delay > max_frame_delay()`.
    /// - Returns [`InvalidRequestKind::InputDelayDecreaseUnsupported`] if `delay` is less
    ///   than the current delay and inputs have already been added.
    /// - Returns [`InternalErrorKind::InputQueueGapFillFailed`] if gap-fill replication
    ///   fails (indicates an internal invariant violation).
    ///
    /// [`advance_queue_head`]: Self::advance_queue_head
    pub fn set_frame_delay(&mut self, delay: usize) -> Result<(), FortressError> {
        // Frozen queues represent a dropped peer under ContinueWithout. Once
        // frozen, their simulation input must remain stable regardless of
        // later queue mutation attempts, and `set_frame_delay` is contractually
        // a silent no-op. This guard MUST precede any validation (including the
        // `delay > max_frame_delay()` check) so an out-of-range delay does not
        // surface an error on a queue whose owner is already gone.
        if self.frozen {
            return Ok(());
        }

        let max_delay = self.max_frame_delay();
        if delay > max_delay {
            return Err(InvalidRequestKind::FrameDelayTooLarge { delay, max_delay }.into());
        }

        // No-op: nothing to do.
        if delay == self.frame_delay {
            return Ok(());
        }

        // Initial setup before any inputs — safe to just update.
        if self.last_added_frame.is_null() {
            self.frame_delay = delay;
            return Ok(());
        }

        // Decreasing delay mid-session is unsupported: it would require dropping
        // already-queued (and potentially already-sent) inputs.
        if delay < self.frame_delay {
            return Err(InvalidRequestKind::InputDelayDecreaseUnsupported {
                current: self.frame_delay,
                requested: delay,
            }
            .into());
        }

        // Increasing delay mid-session: replicate the most-recent input to fill the
        // gap created by the larger delay. This preserves the sequential invariant
        // `input.frame + frame_delay == last_added_frame + 1` for the next user
        // input. Replicating the last input is the same strategy used by
        // `advance_queue_head` for initial-delay setup.
        let delta = delay - self.frame_delay;
        let prev_position = match self.head {
            0 => self.queue_length - 1,
            _ => self.head - 1,
        };
        let last_input = match self.inputs.get(prev_position) {
            Some(input) => *input,
            None => {
                return Err(FortressError::InternalErrorStructured {
                    kind: InternalErrorKind::IndexOutOfBounds(IndexOutOfBounds {
                        name: "inputs",
                        index: prev_position,
                        length: self.inputs.len(),
                    }),
                });
            },
        };
        let snapshot_head = self.head;
        let snapshot_tail = self.tail;
        let snapshot_length = self.length;
        let snapshot_first_frame = self.first_frame;
        let snapshot_last_added_frame = self.last_added_frame;
        let snapshot_first_incorrect_frame = self.first_incorrect_frame;
        let snapshot_last_requested_frame = self.last_requested_frame;
        let snapshot_frame_delay = self.frame_delay;
        let snapshot_inputs = self.inputs.clone();
        let snapshot_prediction = self.prediction;
        let snapshot_last_confirmed_input = self.last_confirmed_input;

        for _ in 0..delta {
            let next_frame = safe_frame_add!(
                self.last_added_frame,
                1,
                "InputQueue::set_frame_delay gap fill"
            );
            // Replicate the previous input's payload at the new frame.
            let filler = PlayerInput {
                frame: next_frame,
                input: last_input.input,
            };
            if !self.add_input_by_frame(filler, next_frame) {
                self.head = snapshot_head;
                self.tail = snapshot_tail;
                self.length = snapshot_length;
                self.first_frame = snapshot_first_frame;
                self.last_added_frame = snapshot_last_added_frame;
                self.first_incorrect_frame = snapshot_first_incorrect_frame;
                self.last_requested_frame = snapshot_last_requested_frame;
                self.frame_delay = snapshot_frame_delay;
                self.inputs = snapshot_inputs;
                self.prediction = snapshot_prediction;
                self.last_confirmed_input = snapshot_last_confirmed_input;

                return Err(FortressError::InternalErrorStructured {
                    kind: InternalErrorKind::InputQueueGapFillFailed { frame: next_frame },
                });
            }
            // `add_input_by_frame` is the only mutator that updates
            // `last_added_frame`; this is a debug-only sanity check that the
            // invariant holds. In release builds it is compiled out — a real
            // mismatch would have been reported by `add_input_by_frame` via
            // `report_violation!` and surfaced through the `false` return
            // above.
            debug_assert_eq!(
                self.last_added_frame, next_frame,
                "add_input_by_frame must advance last_added_frame to next_frame"
            );
        }

        self.frame_delay = delay;
        Ok(())
    }

    /// Returns the current frame delay for this input queue.
    #[must_use]
    pub fn frame_delay(&self) -> usize {
        self.frame_delay
    }

    /// Returns the most recently added input frame, or [`Frame::NULL`] if no
    /// inputs have been added yet.
    ///
    /// # Note
    /// This accessor is exposed for use by the session/protocol layer to
    /// coordinate the network-level pending-output queue with the input queue
    /// after a mid-session frame-delay change.
    #[must_use]
    pub(crate) fn last_added_frame(&self) -> Frame {
        self.last_added_frame
    }

    /// Returns the most recently confirmed input value for this player, or
    /// `None` if no inputs have ever been added.
    ///
    /// Used by the sync layer to surface the last good input as the dropped
    /// peer's reported input after a graceful peer drop, paired with status
    /// [`crate::InputStatus::Disconnected`].
    #[must_use]
    pub(crate) fn last_confirmed_input(&self) -> Option<T::Input> {
        self.last_confirmed_input
    }

    /// Resets the prediction state.
    pub fn reset_prediction(&mut self) {
        self.prediction.frame = Frame::NULL;
        self.first_incorrect_frame = Frame::NULL;
        self.last_requested_frame = Frame::NULL;
    }

    /// Returns a `PlayerInput` only if the input for the requested frame is confirmed.
    /// In contrast to `input()`, this will not return a prediction if there is no confirmed input for the frame.
    pub fn confirmed_input(
        &self,
        requested_frame: Frame,
    ) -> Result<PlayerInput<T::Input>, crate::FortressError> {
        let offset = requested_frame.as_i32() as usize % self.queue_length;

        let input = self
            .inputs
            .get(offset)
            .ok_or(FortressError::InternalErrorStructured {
                kind: InternalErrorKind::IndexOutOfBounds(IndexOutOfBounds {
                    name: "inputs",
                    index: offset,
                    length: self.inputs.len(),
                }),
            })?;
        if input.frame == requested_frame {
            return Ok(*input);
        }

        // the requested confirmed input should not be before a prediction. We should not have asked for a known incorrect frame.
        Err(InvalidRequestKind::NoConfirmedInput {
            frame: requested_frame,
        }
        .into())
    }

    /// Discards confirmed frames **before** the given `frame` from the queue.
    /// All confirmed frames are guaranteed to be synchronized between players,
    /// so there is no need to save the inputs anymore.
    ///
    /// Note: After calling `discard_confirmed_frames(5)`, frames 0-4 are discarded
    /// and frame 5 becomes the new tail (oldest frame in queue).
    pub fn discard_confirmed_frames(&mut self, mut frame: Frame) {
        // An EMPTY queue holds nothing to discard — return quietly (chunk-N5
        // noise downgrade, S34 residual 5). A hot-join reactivated queue
        // (`reset_to_frame`: blanked ring, `length == 0`) is legitimately
        // empty while the session's confirmed frame still trails the slot's
        // first post-reactivation input, and every survivor advance in that
        // window used to land in the offset arm below with an Error-severity
        // "Discard offset N exceeds queue length 0" violation. Neither arm
        // below is meaningful on an empty ring: the `frame >=
        // last_added_frame` arm would fabricate `length = 1` out of a
        // blanked slot, and the offset arm's `checked_sub(length == 0)`
        // failure was the reported noise. Skipping is vacuously correct in
        // every shape (there are no confirmed entries to drop).
        if self.length == 0 {
            #[cfg(not(kani))] // Kani: tracing macros explode CBMC state space.
            tracing::trace!(
                "discard_confirmed_frames({}): the queue is empty; nothing to discard",
                frame
            );
            return;
        }

        // we only drop frames until the last frame that was requested, otherwise we might delete data still needed
        if !self.last_requested_frame.is_null() {
            frame = cmp::min(frame, self.last_requested_frame);
        }

        // move the tail to "delete inputs", wrap around if necessary
        if frame >= self.last_added_frame {
            // delete all but most recent - set tail to the position of the most recent input
            // The most recent input is at (head - 1), so tail should point there
            // This maintains the circular buffer invariant: length = (head - tail) mod queue_length
            self.tail = if self.head == 0 {
                self.queue_length - 1
            } else {
                self.head - 1
            };
            self.length = 1;
        } else if let Some(tail_input) = self.inputs.get(self.tail) {
            if frame <= tail_input.frame {
                // The target frame is at or before the current tail - nothing to delete
                // (frames before tail don't exist in the queue)
            } else {
                // Discard frames from tail up to (but not including) 'frame'
                // After this, 'frame' becomes the new tail
                let tail_frame = tail_input.frame;
                let Some(offset) = frame_distance_usize(tail_frame, frame) else {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::InputQueue,
                        "Discard frame distance from {} to {} overflowed",
                        tail_frame,
                        frame
                    );
                    return;
                };
                let Some(new_length) = self.length.checked_sub(offset) else {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::InputQueue,
                        "Discard offset {} exceeds queue length {}",
                        offset,
                        self.length
                    );
                    return;
                };
                let Some(new_tail) = circular_index_add(self.tail, offset, self.queue_length)
                else {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::InputQueue,
                        "Failed to advance tail {} by {} within queue length {}",
                        self.tail,
                        offset,
                        self.queue_length
                    );
                    return;
                };
                self.tail = new_tail;
                self.length = new_length;
            }
        }
    }

    /// Returns the game input of a single player for a given frame.
    /// If that input does not exist, returns a prediction instead.
    ///
    /// # Determinism
    ///
    /// When predicting, this method uses `RepeatLastConfirmed` strategy, which returns
    /// the last confirmed input for this player (or default if none confirmed yet).
    ///
    /// This is deterministic because:
    /// - `last_confirmed_input` is only updated when confirmed inputs arrive
    /// - Confirmed inputs are synchronized across all peers via the network protocol
    /// - Therefore all peers have the same `last_confirmed_input` for any given player
    ///
    /// This is DIFFERENT from the original GGPO approach of using "last added" input,
    /// which depended on local timing and caused desyncs.
    ///
    /// # Returns
    /// Returns `None` if called when a prediction error exists or if the requested frame
    /// is before the oldest frame in the queue. In normal operation, this should not happen.
    pub fn input(&mut self, requested_frame: Frame) -> Option<(T::Input, InputStatus)> {
        // No one should ever try to grab any input when we have a prediction error.
        // Doing so means that we're just going further down the wrong path.
        if !self.first_incorrect_frame.is_null() {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "Attempted to get input while prediction error exists (first_incorrect_frame={})",
                self.first_incorrect_frame
            );
            return None;
        }

        // Remember the last requested frame number for later. We'll need this in add_input() to drop out of prediction mode.
        self.last_requested_frame = requested_frame;

        // Verify that we request a frame that still exists
        let tail_input = self.inputs.get(self.tail)?;
        if requested_frame < tail_input.frame {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "Requested frame {} is before oldest frame {} in queue",
                requested_frame,
                tail_input.frame
            );
            return None;
        }

        // We currently don't have a prediction frame
        if self.prediction.frame.as_i32() < 0 {
            //  If the frame requested is in our range, fetch it out of the queue and return it.
            let Some(mut offset) = frame_distance_usize(tail_input.frame, requested_frame) else {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InputQueue,
                    "Requested frame distance from {} to {} overflowed",
                    tail_input.frame,
                    requested_frame
                );
                return None;
            };

            if offset < self.length {
                let Some(index) = circular_index_add(self.tail, offset, self.queue_length) else {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::InputQueue,
                        "Failed to map requested-frame offset {} from tail {} within queue length {}",
                        offset,
                        self.tail,
                        self.queue_length
                    );
                    return None;
                };
                offset = index;
                // Verify circular buffer indexing correctness
                let input_at_offset = self.inputs.get(offset)?;
                if input_at_offset.frame != requested_frame {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InputQueue,
                        "Circular buffer index mismatch: expected frame {}, got frame {} at offset {}",
                        requested_frame,
                        input_at_offset.frame,
                        offset
                    );
                    return None;
                }
                return Some((input_at_offset.input, InputStatus::Confirmed));
            }

            // The requested frame isn't in the queue. This means we need to return a prediction frame.
            // Use RepeatLastConfirmed strategy with the synchronized last_confirmed_input.
            // This is deterministic because last_confirmed_input is only updated when
            // confirmed inputs arrive, which are synchronized across all peers.
            let predicted_input = RepeatLastConfirmed.predict(
                requested_frame,
                self.last_confirmed_input,
                self.player_index,
            );
            // A prediction episode always begins at the queue's FIRST MISSING
            // frame: `last_added_frame + 1`, or frame 0 on a queue that has
            // never accepted an input (`advance_queue_head` gap-fills a virgin
            // queue from frame 0, so frame 0 is always its first physical add).
            // Inputs are added strictly sequentially, so this keeps
            // `prediction.frame` equal to the next arrival's frame for the
            // whole episode: every arrival is compared against the episode's
            // frozen value in `add_input_by_frame`, and no arrival can land
            // below `prediction.frame`. Entering at `requested_frame` instead
            // would let a rollback re-simulation that starts above this queue's
            // missing window (ordinary N>=3 cross-endpoint jitter) skip the
            // misprediction comparison for every frame in
            // `[last_added_frame + 1, requested_frame)`, permanently swallowing
            // rollback for that window (finding F17). Mirrors `AddRemoteInput`
            // in `specs/tla/InputQueue.tla`, which compares every sequential
            // arrival unconditionally. Only the episode's frame bookkeeping
            // starts at the first missing frame; the returned VALUE covers any
            // requested frame at or beyond it.
            // The single expression covers the virgin case: `Frame::NULL` is
            // -1, so `last_added_frame + 1` evaluates to frame 0 — the first
            // frame a virgin queue physically adds — without overflow.
            let entry_frame = safe_frame_add!(
                self.last_added_frame,
                1,
                "InputQueue::input prediction entry"
            );
            self.prediction = PlayerInput {
                frame: entry_frame,
                input: predicted_input,
            };
        }

        // We must be predicting, so we return the prediction frame contents.
        if self.prediction.frame.is_null() {
            report_violation!(
                ViolationSeverity::Critical,
                ViolationKind::InputQueue,
                "Prediction frame is null when it should be set"
            );
            return None;
        }
        let prediction_to_return = self.prediction; // PlayerInput has copy semantics
        Some((prediction_to_return.input, InputStatus::Predicted))
    }

    /// Freezes this input queue. After this call, [`Self::add_input`] becomes a
    /// no-op (silently dropping subsequent inputs without advancing the queue),
    /// and the most recently added input remains the queue's permanent
    /// confirmed value.
    ///
    /// Used by the session layer to support graceful peer drop. The
    /// `Disconnected` status reported to game code for a frozen player is
    /// produced by the `connect_status.disconnected` flag at the
    /// [`crate::sync_layer::SyncLayer`] level, not by this queue. This method
    /// only stops further mutations to the queue so remaining peers can keep
    /// reading the last confirmed value deterministically.
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Freezes this input queue at a specific **agreed freeze frame**, rolling
    /// the queue's `last_confirmed_input` back to the value confirmed at
    /// `freeze_frame` before freezing.
    ///
    /// # Why this exists (graceful peer drop under packet loss)
    ///
    /// When a peer drops in an N≥3 full-mesh session using
    /// `DisconnectBehavior::ContinueWithout`, every survivor freezes the dropped
    /// slot so it repeats the dropped peer's last good input forever. Without
    /// coordination, each survivor would repeat *its own* last received input —
    /// but under packet loss survivors may have received the dropped peer's
    /// inputs through **different** frames (per-link delivery; a now-terminal
    /// endpoint never re-supplies them). One survivor freezing at frame 10's
    /// value while another freezes at frame 8's value yields divergent confirmed
    /// history — a silent desync.
    ///
    /// The session layer computes a single **agreed freeze frame** `F` as the
    /// global minimum across all peers of each peer's received frame for the
    /// dropped slot (see `update_player_disconnects` /
    /// `remote_disconnect_snapshot` in `p2p_session.rs`). Because `F` is a global
    /// min, every survivor has a confirmed input *at* `F` (each received at least
    /// through `F`), and that value is identical across survivors. Rolling
    /// `last_confirmed_input` back to the value at `F` therefore makes every
    /// survivor repeat the **same** deterministic value. The existing
    /// `disconnect_frame` rollback then re-simulates any survivor's extra frames
    /// (`F+1..received`) using that agreed value, converging confirmed history.
    ///
    /// # Behavior
    ///
    /// While the queue is **not yet frozen**:
    /// - If `freeze_frame` is [`Frame::NULL`], there is no agreed freeze frame to
    ///   roll back to — the expected case for a reserved hot-join slot frozen
    ///   from frame 0 (no confirmed inputs yet) or a peer dropped before any
    ///   confirmed input. Leave `last_confirmed_input` **unchanged** and freeze
    ///   **silently**: this is a normal "no agreed frame" signal, *not* a
    ///   violation (mirroring [`Self::set_frozen_value_at`]'s NULL no-op).
    /// - If `freeze_frame` is non-NULL and a confirmed input exists at that frame
    ///   in the ring buffer (via [`Self::confirmed_input`]), set
    ///   `last_confirmed_input` to that input's value, then freeze.
    /// - If `freeze_frame` is non-NULL but no confirmed input exists at that frame
    ///   (e.g. evicted from the ring buffer, or never received), leave
    ///   `last_confirmed_input` **unchanged** (fail-safe), report a
    ///   [`ViolationSeverity::Warning`] violation describing the miss, and still
    ///   freeze. This never panics.
    ///
    /// If the queue is already frozen, this is a no-op (mirrors [`Self::freeze`]
    /// idempotence and avoids mutating an already-settled frozen value).
    pub(crate) fn freeze_at(&mut self, freeze_frame: Frame) {
        if self.frozen {
            return;
        }

        // Best-effort initial value-set, then freeze. The convergence guarantee
        // — rolling the frozen value DOWN to the global-min agreed frame `F` on
        // the direct-detection / re-adjust paths — is provided separately by
        // [`Self::set_frozen_value_at`], driven from
        // `P2PSession::disconnect_player_at_frames`. Here we seed an initial
        // value and flip the frozen flag.
        //
        // `Frame::NULL` is the expected "no agreed freeze frame" case (a reserved
        // hot-join slot frozen from frame 0; a drop before any confirmed input):
        // `roll_confirmed_input_to` is already a no-op for it and there is no
        // value to seed, so freeze silently — not a violation, mirroring
        // `set_frozen_value_at`. Only a *non-NULL* frame missing from the ring
        // (evicted or never received) is unexpected and warrants the `Warning`.
        if !freeze_frame.is_null() && !self.roll_confirmed_input_to(freeze_frame) {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::InputQueue,
                "freeze_at: no confirmed input at agreed freeze frame {}; leaving last_confirmed_input unchanged",
                freeze_frame
            );
        }

        self.frozen = true;
    }

    /// Rolls an **already-frozen** queue's `last_confirmed_input` to the value
    /// confirmed at `frame`, if one exists. This is the non-idempotent
    /// counterpart to [`Self::freeze_at`]: where `freeze_at` only seeds a value
    /// while transitioning *into* the frozen state (and no-ops once frozen),
    /// this method intentionally *updates* a queue that is already frozen.
    ///
    /// # Why this exists (closing the under-loss desync on the direct paths)
    ///
    /// `freeze_at` rolls the frozen value to the agreed freeze frame `F` only on
    /// the gossip-propagation path, where the session supplies a global-min `F`.
    /// On the **direct-detection** paths (own-endpoint timeout, `remove_player`),
    /// the survivor that detected the drop freezes at its OWN locally-received
    /// frame, which under asymmetric packet loss can be HIGHER than the global
    /// min. The disconnect machinery later converges every survivor's
    /// `local_connect_status[handle].last_frame` DOWN to the same global-min `F`
    /// (`disconnect_player_at_frames` mins it on the re-adjust branch), but
    /// `freeze_at` is idempotent and would never lower the already-frozen value.
    ///
    /// This method is the chokepoint fix: whenever the disconnect machinery
    /// sets/lowers `status.last_frame` for a frozen handle, the session calls
    /// this with the new `last_frame`, re-rolling the frozen value to track `F`
    /// **down**. Because every survivor converges `status.last_frame` to the
    /// identical global min, and every survivor received the dropped peer's
    /// input at `F` (`F` is the min, hence `<=` each survivor's received frame),
    /// the re-rolled value is byte-identical across survivors — closing the
    /// desync.
    ///
    /// # Behavior (fail-safe)
    ///
    /// - If the queue is **not** frozen, this is a no-op (it never seeds a value
    ///   on a live queue; only the freeze transition may do that).
    /// - If `frame` is [`Frame::NULL`], this is a no-op (no agreed frame yet).
    /// - If the queue is frozen and a confirmed input exists at `frame`, sets
    ///   `last_confirmed_input` to that value.
    /// - If the queue is frozen, `frame` is non-NULL, but no confirmed input
    ///   exists at `frame` (evicted from the ring buffer, or never received),
    ///   the value is left **unchanged** (fail-safe) and a
    ///   [`ViolationSeverity::Warning`] is reported. This never panics.
    pub(crate) fn set_frozen_value_at(&mut self, frame: Frame) {
        if !self.frozen {
            return;
        }
        if frame.is_null() {
            return;
        }
        if !self.roll_confirmed_input_to(frame) {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::InputQueue,
                "set_frozen_value_at: no confirmed input at agreed freeze frame {}; leaving last_confirmed_input at its last agreed value",
                frame
            );
        }
    }

    /// Sets `last_confirmed_input` to the value confirmed at `frame`, returning
    /// whether a confirmed input was found and applied.
    ///
    /// Shared lookup helper for [`Self::freeze_at`] and
    /// [`Self::set_frozen_value_at`]. Returns `false` (leaving
    /// `last_confirmed_input` unchanged) when `frame` is [`Frame::NULL`] or when
    /// no confirmed input exists at `frame` (evicted or never received). Never
    /// panics. The caller decides whether and how to report a violation, since
    /// the two callers want different wording.
    fn roll_confirmed_input_to(&mut self, frame: Frame) -> bool {
        if frame.is_null() {
            return false;
        }
        match self.confirmed_input(frame) {
            Ok(input) => {
                self.last_confirmed_input = Some(input.input);
                true
            },
            Err(_) => false,
        }
    }

    /// Returns whether this queue has been frozen via [`Self::freeze`].
    #[must_use]
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Re-freezes a queue that was reopened by [`Self::reset_to_frame`],
    /// restoring `value` (the caller-captured **pre-reopen**
    /// `last_confirmed_input`) as the frozen value.
    ///
    /// This is the **abort-path inverse** of `reset_to_frame` for the N-peer
    /// hot-join survivor: on a coordinator `JoinAborted`, a survivor that
    /// already reopened the dropped slot at the activation frame `F` must
    /// return it to the reserved/frozen state, repeating the identical agreed
    /// value every survivor shares. The value must be captured by the caller
    /// **before** the reopen: `reset_to_frame` itself preserves
    /// `last_confirmed_input`, but any real joiner input confirmed by the
    /// reopened queue before the abort arrives overwrites it (`add_input`
    /// tracks the latest confirmed input) — restoring whatever the queue
    /// currently holds could therefore leak the aborted attempt's input into
    /// the frozen stream, diverging from survivors that never received it.
    /// The session separately restores the pre-reopen connection status.
    ///
    /// [`Self::freeze_at`] cannot be used for this: a frame-targeted re-freeze
    /// would look up `confirmed_input` in the ring buffer that
    /// `reset_to_frame` blanked (or that now holds the aborted attempt's
    /// frames `>= F`), fail, and report a spurious violation for what is a
    /// deliberate, specified restore. Any post-reopen ring contents become
    /// unreachable (a frozen disconnected slot is served from
    /// `last_confirmed_input` past its connection status `last_frame`, and a
    /// future re-serve repositions the queue through `reset_to_frame` again,
    /// which re-blanks the ring). Idempotent given the same `value`.
    #[cfg(feature = "hot-join")]
    pub(crate) fn refreeze_with_value(&mut self, value: Option<T::Input>) {
        self.last_confirmed_input = value;
        self.frozen = true;
    }

    /// Unfreezes this queue, re-enabling [`Self::add_input`].
    ///
    /// This is the counterpart to [`Self::freeze`]. It is used when a slot that
    /// was gracefully dropped (and therefore frozen, repeating its last
    /// confirmed input forever) is reactivated by a hot-joining peer: the slot
    /// must stop auto-confirming the frozen value and resume accepting that
    /// peer's real inputs.
    ///
    /// Unfreezing alone does **not** reposition the queue. Because the queue's
    /// `last_added_frame` is still whatever it was when the slot froze, the very
    /// next [`Self::add_input`] would have to be the immediately sequential
    /// frame. Callers reactivating a slot at a non-zero activation frame should
    /// therefore follow this with [`Self::reset_to_frame`], which both unfreezes
    /// and repositions the queue to accept inputs from the activation frame
    /// onward.
    // dead_code: consumed by chunk 5's session orchestration (host reactivation
    // of a reserved/dropped slot). Only the input-queue primitive lands here.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub fn unfreeze(&mut self) {
        self.frozen = false;
    }

    /// Repositions the queue so that its **next accepted input is for game frame
    /// `frame`**, exactly as if the queue had just finished confirming every
    /// frame before `frame`.
    ///
    /// This is the core hot-join primitive. It is used in two places:
    /// - On a **host** reactivating a frozen/reserved slot, where
    ///   `last_confirmed_input` holds the frozen value (the deterministic
    ///   prediction base for frames `>= frame` until the joiner's real inputs
    ///   arrive).
    /// - On a **fresh joiner** layer fast-forwarded to a snapshot's activation
    ///   frame, where `last_confirmed_input` is `None` (the queue has never seen
    ///   an input).
    ///
    /// In both cases `last_confirmed_input` is **preserved** — it is the value
    /// [`Self::input`] predicts with, and clearing it would change the
    /// deterministic prediction seen by other peers.
    ///
    /// # Postconditions (on success, i.e. `frame` is non-negative)
    /// - [`Self::is_frozen`] returns `false`.
    /// - The next [`Self::add_input`] whose effective frame (`frame +
    ///   frame_delay`) equals `frame + frame_delay` is accepted (returns a real
    ///   frame, not [`Frame::NULL`]); concretely, with the queue's current
    ///   `frame_delay`, `add_input(input @ frame)` is accepted and subsequent
    ///   sequential inputs (`frame + 1`, `frame + 2`, …) are accepted too.
    /// - Until a real input for `frame` arrives, [`Self::input`] for `frame` (or
    ///   any later frame) returns a prediction built from the preserved
    ///   `last_confirmed_input` via [`RepeatLastConfirmed`].
    /// - The circular buffer is realigned so that [`Self::confirmed_input`] —
    ///   which indexes absolutely as `inputs[f % queue_length]` — keeps working
    ///   after the reset: once `add_input(input @ frame)` is accepted,
    ///   `confirmed_input(frame + frame_delay)` returns that input.
    ///
    /// # Pre-activation `confirmed_input` surface
    /// After a reset to `F`, the only confirmed slot that exists before the first
    /// real input is the phantom predecessor: `confirmed_input(last_added_frame)`
    /// (i.e. `F + frame_delay - 1`, the frame just before the first accepted
    /// input) returns the preserved `last_confirmed_input` (the frozen value on a
    /// reactivated host queue, or `T::Input::default()` if none was preserved) —
    /// semantically correct for that pre-activation frame. [`Self::confirmed_input`]
    /// for any frame *below* that returns
    /// [`InvalidRequestKind::NoConfirmedInput`](crate::error::InvalidRequestKind::NoConfirmedInput).
    /// Callers (the chunk-5 session orchestration) must therefore never request a
    /// reactivated slot's `confirmed_input` for frames below its activation frame
    /// minus one.
    ///
    /// For `frame == 0` with `frame_delay == 0` this reproduces fresh-queue
    /// behavior: `last_added_frame` becomes [`Frame::NULL`] and `first_frame`
    /// becomes `true`, so the first-input path of [`Self::add_input`] is taken.
    ///
    /// # No-op on invalid input
    /// `frame` must be non-negative. On a negative or [`Frame::NULL`] frame this
    /// reports a violation and leaves the queue **unchanged** (it does not
    /// panic).
    // dead_code: consumed by chunk 5's session orchestration; the public session
    // API that drives reactivation/seek lands there.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn reset_to_frame(&mut self, frame: Frame) {
        if frame.as_i32() < 0 {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "reset_to_frame called with negative frame {} (frames must be non-negative)",
                frame
            );
            return;
        }

        // `last_added_frame` is the frame stamp of the most recently accepted
        // input *with frame delay applied*. After this reset the next accepted
        // input is for game `frame`, so its delayed stamp is `frame +
        // frame_delay`. `add_input` requires `input.frame + frame_delay ==
        // last_added_frame + 1` (unless `last_added_frame.is_null()`), so we set
        // `last_added_frame = frame + frame_delay - 1`. Compute it with checked
        // arithmetic and bail (leaving the queue unchanged) on overflow, matching
        // the file's arithmetic-safety idiom. We do this *before* mutating any
        // field so the early return preserves the queue's prior state.
        let Some(delayed) = frame.checked_add(self.frame_delay as i32) else {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "reset_to_frame frame {} + frame_delay {} overflowed",
                frame,
                self.frame_delay
            );
            return;
        };
        let Some(new_last_added) = delayed.checked_sub(1) else {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "reset_to_frame last_added_frame underflowed for frame {} (delay {})",
                frame,
                self.frame_delay
            );
            return;
        };

        // Re-blank every slot so `input(frame)` predicts cleanly: `input`
        // errors if the requested frame is before `inputs[tail].frame`, and a
        // NULL tail-slot frame (slot `tail == 0` below) is always `<=` any
        // valid requested frame.
        for slot in self.inputs.iter_mut() {
            *slot = PlayerInput::blank_input(Frame::NULL);
        }

        // Align the empty circular buffer so the *next* accepted write lands at
        // the absolute slot `confirmed_input` reads for that frame.
        // `confirmed_input(f)` indexes absolutely as `inputs[f % queue_length]`;
        // a normally-filled queue upholds "frame X lives at slot X %
        // queue_length" because it starts at `head == 0` and advances one slot
        // per sequential add. The next add here is for the delayed frame `frame
        // + frame_delay` (== `new_last_added + 1` == `delayed`), so we must place
        // `head`/`tail` at `delayed % queue_length`. (The previous `head = tail =
        // 0` broke this: a write for frame F landed at slot 0 but
        // `confirmed_input(F)` looked at slot `F % queue_length`.)
        //
        // `delayed` is known non-negative here: `frame >= 0` (guarded above) and
        // `self.frame_delay` is a `usize`, so `frame + frame_delay >= 0`. The
        // `as usize` cast is therefore lossless for the in-range value.
        let next_slot = (delayed.as_i32() as usize) % self.queue_length;
        self.head = next_slot;
        self.tail = next_slot;
        self.length = 0;
        self.last_added_frame = new_last_added;
        // For `frame == 0, frame_delay == 0`, `new_last_added == Frame::NULL`,
        // which puts the queue back on the fresh-queue first-input path.
        self.first_frame = self.last_added_frame.is_null();
        self.first_incorrect_frame = Frame::NULL;
        self.last_requested_frame = Frame::NULL;
        self.prediction = PlayerInput::blank_input(Frame::NULL);
        self.frozen = false;
        // `last_confirmed_input` is intentionally preserved: it is the
        // deterministic prediction base (frozen value on a reactivated host
        // queue, `None` on a fresh joiner queue).

        // Seed the "phantom previous input" slot when the queue is *not* on the
        // first-input path. `add_input(frame)` routes through
        // `advance_queue_head`, which (with `first_frame == false`) derives its
        // expected next frame from the slot just behind `head` — i.e. the
        // most-recently-added position. With `head == next_slot`, that slot is
        // `next_slot - 1` (wrapping to `queue_length - 1` when `next_slot == 0`).
        // The circular-buffer contract `add_input_by_frame` upholds is that this
        // slot's frame equals `last_added_frame`; without restoring it the
        // (just-blanked, NULL-framed) slot would make `advance_queue_head`
        // expect frame 0 and attempt a spurious gap-fill, dropping the input. We
        // stamp it with `last_added_frame` and the preserved confirmed value
        // (the value is never read for this empty queue — `input` predicts via
        // `last_confirmed_input` — but keeping it consistent avoids surprises).
        // When `last_added_frame.is_null()` (`first_frame == true`, the frame-0
        // path) `advance_queue_head` ignores this slot entirely, so we leave it
        // blank.
        if !self.last_added_frame.is_null() {
            let phantom = PlayerInput {
                frame: self.last_added_frame,
                input: self.last_confirmed_input.unwrap_or_default(),
            };
            // Phantom predecessor lives at `head - 1`, wrapping. `next_slot` and
            // `queue_length` are both valid here, so this is a plain modular
            // predecessor (no `saturating_sub` masking an out-of-range index).
            let phantom_position = if next_slot == 0 {
                self.queue_length - 1
            } else {
                next_slot - 1
            };
            if let Some(slot) = self.inputs.get_mut(phantom_position) {
                *slot = phantom;
            } else {
                report_violation!(
                    ViolationSeverity::Critical,
                    ViolationKind::InputQueue,
                    "reset_to_frame: phantom previous slot index {} out of bounds (queue_length {})",
                    phantom_position,
                    self.queue_length
                );
            }
        }
    }

    /// Adds an input frame to the queue. Will consider the set frame delay.
    ///
    /// If the queue has been frozen via [`Self::freeze`], this method is a
    /// no-op: it returns the queue's current `last_added_frame` (which may be
    /// [`Frame::NULL`] if no input was ever added) without modifying any
    /// queue state. Returning the existing `last_added_frame` rather than
    /// [`Frame::NULL`] avoids signalling a "drop" to callers that distinguish
    /// drops from accepted inputs while still indicating no progress was made.
    pub fn add_input(&mut self, input: PlayerInput<T::Input>) -> Frame {
        if self.frozen {
            // Silently ignore inputs while frozen. Return the existing
            // last_added_frame so callers do not interpret this as a "drop"
            // (Frame::NULL). No queue state is mutated.
            return self.last_added_frame;
        }

        // Verify that inputs are passed in sequentially by the user, regardless of frame delay.
        let input_with_delay = safe_frame_add!(
            input.frame,
            self.frame_delay as i32,
            "InputQueue::add_input delay"
        );
        let expected_frame =
            safe_frame_add!(self.last_added_frame, 1, "InputQueue::add_input expected");
        if !self.last_added_frame.is_null() && input_with_delay != expected_frame {
            // drop the input if not given sequentially
            return Frame::NULL;
        }

        // Move the queue head to the correct point in preparation to input the frame into the queue.
        let new_frame = self.advance_queue_head(input.frame);
        // if the frame is valid, then add the input
        if !new_frame.is_null() && !self.add_input_by_frame(input, new_frame) {
            // Invariant violation occurred during add
            return Frame::NULL;
        }
        new_frame
    }

    /// Adds an input frame to the queue at the given frame number. If there are predicted inputs, we will check those and mark them as incorrect, if necessary.
    /// Returns true if the input was added successfully, false if an invariant violation was detected.
    fn add_input_by_frame(&mut self, input: PlayerInput<T::Input>, frame_number: Frame) -> bool {
        let previous_position = match self.head {
            0 => self.queue_length - 1,
            _ => self.head - 1,
        };

        // Verify inputs are added sequentially
        let expected_next = safe_frame_add!(
            self.last_added_frame,
            1,
            "InputQueue::add_input_by_frame expected"
        );
        if !self.last_added_frame.is_null() && frame_number != expected_next {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "Input frame {} is not sequential (last_added={})",
                frame_number,
                self.last_added_frame
            );
            return false;
        }
        if frame_number != 0 {
            let expected_prev =
                safe_frame_sub!(frame_number, 1, "InputQueue::add_input_by_frame prev");
            if let Some(prev_input) = self.inputs.get(previous_position) {
                if prev_input.frame != expected_prev {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::InputQueue,
                        "Previous input frame {} does not precede current frame {}",
                        prev_input.frame,
                        frame_number
                    );
                    return false;
                }
            } else {
                report_violation!(
                    ViolationSeverity::Critical,
                    ViolationKind::InputQueue,
                    "Invalid previous_position {} in add_input_by_frame",
                    previous_position
                );
                return false;
            }
        }

        // Add the frame to the back of the queue
        if let Some(head_input) = self.inputs.get_mut(self.head) {
            *head_input = input;
            head_input.frame = frame_number;
        } else {
            report_violation!(
                ViolationSeverity::Critical,
                ViolationKind::InputQueue,
                "Invalid head index {} in add_input_by_frame",
                self.head
            );
            return false;
        }
        let Some(next_head) = circular_index_add(self.head, 1, self.queue_length) else {
            report_violation!(
                ViolationSeverity::Critical,
                ViolationKind::InputQueue,
                "Failed to advance head {} within queue length {}",
                self.head,
                self.queue_length
            );
            return false;
        };
        let Some(next_length) = self.length.checked_add(1) else {
            report_violation!(
                ViolationSeverity::Critical,
                ViolationKind::InputQueue,
                "Queue length overflow while adding input (length={}, capacity={})",
                self.length,
                self.queue_length
            );
            return false;
        };
        self.head = next_head;
        self.length = next_length;

        // Verify queue doesn't overflow
        if self.length > self.queue_length {
            report_violation!(
                ViolationSeverity::Critical,
                ViolationKind::InputQueue,
                "Queue overflow: length {} exceeds capacity {}",
                self.length,
                self.queue_length
            );
            // Restore invariant by capping length
            self.length = self.queue_length;
            return false;
        }

        self.first_frame = false;
        self.last_added_frame = frame_number;

        // Update the last confirmed input. This is critical for deterministic predictions.
        // All inputs added to the queue are confirmed (either local or received from network).
        self.last_confirmed_input = Some(input.input);

        // We have been predicting. See if the inputs we've gotten match what we've been predicting. If so, don't worry about it.
        if !self.prediction.frame.is_null() {
            if frame_number != self.prediction.frame {
                // IMPOSSIBLE STATE. A prediction episode enters at the queue's
                // first missing frame (see `input`) and advances by exactly one
                // per accepted arrival, while the sequential guards above force
                // `frame_number == last_added_frame + 1`, so the two always
                // agree. If the bookkeeping is ever observed broken, fail
                // TOWARD rollback: the input was physically added above, so
                // silently skipping the misprediction comparison would leave a
                // permanently divergent applied trajectory with no rollback and
                // no event (finding F17). Conservatively mark the earlier of
                // the two frames as mispredicted so the next `advance_frame`
                // re-simulates through both, and realign the episode to the
                // arrival so subsequent arrivals keep being compared.
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InputQueue,
                    "Frame {} doesn't match prediction frame {}",
                    frame_number,
                    self.prediction.frame
                );
                if self.first_incorrect_frame.is_null() {
                    self.first_incorrect_frame = cmp::min(frame_number, self.prediction.frame);
                }
                self.prediction.frame = safe_frame_add!(
                    frame_number,
                    1,
                    "InputQueue::add_input_by_frame prediction realign"
                );
                return true;
            }

            // Remember the first input which was incorrect so we can report it
            if self.first_incorrect_frame.is_null() && !self.prediction.equal(&input, true) {
                self.first_incorrect_frame = frame_number;
            }

            // If this input is the same frame as the last one requested and we still haven't found any mispredicted inputs, we can exit prediction mode.
            // Otherwise, advance the prediction frame count up.
            if self.prediction.frame == self.last_requested_frame
                && self.first_incorrect_frame.is_null()
            {
                self.prediction.frame = Frame::NULL;
            } else {
                self.prediction.frame = safe_frame_add!(
                    self.prediction.frame,
                    1,
                    "InputQueue::add_input_by_frame prediction"
                );
            }
        }

        true
    }

    /// Advances the queue head to the next frame, handling frame delay by filling gaps.
    ///
    /// When frame delay is configured, this function fills the gap between the expected
    /// frame and the actual input frame by replicating the previous input. This is
    /// necessary for initial inputs when delay > 0.
    ///
    /// Returns [`Frame::NULL`] if the input would be out of order (expected > input with delay).
    /// Otherwise, returns the frame number with delay applied.
    ///
    /// # Note on mid-session delay changes
    ///
    /// Mid-session **increases** of `frame_delay` are supported (see
    /// [`set_frame_delay`]); the gap created by the larger delay is back-filled by
    /// replicating the most recently added input, performed inside `set_frame_delay`
    /// itself. By the time the next user input reaches `add_input` after such an
    /// increase, the queue's `last_added_frame` has already been advanced so that the
    /// sequential invariant `input_frame + frame_delay == last_added_frame + 1` holds
    /// without needing another gap-fill in this function.
    ///
    /// Mid-session **decreases** of `frame_delay` are rejected by `set_frame_delay`
    /// and therefore do not reach this function.
    ///
    /// [`set_frame_delay`]: Self::set_frame_delay
    /// [`add_input`]: Self::add_input
    fn advance_queue_head(&mut self, mut input_frame: Frame) -> Frame {
        let previous_position = match self.head {
            0 => self.queue_length - 1,
            _ => self.head - 1,
        };

        let mut expected_frame = if self.first_frame {
            Frame::new(0)
        } else {
            match self.inputs.get(previous_position) {
                Some(prev_input) => {
                    safe_frame_add!(
                        prev_input.frame,
                        1,
                        "InputQueue::advance_queue_head expected"
                    )
                },
                None => {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InputQueue,
                        "Invalid previous_position {} in advance_queue_head",
                        previous_position
                    );
                    return Frame::NULL;
                },
            }
        };

        input_frame = safe_frame_add!(
            input_frame,
            self.frame_delay as i32,
            "InputQueue::advance_queue_head delay"
        );

        // If the expected frame is ahead of the input (frame delay decreased), reject the input
        if expected_frame > input_frame {
            return Frame::NULL;
        }

        // Fill any gap between expected_frame and input_frame by replicating the previous input.
        // This handles the initial delay setup when frame_delay > 0.
        while expected_frame < input_frame {
            let input_to_replicate = match self.inputs.get(previous_position) {
                Some(input) => *input,
                None => {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InputQueue,
                        "Invalid previous_position {} in gap fill loop",
                        previous_position
                    );
                    return Frame::NULL;
                },
            };
            if !self.add_input_by_frame(input_to_replicate, expected_frame) {
                return Frame::NULL;
            }
            expected_frame =
                safe_frame_add!(expected_frame, 1, "InputQueue::advance_queue_head gap fill");
        }

        // After filling gaps, verify the frame is sequential
        let previous_position = match self.head {
            0 => self.queue_length - 1,
            _ => self.head - 1,
        };
        if input_frame != 0 {
            match self.inputs.get(previous_position) {
                Some(prev_input) => {
                    let expected = safe_frame_add!(
                        prev_input.frame,
                        1,
                        "InputQueue::advance_queue_head verify"
                    );
                    if input_frame != expected {
                        report_violation!(
                            ViolationSeverity::Error,
                            ViolationKind::InputQueue,
                            "Frame sequencing broken after gap fill: input_frame={}, prev_frame={}",
                            input_frame,
                            prev_input.frame
                        );
                        return Frame::NULL;
                    }
                },
                None => {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InputQueue,
                        "Invalid previous_position {} after gap fill",
                        previous_position
                    );
                    return Frame::NULL;
                },
            }
        }
        input_frame
    }
}

impl<T: Config> InvariantChecker for InputQueue<T> {
    /// Checks the invariants of the InputQueue.
    ///
    /// # Invariants
    ///
    /// 1. `length` must not exceed `queue_length`
    /// 2. `head` and `tail` must be valid indices (< queue_length)
    /// 3. `length` must be consistent with head/tail positions (accounting for full queue)
    /// 4. If `length > 0`, the frames in the queue should be consecutive
    /// 5. `frame_delay` must be within reasonable bounds
    /// 6. `first_incorrect_frame` should be NULL_FRAME or >= 0
    fn check_invariants(&self) -> Result<(), InvariantViolation> {
        // Invariant 1: length <= queue_length
        if self.length > self.queue_length {
            return Err(
                InvariantViolation::new("InputQueue", "length exceeds queue_length")
                    .with_bounds_violation("length", self.length, 0, self.queue_length),
            );
        }

        // Invariant 2: head and tail are valid indices
        if self.head >= self.queue_length {
            return Err(
                InvariantViolation::new("InputQueue", "head index out of bounds")
                    .with_bounds_violation("head", self.head, 0, self.queue_length - 1),
            );
        }

        if self.tail >= self.queue_length {
            return Err(
                InvariantViolation::new("InputQueue", "tail index out of bounds")
                    .with_bounds_violation("tail", self.tail, 0, self.queue_length - 1),
            );
        }

        // Invariant 3: length is consistent with head/tail positions
        // Note: In a circular buffer, when head == tail, the queue can be either empty (length=0)
        // or full (length=queue_length). We cannot distinguish these from head/tail alone.
        // The stored `length` field is the source of truth.
        //
        // We verify: if head != tail, length must match the circular distance.
        // If head == tail, length must be either 0 or queue_length.
        if self.head == self.tail {
            // When head == tail, queue is either empty or full
            if self.length != 0 && self.length != self.queue_length {
                return Err(InvariantViolation::new(
                    "InputQueue",
                    "when head==tail, length must be 0 (empty) or queue_length (full)",
                )
                .with_details(format!(
                    "length={}, queue_length={}, head={}, tail={}",
                    self.length, self.queue_length, self.head, self.tail
                )));
            }
        } else {
            // When head != tail, we can calculate the expected length
            let calculated_length = if self.head > self.tail {
                self.head - self.tail
            } else {
                self.queue_length - self.tail + self.head
            };

            if self.length != calculated_length {
                return Err(InvariantViolation::new(
                    "InputQueue",
                    "length does not match head/tail positions",
                )
                .with_details(format!(
                    "length={}, calculated={}, head={}, tail={}",
                    self.length, calculated_length, self.head, self.tail
                )));
            }
        }

        // Invariant 4: inputs vector has correct size
        if self.inputs.len() != self.queue_length {
            return Err(
                InvariantViolation::new("InputQueue", "inputs vector has incorrect size")
                    .with_bounds_violation(
                        "size",
                        self.inputs.len(),
                        self.queue_length,
                        self.queue_length,
                    ),
            );
        }

        // Invariant 5: frame_delay is reasonable (less than 256 frames)
        if self.frame_delay > 255 {
            return Err(InvariantViolation::new(
                "InputQueue",
                "frame_delay exceeds reasonable bounds",
            )
            .with_bounds_violation("frame_delay", self.frame_delay, 0, 255));
        }

        // Invariant 6: first_incorrect_frame is either NULL or a valid frame
        if !self.first_incorrect_frame.is_null() && self.first_incorrect_frame.as_i32() < 0 {
            return Err(
                InvariantViolation::new("InputQueue", "first_incorrect_frame is invalid")
                    .with_field_value("first_incorrect_frame", self.first_incorrect_frame),
            );
        }

        // Invariant 7: last_requested_frame is either NULL or a valid frame
        if !self.last_requested_frame.is_null() && self.last_requested_frame.as_i32() < 0 {
            return Err(
                InvariantViolation::new("InputQueue", "last_requested_frame is invalid")
                    .with_field_value("last_requested_frame", self.last_requested_frame),
            );
        }

        // Invariant 8: last_added_frame is either NULL or a valid frame
        if !self.last_added_frame.is_null() && self.last_added_frame.as_i32() < 0 {
            return Err(
                InvariantViolation::new("InputQueue", "last_added_frame is invalid")
                    .with_field_value("last_added_frame", self.last_added_frame),
            );
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
mod input_queue_tests {

    use std::net::SocketAddr;

    use serde::{Deserialize, Serialize};

    use super::*;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
    struct TestInput {
        inp: u8,
    }

    #[derive(Clone, Debug)]
    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = Vec<u8>;
        type Address = SocketAddr;
    }

    /// Helper to create a test queue, unwrapping the Option for test convenience.
    fn test_queue(player_index: usize) -> InputQueue<TestConfig> {
        InputQueue::<TestConfig>::new(player_index).expect("Failed to create test queue")
    }

    /// Helper to assert that every queue field remains unchanged.
    #[track_caller]
    fn assert_queue_unchanged(queue: &InputQueue<TestConfig>, before: &InputQueue<TestConfig>) {
        assert_eq!(queue.head, before.head);
        assert_eq!(queue.tail, before.tail);
        assert_eq!(queue.length, before.length);
        assert_eq!(queue.first_frame, before.first_frame);
        assert_eq!(queue.last_added_frame, before.last_added_frame);
        assert_eq!(queue.first_incorrect_frame, before.first_incorrect_frame);
        assert_eq!(queue.last_requested_frame, before.last_requested_frame);
        assert_eq!(queue.frame_delay, before.frame_delay);
        assert_eq!(queue.player_index, before.player_index);
        assert_eq!(queue.queue_length, before.queue_length);
        assert_eq!(queue.inputs, before.inputs);
        assert_eq!(queue.prediction, before.prediction);
        assert_eq!(queue.last_confirmed_input, before.last_confirmed_input);
        assert_eq!(queue.frozen, before.frozen);
    }

    #[test]
    fn circular_index_add_advances_without_wrap() {
        assert_eq!(circular_index_add(2, 3, 8), Some(5));
    }

    #[test]
    fn circular_index_add_wraps_without_overflow() {
        assert_eq!(circular_index_add(6, 5, 8), Some(3));
    }

    #[test]
    fn circular_index_add_handles_huge_offset() {
        assert_eq!(
            circular_index_add(usize::MAX - 2, usize::MAX - 1, usize::MAX),
            Some(usize::MAX - 3)
        );
    }

    #[test]
    fn circular_index_add_rejects_invalid_modulus_or_index() {
        assert_eq!(circular_index_add(0, 1, 0), None);
        assert_eq!(circular_index_add(8, 1, 8), None);
    }

    #[test]
    fn frame_distance_usize_rejects_overflow_or_negative_distance() {
        assert_eq!(
            frame_distance_usize(Frame::NULL, Frame::new(i32::MAX)),
            None
        );
        assert_eq!(frame_distance_usize(Frame::new(10), Frame::new(5)), None);
    }

    #[test]
    fn input_rejects_overflowing_request_distance_without_panic() {
        let mut queue = test_queue(0);

        let result = queue.input(Frame::new(i32::MAX));

        assert!(result.is_none());
    }

    #[test]
    fn test_add_input_wrong_frame() {
        let mut queue = test_queue(0);
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 0 });
        assert_eq!(queue.add_input(input), Frame::new(0)); // fine
        let input_wrong_frame = PlayerInput::new(Frame::new(3), TestInput { inp: 0 });
        assert_eq!(queue.add_input(input_wrong_frame), Frame::NULL); // input dropped
    }

    #[test]
    fn test_add_input_twice() {
        let mut queue = test_queue(0);
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 0 });
        assert_eq!(queue.add_input(input), Frame::new(0)); // fine
        assert_eq!(queue.add_input(input), Frame::NULL); // input dropped
    }

    #[test]
    fn test_add_input_sequentially() {
        let mut queue = test_queue(0);
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
            queue.add_input(input);
            assert_eq!(queue.last_added_frame, Frame::new(i));
            assert_eq!(queue.length, (i + 1) as usize);
        }
    }

    #[test]
    fn test_input_sequentially() {
        let mut queue = test_queue(0);
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
            assert_eq!(queue.last_added_frame, Frame::new(i));
            assert_eq!(queue.length, (i + 1) as usize);
            let (input_in_queue, _status) = queue
                .input(Frame::new(i))
                .expect("input should be available");
            assert_eq!(input_in_queue.inp, i as u8);
        }
    }

    #[test]
    fn test_delayed_inputs() {
        let mut queue = test_queue(0);
        let delay: i32 = 2;
        queue.set_frame_delay(delay as usize).expect("valid delay");
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
            assert_eq!(queue.last_added_frame, Frame::new(i + delay));
            assert_eq!(queue.length, (i + delay + 1) as usize);
            let (input_in_queue, _status) = queue
                .input(Frame::new(i))
                .expect("input should be available");
            let correct_input = std::cmp::max(0, i - delay) as u8;
            assert_eq!(input_in_queue.inp, correct_input);
        }
    }

    #[test]
    fn test_confirmed_input_success() {
        let mut queue = test_queue(0);
        // Add inputs for frames 0-4
        for i in 0..5i32 {
            let input = PlayerInput::new(
                Frame::new(i),
                TestInput {
                    inp: (i * 10) as u8,
                },
            );
            queue.add_input(input);
        }
        // Retrieve confirmed input for frame 2
        let confirmed = queue.confirmed_input(Frame::new(2)).unwrap();
        assert_eq!(confirmed.frame, Frame::new(2));
        assert_eq!(confirmed.input.inp, 20);
    }

    #[test]
    fn test_confirmed_input_not_found() {
        let mut queue = test_queue(0);
        // Add inputs for frames 0-2
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }
        // Try to get input for frame 10 which doesn't exist
        let result = queue.confirmed_input(Frame::new(10));
        assert!(result.is_err());
    }

    #[test]
    fn test_discard_confirmed_frames_partial() {
        let mut queue = test_queue(0);
        // Add 10 inputs
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }
        assert_eq!(queue.length, 10);

        // Discard frames up to 5
        queue.discard_confirmed_frames(Frame::new(5));

        // Should have discarded frames 0-4, keeping 5-9 (5 frames)
        assert_eq!(queue.length, 5);

        // Frame 5 should still be retrievable
        queue.confirmed_input(Frame::new(5)).unwrap();
    }

    #[test]
    fn test_discard_confirmed_frames_all_but_one() {
        let mut queue = test_queue(0);
        // Add 10 inputs
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Discard all frames (should keep at least the most recent)
        queue.discard_confirmed_frames(Frame::new(100));
        assert_eq!(queue.length, 1);
    }

    /// Chunk-N5 noise downgrade (S34 residual 5): discarding on an EMPTY
    /// queue is a quiet no-op — nothing exists to discard, so no arm runs
    /// and no state changes. The reactivation shape (a hot-join reopened
    /// queue is `reset_to_frame`-blanked to `length == 0` while the
    /// session's confirmed frame still trails the slot's first
    /// post-reactivation input) used to land in the offset arm with an
    /// Error-severity "Discard offset N exceeds queue length 0" violation
    /// on every survivor advance, and the `frame >= last_added_frame` arm
    /// would have fabricated `length = 1` out of a blanked slot.
    #[cfg(feature = "hot-join")]
    #[test]
    fn discard_confirmed_frames_on_empty_queue_is_quiet_noop() {
        let mut queue = test_queue(0);
        queue.reset_to_frame(Frame::new(11));
        assert_eq!(queue.length, 0, "reset_to_frame blanks the ring");
        let (head, tail, last_added) = (queue.head, queue.tail, queue.last_added_frame);

        // The reactivation-window shape: discard below the activation frame
        // (the offset arm's noise trigger pre-downgrade)...
        queue.discard_confirmed_frames(Frame::new(9));
        // ...and at/above last_added (the arm that would fabricate
        // `length = 1` from a blanked slot).
        queue.discard_confirmed_frames(Frame::new(11));

        assert_eq!(queue.length, 0, "an empty queue stays empty");
        assert_eq!(
            (queue.head, queue.tail, queue.last_added_frame),
            (head, tail, last_added),
            "the no-op leaves the ring untouched"
        );
        // The queue still accepts its repositioned first input normally.
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(11), TestInput { inp: 7 })),
            Frame::new(11)
        );
    }

    /// Test that discard_confirmed_frames keeps the most recent input accessible
    /// and maintains proper circular buffer invariants.
    #[test]
    fn test_discard_all_but_one_preserves_most_recent() {
        let mut queue = test_queue(0);

        // Add 5 inputs
        for i in 0..5i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Before discard: head=5, tail=0, length=5
        assert_eq!(queue.length, 5);
        assert_eq!(queue.head, 5);
        assert_eq!(queue.tail, 0);

        // Discard all - this triggers the "keep most recent" logic
        queue.discard_confirmed_frames(Frame::new(100));

        // Length should be 1 (keeping the most recent input)
        assert_eq!(queue.length, 1);

        // After fix: tail should point to the last input (head - 1 = 4)
        // This maintains the invariant: length = head - tail = 5 - 4 = 1
        assert_eq!(queue.tail, 4);
        assert_eq!(queue.head, 5);

        // Verify the invariants now pass
        assert!(
            queue.check_invariants().is_ok(),
            "Invariants should pass after discard_all_but_one"
        );

        // The most recent input (frame 4) should still be accessible
        let confirmed = queue.confirmed_input(Frame::new(4)).unwrap();
        assert_eq!(confirmed.input.inp, 4);
    }

    /// Regression test: discard_confirmed_frames with head at position 0 (wraparound edge case)
    #[test]
    fn test_discard_all_but_one_with_head_at_zero() {
        let mut queue = test_queue(0);

        // Fill the queue completely and then some to cause head wraparound
        // INPUT_QUEUE_LENGTH = 128, so adding 128 inputs will put head at 0
        for i in 0..128i32 {
            let input = PlayerInput::new(
                Frame::new(i),
                TestInput {
                    inp: (i % 256) as u8,
                },
            );
            queue.add_input(input);
        }

        // Head should now be at 0 (wrapped around)
        assert_eq!(queue.head, 0);
        assert_eq!(queue.length, 128);

        // Discard all but most recent
        queue.discard_confirmed_frames(Frame::new(1000));

        // Should keep exactly 1 input
        assert_eq!(queue.length, 1);

        // Tail should be at 127 (head - 1 with wraparound: 0 - 1 = -1 -> 127)
        assert_eq!(queue.tail, 127);

        // Invariants should pass
        assert!(
            queue.check_invariants().is_ok(),
            "Invariants should pass with head at 0"
        );

        // The most recent input (frame 127) should be accessible
        let confirmed = queue.confirmed_input(Frame::new(127)).unwrap();
        assert_eq!(confirmed.input.inp, 127);
    }

    /// Regression test: multiple consecutive discard_all_but_one operations maintain invariants
    #[test]
    fn test_multiple_discards_maintain_invariants() {
        let mut queue = test_queue(0);

        for cycle in 0..3 {
            // Add some inputs
            for i in 0..10i32 {
                let frame = Frame::new(cycle * 10 + i);
                let input = PlayerInput::new(frame, TestInput { inp: i as u8 });
                queue.add_input(input);
            }

            // Discard all but most recent
            queue.discard_confirmed_frames(Frame::new(10000));

            // Should have exactly 1 input
            assert_eq!(queue.length, 1, "Cycle {} should have length 1", cycle);

            // Invariants should pass
            assert!(
                queue.check_invariants().is_ok(),
                "Invariants should pass after cycle {}",
                cycle
            );
        }
    }

    /// Regression test: verify discard preserves the correct frame value, not just any input
    #[test]
    fn test_discard_all_preserves_correct_frame_value() {
        let mut queue = test_queue(0);

        // Add inputs with unique values to verify we keep the RIGHT one
        for i in 0..5i32 {
            let input = PlayerInput::new(
                Frame::new(i),
                TestInput {
                    inp: (i * 10 + 5) as u8,
                },
            );
            queue.add_input(input);
        }

        // Last added is frame 4, with value 45
        queue.discard_confirmed_frames(Frame::new(100));

        assert_eq!(queue.length, 1);

        // Verify we kept frame 4 with value 45
        let input = queue.confirmed_input(Frame::new(4)).unwrap();
        assert_eq!(input.input.inp, 45);
        assert_eq!(input.frame, Frame::new(4));
    }

    /// Regression test: full lifecycle of add -> discard -> add maintains invariants
    #[test]
    fn test_full_lifecycle_maintains_invariants() {
        let mut queue = test_queue(0);

        // Phase 1: Add initial inputs
        for i in 0..5i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }
        assert!(queue.check_invariants().is_ok(), "Phase 1 invariants");

        // Phase 2: Partial discard
        queue.discard_confirmed_frames(Frame::new(2));
        assert!(queue.check_invariants().is_ok(), "Phase 2 invariants");

        // Phase 3: Add more inputs
        for i in 5..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }
        assert!(queue.check_invariants().is_ok(), "Phase 3 invariants");

        // Phase 4: Discard all but one
        queue.discard_confirmed_frames(Frame::new(100));
        assert!(queue.check_invariants().is_ok(), "Phase 4 invariants");
        assert_eq!(queue.length, 1);

        // Phase 5: Continue adding
        for i in 10..15i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }
        assert!(queue.check_invariants().is_ok(), "Phase 5 invariants");

        // Should now have 6 inputs (1 kept + 5 new)
        assert_eq!(queue.length, 6);
    }

    #[test]
    fn test_discard_confirmed_frames_respects_last_requested() {
        let mut queue = test_queue(0);
        // Add 10 inputs
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Request frame 3 (this sets last_requested_frame)
        let _ = queue.input(Frame::new(3)).expect("input");

        // Try to discard up to frame 8, but should only discard up to 3
        queue.discard_confirmed_frames(Frame::new(8));

        // Frame 3 should still be available
        queue.confirmed_input(Frame::new(3)).unwrap();
    }

    #[test]
    fn test_discard_nothing_when_frame_before_tail() {
        let mut queue = test_queue(0);
        // Add inputs for frames 0-9
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }
        let initial_length = queue.length;

        // Discard frame -1 (before any frames)
        queue.discard_confirmed_frames(Frame::new(-1));

        // Nothing should be discarded
        assert_eq!(queue.length, initial_length);
    }

    #[test]
    fn test_reset_prediction() {
        let mut queue = test_queue(0);
        // Add a couple of inputs
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Request a frame beyond what we have (triggers prediction)
        let (_, status) = queue.input(Frame::new(5)).expect("input");
        assert_eq!(status, InputStatus::Predicted);
        assert!(queue.prediction.frame.as_i32() >= 0);

        // Reset prediction
        queue.reset_prediction();
        assert_eq!(queue.prediction.frame, Frame::NULL);
        assert_eq!(queue.first_incorrect_frame, Frame::NULL);
        assert_eq!(queue.last_requested_frame, Frame::NULL);
    }

    #[test]
    fn test_prediction_returns_last_confirmed_input() {
        let mut queue = test_queue(0);
        // Add inputs with specific values
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 42 }); // All inputs are 42
            queue.add_input(input);
        }

        // Request frame 5 (beyond what we have)
        let (predicted_input, status) = queue.input(Frame::new(5)).expect("input");
        assert_eq!(status, InputStatus::Predicted);
        // Prediction uses RepeatLastConfirmed - returns last confirmed input (42)
        assert_eq!(predicted_input.inp, 42);
    }

    #[test]
    fn test_first_incorrect_frame_detection() {
        let mut queue = test_queue(0);
        // Add initial input with a specific value
        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 10 });
        queue.add_input(input0);

        // Request frame 1 (triggers prediction using RepeatLastConfirmed = 10)
        let (predicted, status) = queue.input(Frame::new(1)).expect("input");
        assert_eq!(status, InputStatus::Predicted);
        assert_eq!(predicted.inp, 10); // Predicted to be last confirmed (10)

        // Now add the actual input for frame 1 with DIFFERENT value
        let input1 = PlayerInput::new(Frame::new(1), TestInput { inp: 99 }); // Different from 10!
        queue.add_input(input1);

        // The first incorrect frame should be detected (prediction was 10, actual was 99)
        assert_eq!(queue.first_incorrect_frame(), Frame::new(1));
    }

    #[test]
    fn test_first_incorrect_frame_correct_prediction() {
        let mut queue = test_queue(0);
        // Add initial input with specific value
        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 42 });
        queue.add_input(input0);

        // Request frame 1 (triggers prediction using RepeatLastConfirmed = 42)
        let _ = queue.input(Frame::new(1)).expect("input");

        // Add actual input for frame 1 with SAME value (correct prediction)
        let input1 = PlayerInput::new(Frame::new(1), TestInput { inp: 42 }); // Same as prediction
        queue.add_input(input1);

        // No incorrect frame should be detected
        assert_eq!(queue.first_incorrect_frame(), Frame::NULL);
    }

    #[test]
    fn test_queue_wraparound() {
        let mut queue = test_queue(0);

        // Add more inputs than queue capacity to test wraparound
        // We add INPUT_QUEUE_LENGTH inputs, then discard some, then add more
        for i in 0..64i32 {
            let input = PlayerInput::new(
                Frame::new(i),
                TestInput {
                    inp: (i % 256) as u8,
                },
            );
            queue.add_input(input);
        }

        // Discard old frames
        queue.discard_confirmed_frames(Frame::new(60));

        // Add more inputs that will wrap around
        for i in 64..100i32 {
            let input = PlayerInput::new(
                Frame::new(i),
                TestInput {
                    inp: (i % 256) as u8,
                },
            );
            queue.add_input(input);
        }

        // Verify we can still retrieve the most recent inputs
        let confirmed = queue.confirmed_input(Frame::new(99)).unwrap();
        assert_eq!(confirmed.input.inp, 99);
    }

    #[test]
    fn test_input_returns_confirmed_status() {
        let mut queue = test_queue(0);
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        queue.add_input(input);

        let (retrieved, status) = queue.input(Frame::new(0)).expect("input");
        assert_eq!(status, InputStatus::Confirmed);
        assert_eq!(retrieved.inp, 5);
    }

    #[test]
    fn test_input_returns_predicted_status() {
        let mut queue = test_queue(0);
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        queue.add_input(input);

        // Request frame beyond what we have
        let (_, status) = queue.input(Frame::new(10)).expect("input");
        assert_eq!(status, InputStatus::Predicted);
    }

    #[test]
    fn test_frame_delay_change_increase() {
        let mut queue = test_queue(0);

        // Start with delay of 2 from the beginning
        queue.set_frame_delay(2).expect("valid delay");

        // Add first input (frame 0)
        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 1 });
        queue.add_input(input0);
        // With delay 2, frame 0 becomes frame 2 in the queue
        assert_eq!(queue.last_added_frame, Frame::new(2));

        // Add second input (frame 1)
        let input1 = PlayerInput::new(Frame::new(1), TestInput { inp: 2 });
        queue.add_input(input1);
        // With delay 2, frame 1 becomes frame 3 in the queue
        assert_eq!(queue.last_added_frame, Frame::new(3));
    }

    /// Increasing frame delay mid-session replicates the most recently added
    /// input across the new gap so subsequent sequential inputs continue to be
    /// accepted at `input.frame + new_delay == last_added_frame + 1`.
    #[test]
    fn test_frame_delay_increase_mid_session_replicates_last_input() {
        let mut queue = test_queue(0);

        // Add inputs at frames 0..=2 with delay 0.
        for i in 0..=2i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            assert_eq!(queue.add_input(input), Frame::new(i));
        }
        assert_eq!(queue.last_added_frame, Frame::new(2));

        // Increase delay mid-session to 2. Two filler frames (3 and 4) should be
        // replicated from the last input (frame 2, inp=2).
        queue.set_frame_delay(2).expect("increase mid-session ok");
        assert_eq!(queue.last_added_frame, Frame::new(4));

        // Replicated frames should be confirmable and equal to the previous input.
        let frame3 = queue
            .confirmed_input(Frame::new(3))
            .expect("frame 3 replicated");
        let frame4 = queue
            .confirmed_input(Frame::new(4))
            .expect("frame 4 replicated");
        assert_eq!(frame3.input.inp, 2);
        assert_eq!(frame4.input.inp, 2);

        // Adding a new input at frame 3 (with delay 2) should land at frame 5.
        let new_input = PlayerInput::new(Frame::new(3), TestInput { inp: 99 });
        assert_eq!(queue.add_input(new_input), Frame::new(5));
        assert_eq!(queue.last_added_frame, Frame::new(5));
        let frame5 = queue.confirmed_input(Frame::new(5)).expect("frame 5 added");
        assert_eq!(frame5.input.inp, 99);
    }

    /// Decreasing frame delay mid-session is unsupported and must return a
    /// `InputDelayDecreaseUnsupported` error.
    #[test]
    fn test_frame_delay_decrease_mid_session_returns_error() {
        let mut queue = test_queue(0);
        queue.set_frame_delay(2).expect("initial delay 2");

        // Add a single input so we are mid-session.
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 1 });
        queue.add_input(input);
        let before = queue.clone();

        let err = queue
            .set_frame_delay(1)
            .expect_err("decrease should be rejected");
        match err {
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::InputDelayDecreaseUnsupported { current, requested },
            } => {
                assert_eq!(current, 2);
                assert_eq!(requested, 1);
            },
            other => panic!("unexpected error variant: {other:?}"),
        }
        assert_queue_unchanged(&queue, &before);
    }

    /// Increasing frame delay mid-session must be transactional: if the queue
    /// cannot accept all filler frames, no filler frame or delay update remains.
    #[test]
    fn test_frame_delay_increase_gap_fill_failure_preserves_state() {
        let mut queue =
            InputQueue::<TestConfig>::with_queue_length(0, 4).expect("valid queue length");

        for i in 0..4i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            assert_eq!(queue.add_input(input), Frame::new(i));
        }
        assert_eq!(queue.length, queue.queue_length);
        queue.check_invariants().expect("full queue is valid");
        let before = queue.clone();

        let err = queue
            .set_frame_delay(1)
            .expect_err("full queue cannot accept gap-fill frame");
        match err {
            FortressError::InternalErrorStructured {
                kind: InternalErrorKind::InputQueueGapFillFailed { frame },
            } => {
                assert_eq!(frame, Frame::new(4));
            },
            other => panic!("unexpected error variant: {other:?}"),
        }

        assert_queue_unchanged(&queue, &before);
        queue
            .check_invariants()
            .expect("failed delay increase preserves invariants");
        assert!(
            queue.confirmed_input(Frame::new(4)).is_err(),
            "failed gap fill must not leave a filler frame behind"
        );
    }

    /// Setting the frame delay to its current value is a no-op even mid-session.
    #[test]
    fn test_frame_delay_no_op_when_unchanged() {
        let mut queue = test_queue(0);
        queue.set_frame_delay(1).expect("initial delay 1");

        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 7 });
        queue.add_input(input);
        let last_added_before = queue.last_added_frame;

        // Setting to the same value should be a no-op (no gap fill, no error).
        queue
            .set_frame_delay(1)
            .expect("no-op set should succeed mid-session");
        assert_eq!(queue.frame_delay(), 1);
        assert_eq!(queue.last_added_frame, last_added_before);
    }

    /// Initial-setup case: setting the frame delay before any inputs are added
    /// should always succeed (including decreases) because there are no queued
    /// inputs to invalidate.
    #[test]
    fn test_frame_delay_initial_setup_no_inputs_yet() {
        let mut queue = test_queue(0);
        assert_eq!(queue.frame_delay(), 0);

        queue.set_frame_delay(5).expect("set delay to 5");
        assert_eq!(queue.frame_delay(), 5);

        // Decreasing before any inputs is allowed because last_added_frame is NULL.
        queue.set_frame_delay(2).expect("decrease before inputs");
        assert_eq!(queue.frame_delay(), 2);

        queue.set_frame_delay(0).expect("decrease to zero");
        assert_eq!(queue.frame_delay(), 0);
    }

    #[test]
    fn test_blank_prediction_on_frame_zero() {
        let mut queue = test_queue(0);

        // Request frame 0 without any inputs (edge case)
        // This should return a blank prediction
        let (predicted, status) = queue.input(Frame::new(0)).expect("input");
        assert_eq!(status, InputStatus::Predicted);
        assert_eq!(predicted.inp, TestInput::default().inp);
    }

    // ==========================================
    // Invariant Checker Tests
    // ==========================================

    #[test]
    fn test_invariant_checker_new_queue() {
        let queue = test_queue(0);
        queue.check_invariants().unwrap();
    }

    #[test]
    fn test_invariant_checker_after_add_input() {
        let mut queue = test_queue(0);
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
            assert!(
                queue.check_invariants().is_ok(),
                "Invariants broken after adding frame {}",
                i
            );
        }
    }

    #[test]
    fn test_invariant_checker_after_discard() {
        let mut queue = test_queue(0);
        for i in 0..20i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        queue.discard_confirmed_frames(Frame::new(10));
        queue.check_invariants().unwrap();
    }

    #[test]
    fn test_invariant_checker_with_frame_delay() {
        let mut queue = test_queue(0);
        queue.set_frame_delay(5).expect("valid delay");

        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
            queue.check_invariants().unwrap();
        }
    }

    #[test]
    fn test_invariant_checker_after_reset_prediction() {
        let mut queue = test_queue(0);
        for i in 0..5i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
            queue.add_input(input);
        }
        let _ = queue.input(Frame::new(10)).expect("input"); // Trigger prediction

        queue.reset_prediction();
        queue.check_invariants().unwrap();
    }

    // ========================================================================
    // Frame Delay Bounds Checking Tests
    // These tests verify the fix for a Kani-discovered edge case where
    // frame_delay >= INPUT_QUEUE_LENGTH could cause circular buffer overflow.
    // ========================================================================

    /// Regression test: set_frame_delay rejects delay >= INPUT_QUEUE_LENGTH
    /// This constraint was discovered through Kani formal verification.
    #[test]
    fn test_set_frame_delay_rejects_excessive_delay() {
        let mut queue = test_queue(0);

        // Delay exactly at the limit should fail
        let result = queue.set_frame_delay(INPUT_QUEUE_LENGTH);
        assert!(result.is_err());
        if let Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::FrameDelayTooLarge { delay, max_delay },
        }) = result
        {
            assert_eq!(delay, INPUT_QUEUE_LENGTH);
            assert_eq!(max_delay, INPUT_QUEUE_LENGTH - 1);
        } else {
            panic!(
                "Expected InvalidRequestStructured with FrameDelayTooLarge, got {:?}",
                result
            );
        }

        // Delay well above limit should also fail
        let result = queue.set_frame_delay(INPUT_QUEUE_LENGTH + 100);
        assert!(result.is_err());
    }

    /// Test: Maximum allowed delay (INPUT_QUEUE_LENGTH - 1) should be accepted
    #[test]
    fn test_set_frame_delay_accepts_max_valid_delay() {
        let mut queue = test_queue(0);

        // Delay at MAX_FRAME_DELAY should succeed
        queue.set_frame_delay(MAX_FRAME_DELAY).unwrap();
        assert_eq!(queue.frame_delay, MAX_FRAME_DELAY);
    }

    /// Test: Zero delay should be accepted (common case)
    #[test]
    fn test_set_frame_delay_accepts_zero() {
        let mut queue = test_queue(0);

        queue.set_frame_delay(0).unwrap();
        assert_eq!(queue.frame_delay, 0);
    }

    /// Test: Typical game delays (1-8 frames) should all be accepted
    #[test]
    fn test_set_frame_delay_accepts_typical_values() {
        for delay in 1..=8 {
            let mut queue = test_queue(0);
            let result = queue.set_frame_delay(delay);
            assert!(result.is_ok(), "Delay {} should be accepted", delay);
            assert_eq!(queue.frame_delay, delay);
        }
    }

    /// Test: `frame_delay()` getter returns the value set by `set_frame_delay()`
    #[test]
    fn test_frame_delay_getter_round_trip() {
        // In tests: unwrap is allowed
        let mut queue = test_queue(0);
        assert_eq!(queue.frame_delay(), 0);

        queue.set_frame_delay(3).unwrap();
        assert_eq!(queue.frame_delay(), 3);

        queue.set_frame_delay(0).unwrap();
        assert_eq!(queue.frame_delay(), 0);

        queue.set_frame_delay(MAX_FRAME_DELAY).unwrap();
        assert_eq!(queue.frame_delay(), MAX_FRAME_DELAY);
    }

    /// Test: After successful delay set, adding inputs works correctly
    #[test]
    fn test_frame_delay_with_inputs_after_set() {
        let mut queue = test_queue(0);
        queue.set_frame_delay(3).expect("valid delay");

        // Add input at frame 0
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 42 });
        let result = queue.add_input(input);

        // Should be stored at frame 0 + 3 = 3
        assert_eq!(result, Frame::new(3));
        assert_eq!(queue.last_added_frame, Frame::new(3));
        queue.check_invariants().unwrap();
    }

    /// Test: Drive the queue for many frames at delay=0, increase the delay
    /// mid-session, drive many more frames, and verify both the structural
    /// invariants and the integrity of the gap-fill replication.
    #[test]
    fn test_set_frame_delay_increase_after_many_frames() {
        let mut queue = test_queue(0);

        // Phase 1: 50 frames at delay=0.
        for i in 0..50i32 {
            let added = queue.add_input(PlayerInput::new(
                Frame::new(i),
                TestInput {
                    inp: (i & 0xff) as u8,
                },
            ));
            assert_eq!(added, Frame::new(i));
        }
        assert_eq!(queue.last_added_frame, Frame::new(49));
        queue
            .check_invariants()
            .expect("invariants hold after phase 1");

        // Mid-session increase: delay 0 -> 3. Replicates the last input
        // (frame 49, inp=49) across frames 50, 51, 52.
        queue
            .set_frame_delay(3)
            .expect("mid-session increase should succeed");
        assert_eq!(queue.last_added_frame, Frame::new(52));
        queue
            .check_invariants()
            .expect("invariants hold after delay increase");

        // Replicated frames carry the last pre-change input value.
        for f in 50..=52i32 {
            let inp = queue
                .confirmed_input(Frame::new(f))
                .expect("replicated frame is confirmed");
            assert_eq!(
                inp.input.inp, 49,
                "replicated frame {f} should hold the last pre-change input value"
            );
        }

        // Phase 2: 30 more frames at delay=3. Each user input lands at
        // current_frame + 3 in queue space.
        for j in 0..30i32 {
            let user_frame = Frame::new(50 + j); // user-side frame
            let expected_queue_frame = Frame::new(50 + j + 3);
            let added = queue.add_input(PlayerInput::new(
                user_frame,
                TestInput {
                    inp: (100 + j) as u8,
                },
            ));
            assert_eq!(
                added, expected_queue_frame,
                "user frame {user_frame:?} with delay 3 should land at {expected_queue_frame:?}"
            );
        }
        // After 30 user frames added at delay 3, last_added_frame = 49 + 3 + 30 = 82.
        assert_eq!(queue.last_added_frame, Frame::new(82));
        queue
            .check_invariants()
            .expect("invariants hold after phase 2");

        // Spot-check: queue holds the post-change inputs at the right frames.
        for j in 0..30i32 {
            let queue_frame = Frame::new(53 + j);
            let inp = queue
                .confirmed_input(queue_frame)
                .expect("post-change frame is confirmed");
            assert_eq!(inp.input.inp, (100 + j) as u8);
        }
    }

    /// Test: After rejected delay, queue state remains unchanged
    #[test]
    fn test_rejected_frame_delay_preserves_state() {
        let mut queue = test_queue(0);

        // Set a valid delay first
        queue.set_frame_delay(5).expect("valid delay");
        assert_eq!(queue.frame_delay, 5);

        // Try to set an invalid delay
        let result = queue.set_frame_delay(INPUT_QUEUE_LENGTH);
        assert!(result.is_err());

        // Original delay should be preserved
        assert_eq!(queue.frame_delay, 5);
    }

    /// Test: Ensure the constant relationship is correct
    #[test]
    fn test_max_frame_delay_constant_is_correct() {
        assert_eq!(
            MAX_FRAME_DELAY,
            INPUT_QUEUE_LENGTH - 1,
            "MAX_FRAME_DELAY should be INPUT_QUEUE_LENGTH - 1"
        );
    }

    // ==========================================
    // Queue Length Configuration Tests
    // ==========================================

    #[test]
    fn test_with_queue_length_minimum() {
        // Queue length of 2 is the minimum
        let queue = InputQueue::<TestConfig>::with_queue_length(0, 2);
        assert!(queue.is_some());
        let queue = queue.unwrap();
        assert_eq!(queue.queue_length(), 2);
    }

    #[test]
    fn test_with_queue_length_below_minimum_fails() {
        // Queue length of 1 should fail
        let queue = InputQueue::<TestConfig>::with_queue_length(0, 1);
        assert!(queue.is_none());

        // Queue length of 0 should fail
        let queue = InputQueue::<TestConfig>::with_queue_length(0, 0);
        assert!(queue.is_none());
    }

    #[test]
    fn try_with_queue_length_below_minimum_returns_structured_error() {
        // The fallible constructor returns the structured error directly (and
        // does NOT log a violation itself — only its infallible wrapper does, so
        // an invalid queue length produces a single violation, not duplicates).
        let err = InputQueue::<TestConfig>::try_with_queue_length(0, 1).unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::QueueLengthTooSmall { length: 1 }
            }
        ));
    }

    #[test]
    fn try_with_queue_length_reports_allocation_failure_for_impossible_length() {
        let err = InputQueue::<TestConfig>::try_with_queue_length(0, usize::MAX).unwrap_err();

        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::AllocationFailed {
                    context: "input_queue.inputs",
                    requested_elements: u64::MAX,
                }
            }
        ));
    }

    #[test]
    fn test_with_queue_length_custom() {
        let queue = InputQueue::<TestConfig>::with_queue_length(0, 64);
        assert!(queue.is_some());
        let queue = queue.unwrap();
        assert_eq!(queue.queue_length(), 64);
        assert_eq!(queue.max_frame_delay(), 63);
    }

    #[test]
    fn test_max_frame_delay_depends_on_queue_length() {
        let queue = InputQueue::<TestConfig>::with_queue_length(0, 16).unwrap();
        assert_eq!(queue.max_frame_delay(), 15);

        let queue = InputQueue::<TestConfig>::with_queue_length(0, 256).unwrap();
        assert_eq!(queue.max_frame_delay(), 255);
    }

    // ==========================================
    // Edge Cases in input() Method
    // ==========================================

    #[test]
    fn test_input_returns_none_when_prediction_error_exists() {
        let mut queue = test_queue(0);

        // Add inputs and trigger a prediction error
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
            queue.add_input(input);
        }

        // Request frame 3 (triggers prediction of 0)
        let _ = queue.input(Frame::new(3)).expect("prediction");

        // Add actual input with different value to trigger mismatch
        let actual = PlayerInput::new(Frame::new(3), TestInput { inp: 99 });
        queue.add_input(actual);

        // Now first_incorrect_frame should be set
        assert_eq!(queue.first_incorrect_frame(), Frame::new(3));

        // Calling input() when prediction error exists should return None
        let result = queue.input(Frame::new(4));
        assert!(result.is_none());
    }

    #[test]
    fn test_input_returns_none_when_frame_before_tail() {
        let mut queue = test_queue(0);

        // Add inputs for frames 0-9
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Discard frames 0-4 (keep 5-9)
        queue.discard_confirmed_frames(Frame::new(5));

        // Try to get frame 3 which was discarded - should return None
        let result = queue.input(Frame::new(3));
        assert!(result.is_none());
    }

    // ==========================================
    // Multiple Prediction Continuations
    // ==========================================

    #[test]
    fn test_consecutive_predictions_advance_frame() {
        let mut queue = test_queue(0);

        // Add initial inputs
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 10 });
            queue.add_input(input);
        }

        // Request a predicted frame ABOVE the first missing frame (3). The
        // episode must enter at the first missing frame, not the requested one,
        // so the sequential arrival for frame 3 is compared.
        let (_pred1, status1) = queue.input(Frame::new(5)).expect("prediction 1");
        assert_eq!(status1, InputStatus::Predicted);
        assert_eq!(queue.prediction.frame, Frame::new(3));

        // Now add the actual input for frame 3 (correct prediction). It must be
        // ACCEPTED (the old requested-frame entry semantics made this arrival
        // miss `prediction.frame` and bail with Frame::NULL).
        let input3 = PlayerInput::new(Frame::new(3), TestInput { inp: 10 }); // Same as prediction
        assert_eq!(queue.add_input(input3), Frame::new(3));

        // Prediction advances to exactly the next missing frame; the episode
        // stays live because last_requested_frame (5) is not reached yet.
        assert_eq!(queue.prediction.frame, Frame::new(4));
        assert_eq!(queue.first_incorrect_frame(), Frame::NULL);
    }

    // ==========================================
    // Prediction-episode entry frame (finding F17)
    // ==========================================

    /// A prediction episode RE-ENTERED after a rollback (`reset_prediction`)
    /// must start at the queue's first missing frame, not at the requested
    /// frame. This is the rollback re-simulation shape: the re-sim's first
    /// `input()` request can be ABOVE this queue's missing window when another
    /// queue's misprediction triggered the rollback (ordinary N>=3
    /// cross-endpoint jitter).
    #[test]
    fn prediction_reentry_after_reset_enters_at_first_missing_frame_not_requested() {
        let mut queue = test_queue(0);
        for i in 0..=4i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 10 });
            assert_eq!(queue.add_input(input), Frame::new(i));
        }

        // First episode: frames 5..=7 are served as predictions.
        for f in 5..=7i32 {
            let (_, status) = queue.input(Frame::new(f)).expect("prediction");
            assert_eq!(status, InputStatus::Predicted);
        }
        assert_eq!(queue.prediction.frame, Frame::new(5));

        // A rollback elsewhere resets prediction, then the re-simulation's
        // first request for this queue lands at frame 7 — above the queue's
        // first missing frame 5.
        queue.reset_prediction();
        let (_, status) = queue.input(Frame::new(7)).expect("re-entry prediction");
        assert_eq!(status, InputStatus::Predicted);
        assert_eq!(
            queue.prediction.frame,
            Frame::new(5),
            "episode re-entry must land at the first missing frame (5), not the requested frame (7)"
        );
    }

    /// An arrival INSIDE a re-entered episode's missing window must be
    /// compared against the episode's frozen value: a mismatch sets
    /// `first_incorrect_frame` to exactly that frame. Under the old
    /// requested-frame entry semantics this arrival (below the episode frame)
    /// was added but its comparison silently skipped — no rollback ever
    /// re-simulated the window (finding F17).
    #[test]
    fn arrival_below_reentered_episode_request_mismatch_sets_first_incorrect() {
        let mut queue = test_queue(0);
        for i in 0..=4i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 10 });
            assert_eq!(queue.add_input(input), Frame::new(i));
        }
        let _ = queue.input(Frame::new(6)).expect("first episode");
        queue.reset_prediction();
        // Re-enter at requested frame 8; first missing frame is 5.
        let _ = queue.input(Frame::new(8)).expect("re-entry prediction");

        // The real input for frame 5 arrives and DIFFERS from the frozen
        // prediction (10): the comparison must fire.
        let input5 = PlayerInput::new(Frame::new(5), TestInput { inp: 99 });
        assert_eq!(queue.add_input(input5), Frame::new(5));
        assert_eq!(
            queue.first_incorrect_frame(),
            Frame::new(5),
            "mismatching in-window arrival must set first_incorrect_frame"
        );
    }

    /// The matching counterpart: an in-window arrival that EQUALS the frozen
    /// prediction is accepted, leaves `first_incorrect_frame` unset, and
    /// advances the episode to the next missing frame.
    #[test]
    fn arrival_below_reentered_episode_request_match_does_not_set_first_incorrect() {
        let mut queue = test_queue(0);
        for i in 0..=4i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 10 });
            assert_eq!(queue.add_input(input), Frame::new(i));
        }
        let _ = queue.input(Frame::new(6)).expect("first episode");
        queue.reset_prediction();
        let _ = queue.input(Frame::new(8)).expect("re-entry prediction");

        // The real input for frame 5 matches the frozen prediction.
        let input5 = PlayerInput::new(Frame::new(5), TestInput { inp: 10 });
        assert_eq!(queue.add_input(input5), Frame::new(5));
        assert_eq!(queue.first_incorrect_frame(), Frame::NULL);
        // Episode advances per-arrival and stays live until the requested
        // frame (8) is confirmed.
        assert_eq!(queue.prediction.frame, Frame::new(6));
    }

    /// A queue that has never accepted an input enters its prediction episode
    /// at frame 0 — the frame `advance_queue_head` gap-fills a virgin queue
    /// from — even when the first request arrives at a later frame (a session
    /// that advanced several frames before this peer's first input landed).
    /// Every gap-filled and real arrival is then compared.
    #[test]
    fn prediction_entry_on_virgin_queue_starts_at_frame_zero() {
        let mut queue = test_queue(0);

        // First-ever request lands at frame 3 (the session is 3 frames in).
        let (value, status) = queue.input(Frame::new(3)).expect("virgin prediction");
        assert_eq!(status, InputStatus::Predicted);
        assert_eq!(value, TestInput::default());
        assert_eq!(
            queue.prediction.frame,
            Frame::new(0),
            "virgin-queue episode must enter at frame 0, the first physical add"
        );

        // The peer's first real input arrives stamped at frame 2 (e.g. the
        // sender used input delay 2): `advance_queue_head` gap-fills frames 0
        // and 1 with the blank input, which matches the blank prediction, and
        // the real frame-2 input differs — the mismatch must be detected.
        let real = PlayerInput::new(Frame::new(2), TestInput { inp: 7 });
        assert_eq!(queue.add_input(real), Frame::new(2));
        assert_eq!(
            queue.first_incorrect_frame(),
            Frame::new(2),
            "the first differing arrival on a virgin queue must be detected"
        );
    }

    /// The value-coincidence hazard is closed by construction: a mismatch in
    /// the middle of the missing window is detected even when the LAST arrival
    /// of the window equals the frozen prediction. Under the old
    /// requested-frame entry semantics only the arrival at the requested frame
    /// was compared; here that arrival (frame 6, value 10) coincides with the
    /// frozen value, so the frame-4 divergence (42, already applied as 10)
    /// would have been permanently swallowed — no rollback, no event.
    #[test]
    fn swallowed_window_mismatch_with_coinciding_boundary_value_sets_first_incorrect() {
        let mut queue = test_queue(0);
        for i in 0..=2i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 10 });
            assert_eq!(queue.add_input(input), Frame::new(i));
        }

        // Episode 1 applies the frozen value (10) for frames 3..=6.
        for f in 3..=6i32 {
            let (value, status) = queue.input(Frame::new(f)).expect("prediction");
            assert_eq!(status, InputStatus::Predicted);
            assert_eq!(value.inp, 10);
        }

        // Rollback elsewhere; the re-simulation re-enters at requested frame 6.
        queue.reset_prediction();
        let _ = queue.input(Frame::new(6)).expect("re-entry prediction");
        assert_eq!(queue.prediction.frame, Frame::new(3));

        // Arrivals: 3 matches, 4 DIFFERS (the simulated trajectory applied 10
        // for frame 4), 5 and 6 match the frozen value again.
        for (frame, value) in [(3i32, 10u8), (4, 42), (5, 10), (6, 10)] {
            let input = PlayerInput::new(Frame::new(frame), TestInput { inp: value });
            assert_eq!(queue.add_input(input), Frame::new(frame));
        }
        assert_eq!(
            queue.first_incorrect_frame(),
            Frame::new(4),
            "the in-window divergence must be detected even though the \
             requested-frame arrival coincides with the frozen value"
        );
    }

    // ==========================================
    // confirmed_input Edge Cases
    // ==========================================

    #[test]
    fn test_confirmed_input_at_tail() {
        let mut queue = test_queue(0);

        // Add frames 0-9
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Discard frames before 5
        queue.discard_confirmed_frames(Frame::new(5));

        // Frame 5 (now at tail) should be retrievable
        let confirmed = queue.confirmed_input(Frame::new(5)).unwrap();
        assert_eq!(confirmed.input.inp, 5);
    }

    #[test]
    fn test_confirmed_input_at_head() {
        let mut queue = test_queue(0);

        // Add frames 0-9
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Frame 9 (most recent, at head-1) should be retrievable
        let confirmed = queue.confirmed_input(Frame::new(9)).unwrap();
        assert_eq!(confirmed.input.inp, 9);
    }

    // ==========================================
    // Debug Trait Tests
    // ==========================================

    #[test]
    fn test_input_queue_debug() {
        let queue = test_queue(0);
        let debug_str = format!("{:?}", queue);
        assert!(debug_str.contains("InputQueue"));
        assert!(debug_str.contains("head"));
        assert!(debug_str.contains("tail"));
    }

    // ==========================================
    // Clone and Copy Trait Tests
    // ==========================================

    #[test]
    fn test_input_queue_clone() {
        let mut original = test_queue(0);

        // Add some data
        for i in 0..5i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            original.add_input(input);
        }

        let cloned = original.clone();

        // Verify clone has same state
        assert_eq!(cloned.length, original.length);
        assert_eq!(cloned.head, original.head);
        assert_eq!(cloned.tail, original.tail);
        assert_eq!(cloned.last_added_frame, original.last_added_frame);
    }

    // ==========================================
    // Constant Value Tests
    // ==========================================

    #[test]
    fn test_input_queue_length_constant() {
        // Verify INPUT_QUEUE_LENGTH is the expected value (128 in production)
        assert_eq!(INPUT_QUEUE_LENGTH, 128);
    }

    #[test]
    fn test_input_queue_length_is_power_of_two() {
        // Power of two is beneficial for modular arithmetic
        assert!(INPUT_QUEUE_LENGTH.is_power_of_two());
    }

    // ==========================================
    // Freeze (graceful peer drop) Tests
    // ==========================================

    #[test]
    fn test_freeze_no_op_on_add_input() {
        // In tests: unwrap is allowed
        let mut queue = test_queue(0);
        // Add a few inputs to give the queue a known state.
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            assert_eq!(queue.add_input(input), Frame::new(i));
        }

        // Snapshot the queue state before freezing.
        let last_added_before = queue.last_added_frame;
        let length_before = queue.length;
        let head_before = queue.head;
        let tail_before = queue.tail;

        // Freeze the queue and assert the flag observably changed.
        assert!(!queue.is_frozen());
        queue.freeze();
        assert!(queue.is_frozen());

        // Attempting to add an input now must be a no-op and return the
        // existing last_added_frame (not Frame::NULL — that would signal a
        // drop, which callers may handle differently from "frozen").
        let frame_4_input = PlayerInput::new(Frame::new(3), TestInput { inp: 99 });
        let returned = queue.add_input(frame_4_input);
        assert_eq!(
            returned, last_added_before,
            "frozen add_input must return last_added_frame, not Frame::NULL"
        );

        // Queue state must be entirely unchanged.
        assert_eq!(queue.last_added_frame, last_added_before);
        assert_eq!(queue.length, length_before);
        assert_eq!(queue.head, head_before);
        assert_eq!(queue.tail, tail_before);

        // Repeated calls remain no-ops.
        assert_eq!(queue.add_input(frame_4_input), last_added_before);
        assert_eq!(queue.length, length_before);
    }

    #[test]
    fn test_freeze_no_op_on_set_frame_delay() {
        let mut queue = test_queue(0);
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            assert_eq!(queue.add_input(input), Frame::new(i));
        }

        queue.freeze();
        let before = queue.clone();

        queue
            .set_frame_delay(2)
            .expect("valid frame-delay request on frozen queue should be a no-op");

        assert_queue_unchanged(&queue, &before);
    }

    #[test]
    fn test_freeze_no_op_on_set_frame_delay_with_oversized_delay() {
        // Regression: even an out-of-range `delay` must be a silent no-op on a
        // frozen queue. The frozen guard must precede the `FrameDelayTooLarge`
        // validation; otherwise we surface an error for a peer that is gone.
        let mut queue = test_queue(0);
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            assert_eq!(queue.add_input(input), Frame::new(i));
        }

        queue.freeze();
        let before = queue.clone();
        let oversized = queue.max_frame_delay() + 1;

        queue
            .set_frame_delay(oversized)
            .expect("oversized frame-delay on a frozen queue must be a silent no-op, not an error");

        assert_queue_unchanged(&queue, &before);
    }

    #[test]
    fn test_freeze_prediction_returns_last_confirmed_with_disconnected_status() {
        // In tests: unwrap is allowed.
        // Queue-level note: the input() method returns InputStatus::Predicted
        // when serving a frame past the queue. The Disconnected status is
        // reported at the SyncLayer level using `connect_status.disconnected`
        // (see SyncLayer::synchronized_inputs), not by the queue itself.
        // This test verifies the queue keeps producing the last confirmed
        // value even after freezing, so the SyncLayer can deterministically
        // hand it back to the game with Disconnected status.
        let mut queue = test_queue(0);
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: (i + 1) as u8 });
            queue.add_input(input);
        }
        // last_confirmed_input is the input added at frame 2 (value 3).
        queue.freeze();

        // Request a frame past the queue's last_added_frame. The queue is
        // now in a state where the frame is not in the buffer, so it returns
        // a prediction based on last_confirmed_input.
        let (returned_input, status) = queue
            .input(Frame::new(10))
            .expect("frozen queue should still serve predictions");

        // Last confirmed input had inp = 3.
        assert_eq!(
            returned_input.inp, 3,
            "frozen queue must keep returning the last confirmed input"
        );
        // Status at the queue level is Predicted; SyncLayer reinterprets
        // this as Disconnected when connect_status[i].disconnected = true.
        assert_eq!(status, InputStatus::Predicted);
    }

    // ----------------------------------------------------------------------
    // Frozen-value rollback (graceful peer drop, under-loss convergence)
    // ----------------------------------------------------------------------

    /// Adds sequential confirmed inputs `0..=last` with payload `inp == frame`.
    fn fill_sequential(queue: &mut InputQueue<TestConfig>, last: i32) {
        for f in 0..=last {
            let added = queue.add_input(PlayerInput::new(
                Frame::new(f),
                TestInput {
                    inp: u8::try_from(f).expect("frame fits in u8 for test"),
                },
            ));
            assert_eq!(added, Frame::new(f), "input {f} should be accepted");
        }
    }

    #[test]
    fn freeze_at_rolls_last_confirmed_input_back_to_earlier_frame() {
        let mut queue = test_queue(0);
        fill_sequential(&mut queue, 5);
        // Most-recently confirmed value is frame 5's payload.
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 5 }));

        // Freeze at an EARLIER frame: last_confirmed_input must roll back to it.
        queue.freeze_at(Frame::new(2));
        assert!(queue.is_frozen());
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 2 }));
    }

    #[test]
    fn freeze_at_with_null_frame_leaves_value_unchanged() {
        let mut queue = test_queue(0);
        fill_sequential(&mut queue, 5);
        let before_value = queue.last_confirmed_input;

        queue.freeze_at(Frame::NULL);
        assert!(queue.is_frozen());
        // NULL freeze frame: value left exactly as the most-recent confirmed.
        assert_eq!(queue.last_confirmed_input, before_value);
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 5 }));
    }

    /// Test-only capture of `report_violation!` output.
    ///
    /// `freeze_at` (and its siblings) report violations through the global
    /// `report_violation!` macro, which routes to `TracingObserver` — a
    /// `tracing::warn!` / `error!` event, not a per-session observer. To assert
    /// *whether* a violation fired during a focused call, these helpers install a
    /// thread-local capturing subscriber for the duration of `f`. Thread-local
    /// (`tracing::subscriber::with_default`), so tests running in parallel never
    /// observe each other's events.
    mod violation_capture {
        use std::sync::{Arc, Mutex};

        use tracing::field::{Field, Visit};
        use tracing::{Event, Level, Subscriber};
        use tracing_subscriber::layer::{Context, Layer};
        use tracing_subscriber::prelude::*;

        /// A single captured tracing event: its severity level and message text.
        #[derive(Clone, Debug)]
        pub(super) struct Captured {
            pub level: Level,
            pub message: String,
        }

        /// Captures the special `message` field (the formatted violation text);
        /// all other fields default through `record_debug` and are ignored.
        #[derive(Default)]
        struct MessageVisitor {
            message: String,
        }

        impl Visit for MessageVisitor {
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.message = format!("{value:?}");
                }
            }
        }

        struct CaptureLayer {
            events: Arc<Mutex<Vec<Captured>>>,
        }

        impl<S: Subscriber> Layer<S> for CaptureLayer {
            fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
                let mut visitor = MessageVisitor::default();
                event.record(&mut visitor);
                if let Ok(mut events) = self.events.lock() {
                    events.push(Captured {
                        level: *event.metadata().level(),
                        message: visitor.message,
                    });
                }
            }
        }

        /// Runs `f` with a thread-local capturing subscriber and returns its
        /// result alongside every tracing event emitted during the call.
        pub(super) fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<Captured>) {
            let events = Arc::new(Mutex::new(Vec::new()));
            let subscriber = tracing_subscriber::registry().with(CaptureLayer {
                events: Arc::clone(&events),
            });
            let result = tracing::subscriber::with_default(subscriber, f);
            let captured = events.lock().expect("capture mutex poisoned").clone();
            (result, captured)
        }

        /// The captured events that `report_violation!` would emit — WARN
        /// (`Warning`) or ERROR (`Error`/`Critical`). INFO/DEBUG/TRACE noise is
        /// filtered out so the assertion is about violations specifically.
        pub(super) fn violations(events: &[Captured]) -> Vec<&Captured> {
            events
                .iter()
                .filter(|e| e.level == Level::WARN || e.level == Level::ERROR)
                .collect()
        }
    }

    #[test]
    fn freeze_at_with_null_frame_emits_no_violation() {
        // `Frame::NULL` is the expected "no agreed freeze frame yet" signal
        // (a reserved hot-join slot frozen from frame 0 with no confirmed inputs;
        // a peer dropped before any confirmed input). It must NOT report a
        // violation — mirroring the already-correct `set_frozen_value_at` NULL
        // no-op. Regression test for the spurious-Warning defect: RED before the
        // fix, which reported a `Warning` for *every* NULL freeze.
        let ((), events) = violation_capture::capture(|| {
            let mut queue = test_queue(0);
            fill_sequential(&mut queue, 5);
            queue.freeze_at(Frame::NULL);
            assert!(queue.is_frozen());
        });
        let violations = violation_capture::violations(&events);
        assert!(
            violations.is_empty(),
            "freeze_at(NULL) must not report a violation, got: {violations:?}"
        );
    }

    #[test]
    fn freeze_at_with_missing_nonnull_frame_still_warns() {
        // A *non-NULL* freeze frame with no confirmed input in the ring (evicted,
        // or never received) is the genuinely-unexpected case and MUST still
        // surface a `Warning`. Guards the fix against over-correction: it
        // silences only the expected NULL case, never a real miss.
        let ((), events) = violation_capture::capture(|| {
            let mut queue = test_queue(0);
            fill_sequential(&mut queue, 5);
            // Frame 100 was never added: `roll_confirmed_input_to` fails.
            queue.freeze_at(Frame::new(100));
            assert!(queue.is_frozen());
        });
        let violations = violation_capture::violations(&events);
        assert_eq!(
            violations.len(),
            1,
            "freeze_at(non-NULL missing frame) must report exactly one violation, got: {violations:?}"
        );
        assert_eq!(violations[0].level, tracing::Level::WARN);
        assert!(
            violations[0]
                .message
                .contains("no confirmed input at agreed freeze frame"),
            "unexpected violation message: {:?}",
            violations[0].message
        );
    }

    #[test]
    fn freeze_at_with_present_frame_emits_no_violation() {
        // The happy path (a non-NULL frame WITH a confirmed input) rolls the
        // value and must not warn.
        let ((), events) = violation_capture::capture(|| {
            let mut queue = test_queue(0);
            fill_sequential(&mut queue, 5);
            queue.freeze_at(Frame::new(2));
            assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 2 }));
        });
        let violations = violation_capture::violations(&events);
        assert!(
            violations.is_empty(),
            "freeze_at(present frame) must not report a violation, got: {violations:?}"
        );
    }

    #[test]
    fn set_frozen_value_at_lowers_already_frozen_value_to_earlier_frame() {
        // The key NEW behavior: a survivor that initially froze "high" must be
        // corrected DOWN to the global-min agreed frame `F` once it converges.
        let mut queue = test_queue(0);
        fill_sequential(&mut queue, 5);

        // Initial freeze at frame 4 (the survivor's own higher received frame).
        queue.freeze_at(Frame::new(4));
        assert!(queue.is_frozen());
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 4 }));

        // The disconnect machinery converges F DOWN to frame 2 and re-rolls.
        queue.set_frozen_value_at(Frame::new(2));
        assert!(queue.is_frozen());
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 2 }));
    }

    #[test]
    fn set_frozen_value_at_with_null_frame_leaves_value_unchanged() {
        let mut queue = test_queue(0);
        fill_sequential(&mut queue, 5);
        queue.freeze_at(Frame::new(4));
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 4 }));

        queue.set_frozen_value_at(Frame::NULL);
        // NULL agreed frame: no-op.
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 4 }));
    }

    #[test]
    fn set_frozen_value_at_on_unfrozen_queue_is_noop() {
        let mut queue = test_queue(0);
        fill_sequential(&mut queue, 5);
        // NOT frozen.
        assert!(!queue.is_frozen());
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 5 }));

        queue.set_frozen_value_at(Frame::new(2));
        // Must not mutate a live queue, and must not freeze it.
        assert!(!queue.is_frozen());
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 5 }));
    }

    #[test]
    fn set_frozen_value_at_with_missing_frame_leaves_value_unchanged() {
        // Evicted/missing-frame fail-safe: requesting a frame that has no
        // confirmed input (here, a frame far beyond what was ever added) must
        // leave the frozen value untouched rather than clobbering it.
        let mut queue = test_queue(0);
        fill_sequential(&mut queue, 5);
        queue.freeze_at(Frame::new(4));
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 4 }));

        // Frame 100 was never added: confirmed_input(100) errors -> no-op.
        queue.set_frozen_value_at(Frame::new(100));
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 4 }));
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod property_tests {
    use super::*;
    use crate::test_config::miri_case_count;
    use proptest::prelude::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
    struct TestInput {
        inp: u8,
    }

    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = Vec<u8>;
        type Address = SocketAddr;
    }

    fn test_queue(player_index: usize) -> InputQueue<TestConfig> {
        InputQueue::<TestConfig>::new(player_index)
            .expect("test_queue: InputQueue::new should succeed for valid player_index")
    }

    // Strategy for generating input values
    fn input_value() -> impl Strategy<Value = u8> {
        any::<u8>()
    }

    // Strategy for generating number of frames to add (1-100)
    fn frame_count() -> impl Strategy<Value = usize> {
        1usize..=100
    }

    // Strategy for generating frame delays (0-7)
    fn frame_delay() -> impl Strategy<Value = usize> {
        0usize..=7
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]
        /// Property: Sequential inputs are always stored correctly
        #[test]
        fn prop_sequential_inputs_stored(
            count in frame_count(),
            seed in any::<u64>(),
        ) {
            let mut queue = test_queue(0);
            let mut rng = seed;

            // Add sequential inputs
            for i in 0..count as i32 {
                // Simple PRNG for deterministic "random" values
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let input_val = (rng >> 56) as u8;

                let input = PlayerInput::new(Frame::new(i), TestInput { inp: input_val });
                let result = queue.add_input(input);
                prop_assert_eq!(result, Frame::new(i));
            }

            prop_assert_eq!(queue.length, count);
            prop_assert_eq!(queue.last_added_frame, Frame::new(count as i32 - 1));
        }

        /// Property: Inputs can be retrieved after being added
        #[test]
        fn prop_input_retrieval(
            count in 1usize..=50,
        ) {
            let mut queue = test_queue(0);

            // Add inputs with frame number as value
            for i in 0..count as i32 {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
                queue.add_input(input);
            }

            // Verify all inputs can be retrieved
            for i in 0..count as i32 {
                let confirmed = queue.confirmed_input(Frame::new(i)).unwrap();
                prop_assert_eq!(confirmed.input.inp, i as u8);
            }
        }

        /// Property: Discard preserves inputs after discard frame
        #[test]
        fn prop_discard_preserves_later_frames(
            total in 10usize..=50,
            discard_up_to in 0usize..=9,
        ) {
            let mut queue = test_queue(0);

            // Add inputs
            for i in 0..total as i32 {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
                queue.add_input(input);
            }

            // Discard frames
            queue.discard_confirmed_frames(Frame::new(discard_up_to as i32));

            // All frames after discard_up_to should still be retrievable
            for i in (discard_up_to as i32 + 1)..(total as i32) {
                let result = queue.confirmed_input(Frame::new(i));
                prop_assert!(result.is_ok(), "Frame {} should be available", i);
            }
        }

        /// Property: Frame delay consistently shifts all inputs
        #[test]
        fn prop_frame_delay_shifts_inputs(
            delay in frame_delay(),
            count in 1usize..=30,
        ) {
            let mut queue = test_queue(0);
            queue.set_frame_delay(delay).expect("valid delay");

            // Add inputs
            for i in 0..count as i32 {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
                queue.add_input(input);
            }

            // last_added_frame should be shifted by delay
            prop_assert_eq!(queue.last_added_frame, Frame::new((count as i32 - 1) + delay as i32));
        }

        /// Property: Prediction uses last confirmed input (RepeatLastConfirmed)
        /// This ensures determinism across peers because last_confirmed_input is synchronized.
        #[test]
        fn prop_prediction_uses_last_confirmed_input(
            count in 1usize..=20,
            last_value in input_value(),
        ) {
            let mut queue = test_queue(0);

            // Add inputs, with last one having specific value
            for i in 0..(count - 1) as i32 {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
                queue.add_input(input);
            }
            // Add last input with known value - this becomes the last_confirmed_input
            let last_input = PlayerInput::new(
                Frame::new((count - 1) as i32),
                TestInput { inp: last_value },
            );
            queue.add_input(last_input);

            // Request frame beyond what we have
            let future_frame = Frame::new(count as i32 + 5);
            let (predicted, status) = queue.input(future_frame).expect("input");

            prop_assert_eq!(status, InputStatus::Predicted);
            // Prediction uses RepeatLastConfirmed, which returns last_confirmed_input
            prop_assert_eq!(predicted.inp, last_value);
        }

        /// Property: Queue length is bounded when regularly discarding old frames
        /// Note: The InputQueue asserts if length exceeds INPUT_QUEUE_LENGTH.
        /// In practice, discard_confirmed_frames() must be called regularly.
        #[test]
        fn prop_queue_length_bounded_with_discard(
            count in 1usize..=200,
        ) {
            let mut queue = test_queue(0);

            // Add inputs with periodic discard (simulating real usage)
            for i in 0..count as i32 {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
                queue.add_input(input);

                // Discard old frames to prevent overflow (every 64 frames)
                if i > 64 && i % 32 == 0 {
                    queue.discard_confirmed_frames(Frame::new(i - 32));
                }
            }

            // Length should never exceed INPUT_QUEUE_LENGTH
            prop_assert!(queue.length <= INPUT_QUEUE_LENGTH);
        }

        /// Property: Duplicate inputs are rejected
        #[test]
        fn prop_duplicate_inputs_rejected(
            frame in 0i32..100,
            value in input_value(),
        ) {
            let mut queue = test_queue(0);

            // Add inputs up to and including target frame
            for i in 0..=frame {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: value });
                queue.add_input(input);
            }

            let length_before = queue.length;

            // Try to add duplicate
            let duplicate = PlayerInput::new(Frame::new(frame), TestInput { inp: value });
            let result = queue.add_input(duplicate);

            prop_assert_eq!(result, Frame::NULL);
            prop_assert_eq!(queue.length, length_before);
        }

        /// Property: Non-sequential inputs are rejected
        #[test]
        fn prop_non_sequential_inputs_rejected(
            base_frame in 0i32..50,
            skip in 2i32..10,
        ) {
            let mut queue = test_queue(0);

            // Add inputs sequentially up to base_frame
            for i in 0..=base_frame {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
                queue.add_input(input);
            }

            // Try to add non-sequential input (skipping frames)
            let skipped_frame = base_frame + skip;
            let input = PlayerInput::new(Frame::new(skipped_frame), TestInput { inp: 0 });
            let result = queue.add_input(input);

            prop_assert_eq!(result, Frame::NULL);
        }

        /// Property: First incorrect frame is detected when prediction differs
        #[test]
        fn prop_incorrect_frame_detection(
            count in 2usize..=20,
        ) {
            let mut queue = test_queue(0);

            // Add initial inputs with value 0
            for i in 0..(count - 1) as i32 {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
                queue.add_input(input);
            }

            // Request the next frame (triggers prediction of 0)
            let predicted_frame = Frame::new((count - 1) as i32);
            let (predicted, _) = queue.input(predicted_frame).expect("input");
            prop_assert_eq!(predicted.inp, 0); // Prediction based on last input

            // Add actual input with DIFFERENT value
            let actual = PlayerInput::new(predicted_frame, TestInput { inp: 99 });
            queue.add_input(actual);

            // Should detect incorrect prediction
            prop_assert_eq!(queue.first_incorrect_frame(), predicted_frame);
        }

        /// Property: Reset prediction clears prediction state
        #[test]
        fn prop_reset_clears_state(
            count in 1usize..=10,
        ) {
            let mut queue = test_queue(0);

            // Add some inputs and trigger prediction
            for i in 0..count as i32 {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
                queue.add_input(input);
            }
            let _ = queue.input(Frame::new(count as i32 + 5)).expect("input"); // Trigger prediction

            // Reset
            queue.reset_prediction();

            prop_assert_eq!(queue.prediction.frame, Frame::NULL);
            prop_assert_eq!(queue.first_incorrect_frame, Frame::NULL);
            prop_assert_eq!(queue.last_requested_frame, Frame::NULL);
        }
    }
}

// ###################
// # KANI PROOFS     #
// ###################

/// Kani proofs for InputQueue buffer bounds (INV-4, INV-5 from formal-spec.md).
///
/// These proofs verify:
/// - INV-4: Queue length is always bounded by INPUT_QUEUE_LENGTH
/// - INV-5: Queue indices (head, tail) are always valid (< INPUT_QUEUE_LENGTH)
/// - Circular buffer wraparound is correct
/// - Length calculation matches actual buffer usage
///
/// # Configurable Constants Alignment (Phase 9/10)
///
/// Production code now allows configurable queue lengths via [`InputQueueConfig`]:
/// - `InputQueueConfig::standard()` - 128 frames (default)
/// - `InputQueueConfig::high_latency()` - 256 frames
/// - `InputQueueConfig::minimal()` - 32 frames
///
/// Kani uses `INPUT_QUEUE_LENGTH = 7` (via `#[cfg(kani)]`) for tractable verification.
/// The invariants verified here are **size-independent** - they hold for any queue
/// length >= 2. The proofs verify:
/// - Circular buffer arithmetic correctness (wraparound at any boundary)
/// - Index bounds checking (always < queue_length)
/// - Length bounds (always <= queue_length)
///
/// Therefore, proofs passing for queue_length=7 imply correctness for 32, 128, 256.
///
/// Note: Requires Kani verifier. Install with:
///   cargo install --locked kani-verifier
///   cargo kani setup
///
/// Run proofs with:
///   cargo kani --tests
///
/// ## Unwind Bound Guidelines
///
/// Kani proofs require sufficient unwind bounds to verify loops. Key considerations:
///
/// 1. **Buffer initialization**: Creating `InputQueue` via `test_queue()` fills its
///    backing buffer (a `#[cfg(kani)]` heap-free [`ProofVec`](crate::proof_vec::ProofVec)) with
///    `INPUT_QUEUE_LENGTH` (7 under Kani) blank inputs via a `0..queue_length` push
///    loop, so the harness needs unwind >= 8 (7 fill iterations + 1 for the
///    unwinding-assertion check). The proofs here use 10 for margin.
///
/// 2. **Additional loops**: Add the maximum loop iterations to the base unwind:
///    - `for i in 0..N` requires +N iterations
///    - Nested or sequential loops multiply requirements
///
/// 3. **Symbolic ranges**: Large symbolic ranges (e.g., `kani::any()` with 0-100) create
///    exponential path explosion. Prefer concrete values or very small ranges (1-3).
///
/// 4. **Recommended base**: Start with `unwind(10)` for proofs using `test_queue()`,
///    then add loop iterations and test. If verification times out, simplify the proof
///    by using concrete values instead of symbolic ranges.
///
/// Example calculations:
/// - `test_queue()` only: unwind(10)
/// - `test_queue()` + `for i in 0..3`: unwind(10 + 3 + buffer) = unwind(15)
/// - `test_queue()` + `for i in 0..5`: unwind(10 + 5 + buffer) = unwind(17)
#[cfg(kani)]
mod kani_input_queue_proofs {
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
        type State = Vec<u8>;
        type Address = SocketAddr;
    }

    /// Helper to create a test queue for Kani proofs.
    fn test_queue(player_index: usize) -> InputQueue<TestConfig> {
        InputQueue::<TestConfig>::new(player_index)
            .expect("test_queue: InputQueue::new should succeed for valid player_index")
    }

    /// Proof: New queue has valid initial state.
    ///
    /// Verifies INV-4 (length = 0) and INV-5 (head = tail = 0) at initialization.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Initial queue state validity (INV-4, INV-5)
    /// - Related: proof_add_single_input_maintains_invariants
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_new_queue_valid() {
        let queue = test_queue(0);

        // INV-4: length bounded
        kani::assert(queue.length == 0, "New queue should have length 0");
        kani::assert(
            queue.length <= INPUT_QUEUE_LENGTH,
            "Length should be bounded by INPUT_QUEUE_LENGTH",
        );

        // INV-5: indices valid
        kani::assert(queue.head == 0, "New queue head should be 0");
        kani::assert(queue.tail == 0, "New queue tail should be 0");
        kani::assert(
            queue.head < INPUT_QUEUE_LENGTH,
            "Head should be within bounds",
        );
        kani::assert(
            queue.tail < INPUT_QUEUE_LENGTH,
            "Tail should be within bounds",
        );

        // Additional invariants
        kani::assert(queue.first_frame, "New queue should have first_frame flag");
        kani::assert(
            queue.last_added_frame.is_null(),
            "New queue should have null last_added_frame",
        );
    }

    /// Proof: Single add_input maintains invariants.
    ///
    /// Verifies that adding a single input maintains INV-4 and INV-5.
    ///
    /// Note: unwind(10) covers the construction loop that fills the queue's
    /// backing buffer with INPUT_QUEUE_LENGTH (7 under Kani) blank inputs.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Single add_input preserves invariants (INV-4, INV-5)
    /// - Related: proof_new_queue_valid, proof_sequential_inputs_maintain_invariants
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_add_single_input_maintains_invariants() {
        let mut queue = test_queue(0);

        let input_val: u8 = kani::any();
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: input_val });

        let result = queue.add_input(input);

        // Input should be accepted (frame 0 is first input)
        kani::assert(
            result == Frame::new(0),
            "First input should be accepted at frame 0",
        );

        // INV-4: length bounded
        kani::assert(queue.length == 1, "Length should be 1 after first input");
        kani::assert(
            queue.length <= INPUT_QUEUE_LENGTH,
            "Length should be bounded",
        );

        // INV-5: indices valid
        kani::assert(queue.head == 1, "Head should advance to 1");
        kani::assert(queue.tail == 0, "Tail should remain at 0");
        kani::assert(
            queue.head < INPUT_QUEUE_LENGTH,
            "Head should be within bounds",
        );
        kani::assert(
            queue.tail < INPUT_QUEUE_LENGTH,
            "Tail should be within bounds",
        );
    }

    /// Proof: Sequential inputs maintain invariants (concrete iteration for tractability).
    ///
    /// Verifies INV-4 and INV-5 hold after adding multiple sequential inputs.
    ///
    /// Note: This proof uses a concrete iteration count (2 inputs) to keep verification
    /// tractable. The invariants are verified at each step, proving they are maintained
    /// for sequential additions. Combined with proof_add_single_input_maintains_invariants,
    /// this provides coverage for the general case via induction.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Sequential add_input preserves invariants (INV-4, INV-5)
    /// - Related: proof_add_single_input_maintains_invariants
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_sequential_inputs_maintain_invariants() {
        let mut queue = test_queue(0);

        // Add first input
        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 0 });
        let result0 = queue.add_input(input0);
        kani::assert(
            result0 == Frame::new(0),
            "First input should be accepted at frame 0",
        );
        kani::assert(queue.length == 1, "Length should be 1 after first input");
        kani::assert(
            queue.length <= INPUT_QUEUE_LENGTH,
            "Length should be bounded",
        );
        kani::assert(
            queue.head < INPUT_QUEUE_LENGTH,
            "Head should be within bounds",
        );
        kani::assert(
            queue.tail < INPUT_QUEUE_LENGTH,
            "Tail should be within bounds",
        );

        // Add second input
        let input1 = PlayerInput::new(Frame::new(1), TestInput { inp: 1 });
        let result1 = queue.add_input(input1);
        kani::assert(
            result1 == Frame::new(1),
            "Second input should be accepted at frame 1",
        );
        kani::assert(queue.length == 2, "Length should be 2 after second input");
        kani::assert(
            queue.length <= INPUT_QUEUE_LENGTH,
            "Length should be bounded",
        );
        kani::assert(
            queue.head < INPUT_QUEUE_LENGTH,
            "Head should be within bounds",
        );
        kani::assert(
            queue.tail < INPUT_QUEUE_LENGTH,
            "Tail should be within bounds",
        );
    }

    /// Proof: Head wraparound is correct.
    ///
    /// Verifies that head index wraps around correctly when reaching INPUT_QUEUE_LENGTH.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Circular buffer head wraparound
    /// - Related: proof_queue_index_calculation, proof_length_calculation_consistent
    #[kani::proof]
    #[kani::unwind(2)]
    fn proof_head_wraparound() {
        let head: usize = kani::any();
        kani::assume(head < INPUT_QUEUE_LENGTH);

        let new_head = (head + 1) % INPUT_QUEUE_LENGTH;

        kani::assert(
            new_head < INPUT_QUEUE_LENGTH,
            "Wrapped head should be within bounds",
        );

        if head == INPUT_QUEUE_LENGTH - 1 {
            kani::assert(new_head == 0, "Head should wrap to 0");
        } else {
            kani::assert(new_head == head + 1, "Head should increment normally");
        }
    }

    /// Proof: Queue index calculation is always valid.
    ///
    /// Verifies that frame-to-index calculation (frame % INPUT_QUEUE_LENGTH) is always valid.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Frame-to-index modulo bounds (INV-5)
    /// - Related: proof_head_wraparound, proof_frame_modulo_for_queue
    #[kani::proof]
    #[kani::unwind(2)]
    fn proof_queue_index_calculation() {
        let frame: i32 = kani::any();
        kani::assume(frame >= 0 && frame <= 10_000_000);

        let index = frame as usize % INPUT_QUEUE_LENGTH;

        kani::assert(
            index < INPUT_QUEUE_LENGTH,
            "Calculated index should be within bounds",
        );
    }

    /// Proof: Length calculation is consistent with head/tail.
    ///
    /// Verifies the circular buffer length formula: length = (head - tail + N) % N
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Circular buffer length formula correctness
    /// - Related: proof_head_wraparound, proof_queue_index_calculation
    #[kani::proof]
    #[kani::unwind(2)]
    fn proof_length_calculation_consistent() {
        let head: usize = kani::any();
        let tail: usize = kani::any();
        let length: usize = kani::any();

        kani::assume(head < INPUT_QUEUE_LENGTH);
        kani::assume(tail < INPUT_QUEUE_LENGTH);
        kani::assume(length <= INPUT_QUEUE_LENGTH);

        // For a valid queue state, length should match circular distance
        let calculated_length = if head >= tail {
            head - tail
        } else {
            INPUT_QUEUE_LENGTH - tail + head
        };

        // Verify the circular buffer property
        kani::assert(
            calculated_length <= INPUT_QUEUE_LENGTH,
            "Calculated length should be bounded",
        );
    }

    /// Proof: discard_confirmed_frames maintains invariants.
    ///
    /// Verifies that discarding frames maintains INV-4 and INV-5.
    ///
    /// Note: unwind(15) accounts for Vec initialization (8) + loop iterations (5) + buffer
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Discard operation preserves invariants (INV-4, INV-5)
    /// - Related: proof_add_single_input_maintains_invariants
    #[kani::proof]
    #[kani::unwind(15)]
    fn proof_discard_maintains_invariants() {
        let mut queue = test_queue(0);

        // Add a few inputs first
        for i in 0..5i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        let discard_frame: i32 = kani::any();
        kani::assume(discard_frame >= 0 && discard_frame <= 10);

        queue.discard_confirmed_frames(Frame::new(discard_frame));

        // INV-4: length bounded
        kani::assert(
            queue.length <= INPUT_QUEUE_LENGTH,
            "Length should be bounded after discard",
        );
        kani::assert(queue.length >= 1, "Should keep at least one entry");

        // INV-5: indices valid
        kani::assert(
            queue.head < INPUT_QUEUE_LENGTH,
            "Head should be within bounds after discard",
        );
        kani::assert(
            queue.tail < INPUT_QUEUE_LENGTH,
            "Tail should be within bounds after discard",
        );
    }

    /// Proof: Frame delay doesn't violate invariants.
    ///
    /// Verifies that setting frame delay maintains valid queue state.
    /// Tests with concrete delay values (0 and 2) to ensure both zero and non-zero
    /// delay paths are verified while keeping verification tractable.
    ///
    /// Note: unwind(15) accounts for Vec initialization (8) + frame delay iterations + buffer
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Frame delay preserves invariants (INV-4, INV-5)
    /// - Related: proof_add_single_input_maintains_invariants
    #[kani::proof]
    #[kani::unwind(15)]
    fn proof_frame_delay_maintains_invariants() {
        let mut queue = test_queue(0);

        // Test with delay = 2 (non-zero delay to exercise frame delay logic)
        let delay: usize = 2;

        // set_frame_delay should succeed for valid delays (< INPUT_QUEUE_LENGTH)
        let set_result = queue.set_frame_delay(delay);
        kani::assert(set_result.is_ok(), "Valid delay should be accepted");

        // Add input with delay
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 0 });
        let result = queue.add_input(input);

        // With delay, the actual frame stored is frame + delay
        kani::assert(
            result.as_i32() == delay as i32,
            "With delay, should store at frame 0 + delay",
        );

        // INV-4 and INV-5 should hold
        kani::assert(
            queue.length <= INPUT_QUEUE_LENGTH,
            "Length should be bounded",
        );
        kani::assert(
            queue.head < INPUT_QUEUE_LENGTH,
            "Head should be within bounds",
        );
        kani::assert(
            queue.tail < INPUT_QUEUE_LENGTH,
            "Tail should be within bounds",
        );
    }

    /// Proof: Mid-session frame-delay increases gap-fill with confirmed inputs.
    ///
    /// Verifies the runtime delay-increase contract on actual `InputQueue`
    /// state: increasing delay after inputs exist replicates the most recent
    /// confirmed input into the created gap and leaves the next user input
    /// sequential under the new delay.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Delay increase gap-fill and post-change sequentiality
    /// - Related: proof_frame_delay_maintains_invariants
    #[kani::proof]
    #[kani::unwind(16)]
    fn proof_frame_delay_increase_gap_fills_confirmed_inputs() {
        let mut queue = test_queue(0);

        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 4 });
        let input1 = PlayerInput::new(Frame::new(1), TestInput { inp: 9 });
        kani::assert(
            queue.add_input(input0) == Frame::new(0),
            "First input should land at frame 0",
        );
        kani::assert(
            queue.add_input(input1) == Frame::new(1),
            "Second input should land at frame 1",
        );

        let result = queue.set_frame_delay(2);
        kani::assert(result.is_ok(), "Delay increase should succeed");
        kani::assert(queue.frame_delay == 2, "Frame delay should update");
        kani::assert(
            queue.last_added_frame == Frame::new(3),
            "Delay increase should add two gap-fill frames",
        );

        let gap2 = queue.confirmed_input(Frame::new(2));
        let gap3 = queue.confirmed_input(Frame::new(3));
        kani::assert(gap2.is_ok(), "First gap-fill frame should be confirmed");
        kani::assert(gap3.is_ok(), "Second gap-fill frame should be confirmed");

        if let Ok(input) = gap2 {
            kani::assert(
                input.input.inp == 9,
                "First gap-fill input should replicate the previous input",
            );
        }
        if let Ok(input) = gap3 {
            kani::assert(
                input.input.inp == 9,
                "Second gap-fill input should replicate the previous input",
            );
        }

        let next = PlayerInput::new(Frame::new(2), TestInput { inp: 11 });
        kani::assert(
            queue.add_input(next) == Frame::new(4),
            "Next user input should remain sequential under new delay",
        );
    }

    /// Proof: Mid-session frame-delay decreases are rejected without mutation.
    ///
    /// Verifies the transactional rejection path for unsupported delay
    /// decreases after inputs exist.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Delay decrease rejection and state preservation
    /// - Related: proof_frame_delay_increase_gap_fills_confirmed_inputs
    #[kani::proof]
    #[kani::unwind(12)]
    fn proof_frame_delay_decrease_rejected_no_mutation() {
        let mut queue = test_queue(0);

        kani::assert(
            queue.set_frame_delay(2).is_ok(),
            "Initial delay setup should succeed",
        );
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 7 });
        kani::assert(
            queue.add_input(input) == Frame::new(2),
            "Delayed input should land at frame 2",
        );

        let old_head = queue.head;
        let old_tail = queue.tail;
        let old_length = queue.length;
        let old_last_added = queue.last_added_frame;
        let old_first_incorrect = queue.first_incorrect_frame;
        let old_last_requested = queue.last_requested_frame;
        let old_delay = queue.frame_delay;
        let old_frozen = queue.frozen;

        let result = queue.set_frame_delay(1);
        kani::assert(result.is_err(), "Mid-session delay decrease should fail");
        kani::assert(queue.head == old_head, "Head should not change");
        kani::assert(queue.tail == old_tail, "Tail should not change");
        kani::assert(queue.length == old_length, "Length should not change");
        kani::assert(
            queue.last_added_frame == old_last_added,
            "last_added_frame should not change",
        );
        kani::assert(
            queue.first_incorrect_frame == old_first_incorrect,
            "first_incorrect_frame should not change",
        );
        kani::assert(
            queue.last_requested_frame == old_last_requested,
            "last_requested_frame should not change",
        );
        kani::assert(queue.frame_delay == old_delay, "Delay should not change");
        kani::assert(queue.frozen == old_frozen, "Frozen flag should not change");

        let confirmed = queue.confirmed_input(Frame::new(2));
        kani::assert(
            confirmed.is_ok(),
            "Previously confirmed delayed input should remain available",
        );
        if let Ok(input) = confirmed {
            kani::assert(
                input.input.inp == 7,
                "Previously confirmed delayed input should be unchanged",
            );
        }
    }

    /// Proof: Frozen queues do not mutate when new inputs are added.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Freeze no-op behavior for subsequent add_input calls
    /// - Related: proof_frame_delay_decrease_rejected_no_mutation
    #[kani::proof]
    #[kani::unwind(12)]
    fn proof_freeze_add_input_no_mutation() {
        let mut queue = test_queue(0);

        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 1 });
        let input1 = PlayerInput::new(Frame::new(1), TestInput { inp: 2 });
        queue.add_input(input0);
        queue.add_input(input1);
        queue.freeze();

        let old_head = queue.head;
        let old_tail = queue.tail;
        let old_length = queue.length;
        let old_last_added = queue.last_added_frame;
        let old_delay = queue.frame_delay;

        let attempted = PlayerInput::new(Frame::new(2), TestInput { inp: 99 });
        let returned = queue.add_input(attempted);

        kani::assert(
            returned == old_last_added,
            "Frozen add_input should return existing last_added_frame",
        );
        kani::assert(queue.is_frozen(), "Queue should remain frozen");
        kani::assert(queue.head == old_head, "Head should not change");
        kani::assert(queue.tail == old_tail, "Tail should not change");
        kani::assert(queue.length == old_length, "Length should not change");
        kani::assert(
            queue.last_added_frame == old_last_added,
            "last_added_frame should not change",
        );
        kani::assert(queue.frame_delay == old_delay, "Delay should not change");

        let confirmed = queue.confirmed_input(old_last_added);
        kani::assert(
            confirmed.is_ok(),
            "Last confirmed input should remain available after freeze",
        );
        if let Ok(input) = confirmed {
            kani::assert(
                input.input.inp == 2,
                "Frozen queue should preserve last confirmed input",
            );
        }
    }

    /// Proof: Confirmed retrieval matches the delayed frame produced by add_input.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: add_input return frame and confirmed_input agree under delay
    /// - Related: proof_frame_delay_maintains_invariants
    #[kani::proof]
    #[kani::unwind(12)]
    fn proof_confirmed_input_matches_delayed_add() {
        let mut queue = test_queue(0);

        kani::assert(
            queue.set_frame_delay(1).is_ok(),
            "Initial delay setup should succeed",
        );
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 42 });
        let actual_frame = queue.add_input(input);

        kani::assert(
            actual_frame == Frame::new(1),
            "Input should land at delayed frame",
        );

        let confirmed = queue.confirmed_input(actual_frame);
        kani::assert(
            confirmed.is_ok(),
            "Delayed frame should be retrievable as confirmed input",
        );
        if let Ok(confirmed_input) = confirmed {
            kani::assert(
                confirmed_input.frame == actual_frame,
                "Confirmed input frame should match add_input return",
            );
            kani::assert(
                confirmed_input.input.inp == 42,
                "Confirmed input payload should match add_input payload",
            );
        }
    }

    /// Proof: Non-sequential inputs are rejected.
    ///
    /// Verifies that add_input rejects non-sequential frame inputs, preserving invariants.
    /// Uses a concrete skip value (3) rather than symbolic to keep verification tractable.
    /// The proof is about rejection behavior for non-sequential frames, not about all gap sizes.
    ///
    /// Note: unwind(20) accounts for Vec initialization (8) + add_input operations (2 calls) + buffer.
    /// With skip=3, the non-sequential check rejects the input before the gap-fill loop runs
    /// (gap-fill loop iterates 0 times since rejection happens in add_input's sequentiality guard).
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Non-sequential input rejection
    /// - Related: proof_sequential_inputs_maintain_invariants (covers the complementary
    ///   case: sequential inputs are accepted and maintain invariants)
    #[kani::proof]
    #[kani::unwind(20)]
    fn proof_non_sequential_rejected() {
        let mut queue = test_queue(0);

        // Add first input
        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 0 });
        queue.add_input(input0);

        // Try to add non-sequential input (skip frame 1, jump to frame 3)
        let skip: i32 = 3;

        let bad_input = PlayerInput::new(Frame::new(skip), TestInput { inp: 1 });
        let result = queue.add_input(bad_input);

        kani::assert(result.is_null(), "Non-sequential input should be rejected");
        kani::assert(queue.length == 1, "Length should not change on rejection");
    }

    /// Proof: reset_prediction maintains structural invariants.
    ///
    /// Note: unwind(12) accounts for Vec initialization (8) + loop iterations (3) + buffer
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: reset_prediction preserves structure
    /// - Related: proof_new_queue_valid
    #[kani::proof]
    #[kani::unwind(12)]
    fn proof_reset_maintains_structure() {
        let mut queue = test_queue(0);

        // Add some inputs
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
            queue.add_input(input);
        }

        let old_length = queue.length;
        let old_head = queue.head;
        let old_tail = queue.tail;

        queue.reset_prediction();

        // Structure should be preserved
        kani::assert(queue.length == old_length, "Length should be preserved");
        kani::assert(queue.head == old_head, "Head should be preserved");
        kani::assert(queue.tail == old_tail, "Tail should be preserved");

        // Prediction state should be reset
        kani::assert(
            queue.first_incorrect_frame.is_null(),
            "first_incorrect_frame should be null",
        );
        kani::assert(
            queue.prediction.frame.is_null(),
            "prediction frame should be null",
        );
        kani::assert(
            queue.last_requested_frame.is_null(),
            "last_requested_frame should be null",
        );
    }

    /// Proof: Confirmed input retrieval is valid for stored frames.
    ///
    /// Note: unwind(15) accounts for Vec initialization (8) + loop iterations (3) + buffer
    /// Uses concrete values to keep verification tractable while still proving index validity.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: confirmed_input index bounds validity
    /// - Related: proof_queue_index_calculation
    #[kani::proof]
    #[kani::unwind(15)]
    fn proof_confirmed_input_valid_index() {
        let mut queue = test_queue(0);

        // Add 3 inputs (concrete count for tractability)
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Request a frame (symbolic, but bounded by concrete count)
        let request_frame: i32 = kani::any();
        kani::assume(request_frame >= 0 && request_frame < 3);

        // Call confirmed_input to trigger internal index calculations
        let _result = queue.confirmed_input(Frame::new(request_frame));

        // Index calculation should be valid
        let offset = request_frame as usize % INPUT_QUEUE_LENGTH;
        kani::assert(
            offset < INPUT_QUEUE_LENGTH,
            "Calculated offset should be valid",
        );
    }

    /// Proof: Mid-session delay increase fills the exact new gap.
    ///
    /// Uses a concrete delta of 2 for tractability while covering the runtime
    /// branch that snapshots state, appends filler frames, and updates
    /// `frame_delay`.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Runtime delay increase gap-fill sequence
    /// - Related: proof_frame_delay_maintains_invariants,
    ///   proof_delay_decrease_after_input_rejected_no_mutation
    #[kani::proof]
    #[kani::unwind(15)]
    fn proof_mid_session_delay_increase_gap_fills_sequentially() {
        let mut queue = test_queue(0);

        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 7 });
        let added = queue.add_input(input);
        kani::assert(added == Frame::new(0), "initial input should be accepted");

        let result = queue.set_frame_delay(2);
        kani::assert(result.is_ok(), "valid delay increase should succeed");
        kani::assert(queue.frame_delay == 2, "frame delay should be updated");
        kani::assert(
            queue.last_added_frame == Frame::new(2),
            "delay increase by 2 should append two filler frames",
        );
        kani::assert(queue.length == 3, "queue should contain input plus fillers");
        kani::assert(queue.head < INPUT_QUEUE_LENGTH, "head remains in bounds");
        kani::assert(queue.tail < INPUT_QUEUE_LENGTH, "tail remains in bounds");

        let frame1 = queue.confirmed_input(Frame::new(1));
        let frame2 = queue.confirmed_input(Frame::new(2));
        kani::assert(frame1.is_ok(), "first filler frame should be confirmed");
        kani::assert(frame2.is_ok(), "second filler frame should be confirmed");
        if let Ok(filler) = frame1 {
            kani::assert(filler.input.inp == 7, "first filler repeats last input");
        }
        if let Ok(filler) = frame2 {
            kani::assert(filler.input.inp == 7, "second filler repeats last input");
        }
    }

    /// Proof: Mid-session delay decreases are rejected without mutating state.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Rejected delay decrease no-mutation
    /// - Related: proof_mid_session_delay_increase_gap_fills_sequentially
    #[kani::proof]
    #[kani::unwind(12)]
    fn proof_delay_decrease_after_input_rejected_no_mutation() {
        let mut queue = test_queue(0);

        let set_result = queue.set_frame_delay(2);
        kani::assert(set_result.is_ok(), "initial delay should be accepted");
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 9 });
        let added = queue.add_input(input);
        kani::assert(added == Frame::new(2), "delayed input should be accepted");

        let head_before = queue.head;
        let tail_before = queue.tail;
        let length_before = queue.length;
        let last_added_before = queue.last_added_frame;
        let first_incorrect_before = queue.first_incorrect_frame;
        let last_requested_before = queue.last_requested_frame;
        let delay_before = queue.frame_delay;

        let decrease = queue.set_frame_delay(1);
        kani::assert(decrease.is_err(), "delay decrease should be rejected");
        kani::assert(queue.head == head_before, "head should be unchanged");
        kani::assert(queue.tail == tail_before, "tail should be unchanged");
        kani::assert(queue.length == length_before, "length should be unchanged");
        kani::assert(
            queue.last_added_frame == last_added_before,
            "last_added_frame should be unchanged",
        );
        kani::assert(
            queue.first_incorrect_frame == first_incorrect_before,
            "first_incorrect_frame should be unchanged",
        );
        kani::assert(
            queue.last_requested_frame == last_requested_before,
            "last_requested_frame should be unchanged",
        );
        kani::assert(
            queue.frame_delay == delay_before,
            "delay should be unchanged",
        );
    }

    /// Proof: Freezing makes later queue mutations no-ops.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Freeze no-mutation after later set_frame_delay/add_input
    /// - Related: proof_delay_decrease_after_input_rejected_no_mutation
    #[kani::proof]
    #[kani::unwind(12)]
    fn proof_freeze_add_input_noop_preserves_state() {
        let mut queue = test_queue(0);

        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 4 });
        let input1 = PlayerInput::new(Frame::new(1), TestInput { inp: 5 });
        let added0 = queue.add_input(input0);
        let added1 = queue.add_input(input1);
        kani::assert(added0 == Frame::new(0), "first input accepted");
        kani::assert(added1 == Frame::new(1), "second input accepted");

        queue.freeze();

        let head_before = queue.head;
        let tail_before = queue.tail;
        let length_before = queue.length;
        let last_added_before = queue.last_added_frame;
        let first_incorrect_before = queue.first_incorrect_frame;
        let last_requested_before = queue.last_requested_frame;
        let delay_before = queue.frame_delay;

        let delay_result = queue.set_frame_delay(2);
        kani::assert(
            delay_result.is_ok(),
            "valid set_frame_delay on frozen queue should be a no-op",
        );
        kani::assert(queue.head == head_before, "head should be unchanged");
        kani::assert(queue.tail == tail_before, "tail should be unchanged");
        kani::assert(queue.length == length_before, "length should be unchanged");
        kani::assert(
            queue.last_added_frame == last_added_before,
            "last_added_frame should be unchanged",
        );
        kani::assert(
            queue.first_incorrect_frame == first_incorrect_before,
            "first_incorrect_frame should be unchanged",
        );
        kani::assert(
            queue.last_requested_frame == last_requested_before,
            "last_requested_frame should be unchanged",
        );
        kani::assert(
            queue.frame_delay == delay_before,
            "delay should be unchanged",
        );

        let late = PlayerInput::new(Frame::new(2), TestInput { inp: 99 });
        let result = queue.add_input(late);

        kani::assert(
            result == last_added_before,
            "frozen add_input should return existing last_added_frame",
        );
        kani::assert(queue.head == head_before, "head should be unchanged");
        kani::assert(queue.tail == tail_before, "tail should be unchanged");
        kani::assert(queue.length == length_before, "length should be unchanged");
        kani::assert(
            queue.last_added_frame == last_added_before,
            "last_added_frame should be unchanged",
        );
        kani::assert(
            queue.first_incorrect_frame == first_incorrect_before,
            "first_incorrect_frame should be unchanged",
        );
        kani::assert(
            queue.last_requested_frame == last_requested_before,
            "last_requested_frame should be unchanged",
        );
        kani::assert(
            queue.frame_delay == delay_before,
            "delay should be unchanged",
        );
    }

    /// Proof: A prediction episode enters at the queue's first missing frame.
    ///
    /// Verifies the F17 episode-entry invariant on `input()`: after a
    /// bounded-symbolic history (virgin queue or one sequential add — the
    /// entry expression depends only on `last_added_frame`, so longer
    /// histories add no new behavior), a request that engages prediction —
    /// at or beyond the first missing frame — freezes `prediction.frame` at
    /// exactly `last_added_frame + 1` (frame 0 on a virgin queue), NOT at the
    /// requested frame. The follow-up sequential arrival therefore lands
    /// exactly on the episode frame (the no-swallow invariant): its
    /// misprediction comparison runs, a differing value is flagged in
    /// `first_incorrect_frame`, and a matching value either exits the episode
    /// (requested frame confirmed) or advances it to the next missing frame.
    ///
    /// Note: unwind(10) covers buffer construction (7 + 1) plus margin; there
    /// are no other loops (the gap-fill loop in `advance_queue_head` runs
    /// zero iterations for sequential delay-0 adds). A wider variant with a
    /// 0..=2 symbolic add loop also verifies but takes >10 min; this shape
    /// runs in seconds.
    ///
    /// - Tier: 2 (registered with the other `InputQueue` proofs, several of
    ///   which also run below the nominal 30s Tier-1 band)
    /// - Verifies: Prediction-episode entry frame and no-swallow arrival (F17)
    /// - Related: proof_reset_maintains_structure,
    ///   proof_sequential_inputs_maintain_invariants
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_prediction_entry_at_first_missing_frame() {
        let mut queue = test_queue(0);

        // Bounded-symbolic history: virgin queue (first missing frame 0) or
        // one accepted sequential input (first missing frame 1).
        let virgin: bool = kani::any();
        if !virgin {
            let input = PlayerInput::new(Frame::new(0), TestInput { inp: 10 });
            kani::assert(
                queue.add_input(input) == Frame::new(0),
                "sequential add should be accepted",
            );
        }
        let first_missing: i32 = if virgin { 0 } else { 1 };

        // Request at or beyond the first missing frame: covers both the
        // lockstep shape (requested == first missing frame) and the F17
        // rollback re-simulation shape (requested above the missing window).
        let requested: i32 = kani::any();
        kani::assume(requested >= first_missing && requested <= first_missing + 1);
        let result = queue.input(Frame::new(requested));

        kani::assert(
            result.is_some(),
            "prediction request should produce an input",
        );
        if let Some((_, status)) = result {
            kani::assert(
                status == InputStatus::Predicted,
                "missing frame should be served as a prediction",
            );
        }
        // Episode-entry invariant: the episode is frozen at the first missing
        // frame (last_added_frame + 1; frame 0 on a virgin queue).
        kani::assert(
            queue.prediction.frame == Frame::new(first_missing),
            "prediction episode must enter at the first missing frame",
        );

        // No-swallow invariant: the next sequential arrival lands exactly on
        // the episode frame, so its misprediction comparison runs.
        let arrival = PlayerInput::new(Frame::new(first_missing), TestInput { inp: 10 });
        kani::assert(
            queue.add_input(arrival) == Frame::new(first_missing),
            "sequential arrival must land on the episode frame",
        );

        if virgin {
            // Virgin queue: the frozen prediction is the blank default (no
            // confirmed input yet), so the differing arrival (10) is detected.
            kani::assert(
                queue.first_incorrect_frame == Frame::new(0),
                "differing arrival on a virgin queue must be flagged",
            );
            kani::assert(
                queue.prediction.frame == Frame::new(1),
                "episode advances past the mispredicted arrival",
            );
        } else {
            // The arrival matches the frozen prediction (both 10).
            kani::assert(
                queue.first_incorrect_frame.is_null(),
                "matching arrival must not be flagged",
            );
            if requested == first_missing {
                kani::assert(
                    queue.prediction.frame.is_null(),
                    "episode exits once the requested frame is confirmed",
                );
            } else {
                kani::assert(
                    queue.prediction.frame == Frame::new(first_missing + 1),
                    "episode advances to the next missing frame",
                );
            }
        }
    }
}

#[cfg(all(test, feature = "hot-join"))]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod hot_join_input_queue_tests {

    use std::net::SocketAddr;

    use serde::{Deserialize, Serialize};

    use super::*;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
    struct TestInput {
        inp: u8,
    }

    #[derive(Clone, Debug)]
    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = Vec<u8>;
        type Address = SocketAddr;
    }

    fn test_queue(player_index: usize) -> InputQueue<TestConfig> {
        InputQueue::<TestConfig>::new(player_index).expect("Failed to create test queue")
    }

    /// Test 1: `unfreeze` re-enables `add_input`.
    #[test]
    fn unfreeze_reenables_add_input() {
        let mut queue = test_queue(0);

        // Add a couple of inputs so the queue has real state.
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(0), TestInput { inp: 1 })),
            Frame::new(0)
        );
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(1), TestInput { inp: 2 })),
            Frame::new(1)
        );

        queue.freeze();
        assert!(queue.is_frozen());

        // While frozen, add_input is a no-op: it returns the existing
        // last_added_frame (1), NOT a newly-accepted frame, and does not advance.
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(2), TestInput { inp: 3 })),
            Frame::new(1)
        );
        assert_eq!(queue.last_added_frame, Frame::new(1));

        queue.unfreeze();
        assert!(!queue.is_frozen());

        // After unfreeze, the next sequential input (frame 2) is accepted.
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(2), TestInput { inp: 3 })),
            Frame::new(2)
        );
        assert_eq!(queue.last_added_frame, Frame::new(2));
    }

    /// Test 2: `reset_to_frame` to a non-zero frame (delay 0) accepts inputs at F.
    #[test]
    fn reset_to_frame_nonzero_accepts_at_frame() {
        let mut queue = test_queue(0);
        let f = Frame::new(50);

        queue.reset_to_frame(f);
        assert!(!queue.is_frozen());

        // add_input(input @ F) is accepted (returns F, not NULL).
        assert_eq!(
            queue.add_input(PlayerInput::new(f, TestInput { inp: 7 })),
            f
        );

        // input(F) returns the just-added input as Confirmed.
        let (value, status) = queue.input(f).expect("input at F should be available");
        assert_eq!(value.inp, 7);
        assert_eq!(status, InputStatus::Confirmed);

        // input(F+1) is a prediction (no real input yet).
        let (_pred, pred_status) = queue
            .input(Frame::new(51))
            .expect("input at F+1 should predict");
        assert_eq!(pred_status, InputStatus::Predicted);

        // Sequential adds continue to be accepted.
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(51), TestInput { inp: 8 })),
            Frame::new(51)
        );
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(52), TestInput { inp: 9 })),
            Frame::new(52)
        );
    }

    /// Test 2 (delay variant): reset honors a non-zero frame_delay.
    #[test]
    fn reset_to_frame_with_delay_accepts_at_frame() {
        let mut queue = test_queue(0);
        queue.set_frame_delay(2).expect("valid delay");
        let f = Frame::new(50);

        queue.reset_to_frame(f);

        // With delay 2, add_input(input @ F) is accepted and reports the
        // delayed frame F + delay = 52.
        assert_eq!(
            queue.add_input(PlayerInput::new(f, TestInput { inp: 7 })),
            Frame::new(52)
        );
        // Subsequent sequential user frames are accepted.
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(51), TestInput { inp: 8 })),
            Frame::new(53)
        );
    }

    /// Regression guard for the Critical frame-indexing bug: after
    /// `reset_to_frame(F)` the circular buffer must be realigned so that
    /// `confirmed_input` (which indexes absolutely as `inputs[f % queue_length]`)
    /// finds the input added at `F`. F is chosen so that `F % queue_length != 0`
    /// (50 % 128 == 50); under the old `head = tail = 0` code the input for F
    /// landed at slot 0 while `confirmed_input` read slot 50, returning
    /// `NoConfirmedInput`. Covers both delay 0 and a delay > 0 case.
    #[test]
    fn reset_to_frame_then_confirmed_input_roundtrips() {
        // --- delay 0 ---
        {
            let mut queue = test_queue(0);
            let f = Frame::new(50);
            assert_ne!(f.as_i32() as usize % queue.queue_length(), 0);

            queue.reset_to_frame(f);

            // Accept the joiner's real input at the activation frame.
            assert_eq!(
                queue.add_input(PlayerInput::new(f, TestInput { inp: 7 })),
                f
            );

            // confirmed_input(F + delay) == confirmed_input(F) returns the added
            // value (this is the assertion that fails under the old code).
            let confirmed = queue
                .confirmed_input(f)
                .expect("confirmed_input(F) must find the input added after reset");
            assert_eq!(confirmed.frame, f);
            assert_eq!(confirmed.input.inp, 7);

            // input(F) still reports Confirmed via the tail-relative path.
            let (value, status) = queue.input(f).expect("input at F should be available");
            assert_eq!(value.inp, 7);
            assert_eq!(status, InputStatus::Confirmed);

            // A second sequential add is also addressable by confirmed_input.
            assert_eq!(
                queue.add_input(PlayerInput::new(Frame::new(51), TestInput { inp: 8 })),
                Frame::new(51)
            );
            let confirmed_next = queue
                .confirmed_input(Frame::new(51))
                .expect("confirmed_input(F+1) must find the second input");
            assert_eq!(confirmed_next.input.inp, 8);

            // Discard + invariants still hold after the realigned writes.
            queue.discard_confirmed_frames(Frame::new(51));
            assert!(queue.check_invariants().is_ok());
        }

        // --- delay > 0 ---
        {
            let mut queue = test_queue(0);
            let delay = 3usize;
            queue.set_frame_delay(delay).expect("valid delay");
            let f = Frame::new(50);
            let delayed = Frame::new(f.as_i32() + delay as i32); // 53
            assert_ne!(delayed.as_i32() as usize % queue.queue_length(), 0);

            queue.reset_to_frame(f);

            // add_input(input @ F) is accepted and reports the delayed frame.
            assert_eq!(
                queue.add_input(PlayerInput::new(f, TestInput { inp: 7 })),
                delayed
            );

            // confirmed_input is keyed by the DELAYED frame (that is the frame
            // actually stored in the queue).
            let confirmed = queue
                .confirmed_input(delayed)
                .expect("confirmed_input(F+delay) must find the input added after reset");
            assert_eq!(confirmed.frame, delayed);
            assert_eq!(confirmed.input.inp, 7);

            // input(F+delay) reports Confirmed: with frame delay the input is
            // stored at its DELAYED frame, and the simulation queries `input` by
            // that stored frame (the queue stamps slots with `frame + delay`).
            let (value, status) = queue
                .input(delayed)
                .expect("input at F+delay should be available");
            assert_eq!(value.inp, 7);
            assert_eq!(status, InputStatus::Confirmed);

            // Second sequential add: confirmed_input(F+1+delay) works.
            assert_eq!(
                queue.add_input(PlayerInput::new(Frame::new(51), TestInput { inp: 8 })),
                Frame::new(54)
            );
            let confirmed_next = queue
                .confirmed_input(Frame::new(54))
                .expect("confirmed_input(F+1+delay) must find the second input");
            assert_eq!(confirmed_next.input.inp, 8);

            queue.discard_confirmed_frames(Frame::new(54));
            assert!(queue.check_invariants().is_ok());
        }
    }

    /// Test 3: `reset_to_frame(0)` reproduces fresh-queue behavior.
    #[test]
    fn reset_to_frame_zero_matches_fresh_queue() {
        let mut queue = test_queue(0);
        // Mutate the queue first so reset has something to undo.
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(0), TestInput { inp: 1 })),
            Frame::new(0)
        );
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(1), TestInput { inp: 2 })),
            Frame::new(1)
        );

        queue.reset_to_frame(Frame::new(0));

        // Matches a fresh queue: first-input path is taken.
        assert_eq!(queue.last_added_frame, Frame::NULL);
        assert!(queue.first_frame);
        assert!(!queue.is_frozen());

        // add_input(input @ 0) is accepted.
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(0), TestInput { inp: 5 })),
            Frame::new(0)
        );
        let (value, status) = queue
            .input(Frame::new(0))
            .expect("input at 0 should be available");
        assert_eq!(value.inp, 5);
        assert_eq!(status, InputStatus::Confirmed);
    }

    /// Test 4: `reset_to_frame` preserves `last_confirmed_input`, so a
    /// pre-real-input prediction equals the preserved confirmed value.
    #[test]
    fn reset_to_frame_preserves_last_confirmed_input() {
        let mut queue = test_queue(0);

        // `last_confirmed_input` is set by the add_input path (see
        // `add_input_by_frame`). Add an input with a distinctive, non-default
        // value so we can tell a preserved prediction from a default one.
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(0), TestInput { inp: 200 })),
            Frame::new(0)
        );
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 200 }));

        // Reset forward to a later activation frame.
        let f = Frame::new(10);
        queue.reset_to_frame(f);

        // White-box: the field itself is preserved (not cleared to None).
        assert_eq!(queue.last_confirmed_input, Some(TestInput { inp: 200 }));

        // Behavioral: input(F) before any real input predicts the preserved
        // confirmed value (200), NOT the default (0).
        let (value, status) = queue.input(f).expect("input at F should predict");
        assert_eq!(status, InputStatus::Predicted);
        assert_eq!(value.inp, 200);
    }

    /// Test 5: freeze then `reset_to_frame(F)` leaves the queue unfrozen and
    /// accepting at F (the host reactivation path).
    #[test]
    fn reset_to_frame_after_freeze_reactivates() {
        let mut queue = test_queue(0);
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(0), TestInput { inp: 1 })),
            Frame::new(0)
        );
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(1), TestInput { inp: 2 })),
            Frame::new(1)
        );

        queue.freeze();
        assert!(queue.is_frozen());

        let f = Frame::new(20);
        queue.reset_to_frame(f);

        // Reset unfreezes the queue (reactivation).
        assert!(!queue.is_frozen());

        // And it accepts the joiner's real input at the activation frame.
        assert_eq!(
            queue.add_input(PlayerInput::new(f, TestInput { inp: 42 })),
            f
        );
        let (value, status) = queue.input(f).expect("input at F should be available");
        assert_eq!(value.inp, 42);
        assert_eq!(status, InputStatus::Confirmed);
    }

    /// `refreeze_with_value` after a `reset_to_frame` reopen restores the
    /// frozen state with the caller-captured *pre-reopen* frozen value (the
    /// N-peer survivor `JoinAborted` path), and re-blocks `add_input`. The
    /// joiner input confirmed by the reopened queue (which overwrote
    /// `last_confirmed_input`) must NOT leak into the restored value.
    #[test]
    fn refreeze_with_value_after_reset_restores_pre_reopen_value() {
        let mut queue = test_queue(0);
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(0), TestInput { inp: 7 })),
            Frame::new(0)
        );
        queue.freeze();
        // The caller captures the agreed frozen value BEFORE the reopen.
        let pre_reopen_value = queue.last_confirmed_input;
        assert_eq!(pre_reopen_value, Some(TestInput { inp: 7 }));

        // Reopen at the activation frame (the survivor reopen), then accept the
        // joiner's real input at F — the aborted attempt's residue, which
        // overwrites the queue's tracked last-confirmed input.
        let f = Frame::new(20);
        queue.reset_to_frame(f);
        assert!(!queue.is_frozen());
        assert_eq!(
            queue.add_input(PlayerInput::new(f, TestInput { inp: 99 })),
            f
        );
        assert_eq!(
            queue.last_confirmed_input,
            Some(TestInput { inp: 99 }),
            "the accepted real input overwrites last_confirmed_input (why the caller must capture pre-reopen)"
        );

        // Abort: re-freeze with the captured value. The frozen value must be
        // the PRE-reopen value, and further inputs are silently ignored again
        // (the frozen no-op returns the existing last_added_frame and mutates
        // nothing).
        queue.refreeze_with_value(pre_reopen_value);
        assert!(queue.is_frozen());
        assert_eq!(queue.last_confirmed_input, pre_reopen_value);
        assert_eq!(
            queue.add_input(PlayerInput::new(
                safe_frame_add!(f, 1, "test"),
                TestInput { inp: 100 }
            )),
            f,
            "a re-frozen queue must ignore inputs (no-op returns last_added_frame)"
        );
        assert_eq!(
            queue.last_confirmed_input, pre_reopen_value,
            "the ignored input must not alter the restored frozen value"
        );
    }

    /// `refreeze_with_value` is idempotent on an already-frozen queue given the
    /// same value.
    #[test]
    fn refreeze_with_value_is_idempotent_when_frozen() {
        let mut queue = test_queue(0);
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(0), TestInput { inp: 7 })),
            Frame::new(0)
        );
        queue.freeze();
        let frozen_value = queue.last_confirmed_input;

        queue.refreeze_with_value(frozen_value);

        assert!(queue.is_frozen());
        assert_eq!(queue.last_confirmed_input, frozen_value);
    }

    /// Test 6: `reset_to_frame` with a negative frame is a no-op and does not panic.
    #[test]
    fn reset_to_frame_negative_is_noop() {
        let mut queue = test_queue(0);
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(0), TestInput { inp: 1 })),
            Frame::new(0)
        );
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(1), TestInput { inp: 2 })),
            Frame::new(1)
        );

        let before = queue.clone();

        // NULL frame: no-op.
        queue.reset_to_frame(Frame::NULL);
        assert_eq!(queue.head, before.head);
        assert_eq!(queue.tail, before.tail);
        assert_eq!(queue.length, before.length);
        assert_eq!(queue.first_frame, before.first_frame);
        assert_eq!(queue.last_added_frame, before.last_added_frame);
        assert_eq!(queue.last_confirmed_input, before.last_confirmed_input);
        assert_eq!(queue.frozen, before.frozen);

        // A different negative frame: also a no-op.
        queue.reset_to_frame(Frame::new(-5));
        assert_eq!(queue.last_added_frame, before.last_added_frame);
        assert_eq!(queue.length, before.length);
    }

    /// Hot-join seeding: after `reset_to_frame(F)` a prediction episode enters
    /// at the activation frame `F` — the repositioned queue's first missing
    /// frame — even when the first request lands beyond it, so the joiner's
    /// real activation-frame input is compared against the preserved frozen
    /// value and a divergence triggers a rollback.
    #[test]
    fn prediction_entry_after_reset_to_frame_starts_at_activation_frame() {
        let mut queue = test_queue(0);
        // The frozen value preserved across the reset (the prediction base).
        assert_eq!(
            queue.add_input(PlayerInput::new(Frame::new(0), TestInput { inp: 200 })),
            Frame::new(0)
        );
        queue.freeze();

        let f = Frame::new(20);
        queue.reset_to_frame(f);

        // The host advances past the activation frame before the joiner's real
        // inputs arrive: the first request lands at frame 23.
        let (value, status) = queue.input(Frame::new(23)).expect("prediction after reset");
        assert_eq!(status, InputStatus::Predicted);
        assert_eq!(value.inp, 200, "prediction uses the preserved frozen value");
        assert_eq!(
            queue.prediction.frame, f,
            "episode must enter at the activation frame, not the requested frame"
        );

        // The joiner's real activation-frame input differs from the frozen
        // value: the misprediction must be detected at exactly F.
        assert_eq!(
            queue.add_input(PlayerInput::new(f, TestInput { inp: 42 })),
            f
        );
        assert_eq!(queue.first_incorrect_frame(), f);
    }
}
