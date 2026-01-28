//! Input prediction strategies for rollback networking.
//!
//! This module contains the [`PredictionStrategy`] trait and built-in implementations
//! for predicting player inputs when actual inputs haven't arrived yet.
//!
//! # Determinism Requirement
//!
//! **CRITICAL**: All prediction strategies MUST be deterministic across all peers.
//! Both peers must produce the exact same predicted input given the same arguments.
//! If predictions differ between peers, they will desync during rollback.
//!
//! # Built-in Strategies
//!
//! - [`RepeatLastConfirmed`]: Repeats the last confirmed input (default)
//! - [`BlankPrediction`]: Always returns the default (blank) input
//!
//! # Custom Strategies
//!
//! You can implement custom prediction strategies for game-specific behavior:
//!
//! ```ignore
//! use fortress_rollback::{Frame, PredictionStrategy};
//!
//! struct MyPrediction;
//!
//! impl<I: Copy + Default> PredictionStrategy<I> for MyPrediction {
//!     fn predict(&self, frame: Frame, last_confirmed_input: Option<I>, _player_index: usize) -> I {
//!         // For a fighting game, you might predict "hold block" as a safe default
//!         // This MUST be deterministic - don't use random values or timing-dependent data!
//!         last_confirmed_input.unwrap_or_default()
//!     }
//! }
//! ```

use crate::Frame;

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
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlankPrediction;

impl<I: Copy + Default> PredictionStrategy<I> for BlankPrediction {
    fn predict(&self, _frame: Frame, _last_confirmed: Option<I>, _player_index: usize) -> I {
        I::default()
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

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
    struct TestInput {
        inp: u8,
    }

    #[test]
    fn test_blank_prediction_strategy() {
        let strategy = BlankPrediction;

        // Should always return default regardless of last confirmed
        let result: TestInput = strategy.predict(Frame::new(0), Some(TestInput { inp: 42 }), 0);
        assert_eq!(result, TestInput::default());

        let result: TestInput = strategy.predict(Frame::new(10), None, 1);
        assert_eq!(result, TestInput::default());
    }

    #[test]
    fn test_repeat_last_confirmed_strategy() {
        let strategy = RepeatLastConfirmed;

        // Should return last confirmed input when available
        let result: TestInput = strategy.predict(Frame::new(5), Some(TestInput { inp: 99 }), 0);
        assert_eq!(result.inp, 99);

        // Should return default when no last confirmed
        let result: TestInput = strategy.predict(Frame::new(0), None, 0);
        assert_eq!(result, TestInput::default());
    }

    #[test]
    fn test_prediction_strategy_player_index_ignored_by_default() {
        // Both default strategies ignore player_index
        let repeat = RepeatLastConfirmed;
        let blank = BlankPrediction;

        let last_input = Some(TestInput { inp: 42 });

        // Same result regardless of player_index
        for player_idx in 0..10 {
            let repeat_result: TestInput = repeat.predict(Frame::new(5), last_input, player_idx);
            assert_eq!(repeat_result.inp, 42);

            let blank_result: TestInput = blank.predict(Frame::new(5), last_input, player_idx);
            assert_eq!(blank_result, TestInput::default());
        }
    }

    #[test]
    fn test_repeat_last_confirmed_debug() {
        let strategy = RepeatLastConfirmed;
        let debug_str = format!("{:?}", strategy);
        assert!(debug_str.contains("RepeatLastConfirmed"));
    }

    #[test]
    fn test_blank_prediction_debug() {
        let strategy = BlankPrediction;
        let debug_str = format!("{:?}", strategy);
        assert!(debug_str.contains("BlankPrediction"));
    }

    #[test]
    fn test_repeat_last_confirmed_copy() {
        let a = RepeatLastConfirmed;
        let b = a; // Copy
        let debug_a = format!("{:?}", a);
        let debug_b = format!("{:?}", b);
        assert_eq!(debug_a, debug_b);
    }

    #[test]
    fn test_blank_prediction_copy() {
        let a = BlankPrediction;
        let b = a; // Copy
        let debug_a = format!("{:?}", a);
        let debug_b = format!("{:?}", b);
        assert_eq!(debug_a, debug_b);
    }
}
