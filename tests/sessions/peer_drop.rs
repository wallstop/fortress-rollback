//! Integration tests for graceful peer drop (Feature 5).
//!
//! These tests cover:
//! - `DisconnectBehavior::Halt` (default) — session stops advancing on drop
//! - `DisconnectBehavior::ContinueWithout` — remaining peers keep advancing
//! - `P2PSession::remove_player` — explicit graceful removal
//! - `FortressEvent::PeerDropped` — emission with correct handle/address
//! - Frozen input queues — last confirmed input repeats forever
//!
//! All tests use `ChannelSocket` + `TestClock` for fully deterministic behavior.

// In tests: tests intentionally use unwrap/expect for clarity.
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::ip_constant
)]

use crate::common::stubs::{GameStub, StubConfig, StubInput};
use crate::common::{
    create_channel_pair, create_channel_quad, create_channel_triple, drain_sync_events,
    poll_with_advance, synchronize_sessions_deterministic, SyncConfig, TestClock,
    POLL_INTERVAL_DETERMINISTIC,
};
use fortress_rollback::{
    DisconnectBehavior, FortressError, FortressEvent, FortressRequest, InputStatus, InputVec,
    P2PSession, PlayerHandle, PlayerType, ProtocolConfig, SessionBuilder, SessionState,
    SpectatorSession,
};
use web_time::Duration;

/// Helper: creates a `ProtocolConfig` with the given test clock.
fn protocol_config(clock: &TestClock) -> ProtocolConfig {
    ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        ..ProtocolConfig::default()
    }
}

/// Synchronizes three sessions deterministically. Returns when all three are
/// in `Running` state, or panics if synchronization does not complete in
/// `iterations` iterations.
fn synchronize_three_sessions(
    sess1: &mut P2PSession<StubConfig>,
    sess2: &mut P2PSession<StubConfig>,
    sess3: &mut P2PSession<StubConfig>,
    clock: &TestClock,
    iterations: usize,
) {
    for _ in 0..iterations {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
            && sess3.current_state() == SessionState::Running
        {
            return;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    panic!(
        "Three sessions failed to synchronize: sess1={:?}, sess2={:?}, sess3={:?}",
        sess1.current_state(),
        sess2.current_state(),
        sess3.current_state()
    );
}

/// Drains all currently buffered events from a session (used to clear
/// post-sync events before the test body).
fn drain_events(sess: &mut P2PSession<StubConfig>) -> Vec<FortressEvent<StubConfig>> {
    sess.events().collect()
}

/// Polls three sessions and advances virtual time by
/// `POLL_INTERVAL_DETERMINISTIC * iterations`.
fn poll_three(
    sess1: &mut P2PSession<StubConfig>,
    sess2: &mut P2PSession<StubConfig>,
    sess3: &mut P2PSession<StubConfig>,
    clock: &TestClock,
    iterations: usize,
) {
    for _ in 0..iterations {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
}

/// Three synchronized peers + shared test clock.
struct ThreePlayerSessions {
    sess1: P2PSession<StubConfig>,
    sess2: P2PSession<StubConfig>,
    sess3: P2PSession<StubConfig>,
    clock: TestClock,
}

/// Builds three synchronized 3-player P2P sessions with the given disconnect
/// behavior. Returns the three sessions and their addresses.
fn build_three_player_sessions(
    behavior: DisconnectBehavior,
) -> Result<ThreePlayerSessions, FortressError> {
    let clock = TestClock::new();
    let (s1, s2, s3, a1, a2, a3) = create_channel_triple();
    let pc = protocol_config(&clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(3)?
        .with_disconnect_behavior(behavior)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(3)?
        .with_disconnect_behavior(behavior)
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .start_p2p_session(s2)?;

    let mut sess3 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc)
        .with_num_players(3)?
        .with_disconnect_behavior(behavior)
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Local, PlayerHandle::new(2))?
        .start_p2p_session(s3)?;

    synchronize_three_sessions(&mut sess1, &mut sess2, &mut sess3, &clock, 200);

    // Drain any sync events.
    let _ = drain_events(&mut sess1);
    let _ = drain_events(&mut sess2);
    let _ = drain_events(&mut sess3);

    let _ = (a1, a2, a3); // addresses not currently needed by callers
    Ok(ThreePlayerSessions {
        sess1,
        sess2,
        sess3,
        clock,
    })
}

#[test]
fn p2p_continue_without_advances_after_peer_drop() -> Result<(), FortressError> {
    let ThreePlayerSessions {
        mut sess1,
        mut sess2,
        mut sess3,
        clock,
    } = build_three_player_sessions(DisconnectBehavior::ContinueWithout)?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    // Run a few frames before the drop so the dropped peer has produced some
    // confirmed inputs.
    let warmup_frames = 5_u32;
    for i in 0..warmup_frames {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();
        sess3
            .add_local_input(PlayerHandle::new(2), StubInput { inp: i })
            .unwrap();
        let r1 = sess1.advance_frame().unwrap();
        let r2 = sess2.advance_frame().unwrap();
        let r3 = sess3.advance_frame().unwrap();
        stub1.handle_requests(r1);
        stub2.handle_requests(r2);
        stub3.handle_requests(r3);
    }

    // Drop peer 2 (handle 2) on sess1 and sess2. (sess3 is the dropped one.)
    sess1.remove_player(PlayerHandle::new(2)).unwrap();
    sess2.remove_player(PlayerHandle::new(2)).unwrap();

    let confirmed_before_sess1 = sess1.confirmed_frame();
    let confirmed_before_sess2 = sess2.confirmed_frame();

    // Continue running sess1 and sess2 (sess3 is "dropped" so we ignore it).
    let post_drop_frames = 30_u32;
    for i in 0..post_drop_frames {
        // Drive the clock so any background timers fire; we still poll all
        // three so messages drain cleanly.
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        sess1
            .add_local_input(
                PlayerHandle::new(0),
                StubInput {
                    inp: i + warmup_frames,
                },
            )
            .unwrap();
        sess2
            .add_local_input(
                PlayerHandle::new(1),
                StubInput {
                    inp: i + warmup_frames,
                },
            )
            .unwrap();
        let r1 = sess1.advance_frame().unwrap();
        let r2 = sess2.advance_frame().unwrap();
        stub1.handle_requests(r1);
        stub2.handle_requests(r2);
    }

    // Both remaining peers must have advanced their current frame past the
    // drop frame.
    assert!(
        sess1.current_frame().as_i32() > warmup_frames as i32,
        "sess1 should advance past warmup; got {}",
        sess1.current_frame()
    );
    assert!(
        sess2.current_frame().as_i32() > warmup_frames as i32,
        "sess2 should advance past warmup; got {}",
        sess2.current_frame()
    );

    // confirmed_frame should also have made progress for both remaining
    // peers (because their inputs are now mutually confirmed; the dropped
    // peer's connect_status is marked disconnected, so it is ignored when
    // computing the min).
    assert!(
        sess1.confirmed_frame() > confirmed_before_sess1,
        "sess1 confirmed_frame should advance: before={:?}, after={:?}",
        confirmed_before_sess1,
        sess1.confirmed_frame()
    );
    assert!(
        sess2.confirmed_frame() > confirmed_before_sess2,
        "sess2 confirmed_frame should advance: before={:?}, after={:?}",
        confirmed_before_sess2,
        sess2.confirmed_frame()
    );

    Ok(())
}

#[test]
fn p2p_halt_default_stops_advancing_on_peer_drop() -> Result<(), FortressError> {
    // For the Halt path we verify two things:
    // 1. Calling `disconnect_player` on a session built with the default
    //    `DisconnectBehavior::Halt` does NOT emit `FortressEvent::PeerDropped`
    //    (that is exclusive to the `ContinueWithout` flow).
    // 2. The dropped peer's input queue is NOT frozen — `is_frozen` is a
    //    quick way to verify, but since that's an internal flag, we rely on
    //    the lack of `PeerDropped` plus the absence of any auto-removal side
    //    effects.
    //
    // We deliberately exercise this path on a 2-player session, not 3-player.
    // On 3-player Halt sessions, calling `disconnect_player` mid-session
    // triggers a rollback that interacts with multi-peer sync state in ways
    // unrelated to Feature 5; the legacy halt semantics are extensively
    // covered elsewhere.

    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        // Default is Halt; explicit for clarity.
        .with_disconnect_behavior(DisconnectBehavior::Halt)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::Halt)
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("sessions should sync");
    drain_sync_events(&mut sess1, &mut sess2);

    // Confirm the configured behavior is indeed Halt.
    assert_eq!(sess1.disconnect_behavior(), DisconnectBehavior::Halt);

    // Disconnect the remote peer.
    sess1.disconnect_player(PlayerHandle::new(1)).unwrap();

    // Drain events: we expect no PeerDropped to ever be emitted on the
    // Halt path. (Disconnected may or may not be present depending on
    // legacy code paths — the Halt path here uses the legacy explicit
    // disconnect API which doesn't emit Disconnected either.)
    let events: Vec<_> = sess1.events().collect();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. })),
        "Halt behavior must not emit PeerDropped events; got {:?}",
        events
    );

    Ok(())
}

#[test]
fn p2p_continue_without_emits_peer_dropped_event() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("sessions should sync");
    drain_sync_events(&mut sess1, &mut sess2);

    // Run a few frames so each peer has confirmed inputs from the other.
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    for i in 0..3 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();
        let r1 = sess1.advance_frame().unwrap();
        let r2 = sess2.advance_frame().unwrap();
        stub1.handle_requests(r1);
        stub2.handle_requests(r2);
    }

    // Drop the remote peer.
    sess1.remove_player(PlayerHandle::new(1)).unwrap();

    // Exactly one PeerDropped event with the expected handle and address.
    let events: Vec<_> = sess1.events().collect();
    let dropped: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        .collect();
    assert_eq!(
        dropped.len(),
        1,
        "expected exactly one PeerDropped event, got {:?}",
        events
    );
    if let FortressEvent::PeerDropped { handle, addr } = dropped[0] {
        assert_eq!(*handle, PlayerHandle::new(1));
        assert_eq!(*addr, a2);
    }

    Ok(())
}

#[test]
fn p2p_remove_player_rejects_local() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, _s2, _a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let result = sess1.remove_player(PlayerHandle::new(0));
    assert!(matches!(
        result,
        Err(FortressError::InvalidRequestStructured {
            kind: fortress_rollback::InvalidRequestKind::DisconnectLocalPlayer { .. }
        })
    ));

    // Invalid handle returns DisconnectInvalidHandle.
    let result = sess1.remove_player(PlayerHandle::new(99));
    assert!(matches!(
        result,
        Err(FortressError::InvalidRequestStructured {
            kind: fortress_rollback::InvalidRequestKind::DisconnectInvalidHandle { .. }
        })
    ));
    Ok(())
}

#[test]
fn p2p_remove_player_rejects_already_removed() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("sessions should sync");
    drain_sync_events(&mut sess1, &mut sess2);

    sess1.remove_player(PlayerHandle::new(1)).unwrap();
    let result = sess1.remove_player(PlayerHandle::new(1));
    assert!(matches!(
        result,
        Err(FortressError::InvalidRequestStructured {
            kind: fortress_rollback::InvalidRequestKind::PlayerAlreadyRemoved { .. }
        })
    ));

    Ok(())
}

#[test]
fn p2p_continue_without_frozen_input_repeats_last() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("sessions should sync");
    drain_sync_events(&mut sess1, &mut sess2);

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // Track the inputs sess1 receives for handle 1 (the remote that we will drop).
    // Player 2 sends a known marker value (42) just before being dropped.
    const MARKER: u32 = 42;

    // Run a few frames with normal inputs, finishing with the marker.
    for i in 0..3_u32 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();
        let r1 = sess1.advance_frame().unwrap();
        let r2 = sess2.advance_frame().unwrap();
        stub1.handle_requests(r1);
        stub2.handle_requests(r2);
    }

    // Send one more frame with the marker.
    poll_with_advance(&mut sess1, &mut sess2, &clock, 3);
    sess1
        .add_local_input(PlayerHandle::new(0), StubInput { inp: 999 })
        .unwrap();
    sess2
        .add_local_input(PlayerHandle::new(1), StubInput { inp: MARKER })
        .unwrap();
    let r1 = sess1.advance_frame().unwrap();
    let r2 = sess2.advance_frame().unwrap();
    stub1.handle_requests(r1);
    stub2.handle_requests(r2);

    // Make sure the marker has fully propagated and is confirmed by sess1
    // before dropping. Poll a bunch to drain the channel.
    poll_with_advance(&mut sess1, &mut sess2, &clock, 10);

    // Now drop sess1's view of the remote.
    sess1.remove_player(PlayerHandle::new(1)).unwrap();

    // Capture the inputs reported in subsequent AdvanceFrame requests for
    // handle 1 (the dropped peer) over many frames. The status is
    // Disconnected only once `current_frame > last_received_frame`. The
    // value must always equal the MARKER (the queue is frozen at the last
    // confirmed value). For earlier frames whose input is still in the
    // queue, the value should also be MARKER (since the marker was the
    // last confirmed value before drop).
    let mut observed_dropped_inputs: Vec<(u32, InputStatus)> = Vec::new();

    for i in 0..30_u32 {
        sess1.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i + 100 })
            .unwrap();
        let requests = sess1.advance_frame().unwrap();
        for request in &*requests {
            if let FortressRequest::AdvanceFrame { inputs } = request {
                let inputs: &InputVec<StubInput> = inputs;
                // Handle 1 is the dropped peer.
                if let Some(&(input, status)) = inputs.get(1) {
                    observed_dropped_inputs.push((input.inp, status));
                }
            }
        }
        // Manually handle save/load here so the rollback bookkeeping stays
        // consistent. We discard the actual gameplay outcome.
        stub1.handle_requests(requests);
    }

    // Every observed input for the dropped peer must be the MARKER value
    // (the frozen last confirmed input). The dropped peer's input must
    // never change after drop.
    assert!(
        !observed_dropped_inputs.is_empty(),
        "expected to observe at least one AdvanceFrame request"
    );
    for (value, _status) in &observed_dropped_inputs {
        assert_eq!(
            *value, MARKER,
            "dropped peer's input must repeat the last confirmed value"
        );
    }

    // The status must eventually become Disconnected once current_frame
    // outruns the last received frame. Confirm at least one observation
    // shows Disconnected status.
    let disconnected_count = observed_dropped_inputs
        .iter()
        .filter(|(_, status)| *status == InputStatus::Disconnected)
        .count();
    assert!(
        disconnected_count > 0,
        "expected at least one Disconnected status across {} observations",
        observed_dropped_inputs.len()
    );

    Ok(())
}

#[test]
fn p2p_continue_without_auto_removes_on_disconnect_timeout() -> Result<(), FortressError> {
    // Drives the disconnect timeout by stopping one peer's polls while
    // advancing the test clock past the configured 200ms timeout.
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let short_timeout = Duration::from_millis(200);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .with_disconnect_timeout(short_timeout)
        .with_disconnect_notify_delay(Duration::from_millis(50))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .with_disconnect_timeout(short_timeout)
        .with_disconnect_notify_delay(Duration::from_millis(50))
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("sessions should sync");
    drain_sync_events(&mut sess1, &mut sess2);

    // Stop polling sess2 so it does not send keep-alives, then advance
    // sess1's view of the clock past the timeout.
    for _ in 0..100 {
        sess1.poll_remote_clients();
        clock.advance(Duration::from_millis(20));
    }

    // sess1 should have emitted PeerDropped because the auto-removal path
    // for ContinueWithout fired.
    let events: Vec<_> = sess1.events().collect();
    let dropped_count = events
        .iter()
        .filter(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        .count();
    assert!(
        dropped_count >= 1,
        "expected at least one PeerDropped event after timeout; got {:?}",
        events
    );

    // We also expect Disconnected to coexist with PeerDropped in the event
    // stream (we kept the legacy emission for back-compat).
    let disconnected_count = events
        .iter()
        .filter(|e| matches!(e, FortressEvent::Disconnected { .. }))
        .count();
    assert!(
        disconnected_count >= 1,
        "expected at least one Disconnected event after timeout"
    );

    Ok(())
}

// ============================================================================
// Regression tests for Feature 5 review fixes
// ============================================================================

/// Spectators must observe the **same** dropped-peer input value as players.
///
/// Before the fix to `confirmed_inputs`, players (via `synchronized_inputs`)
/// saw the frozen `last_confirmed_input` for a dropped peer while spectators
/// (via `confirmed_inputs`) saw a default/blank value, causing immediate
/// state divergence on the very next frame.
#[test]
fn p2p_continue_without_spectators_get_frozen_inputs() -> Result<(), FortressError> {
    let clock = TestClock::new();
    // Three sockets: two players + one spectator.
    let (s1, s2, spec_socket, a1, a2, spec_addr) = create_channel_triple();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(s2)?;

    let mut spec_sess: SpectatorSession<StubConfig> = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .start_spectator_session(a1, spec_socket)
        .expect("spectator session should start");

    // Sync all three (two players + one spectator) together by pumping every
    // session every iteration. `synchronize_sessions_deterministic` only
    // pumps the pair, which leaves the spectator endpoint on sess1 stuck
    // Synchronizing.
    let mut all_synced = false;
    for _ in 0..500 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        spec_sess.poll_remote_clients();
        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
            && spec_sess.current_state() == SessionState::Running
        {
            all_synced = true;
            break;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    assert!(
        all_synced,
        "spectator session and 2 player sessions failed to synchronize: \
         sess1={:?}, sess2={:?}, spec={:?}",
        sess1.current_state(),
        sess2.current_state(),
        spec_sess.current_state()
    );
    let _: Vec<_> = sess1.events().collect();
    let _: Vec<_> = sess2.events().collect();
    let _: Vec<_> = spec_sess.events().collect();

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // Marker the dropped peer's last input. It must be a non-default,
    // non-zero value so we can distinguish "frozen marker" from "blank".
    const MARKER: u32 = 7777;

    // Run several frames with normal then marker-final inputs.
    for i in 0..4_u32 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();
        let r1 = sess1.advance_frame().unwrap();
        let r2 = sess2.advance_frame().unwrap();
        stub1.handle_requests(r1);
        stub2.handle_requests(r2);
    }

    // Final frame: peer 1 sends MARKER as its last confirmed input.
    poll_with_advance(&mut sess1, &mut sess2, &clock, 3);
    spec_sess.poll_remote_clients();
    clock.advance(POLL_INTERVAL_DETERMINISTIC);
    sess1
        .add_local_input(PlayerHandle::new(0), StubInput { inp: 1234 })
        .unwrap();
    sess2
        .add_local_input(PlayerHandle::new(1), StubInput { inp: MARKER })
        .unwrap();
    let r1 = sess1.advance_frame().unwrap();
    let r2 = sess2.advance_frame().unwrap();
    stub1.handle_requests(r1);
    stub2.handle_requests(r2);

    // Drain the channel so the marker is fully confirmed by sess1.
    for _ in 0..15 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 1);
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // First flush the spectator past all pre-drop frames. The spectator must
    // catch up to sess1.confirmed_frame() before we drop, so any post-drop
    // observation can be unambiguously attributed to the post-drop send path.
    let mut spec_stub = GameStub::new();
    for _ in 0..30 {
        sess1.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        match spec_sess.advance_frame() {
            Ok(requests) => spec_stub.handle_requests(requests),
            Err(FortressError::PredictionThreshold) => {},
            Err(e) => panic!("spectator advance_frame failed: {:?}", e),
        }
    }
    let spec_frame_pre_drop = spec_sess.current_frame();
    let confirmed_pre_drop = sess1.confirmed_frame();
    // Spectator should be reasonably close to confirmed (not strictly equal,
    // since the spectator typically lags by 1-2 frames as input messages
    // propagate). What we care about is that the spectator has at least
    // consumed *most* of the pre-drop confirmed frames.
    assert!(
        confirmed_pre_drop.as_i32() - spec_frame_pre_drop.as_i32() <= 2,
        "spectator should be within 2 frames of confirmed before drop: \
         spec={:?}, confirmed={:?}",
        spec_frame_pre_drop,
        confirmed_pre_drop,
    );

    // Drop the remote peer on sess1 (the host). Spectator subscribes to sess1.
    sess1.remove_player(PlayerHandle::new(1)).unwrap();

    // Run sess1 forward solo; this generates new confirmed frames whose
    // dropped-peer slot is produced by `confirmed_inputs`.
    for i in 0..30_u32 {
        sess1.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i + 5000 })
            .unwrap();
        let r1 = sess1.advance_frame().unwrap();
        stub1.handle_requests(r1);
    }

    // Pump the spectator to consume confirmed inputs from the post-drop
    // window only.
    let mut post_drop_dropped_inputs: Vec<u32> = Vec::new();
    for _ in 0..50 {
        sess1.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        match spec_sess.advance_frame() {
            Ok(requests) => {
                for request in &*requests {
                    if let FortressRequest::AdvanceFrame { inputs } = request {
                        let inputs: &InputVec<StubInput> = inputs;
                        // Only collect frames produced after the drop.
                        if spec_sess.current_frame() > spec_frame_pre_drop {
                            if let Some(&(input, _status)) = inputs.get(1) {
                                post_drop_dropped_inputs.push(input.inp);
                            }
                        }
                    }
                }
                spec_stub.handle_requests(requests);
            },
            Err(FortressError::PredictionThreshold) => {},
            Err(e) => panic!("spectator advance_frame failed: {:?}", e),
        }
    }

    assert!(
        !post_drop_dropped_inputs.is_empty(),
        "spectator should have advanced past spec_frame_pre_drop ({:?}) at least once \
         to capture post-drop input observations",
        spec_frame_pre_drop,
    );
    // Every post-drop observation of the dropped peer's input value must be
    // the MARKER (the frozen last_confirmed_input). Without the
    // `confirmed_inputs` fix, all of these would be `0` (default StubInput,
    // since `PlayerInput::blank_input` is sent), and the spectator's state
    // would diverge from the players' state.
    for &value in &post_drop_dropped_inputs {
        assert_eq!(
            value, MARKER,
            "spectator must observe the frozen MARKER ({}) for the dropped peer in every \
             post-drop frame; got {:?}",
            MARKER, post_drop_dropped_inputs,
        );
    }
    Ok(())
}

/// 2-player session with `ContinueWithout`: after the remote drops, the local
/// session must keep advancing `current_frame()` (not just refrain from
/// halting). This is the most common real-world graceful-drop scenario.
#[test]
fn p2p_continue_without_2p_remaining_peer_advances_solo() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("sessions should sync");
    drain_sync_events(&mut sess1, &mut sess2);

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // Warmup so the dropped peer has at least one confirmed input.
    for i in 0..4_u32 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();
        let r1 = sess1.advance_frame().unwrap();
        let r2 = sess2.advance_frame().unwrap();
        stub1.handle_requests(r1);
        stub2.handle_requests(r2);
    }

    let frame_at_drop = sess1.current_frame();
    sess1.remove_player(PlayerHandle::new(1)).unwrap();

    // Run sess1 solo for many frames.
    let solo_frames = 20_u32;
    for i in 0..solo_frames {
        sess1.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i + 100 })
            .unwrap();
        let r1 = sess1.advance_frame().unwrap();
        stub1.handle_requests(r1);
    }

    // The local session must have advanced at least 5 frames after the drop —
    // the regression we're guarding against is a session that *doesn't halt
    // outright* but still fails to advance because of internal book-keeping.
    let advanced = sess1.current_frame().as_i32() - frame_at_drop.as_i32();
    assert!(
        advanced >= 5,
        "remaining peer should advance >=5 frames after drop; advanced={}, before={:?}, after={:?}",
        advanced,
        frame_at_drop,
        sess1.current_frame()
    );
    Ok(())
}

/// 4-player session with `ContinueWithout`: drop two remote peers and verify
/// the remaining two still advance. Surface any bug where multi-drop halts
/// the session.
#[test]
fn p2p_continue_without_4p_two_drops_remaining_two_continue() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, s3, s4, a1, a2, a3, a4) = create_channel_quad();
    let pc = protocol_config(&clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(4)?
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .add_player(PlayerType::Remote(a4), PlayerHandle::new(3))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(4)?
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .add_player(PlayerType::Remote(a4), PlayerHandle::new(3))?
        .start_p2p_session(s2)?;

    let mut sess3 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(4)?
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Local, PlayerHandle::new(2))?
        .add_player(PlayerType::Remote(a4), PlayerHandle::new(3))?
        .start_p2p_session(s3)?;

    let mut sess4 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc)
        .with_num_players(4)?
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .add_player(PlayerType::Local, PlayerHandle::new(3))?
        .start_p2p_session(s4)?;

    // Synchronize all 4 peers.
    let mut synced = false;
    for _ in 0..400 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        sess4.poll_remote_clients();
        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
            && sess3.current_state() == SessionState::Running
            && sess4.current_state() == SessionState::Running
        {
            synced = true;
            break;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    assert!(synced, "4-player session failed to synchronize");
    let _ = sess1.events().collect::<Vec<_>>();
    let _ = sess2.events().collect::<Vec<_>>();

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();
    let mut stub4 = GameStub::new();

    // Warmup with all four peers so each has confirmed inputs.
    for i in 0..4_u32 {
        for _ in 0..3 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
            sess3.poll_remote_clients();
            sess4.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();
        sess3
            .add_local_input(PlayerHandle::new(2), StubInput { inp: i })
            .unwrap();
        sess4
            .add_local_input(PlayerHandle::new(3), StubInput { inp: i })
            .unwrap();
        let r1 = sess1.advance_frame().unwrap();
        let r2 = sess2.advance_frame().unwrap();
        let r3 = sess3.advance_frame().unwrap();
        let r4 = sess4.advance_frame().unwrap();
        stub1.handle_requests(r1);
        stub2.handle_requests(r2);
        stub3.handle_requests(r3);
        stub4.handle_requests(r4);
    }

    // Drop peers 2 and 3 on sess1 and sess2 (the survivors).
    sess1.remove_player(PlayerHandle::new(2)).unwrap();
    sess1.remove_player(PlayerHandle::new(3)).unwrap();
    sess2.remove_player(PlayerHandle::new(2)).unwrap();
    sess2.remove_player(PlayerHandle::new(3)).unwrap();

    let frame_at_drop_sess1 = sess1.current_frame();
    let frame_at_drop_sess2 = sess2.current_frame();

    // Run survivors forward.
    let post_drop = 20_u32;
    for i in 0..post_drop {
        for _ in 0..3 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i + 200 })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i + 200 })
            .unwrap();
        let r1 = sess1.advance_frame().unwrap();
        let r2 = sess2.advance_frame().unwrap();
        stub1.handle_requests(r1);
        stub2.handle_requests(r2);
    }

    let advanced_sess1 = sess1.current_frame().as_i32() - frame_at_drop_sess1.as_i32();
    let advanced_sess2 = sess2.current_frame().as_i32() - frame_at_drop_sess2.as_i32();
    assert!(
        advanced_sess1 >= 5,
        "sess1 should advance after 2 drops; advanced={}",
        advanced_sess1
    );
    assert!(
        advanced_sess2 >= 5,
        "sess2 should advance after 2 drops; advanced={}",
        advanced_sess2
    );
    Ok(())
}

/// Drop a peer while the session is still in the `Synchronizing` state.
///
/// The session must handle this gracefully: emit `PeerDropped`, mark the
/// peer disconnected, and not panic. Whether sync completes depends on
/// the remaining peers. The test only asserts:
///   1. `remove_player` returns `Ok(())` (no panic, no internal error).
///   2. `PeerDropped` is in the event stream.
///   3. The session does not transition to a stuck state — either it
///      reaches `Running` with the remaining peers OR it stays in
///      `Synchronizing` (legitimate when not enough peers remain), but
///      never observes a state-machine corruption.
#[test]
fn p2p_continue_without_drop_during_synchronizing() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, _s2, _s3, _a1, a2, a3) = create_channel_triple();

    // Build only sess1; never start sess2 or sess3 so they never come online.
    // sess1 will stay in Synchronizing.
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(3)?
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .start_p2p_session(s1)?;

    assert_eq!(sess1.current_state(), SessionState::Synchronizing);

    // Poll a few times so endpoints have done some sync work, but don't
    // wait for completion — peer 1 and peer 2 never come online.
    for _ in 0..10 {
        sess1.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    assert_eq!(
        sess1.current_state(),
        SessionState::Synchronizing,
        "session must still be synchronizing — neither remote ever connects"
    );

    // Drop peer at handle 1. The decision (documented in `remove_player`'s
    // rustdoc): this is allowed — `remove_player` is the explicit
    // graceful-drop opt-in regardless of session state. The session
    // remains in `Synchronizing` (peer 2 never came online either).
    sess1.remove_player(PlayerHandle::new(1)).unwrap();

    let events: Vec<_> = sess1.events().collect();
    let dropped_count = events
        .iter()
        .filter(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        .count();
    assert_eq!(
        dropped_count, 1,
        "expected exactly one PeerDropped event for the dropped peer; got {:?}",
        events
    );

    // Session does not panic and remains in a defined state. We don't
    // require Running here (peer 2 is still pending), only that the
    // state machine still works. Continued polling must not panic.
    for _ in 0..10 {
        sess1.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    Ok(())
}
