use crate::Frame;

/// Represents the game state of your game for a single frame.
///
/// The `data` holds the game state, `frame` indicates the associated frame number
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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod game_state_tests {
    use super::*;

    // ==========================================
    // GameState Tests
    // ==========================================

    #[test]
    fn default_state_has_null_frame() {
        let state: GameState<u32> = GameState::default();
        assert_eq!(state.frame, Frame::NULL);
        assert!(state.data.is_none());
        assert!(state.checksum.is_none());
    }

    #[test]
    fn game_state_stores_frame() {
        let state = GameState {
            frame: Frame::new(42),
            data: Some(123u32),
            checksum: None,
        };
        assert_eq!(state.frame, Frame::new(42));
    }

    #[test]
    fn game_state_stores_data() {
        let state = GameState {
            frame: Frame::new(0),
            data: Some("test state".to_string()),
            checksum: None,
        };
        assert_eq!(state.data, Some("test state".to_string()));
    }

    #[test]
    fn game_state_stores_checksum() {
        let state = GameState {
            frame: Frame::new(0),
            data: Some(0u8),
            checksum: Some(0xDEAD_BEEF),
        };
        assert_eq!(state.checksum, Some(0xDEAD_BEEF));
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn game_state_clone_preserves_all_fields() {
        let state = GameState {
            frame: Frame::new(100),
            data: Some(vec![1, 2, 3]),
            checksum: Some(12345),
        };
        let cloned = state.clone();
        assert_eq!(cloned.frame, Frame::new(100));
        assert_eq!(cloned.data, Some(vec![1, 2, 3]));
        assert_eq!(cloned.checksum, Some(12345));
    }

    #[test]
    fn game_state_debug_format() {
        let state = GameState {
            frame: Frame::new(5),
            data: Some(42u32),
            checksum: Some(100),
        };
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("frame"));
        assert!(debug_str.contains("data"));
        assert!(debug_str.contains("checksum"));
    }

    #[test]
    fn game_state_with_none_data() {
        let state: GameState<String> = GameState {
            frame: Frame::new(10),
            data: None,
            checksum: Some(999),
        };
        assert!(state.data.is_none());
        assert_eq!(state.checksum, Some(999));
    }

    #[test]
    fn game_state_max_frame_value() {
        let state = GameState {
            frame: Frame::new(i32::MAX),
            data: Some(0u8),
            checksum: None,
        };
        assert_eq!(state.frame, Frame::new(i32::MAX));
    }

    #[test]
    fn game_state_max_checksum_value() {
        let state = GameState {
            frame: Frame::new(0),
            data: Some(0u8),
            checksum: Some(u128::MAX),
        };
        assert_eq!(state.checksum, Some(u128::MAX));
    }

    #[test]
    fn game_state_complex_data_type() {
        #[derive(Clone, Debug, PartialEq)]
        struct ComplexState {
            position: (f32, f32),
            velocity: (f32, f32),
            health: i32,
        }

        let state = GameState {
            frame: Frame::new(50),
            data: Some(ComplexState {
                position: (1.5, 2.5),
                velocity: (-0.5, 0.0),
                health: 100,
            }),
            checksum: Some(0xCAFEBABE),
        };

        assert_eq!(state.frame, Frame::new(50));
        assert_eq!(state.data.as_ref().unwrap().health, 100);
        assert_eq!(state.checksum, Some(0xCAFEBABE));
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod player_input_tests {
    use super::*;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default, Debug)]
    struct TestInput {
        inp: u8,
    }

    // ==========================================
    // Constructor Tests
    // ==========================================

    #[test]
    fn new_creates_player_input() {
        let input = PlayerInput::new(Frame::new(5), TestInput { inp: 42 });
        assert_eq!(input.frame, Frame::new(5));
        assert_eq!(input.input.inp, 42);
    }

    #[test]
    fn blank_input_uses_default() {
        let input = PlayerInput::<TestInput>::blank_input(Frame::new(10));
        assert_eq!(input.frame, Frame::new(10));
        assert_eq!(input.input, TestInput::default());
    }

    #[test]
    fn blank_input_with_null_frame() {
        let input = PlayerInput::<TestInput>::blank_input(Frame::NULL);
        assert_eq!(input.frame, Frame::NULL);
        assert_eq!(input.input, TestInput::default());
    }

    // ==========================================
    // Equality Tests (equal method)
    // ==========================================

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

    #[test]
    fn equal_different_frames_input_only_true() {
        let input1 = PlayerInput::new(Frame::new(100), TestInput { inp: 50 });
        let input2 = PlayerInput::new(Frame::new(200), TestInput { inp: 50 });
        // With input_only=true, frames are ignored
        assert!(input1.equal(&input2, true));
    }

    #[test]
    fn equal_different_frames_input_only_false() {
        let input1 = PlayerInput::new(Frame::new(100), TestInput { inp: 50 });
        let input2 = PlayerInput::new(Frame::new(200), TestInput { inp: 50 });
        // With input_only=false, frames matter
        assert!(!input1.equal(&input2, false));
    }

    #[test]
    fn equal_null_frames() {
        let input1 = PlayerInput::new(Frame::NULL, TestInput { inp: 10 });
        let input2 = PlayerInput::new(Frame::NULL, TestInput { inp: 10 });
        assert!(input1.equal(&input2, false));
    }

    // ==========================================
    // PartialEq Trait Tests
    // ==========================================

    #[test]
    fn partial_eq_same_inputs() {
        let input1 = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        let input2 = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        assert_eq!(input1, input2);
    }

    #[test]
    fn partial_eq_different_inputs() {
        let input1 = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        let input2 = PlayerInput::new(Frame::new(0), TestInput { inp: 6 });
        assert_ne!(input1, input2);
    }

    #[test]
    fn partial_eq_different_frames() {
        let input1 = PlayerInput::new(Frame::new(0), TestInput { inp: 5 });
        let input2 = PlayerInput::new(Frame::new(1), TestInput { inp: 5 });
        assert_ne!(input1, input2);
    }

    // ==========================================
    // Clone/Copy Tests
    // ==========================================

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn clone_preserves_data() {
        let input = PlayerInput::new(Frame::new(42), TestInput { inp: 99 });
        let cloned = input.clone();
        assert_eq!(input, cloned);
    }

    #[test]
    fn copy_semantics() {
        let input = PlayerInput::new(Frame::new(1), TestInput { inp: 2 });
        let copied = input; // Should copy, not move
        assert_eq!(input.frame, copied.frame);
        assert_eq!(input.input, copied.input);
    }

    // ==========================================
    // Debug Tests
    // ==========================================

    #[test]
    fn debug_format_contains_fields() {
        let input = PlayerInput::new(Frame::new(10), TestInput { inp: 20 });
        let debug_str = format!("{:?}", input);
        assert!(debug_str.contains("frame"));
        assert!(debug_str.contains("input"));
    }

    // ==========================================
    // Edge Cases
    // ==========================================

    #[test]
    fn max_frame_value() {
        let input = PlayerInput::new(Frame::new(i32::MAX), TestInput { inp: 0 });
        assert_eq!(input.frame, Frame::new(i32::MAX));
    }

    #[test]
    fn max_input_value() {
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: u8::MAX });
        assert_eq!(input.input.inp, u8::MAX);
    }

    #[test]
    fn frame_zero() {
        let input = PlayerInput::new(Frame::new(0), TestInput { inp: 1 });
        assert_eq!(input.frame, Frame::new(0));
    }

    // ==========================================
    // Complex Input Type Tests
    // ==========================================

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default, Debug)]
    struct ComplexInput {
        x: i16,
        y: i16,
        buttons: u32,
    }

    #[test]
    fn complex_input_new() {
        let input = PlayerInput::new(
            Frame::new(55),
            ComplexInput {
                x: -100,
                y: 200,
                buttons: 0b1111,
            },
        );
        assert_eq!(input.frame, Frame::new(55));
        assert_eq!(input.input.x, -100);
        assert_eq!(input.input.y, 200);
        assert_eq!(input.input.buttons, 0b1111);
    }

    #[test]
    fn complex_input_blank() {
        let input = PlayerInput::<ComplexInput>::blank_input(Frame::new(99));
        assert_eq!(input.frame, Frame::new(99));
        assert_eq!(input.input, ComplexInput::default());
    }

    #[test]
    fn complex_input_equal() {
        let input1 = PlayerInput::new(
            Frame::new(1),
            ComplexInput {
                x: 10,
                y: 20,
                buttons: 30,
            },
        );
        let input2 = PlayerInput::new(
            Frame::new(1),
            ComplexInput {
                x: 10,
                y: 20,
                buttons: 30,
            },
        );
        assert!(input1.equal(&input2, false));
        assert!(input1.equal(&input2, true));
    }
}
