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
//! cargo run --bin network_test_peer -- \
//!     --local-port 9001 \
//!     --player-index 0 \
//!     --peer 127.0.0.1:9002 \
//!     --frames 100
//!
//! # Start peer 2 (player 1)
//! cargo run --bin network_test_peer -- \
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

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use fortress_rollback::{
    ChaosConfig, ChaosSocket, Config, FortressRequest, InputStatus, PlayerHandle, PlayerType,
    SessionBuilder, SessionState, UdpNonBlockingSocket,
};
use serde::{Deserialize, Serialize};

// Simple deterministic game state for testing
#[derive(Clone, Default, Hash, Serialize, Deserialize)]
struct TestState {
    frame: i32,
    // Accumulator that changes based on inputs
    value: i64,
}

impl TestState {
    fn advance(&mut self, inputs: &[(TestInput, InputStatus)]) {
        self.frame += 1;
        for (i, (input, status)) in inputs.iter().enumerate() {
            match status {
                InputStatus::Confirmed | InputStatus::Predicted => {
                    // Deterministic state update based on player index and input
                    self.value = self.value.wrapping_add(input.value as i64 * (i as i64 + 1));
                }
                InputStatus::Disconnected => {
                    // Disconnected players contribute 0
                }
            }
        }
    }

    fn checksum(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
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
}

impl TestGame {
    fn new() -> Self {
        Self {
            state: TestState::default(),
            rollback_count: 0,
        }
    }

    fn handle_requests(&mut self, requests: Vec<FortressRequest<TestConfig>>) {
        for request in requests {
            match request {
                FortressRequest::LoadGameState { cell, .. } => {
                    self.state = cell.load().unwrap();
                    self.rollback_count += 1;
                }
                FortressRequest::SaveGameState { cell, frame } => {
                    let checksum = self.state.checksum() as u128;
                    cell.save(frame, Some(self.state.clone()), Some(checksum));
                }
                FortressRequest::AdvanceFrame { inputs } => {
                    self.state.advance(&inputs);
                }
            }
        }
    }
}

#[derive(Serialize)]
struct TestResult {
    success: bool,
    final_frame: i32,
    checksum: u64,
    rollbacks: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
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
            }
            "--player-index" => {
                i += 1;
                result.player_index = args[i].parse().expect("Invalid player index");
            }
            "--peer" => {
                i += 1;
                result.peer_addr = Some(args[i].parse().expect("Invalid peer address"));
            }
            "--frames" => {
                i += 1;
                result.target_frames = args[i].parse().expect("Invalid frame count");
            }
            "--packet-loss" => {
                i += 1;
                result.packet_loss = args[i].parse().expect("Invalid packet loss rate");
            }
            "--latency" => {
                i += 1;
                result.latency_ms = args[i].parse().expect("Invalid latency");
            }
            "--jitter" => {
                i += 1;
                result.jitter_ms = args[i].parse().expect("Invalid jitter");
            }
            "--seed" => {
                i += 1;
                result.seed = Some(args[i].parse().expect("Invalid seed"));
            }
            "--timeout" => {
                i += 1;
                result.timeout_secs = args[i].parse().expect("Invalid timeout");
            }
            "--input-delay" => {
                i += 1;
                result.input_delay = args[i].parse().expect("Invalid input delay");
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
            }
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
}

fn main() {
    let args = parse_args();

    if args.local_port == 0 {
        output_error("--local-port is required");
        return;
    }
    if args.peer_addr.is_none() {
        output_error("--peer is required");
        return;
    }
    if args.target_frames == 0 {
        output_error("--frames is required");
        return;
    }

    let result = run_test(&args);
    let json = serde_json::to_string(&result).unwrap();
    println!("{}", json);
    io::stdout().flush().unwrap();
}

fn output_error(msg: &str) {
    let result = TestResult {
        success: false,
        final_frame: 0,
        checksum: 0,
        rollbacks: 0,
        error: Some(msg.to_string()),
    };
    let json = serde_json::to_string(&result).unwrap();
    println!("{}", json);
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
    let chaos_config = chaos_builder.build();

    // Create socket with chaos
    let inner_socket = match UdpNonBlockingSocket::bind_to_port(args.local_port) {
        Ok(s) => s,
        Err(e) => {
            return TestResult {
                success: false,
                final_frame: 0,
                checksum: 0,
                rollbacks: 0,
                error: Some(format!("Failed to bind socket: {}", e)),
            };
        }
    };
    let socket = ChaosSocket::new(inner_socket, chaos_config);

    // Build session
    let peer_addr = args.peer_addr.unwrap();
    let num_players = 2; // Currently only supports 2-player testing

    let mut sess_builder = SessionBuilder::<TestConfig>::new()
        .with_num_players(num_players)
        .with_input_delay(args.input_delay);

    // Add players based on our index
    let local_handle = PlayerHandle::new(args.player_index);
    let remote_handle = PlayerHandle::new(if args.player_index == 0 { 1 } else { 0 });

    sess_builder = match sess_builder.add_player(PlayerType::Local, local_handle) {
        Ok(b) => b,
        Err(e) => {
            return TestResult {
                success: false,
                final_frame: 0,
                checksum: 0,
                rollbacks: 0,
                error: Some(format!("Failed to add local player: {}", e)),
            };
        }
    };

    sess_builder = match sess_builder.add_player(PlayerType::Remote(peer_addr), remote_handle) {
        Ok(b) => b,
        Err(e) => {
            return TestResult {
                success: false,
                final_frame: 0,
                checksum: 0,
                rollbacks: 0,
                error: Some(format!("Failed to add remote player: {}", e)),
            };
        }
    };

    let mut session = match sess_builder.start_p2p_session(socket) {
        Ok(s) => s,
        Err(e) => {
            return TestResult {
                success: false,
                final_frame: 0,
                checksum: 0,
                rollbacks: 0,
                error: Some(format!("Failed to start session: {}", e)),
            };
        }
    };

    let mut game = TestGame::new();
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
            return TestResult {
                success: false,
                final_frame: game.state.frame,
                checksum: game.state.checksum(),
                rollbacks: game.rollback_count,
                error: Some("Timeout".to_string()),
            };
        }

        // Poll network
        session.poll_remote_clients();

        // Check for completion
        if game.state.frame >= args.target_frames {
            return TestResult {
                success: true,
                final_frame: game.state.frame,
                checksum: game.state.checksum(),
                rollbacks: game.rollback_count,
                error: None,
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
                return TestResult {
                    success: false,
                    final_frame: game.state.frame,
                    checksum: game.state.checksum(),
                    rollbacks: game.rollback_count,
                    error: Some(format!("Failed to add input: {}", e)),
                };
            }

            match session.advance_frame() {
                Ok(requests) => game.handle_requests(requests),
                Err(e) => {
                    return TestResult {
                        success: false,
                        final_frame: game.state.frame,
                        checksum: game.state.checksum(),
                        rollbacks: game.rollback_count,
                        error: Some(format!("Failed to advance frame: {}", e)),
                    };
                }
            }
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(Duration::from_millis(1));
    }
}
