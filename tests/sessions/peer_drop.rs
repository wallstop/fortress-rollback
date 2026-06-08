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
    create_channel_pair, create_channel_quad, create_channel_triple, create_filtered_channel_quad,
    create_filtered_channel_triple, drain_sync_events, poll_with_advance,
    synchronize_sessions_deterministic, BlockedLinks, BusSocket, FilterSocket, RoutingBus,
    SyncConfig, TestClock, POLL_INTERVAL_DETERMINISTIC,
};
use fortress_rollback::{
    telemetry::{CollectingObserver, ViolationSeverity},
    DesyncDetection, DisconnectBehavior, FortressError, FortressEvent, FortressRequest, Frame,
    InputStatus, InputVec, P2PSession, PlayerHandle, PlayerType, ProtocolConfig, SaveMode,
    SessionBuilder, SessionState, SpectatorSession,
};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
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
// Regression (relay-clobber, N=4): audit finding F4 — a relayed lowering of a
// dropped slot's freeze frame must NOT be clobbered by a survivor's own stale
// monotone-`max` view, or two survivors freeze the dropped slot at different
// frames -> divergent confirmed history (silent desync).
// ============================================================================
//
// This generalizes the 3-player asymmetric-loss machinery above to FOUR peers,
// which is the minimum to exhibit a *relay*-clobber: peers A(0), B(1), C(2),
// D(3) in a `ContinueWithout` full mesh; D is the dropped slot, A/B/C are the
// survivors.
//
// Why 3 survivors are required (why the 3-player tests above CANNOT manifest it):
// in a 3-node mesh there is exactly one dropped peer and two survivors, and the
// two survivors are joined by a DIRECT link. When D's lowest-view survivor
// gossips its freeze frame, that gossip travels the direct survivor<->survivor
// link, which always carries truth — there is no third party whose stale, higher
// view sits between them to clobber the lowering. With THREE survivors we can
// additionally sever the direct A<->C link, forcing C's lower view of D to reach
// A only by transiting the relay B. The lowered freeze frame then arrives at A
// inside a packet whose OTHER `peer_connect_status` entries reflect B's (or A's
// own) higher knowledge, so a naive monotone-`max` merge of the D slot would
// re-raise (clobber) the relayed lowering. That is exactly the F4 bug:
// pre-fix, `merge_peer_connect_status` took `max` for ALL slots, so A's D-slot
// freeze frame snapped back UP to A's own (higher) received frame after the
// relayed lowering, while C stayed frozen LOW -> A and C repeat different D
// values across the freeze window. The fix converges a DISCONNECTED slot's
// `last_frame` DOWNWARD. The branch this test exercises is the FIRST-ADOPT
// down-step: when A first learns (from B's relayed gossip) that D is
// disconnected, it ADOPTS B's lower freeze frame instead of keeping/maxing its
// own higher received frame; combined with the session-layer
// `update_player_disconnects` min over running endpoints, the relayed lowering
// survives and every survivor freezes D at the global-min `F`. (The sibling
// both-disconnected `min` branch — which rejects a later stale HIGHER re-gossip —
// is a distinct guard covered by the `on_input_disconnected_slot_ignores_stale_
// higher_freeze_gossip` unit test, and is not the branch under test here.)
//
// Deterministic engineering of A>B>C reception of D (see the test body for the
// exact frame numbers):
//   1. Warm up all four in lockstep with DISTINCT per-peer inputs (D's stream is
//      non-constant, so a frozen D value is frame-sensitive — a constant stream
//      would make every freeze frame look byte-equal and render the test vacuous).
//   2. Block D->C first and advance: D keeps delivering to A and B, so C's
//      `local_connect_status[D].last_frame` falls behind.
//   3. Then ALSO block D->B and advance more: now D delivers only to A, so when D
//      goes silent A has received the MOST of D, B the MIDDLE, C the LEAST.
//   4. Block the A<->C link in BOTH directions, so C's low view of D can never
//      reach A on the direct edge — it must transit the relay B. (Once A and C
//      drop each other below, each EXCLUDES the other's endpoint from its per-slot
//      disconnect minimum, which is what denies A any direct source of C's low D.)
//   5. Make D fully silent and have every survivor `remove_player(D)` (and A/C
//      `remove_player` each other). Pumping poll + advance lets C's low D gossip
//      reach B, B lower its D-slot freeze frame to the global min `F`, and B
//      RE-GOSSIP that lowered, disconnected value onward to A, converging all
//      three. We use `remove_player` rather than the auto-timeout path so the drop
//      is instant (confirmation can't discard the freeze-window frames the
//      post-drop rollback must re-simulate) and so the A<->C *timeout* never fires.
//
// Oracle (must hold on FIXED code): over the shared confirmed-frame range, the
// recorded confirmed game state is byte-identical across A and C (and B). Pre-fix
// the D freeze window diverges (A high, C low). This test PASSES on current code
// and goes RED if the down-convergence in `merge_peer_connect_status` is
// neutralized to plain `max` (proven separately).

/// Builds four synchronized `ContinueWithout` sessions over filtered sockets with
/// a short disconnect timeout, returning the four sessions, the shared
/// blocked-links handle, the four addresses, and the clock. This is the 4-player
/// analog of [`build_filtered_three_player_sessions`].
#[allow(clippy::type_complexity)]
fn build_filtered_four_player_sessions(
    disconnect_timeout: Duration,
) -> Result<
    (
        P2PSession<StubConfig>,
        P2PSession<StubConfig>,
        P2PSession<StubConfig>,
        P2PSession<StubConfig>,
        BlockedLinks,
        SocketAddr,
        SocketAddr,
        SocketAddr,
        SocketAddr,
        TestClock,
    ),
    FortressError,
> {
    let clock = TestClock::new();
    let (s1, s2, s3, s4, a1, a2, a3, a4, blocked) = create_filtered_channel_quad();
    let pc = protocol_config(&clock);

    let build = |local: PlayerHandle,
                 socket: FilterSocket,
                 remotes: [(PlayerHandle, SocketAddr); 3]|
     -> Result<P2PSession<StubConfig>, FortressError> {
        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(pc.clone())
            .with_num_players(4)?
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
        [
            (PlayerHandle::new(1), a2),
            (PlayerHandle::new(2), a3),
            (PlayerHandle::new(3), a4),
        ],
    )?;
    let mut sess2 = build(
        PlayerHandle::new(1),
        s2,
        [
            (PlayerHandle::new(0), a1),
            (PlayerHandle::new(2), a3),
            (PlayerHandle::new(3), a4),
        ],
    )?;
    let mut sess3 = build(
        PlayerHandle::new(2),
        s3,
        [
            (PlayerHandle::new(0), a1),
            (PlayerHandle::new(1), a2),
            (PlayerHandle::new(3), a4),
        ],
    )?;
    let mut sess4 = build(
        PlayerHandle::new(3),
        s4,
        [
            (PlayerHandle::new(0), a1),
            (PlayerHandle::new(1), a2),
            (PlayerHandle::new(2), a3),
        ],
    )?;

    synchronize_four_sessions(&mut sess1, &mut sess2, &mut sess3, &mut sess4, &clock, 500);
    let _ = drain_events(&mut sess1);
    let _ = drain_events(&mut sess2);
    let _ = drain_events(&mut sess3);
    let _ = drain_events(&mut sess4);

    Ok((sess1, sess2, sess3, sess4, blocked, a1, a2, a3, a4, clock))
}

/// Synchronizes four sessions deterministically. Returns when all four are in
/// `Running` state, or panics if synchronization does not complete in
/// `iterations` iterations. The 4-player analog of [`synchronize_three_sessions`].
fn synchronize_four_sessions(
    sess1: &mut P2PSession<StubConfig>,
    sess2: &mut P2PSession<StubConfig>,
    sess3: &mut P2PSession<StubConfig>,
    sess4: &mut P2PSession<StubConfig>,
    clock: &TestClock,
    iterations: usize,
) {
    for _ in 0..iterations {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        sess4.poll_remote_clients();
        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
            && sess3.current_state() == SessionState::Running
            && sess4.current_state() == SessionState::Running
        {
            return;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    panic!(
        "Four sessions failed to synchronize: sess1={:?}, sess2={:?}, sess3={:?}, sess4={:?}",
        sess1.current_state(),
        sess2.current_state(),
        sess3.current_state(),
        sess4.current_state()
    );
}

/// Polls four sessions and advances virtual time by
/// `POLL_INTERVAL_DETERMINISTIC * iterations`. The 4-player analog of
/// [`poll_three`].
fn poll_four(
    sess1: &mut P2PSession<StubConfig>,
    sess2: &mut P2PSession<StubConfig>,
    sess3: &mut P2PSession<StubConfig>,
    sess4: &mut P2PSession<StubConfig>,
    clock: &TestClock,
    iterations: usize,
) {
    for _ in 0..iterations {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        sess4.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
}

#[test]
fn p2p_n4_relay_clobber_dropped_slot_freeze_frame_converges_across_survivors(
) -> Result<(), FortressError> {
    // See the block comment above for the full F4 mechanism, the relay
    // requirement, and why N=4 + a severed A<->C link is necessary. In short:
    // A/B/C survive, D is dropped; A receives D's inputs through the HIGHEST
    // frame, B a MIDDLE frame, C the LOWEST (each gap one frame); the A<->C link
    // is severed so C's low view of D reaches A only via the relay B; every
    // survivor then drops D (and A/C drop each other) via `remove_player`, and we
    // pump gossip so B's lowered, disconnected D freeze frame relays to A.
    // Post-fix every survivor freezes D at the global-min frame `F`, so the
    // recorded confirmed state is byte-identical across survivors. Pre-fix A's
    // D-slot freeze frame is clobbered back UP by a stale monotone-`max` merge of
    // the relayed lowering, diverging from B and C across the freeze window (this
    // very test reds with the down-convergence in `merge_peer_connect_status`
    // neutralized to plain `max`).
    //
    // We use the explicit `remove_player` direct-detection path (not auto-timeout)
    // for two reasons: (1) it drives the drop instantly, so confirmation cannot run
    // 400ms / dozens of frames past the freeze frame `F` and discard the states the
    // post-drop rollback must re-simulate; (2) it lets A and C take each other off
    // without the A<->C *timeout* also firing, keeping the severance purely a
    // gossip-routing constraint. This mirrors the 3-player
    // `p2p_remove_player_under_asymmetric_loss_*` test's pacing discipline.
    let (
        mut sess_a,
        mut sess_b,
        mut sess_c,
        mut sess_d,
        blocked,
        a_addr,
        b_addr,
        c_addr,
        d_addr,
        clock,
    ) = build_filtered_four_player_sessions(Duration::from_millis(400))?;

    let mut stub_a = GameStub::new();
    let mut stub_b = GameStub::new();
    let mut stub_c = GameStub::new();
    let mut stub_d = GameStub::new();

    let mut states_a: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states_b: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states_c: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // --- Phase 1: warmup with all links open so all four confirm together. Each
    // peer feeds a DISTINCT deterministic input stream; crucially D's stream
    // (i + 3000) is non-constant, so a frozen D value is frame-sensitive. ---
    let warmup_frames = 8_u32;
    for i in 0..warmup_frames {
        poll_four(
            &mut sess_a,
            &mut sess_b,
            &mut sess_c,
            &mut sess_d,
            &clock,
            3,
        );
        try_advance_recording(
            &mut sess_a,
            &mut stub_a,
            PlayerHandle::new(0),
            i,
            &mut states_a,
        )?;
        try_advance_recording(
            &mut sess_b,
            &mut stub_b,
            PlayerHandle::new(1),
            i + 1000,
            &mut states_b,
        )?;
        try_advance_recording(
            &mut sess_c,
            &mut stub_c,
            PlayerHandle::new(2),
            i + 2000,
            &mut states_c,
        )?;
        try_advance_recording(
            &mut sess_d,
            &mut stub_d,
            PlayerHandle::new(3),
            i + 3000,
            &mut sink,
        )?;
    }
    // Let confirmed inputs settle so the warmup frames are mutually confirmed.
    poll_four(
        &mut sess_a,
        &mut sess_b,
        &mut sess_c,
        &mut sess_d,
        &clock,
        12,
    );

    // --- Phase 2a: block ONLY D -> C, then advance EXACTLY ONE frame. D delivers
    // its DISTINCT input to A and B but not C, so C's locally-received view of D
    // (`local_connect_status[D].last_frame`) falls ONE frame behind A's and B's.
    // We deliberately keep every asymmetry gap to a SINGLE frame and DO NOT drain
    // afterward: a larger gap (or draining) would let a survivor confirm and
    // discard frames below the eventual global-min freeze `F`, so the post-drop
    // re-roll/rollback could no longer reach `F` — the separately documented
    // discard-before-convergence limitation, not the relay-clobber under test. ---
    blocked.block(d_addr, c_addr);
    poll_four(
        &mut sess_a,
        &mut sess_b,
        &mut sess_c,
        &mut sess_d,
        &clock,
        1,
    );
    try_advance_recording(
        &mut sess_a,
        &mut stub_a,
        PlayerHandle::new(0),
        20,
        &mut states_a,
    )?;
    try_advance_recording(
        &mut sess_b,
        &mut stub_b,
        PlayerHandle::new(1),
        1020,
        &mut states_b,
    )?;
    try_advance_recording(
        &mut sess_c,
        &mut stub_c,
        PlayerHandle::new(2),
        2020,
        &mut states_c,
    )?;
    try_advance_recording(
        &mut sess_d,
        &mut stub_d,
        PlayerHandle::new(3),
        3020,
        &mut sink,
    )?;
    poll_four(
        &mut sess_a,
        &mut sess_b,
        &mut sess_c,
        &mut sess_d,
        &clock,
        1,
    );

    // --- Phase 2b: ALSO block D -> B, then advance EXACTLY ONE more frame. Now D
    // delivers only to A, so B's received view of D stalls ONE frame above C's and
    // ONE frame below A's. Net result: A's received D frame > B's > C's, each by a
    // single frame, with the global-min freeze frame `F` = C's frame. ---
    blocked.block(d_addr, b_addr);
    poll_four(
        &mut sess_a,
        &mut sess_b,
        &mut sess_c,
        &mut sess_d,
        &clock,
        1,
    );
    try_advance_recording(
        &mut sess_a,
        &mut stub_a,
        PlayerHandle::new(0),
        40,
        &mut states_a,
    )?;
    try_advance_recording(
        &mut sess_b,
        &mut stub_b,
        PlayerHandle::new(1),
        1040,
        &mut states_b,
    )?;
    try_advance_recording(
        &mut sess_c,
        &mut stub_c,
        PlayerHandle::new(2),
        2040,
        &mut states_c,
    )?;
    // D keeps advancing locally with a distinct value; only A receives it now.
    try_advance_recording(
        &mut sess_d,
        &mut stub_d,
        PlayerHandle::new(3),
        3040,
        &mut sink,
    )?;
    // One light poll so A absorbs D's in-flight extra frame (D -> A still open),
    // but NOT enough to advance mutual confirmation past the asymmetric window.
    poll_four(
        &mut sess_a,
        &mut sess_b,
        &mut sess_c,
        &mut sess_d,
        &clock,
        1,
    );

    // --- Phase 3: sever the A<->C link in BOTH directions. This is the crux that
    // makes the desync a *relay*-clobber (and the reason ≥3 survivors are
    // required). After A and C drop each other (below), each EXCLUDES the other's
    // endpoint from its per-slot disconnect minimum, so C's low view of D can
    // reach A ONLY relayed through B. Because we drive the drop immediately via
    // `remove_player` (no clock pump), the A<->C timeout never fires on its own —
    // the explicit removals below are what take A and C off each other. ---
    blocked.block(a_addr, c_addr);
    blocked.block(c_addr, a_addr);

    // --- Phase 4: every survivor drops D (and A/C drop each other, since their
    // link is now severed) via explicit `remove_player`. On this direct path each
    // survivor's INITIAL D freeze frame is its OWN received frame (A high, B
    // middle, C low). Then we pump poll + advance so gossip propagates through
    // `update_player_disconnects`: C (low) -> B lowers B's D freeze to the global
    // min `F`, and B RE-GOSSIPS that lowered, DISCONNECTED freeze frame onward to
    // A. A's only surviving source of D's truth is that relayed value (A<->C
    // severed, D silent), so the merge of A's stale high D view with B's relayed
    // low one decides A's freeze frame: down-converging (A adopts B's lower relayed
    // freeze frame on first learning D is disconnected) keeps `F` (fixed);
    // monotone-`max` clobbers it back up (the F4 bug). ---
    blocked.block(d_addr, a_addr);
    blocked.block(d_addr, b_addr);
    blocked.block(d_addr, c_addr);

    sess_a.remove_player(PlayerHandle::new(3)).unwrap();
    sess_b.remove_player(PlayerHandle::new(3)).unwrap();
    sess_c.remove_player(PlayerHandle::new(3)).unwrap();
    sess_a.remove_player(PlayerHandle::new(2)).unwrap();
    sess_c.remove_player(PlayerHandle::new(0)).unwrap();

    let mut a_dropped = false;
    let mut b_dropped = false;
    let mut c_dropped = false;
    for _ in 0..120 {
        sess_a.poll_remote_clients();
        sess_b.poll_remote_clients();
        sess_c.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        try_advance_recording(
            &mut sess_a,
            &mut stub_a,
            PlayerHandle::new(0),
            500,
            &mut states_a,
        )?;
        try_advance_recording(
            &mut sess_b,
            &mut stub_b,
            PlayerHandle::new(1),
            1500,
            &mut states_b,
        )?;
        try_advance_recording(
            &mut sess_c,
            &mut stub_c,
            PlayerHandle::new(2),
            2500,
            &mut states_c,
        )?;

        if sess_a
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            a_dropped = true;
        }
        if sess_b
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            b_dropped = true;
        }
        if sess_c
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            c_dropped = true;
        }
    }

    // All three survivors must have actually dropped D and advanced past the drop.
    assert!(
        a_dropped,
        "sess_a must emit PeerDropped for the dropped peer D"
    );
    assert!(
        b_dropped,
        "sess_b must emit PeerDropped for the dropped peer D"
    );
    assert!(
        c_dropped,
        "sess_c must emit PeerDropped for the dropped peer D"
    );
    assert!(
        sess_a.confirmed_frame().as_i32() > warmup_frames as i32,
        "sess_a confirmed_frame must advance past the drop; got {:?}",
        sess_a.confirmed_frame()
    );
    assert!(
        sess_b.confirmed_frame().as_i32() > warmup_frames as i32,
        "sess_b confirmed_frame must advance past the drop; got {:?}",
        sess_b.confirmed_frame()
    );
    assert!(
        sess_c.confirmed_frame().as_i32() > warmup_frames as i32,
        "sess_c confirmed_frame must advance past the drop; got {:?}",
        sess_c.confirmed_frame()
    );

    // --- The desync check (primary oracle): over the shared confirmed-frame range
    // every frame all three survivors recorded must have byte-equal recorded
    // state. The freeze window for D (frames between the global-min freeze `F` and
    // A's higher received frame) is the region that diverges pre-fix: A repeats
    // D's high-frame value there while C repeats D's low (global-min) value. The
    // relayed lowering must converge A down to C's value for these to match. ---
    let confirmed_bound = sess_a
        .confirmed_frame()
        .as_i32()
        .min(sess_b.confirmed_frame().as_i32())
        .min(sess_c.confirmed_frame().as_i32());
    let mut compared = 0_u32;
    let mut divergences: Vec<(i32, &'static str, StateStub, StateStub)> = Vec::new();
    for (&frame, &state_a) in &states_a {
        if frame > confirmed_bound {
            continue;
        }
        // A vs C is the relay-clobber pair (their direct link was severed, so C's
        // lowering reached A only via B). A vs B catches any residual divergence.
        if let Some(&state_c) = states_c.get(&frame) {
            compared += 1;
            if state_a != state_c {
                divergences.push((frame, "A!=C", state_a, state_c));
            }
        }
        if let Some(&state_b) = states_b.get(&frame) {
            if state_a != state_b {
                divergences.push((frame, "A!=B", state_a, state_b));
            }
        }
    }

    assert!(
        compared > 0,
        "no confirmed frames were compared across survivors (bound={confirmed_bound}); \
         the relay-clobber repro did not exercise the drop path"
    );
    assert!(
        divergences.is_empty(),
        "confirmed state diverged across survivors after N=4 relay-clobber drop \
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

// ============================================================================
// Regression (liveness): audit finding F8 — a gossip-lowered disconnect frame
// that falls OUTSIDE the live prediction window must not permanently stall
// `advance_frame`.
// ============================================================================
//
// Mechanism reproduced here (3 peers A/B/C, `ContinueWithout`, small
// `max_prediction`): under asymmetric loss A receives C's input through a HIGHER
// frame than B does (C -> B is blocked). C then goes silent and both survivors
// auto-drop it. A first freezes C at its OWN (high) received frame, so A's
// initial `disconnect_frame` is in-window and A keeps advancing. Later B's gossip
// (C confirmed only through a LOW frame) propagates through
// `update_player_disconnects`, which mines A's agreed frame DOWN to the global
// min `F` and lowers A's `disconnect_frame` to `F + 1`. By then A has advanced
// well past `current_frame - max_prediction`, so the disconnect-induced rollback
// target sits BELOW the window floor.
//
// Before the fix, `adjust_gamestate` asked `load_frame` for that out-of-window
// frame, which returned `OutsidePredictionWindow`; the `?` propagated out of
// `advance_frame` BEFORE `disconnect_frame` was cleared, so every subsequent
// `advance_frame` recomputed the same out-of-window target and failed identically
// — a permanent stall (the probe for this scenario observed A frozen at one frame
// with 100% of `advance_frame` calls returning `OutsidePredictionWindow`).
//
// After the fix, `adjust_gamestate` clamps the load target UP to the window floor,
// `load_frame` succeeds, `disconnect_frame` is cleared, and A stays live. Frames
// below the floor remain unrecoverable (the separately documented
// discard-before-convergence residual); this test asserts LIVENESS only, not that
// those below-window frames are corrected.
//
// This test FAILS before the fix (A stalls, returning `OutsidePredictionWindow`)
// and PASSES after (A keeps advancing).
#[test]
fn p2p_continue_without_gossip_lowered_disconnect_outside_window_stays_live(
) -> Result<(), FortressError> {
    // Arrange: 3 `ContinueWithout` sessions over filtered sockets with a SMALL
    // prediction window so "outside the window" is easy to engineer, plus a
    // violation observer on the survivor under test (A) so we can prove the clamp
    // logs at most a Warning (never the genuine `frame_to_load > first_incorrect`
    // Error, and never an Error/Critical FrameSync violation).
    const MAX_PREDICTION: usize = 2;
    let clock = TestClock::new();
    let (s1, s2, s3, a1, a2, a3, blocked) = create_filtered_channel_triple();
    let pc = protocol_config(&clock);
    let observer = Arc::new(CollectingObserver::new());

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(3)?
        .with_max_prediction_window(MAX_PREDICTION)
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .with_disconnect_timeout(Duration::from_millis(400))
        .with_disconnect_notify_delay(Duration::from_millis(100))
        .with_violation_observer(observer.clone())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .start_p2p_session(s1)?;
    let build_remote = |local: PlayerHandle,
                        socket: FilterSocket,
                        remotes: [(PlayerHandle, SocketAddr); 2]|
     -> Result<P2PSession<StubConfig>, FortressError> {
        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(pc.clone())
            .with_num_players(3)?
            .with_max_prediction_window(MAX_PREDICTION)
            .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
            .with_disconnect_timeout(Duration::from_millis(400))
            .with_disconnect_notify_delay(Duration::from_millis(100))
            .add_player(PlayerType::Local, local)?;
        for (handle, addr) in remotes {
            builder = builder.add_player(PlayerType::Remote(addr), handle)?;
        }
        builder.start_p2p_session(socket)
    };
    let mut sess2 = build_remote(
        PlayerHandle::new(1),
        s2,
        [(PlayerHandle::new(0), a1), (PlayerHandle::new(2), a3)],
    )?;
    let mut sess3 = build_remote(
        PlayerHandle::new(2),
        s3,
        [(PlayerHandle::new(0), a1), (PlayerHandle::new(1), a2)],
    )?;
    synchronize_three_sessions(&mut sess1, &mut sess2, &mut sess3, &clock, 500);
    let _ = drain_events(&mut sess1);
    let _ = drain_events(&mut sess2);
    let _ = drain_events(&mut sess3);

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // --- Phase 1: warmup, all links open, all three confirm together. C emits a
    // CONSTANT input so that A receiving C's frames never causes a prediction-miss
    // rollback (default prediction repeats the last input), letting A's
    // `connect_status[C].last_frame` climb without churn. ---
    const C_CONST_INPUT: u32 = 7;
    for i in 0..4u32 {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        try_advance_recording(&mut sess1, &mut stub1, PlayerHandle::new(0), i, &mut sink)?;
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1000,
            &mut sink,
        )?;
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            C_CONST_INPUT,
            &mut sink,
        )?;
    }
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 8);

    // --- Phase 2: block ONLY C -> B. C keeps delivering its constant input to A,
    // so A's `connect_status[C].last_frame` climbs HIGHER than B's. This is the
    // asymmetry that makes the eventual gossip lower A's agreed frame (and
    // `disconnect_frame`) far below A's current frame. ---
    blocked.block(a3, a2);
    for _ in 0..12u32 {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 2);
        let _ = try_advance_recording(&mut sess1, &mut stub1, PlayerHandle::new(0), 50, &mut sink);
        let _ = try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            1050,
            &mut sink,
        );
        let _ = try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            C_CONST_INPUT,
            &mut sink,
        );
    }
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 4);

    // --- Phase 3: C goes fully silent. Pump A + B past the disconnect timeout so
    // both auto-drop C; B's low-frame gossip about C reaches A and mines A's
    // `disconnect_frame` down below A's prediction-window floor. ---
    blocked.block(a3, a1);
    let mut sess1_dropped = false;
    let frame_before_drive = sess1.current_frame();
    for _ in 0..60 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        // Tolerate only the normal throttle/sync errors here while the drop
        // converges; the assertions below drive the post-convergence behavior.
        let _ = try_advance_recording(&mut sess1, &mut stub1, PlayerHandle::new(0), 500, &mut sink);
        let _ = try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            1500,
            &mut sink,
        );
        if sess1
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            sess1_dropped = true;
        }
    }
    assert!(
        sess1_dropped,
        "sess1 must auto-drop the silent peer for this repro to exercise the gossip-lowered \
         disconnect path"
    );

    // Act + Assert (liveness): drive A forward and require every `advance_frame`
    // to make progress without ever returning `OutsidePredictionWindow`. Pre-fix,
    // the gossip-lowered out-of-window `disconnect_frame` makes EVERY call here
    // return `Err(InvalidFrameStructured { reason: OutsidePredictionWindow, .. })`
    // and `current_frame` never advances (permanent stall).
    let mut outside_window_errors = 0u32;
    let mut advanced_ok = 0u32;
    for _ in 0..40 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        // Keep B alive so A's confirmed frame (and therefore A's throttle) keeps
        // climbing, otherwise A would legitimately stop on the prediction window.
        let _ = try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            1500,
            &mut sink,
        );

        match sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: 500 }) {
            Ok(()) => {},
            // Normal throttle/sync backpressure — not the F8 stall.
            Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => continue,
            Err(other) => return Err(other),
        }
        match sess1.advance_frame() {
            Ok(requests) => {
                advanced_ok += 1;
                stub1.handle_requests(requests);
            },
            Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
            Err(FortressError::InvalidFrameStructured {
                reason: fortress_rollback::InvalidFrameReason::OutsidePredictionWindow { .. },
                ..
            }) => {
                outside_window_errors += 1;
            },
            Err(other) => return Err(other),
        }
    }

    assert_eq!(
        outside_window_errors, 0,
        "F8 regression: advance_frame returned OutsidePredictionWindow after a gossip-lowered \
         disconnect frame fell outside the prediction window — the session is permanently stalled"
    );
    assert!(
        advanced_ok > 0,
        "advance_frame never made progress after the drop; expected the survivor to stay live"
    );
    assert!(
        sess1.current_frame() > frame_before_drive,
        "survivor did not advance past where it was when the drop converged (before={:?}, \
         after={:?}) — it is stuck",
        frame_before_drive,
        sess1.current_frame()
    );

    // Assert (no spurious violation): the out-of-window clamp must NOT fire the
    // genuine `frame_to_load > first_incorrect` Error, and must not raise any
    // Error/Critical FrameSync violation. At most a Warning explaining the
    // gossip-lowered-disconnect-frame-outside-window residual is allowed.
    let violations = observer.violations();
    let genuine_bug_errors: Vec<_> = violations
        .iter()
        .filter(|v| v.message.contains("this indicates a bug"))
        .collect();
    assert!(
        genuine_bug_errors.is_empty(),
        "the legitimate out-of-window clamp must not trip the genuine sparse-mode \
         'frame_to_load > first_incorrect' Error: {genuine_bug_errors:?}"
    );
    let serious: Vec<_> = violations
        .iter()
        .filter(|v| {
            matches!(
                v.severity,
                ViolationSeverity::Error | ViolationSeverity::Critical
            )
        })
        .collect();
    assert!(
        serious.is_empty(),
        "no Error/Critical violations expected on the gossip-lowered out-of-window path; got: {serious:?}"
    );

    Ok(())
}

// ============================================================================
// Arbitration (contested audit finding F7): "Sparse saving skips re-simulation
// below a gossip-lowered disconnect frame (N>=3 staggered drop)".
// ============================================================================
//
// Scenario (3 peers A/B/C, `ContinueWithout`, `SaveMode::Sparse`): under
// asymmetric loss A receives C's DISTINCT inputs through a HIGHER frame than B
// does (C -> B blocked), so A's `connect_status[C].last_frame` and A's sole
// sparse-saved state both climb. C then goes silent; both survivors auto-drop
// it. B (which only ever confirmed C through a LOW frame) gossips that low frame
// to A. `update_player_disconnects` mines A's agreed frame DOWN to the global
// min `F` and sets A's `disconnect_frame = F + 1`. On A's next `advance_frame`,
// `check_simulation_consistency` returns `first_incorrect = F + 1`, but in
// sparse mode `adjust_gamestate` loads `frame_to_load = last_saved_frame`, which
// is ABOVE `first_incorrect`. The S20/F8 clamp only raises the load target UP to
// the window floor, never down to `first_incorrect`, so frames
// `[first_incorrect, last_saved_frame)` are never re-simulated with C correctly
// frozen at `F`.
//
// F7 claims this leaves A's confirmed history for `[F+1, last_saved_frame)`
// computed with C's pre-convergence (distinct, high-frame) inputs while B
// re-simulated those frames with C frozen at `F`, so A and B diverge on the
// confirmed stream.
//
// CRUX of the arbitration (why F7 may already be handled): C's *confirmed input*
// at every frame `> F` is read via the frozen-slot bypass in
// `SyncLayer::confirmed_inputs` (`con_stat.disconnected && con_stat.last_frame <
// frame` -> `queue.last_confirmed_input()`), which `set_frozen_value_at`
// converges to the same value on EVERY survivor regardless of what is in the
// re-simulated saved history. But the RECORDED GAME STATE for an un-re-simulated
// frame still reflects whatever C value A originally simulated with (its distinct
// high-frame inputs), because `adjust_gamestate` never re-ran those frames. This
// test settles whether that state actually diverges across survivors on the
// confirmed stream, or whether the frozen bypass + sparse confirmed-frame
// clamping makes the gap unobservable on confirmed frames.
//
// It asserts byte-identical recorded state across A and B for every mutually
// confirmed frame. (The genuine `frame_to_load > first_incorrect` Error logged by
// `adjust_gamestate` is NOT asserted here: it routes through the bare
// `report_violation!` macro to the global `TracingObserver` only and never reaches
// a session `CollectingObserver`, so it is unobservable from this test. See the
// note at the verdict assertion below.) The verdict is whichever way it runs on
// current code.
#[test]
fn p2p_sparse_continue_without_gossip_lowered_disconnect_confirmed_stream_converges(
) -> Result<(), FortressError> {
    // Small prediction window so "below last_saved_frame but in window" is easy to
    // engineer; sparse save mode so the single saved state is what
    // `adjust_gamestate` loads.
    const MAX_PREDICTION: usize = 4;
    let clock = TestClock::new();
    let (s1, s2, s3, a1, a2, a3, blocked) = create_filtered_channel_triple();
    let pc = protocol_config(&clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(3)?
        .with_max_prediction_window(MAX_PREDICTION)
        .with_save_mode(SaveMode::Sparse)
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .with_disconnect_timeout(Duration::from_millis(400))
        .with_disconnect_notify_delay(Duration::from_millis(100))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .start_p2p_session(s1)?;
    let build_remote = |local: PlayerHandle,
                        socket: FilterSocket,
                        remotes: [(PlayerHandle, SocketAddr); 2]|
     -> Result<P2PSession<StubConfig>, FortressError> {
        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(pc.clone())
            .with_num_players(3)?
            .with_max_prediction_window(MAX_PREDICTION)
            .with_save_mode(SaveMode::Sparse)
            .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
            .with_disconnect_timeout(Duration::from_millis(400))
            .with_disconnect_notify_delay(Duration::from_millis(100))
            .add_player(PlayerType::Local, local)?;
        for (handle, addr) in remotes {
            builder = builder.add_player(PlayerType::Remote(addr), handle)?;
        }
        builder.start_p2p_session(socket)
    };
    let mut sess2 = build_remote(
        PlayerHandle::new(1),
        s2,
        [(PlayerHandle::new(0), a1), (PlayerHandle::new(2), a3)],
    )?;
    let mut sess3 = build_remote(
        PlayerHandle::new(2),
        s3,
        [(PlayerHandle::new(0), a1), (PlayerHandle::new(1), a2)],
    )?;
    synchronize_three_sessions(&mut sess1, &mut sess2, &mut sess3, &clock, 500);
    let _ = drain_events(&mut sess1);
    let _ = drain_events(&mut sess2);
    let _ = drain_events(&mut sess3);

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();
    // Per-survivor recorded confirmed state (re-simulated frames overwrite,
    // so the final value for a confirmed frame is the survivor's settled state).
    let mut states1: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states2: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // --- Phase 1: warmup, all links open, all three confirm together. C emits
    // DISTINCT inputs every frame so the dropped slot's frozen value is
    // frame-sensitive: simulating C's frame-k input vs. its frozen-at-F input
    // yields different `StateStub` parity, surfacing any un-corrected gap. ---
    for i in 0..6u32 {
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

    // --- Phase 2: block ONLY C -> B. C keeps delivering DISTINCT inputs to A, so
    // A confirms C through a HIGHER frame than B and A's sole sparse-saved state
    // climbs with it. ---
    blocked.block(a3, a2);
    for i in 0..8u32 {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 2);
        let _ = try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            i + 50,
            &mut states1,
        );
        let _ = try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1050,
            &mut states2,
        );
        let _ = try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            i + 3000,
            &mut sink,
        );
    }
    // Light poll so A absorbs C's in-flight extra frames (C -> A still open).
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);

    // --- Phase 3: C goes fully silent. Pump A + B past the disconnect timeout so
    // both auto-drop C; B's low-frame gossip about C reaches A and mines A's
    // agreed frame (and `disconnect_frame`) DOWN below A's `last_saved_frame`,
    // forcing the sparse `frame_to_load > first_incorrect` situation. ---
    blocked.block(a3, a1);
    let mut sess1_dropped = false;
    let mut sess2_dropped = false;
    for _ in 0..80 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        let _ = try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            500,
            &mut states1,
        );
        let _ = try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            1500,
            &mut states2,
        );
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
    assert!(
        sess1_dropped,
        "sess1 must auto-drop the silent peer for this repro to exercise the gossip-lowered \
         disconnect path"
    );
    assert!(
        sess2_dropped,
        "sess2 must auto-drop the silent peer for this repro to exercise the gossip-lowered \
         disconnect path"
    );

    // --- Phase 4: let the gossip fully converge and keep both survivors advancing
    // so their confirmed frames climb past the drop. ---
    for _ in 0..40 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        let _ = try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            500,
            &mut states1,
        );
        let _ = try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            1500,
            &mut states2,
        );
    }

    assert!(
        sess1.confirmed_frame().as_i32() > 6,
        "sess1 confirmed_frame must advance past the warmup/drop; got {:?}",
        sess1.confirmed_frame()
    );
    assert!(
        sess2.confirmed_frame().as_i32() > 6,
        "sess2 confirmed_frame must advance past the warmup/drop; got {:?}",
        sess2.confirmed_frame()
    );

    // --- Verdict assertion (1): every frame both survivors consider confirmed
    // (and both recorded) must have byte-equal recorded state. If F7 is real, the
    // un-re-simulated `[F+1, last_saved_frame)` gap diverges here.
    //
    // This state-equality check is the genuine red->green guard for F7. We do NOT
    // assert on the sparse-mode `frame_to_load > first_incorrect` Error (tagged
    // "this indicates a bug") emitted by `adjust_gamestate`: that guard uses the
    // bare `report_violation!` macro, which routes ONLY to the global
    // `TracingObserver` and never pushes into a session's `CollectingObserver`
    // (documented at `src/network/protocol/mod.rs` near the
    // `enqueue_replicated_input_drops_entry_when_pending_output_full` test). The
    // observer wired onto `sess1` therefore cannot observe that violation, and no
    // test in this suite installs a `tracing-subscriber` layer to capture it.
    // Byte-identical confirmed state across survivors is the contract that
    // actually matters and the divergence the bug produces, so we rely on it. ---
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
        "no confirmed frames were compared across both survivors (bound={confirmed_bound}); \
         the repro did not exercise the drop path"
    );
    assert!(
        divergences.is_empty(),
        "F7: confirmed state diverged across survivors after a gossip-lowered disconnect under \
         sparse saving (bound={confirmed_bound}, compared={compared}): {divergences:?}"
    );

    Ok(())
}

// ============================================================================
// ARBITRATION (audit finding F9): host-attached spectator stream convergence
// when 3rd-peer gossip lowers the agreed freeze frame AFTER frames were
// already forwarded to the spectator.
// ============================================================================
//
// Scenario (N=3 `ContinueWithout`, spectator attached to the host P0):
//   - Asymmetric loss: P0 receives P2 through a HIGHER frame than P1.
//   - P0 directly detects P2's drop (`remove_player`) and freezes P2 "high" at
//     its own received frame; with P2 excluded, `confirmed_frame` advances and
//     `send_confirmed_inputs_to_spectators` forwards frames carrying P2's
//     high-frame value to the attached spectator.
//   - Later P1's gossip (P2 confirmed only through a LOWER frame) propagates via
//     `update_player_disconnects`, which mines P0's agreed frame DOWN to the
//     global-min `F` and re-rolls the frozen value (`set_frozen_value_at`) +
//     arms a rollback.
//
// The player-side stream is re-rollable, so the host's own recorded confirmed
// inputs for the dropped slot converge to the post-gossip value. The spectator
// stream is append-only/monotonic (`next_spectator_frame` only `try_add(1)`s and
// the host only sends `next_spectator_frame..=confirmed_frame`), so if the host
// forwarded the dropped slot at a value/status that the later gossip changes, the
// spectator is stranded on the pre-convergence value -> silent desync.
//
// This test compares, per frame, the dropped slot's (value, InputStatus) the
// SPECTATOR surfaces in its `AdvanceFrame` against what the HOST surfaces in its
// post-convergence `AdvanceFrame` for the same frame. They must be byte-and-status
// equal. If F9 is real, frames forwarded before convergence differ.

use fortress_rollback::{Message, NonBlockingSocket};

/// A [`BusSocket`] wrapper that drops sends on currently-blocked directional
/// links (driven by a shared [`BlockedLinks`]). Lets the F9 repro model
/// directional asymmetric loss on a 4-node mesh (3 players + 1 spectator) that
/// `create_filtered_channel_triple` (3 nodes only) cannot express, because the
/// host must also be able to send to the spectator address.
struct FilterBusSocket {
    inner: BusSocket,
    local_addr: SocketAddr,
    blocked: BlockedLinks,
}

impl FilterBusSocket {
    fn new(bus: &RoutingBus, addr: SocketAddr, blocked: BlockedLinks) -> Self {
        Self {
            inner: bus.socket(addr),
            local_addr: addr,
            blocked,
        }
    }
}

impl NonBlockingSocket<SocketAddr> for FilterBusSocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        if self.blocked.is_blocked_pub(self.local_addr, *addr) {
            return;
        }
        self.inner.send_to(msg, addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        self.inner.receive_all_messages()
    }
}

/// Handles `requests` against `stub`, recording the dropped slot (player index 2)
/// `(value, InputStatus)` keyed by the stub's resulting absolute frame. Because
/// `LoadGameState` resets the stub frame and `AdvanceFrame` increments it,
/// rollback re-simulations re-key the same frame, so the LAST write per frame is
/// that frame's post-convergence value.
fn record_dropped_slot(
    stub: &mut GameStub,
    requests: fortress_rollback::RequestVec<StubConfig>,
    map: &mut BTreeMap<i32, (u32, InputStatus)>,
) {
    const DROPPED: usize = 2;
    for request in requests {
        match request {
            FortressRequest::LoadGameState { cell, .. } => {
                let loaded = cell.load().expect("load cell");
                stub.gs = loaded;
            },
            FortressRequest::SaveGameState { cell, frame } => {
                let checksum = crate::common::calculate_hash(&stub.gs);
                cell.save(frame, Some(stub.gs), Some(checksum as u128));
            },
            FortressRequest::AdvanceFrame { inputs } => {
                let dropped = inputs
                    .get(DROPPED)
                    .map(|&(input, status)| (input.inp, status));
                stub.gs.advance_frame_pub(inputs);
                if let Some(vs) = dropped {
                    map.insert(stub.gs.frame, vs);
                }
            },
        }
    }
}

#[test]
fn p2p_continue_without_spectator_converges_after_gossip_lowered_freeze_frame(
) -> Result<(), FortressError> {
    let clock = TestClock::new();
    let bus = RoutingBus::new();
    let blocked = BlockedLinks::new();

    let a0: SocketAddr = ([127, 0, 0, 1], 30001).into();
    let a1: SocketAddr = ([127, 0, 0, 1], 30002).into();
    let a2: SocketAddr = ([127, 0, 0, 1], 30003).into();
    let spec_addr: SocketAddr = ([127, 0, 0, 1], 30004).into();

    let s0 = FilterBusSocket::new(&bus, a0, blocked.clone());
    let s1 = FilterBusSocket::new(&bus, a1, blocked.clone());
    let s2 = FilterBusSocket::new(&bus, a2, blocked.clone());
    let spec_socket = FilterBusSocket::new(&bus, spec_addr, blocked.clone());

    let pc = protocol_config(&clock);

    // Host (P0) carries the attached spectator. Generous disconnect timeout so the
    // explicit `remove_player` direct-detection path fires, not the auto-timeout.
    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(3)?
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .with_disconnect_timeout(Duration::from_secs(5))
        .with_disconnect_notify_delay(Duration::from_millis(100))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(2))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(3))?
        .start_p2p_session(s0)?;

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(3)?
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .with_disconnect_timeout(Duration::from_secs(5))
        .with_disconnect_notify_delay(Duration::from_millis(100))
        .add_player(PlayerType::Remote(a0), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(2))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(3)?
        .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
        .with_disconnect_timeout(Duration::from_secs(5))
        .with_disconnect_notify_delay(Duration::from_millis(100))
        .add_player(PlayerType::Remote(a0), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(1))?
        .add_player(PlayerType::Local, PlayerHandle::new(2))?
        .start_p2p_session(s2)?;

    let mut spec_sess: SpectatorSession<StubConfig> = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc)
        .with_num_players(3)?
        .start_spectator_session(a0, spec_socket)
        .expect("spectator session should start");

    // --- Sync all four together. ---
    let mut all_synced = false;
    for _ in 0..1000 {
        host.poll_remote_clients();
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        spec_sess.poll_remote_clients();
        if host.current_state() == SessionState::Running
            && sess1.current_state() == SessionState::Running
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
        "all four sessions must synchronize: host={:?}, sess1={:?}, sess2={:?}, spec={:?}",
        host.current_state(),
        sess1.current_state(),
        sess2.current_state(),
        spec_sess.current_state(),
    );
    let _ = drain_events(&mut host);
    let _ = drain_events(&mut sess1);
    let _ = drain_events(&mut sess2);
    let _: Vec<_> = spec_sess.events().collect();

    let mut host_stub = GameStub::new();
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut spec_stub = GameStub::new();

    // Host's recorded per-frame (value, status) for the dropped slot, captured at
    // every AdvanceFrame the host emits (including re-simulated rollback frames),
    // so the LAST value recorded for each frame is the post-convergence one. The
    // absolute frame is read from the stub itself (LoadGameState resets it,
    // AdvanceFrame increments it), so rollbacks key correctly.
    let mut host_dropped: BTreeMap<i32, (u32, InputStatus)> = BTreeMap::new();
    // Spectator's per-frame (value, status) for the dropped slot.
    let mut spec_dropped: BTreeMap<i32, (u32, InputStatus)> = BTreeMap::new();

    // --- Phase 1: warmup, all links open. ---
    for i in 0..6_u32 {
        host.poll_remote_clients();
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        host.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess1.add_local_input(PlayerHandle::new(1), StubInput { inp: i + 1000 })?;
        sess2.add_local_input(PlayerHandle::new(2), StubInput { inp: i + 2000 })?;

        let r0 = host.advance_frame()?;
        record_dropped_slot(&mut host_stub, r0, &mut host_dropped);
        let r1 = sess1.advance_frame()?;
        stub1.handle_requests(r1);
        let r2 = sess2.advance_frame()?;
        stub2.handle_requests(r2);
    }
    // Let confirmations settle.
    for _ in 0..10 {
        host.poll_remote_clients();
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // --- Phase 2: asymmetric loss. Block ONLY P2 -> P1, with DISTINCT P2 inputs,
    // so the host (P0) receives P2 through a HIGHER frame than P1 does. Keep the
    // gap to a SINGLE frame so the eventual global-min `F` stays inside the
    // un-discarded window (mirrors the player-side remove_player F-convergence
    // repro above). ---
    blocked.block(a2, a1);

    for i in 0..1_u32 {
        host.poll_remote_clients();
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        host.add_local_input(PlayerHandle::new(0), StubInput { inp: i + 20 })?;
        sess1.add_local_input(PlayerHandle::new(1), StubInput { inp: i + 1020 })?;
        // P2 keeps producing DISTINCT values; only the host receives them.
        sess2.add_local_input(PlayerHandle::new(2), StubInput { inp: i + 3000 })?;

        let r0 = host.advance_frame()?;
        record_dropped_slot(&mut host_stub, r0, &mut host_dropped);
        let r1 = sess1.advance_frame()?;
        stub1.handle_requests(r1);
        let r2 = sess2.advance_frame()?;
        stub2.handle_requests(r2);
    }
    // One light poll so the host absorbs P2's in-flight extra frame (P2 -> P0
    // open), but NOT enough to advance mutual confirmation past P1's stalled
    // P2 frame.
    host.poll_remote_clients();
    sess1.poll_remote_clients();
    sess2.poll_remote_clients();
    clock.advance(POLL_INTERVAL_DETERMINISTIC);

    // --- Phase 3: the HOST directly detects P2's drop and freezes "high" at its
    // own (higher) received frame. With P2 excluded, the host's confirmed_frame
    // advances and forwards frames carrying P2's high-frame value to the
    // spectator BEFORE the lower gossip arrives. ---
    host.remove_player(PlayerHandle::new(2))?;

    // Advance the host a few frames solo-ish so it FORWARDS confirmed frames
    // (with P2 frozen "high") to the spectator before P1's gossip lands. Pump the
    // spectator here too so it COMMITS those pre-convergence frames.
    for i in 0..4_u32 {
        host.poll_remote_clients();
        sess1.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        host.add_local_input(PlayerHandle::new(0), StubInput { inp: i + 500 })?;
        let r0 = host.advance_frame()?;
        record_dropped_slot(&mut host_stub, r0, &mut host_dropped);

        // Spectator consumes whatever the host has forwarded so far.
        match spec_sess.advance_frame() {
            Ok(requests) => record_dropped_slot(&mut spec_stub, requests, &mut spec_dropped),
            Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
            Err(e) => panic!("spectator advance_frame failed: {:?}", e),
        }
    }

    // --- Phase 4: P1 also drops P2 (its own lower received frame) and gossip
    // propagates so the host mines its agreed frame DOWN to the global-min `F`
    // and re-rolls + rolls back the dropped slot's value. Keep pumping the
    // spectator: a CORRECT implementation must re-send / correct the previously
    // forwarded frames; a buggy one strands the spectator. ---
    sess1.remove_player(PlayerHandle::new(2))?;
    for i in 0..40_u32 {
        host.poll_remote_clients();
        sess1.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        host.add_local_input(PlayerHandle::new(0), StubInput { inp: i + 700 })?;
        let r0 = host.advance_frame()?;
        record_dropped_slot(&mut host_stub, r0, &mut host_dropped);

        // sess1 (the co-survivor) drives gossip; it may legitimately be
        // throttled (prediction window) or its input dropped while converging.
        // Tolerate exactly those, propagate anything unexpected.
        match sess1.add_local_input(PlayerHandle::new(1), StubInput { inp: i + 1700 }) {
            Ok(()) => match sess1.advance_frame() {
                Ok(r1) => stub1.handle_requests(r1),
                Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
                Err(e) => return Err(e),
            },
            Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
            Err(e) => return Err(e),
        }

        match spec_sess.advance_frame() {
            Ok(requests) => record_dropped_slot(&mut spec_stub, requests, &mut spec_dropped),
            Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
            Err(e) => panic!("spectator advance_frame failed: {:?}", e),
        }
    }

    // --- Verdict: every frame the spectator surfaced for the dropped slot must
    // match the host's POST-CONVERGENCE (value, status) for that frame. The host
    // map's last write per frame is post-convergence (rollback re-simulates). ---
    let mut compared = 0_u32;
    let mut divergences: Vec<(i32, (u32, InputStatus), (u32, InputStatus))> = Vec::new();
    for (&frame, &spec_vs) in &spec_dropped {
        if let Some(&host_vs) = host_dropped.get(&frame) {
            compared += 1;
            if spec_vs != host_vs {
                divergences.push((frame, spec_vs, host_vs));
            }
        }
    }

    assert!(
        compared > 0,
        "no dropped-slot frames were compared between spectator and host; \
         the repro did not exercise the forward-then-gossip path \
         (spec={spec_dropped:?}, host={host_dropped:?})"
    );
    assert!(
        divergences.is_empty(),
        "F9: spectator's dropped-slot (value, status) diverged from the host's \
         post-convergence values after a gossip-lowered freeze frame \
         (compared={compared}): {divergences:?}\n\
         full spectator map: {spec_dropped:?}\n\
         full host map: {host_dropped:?}",
    );

    Ok(())
}

// ============================================================================
// ARBITRATION (Completeness-Critic #4): staggered multi-drop convergence with
// OPPOSITE observation order on the two survivors.
// ============================================================================
//
// The finding: a single scalar `disconnect_frame = min(existing, new)` could
// "collapse" staggered multi-drops so that a SECOND, lower-framed drop of peer D
// lowers the rollback floor and re-simulates across peer C's earlier freeze
// boundary, WITHOUT re-validating C's per-handle frozen value against the new
// floor — and the question is whether the two survivors, who observe the two
// drops in OPPOSITE orders (and initially freeze each dropped slot at DIFFERENT
// local frames under asymmetric loss), converge to a byte-identical confirmed
// history.
//
// Code trace (HEAD) establishes this is NOT a bug:
//   * `self.disconnect_frame` (p2p_session.rs ~:3303) is ONLY a rollback FLOOR;
//     it is read solely by `adjust_gamestate` to pick `frame_to_load`. It never
//     feeds any slot's frozen value. A lower floor merely re-simulates MORE
//     frames; each re-simulated frame surfaces a disconnected+frozen slot's value
//     via `synchronized_inputs` -> `queue.last_confirmed_input()`, which is
//     derived purely from THAT slot's own queue at THAT slot's own converged
//     `status.last_frame` (set by `set_frozen_value_at`).
//   * `set_frozen_value_at(handle, F_handle)` -> `roll_confirmed_input_to(F_handle)`
//     -> `confirmed_input(F_handle)` indexes `inputs[F_handle % queue_length]` and
//     returns it iff `input.frame == F_handle`. The result depends ONLY on the
//     handle, the converged frame, and that handle's confirmed buffer content —
//     never on call order or on another handle's drop. `status.last_frame` is
//     mined DOWN via `min` (commutative), so the converged frame per handle is
//     order-independent. C's final frozen value is identical whether C dropped
//     before or after D.
//   * `disconnect_player_at_frames` mutates `local_connect_status` and calls
//     `set_frozen_value_at` ONLY for the dropping endpoint's own handles, so D's
//     later, lower drop never disturbs C's already-converged frozen value/status.
//
// This test is the empirical 110/100 confirmation: 4 players, survivors A(0) and
// B(1); droppers C(2) and D(3). Asymmetric loss makes A receive C "high"/D "low"
// and B receive C "low"/D "high"; A drops C-then-D while B drops D-then-C (each
// survivor's SECOND drop being the LOWER-framed one, exercising the `min`-lowered
// floor re-simulating across the first drop's freeze boundary). With
// `DesyncDetection::On { interval: 1 }` the oracle asserts (1) ZERO DesyncDetected
// on either survivor, and (2) byte-hash equality of every shared confirmed frame
// recorded on both survivors after both drops converge.

/// Drains a survivor's events, returning any `DesyncDetected` as tuples.
fn collect_desyncs(sess: &mut P2PSession<StubConfig>) -> Vec<(Frame, u128, u128, SocketAddr)> {
    sess.events()
        .filter_map(|e| match e {
            FortressEvent::DesyncDetected {
                frame,
                local_checksum,
                remote_checksum,
                addr,
            } => Some((frame, local_checksum, remote_checksum, addr)),
            _ => None,
        })
        .collect()
}

/// Tolerant per-survivor advance: records confirmed state, skipping frames the
/// session legitimately throttles (prediction window / not-yet-synced) while
/// converging. Mirrors the `try_advance_recording` discipline used by the
/// single-drop under-loss repro above.
fn advance_survivor_recording(
    sess: &mut P2PSession<StubConfig>,
    stub: &mut GameStub,
    handle: PlayerHandle,
    value: u32,
    states: &mut BTreeMap<i32, StateStub>,
    desyncs: &mut Vec<(Frame, u128, u128, SocketAddr)>,
) -> Result<(), FortressError> {
    match sess.add_local_input(handle, StubInput { inp: value }) {
        Ok(()) => match sess.advance_frame() {
            Ok(r) => stub.handle_requests_recording(r, states),
            Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
            Err(e) => return Err(e),
        },
        Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
        Err(e) => return Err(e),
    }
    desyncs.extend(collect_desyncs(sess));
    Ok(())
}

/// Tolerant explicit drop: the co-survivor's earlier drop of the same peer
/// gossips over and may auto-remove the slot before our explicit call. Either
/// way the disconnect machinery converges the slot, so tolerate
/// `PlayerAlreadyRemoved` / `AlreadyDisconnected` (the staggered-floor path still
/// ran).
fn drop_player_tolerant(
    sess: &mut P2PSession<StubConfig>,
    handle: PlayerHandle,
) -> Result<(), FortressError> {
    match sess.remove_player(handle) {
        Ok(())
        | Err(FortressError::InvalidRequestStructured {
            kind:
                fortress_rollback::InvalidRequestKind::PlayerAlreadyRemoved { .. }
                | fortress_rollback::InvalidRequestKind::AlreadyDisconnected { .. },
        }) => Ok(()),
        Err(e) => Err(e),
    }
}

#[test]
fn staggered_two_drops_opposite_order_survivors_converge_no_desync() -> Result<(), FortressError> {
    // ----- Arrange -----
    let clock = TestClock::new();
    let bus = RoutingBus::new();
    let blocked = BlockedLinks::new();

    let a0: SocketAddr = ([127, 0, 0, 1], 40001).into(); // survivor A
    let a1: SocketAddr = ([127, 0, 0, 1], 40002).into(); // survivor B
    let a2: SocketAddr = ([127, 0, 0, 1], 40003).into(); // dropper C
    let a3: SocketAddr = ([127, 0, 0, 1], 40004).into(); // dropper D

    let s0 = FilterBusSocket::new(&bus, a0, blocked.clone());
    let s1 = FilterBusSocket::new(&bus, a1, blocked.clone());
    let s2 = FilterBusSocket::new(&bus, a2, blocked.clone());
    let s3 = FilterBusSocket::new(&bus, a3, blocked.clone());

    let pc = protocol_config(&clock);

    // Generous disconnect timeout so the EXPLICIT `remove_player` direct-detection
    // path fires (not the auto-timeout), and survivors converge via gossip.
    let build = |socket: FilterBusSocket,
                 local: PlayerHandle|
     -> Result<P2PSession<StubConfig>, FortressError> {
        let mut b = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(pc.clone())
            .with_num_players(4)?
            .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
            .with_disconnect_timeout(Duration::from_secs(30))
            .with_disconnect_notify_delay(Duration::from_millis(100))
            .with_desync_detection_mode(DesyncDetection::On { interval: 1 });
        for (h, addr) in [(0, a0), (1, a1), (2, a2), (3, a3)] {
            b = if PlayerHandle::new(h) == local {
                b.add_player(PlayerType::Local, local)?
            } else {
                b.add_player(PlayerType::Remote(addr), PlayerHandle::new(h))?
            };
        }
        b.start_p2p_session(socket)
    };

    let mut sess_a = build(s0, PlayerHandle::new(0))?;
    let mut sess_b = build(s1, PlayerHandle::new(1))?;
    let mut sess_c = build(s2, PlayerHandle::new(2))?;
    let mut sess_d = build(s3, PlayerHandle::new(3))?;

    // Synchronize all four.
    let mut synced = false;
    for _ in 0..1000 {
        sess_a.poll_remote_clients();
        sess_b.poll_remote_clients();
        sess_c.poll_remote_clients();
        sess_d.poll_remote_clients();
        if sess_a.current_state() == SessionState::Running
            && sess_b.current_state() == SessionState::Running
            && sess_c.current_state() == SessionState::Running
            && sess_d.current_state() == SessionState::Running
        {
            synced = true;
            break;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    assert!(synced, "4-player session failed to synchronize");
    let _ = collect_desyncs(&mut sess_a);
    let _ = collect_desyncs(&mut sess_b);
    let _ = drain_events(&mut sess_c);
    let _ = drain_events(&mut sess_d);

    let mut stub_a = GameStub::new();
    let mut stub_b = GameStub::new();
    let mut stub_c = GameStub::new();
    let mut stub_d = GameStub::new();

    // Per-frame recorded confirmed state on each survivor (last write per frame is
    // post-rollback / post-convergence). Byte-hash equality of these is oracle (2).
    let mut states_a: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states_b: BTreeMap<i32, StateStub> = BTreeMap::new();

    // Accumulated DesyncDetected across the whole post-drop run (oracle (1)).
    let mut desyncs_a: Vec<(Frame, u128, u128, SocketAddr)> = Vec::new();
    let mut desyncs_b: Vec<(Frame, u128, u128, SocketAddr)> = Vec::new();

    // ----- Act: warmup with all four peers so each has confirmed inputs. -----
    let warmup_frames = 8_u32;
    for i in 0..warmup_frames {
        for _ in 0..3 {
            sess_a.poll_remote_clients();
            sess_b.poll_remote_clients();
            sess_c.poll_remote_clients();
            sess_d.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
        sess_a.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        stub_a.handle_requests_recording(sess_a.advance_frame()?, &mut states_a);
        sess_b.add_local_input(PlayerHandle::new(1), StubInput { inp: i + 1000 })?;
        stub_b.handle_requests_recording(sess_b.advance_frame()?, &mut states_b);
        sess_c.add_local_input(PlayerHandle::new(2), StubInput { inp: i + 2000 })?;
        stub_c.handle_requests(sess_c.advance_frame()?);
        sess_d.add_local_input(PlayerHandle::new(3), StubInput { inp: i + 3000 })?;
        stub_d.handle_requests(sess_d.advance_frame()?);
    }
    // Let confirmations settle so the warmup frames are mutually confirmed.
    for _ in 0..12 {
        sess_a.poll_remote_clients();
        sess_b.poll_remote_clients();
        sess_c.poll_remote_clients();
        sess_d.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    desyncs_a.extend(collect_desyncs(&mut sess_a));
    desyncs_b.extend(collect_desyncs(&mut sess_b));

    // Asymmetric reception so each survivor freezes the two dropped slots at
    // DIFFERENT local frames before convergence:
    //   * C -> B blocked  (A receives C "high", B receives C "low")
    //   * D -> A blocked  (B receives D "high", A receives D "low")
    // Keep the gap to a SINGLE frame and DO NOT drain afterward, so neither
    // survivor confirms + discards the inputs the post-drop rollback needs; this
    // keeps the eventual per-handle global-min `F` inside the un-discarded
    // prediction window (mirrors the single-drop under-loss repro above).
    blocked.block(a2, a1);
    blocked.block(a3, a0);

    for i in 0..1_u32 {
        sess_a.poll_remote_clients();
        sess_b.poll_remote_clients();
        sess_c.poll_remote_clients();
        sess_d.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        sess_a.add_local_input(PlayerHandle::new(0), StubInput { inp: i + 20 })?;
        stub_a.handle_requests_recording(sess_a.advance_frame()?, &mut states_a);
        sess_b.add_local_input(PlayerHandle::new(1), StubInput { inp: i + 1020 })?;
        stub_b.handle_requests_recording(sess_b.advance_frame()?, &mut states_b);
        sess_c.add_local_input(PlayerHandle::new(2), StubInput { inp: i + 2500 })?;
        stub_c.handle_requests(sess_c.advance_frame()?);
        sess_d.add_local_input(PlayerHandle::new(3), StubInput { inp: i + 3500 })?;
        stub_d.handle_requests(sess_d.advance_frame()?);
    }
    // One light poll so each survivor absorbs the in-flight extra frame on its
    // OPEN direction (A absorbs C-high, B absorbs D-high), but not enough to
    // advance mutual confirmation past the stalled blocked-direction frame.
    sess_a.poll_remote_clients();
    sess_b.poll_remote_clients();
    sess_c.poll_remote_clients();
    sess_d.poll_remote_clients();
    clock.advance(POLL_INTERVAL_DETERMINISTIC);

    // The two survivors directly detect the two drops in OPPOSITE orders:
    //   * A: drop C first (received high), then D second (received low).
    //   * B: drop D first (received high), then C second (received low).
    // Each survivor's SECOND drop is the one it received at the LOWER frame, so it
    // lowers `disconnect_frame` and re-simulates across the FIRST drop's freeze
    // boundary — exactly the staggered-floor scenario the finding targets, and in
    // opposite order on the two survivors.
    sess_a.remove_player(PlayerHandle::new(2))?; // A: C first (high)
    sess_b.remove_player(PlayerHandle::new(3))?; // B: D first (high)
    sess_a.remove_player(PlayerHandle::new(3))?; // A: D second (low)
    sess_b.remove_player(PlayerHandle::new(2))?; // B: C second (low)

    // Pump P_a + P_b so gossip propagates through `update_player_disconnects`,
    // converging each survivor's per-handle agreed frame DOWN to the global min
    // and re-rolling each frozen value with it.
    for i in 0..80_u32 {
        sess_a.poll_remote_clients();
        sess_b.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        // Re-assert the drops in case gossip ordering surfaced a fresh endpoint;
        // tolerated if already removed.
        drop_player_tolerant(&mut sess_a, PlayerHandle::new(3))?;
        drop_player_tolerant(&mut sess_b, PlayerHandle::new(2))?;

        advance_survivor_recording(
            &mut sess_a,
            &mut stub_a,
            PlayerHandle::new(0),
            i + 700,
            &mut states_a,
            &mut desyncs_a,
        )?;
        advance_survivor_recording(
            &mut sess_b,
            &mut stub_b,
            PlayerHandle::new(1),
            i + 1700,
            &mut states_b,
            &mut desyncs_b,
        )?;
    }
    // Final settle so the last gossiped checksums are compared.
    for _ in 0..20 {
        sess_a.poll_remote_clients();
        sess_b.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        desyncs_a.extend(collect_desyncs(&mut sess_a));
        desyncs_b.extend(collect_desyncs(&mut sess_b));
    }

    // ----- Assert -----
    // Oracle (1): no DesyncDetected on either survivor across the whole run.
    assert_eq!(
        desyncs_a,
        Vec::new(),
        "survivor A reported DesyncDetected after staggered opposite-order drops"
    );
    assert_eq!(
        desyncs_b,
        Vec::new(),
        "survivor B reported DesyncDetected after staggered opposite-order drops"
    );

    // Oracle (2): byte-hash equality of every shared confirmed frame recorded on
    // both survivors. Only compare frames at/below each survivor's confirmed bound
    // (frames beyond it may still hold predictions/in-flight re-simulations).
    assert!(
        sess_a.confirmed_frame().as_i32() > warmup_frames as i32,
        "survivor A confirmed_frame must advance past warmup+drops; got {:?}",
        sess_a.confirmed_frame()
    );
    assert!(
        sess_b.confirmed_frame().as_i32() > warmup_frames as i32,
        "survivor B confirmed_frame must advance past warmup+drops; got {:?}",
        sess_b.confirmed_frame()
    );
    let confirmed_bound = std::cmp::min(
        sess_a.confirmed_frame().as_i32(),
        sess_b.confirmed_frame().as_i32(),
    );
    let mut compared = 0_u32;
    let mut divergences: Vec<(i32, u64, u64)> = Vec::new();
    for (&frame, &state_a) in &states_a {
        if frame > confirmed_bound {
            continue;
        }
        if let Some(&state_b) = states_b.get(&frame) {
            compared += 1;
            let ha = crate::common::calculate_hash(&state_a);
            let hb = crate::common::calculate_hash(&state_b);
            if ha != hb {
                divergences.push((frame, ha, hb));
            }
        }
    }
    assert!(
        compared > 0,
        "no confirmed frames were compared across both survivors (bound={confirmed_bound}); \
         the repro did not exercise the staggered multi-drop path"
    );
    assert!(
        divergences.is_empty(),
        "C4: confirmed state hash diverged across survivors after staggered \
         opposite-order multi-drop (bound={confirmed_bound}, compared={compared}): \
         {divergences:?}"
    );

    Ok(())
}
