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

use crate::common::stubs::{GameStub, StateStub, StubConfig, StubInput};
use crate::common::{
    create_channel_pair, create_channel_quad, create_channel_triple,
    create_filtered_channel_triple, drain_sync_events, poll_with_advance,
    synchronize_sessions_deterministic, BlockedLinks, FilterSocket, SyncConfig, TestClock,
    POLL_INTERVAL_DETERMINISTIC,
};
use fortress_rollback::{
    DisconnectBehavior, FortressError, FortressEvent, FortressRequest, Frame, InputStatus,
    InputVec, P2PSession, PlayerHandle, PlayerType, ProtocolConfig, SessionBuilder, SessionState,
    SpectatorSession,
};
use std::collections::BTreeMap;
use std::net::SocketAddr;
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

fn advance_session(
    session: &mut P2PSession<StubConfig>,
    stub: &mut GameStub,
    handle: PlayerHandle,
    value: u32,
) -> Result<Vec<(Frame, InputVec<StubInput>)>, FortressError> {
    session.add_local_input(handle, StubInput { inp: value })?;
    let mut frame = session.current_frame();
    let requests = session.advance_frame()?;
    let mut advanced_inputs = Vec::new();
    for request in &*requests {
        if let FortressRequest::AdvanceFrame { inputs } = request {
            advanced_inputs.push((frame, inputs.clone()));
            frame = Frame::new(frame.as_i32() + 1);
        }
    }
    stub.handle_requests(requests);
    Ok(advanced_inputs)
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
fn p2p_continue_without_propagated_disconnect_freezes_dropped_peer() -> Result<(), FortressError> {
    let ThreePlayerSessions {
        mut sess1,
        mut sess2,
        mut sess3,
        clock,
    } = build_three_player_sessions(DisconnectBehavior::ContinueWithout)?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    const MARKER_C: u32 = 4242;
    for i in 0..3_u32 {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        advance_session(&mut sess1, &mut stub1, PlayerHandle::new(0), i)?;
        advance_session(&mut sess2, &mut stub2, PlayerHandle::new(1), i + 10)?;
        advance_session(&mut sess3, &mut stub3, PlayerHandle::new(2), i + 20)?;
    }
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 8);
    advance_session(&mut sess1, &mut stub1, PlayerHandle::new(0), 100)?;
    advance_session(&mut sess2, &mut stub2, PlayerHandle::new(1), 200)?;
    advance_session(&mut sess3, &mut stub3, PlayerHandle::new(2), MARKER_C)?;
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 15);

    // Session 2 learns that C is gone first. Session 1 must learn that
    // through B's propagated connection status, even though C has not timed
    // out locally on session 1.
    sess2.remove_player(PlayerHandle::new(2))?;
    let _ = drain_events(&mut sess2);

    let mut observed_c = Vec::new();
    for i in 0..30_u32 {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        advance_session(&mut sess2, &mut stub2, PlayerHandle::new(1), i + 300)?;
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        let inputs = advance_session(&mut sess1, &mut stub1, PlayerHandle::new(0), i + 400)?;
        for (frame, frame_inputs) in inputs {
            if let Some(&(input, status)) = frame_inputs.get(2) {
                observed_c.push((frame, input.inp, status));
            }
        }
    }

    let events = drain_events(&mut sess1);
    let peer_dropped_count = events
        .iter()
        .filter(|event| matches!(event, FortressEvent::PeerDropped { .. }))
        .count();
    assert_eq!(
        peer_dropped_count, 1,
        "propagated ContinueWithout drop must emit exactly one PeerDropped; got {events:?}"
    );
    let peer_dropped_c_count = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                FortressEvent::PeerDropped {
                    handle,
                    ..
                } if *handle == PlayerHandle::new(2)
            )
        })
        .count();
    assert_eq!(
        peer_dropped_c_count, 1,
        "propagated ContinueWithout drop must emit PeerDropped for C exactly once; got {events:?}"
    );
    let disconnected_count = events
        .iter()
        .filter(|event| matches!(event, FortressEvent::Disconnected { .. }))
        .count();
    assert_eq!(
        disconnected_count, 1,
        "propagated ContinueWithout drop must emit exactly one Disconnected; got {events:?}"
    );
    let first_disconnected_index = observed_c
        .iter()
        .position(|&(_, _, status)| status == InputStatus::Disconnected)
        .expect("propagated drop must eventually mark C disconnected");
    let connected_after_cutoff: Vec<_> = observed_c
        .iter()
        .skip(first_disconnected_index)
        .filter(|&&(_, _, status)| status != InputStatus::Disconnected)
        .collect();
    assert_eq!(
        connected_after_cutoff,
        Vec::<&(Frame, u32, InputStatus)>::new(),
        "frames after propagated cutoff must stay disconnected; got {observed_c:?}"
    );
    let disconnected_marker_count = observed_c
        .iter()
        .filter(|&&(_, value, status)| value == MARKER_C && status == InputStatus::Disconnected)
        .count();
    assert!(
        disconnected_marker_count > 0,
        "session 1 must eventually simulate C's frozen marker as disconnected; got {observed_c:?}"
    );

    Ok(())
}

#[test]
fn p2p_halt_propagated_disconnect_transitions_to_synchronizing() -> Result<(), FortressError> {
    let ThreePlayerSessions {
        mut sess1,
        mut sess2,
        mut sess3,
        clock,
    } = build_three_player_sessions(DisconnectBehavior::Halt)?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    for i in 0..4_u32 {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        advance_session(&mut sess1, &mut stub1, PlayerHandle::new(0), i)?;
        advance_session(&mut sess2, &mut stub2, PlayerHandle::new(1), i + 10)?;
        advance_session(&mut sess3, &mut stub3, PlayerHandle::new(2), i + 20)?;
    }
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 10);

    sess2.remove_player(PlayerHandle::new(2))?;
    let _ = drain_events(&mut sess2);

    let mut detected_without_advance = false;
    for i in 0..12_u32 {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        advance_session(&mut sess2, &mut stub2, PlayerHandle::new(1), i + 100)?;
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        let frame_before_detecting_call = sess1.current_frame();
        sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: i + 200 })?;
        match sess1.advance_frame() {
            Err(FortressError::NotSynchronized) => {
                assert_eq!(
                    sess1.current_frame(),
                    frame_before_detecting_call,
                    "detecting a propagated Halt drop must not advance one extra frame"
                );
                detected_without_advance = true;
                break;
            },
            Ok(requests) => {
                stub1.handle_requests(requests);
            },
            Err(err) => return Err(err),
        }
    }

    assert!(
        detected_without_advance,
        "session 1 should detect the propagated Halt drop during advance_frame"
    );
    assert_eq!(
        sess1.current_state(),
        SessionState::Synchronizing,
        "propagated Halt drop must fail closed"
    );
    let events = drain_events(&mut sess1);
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, FortressEvent::PeerDropped { .. })),
        "Halt propagated drop must not emit PeerDropped; got {events:?}"
    );
    sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: 999 })?;
    assert!(
        matches!(sess1.advance_frame(), Err(FortressError::NotSynchronized)),
        "Halt propagated drop must reject further frame advance"
    );

    Ok(())
}

#[test]
fn p2p_halt_default_stops_advancing_on_peer_drop() -> Result<(), FortressError> {
    // For the Halt path we verify two things:
    // 1. Calling `disconnect_player` on a session built with the default
    //    `DisconnectBehavior::Halt` does NOT emit `FortressEvent::PeerDropped`
    //    (that is exclusive to the `ContinueWithout` flow).
    // 2. The session stops advancing after the drop instead of substituting
    //    default input for the disconnected peer.
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
    let current_before_disconnect = sess1.current_frame();
    let confirmed_before_disconnect = sess1.confirmed_frame();
    sess1.disconnect_player(PlayerHandle::new(1)).unwrap();
    assert_eq!(sess1.current_state(), SessionState::Synchronizing);

    sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: 99 })?;
    let advance_result = sess1.advance_frame();
    assert!(
        matches!(advance_result, Err(FortressError::NotSynchronized)),
        "Halt behavior must reject frame advance after explicit disconnect"
    );
    assert_eq!(
        sess1.current_frame(),
        current_before_disconnect,
        "Halt behavior must not advance current_frame after disconnect"
    );
    assert_eq!(
        sess1.confirmed_frame(),
        confirmed_before_disconnect,
        "Halt behavior must not advance confirmed_frame after disconnect"
    );

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
fn p2p_continue_without_late_packets_after_freeze_do_not_mutate_input() -> Result<(), FortressError>
{
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

    const FROZEN_MARKER: u32 = 31337;
    const LATE_PACKET_MARKER: u32 = 999_001;

    for i in 0..3_u32 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);
        sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess2.add_local_input(
            PlayerHandle::new(1),
            StubInput {
                inp: if i == 2 { FROZEN_MARKER } else { i + 100 },
            },
        )?;
        let r1 = sess1.advance_frame()?;
        let r2 = sess2.advance_frame()?;
        stub1.handle_requests(r1);
        stub2.handle_requests(r2);
    }
    poll_with_advance(&mut sess1, &mut sess2, &clock, 15);

    sess1.remove_player(PlayerHandle::new(1))?;
    let frame_at_drop = sess1.current_frame();

    // Keep the dropped peer sending new inputs and poll sess1 so those late
    // packets are delivered after sess1 has frozen handle 1. The frozen queue
    // must ignore them and keep returning FROZEN_MARKER.
    let mut observed_remote_inputs = Vec::new();
    for i in 0..24_u32 {
        sess2.add_local_input(
            PlayerHandle::new(1),
            StubInput {
                inp: LATE_PACKET_MARKER + i,
            },
        )?;
        match sess2.advance_frame() {
            Ok(requests) => stub2.handle_requests(requests),
            Err(FortressError::NotSynchronized) => {},
            Err(err) => panic!("unexpected dropped-peer advance_frame error: {err:?}"),
        }

        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);
        sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: 50_000 + i })?;
        let requests = sess1.advance_frame()?;
        for request in &*requests {
            if let FortressRequest::AdvanceFrame { inputs } = request {
                if sess1.current_frame() > frame_at_drop {
                    if let Some(&(input, status)) = inputs.get(1) {
                        observed_remote_inputs.push((input.inp, status));
                    }
                }
            }
        }
        stub1.handle_requests(requests);
    }

    assert!(
        !observed_remote_inputs.is_empty(),
        "expected post-freeze observations from the dropped handle"
    );
    assert!(
        observed_remote_inputs
            .iter()
            .any(|(_, status)| *status == InputStatus::Disconnected),
        "late packets must not prevent the frozen handle from reporting Disconnected; got {observed_remote_inputs:?}"
    );
    for (value, _status) in &observed_remote_inputs {
        assert_eq!(
            *value, FROZEN_MARKER,
            "late packets after freeze must not replace the frozen input; got {observed_remote_inputs:?}"
        );
    }

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

#[test]
fn p2p_halt_auto_timeout_stops_advancing_without_peer_dropped() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let short_timeout = Duration::from_millis(200);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::Halt)
        .with_disconnect_timeout(short_timeout)
        .with_disconnect_notify_delay(Duration::from_millis(50))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_behavior(DisconnectBehavior::Halt)
        .with_disconnect_timeout(short_timeout)
        .with_disconnect_notify_delay(Duration::from_millis(50))
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("sessions should sync");
    drain_sync_events(&mut sess1, &mut sess2);

    for _ in 0..100 {
        sess1.poll_remote_clients();
        clock.advance(Duration::from_millis(20));
    }

    let current_after_timeout = sess1.current_frame();
    let confirmed_after_timeout = sess1.confirmed_frame();
    assert_eq!(sess1.current_state(), SessionState::Synchronizing);

    let events: Vec<_> = sess1.events().collect();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, FortressEvent::Disconnected { .. })),
        "Halt timeout should still emit the legacy Disconnected event; got {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. })),
        "Halt timeout must not emit PeerDropped; got {events:?}"
    );

    sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: 100 })?;
    let advance_result = sess1.advance_frame();
    assert!(
        matches!(advance_result, Err(FortressError::NotSynchronized)),
        "Halt timeout must reject frame advance after disconnect"
    );
    assert_eq!(sess1.current_frame(), current_after_timeout);
    assert_eq!(sess1.confirmed_frame(), confirmed_after_timeout);

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

// ============================================================================
// Multi-handle endpoint regression tests
//
// A single remote `T::Address` can own multiple `PlayerHandle` (e.g. couch
// co-op behind one socket). The graceful-drop contract — "freeze each
// affected player's input queue so simulation keeps producing the last
// confirmed input" — must apply to *every* handle owned by the dropped
// endpoint, not just the targeted one. These tests guard against regressions
// where only the targeted handle's queue is frozen.
// ============================================================================

/// Two synchronized peers where session A registers session B as **two**
/// remote player handles sharing a single address. Returns the sessions and
/// the shared address `addr_b`.
struct MultiHandleSessions {
    sess_a: P2PSession<StubConfig>,
    sess_b: P2PSession<StubConfig>,
    addr_b: std::net::SocketAddr,
    clock: TestClock,
}

#[track_caller]
fn build_multi_handle_sessions(
    behavior: DisconnectBehavior,
    disconnect_timeout: Option<Duration>,
) -> Result<MultiHandleSessions, FortressError> {
    let clock = TestClock::new();
    let (s_a, s_b, a_a, a_b) = create_channel_pair();

    let mut a_builder = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(3)?
        .with_disconnect_behavior(behavior);
    let mut b_builder = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(3)?
        .with_disconnect_behavior(behavior);
    if let Some(timeout) = disconnect_timeout {
        a_builder = a_builder
            .with_disconnect_timeout(timeout)
            .with_disconnect_notify_delay(Duration::from_millis(50));
        b_builder = b_builder
            .with_disconnect_timeout(timeout)
            .with_disconnect_notify_delay(Duration::from_millis(50));
    }

    // Session A: local at handle 0; handles 1 AND 2 are remote at addr_b
    // (the two players that session B owns locally).
    let mut sess_a = a_builder
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a_b), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a_b), PlayerHandle::new(2))?
        .start_p2p_session(s_a)?;

    // Session B: handle 0 is remote (session A); handles 1 and 2 are local.
    let mut sess_b = b_builder
        .add_player(PlayerType::Remote(a_a), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Local, PlayerHandle::new(2))?
        .start_p2p_session(s_b)?;

    synchronize_sessions_deterministic(&mut sess_a, &mut sess_b, &clock, &SyncConfig::default())
        .expect("multi-handle sessions should sync");
    drain_sync_events(&mut sess_a, &mut sess_b);

    Ok(MultiHandleSessions {
        sess_a,
        sess_b,
        addr_b: a_b,
        clock,
    })
}

/// `remove_player` on a multi-handle endpoint must freeze every handle's
/// input queue and emit one `PeerDropped` per handle, followed by a single
/// address-level `Disconnected`. Both handles' inputs must surface the last
/// confirmed value (not the default), confirming the queues are actually
/// frozen.
#[test]
fn p2p_remove_player_multi_handle_freezes_all_handles_at_address() -> Result<(), FortressError> {
    let MultiHandleSessions {
        mut sess_a,
        mut sess_b,
        addr_b,
        clock,
    } = build_multi_handle_sessions(DisconnectBehavior::ContinueWithout, None)?;

    let mut stub_a = GameStub::new();
    let mut stub_b = GameStub::new();

    // Distinct marker inputs so we can tell them apart in the frozen state.
    const MARKER_H1: u32 = 1111;
    const MARKER_H2: u32 = 2222;

    // Run several frames so each handle has a confirmed input, then a final
    // frame that establishes the markers as the last confirmed input.
    for i in 0..3_u32 {
        poll_with_advance(&mut sess_a, &mut sess_b, &clock, 3);
        sess_a
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess_b
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i + 10 })
            .unwrap();
        sess_b
            .add_local_input(PlayerHandle::new(2), StubInput { inp: i + 20 })
            .unwrap();
        let r_a = sess_a.advance_frame().unwrap();
        let r_b = sess_b.advance_frame().unwrap();
        stub_a.handle_requests(r_a);
        stub_b.handle_requests(r_b);
    }

    // Final frame: B sends MARKER_H1 / MARKER_H2 as the last confirmed inputs.
    poll_with_advance(&mut sess_a, &mut sess_b, &clock, 3);
    sess_a
        .add_local_input(PlayerHandle::new(0), StubInput { inp: 9999 })
        .unwrap();
    sess_b
        .add_local_input(PlayerHandle::new(1), StubInput { inp: MARKER_H1 })
        .unwrap();
    sess_b
        .add_local_input(PlayerHandle::new(2), StubInput { inp: MARKER_H2 })
        .unwrap();
    let r_a = sess_a.advance_frame().unwrap();
    let r_b = sess_b.advance_frame().unwrap();
    stub_a.handle_requests(r_a);
    stub_b.handle_requests(r_b);

    // Drain so the markers are fully confirmed by sess_a.
    poll_with_advance(&mut sess_a, &mut sess_b, &clock, 15);

    // Drop the multi-handle endpoint by calling remove_player for ONLY
    // handle 1. The fix must drop both handle 1 AND handle 2 (sharing addr_b).
    sess_a.remove_player(PlayerHandle::new(1)).unwrap();

    // Capture all events emitted on sess_a after remove_player.
    let events: Vec<_> = sess_a.events().collect();
    let dropped_handles: Vec<PlayerHandle> = events
        .iter()
        .filter_map(|e| match e {
            FortressEvent::PeerDropped { handle, addr } => {
                assert_eq!(
                    *addr, addr_b,
                    "PeerDropped addr must match dropped endpoint"
                );
                Some(*handle)
            },
            _ => None,
        })
        .collect();

    assert_eq!(
        dropped_handles.len(),
        2,
        "expected exactly two PeerDropped events (handles 1 and 2); got {:?}",
        events
    );
    assert!(
        dropped_handles.contains(&PlayerHandle::new(1)),
        "PeerDropped events must include handle 1; got {:?}",
        dropped_handles
    );
    assert!(
        dropped_handles.contains(&PlayerHandle::new(2)),
        "PeerDropped events must include handle 2 (multi-handle endpoint regression); got {:?}",
        dropped_handles
    );

    let disconnected_count = events
        .iter()
        .filter(|e| matches!(e, FortressEvent::Disconnected { .. }))
        .count();
    assert_eq!(
        disconnected_count, 1,
        "expected exactly one Disconnected event (per address); got {:?}",
        events
    );

    // Advance solo on sess_a; the input slot for BOTH handle 1 AND handle 2
    // must surface the frozen markers, never the default value (0).
    let mut h1_observations: Vec<(u32, InputStatus)> = Vec::new();
    let mut h2_observations: Vec<(u32, InputStatus)> = Vec::new();
    for i in 0..30_u32 {
        sess_a.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        sess_a
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i + 100 })
            .unwrap();
        let requests = sess_a.advance_frame().unwrap();
        for request in &*requests {
            if let FortressRequest::AdvanceFrame { inputs } = request {
                let inputs: &InputVec<StubInput> = inputs;
                if let Some(&(input, status)) = inputs.get(1) {
                    h1_observations.push((input.inp, status));
                }
                if let Some(&(input, status)) = inputs.get(2) {
                    h2_observations.push((input.inp, status));
                }
            }
        }
        stub_a.handle_requests(requests);
    }

    assert!(
        !h1_observations.is_empty() && !h2_observations.is_empty(),
        "should have observed at least one AdvanceFrame request"
    );
    for (value, _status) in &h1_observations {
        assert_eq!(
            *value, MARKER_H1,
            "handle 1 input must be the frozen MARKER_H1 ({}); got {:?}",
            MARKER_H1, h1_observations
        );
    }
    for (value, _status) in &h2_observations {
        assert_eq!(
            *value, MARKER_H2,
            "handle 2 input must be the frozen MARKER_H2 ({}) — multi-handle freeze regression; \
             got {:?}",
            MARKER_H2, h2_observations
        );
    }

    // At least one observation per handle must report Disconnected status
    // once current_frame has outrun the last received frame.
    assert!(
        h1_observations
            .iter()
            .any(|(_, s)| *s == InputStatus::Disconnected),
        "handle 1 must report Disconnected status at least once"
    );
    assert!(
        h2_observations
            .iter()
            .any(|(_, s)| *s == InputStatus::Disconnected),
        "handle 2 must report Disconnected status at least once \
         (multi-handle freeze regression — handle 2 was previously left unfrozen)"
    );

    Ok(())
}

/// Same multi-handle setup, but trigger auto-disconnect via timeout under
/// `ContinueWithout`. Both handles must end up frozen and surface
/// `PeerDropped` events.
#[test]
fn p2p_continue_without_multi_handle_auto_drop() -> Result<(), FortressError> {
    let short_timeout = Duration::from_millis(200);
    let MultiHandleSessions {
        mut sess_a,
        sess_b,
        addr_b,
        clock,
    } = build_multi_handle_sessions(DisconnectBehavior::ContinueWithout, Some(short_timeout))?;

    // Stop polling sess_b (consume it so it goes silent), then advance sess_a
    // past the timeout.
    drop(sess_b);
    for _ in 0..100 {
        sess_a.poll_remote_clients();
        clock.advance(Duration::from_millis(20));
    }

    let events: Vec<_> = sess_a.events().collect();
    let dropped_handles: Vec<PlayerHandle> = events
        .iter()
        .filter_map(|e| match e {
            FortressEvent::PeerDropped { handle, addr } => {
                assert_eq!(
                    *addr, addr_b,
                    "PeerDropped addr must match dropped endpoint"
                );
                Some(*handle)
            },
            _ => None,
        })
        .collect();

    // Both handles share the same address; auto-drop must emit both.
    assert!(
        dropped_handles.contains(&PlayerHandle::new(1))
            && dropped_handles.contains(&PlayerHandle::new(2)),
        "auto-drop on multi-handle endpoint must emit PeerDropped for both handle 1 and handle 2; \
         got {:?}",
        dropped_handles
    );

    // Exactly one address-level Disconnected event for the endpoint.
    let disconnected_count = events
        .iter()
        .filter(|e| matches!(e, FortressEvent::Disconnected { .. }))
        .count();
    assert_eq!(
        disconnected_count, 1,
        "expected exactly one Disconnected event for the multi-handle endpoint; got {:?}",
        events
    );

    Ok(())
}

/// Verifies the documented event ordering: all `PeerDropped` for an endpoint
/// come before that endpoint's `Disconnected`, in the same `events()` batch.
/// (The relative order of `PeerDropped` events for distinct handles at the
/// same address is intentionally not constrained.)
#[test]
fn p2p_remove_player_multi_handle_emits_both_peer_dropped_then_disconnected(
) -> Result<(), FortressError> {
    let MultiHandleSessions {
        mut sess_a,
        mut sess_b,
        addr_b: _addr_b,
        clock,
    } = build_multi_handle_sessions(DisconnectBehavior::ContinueWithout, None)?;

    let mut stub_a = GameStub::new();
    let mut stub_b = GameStub::new();

    // Warmup so the endpoint has confirmed inputs for both handles.
    for i in 0..2_u32 {
        poll_with_advance(&mut sess_a, &mut sess_b, &clock, 3);
        sess_a
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess_b
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i + 10 })
            .unwrap();
        sess_b
            .add_local_input(PlayerHandle::new(2), StubInput { inp: i + 20 })
            .unwrap();
        let r_a = sess_a.advance_frame().unwrap();
        let r_b = sess_b.advance_frame().unwrap();
        stub_a.handle_requests(r_a);
        stub_b.handle_requests(r_b);
    }

    // Drop ONE handle of the multi-handle endpoint; the fix drops both.
    sess_a.remove_player(PlayerHandle::new(1)).unwrap();

    // Capture the batch.
    let events: Vec<_> = sess_a.events().collect();

    // Find indices of all PeerDropped events and the Disconnected event.
    let peer_dropped_indices: Vec<usize> = events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| matches!(e, FortressEvent::PeerDropped { .. }).then_some(i))
        .collect();
    let disconnected_index = events
        .iter()
        .position(|e| matches!(e, FortressEvent::Disconnected { .. }));

    assert_eq!(
        peer_dropped_indices.len(),
        2,
        "expected exactly two PeerDropped events for the multi-handle endpoint; got {:?}",
        events
    );
    let disc_idx = disconnected_index.expect("Disconnected event must be present in batch");
    for &pd_idx in &peer_dropped_indices {
        assert!(
            pd_idx < disc_idx,
            "every PeerDropped must precede the address-level Disconnected in the same batch; \
             pd_idx={}, disc_idx={}, events={:?}",
            pd_idx,
            disc_idx,
            events
        );
    }

    Ok(())
}

// ============================================================================
// Regression: under-loss graceful-drop desync (Chunk N0)
// ============================================================================
//
// In an N>=3 full-mesh `ContinueWithout` session, when a peer drops under
// *asymmetric* packet loss, survivors can have received the dropped peer's
// inputs through DIFFERENT frames (per-link delivery; a now-terminal endpoint
// never re-supplies them). Before the fix, each survivor froze the dropped slot
// at its OWN last-received value, so a survivor that received more of the dropped
// peer's frames repeated a different value than a survivor that received fewer —
// divergent confirmed history = silent desync. The fix rolls every survivor's
// frozen value back to the value at the globally-agreed freeze frame `F` (the
// global min over peers of the dropped slot's received frame), so all survivors
// repeat the IDENTICAL value.
//
// This is a deterministic asymmetric-loss repro: P3 keeps delivering to P1 but
// is blocked to P2 for several frames (so P1 confirms P3 through a higher frame
// than P2), then P3 goes fully silent and both survivors auto-drop it via the
// disconnect-timeout path through `update_player_disconnects`. The assertion
// checks cross-peer byte-equality of recorded confirmed state for every frame
// both peers consider confirmed. It FAILS before the fix and PASSES after.

/// Builds three synchronized `ContinueWithout` sessions over filtered sockets
/// with a short disconnect timeout, returning the sessions, the shared
/// blocked-links handle, the three addresses, and the clock.
#[allow(clippy::type_complexity)]
fn build_filtered_three_player_sessions(
    disconnect_timeout: Duration,
) -> Result<
    (
        P2PSession<StubConfig>,
        P2PSession<StubConfig>,
        P2PSession<StubConfig>,
        BlockedLinks,
        SocketAddr,
        SocketAddr,
        SocketAddr,
        TestClock,
    ),
    FortressError,
> {
    let clock = TestClock::new();
    let (s1, s2, s3, a1, a2, a3, blocked) = create_filtered_channel_triple();
    let pc = protocol_config(&clock);

    let build = |local: PlayerHandle,
                 socket: FilterSocket,
                 remotes: [(PlayerHandle, SocketAddr); 2]|
     -> Result<P2PSession<StubConfig>, FortressError> {
        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(pc.clone())
            .with_num_players(3)?
            .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
            .with_disconnect_timeout(disconnect_timeout)
            .with_disconnect_notify_delay(Duration::from_millis(100))
            .add_player(PlayerType::Local, local)?;
        for (handle, addr) in remotes {
            builder = builder.add_player(PlayerType::Remote(addr), handle)?;
        }
        builder.start_p2p_session(socket)
    };

    let mut sess1 = build(
        PlayerHandle::new(0),
        s1,
        [(PlayerHandle::new(1), a2), (PlayerHandle::new(2), a3)],
    )?;
    let mut sess2 = build(
        PlayerHandle::new(1),
        s2,
        [(PlayerHandle::new(0), a1), (PlayerHandle::new(2), a3)],
    )?;
    let mut sess3 = build(
        PlayerHandle::new(2),
        s3,
        [(PlayerHandle::new(0), a1), (PlayerHandle::new(1), a2)],
    )?;

    synchronize_three_sessions(&mut sess1, &mut sess2, &mut sess3, &clock, 500);
    let _ = drain_events(&mut sess1);
    let _ = drain_events(&mut sess2);
    let _ = drain_events(&mut sess3);

    Ok((sess1, sess2, sess3, blocked, a1, a2, a3, clock))
}

/// Advances a session one frame with the given local input, recording confirmed
/// state into `states` (via `handle_requests_recording`, which captures every
/// re-simulated frame). Tolerates `PredictionThreshold`/`NotSynchronized`
/// (returns `false` so the caller can poll and retry); propagates other errors.
fn try_advance_recording(
    session: &mut P2PSession<StubConfig>,
    stub: &mut GameStub,
    handle: PlayerHandle,
    value: u32,
    states: &mut BTreeMap<i32, StateStub>,
) -> Result<bool, FortressError> {
    match session.add_local_input(handle, StubInput { inp: value }) {
        Ok(()) => {},
        Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {
            return Ok(false)
        },
        Err(other) => return Err(other),
    }
    match session.advance_frame() {
        Ok(requests) => {
            stub.handle_requests_recording(requests, states);
            Ok(true)
        },
        Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => Ok(false),
        Err(other) => Err(other),
    }
}

#[test]
fn p2p_continue_without_under_asymmetric_loss_freezes_dropped_peer_consistently(
) -> Result<(), FortressError> {
    // Guards the *freeze-time* leg of the agreement: under asymmetric loss the
    // auto-timeout drop freezes each survivor's dropped slot at the global-min
    // agreed frame `F` (via `freeze_at` seeded from the gossiped `last_frame`
    // override), so survivors that received different amounts of P3's input still
    // repeat the identical value. (The complementary `remove_player` test below
    // exercises the re-roll *convergence* chokepoint in `disconnect_player_at_frames`
    // — the path that corrects a survivor which first froze "high" on local
    // detection.) Short timeout keeps the test fast; notify delay is even shorter
    // so the protocol starts the disconnect sequence promptly once P3 goes silent.
    let (mut sess1, mut sess2, mut sess3, blocked, a1, a2, a3, clock) =
        build_filtered_three_player_sessions(Duration::from_millis(400))?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    let mut states1: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states2: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // --- Phase 1: warmup with all links open so all three confirm together. ---
    let warmup_frames = 8_u32;
    for i in 0..warmup_frames {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            i,
            &mut states1,
        )?;
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1000,
            &mut states2,
        )?;
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            i + 2000,
            &mut sink,
        )?;
    }
    // Let confirmed inputs settle so the warmup frames are mutually confirmed.
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 12);

    // --- Phase 2: asymmetric loss. Block ONLY P3 -> P2. P3 keeps delivering to
    // P1 and keeps producing DISTINCT local inputs (i + 3000), so P1 confirms P3
    // through a higher frame than P2 does. Each distinct value makes the dropped
    // slot's frozen value frame-sensitive, so divergent freeze frames surface as
    // divergent recorded state. ---
    blocked.block(a3, a2);

    let loss_window = 4_u32;
    for i in 0..loss_window {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            i + 20,
            &mut states1,
        )?;
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1020,
            &mut states2,
        )?;
        // P3 keeps advancing locally with distinct values; only P1 receives them.
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            i + 3000,
            &mut sink,
        )?;
    }
    // Drain deliveries: P1 absorbs P3's extra frames; P2 does not.
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 6);

    // --- Phase 3: P3 goes fully silent (block P3 -> everyone, stop advancing
    // P3). Pump P1 + P2 + advance the clock past the disconnect timeout so both
    // auto-drop P3 via the ContinueWithout timeout path. ---
    blocked.block(a3, a1);

    let mut sess1_dropped = false;
    let mut sess2_dropped = false;
    for _ in 0..80 {
        // Poll P1 and P2 only (P3 is gone). Advance the clock so the disconnect
        // timeout fires.
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            500,
            &mut states1,
        )?;
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            1500,
            &mut states2,
        )?;

        if sess1
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            sess1_dropped = true;
        }
        if sess2
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            sess2_dropped = true;
        }
    }

    // Both survivors must have actually dropped P3 and advanced past the drop.
    assert!(
        sess1_dropped,
        "sess1 must emit PeerDropped for the timed-out peer"
    );
    assert!(
        sess2_dropped,
        "sess2 must emit PeerDropped for the timed-out peer"
    );
    assert!(
        sess1.confirmed_frame().as_i32() > warmup_frames as i32,
        "sess1 confirmed_frame must advance past the drop; got {:?}",
        sess1.confirmed_frame()
    );
    assert!(
        sess2.confirmed_frame().as_i32() > warmup_frames as i32,
        "sess2 confirmed_frame must advance past the drop; got {:?}",
        sess2.confirmed_frame()
    );

    // --- The desync check: every frame both peers consider confirmed (and that
    // both recorded) must have byte-equal recorded state. Pre-fix this FAILS
    // because the survivors froze the dropped slot at divergent values across the
    // asymmetric-loss window. ---
    let confirmed_bound = std::cmp::min(
        sess1.confirmed_frame().as_i32(),
        sess2.confirmed_frame().as_i32(),
    );
    let mut compared = 0_u32;
    let mut divergences: Vec<(i32, StateStub, StateStub)> = Vec::new();
    for (&frame, &state1) in &states1 {
        if frame > confirmed_bound {
            continue;
        }
        if let Some(&state2) = states2.get(&frame) {
            compared += 1;
            if state1 != state2 {
                divergences.push((frame, state1, state2));
            }
        }
    }

    assert!(
        compared > 0,
        "no confirmed frames were compared across both peers (bound={confirmed_bound}); \
         the repro did not exercise the drop path"
    );
    assert!(
        divergences.is_empty(),
        "confirmed state diverged across survivors after under-loss graceful drop \
         (bound={confirmed_bound}, compared={compared}): {divergences:?}"
    );

    Ok(())
}

// ============================================================================
// Regression (direct-detection path): `remove_player` under asymmetric loss
// ============================================================================
//
// This complements the timeout/gossip repro above. Here both survivors take the
// DIRECT-DETECTION path: each explicitly `remove_player(P3)` while P1 has
// confirmed P3 through a HIGHER frame than P2 (asymmetric loss blocked P3 -> P2
// for a window). On the direct path `remove_player` passes `last_frame_overrides
// = None`, so each survivor's *initial* freeze frame is its OWN local received
// frame — which differs across survivors. Only after gossip propagates through
// `update_player_disconnects` does each survivor converge its
// `local_connect_status[2].last_frame` DOWN to the global-min agreed frame `F`.
//
// Before the frozen-value re-roll fix, that convergence lowers `status.last_frame`
// but the already-frozen queue's repeated value is NEVER corrected (the re-adjust
// path runs with `event_policy == Suppress`, skipping the Emit-gated freeze, and
// `freeze_at` is idempotent). So the survivor that froze "high" keeps repeating a
// different value than the survivor that froze "low" -> divergent confirmed
// history. After the fix, `disconnect_player_at_frames` re-rolls the frozen value
// to track `F` down on EVERY path, so both survivors repeat the identical value.
//
// This test FAILS before the re-roll change and PASSES after.
#[test]
fn p2p_remove_player_under_asymmetric_loss_freezes_dropped_peer_consistently(
) -> Result<(), FortressError> {
    // Generous timeout so the auto-drop timeout does not fire before we
    // explicitly `remove_player`: this test exercises the EXPLICIT
    // `remove_player` direct-detection path, not the timeout path. The explicit
    // removal in Phase 3 happens well within this window.
    let (mut sess1, mut sess2, mut sess3, blocked, _a1, a2, a3, clock) =
        build_filtered_three_player_sessions(Duration::from_secs(2))?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    let mut states1: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states2: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // --- Phase 1: warmup, all links open so all three confirm together. ---
    let warmup_frames = 8_u32;
    for i in 0..warmup_frames {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            i,
            &mut states1,
        )?;
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1000,
            &mut states2,
        )?;
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            i + 2000,
            &mut sink,
        )?;
    }
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 12);

    // --- Phase 2: asymmetric loss. Block ONLY P3 -> P2 with DISTINCT P3 inputs,
    // so P1 *receives* P3 through a higher frame than P2 does. We keep the window
    // SMALL and DO NOT let mutual confirmation advance past the lower (P2) frame:
    // confirmation requires every peer's acks, and P3 -> P2 is blocked, so the
    // mesh-confirmed frame for the P3 slot stays at P2's lower value. The
    // divergence we want is in each survivor's *locally received* frame
    // (`local_connect_status[2].last_frame`), which differs across survivors and
    // drives the initial freeze on the direct `remove_player` path. ---
    blocked.block(a3, a2);

    // Keep the asymmetry to a SINGLE frame: P1 receives exactly one more P3
    // frame than P2. A larger gap would let P1 confirm + discard frames below
    // the eventual global-min `F`, so the post-drop rollback could no longer
    // reach `F` — the genuine (and separately documented) TOCTOU limitation,
    // not the bug under test. A one-frame gap keeps `F` within the un-discarded
    // window so the re-roll + rollback can actually converge.
    let loss_window = 1_u32;
    for i in 0..loss_window {
        // Poll only ONCE per frame and DO NOT drain afterward, so neither
        // survivor confirms past the asymmetric window (which would discard the
        // very inputs the post-drop rollback needs).
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 1);
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            i + 20,
            &mut states1,
        )?;
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1020,
            &mut states2,
        )?;
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            i + 3000,
            &mut sink,
        )?;
    }
    // One light poll so P1 absorbs P3's in-flight extra frames (P3 -> P1 open),
    // but NOT enough to advance mutual confirmation past P2's stalled P3 frame.
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 1);

    // --- Phase 3: BOTH survivors explicitly `remove_player(P3)`. P1 received P3
    // through a higher frame than P2 (P3 -> P2 is still blocked), so each
    // survivor freezes at its OWN (divergent) local received frame. ---
    sess1.remove_player(PlayerHandle::new(2)).unwrap();
    sess2.remove_player(PlayerHandle::new(2)).unwrap();

    // --- Phase 4: pump P1 + P2 so gossip propagates through
    // `update_player_disconnects`, converging each survivor's agreed frame DOWN
    // to the global min and (post-fix) re-rolling the frozen value with it. ---
    for _ in 0..80 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            500,
            &mut states1,
        )?;
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            1500,
            &mut states2,
        )?;
    }

    assert!(
        sess1.confirmed_frame().as_i32() > warmup_frames as i32,
        "sess1 confirmed_frame must advance past the drop; got {:?}",
        sess1.confirmed_frame()
    );
    assert!(
        sess2.confirmed_frame().as_i32() > warmup_frames as i32,
        "sess2 confirmed_frame must advance past the drop; got {:?}",
        sess2.confirmed_frame()
    );

    // --- The desync check: every frame both peers consider confirmed (and that
    // both recorded) must have byte-equal recorded state. ---
    let confirmed_bound = std::cmp::min(
        sess1.confirmed_frame().as_i32(),
        sess2.confirmed_frame().as_i32(),
    );
    let mut compared = 0_u32;
    let mut divergences: Vec<(i32, StateStub, StateStub)> = Vec::new();
    for (&frame, &state1) in &states1 {
        if frame > confirmed_bound {
            continue;
        }
        if let Some(&state2) = states2.get(&frame) {
            compared += 1;
            if state1 != state2 {
                divergences.push((frame, state1, state2));
            }
        }
    }

    assert!(
        compared > 0,
        "no confirmed frames were compared across both peers (bound={confirmed_bound}); \
         the repro did not exercise the drop path"
    );
    assert!(
        divergences.is_empty(),
        "confirmed state diverged across survivors after under-loss remove_player drop \
         (bound={confirmed_bound}, compared={compared}): {divergences:?}"
    );

    Ok(())
}
