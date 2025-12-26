//! Headless network test peer for multi-process testing.
//!
//! This binary runs as a single peer in a P2P session, communicating over
//! real UDP sockets. It can be spawned multiple times by a test runner to
//! validate network behavior under various conditions.
//!
//! # Usage
//!
//! ```bash
//! # Start peer 1 (player 0)
//! cargo run -p network-test-peer -- \
//!     --local-port 9001 \
//!     --player-index 0 \
//!     --peer 127.0.0.1:9002 \
//!     --frames 100
//!
//! # Start peer 2 (player 1)
//! cargo run -p network-test-peer -- \
//!     --local-port 9002 \
//!     --player-index 1 \
//!     --peer 127.0.0.1:9001 \
//!     --frames 100
//! ```
//!
//! # Chaos Options
//!
//! ```bash
//! --packet-loss 0.1       # 10% packet loss
//! --latency 50            # 50ms latency
//! --jitter 20             # ±20ms jitter
//! --seed 42               # Deterministic chaos
//! ```
//!
//! # Output
//!
//! On success, outputs JSON with results:
//! ```json
//! {"success": true, "final_frame": 100, "checksum": 12345, "rollbacks": 5}
//! ```

use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use fortress_rollback::{
    hash::DeterministicHasher, ChaosConfig, ChaosSocket, Config, FortressRequest, Frame,
    InputStatus, PlayerHandle, PlayerType, SessionBuilder, SessionState, SyncConfig,
    UdpNonBlockingSocket,
};
use serde::{Deserialize, Serialize};

// Simple deterministic game state for testing
#[derive(Clone, Default, Hash, Serialize, Deserialize)]
struct TestState {
    // The current frame number (for display/debugging)
    frame: i32,
    // Accumulator that changes based on ALL inputs (including predictions)
    // Used for smooth gameplay display
    value: i64,
}

/// Debug log entry for tracing state progression
#[derive(Serialize)]
struct DebugEntry {
    event: String,
    frame: i32,
    value: i64,
    inputs: Vec<(u32, String)>, // (input_value, status)
}

/// Debug log for the entire session
struct DebugLog {
    entries: Vec<DebugEntry>,
    enabled: bool,
}

impl DebugLog {
    fn new(enabled: bool) -> Self {
        Self {
            entries: Vec::new(),
            enabled,
        }
    }

    fn log_advance(&mut self, frame: i32, value: i64, inputs: &[(TestInput, InputStatus)]) {
        if !self.enabled {
            return;
        }
        let input_entries: Vec<(u32, String)> = inputs
            .iter()
            .map(|(input, status)| {
                let status_str = match status {
                    InputStatus::Confirmed => "confirmed",
                    InputStatus::Predicted => "predicted",
                    InputStatus::Disconnected => "disconnected",
                };
                (input.value, status_str.to_string())
            })
            .collect();
        self.entries.push(DebugEntry {
            event: "advance".to_string(),
            frame,
            value,
            inputs: input_entries,
        });
    }

    fn log_load(&mut self, frame: i32, value: i64) {
        if !self.enabled {
            return;
        }
        self.entries.push(DebugEntry {
            event: "load".to_string(),
            frame,
            value,
            inputs: Vec::new(),
        });
    }

    fn log_save(&mut self, frame: i32, value: i64) {
        if !self.enabled {
            return;
        }
        self.entries.push(DebugEntry {
            event: "save".to_string(),
            frame,
            value,
            inputs: Vec::new(),
        });
    }
}

impl TestState {
    fn advance(&mut self, inputs: &[(TestInput, InputStatus)]) {
        for (i, (input, status)) in inputs.iter().enumerate() {
            match status {
                InputStatus::Confirmed | InputStatus::Predicted => {
                    // Update display value with all inputs (for smooth gameplay)
                    self.value = self.value.wrapping_add(input.value as i64 * (i as i64 + 1));
                },
                InputStatus::Disconnected => {
                    // Disconnected players contribute 0
                },
            }
        }
        self.frame += 1;
    }
}

/// Information about the checksum computation for diagnostic purposes.
#[derive(Serialize)]
struct ChecksumDiagnostics {
    start_frame: i32,
    end_frame: i32,
    frames_included: i32,
    frames_missing: Vec<i32>,
    confirmed_frame: i32,
    /// The session's current frame when the checksum was computed.
    /// This helps diagnose issues where frames were discarded due to session advancement.
    current_frame: i32,
}

/// Compute a checksum from confirmed inputs for a recent window of frames.
/// This is deterministic because both peers have the same confirmed inputs,
/// and we only use frames that are guaranteed to be in the input queue.
///
/// Returns the checksum and diagnostic information about which frames were included.
fn compute_confirmed_checksum_with_diagnostics<
    T: Config<Input = TestInput, Address = SocketAddr>,
>(
    session: &fortress_rollback::P2PSession<T>,
    target_frames: i32,
) -> (u64, ChecksumDiagnostics) {
    let mut hasher = DeterministicHasher::new();

    // Use a window of the last 64 frames (half of input queue capacity).
    // This ensures the frames are still available in the queue.
    const WINDOW_SIZE: i32 = 64;
    let start_frame = std::cmp::max(0, target_frames - WINDOW_SIZE);

    let mut frames_included = 0;
    let mut frames_missing = Vec::new();

    for frame_num in start_frame..target_frames {
        let frame = Frame::new(frame_num);
        match session.confirmed_inputs_for_frame(frame) {
            Ok(inputs) => {
                frames_included += 1;
                // Hash each player's input for this frame
                for (player_idx, input) in inputs.iter().enumerate() {
                    // Hash player index, frame, and input value
                    (player_idx as u32).hash(&mut hasher);
                    frame_num.hash(&mut hasher);
                    input.value.hash(&mut hasher);
                }
            },
            Err(_) => {
                frames_missing.push(frame_num);
            },
        }
    }

    let diagnostics = ChecksumDiagnostics {
        start_frame,
        end_frame: target_frames,
        frames_included,
        frames_missing,
        confirmed_frame: session.confirmed_frame().as_i32(),
        current_frame: session.current_frame().as_i32(),
    };

    (hasher.finish(), diagnostics)
}

/// Compute the game state value from confirmed inputs only.
/// This is deterministic because both peers have the same confirmed inputs.
///
/// We compute over a RECENT window of frames rather than all frames, because
/// older frames may have been discarded from the input queue. The window size
/// is chosen to be well within the input queue capacity (128 frames).
///
/// Returns the computed value and diagnostic information.
fn compute_confirmed_game_value_with_diagnostics<
    T: Config<Input = TestInput, Address = SocketAddr>,
>(
    session: &fortress_rollback::P2PSession<T>,
    target_frames: i32,
) -> (i64, ChecksumDiagnostics) {
    let mut value: i64 = 0;

    // Use a window of the last 64 frames (half of input queue capacity).
    // This ensures the frames are still available in the queue.
    const WINDOW_SIZE: i32 = 64;
    let start_frame = std::cmp::max(0, target_frames - WINDOW_SIZE);

    let mut frames_included = 0;
    let mut frames_missing = Vec::new();

    for frame_num in start_frame..target_frames {
        let frame = Frame::new(frame_num);
        match session.confirmed_inputs_for_frame(frame) {
            Ok(inputs) => {
                frames_included += 1;
                // Apply each player's input using the same formula as TestState::advance
                for (i, input) in inputs.iter().enumerate() {
                    value = value.wrapping_add(input.value as i64 * (i as i64 + 1));
                }
            },
            Err(_) => {
                frames_missing.push(frame_num);
            },
        }
    }

    let diagnostics = ChecksumDiagnostics {
        start_frame,
        end_frame: target_frames,
        frames_included,
        frames_missing,
        confirmed_frame: session.confirmed_frame().as_i32(),
        current_frame: session.current_frame().as_i32(),
    };

    (value, diagnostics)
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
struct TestInput {
    value: u32,
}

struct TestConfig;
impl Config for TestConfig {
    type Input = TestInput;
    type State = TestState;
    type Address = SocketAddr;
}

struct TestGame {
    state: TestState,
    rollback_count: u32,
    debug_log: DebugLog,
}

impl TestGame {
    fn new(debug_enabled: bool) -> Self {
        Self {
            state: TestState::default(),
            rollback_count: 0,
            debug_log: DebugLog::new(debug_enabled),
        }
    }

    fn handle_requests(&mut self, requests: Vec<FortressRequest<TestConfig>>) {
        for request in requests {
            match request {
                FortressRequest::LoadGameState { cell, frame } => {
                    self.state = cell.load().unwrap();
                    self.rollback_count += 1;
                    self.debug_log.log_load(frame.as_i32(), self.state.value);
                },
                FortressRequest::SaveGameState { cell, frame } => {
                    // Use a simple checksum for save - not used for final comparison
                    let checksum = self.state.value as u128;
                    self.debug_log.log_save(frame.as_i32(), self.state.value);
                    cell.save(frame, Some(self.state.clone()), Some(checksum));
                },
                FortressRequest::AdvanceFrame { inputs } => {
                    // Log BEFORE advancing so we can see the inputs that are being used
                    self.debug_log
                        .log_advance(self.state.frame, self.state.value, &inputs);
                    self.state.advance(&inputs);
                },
                _ => unreachable!("Unknown request type"),
            }
        }
    }
}

#[derive(Serialize)]
struct TestResult {
    success: bool,
    final_frame: i32,
    final_value: i64, // Added for debugging
    checksum: u64,
    rollbacks: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug_log: Option<Vec<DebugEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostics: Option<ChecksumDiagnostics>,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut result = Args::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--local-port" => {
                i += 1;
                result.local_port = args[i].parse().expect("Invalid port");
            },
            "--player-index" => {
                i += 1;
                result.player_index = args[i].parse().expect("Invalid player index");
            },
            "--peer" => {
                i += 1;
                result.peer_addr = Some(args[i].parse().expect("Invalid peer address"));
            },
            "--frames" => {
                i += 1;
                result.target_frames = args[i].parse().expect("Invalid frame count");
            },
            "--packet-loss" => {
                i += 1;
                result.packet_loss = args[i].parse().expect("Invalid packet loss rate");
            },
            "--latency" => {
                i += 1;
                result.latency_ms = args[i].parse().expect("Invalid latency");
            },
            "--jitter" => {
                i += 1;
                result.jitter_ms = args[i].parse().expect("Invalid jitter");
            },
            "--seed" => {
                i += 1;
                result.seed = Some(args[i].parse().expect("Invalid seed"));
            },
            "--timeout" => {
                i += 1;
                result.timeout_secs = args[i].parse().expect("Invalid timeout");
            },
            "--input-delay" => {
                i += 1;
                result.input_delay = args[i].parse().expect("Invalid input delay");
            },
            "--debug" => {
                result.debug = true;
            },
            "--reorder-rate" => {
                i += 1;
                result.reorder_rate = args[i].parse().expect("Invalid reorder rate");
            },
            "--reorder-buffer" => {
                i += 1;
                result.reorder_buffer_size = args[i].parse().expect("Invalid reorder buffer size");
            },
            "--duplicate-rate" => {
                i += 1;
                result.duplicate_rate = args[i].parse().expect("Invalid duplicate rate");
            },
            "--burst-loss-prob" => {
                i += 1;
                result.burst_loss_prob = args[i].parse().expect("Invalid burst loss probability");
            },
            "--burst-loss-len" => {
                i += 1;
                result.burst_loss_len = args[i].parse().expect("Invalid burst loss length");
            },
            "--sync-preset" => {
                i += 1;
                result.sync_preset = Some(args[i].clone());
            },
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
            },
        }
        i += 1;
    }

    result
}

#[derive(Default)]
struct Args {
    local_port: u16,
    player_index: usize,
    peer_addr: Option<SocketAddr>,
    target_frames: i32,
    packet_loss: f64,
    latency_ms: u64,
    jitter_ms: u64,
    seed: Option<u64>,
    timeout_secs: u64,
    input_delay: usize,
    debug: bool,
    // Extended chaos options
    reorder_rate: f64,
    reorder_buffer_size: usize,
    duplicate_rate: f64,
    burst_loss_prob: f64,
    burst_loss_len: usize,
    // Sync configuration preset
    sync_preset: Option<String>,
}

fn main() {
    let args = parse_args();

    if args.local_port == 0 {
        output_error("--local-port is required");
        std::process::exit(1);
    }
    if args.peer_addr.is_none() {
        output_error("--peer is required");
        std::process::exit(1);
    }
    if args.target_frames == 0 {
        output_error("--frames is required");
        std::process::exit(1);
    }

    let result = run_test(&args);
    let json = serde_json::to_string(&result).unwrap();
    println!("{json}");
    io::stdout().flush().unwrap();

    // Exit with non-zero code on failure so docker compose can detect it
    if !result.success {
        std::process::exit(1);
    }
}

fn output_error(msg: &str) {
    let result = TestResult {
        success: false,
        final_frame: 0,
        final_value: 0,
        checksum: 0,
        rollbacks: 0,
        error: Some(msg.to_string()),
        debug_log: None,
        diagnostics: None,
    };
    let json = serde_json::to_string(&result).unwrap();
    println!("{json}");
}

fn run_test(args: &Args) -> TestResult {
    // Build chaos config
    let mut chaos_builder = ChaosConfig::builder();
    if args.packet_loss > 0.0 {
        chaos_builder = chaos_builder.packet_loss_rate(args.packet_loss);
    }
    if args.latency_ms > 0 {
        chaos_builder = chaos_builder.latency_ms(args.latency_ms);
    }
    if args.jitter_ms > 0 {
        chaos_builder = chaos_builder.jitter_ms(args.jitter_ms);
    }
    if let Some(seed) = args.seed {
        chaos_builder = chaos_builder.seed(seed);
    }
    // Extended chaos options
    if args.reorder_rate > 0.0 {
        chaos_builder = chaos_builder.reorder_rate(args.reorder_rate);
    }
    if args.reorder_buffer_size > 0 {
        chaos_builder = chaos_builder.reorder_buffer_size(args.reorder_buffer_size);
    }
    if args.duplicate_rate > 0.0 {
        chaos_builder = chaos_builder.duplication_rate(args.duplicate_rate);
    }
    if args.burst_loss_prob > 0.0 {
        chaos_builder = chaos_builder.burst_loss(args.burst_loss_prob, args.burst_loss_len);
    }
    let chaos_config = chaos_builder.build();

    // Create socket with chaos
    let inner_socket = match UdpNonBlockingSocket::bind_to_port(args.local_port) {
        Ok(s) => s,
        Err(e) => {
            return TestResult {
                success: false,
                final_frame: 0,
                final_value: 0,
                checksum: 0,
                rollbacks: 0,
                error: Some(format!("Failed to bind socket: {e}")),
                debug_log: None,
                diagnostics: None,
            };
        },
    };
    let socket = ChaosSocket::new(inner_socket, chaos_config);

    // Build session
    let peer_addr = args.peer_addr.unwrap();
    let num_players = 2; // Currently only supports 2-player testing

    // Select sync config preset based on network conditions
    let sync_config = match args.sync_preset.as_deref() {
        Some("lan") => SyncConfig::lan(),
        Some("lossy") => SyncConfig::lossy(),
        Some("mobile") => SyncConfig::mobile(),
        Some("high_latency") => SyncConfig::high_latency(),
        Some("competitive") => SyncConfig::competitive(),
        Some("extreme") => SyncConfig::extreme(),
        Some("stress_test") => SyncConfig::stress_test(),
        Some(preset) => {
            return TestResult {
                success: false,
                final_frame: 0,
                final_value: 0,
                checksum: 0,
                rollbacks: 0,
                error: Some(format!(
                    "Unknown sync preset: '{}'. Valid presets: lan, lossy, mobile, high_latency, competitive, extreme, stress_test",
                    preset
                )),
                debug_log: None,
                diagnostics: None,
            };
        },
        None => SyncConfig::default(),
    };

    let mut sess_builder = SessionBuilder::<TestConfig>::new()
        .with_num_players(num_players)
        .unwrap()
        .with_input_delay(args.input_delay)
        .unwrap()
        .with_sync_config(sync_config);

    // Add players based on our index
    let local_handle = PlayerHandle::new(args.player_index);
    // For a 2-player session, get the other player's index (0 -> 1, 1 -> 0)
    let remote_handle = PlayerHandle::new(1 - args.player_index);

    sess_builder = match sess_builder.add_player(PlayerType::Local, local_handle) {
        Ok(b) => b,
        Err(e) => {
            return TestResult {
                success: false,
                final_frame: 0,
                final_value: 0,
                checksum: 0,
                rollbacks: 0,
                error: Some(format!("Failed to add local player: {e}")),
                debug_log: None,
                diagnostics: None,
            };
        },
    };

    sess_builder = match sess_builder.add_player(PlayerType::Remote(peer_addr), remote_handle) {
        Ok(b) => b,
        Err(e) => {
            return TestResult {
                success: false,
                final_frame: 0,
                final_value: 0,
                checksum: 0,
                rollbacks: 0,
                error: Some(format!("Failed to add remote player: {e}")),
                debug_log: None,
                diagnostics: None,
            };
        },
    };

    let mut session = match sess_builder.start_p2p_session(socket) {
        Ok(s) => s,
        Err(e) => {
            return TestResult {
                success: false,
                final_frame: 0,
                final_value: 0,
                checksum: 0,
                rollbacks: 0,
                error: Some(format!("Failed to start session: {e}")),
                debug_log: None,
                diagnostics: None,
            };
        },
    };

    let mut game = TestGame::new(args.debug);
    let start_time = Instant::now();
    let timeout = Duration::from_secs(if args.timeout_secs > 0 {
        args.timeout_secs
    } else {
        60
    });

    // Main loop
    loop {
        // Check timeout
        if start_time.elapsed() > timeout {
            // Compute checksum from confirmed inputs (even though we timed out)
            let (checksum, diagnostics) =
                compute_confirmed_checksum_with_diagnostics(&session, args.target_frames);
            return TestResult {
                success: false,
                final_frame: game.state.frame,
                final_value: game.state.value,
                checksum,
                rollbacks: game.rollback_count,
                error: Some(format!(
                    "Timeout (current_frame={}, confirmed_frame={}, target={})",
                    session.current_frame(),
                    session.confirmed_frame(),
                    args.target_frames
                )),
                debug_log: if args.debug {
                    Some(game.debug_log.entries)
                } else {
                    None
                },
                diagnostics: Some(diagnostics),
            };
        }

        // Poll network to receive any pending inputs
        session.poll_remote_clients();

        // Check for completion:
        // We're done when:
        // 1. Game state frame >= target
        // 2. All inputs up to target are confirmed
        // 3. We've continued polling to allow final rollbacks to complete
        //
        // The settle period ensures that if one peer received inputs slightly later
        // and triggered a rollback, we give time for that rollback to complete.
        let confirmed = session.confirmed_frame();
        if game.state.frame >= args.target_frames && confirmed.as_i32() >= args.target_frames {
            // IMPORTANT: Compute checksum BEFORE the settle phase!
            // The settle phase may advance frames, causing set_last_confirmed_frame to
            // discard inputs from the queue. We need to capture the checksum while
            // all frames from [target_frames - 64, target_frames) are still available.
            //
            // The checksum is deterministic because:
            // 1. confirmed_frame >= target_frames means all inputs up to target are confirmed
            // 2. Both peers have the same confirmed inputs for these frames
            // 3. We compute the checksum over a fixed window [target_frames - 64, target_frames)
            let (checksum, checksum_diagnostics) =
                compute_confirmed_checksum_with_diagnostics(&session, args.target_frames);

            let (confirmed_value, value_diagnostics) =
                compute_confirmed_game_value_with_diagnostics::<TestConfig>(
                    &session,
                    args.target_frames,
                );

            // Continue polling for a settle period to ensure rollbacks complete.
            // During settle, we ONLY poll for network messages - we don't advance new frames.
            // This ensures both peers have finished processing all pending messages.
            // Note: 500ms is generous to handle slow CI VMs where scheduling delays can be significant.
            //
            // IMPORTANT: We do NOT call advance_frame() during settle anymore!
            // Advancing frames causes set_last_confirmed_frame to be called, which discards
            // old inputs from the queue. This was causing checksum mismatches because peers
            // would advance different amounts based on latency, resulting in different
            // frames being available when computing the checksum.
            let settle_start = Instant::now();
            let settle_duration = Duration::from_millis(500);

            while settle_start.elapsed() < settle_duration {
                session.poll_remote_clients();
                std::thread::sleep(Duration::from_millis(5));
            }

            // Warn if diagnostics show missing frames (this helps debug desync issues)
            if !checksum_diagnostics.frames_missing.is_empty() {
                eprintln!(
                    "WARNING: Missing {} frames in checksum computation: {:?}",
                    checksum_diagnostics.frames_missing.len(),
                    checksum_diagnostics.frames_missing
                );
            }

            // Sanity check: value_diagnostics and checksum_diagnostics should match
            if value_diagnostics.frames_included != checksum_diagnostics.frames_included {
                eprintln!(
                    "WARNING: Diagnostics mismatch - value included {} frames, checksum included {}",
                    value_diagnostics.frames_included, checksum_diagnostics.frames_included
                );
            }

            return TestResult {
                success: true,
                final_frame: game.state.frame,
                final_value: confirmed_value, // Use confirmed value, not speculative
                checksum,
                rollbacks: game.rollback_count,
                error: None,
                debug_log: if args.debug {
                    Some(game.debug_log.entries)
                } else {
                    None
                },
                diagnostics: Some(checksum_diagnostics),
            };
        }

        // Only advance if running
        if session.current_state() == SessionState::Running {
            // Generate deterministic input based on session's current frame (not game state frame!)
            // This is critical: game.state.frame gets rewound during rollbacks, but input generation
            // must use the session's frame to ensure deterministic behavior across peers.
            let session_frame = session.current_frame().as_i32();
            let input = TestInput {
                value: (session_frame as u32).wrapping_mul(args.player_index as u32 + 1),
            };

            if let Err(e) = session.add_local_input(local_handle, input) {
                let (checksum, diagnostics) =
                    compute_confirmed_checksum_with_diagnostics(&session, args.target_frames);
                return TestResult {
                    success: false,
                    final_frame: game.state.frame,
                    final_value: game.state.value,
                    checksum,
                    rollbacks: game.rollback_count,
                    error: Some(format!("Failed to add input: {e}")),
                    debug_log: if args.debug {
                        Some(game.debug_log.entries)
                    } else {
                        None
                    },
                    diagnostics: Some(diagnostics),
                };
            }

            match session.advance_frame() {
                Ok(requests) => game.handle_requests(requests),
                Err(e) => {
                    let (checksum, diagnostics) =
                        compute_confirmed_checksum_with_diagnostics(&session, args.target_frames);
                    return TestResult {
                        success: false,
                        final_frame: game.state.frame,
                        final_value: game.state.value,
                        checksum,
                        rollbacks: game.rollback_count,
                        error: Some(format!("Failed to advance frame: {e}")),
                        debug_log: if args.debug {
                            Some(game.debug_log.entries)
                        } else {
                            None
                        },
                        diagnostics: Some(diagnostics),
                    };
                },
            }
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(Duration::from_millis(1));
    }
}
