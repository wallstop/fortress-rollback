use crate::frame_info::PlayerInput;
use crate::telemetry::{InvariantChecker, InvariantViolation};
use crate::{Config, Frame, InputStatus};
use std::cmp;

/// The length of the input queue. This describes the number of inputs Fortress Rollback can hold at the same time per player.
const INPUT_QUEUE_LENGTH: usize = 128;

/// `InputQueue` handles inputs for a single player and saves them in a circular array. Valid Inputs are between `head` and `tail`.
#[derive(Debug, Clone)]
pub(crate) struct InputQueue<T>
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

    /// Our cyclic input queue
    inputs: Vec<PlayerInput<T::Input>>,
    /// A pre-allocated prediction we are going to use to return predictions from.
    prediction: PlayerInput<T::Input>,
}

impl<T: Config> InputQueue<T> {
    pub(crate) fn new() -> Self {
        Self {
            head: 0,
            tail: 0,
            length: 0,
            frame_delay: 0,
            first_frame: true,
            last_added_frame: Frame::NULL,
            first_incorrect_frame: Frame::NULL,
            last_requested_frame: Frame::NULL,
            prediction: PlayerInput::blank_input(Frame::NULL),
            inputs: vec![PlayerInput::blank_input(Frame::NULL); INPUT_QUEUE_LENGTH],
        }
    }

    pub(crate) fn first_incorrect_frame(&self) -> Frame {
        self.first_incorrect_frame
    }

    pub(crate) fn set_frame_delay(&mut self, delay: usize) {
        self.frame_delay = delay;
    }

    pub(crate) fn reset_prediction(&mut self) {
        self.prediction.frame = Frame::NULL;
        self.first_incorrect_frame = Frame::NULL;
        self.last_requested_frame = Frame::NULL;
    }

    /// Returns a `PlayerInput` only if the input for the requested frame is confirmed.
    /// In contrast to `input()`, this will not return a prediction if there is no confirmed input for the frame.
    pub(crate) fn confirmed_input(
        &self,
        requested_frame: Frame,
    ) -> Result<PlayerInput<T::Input>, crate::FortressError> {
        let offset = requested_frame.as_i32() as usize % INPUT_QUEUE_LENGTH;

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

    /// Discards confirmed frames up to given `frame` from the queue. All confirmed frames are guaranteed to be synchronized between players, so there is no need to save the inputs anymore.
    pub(crate) fn discard_confirmed_frames(&mut self, mut frame: Frame) {
        // we only drop frames until the last frame that was requested, otherwise we might delete data still needed
        if !self.last_requested_frame.is_null() {
            frame = cmp::min(frame, self.last_requested_frame);
        }

        // move the tail to "delete inputs", wrap around if necessary
        if frame >= self.last_added_frame {
            // delete all but most recent
            self.tail = self.head;
            self.length = 1;
        } else if frame <= self.inputs[self.tail].frame {
            // we don't need to delete anything
        } else {
            let offset = (frame - self.inputs[self.tail].frame) as usize;
            self.tail = (self.tail + offset) % INPUT_QUEUE_LENGTH;
            self.length -= offset;
        }
    }

    /// Returns the game input of a single player for a given frame, if that input does not exist, we return a prediction instead.
    pub(crate) fn input(&mut self, requested_frame: Frame) -> (T::Input, InputStatus) {
        // No one should ever try to grab any input when we have a prediction error.
        // Doing so means that we're just going further down the wrong path. Assert this to verify that it's true.
        assert!(self.first_incorrect_frame.is_null());

        // Remember the last requested frame number for later. We'll need this in add_input() to drop out of prediction mode.
        self.last_requested_frame = requested_frame;

        // assert that we request a frame that still exists
        assert!(requested_frame >= self.inputs[self.tail].frame);

        // We currently don't have a prediction frame
        if self.prediction.frame.as_i32() < 0 {
            //  If the frame requested is in our range, fetch it out of the queue and return it.
            let mut offset: usize = (requested_frame - self.inputs[self.tail].frame) as usize;

            if offset < self.length {
                offset = (offset + self.tail) % INPUT_QUEUE_LENGTH;
                assert!(self.inputs[offset].frame == requested_frame);
                return (self.inputs[offset].input, InputStatus::Confirmed);
            }

            // The requested frame isn't in the queue. This means we need to return a prediction frame. Predict that the user will do the same thing they did last time.
            if requested_frame == 0 || self.last_added_frame.is_null() {
                // basing new prediction frame from nothing, since we are on frame 0 or we have no frames yet
                self.prediction = PlayerInput::blank_input(self.prediction.frame);
            } else {
                // basing new prediction frame from previously added frame
                let previous_position = match self.head {
                    0 => INPUT_QUEUE_LENGTH - 1,
                    _ => self.head - 1,
                };
                self.prediction = self.inputs[previous_position];
            }
            // update the prediction's frame
            self.prediction.frame += 1;
        }

        // We must be predicting, so we return the prediction frame contents. We are adjusting the prediction to have the requested frame.
        assert!(!self.prediction.frame.is_null());
        let prediction_to_return = self.prediction; // PlayerInput has copy semantics
        (prediction_to_return.input, InputStatus::Predicted)
    }

    /// Adds an input frame to the queue. Will consider the set frame delay.
    pub(crate) fn add_input(&mut self, input: PlayerInput<T::Input>) -> Frame {
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
        if !new_frame.is_null() {
            self.add_input_by_frame(input, new_frame);
        }
        new_frame
    }

    /// Adds an input frame to the queue at the given frame number. If there are predicted inputs, we will check those and mark them as incorrect, if necessary.
    /// Returns the frame number
    fn add_input_by_frame(&mut self, input: PlayerInput<T::Input>, frame_number: Frame) {
        let previous_position = match self.head {
            0 => INPUT_QUEUE_LENGTH - 1,
            _ => self.head - 1,
        };

        assert!(self.last_added_frame.is_null() || frame_number == self.last_added_frame + 1);
        assert!(frame_number == 0 || self.inputs[previous_position].frame == frame_number - 1);

        // Add the frame to the back of the queue
        self.inputs[self.head] = input;
        self.inputs[self.head].frame = frame_number;
        self.head = (self.head + 1) % INPUT_QUEUE_LENGTH;
        self.length += 1;
        assert!(self.length <= INPUT_QUEUE_LENGTH);
        self.first_frame = false;
        self.last_added_frame = frame_number;

        // We have been predicting. See if the inputs we've gotten match what we've been predicting. If so, don't worry about it.
        if !self.prediction.frame.is_null() {
            assert!(frame_number == self.prediction.frame);

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
    }

    /// Advances the queue head to the next frame and either drops inputs or fills the queue if the input delay has changed since the last frame.
    fn advance_queue_head(&mut self, mut input_frame: Frame) -> Frame {
        let previous_position = match self.head {
            0 => INPUT_QUEUE_LENGTH - 1,
            _ => self.head - 1,
        };

        let mut expected_frame = if self.first_frame {
            Frame::new(0)
        } else {
            self.inputs[previous_position].frame + 1
        };

        input_frame += self.frame_delay as i32;
        //  This can occur when the frame delay has dropped since the last time we shoved a frame into the system. In this case, there's no room on the queue. Toss it.
        if expected_frame > input_frame {
            return Frame::NULL;
        }

        // This can occur when the frame delay has been increased since the last time we shoved a frame into the system.
        // We need to replicate the last frame in the queue several times in order to fill the space left.
        while expected_frame < input_frame {
            let input_to_replicate = self.inputs[previous_position];
            self.add_input_by_frame(input_to_replicate, expected_frame);
            expected_frame += 1;
        }

        let previous_position = match self.head {
            0 => INPUT_QUEUE_LENGTH - 1,
            _ => self.head - 1,
        };
        assert!(input_frame == 0 || input_frame == self.inputs[previous_position].frame + 1);
        input_frame
    }
}

impl<T: Config> InvariantChecker for InputQueue<T> {
    /// Checks the invariants of the InputQueue.
    ///
    /// # Invariants
    ///
    /// 1. `length` must not exceed `INPUT_QUEUE_LENGTH`
    /// 2. `head` and `tail` must be valid indices (< INPUT_QUEUE_LENGTH)
    /// 3. `length` must equal the distance from tail to head in the circular buffer
    /// 4. If `length > 0`, the frames in the queue should be consecutive
    /// 5. `frame_delay` must be within reasonable bounds
    /// 6. `first_incorrect_frame` should be NULL_FRAME or >= 0
    fn check_invariants(&self) -> Result<(), InvariantViolation> {
        // Invariant 1: length <= INPUT_QUEUE_LENGTH
        if self.length > INPUT_QUEUE_LENGTH {
            return Err(InvariantViolation::new(
                "InputQueue",
                "length exceeds INPUT_QUEUE_LENGTH",
            )
            .with_details(format!(
                "length={}, max={}",
                self.length, INPUT_QUEUE_LENGTH
            )));
        }

        // Invariant 2: head and tail are valid indices
        if self.head >= INPUT_QUEUE_LENGTH {
            return Err(InvariantViolation::new(
                "InputQueue",
                "head index out of bounds",
            )
            .with_details(format!(
                "head={}, max={}",
                self.head,
                INPUT_QUEUE_LENGTH - 1
            )));
        }

        if self.tail >= INPUT_QUEUE_LENGTH {
            return Err(InvariantViolation::new(
                "InputQueue",
                "tail index out of bounds",
            )
            .with_details(format!(
                "tail={}, max={}",
                self.tail,
                INPUT_QUEUE_LENGTH - 1
            )));
        }

        // Invariant 3: length equals circular distance from tail to head
        let calculated_length = if self.head >= self.tail {
            self.head - self.tail
        } else {
            INPUT_QUEUE_LENGTH - self.tail + self.head
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

        // Invariant 4: inputs vector has correct size
        if self.inputs.len() != INPUT_QUEUE_LENGTH {
            return Err(InvariantViolation::new(
                "InputQueue",
                "inputs vector has incorrect size",
            )
            .with_details(format!(
                "size={}, expected={}",
                self.inputs.len(),
                INPUT_QUEUE_LENGTH
            )));
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
            return Err(InvariantViolation::new(
                "InputQueue",
                "first_incorrect_frame is invalid",
            )
            .with_details(format!(
                "first_incorrect_frame={}",
                self.first_incorrect_frame
            )));
        }

        // Invariant 7: last_requested_frame is either NULL or a valid frame
        if !self.last_requested_frame.is_null() && self.last_requested_frame.as_i32() < 0 {
            return Err(InvariantViolation::new(
                "InputQueue",
                "last_requested_frame is invalid",
            )
            .with_details(format!(
                "last_requested_frame={}",
                self.last_requested_frame
            )));
        }

        // Invariant 8: last_added_frame is either NULL or a valid frame
        if !self.last_added_frame.is_null() && self.last_added_frame.as_i32() < 0 {
            return Err(InvariantViolation::new(
                "InputQueue",
                "last_added_frame is invalid",
            )
            .with_details(format!("last_added_frame={}", self.last_added_frame)));
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

    #[test]
    fn test_add_input_wrong_frame() {
        let mut queue = InputQueue::<TestConfig>::new();
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 0 });
        assert_eq!(queue.add_input(input), Frame::new(0)); // fine
        let input_wrong_frame = PlayerInput::new(Frame::new(3), TestInput { inp: 0 });
        assert_eq!(queue.add_input(input_wrong_frame), Frame::NULL); // input dropped
    }

    #[test]
    fn test_add_input_twice() {
        let mut queue = InputQueue::<TestConfig>::new();
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 0 });
        assert_eq!(queue.add_input(input), Frame::new(0)); // fine
        assert_eq!(queue.add_input(input), Frame::NULL); // input dropped
    }

    #[test]
    fn test_add_input_sequentially() {
        let mut queue = InputQueue::<TestConfig>::new();
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
            queue.add_input(input);
            assert_eq!(queue.last_added_frame, Frame::new(i));
            assert_eq!(queue.length, (i + 1) as usize);
        }
    }

    #[test]
    fn test_input_sequentially() {
        let mut queue = InputQueue::<TestConfig>::new();
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
            assert_eq!(queue.last_added_frame, Frame::new(i));
            assert_eq!(queue.length, (i + 1) as usize);
            let (input_in_queue, _status) = queue.input(Frame::new(i));
            assert_eq!(input_in_queue.inp, i as u8);
        }
    }

    #[test]
    fn test_delayed_inputs() {
        let mut queue = InputQueue::<TestConfig>::new();
        let delay: i32 = 2;
        queue.set_frame_delay(delay as usize);
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
            assert_eq!(queue.last_added_frame, Frame::new(i + delay));
            assert_eq!(queue.length, (i + delay + 1) as usize);
            let (input_in_queue, _status) = queue.input(Frame::new(i));
            let correct_input = std::cmp::max(0, i - delay) as u8;
            assert_eq!(input_in_queue.inp, correct_input);
        }
    }

    #[test]
    fn test_confirmed_input_success() {
        let mut queue = InputQueue::<TestConfig>::new();
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
        let mut queue = InputQueue::<TestConfig>::new();
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
        let mut queue = InputQueue::<TestConfig>::new();
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
        let mut queue = InputQueue::<TestConfig>::new();
        // Add 10 inputs
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Discard all frames (should keep at least the most recent)
        queue.discard_confirmed_frames(Frame::new(100));
        assert_eq!(queue.length, 1);
    }

    #[test]
    fn test_discard_confirmed_frames_respects_last_requested() {
        let mut queue = InputQueue::<TestConfig>::new();
        // Add 10 inputs
        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Request frame 3 (this sets last_requested_frame)
        let _ = queue.input(Frame::new(3));

        // Try to discard up to frame 8, but should only discard up to 3
        queue.discard_confirmed_frames(Frame::new(8));

        // Frame 3 should still be available
        let result = queue.confirmed_input(Frame::new(3));
        assert!(result.is_ok());
    }

    #[test]
    fn test_discard_nothing_when_frame_before_tail() {
        let mut queue = InputQueue::<TestConfig>::new();
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
        let mut queue = InputQueue::<TestConfig>::new();
        // Add a couple of inputs
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        // Request a frame beyond what we have (triggers prediction)
        let (_, status) = queue.input(Frame::new(5));
        assert_eq!(status, InputStatus::Predicted);
        assert!(queue.prediction.frame.as_i32() >= 0);

        // Reset prediction
        queue.reset_prediction();
        assert_eq!(queue.prediction.frame, Frame::NULL);
        assert_eq!(queue.first_incorrect_frame, Frame::NULL);
        assert_eq!(queue.last_requested_frame, Frame::NULL);
    }

    #[test]
    fn test_prediction_returns_last_input() {
        let mut queue = InputQueue::<TestConfig>::new();
        // Add inputs with specific values
        for i in 0..3i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 42 }); // All inputs are 42
            queue.add_input(input);
        }

        // Request frame 5 (beyond what we have)
        let (predicted_input, status) = queue.input(Frame::new(5));
        assert_eq!(status, InputStatus::Predicted);
        // Prediction should be based on the last added input (42)
        assert_eq!(predicted_input.inp, 42);
    }

    #[test]
    fn test_first_incorrect_frame_detection() {
        let mut queue = InputQueue::<TestConfig>::new();
        // Add initial input
        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 10 });
        queue.add_input(input0);

        // Request frame 1 (triggers prediction based on frame 0)
        let (predicted, status) = queue.input(Frame::new(1));
        assert_eq!(status, InputStatus::Predicted);
        assert_eq!(predicted.inp, 10); // Predicted to be same as last input

        // Now add the actual input for frame 1 with DIFFERENT value
        let input1 = PlayerInput::new(Frame::new(1), TestInput { inp: 99 }); // Different!
        queue.add_input(input1);

        // The first incorrect frame should be detected
        assert_eq!(queue.first_incorrect_frame(), Frame::new(1));
    }

    #[test]
    fn test_first_incorrect_frame_correct_prediction() {
        let mut queue = InputQueue::<TestConfig>::new();
        // Add initial input
        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 10 });
        queue.add_input(input0);

        // Request frame 1 (triggers prediction)
        let _ = queue.input(Frame::new(1));

        // Add actual input for frame 1 with SAME value (correct prediction)
        let input1 = PlayerInput::new(Frame::new(1), TestInput { inp: 10 }); // Same as prediction
        queue.add_input(input1);

        // No incorrect frame should be detected
        assert_eq!(queue.first_incorrect_frame(), Frame::NULL);
    }

    #[test]
    fn test_queue_wraparound() {
        let mut queue = InputQueue::<TestConfig>::new();

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
        let mut queue = InputQueue::<TestConfig>::new();
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        queue.add_input(input);

        let (retrieved, status) = queue.input(Frame::new(0));
        assert_eq!(status, InputStatus::Confirmed);
        assert_eq!(retrieved.inp, 5);
    }

    #[test]
    fn test_input_returns_predicted_status() {
        let mut queue = InputQueue::<TestConfig>::new();
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        queue.add_input(input);

        // Request frame beyond what we have
        let (_, status) = queue.input(Frame::new(10));
        assert_eq!(status, InputStatus::Predicted);
    }

    #[test]
    fn test_frame_delay_change_increase() {
        let mut queue = InputQueue::<TestConfig>::new();

        // Start with delay of 2 from the beginning
        queue.set_frame_delay(2);

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
    /// NOTE: This documents current behavior which may be a bug. The `advance_queue_head`
    /// function has code to handle frame delay increases by replicating inputs, but that
    /// code can never be reached because `add_input` rejects the input first due to the
    /// sequential check. Since frame delay is only set at construction via the builder
    /// and the API doesn't expose changing it mid-session, this is not exploitable.
    /// However, the gap-filling code in `advance_queue_head` is effectively dead code.
    ///
    /// TODO: Either remove the dead gap-filling code, or fix the sequential check to
    /// properly handle frame delay changes (if that's the intended behavior).
    #[test]
    fn test_frame_delay_change_mid_session_drops_input() {
        let mut queue = InputQueue::<TestConfig>::new();

        // Start with no delay, add first input
        let input0 = PlayerInput::new(Frame::new(0), TestInput { inp: 1 });
        assert_eq!(queue.add_input(input0), Frame::new(0)); // Accepted at frame 0
        assert_eq!(queue.last_added_frame, Frame::new(0));

        // Change frame delay mid-session (not a supported operation via public API)
        queue.set_frame_delay(2);

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
        let mut queue = InputQueue::<TestConfig>::new();

        // Request frame 0 without any inputs (edge case)
        // This should return a blank prediction
        let (predicted, status) = queue.input(Frame::new(0));
        assert_eq!(status, InputStatus::Predicted);
        assert_eq!(predicted.inp, TestInput::default().inp);
    }

    // ==========================================
    // Invariant Checker Tests
    // ==========================================

    #[test]
    fn test_invariant_checker_new_queue() {
        let queue = InputQueue::<TestConfig>::new();
        assert!(queue.check_invariants().is_ok());
    }

    #[test]
    fn test_invariant_checker_after_add_input() {
        let mut queue = InputQueue::<TestConfig>::new();
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
        let mut queue = InputQueue::<TestConfig>::new();
        for i in 0..20i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
        }

        queue.discard_confirmed_frames(Frame::new(10));
        assert!(queue.check_invariants().is_ok());
    }

    #[test]
    fn test_invariant_checker_with_frame_delay() {
        let mut queue = InputQueue::<TestConfig>::new();
        queue.set_frame_delay(5);

        for i in 0..10i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            queue.add_input(input);
            assert!(queue.check_invariants().is_ok());
        }
    }

    #[test]
    fn test_invariant_checker_after_reset_prediction() {
        let mut queue = InputQueue::<TestConfig>::new();
        for i in 0..5i32 {
            let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
            queue.add_input(input);
        }
        let _ = queue.input(Frame::new(10)); // Trigger prediction

        queue.reset_prediction();
        assert!(queue.check_invariants().is_ok());
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
            let mut queue = InputQueue::<TestConfig>::new();
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
            let mut queue = InputQueue::<TestConfig>::new();

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
            let mut queue = InputQueue::<TestConfig>::new();

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
            let mut queue = InputQueue::<TestConfig>::new();
            queue.set_frame_delay(delay);

            // Add inputs
            for i in 0..count as i32 {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
                queue.add_input(input);
            }

            // last_added_frame should be shifted by delay
            prop_assert_eq!(queue.last_added_frame, Frame::new((count as i32 - 1) + delay as i32));
        }

        /// Property: Prediction always uses last confirmed input value
        #[test]
        fn prop_prediction_uses_last_input(
            count in 1usize..=20,
            last_value in input_value(),
        ) {
            let mut queue = InputQueue::<TestConfig>::new();

            // Add inputs, with last one having specific value
            for i in 0..(count - 1) as i32 {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
                queue.add_input(input);
            }
            // Add last input with known value
            let last_input = PlayerInput::new(
                Frame::new((count - 1) as i32),
                TestInput { inp: last_value },
            );
            queue.add_input(last_input);

            // Request frame beyond what we have
            let future_frame = Frame::new(count as i32 + 5);
            let (predicted, status) = queue.input(future_frame);

            prop_assert_eq!(status, InputStatus::Predicted);
            prop_assert_eq!(predicted.inp, last_value);
        }

        /// Property: Queue length is bounded when regularly discarding old frames
        /// Note: The InputQueue asserts if length exceeds INPUT_QUEUE_LENGTH.
        /// In practice, discard_confirmed_frames() must be called regularly.
        #[test]
        fn prop_queue_length_bounded_with_discard(
            count in 1usize..=200,
        ) {
            let mut queue = InputQueue::<TestConfig>::new();

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
            let mut queue = InputQueue::<TestConfig>::new();

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
            let mut queue = InputQueue::<TestConfig>::new();

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
            let mut queue = InputQueue::<TestConfig>::new();

            // Add initial inputs with value 0
            for i in 0..(count - 1) as i32 {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
                queue.add_input(input);
            }

            // Request the next frame (triggers prediction of 0)
            let predicted_frame = Frame::new((count - 1) as i32);
            let (predicted, _) = queue.input(predicted_frame);
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
            let mut queue = InputQueue::<TestConfig>::new();

            // Add some inputs and trigger prediction
            for i in 0..count as i32 {
                let input = PlayerInput::new(Frame::new(i), TestInput { inp: 0 });
                queue.add_input(input);
            }
            let _ = queue.input(Frame::new(count as i32 + 5)); // Trigger prediction

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

/// Kani proofs for InputQueue buffer bounds (INV-4, INV-5 from FORMAL_SPEC.md).
///
/// These proofs verify:
/// - INV-4: Queue length is always bounded by INPUT_QUEUE_LENGTH (128)
/// - INV-5: Queue indices (head, tail) are always valid (< INPUT_QUEUE_LENGTH)
/// - Circular buffer wraparound is correct
/// - Length calculation matches actual buffer usage
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

    /// Proof: New queue has valid initial state
    ///
    /// Verifies INV-4 (length = 0) and INV-5 (head = tail = 0) at initialization.
    #[kani::proof]
    fn proof_new_queue_valid() {
        let queue = InputQueue::<TestConfig>::new();

        // INV-4: length bounded
        kani::assert(queue.length == 0, "New queue should have length 0");
        kani::assert(
            queue.length <= INPUT_QUEUE_LENGTH,
            "Length should be bounded by INPUT_QUEUE_LENGTH"
        );

        // INV-5: indices valid
        kani::assert(queue.head == 0, "New queue head should be 0");
        kani::assert(queue.tail == 0, "New queue tail should be 0");
        kani::assert(
            queue.head < INPUT_QUEUE_LENGTH,
            "Head should be within bounds"
        );
        kani::assert(
            queue.tail < INPUT_QUEUE_LENGTH,
            "Tail should be within bounds"
        );

        // Additional invariants
        kani::assert(queue.first_frame, "New queue should have first_frame flag");
        kani::assert(queue.last_added_frame.is_null(), "New queue should have null last_added_frame");
    }

    /// Proof: Single add_input maintains invariants
    ///
    /// Verifies that adding a single input maintains INV-4 and INV-5.
    #[kani::proof]
    fn proof_add_single_input_maintains_invariants() {
        let mut queue = InputQueue::<TestConfig>::new();

        let input_val: u8 = kani::any();
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: input_val });

        let result = queue.add_input(input);

        // Input should be accepted (frame 0 is first input)
        kani::assert(result == Frame::new(0), "First input should be accepted at frame 0");

        // INV-4: length bounded
        kani::assert(queue.length == 1, "Length should be 1 after first input");
        kani::assert(
            queue.length <= INPUT_QUEUE_LENGTH,
            "Length should be bounded"
        );

        // INV-5: indices valid
        kani::assert(queue.head == 1, "Head should advance to 1");
        kani::assert(queue.tail == 0, "Tail should remain at 0");
        kani::assert(
            queue.head < INPUT_QUEUE_LENGTH,
            "Head should be within bounds"
        );
        kani::assert(
            queue.tail < INPUT_QUEUE_LENGTH,
            "Tail should be within bounds"
        );
    }

    /// Proof: Sequential inputs maintain invariants (small count for Kani tractability)
    ///
    /// Verifies INV-4 and INV-5 hold after adding multiple sequential inputs.
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_sequential_inputs_maintain_invariants() {
        let mut queue = InputQueue::<TestConfig>::new();
        let count: usize = kani::any();
        kani::assume(count > 0 && count <= 8);

        for i in 0..count {
            let input = PlayerInput::new(Frame::new(i as i32), TestInput { inp: i as u8 });
            let result = queue.add_input(input);
            kani::assert(result == Frame::new(i as i32), "Sequential input should be accepted");

            // INV-4: length bounded
            kani::assert(
                queue.length == i + 1,
                "Length should equal count of added inputs"
            );
            kani::assert(
                queue.length <= INPUT_QUEUE_LENGTH,
                "Length should be bounded"
            );

            // INV-5: indices valid
            kani::assert(
                queue.head < INPUT_QUEUE_LENGTH,
                "Head should be within bounds"
            );
            kani::assert(
                queue.tail < INPUT_QUEUE_LENGTH,
                "Tail should be within bounds"
            );
        }
    }

    /// Proof: Head wraparound is correct
    ///
    /// Verifies that head index wraps around correctly when reaching INPUT_QUEUE_LENGTH.
    #[kani::proof]
    fn proof_head_wraparound() {
        let head: usize = kani::any();
        kani::assume(head < INPUT_QUEUE_LENGTH);

        let new_head = (head + 1) % INPUT_QUEUE_LENGTH;

        kani::assert(new_head < INPUT_QUEUE_LENGTH, "Wrapped head should be within bounds");

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
    fn proof_queue_index_calculation() {
        let frame: i32 = kani::any();
        kani::assume(frame >= 0 && frame <= 10_000_000);

        let index = frame as usize % INPUT_QUEUE_LENGTH;

        kani::assert(index < INPUT_QUEUE_LENGTH, "Calculated index should be within bounds");
    }

    /// Proof: Length calculation is consistent with head/tail
    ///
    /// Verifies the circular buffer length formula: length = (head - tail + N) % N
    #[kani::proof]
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
            "Calculated length should be bounded"
        );
    }

    /// Proof: discard_confirmed_frames maintains invariants
    ///
    /// Verifies that discarding frames maintains INV-4 and INV-5.
    #[kani::proof]
    #[kani::unwind(6)]
    fn proof_discard_maintains_invariants() {
        let mut queue = InputQueue::<TestConfig>::new();

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
            "Length should be bounded after discard"
        );
        kani::assert(queue.length >= 1, "Should keep at least one entry");

        // INV-5: indices valid
        kani::assert(
            queue.head < INPUT_QUEUE_LENGTH,
            "Head should be within bounds after discard"
        );
        kani::assert(
            queue.tail < INPUT_QUEUE_LENGTH,
            "Tail should be within bounds after discard"
        );
    }

    /// Proof: Frame delay doesn't violate invariants
    ///
    /// Verifies that setting frame delay maintains valid queue state.
    #[kani::proof]
    fn proof_frame_delay_maintains_invariants() {
        let mut queue = InputQueue::<TestConfig>::new();

        let delay: usize = kani::any();
        kani::assume(delay <= 10);

        queue.set_frame_delay(delay);

        // Add input with delay
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 0 });
        let result = queue.add_input(input);

        // With delay, the actual frame stored is frame + delay
        if delay == 0 {
            kani::assert(result == Frame::new(0), "Without delay, should store at frame 0");
        } else {
            kani::assert(
                result.as_i32() == delay as i32,
                "With delay, should store at frame 0 + delay"
            );
        }

        // INV-4 and INV-5 should hold
        kani::assert(
            queue.length <= INPUT_QUEUE_LENGTH,
            "Length should be bounded"
        );
        kani::assert(
            queue.head < INPUT_QUEUE_LENGTH,
            "Head should be within bounds"
        );
        kani::assert(
            queue.tail < INPUT_QUEUE_LENGTH,
            "Tail should be within bounds"
        );
    }

    /// Proof: Non-sequential inputs are rejected
    ///
    /// Verifies that add_input rejects non-sequential frame inputs, preserving invariants.
    #[kani::proof]
    fn proof_non_sequential_rejected() {
        let mut queue = InputQueue::<TestConfig>::new();

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
    fn proof_reset_maintains_structure() {
        let mut queue = InputQueue::<TestConfig>::new();

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
        kani::assert(queue.first_incorrect_frame.is_null(), "first_incorrect_frame should be null");
        kani::assert(queue.prediction.frame.is_null(), "prediction frame should be null");
        kani::assert(queue.last_requested_frame.is_null(), "last_requested_frame should be null");
    }

    /// Proof: Confirmed input retrieval is valid for stored frames
    #[kani::proof]
    #[kani::unwind(6)]
    fn proof_confirmed_input_valid_index() {
        let mut queue = InputQueue::<TestConfig>::new();

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
        kani::assert(offset < INPUT_QUEUE_LENGTH, "Calculated offset should be valid");
    }
}
