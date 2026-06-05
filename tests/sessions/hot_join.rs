//! Integration tests for hot join (Feature 6), reserved-slot model.
//!
//! These tests cover the end-to-end host-mediated, 2-peer hot-join flow:
//! - A host runs solo with a reserved (frozen/disconnected) slot.
//! - A joiner synchronizes, requests a state snapshot, loads it, and resumes
//!   contributing real inputs for the reserved slot from an activation frame.
//! - Both peers then advance in lockstep with no desync.
//!
//! All tests use `ChannelSocket` + `TestClock` for fully deterministic behavior.
#![cfg(feature = "hot-join")]
// In tests: tests intentionally use unwrap/expect for clarity.
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::ip_constant
)]

use std::collections::BTreeMap;

use crate::common::stubs::{GameStub, StateStub, StubConfig, StubInput};
use crate::common::{TestClock, POLL_INTERVAL_DETERMINISTIC};
use fortress_rollback::{
    DesyncDetection, FortressError, FortressEvent, FortressRequest, Frame, InputStatus, InputVec,
    Message, NonBlockingSocket, P2PSession, PlayerHandle, PlayerType, ProtocolConfig,
    SessionBuilder, SessionState,
};

/// Local mirror of the crate-private `DEFAULT_HOT_JOIN_SERVE_TIMEOUT_POLLS`
/// constant in `src/sessions/p2p_session.rs`. The Phase-4 serve-timeout tests
/// size their poll budgets as multiples of this; if the production default
/// changes, update this to match (the tests assert behavior, not the exact
/// value, so a small drift only over-/under-drives the loop).
const HOT_JOIN_SERVE_TIMEOUT_POLLS_TEST: usize = 600;

/// Helper: creates a `ProtocolConfig` with the given test clock.
fn protocol_config(clock: &TestClock) -> ProtocolConfig {
    ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        ..ProtocolConfig::default()
    }
}

/// Returns `true` if `msg` is a hot-join `StateSnapshot` message.
///
/// `Message`'s fields are `pub(crate)`, so the test cannot match on the body
/// directly. Instead it serializes the message (which derives `Serialize`) and
/// inspects the externally-tagged enum representation: a `StateSnapshot` body
/// serializes with a top-level `"StateSnapshot"` tag. This is deterministic and
/// independent of message size.
fn is_state_snapshot(msg: &Message) -> bool {
    match serde_json::to_string(msg) {
        Ok(json) => json.contains("StateSnapshot\":") && !json.contains("StateSnapshotAck\":"),
        Err(_) => false,
    }
}

/// Returns `true` if `msg` is a hot-join `StateSnapshotAck` message. Mirror of
/// [`is_state_snapshot`] for the ack body (used to count acks a joiner emits).
fn is_state_snapshot_ack(msg: &Message) -> bool {
    match serde_json::to_string(msg) {
        Ok(json) => json.contains("StateSnapshotAck\":"),
        Err(_) => false,
    }
}

/// A deterministic [`NonBlockingSocket`] wrapper that COUNTS the hot-join
/// `StateSnapshot` and `StateSnapshotAck` messages its owner *sends* (delegating
/// the actual transport to the inner [`ChannelSocket`]) and, optionally, drops
/// every outgoing `StateSnapshot` so a join never completes.
///
/// Used to pin the "exactly one send per poll" fixes: by snapshotting the
/// counters around each `poll_remote_clients` call a test can assert that a host
/// sends at most one snapshot per poll, and a joiner at most one ack per poll,
/// even when duplicate snapshots are arriving.
struct CountingSocket {
    inner: crate::common::ChannelSocket,
    /// Count of `StateSnapshot` messages this socket has sent.
    snapshots_sent: std::rc::Rc<std::cell::Cell<usize>>,
    /// Count of `StateSnapshotAck` messages this socket has sent.
    acks_sent: std::rc::Rc<std::cell::Cell<usize>>,
    /// When `true`, every outgoing `StateSnapshot` is silently dropped (the
    /// counter still increments, recording the *attempt*). Lets a host-side test
    /// keep a serve from ever completing while still counting per-poll sends.
    drop_outgoing_snapshots: bool,
    /// When `true`, every outgoing `StateSnapshotAck` is silently dropped (the
    /// counter still increments). Lets a joiner-side test keep the host's serve
    /// open (no ack ever lands) so the joiner keeps receiving duplicate snapshots
    /// and re-acking, while the per-poll ack count is still measured.
    drop_outgoing_acks: bool,
}

impl NonBlockingSocket<std::net::SocketAddr> for CountingSocket {
    fn send_to(&mut self, msg: &Message, addr: &std::net::SocketAddr) {
        if is_state_snapshot(msg) {
            self.snapshots_sent.set(self.snapshots_sent.get() + 1);
            if self.drop_outgoing_snapshots {
                return;
            }
        } else if is_state_snapshot_ack(msg) {
            self.acks_sent.set(self.acks_sent.get() + 1);
            if self.drop_outgoing_acks {
                return;
            }
        }
        self.inner.send_to(msg, addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(std::net::SocketAddr, Message)> {
        self.inner.receive_all_messages()
    }
}

/// A deterministic [`NonBlockingSocket`] wrapper that DROPS the first
/// `drops_remaining` `StateSnapshot` messages it would otherwise deliver, then
/// behaves exactly like the inner [`ChannelSocket`].
///
/// Used to pin MAJOR-1: if the host's first snapshot is lost, the host's
/// reliable retransmit must still let the join complete. All other traffic
/// (sync, input, quality, acks) passes through untouched, and message ordering
/// is otherwise preserved.
struct DropSnapshotSocket {
    inner: crate::common::ChannelSocket,
    drops_remaining: usize,
    /// Shared counter of snapshots actually dropped, so the test can confirm the
    /// drop fired (non-vacuity) and a retransmit was genuinely required.
    dropped: std::rc::Rc<std::cell::Cell<usize>>,
}

impl NonBlockingSocket<std::net::SocketAddr> for DropSnapshotSocket {
    fn send_to(&mut self, msg: &Message, addr: &std::net::SocketAddr) {
        self.inner.send_to(msg, addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(std::net::SocketAddr, Message)> {
        let mut out = Vec::new();
        for (from, msg) in self.inner.receive_all_messages() {
            if self.drops_remaining > 0 && is_state_snapshot(&msg) {
                // Deterministically drop this snapshot delivery (simulated loss).
                self.drops_remaining -= 1;
                self.dropped.set(self.dropped.get() + 1);
                continue;
            }
            out.push((from, msg));
        }
        out
    }
}

/// A deterministic [`NonBlockingSocket`] wrapper that drops EVERY incoming
/// `StateSnapshot` while a shared `dropping` flag is set, and passes everything
/// else (sync, input, quality reports, keepalives, acks) through untouched.
///
/// The flag is externally toggleable so a test can drop snapshots for a window
/// (forcing the host's Phase-4 serve timeout) and then re-enable delivery to
/// verify an in-session retry completes. Used to pin FIX 2 (Phase-4 timeout) and
/// FIX 3 (no disconnect spam + in-session retry).
struct GatedDropSnapshotSocket {
    inner: crate::common::ChannelSocket,
    /// While `true`, all `StateSnapshot` deliveries are dropped.
    dropping: std::rc::Rc<std::cell::Cell<bool>>,
    /// Count of snapshots actually dropped (non-vacuity).
    dropped: std::rc::Rc<std::cell::Cell<usize>>,
}

impl NonBlockingSocket<std::net::SocketAddr> for GatedDropSnapshotSocket {
    fn send_to(&mut self, msg: &Message, addr: &std::net::SocketAddr) {
        self.inner.send_to(msg, addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(std::net::SocketAddr, Message)> {
        let mut out = Vec::new();
        for (from, msg) in self.inner.receive_all_messages() {
            if self.dropping.get() && is_state_snapshot(&msg) {
                self.dropped.set(self.dropped.get() + 1);
                continue;
            }
            out.push((from, msg));
        }
        out
    }
}

/// Advances a running session by one frame with the given local input, routing
/// the resulting requests through the game stub. Records the game state after
/// **every** `AdvanceFrame` (including rollback re-simulations within a single
/// call), keyed by the simulated frame. Because a confirmed frame's *last*
/// re-simulation uses the corrected (rolled-back) inputs, the final recorded
/// value for any confirmed frame is its confirmed state — making cross-peer
/// equality at confirmed frames a sound no-desync check.
fn advance_and_record(
    session: &mut P2PSession<StubConfig>,
    stub: &mut GameStub,
    handle: PlayerHandle,
    value: u32,
    states: &mut BTreeMap<i32, StateStub>,
) -> Result<(), FortressError> {
    session.add_local_input(handle, StubInput { inp: value })?;
    let requests = session.advance_frame()?;
    stub.handle_requests_recording(requests, states);
    Ok(())
}

/// Advances a running session by one frame with the given local input.
fn advance_session(
    session: &mut P2PSession<StubConfig>,
    stub: &mut GameStub,
    handle: PlayerHandle,
    value: u32,
) -> Result<(), FortressError> {
    session.add_local_input(handle, StubInput { inp: value })?;
    let requests = session.advance_frame()?;
    stub.handle_requests(requests);
    Ok(())
}

/// Drains all currently buffered events from a session.
fn drain_events(sess: &mut P2PSession<StubConfig>) -> Vec<FortressEvent<StubConfig>> {
    sess.events().collect()
}

// ============================================================================
// Checkpoint A: host with a reserved slot runs solo
// ============================================================================

/// A host built with one local player and one reserved player reaches
/// `Running` immediately and advances solo for several frames, with the
/// reserved slot reporting `Disconnected`/frozen-default input.
#[test]
fn add_reserved_player_without_join_keeps_running() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (host_socket, _joiner_socket, _host_addr, joiner_addr) =
        crate::common::create_channel_pair();

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;

    // The host reaches Running immediately: the reserved endpoint does not gate
    // synchronization (it is skipped by check_initial_sync).
    host.poll_remote_clients();
    assert_eq!(
        host.current_state(),
        SessionState::Running,
        "host with a reserved slot should reach Running solo"
    );
    let _ = drain_events(&mut host);

    let mut stub = GameStub::new();
    let mut observed_reserved: Vec<(u32, InputStatus)> = Vec::new();

    // Advance solo for several frames, feeding only the local player's input.
    for i in 0..8_u32 {
        host.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        host.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        let requests = host.advance_frame()?;
        for request in &*requests {
            if let FortressRequest::AdvanceFrame { inputs } = request {
                let inputs: &InputVec<StubInput> = inputs;
                if let Some(&(input, status)) = inputs.get(1) {
                    observed_reserved.push((input.inp, status));
                }
            }
        }
        stub.handle_requests(requests);
    }

    assert!(
        host.current_frame().as_i32() >= 5,
        "host should advance solo past several frames; got {}",
        host.current_frame()
    );
    // The reserved slot must be frozen at the default input (0) and eventually
    // report Disconnected, exactly like a Feature-5 dropped slot.
    assert!(
        !observed_reserved.is_empty(),
        "expected to observe the reserved slot in AdvanceFrame inputs"
    );
    for (value, _status) in &observed_reserved {
        assert_eq!(
            *value, 0,
            "reserved slot must report the frozen default input (0); got {observed_reserved:?}"
        );
    }
    assert!(
        observed_reserved
            .iter()
            .any(|(_, status)| *status == InputStatus::Disconnected),
        "reserved slot must eventually report Disconnected; got {observed_reserved:?}"
    );

    Ok(())
}

// ============================================================================
// hot_join_joiner_advance_before_snapshot_errs
// ============================================================================

/// A joiner in `HotJoining` returns `Err(NotSynchronized)` from `advance_frame`
/// until it has applied the snapshot.
#[test]
fn hot_join_joiner_advance_before_snapshot_errs() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;

    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    assert_eq!(joiner.current_state(), SessionState::HotJoining);

    // Before any sync/snapshot, advancing the joiner errors.
    joiner.add_local_input(PlayerHandle::new(1), StubInput { inp: 1 })?;
    assert!(
        matches!(joiner.advance_frame(), Err(FortressError::NotSynchronized)),
        "joiner must return NotSynchronized before applying a snapshot"
    );

    // Get the host running and producing saved states (so a snapshot can be served).
    let mut host_stub = GameStub::new();
    let mut became_running = false;
    for _ in 0..200 {
        host.poll_remote_clients();
        if host.current_state() == SessionState::Running {
            let _ = advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 7);
        }
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        // While still HotJoining, the joiner must keep returning NotSynchronized.
        if joiner.current_state() == SessionState::HotJoining {
            joiner.add_local_input(PlayerHandle::new(1), StubInput { inp: 1 })?;
            assert!(
                matches!(joiner.advance_frame(), Err(FortressError::NotSynchronized)),
                "joiner must return NotSynchronized while still HotJoining"
            );
        } else {
            became_running = true;
            break;
        }
    }

    assert!(
        became_running,
        "joiner should eventually transition to Running after applying a snapshot"
    );

    Ok(())
}

// ============================================================================
// MINOR-3: lockstep (max_prediction == 0) hot-join is rejected at build time
// ============================================================================

/// A hot-join host or joiner configured with `max_prediction == 0` (lockstep)
/// must be rejected at build time: in lockstep the host never saves state and so
/// can never serve a snapshot, which would hang a joiner forever. Both
/// `start_p2p_session` (host serving) and `start_hot_join_session` (joiner) must
/// return an error.
#[test]
fn lockstep_hot_join_is_rejected_at_build_time() {
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    // Host serving hot-joins in lockstep must be rejected.
    let host_result = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_max_prediction_window(0)
        .with_hot_join(true)
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .unwrap()
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))
        .unwrap()
        .start_p2p_session(host_socket);
    assert!(
        matches!(
            host_result,
            Err(FortressError::InvalidRequestStructured { .. })
        ),
        "lockstep hot-join host must be rejected at build time"
    );

    // Joiner in lockstep must be rejected.
    let joiner_result = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_max_prediction_window(0)
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(1))
        .unwrap()
        .start_hot_join_session(joiner_socket, host_addr);
    assert!(
        matches!(
            joiner_result,
            Err(FortressError::InvalidRequestStructured { .. })
        ),
        "lockstep hot-join joiner must be rejected at build time"
    );

    // A non-hot-join lockstep session must STILL be allowed (the guard is scoped
    // strictly to hot-join hosts/joiners).
    let (plain_socket, _s2, _a1, plain_remote) = crate::common::create_channel_pair();
    let plain = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_max_prediction_window(0)
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .unwrap()
        .add_player(PlayerType::Remote(plain_remote), PlayerHandle::new(1))
        .unwrap()
        .start_p2p_session(plain_socket);
    assert!(
        plain.is_ok(),
        "a normal (non-hot-join) lockstep session must still build"
    );
}

// ============================================================================
// Negative: a join request for a non-reserved handle is ignored
// ============================================================================

/// A joiner that requests a handle the host did not reserve is ignored: the
/// host never serves a snapshot and the joiner stays in `HotJoining`.
#[test]
fn hot_join_non_reserved_handle_is_ignored() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    // Host reserves handle 1, and uses handle 0 as local. The joiner below will
    // (incorrectly) ask to fill handle 0 — a non-reserved (local) handle.
    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;

    // Joiner declares its local handle as 0 (the host's local slot, NOT reserved).
    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    let mut host_stub = GameStub::new();
    for _ in 0..60 {
        host.poll_remote_clients();
        if host.current_state() == SessionState::Running {
            let _ = advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 3);
        }
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // The host must never have emitted PeerJoined for the bogus request, and the
    // joiner must still be HotJoining (no snapshot served for handle 0).
    let host_events = drain_events(&mut host);
    assert!(
        !host_events
            .iter()
            .any(|e| matches!(e, FortressEvent::PeerJoined { .. })),
        "host must not emit PeerJoined for a non-reserved handle; got {host_events:?}"
    );
    assert_eq!(
        joiner.current_state(),
        SessionState::HotJoining,
        "joiner requesting a non-reserved handle must stay HotJoining"
    );

    Ok(())
}

// ============================================================================
// Headline: host runs solo, then accepts a join without desync
// ============================================================================

/// The headline end-to-end test: a host runs solo with a reserved slot, a peer
/// hot-joins, loads the snapshot, and both advance in lockstep with no desync.
#[test]
fn host_with_reserved_slot_runs_solo_then_accepts_join_without_desync() -> Result<(), FortressError>
{
    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    // Host: player 0 local, player 1 reserved for the joiner; serve hot-joins.
    // Desync detection interval is set LOW so the built-in checksum-comparison
    // `DesyncDetected` path actually runs across the post-join range (makes the
    // desync gate non-vacuous), in addition to the independent byte-equal check.
    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join(true)
        .with_desync_detection_mode(DesyncDetection::On { interval: 2 })
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;

    host.poll_remote_clients();
    assert_eq!(host.current_state(), SessionState::Running);
    let _ = drain_events(&mut host);

    let mut host_stub = GameStub::new();
    // Per-frame state recorded on each side; compared at overlapping frames.
    let mut host_states: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut joiner_states: BTreeMap<i32, StateStub> = BTreeMap::new();

    // Advance ~5 frames solo, feeding deterministic host inputs.
    for i in 0..5_u32 {
        host.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        advance_and_record(
            &mut host,
            &mut host_stub,
            PlayerHandle::new(0),
            10 + i,
            &mut host_states,
        )?;
    }
    assert!(host.current_frame().as_i32() >= 5);

    // Build the joiner: player 1 local (the reserved slot), host as remote.
    // Same low desync interval so the joiner side also runs the checksum gate.
    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_desync_detection_mode(DesyncDetection::On { interval: 2 })
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;
    assert_eq!(joiner.current_state(), SessionState::HotJoining);

    let mut joiner_stub = GameStub::new();
    let mut host_peer_joined = false;
    // MAJOR-3 closure assertion: the host must never emit `Disconnected` during
    // a normal join (the pause makes joiner-endpoint pending_output overflow
    // structurally impossible). Tracked across the whole join below.
    let mut host_disconnected = false;

    // Drive both sessions' poll+advance loop until the joiner loads the snapshot.
    let mut snapshot_frame: Option<Frame> = None;
    for _ in 0..200 {
        host.poll_remote_clients();
        for e in drain_events(&mut host) {
            if matches!(e, FortressEvent::PeerJoined { .. }) {
                host_peer_joined = true;
            }
            if matches!(e, FortressEvent::Disconnected { .. }) {
                host_disconnected = true;
            }
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        // Host keeps advancing solo while waiting for the join to complete.
        if host.current_state() == SessionState::Running {
            let v = 100 + (host.current_frame().as_i32() as u32 % 7);
            advance_and_record(
                &mut host,
                &mut host_stub,
                PlayerHandle::new(0),
                v,
                &mut host_states,
            )?;
        }

        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        if joiner.current_state() == SessionState::Running {
            // The first advance_frame must return exactly the LoadGameState.
            let requests = joiner.advance_frame()?;
            let mut load_count = 0;
            let mut other_count = 0;
            let mut loaded_frame = None;
            for request in &*requests {
                match request {
                    FortressRequest::LoadGameState { frame, .. } => {
                        load_count += 1;
                        loaded_frame = Some(*frame);
                    },
                    _ => other_count += 1,
                }
            }
            assert_eq!(
                load_count, 1,
                "joiner's first advance_frame must return exactly one LoadGameState"
            );
            assert_eq!(
                other_count, 0,
                "joiner's first advance_frame must return ONLY the LoadGameState"
            );
            joiner_stub.handle_requests(requests);
            snapshot_frame = loaded_frame;
            joiner_states.insert(joiner_stub.gs.frame, joiner_stub.gs);
            break;
        }
    }

    let snapshot_frame = snapshot_frame.expect("joiner must load a snapshot");

    // The joiner's loaded state must equal the host's recorded state at the
    // snapshot frame (byte-equal: same StateStub).
    let host_state_at_snapshot = host_states
        .get(&snapshot_frame.as_i32())
        .copied()
        .expect("host recorded a state at the snapshot frame");
    assert_eq!(
        joiner_stub.gs.frame,
        snapshot_frame.as_i32(),
        "joiner must be positioned at the snapshot frame after load"
    );
    assert_eq!(
        joiner_stub.gs, host_state_at_snapshot,
        "joiner's loaded state must byte-equal the host's state at the snapshot frame"
    );

    // Now advance BOTH ~20 more frames with deterministic local inputs on each
    // side; route packets; advance the clock.
    for i in 0..20_u32 {
        for _ in 0..3 {
            host.poll_remote_clients();
            for e in drain_events(&mut host) {
                if matches!(e, FortressEvent::PeerJoined { .. }) {
                    host_peer_joined = true;
                }
                if matches!(e, FortressEvent::Disconnected { .. }) {
                    host_disconnected = true;
                }
            }
            joiner.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }

        if host.current_state() == SessionState::Running {
            advance_and_record(
                &mut host,
                &mut host_stub,
                PlayerHandle::new(0),
                1000 + i,
                &mut host_states,
            )?;
        }
        if joiner.current_state() == SessionState::Running {
            advance_and_record(
                &mut joiner,
                &mut joiner_stub,
                PlayerHandle::new(1),
                2000 + i,
                &mut joiner_states,
            )?;
        }
    }

    // Drain remaining packets AND keep advancing both sides so each side both
    // exchanges checksums and then runs the checksum comparison (which only
    // happens inside `advance_frame`). This both converges the confirmed frame
    // and makes the desync gate non-vacuous (`last_verified_frame` advances).
    for i in 0..40_u32 {
        for _ in 0..3 {
            host.poll_remote_clients();
            for e in drain_events(&mut host) {
                if matches!(e, FortressEvent::PeerJoined { .. }) {
                    host_peer_joined = true;
                }
                if matches!(e, FortressEvent::Disconnected { .. }) {
                    host_disconnected = true;
                }
            }
            joiner.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
        if host.current_state() == SessionState::Running {
            advance_and_record(
                &mut host,
                &mut host_stub,
                PlayerHandle::new(0),
                3000 + i,
                &mut host_states,
            )?;
        }
        if joiner.current_state() == SessionState::Running {
            advance_and_record(
                &mut joiner,
                &mut joiner_stub,
                PlayerHandle::new(1),
                4000 + i,
                &mut joiner_states,
            )?;
        }
    }
    // A final pure-poll drain so any last in-flight inputs/checksums land.
    for _ in 0..30 {
        host.poll_remote_clients();
        for e in drain_events(&mut host) {
            if matches!(e, FortressEvent::PeerJoined { .. }) {
                host_peer_joined = true;
            }
            if matches!(e, FortressEvent::Disconnected { .. }) {
                host_disconnected = true;
            }
        }
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // MAJOR-3: the host must not have emitted a Disconnected event at any point
    // during the normal join. The pause prevents the joiner-endpoint
    // pending_output from growing while the joiner withholds acks, so the
    // pending-output-overflow disconnect is structurally impossible.
    let final_host_events = drain_events(&mut host);
    if final_host_events
        .iter()
        .any(|e| matches!(e, FortressEvent::Disconnected { .. }))
    {
        host_disconnected = true;
    }
    assert!(
        !host_disconnected,
        "host must NOT emit Disconnected during a normal join (MAJOR-3); got {final_host_events:?}"
    );

    // No DesyncDetected event may have fired on either side.
    let host_events = final_host_events;
    let joiner_events = drain_events(&mut joiner);
    assert!(
        !host_events
            .iter()
            .any(|e| matches!(e, FortressEvent::DesyncDetected { .. })),
        "host must not detect a desync; got {host_events:?}"
    );
    assert!(
        !joiner_events
            .iter()
            .any(|e| matches!(e, FortressEvent::DesyncDetected { .. })),
        "joiner must not detect a desync; got {joiner_events:?}"
    );

    // NON-VACUOUS desync gate (N1): with interval 2, the built-in checksum
    // comparison must have actually RUN and verified matching checksums on BOTH
    // sides past the snapshot frame. If verification never happened, the
    // "no DesyncDetected" assertions above would be vacuously true; these checks
    // prove the checksum path executed and agreed across the post-join range.
    let host_verified = host
        .last_verified_frame()
        .expect("host must have verified at least one checksum frame post-join");
    let joiner_verified = joiner
        .last_verified_frame()
        .expect("joiner must have verified at least one checksum frame post-join");
    assert!(
        host_verified.as_i32() > snapshot_frame.as_i32(),
        "host must verify checksums past the snapshot frame (non-vacuous desync gate); \
         verified={host_verified:?}, snapshot={snapshot_frame:?}"
    );
    assert!(
        joiner_verified.as_i32() > snapshot_frame.as_i32(),
        "joiner must verify checksums past the snapshot frame (non-vacuous desync gate); \
         verified={joiner_verified:?}, snapshot={snapshot_frame:?}"
    );

    // The host must have emitted PeerJoined for the reserved handle at some point.
    assert!(
        host_peer_joined,
        "host must emit PeerJoined after the joiner completes the handshake"
    );

    // Both must have advanced past the snapshot frame.
    assert!(
        host.current_frame().as_i32() > snapshot_frame.as_i32(),
        "host should have advanced past the snapshot frame"
    );
    assert!(
        joiner.current_frame().as_i32() > snapshot_frame.as_i32(),
        "joiner should have advanced past the snapshot frame"
    );

    // CORRECTNESS GATE (no-desync proof): for every CONFIRMED frame at or after
    // the snapshot frame that both sides recorded, the byte-level StateStub must
    // be identical. A confirmed frame's last simulation used the real (rolled-
    // back-and-corrected) inputs from both peers, so equality here proves the
    // rollback machinery reconciled both peers into a single shared simulation.
    // Frames above min_confirmed may still hold predicted state on one side and
    // are deliberately excluded.
    let min_confirmed = std::cmp::min(
        host.confirmed_frame().as_i32(),
        joiner.confirmed_frame().as_i32(),
    );
    assert!(
        min_confirmed > snapshot_frame.as_i32(),
        "both peers should have confirmed frames past the snapshot frame; \
         min_confirmed={min_confirmed}, snapshot={snapshot_frame:?}"
    );
    let mut compared = 0;
    for (frame, host_state) in &host_states {
        if *frame < snapshot_frame.as_i32() || *frame > min_confirmed {
            continue;
        }
        if let Some(joiner_state) = joiner_states.get(frame) {
            assert_eq!(
                host_state, joiner_state,
                "host and joiner game state must byte-equal at confirmed frame {frame}"
            );
            compared += 1;
        }
    }
    assert!(
        compared >= 5,
        "expected at least 5 overlapping confirmed frames to compare; got {compared}"
    );

    Ok(())
}

// ============================================================================
// MAJOR-1: loss tolerance — a dropped first snapshot still completes the join
// ============================================================================

/// The host's first `StateSnapshot` delivery to the joiner is DROPPED, yet the
/// join still completes (the host's reliable retransmit re-sends the cached
/// snapshot), the joiner loads it, and post-join states match with no desync.
/// This pins MAJOR-1 (a lost serve must not wedge the joiner forever).
#[test]
fn hot_join_completes_when_first_snapshot_is_dropped() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    // Wrap the joiner's socket so the FIRST snapshot it would receive is dropped.
    let dropped = std::rc::Rc::new(std::cell::Cell::new(0_usize));
    let joiner_socket = DropSnapshotSocket {
        inner: joiner_socket,
        drops_remaining: 1,
        dropped: std::rc::Rc::clone(&dropped),
    };

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join(true)
        .with_desync_detection_mode(DesyncDetection::On { interval: 2 })
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;

    host.poll_remote_clients();
    assert_eq!(host.current_state(), SessionState::Running);
    let _ = drain_events(&mut host);

    let mut host_stub = GameStub::new();
    let mut host_states: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut joiner_states: BTreeMap<i32, StateStub> = BTreeMap::new();

    // Host advances a few frames solo so a snapshot can be served.
    for i in 0..5_u32 {
        host.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        advance_and_record(
            &mut host,
            &mut host_stub,
            PlayerHandle::new(0),
            10 + i,
            &mut host_states,
        )?;
    }

    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_desync_detection_mode(DesyncDetection::On { interval: 2 })
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    let mut joiner_stub = GameStub::new();
    let mut snapshot_frame: Option<Frame> = None;
    let mut snapshots_seen = 0_u32;

    // Drive until the joiner loads the snapshot. It must take MORE than one
    // snapshot send (the first was dropped), proving the retransmit fired.
    for _ in 0..400 {
        host.poll_remote_clients();
        let _ = drain_events(&mut host);
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        if host.current_state() == SessionState::Running {
            let v = 100 + (host.current_frame().as_i32() as u32 % 7);
            advance_and_record(
                &mut host,
                &mut host_stub,
                PlayerHandle::new(0),
                v,
                &mut host_states,
            )?;
        }

        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        if joiner.current_state() == SessionState::Running {
            let requests = joiner.advance_frame()?;
            for request in &*requests {
                if let FortressRequest::LoadGameState { frame, .. } = request {
                    snapshot_frame = Some(*frame);
                    snapshots_seen += 1;
                }
            }
            joiner_stub.handle_requests(requests);
            joiner_states.insert(joiner_stub.gs.frame, joiner_stub.gs);
            break;
        }
    }

    let snapshot_frame = snapshot_frame.expect("join must complete despite the dropped snapshot");
    assert_eq!(
        snapshots_seen, 1,
        "exactly one LoadGameState should be issued"
    );
    // Non-vacuity: the first snapshot was genuinely dropped, so the join could
    // only have completed via the host's reliable retransmit (this is the MAJOR-1
    // fix; without it the joiner would be wedged in HotJoining forever).
    assert_eq!(
        dropped.get(),
        1,
        "exactly one snapshot must have been dropped (otherwise the test is vacuous)"
    );

    // The loaded state must byte-equal the host's state at the snapshot frame.
    let host_state_at_snapshot = host_states
        .get(&snapshot_frame.as_i32())
        .copied()
        .expect("host recorded a state at the snapshot frame");
    assert_eq!(
        joiner_stub.gs, host_state_at_snapshot,
        "joiner's loaded state must byte-equal the host's state at the snapshot frame"
    );

    // Advance both and confirm no desync post-join.
    for i in 0..20_u32 {
        for _ in 0..3 {
            host.poll_remote_clients();
            let _ = drain_events(&mut host);
            joiner.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
        if host.current_state() == SessionState::Running {
            advance_and_record(
                &mut host,
                &mut host_stub,
                PlayerHandle::new(0),
                1000 + i,
                &mut host_states,
            )?;
        }
        if joiner.current_state() == SessionState::Running {
            advance_and_record(
                &mut joiner,
                &mut joiner_stub,
                PlayerHandle::new(1),
                2000 + i,
                &mut joiner_states,
            )?;
        }
    }
    for _ in 0..120 {
        host.poll_remote_clients();
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    let host_events = drain_events(&mut host);
    let joiner_events = drain_events(&mut joiner);
    assert!(
        !host_events
            .iter()
            .any(|e| matches!(e, FortressEvent::DesyncDetected { .. })),
        "host must not detect a desync after a lossy join; got {host_events:?}"
    );
    assert!(
        !joiner_events
            .iter()
            .any(|e| matches!(e, FortressEvent::DesyncDetected { .. })),
        "joiner must not detect a desync after a lossy join; got {joiner_events:?}"
    );

    let min_confirmed = std::cmp::min(
        host.confirmed_frame().as_i32(),
        joiner.confirmed_frame().as_i32(),
    );
    assert!(
        min_confirmed > snapshot_frame.as_i32(),
        "both peers should confirm past the snapshot frame; min_confirmed={min_confirmed}, snapshot={snapshot_frame:?}"
    );
    let mut compared = 0;
    for (frame, host_state) in &host_states {
        if *frame < snapshot_frame.as_i32() || *frame > min_confirmed {
            continue;
        }
        if let Some(joiner_state) = joiner_states.get(frame) {
            assert_eq!(
                host_state, joiner_state,
                "host and joiner state must byte-equal at confirmed frame {frame} after a lossy join"
            );
            compared += 1;
        }
    }
    assert!(
        compared >= 3,
        "expected at least 3 overlapping confirmed frames; got {compared}"
    );

    Ok(())
}

// ============================================================================
// MAJOR-2: an abandoned join must not kill the host
// ============================================================================

/// A joiner sends a `JoinRequest` (after syncing) and then goes SILENT (the test
/// stops polling/routing it). The host must (a) never fall back to
/// `Synchronizing`, (b) resume advancing solo after the serve times out, and (c)
/// keep the slot frozen/`Disconnected`. Pins MAJOR-2.
#[test]
fn hot_join_abandoned_join_does_not_kill_host() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join(true)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;

    host.poll_remote_clients();
    assert_eq!(host.current_state(), SessionState::Running);
    let _ = drain_events(&mut host);

    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    let mut host_stub = GameStub::new();

    // Phase A: drive both until the host has begun serving (emits JoinRequested),
    // i.e. the joiner synced and requested. The moment the host serves, we BREAK
    // *before polling the joiner again* — so the joiner never receives/processes
    // the snapshot and never acks. This models a joiner that sends a JoinRequest
    // and then goes silent mid-handshake (the worst case for the host).
    let mut serve_started = false;
    for _ in 0..200 {
        host.poll_remote_clients();
        for e in drain_events(&mut host) {
            if matches!(e, FortressEvent::JoinRequested { .. }) {
                serve_started = true;
            }
        }
        if serve_started {
            // Do NOT poll the joiner again: it abandons the join here.
            break;
        }
        if host.current_state() == SessionState::Running {
            let _ = advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 7);
        }
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    assert!(
        serve_started,
        "host should have started serving (emitted JoinRequested) before the joiner abandons"
    );
    // Sanity: the host is actively serving (paused) and the slot is still reserved.
    assert_eq!(
        host.current_state(),
        SessionState::Running,
        "host should still be Running (paused) right after starting the serve"
    );

    // The host is now PAUSED (serving). Record the frame it paused at.
    let paused_frame = host.current_frame().as_i32();

    // Phase B: the joiner is SILENT. Drive the host alone for MORE than the
    // serve timeout (600 polls). The host must keep running and eventually
    // resume advancing once the serve aborts.
    let mut became_synchronizing = false;
    for _ in 0..900 {
        host.poll_remote_clients();
        for e in drain_events(&mut host) {
            // A Disconnected event would mean the host halted the slot/session.
            if matches!(e, FortressEvent::Disconnected { .. }) {
                became_synchronizing = true;
            }
        }
        if host.current_state() == SessionState::Synchronizing {
            became_synchronizing = true;
        }
        // The host keeps trying to advance; while paused this is Ok(empty), and
        // once the serve aborts it resumes advancing.
        if host.current_state() == SessionState::Running {
            let _ = advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 9);
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // (a) The host never fell back to Synchronizing (and emitted no Disconnected).
    assert!(
        !became_synchronizing,
        "host must NOT fall back to Synchronizing when a join is abandoned (MAJOR-2)"
    );
    assert_eq!(
        host.current_state(),
        SessionState::Running,
        "host must remain Running after an abandoned join"
    );

    // (b) The host resumed advancing solo after the serve timed out.
    assert!(
        host.current_frame().as_i32() > paused_frame,
        "host should resume advancing solo after the serve aborts; paused_frame={paused_frame}, now={}",
        host.current_frame()
    );

    // (c) The reserved slot is still frozen/Disconnected (frozen default input 0).
    let mut observed_reserved: Vec<(u32, InputStatus)> = Vec::new();
    for _ in 0..6 {
        host.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        host.add_local_input(PlayerHandle::new(0), StubInput { inp: 5 })?;
        let requests = host.advance_frame()?;
        for request in &*requests {
            if let FortressRequest::AdvanceFrame { inputs } = request {
                let inputs: &InputVec<StubInput> = inputs;
                if let Some(&(input, status)) = inputs.get(1) {
                    observed_reserved.push((input.inp, status));
                }
            }
        }
        host_stub.handle_requests(requests);
    }
    assert!(
        !observed_reserved.is_empty(),
        "expected to observe the still-reserved slot in AdvanceFrame inputs"
    );
    for (value, _status) in &observed_reserved {
        assert_eq!(
            *value, 0,
            "abandoned reserved slot must still report the frozen default input (0); got {observed_reserved:?}"
        );
    }
    assert!(
        observed_reserved
            .iter()
            .any(|(_, status)| *status == InputStatus::Disconnected),
        "abandoned reserved slot must still report Disconnected; got {observed_reserved:?}"
    );

    Ok(())
}

// ============================================================================
// FIX 1: checksum schedule must align to the host's interval grid for a
// MISALIGNED activation frame (F % interval != 0)
// ============================================================================

/// Hot-join with a desync-detection interval that does NOT divide the activation
/// frame `F`. Checksum comparison is by exact frame-number match and the host
/// (running from frame 0) only sends/stores checksums at multiples of `interval`.
/// If the joiner anchored its checksum schedule at `F` (the old code), its grid
/// would be `F+interval, F+2*interval, …` — offset from the host's grid by
/// `F % interval` and NEVER overlapping it, so neither side could ever compare a
/// checksum and `last_verified_frame()` would stay `None`. The fix re-roots the
/// joiner onto the host's global grid (first send at the first multiple of
/// `interval` that is >= F), so both sides verify matching checksums past `F`.
///
/// This test FAILS against the old `last_sent_checksum_frame = F` code (verified
/// by temporary revert) and PASSES with the grid-aligned fix.
#[test]
fn hot_join_misaligned_interval_still_verifies_checksums() -> Result<(), FortressError> {
    // Interval 3 is deliberately chosen NOT to divide the activation frame this
    // scenario produces (asserted non-vacuously below).
    const INTERVAL: u32 = 3;

    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join(true)
        .with_desync_detection_mode(DesyncDetection::On { interval: INTERVAL })
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;

    host.poll_remote_clients();
    assert_eq!(host.current_state(), SessionState::Running);
    let _ = drain_events(&mut host);

    let mut host_stub = GameStub::new();
    let mut host_states: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut joiner_states: BTreeMap<i32, StateStub> = BTreeMap::new();

    // Advance solo a few frames so a snapshot can be served.
    for i in 0..5_u32 {
        host.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        advance_and_record(
            &mut host,
            &mut host_stub,
            PlayerHandle::new(0),
            10 + i,
            &mut host_states,
        )?;
    }

    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_desync_detection_mode(DesyncDetection::On { interval: INTERVAL })
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    let mut joiner_stub = GameStub::new();
    let mut snapshot_frame: Option<Frame> = None;

    // Drive until the joiner loads the snapshot.
    for _ in 0..200 {
        host.poll_remote_clients();
        let _ = drain_events(&mut host);
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        if host.current_state() == SessionState::Running {
            let v = 100 + (host.current_frame().as_i32() as u32 % 7);
            advance_and_record(
                &mut host,
                &mut host_stub,
                PlayerHandle::new(0),
                v,
                &mut host_states,
            )?;
        }
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        if joiner.current_state() == SessionState::Running {
            let requests = joiner.advance_frame()?;
            for request in &*requests {
                if let FortressRequest::LoadGameState { frame, .. } = request {
                    snapshot_frame = Some(*frame);
                }
            }
            joiner_stub.handle_requests(requests);
            joiner_states.insert(joiner_stub.gs.frame, joiner_stub.gs);
            break;
        }
    }

    let snapshot_frame = snapshot_frame.expect("joiner must load a snapshot");

    // NON-VACUITY for FIX 1: the activation frame must be MISALIGNED to the
    // interval grid. If it happened to align, this test would not exercise the
    // bug. (Deterministic under TestClock, so this is a stable invariant.)
    assert_ne!(
        snapshot_frame.as_i32() % INTERVAL as i32,
        0,
        "test setup must produce a MISALIGNED activation frame (F={snapshot_frame:?}, interval={INTERVAL}); \
         adjust the solo-advance count or interval if this fires"
    );

    // Advance BOTH well past several interval boundaries so checksums are sent,
    // exchanged, and compared on both sides.
    for i in 0..30_u32 {
        for _ in 0..3 {
            host.poll_remote_clients();
            let _ = drain_events(&mut host);
            joiner.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
        if host.current_state() == SessionState::Running {
            advance_and_record(
                &mut host,
                &mut host_stub,
                PlayerHandle::new(0),
                1000 + i,
                &mut host_states,
            )?;
        }
        if joiner.current_state() == SessionState::Running {
            advance_and_record(
                &mut joiner,
                &mut joiner_stub,
                PlayerHandle::new(1),
                2000 + i,
                &mut joiner_states,
            )?;
        }
    }
    // Pure-poll drain so the last in-flight checksums land and are compared.
    for _ in 0..60 {
        host.poll_remote_clients();
        let _ = drain_events(&mut host);
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // No desync may have been reported on either side.
    let host_events = drain_events(&mut host);
    let joiner_events = drain_events(&mut joiner);
    assert!(
        !host_events
            .iter()
            .any(|e| matches!(e, FortressEvent::DesyncDetected { .. })),
        "host must not detect a desync with a misaligned interval; got {host_events:?}"
    );
    assert!(
        !joiner_events
            .iter()
            .any(|e| matches!(e, FortressEvent::DesyncDetected { .. })),
        "joiner must not detect a desync with a misaligned interval; got {joiner_events:?}"
    );

    // THE FIX-1 ASSERTION: the checksum desync gate must have actually RUN and
    // matched on BOTH sides past the (misaligned) activation frame. On the old
    // `= F` code the two grids never share a frame, so `last_verified_frame()`
    // stays `None` here and this fails. With the grid-aligned fix both advance.
    let host_verified = host.last_verified_frame().expect(
        "host must verify a checksum frame post-join with a MISALIGNED interval (FIX 1); \
         this is None on the old `last_sent_checksum_frame = F` code",
    );
    let joiner_verified = joiner.last_verified_frame().expect(
        "joiner must verify a checksum frame post-join with a MISALIGNED interval (FIX 1); \
         this is None on the old `last_sent_checksum_frame = F` code",
    );
    assert!(
        host_verified.as_i32() > snapshot_frame.as_i32(),
        "host must verify past the activation frame; verified={host_verified:?}, F={snapshot_frame:?}"
    );
    assert!(
        joiner_verified.as_i32() > snapshot_frame.as_i32(),
        "joiner must verify past the activation frame; verified={joiner_verified:?}, F={snapshot_frame:?}"
    );
    // The verified frames must lie on the host's global interval grid.
    assert_eq!(
        host_verified.as_i32() % INTERVAL as i32,
        0,
        "verified frame must be on the interval grid; got {host_verified:?}"
    );
    assert_eq!(
        joiner_verified.as_i32() % INTERVAL as i32,
        0,
        "verified frame must be on the interval grid; got {joiner_verified:?}"
    );

    // Byte-level no-desync gate at confirmed frames >= F (same as the headline).
    let min_confirmed = std::cmp::min(
        host.confirmed_frame().as_i32(),
        joiner.confirmed_frame().as_i32(),
    );
    let mut compared = 0;
    for (frame, host_state) in &host_states {
        if *frame < snapshot_frame.as_i32() || *frame > min_confirmed {
            continue;
        }
        if let Some(joiner_state) = joiner_states.get(frame) {
            assert_eq!(
                host_state, joiner_state,
                "host and joiner state must byte-equal at confirmed frame {frame}"
            );
            compared += 1;
        }
    }
    assert!(
        compared >= 3,
        "expected at least 3 overlapping confirmed frames to compare; got {compared}"
    );

    Ok(())
}

// ============================================================================
// FIX 2: the Phase-4 serve timeout (HOT_JOIN_SERVE_TIMEOUT_POLLS) is exercised
// when the joiner keeps its endpoint alive but never acks the snapshot
// ============================================================================

/// A joiner that KEEPS its endpoint alive (keeps polling, so keepalives/quality
/// flow and the endpoint never disconnect-times-out) but NEVER receives a
/// snapshot (the socket drops every `StateSnapshot` forever) drives the host into
/// its Phase-4 serve timeout. The host must: hit the timeout (drop the slot from
/// `joining` but KEEP it reserved), never fall back to `Synchronizing`, never
/// emit a user-facing `Disconnected`, and resume advancing solo. This pins the
/// Phase-4 backstop, which the existing abandoned-join test cannot reach (there
/// the endpoint disconnect-timeout preempts it).
#[test]
fn hot_join_phase4_serve_timeout_keeps_slot_reserved() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    // Drop EVERY snapshot delivered to the joiner, forever.
    let dropping = std::rc::Rc::new(std::cell::Cell::new(true));
    let dropped = std::rc::Rc::new(std::cell::Cell::new(0_usize));
    let joiner_socket = GatedDropSnapshotSocket {
        inner: joiner_socket,
        dropping: std::rc::Rc::clone(&dropping),
        dropped: std::rc::Rc::clone(&dropped),
    };

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join(true)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;
    host.poll_remote_clients();
    assert_eq!(host.current_state(), SessionState::Running);
    let _ = drain_events(&mut host);

    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    let mut host_stub = GameStub::new();
    let mut serve_started = false;
    let mut bad_state = false;
    let mut host_disconnect = false;
    let mut peer_joined = false;
    let mut join_requested = 0_usize;
    let mut paused_frame: Option<i32> = None;

    // Drive ~3 full timeout windows of polls (well past 600). The joiner keeps
    // polling — so its endpoint stays alive — but never gets a snapshot, so it
    // never acks. Expect the host to re-serve/timeout in a loop while staying
    // healthy and slowly advancing solo.
    for _ in 0..(HOT_JOIN_SERVE_TIMEOUT_POLLS_TEST * 3) {
        host.poll_remote_clients();
        for e in drain_events(&mut host) {
            match e {
                FortressEvent::JoinRequested { .. } => {
                    join_requested += 1;
                    serve_started = true;
                    if paused_frame.is_none() {
                        paused_frame = Some(host.current_frame().as_i32());
                    }
                },
                FortressEvent::PeerJoined { .. } => peer_joined = true,
                FortressEvent::Disconnected { .. } => host_disconnect = true,
                _ => {},
            }
        }
        if host.current_state() == SessionState::Synchronizing {
            bad_state = true;
        }
        if host.current_state() == SessionState::Running {
            let _ = advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 7);
        }
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // The host served at least once and the snapshots were genuinely dropped.
    assert!(
        serve_started,
        "host should have started serving at least once"
    );
    assert!(
        dropped.get() > 0,
        "snapshots must have been dropped (non-vacuous)"
    );
    // The Phase-4 timeout MUST have fired: the only way the host re-serves (a
    // second JoinRequested) is after a prior serve aborts at the timeout, and the
    // only way the host advances past where it first paused is unpausing via that
    // timeout (the join never completed — peer_joined is false).
    assert!(
        join_requested >= 2,
        "host must re-serve after a Phase-4 timeout (>=2 JoinRequested); got {join_requested}"
    );
    assert!(
        !peer_joined,
        "join must NOT have completed (snapshots dropped)"
    );

    // (a) Never fell back to Synchronizing and never emitted user-facing Disconnected.
    assert!(
        !bad_state,
        "host must NOT fall back to Synchronizing (FIX 2)"
    );
    assert!(
        !host_disconnect,
        "host must NOT emit a user-facing Disconnected on a Phase-4 timeout (FIX 2)"
    );
    assert_eq!(
        host.current_state(),
        SessionState::Running,
        "host must remain Running through repeated Phase-4 timeouts"
    );

    // (b) The host resumed advancing solo past where it first paused to serve.
    let paused_frame = paused_frame.expect("host must have paused to serve");
    assert!(
        host.current_frame().as_i32() > paused_frame,
        "host must resume advancing solo after the serve times out; paused={paused_frame}, now={}",
        host.current_frame()
    );

    // (c) The slot stays reserved/frozen: it still reports the frozen default
    // input (0) and Disconnected, exactly like a never-filled reserved slot.
    //
    // The joiner is now left SILENT (we stop polling it) so the host's current
    // open serve (if any) times out one final time and the host then advances
    // solo without re-serving. We drive enough polls to span a full timeout
    // window and collect the reserved slot's input whenever the host actually
    // advances (it returns an empty request set while paused mid-serve).
    let mut observed_reserved: Vec<(u32, InputStatus)> = Vec::new();
    for _ in 0..(HOT_JOIN_SERVE_TIMEOUT_POLLS_TEST + 50) {
        host.poll_remote_clients();
        let _ = drain_events(&mut host);
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        if host.current_state() != SessionState::Running {
            continue;
        }
        host.add_local_input(PlayerHandle::new(0), StubInput { inp: 5 })?;
        let requests = host.advance_frame()?;
        for request in &*requests {
            if let FortressRequest::AdvanceFrame { inputs } = request {
                let inputs: &InputVec<StubInput> = inputs;
                if let Some(&(input, status)) = inputs.get(1) {
                    observed_reserved.push((input.inp, status));
                }
            }
        }
        host_stub.handle_requests(requests);
    }
    assert!(
        !observed_reserved.is_empty(),
        "expected to observe the still-reserved slot in AdvanceFrame inputs"
    );
    for (value, _status) in &observed_reserved {
        assert_eq!(
            *value, 0,
            "timed-out reserved slot must still report the frozen default input (0); got {observed_reserved:?}"
        );
    }
    assert!(
        observed_reserved
            .iter()
            .any(|(_, status)| *status == InputStatus::Disconnected),
        "timed-out reserved slot must still report Disconnected; got {observed_reserved:?}"
    );

    Ok(())
}

// ============================================================================
// FIX 3a: a Phase-4 serve abort must NOT leave the host endpoint's pending_output
// growing without bound (no `Disconnected`-overflow spam)
// ============================================================================

/// With the joiner endpoint alive but the snapshot dropped forever, the host
/// enters a re-serve/timeout loop. On each Phase-4 abort the host clears the
/// joiner endpoint's `pending_output` (the abandoned joiner never needs those
/// pre-snapshot host inputs — a retry loads a snapshot). Without that cleanup the
/// queue accumulates ~1 entry per timeout cycle until it hits
/// `pending_output_limit` (128 by default), after which `send_input` emits a
/// suppressed `Event::Disconnected` on EVERY frame forever (the wedge/spam). This
/// test drives MANY timeout cycles and asserts the host->joiner queue stays tiny.
///
/// This FAILS against the no-cleanup code (the queue climbs to the 128 limit) and
/// PASSES with the `clear_pending_output()` fix (verified by temporary revert:
/// max_send_queue 128 without the fix, 1 with it).
#[test]
fn hot_join_phase4_abort_does_not_spam_send_queue() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    let dropping = std::rc::Rc::new(std::cell::Cell::new(true));
    let dropped = std::rc::Rc::new(std::cell::Cell::new(0_usize));
    let joiner_socket = GatedDropSnapshotSocket {
        inner: joiner_socket,
        dropping: std::rc::Rc::clone(&dropping),
        dropped: std::rc::Rc::clone(&dropped),
    };

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join(true)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;
    host.poll_remote_clients();
    let _ = drain_events(&mut host);

    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    let mut host_stub = GameStub::new();
    let mut join_requested = 0_usize;
    let mut max_send_queue = 0_usize;

    // Drive enough polls for many timeout cycles. With the default
    // pending_output_limit (128), the no-cleanup code reaches the limit only
    // after ~128 cycles; drive comfortably past that.
    for _ in 0..(HOT_JOIN_SERVE_TIMEOUT_POLLS_TEST * 200) {
        host.poll_remote_clients();
        for e in drain_events(&mut host) {
            if matches!(e, FortressEvent::JoinRequested { .. }) {
                join_requested += 1;
            }
        }
        if host.current_state() == SessionState::Running {
            let _ = advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 7);
        }
        if let Ok(stats) = host.network_stats(PlayerHandle::new(1)) {
            max_send_queue = max_send_queue.max(stats.send_queue_len);
        }
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Non-vacuity: many timeout cycles genuinely happened.
    assert!(
        join_requested >= 150,
        "test must drive many timeout cycles (>=150 serves); got {join_requested}"
    );
    assert!(dropped.get() > 0, "snapshots must have been dropped");

    // THE FIX-3a ASSERTION: the host->joiner pending_output must stay tiny. The
    // cleanup resets it to ~0 on every abort, so it never climbs toward the
    // overflow limit. A small cushion (well under 128) absorbs the at-most-one
    // frame the host advances between an abort and the next re-serve.
    assert!(
        max_send_queue <= 4,
        "host->joiner pending_output must NOT accumulate across Phase-4 aborts \
         (FIX 3a no-spam); got max_send_queue={max_send_queue} over {join_requested} cycles"
    );

    Ok(())
}

// ============================================================================
// FIX 3b: in-session retry — a join abandoned long enough to trip Phase-4 can be
// re-driven to completion by the SAME joiner once snapshots flow again
// ============================================================================

/// A join is abandoned (every snapshot dropped) long enough for the host's
/// Phase-4 timeout to fire and abort the serve. Because the slot stays reserved,
/// the still-alive joiner (which keeps re-sending `JoinRequest` while
/// `HotJoining`) re-opens a serve once snapshots flow again, and the join
/// completes IN-SESSION with matching post-join state. This proves the report's
/// "a returning joiner may retry from scratch with a fresh JoinRequest" claim is
/// real for the 2-peer reserved-slot path (and that `clear_pending_output()` on
/// abort leaves the endpoint able to re-serve cleanly).
#[test]
fn hot_join_in_session_retry_after_phase4_timeout_completes() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    let dropping = std::rc::Rc::new(std::cell::Cell::new(true));
    let dropped = std::rc::Rc::new(std::cell::Cell::new(0_usize));
    let joiner_socket = GatedDropSnapshotSocket {
        inner: joiner_socket,
        dropping: std::rc::Rc::clone(&dropping),
        dropped: std::rc::Rc::clone(&dropped),
    };

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join(true)
        .with_desync_detection_mode(DesyncDetection::On { interval: 2 })
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;
    host.poll_remote_clients();
    assert_eq!(host.current_state(), SessionState::Running);
    let _ = drain_events(&mut host);

    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_desync_detection_mode(DesyncDetection::On { interval: 2 })
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    let mut host_stub = GameStub::new();
    let mut joiner_stub = GameStub::new();
    let mut host_states: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut joiner_states: BTreeMap<i32, StateStub> = BTreeMap::new();

    let mut join_requested = 0_usize;
    let mut host_disconnect = false;
    let mut snapshot_frame: Option<Frame> = None;

    // Phase A: drop snapshots until the host has re-served at least once (>= 2
    // JoinRequested ⇒ a Phase-4 timeout fired), THEN allow snapshots so the next
    // re-serve completes. The joiner re-requests on its own while HotJoining.
    for _ in 0..(HOT_JOIN_SERVE_TIMEOUT_POLLS_TEST * 4) {
        host.poll_remote_clients();
        for e in drain_events(&mut host) {
            match e {
                FortressEvent::JoinRequested { .. } => join_requested += 1,
                FortressEvent::Disconnected { .. } => host_disconnect = true,
                _ => {},
            }
        }
        if host.current_state() == SessionState::Running {
            advance_and_record(
                &mut host,
                &mut host_stub,
                PlayerHandle::new(0),
                7,
                &mut host_states,
            )?;
        }
        joiner.poll_remote_clients();
        if joiner.current_state() == SessionState::Running {
            let requests = joiner.advance_frame()?;
            for request in &*requests {
                if let FortressRequest::LoadGameState { frame, .. } = request {
                    snapshot_frame = Some(*frame);
                }
            }
            joiner_stub.handle_requests(requests);
            joiner_states.insert(joiner_stub.gs.frame, joiner_stub.gs);
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        // Once a timeout has happened, stop dropping so the retry can complete.
        if join_requested >= 2 {
            dropping.set(false);
        }
        if snapshot_frame.is_some() {
            break;
        }
    }

    // The Phase-4 timeout fired (>=2 serves) and snapshots were genuinely dropped.
    assert!(
        join_requested >= 2,
        "a Phase-4 timeout must have fired before the retry (>=2 serves); got {join_requested}"
    );
    assert!(
        dropped.get() > 0,
        "snapshots must have been dropped during the abandon window"
    );
    let snapshot_frame =
        snapshot_frame.expect("the in-session retry must complete (joiner loads a snapshot)");
    assert!(
        !host_disconnect,
        "host must not emit a user-facing Disconnected across the abort+retry"
    );

    // Advance BOTH past the snapshot frame and converge.
    for i in 0..25_u32 {
        for _ in 0..3 {
            host.poll_remote_clients();
            for e in drain_events(&mut host) {
                if matches!(e, FortressEvent::Disconnected { .. }) {
                    host_disconnect = true;
                }
            }
            joiner.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
        if host.current_state() == SessionState::Running {
            advance_and_record(
                &mut host,
                &mut host_stub,
                PlayerHandle::new(0),
                1000 + i,
                &mut host_states,
            )?;
        }
        if joiner.current_state() == SessionState::Running {
            advance_and_record(
                &mut joiner,
                &mut joiner_stub,
                PlayerHandle::new(1),
                2000 + i,
                &mut joiner_states,
            )?;
        }
    }
    for _ in 0..40 {
        host.poll_remote_clients();
        for e in drain_events(&mut host) {
            if matches!(e, FortressEvent::Disconnected { .. }) {
                host_disconnect = true;
            }
        }
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    assert!(
        !host_disconnect,
        "host must not emit a user-facing Disconnected after the in-session retry completes"
    );

    let host_events = drain_events(&mut host);
    let joiner_events = drain_events(&mut joiner);
    assert!(
        !host_events
            .iter()
            .any(|e| matches!(e, FortressEvent::DesyncDetected { .. })),
        "host must not detect a desync after an in-session retry; got {host_events:?}"
    );
    assert!(
        !joiner_events
            .iter()
            .any(|e| matches!(e, FortressEvent::DesyncDetected { .. })),
        "joiner must not detect a desync after an in-session retry; got {joiner_events:?}"
    );

    // Post-retry state must match at confirmed frames >= the (retry) snapshot frame.
    let min_confirmed = std::cmp::min(
        host.confirmed_frame().as_i32(),
        joiner.confirmed_frame().as_i32(),
    );
    assert!(
        min_confirmed > snapshot_frame.as_i32(),
        "both peers must confirm past the retry snapshot frame; min_confirmed={min_confirmed}, F={snapshot_frame:?}"
    );
    let mut compared = 0;
    for (frame, host_state) in &host_states {
        if *frame < snapshot_frame.as_i32() || *frame > min_confirmed {
            continue;
        }
        if let Some(joiner_state) = joiner_states.get(frame) {
            assert_eq!(
                host_state, joiner_state,
                "host and joiner state must byte-equal at confirmed frame {frame} after retry"
            );
            compared += 1;
        }
    }
    assert!(
        compared >= 3,
        "expected at least 3 overlapping confirmed frames after retry; got {compared}"
    );

    Ok(())
}

// ============================================================================
// Regression: single snapshot send per host poll (host-side send dedup)
// ============================================================================

/// While a serve is open the host must send **at most one** `StateSnapshot` per
/// `poll_remote_clients` call. A prior version sent the snapshot once when
/// opening the serve (Phase 1) and then re-sent it again in Phase 2 of the same
/// poll, doubling the first poll's snapshot traffic and desyncing the
/// `polls_since_serve` timeout accounting. Phase 2 is now the sole send site, so
/// each open serve emits exactly one snapshot per poll.
#[test]
fn hot_join_host_sends_at_most_one_snapshot_per_poll() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    let snapshots_sent = std::rc::Rc::new(std::cell::Cell::new(0_usize));
    // Drop the host's outgoing snapshots so the joiner never acks: the serve
    // stays open for the whole run and we observe many consecutive serving polls,
    // each of which must send exactly one snapshot.
    let host_socket = CountingSocket {
        inner: host_socket,
        snapshots_sent: std::rc::Rc::clone(&snapshots_sent),
        acks_sent: std::rc::Rc::new(std::cell::Cell::new(0)),
        drop_outgoing_snapshots: true,
        drop_outgoing_acks: false,
    };

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join(true)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;

    // Advance solo a few frames so the host has a saved state to serve.
    let mut host_stub = GameStub::new();
    for _ in 0..5 {
        host.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 1)?;
    }
    let _ = drain_events(&mut host);

    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    let mut serving_polls = 0_usize;
    for _ in 0..40 {
        let before = snapshots_sent.get();
        host.poll_remote_clients();
        let delta = snapshots_sent.get() - before;
        assert!(
            delta <= 1,
            "host must send at most one snapshot per poll; sent {delta} this poll"
        );
        if delta == 1 {
            serving_polls += 1;
        }
        // Host is paused while the serve is open; advancing is a no-op but kept
        // for realism. (Solo advance only progresses once the serve closes.)
        if host.current_state() == SessionState::Running {
            let _ = advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 1);
        }
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Non-vacuity: the serve really did stay open and send across many polls.
    assert!(
        serving_polls >= 10,
        "expected many serving polls (each sending one snapshot); got {serving_polls}"
    );

    Ok(())
}

// ============================================================================
// Regression: single ack send per joiner poll (joiner-side send dedup)
// ============================================================================

/// After applying the snapshot the joiner must send **at most one**
/// `StateSnapshotAck` per `poll_remote_clients` call, even while duplicate
/// snapshots keep arriving. A prior version could send two acks on such a poll
/// (the bounded resend AND the duplicate-snapshot handler both fired); the ack
/// path is now a single send site. Also exercises
/// [`SessionBuilder::with_hot_join_ack_resends`].
#[test]
fn hot_join_joiner_sends_at_most_one_ack_per_poll() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    let acks_sent = std::rc::Rc::new(std::cell::Cell::new(0_usize));
    // Drop the joiner's outgoing acks so the host never completes the join and
    // keeps re-sending the snapshot every poll. The joiner therefore receives a
    // duplicate snapshot on every poll while Running — the exact condition that
    // used to trigger a double ack.
    let joiner_socket = CountingSocket {
        inner: joiner_socket,
        snapshots_sent: std::rc::Rc::new(std::cell::Cell::new(0)),
        acks_sent: std::rc::Rc::clone(&acks_sent),
        drop_outgoing_snapshots: false,
        drop_outgoing_acks: true,
    };

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join(true)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;

    let mut host_stub = GameStub::new();
    for _ in 0..5 {
        host.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 1)?;
    }
    let _ = drain_events(&mut host);

    // Long ack-resend budget so the joiner keeps acking for the whole run.
    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join_ack_resends(50)
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    let mut max_acks_per_poll = 0_usize;
    let mut polls_with_ack = 0_usize;
    for _ in 0..50 {
        host.poll_remote_clients();
        if host.current_state() == SessionState::Running {
            let _ = advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 1);
        }
        let before = acks_sent.get();
        joiner.poll_remote_clients();
        let delta = acks_sent.get() - before;
        assert!(
            delta <= 1,
            "joiner must send at most one ack per poll; sent {delta} this poll"
        );
        max_acks_per_poll = max_acks_per_poll.max(delta);
        if delta == 1 {
            polls_with_ack += 1;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Non-vacuity: the joiner really did re-ack across many polls (so a
    // double-ack would have been observed had it regressed).
    assert!(
        polls_with_ack >= 5,
        "expected the joiner to re-ack across several polls; got {polls_with_ack}"
    );
    assert_eq!(
        max_acks_per_poll, 1,
        "the joiner must have acked (max per poll == 1), never zero-throughout nor doubled"
    );

    Ok(())
}

// ============================================================================
// Regression: serve-timeout is configurable and aborts at EXACTLY N polls
// ============================================================================

/// A serve timeout of `0` is nonsensical (the serve would be aborted before any
/// joiner could complete the handshake) and must be rejected at the builder
/// boundary.
#[test]
fn with_hot_join_serve_timeout_polls_zero_is_rejected() -> Result<(), FortressError> {
    let result = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_hot_join_serve_timeout_polls(0);
    assert!(
        matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: fortress_rollback::InvalidRequestKind::NotSupported { .. }
            })
        ),
        "a zero serve timeout must be rejected"
    );
    Ok(())
}

/// With a custom (small) serve timeout of `N`, each in-flight serve must stay
/// open for **exactly** `N` polls — sending exactly `N` snapshots — before the
/// host aborts it and (because the slot stays reserved) re-opens a fresh serve
/// on the next `JoinRequest`. This pins both the configurability of the timeout
/// and the inclusive `>= N` boundary (a prior `> N` kept the serve open one poll
/// — one extra snapshot — too long).
#[test]
fn hot_join_custom_serve_timeout_aborts_at_exactly_configured_polls() -> Result<(), FortressError> {
    const N: usize = 4;

    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    let snapshots_sent = std::rc::Rc::new(std::cell::Cell::new(0_usize));
    // Drop the host's outgoing snapshots so the joiner never acks: every serve
    // runs to its timeout, then the still-HotJoining joiner's next JoinRequest
    // re-opens a fresh serve. We count snapshots between consecutive
    // JoinRequested events.
    let host_socket = CountingSocket {
        inner: host_socket,
        snapshots_sent: std::rc::Rc::clone(&snapshots_sent),
        acks_sent: std::rc::Rc::new(std::cell::Cell::new(0)),
        drop_outgoing_snapshots: true,
        drop_outgoing_acks: false,
    };

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join(true)
        .with_hot_join_serve_timeout_polls(N)?
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;

    let mut host_stub = GameStub::new();
    for _ in 0..5 {
        host.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 1)?;
    }
    let _ = drain_events(&mut host);

    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    // Snapshot count recorded at each JoinRequested event; the gap between
    // consecutive records is the number of snapshots one serve sent.
    let mut last_at_join_req: Option<usize> = None;
    let mut per_serve_snapshots: Vec<usize> = Vec::new();
    for _ in 0..(N * 10) {
        host.poll_remote_clients();
        for e in drain_events(&mut host) {
            if matches!(e, FortressEvent::JoinRequested { .. }) {
                let now = snapshots_sent.get();
                if let Some(prev) = last_at_join_req {
                    per_serve_snapshots.push(now - prev);
                }
                last_at_join_req = Some(now);
            }
        }
        if host.current_state() == SessionState::Running {
            let _ = advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 1);
        }
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    assert!(
        per_serve_snapshots.len() >= 3,
        "expected several serve cycles to measure; got {per_serve_snapshots:?}"
    );
    for count in &per_serve_snapshots {
        assert_eq!(
            *count, N,
            "each serve must send exactly N={N} snapshots before the timeout aborts it; got {per_serve_snapshots:?}"
        );
    }

    Ok(())
}

// ============================================================================
// Regression: reserved-slot disconnect during an in-flight serve (Class A)
// ============================================================================

/// If the reserved-slot endpoint disconnects (its protocol disconnect-timeout
/// fires) **while a serve is in flight**, the host must abort that serve through
/// the single teardown path — which keeps the slot reserved/frozen and clears
/// the endpoint's stale `pending_output` exactly like the Phase-4 timeout path —
/// without emitting a user-facing `Disconnected` or halting. (The pending_output
/// clear itself is not separately observable here because a disconnected
/// endpoint's `network_stats` is unavailable; it is guaranteed structurally by
/// the shared `abort_hot_join_serve` helper and pinned for the timeout path by
/// `hot_join_phase4_abort_does_not_spam_send_queue`.)
#[test]
fn hot_join_reserved_disconnect_during_serve_keeps_host_running() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (host_socket, joiner_socket, host_addr, joiner_addr) = crate::common::create_channel_pair();

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_hot_join(true)
        // Short timeouts so the joiner-endpoint disconnect fires quickly once it
        // goes silent (notify < timeout).
        .with_disconnect_notify_delay(std::time::Duration::from_millis(200))
        .with_disconnect_timeout(std::time::Duration::from_millis(600))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_reserved_player(joiner_addr, PlayerHandle::new(1))?
        .start_p2p_session(host_socket)?;

    let mut host_stub = GameStub::new();
    for _ in 0..5 {
        host.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 1)?;
    }
    let _ = drain_events(&mut host);

    let mut joiner = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Remote(host_addr), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_hot_join_session(joiner_socket, host_addr)?;

    // Drive both until the host opens a serve (JoinRequested), then immediately
    // stop polling the joiner so its endpoint goes silent mid-serve.
    let mut serve_opened = false;
    let mut host_disconnect_event = false;
    for _ in 0..200 {
        host.poll_remote_clients();
        for e in drain_events(&mut host) {
            match e {
                FortressEvent::JoinRequested { .. } => serve_opened = true,
                FortressEvent::Disconnected { .. } => host_disconnect_event = true,
                _ => {},
            }
        }
        if serve_opened {
            break;
        }
        if host.current_state() == SessionState::Running {
            let _ = advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 1);
        }
        joiner.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    assert!(serve_opened, "host must open a serve before the disconnect");

    // Joiner now silent. Advance the host clock past the disconnect timeout so
    // the joiner endpoint's protocol disconnect fires while the serve is open.
    let mut endpoint_disconnected = false;
    for _ in 0..60 {
        host.poll_remote_clients();
        for e in drain_events(&mut host) {
            if matches!(e, FortressEvent::Disconnected { .. }) {
                host_disconnect_event = true;
            }
        }
        if host.current_state() == SessionState::Running {
            let _ = advance_session(&mut host, &mut host_stub, PlayerHandle::new(0), 1);
        }
        // Once the reserved endpoint's protocol has disconnected, its network
        // stats become unavailable — our proof the disconnect path actually ran.
        if host.network_stats(PlayerHandle::new(1)).is_err() {
            endpoint_disconnected = true;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Non-vacuity: the disconnect path really fired.
    assert!(
        endpoint_disconnected,
        "the reserved joiner endpoint must have disconnected (its stats became unavailable)"
    );
    // The reserved-slot exemption must hold: no user-facing Disconnected, and the
    // host keeps running solo rather than halting.
    assert!(
        !host_disconnect_event,
        "host must NOT emit a user-facing Disconnected for a reserved-slot disconnect"
    );
    assert_eq!(
        host.current_state(),
        SessionState::Running,
        "host must keep running solo after a reserved-slot disconnect during a serve"
    );
    // The slot stays reserved/frozen (reports Disconnected input) after the
    // abort, so a peer can still retry. Read its status from the AdvanceFrame
    // request, exactly like the solo-reserved-slot checks above.
    host.add_local_input(PlayerHandle::new(0), StubInput { inp: 1 })?;
    let requests = host.advance_frame()?;
    let mut reserved_status = None;
    for request in &*requests {
        if let FortressRequest::AdvanceFrame { inputs } = request {
            let inputs: &InputVec<StubInput> = inputs;
            if let Some(&(_, status)) = inputs.get(1) {
                reserved_status = Some(status);
            }
        }
    }
    host_stub.handle_requests(requests);
    assert_eq!(
        reserved_status,
        Some(InputStatus::Disconnected),
        "reserved slot must still report Disconnected after the abort"
    );

    Ok(())
}
