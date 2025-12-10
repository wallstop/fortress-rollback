//! Fuzz target for input queue operations via SyncTestSession.
//!
//! This target exercises the input queue through the public SyncTestSession API,
//! which is the synchronous testing mode that validates input handling.
//!
//! # Safety Properties Tested
//! - No panics on arbitrary input sequences
//! - No panics on frame advancement patterns
//! - Graceful handling of prediction and rollback scenarios

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use fortress_rollback::{Config, FortressRequest, InputQueueConfig, PlayerType, SessionBuilder};
use std::net::SocketAddr;

/// Operations that can be performed on a sync test session
#[derive(Debug, Arbitrary)]
enum SessionOp {
    /// Add local input for a player
    AddLocalInput {
        /// Player index (will be modulo'd to valid range)
        player: u8,
        /// Input value
        input: u8,
    },
    /// Advance the session by one frame
    AdvanceFrame,
    /// Get current session state
    GetState,
}

/// Fuzz input structure
#[derive(Debug, Arbitrary)]
struct FuzzInput {
    /// Number of players (1-4 for reasonable testing)
    num_players: u8,
    /// Check distance for sync testing
    check_distance: u8,
    /// Input delay
    input_delay: u8,
    /// Sequence of operations to perform
    operations: Vec<SessionOp>,
}

/// Stub configuration for testing
struct TestConfig;

impl Config for TestConfig {
    type Input = u8;
    type State = Vec<u8>;
    type Address = SocketAddr;
}

fuzz_target!(|fuzz_input: FuzzInput| {
    // Clamp values to reasonable ranges to avoid OOM and timeouts
    let num_players = (fuzz_input.num_players % 4).max(1) as usize;
    let check_distance = (fuzz_input.check_distance % 16).max(1) as usize;
    let input_delay = (fuzz_input.input_delay % 8) as usize;

    // Limit operations to prevent timeouts
    let max_ops = 1000;
    let operations = if fuzz_input.operations.len() > max_ops {
        &fuzz_input.operations[..max_ops]
    } else {
        &fuzz_input.operations
    };

    // Build the session
    let mut builder = SessionBuilder::<TestConfig>::new()
        .with_num_players(num_players)
        .with_input_delay(input_delay)
        .with_input_queue_config(InputQueueConfig::minimal())
        .with_check_distance(check_distance);

    // Add players - builder takes ownership on each add_player call
    for i in 0..num_players {
        match builder.add_player(PlayerType::Local, i.into()) {
            Ok(b) => builder = b,
            Err(_) => return, // Invalid configuration, skip
        }
    }

    // Try to start the sync test session
    let session_result = builder.start_synctest_session();
    let mut session = match session_result {
        Ok(s) => s,
        Err(_) => return, // Invalid configuration, skip
    };

    // Track current game state for save/load requests
    let mut current_state: Vec<u8> = Vec::new();

    // Execute operations
    for op in operations {
        match op {
            SessionOp::AddLocalInput { player, input } => {
                let player_handle = (*player as usize) % num_players;
                // Ignore errors - we're testing that it doesn't panic
                let _ = session.add_local_input(player_handle.into(), *input);
            }
            SessionOp::AdvanceFrame => {
                // Call advance_frame which returns requests
                let advance_result = session.advance_frame();
                if let Ok(requests) = advance_result {
                    for request in requests {
                        match request {
                            FortressRequest::SaveGameState { cell, frame } => {
                                // Save current state
                                cell.save(frame, Some(current_state.clone()), None);
                            }
                            FortressRequest::LoadGameState { cell, .. } => {
                                // Load state
                                if let Some(state) = cell.load() {
                                    current_state = state;
                                }
                            }
                            FortressRequest::AdvanceFrame { inputs } => {
                                // Update state based on inputs
                                for (input, _status) in inputs {
                                    current_state.push(input);
                                }
                                // Keep state bounded
                                if current_state.len() > 1024 {
                                    current_state =
                                        current_state[current_state.len() - 512..].to_vec();
                                }
                            }
                        }
                    }
                }
            }
            SessionOp::GetState => {
                // Just query state - should never panic
                let _ = session.current_frame();
                let _ = session.num_players();
                let _ = session.max_prediction();
            }
        }
    }
});

