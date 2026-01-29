//! # Request Handling Example
//!
//! This example demonstrates the different ways to handle [`FortressRequest`]
//! in your game loop:
//!
//! 1. **Manual matching** — Full control with explicit `match` statements
//! 2. **`handle_requests!` macro** — Concise, less boilerplate
//!
//! It also shows:
//! - Using [`compute_checksum`] for desync detection
//! - That `FortressRequest` is exhaustively matchable (no wildcard needed)
//!
//! Run with: `cargo run --example request_handling`
//!
//! [`FortressRequest`]: fortress_rollback::FortressRequest
//! [`compute_checksum`]: fortress_rollback::compute_checksum

// Allow example-specific patterns
#![allow(
    dead_code, // Demonstration functions may not be called from main
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::disallowed_macros,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use fortress_rollback::{
    compute_checksum, handle_requests, Config, DesyncDetection, FortressRequest, Frame,
    GameStateCell, InputStatus, InputVec, PlayerHandle, PlayerType, SessionBuilder, SessionState,
    UdpNonBlockingSocket,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

// ============================================================================
// Game Types
// ============================================================================

/// Input type sent over the network.
///
/// Must be `Copy + Clone + PartialEq + Default + Serialize + Deserialize`.
#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
struct GameInput {
    buttons: u8,
}

/// Game state that will be saved/loaded during rollback.
///
/// Must be `Clone`, and `Serialize + Deserialize` for checksums.
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
struct GameState {
    frame: i32,
    score: u64,
    player_x: [f32; 2],
}

impl GameState {
    /// Advance the game state by one frame with the given inputs.
    fn update(&mut self, inputs: &[(GameInput, InputStatus)]) {
        self.frame += 1;

        for (player_idx, (input, status)) in inputs.iter().enumerate() {
            if *status == InputStatus::Disconnected {
                // Player is disconnected — use AI or freeze
                continue;
            }

            // Example: Move player based on button input
            if input.buttons & 0x01 != 0 {
                self.player_x[player_idx] += 1.0;
            }
            if input.buttons & 0x02 != 0 {
                self.player_x[player_idx] -= 1.0;
            }
        }
    }
}

/// Configuration type for the session.
struct GameConfig;

impl Config for GameConfig {
    type Input = GameInput;
    type State = GameState;
    type Address = SocketAddr;
}

// ============================================================================
// Request Handling — Method 1: Manual Matching
// ============================================================================

/// Handle requests using explicit match statements.
///
/// This approach gives you full control and makes the logic explicit.
///
/// ## Key Points
///
/// - **No wildcard needed**: `FortressRequest` is exhaustively matchable (not
///   `#[non_exhaustive]`). If a new variant is added in a future version, the
///   compiler will notify you — you won't silently miss handling it.
///
/// - **Process in order**: Requests must be processed in the order returned.
///   Do not sort, filter, or reorder them.
///
/// - **`compute_checksum`**: Enables desync detection by computing a
///   deterministic hash of your game state.
fn handle_requests_manual(requests: Vec<FortressRequest<GameConfig>>, game_state: &mut GameState) {
    for request in requests {
        // No wildcard `_ =>` arm needed — all variants are covered
        match request {
            FortressRequest::SaveGameState { cell, frame } => {
                // Clone the current state
                let state_copy = game_state.clone();

                // Compute a checksum for desync detection.
                // compute_checksum() uses bincode + FNV-1a for deterministic hashing.
                // Returns Result — use .ok() to convert to Option for cell.save().
                let checksum = compute_checksum(game_state).ok();

                // Save the state with its checksum
                cell.save(frame, Some(state_copy), checksum);

                println!(
                    "Saved state at frame {} with checksum {}",
                    frame.as_i32(),
                    checksum.map_or("None".to_string(), |c| format!("{:#034x}", c))
                );
            },

            FortressRequest::LoadGameState { cell, frame } => {
                // LoadGameState is only requested for previously saved frames.
                // A missing state would indicate a library bug, but we handle gracefully.
                if let Some(loaded) = cell.load() {
                    *game_state = loaded;
                    println!(
                        "Loaded state from frame {} (rollback occurred)",
                        frame.as_i32()
                    );
                } else {
                    // This should never happen in normal operation
                    eprintln!(
                        "WARNING: LoadGameState for frame {:?} but no state found",
                        frame
                    );
                }
            },

            FortressRequest::AdvanceFrame { inputs } => {
                // Apply inputs to advance the game
                game_state.update(inputs.as_slice());
                println!("Advanced to frame {}", game_state.frame);
            },
        }
    }
}

// ============================================================================
// Request Handling — Method 2: Using handle_requests! Macro
// ============================================================================

/// Handle requests using the `handle_requests!` macro.
///
/// This approach reduces boilerplate while maintaining the same behavior.
/// The macro handles iteration and matching internally.
///
/// ## Key Points
///
/// - **Same semantics**: The macro processes requests in order, just like
///   manual matching.
///
/// - **Type annotations**: Closure parameters need type annotations since
///   the macro can't infer them.
///
/// - **Exhaustive**: The macro handles all `FortressRequest` variants.
#[allow(unused)]
fn handle_requests_with_macro(
    requests: Vec<FortressRequest<GameConfig>>,
    game_state: &mut GameState,
) {
    handle_requests!(
        requests,
        save: |cell: GameStateCell<GameState>, frame: Frame| {
            // Use compute_checksum for desync detection
            let checksum = compute_checksum(game_state).ok();
            cell.save(frame, Some(game_state.clone()), checksum);
            println!("Saved state at frame {} (via macro)", frame.as_i32());
        },
        load: |cell: GameStateCell<GameState>, frame: Frame| {
            if let Some(loaded) = cell.load() {
                *game_state = loaded;
                println!("Loaded state from frame {} (via macro)", frame.as_i32());
            } else {
                eprintln!("WARNING: LoadGameState for frame {:?} but no state found", frame);
            }
        },
        advance: |inputs: InputVec<GameInput>| {
            game_state.update(inputs.as_slice());
            println!("Advanced to frame {} (via macro)", game_state.frame);
        }
    );
}

// ============================================================================
// Request Handling — Lockstep Mode
// ============================================================================

/// Handle requests in lockstep mode (no rollbacks).
///
/// When `max_prediction_window` is 0, the session operates in lockstep mode.
/// You'll never receive `SaveGameState` or `LoadGameState` requests.
#[allow(unused)]
fn handle_requests_lockstep(
    requests: Vec<FortressRequest<GameConfig>>,
    game_state: &mut GameState,
) {
    handle_requests!(
        requests,
        save: |_, _| {
            // Never called in lockstep mode — no rollbacks
        },
        load: |_, _| {
            // Never called in lockstep mode — no rollbacks
        },
        advance: |inputs: InputVec<GameInput>| {
            game_state.update(inputs.as_slice());
        }
    );
}

// ============================================================================
// Main — Demonstration
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Fortress Rollback Request Handling Example ===\n");

    // Demonstrate both approaches with a simulated session
    println!("This example shows how to handle FortressRequest variants.\n");

    println!("Method 1: Manual Matching");
    println!("  - Full control with explicit match statements");
    println!("  - No wildcard `_ =>` needed (exhaustive enum)");
    println!("  - Good for complex logic or debugging\n");

    println!("Method 2: handle_requests! Macro");
    println!("  - Less boilerplate, same semantics");
    println!("  - Processes requests in order automatically");
    println!("  - Good for typical game loops\n");

    println!("Key Points:");
    println!("  1. FortressRequest is NOT #[non_exhaustive]");
    println!("     → No wildcard arm needed; compiler warns on new variants");
    println!("  2. Use compute_checksum() for desync detection");
    println!("     → Returns Result; use .ok() for cell.save()");
    println!("  3. Always process requests in order\n");

    // Create a minimal session to show the API (won't actually run network)
    println!("--- Creating Example Session ---\n");

    // Bind to a local socket (demonstration only)
    let socket = UdpNonBlockingSocket::bind_to_port(0)?;
    let local_addr = socket.local_addr()?;
    println!("Bound to {}", local_addr);

    // Create a 2-player session with desync detection enabled
    let mut session = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)?
        .with_input_delay(2)?
        .with_max_prediction_window(8)
        .with_desync_detection_mode(DesyncDetection::On { interval: 60 })
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    println!("Session created with desync detection enabled (every 60 frames)\n");

    // Simulate a few frames
    let mut game_state = GameState::default();

    println!("--- Simulating Frames ---\n");

    for frame_num in 0..5 {
        // Add local input for both players
        let input = GameInput {
            buttons: (frame_num % 3) as u8,
        };
        session.add_local_input(PlayerHandle::new(0), input)?;
        session.add_local_input(PlayerHandle::new(1), input)?;

        // Check if session is ready (both "local" players are always ready)
        if session.current_state() == SessionState::Running {
            // Advance and get requests
            let requests = session.advance_frame()?;

            // Handle requests using manual matching
            handle_requests_manual(requests, &mut game_state);
        }
    }

    println!("\n--- Final State ---");
    println!("Frame: {}", game_state.frame);
    println!("Score: {}", game_state.score);
    println!("Player positions: {:?}", game_state.player_x);

    // Compute final checksum
    let final_checksum = compute_checksum(&game_state)?;
    println!("Final checksum: {:#034x}", final_checksum);

    println!("\n=== Example Complete ===");

    Ok(())
}
