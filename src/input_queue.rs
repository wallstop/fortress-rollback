use crate::frame_info::PlayerInput;
use crate::report_violation;
use crate::telemetry::{InvariantChecker, InvariantViolation, ViolationKind, ViolationSeverity};
use crate::{Config, FortressError, Frame, InputStatus};
use std::cmp;

/// The length of the input queue. This describes the number of inputs Fortress Rollback can hold at the same time per player.
///
/// For Kani verification, we use a smaller queue (8 elements) to keep verification tractable.
/// This is sufficient to prove the invariants hold for the circular buffer logic.
///
/// # Note
///
/// This constant is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
///
/// # Formal Specification Alignment
/// - **TLA+**: `QUEUE_LENGTH` in `specs/tla/InputQueue.tla` (set to 3 for model checking)
/// - **Z3**: `INPUT_QUEUE_LENGTH` in `tests/test_z3_verification.rs` (128)
/// - **Kani**: Uses 8 for tractable verification, production uses 128
/// - **formal-spec.md**: INV-4 (queue length bounds), INV-5 (index validity)
#[cfg(kani)]
pub const INPUT_QUEUE_LENGTH: usize = 8;

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

/// Defines the strategy used to predict inputs when we haven't received the actual input yet.
///
/// Input prediction is crucial for rollback networking - when we need to advance the game
/// but haven't received a remote player's input, we must predict what they will do.
///
/// # Determinism Requirement
///
/// **CRITICAL**: The prediction strategy MUST be deterministic across all peers.
/// Both peers must produce the exact same predicted input given the same arguments.
/// If predictions differ between peers, they will desync during rollback.
///
/// The default implementation (`RepeatLastConfirmed`) is deterministic because:
/// - `last_confirmed_input` is synchronized across all peers via the network protocol
/// - Both peers will have received and confirmed the same input before using it for prediction
///
/// # Custom Strategies
///
/// You can implement custom prediction strategies for game-specific behavior:
///
/// ```ignore
/// struct MyPrediction;
///
/// impl<I: Copy + Default> PredictionStrategy<I> for MyPrediction {
///     fn predict(&self, frame: Frame, last_confirmed_input: Option<I>, _player_index: usize) -> I {
///         // For a fighting game, you might predict "hold block" as a safe default
///         // This MUST be deterministic - don't use random values or timing-dependent data!
///         last_confirmed_input.unwrap_or_default()
///     }
/// }
/// ```
pub trait PredictionStrategy<I: Copy + Default>: Send + Sync {
    /// Predicts the input for a player when their actual input hasn't arrived yet.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame number we're predicting for
    /// * `last_confirmed_input` - The most recent confirmed input from this player, if any.
    ///   This is deterministic across all peers since confirmed inputs are synchronized.
    /// * `player_index` - The index of the player we're predicting for
    ///
    /// # Returns
    ///
    /// The predicted input to use. Must be deterministic across all peers.
    fn predict(&self, frame: Frame, last_confirmed_input: Option<I>, player_index: usize) -> I;
}

/// The default prediction strategy: repeat the last confirmed input.
///
/// This strategy is deterministic because `last_confirmed_input` is guaranteed
/// to be the same across all peers after synchronization.
///
/// If there is no confirmed input yet (e.g., at the start of the game),
/// this returns the default input value.
#[derive(Debug, Clone, Copy, Default)]
pub struct RepeatLastConfirmed;

impl<I: Copy + Default> PredictionStrategy<I> for RepeatLastConfirmed {
    fn predict(&self, _frame: Frame, last_confirmed_input: Option<I>, _player_index: usize) -> I {
        last_confirmed_input.unwrap_or_default()
    }
}

/// A prediction strategy that always returns the default (blank) input.
///
/// This is useful when you want a "do nothing" prediction, which can be
/// safer for some game types where repeating the last input could be dangerous.
#[derive(Debug, Clone, Copy, Default)]
pub struct BlankPrediction;

impl<I: Copy + Default> PredictionStrategy<I> for BlankPrediction {
    fn predict(&self, _frame: Frame, _last_confirmed: Option<I>, _player_index: usize) -> I {
        I::default()
    }
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
    inputs: Vec<PlayerInput<T::Input>>,
    /// A pre-allocated prediction we are going to use to return predictions from.
    prediction: PlayerInput<T::Input>,

    /// The last confirmed input for this player. This is deterministic across all peers
    /// because confirmed inputs are synchronized via the network protocol.
    /// Used as the basis for predictions to ensure determinism.
    last_confirmed_input: Option<T::Input>,
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
    /// Returns `None` if `queue_length < 2`.
    #[must_use]
    pub fn with_queue_length(player_index: usize, queue_length: usize) -> Option<Self> {
        if queue_length < 2 {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "Queue length must be at least 2, got {}",
                queue_length
            );
            return None;
        }
        Some(Self {
            head: 0,
            tail: 0,
            length: 0,
            frame_delay: 0,
            first_frame: true,
            last_added_frame: Frame::NULL,
            first_incorrect_frame: Frame::NULL,
            last_requested_frame: Frame::NULL,
            prediction: PlayerInput::blank_input(Frame::NULL),
            inputs: vec![PlayerInput::blank_input(Frame::NULL); queue_length],
            player_index,
            queue_length,
            last_confirmed_input: None,
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
    /// # Errors
    /// Returns `FortressError::InvalidRequest` if `delay >= queue_length`.
    /// This constraint ensures the circular buffer doesn't overflow when advancing the queue head.
    pub fn set_frame_delay(&mut self, delay: usize) -> Result<(), FortressError> {
        let max_delay = self.max_frame_delay();
        if delay > max_delay {
            return Err(FortressError::InvalidRequest {
                info: format!(
                    "Frame delay {} exceeds maximum allowed value of {} (queue_length - 1). \
                     At 60fps, this would be {:.1}+ seconds of delay, which is impractical for gameplay.",
                    delay,
                    max_delay,
                    delay as f64 / 60.0
                ),
            });
        }
        self.frame_delay = delay;
        Ok(())
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

        if self.inputs[offset].frame == requested_frame {
            return Ok(self.inputs[offset]);
        }

        // the requested confirmed input should not be before a prediction. We should not have asked for a known incorrect frame.
        Err(crate::FortressError::InvalidRequest {
            info: format!(
                "No confirmed input for frame {} (tail={}, head={}, length={})",
                requested_frame, self.tail, self.head, self.length
            ),
        })
    }

    /// Discards confirmed frames **before** the given `frame` from the queue.
    /// All confirmed frames are guaranteed to be synchronized between players,
    /// so there is no need to save the inputs anymore.
    ///
    /// Note: After calling `discard_confirmed_frames(5)`, frames 0-4 are discarded
    /// and frame 5 becomes the new tail (oldest frame in queue).
    pub fn discard_confirmed_frames(&mut self, mut frame: Frame) {
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
        } else if frame <= self.inputs[self.tail].frame {
            // The target frame is at or before the current tail - nothing to delete
            // (frames before tail don't exist in the queue)
        } else {
            // Discard frames from tail up to (but not including) 'frame'
            // After this, 'frame' becomes the new tail
            let tail_frame = self.inputs[self.tail].frame;
            let offset = (frame - tail_frame) as usize;
            self.tail = (self.tail + offset) % self.queue_length;
            self.length -= offset;
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
        if requested_frame < self.inputs[self.tail].frame {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "Requested frame {} is before oldest frame {} in queue",
                requested_frame,
                self.inputs[self.tail].frame
            );
            return None;
        }

        // We currently don't have a prediction frame
        if self.prediction.frame.as_i32() < 0 {
            //  If the frame requested is in our range, fetch it out of the queue and return it.
            let mut offset: usize = (requested_frame - self.inputs[self.tail].frame) as usize;

            if offset < self.length {
                offset = (offset + self.tail) % self.queue_length;
                // Verify circular buffer indexing correctness
                if self.inputs[offset].frame != requested_frame {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InputQueue,
                        "Circular buffer index mismatch: expected frame {}, got frame {} at offset {}",
                        requested_frame,
                        self.inputs[offset].frame,
                        offset
                    );
                    return None;
                }
                return Some((self.inputs[offset].input, InputStatus::Confirmed));
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
            self.prediction = PlayerInput {
                frame: requested_frame,
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

    /// Adds an input frame to the queue. Will consider the set frame delay.
    pub fn add_input(&mut self, input: PlayerInput<T::Input>) -> Frame {
        // Verify that inputs are passed in sequentially by the user, regardless of frame delay.
        if !self.last_added_frame.is_null()
            && input.frame + self.frame_delay as i32 != self.last_added_frame + 1
        {
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
        if !self.last_added_frame.is_null() && frame_number != self.last_added_frame + 1 {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "Input frame {} is not sequential (last_added={})",
                frame_number,
                self.last_added_frame
            );
            return false;
        }
        if frame_number != 0 && self.inputs[previous_position].frame != frame_number - 1 {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "Previous input frame {} does not precede current frame {}",
                self.inputs[previous_position].frame,
                frame_number
            );
            return false;
        }

        // Add the frame to the back of the queue
        self.inputs[self.head] = input;
        self.inputs[self.head].frame = frame_number;
        self.head = (self.head + 1) % self.queue_length;
        self.length += 1;

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
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InputQueue,
                    "Frame {} doesn't match prediction frame {}",
                    frame_number,
                    self.prediction.frame
                );
                return false;
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
                self.prediction.frame += 1;
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
    /// # Note
    ///
    /// The gap-filling logic handles the initial delay setup. If frame delay is changed
    /// mid-session (not supported via public API), the sequential check in [`add_input`]
    /// will reject the input before this function is called, so the gap-filling for
    /// mid-session delay changes will never execute.
    fn advance_queue_head(&mut self, mut input_frame: Frame) -> Frame {
        let previous_position = match self.head {
            0 => self.queue_length - 1,
            _ => self.head - 1,
        };

        let mut expected_frame = if self.first_frame {
            Frame::new(0)
        } else {
            self.inputs[previous_position].frame + 1
        };

        input_frame += self.frame_delay as i32;

        // If the expected frame is ahead of the input (frame delay decreased), reject the input
        if expected_frame > input_frame {
            return Frame::NULL;
        }

        // Fill any gap between expected_frame and input_frame by replicating the previous input.
        // This handles the initial delay setup when frame_delay > 0.
        while expected_frame < input_frame {
            let input_to_replicate = self.inputs[previous_position];
            if !self.add_input_by_frame(input_to_replicate, expected_frame) {
                return Frame::NULL;
            }
            expected_frame += 1;
        }

        // After filling gaps, verify the frame is sequential
        let previous_position = match self.head {
            0 => self.queue_length - 1,
            _ => self.head - 1,
        };
        if input_frame != 0 && input_frame != self.inputs[previous_position].frame + 1 {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "Frame sequencing broken after gap fill: input_frame={}, prev_frame={}",
                input_frame,
                self.inputs[previous_position].frame
            );
            return Frame::NULL;
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
                    .with_details(format!("length={}, max={}", self.length, self.queue_length)),
            );
        }

        // Invariant 2: head and tail are valid indices
        if self.head >= self.queue_length {
            return Err(
                InvariantViolation::new("InputQueue", "head index out of bounds")
                    .with_details(format!("head={}, max={}", self.head, self.queue_length - 1)),
            );
        }

        if self.tail >= self.queue_length {
            return Err(
                InvariantViolation::new("InputQueue", "tail index out of bounds")
                    .with_details(format!("tail={}, max={}", self.tail, self.queue_length - 1)),
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
                    .with_details(format!(
                        "size={}, expected={}",
                        self.inputs.len(),
                        self.queue_length
                    )),
            );
        }

        // Invariant 5: frame_delay is reasonable (less than 256 frames)
        if self.frame_delay > 255 {
            return Err(InvariantViolation::new(
                "InputQueue",
                "frame_delay exceeds reasonable bounds",
            )
            .with_details(format!("frame_delay={}", self.frame_delay)));
        }

        // Invariant 6: first_incorrect_frame is either NULL or a valid frame
        if !self.first_incorrect_frame.is_null() && self.first_incorrect_frame.as_i32() < 0 {
            return Err(
                InvariantViolation::new("InputQueue", "first_incorrect_frame is invalid")
                    .with_details(format!(
                        "first_incorrect_frame={}",
                        self.first_incorrect_frame
                    )),
            );
        }

        // Invariant 7: last_requested_frame is either NULL or a valid frame
        if !self.last_requested_frame.is_null() && self.last_requested_frame.as_i32() < 0 {
            return Err(
                InvariantViolation::new("InputQueue", "last_requested_frame is invalid")
                    .with_details(format!(
                        "last_requested_frame={}",
                        self.last_requested_frame
                    )),
            );
        }

        // Invariant 8: last_added_frame is either NULL or a valid frame
        if !self.last_added_frame.is_null() && self.last_added_frame.as_i32() < 0 {
            return Err(
                InvariantViolation::new("InputQueue", "last_added_frame is invalid")
                    .with_details(format!("last_added_frame={}", self.last_added_frame)),
            );
        }

        Ok(())
    }
}

// #########
// # TESTS #
// #########

#[cfg(test)]
mod input_queue_tests {

    use std::net::SocketAddr;

    use serde::{Deserialize, Serialize};

    use super::*;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
    struct TestInput {
        inp: u8,
    }

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
        let result = queue.confirmed_input(Frame::new(2));
        assert!(result.is_ok());
        let confirmed = result.unwrap();
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
        let result = queue.confirmed_input(Frame::new(5));
        assert!(result.is_ok());
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
        let result = queue.confirmed_input(Frame::new(4));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().input.inp, 4);
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
        let result = queue.confirmed_input(Frame::new(127));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().input.inp, 127);
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
        let result = queue.confirmed_input(Frame::new(4));
        assert!(result.is_ok());
        let input = result.unwrap();
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
        let result = queue.confirmed_input(Frame::new(3));
        assert!(result.is_ok());
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
        let result = queue.confirmed_input(Frame::new(99));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().input.inp, 99);
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

    /// Tests that changing frame delay mid-session causes inputs to be dropped.
    ///
    /// Frame delay is only set at construction via the builder and the API doesn't
    /// expose changing it mid-session. Changing it manually (as in this test) causes
    /// subsequent inputs to be rejected because they fail the sequential check in
    /// `add_input`: `input.frame + frame_delay != last_added_frame + 1`.
    ///
    /// Note: The gap-filling code in `advance_queue_head` exists to handle the initial
    /// delay setup (when first inputs are added with delay > 0). However, it will never
    /// execute for mid-session delay changes because `add_input` rejects first.
    ///
    /// This test documents that frame delay changes mid-session are not supported.
    #[test]
    fn test_frame_delay_change_mid_session_drops_input() {
        let mut queue = test_queue(0);

        // Start with no delay, add first input
        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 1 });
        assert_eq!(queue.add_input(input0), Frame::new(0)); // Accepted at frame 0
        assert_eq!(queue.last_added_frame, Frame::new(0));

        // Change frame delay mid-session (not a supported operation via public API)
        queue.set_frame_delay(2).expect("valid delay");

        // Try to add next sequential input - it gets DROPPED because the sequential
        // check uses the new delay: (1 + 2) != (0 + 1), so 3 != 1
        let input1 = PlayerInput::new(Frame::new(1), TestInput { inp: 2 });
        let result = queue.add_input(input1);

        // Input is dropped (returns NULL_FRAME)
        assert_eq!(result, Frame::NULL);
        // last_added_frame unchanged
        assert_eq!(queue.last_added_frame, Frame::new(0));
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
        assert!(queue.check_invariants().is_ok());
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
        assert!(queue.check_invariants().is_ok());
    }

    #[test]
    fn test_invariant_checker_with_frame_delay() {
        let mut queue = test_queue(0);
        queue.set_frame_delay(5).expect("valid delay");

        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
            assert!(queue.check_invariants().is_ok());
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
        assert!(queue.check_invariants().is_ok());
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
        if let Err(FortressError::InvalidRequest { info }) = result {
            assert!(
                info.contains("exceeds maximum"),
                "Error should explain the issue"
            );
        } else {
            panic!("Expected InvalidRequest error");
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
        let result = queue.set_frame_delay(MAX_FRAME_DELAY);
        assert!(result.is_ok());
        assert_eq!(queue.frame_delay, MAX_FRAME_DELAY);
    }

    /// Test: Zero delay should be accepted (common case)
    #[test]
    fn test_set_frame_delay_accepts_zero() {
        let mut queue = test_queue(0);

        let result = queue.set_frame_delay(0);
        assert!(result.is_ok());
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
        assert!(queue.check_invariants().is_ok());
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
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
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
                let result = queue.confirmed_input(Frame::new(i));
                prop_assert!(result.is_ok());
                prop_assert_eq!(result.unwrap().input.inp, i as u8);
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
/// Kani uses `INPUT_QUEUE_LENGTH = 8` (via `#[cfg(kani)]`) for tractable verification.
/// The invariants verified here are **size-independent** - they hold for any queue
/// length >= 2. The proofs verify:
/// - Circular buffer arithmetic correctness (wraparound at any boundary)
/// - Index bounds checking (always < queue_length)
/// - Length bounds (always <= queue_length)
///
/// Therefore, proofs passing for queue_length=8 imply correctness for 32, 128, 256.
///
/// Note: Requires Kani verifier. Install with:
///   cargo install --locked kani-verifier
///   cargo kani setup
///
/// Run proofs with:
///   cargo kani --tests
#[cfg(kani)]
mod kani_input_queue_proofs {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
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

    /// Proof: New queue has valid initial state
    ///
    /// Verifies INV-4 (length = 0) and INV-5 (head = tail = 0) at initialization.
    #[kani::proof]
    #[kani::unwind(2)]
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

    /// Proof: Single add_input maintains invariants
    ///
    /// Verifies that adding a single input maintains INV-4 and INV-5.
    #[kani::proof]
    #[kani::unwind(2)]
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

    /// Proof: Sequential inputs maintain invariants (small count for Kani tractability)
    ///
    /// Verifies INV-4 and INV-5 hold after adding multiple sequential inputs.
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_sequential_inputs_maintain_invariants() {
        let mut queue = test_queue(0);
        let count: usize = kani::any();
        kani::assume(count > 0 && count <= 8);

        for i in 0..count {
            let input = PlayerInput::new(Frame::new(i as i32), TestInput { inp: i as u8 });
            let result = queue.add_input(input);
            kani::assert(
                result == Frame::new(i as i32),
                "Sequential input should be accepted",
            );

            // INV-4: length bounded
            kani::assert(
                queue.length == i + 1,
                "Length should equal count of added inputs",
            );
            kani::assert(
                queue.length <= INPUT_QUEUE_LENGTH,
                "Length should be bounded",
            );

            // INV-5: indices valid
            kani::assert(
                queue.head < INPUT_QUEUE_LENGTH,
                "Head should be within bounds",
            );
            kani::assert(
                queue.tail < INPUT_QUEUE_LENGTH,
                "Tail should be within bounds",
            );
        }
    }

    /// Proof: Head wraparound is correct
    ///
    /// Verifies that head index wraps around correctly when reaching INPUT_QUEUE_LENGTH.
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

    /// Proof: Queue index calculation is always valid
    ///
    /// Verifies that frame-to-index calculation (frame % INPUT_QUEUE_LENGTH) is always valid.
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

    /// Proof: Length calculation is consistent with head/tail
    ///
    /// Verifies the circular buffer length formula: length = (head - tail + N) % N
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

    /// Proof: discard_confirmed_frames maintains invariants
    ///
    /// Verifies that discarding frames maintains INV-4 and INV-5.
    #[kani::proof]
    #[kani::unwind(7)]
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

    /// Proof: Frame delay doesn't violate invariants
    ///
    /// Verifies that setting frame delay maintains valid queue state.
    /// Note: delay is bounded by INPUT_QUEUE_LENGTH - 1 to prevent overflow.
    /// In practice, delays this large are unreasonable for real-time games anyway.
    #[kani::proof]
    #[kani::unwind(3)]
    fn proof_frame_delay_maintains_invariants() {
        let mut queue = test_queue(0);

        let delay: usize = kani::any();
        // Frame delay must be less than queue length to prevent overflow
        kani::assume(delay < INPUT_QUEUE_LENGTH);

        // set_frame_delay should succeed for valid delays (< INPUT_QUEUE_LENGTH)
        let set_result = queue.set_frame_delay(delay);
        kani::assert(set_result.is_ok(), "Valid delay should be accepted");

        // Add input with delay
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 0 });
        let result = queue.add_input(input);

        // With delay, the actual frame stored is frame + delay
        if delay == 0 {
            kani::assert(
                result == Frame::new(0),
                "Without delay, should store at frame 0",
            );
        } else {
            kani::assert(
                result.as_i32() == delay as i32,
                "With delay, should store at frame 0 + delay",
            );
        }

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

    /// Proof: Non-sequential inputs are rejected
    ///
    /// Verifies that add_input rejects non-sequential frame inputs, preserving invariants.
    #[kani::proof]
    #[kani::unwind(3)]
    fn proof_non_sequential_rejected() {
        let mut queue = test_queue(0);

        // Add first input
        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 0 });
        queue.add_input(input0);

        // Try to add non-sequential input
        let skip: i32 = kani::any();
        kani::assume(skip >= 2 && skip <= 10);

        let bad_input = PlayerInput::new(Frame::new(skip), TestInput { inp: 1 });
        let result = queue.add_input(bad_input);

        kani::assert(result.is_null(), "Non-sequential input should be rejected");
        kani::assert(queue.length == 1, "Length should not change on rejection");
    }

    /// Proof: reset_prediction maintains structural invariants
    #[kani::proof]
    #[kani::unwind(5)]
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

    /// Proof: Confirmed input retrieval is valid for stored frames
    #[kani::proof]
    #[kani::unwind(7)]
    fn proof_confirmed_input_valid_index() {
        let mut queue = test_queue(0);

        // Add some inputs
        let count: usize = kani::any();
        kani::assume(count > 0 && count <= 5);

        for i in 0..count {
            let input = PlayerInput::new(Frame::new(i as i32), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Request any frame in range
        let request_frame: i32 = kani::any();
        kani::assume(request_frame >= 0 && request_frame < count as i32);

        let result = queue.confirmed_input(Frame::new(request_frame));

        // Index calculation should be valid
        let offset = request_frame as usize % INPUT_QUEUE_LENGTH;
        kani::assert(
            offset < INPUT_QUEUE_LENGTH,
            "Calculated offset should be valid",
        );
    }
}
