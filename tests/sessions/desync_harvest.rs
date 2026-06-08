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
//! Arbitration verdict: **NOTABUG**. The structural reason is pinned by the unit
//! tests in `src/sessions/p2p_session.rs`:
//! `checksum_harvest_uses_exact_frame_to_send_cell_not_a_later_cell` and
//! `checksum_harvest_skips_speculative_cell_above_last_confirmed_frame` (the
//! exact-frame `saved_state_by_frame` match plus the `<= last_confirmed_frame`
//! gate). Those are the genuinely library-constraining, provably non-vacuous
//! guards for the invariant.
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
    DesyncDetection, FortressEvent, Message, NonBlockingSocket, PlayerHandle, PlayerType,
    SessionBuilder, SessionState,
};

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
    let inboxes = Arc::new(Mutex::new(BTreeMap::new()));
    let tick = Arc::new(Mutex::new(0u64));
    let mk = |i: usize| DelaySocket {
        local_addr: addrs[i],
        inboxes: Arc::clone(&inboxes),
        pending: VecDeque::new(),
        tick: Arc::clone(&tick),
        delay: if i == delayed_peer { delay } else { 0 },
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
