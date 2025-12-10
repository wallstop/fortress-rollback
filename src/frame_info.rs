use crate::Frame;

/// Represents the game state of your game for a single frame. The `data` holds the game state, `frame` indicates the associated frame number
/// and `checksum` can additionally be provided for use during a `SyncTestSession`.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
#[derive(Debug, Clone)]
pub struct GameState<S> {
    /// The frame to which this info belongs to.
    pub frame: Frame,
    /// The game state
    pub data: Option<S>,
    /// The checksum of the gamestate.
    pub checksum: Option<u128>,
}

impl<S> Default for GameState<S> {
    fn default() -> Self {
        Self {
            frame: Frame::NULL,
            data: None,
            checksum: None,
        }
    }
}

/// Represents an input for a single player in a single frame. The associated frame is denoted with `frame`.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PlayerInput<I>
where
    I: Copy + Clone + PartialEq,
{
    /// The frame to which this info belongs to. [`Frame::NULL`] represents an invalid frame
    pub frame: Frame,
    /// The input struct given by the user
    pub input: I,
}

impl<I: Copy + Clone + PartialEq + Default> PlayerInput<I> {
    /// Creates a new `PlayerInput` with the given frame and input.
    pub fn new(frame: Frame, input: I) -> Self {
        Self { frame, input }
    }

    /// Creates a blank input with the default value for the input type.
    #[must_use]
    pub fn blank_input(frame: Frame) -> Self {
        Self {
            frame,
            input: I::default(),
        }
    }

    pub(crate) fn equal(&self, other: &Self, input_only: bool) -> bool {
        (input_only || self.frame == other.frame) && self.input == other.input
    }
}

// #########
// # TESTS #
// #########

#[cfg(test)]
mod game_input_tests {
    use super::*;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default)]
    struct TestInput {
        inp: u8,
    }

    #[test]
    fn test_input_equality() {
        let input1 = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        let input2 = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        assert!(input1.equal(&input2, false));
    }

    #[test]
    fn test_input_equality_input_only() {
        let input1 = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        let input2 = PlayerInput::new(Frame::new(5), TestInput { inp: 5 });
        assert!(input1.equal(&input2, true)); // different frames, but does not matter
    }

    #[test]
    fn test_input_equality_fail() {
        let input1 = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        let input2 = PlayerInput::new(Frame::new(0), TestInput { inp: 7 });
        assert!(!input1.equal(&input2, false)); // different bits
    }
}
