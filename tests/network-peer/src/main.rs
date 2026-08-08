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
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
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

/// Compute a best-effort checksum from confirmed inputs still retained in a
/// recent frame window.
///
/// Confirmation advancement may already have evicted some frames, so this is
/// diagnostic-only and reports every missing frame. Successful determinism
/// checks use the retained target-state checksum instead.
fn compute_confirmed_checksum_with_diagnostics<
    T: Config<Input = TestInput, Address = SocketAddr>,
>(
    session: &fortress_rollback::P2PSession<T>,
    target_frames: i32,
) -> (u64, ChecksumDiagnostics) {
    let mut hasher = DeterministicHasher::new();

    // Limit diagnostics to the last 64 frames (half of input queue capacity).
    // Older confirmed frames may already have been evicted.
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

/// Returns whether the peer has simulated the requested number of frames and
/// confirmed every input in that exclusive frame range.
///
/// `target_frames` is a frame count: reaching game frame `N` means the game
/// simulated input frames `[0, N)`, so the last required confirmed input is
/// `N - 1`. Invalid non-positive targets never satisfy the oracle.
fn target_completion_reached(game_frame: i32, confirmed_frame: Frame, target_frames: i32) -> bool {
    let Some(last_required_input_frame) = target_frames.checked_sub(1) else {
        return false;
    };

    target_frames > 0
        && game_frame >= target_frames
        && confirmed_frame.as_i32() >= last_required_input_frame
}

const COMPLETION_BARRIER_MAGIC: [u8; 4] = *b"FTRB";
const COMPLETION_BARRIER_IO_SLICE: Duration = Duration::from_secs(1);
const COMPLETION_BARRIER_MESSAGE_LEN: usize = 9;
const COMPLETION_READY: u8 = 1;
const COMPLETION_ACK: u8 = 2;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CompletionMessage {
    Ready { player_index: usize },
    Ack { player_index: usize },
}

impl CompletionMessage {
    fn encode(self) -> io::Result<[u8; COMPLETION_BARRIER_MESSAGE_LEN]> {
        let (kind, player_index) = match self {
            Self::Ready { player_index } => (COMPLETION_READY, player_index),
            Self::Ack { player_index } => (COMPLETION_ACK, player_index),
        };
        let player_index = u32::try_from(player_index).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "completion barrier player index exceeds u32",
            )
        })?;
        let mut message = [0_u8; COMPLETION_BARRIER_MESSAGE_LEN];
        message[..4].copy_from_slice(&COMPLETION_BARRIER_MAGIC);
        message[4] = kind;
        message[5..].copy_from_slice(&player_index.to_le_bytes());
        Ok(message)
    }

    fn decode(message: [u8; COMPLETION_BARRIER_MESSAGE_LEN]) -> io::Result<Self> {
        if message[..4] != COMPLETION_BARRIER_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "completion barrier received invalid magic",
            ));
        }
        let player_index = u32::from_le_bytes([message[5], message[6], message[7], message[8]]);
        let player_index = usize::try_from(player_index).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "completion barrier player index exceeds usize",
            )
        })?;
        match message[4] {
            COMPLETION_READY => Ok(Self::Ready { player_index }),
            COMPLETION_ACK => Ok(Self::Ack { player_index }),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "completion barrier received invalid message kind",
            )),
        }
    }
}

/// Pure readiness state for the test-only multi-process completion barrier.
///
/// A peer may exit only after every participant has independently reached the
/// exclusive target-frame oracle and every pair has completed its same-stream
/// READY/ACK exchange. Keeping this state separate from the socket driver makes
/// incomplete, duplicate, reordered, and invalid exchanges deterministic to
/// test.
struct CompletionBarrierState {
    ready: Vec<bool>,
}

impl CompletionBarrierState {
    fn new(num_players: usize) -> Option<Self> {
        if num_players == 0 {
            return None;
        }

        Some(Self {
            ready: vec![false; num_players],
        })
    }

    fn observe_ready(&mut self, player_index: usize) -> bool {
        let Some(ready) = self.ready.get_mut(player_index) else {
            return false;
        };
        let newly_ready = !*ready;
        *ready = true;
        newly_ready
    }

    fn inbound_complete(&self) -> bool {
        self.ready.iter().all(|ready| *ready)
    }

    fn exchange_complete(&self, announced_to_peer: &[bool]) -> bool {
        let expected_remote_count = self.ready.len().saturating_sub(1);
        self.inbound_complete()
            && announced_to_peer.len() == expected_remote_count
            && announced_to_peer.iter().all(|announced| *announced)
    }

    fn pending_players(&self) -> Vec<usize> {
        self.ready
            .iter()
            .enumerate()
            .filter_map(|(index, ready)| (!ready).then_some(index))
            .collect()
    }
}

/// Test-only completion exchange over TCP on the same numeric ports used by
/// the production UDP session. TCP and UDP have independent port namespaces,
/// so this cannot consume or perturb production protocol packets.
///
/// The listener is active for the whole run. Once the local completion oracle
/// is satisfied, the lower player index initiates one same-stream READY/ACK
/// exchange per pair. Directed initiation avoids mutual connect/read deadlock;
/// an initiator retries until it reads the validated ACK, while the receiver
/// counts the pair only after its bounded ACK write succeeds.
struct CompletionBarrier {
    listener: TcpListener,
    peers: Vec<SocketAddr>,
    acknowledged_by_peer: Vec<bool>,
    local_player: usize,
    state: CompletionBarrierState,
    last_io_error: Option<String>,
}

impl CompletionBarrier {
    fn read_message(stream: &mut TcpStream, io_timeout: Duration) -> io::Result<CompletionMessage> {
        // Accepted-stream nonblocking inheritance differs across platforms.
        // Force blocking mode explicitly, then bound that blocking read by the
        // remaining completion-barrier slice. This allows a valid split write
        // to finish without an immediate WouldBlock while preserving the
        // caller's existing overall deadline.
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(io_timeout))?;
        let mut message = [0_u8; COMPLETION_BARRIER_MESSAGE_LEN];
        stream.read_exact(&mut message)?;
        CompletionMessage::decode(message)
    }

    fn bind(args: &Args, num_players: usize) -> io::Result<Self> {
        let local_addr = SocketAddr::from(([127, 0, 0, 1], args.local_port));
        let listener = TcpListener::bind(local_addr)?;
        listener.set_nonblocking(true)?;
        if args.player_index >= num_players {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "completion barrier player {} is outside 0..{}",
                    args.player_index, num_players
                ),
            ));
        }
        let state = CompletionBarrierState::new(num_players).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "completion barrier requires at least one player",
            )
        })?;

        Ok(Self {
            listener,
            peers: args.peers.clone(),
            acknowledged_by_peer: vec![false; args.peers.len()],
            local_player: args.player_index,
            state,
            last_io_error: None,
        })
    }

    fn poll(&mut self, local_ready: bool, remaining_budget: Duration) {
        let poll_start = Instant::now();
        let Some(poll_budget) = Self::bounded_io_timeout(remaining_budget) else {
            self.last_io_error =
                Some("completion barrier exhausted its existing timeout".to_string());
            return;
        };
        loop {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    let Some(io_timeout) = Self::io_timeout(poll_start, poll_budget) else {
                        self.last_io_error =
                            Some("completion barrier exhausted its existing timeout".to_string());
                        break;
                    };
                    match Self::read_message(&mut stream, io_timeout) {
                        Ok(CompletionMessage::Ready { player_index }) => {
                            if local_ready && player_index < self.local_player {
                                if let Some(peer_index) = self.remote_peer_index(player_index) {
                                    let ack = CompletionMessage::Ack {
                                        player_index: self.local_player,
                                    };
                                    match Self::write_message(
                                        &mut stream,
                                        ack,
                                        poll_start,
                                        poll_budget,
                                    ) {
                                        Ok(()) => {
                                            self.state.observe_ready(player_index);
                                            self.acknowledged_by_peer[peer_index] = true;
                                        },
                                        Err(error) => {
                                            self.last_io_error = Some(format!(
                                                "completion barrier failed to ACK player {player_index}: {error}"
                                            ));
                                        },
                                    }
                                }
                            }
                        },
                        Ok(CompletionMessage::Ack { .. }) => {
                            self.last_io_error = Some(
                                "completion barrier received ACK without a READY request"
                                    .to_string(),
                            );
                        },
                        Err(error) => {
                            self.last_io_error = Some(format!(
                                "completion barrier failed to read announcement: {error}"
                            ));
                        },
                    }
                },
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => {
                    self.last_io_error =
                        Some(format!("completion barrier failed to accept: {error}"));
                    break;
                },
            }
        }

        if local_ready {
            self.state.observe_ready(self.local_player);
            self.announce_to_higher_players(poll_start, poll_budget);
        }
    }

    fn announce_to_higher_players(&mut self, poll_start: Instant, remaining_budget: Duration) {
        for (peer_index, peer_addr) in self.peers.iter().copied().enumerate() {
            let remote_player = self.remote_player_index(peer_index);
            if remote_player < self.local_player || self.acknowledged_by_peer[peer_index] {
                continue;
            }
            match Self::send_ready_and_read_ack(
                peer_addr,
                self.local_player,
                remote_player,
                poll_start,
                remaining_budget,
            ) {
                Ok(()) => {
                    self.state.observe_ready(remote_player);
                    self.acknowledged_by_peer[peer_index] = true;
                },
                Err(error) => {
                    self.last_io_error = Some(format!(
                        "completion barrier READY/ACK with {peer_addr} failed: {error}"
                    ));
                },
            }
        }
    }

    fn send_ready_and_read_ack(
        peer_addr: SocketAddr,
        local_player: usize,
        expected_remote_player: usize,
        poll_start: Instant,
        remaining_budget: Duration,
    ) -> io::Result<()> {
        let io_timeout = Self::io_timeout(poll_start, remaining_budget).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "completion barrier exhausted its existing timeout",
            )
        })?;
        let mut stream = TcpStream::connect_timeout(&peer_addr, io_timeout)?;
        Self::write_message(
            &mut stream,
            CompletionMessage::Ready {
                player_index: local_player,
            },
            poll_start,
            remaining_budget,
        )?;
        let read_timeout = Self::io_timeout(poll_start, remaining_budget).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "completion barrier exhausted its existing timeout",
            )
        })?;
        match Self::read_message(&mut stream, read_timeout)? {
            CompletionMessage::Ack { player_index }
                if player_index == expected_remote_player => Ok(()),
            CompletionMessage::Ack { player_index } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "completion barrier expected ACK from player {expected_remote_player}, got {player_index}"
                ),
            )),
            CompletionMessage::Ready { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "completion barrier expected ACK, got READY",
            )),
        }
    }

    fn write_message(
        stream: &mut TcpStream,
        message: CompletionMessage,
        poll_start: Instant,
        remaining_budget: Duration,
    ) -> io::Result<()> {
        let write_timeout = Self::io_timeout(poll_start, remaining_budget).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "completion barrier exhausted its existing timeout",
            )
        })?;
        stream.set_nonblocking(false)?;
        stream.set_write_timeout(Some(write_timeout))?;
        stream.write_all(&message.encode()?)
    }

    fn remote_peer_index(&self, player_index: usize) -> Option<usize> {
        if player_index >= self.state.ready.len() || player_index == self.local_player {
            return None;
        }
        Some(if player_index < self.local_player {
            player_index
        } else {
            player_index - 1
        })
    }

    fn remote_player_index(&self, peer_index: usize) -> usize {
        if peer_index < self.local_player {
            peer_index
        } else {
            peer_index + 1
        }
    }

    fn io_timeout(poll_start: Instant, remaining_budget: Duration) -> Option<Duration> {
        let remaining = remaining_budget.checked_sub(poll_start.elapsed())?;
        Self::bounded_io_timeout(remaining)
    }

    fn bounded_io_timeout(remaining: Duration) -> Option<Duration> {
        if remaining.is_zero() {
            return None;
        }
        Some(std::cmp::min(remaining, COMPLETION_BARRIER_IO_SLICE))
    }

    fn is_complete(&self) -> bool {
        self.state.exchange_complete(&self.acknowledged_by_peer)
    }

    fn timeout_diagnostic(&self) -> String {
        format!(
            "pending_players={:?}, unacknowledged_peers={}, last_io_error={:?}",
            self.state.pending_players(),
            self.acknowledged_by_peer
                .iter()
                .filter(|acknowledged| !**acknowledged)
                .count(),
            self.last_io_error
        )
    }
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
    target_frame: i32,
    target_snapshot: Option<TestState>,
    rollback_count: u32,
    debug_log: DebugLog,
}

impl TestGame {
    fn new(debug_enabled: bool, target_frame: i32) -> Self {
        Self {
            state: TestState::default(),
            target_frame,
            target_snapshot: None,
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
                    self.observe_loaded_state(frame.as_i32());
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
                    self.observe_advanced_state();
                },
            }
        }
    }

    fn observe_loaded_state(&mut self, frame: i32) {
        if frame < self.target_frame {
            // A rollback below the target invalidates any previously captured
            // speculative target. A later re-simulation must replace it.
            self.target_snapshot = None;
        } else if frame == self.target_frame {
            self.target_snapshot = Some(self.state.clone());
        }
    }

    fn observe_advanced_state(&mut self) {
        if self.state.frame == self.target_frame {
            self.target_snapshot = Some(self.state.clone());
        }
    }

    fn target_snapshot(&self) -> Option<&TestState> {
        self.target_snapshot.as_ref()
    }
}

fn target_state_checksum(state: &TestState) -> u64 {
    let mut hasher = DeterministicHasher::new();
    state.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum DriveOutcome {
    Advanced,
    NotRunning,
}

fn target_capture_ready(
    drive_outcome: DriveOutcome,
    game_frame: i32,
    confirmed_frame: Frame,
    target_frames: i32,
) -> bool {
    drive_outcome == DriveOutcome::Advanced
        && target_completion_reached(game_frame, confirmed_frame, target_frames)
}

/// Drives one ordinary production-protocol frame for the test peer.
///
/// The completion barrier reuses this exact path after capturing the target
/// checksum so pending input and connect-status gossip continue to flow until
/// every peer reaches the target. The captured target result is unaffected by
/// any drain frames.
fn drive_session_frame(
    session: &mut fortress_rollback::P2PSession<TestConfig>,
    game: &mut TestGame,
    local_handle: PlayerHandle,
    player_index: usize,
) -> Result<DriveOutcome, String> {
    if session.current_state() != SessionState::Running {
        return Ok(DriveOutcome::NotRunning);
    }

    // Input generation follows the session frame, not the rollback-sensitive
    // game frame, exactly as it does during the measured target interval.
    let session_frame = session.current_frame().as_i32();
    let input = TestInput {
        value: (session_frame as u32).wrapping_mul(player_index as u32 + 1),
    };
    session
        .add_local_input(local_handle, input)
        .map_err(|error| format!("Failed to add input: {error}"))?;
    let requests = session
        .advance_frame()
        .map_err(|error| format!("Failed to advance frame: {error}"))?;
    game.handle_requests(requests);
    Ok(DriveOutcome::Advanced)
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
            "--completion-barrier" => {
                result.completion_barrier = true;
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
    /// Keep completed peers alive until every process reaches the same target.
    completion_barrier: bool,
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
    if args.target_frames <= 0 {
        output_error("--frames must be positive");
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
    let num_players = args.peers.len() + 1;
    let mut completion_barrier = if args.completion_barrier {
        match CompletionBarrier::bind(args, num_players) {
            Ok(barrier) => Some(barrier),
            Err(error) => {
                return TestResult {
                    success: false,
                    final_frame: 0,
                    final_value: 0,
                    checksum: 0,
                    rollbacks: 0,
                    desync_detected: 0,
                    error_kind: Some("io".to_string()),
                    error: Some(format!("Failed to bind completion barrier: {error}")),
                    debug_log: None,
                    diagnostics: None,
                    runtime: None,
                };
            },
        }
    } else {
        None
    };

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

    let mut game = TestGame::new(args.debug, args.target_frames);
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
                    "Timeout (current_frame={}, confirmed_frame={}, target_frame_exclusive={})",
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
        if let Some(barrier) = &mut completion_barrier {
            barrier.poll(false, timeout.saturating_sub(start_time.elapsed()));
        }

        // Check whether the requested exclusive frame range is ready to capture:
        // 1. Game state frame >= the requested frame count
        // 2. All inputs in the exclusive range [0, target) are confirmed
        //
        // The settle period below then gives a peer that received inputs slightly
        // later time to complete its final rollback without changing this oracle.
        let confirmed = session.confirmed_frame();
        if target_completion_reached(game.state.frame, confirmed, args.target_frames) {
            // Force one ordinary production-protocol drive before capture. If
            // confirming the last target input scheduled a final rollback, this
            // applies the Load/Advance requests now. TestGame clears a target
            // snapshot on every rollback below target and replaces it only when
            // re-simulation reaches target again, so absence fails closed.
            let drive_outcome =
                match drive_session_frame(&mut session, &mut game, local_handle, args.player_index)
                {
                    Ok(outcome) => outcome,
                    Err(error) => {
                        let (checksum, diagnostics) = compute_confirmed_checksum_with_diagnostics(
                            &session,
                            args.target_frames,
                        );
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
                            error: Some(format!("Final target rollback drive failed: {error}")),
                            debug_log: if args.debug {
                                Some(game.debug_log.entries)
                            } else {
                                None
                            },
                            diagnostics: Some(diagnostics),
                            runtime: Some(runtime),
                        };
                    },
                };
            drain_session_events(&mut session, &mut event_summary);
            if !target_capture_ready(
                drive_outcome,
                game.state.frame,
                session.confirmed_frame(),
                args.target_frames,
            ) {
                // Synchronizing means no flush occurred, and confirmed_frame
                // may regress after topology updates. In either case resume the
                // ordinary poll/drive loop without capturing or announcing.
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }

            let (checksum_diagnostics, target_state) = {
                let (_, diagnostics) =
                    compute_confirmed_checksum_with_diagnostics(&session, args.target_frames);
                let Some(target_state) = game.target_snapshot().cloned() else {
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
                        checksum: 0,
                        rollbacks: game.rollback_count,
                        desync_detected: event_summary.desync_detected,
                        error_kind: Some("target_snapshot".to_string()),
                        error: Some(format!(
                            "Target state snapshot missing after confirmed exclusive range [0, {})",
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
                };
                (diagnostics, target_state)
            };
            let checksum = target_state_checksum(&target_state);
            let confirmed_value = target_state.value;

            if let Some(barrier) = &mut completion_barrier {
                barrier.poll(true, timeout.saturating_sub(start_time.elapsed()));
            }

            // Continue polling for a settle period to ensure rollbacks complete.
            // During settle, we ONLY poll for network messages - we don't advance new frames.
            // This ensures both peers have finished processing all pending messages.
            // Note: 500ms is generous to handle slow CI VMs where scheduling delays can be significant.
            //
            // IMPORTANT: We do NOT call advance_frame() during this 500ms settle.
            // This retains the established settle behavior independently of the
            // retained target-state snapshot above. The optional completion barrier may
            // drive later frames only after both are complete.
            let settle_start = Instant::now();
            let settle_duration = Duration::from_millis(500);

            while settle_start.elapsed() < settle_duration {
                session.poll_remote_clients();
                drain_session_events(&mut session, &mut event_summary);
                if let Some(barrier) = &mut completion_barrier {
                    barrier.poll(true, timeout.saturating_sub(start_time.elapsed()));
                }
                std::thread::sleep(Duration::from_millis(5));
            }

            // Completion is local knowledge: this peer confirming frame N - 1
            // does not prove every remote has received this peer's frame N - 1.
            // After preserving the polling-only settle above, the test-only
            // barrier keeps early finishers alive and drives ordinary drain
            // frames so input/connect-status gossip continues until every
            // process independently reports the same completion oracle. It uses
            // the existing overall test timeout; no extra wall-clock allowance
            // or retry-based acceptance is added.
            if let Some(barrier) = &mut completion_barrier {
                while !barrier.is_complete() {
                    if start_time.elapsed() > timeout {
                        let barrier_diagnostic = barrier.timeout_diagnostic();
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
                            final_value: confirmed_value,
                            checksum,
                            rollbacks: game.rollback_count,
                            desync_detected: event_summary.desync_detected,
                            error_kind: Some("completion_barrier".to_string()),
                            error: Some(format!(
                                "Completion barrier timed out ({barrier_diagnostic})"
                            )),
                            debug_log: if args.debug {
                                Some(game.debug_log.entries)
                            } else {
                                None
                            },
                            diagnostics: Some(checksum_diagnostics),
                            runtime: Some(runtime),
                        };
                    }

                    session.poll_remote_clients();
                    drain_session_events(&mut session, &mut event_summary);
                    barrier.poll(true, timeout.saturating_sub(start_time.elapsed()));
                    if let Err(error) = drive_session_frame(
                        &mut session,
                        &mut game,
                        local_handle,
                        args.player_index,
                    ) {
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
                            final_value: confirmed_value,
                            checksum,
                            rollbacks: game.rollback_count,
                            desync_detected: event_summary.desync_detected,
                            error_kind: Some("session".to_string()),
                            error: Some(format!("Completion drain failed: {error}")),
                            debug_log: if args.debug {
                                Some(game.debug_log.entries)
                            } else {
                                None
                            },
                            diagnostics: Some(checksum_diagnostics),
                            runtime: Some(runtime),
                        };
                    }
                    drain_session_events(&mut session, &mut event_summary);
                    std::thread::sleep(Duration::from_millis(5));
                }
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
                final_value: confirmed_value,
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

        if let Err(error) =
            drive_session_frame(&mut session, &mut game, local_handle, args.player_index)
        {
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
                error: Some(error),
                debug_log: if args.debug {
                    Some(game.debug_log.entries)
                } else {
                    None
                },
                diagnostics: Some(diagnostics),
                runtime: Some(runtime),
            };
        }
        drain_session_events(&mut session, &mut event_summary);

        // Small sleep to avoid busy-waiting
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_oracle_uses_exclusive_target_input_range() {
        struct Case {
            name: &'static str,
            game_frame: i32,
            confirmed_frame: i32,
            target_frames: i32,
            expected: bool,
        }

        let cases = [
            Case {
                name: "one_frame_target_confirms_frame_zero",
                game_frame: 1,
                confirmed_frame: 0,
                target_frames: 1,
                expected: true,
            },
            Case {
                name: "hundred_frame_target_confirms_frame_ninety_nine",
                game_frame: 100,
                confirmed_frame: 99,
                target_frames: 100,
                expected: true,
            },
            Case {
                name: "insufficient_game_frame_is_incomplete",
                game_frame: 99,
                confirmed_frame: 99,
                target_frames: 100,
                expected: false,
            },
            Case {
                name: "insufficient_confirmed_frame_is_incomplete",
                game_frame: 100,
                confirmed_frame: 98,
                target_frames: 100,
                expected: false,
            },
            Case {
                name: "zero_target_is_invalid",
                game_frame: 0,
                confirmed_frame: 0,
                target_frames: 0,
                expected: false,
            },
            Case {
                name: "negative_target_is_invalid",
                game_frame: 100,
                confirmed_frame: 100,
                target_frames: -1,
                expected: false,
            },
            Case {
                name: "minimum_target_does_not_overflow",
                game_frame: 100,
                confirmed_frame: 100,
                target_frames: i32::MIN,
                expected: false,
            },
        ];

        for case in cases {
            assert_eq!(
                target_completion_reached(
                    case.game_frame,
                    Frame::new(case.confirmed_frame),
                    case.target_frames,
                ),
                case.expected,
                "case {}",
                case.name,
            );
        }
    }

    #[test]
    fn completion_barrier_requires_every_distinct_player() {
        struct Step {
            name: &'static str,
            player_index: usize,
            newly_ready: bool,
            complete: bool,
            pending: &'static [usize],
        }

        let steps = [
            Step {
                name: "local_peer_alone_is_not_complete",
                player_index: 1,
                newly_ready: true,
                complete: false,
                pending: &[0, 2],
            },
            Step {
                name: "first_remote_is_not_complete",
                player_index: 0,
                newly_ready: true,
                complete: false,
                pending: &[2],
            },
            Step {
                name: "duplicate_announcement_is_idempotent",
                player_index: 0,
                newly_ready: false,
                complete: false,
                pending: &[2],
            },
            Step {
                name: "out_of_range_announcement_is_ignored",
                player_index: 3,
                newly_ready: false,
                complete: false,
                pending: &[2],
            },
            Step {
                name: "last_distinct_remote_completes_barrier",
                player_index: 2,
                newly_ready: true,
                complete: true,
                pending: &[],
            },
        ];

        let mut state = CompletionBarrierState::new(3).expect("three-player barrier is valid");
        for step in steps {
            assert_eq!(
                state.observe_ready(step.player_index),
                step.newly_ready,
                "newly-ready result for {}",
                step.name
            );
            assert_eq!(
                state.inbound_complete(),
                step.complete,
                "completion result for {}",
                step.name
            );
            assert_eq!(
                state.pending_players(),
                step.pending,
                "pending players for {}",
                step.name
            );
        }
    }

    #[test]
    fn completion_barrier_rejects_empty_participant_set() {
        assert!(CompletionBarrierState::new(0).is_none());
    }

    #[test]
    fn completion_barrier_requires_inbound_and_outbound_completion() {
        let mut state = CompletionBarrierState::new(3).expect("three-player barrier is valid");
        for player_index in 0..3 {
            assert!(state.observe_ready(player_index));
        }
        assert!(state.inbound_complete());

        assert!(
            !state.exchange_complete(&[true, false]),
            "all inbound announcements cannot hide an incomplete outbound announcement"
        );
        assert!(
            !state.exchange_complete(&[true]),
            "an incomplete outbound peer set cannot satisfy the barrier"
        );
        assert!(state.exchange_complete(&[true, true]));
    }

    #[test]
    fn completion_barrier_partial_ready_gets_bounded_same_stream_ack() {
        use std::sync::mpsc;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
        listener
            .set_nonblocking(true)
            .expect("model the nonblocking barrier listener");
        let listener_addr = listener.local_addr().expect("listener address");
        let (prefix_sent_tx, prefix_sent_rx) = mpsc::channel();
        let (finish_write_tx, finish_write_rx) = mpsc::channel();

        let client = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(listener_addr).expect("connect to barrier");
            let ready = CompletionMessage::Ready { player_index: 0 }
                .encode()
                .expect("encode READY");
            stream.write_all(&ready[..4]).expect("write READY prefix");
            prefix_sent_tx.send(()).expect("signal prefix");
            finish_write_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("wait to finish READY");
            std::thread::sleep(Duration::from_millis(75));
            stream.write_all(&ready[4..]).expect("write READY suffix");
            CompletionBarrier::read_message(&mut stream, Duration::from_secs(1))
                .expect("read same-stream ACK")
        });

        prefix_sent_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("client sent READY prefix");
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(accepted) => break accepted,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::yield_now();
                },
                Err(error) => panic!("accept barrier client: {error}"),
            }
        };

        let mut state = CompletionBarrierState::new(2).expect("two-player barrier is valid");
        state.observe_ready(1);
        let mut acknowledged = [false];
        assert!(!state.exchange_complete(&acknowledged));

        let poll_start = Instant::now();
        finish_write_tx.send(()).expect("release READY suffix");
        let message = CompletionBarrier::read_message(&mut stream, COMPLETION_BARRIER_IO_SLICE)
            .expect("blocking read accepts delayed READY suffix");
        assert_eq!(message, CompletionMessage::Ready { player_index: 0 });
        CompletionBarrier::write_message(
            &mut stream,
            CompletionMessage::Ack { player_index: 1 },
            poll_start,
            COMPLETION_BARRIER_IO_SLICE,
        )
        .expect("write bounded same-stream ACK");
        assert!(
            poll_start.elapsed() > Duration::from_millis(25),
            "delayed split write must exceed the former unsafe 25ms slice"
        );

        assert_eq!(
            client.join().expect("client thread"),
            CompletionMessage::Ack { player_index: 1 }
        );
        state.observe_ready(0);
        acknowledged[0] = true;
        assert!(state.exchange_complete(&acknowledged));
    }

    #[test]
    fn completion_barrier_io_is_bounded_by_slice_and_existing_budget() {
        assert_eq!(CompletionBarrier::bounded_io_timeout(Duration::ZERO), None);
        assert_eq!(
            CompletionBarrier::bounded_io_timeout(Duration::from_millis(10)),
            Some(Duration::from_millis(10))
        );
        assert_eq!(
            CompletionBarrier::bounded_io_timeout(Duration::from_secs(10)),
            Some(COMPLETION_BARRIER_IO_SLICE)
        );
    }

    #[test]
    fn target_snapshot_is_invalidated_by_rollback_below_target() {
        let mut game = TestGame::new(false, 100);
        game.state = TestState {
            frame: 100,
            value: 41,
        };
        game.observe_advanced_state();
        assert_eq!(game.target_snapshot().map(|state| state.value), Some(41));

        game.state = TestState {
            frame: 99,
            value: 40,
        };
        game.observe_loaded_state(99);
        assert!(
            game.target_snapshot().is_none(),
            "rollback below target must not leave a vacuous/stale success snapshot"
        );

        game.state = TestState {
            frame: 100,
            value: 42,
        };
        game.observe_loaded_state(100);
        assert_eq!(game.target_snapshot().map(|state| state.value), Some(42));
    }

    #[test]
    fn target_capture_requires_an_advance_and_rechecked_confirmation() {
        assert!(target_capture_ready(
            DriveOutcome::Advanced,
            100,
            Frame::new(99),
            100
        ));
        assert!(!target_capture_ready(
            DriveOutcome::NotRunning,
            100,
            Frame::new(99),
            100
        ));
        assert!(!target_capture_ready(
            DriveOutcome::Advanced,
            101,
            Frame::new(98),
            100
        ));
    }
}
