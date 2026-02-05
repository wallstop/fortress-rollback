//! # SyncTest Example
//!
//! Demonstrates using `SyncTestSession` to verify determinism in your game logic.
//!
//! `SyncTestSession` is a testing tool that simulates rollbacks every frame and
//! verifies that your game state produces identical checksums when resimulated.
//! This is essential for catching determinism bugs early in development.
//!
//! ## How It Works
//!
//! 1. Every frame, the session saves the current state with a checksum
//! 2. After advancing past `check_distance` frames, it rolls back and resimulates
//! 3. If the resimulated state produces a different checksum, it reports an error
//!
//! ## When to Use
//!
//! - During development to catch non-deterministic code
//! - In CI/CD pipelines to prevent determinism regressions
//! - When debugging desync issues in multiplayer games
//!
//! Run with: `cargo run --example sync_test`

// Allow example-specific patterns
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::disallowed_macros,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use fortress_rollback::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

// ============================================================================
// Game State - A simple counter that increments based on input
// ============================================================================

/// A minimal game state for determinism testing.
///
/// This is intentionally simple to demonstrate the sync test workflow.
/// In a real game, this would contain all game entities, physics state, etc.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
struct CounterState {
    /// The current frame number (for verification)
    frame: i32,
    /// Accumulated value from all player inputs
    total: u64,
    /// Per-player counters
    player_values: [u64; 2],
}

impl CounterState {
    /// Advances the state by one frame using the provided inputs.
    ///
    /// This is where your actual game logic would go. For determinism,
    /// this function must produce identical results given identical inputs.
    fn advance(&mut self, inputs: &[(CounterInput, InputStatus)]) {
        self.frame += 1;

        for (player_idx, (input, status)) in inputs.iter().enumerate() {
            // Skip disconnected players
            if *status == InputStatus::Disconnected {
                continue;
            }

            // Apply input
            if input.increment {
                let increment = u64::from(input.amount);
                self.player_values[player_idx] =
                    self.player_values[player_idx].wrapping_add(increment);
                self.total = self.total.wrapping_add(increment);
            }
        }
    }

    /// Computes a deterministic checksum of the game state.
    ///
    /// This is critical for desync detection. The checksum must:
    /// - Include ALL game state that affects gameplay
    /// - Be computed the same way on all machines
    /// - Use deterministic hashing (no HashMap iteration order, etc.)
    fn checksum(&self) -> u128 {
        // Simple checksum using XOR and bit shifting
        // In production, consider using a proper hash like FNV-1a or xxHash
        let mut hash: u128 = 0;
        hash ^= self.frame as u128;
        hash ^= (self.total as u128) << 32;
        hash ^= (self.player_values[0] as u128) << 64;
        hash ^= (self.player_values[1] as u128) << 96;
        hash
    }
}

// ============================================================================
// Input Type - What each player sends each frame
// ============================================================================

/// Player input for the counter game.
///
/// Input types must be:
/// - `Copy` + `Clone` - for efficient handling
/// - `PartialEq` - for prediction comparison
/// - `Default` - for "no input" / disconnected state
/// - `Serialize` + `Deserialize` - for network transmission
#[derive(Copy, Clone, PartialEq, Default, Debug, Serialize, Deserialize)]
struct CounterInput {
    /// Whether to add to the counter this frame
    increment: bool,
    /// How much to add (if incrementing)
    amount: u8,
}

// ============================================================================
// Config Type - Ties everything together
// ============================================================================

/// Configuration marker struct for the session.
///
/// This associates your Input, State, and Address types together.
struct CounterConfig;

impl Config for CounterConfig {
    type Input = CounterInput;
    type State = CounterState;
    type Address = SocketAddr;
}

// ============================================================================
// Main Example
// ============================================================================

fn main() -> Result<(), FortressError> {
    println!("=== Fortress Rollback Sync Test Example ===\n");

    // Run a basic sync test
    basic_sync_test()?;

    // Demonstrate what happens with non-determinism (commented out by default)
    // This would fail and demonstrate error handling:
    // non_deterministic_test()?;

    println!("\n=== All sync tests passed! ===");
    Ok(())
}

/// Runs a basic sync test to verify deterministic game logic.
fn basic_sync_test() -> Result<(), FortressError> {
    println!("--- Basic Sync Test ---\n");

    // Step 1: Create a SyncTestSession using SessionBuilder
    //
    // Key configuration options:
    // - num_players: How many players in the game
    // - check_distance: How many frames back to rollback and verify (must be < max_prediction)
    // - input_delay: Simulated input delay (usually 0 for sync tests)
    // - max_prediction: Maximum frames that can be predicted ahead

    let num_players = 2;
    let check_distance = 2; // Roll back 2 frames and resimulate
    let input_delay = 0;
    let max_prediction = 8;

    let mut session = SessionBuilder::<CounterConfig>::new()
        .with_num_players(num_players)?
        .with_check_distance(check_distance)
        .with_input_delay(input_delay)?
        .with_max_prediction_window(max_prediction)
        .start_synctest_session()?;

    println!("Created SyncTestSession:");
    println!("  - Players: {}", session.num_players());
    println!("  - Check distance: {}", session.check_distance());
    println!("  - Max prediction: {}", session.max_prediction());
    println!();

    // Step 2: Initialize game state
    let mut game_state = CounterState::default();

    // Step 3: Run the simulation for several frames
    let total_frames = 20;
    println!("Running {} frames of simulation...\n", total_frames);

    for frame in 0..total_frames {
        // Step 3a: Add input for ALL players
        // Use local_player_handles() to get all player handles in a sync test.
        // In a sync test, all players are treated as local.
        for (idx, handle) in session.local_player_handles().into_iter().enumerate() {
            let input = CounterInput {
                increment: frame % 3 != 0, // Increment 2 out of every 3 frames
                amount: ((idx + 1) * 10) as u8,
            };
            session.add_local_input(handle, input)?;
        }

        // Step 3b: Advance the frame and get requests
        // If checksums don't match, this returns MismatchedChecksum error
        let requests = session.advance_frame()?;

        // Step 3c: Process ALL requests in order
        // This is critical - requests must be handled in the exact order given
        for request in requests {
            match request {
                FortressRequest::SaveGameState { cell, frame } => {
                    // Save the current state with its checksum
                    let checksum = game_state.checksum();
                    cell.save(frame, Some(game_state.clone()), Some(checksum));
                },
                FortressRequest::LoadGameState { cell, .. } => {
                    // Load a previously saved state (during rollback)
                    if let Some(loaded_state) = cell.load() {
                        game_state = loaded_state;
                    }
                },
                FortressRequest::AdvanceFrame { inputs } => {
                    // Advance game state with the provided inputs
                    game_state.advance(&inputs);
                },
            }
        }

        // Progress indication
        if (frame + 1) % 5 == 0 || frame == 0 {
            println!(
                "Frame {:>2}: total = {:>4}, players = {:?}",
                session.current_frame().as_i32(),
                game_state.total,
                game_state.player_values
            );
        }
    }

    println!();
    println!("Simulation complete!");
    println!("  - Final frame: {}", session.current_frame().as_i32());
    println!("  - Final total: {}", game_state.total);
    println!("  - Final checksum: {:032x}", game_state.checksum());
    println!();
    println!("✓ No checksum mismatches detected - game logic is deterministic!");

    Ok(())
}

/// Example of what happens when game logic is non-deterministic.
///
/// This is commented out in main() because it intentionally fails.
/// Uncomment the call in main() to see the error handling in action.
#[allow(dead_code)]
fn non_deterministic_test() -> Result<(), FortressError> {
    println!("--- Non-Deterministic Test (Expected to Fail) ---\n");

    let mut session = SessionBuilder::<CounterConfig>::new()
        .with_num_players(1)?
        .with_check_distance(2)
        .with_input_delay(0)?
        .with_max_prediction_window(8)
        .start_synctest_session()?;

    // State that intentionally produces different checksums on resimulation
    #[derive(Clone, Default)]
    struct BadState {
        value: u64,
        // Using a counter that changes behavior based on call count
        // This is non-deterministic and will cause checksum mismatches
        save_count: u64,
    }

    let mut game_state = CounterState::default();
    let mut save_count: u64 = 0;

    for frame in 0..20 {
        session.add_local_input(
            PlayerHandle::new(0),
            CounterInput {
                increment: true,
                amount: 1,
            },
        )?;

        match session.advance_frame() {
            Ok(requests) => {
                for request in requests {
                    match request {
                        FortressRequest::SaveGameState { cell, frame } => {
                            save_count += 1;
                            // BUG: Using save_count in checksum makes it non-deterministic!
                            // The first save will have a different checksum than resimulation.
                            let bad_checksum = game_state.checksum() ^ save_count as u128;
                            cell.save(frame, Some(game_state.clone()), Some(bad_checksum));
                        },
                        FortressRequest::LoadGameState { cell, .. } => {
                            if let Some(loaded) = cell.load() {
                                game_state = loaded;
                            }
                        },
                        FortressRequest::AdvanceFrame { inputs } => {
                            game_state.advance(&inputs);
                        },
                    }
                }
            },
            Err(FortressError::MismatchedChecksum {
                current_frame,
                mismatched_frames,
            }) => {
                println!("✗ Detected checksum mismatch!");
                println!("  - Current frame: {}", current_frame.as_i32());
                println!(
                    "  - Mismatched frames: {:?}",
                    mismatched_frames
                        .iter()
                        .map(|f| f.as_i32())
                        .collect::<Vec<_>>()
                );
                println!();
                println!("This indicates non-deterministic game logic.");
                println!("Common causes:");
                println!("  - Using HashMap (iteration order is random)");
                println!("  - Using system time or random without seeded RNG");
                println!("  - Floating-point operations with different precision");
                println!("  - Reading external state (files, network, etc.)");
                return Ok(()); // Return success since we expected this failure
            },
            Err(e) => return Err(e),
        }

        if frame % 5 == 0 {
            println!("Frame {}: still running...", frame);
        }
    }

    println!("Unexpected: no mismatch detected!");
    Ok(())
}
