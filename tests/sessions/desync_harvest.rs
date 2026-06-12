//! End-to-end pipeline smoke test for the Session-26 "desync-checksum harvest"
//! lead (NORMAL prediction-rollback path, NOT the disconnect path which is
//! finding F11).
//!
//! ## What the lead asked
//!
//! Can `check_checksum_send_interval` (`src/sessions/p2p_session.rs`) ever
//! harvest, store, and gossip a checksum taken from a saved cell that was last
//! written during SPECULATIVE simulation (predicted inputs) even though
//! `frame_to_send <= last_confirmed_frame`? If so, two honest peers whose
//! CONFIRMED game state is byte-identical could still exchange differing
//! harvested checksums and raise a FALSE-POSITIVE `DesyncDetected`.
//!
//! BINDING ARBITRATION VERDICT (S29): **REAL** — a genuine library bug. An
//! earlier session (S27) arbitrated it NOTABUG; that verdict was OVERTURNED by
//! a decisive per-frame-digest experiment on the real-UDP `network_test_peer`
//! (see below). **FIXED (S30) at the root cause**, which S30's adversarial
//! review re-rooted from `adjust_gamestate` into the **InputQueue prediction
//! lifecycle** (`src/input_queue/mod.rs`): a prediction episode re-entered by a
//! rollback re-simulation used to start at the REQUESTED frame instead of the
//! queue's first missing frame, so arrivals for the skipped window
//! `[first_missing, requested)` were physically added but their misprediction
//! comparison was silently swallowed — `first_incorrect_frame` never fired for
//! that window, no rollback ever re-simulated it, and the victim's applied
//! trajectory (and every cell saved from it) stayed stale. Save semantics, for
//! precision: `cell[F]` holds the state BEFORE frame `F`'s inputs are applied —
//! a pure function of the inputs at frames STRICTLY BELOW `F` (the S29 header
//! previously said "AFTER", which was wrong) — so the staleness in `cell[F]`
//! lives in swallowed predicted inputs at frames below `F`. The
//! `adjust_gamestate` `if i > 0` save guard (~`src/sessions/p2p_session.rs:3555`),
//! which never re-saves the loaded boundary cell, is an ACCESSORY: it preserves
//! an already-stale boundary cell, but with every arrival compared (the S30
//! fix) the boundary cell is provably always clean (a rollback to
//! `first_incorrect == F` implies every applied frame `< F` matched its
//! confirmed input). Once `last_confirmed_frame` advanced past the stale window,
//! the harvest in `check_checksum_send_interval` — which runs at the TOP of
//! `advance_frame` (line ~642), BEFORE the rollback and BEFORE
//! `set_last_confirmed_frame` — read, STORED, and SENT stale cells' checksums
//! even though `F <= last_confirmed_frame && F <= last_saved_frame`. Honest
//! peers whose CONFIRMED state was byte-identical then exchanged differing
//! harvested checksums and raised a FALSE-POSITIVE `DesyncDetected`.
//!
//! WHY S27's "structural impossibility" argument is WRONG: S27 reasoned that
//! the invariant `last_confirmed_frame <= first_incorrect` guarantees a confirmed
//! frame's cell "has been re-saved with confirmed inputs after any rollback". It
//! has NOT: the rollback LOADS `cell[first_incorrect]` but the `i > 0` guard
//! never re-SAVES it, so the boundary cell keeps its speculative content. The
//! invariant constrains where rollback *reloads*, not whether the boundary cell
//! was rewritten.
//!
//! ORCHESTRATOR'S DECISIVE EXPERIMENT (refutes "faithful game unaffected"):
//! replacing the binary's checksum with a NON-accumulator faithful per-frame
//! digest = `hash(frame, [each player's input.value])` (overwritten each frame,
//! NOT a running accumulator) STILL fired `DesyncDetected total=4` at frame 60 in
//! ~5/12 real-UDP 3-peer runs at 0% loss. In a captured failure ALL three peers
//! had IDENTICAL final confirmed-window checksums (so the confirmed inputs at the
//! boundary frame are provably byte-identical), YET the odd peer STORED a
//! different value for the interval frame. Since that digest is a deterministic
//! function of the confirmed inputs, the odd peer's stored value could only have
//! been computed from PREDICTED inputs ⇒ its saved boundary cell was genuinely
//! STALE at harvest time. (Swapping to a hash-of-full-state checksum — GameStub's
//! faithful style — also still fired total=4, confirming the symptom is
//! independent of checksum content: the SAVED CELL is stale, and no game-side
//! checksum function can rescue a cell the library never re-requests a save of.)
//!
//! The two unit tests in `src/sessions/p2p_session.rs`
//! (`checksum_harvest_uses_exact_frame_to_send_cell_not_a_later_cell` and
//! `checksum_harvest_skips_speculative_cell_above_last_confirmed_frame`) remain
//! VALID for exactly what they assert — the exact-frame `saved_state_by_frame`
//! match and the `<= last_confirmed_frame` harvest gate. They do NOT, and cannot,
//! exercise the un-re-saved boundary cell, so they neither prove nor disprove the
//! REAL verdict. Tracked in `progress/session-29-*` and `N-PLAYER-DESYNC-AUDIT.md`.
//! The `src/` fix landed in S30 (InputQueue prediction-episode entry at the
//! first missing frame): see the "F17 DETERMINISTIC REPRODUCTION" section and
//! the "NEXT ACTION" note at the bottom of this file.
//!
//! ## What THIS test is (and is NOT)
//!
//! This is a high-level **no-regression / pipeline smoke test**. It exercises the
//! full harvest -> send (`ChecksumReport`) -> compare
//! (`compare_local_checksums_against_peers`) -> `DesyncDetected` pipeline
//! end-to-end at N=3 with 0% packet loss under DEEP prediction and rollback, and
//! asserts (1) no false-positive `DesyncDetected` fires and (2) the confirmed
//! game states are byte-identical across all three peers. That is valuable
//! coverage — it catches a regression that would wire the harvest/compare
//! pipeline to a false positive under realistic deep-rollback traffic — but it is
//! NOT the red test for the harvest invariant.
//!
//! ### Why it is NOT the invariant's red test
//!
//! The harness has SYMMETRIC deterministic prediction: every peer predicts the
//! delayed peer's inputs with the same `RepeatLastConfirmed` strategy from the
//! same observed history, so all peers harvest the SAME speculative checksum at a
//! given checksum-interval frame even when that speculative value is "wrong".
//! `compare_local_checksums_against_peers` therefore never sees a mismatch,
//! whether or not the library is correct. Concretely, neutralizing the harvest in
//! `src/` (e.g. keying the last-saved cell's checksum as `frame_to_send`, or
//! dropping the `<= last_confirmed_frame` guard) does NOT turn this test red. So
//! this test passes regardless of the harvest invariant's correctness; the
//! invariant itself is constrained by the two unit tests named above, not here.
//! See the unit tests for the provable red->green transitions.
//!
//! ## Harness
//!
//! 3 peers over in-memory channel sockets with a deterministic DELAYING socket on
//! one peer's outbound links. The delay defers — but never drops — that peer's
//! input packets (0% loss, no disconnect), forcing the other two peers to
//! deep-predict its inputs and then ROLL BACK when the real (per-frame-distinct,
//! hence repeatedly mispredicted) inputs finally arrive, crossing the
//! checksum-send interval. The faithful checksum ([`GameStub`]'s
//! `save_game_state` hashes the full `StateStub { frame, state }`, a pure
//! function of the confirmed-input sequence) means a genuine confirmed-state
//! agreement MUST agree on checksums, so any `DesyncDetected` here would be a real
//! library false-positive.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use fortress_rollback::{
    DesyncDetection, FortressEvent, Frame, Message, NonBlockingSocket, PlayerHandle, PlayerType,
    SessionBuilder, SessionState,
};

use crate::common::reorder_socket::create_reorder_mesh_triple;
use crate::common::stubs::{GameStub, StateStub, StubConfig, StubInput};

/// A [`NonBlockingSocket`] wrapper that defers every outbound message by a fixed
/// number of "release ticks". Messages are NEVER dropped: each is stored with a
/// release tick and flushed into the underlying inbox once the shared tick
/// counter reaches it, in FIFO order. This models pure latency (0% packet loss),
/// which is exactly what the lead requires (deep prediction + rollback with no
/// disconnect and no loss).
struct DelaySocket {
    local_addr: SocketAddr,
    /// Per-destination inbox shared with every other socket on the mesh.
    inboxes: Arc<Mutex<BTreeMap<SocketAddr, VecDeque<(SocketAddr, Message)>>>>,
    /// Outbound messages waiting to be released, tagged with their release tick.
    pending: VecDeque<(u64, SocketAddr, Message)>,
    /// Shared monotonically-increasing tick counter.
    tick: Arc<Mutex<u64>>,
    /// How many ticks to defer this socket's outbound messages. `0` = immediate.
    delay: u64,
}

impl DelaySocket {
    /// Releases all pending messages whose release tick has arrived.
    fn flush_due(&mut self) {
        let now = *self.tick.lock().unwrap();
        let mut inboxes = self.inboxes.lock().unwrap();
        while let Some(&(release_at, _, _)) = self.pending.front() {
            if release_at > now {
                break;
            }
            let (_, dst, msg) = self.pending.pop_front().unwrap();
            inboxes
                .entry(dst)
                .or_default()
                .push_back((self.local_addr, msg));
        }
    }
}

impl NonBlockingSocket<SocketAddr> for DelaySocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        if self.delay == 0 {
            self.inboxes
                .lock()
                .unwrap()
                .entry(*addr)
                .or_default()
                .push_back((self.local_addr, msg.clone()));
        } else {
            let release_at = *self.tick.lock().unwrap() + self.delay;
            self.pending.push_back((release_at, *addr, msg.clone()));
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        // Releasing on receive keeps delivery driven entirely by the poll loop:
        // each poll round flushes whatever became due since the last tick.
        self.flush_due();
        let mut inboxes = self.inboxes.lock().unwrap();
        match inboxes.get_mut(&self.local_addr) {
            Some(queue) => queue.drain(..).collect(),
            None => Vec::new(),
        }
    }
}

/// Builds three [`DelaySocket`]s on a shared mesh. Only peer index `delayed_peer`
/// has a non-zero outbound delay; the others deliver immediately.
fn build_delay_mesh(
    addrs: [SocketAddr; 3],
    delayed_peer: usize,
    delay: u64,
) -> (Arc<Mutex<u64>>, [DelaySocket; 3]) {
    let mut delays = [0u64; 3];
    delays[delayed_peer] = delay;
    build_delay_mesh_per_peer(addrs, delays)
}

/// Builds three [`DelaySocket`]s on a shared mesh with an INDEPENDENT outbound
/// delay per peer. Unlike [`build_delay_mesh`] (single delayed peer, symmetric
/// from the perspective of the two observers), this lets every peer sit at a
/// DIFFERENT prediction depth relative to every other peer, which is the
/// symmetry-breaker the asymmetric-harvest red test needs: with `[0, 1, 2]`,
/// peer 0 sees peer 1 one tick late and peer 2 two ticks late, peer 1 sees peer
/// 2 two ticks late, etc., so the three peers predict each other to DIFFERENT
/// depths and can harvest DIFFERENT speculative checksums for the same interval
/// frame.
fn build_delay_mesh_per_peer(
    addrs: [SocketAddr; 3],
    delays: [u64; 3],
) -> (Arc<Mutex<u64>>, [DelaySocket; 3]) {
    let inboxes = Arc::new(Mutex::new(BTreeMap::new()));
    let tick = Arc::new(Mutex::new(0u64));
    let mk = |i: usize| DelaySocket {
        local_addr: addrs[i],
        inboxes: Arc::clone(&inboxes),
        pending: VecDeque::new(),
        tick: Arc::clone(&tick),
        delay: delays[i],
    };
    (Arc::clone(&tick), [mk(0), mk(1), mk(2)])
}

/// Polls all three sessions and advances the shared delivery tick a few times so
/// deferred packets are released in order.
fn pump(sessions: &mut [fortress_rollback::P2PSession<StubConfig>; 3], tick: &Arc<Mutex<u64>>) {
    for _ in 0..4 {
        for s in sessions.iter_mut() {
            s.poll_remote_clients();
        }
        *tick.lock().unwrap() += 1;
    }
}

/// Faithful-checksum pipeline smoke test under deep prediction + rollback. See
/// the module docs for why this is NOT the invariant's red test (symmetric
/// prediction makes all peers harvest the same speculative value, so it cannot
/// distinguish a correct harvest from a neutralized one). The genuinely
/// library-constraining red->green coverage lives in the
/// `checksum_harvest_*` unit tests in `src/sessions/p2p_session.rs`.
///
/// ORACLE (all must hold; (1)+(2) are the smoke-test value, (3) verifies the
/// premise that the test actually exercised deep rollback):
///   1. No `DesyncDetected` event fires on ANY peer under the full
///      harvest->send->compare pipeline.
///   2. The confirmed game states are byte-identical across all three peers at
///      every commonly-confirmed frame (so the inputs genuinely agreed and the
///      faithful checksum SHOULD match).
///   3. Genuine rollbacks occurred on the predicting peers (counted via
///      `LoadGameState` requests, each of which begins a rollback), proving the
///      deep-prediction-rollback premise was actually exercised rather than
///      assumed.
#[test]
fn faithful_checksum_no_false_desync_under_deep_prediction_rollback() {
    const INTERVAL: u32 = 2;
    const DELAY: u64 = 3;
    const FRAMES: u32 = 80;

    let addrs: [SocketAddr; 3] = [
        ([127, 0, 0, 1], 30001).into(),
        ([127, 0, 0, 1], 30002).into(),
        ([127, 0, 0, 1], 30003).into(),
    ];

    // Peer 2 (handle index 2) is the deep-prediction racer: its inputs reach the
    // others late, so peers 0 and 1 predict and then roll back when the real,
    // per-frame-distinct inputs land.
    let (tick, [sock0, sock1, sock2]) = build_delay_mesh(addrs, 2, DELAY);

    let build = |local: usize, sock: DelaySocket| {
        let mut b = SessionBuilder::<StubConfig>::new()
            .with_num_players(3)
            .unwrap()
            .with_desync_detection_mode(DesyncDetection::On { interval: INTERVAL });
        for (h, addr) in addrs.iter().enumerate() {
            b = if h == local {
                b.add_player(PlayerType::Local, PlayerHandle::new(h))
                    .unwrap()
            } else {
                b.add_player(PlayerType::Remote(*addr), PlayerHandle::new(h))
                    .unwrap()
            };
        }
        b.start_p2p_session(sock).unwrap()
    };

    let mut sessions = [build(0, sock0), build(1, sock1), build(2, sock2)];

    // Synchronize.
    let mut synced = false;
    for _ in 0..2000 {
        pump(&mut sessions, &tick);
        if sessions
            .iter()
            .all(|s| s.current_state() == SessionState::Running)
        {
            synced = true;
            break;
        }
    }
    assert!(synced, "all three sessions should synchronize");

    let mut stubs = [GameStub::new(), GameStub::new(), GameStub::new()];
    // Per-peer record of the most-recent state observed at each (post-advance)
    // frame. After the run, the entry at a confirmed frame is the confirmed
    // state (rollback re-simulations overwrite speculative entries).
    let mut states: [BTreeMap<i32, StateStub>; 3] =
        [BTreeMap::new(), BTreeMap::new(), BTreeMap::new()];

    let mut desync_events: Vec<(usize, i32, u128, u128)> = Vec::new();

    // Per-peer rollback count. Every rollback begins with a `LoadGameState`
    // request (the engine restores the last confirmed cell before re-simulating),
    // so counting those is a faithful proxy for "a rollback happened". Peers 0 and
    // 1 deep-predict the delayed peer 2 and must roll back when its real inputs
    // land, so these counts must be well above zero — that is ORACLE PART 3.
    let mut rollbacks = [0usize; 3];

    for f in 0..FRAMES {
        pump(&mut sessions, &tick);

        // Distinct, per-frame-VARYING inputs per peer. Variation is what makes
        // RepeatLastConfirmed mispredict the delayed peer, forcing rollbacks
        // that cross the checksum interval.
        let inputs = [
            StubInput { inp: f * 7 + 1 },
            StubInput { inp: f * 13 + 2 },
            StubInput { inp: f * 31 + 5 },
        ];

        for (i, s) in sessions.iter_mut().enumerate() {
            // Each session only owns its own local handle.
            if s.add_local_input(PlayerHandle::new(i), inputs[i]).is_err() {
                // Prediction window full this round; pump and retry next loop.
                continue;
            }
        }

        for (i, s) in sessions.iter_mut().enumerate() {
            match s.advance_frame() {
                Ok(reqs) => {
                    rollbacks[i] += reqs
                        .iter()
                        .filter(|r| {
                            matches!(r, fortress_rollback::FortressRequest::LoadGameState { .. })
                        })
                        .count();
                    stubs[i].handle_requests_recording(reqs, &mut states[i]);
                },
                Err(fortress_rollback::FortressError::PredictionThreshold) => { /* skip */ },
                Err(e) => panic!("peer {i} advance_frame failed: {e:?}"),
            }
        }

        for (i, s) in sessions.iter_mut().enumerate() {
            for ev in s.events() {
                if let FortressEvent::DesyncDetected {
                    frame,
                    local_checksum,
                    remote_checksum,
                    ..
                } = ev
                {
                    desync_events.push((i, frame.as_i32(), local_checksum, remote_checksum));
                }
            }
        }
    }

    // Drain stragglers.
    for _ in 0..40 {
        pump(&mut sessions, &tick);
        for (i, s) in sessions.iter_mut().enumerate() {
            for ev in s.events() {
                if let FortressEvent::DesyncDetected {
                    frame,
                    local_checksum,
                    remote_checksum,
                    ..
                } = ev
                {
                    desync_events.push((i, frame.as_i32(), local_checksum, remote_checksum));
                }
            }
        }
    }

    // --- ORACLE PART 2: confirmed states are byte-identical across peers. ---
    // Only compare frames every peer has a recorded (and hence confirmed-by-end)
    // state for, up to the slowest peer's confirmed frame. This proves the
    // confirmed inputs genuinely agreed, so a faithful checksum MUST match —
    // making any DesyncDetected a true false-positive.
    let min_confirmed = sessions
        .iter()
        .map(|s| s.confirmed_frame().as_i32())
        .min()
        .unwrap_or(0);
    assert!(
        min_confirmed > i64::from(INTERVAL) as i32,
        "test must drive past the first checksum interval (min_confirmed={min_confirmed})"
    );

    let mut compared = 0;
    for frame in 1..=min_confirmed {
        let s0 = states[0].get(&frame);
        let s1 = states[1].get(&frame);
        let s2 = states[2].get(&frame);
        if let (Some(a), Some(b), Some(c)) = (s0, s1, s2) {
            assert_eq!(
                a, b,
                "confirmed state mismatch peer0 vs peer1 at frame {frame}"
            );
            assert_eq!(
                a, c,
                "confirmed state mismatch peer0 vs peer2 at frame {frame}"
            );
            compared += 1;
        }
    }
    assert!(
        compared >= 10,
        "expected to compare many confirmed frames, only compared {compared}"
    );

    // --- ORACLE PART 3: the deep-prediction-rollback premise actually held. ---
    // The two non-delayed peers (0 and 1) predict the delayed peer 2 and must roll
    // back when its real, per-frame-distinct inputs arrive. If NO rollbacks
    // occurred the run would not exercise the harvest window the lead is about, so
    // PARTS 1 and 2 would be vacuous w.r.t. "deep prediction rollback". Assert the
    // predicting peers genuinely rolled back.
    assert!(
        rollbacks[0] > 0 && rollbacks[1] > 0,
        "deep-prediction-rollback premise unmet: predicting peers rolled back \
         {} (peer0) and {} (peer1) times; expected both > 0",
        rollbacks[0],
        rollbacks[1]
    );

    // --- ORACLE PART 1: no false-positive DesyncDetected. ---
    assert!(
        desync_events.is_empty(),
        "FALSE-POSITIVE DesyncDetected with a FAITHFUL checksum and byte-identical \
         confirmed states (compared {compared} frames): {desync_events:?}"
    );
}

/// Structural outcome of one ASYMMETRIC-delay run, with a GROUND-TRUTH oracle on
/// the confirmed *inputs* (not just the recorded states) so we can decide, for
/// every `DesyncDetected`, whether it is a TRUE positive (the confirmed inputs at
/// that frame genuinely diverged across peers) or a FALSE positive (the confirmed
/// inputs agreed, so the faithful checksum should have matched).
#[derive(Debug, PartialEq, Eq)]
struct AsymmetricOutcome {
    /// `(observing_peer, frame, local_checksum, remote_checksum)` for every
    /// `DesyncDetected` event seen on any peer.
    desync_events: Vec<(usize, i32, u128, u128)>,
    /// Frames whose CONFIRMED inputs (the inputs applied to reach the frame, last
    /// write wins = the final/confirmed re-simulation) DIFFERED across peers. A
    /// desync at one of these frames is a genuine state divergence, NOT a false
    /// positive. (Empirically these arise when the harness drives peers to confirm
    /// a frame before the slowest peer's real input is delivered — i.e. an
    /// artifact of an overly-aggressive pump cadence, not a library defect.)
    confirmed_input_mismatch_frames: Vec<i32>,
    /// Count of frames whose confirmed inputs AGREED across all three peers.
    confirmed_input_agree_frames: usize,
    /// Rollback counts per peer (proves the deep-prediction-rollback premise).
    rollbacks: [usize; 3],
}

impl AsymmetricOutcome {
    /// The genuine false positives: `DesyncDetected` events fired on frames whose
    /// confirmed inputs AGREED across peers (so the confirmed state — and its
    /// faithful checksum — must also agree, making the event spurious).
    fn false_positive_desync_frames(&self) -> Vec<i32> {
        self.desync_events
            .iter()
            .map(|&(_, frame, _, _)| frame)
            .filter(|f| !self.confirmed_input_mismatch_frames.contains(f))
            .collect()
    }
}

/// Drives the asymmetric-delay 3-peer scenario end-to-end and returns a
/// structural [`AsymmetricOutcome`]. Pure function of its inputs (fixed
/// addresses, shared logical tick, no wall-clock), so two calls with identical
/// arguments MUST produce identical outcomes — that is what
/// [`asymmetric_delay_deep_rollback_is_deterministic`] asserts.
///
/// The confirmed-input oracle is built by mirroring the [`GameStub`] frame
/// counter through each peer's request stream: a `LoadGameState` resets the
/// cursor to the loaded cell's frame, each `AdvanceFrame` increments it, and the
/// applied inputs are keyed by the resulting frame with last-write-wins — so the
/// final entry at a confirmed frame is the inputs of its confirmed re-simulation.
fn run_asymmetric_scenario(
    delays: [u64; 3],
    interval: u32,
    frames: u32,
    base_port: u16,
) -> AsymmetricOutcome {
    let addrs: [SocketAddr; 3] = [
        ([127, 0, 0, 1], base_port).into(),
        ([127, 0, 0, 1], base_port + 1).into(),
        ([127, 0, 0, 1], base_port + 2).into(),
    ];

    let (tick, [sock0, sock1, sock2]) = build_delay_mesh_per_peer(addrs, delays);

    let build = |local: usize, sock: DelaySocket| {
        let mut b = SessionBuilder::<StubConfig>::new()
            .with_num_players(3)
            .unwrap()
            .with_desync_detection_mode(DesyncDetection::On { interval });
        for (h, addr) in addrs.iter().enumerate() {
            b = if h == local {
                b.add_player(PlayerType::Local, PlayerHandle::new(h))
                    .unwrap()
            } else {
                b.add_player(PlayerType::Remote(*addr), PlayerHandle::new(h))
                    .unwrap()
            };
        }
        b.start_p2p_session(sock).unwrap()
    };

    let mut sessions = [build(0, sock0), build(1, sock1), build(2, sock2)];

    let mut synced = false;
    for _ in 0..2000 {
        pump(&mut sessions, &tick);
        if sessions
            .iter()
            .all(|s| s.current_state() == SessionState::Running)
        {
            synced = true;
            break;
        }
    }
    assert!(synced, "all three sessions should synchronize");

    let mut stubs = [GameStub::new(), GameStub::new(), GameStub::new()];
    // Ground-truth confirmed inputs: inputs applied to reach each frame, keyed by
    // frame with last-write-wins (final value = confirmed re-simulation inputs).
    let mut applied_inputs: [BTreeMap<i32, Vec<u32>>; 3] =
        [BTreeMap::new(), BTreeMap::new(), BTreeMap::new()];
    let mut desync_events: Vec<(usize, i32, u128, u128)> = Vec::new();
    let mut rollbacks = [0usize; 3];

    let drain_events = |sessions: &mut [fortress_rollback::P2PSession<StubConfig>; 3],
                        desync_events: &mut Vec<(usize, i32, u128, u128)>| {
        for (i, s) in sessions.iter_mut().enumerate() {
            for ev in s.events() {
                if let FortressEvent::DesyncDetected {
                    frame,
                    local_checksum,
                    remote_checksum,
                    ..
                } = ev
                {
                    desync_events.push((i, frame.as_i32(), local_checksum, remote_checksum));
                }
            }
        }
    };

    for f in 0..frames {
        pump(&mut sessions, &tick);

        // Per-frame-distinct, per-peer-distinct inputs. Variation forces
        // RepeatLastConfirmed to mispredict every delayed peer, so every observer
        // rolls back across the checksum interval — and because each peer is
        // delayed by a DIFFERENT amount, the observers reach the interval frame at
        // DIFFERENT prediction depths and harvest DIFFERENT speculative checksums.
        let inputs = [
            StubInput { inp: f * 7 + 1 },
            StubInput { inp: f * 13 + 2 },
            StubInput { inp: f * 31 + 5 },
        ];

        for (i, s) in sessions.iter_mut().enumerate() {
            let _ = s.add_local_input(PlayerHandle::new(i), inputs[i]);
        }

        for (i, s) in sessions.iter_mut().enumerate() {
            match s.advance_frame() {
                Ok(reqs) => {
                    rollbacks[i] += reqs
                        .iter()
                        .filter(|r| {
                            matches!(r, fortress_rollback::FortressRequest::LoadGameState { .. })
                        })
                        .count();
                    // Record the confirmed-input oracle by mirroring the GameStub
                    // frame counter through the request stream.
                    let mut cursor = stubs[i].current_frame();
                    for r in reqs.iter() {
                        match r {
                            fortress_rollback::FortressRequest::LoadGameState { cell, .. } => {
                                cursor = cell.frame().as_i32();
                            },
                            fortress_rollback::FortressRequest::AdvanceFrame { inputs } => {
                                cursor += 1;
                                let vals: Vec<u32> =
                                    inputs.iter().map(|(inp, _)| inp.inp).collect();
                                applied_inputs[i].insert(cursor, vals);
                            },
                            fortress_rollback::FortressRequest::SaveGameState { .. } => {},
                        }
                    }
                    stubs[i].handle_requests(reqs);
                },
                Err(fortress_rollback::FortressError::PredictionThreshold) => {},
                Err(e) => panic!("peer {i} advance_frame failed: {e:?}"),
            }
        }

        drain_events(&mut sessions, &mut desync_events);
    }

    for _ in 0..60 {
        pump(&mut sessions, &tick);
        drain_events(&mut sessions, &mut desync_events);
    }

    // Compare confirmed inputs across peers at every commonly-recorded frame.
    let min_confirmed = sessions
        .iter()
        .map(|s| s.confirmed_frame().as_i32())
        .min()
        .unwrap_or(0);

    let mut confirmed_input_mismatch_frames = Vec::new();
    let mut confirmed_input_agree_frames = 0usize;
    for frame in 1..=min_confirmed {
        if let (Some(a), Some(b), Some(c)) = (
            applied_inputs[0].get(&frame),
            applied_inputs[1].get(&frame),
            applied_inputs[2].get(&frame),
        ) {
            if a == b && a == c {
                confirmed_input_agree_frames += 1;
            } else {
                confirmed_input_mismatch_frames.push(frame);
            }
        }
    }

    AsymmetricOutcome {
        desync_events,
        confirmed_input_mismatch_frames,
        confirmed_input_agree_frames,
        rollbacks,
    }
}

/// CHARACTERIZATION TEST for the NORMAL-PATH (non-disconnect) checksum-harvest
/// invariant — the normal-path analog of finding F11.
///
/// ## Background: the lead and why this test exists
///
/// A false-positive `DesyncDetected` was observed on the NORMAL prediction-rollback
/// path under real UDP (0% loss, no disconnect): peers whose final confirmed state
/// was byte-identical nonetheless stored differing per-frame harvested checksums in
/// `local_checksum_history`, raising a spurious desync. S27 arbitrated this NOTABUG;
/// S29 OVERTURNED that to REAL (see the module header — the un-re-saved boundary
/// cell `cell[first_incorrect]`). This ASYMMETRIC-delay test still cannot reproduce
/// the real bug in-process (FIFO delivery cannot strand the boundary cell), but it
/// breaks the harvest SYMMETRY S27's probe could not, so it is valuable boundary
/// characterization and a regression lock — not the bug's deterministic repro.
///
/// This test breaks that symmetry with ASYMMETRIC per-peer delays (`[0, 1, 2]`):
/// each peer sits at a DIFFERENT prediction depth, so the three peers genuinely
/// harvest DIFFERENT speculative checksums for the same interval frame
/// (`check_checksum_send_interval`, `src/sessions/p2p_session.rs`). Per-frame-distinct
/// inputs guarantee constant misprediction → deep rollback across the small
/// checksum interval. The faithful [`GameStub`] checksum is a pure hash of the full
/// confirmed `StateStub`.
///
/// ## What it asserts (the invariant, GROUND-TRUTH oracle)
///
/// Using a confirmed-INPUT oracle (not merely recorded states), the test asserts:
///   1. NO `DesyncDetected` fires on any frame whose confirmed inputs AGREED across
///      all peers (a genuine false positive). This is
///      [`AsymmetricOutcome::false_positive_desync_frames`].
///   2. Deep rollback actually occurred (premise: the harvest window was exercised).
///   3. Many frames had agreeing confirmed inputs (premise: the run was non-vacuous).
///
/// ## Result (in-process CHARACTERIZATION — does NOT reproduce the real bug)
///
/// Despite genuinely-asymmetric speculative harvesting, NO false positive was
/// observed here, nor across an exhaustive scratch sweep (196 structured + 600
/// randomized delay/pump/interval configs, ~14k total desync events): EVERY
/// in-process `DesyncDetected` coincided with a genuine confirmed-input divergence
/// (caused by an aggressive pump cadence confirming a frame before the slow peer's
/// real input arrived), never with agreeing confirmed inputs.
///
/// CAUTION: this in-process green result alone does NOT clear the library. The
/// FIFO `DelaySocket` cannot drive ONE peer to `first_incorrect ==
/// interval_frame` while another's lands below it, so it could never stage the
/// cross-link reordering the REAL bug needed (S29 verdict: REAL — see the
/// module header; FIXED in S30). S27 misread this same green result as "the
/// rollback re-saves the cell before the harvest reads it"; in fact the
/// `i > 0` save guard in `adjust_gamestate` (`src/sessions/p2p_session.rs:3555`)
/// never re-saves the loaded boundary cell `cell[first_incorrect]` — which was
/// exactly the cell stranded stale on the pre-S30 real-UDP path. This test
/// characterizes the boundary; the bug's deterministic repro is the reorder
/// choreography below.
#[test]
fn asymmetric_delay_deep_rollback_no_false_desync() {
    let outcome = run_asymmetric_scenario([0, 1, 2], 2, 120, 31001);

    // PREMISE: deep rollback actually happened (the harvest window was exercised).
    assert!(
        outcome.rollbacks.iter().any(|&r| r > 0),
        "deep-prediction-rollback premise unmet: rollbacks={:?}",
        outcome.rollbacks
    );

    // PREMISE: the run was non-vacuous — many frames had agreeing confirmed inputs.
    assert!(
        outcome.confirmed_input_agree_frames >= 10,
        "expected many frames with agreeing confirmed inputs, got {} ({outcome:?})",
        outcome.confirmed_input_agree_frames
    );

    // INVARIANT: no DesyncDetected fired on a frame whose confirmed inputs agreed.
    let false_positives = outcome.false_positive_desync_frames();
    assert!(
        false_positives.is_empty(),
        "FALSE-POSITIVE DesyncDetected on frame(s) whose confirmed inputs AGREED \
         across peers (faithful checksum should have matched): {false_positives:?} \
         ({outcome:?})"
    );
}

/// Determinism guard for the asymmetric scenario: the in-process harness uses
/// fixed addresses and a shared logical tick (no wall-clock), so repeated runs
/// MUST produce a byte-identical [`AsymmetricOutcome`]. Runs the scenario 10x and
/// asserts every run equals the first.
#[test]
fn asymmetric_delay_deep_rollback_is_deterministic() {
    let first = run_asymmetric_scenario([0, 1, 2], 2, 120, 32001);
    for run in 1..10 {
        let next = run_asymmetric_scenario([0, 1, 2], 2, 120, 32001);
        assert_eq!(
            first, next,
            "asymmetric scenario was NON-deterministic on run {run}: \
             first={first:?} next={next:?}"
        );
    }
}

// ===========================================================================
// FAITHFUL-CHECKSUM no-false-desync guard (NORMAL prediction-rollback path)
// ===========================================================================
//
// The scenarios above use the `StateStub` (`+2/-1` parity) checksum, which
// collides far too often to distinguish a stale speculative cell from the
// confirmed one (two different confirmed-input sequences routinely hash equal).
// The tests below instead use an INJECTIVE accumulator whose checksum is a
// FAITHFUL, near-collision-free hash of the saved state, so any checksum
// mismatch is a genuine saved-state divergence — exactly the condition the
// Session-26/S27 "normal-path stale-speculative-cell" lead was about.
//
// `InjConfig` below is that stub: its state folds every player's input into a
// running accumulator with a large odd multiplier (FNV-style), so the state
// after a frame is a (practically) injective function of the full confirmed
// input history, and its checksum is a faithful hash of `(frame, accumulator)`.
//
// SCOPE (what these in-process tests DO and do NOT establish): with a FAITHFUL
// checksum the IN-PROCESS harness raises NO false-positive `DesyncDetected` on
// the normal prediction-rollback path across an exhaustive scratch sweep (1500
// structured delay / interval / pump-cadence configs): EVERY in-process
// `DesyncDetected` coincided with a genuine confirmed-INPUT divergence (a harness
// pump-cadence artifact), never with agreeing confirmed inputs. That is genuine,
// non-vacuous coverage — it CHARACTERIZES the boundary and LOCKS the exact-frame
// harvest invariant against regression.
//
// BUT these tests do NOT (and structurally CANNOT) reproduce the real bug. The
// FIFO `DelaySocket` cannot deterministically drive ONE peer to
// `first_incorrect == interval_frame` while another peer's `first_incorrect`
// lands BELOW it; the in-order delivery re-saves cells past the boundary and
// confirmation never strands a stale boundary cell without a later refreshing
// rollback. So the asymmetric-prediction false positive never arises here.
//
// The pre-S30 real-UDP `network_test_peer` reproducer observed `DesyncDetected
// total=4` at 0% loss on ~50% of 3-peer runs. S27 attributed this to the
// binary's SPECULATIVE display accumulator `state.value` (a non-faithful
// checksum) and concluded a faithful game would be unaffected. THE
// ORCHESTRATOR'S PER-FRAME-DIGEST EXPERIMENT REFUTED THAT (see the module
// header): a FAITHFUL per-frame digest — and, separately, a hash-of-full-state
// checksum in GameStub's own style — STILL fired `total=4` at frame 60, with
// the odd peer storing a value that could only come from PREDICTED inputs. The
// false positive was REAL (S29): the staleness lived in LIBRARY-saved cells,
// which no game-side checksum function can rescue. These tests therefore remain
// as boundary CHARACTERIZATION and a regression lock for the exact-frame
// harvest invariant; the deterministic repro is the reorder choreography below,
// and the S30 root-cause fix lives in `src/input_queue/mod.rs` (see
// `progress/session-29-*` and `N-PLAYER-DESYNC-AUDIT.md` for history).

mod inj {
    use serde::{Deserialize, Serialize};

    use fortress_rollback::{Config, FortressRequest, Frame, GameStateCell, InputVec, RequestVec};

    use crate::common::stubs::StubInput;

    /// Injective accumulator state. The accumulator is updated as
    /// `acc = acc * MUL + weighted_input_sum`, which is (for the small frame
    /// counts in this test) a collision-free function of the confirmed input
    /// history — so a speculative save and the confirmed save at the same frame
    /// produce DIFFERENT accumulators (and thus different faithful checksums)
    /// whenever the inputs that fed them differ.
    #[derive(Default, Copy, Clone, Hash, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct InjState {
        pub frame: i32,
        pub acc: u64,
    }

    impl InjState {
        fn advance(&mut self, inputs: &InputVec<StubInput>) {
            // Large odd FNV-style multiplier keeps the accumulator injective over
            // the confirmed input history for this test's frame counts.
            const MUL: u64 = 0x0100_0000_01b3;
            // Weight each player's input by its (1-based) index so player order is
            // significant — distinct per-peer inputs cannot cancel out.
            let mut weighted: u64 = 0;
            for (idx, (input, _)) in inputs.iter().enumerate() {
                weighted =
                    weighted.wrapping_add((u64::from(input.inp)).wrapping_mul((idx as u64) + 1));
            }
            self.acc = self.acc.wrapping_mul(MUL).wrapping_add(weighted);
            self.frame += 1;
        }

        /// Faithful checksum: a pure hash of the full saved state. Equal states
        /// (and only equal states, modulo a 1-in-2^64 hash collision) compare
        /// equal, so any checksum mismatch is a genuine saved-state divergence.
        fn checksum(&self) -> u128 {
            // 128-bit mix of (frame, acc) so the checksum is faithful to the
            // whole state, not just the accumulator.
            let f = (self.frame as u32) as u128;
            let a = self.acc as u128;
            (a << 32) ^ f ^ (a.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        }
    }

    #[derive(Debug)]
    pub struct InjConfig;

    impl Config for InjConfig {
        type Input = StubInput;
        type State = InjState;
        type Address = std::net::SocketAddr;
    }

    /// Game stub over [`InjConfig`] that records the most-recent state at each
    /// (post-advance) frame, so confirmed states can be compared across peers.
    pub struct InjStub {
        pub gs: InjState,
    }

    impl InjStub {
        #[must_use]
        pub fn new() -> Self {
            Self {
                gs: InjState::default(),
            }
        }

        pub fn handle_requests_recording(
            &mut self,
            requests: RequestVec<InjConfig>,
            states: &mut std::collections::BTreeMap<i32, InjState>,
        ) {
            for request in requests {
                match request {
                    FortressRequest::LoadGameState { cell, .. } => {
                        self.gs = cell.load().unwrap();
                    },
                    FortressRequest::SaveGameState { cell, frame } => {
                        self.save(cell, frame);
                    },
                    FortressRequest::AdvanceFrame { inputs } => {
                        self.gs.advance(&inputs);
                        states.insert(self.gs.frame, self.gs);
                    },
                }
            }
        }

        fn save(&self, cell: GameStateCell<InjState>, frame: Frame) {
            assert_eq!(self.gs.frame, frame.as_i32());
            cell.save(frame, Some(self.gs), Some(self.gs.checksum()));
        }
    }
}

/// Structural outcome of one injective-accumulator asymmetric run, with a
/// confirmed-INPUT ground-truth oracle (so every `DesyncDetected` can be
/// classified TRUE vs FALSE positive) AND the recorded confirmed states (so we
/// can assert byte-identity directly).
#[derive(Debug, PartialEq, Eq)]
struct InjOutcome {
    desync_events: Vec<(usize, i32, u128, u128)>,
    confirmed_input_mismatch_frames: Vec<i32>,
    confirmed_input_agree_frames: usize,
    /// Number of (frame) positions at which all three peers recorded a state and
    /// those states were byte-identical.
    states_compared: usize,
    /// Confirmed frames (agreeing confirmed inputs) whose recorded confirmed
    /// STATES nonetheless differed across peers — a genuine state divergence.
    state_mismatch_frames: Vec<i32>,
    rollbacks: [usize; 3],
}

impl InjOutcome {
    /// `DesyncDetected` events fired on frames whose confirmed inputs AGREED
    /// across peers — genuine false positives (the faithful injective checksum
    /// should have matched).
    fn false_positive_desync_frames(&self) -> Vec<i32> {
        self.desync_events
            .iter()
            .map(|&(_, frame, _, _)| frame)
            .filter(|f| !self.confirmed_input_mismatch_frames.contains(f))
            .collect()
    }
}

/// Drives the injective-accumulator asymmetric scenario end-to-end. Pure
/// function of its arguments (fixed addresses, shared logical tick, no
/// wall-clock), so repeated calls with identical arguments produce identical
/// outcomes.
#[allow(clippy::too_many_lines)]
fn run_inj_scenario(delays: [u64; 3], interval: u32, frames: u32, base_port: u16) -> InjOutcome {
    use inj::{InjConfig, InjState, InjStub};

    let addrs: [SocketAddr; 3] = [
        ([127, 0, 0, 1], base_port).into(),
        ([127, 0, 0, 1], base_port + 1).into(),
        ([127, 0, 0, 1], base_port + 2).into(),
    ];

    let (tick, [sock0, sock1, sock2]) = build_delay_mesh_per_peer(addrs, delays);

    let build = |local: usize, sock: DelaySocket| {
        let mut b = SessionBuilder::<InjConfig>::new()
            .with_num_players(3)
            .unwrap()
            .with_desync_detection_mode(DesyncDetection::On { interval });
        for (h, addr) in addrs.iter().enumerate() {
            b = if h == local {
                b.add_player(PlayerType::Local, PlayerHandle::new(h))
                    .unwrap()
            } else {
                b.add_player(PlayerType::Remote(*addr), PlayerHandle::new(h))
                    .unwrap()
            };
        }
        b.start_p2p_session(sock).unwrap()
    };

    let mut sessions = [build(0, sock0), build(1, sock1), build(2, sock2)];

    let pump_inj = |sessions: &mut [fortress_rollback::P2PSession<InjConfig>; 3],
                    tick: &Arc<Mutex<u64>>| {
        for _ in 0..4 {
            for s in sessions.iter_mut() {
                s.poll_remote_clients();
            }
            *tick.lock().unwrap() += 1;
        }
    };

    let mut synced = false;
    for _ in 0..2000 {
        pump_inj(&mut sessions, &tick);
        if sessions
            .iter()
            .all(|s| s.current_state() == SessionState::Running)
        {
            synced = true;
            break;
        }
    }
    assert!(synced, "all three sessions should synchronize");

    let mut stubs = [InjStub::new(), InjStub::new(), InjStub::new()];
    let mut states: [BTreeMap<i32, InjState>; 3] =
        [BTreeMap::new(), BTreeMap::new(), BTreeMap::new()];
    let mut applied_inputs: [BTreeMap<i32, Vec<u32>>; 3] =
        [BTreeMap::new(), BTreeMap::new(), BTreeMap::new()];
    let mut desync_events: Vec<(usize, i32, u128, u128)> = Vec::new();
    let mut rollbacks = [0usize; 3];

    let drain_events = |sessions: &mut [fortress_rollback::P2PSession<InjConfig>; 3],
                        desync_events: &mut Vec<(usize, i32, u128, u128)>| {
        for (i, s) in sessions.iter_mut().enumerate() {
            for ev in s.events() {
                if let FortressEvent::DesyncDetected {
                    frame,
                    local_checksum,
                    remote_checksum,
                    ..
                } = ev
                {
                    desync_events.push((i, frame.as_i32(), local_checksum, remote_checksum));
                }
            }
        }
    };

    for f in 0..frames {
        pump_inj(&mut sessions, &tick);

        let inputs = [
            StubInput { inp: f * 7 + 1 },
            StubInput { inp: f * 13 + 2 },
            StubInput { inp: f * 31 + 5 },
        ];

        for (i, s) in sessions.iter_mut().enumerate() {
            let _ = s.add_local_input(PlayerHandle::new(i), inputs[i]);
        }

        for (i, s) in sessions.iter_mut().enumerate() {
            match s.advance_frame() {
                Ok(reqs) => {
                    rollbacks[i] += reqs
                        .iter()
                        .filter(|r| {
                            matches!(r, fortress_rollback::FortressRequest::LoadGameState { .. })
                        })
                        .count();
                    // Mirror the GameStub frame counter through the request stream
                    // to build the confirmed-input oracle (last write wins).
                    let mut cursor = stubs[i].gs.frame;
                    for r in reqs.iter() {
                        match r {
                            fortress_rollback::FortressRequest::LoadGameState { cell, .. } => {
                                cursor = cell.frame().as_i32();
                            },
                            fortress_rollback::FortressRequest::AdvanceFrame { inputs } => {
                                cursor += 1;
                                let vals: Vec<u32> =
                                    inputs.iter().map(|(inp, _)| inp.inp).collect();
                                applied_inputs[i].insert(cursor, vals);
                            },
                            fortress_rollback::FortressRequest::SaveGameState { .. } => {},
                        }
                    }
                    stubs[i].handle_requests_recording(reqs, &mut states[i]);
                },
                Err(fortress_rollback::FortressError::PredictionThreshold) => {},
                Err(e) => panic!("peer {i} advance_frame failed: {e:?}"),
            }
        }

        drain_events(&mut sessions, &mut desync_events);
    }

    for _ in 0..80 {
        pump_inj(&mut sessions, &tick);
        drain_events(&mut sessions, &mut desync_events);
    }

    let min_confirmed = sessions
        .iter()
        .map(|s| s.confirmed_frame().as_i32())
        .min()
        .unwrap_or(0);

    let mut confirmed_input_mismatch_frames = Vec::new();
    let mut confirmed_input_agree_frames = 0usize;
    for frame in 1..=min_confirmed {
        if let (Some(a), Some(b), Some(c)) = (
            applied_inputs[0].get(&frame),
            applied_inputs[1].get(&frame),
            applied_inputs[2].get(&frame),
        ) {
            if a == b && a == c {
                confirmed_input_agree_frames += 1;
            } else {
                confirmed_input_mismatch_frames.push(frame);
            }
        }
    }

    let mut states_compared = 0usize;
    let mut state_mismatch_frames = Vec::new();
    for frame in 1..=min_confirmed {
        if let (Some(a), Some(b), Some(c)) = (
            states[0].get(&frame),
            states[1].get(&frame),
            states[2].get(&frame),
        ) {
            // Only consider frames whose confirmed inputs agreed — a
            // confirmed-input mismatch is a harness pump-cadence artifact, not a
            // library defect (documented in the asymmetric scenario above). On
            // those frames the confirmed states MUST be byte-identical; record any
            // divergence so the caller can assert (the regression test) or inspect
            // (the scratch sweep) without aborting the run mid-way.
            if !confirmed_input_mismatch_frames.contains(&frame) {
                if a == b && a == c {
                    states_compared += 1;
                } else {
                    state_mismatch_frames.push(frame);
                }
            }
        }
    }

    InjOutcome {
        desync_events,
        confirmed_input_mismatch_frames,
        confirmed_input_agree_frames,
        states_compared,
        state_mismatch_frames,
        rollbacks,
    }
}

/// NON-VACUOUS faithful-checksum guard for the NORMAL prediction-rollback path
/// (the normal-path analog of finding F11's concern).
///
/// Uses the INJECTIVE accumulator stub so a stale speculative `cell[F]` would
/// deterministically differ from the confirmed cell (a faithful, collision-free
/// checksum). Asymmetric per-peer delays `[0, 2, 4]` drive the three peers to
/// DIFFERENT prediction depths so their correcting rollbacks land on different
/// `first_incorrect` values relative to the checksum-interval frame, and they
/// genuinely harvest DIFFERENT speculative checksums for the same interval frame.
/// The small interval keeps an interval frame inside the live rollback window.
///
/// ORACLE (all must hold):
///   1. PREMISE: deep rollback actually happened (the harvest window was exercised).
///   2. PREMISE: many frames had agreeing confirmed inputs AND byte-identical
///      confirmed states (non-vacuous; a faithful checksum MUST match on these).
///   3. INVARIANT: NO `DesyncDetected` fired on a frame whose confirmed inputs
///      agreed (which would be a genuine false positive on byte-identical state).
///
/// SCOPE: in THIS in-process harness (FIFO delivery) a faithful checksum raises
/// no false positive, so the test locks the exact-frame harvest invariant against
/// regression. It alone does NOT clear the library: the FIFO socket cannot stage
/// the cross-link reordering the REAL bug needed, so it could not reproduce it
/// (S29 — see the module header; FIXED in S30). The orchestrator's
/// per-frame-digest and full-state-hash experiments on `network_test_peer`
/// showed even a FAITHFUL checksum still fired the pre-S30 real-UDP false
/// positive, because the staleness was in library-saved cells, not the game's
/// checksum function. Treat this as boundary characterization + regression
/// lock; the bug's deterministic repro is the reorder choreography below.
#[test]
fn injective_asymmetric_rollback_no_false_desync_with_faithful_checksum() {
    let outcome = run_inj_scenario([0, 2, 4], 4, 120, 33001);

    assert!(
        outcome.rollbacks.iter().any(|&r| r > 0),
        "deep-prediction-rollback premise unmet: rollbacks={:?}",
        outcome.rollbacks
    );

    assert!(
        outcome.confirmed_input_agree_frames >= 10,
        "expected many frames with agreeing confirmed inputs, got {} ({outcome:?})",
        outcome.confirmed_input_agree_frames
    );

    assert!(
        outcome.states_compared >= 10,
        "expected to compare many byte-identical confirmed states, got {} ({outcome:?})",
        outcome.states_compared
    );

    // The confirmed states are byte-identical across peers (proves any
    // DesyncDetected would be a FALSE positive — the faithful injective checksum
    // should have matched).
    assert!(
        outcome.state_mismatch_frames.is_empty(),
        "confirmed STATE divergence on frames with agreeing confirmed inputs: {:?} ({outcome:?})",
        outcome.state_mismatch_frames
    );

    let false_positives = outcome.false_positive_desync_frames();
    assert!(
        false_positives.is_empty(),
        "FALSE-POSITIVE DesyncDetected on frame(s) whose confirmed inputs AGREED \
         across peers with a FAITHFUL injective checksum (the saved speculative \
         cell was stale): {false_positives:?} ({outcome:?})"
    );
}

/// Determinism guard: the injective scenario is a pure function of its arguments,
/// so 10 repeated runs MUST produce a byte-identical [`InjOutcome`].
#[test]
fn injective_asymmetric_rollback_is_deterministic() {
    let first = run_inj_scenario([0, 2, 4], 4, 120, 34001);
    for run in 1..10 {
        let next = run_inj_scenario([0, 2, 4], 4, 120, 34001);
        assert_eq!(
            first, next,
            "injective scenario was NON-deterministic on run {run}: \
             first={first:?} next={next:?}"
        );
    }
}

// ===========================================================================
// F17 DETERMINISTIC REPRODUCTION (S30) — hold-then-release reordering test
// (RED on the pre-S30 code; GREEN since the S30 InputQueue fix)
// ===========================================================================
//
// This section is the deterministic in-process reproduction S29 said was the
// prerequisite for the F17 fix. It uses the `ReorderSocket` hold-then-release
// mesh (`tests/common/reorder_socket.rs`) to stage the cross-link arrival
// inversion a FIFO delay socket structurally cannot express. The mechanism
// steps below describe the PRE-S30 defect this choreography pinned down; the
// S30 fix (prediction episodes always enter at the queue's first missing
// frame, `src/input_queue/mod.rs::input`) removes step 2's faulty re-entry,
// which makes steps 3-5 unreachable.
//
// ## Resolved save semantics (read from code, settling the S29/S27 ambiguity)
//
// In `advance_frame` the SaveGameState for the current frame F is pushed
// (`src/sessions/p2p_session.rs:696`, frame-0 special case `:663`) BEFORE the
// AdvanceFrame{inputs at F} request (`:800-803`); inside the rollback re-sim
// loop the save at `:3555-3557` likewise precedes that iteration's
// AdvanceFrame push (`:3561-3562`). So `cell[F]` holds the state BEFORE frame
// F's inputs are applied — a pure function of the inputs at frames 0..=F-1
// (confirmed or PREDICTED, as known at save time) and NEVER of frame F's own
// input. The S29 module-header parenthetical ("a saved cell holds the state
// AFTER applying that frame's input") is wrong on that detail; the staleness
// in `cell[F]` therefore lives in predicted inputs at frames STRICTLY BELOW F.
//
// ## The exact stale-stamp path this choreography pinned down (pre-S30 code)
//
// Topology: A (player 0, the victim), B (player 1), C (player 2); per-frame-
// distinct inputs; `DesyncDetection::On { interval: 2 }`; max_prediction 8.
//
// 1. Hold the directed link B->A from frame J=10. A's input queue for B
//    enters prediction at its first missing frame 10 (entry
//    `src/input_queue/mod.rs::input`, value = B@9 under
//    RepeatLastConfirmed). A keeps advancing; its forward saves of
//    cells 11..=17 embed the PREDICTED B@10.. values. A's
//    `last_confirmed_frame` pins at 9 (B's slot), so the checksum harvest
//    (`src/sessions/p2p_session.rs:4136`, gate `:4145-4146`) parks at
//    `frame_to_send = 10`.
// 2. Hold C->A from frame F=14 (so A's C-queue predicts from exactly 14),
//    then RELEASE C->A. C's real inputs 14.. arrive and mismatch ->
//    `first_incorrect = 14` -> `adjust_gamestate` loads `cell[14]`
//    (`:3510`), resets prediction (`:3523`), and the re-sim's FIRST request
//    for B is at frame 14. THE DEFECT: B's queue re-entered prediction with
//    `prediction.frame = requested_frame = 14` — ABOVE its first missing
//    frame 10 — while the prediction VALUE stayed B@9. (The S30 fix makes
//    the re-entry land at the first missing frame 10 instead, so every
//    arrival below is compared and steps 3-5 cannot occur.)
// 3. RELEASE B->A. B's real inputs 10..13 were ADDED to the queue but their
//    prediction-miss comparison was SKIPPED: `add_input_by_frame` bailed with
//    a reported violation when `frame_number != prediction.frame`, so
//    `first_incorrect` was NEVER set for 10..13. B@14 was compared (against
//    B@9) -> `first_incorrect = 14` AGAIN.
// 4. The correcting rollback loaded the STALE `cell[14]` (its B-slots for
//    frames 10..13 still held the predicted B@9 value), the `if i > 0` guard
//    (`src/sessions/p2p_session.rs:3555`) never re-saves the loaded boundary
//    cell, and every cell re-saved by the re-sim inherited the stale base.
//    No rollback ever targeted <= 13 again (the comparisons were swallowed),
//    so cells 11..=14 kept their predicted-B content forever while
//    `last_confirmed_frame` jumped past them (set `:711-712` from
//    `confirmed_frame()` -- connection-status watermarks, NOT cell
//    freshness).
// 5. The harvest then marched: frame 10's cell is clean (it embeds only
//    inputs <= 9), but the harvests of frames 12..=18 read, STORED
//    (`local_checksum_history`) and GOSSIPED (`send_checksum_report`,
//    `:4184-4191`) checksums stamped from prediction-influenced states.
//    For flagged frames <= 18 — the surgical F17 fingerprint, 4 events per
//    interval frame — B's and C's cells for the same frames are clean and
//    agree with each other while A's differs (A odd one out), exactly the
//    S29 decisive-evidence fingerprint (identical confirmed-input digests,
//    differing stored interval-frame checksum), and
//    `compare_local_checksums_against_peers` (`:4093-4134`) raised
//    `DesyncDetected` on every peer — while the CONFIRMED INPUT STREAMS
//    (the input-queue contents, read back via the public
//    `confirmed_inputs_for_frame`) are byte-identical on all three peers at
//    every flagged frame. Flagged frames >= 20 were a DIFFERENT, heavier
//    regime: A's swallowed window stalled it at PredictionThreshold long
//    enough that B and C also entered (legitimate) deep prediction against
//    the silent victim, and the captured run cascaded into a 6-events-per-
//    frame THREE-WAY divergence (three distinct stored checksums) for frames
//    20..=40 — 66 of the 82 observed events. B's/C's cells are NOT clean
//    there, so the "B==C, A odd" narrative applies only to <= 18; the
//    cascade is additional severity evidence that the swallowed window
//    poisons the whole mesh's harvest stream, not just the victim's.
//
// ## Honesty note (what the pre-S30 code additionally did, beyond S29's text)
//
// Because the swallowed window 10..13 was never re-simulated with B's real
// inputs, the victim's APPLIED trajectory (the states its game actually
// simulated through) also stayed divergent from B/C's from frame 11 onward —
// the staleness was NOT single-frame/self-limiting as the audit's F17
// summary suggested (S29's "staleness spans multiple frames" note was
// already pointing at this), and with `DesyncDetection::Off` it was a fully
// silent gameplay desync at 0% loss. The S30 fix lands at that root: a
// prediction episode always begins at the queue's first missing frame
// (`src/input_queue/mod.rs::input`), so every arrival is compared against
// the value the episode actually applied, the swallowed-window state is
// unconstructible, a mismatch at 10 triggers the rollback that re-simulates
// the window with B's real inputs, and no stale stamp is ever harvested —
// no `DesyncDetected` fires. (The S29 fix spec — a per-cell
// "all-inputs-confirmed-at-save" bit plus a newest-confirmed-clean-cell
// boundary load — was symptom-layer and is superseded by the root-cause
// fix.) This test asserts the fixed behavior, including the stronger
// applied-trajectory-convergence oracle below, so it was RED before the fix
// and is GREEN with it.

/// Structural outcome of one hold-then-release reordering run.
#[derive(Debug, PartialEq, Eq)]
struct ReorderOutcome {
    /// `(observing_peer, frame, local_checksum, remote_checksum)` for every
    /// `DesyncDetected` event seen on any peer.
    desync_events: Vec<(usize, i32, u128, u128)>,
    /// Count of `Disconnected` events across all peers (premise: must stay 0 —
    /// the choreography is pure reordering, 0% loss, no disconnect).
    disconnect_events: usize,
    /// Rollback count per peer (`LoadGameState` requests).
    rollbacks: [usize; 3],
    /// The minimum (over peers) confirmed frame at the end of the run.
    min_confirmed: i32,
    /// Per-peer confirmed input streams read back through the PUBLIC
    /// `confirmed_inputs_for_frame` API at the end of the run, keyed by frame
    /// (values are each player's `StubInput::inp`). This is the ground-truth
    /// oracle proving any `DesyncDetected` is a false positive w.r.t. the
    /// confirmed inputs: the input-queue contents are what the confirmed
    /// state is DEFINED by.
    confirmed_inputs: [BTreeMap<i32, Vec<u32>>; 3],
    /// Number of frames `<= min_confirmed` at which all three peers recorded
    /// an applied state and those states were byte-identical (premise: must
    /// be substantial for the mismatch oracle below to be non-vacuous).
    applied_states_compared: usize,
    /// Frames `<= min_confirmed` at which the peers' final APPLIED states —
    /// the last state each game actually simulated through that frame,
    /// last-write-wins across re-simulations — DIFFER across peers. This is
    /// the STRONGER invariant the F17 fix restores: not only must no false
    /// `DesyncDetected` fire, the applied trajectories themselves must
    /// converge (the pre-S30 swallowed window left the victim's applied
    /// trajectory permanently divergent with no rollback and no event — a
    /// silent gameplay desync when detection is `Off`).
    applied_state_mismatch_frames: Vec<i32>,
}

/// Drives the F17 hold-then-release choreography end-to-end (see the section
/// header above for the frame-by-frame mechanism). Pure function of its
/// argument: fixed addresses, single-threaded, shared-mutex delivery, no
/// wall-clock dependence in delivery or release timing — so repeated calls
/// produce identical outcomes ([`reordered_arrival_choreography_is_deterministic`]).
#[allow(clippy::too_many_lines)]
fn run_reorder_choreography(base_port: u16) -> ReorderOutcome {
    use inj::{InjConfig, InjState, InjStub};

    // Checksum-exchange cadence. With interval 2 the harvest parks at
    // frame_to_send = 10 while B->A is held (last_confirmed pinned at 9) and
    // then marches 10, 12, 14, ... after release — frames 12/14/16 land
    // squarely in the stale window.
    const INTERVAL: u32 = 2;
    const MAX_PREDICTION: usize = 8;
    /// B->A held from the iteration where every session sits at frame 10, so
    /// A's first missing B-input (and B-prediction entry) is exactly 10.
    const HOLD_B_TO_A_AT: u32 = 10;
    /// C->A held from frame 14: A's C-prediction entry — and therefore the
    /// boundary-rollback target `first_incorrect` — is exactly 14.
    const HOLD_C_TO_A_AT: u32 = 14;
    /// Release C->A while B->A is still held: the resulting rollback at 14
    /// re-enters B's prediction at frame 14 (above B's missing window).
    const RELEASE_C_TO_A_AT: u32 = 18;
    /// Release B->A afterwards: frames 10..13 are added-but-never-compared,
    /// frame 14 mismatches, and the boundary rollback re-simulates from the
    /// stale `cell[14]`.
    const RELEASE_B_TO_A_AT: u32 = 21;
    /// Enough post-release iterations for the harvest to march across the
    /// stale window and for every peer to compare the gossiped reports.
    const ITERATIONS: u32 = 48;

    let addrs: [SocketAddr; 3] = [
        ([127, 0, 0, 1], base_port).into(),
        ([127, 0, 0, 1], base_port + 1).into(),
        ([127, 0, 0, 1], base_port + 2).into(),
    ];

    let ([sock_a, sock_b, sock_c], links) = create_reorder_mesh_triple(addrs);

    let build = |local: usize, sock| {
        let mut b = SessionBuilder::<InjConfig>::new()
            .with_num_players(3)
            .unwrap()
            .with_max_prediction_window(MAX_PREDICTION)
            .with_desync_detection_mode(DesyncDetection::On { interval: INTERVAL })
            // The held links carry no traffic for a handful of iterations of
            // wall-clock time; push the (wall-clock-based) disconnect and
            // notify timers out of reach so a slow CI machine cannot turn the
            // pure-reordering choreography into a disconnect.
            .with_disconnect_timeout(std::time::Duration::from_secs(3600))
            .with_disconnect_notify_delay(std::time::Duration::from_secs(3600));
        for (h, addr) in addrs.iter().enumerate() {
            b = if h == local {
                b.add_player(PlayerType::Local, PlayerHandle::new(h))
                    .unwrap()
            } else {
                b.add_player(PlayerType::Remote(*addr), PlayerHandle::new(h))
                    .unwrap()
            };
        }
        b.start_p2p_session(sock).unwrap()
    };

    let mut sessions = [build(0, sock_a), build(1, sock_b), build(2, sock_c)];

    let pump = |sessions: &mut [fortress_rollback::P2PSession<InjConfig>; 3]| {
        for _ in 0..4 {
            for s in sessions.iter_mut() {
                s.poll_remote_clients();
            }
        }
    };

    let mut synced = false;
    for _ in 0..2000 {
        pump(&mut sessions);
        if sessions
            .iter()
            .all(|s| s.current_state() == SessionState::Running)
        {
            synced = true;
            break;
        }
    }
    assert!(synced, "all three sessions should synchronize");

    let mut stubs = [InjStub::new(), InjStub::new(), InjStub::new()];
    let mut states: [BTreeMap<i32, InjState>; 3] =
        [BTreeMap::new(), BTreeMap::new(), BTreeMap::new()];
    let mut desync_events: Vec<(usize, i32, u128, u128)> = Vec::new();
    let mut disconnect_events = 0usize;
    let mut rollbacks = [0usize; 3];

    let drain_events = |sessions: &mut [fortress_rollback::P2PSession<InjConfig>; 3],
                        desync_events: &mut Vec<(usize, i32, u128, u128)>,
                        disconnect_events: &mut usize| {
        for (i, s) in sessions.iter_mut().enumerate() {
            for ev in s.events() {
                match ev {
                    FortressEvent::DesyncDetected {
                        frame,
                        local_checksum,
                        remote_checksum,
                        ..
                    } => {
                        desync_events.push((i, frame.as_i32(), local_checksum, remote_checksum));
                    },
                    FortressEvent::Disconnected { .. } => *disconnect_events += 1,
                    _ => {},
                }
            }
        }
    };

    for k in 0..ITERATIONS {
        // Toggle the choreography's holds/releases at the START of the
        // iteration, before this iteration's inputs are produced and sent.
        if k == HOLD_B_TO_A_AT {
            links.hold(addrs[1], addrs[0]);
        }
        if k == HOLD_C_TO_A_AT {
            links.hold(addrs[2], addrs[0]);
        }
        if k == RELEASE_C_TO_A_AT {
            // PREMISE: the hold actually captured traffic — releasing an empty
            // link would silently turn the choreography into plain FIFO play.
            assert!(
                links.held_len(addrs[2], addrs[0]) > 0,
                "C->A hold captured no traffic before release at iteration {k}"
            );
            links.release(addrs[2], addrs[0]);
        }
        if k == RELEASE_B_TO_A_AT {
            assert!(
                links.held_len(addrs[1], addrs[0]) > 0,
                "B->A hold captured no traffic before release at iteration {k}"
            );
            links.release(addrs[1], addrs[0]);
        }

        pump(&mut sessions);

        // Per-frame-distinct, per-peer-distinct inputs so RepeatLastConfirmed
        // mispredicts every held frame (the same generators as the scenarios
        // above).
        let inputs = [
            StubInput { inp: k * 7 + 1 },
            StubInput { inp: k * 13 + 2 },
            StubInput { inp: k * 31 + 5 },
        ];

        // Advance order C, B, A: the victim A advances LAST so the
        // poll_remote_clients at the top of its own advance_frame already
        // sees both remotes' freshly-sent inputs for this frame. On open
        // links A therefore NEVER predicts, which keeps the only prediction
        // episodes at A exactly the two the holds choreograph (B from 10,
        // C from 14) and makes the rollback targets deterministic.
        for &i in &[2usize, 1, 0] {
            let _ = sessions[i].add_local_input(PlayerHandle::new(i), inputs[i]);
            match sessions[i].advance_frame() {
                Ok(reqs) => {
                    rollbacks[i] += reqs
                        .iter()
                        .filter(|r| {
                            matches!(r, fortress_rollback::FortressRequest::LoadGameState { .. })
                        })
                        .count();
                    stubs[i].handle_requests_recording(reqs, &mut states[i]);
                },
                Err(fortress_rollback::FortressError::PredictionThreshold) => {},
                Err(e) => panic!("peer {i} advance_frame failed: {e:?}"),
            }
        }

        drain_events(&mut sessions, &mut desync_events, &mut disconnect_events);
    }

    // Drain stragglers (no further advances: comparisons only run inside
    // advance_frame, so all comparable reports have already been compared in
    // the loop above; this only collects already-queued events).
    for _ in 0..40 {
        pump(&mut sessions);
        drain_events(&mut sessions, &mut desync_events, &mut disconnect_events);
    }

    let min_confirmed = sessions
        .iter()
        .map(|s| s.confirmed_frame().as_i32())
        .min()
        .unwrap_or(0);

    // Read back every peer's CONFIRMED input stream through the public API.
    // The input-queue ring (length 128) still holds every frame of this short
    // run, so this works for the whole confirmed range despite the queues'
    // tail trimming.
    let mut confirmed_inputs: [BTreeMap<i32, Vec<u32>>; 3] =
        [BTreeMap::new(), BTreeMap::new(), BTreeMap::new()];
    for (i, s) in sessions.iter().enumerate() {
        let peer_confirmed = s.confirmed_frame().as_i32();
        for frame in 1..=peer_confirmed {
            if let Ok(inp) = s.confirmed_inputs_for_frame(Frame::new(frame)) {
                confirmed_inputs[i].insert(frame, inp.iter().map(|x| x.inp).collect());
            }
        }
    }

    // Cross-peer applied-state oracle: at every commonly-recorded frame up to
    // the slowest peer's confirmed frame, the final (last-write-wins) applied
    // states must be byte-identical across peers. See the field docs on
    // [`ReorderOutcome`].
    let mut applied_states_compared = 0usize;
    let mut applied_state_mismatch_frames = Vec::new();
    for frame in 1..=min_confirmed {
        if let (Some(a), Some(b), Some(c)) = (
            states[0].get(&frame),
            states[1].get(&frame),
            states[2].get(&frame),
        ) {
            if a == b && a == c {
                applied_states_compared += 1;
            } else {
                applied_state_mismatch_frames.push(frame);
            }
        }
    }

    ReorderOutcome {
        desync_events,
        disconnect_events,
        rollbacks,
        min_confirmed,
        confirmed_inputs,
        applied_states_compared,
        applied_state_mismatch_frames,
    }
}

/// F17 REGRESSION TEST (deterministic in-process reproduction; see the
/// section header above for the exact pre-S30 stale-stamp path). RED before
/// the S30 InputQueue fix (82 false-positive `DesyncDetected` events); GREEN
/// with it.
///
/// A 3-peer mesh at 0% loss with hold-then-release reordering on the two
/// links into peer A drives A's correcting boundary rollback to land at
/// `first_incorrect == 14` while its B-input window 10..13 arrives only
/// afterwards. Pre-S30, that window's misprediction comparisons were
/// swallowed, stranding prediction-stamped cells that the checksum harvest
/// stored and gossiped: every peer raised `DesyncDetected` although the
/// CONFIRMED INPUT STREAMS — read back through the public
/// `confirmed_inputs_for_frame` API — are byte-identical across all three
/// peers at every flagged frame, and the victim's applied trajectory stayed
/// silently divergent.
///
/// ORACLE (the FIXED behavior):
///   1. PREMISE: clean network — zero `Disconnected` events (pure
///      reordering; nothing is dropped).
///   2. PREMISE: the run drove well past the first checksum-interval frame.
///   3. PREMISE: the victim peer genuinely rolled back (>= 2 rollbacks: the
///      quirk-setting rollback and the boundary rollback).
///   4. FALSE-POSITIVE PROOF: at every `DesyncDetected` frame the confirmed
///      inputs are byte-identical across all peers (so a faithful checksum
///      of confirmed state MUST agree — the mismatch can only come from a
///      stale, prediction-influenced saved cell).
///   5. FIXED BEHAVIOR: NO `DesyncDetected` fires at all.
///   6. FIXED BEHAVIOR (stronger invariant): the peers' APPLIED state
///      trajectories converge — zero cross-peer applied-state mismatches at
///      commonly-confirmed frames (pre-S30 the swallowed window left them
///      divergent even with detection `Off`).
#[test]
fn reordered_arrival_boundary_rollback_does_not_false_positive_desync() {
    let outcome = run_reorder_choreography(35001);

    // PREMISE 1: pure reordering — no peer was disconnected.
    assert_eq!(
        outcome.disconnect_events, 0,
        "choreography must not disconnect anyone ({outcome:?})"
    );

    // PREMISE 2: the run confirmed well past the first interval frames.
    assert!(
        outcome.min_confirmed >= 20,
        "test must drive well past the stale checksum-interval frames \
         (min_confirmed={})",
        outcome.min_confirmed
    );

    // PREMISE 3: the victim peer genuinely rolled back.
    assert!(
        outcome.rollbacks[0] >= 2,
        "choreography premise unmet: victim peer rolled back {} times, \
         expected >= 2 (rollbacks={:?})",
        outcome.rollbacks[0],
        outcome.rollbacks
    );

    // PREMISE 4 / FALSE-POSITIVE PROOF: every flagged frame's confirmed
    // inputs are byte-identical across all three peers. If this ever fails,
    // the event would be a TRUE desync and this test's premise — not the
    // library's checksum pipeline — is broken.
    for &(peer, frame, local, remote) in &outcome.desync_events {
        let (a, b, c) = (
            outcome.confirmed_inputs[0].get(&frame),
            outcome.confirmed_inputs[1].get(&frame),
            outcome.confirmed_inputs[2].get(&frame),
        );
        match (a, b, c) {
            (Some(a), Some(b), Some(c)) => {
                assert!(
                    a == b && a == c,
                    "TRUE desync at flagged frame {frame} (peer {peer}, \
                     local={local:#x}, remote={remote:#x}): confirmed inputs \
                     differ across peers: {a:?} vs {b:?} vs {c:?}"
                );
            },
            _ => panic!(
                "flagged frame {frame} missing from a peer's confirmed-input \
                 capture (peer {peer}): {a:?} / {b:?} / {c:?} ({outcome:?})"
            ),
        }
    }

    // FIXED BEHAVIOR (finding F17): with byte-identical confirmed inputs on
    // every flagged frame, no DesyncDetected may fire. Pre-S30 the stale
    // prediction-stamped checksums were stored and gossiped, so this fired on
    // all three peers (82 events).
    assert!(
        outcome.desync_events.is_empty(),
        "FALSE-POSITIVE DesyncDetected: {} event(s) fired although the \
         confirmed inputs at every flagged frame are byte-identical across \
         all peers (stale prediction-stamped saved cell harvested — finding \
         F17): {:?}",
        outcome.desync_events.len(),
        outcome.desync_events
    );

    // FIXED BEHAVIOR, stronger invariant: the applied trajectories converge.
    // Every commonly-confirmed frame's final applied state is byte-identical
    // across all three peers — the pre-S30 swallowed window left the victim's
    // applied states divergent from frame 11 onward with no rollback and no
    // event (a silent gameplay desync with detection Off).
    assert!(
        outcome.applied_states_compared >= 20,
        "applied-state oracle is vacuous: only {} frames compared ({outcome:?})",
        outcome.applied_states_compared
    );
    assert!(
        outcome.applied_state_mismatch_frames.is_empty(),
        "APPLIED-STATE divergence across peers at confirmed frames {:?} — the \
         rollback that should reconverge the trajectories never happened \
         (finding F17): {outcome:?}",
        outcome.applied_state_mismatch_frames
    );
}

/// Determinism guard for the F17 reproduction: the choreography is a pure
/// function of its argument (fixed addresses, single-threaded shared-mutex
/// delivery, frame-indexed holds/releases, no wall-clock dependence), so 10
/// repeated runs MUST produce a byte-identical [`ReorderOutcome`] — including
/// the same false-positive `DesyncDetected` events while the bug is unfixed,
/// and the same empty event list once it is fixed.
#[test]
fn reordered_arrival_choreography_is_deterministic() {
    let first = run_reorder_choreography(36001);
    for run in 1..10 {
        let next = run_reorder_choreography(36001);
        assert_eq!(
            first, next,
            "reorder choreography was NON-deterministic on run {run}"
        );
    }
}

// ===========================================================================
// NEXT ACTION (S29) — DEFERRED `src/` FIX SPEC (historical; RESOLVED in S30)
// ===========================================================================
//
// S30 RESOLUTION: the fix landed at the ROOT CAUSE instead of this spec. The
// S30 adversarial review re-rooted F17 from the saved-cell layer into the
// InputQueue prediction lifecycle: a rollback-re-entered prediction episode
// started at the REQUESTED frame instead of the queue's first missing frame,
// silently swallowing the misprediction comparison for the skipped window.
// `InputQueue::input` now always enters an episode at the first missing frame
// (`src/input_queue/mod.rs`), which makes every arrival compared, makes the
// stale window unconstructible, and keeps the loaded boundary cell provably
// clean — so the per-cell metadata machinery specified in (a)/(b) below is
// unnecessary (it was symptom-layer and had verified holes). The text below
// is kept as the historical S29 analysis; its (c) prerequisite DID land (the
// reorder choreography above) and gates the fix as the regression test.
//
// VERDICT: REAL (see the module header). No production change landed in S29
// because a correctness-first codebase requires a RED test first, and no
// DETERMINISTIC in-process reproduction existed yet (the FIFO/injective harness
// here cannot strand the boundary cell). Both fixes explored in S29 FAIL:
//   - A "saved-while-confirmed" harvest GATE regresses legitimate detection: many
//     cells are saved speculatively-but-CORRECT and never re-saved (e.g.
//     `test_desync_detection_intervals_data_driven`), so gating on "was this cell
//     re-saved after confirmation" suppresses real desyncs.
//   - A one-frame F7-style boundary refresh (load one frame earlier) does NOT
//     reduce the real-UDP count (13/25 -> 11/25), because the staleness can span
//     MULTIPLE frames in the window, not just the single boundary frame.
//
// (a) PRECISE STALE CONDITION TO DETECT
//     A saved cell `cell[F]` whose stored checksum reflects PREDICTED (not-yet-
//     confirmed) inputs at harvest time, i.e. `cell[F]` was last written during a
//     speculative save OR loaded-but-never-re-saved as the boundary of a
//     `first_incorrect == F` rollback, and `last_confirmed_frame` has since
//     advanced to `>= F` with NO subsequent rollback re-saving `cell[F]`. The
//     harvest gate `F <= last_confirmed_frame && F <= last_saved_frame` is
//     necessary but NOT sufficient — it does not detect this staleness.
//
// (b) CANDIDATE FIX (trajectory-preserving)
//     Track, per saved cell, an "all-inputs-confirmed-at-save" bit (true iff every
//     player's input applied to reach that frame was a CONFIRMED input, not a
//     prediction, at save time). Then in `adjust_gamestate`, when a boundary
//     rollback's loaded cell is NOT confirmed-clean, load instead from the NEWEST
//     confirmed-clean cell `<= first_incorrect` (an F7-style earlier load, but to
//     the newest clean frame rather than exactly one frame earlier) and let the
//     re-simulation REBUILD every cell from `that clean base .. current` — so
//     ALL stale cells in the window are re-saved from a clean base, not just the
//     boundary. This preserves the confirmed trajectory (it only re-derives cells
//     that were stale) and closes the multi-frame-staleness gap the one-earlier
//     refresh missed. Alternatively/additionally, gate the harvest on the
//     per-cell confirmed-clean bit (skip-and-retry, mirroring the F11/M1 deferral
//     at `src/sessions/p2p_session.rs:4167`) so a stale cell is never harvested
//     even if a refresh is missed — but ONLY if the clean bit is precise enough to
//     avoid the legitimate-detection regression above.
//
// (c) DETERMINISTIC-REPRO PREREQUISITE (the red test that must come first)
//     The in-process harness needs a socket/schedule that deterministically drives
//     `first_incorrect == interval_frame` on ONE peer while ANOTHER peer's
//     `first_incorrect < interval_frame` (so one peer strands the boundary cell at
//     the interval frame and the other re-saves through it). A FIFO `DelaySocket`
//     cannot do this; the likely tool is a REORDERING in-process socket (deliver
//     a later packet before an earlier one for one link) or a hand-scheduled
//     arrival order that forces the asymmetric `first_incorrect` split. With that
//     RED test green-able only by the fix in (b), the deferred `src/` change can
//     land under the red-test-first policy.
//
//     S30 UPDATE: that prerequisite now EXISTS — see the "F17 DETERMINISTIC
//     REPRODUCTION (S30)" section above
//     (`reordered_arrival_boundary_rollback_does_not_false_positive_desync`,
//     RED on the pre-S30 code) and the hold-then-release `ReorderSocket` in
//     `tests/common/reorder_socket.rs`. The S30 root-cause fix (InputQueue
//     prediction-episode entry at the first missing frame) turned it GREEN.
