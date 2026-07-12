//! Headless network test peer for multi-process testing.
//!
//! This binary runs as a single peer in a P2P session, communicating over
//! real UDP sockets. It can be spawned multiple times by a test runner to
//! validate network behavior under various conditions, including N-player
//! (N >= 3) meshes.
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
//! `--peer` is repeatable: pass it once per remote peer to form an N-player
//! mesh. The number of players is `(number of --peer args) + 1`. Remote handles
//! are assigned as `(0..num_players).filter(|h| h != player_index)` in ascending
//! order, zipped with the `--peer` addresses in the order they are given, so the
//! addresses for a peer must be listed in ascending-remote-handle order. For
//! example, peer at `player_index 1` in a 3-player mesh would pass
//! `--peer <addr of player 0> --peer <addr of player 2>`.
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
    hash::DeterministicHasher, ChaosConfig, ChaosSocket, Config, FortressEvent, FortressRequest,
    Frame, InputStatus, PlayerHandle, PlayerType, ProtocolConfig, RequestVec, SessionBuilder,
    SessionState, SyncConfig, TimeSyncConfig, UdpNonBlockingSocket,
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
#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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

    fn handle_requests(&mut self, requests: RequestVec<TestConfig>) {
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
    /// Number of `FortressEvent::DesyncDetected` events this peer observed.
    ///
    /// Surfaced as a top-level field (in addition to `runtime.events`) so the
    /// driver can read it without depending on `runtime` being present. The
    /// N-peer determinism tests assert this is zero on a clean network: the
    /// historical 0%-loss false positive (once attributed to this harness's
    /// speculative `state.value` checksum) was root-caused in S30 as library
    /// finding F17 -- `InputQueue::input` re-entered a prediction episode at
    /// the requested frame instead of the queue's first missing frame,
    /// silently swallowing misprediction comparisons for the skipped window --
    /// and fixed there, so a nonzero count now indicates a genuine library
    /// regression (see `verify_determinism_n` and the module note in
    /// tests/network/multi_process.rs).
    #[serde(default)]
    desync_detected: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug_log: Option<Vec<DebugEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostics: Option<ChecksumDiagnostics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime: Option<RuntimeDiagnostics>,
}

#[derive(Clone, Default, Serialize)]
struct EventSummary {
    synchronizing: u32,
    synchronized: u32,
    network_interrupted: u32,
    network_resumed: u32,
    disconnected: u32,
    sync_timeout: u32,
    incompatible_session: u32,
    wait_recommendation: u32,
    input_delay_recommendation: u32,
    desync_detected: u32,
    peer_dropped: u32,
    replay_desync: u32,
    spectator_divergence: u32,
    #[cfg(feature = "hot-join")]
    join_requested: u32,
    #[cfg(feature = "hot-join")]
    peer_joined: u32,
}

impl EventSummary {
    fn record(&mut self, event: FortressEvent<TestConfig>) {
        // This match is intentionally exhaustive (no `_ =>` wildcard): it is the
        // compile-time detector for newly added `FortressEvent` variants. Each
        // per-variant counter is a diagnostic surfaced in the peer's JSON output,
        // so a new variant must fail to compile here until it is wired through.
        match event {
            FortressEvent::Synchronizing { .. } => self.synchronizing += 1,
            FortressEvent::Synchronized { .. } => self.synchronized += 1,
            FortressEvent::Disconnected { .. } => self.disconnected += 1,
            FortressEvent::NetworkInterrupted { .. } => self.network_interrupted += 1,
            FortressEvent::NetworkResumed { .. } => self.network_resumed += 1,
            FortressEvent::WaitRecommendation { .. } => self.wait_recommendation += 1,
            FortressEvent::DesyncDetected { .. } => self.desync_detected += 1,
            FortressEvent::SyncTimeout { .. } => self.sync_timeout += 1,
            FortressEvent::IncompatibleSession { .. } => self.incompatible_session += 1,
            FortressEvent::ReplayDesync { .. } => self.replay_desync += 1,
            FortressEvent::InputDelayRecommendation { .. } => {
                self.input_delay_recommendation += 1;
            },
            FortressEvent::PeerDropped { .. } => self.peer_dropped += 1,
            FortressEvent::SpectatorDivergence { .. } => self.spectator_divergence += 1,
            #[cfg(feature = "hot-join")]
            FortressEvent::JoinRequested { .. } => self.join_requested += 1,
            #[cfg(feature = "hot-join")]
            FortressEvent::PeerJoined { .. } => self.peer_joined += 1,
        }
    }
}

#[derive(Serialize)]
struct RuntimeDiagnostics {
    session_state: String,
    current_frame: i32,
    confirmed_frame: i32,
    target_frame: i32,
    elapsed_ms: u128,
    sync_preset: Option<String>,
    sync_config: String,
    protocol_config: String,
    time_sync_config: String,
    sync_health: String,
    events: EventSummary,
}

fn protocol_config_for_preset(preset: Option<&str>) -> ProtocolConfig {
    match preset {
        Some("mobile" | "extreme" | "stress_test") => ProtocolConfig::mobile(),
        Some("high_latency") => ProtocolConfig::high_latency(),
        _ => ProtocolConfig::default(),
    }
}

fn time_sync_config_for_preset(preset: Option<&str>) -> TimeSyncConfig {
    match preset {
        Some("lan") => TimeSyncConfig::lan(),
        Some("competitive") => TimeSyncConfig::competitive(),
        Some("mobile" | "extreme" | "stress_test" | "high_latency") => TimeSyncConfig::mobile(),
        _ => TimeSyncConfig::default(),
    }
}

fn drain_session_events(
    session: &mut fortress_rollback::P2PSession<TestConfig>,
    events: &mut EventSummary,
) {
    for event in session.events() {
        events.record(event);
    }
}

// Aggregates many independent diagnostic fields into a single struct for the
// JSON report; grouping them into sub-structs would not aid this test harness.
#[allow(clippy::too_many_arguments)]
fn runtime_diagnostics(
    session: &fortress_rollback::P2PSession<TestConfig>,
    target_frame: i32,
    start_time: Instant,
    sync_preset: &Option<String>,
    sync_config: SyncConfig,
    protocol_config: &ProtocolConfig,
    time_sync_config: TimeSyncConfig,
    events: &EventSummary,
) -> RuntimeDiagnostics {
    RuntimeDiagnostics {
        session_state: session.current_state().to_string(),
        current_frame: session.current_frame().as_i32(),
        confirmed_frame: session.confirmed_frame().as_i32(),
        target_frame,
        elapsed_ms: start_time.elapsed().as_millis(),
        sync_preset: sync_preset.clone(),
        sync_config: sync_config.to_string(),
        protocol_config: protocol_config.to_string(),
        time_sync_config: time_sync_config.to_string(),
        sync_health: format!("{:?}", session.all_sync_health()),
        events: events.clone(),
    }
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
                // `--peer` is repeatable: each occurrence appends one remote
                // address. A single `--peer` yields a 1-element Vec, keeping
                // the 2-peer call path byte-identical to the original behavior.
                result
                    .peers
                    .push(args[i].parse().expect("Invalid peer address"));
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
    /// Remote peer addresses, in the order received on the command line.
    ///
    /// `--peer` may be passed one or more times. For a 2-player session this is
    /// a single-element `Vec`; for an N-player mesh it holds the `N - 1` remote
    /// peer addresses. The local player is `player_index`; the remote handles
    /// are `(0..num_players).filter(|h| *h != player_index)` in ascending order,
    /// zipped with `peers` in the order received (see `run_test`).
    peers: Vec<SocketAddr>,
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
    if args.peers.is_empty() {
        output_error("--peer is required (one or more times)");
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
        desync_detected: 0,
        error_kind: Some("configuration".to_string()),
        error: Some(msg.to_string()),
        debug_log: None,
        diagnostics: None,
        runtime: None,
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
                desync_detected: 0,
                error_kind: Some("io".to_string()),
                error: Some(format!("Failed to bind socket: {e}")),
                debug_log: None,
                diagnostics: None,
                runtime: None,
            };
        },
    };
    let socket = ChaosSocket::new(inner_socket, chaos_config);

    // Build session. The session has one local player (this process) plus one
    // remote player per `--peer` address, so an N-player mesh is N processes
    // each launched with N-1 `--peer` args.
    let num_players = args.peers.len() + 1;

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
                desync_detected: 0,
                error_kind: Some("configuration".to_string()),
                error: Some(format!(
                    "Unknown sync preset: '{}'. Valid presets: lan, lossy, mobile, high_latency, competitive, extreme, stress_test",
                    preset
                )),
                debug_log: None,
                diagnostics: None,
                runtime: None,
            };
        },
        None => SyncConfig::default(),
    };
    let protocol_config = protocol_config_for_preset(args.sync_preset.as_deref());
    let time_sync_config = time_sync_config_for_preset(args.sync_preset.as_deref());

    let mut sess_builder = SessionBuilder::<TestConfig>::new()
        .with_num_players(num_players)
        .unwrap()
        .with_input_delay(args.input_delay)
        .unwrap()
        .with_sync_config(sync_config)
        .with_protocol_config(protocol_config.clone())
        .with_time_sync_config(time_sync_config);

    // Add players based on our index.
    //
    // Handle <-> address mapping convention (must match the test driver's
    // `n_peer_mesh_configs`): the local player owns `player_index`. The remote
    // handles are every other handle in `0..num_players`, i.e.
    // `(0..num_players).filter(|h| *h != player_index)` in ascending order,
    // zipped with `args.peers` in the order they were received. The driver lists
    // each peer's remote addresses in ascending-remote-handle order, so peer P's
    // K-th `--peer` address is the process listening for handle = the K-th
    // element of that ascending filtered handle sequence. For the 2-player case
    // this reduces to the original `remote_handle = 1 - player_index` (the single
    // handle != player_index), keeping behavior byte-identical.
    let local_handle = PlayerHandle::new(args.player_index);

    sess_builder = match sess_builder.add_player(PlayerType::Local, local_handle) {
        Ok(b) => b,
        Err(e) => {
            return TestResult {
                success: false,
                final_frame: 0,
                final_value: 0,
                checksum: 0,
                rollbacks: 0,
                desync_detected: 0,
                error_kind: Some("configuration".to_string()),
                error: Some(format!("Failed to add local player: {e}")),
                debug_log: None,
                diagnostics: None,
                runtime: None,
            };
        },
    };

    // Ascending remote handles paired with the received peer addresses.
    let remote_handles = (0..num_players).filter(|&h| h != args.player_index);
    for (remote_index, peer_addr) in remote_handles.zip(args.peers.iter().copied()) {
        let remote_handle = PlayerHandle::new(remote_index);
        sess_builder = match sess_builder.add_player(PlayerType::Remote(peer_addr), remote_handle) {
            Ok(b) => b,
            Err(e) => {
                return TestResult {
                    success: false,
                    final_frame: 0,
                    final_value: 0,
                    checksum: 0,
                    rollbacks: 0,
                    desync_detected: 0,
                    error_kind: Some("configuration".to_string()),
                    error: Some(format!(
                        "Failed to add remote player (handle {remote_index}, addr {peer_addr}): {e}"
                    )),
                    debug_log: None,
                    diagnostics: None,
                    runtime: None,
                };
            },
        };
    }

    let mut session = match sess_builder.start_p2p_session(socket) {
        Ok(s) => s,
        Err(e) => {
            return TestResult {
                success: false,
                final_frame: 0,
                final_value: 0,
                checksum: 0,
                rollbacks: 0,
                desync_detected: 0,
                error_kind: Some("session".to_string()),
                error: Some(format!("Failed to start session: {e}")),
                debug_log: None,
                diagnostics: None,
                runtime: None,
            };
        },
    };

    let mut game = TestGame::new(args.debug);
    let mut event_summary = EventSummary::default();
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
            let runtime = runtime_diagnostics(
                &session,
                args.target_frames,
                start_time,
                &args.sync_preset,
                sync_config,
                &protocol_config,
                time_sync_config,
                &event_summary,
            );
            return TestResult {
                success: false,
                final_frame: game.state.frame,
                final_value: game.state.value,
                checksum,
                rollbacks: game.rollback_count,
                desync_detected: event_summary.desync_detected,
                error_kind: Some("timeout".to_string()),
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
                runtime: Some(runtime),
            };
        }

        // Poll network to receive any pending inputs
        session.poll_remote_clients();
        drain_session_events(&mut session, &mut event_summary);

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
                drain_session_events(&mut session, &mut event_summary);
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

            let runtime = runtime_diagnostics(
                &session,
                args.target_frames,
                start_time,
                &args.sync_preset,
                sync_config,
                &protocol_config,
                time_sync_config,
                &event_summary,
            );
            return TestResult {
                success: true,
                final_frame: game.state.frame,
                final_value: confirmed_value, // Use confirmed value, not speculative
                checksum,
                rollbacks: game.rollback_count,
                desync_detected: event_summary.desync_detected,
                error_kind: None,
                error: None,
                debug_log: if args.debug {
                    Some(game.debug_log.entries)
                } else {
                    None
                },
                diagnostics: Some(checksum_diagnostics),
                runtime: Some(runtime),
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
                let runtime = runtime_diagnostics(
                    &session,
                    args.target_frames,
                    start_time,
                    &args.sync_preset,
                    sync_config,
                    &protocol_config,
                    time_sync_config,
                    &event_summary,
                );
                return TestResult {
                    success: false,
                    final_frame: game.state.frame,
                    final_value: game.state.value,
                    checksum,
                    rollbacks: game.rollback_count,
                    desync_detected: event_summary.desync_detected,
                    error_kind: Some("session".to_string()),
                    error: Some(format!("Failed to add input: {e}")),
                    debug_log: if args.debug {
                        Some(game.debug_log.entries)
                    } else {
                        None
                    },
                    diagnostics: Some(diagnostics),
                    runtime: Some(runtime),
                };
            }

            match session.advance_frame() {
                Ok(requests) => {
                    drain_session_events(&mut session, &mut event_summary);
                    game.handle_requests(requests);
                },
                Err(e) => {
                    let (checksum, diagnostics) =
                        compute_confirmed_checksum_with_diagnostics(&session, args.target_frames);
                    let runtime = runtime_diagnostics(
                        &session,
                        args.target_frames,
                        start_time,
                        &args.sync_preset,
                        sync_config,
                        &protocol_config,
                        time_sync_config,
                        &event_summary,
                    );
                    return TestResult {
                        success: false,
                        final_frame: game.state.frame,
                        final_value: game.state.value,
                        checksum,
                        rollbacks: game.rollback_count,
                        desync_detected: event_summary.desync_detected,
                        error_kind: Some("session".to_string()),
                        error: Some(format!("Failed to advance frame: {e}")),
                        debug_log: if args.debug {
                            Some(game.debug_log.entries)
                        } else {
                            None
                        },
                        diagnostics: Some(diagnostics),
                        runtime: Some(runtime),
                    };
                },
            }
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(Duration::from_millis(1));
    }
}
