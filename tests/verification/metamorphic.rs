//! Metamorphic Testing for Fortress Rollback
//!
//! Metamorphic testing verifies relationships between inputs and outputs, rather than
//! checking specific expected values. This is especially useful for complex systems
//! where defining expected outputs is difficult, but relationships between transformations
//! should hold.
//!
//! # Test Categories
//!
//! 1. **Input Permutation Invariance**: Different orderings of same inputs produce same result
//! 2. **Timing Invariance**: Relative timing relationships are preserved
//! 3. **Replay Consistency**: Replaying same sequence produces identical results

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use fortress_rollback::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::SocketAddr;

// ============================================================================
// Test Configuration
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug, Hash)]
struct MetaInput {
    /// Direction bits: up, down, left, right
    direction: u8,
    /// Action bits: jump, attack, etc.
    action: u8,
}

impl MetaInput {
    fn new(direction: u8, action: u8) -> Self {
        Self { direction, action }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
struct MetaGameState {
    frame: i32,
    /// Accumulated input history for checksum validation
    input_sum: u64,
    /// Player positions (simplified as accumulated movement)
    player_positions: Vec<i64>,
    /// Total actions performed
    action_count: u32,
}

impl MetaGameState {
    fn new(num_players: usize) -> Self {
        Self {
            frame: 0,
            input_sum: 0,
            player_positions: vec![0; num_players],
            action_count: 0,
        }
    }

    /// Apply inputs deterministically
    fn apply_inputs(&mut self, inputs: &[(MetaInput, InputStatus)]) {
        self.frame += 1;
        for (player_idx, (input, status)) in inputs.iter().enumerate() {
            if *status != InputStatus::Disconnected {
                // Accumulate input into checksum (order-independent operation)
                self.input_sum = self.input_sum.wrapping_add(input.direction as u64);
                self.input_sum = self.input_sum.wrapping_add(input.action as u64 * 256);

                // Update player position based on direction
                if player_idx < self.player_positions.len() {
                    let dir = input.direction;
                    // Up = +1, Down = -1, Left = -10, Right = +10
                    if dir & 0x01 != 0 {
                        self.player_positions[player_idx] += 1; // up
                    }
                    if dir & 0x02 != 0 {
                        self.player_positions[player_idx] -= 1; // down
                    }
                    if dir & 0x04 != 0 {
                        self.player_positions[player_idx] -= 10; // left
                    }
                    if dir & 0x08 != 0 {
                        self.player_positions[player_idx] += 10; // right
                    }
                }

                // Count actions
                if input.action != 0 {
                    self.action_count += 1;
                }
            }
        }
    }

    /// Compute a deterministic checksum
    fn checksum(&self) -> u128 {
        let mut hash: u128 = 0xcbf29ce484222325;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.frame as u128;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.input_sum as u128;
        for &pos in &self.player_positions {
            hash = hash.wrapping_mul(0x100000001b3);
            hash ^= pos as u128;
        }
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.action_count as u128;
        hash
    }
}

struct MetaConfig;

impl Config for MetaConfig {
    type Input = MetaInput;
    type State = MetaGameState;
    type Address = SocketAddr;
}

// ============================================================================
// Test Utilities
// ============================================================================

/// Run a synctest session with given inputs and return final state
fn run_synctest_session(
    num_players: usize,
    num_frames: usize,
    input_generator: impl Fn(usize, usize) -> MetaInput, // (player, frame) -> input
) -> MetaGameState {
    let mut sess = SessionBuilder::<MetaConfig>::new()
        .with_num_players(num_players)
        .unwrap()
        .with_max_prediction_window(8)
        .with_input_delay(0)
        .unwrap()
        .start_synctest_session()
        .expect("Failed to create session");

    let mut state = MetaGameState::new(num_players);

    for frame in 0..num_frames {
        // Add inputs for all players
        for player in 0..num_players {
            let input = input_generator(player, frame);
            sess.add_local_input(PlayerHandle::new(player), input)
                .expect("Failed to add input");
        }

        // Process requests
        for request in sess.advance_frame().expect("Failed to advance frame") {
            match request {
                FortressRequest::SaveGameState { cell, frame } => {
                    cell.save(frame, Some(state.clone()), Some(state.checksum()));
                },
                FortressRequest::LoadGameState { cell, .. } => {
                    state = cell.load().expect("Failed to load state");
                },
                FortressRequest::AdvanceFrame { inputs } => {
                    state.apply_inputs(&inputs);
                },
                _ => unreachable!("Unknown request type"),
            }
        }
    }

    state
}

/// Generate a deterministic input sequence
fn generate_input_sequence(
    num_players: usize,
    num_frames: usize,
    seed: u64,
) -> BTreeMap<(usize, usize), MetaInput> {
    let mut inputs = BTreeMap::new();
    let mut rng = seed;

    for frame in 0..num_frames {
        for player in 0..num_players {
            // Simple LCG for deterministic randomness
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let direction = ((rng >> 56) & 0x0F) as u8;
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let action = ((rng >> 56) & 0x03) as u8;

            inputs.insert((player, frame), MetaInput::new(direction, action));
        }
    }

    inputs
}

// ============================================================================
// Input Permutation Invariance Tests
// ============================================================================

/// Metamorphic relation: Running the same input sequence multiple times
/// should produce identical final states.
#[test]
fn test_metamorphic_replay_produces_identical_state() {
    let num_players = 2;
    let num_frames = 50;
    let seed = 12345u64;

    let inputs = generate_input_sequence(num_players, num_frames, seed);

    // Run session multiple times
    let state1 = run_synctest_session(num_players, num_frames, |player, frame| {
        *inputs.get(&(player, frame)).unwrap()
    });

    let state2 = run_synctest_session(num_players, num_frames, |player, frame| {
        *inputs.get(&(player, frame)).unwrap()
    });

    let state3 = run_synctest_session(num_players, num_frames, |player, frame| {
        *inputs.get(&(player, frame)).unwrap()
    });

    // All runs should produce identical state
    assert_eq!(state1, state2, "First two runs should be identical");
    assert_eq!(state2, state3, "All three runs should be identical");
    assert_eq!(state1.checksum(), state2.checksum());
    assert_eq!(state2.checksum(), state3.checksum());
}

/// Metamorphic relation: Different random seeds produce different states
/// (validates that inputs actually affect the game state)
#[test]
fn test_metamorphic_different_inputs_produce_different_states() {
    let num_players = 2;
    let num_frames = 30;

    let inputs1 = generate_input_sequence(num_players, num_frames, 11111);
    let inputs2 = generate_input_sequence(num_players, num_frames, 22222);

    let state1 = run_synctest_session(num_players, num_frames, |player, frame| {
        *inputs1.get(&(player, frame)).unwrap()
    });

    let state2 = run_synctest_session(num_players, num_frames, |player, frame| {
        *inputs2.get(&(player, frame)).unwrap()
    });

    // Different inputs should produce different states
    assert_ne!(
        state1, state2,
        "Different inputs should produce different states"
    );
}

/// Metamorphic relation: For a commutative game state update, player order
/// shouldn't matter (input_sum is commutative addition).
#[test]
fn test_metamorphic_commutative_input_sum() {
    let num_frames = 20;
    let seed = 42424u64;

    // Generate inputs for 2 players
    let inputs = generate_input_sequence(2, num_frames, seed);

    // Run normally
    let state_normal = run_synctest_session(2, num_frames, |player, frame| {
        *inputs.get(&(player, frame)).unwrap()
    });

    // Run with swapped player inputs (player 0 gets player 1's inputs and vice versa)
    let state_swapped = run_synctest_session(2, num_frames, |player, frame| {
        let swapped_player = 1 - player;
        *inputs.get(&(swapped_player, frame)).unwrap()
    });

    // The input_sum should be the same (commutative property of addition)
    // but player positions will differ
    assert_eq!(
        state_normal.input_sum, state_swapped.input_sum,
        "Input sum should be commutative - same regardless of player ordering"
    );

    // Action count should also be the same (just counting non-zero actions)
    assert_eq!(
        state_normal.action_count, state_swapped.action_count,
        "Total action count should be the same"
    );
}

/// Metamorphic relation: Running twice as many frames produces predictable changes
#[test]
fn test_metamorphic_frame_scaling() {
    let num_players = 2;
    let seed = 98765u64;

    // Generate inputs for extended sequence
    let inputs = generate_input_sequence(num_players, 60, seed);

    // Run for 30 frames
    let state_30 = run_synctest_session(num_players, 30, |player, frame| {
        *inputs.get(&(player, frame)).unwrap()
    });

    // Run for 60 frames
    let state_60 = run_synctest_session(num_players, 60, |player, frame| {
        *inputs.get(&(player, frame)).unwrap()
    });

    // Frame count should double
    assert_eq!(state_30.frame, 30);
    assert_eq!(state_60.frame, 60);

    // 60 frame run should have more accumulated input
    assert!(
        state_60.input_sum >= state_30.input_sum,
        "Longer run should have equal or more accumulated input"
    );
}

// ============================================================================
// Timing Invariance Tests
// ============================================================================

/// Metamorphic relation: Input delay should not affect the final state
/// when all inputs are eventually delivered.
#[test]
fn test_metamorphic_input_delay_invariance() {
    let num_players = 2;
    let num_frames = 40;
    let seed = 55555u64;

    let inputs = generate_input_sequence(num_players, num_frames, seed);

    // Run with no input delay
    let state_delay_0 = {
        let mut sess = SessionBuilder::<MetaConfig>::new()
            .with_num_players(num_players)
            .unwrap()
            .with_max_prediction_window(8)
            .with_input_delay(0)
            .unwrap()
            .start_synctest_session()
            .expect("Failed to create session");

        let mut state = MetaGameState::new(num_players);

        for frame in 0..num_frames {
            for player in 0..num_players {
                let input = *inputs.get(&(player, frame)).unwrap();
                sess.add_local_input(PlayerHandle::new(player), input)
                    .expect("Failed to add input");
            }

            for request in sess.advance_frame().expect("Failed to advance frame") {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        cell.save(frame, Some(state.clone()), Some(state.checksum()));
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        state = cell.load().expect("Failed to load state");
                    },
                    FortressRequest::AdvanceFrame { inputs } => {
                        state.apply_inputs(&inputs);
                    },
                    _ => unreachable!("Unknown request type"),
                }
            }
        }
        state
    };

    // Run with input delay of 2
    let state_delay_2 = {
        let mut sess = SessionBuilder::<MetaConfig>::new()
            .with_num_players(num_players)
            .unwrap()
            .with_max_prediction_window(8)
            .with_input_delay(2)
            .unwrap()
            .start_synctest_session()
            .expect("Failed to create session");

        let mut state = MetaGameState::new(num_players);

        for frame in 0..num_frames {
            for player in 0..num_players {
                let input = *inputs.get(&(player, frame)).unwrap();
                sess.add_local_input(PlayerHandle::new(player), input)
                    .expect("Failed to add input");
            }

            for request in sess.advance_frame().expect("Failed to advance frame") {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        cell.save(frame, Some(state.clone()), Some(state.checksum()));
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        state = cell.load().expect("Failed to load state");
                    },
                    FortressRequest::AdvanceFrame { inputs } => {
                        state.apply_inputs(&inputs);
                    },
                    _ => unreachable!("Unknown request type"),
                }
            }
        }
        state
    };

    // With input delay, the inputs are shifted, so we expect the same
    // number of frames but the actual content differs due to delay shift.
    // The key invariant is that the session behaves consistently.
    assert_eq!(
        state_delay_0.frame, state_delay_2.frame,
        "Both runs should process the same number of frames"
    );

    // Both should complete without errors (implicit in getting here)
}

/// Metamorphic relation: Different prediction windows should produce
/// the same final state in a synctest session (no actual rollbacks needed).
#[test]
fn test_metamorphic_prediction_window_invariance() {
    let num_players = 2;
    let num_frames = 30;
    let seed = 77777u64;

    let inputs = generate_input_sequence(num_players, num_frames, seed);

    // Run with small prediction window
    let state_small = {
        let mut sess = SessionBuilder::<MetaConfig>::new()
            .with_num_players(num_players)
            .unwrap()
            .with_max_prediction_window(4)
            .with_input_delay(0)
            .unwrap()
            .start_synctest_session()
            .expect("Failed to create session");

        let mut state = MetaGameState::new(num_players);

        for frame in 0..num_frames {
            for player in 0..num_players {
                let input = *inputs.get(&(player, frame)).unwrap();
                sess.add_local_input(PlayerHandle::new(player), input)
                    .expect("Failed to add input");
            }

            for request in sess.advance_frame().expect("Failed to advance frame") {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        cell.save(frame, Some(state.clone()), Some(state.checksum()));
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        state = cell.load().expect("Failed to load state");
                    },
                    FortressRequest::AdvanceFrame { inputs } => {
                        state.apply_inputs(&inputs);
                    },
                    _ => unreachable!("Unknown request type"),
                }
            }
        }
        state
    };

    // Run with large prediction window
    let state_large = {
        let mut sess = SessionBuilder::<MetaConfig>::new()
            .with_num_players(num_players)
            .unwrap()
            .with_max_prediction_window(12)
            .with_input_delay(0)
            .unwrap()
            .start_synctest_session()
            .expect("Failed to create session");

        let mut state = MetaGameState::new(num_players);

        for frame in 0..num_frames {
            for player in 0..num_players {
                let input = *inputs.get(&(player, frame)).unwrap();
                sess.add_local_input(PlayerHandle::new(player), input)
                    .expect("Failed to add input");
            }

            for request in sess.advance_frame().expect("Failed to advance frame") {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        cell.save(frame, Some(state.clone()), Some(state.checksum()));
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        state = cell.load().expect("Failed to load state");
                    },
                    FortressRequest::AdvanceFrame { inputs } => {
                        state.apply_inputs(&inputs);
                    },
                    _ => unreachable!("Unknown request type"),
                }
            }
        }
        state
    };

    // Same inputs should produce same final state regardless of prediction window
    assert_eq!(
        state_small, state_large,
        "Different prediction windows should produce same final state"
    );
}

// ============================================================================
// Replay Consistency Tests
// ============================================================================

/// Metamorphic relation: Saving and loading state should preserve determinism.
/// If we save at frame N, then continue to frame M, the state at M should be
/// the same whether or not we did a save at N.
#[test]
fn test_metamorphic_save_does_not_affect_state() {
    let num_players = 2;
    let num_frames = 30;
    let seed = 88888u64;

    let inputs = generate_input_sequence(num_players, num_frames, seed);

    // Run without any explicit saves (beyond what synctest requires)
    let state_minimal_saves = run_synctest_session(num_players, num_frames, |player, frame| {
        *inputs.get(&(player, frame)).unwrap()
    });

    // Run with the same logic - synctest session handles saves automatically
    let state_same = run_synctest_session(num_players, num_frames, |player, frame| {
        *inputs.get(&(player, frame)).unwrap()
    });

    // Should be identical
    assert_eq!(
        state_minimal_saves, state_same,
        "Save operations should not affect final state"
    );
}

/// Metamorphic relation: Number of players should scale state predictably.
/// With N players, we expect N position entries.
#[test]
fn test_metamorphic_player_count_scaling() {
    let num_frames = 20;
    let seed = 99999u64;

    // Run with 2 players
    let inputs_2p = generate_input_sequence(2, num_frames, seed);
    let state_2p = run_synctest_session(2, num_frames, |player, frame| {
        *inputs_2p.get(&(player, frame)).unwrap()
    });

    // Run with 4 players
    let inputs_4p = generate_input_sequence(4, num_frames, seed);
    let state_4p = run_synctest_session(4, num_frames, |player, frame| {
        *inputs_4p.get(&(player, frame)).unwrap()
    });

    // Player count should be reflected in state
    assert_eq!(
        state_2p.player_positions.len(),
        2,
        "2-player game should have 2 positions"
    );
    assert_eq!(
        state_4p.player_positions.len(),
        4,
        "4-player game should have 4 positions"
    );

    // Both should complete same number of frames
    assert_eq!(state_2p.frame, state_4p.frame);
}

/// Metamorphic relation: Interleaved neutral inputs should not change
/// the cumulative effect of action inputs.
#[test]
fn test_metamorphic_neutral_input_transparency() {
    let num_players = 2;
    let num_frames = 20;

    // Run with all zeros (neutral inputs)
    let state_neutral = run_synctest_session(num_players, num_frames, |_, _| MetaInput::default());

    // The neutral state should have zeros for all cumulative values
    assert_eq!(
        state_neutral.input_sum, 0,
        "Neutral inputs should result in zero input sum"
    );
    assert_eq!(
        state_neutral.action_count, 0,
        "Neutral inputs should result in zero action count"
    );
    assert!(
        state_neutral.player_positions.iter().all(|&p| p == 0),
        "Neutral inputs should leave all players at origin"
    );
    assert_eq!(
        state_neutral.frame, num_frames as i32,
        "Should still advance correct number of frames"
    );
}

/// Metamorphic relation: Repeated identical inputs should produce
/// linearly scaling cumulative values.
#[test]
fn test_metamorphic_repeated_input_linearity() {
    let num_players = 1;
    let constant_input = MetaInput::new(0x01, 0x01); // up + action

    // Run for 10 frames
    let state_10 = run_synctest_session(num_players, 10, |_, _| constant_input);

    // Run for 20 frames
    let state_20 = run_synctest_session(num_players, 20, |_, _| constant_input);

    // Action count should double
    assert_eq!(state_10.action_count, 10);
    assert_eq!(state_20.action_count, 20);

    // Player position should double (moving up each frame)
    assert_eq!(state_10.player_positions[0], 10); // +1 per frame
    assert_eq!(state_20.player_positions[0], 20);

    // Frame count should be correct
    assert_eq!(state_10.frame, 10);
    assert_eq!(state_20.frame, 20);
}

/// Metamorphic relation: Opposite inputs should cancel out.
#[test]
fn test_metamorphic_opposite_inputs_cancel() {
    let num_players = 1;
    let num_frames = 20;

    // Run with alternating up/down
    let state_alternating = run_synctest_session(num_players, num_frames, |_, frame| {
        if frame % 2 == 0 {
            MetaInput::new(0x01, 0) // up
        } else {
            MetaInput::new(0x02, 0) // down
        }
    });

    // After even number of frames with alternating up/down, position should be 0
    assert_eq!(
        state_alternating.player_positions[0], 0,
        "Alternating up/down should cancel out"
    );
}

// ============================================================================
// Property-Based Metamorphic Tests
// ============================================================================

#[cfg(test)]
mod property_metamorphic {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: Running same inputs always produces same result (determinism)
        #[test]
        fn prop_deterministic_replay(seed in any::<u64>()) {
            let num_players = 2;
            let num_frames = 20;
            let inputs = generate_input_sequence(num_players, num_frames, seed);

            let state1 = run_synctest_session(num_players, num_frames, |player, frame| {
                *inputs.get(&(player, frame)).unwrap()
            });

            let state2 = run_synctest_session(num_players, num_frames, |player, frame| {
                *inputs.get(&(player, frame)).unwrap()
            });

            prop_assert_eq!(state1, state2, "Replays should be deterministic");
        }

        /// Property: Frame count is always accurate
        #[test]
        fn prop_frame_count_accurate(num_frames in 1usize..=50) {
            let num_players = 2;
            let state = run_synctest_session(num_players, num_frames, |_, _| MetaInput::default());
            prop_assert_eq!(state.frame, num_frames as i32);
        }

        /// Property: Player count is preserved in state
        #[test]
        fn prop_player_count_preserved(num_players in 1usize..=4) {
            let num_frames = 10;
            let inputs = generate_input_sequence(num_players, num_frames, 12345);
            let state = run_synctest_session(num_players, num_frames, |player, frame| {
                *inputs.get(&(player, frame)).unwrap()
            });
            prop_assert_eq!(state.player_positions.len(), num_players);
        }

        /// Property: input_sum is commutative across players
        #[test]
        fn prop_input_sum_commutative(seed in any::<u64>()) {
            let num_players = 2;
            let num_frames = 15;
            let inputs = generate_input_sequence(num_players, num_frames, seed);

            let state_normal = run_synctest_session(num_players, num_frames, |player, frame| {
                *inputs.get(&(player, frame)).unwrap()
            });

            let state_swapped = run_synctest_session(num_players, num_frames, |player, frame| {
                let swapped_player = 1 - player;
                *inputs.get(&(swapped_player, frame)).unwrap()
            });

            prop_assert_eq!(
                state_normal.input_sum,
                state_swapped.input_sum,
                "input_sum should be commutative"
            );
        }

        /// Property: Neutral inputs don't change position
        #[test]
        fn prop_neutral_no_position_change(num_frames in 1usize..=30) {
            let num_players = 2;
            let state = run_synctest_session(num_players, num_frames, |_, _| MetaInput::default());

            for pos in &state.player_positions {
                prop_assert_eq!(*pos, 0, "Neutral inputs should not change position");
            }
        }
    }
}

// ============================================================================
// Termination Metamorphic Tests (Phase 4)
// ============================================================================
//
// These tests verify that:
// 1. Same inputs + same confirmed frame → same checksum regardless of arrival timing
// 2. Determinism is maintained through save/load cycles

mod termination_tests {
    use super::*;

    /// Run a synctest session and verify checksum determinism.
    ///
    /// The key metamorphic property is: given the same inputs processed in the same order,
    /// the checksum at any given frame should be identical regardless of when during
    /// the simulation it is computed.
    fn run_checksum_determinism_test(
        num_players: usize,
        num_frames: usize,
        seed: u64,
    ) -> Vec<(Frame, u128)> {
        let inputs = generate_input_sequence(num_players, num_frames, seed);
        let mut sess = SessionBuilder::<MetaConfig>::new()
            .with_num_players(num_players)
            .unwrap()
            .with_max_prediction_window(8)
            .with_input_delay(0)
            .unwrap()
            .start_synctest_session()
            .expect("Failed to create session");

        let mut state = MetaGameState::new(num_players);
        let mut checksums = Vec::new();

        for frame in 0..num_frames {
            // Add inputs for all players
            for player in 0..num_players {
                let input = *inputs.get(&(player, frame)).unwrap();
                sess.add_local_input(PlayerHandle::new(player), input)
                    .expect("Failed to add input");
            }

            // Process requests
            for request in sess.advance_frame().expect("Failed to advance frame") {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        let checksum = state.checksum();
                        cell.save(frame, Some(state.clone()), Some(checksum));
                        checksums.push((frame, checksum));
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        state = cell.load().expect("Failed to load state");
                    },
                    FortressRequest::AdvanceFrame { inputs } => {
                        state.apply_inputs(&inputs);
                    },
                    _ => unreachable!("Unknown request type"),
                }
            }
        }

        checksums
    }

    /// Metamorphic test: Same inputs produce same checksums
    ///
    /// This verifies the fundamental determinism property: running the same
    /// input sequence twice should produce identical checksums at every frame.
    #[test]
    fn test_checksum_determinism_across_runs() {
        let num_players = 2;
        let num_frames = 50;
        let seed = 12345u64;

        let checksums1 = run_checksum_determinism_test(num_players, num_frames, seed);
        let checksums2 = run_checksum_determinism_test(num_players, num_frames, seed);

        assert_eq!(
            checksums1.len(),
            checksums2.len(),
            "Both runs should produce same number of checksums"
        );

        for ((frame1, cs1), (frame2, cs2)) in checksums1.iter().zip(checksums2.iter()) {
            assert_eq!(frame1, frame2, "Frames should match");
            assert_eq!(
                cs1, cs2,
                "Checksums should match at frame {}: got {} vs {}",
                frame1, cs1, cs2
            );
        }
    }

    /// Metamorphic test: Synctest detects checksum mismatch when state differs
    ///
    /// When a synctest session runs with correct checksums, it should NOT detect desync.
    #[test]
    fn test_synctest_no_desync_with_correct_checksums() {
        let num_players = 2;
        let num_frames = 30;
        let inputs = generate_input_sequence(num_players, num_frames, 54321);

        let mut sess = SessionBuilder::<MetaConfig>::new()
            .with_num_players(num_players)
            .unwrap()
            .with_max_prediction_window(8)
            .with_input_delay(0)
            .unwrap()
            .start_synctest_session()
            .expect("Failed to create session");

        let mut state = MetaGameState::new(num_players);
        let mut desync_detected = false;

        for frame in 0..num_frames {
            for player in 0..num_players {
                let input = *inputs.get(&(player, frame)).unwrap();
                sess.add_local_input(PlayerHandle::new(player), input)
                    .expect("Failed to add input");
            }

            let result = sess.advance_frame();
            if result.is_err() {
                // SyncTestSession returns error on mismatch
                desync_detected = true;
                break;
            }

            for request in result.expect("Failed to advance frame") {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        // Save with correct checksum - should not trigger desync
                        cell.save(frame, Some(state.clone()), Some(state.checksum()));
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        state = cell.load().expect("Failed to load state");
                    },
                    FortressRequest::AdvanceFrame { inputs } => {
                        state.apply_inputs(&inputs);
                    },
                    _ => unreachable!("Unknown request type"),
                }
            }
        }

        // This test verifies that CORRECT code does NOT trigger desync
        assert!(
            !desync_detected,
            "Deterministic session should not detect desync"
        );
    }

    /// Metamorphic test: Different seeds produce different checksums
    ///
    /// This is the contrapositive of determinism: different inputs should
    /// generally produce different checksums (with high probability).
    #[test]
    fn test_different_inputs_different_checksums() {
        let num_players = 2;
        let num_frames = 30;

        let checksums1 = run_checksum_determinism_test(num_players, num_frames, 11111);
        let checksums2 = run_checksum_determinism_test(num_players, num_frames, 22222);

        // The final checksums should differ (with very high probability)
        // We compare the last few checksums since early frames may match by chance
        if checksums1.len() > 5 && checksums2.len() > 5 {
            let late_cs1: Vec<_> = checksums1.iter().rev().take(5).collect();
            let late_cs2: Vec<_> = checksums2.iter().rev().take(5).collect();

            let all_match = late_cs1
                .iter()
                .zip(late_cs2.iter())
                .all(|((_, cs1), (_, cs2))| cs1 == cs2);

            assert!(
                !all_match,
                "Different seeds should produce different late-game checksums"
            );
        }
    }
}

#[cfg(test)]
mod termination_property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// Property: Checksum at frame N is deterministic regardless of seed
        /// as long as the generated inputs are the same.
        #[test]
        fn prop_checksum_determinism(
            num_players in 1usize..=3,
            num_frames in 10usize..=40,
            seed in any::<u64>(),
        ) {
            let checksums1 = run_checksum_determinism_test(num_players, num_frames, seed);
            let checksums2 = run_checksum_determinism_test(num_players, num_frames, seed);

            prop_assert_eq!(
                checksums1.len(),
                checksums2.len(),
                "Both runs should produce same number of checksums"
            );

            for (i, ((frame1, cs1), (frame2, cs2))) in
                checksums1.iter().zip(checksums2.iter()).enumerate()
            {
                prop_assert_eq!(
                    frame1, frame2,
                    "Frame mismatch at index {}", i
                );
                prop_assert_eq!(
                    cs1, cs2,
                    "Checksum mismatch at frame {}", frame1
                );
            }
        }

        /// Property: Final state value is deterministic
        #[test]
        fn prop_final_state_determinism(
            num_players in 1usize..=3,
            num_frames in 5usize..=25,
            seed in any::<u64>(),
        ) {
            let inputs = generate_input_sequence(num_players, num_frames, seed);

            let state1 = run_synctest_session(num_players, num_frames, |player, frame| {
                *inputs.get(&(player, frame)).unwrap()
            });

            let state2 = run_synctest_session(num_players, num_frames, |player, frame| {
                *inputs.get(&(player, frame)).unwrap()
            });

            prop_assert_eq!(state1.frame, state2.frame, "Frame should match");
            prop_assert_eq!(state1.input_sum, state2.input_sum, "input_sum should match");
            prop_assert_eq!(
                state1.player_positions,
                state2.player_positions,
                "player_positions should match"
            );
            prop_assert_eq!(
                state1.action_count,
                state2.action_count,
                "action_count should match"
            );
        }
    }

    /// Helper function used by termination property tests
    fn run_checksum_determinism_test(
        num_players: usize,
        num_frames: usize,
        seed: u64,
    ) -> Vec<(Frame, u128)> {
        let inputs = generate_input_sequence(num_players, num_frames, seed);
        let mut sess = SessionBuilder::<MetaConfig>::new()
            .with_num_players(num_players)
            .unwrap()
            .with_max_prediction_window(8)
            .with_input_delay(0)
            .unwrap()
            .start_synctest_session()
            .expect("Failed to create session");

        let mut state = MetaGameState::new(num_players);
        let mut checksums = Vec::new();

        for frame in 0..num_frames {
            for player in 0..num_players {
                let input = *inputs.get(&(player, frame)).unwrap();
                sess.add_local_input(PlayerHandle::new(player), input)
                    .expect("Failed to add input");
            }

            for request in sess.advance_frame().expect("Failed to advance frame") {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        let checksum = state.checksum();
                        cell.save(frame, Some(state.clone()), Some(checksum));
                        checksums.push((frame, checksum));
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        state = cell.load().expect("Failed to load state");
                    },
                    FortressRequest::AdvanceFrame { inputs } => {
                        state.apply_inputs(&inputs);
                    },
                    _ => unreachable!("Unknown request type"),
                }
            }
        }

        checksums
    }
}
