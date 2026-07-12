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
    create_channel_pair, create_channel_quad, create_channel_triple, create_filtered_channel_mesh,
    create_filtered_channel_quad, create_filtered_channel_triple, drain_sync_events,
    poll_with_advance, synchronize_sessions_deterministic, BlockedLinks, BusSocket, FilterSocket,
    RoutingBus, SyncConfig, TestClock, POLL_INTERVAL_DETERMINISTIC,
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
fn p2p_explicit_remove_uses_coordinated_continue_without_under_halt_config(
) -> Result<(), FortressError> {
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

    // The configured Halt policy governs automatic timeout handling.
    // Explicit removal is an application-authorized, mesh-certified
    // ContinueWithout operation on every participant.
    sess2.remove_player(PlayerHandle::new(2))?;
    let _ = drain_events(&mut sess2);

    for i in 0..12_u32 {
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        advance_session(&mut sess2, &mut stub2, PlayerHandle::new(1), i + 100)?;
        poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 3);
        advance_session(&mut sess1, &mut stub1, PlayerHandle::new(0), i + 200)?;
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "an explicit certified removal must override the automatic Halt policy"
    );
    let events = drain_events(&mut sess1);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, FortressEvent::PeerDropped { handle, .. } if *handle == PlayerHandle::new(2))),
        "every certified participant must observe PeerDropped; got {events:?}"
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
fn p2p_remove_player_local_certificate_commits_then_rejects_removed_slot(
) -> Result<(), FortressError> {
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
    let events: Vec<_> = sess1.events().collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, FortressEvent::PeerDropped { handle, .. } if *handle == PlayerHandle::new(1))),
        "the one-member survivor certificate must commit without a network round trip: {events:?}"
    );
    let result = sess1.remove_player(PlayerHandle::new(1));
    assert!(
        matches!(
            &result,
            Err(FortressError::InvalidRequestStructured {
                kind: fortress_rollback::InvalidRequestKind::PlayerAlreadyRemoved { .. }
            })
        ),
        "committed removal must reject a later request: {result:?}"
    );

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
            // The second target is still a declared survivor of the first
            // serialized operation and must keep carrying its certificate
            // until that commit closes. It is excluded from the queued second
            // operation afterward.
            sess3.poll_remote_clients();
            sess4.poll_remote_clients();
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
    build_filtered_three_player_sessions_with_timeouts([
        disconnect_timeout,
        disconnect_timeout,
        disconnect_timeout,
    ])
}

/// Like [`build_filtered_three_player_sessions`] but with a PER-SESSION
/// disconnect timeout (`timeouts[i]` applies to session i+1). Asymmetric
/// timeouts let a test force which survivor detects a silent peer first — the
/// detection-order choreography the N0 freeze-barrier repros below depend on.
#[allow(clippy::type_complexity)]
fn build_filtered_three_player_sessions_with_timeouts(
    timeouts: [Duration; 3],
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
    // 8 = `SessionBuilder` default (`DEFAULT_MAX_PREDICTION_FRAMES`).
    build_filtered_three_player_sessions_with_timeouts_and_prediction(timeouts, 8)
}

/// Like [`build_filtered_three_player_sessions_with_timeouts`] but ALSO with an
/// explicit `max_prediction` window. The small-window (`max_prediction == 2`)
/// N0 liveness repro below needs this: the freeze-barrier deadlock only
/// manifests when both survivors can burn their entire prediction window
/// before any disconnect detection happens.
#[allow(clippy::type_complexity)]
fn build_filtered_three_player_sessions_with_timeouts_and_prediction(
    timeouts: [Duration; 3],
    max_prediction: usize,
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
                 disconnect_timeout: Duration,
                 remotes: [(PlayerHandle, SocketAddr); 2]|
     -> Result<P2PSession<StubConfig>, FortressError> {
        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(pc.clone())
            .with_num_players(3)?
            .with_max_prediction_window(max_prediction)
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
        timeouts[0],
        [(PlayerHandle::new(1), a2), (PlayerHandle::new(2), a3)],
    )?;
    let mut sess2 = build(
        PlayerHandle::new(1),
        s2,
        timeouts[1],
        [(PlayerHandle::new(0), a1), (PlayerHandle::new(2), a3)],
    )?;
    let mut sess3 = build(
        PlayerHandle::new(2),
        s3,
        timeouts[2],
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
    // 8 = `SessionBuilder` default (`DEFAULT_MAX_PREDICTION_FRAMES`).
    build_filtered_four_player_sessions_with_timeouts_and_prediction([disconnect_timeout; 4], 8)
}

/// Like [`build_filtered_four_player_sessions`] but with a PER-SESSION
/// disconnect timeout (`timeouts[i]` applies to session i+1) and an explicit
/// `max_prediction` window — the 4-player analog of
/// [`build_filtered_three_player_sessions_with_timeouts_and_prediction`]. The
/// N=4 mutual-mute liveness repro below needs both: a small window so every
/// survivor can burn its entire prediction window before any disconnect
/// detection happens, and a long timeout on the dying peer so its session never
/// interferes.
#[allow(clippy::type_complexity)]
fn build_filtered_four_player_sessions_with_timeouts_and_prediction(
    timeouts: [Duration; 4],
    max_prediction: usize,
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
                 disconnect_timeout: Duration,
                 remotes: [(PlayerHandle, SocketAddr); 3]|
     -> Result<P2PSession<StubConfig>, FortressError> {
        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(pc.clone())
            .with_num_players(4)?
            .with_max_prediction_window(max_prediction)
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
        timeouts[0],
        [
            (PlayerHandle::new(1), a2),
            (PlayerHandle::new(2), a3),
            (PlayerHandle::new(3), a4),
        ],
    )?;
    let mut sess2 = build(
        PlayerHandle::new(1),
        s2,
        timeouts[1],
        [
            (PlayerHandle::new(0), a1),
            (PlayerHandle::new(2), a3),
            (PlayerHandle::new(3), a4),
        ],
    )?;
    let mut sess3 = build(
        PlayerHandle::new(2),
        s3,
        timeouts[2],
        [
            (PlayerHandle::new(0), a1),
            (PlayerHandle::new(1), a2),
            (PlayerHandle::new(3), a4),
        ],
    )?;
    let mut sess4 = build(
        PlayerHandle::new(3),
        s4,
        timeouts[3],
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

/// Builds five fully-meshed `P2PSession`s over a filtered in-process mesh, with a
/// PER-SESSION disconnect timeout (`timeouts[i]` applies to session i) and an
/// explicit `max_prediction` window — the 5-player analog of
/// [`build_filtered_four_player_sessions_with_timeouts_and_prediction`].
///
/// Returns `(A, B, C, D, C2, blocked, a_A, a_B, a_C, a_D, a_C2, clock)`, where the
/// session order is handles `0..5` (A=h0, B=h1, C=h2, D=h3, C2=h4) and the five
/// addresses are index-aligned to those handles. The N=5 double-failure-relay
/// coverage needs the SECOND relay survivor C2 so the fold-min over relays is a
/// genuine multi-element conjunction (see the test for the choreography).
#[allow(clippy::type_complexity)]
fn build_filtered_five_player_sessions_with_timeouts_and_prediction(
    timeouts: [Duration; 5],
    max_prediction: usize,
) -> Result<
    (
        P2PSession<StubConfig>,
        P2PSession<StubConfig>,
        P2PSession<StubConfig>,
        P2PSession<StubConfig>,
        P2PSession<StubConfig>,
        BlockedLinks,
        SocketAddr,
        SocketAddr,
        SocketAddr,
        SocketAddr,
        SocketAddr,
        TestClock,
    ),
    FortressError,
> {
    let clock = TestClock::new();
    let (mut sockets, addrs, blocked) = create_filtered_channel_mesh(5);
    assert_eq!(sockets.len(), 5, "five-player mesh must yield five sockets");
    assert_eq!(addrs.len(), 5, "five-player mesh must yield five addresses");
    // Drain the socket Vec into named bindings (reverse order so `pop` is in
    // index order). The addresses stay index-aligned to the handles below.
    let s5 = sockets.pop().expect("socket 5");
    let s4 = sockets.pop().expect("socket 4");
    let s3 = sockets.pop().expect("socket 3");
    let s2 = sockets.pop().expect("socket 2");
    let s1 = sockets.pop().expect("socket 1");
    let (a1, a2, a3, a4, a5) = (addrs[0], addrs[1], addrs[2], addrs[3], addrs[4]);
    let pc = protocol_config(&clock);

    let build = |local: PlayerHandle,
                 socket: FilterSocket,
                 disconnect_timeout: Duration,
                 remotes: [(PlayerHandle, SocketAddr); 4]|
     -> Result<P2PSession<StubConfig>, FortressError> {
        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(pc.clone())
            .with_num_players(5)?
            .with_max_prediction_window(max_prediction)
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
        timeouts[0],
        [
            (PlayerHandle::new(1), a2),
            (PlayerHandle::new(2), a3),
            (PlayerHandle::new(3), a4),
            (PlayerHandle::new(4), a5),
        ],
    )?;
    let mut sess2 = build(
        PlayerHandle::new(1),
        s2,
        timeouts[1],
        [
            (PlayerHandle::new(0), a1),
            (PlayerHandle::new(2), a3),
            (PlayerHandle::new(3), a4),
            (PlayerHandle::new(4), a5),
        ],
    )?;
    let mut sess3 = build(
        PlayerHandle::new(2),
        s3,
        timeouts[2],
        [
            (PlayerHandle::new(0), a1),
            (PlayerHandle::new(1), a2),
            (PlayerHandle::new(3), a4),
            (PlayerHandle::new(4), a5),
        ],
    )?;
    let mut sess4 = build(
        PlayerHandle::new(3),
        s4,
        timeouts[3],
        [
            (PlayerHandle::new(0), a1),
            (PlayerHandle::new(1), a2),
            (PlayerHandle::new(2), a3),
            (PlayerHandle::new(4), a5),
        ],
    )?;
    let mut sess5 = build(
        PlayerHandle::new(4),
        s5,
        timeouts[4],
        [
            (PlayerHandle::new(0), a1),
            (PlayerHandle::new(1), a2),
            (PlayerHandle::new(2), a3),
            (PlayerHandle::new(3), a4),
        ],
    )?;

    synchronize_five_sessions(
        &mut sess1, &mut sess2, &mut sess3, &mut sess4, &mut sess5, &clock, 800,
    );
    let _ = drain_events(&mut sess1);
    let _ = drain_events(&mut sess2);
    let _ = drain_events(&mut sess3);
    let _ = drain_events(&mut sess4);
    let _ = drain_events(&mut sess5);

    Ok((
        sess1, sess2, sess3, sess4, sess5, blocked, a1, a2, a3, a4, a5, clock,
    ))
}

/// Synchronizes five sessions deterministically. Returns when all five are in
/// `Running` state, or panics if synchronization does not complete in
/// `iterations` iterations. The 5-player analog of [`synchronize_four_sessions`].
fn synchronize_five_sessions(
    sess1: &mut P2PSession<StubConfig>,
    sess2: &mut P2PSession<StubConfig>,
    sess3: &mut P2PSession<StubConfig>,
    sess4: &mut P2PSession<StubConfig>,
    sess5: &mut P2PSession<StubConfig>,
    clock: &TestClock,
    iterations: usize,
) {
    for _ in 0..iterations {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        sess4.poll_remote_clients();
        sess5.poll_remote_clients();
        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
            && sess3.current_state() == SessionState::Running
            && sess4.current_state() == SessionState::Running
            && sess5.current_state() == SessionState::Running
        {
            return;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    panic!(
        "Five sessions failed to synchronize: sess1={:?}, sess2={:?}, sess3={:?}, sess4={:?}, sess5={:?}",
        sess1.current_state(),
        sess2.current_state(),
        sess3.current_state(),
        sess4.current_state(),
        sess5.current_state()
    );
}

/// Polls five sessions and advances virtual time by
/// `POLL_INTERVAL_DETERMINISTIC * iterations`. The 5-player analog of
/// [`poll_four`].
fn poll_five(
    sess1: &mut P2PSession<StubConfig>,
    sess2: &mut P2PSession<StubConfig>,
    sess3: &mut P2PSession<StubConfig>,
    sess4: &mut P2PSession<StubConfig>,
    sess5: &mut P2PSession<StubConfig>,
    clock: &TestClock,
    iterations: usize,
) {
    for _ in 0..iterations {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        sess4.poll_remote_clients();
        sess5.poll_remote_clients();
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

    // --- Phase 4: every survivor drops D via explicit `remove_player`. On this
    // direct path each
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

    let mut a_dropped = false;
    let mut b_dropped = false;
    let mut c_dropped = false;
    for _ in 0..20 {
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
        if a_dropped && b_dropped && c_dropped {
            break;
        }
    }

    // The D certificate itself traversed A<->B<->C. Restore the unrelated
    // A<->C data path before its ordinary disconnect timeout can start a new,
    // incompatible membership operation.
    blocked.unblock(a_addr, c_addr);
    blocked.unblock(c_addr, a_addr);
    for _ in 0..100 {
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
// ARBITRATION (N=4 residual): "stale-echo freeze" — VERDICT: NOTABUG
// (arbitrated S41). Documented at `p2p_session.rs`
// `remote_slot_confirmed_bound`'s rustdoc ("# Documented residuals").
// ============================================================================
//
// Claim once feared: a third survivor X freezes the dropped slot D using its
// STALE-LOW cache of survivor U's old gossip about D. The moment ANOTHER peer's
// (B's) disconnect report flips X's `queue_connected` for D, X's
// endpoint-terms override (`update_player_disconnects`' min over running
// endpoints) mins that stale-low cached term and converges a freeze frame BELOW
// X's current confirmed bound — which (the fear was) X's confirmation may
// already have passed, re-exposing the window-floor mechanism: X silently
// overwrites confirmed history for D in `(stale_low, X_confirmed]` while U
// (whose own bound stayed high) keeps D's real inputs there.
//
// VERDICT (S41): NOTABUG. The stale-low term is held by a STILL-RUNNING endpoint,
// so the SAME term is folded into X's confirmed BOUND (`remote_slot_confirmed_bound`)
// that the override later mins — bound == applied freeze within a snapshot, so X is
// pinned AT the stale-low value and never confirms past it (no window-floor
// re-exposure). The only cross-call escape is UNREACHABLE in a full mesh
// (see the assertion-site comment below and the rustdoc); the genuine escape needs
// fold-membership asymmetry, which is the sibling `p2p_n4_double_failure_relay_*`
// (REAL) corner.
//
// This test is the IN-MESH REACHABILITY evidence for that NOTABUG: it drives the
// full choreography and the F4 byte oracle (over the shared confirmed-frame range,
// A's and C's recorded confirmed StateStub must be byte-equal) shows the victim C
// pinned low with byte-IDENTICAL confirmed state — confirmation never outran the
// freeze in process. The byte-compare is the oracle; a NON-EMPTY divergence list
// (or a survivor stalling / `advance_frame` erroring) would be a RED reproduction.
//
// Assignment (natural mapping onto the F4 harness): U = A (high-receipt peer
// whose own bound stays high), X = C (stale-holding survivor), flip-trigger =
// B. A->C is severed (as in F4) so C never hears A's NEWER, higher gossip about
// D — C's `endpoint_A.peer_connect_status(D)` is frozen at A's early, low view.
//
// The non-vacuity assertion guards that some confirmed frames were compared; the
// `divergences.is_empty()` assertion pins the NOTABUG outcome so the file stays a
// regression guard (it flips RED if the corner ever reproduces).

/// Stale-echo arbitration (VERDICT: NOTABUG, S41). Engineers C to hold a
/// stale-low cache of A's early gossip about D (A->C severed after A's low gossip
/// lands), then has B drop D and gossip the disconnect to C, flipping C's
/// `queue_connected(D)` while C still holds A's stale-low term. Records confirmed
/// states per survivor and runs the F4 byte oracle, which shows C pinned low with
/// byte-identical confirmed state — confirmation never outran the freeze. Asserts
/// non-vacuity (some confirmed frames compared) and that the survivors' confirmed
/// state did NOT diverge.
#[test]
fn p2p_n4_stale_echo_freeze_dropped_slot_converges_across_survivors() -> Result<(), FortressError> {
    // Long symmetric timeouts: every drop here is driven by explicit
    // `remove_player` + gossip propagation, never by an auto-timeout (so a
    // blocked LINK never collapses an endpoint to non-running on its own and
    // changes the fold membership out from under the choreography).
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
    ) = build_filtered_four_player_sessions(Duration::from_secs(5))?;

    let mut stub_a = GameStub::new();
    let mut stub_b = GameStub::new();
    let mut stub_c = GameStub::new();
    let mut stub_d = GameStub::new();

    let mut states_a: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states_b: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states_c: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    let h_a = PlayerHandle::new(0);
    let h_b = PlayerHandle::new(1);
    let h_c = PlayerHandle::new(2);
    let h_d = PlayerHandle::new(3);

    // --- Phase 1: tiny warmup (all links open) so A's gossip about D reaches C
    // at a LOW frame and all four mutually confirm a couple of frames. D's
    // stream (i + 3000) is non-constant so a frozen D value is frame-sensitive
    // and divergence is byte-detectable. ---
    let warmup_frames = 3_u32;
    for i in 0..warmup_frames {
        poll_four(
            &mut sess_a,
            &mut sess_b,
            &mut sess_c,
            &mut sess_d,
            &clock,
            3,
        );
        try_advance_recording(&mut sess_a, &mut stub_a, h_a, i, &mut states_a)?;
        try_advance_recording(&mut sess_b, &mut stub_b, h_b, i + 1000, &mut states_b)?;
        try_advance_recording(&mut sess_c, &mut stub_c, h_c, i + 2000, &mut states_c)?;
        try_advance_recording(&mut sess_d, &mut stub_d, h_d, i + 3000, &mut sink)?;
    }
    poll_four(
        &mut sess_a,
        &mut sess_b,
        &mut sess_c,
        &mut sess_d,
        &clock,
        4,
    );

    // --- Phase 2: SEVER A->C now (early), freezing C's cache of A's view of D
    // at A's CURRENT (low) gossip. From here C never hears A's newer, higher
    // gossip about D — C's `endpoint_A.peer_connect_status(D)` is stuck low. We
    // keep D->A, D->B, D->C open and advance several frames so A's and B's REAL
    // receipt of D climbs well ABOVE the frozen value C holds for A. ---
    blocked.block(a_addr, c_addr);
    blocked.block(c_addr, a_addr); // sever both directions, as in F4

    // Advance a wider window so A's (and B's) D receipt climbs above the stale
    // value frozen in C's A-cache. C still receives D directly, so C's OWN
    // local receipt of D also climbs — but C's confirmed_frame is BOUNDED by
    // the gossip fold, which includes A's frozen-low cache.
    let climb_window = 6_u32;
    for i in 0..climb_window {
        poll_four(
            &mut sess_a,
            &mut sess_b,
            &mut sess_c,
            &mut sess_d,
            &clock,
            2,
        );
        try_advance_recording(&mut sess_a, &mut stub_a, h_a, 100 + i, &mut states_a)?;
        try_advance_recording(&mut sess_b, &mut stub_b, h_b, 1100 + i, &mut states_b)?;
        try_advance_recording(&mut sess_c, &mut stub_c, h_c, 2100 + i, &mut states_c)?;
        try_advance_recording(&mut sess_d, &mut stub_d, h_d, 3100 + i, &mut sink)?;
    }
    poll_four(
        &mut sess_a,
        &mut sess_b,
        &mut sess_c,
        &mut sess_d,
        &clock,
        4,
    );

    // --- Phase 3: D goes silent to EVERYONE. B drops D first (direct
    // `remove_player`) at B's HIGH receipt, then gossips D-disconnected to C
    // (B->C open), flipping C's `queue_connected(D)`. C still holds A's
    // stale-low cache (A->C severed), so C's override mins down to A's stale-low
    // term. A, meanwhile, dropped D at its OWN high receipt and (A<->C severed)
    // never hears C's lowering directly. ---
    blocked.block(d_addr, a_addr);
    blocked.block(d_addr, b_addr);
    blocked.block(d_addr, c_addr);

    // B drops D at its high receipt and re-gossips the disconnect to C.
    sess_b.remove_player(h_d).unwrap();
    // A drops D at its high receipt; A's own bound for D stays HIGH (A folds its
    // own receipt, not C's lowering — A<->C severed).
    sess_a.remove_player(h_d).unwrap();
    // C does NOT explicitly remove D: we want C to learn D is disconnected via
    // B's GOSSIP (the queue_connected flip), so the stale-low A-cache term is
    // what mines C's freeze frame down — not C's own (higher) receipt.

    let mut c_stall_error: Option<FortressError> = None;
    for _ in 0..200 {
        sess_a.poll_remote_clients();
        sess_b.poll_remote_clients();
        sess_c.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        try_advance_recording(&mut sess_a, &mut stub_a, h_a, 500, &mut states_a)?;
        try_advance_recording(&mut sess_b, &mut stub_b, h_b, 1500, &mut states_b)?;
        // C may stall or error if the mine-down targets confirmed/discarded
        // history (the window-floor re-exposure the claim predicts). Capture
        // (not `?`-propagate) any non-throttle error so the test can report it.
        match try_advance_recording(&mut sess_c, &mut stub_c, h_c, 2500, &mut states_c) {
            Ok(_) => {},
            Err(e) => {
                if c_stall_error.is_none() {
                    c_stall_error = Some(e);
                }
            },
        }
    }

    // --- Byte oracle (F4 pattern): over the shared confirmed range, A vs C is
    // the stale-echo pair (A's bound stayed high; C's was mined down by A's
    // stale-low cached term). A vs B catches residual divergence. ---
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

    // Non-vacuity: the choreography must actually have exercised confirmed
    // frames across survivors, or the arbitration proves nothing.
    assert!(
        compared > 0,
        "no confirmed frames were compared across survivors (bound={confirmed_bound}); \
         the stale-echo choreography did not produce overlapping confirmed history"
    );

    // NOTABUG (arbitrated S41), two-part rationale (matches the
    // `remote_slot_confirmed_bound` rustdoc "# Documented residuals"):
    //
    // (1) INTRA-SNAPSHOT (bound == applied freeze). C's stale-low term is held by
    //     a STILL-RUNNING endpoint (A's cache), so it is folded into C's confirmed
    //     BOUND (`remote_slot_confirmed_bound`) AND into the override
    //     (`update_player_disconnects`) IDENTICALLY — both folds iterate the same
    //     `is_running()` endpoint set. C is therefore pinned AT the stale-low value
    //     the whole window: it never confirms PAST it, and the mine-down rewrites
    //     only PREDICTED frames (no window-floor re-exposure, no divergence, no stall).
    //
    // (2) REACHABILITY (no cross-call escape). The one cross-call escape — a
    //     still-running endpoint adopting a LOWER freeze on its disconnect-flip
    //     (`merge_peer_connect_status`'s first-disconnect `last_frame = remote.last_frame`)
    //     below an already-confirmed frame — is UNREACHABLE in a full mesh: for any
    //     endpoint to gossip a low `{disconnected, F}` that we adopt, some running
    //     endpoint must have gossiped that low `F`, and broadcast gossip (every input
    //     packet carries the full connect-status vector) delivers the same low `F` to
    //     US, pinning our bound at `F` BEFORE we can confirm past it. The genuine
    //     escape requires that source endpoint to be ABSENT from our fold
    //     (fold-membership asymmetry) — i.e. the sibling `p2p_n4_double_failure_relay_*`
    //     (REAL) corner, where the low term's origin dies and is pruned.
    //
    // If this assertion ever fails (divergences non-empty, or C errored), the
    // stale-echo corner has gone RED and this guard must be revisited.
    assert!(
        divergences.is_empty() && c_stall_error.is_none(),
        "stale-echo went RED: the corner reproduced in-process \
         (bound={confirmed_bound}, compared={compared}, divergences={divergences:?}, \
         c_stall_error={c_stall_error:?})"
    );

    Ok(())
}

// ============================================================================
// ARBITRATION (N=4 residual): "double-failure relay" — VERDICT: REAL
// (arbitrated S41; the genuine residual). Documented at `p2p_session.rs`
// `remote_slot_confirmed_bound`'s rustdoc ("# Documented residuals").
// ============================================================================
//
// Documented quote (p2p_session.rs, remote_slot_confirmed_bound rustdoc):
// "an origin survivor that dies AFTER relaying its low freeze value to a third
// peer but BEFORE delivering it to us leaves a window where our bound (no longer
// folding the dead origin's endpoint) exceeds the override later relayed through
// the third peer."
//
// VERDICT (this test): REAL — arbitrated AND adversarially verified faithful /
// non-vacuous in S41. The corner REPRODUCES in-process as a REAL, deterministic,
// cross-survivor CONFIRMED-STATE divergence with FIRST divergence at `F + 1`; it
// VANISHES if the origin stays in the fold or the relay is prompt (so the
// choreography is load-bearing, not an artifact). The test asserts the divergence
// IS present as a CI-safe characterization guard that FLIPS when a future fix
// closes the residual.
//
// Mechanism, as actually driven below (A = "us", B = dying origin, C = relay,
// D = the dropped peer; num_players=4, ContinueWithout, max_prediction=4):
//   - Phase 2 builds a D-receipt gradient by blocking D->B: B's view of D freezes
//     at the global-min `F` while A and C keep receiving D up to a higher `M`.
//     While B still runs and gossips {connected, F}, every survivor folds B's low
//     term, so the freeze barrier PINS confirmed_frame at `F` even though A's and
//     C's receipt of D has climbed to M. C keeps gossiping {connected, M} to A, so
//     A's cached C-view of D (`endpoint_C.peer_connect_status(D)`) reaches M.
//   - Phase 3 arms the DOUBLE FAILURE via explicit removals (no timeout, so fold
//     membership is fully controlled): block D->C (C frozen at M); sever C->A while
//     A's cached C-view of D is still {connected, M} (A's long timeout keeps C's
//     endpoint RUNNING, so A keeps folding C at M); `B.remove_player(D)` (B freezes
//     D at F and gossips {disconnected, F} to C, which ADOPTS F via
//     `merge_peer_connect_status:1847` even though C's own receipt is M); and
//     `A.remove_player(B)` so B's endpoint on A goes non-running and is PRUNED from
//     A's D fold (`remote_slot_confirmed_bound`/`update_player_disconnects` skip
//     `!endpoint.is_running()`) — the "origin dies after relaying to C but before
//     delivering to us" leg. A's D fold is now min(A receipt, C-cache) = M.
//   - Phase 4: A records frames ABOVE `F` with D's REAL inputs (A does not yet know
//     D dropped) and its small window DISCARDS the frames below `current - 4`.
//   - Phase 5 re-opens C->A: C's relayed {disconnected, F} finally lands, A lowers
//     its D term to F and arms a disconnect rollback to F+1 — BELOW A's window
//     floor. The S20 out-of-window clamp (adjust_gamestate:7478,
//     `frame_to_load.max(window_floor)`) keeps A LIVE (Ok, no stall) and
//     re-simulates only the in-window frames with D frozen at F. The already
//     -discarded frames in `(F, floor]` keep A's real-input state, which no longer
//     matches C's frozen-at-F state -> a permanent divergence the relay cannot fix.
//
// This is the N>=4 instance of the discard-before-convergence residual the S32
// barrier closed at N=3 but only NARROWED at N>=4. The "double failure" is: the
// origin B is pruned from A's fold (its low view never reaches A directly) AND
// C->A is delayed past A's record+discard of the high frames.
//
// CLOSED in Session 55 by the sequence-numbered floor-round (the verified-sound
// `AsyncAckSoundRoundSeq` mode). With B pruned and >=2 remotes still running, A is
// in the relay topology, so it solicits each folded relay's CURRENT pessimistic
// floor via `FloorRequest`/`FloorReply` (a relay's floor is the `min` over its own
// freeze and the committed freezes it folds disconnected). While the round to C is
// incomplete (C unreachable during the sever), A HOLDS its confirmed bound for D
// at the current confirmed frame and never discards the contested window; on re-
// open C answers with `F` over a reorder-immune reply channel and A converges.
// Because the reply rides a dedicated seq-validated channel (not the input
// gossip), this closes the warm, cold-cache, AND mid-game-drop reorder facets in
// one mechanism.
//
// Oracle (F4 byte pattern): over the shared confirmed-frame range, A's and C's
// recorded confirmed StateStub must be byte-equal; an EMPTY divergence list is the
// GREEN signal (a NON-EMPTY one is a REGRESSION). The decisive SIGNATURE of this
// corner (vs a plain input split) is that D's confirmed *inputs* also CONVERGE
// across A and C (both frozen at F) — so the library's own input-checksum desync
// detector (BLIND to the pre-fix state divergence) now agrees with the converged
// state. The assertions pin the post-fix convergence as a CI-safe guard.

/// A graceful drop whose declared survivors are split into disconnected
/// components cannot form an all-participant certificate. The frontier must
/// remain held and the attempt must fail closed without emitting `PeerDropped`.
#[test]
fn p2p_n4_double_failure_certificate_partition_fails_closed() -> Result<(), FortressError> {
    // LONG symmetric timeouts so NO auto-timeout fires: every endpoint pruning
    // here is driven by an explicit `remove_player`, so the choreography fully
    // controls fold membership (a blocked LINK never collapses an endpoint to
    // non-running out from under us). SMALL prediction window (4) so A can burn
    // its whole window and DISCARD frames below `F` before C's relayed low lands.
    let long = Duration::from_secs(20);
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
    ) = build_filtered_four_player_sessions_with_timeouts_and_prediction([long; 4], 4)?;

    let mut stub_a = GameStub::new();
    let mut stub_b = GameStub::new();
    let mut stub_c = GameStub::new();
    let mut stub_d = GameStub::new();

    let mut states_a: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states_b: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states_c: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    let h_a = PlayerHandle::new(0);
    let h_b = PlayerHandle::new(1);
    let h_c = PlayerHandle::new(2);
    let h_d = PlayerHandle::new(3);

    // --- Phase 1: warmup, all links open so all four confirm together. D's stream
    // (i + 3000) is non-constant so a frozen D value is frame-sensitive and a
    // divergence is byte-detectable. ---
    let warmup_frames = 6_u32;
    for i in 0..warmup_frames {
        poll_four(
            &mut sess_a,
            &mut sess_b,
            &mut sess_c,
            &mut sess_d,
            &clock,
            3,
        );
        try_advance_recording(&mut sess_a, &mut stub_a, h_a, i, &mut states_a)?;
        try_advance_recording(&mut sess_b, &mut stub_b, h_b, i + 1000, &mut states_b)?;
        try_advance_recording(&mut sess_c, &mut stub_c, h_c, i + 2000, &mut states_c)?;
        try_advance_recording(&mut sess_d, &mut stub_d, h_d, i + 3000, &mut sink)?;
    }
    poll_four(
        &mut sess_a,
        &mut sess_b,
        &mut sess_c,
        &mut sess_d,
        &clock,
        8,
    );

    // --- Phase 2: build the D-receipt gradient B(low=F) < C,A(high=M). Block
    // D -> B so B's receipt of D freezes at the global-min `F`. Keep D -> A and
    // D -> C (and C -> A) OPEN and advance a WIDE window so A's and C's receipt of
    // D climb to `M`. While B still runs and gossips {connected, F}, every
    // survivor folds B's low term, so confirmed_frame is PINNED at `F` (the
    // barrier working) even as A's and C's *receipt* of D climbs to M. Crucially
    // C keeps gossiping {connected, M} to A, so A's cached view of C's D term
    // (`endpoint_C.peer_connect_status(D)`) reaches M. ---
    blocked.block(d_addr, b_addr);
    let climb = 14_u32;
    for i in 0..climb {
        poll_four(
            &mut sess_a,
            &mut sess_b,
            &mut sess_c,
            &mut sess_d,
            &clock,
            2,
        );
        try_advance_recording(&mut sess_a, &mut stub_a, h_a, 100 + i, &mut states_a)?;
        try_advance_recording(&mut sess_b, &mut stub_b, h_b, 1100 + i, &mut states_b)?;
        try_advance_recording(&mut sess_c, &mut stub_c, h_c, 2100 + i, &mut states_c)?;
        try_advance_recording(&mut sess_d, &mut stub_d, h_d, 3100 + i, &mut sink)?;
    }
    poll_four(
        &mut sess_a,
        &mut sess_b,
        &mut sess_c,
        &mut sess_d,
        &clock,
        4,
    );

    // --- Phase 3: arm the DOUBLE FAILURE (all via explicit removals; no timeout).
    //   (a) Block D -> C so C's receipt of D freezes at M (D keeps reaching A).
    //   (b) Sever C -> A and A -> C while A's cached view of C's D term is still
    //       the HIGH {connected, M}. A's LONG timeout keeps C's endpoint RUNNING
    //       on A (a blocked link alone never prunes it), so A keeps folding C at M
    //       — A never hears C lower its D view until we re-open the link.
    //   (c) B `remove_player(D)`: B freezes D at F and gossips {disconnected, F}.
    //       B -> C is OPEN, so C receives it and on first learning D disconnected
    //       ADOPTS F (merge_peer_connect_status:1847) even though C's own receipt
    //       is M -> C is now frozen at the global-min F, BELOW its own receipt.
    //       B -> A is CUT first so A does NOT adopt F here (A's cached B view of D
    //       freezes at its last {connected, F}).
    //   (d) A `remove_player(B)`: B's endpoint on A goes terminal (non-running),
    //       so it is PRUNED from A's D fold (`remote_slot_confirmed_bound` and
    //       `update_player_disconnects` skip `!endpoint.is_running()`). B's low
    //       view is now gone from A's fold; A's D bound = min(A receipt, C-cache)
    //       = M. This is the "origin dies AFTER relaying to the third peer but
    //       BEFORE delivering to us" leg.
    // D stays reachable to A (D -> A open) so A keeps confirming D with REAL
    // inputs. A NEVER removes D — it must learn the drop only via C's late relay. ---
    blocked.block(d_addr, c_addr);
    blocked.block(c_addr, a_addr);
    blocked.block(a_addr, c_addr);
    blocked.block(b_addr, a_addr);
    blocked.block(a_addr, b_addr);

    sess_b.remove_player(h_d).unwrap();
    sess_a.remove_player(h_b).unwrap();

    // --- Phase 4: A confirms/records D HIGH and DISCARDS past F. With B pruned and
    // C frozen-high in A's fold, A advances and records frames above F using D's
    // REAL (connected, then predicted-high) inputs — A does NOT yet know D dropped
    // (`a_dropped_d_p4` stays false) — while its small window discards every frame
    // below `current - max_prediction`. C, meanwhile, receives B's relayed
    // {disconnected, F}, freezes D at F, and (with D mesh-excluded) advances PAST
    // F recording frozen-at-F D values. C -> A is severed so C's lowered view
    // cannot reach A yet. ---
    let mut a_dropped_b = false;
    let mut a_dropped_d_p4 = false;
    let mut a_stall_error: Option<FortressError> = None;
    for _ in 0..120 {
        sess_a.poll_remote_clients();
        sess_b.poll_remote_clients();
        sess_c.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        match try_advance_recording(&mut sess_a, &mut stub_a, h_a, 500, &mut states_a) {
            Ok(_) => {},
            Err(e) => {
                if a_stall_error.is_none() {
                    a_stall_error = Some(e);
                }
            },
        }
        try_advance_recording(&mut sess_b, &mut stub_b, h_b, 1500, &mut states_b)?;
        try_advance_recording(&mut sess_c, &mut stub_c, h_c, 2500, &mut states_c)?;

        for e in sess_a.events() {
            if let FortressEvent::PeerDropped { handle, .. } = e {
                if handle == h_b {
                    a_dropped_b = true;
                }
                if handle == h_d {
                    a_dropped_d_p4 = true;
                }
            }
        }
    }

    // --- Phase 5: the LATE RELAY. Re-open C -> A so C's {disconnected, F} finally
    // reaches A AFTER A recorded + discarded past F. A's `update_player_disconnects`
    // now folds C (running, reporting D disconnected at F) and lowers A's D term to
    // F, arming a disconnect rollback to F+1. F+1 is BELOW A's window floor; the
    // S20 out-of-window clamp (adjust_gamestate:7478, `frame_to_load.max(window_floor)`)
    // keeps A LIVE (returns Ok, no stall) and re-simulates only the in-window
    // frames with D frozen at F. The frames in `(F, floor]` were already discarded
    // and KEEP A's real-input state, which no longer matches C's frozen-at-F state
    // -> a permanent cross-survivor confirmed-state DIVERGENCE the relay cannot
    // repair. ---
    blocked.unblock(c_addr, a_addr);
    blocked.unblock(a_addr, c_addr);

    let mut a_dropped_d_p5 = false;
    for _ in 0..160 {
        sess_a.poll_remote_clients();
        sess_c.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        match try_advance_recording(&mut sess_a, &mut stub_a, h_a, 700, &mut states_a) {
            Ok(_) => {},
            Err(e) => {
                if a_stall_error.is_none() {
                    a_stall_error = Some(e);
                }
            },
        }
        try_advance_recording(&mut sess_c, &mut stub_c, h_c, 2700, &mut states_c)?;

        if sess_a
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { handle, .. } if handle == h_d))
        {
            a_dropped_d_p5 = true;
        }
    }

    // A "learned D disconnected" if it emitted PeerDropped for D in either phase.
    // The interesting (and documented) ordering is p4=false, p5=true: A confirmed
    // past F BEFORE it knew D had dropped, then learned the low freeze via the relay.
    let a_dropped_d = a_dropped_d_p4 || a_dropped_d_p5;

    // --- Byte oracle (F4 pattern): over the shared confirmed range A vs C is the
    // double-failure pair (A's D bound stayed high while B was pruned + C severed;
    // C froze D at the relayed low F). ---
    let confirmed_bound = sess_a
        .confirmed_frame()
        .as_i32()
        .min(sess_c.confirmed_frame().as_i32());
    let mut compared = 0_u32;
    let mut divergences: Vec<(i32, &'static str, StateStub, StateStub)> = Vec::new();
    for (&frame, &state_a) in &states_a {
        if frame > confirmed_bound {
            continue;
        }
        if let Some(&state_c) = states_c.get(&frame) {
            compared += 1;
            if state_a != state_c {
                divergences.push((frame, "A!=C", state_a, state_c));
            }
        }
    }

    // PROBE (the decisive signature of THIS corner vs a plain input split): read
    // D's confirmed INPUT (handle 3) on A vs C at sample in-window frames. The
    // public contract (`confirmed_inputs_for_frame`) says these are identical
    // across peers for any confirmed frame. They DO match here — both peers
    // converged D's frozen input to the global-min `F` — yet the recorded game
    // STATE for those same frames diverges, because A computed/recorded that state
    // with D's REAL inputs BEFORE the freeze converged and the frames were already
    // discarded (below the window floor) when the relay lowered the term. So the
    // library's own input-based desync detector would see NO divergence while the
    // actual confirmed STATE silently disagrees: the documented residual exactly.
    //
    // We sample frames in the QUERYABLE range at end of test (`confirmed_bound`
    // climbs to ~167, and the input ring retains roughly the last
    // `INPUT_QUEUE_LENGTH` == 128 confirmed frames). The EARLY diverging-onset
    // frames (`F + 1` .. the early window) are by now FULLY DISCARDED from the
    // input ring — `confirmed_inputs_for_frame` returns `Err`/`None` for them —
    // which STRENGTHENS the detector-blind claim: the divergence lives in discarded
    // history the input-checksum detector cannot inspect. So we filter to the frames
    // that ARE queryable (both peers return `Ok`) and assert NON-VACUOUSLY that D's
    // converged input matches across A and C on at least one genuine frame (a plain
    // `None == None` on a discarded frame must not count as a match).
    let mut input_probe: Vec<(i32, Option<u32>, Option<u32>)> = Vec::new();
    let mut queryable_matches = 0_u32;
    let mut queryable_mismatch = false;
    for &pf in &[50_i32, 100, 150] {
        if pf > confirmed_bound {
            continue;
        }
        let da = sess_a
            .confirmed_inputs_for_frame(Frame::new(pf))
            .ok()
            .and_then(|v| v.get(3).map(|i| i.inp));
        let dc = sess_c
            .confirmed_inputs_for_frame(Frame::new(pf))
            .ok()
            .and_then(|v| v.get(3).map(|i| i.inp));
        // Only frames queryable on BOTH peers count toward the non-vacuous match
        // (a discarded frame yields `None` on both and proves nothing).
        if da.is_some() && dc.is_some() {
            if da == dc {
                queryable_matches += 1;
            } else {
                queryable_mismatch = true;
            }
        }
        input_probe.push((pf, da, dc));
    }

    // Non-vacuity: the choreography must actually have exercised confirmed frames
    // across survivors, or the arbitration proves nothing.
    assert!(
        compared > 0,
        "no confirmed frames were compared across survivors (bound={confirmed_bound}); \
         the double-failure-relay choreography did not produce overlapping confirmed history"
    );

    assert!(
        !a_dropped_d && !a_dropped_b,
        "an uncertified partitioned operation must not emit a drop: \
         d_p4={a_dropped_d_p4}, d_p5={a_dropped_d_p5}, b={a_dropped_b}"
    );
    assert_eq!(
        sess_a.current_state(),
        SessionState::Synchronizing,
        "A must fail closed when its certificate participants are unreachable"
    );
    assert!(
        confirmed_bound <= warmup_frames as i32,
        "the uncertified frontier advanced past the safe prefix: bound={confirmed_bound}, \
         advance_error={a_stall_error:?}"
    );
    assert!(
        divergences.is_empty(),
        "the held frontier must not expose divergent confirmed state: {divergences:?}"
    );
    assert!(
        !queryable_mismatch,
        "the held prefix must remain identical: {input_probe:?}; matches={queryable_matches}"
    );

    Ok(())
}

#[test]
fn p2p_n5_double_failure_certificate_partition_fails_closed() -> Result<(), FortressError> {
    // LONG symmetric timeouts so NO auto-timeout fires; every prune is an
    // explicit `remove_player`. SMALL prediction window (4) so A can burn its
    // whole window and (absent the HOLD) DISCARD frames below `F` before the
    // relays' low lands.
    let long = Duration::from_secs(20);
    let (
        mut sess_a,
        mut sess_b,
        mut sess_c,
        mut sess_d,
        mut sess_c2,
        blocked,
        a_addr,
        b_addr,
        c_addr,
        d_addr,
        c2_addr,
        clock,
    ) = build_filtered_five_player_sessions_with_timeouts_and_prediction([long; 5], 4)?;

    let mut stub_a = GameStub::new();
    let mut stub_b = GameStub::new();
    let mut stub_c = GameStub::new();
    let mut stub_d = GameStub::new();
    let mut stub_c2 = GameStub::new();

    let mut states_a: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states_c: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states_c2: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink_b: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink_d: BTreeMap<i32, StateStub> = BTreeMap::new();

    let h_a = PlayerHandle::new(0);
    let h_b = PlayerHandle::new(1);
    let h_c = PlayerHandle::new(2);
    let h_d = PlayerHandle::new(3);
    let h_c2 = PlayerHandle::new(4);

    // --- Phase 1: warmup, all links open so all five confirm together. ---
    let warmup_frames = 6_u32;
    for i in 0..warmup_frames {
        poll_five(
            &mut sess_a,
            &mut sess_b,
            &mut sess_c,
            &mut sess_d,
            &mut sess_c2,
            &clock,
            3,
        );
        try_advance_recording(&mut sess_a, &mut stub_a, h_a, i, &mut states_a)?;
        try_advance_recording(&mut sess_b, &mut stub_b, h_b, i + 1000, &mut sink_b)?;
        try_advance_recording(&mut sess_c, &mut stub_c, h_c, i + 2000, &mut states_c)?;
        try_advance_recording(&mut sess_d, &mut stub_d, h_d, i + 3000, &mut sink_d)?;
        try_advance_recording(&mut sess_c2, &mut stub_c2, h_c2, i + 4000, &mut states_c2)?;
    }
    poll_five(
        &mut sess_a,
        &mut sess_b,
        &mut sess_c,
        &mut sess_d,
        &mut sess_c2,
        &clock,
        8,
    );

    // --- Phase 2: build the D-receipt gradient B(low=F) < {A,C,C2}(high≈M). ---
    blocked.block(d_addr, b_addr);
    let climb = 16_u32;
    for i in 0..climb {
        poll_five(
            &mut sess_a,
            &mut sess_b,
            &mut sess_c,
            &mut sess_d,
            &mut sess_c2,
            &clock,
            2,
        );
        try_advance_recording(&mut sess_a, &mut stub_a, h_a, 100 + i, &mut states_a)?;
        try_advance_recording(&mut sess_b, &mut stub_b, h_b, 1100 + i, &mut sink_b)?;
        try_advance_recording(&mut sess_c, &mut stub_c, h_c, 2100 + i, &mut states_c)?;
        try_advance_recording(&mut sess_d, &mut stub_d, h_d, 3100 + i, &mut sink_d)?;
        try_advance_recording(&mut sess_c2, &mut stub_c2, h_c2, 4100 + i, &mut states_c2)?;
    }
    poll_five(
        &mut sess_a,
        &mut sess_b,
        &mut sess_c,
        &mut sess_d,
        &mut sess_c2,
        &clock,
        4,
    );

    // --- Phase 3: arm the DOUBLE FAILURE across BOTH relays (all via explicit
    // removals; no timeout). ---
    blocked.block(d_addr, c_addr);
    blocked.block(d_addr, c2_addr);
    blocked.block(c_addr, a_addr);
    blocked.block(a_addr, c_addr);
    blocked.block(c2_addr, a_addr);
    blocked.block(a_addr, c2_addr);
    blocked.block(b_addr, a_addr);
    blocked.block(a_addr, b_addr);

    // B freezes D at F and gossips {disconnected, F} to C AND C2 (B -> C, B -> C2
    // open); both adopt F. B -> A is cut so A does NOT adopt F here.
    sess_b.remove_player(h_d).unwrap();
    // B's endpoint on A goes terminal → PRUNED from A's D fold. A's running
    // remotes are now {C, C2, D} = 3 running → relay topology engaged with TWO
    // folded relays (one ABOVE the floor).
    sess_a.remove_player(h_b).unwrap();

    // --- Phase 4: A confirms/records D HIGH; the multi-relay HOLD must engage so
    // A never discards the contested window. ---
    let mut a_dropped_b = false;
    let mut a_dropped_d_p4 = false;
    let mut a_stall_error: Option<FortressError> = None;
    for _ in 0..140 {
        sess_a.poll_remote_clients();
        sess_b.poll_remote_clients();
        sess_c.poll_remote_clients();
        sess_c2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        match try_advance_recording(&mut sess_a, &mut stub_a, h_a, 500, &mut states_a) {
            Ok(_) => {},
            Err(e) => {
                if a_stall_error.is_none() {
                    a_stall_error = Some(e);
                }
            },
        }
        try_advance_recording(&mut sess_b, &mut stub_b, h_b, 1500, &mut sink_b)?;
        try_advance_recording(&mut sess_c, &mut stub_c, h_c, 2500, &mut states_c)?;
        try_advance_recording(&mut sess_c2, &mut stub_c2, h_c2, 4500, &mut states_c2)?;

        for e in sess_a.events() {
            if let FortressEvent::PeerDropped { handle, .. } = e {
                if handle == h_b {
                    a_dropped_b = true;
                }
                if handle == h_d {
                    a_dropped_d_p4 = true;
                }
            }
        }
    }

    // --- Phase 5: re-open BOTH relays. Their relayed {disconnected, F} lands on
    // A (arming a below-window rollback the S20 clamp survives) and their
    // `FloorReply`s answer A's round with F, so A converges. ---
    blocked.unblock(c_addr, a_addr);
    blocked.unblock(a_addr, c_addr);
    blocked.unblock(c2_addr, a_addr);
    blocked.unblock(a_addr, c2_addr);

    let mut a_dropped_d_p5 = false;
    for _ in 0..200 {
        sess_a.poll_remote_clients();
        sess_c.poll_remote_clients();
        sess_c2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);

        match try_advance_recording(&mut sess_a, &mut stub_a, h_a, 700, &mut states_a) {
            Ok(_) => {},
            Err(e) => {
                if a_stall_error.is_none() {
                    a_stall_error = Some(e);
                }
            },
        }
        try_advance_recording(&mut sess_c, &mut stub_c, h_c, 2700, &mut states_c)?;
        try_advance_recording(&mut sess_c2, &mut stub_c2, h_c2, 4700, &mut states_c2)?;

        if sess_a
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { handle, .. } if handle == h_d))
        {
            a_dropped_d_p5 = true;
        }
    }

    let a_dropped_d = a_dropped_d_p4 || a_dropped_d_p5;

    // --- Byte oracle (F4 pattern, TWO relay witnesses): over the shared
    // confirmed range A vs C AND A vs C2 must agree. ---
    let confirmed_bound = sess_a
        .confirmed_frame()
        .as_i32()
        .min(sess_c.confirmed_frame().as_i32())
        .min(sess_c2.confirmed_frame().as_i32());
    let mut compared = 0_u32;
    let mut divergences: Vec<(i32, &'static str, StateStub, StateStub)> = Vec::new();
    for (&frame, &state_a) in &states_a {
        if frame > confirmed_bound {
            continue;
        }
        if let Some(&state_c) = states_c.get(&frame) {
            compared += 1;
            if state_a != state_c {
                divergences.push((frame, "A!=C", state_a, state_c));
            }
        }
        if let Some(&state_c2) = states_c2.get(&frame) {
            compared += 1;
            if state_a != state_c2 {
                divergences.push((frame, "A!=C2", state_a, state_c2));
            }
        }
    }

    // PROBE: D's confirmed INPUT (handle 3) on A vs C vs C2 at sample in-window
    // frames must MATCH (the freeze converged) — the decisive signature that this
    // is the silent state-only divergence, not a plain input split.
    let mut input_probe: Vec<(i32, Option<u32>, Option<u32>, Option<u32>)> = Vec::new();
    let mut queryable_matches = 0_u32;
    let mut queryable_mismatch = false;
    for &pf in &[60_i32, 110, 160] {
        if pf > confirmed_bound {
            continue;
        }
        let da = sess_a
            .confirmed_inputs_for_frame(Frame::new(pf))
            .ok()
            .and_then(|v| v.get(3).map(|i| i.inp));
        let dc = sess_c
            .confirmed_inputs_for_frame(Frame::new(pf))
            .ok()
            .and_then(|v| v.get(3).map(|i| i.inp));
        let dc2 = sess_c2
            .confirmed_inputs_for_frame(Frame::new(pf))
            .ok()
            .and_then(|v| v.get(3).map(|i| i.inp));
        // Only frames queryable on ALL THREE peers count toward the non-vacuous
        // match (a discarded frame yields `None` and proves nothing).
        if da.is_some() && dc.is_some() && dc2.is_some() {
            if da == dc && da == dc2 {
                queryable_matches += 1;
            } else {
                queryable_mismatch = true;
            }
        }
        input_probe.push((pf, da, dc, dc2));
    }

    assert!(
        compared > 0,
        "no confirmed frames were compared across survivors (bound={confirmed_bound}); \
         the N=5 double-failure-relay choreography did not produce overlapping confirmed history"
    );
    assert!(
        !a_dropped_d && !a_dropped_b,
        "an uncertified partitioned operation must not emit a drop: \
         d_p4={a_dropped_d_p4}, d_p5={a_dropped_d_p5}, b={a_dropped_b}"
    );
    assert_eq!(
        sess_a.current_state(),
        SessionState::Synchronizing,
        "A must fail closed when both certificate relays are unreachable"
    );
    assert!(
        confirmed_bound <= warmup_frames as i32,
        "the uncertified frontier advanced past the safe prefix: bound={confirmed_bound}, \
         advance_error={a_stall_error:?}"
    );
    assert!(
        divergences.is_empty(),
        "the held frontier must not expose divergent confirmed state: {divergences:?}"
    );
    assert!(
        !queryable_mismatch,
        "the held prefix must remain identical: {input_probe:?}; matches={queryable_matches}"
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
    //
    // Session-59 note: `P2PSession` now routes `report_violation!` to the
    // per-session observer (via the thread-local scope installed in
    // `advance_frame` / `poll_remote_clients`), so the two checks below are no
    // longer vacuous — an Error/Critical violation raised on this path would now
    // be captured and would fail the test. (This particular scenario converges
    // cleanly without raising any violation, so the observer may legitimately be
    // empty; the dedicated routing red-green lives in the `telemetry` unit tests
    // and the `p2p_advance_frame_routes_report_violation_to_session_observer` /
    // `sync_test_construction_violation_routes_to_session_observer` tests.)
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
// `adjust_gamestate` is NOT asserted here. Since session-59 such a
// `report_violation!` raised inside `advance_frame` WOULD reach a per-session
// `CollectingObserver`; this test simply installs none, and no
// `tracing-subscriber` layer either, so the Error is unobservable here. See the
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
    // "this indicates a bug") emitted by `adjust_gamestate`. Since session-59,
    // `report_violation!` raised inside `advance_frame` DOES route to a
    // per-session `CollectingObserver` (via the thread-local scope) — but this
    // test installs no observer on its sessions and no `tracing-subscriber`
    // layer, so it simply does not observe that violation here. Byte-identical
    // confirmed state across survivors is the contract that actually matters and
    // the divergence the bug produces, so we rely on it. ---
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

    // --- Phase 4: the host's coordinated operation already committed P2 on
    // P1 at one non-retracting cut. A second application request therefore
    // observes the committed slot rather than opening a lower gossip freeze. ---
    assert!(matches!(
        sess1.remove_player(PlayerHandle::new(2)),
        Err(FortressError::InvalidRequestStructured {
            kind: fortress_rollback::InvalidRequestKind::PlayerAlreadyRemoved { .. }
        })
    ));
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
        // C and D are each excluded from their own operation but remain a
        // declared participant in the other serialized removal until its
        // certificate commits.
        sess_c.poll_remote_clients();
        sess_d.poll_remote_clients();
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
        sess_c.poll_remote_clients();
        sess_d.poll_remote_clients();
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

// ============================================================================
// Regression (N0 freeze barrier): `confirmed_frame()` must bound each remote
// slot by the GOSSIPED mesh minimum, not by local receipt alone.
// ============================================================================
//
// N>=3 mesh, `ContinueWithout`. Peer C goes silent under asymmetric loss:
// survivor A received C's inputs through a HIGH frame, survivor B through a LOW
// frame `F` (the eventual mesh-agreed freeze frame is the global min = F).
// Before the barrier, `confirmed_frame()` folded only the LOCAL view of each
// slot (and excluded any locally-disconnected slot outright), so A's confirmed
// frame could race past `F` before the mesh agreed on `F`:
//   (a) WINDOW FLOOR — the mechanism these red repros actually hit: the race
//       lets `current_frame` run so far that the `F+1` rollback target falls
//       below the prediction-window floor; the S20 clamp then re-simulates
//       only from the floor, leaving every frame in `(F, floor)` permanently
//       embedding C's real high-frame inputs (the constant post-floor offset
//       in the red output shows the frozen-value re-roll itself SUCCEEDED —
//       only the resimulation was clamped short);
//   (b) RING EVICTION — at extreme stagger only (>= the 128-frame input-ring
//       capacity): C's input at `F` is physically overwritten by ring
//       wrap-around, and only then does the convergence re-roll
//       `set_frozen_value_at(F)` / first-freeze `freeze_at(F)` fail-safe into
//       a STALE (or default) frozen value. The logical discard alone
//       (`set_last_confirmed_frame` through confirmed-1) does NOT defeat the
//       re-roll: `discard_confirmed_frames` only moves the ring's
//       tail/length and `confirmed_input` indexes modulo capacity without a
//       tail check, so "discarded" bytes stay readable until overwritten.
// Either mechanism permanently diverges A's confirmed history for frames
// `(F, ...]` from B's (B repeats C's input at `F`) — a silent desync.
//
// The fix (GGPO PollNPlayers-faithful): a still-connected remote slot
// contributes `min(local receipt, min over running endpoints' gossiped view
// of the slot)`; a LOCALLY-DISCONNECTED slot whose drop is not yet mesh-agreed
// contributes the gossiped views ONLY (the local detection value is dropped,
// exactly as GGPO skips `local_connect_status[i]` when disconnected — folding
// it pins capped survivors against their own detection value, see the
// small-window liveness test below);
// and a slot is excluded only once the disconnect is MESH-AGREED (no running
// endpoint still reports it connected). At N=3 the bound can never exceed any
// freeze frame the mesh can later impose on this peer (every mining-down
// arrives as an endpoint-terms override, the bound folds those same terms,
// and the packet that delivers a third party's flip refreshes that party's
// term before any fold runs), so the confirmed frame can never race past `F`:
// nothing at/above `F` is lost and the throttle keeps the `F+1` rollback
// target inside the window. At N>=4 the bound strictly NARROWS the race;
// S41 arbitrated the two named corners (stale-echo freeze; double-failure
// relay) red-test-first to ONE genuine residual — the double-failure relay
// (stale-echo is NOTABUG: its low term's source endpoint stays in the fold,
// so bound == override). See `remote_slot_confirmed_bound`'s
// "# Documented residuals".
//
// Three orderings ("flavors") are pinned below. All FAIL before the barrier
// (byte divergence at frames > F) and PASS after:
//   - flavor 1 via `remove_player`:   A (high receiver) detects/froze FIRST;
//   - flavor X:                       B (low receiver) times out FIRST while A
//                                     raced with NO disconnect knowledge at all;
//   - flavor 1 via auto-timeout:      like the first, but A's own (short)
//                                     disconnect timeout is the detector.
// A fourth test pins the LIVENESS leg of the barrier at MAX_PREDICTION = 2
// (the gossip-only amendment): both survivors burn their whole window before
// detection and must still release each other afterwards. Two further
// liveness repros (the clean equal-receipt drop and the N=4 staggered
// mutual-mute) pin the protocol-level connect-status NUDGE that closes the
// post-detection gossip-mute pins.
//
// Choreography note (important for future edits): once the barrier holds a
// survivor's confirmed frame at `F`, that survivor stops advancing at
// `F + max_prediction` and — because connect-status gossip travels ONLY in
// Input messages — a survivor pinned at its prediction cap cannot gossip its
// own view through ORDINARY traffic. (The connect-status nudge re-sends a
// status-bearing duplicate Input on the keepalive cadence, but only while a
// locally-disconnected slot awaits mesh agreement, and the tight phases below
// advance the clock ~1ms per round — far below that cadence — so the mute
// reasoning holds within each phase.) The phases below therefore keep at
// least one survivor under its cap whenever a piece of disconnect knowledge
// still needs to travel.

/// Flavor 1 (detect-first, explicit removal): A removes C at its own HIGH
/// received-through frame while B's gossip still says "connected @ F"; A's
/// confirmed frame races (pre-fix) far past `F` before B's lowered knowledge
/// lands, pushing the prediction-window floor above the `F + 1` rollback
/// target (the window-floor mechanism, see the block comment above).
#[test]
fn p2p_n0_detect_first_remove_player_under_asymmetric_loss_converges() -> Result<(), FortressError>
{
    // Long symmetric timeouts: every disconnect in this test is driven by the
    // explicit `remove_player` + gossip propagation, never by a timeout.
    let (mut sess1, mut sess2, mut sess3, blocked, a1, a2, a3, clock) =
        build_filtered_three_player_sessions_with_timeouts([
            Duration::from_secs(3),
            Duration::from_secs(3),
            Duration::from_secs(3),
        ])?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    let mut states1: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states2: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // --- Phase 1: warmup, all links open, all three confirm together. C's
    // input stream (i + 3000) is DISTINCT per frame, so a frozen C value is
    // frame-sensitive and divergent freeze frames surface as divergent bytes. ---
    let warmup_frames = 4_u32;
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
            i + 3000,
            &mut sink,
        )?;
    }
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 6);

    // --- Phase 2: asymmetric loss. Block ONLY C -> B; C keeps advancing with
    // distinct inputs and keeps delivering to A, so A's received-through frame
    // for C climbs several frames past B's (B is stuck at the eventual global
    // min `F`). Pre-fix, A's confirmed frame keeps racing with its local
    // receipts here, dragging its prediction-window floor toward `F`. ---
    blocked.block(a3, a2);
    for i in 0..4_u32 {
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
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            i + 3020,
            &mut sink,
        )?;
    }
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 6);
    let conf_a_after_loss = sess1.confirmed_frame();
    let conf_b_after_loss = sess2.confirmed_frame();

    // --- Phase 3a: C goes fully silent and A removes it IMMEDIATELY at A's own
    // high received-through frame, while B's gossip still claims C connected at
    // the low `F`. Race A forward (B polls but does NOT advance, so B's own
    // lowered knowledge cannot travel yet — gossip rides only Input messages)
    // with near-zero clock steps so no timeout interferes. Pre-fix A's
    // confirmed frame jumps to min(A, B) the moment C's slot is excluded and
    // the prediction window runs far past `F`. ---
    blocked.block(a3, a1);
    sess1.remove_player(PlayerHandle::new(2)).unwrap();
    let events1: Vec<_> = drain_events(&mut sess1);
    assert!(
        events1
            .iter()
            .all(|e| !matches!(e, FortressEvent::PeerDropped { .. })),
        "prepare must not emit PeerDropped before the survivor certificate; got {events1:?}"
    );
    for _ in 0..24 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(Duration::from_millis(1));
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            500,
            &mut states1,
        )?;
    }
    assert!(
        sess1
            .events()
            .any(|event| matches!(event, FortressEvent::PeerDropped { .. })),
        "A must emit PeerDropped after the coordinated certificate commits"
    );

    // --- Phase 3b: release B. Its first advances apply the propagated drop
    // (PeerDropped via gossip, frozen at its OWN low receipt = the global min
    // `F`) and carry its disconnected@F view back to A, which re-adjusts its
    // freeze frame down to `F`. Post-fix the barrier held A's confirmed frame
    // at `F`, so the `F + 1` rollback target is inside the window and the
    // re-simulation corrects everything above `F`; pre-fix the target is below
    // the window floor and the S20 clamp leaves the frames in `(F, floor)`
    // permanently embedding C's real high-frame inputs. ---
    let mut sess2_dropped = false;
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
        if sess2
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            sess2_dropped = true;
        }
    }
    assert!(
        sess2_dropped,
        "B must drop C (via A's propagated disconnect gossip) for this repro"
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

    // --- Oracle: byte-equality of recorded confirmed state across survivors.
    // Pre-fix this fails for every frame above the agreed freeze frame `F`. ---
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
        "N0 flavor 1 (remove_player): confirmed state diverged across survivors \
         (bound={confirmed_bound}, compared={compared}, conf_a_after_loss={conf_a_after_loss:?}, \
         conf_b_after_loss={conf_b_after_loss:?}): {divergences:?}"
    );

    Ok(())
}

/// Flavor X (race-before-any-detection): the LOW receiver B times out C first
/// at `F` while A still sees C connected at a high frame; A's confirmed frame
/// legitimately ran past `F` on C's REAL inputs before ANY disconnect knowledge
/// existed anywhere, so (pre-fix) the `F + 1` rollback target is already below
/// A's prediction-window floor when B's disconnected@F gossip arrives — the
/// S20 clamp re-simulates only from the floor, leaving the frames in
/// `(F, floor)` permanently embedding C's real high-frame inputs.
#[test]
fn p2p_n0_race_before_any_detection_under_asymmetric_loss_converges() -> Result<(), FortressError> {
    // Asymmetric timeouts force the detection ORDER: B (whose last C receipt is
    // oldest) fires at 1000ms; A's timeout is far away, so A's drop can only
    // arrive via B's gossip.
    let (mut sess1, mut sess2, mut sess3, blocked, a1, a2, a3, clock) =
        build_filtered_three_player_sessions_with_timeouts([
            Duration::from_millis(3200),
            Duration::from_secs(1),
            Duration::from_millis(3200),
        ])?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    let mut states1: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states2: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // --- Phase 1: warmup with all links open (distinct C inputs). ---
    let warmup_frames = 4_u32;
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
            i + 3000,
            &mut sink,
        )?;
    }
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 6);

    // --- Phase 2: block ONLY C -> B and keep all three advancing with SINGLE
    // polls per frame (small wall-clock footprint so B's 1000ms timeout does
    // not fire yet). A keeps receiving C's distinct inputs, so A's confirmed
    // frame (pre-fix: local receipts only) races past B's stuck receipt `F`
    // with NO disconnect knowledge anywhere — the flavor-X precondition. ---
    blocked.block(a3, a2);
    for i in 0..6_u32 {
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
            i + 3020,
            &mut sink,
        )?;
    }
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 2);
    let conf_a_after_loss = sess1.confirmed_frame();
    let conf_b_after_loss = sess2.confirmed_frame();

    // --- Phase 3a: C goes fully silent. First let A burn the rest of its
    // prediction window on C's real inputs with near-zero clock steps (no
    // timeout fires; B does not advance, preserving B's own window headroom for
    // the gossip it must send after ITS drop). ---
    blocked.block(a3, a1);
    for _ in 0..16 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(Duration::from_millis(1));
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            500,
            &mut states1,
        )?;
    }

    // --- Phase 3b: advance the clock with polls only until B's disconnect
    // timeout fires. B must detect FIRST (flavor X): assert B emits PeerDropped
    // while A has emitted none. ---
    let mut sess2_dropped = false;
    let mut sess1_dropped_early = false;
    for _ in 0..40 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        if sess1
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            sess1_dropped_early = true;
        }
        if sess2
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            sess2_dropped = true;
            break;
        }
    }
    assert!(
        sess2_dropped,
        "B (short disconnect timeout) must auto-drop the silent peer C first"
    );
    assert!(
        !sess1_dropped_early,
        "flavor X requires B to detect FIRST: A must hold no disconnect knowledge \
         until B's gossip arrives"
    );

    // --- Phase 4: release both. B's first advances roll its history back to
    // `F` (frozen at its own receipt, the global min) and gossip disconnected@F
    // to A; A applies the propagated drop. Post-fix A first-freezes at `F` with
    // confirmation held at `F` (barrier), so the `F + 1` rollback target is
    // inside its window and the re-simulation replaces C's high-frame inputs
    // above `F` with the frozen value; pre-fix the phase-2/3a race pushed the
    // window floor past `F + 1`, so the S20 clamp re-simulates only from the
    // floor and A keeps C's real inputs (wrong values) for frames > F. ---
    let mut sess1_dropped = false;
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
        if sess1
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            sess1_dropped = true;
        }
    }
    assert!(
        sess1_dropped,
        "A must eventually drop C via B's propagated disconnect gossip"
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

    // --- Oracle: byte-equality of recorded confirmed state across survivors. ---
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
        "N0 flavor X (race before any detection): confirmed state diverged across survivors \
         (bound={confirmed_bound}, compared={compared}, conf_a_after_loss={conf_a_after_loss:?}, \
         conf_b_after_loss={conf_b_after_loss:?}): {divergences:?}"
    );

    Ok(())
}

/// Flavor 1 via auto-timeout: like the `remove_player` variant, but A's
/// detection comes from its OWN (short) disconnect timeout while B's (long)
/// timeout never fires — B learns of the drop only via A's gossip, then its
/// lowered disconnected@F view flows back and must correct A.
#[test]
fn p2p_n0_detect_first_auto_timeout_under_asymmetric_loss_converges() -> Result<(), FortressError> {
    // A detects first: short timeout on A, long on B and C.
    let (mut sess1, mut sess2, mut sess3, blocked, a1, a2, a3, clock) =
        build_filtered_three_player_sessions_with_timeouts([
            Duration::from_millis(400),
            Duration::from_secs(3),
            Duration::from_secs(3),
        ])?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    let mut states1: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states2: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // --- Phase 1: warmup with all links open (distinct C inputs). ---
    let warmup_frames = 4_u32;
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
            i + 3000,
            &mut sink,
        )?;
    }
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 6);

    // --- Phase 2: asymmetric loss, C -> B blocked; A's received-through frame
    // for C climbs past B's stuck `F` (and pre-fix A's confirmed frame races
    // with it, dragging the window floor toward `F`). C keeps delivering to A
    // throughout, so A's 400ms timeout cannot fire during this phase. ---
    blocked.block(a3, a2);
    for i in 0..4_u32 {
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
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            i + 3020,
            &mut sink,
        )?;
    }
    poll_three(&mut sess1, &mut sess2, &mut sess3, &clock, 4);
    let conf_a_after_loss = sess1.confirmed_frame();
    let conf_b_after_loss = sess2.confirmed_frame();

    // --- Phase 3a (detection): C fully silent; poll-only with full clock steps
    // until A's OWN 400ms timeout drops C at A's high received-through frame.
    // No advancing here: A must still have prediction-window headroom AFTER its
    // drop so its disconnected gossip (carried only by Input messages) can
    // reach B. B's 3000ms timeout stays far away — A detects FIRST. ---
    blocked.block(a3, a1);
    let mut sess1_dropped = false;
    for _ in 0..30 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        if sess1
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            sess1_dropped = true;
            break;
        }
    }
    assert!(
        sess1_dropped,
        "A (short disconnect timeout) must auto-drop the silent peer C first"
    );

    // --- Phase 3a (race): with C's slot now locally dropped on A, race A
    // forward with near-zero clock steps (B polls but does not advance). Pre-fix
    // the exclusion of the dropped slot lets A's confirmed frame jump to
    // min(A, B) and the window run far past `F`. A's sends carry its
    // disconnected@high view to B. ---
    for _ in 0..24 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(Duration::from_millis(1));
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            500,
            &mut states1,
        )?;
    }

    // --- Phase 3b: release B — it applies the propagated drop at its own low
    // receipt `F`, then gossips disconnected@F back; A re-adjusts down to `F`. ---
    let mut sess2_dropped = false;
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
        if sess2
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            sess2_dropped = true;
        }
    }
    assert!(
        sess2_dropped,
        "B must drop C (via A's propagated disconnect gossip) for this repro"
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

    // --- Oracle: byte-equality of recorded confirmed state across survivors. ---
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
        "N0 flavor 1 (auto-timeout): confirmed state diverged across survivors \
         (bound={confirmed_bound}, compared={compared}, conf_a_after_loss={conf_a_after_loss:?}, \
         conf_b_after_loss={conf_b_after_loss:?}): {divergences:?}"
    );

    Ok(())
}

/// Liveness regression (the freeze-barrier *amendment*): for a slot that is
/// LOCALLY disconnected but not yet mesh-agreed, the confirmed bound must fold
/// the running endpoints' gossiped views ONLY (GGPO `PollNPlayers` parity) —
/// folding the local detection value too pins both survivors at a small
/// prediction window until something else carries the gossip.
///
/// Mechanism (`MAX_PREDICTION = 2`): connect-status gossip travels ONLY in
/// Input messages, a capped session re-adds the same frame (dropped, so no
/// send), and a fully-acked send queue retransmits nothing — so a survivor
/// pinned at `conf + max_prediction` with everything acked is gossip-mute.
/// Under asymmetric loss both survivors pin at `F = B's receipt of C` (the
/// barrier folds B's lagging gossip on both sides) and burn their whole
/// window to `F + 2` BEFORE either detects the silent C. When their disconnect
/// timeouts then fire, a `min(local, gossip)` bound keeps BOTH bounds at `F`
/// (B's own receipt locally / B's stale gossip on A), so both stay capped and
/// neither can send its `disconnected@F` gossip through ordinary traffic —
/// on the pre-nudge tree that was a permanent pin (alive, zero progress),
/// which is what this test went red on; today the connect-status nudge would
/// eventually deliver the gossip, so the amendment's value is IMMEDIATE
/// release: B's gossip-only bound rises to A's stale-HIGHER gossiped view, B
/// regains headroom, its ordinary Input packets carry `disconnected@F` to A,
/// and the release cascades (propagated drop -> mesh agreement -> both free)
/// without waiting on any timer. (The arm choice itself is pinned by the
/// `remote_slot_confirmed_bound_locally_disconnected_folds_gossip_only` unit
/// test; this test pins end-to-end liveness.)
///
/// Choreography note: phase 2 must let A *send at least one Input after
/// receiving a C frame past `F`* (its last pre-cap send carries that higher
/// receipt as gossip) — that stale-higher cached view on B is what the
/// amendment folds to lift B's bound. If A's last gossiped receipt of C were
/// exactly `F` (zero stagger), the amended bound stays at `F` and release
/// falls to the connect-status nudge — that exactly-symmetric corner is
/// covered by `p2p_n0_clean_equal_receipt_timeout_drop_stays_live_and_converges`.
///
/// FAILED on the pre-amendment tree (zero frame progress after the drop) and
/// PASSES after.
#[test]
fn p2p_n0_small_prediction_window_staggered_drop_stays_live_and_converges(
) -> Result<(), FortressError> {
    const MAX_PREDICTION: usize = 2;
    // Short symmetric timeouts on the survivors: both detect the silent C via
    // their OWN auto-timeout while capped (B first — its last C receipt is
    // oldest — then A). C's timeout is irrelevant (it goes silent and is never
    // polled again).
    let (mut sess1, mut sess2, mut sess3, blocked, a1, a2, a3, clock) =
        build_filtered_three_player_sessions_with_timeouts_and_prediction(
            [
                Duration::from_millis(400),
                Duration::from_millis(400),
                Duration::from_secs(3),
            ],
            MAX_PREDICTION,
        )?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    let mut states1: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states2: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // --- Phase 1: warmup "ladder", all links open. Each session polls
    // IMMEDIATELY before its own step, in A -> B -> C order, so every packet a
    // session sends carries its maximally fresh receipts as gossip (an
    // earlier-stepping peer's same-frame input is already merged when a
    // later-stepping peer sends). This freshness is the zero-slack
    // prerequisite: once C -> B is blocked, C's cached gossip on B freezes at
    // C's LAST delivered packet, and if that packet's view of slot A lagged
    // below the eventual `F`, B's pre-detection confirmed frame would pin
    // BELOW `F` — then detecting C (which removes C's endpoint, and its stale
    // views, from every slot's fold) would hand B a frame of headroom that
    // leaks its disconnect gossip and self-releases the pre-amendment pin.
    // With the ladder, C's frozen view of A equals its last delivered input
    // frame (= `F`) exactly. C's inputs are DISTINCT per frame so divergent
    // freeze frames surface as divergent bytes in the oracle. Near-zero clock
    // steps keep all timeouts far away. ---
    let warmup_rounds = 6_u32;
    for i in 0..warmup_rounds {
        sess1.poll_remote_clients();
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            i,
            &mut states1,
        )?;
        sess2.poll_remote_clients();
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1000,
            &mut states2,
        )?;
        sess3.poll_remote_clients();
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            i + 3000,
            &mut sink,
        )?;
        clock.advance(Duration::from_millis(1));
    }

    // --- Phase 2: block ONLY C -> B, then keep the ladder running so all
    // three race to their prediction caps (no timeout may fire here; C keeps
    // delivering to A so A's receipt — and crucially A's last GOSSIPED receipt
    // — of C climbs past B's stuck `F`). The barrier pins both survivors'
    // confirmed frames at `F` (the lagging `F`-valued gossip bounds the
    // connected slot on both sides), so at MAX_PREDICTION = 2 both burn their
    // entire window and go gossip-mute with fully-acked send queues — the
    // deadlock precondition. ---
    blocked.block(a3, a2);
    for i in 0..10_u32 {
        sess1.poll_remote_clients();
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            i + 20,
            &mut states1,
        )?;
        sess2.poll_remote_clients();
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1020,
            &mut states2,
        )?;
        sess3.poll_remote_clients();
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            i + 3020,
            &mut sink,
        )?;
        clock.advance(Duration::from_millis(1));
    }
    let pinned_conf_a = sess1.confirmed_frame();
    let pinned_conf_b = sess2.confirmed_frame();
    // Choreography precondition: both survivors are pinned at the same `F`
    // with their entire prediction window burned. If this fails the repro
    // setup is broken (not the code under test).
    assert_eq!(
        pinned_conf_a, pinned_conf_b,
        "choreography: both survivors must pin at the same mesh-min F"
    );
    assert_eq!(
        sess1.current_frame().as_i32(),
        pinned_conf_a.as_i32() + MAX_PREDICTION as i32,
        "choreography: A must have burned its whole prediction window pre-detection"
    );
    assert_eq!(
        sess2.current_frame().as_i32(),
        pinned_conf_b.as_i32() + MAX_PREDICTION as i32,
        "choreography: B must have burned its whole prediction window pre-detection"
    );

    // --- Phase 3: C goes fully silent. Poll-only with full clock steps until
    // BOTH survivors' own 400ms timeouts drop C (B's fires first — its last C
    // receipt predates the full block). Neither survivor has any window
    // headroom left, so neither can carry its `disconnected@F` knowledge in an
    // Input message: pre-amendment this is the permanent pin. ---
    blocked.block(a3, a1);
    let mut sess1_dropped = false;
    let mut sess2_dropped = false;
    for _ in 0..40 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
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
        if sess1_dropped && sess2_dropped {
            break;
        }
    }
    assert!(
        sess1_dropped && sess2_dropped,
        "choreography: both survivors must auto-drop the silent C \
         (A dropped: {sess1_dropped}, B dropped: {sess2_dropped})"
    );
    let stall_cur_a = sess1.current_frame();
    let stall_cur_b = sess2.current_frame();

    // --- Phase 4 (the liveness assertion target): BOUNDED release loop with
    // full clock steps, driving both survivors. Pre-amendment nothing moves:
    // both bounds stay `min(local, gossip) = F`, both stay capped, no Input
    // (hence no gossip) ever flows — only KeepAlives. Post-amendment B's
    // gossip-only bound rises to A's stale-higher view, B advances and its
    // Input packets deliver `disconnected@F`; the propagated drop converges A
    // down to `F`, mesh agreement excludes the slot on both, and both run free. ---
    for _ in 0..100 {
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

    // (i) Liveness: both survivors made frame progress after the drop.
    assert!(
        sess1.current_frame() > stall_cur_a && sess2.current_frame() > stall_cur_b,
        "N0 liveness: both survivors must make frame progress after the staggered drop; \
         pinned at conf_a={:?}/cur_a={:?}, conf_b={:?}/cur_b={:?} \
         (pinned_conf={pinned_conf_a:?}, stall_cur_a={stall_cur_a:?}, stall_cur_b={stall_cur_b:?})",
        sess1.confirmed_frame(),
        sess1.current_frame(),
        sess2.confirmed_frame(),
        sess2.current_frame(),
    );

    // (ii) Mesh agreement: each survivor's confirmed frame advances past the
    // entire pre-detection stall window (`F + MAX_PREDICTION`), which is only
    // possible once the dropped slot is mesh-agreed-excluded on both sides.
    assert!(
        sess1.confirmed_frame().as_i32() > pinned_conf_a.as_i32() + MAX_PREDICTION as i32,
        "sess1 confirmed_frame must advance past the stalled window after mesh agreement; \
         got {:?} (pinned at {pinned_conf_a:?})",
        sess1.confirmed_frame()
    );
    assert!(
        sess2.confirmed_frame().as_i32() > pinned_conf_b.as_i32() + MAX_PREDICTION as i32,
        "sess2 confirmed_frame must advance past the stalled window after mesh agreement; \
         got {:?} (pinned at {pinned_conf_b:?})",
        sess2.confirmed_frame()
    );

    // (iii) Oracle: byte-equality of recorded confirmed state across survivors. ---
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
        "N0 small-window liveness: confirmed state diverged across survivors \
         (bound={confirmed_bound}, compared={compared}): {divergences:?}"
    );

    Ok(())
}

/// Liveness regression (the connect-status *nudge*): the COMMON clean-drop
/// case. A peer dies cleanly (zero stagger — both survivors hold EXACTLY equal
/// receipts of it), both survivors burn their whole prediction window against
/// the frozen receipt before their disconnect timeouts fire, and both
/// auto-drop the silent peer while capped with fully-acked send queues.
///
/// Mechanism: connect-status gossip historically travelled ONLY in Input
/// messages, and a capped survivor with a fully-acked send queue sends none —
/// so after detection both survivors hold `disconnected@F` locally while each
/// caches the OTHER's stale `connected@F` gossip. The gossip-only bound
/// (`remote_slot_confirmed_bound`) is then `F` on both sides (the other's
/// stale view), both stay capped, no Input ever carries the disconnect, mesh
/// agreement is never reached, and the session pins forever — alive
/// (KeepAlives flow) but making zero progress. Unlike the staggered variant
/// above there is no stale-HIGHER view to lift either bound: this is the
/// zero-stagger corner, and it is the COMMON case (any clean peer death with a
/// quiet link, e.g. process kill or cable pull, lands here).
///
/// The fix is the protocol-level connect-status nudge: while a session holds a
/// locally-disconnected slot that is not yet mesh-agreed, its idle endpoints
/// re-send a status-bearing duplicate Input message (built from
/// `last_acked_input`) on the keepalive cadence. The receiver treats it
/// exactly like any stale retransmitted Input packet — the hoisted
/// connect-status merge still runs — so each survivor's `disconnected@F`
/// reaches the other, mesh agreement excludes the slot on both sides, and both
/// run free.
///
/// FAILS before the nudge (permanent pin: zero frame progress after the drop)
/// and PASSES after. Adapted from the adversarial-review probe
/// `review_n0_clean_equal_receipt_drop_probe`.
#[test]
fn p2p_n0_clean_equal_receipt_timeout_drop_stays_live_and_converges() -> Result<(), FortressError> {
    const MAX_PREDICTION: usize = 2;
    // Short symmetric timeouts on the survivors; C's timeout is irrelevant (it
    // goes silent and is never polled again).
    let (mut sess1, mut sess2, mut sess3, blocked, a1, a2, a3, clock) =
        build_filtered_three_player_sessions_with_timeouts_and_prediction(
            [
                Duration::from_millis(400),
                Duration::from_millis(400),
                Duration::from_secs(3),
            ],
            MAX_PREDICTION,
        )?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    let mut states1: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states2: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // --- Phase 1: warmup ladder, ALL LINKS OPEN the whole time (no asymmetry
    // anywhere — the clean-network precondition). Distinct C inputs (i + 3000)
    // make divergent freeze frames visible as divergent bytes. Near-zero clock
    // steps keep all timeouts far away. ---
    for i in 0..6_u32 {
        sess1.poll_remote_clients();
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            i,
            &mut states1,
        )?;
        sess2.poll_remote_clients();
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1000,
            &mut states2,
        )?;
        sess3.poll_remote_clients();
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            i + 3000,
            &mut sink,
        )?;
        clock.advance(Duration::from_millis(1));
    }

    // --- Phase 2: C dies CLEANLY — both survivor links are cut at the same
    // instant, so both survivors hold exactly equal receipts of C (zero
    // stagger). Both then pump realistically and burn their entire prediction
    // window against the frozen receipt. ---
    blocked.block(a3, a2);
    blocked.block(a3, a1);
    for i in 0..10_u32 {
        sess1.poll_remote_clients();
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            i + 20,
            &mut states1,
        )?;
        sess2.poll_remote_clients();
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1020,
            &mut states2,
        )?;
        clock.advance(Duration::from_millis(1));
    }
    let pinned_conf_a = sess1.confirmed_frame();
    let pinned_conf_b = sess2.confirmed_frame();
    // Choreography preconditions: equal receipts pin both survivors at the
    // same `F` with the whole window burned. If these fail the repro setup is
    // broken (not the code under test).
    assert_eq!(
        pinned_conf_a, pinned_conf_b,
        "choreography: zero stagger must pin both survivors at the same F"
    );
    assert_eq!(
        sess1.current_frame().as_i32(),
        pinned_conf_a.as_i32() + MAX_PREDICTION as i32,
        "choreography: A must have burned its whole prediction window pre-detection"
    );
    assert_eq!(
        sess2.current_frame().as_i32(),
        pinned_conf_b.as_i32() + MAX_PREDICTION as i32,
        "choreography: B must have burned its whole prediction window pre-detection"
    );

    // --- Phase 3: poll-only with full clock steps until BOTH survivors' own
    // 400ms timeouts drop the silent C. Both detect while capped and
    // gossip-mute — the pin precondition. ---
    let mut sess1_dropped = false;
    let mut sess2_dropped = false;
    for _ in 0..40 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
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
        if sess1_dropped && sess2_dropped {
            break;
        }
    }
    assert!(
        sess1_dropped && sess2_dropped,
        "choreography: both survivors must auto-drop the silent C \
         (A dropped: {sess1_dropped}, B dropped: {sess2_dropped})"
    );
    let stall_cur_a = sess1.current_frame();
    let stall_cur_b = sess2.current_frame();

    // --- Phase 4 (the liveness assertion target): BOUNDED release loop with
    // full clock steps, driving both survivors. Pre-nudge nothing moves: both
    // gossip-only bounds equal the other's stale `connected@F` cache, both
    // stay capped, and no Input (hence no gossip) ever flows. Post-nudge each
    // idle endpoint re-sends a status-bearing duplicate Input on the keepalive
    // cadence; the `disconnected@F` gossip crosses, mesh agreement excludes
    // the slot on both sides, and both run free. ---
    for _ in 0..100 {
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

    // (i) Liveness: both survivors made frame progress after the drop.
    assert!(
        sess1.current_frame() > stall_cur_a && sess2.current_frame() > stall_cur_b,
        "N0 clean-drop liveness: both survivors must make frame progress after the drop; \
         pinned at conf_a={:?}/cur_a={:?}, conf_b={:?}/cur_b={:?} \
         (pinned_conf={pinned_conf_a:?}, stall_cur_a={stall_cur_a:?}, stall_cur_b={stall_cur_b:?})",
        sess1.confirmed_frame(),
        sess1.current_frame(),
        sess2.confirmed_frame(),
        sess2.current_frame(),
    );

    // (ii) Mesh agreement: each survivor's confirmed frame advances past the
    // entire pre-detection stall window (`F + MAX_PREDICTION`), which is only
    // possible once the dropped slot is mesh-agreed-excluded on both sides.
    assert!(
        sess1.confirmed_frame().as_i32() > pinned_conf_a.as_i32() + MAX_PREDICTION as i32,
        "sess1 confirmed_frame must advance past the stalled window after mesh agreement; \
         got {:?} (pinned at {pinned_conf_a:?})",
        sess1.confirmed_frame()
    );
    assert!(
        sess2.confirmed_frame().as_i32() > pinned_conf_b.as_i32() + MAX_PREDICTION as i32,
        "sess2 confirmed_frame must advance past the stalled window after mesh agreement; \
         got {:?} (pinned at {pinned_conf_b:?})",
        sess2.confirmed_frame()
    );

    // (iii) Oracle: byte-equality of recorded confirmed state across survivors.
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
        "N0 clean-drop liveness: confirmed state diverged across survivors \
         (bound={confirmed_bound}, compared={compared}): {divergences:?}"
    );

    Ok(())
}

/// Liveness regression (the connect-status *nudge*, N=4 staggered mutual-mute):
/// with THREE survivors holding STAGGERED receipts of the dying peer D
/// (A high, B middle, C low), all capped at detection, only the min-receipt
/// survivor C regains headroom post-detection (its gossip-only bound folds the
/// other two's stale-HIGHER cached views). C's released Inputs deliver its
/// `disconnected@LOW` to A and B — but that is not mesh agreement for them:
/// A still caches B's stale `connected` view of D and B still caches A's, both
/// of which bound A and B at `LOW` (C's stale cache), so each waits forever for
/// the OTHER mute peer's gossip. Pre-nudge this mutual-mute deadlock pins A and
/// B permanently (C also re-pins once it burns the lifted window); post-nudge
/// the idle endpoints' status-bearing duplicate Inputs cross, mesh agreement
/// excludes the slot everywhere, and all three run free and converge D's
/// freeze frame to the global-min `LOW`.
///
/// FAILS before the nudge (A and B make zero frame progress after the drop)
/// and PASSES after.
#[test]
fn p2p_n0_quad_staggered_receipts_mutual_mute_drop_stays_live_and_converges(
) -> Result<(), FortressError> {
    const MAX_PREDICTION: usize = 2;
    // Short symmetric timeouts on the three survivors (all detect the silent D
    // via their OWN auto-timeout while capped); D's timeout is irrelevant (it
    // goes silent and is never polled again).
    let (mut sess1, mut sess2, mut sess3, mut sess4, blocked, a1, a2, a3, a4, clock) =
        build_filtered_four_player_sessions_with_timeouts_and_prediction(
            [
                Duration::from_millis(400),
                Duration::from_millis(400),
                Duration::from_millis(400),
                Duration::from_secs(3),
            ],
            MAX_PREDICTION,
        )?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();
    let mut stub4 = GameStub::new();

    let mut states1: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states2: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states3: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // One ladder round: each session polls IMMEDIATELY before its own step, in
    // A -> B -> C -> D order, so every packet a session sends carries its
    // maximally fresh receipts as gossip (see the 3-player small-window test's
    // ladder note). D's inputs are DISTINCT per frame (i + 7000) so divergent
    // freeze frames surface as divergent bytes. Near-zero clock steps keep all
    // timeouts far away.
    macro_rules! ladder_round {
        ($i:expr) => {{
            sess1.poll_remote_clients();
            try_advance_recording(
                &mut sess1,
                &mut stub1,
                PlayerHandle::new(0),
                $i,
                &mut states1,
            )?;
            sess2.poll_remote_clients();
            try_advance_recording(
                &mut sess2,
                &mut stub2,
                PlayerHandle::new(1),
                $i + 1000,
                &mut states2,
            )?;
            sess3.poll_remote_clients();
            try_advance_recording(
                &mut sess3,
                &mut stub3,
                PlayerHandle::new(2),
                $i + 3000,
                &mut states3,
            )?;
            sess4.poll_remote_clients();
            try_advance_recording(
                &mut sess4,
                &mut stub4,
                PlayerHandle::new(3),
                $i + 7000,
                &mut sink,
            )?;
            clock.advance(Duration::from_millis(1));
        }};
    }

    // --- Phase 1: warmup ladder, all links open. ---
    for i in 0..6_u32 {
        ladder_round!(i);
    }

    // --- Phase 2: staggered loss of D. Block D -> C first (C's receipt of D
    // freezes LOWEST), two rounds later D -> B (middle), while D keeps
    // delivering to A (highest). The lagging LOW gossip from C bounds every
    // survivor's D slot, so all three pin at `LOW` and burn their entire
    // window; their LAST pre-cap sends carry their staggered receipts of D as
    // gossip — the stale cached views the post-detection bounds fold. ---
    blocked.block(a4, a3);
    for i in 0..2_u32 {
        ladder_round!(i + 20);
    }
    blocked.block(a4, a2);
    for i in 0..8_u32 {
        ladder_round!(i + 40);
    }
    let pinned_conf_a = sess1.confirmed_frame();
    let pinned_conf_b = sess2.confirmed_frame();
    let pinned_conf_c = sess3.confirmed_frame();
    // Choreography preconditions: every survivor has burned its whole
    // prediction window pre-detection (capped, fully-acked queue =
    // gossip-mute). The pinned confs need not be exactly equal — the N=4
    // ladder's larger gossip lag legitimately pins the low receiver a frame
    // lower — only the fully-burned windows matter for the mutual-mute setup.
    // If these fail the repro setup is broken (not the code under test).
    assert_eq!(
        sess1.current_frame().as_i32(),
        pinned_conf_a.as_i32() + MAX_PREDICTION as i32,
        "choreography: A must have burned its whole prediction window pre-detection"
    );
    assert_eq!(
        sess2.current_frame().as_i32(),
        pinned_conf_b.as_i32() + MAX_PREDICTION as i32,
        "choreography: B must have burned its whole prediction window pre-detection"
    );
    assert_eq!(
        sess3.current_frame().as_i32(),
        pinned_conf_c.as_i32() + MAX_PREDICTION as i32,
        "choreography: C must have burned its whole prediction window pre-detection"
    );

    // --- Phase 3: D goes fully silent (last link cut, never polled again).
    // Poll-only with full clock steps until ALL THREE survivors' own 400ms
    // timeouts drop D — every detection happens while capped and gossip-mute. ---
    blocked.block(a4, a1);
    let mut sess1_dropped = false;
    let mut sess2_dropped = false;
    let mut sess3_dropped = false;
    for _ in 0..40 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
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
        if sess3
            .events()
            .any(|e| matches!(e, FortressEvent::PeerDropped { .. }))
        {
            sess3_dropped = true;
        }
        if sess1_dropped && sess2_dropped && sess3_dropped {
            break;
        }
    }
    assert!(
        sess1_dropped && sess2_dropped && sess3_dropped,
        "choreography: all three survivors must auto-drop the silent D \
         (A: {sess1_dropped}, B: {sess2_dropped}, C: {sess3_dropped})"
    );
    let stall_cur_a = sess1.current_frame();
    let stall_cur_b = sess2.current_frame();
    let stall_cur_c = sess3.current_frame();

    // --- Phase 4 (the liveness assertion target): BOUNDED release loop with
    // full clock steps, driving all three survivors. Pre-nudge only C moves
    // (and only by the stagger), then the mutual-mute pin holds A and B at
    // zero progress forever. Post-nudge the status-bearing duplicate Inputs
    // deliver every survivor's `disconnected` view to every other; mesh
    // agreement excludes D's slot everywhere and all three run free. ---
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
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
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            3500,
            &mut states3,
        )?;
    }

    // (i) Liveness: all three survivors made frame progress after the drop.
    assert!(
        sess1.current_frame() > stall_cur_a
            && sess2.current_frame() > stall_cur_b
            && sess3.current_frame() > stall_cur_c,
        "N=4 mutual-mute liveness: all three survivors must make frame progress after the drop; \
         conf_a={:?}/cur_a={:?}, conf_b={:?}/cur_b={:?}, conf_c={:?}/cur_c={:?} \
         (pinned_conf={pinned_conf_a:?}, stalls: a={stall_cur_a:?}, b={stall_cur_b:?}, \
          c={stall_cur_c:?})",
        sess1.confirmed_frame(),
        sess1.current_frame(),
        sess2.confirmed_frame(),
        sess2.current_frame(),
        sess3.confirmed_frame(),
        sess3.current_frame(),
    );

    // (ii) Mesh agreement: every survivor's confirmed frame advances past its
    // entire pre-detection stall window, which is only possible once D's slot
    // is mesh-agreed-excluded on all three.
    for (name, conf, pinned) in [
        ("sess1", sess1.confirmed_frame(), pinned_conf_a),
        ("sess2", sess2.confirmed_frame(), pinned_conf_b),
        ("sess3", sess3.confirmed_frame(), pinned_conf_c),
    ] {
        assert!(
            conf.as_i32() > pinned.as_i32() + MAX_PREDICTION as i32,
            "{name} confirmed_frame must advance past the stalled window after mesh agreement; \
             got {conf:?} (pinned at {pinned:?})"
        );
    }

    // (iii) Oracle: byte-equality of recorded confirmed state across all three
    // survivor pairs.
    let confirmed_bound = [
        sess1.confirmed_frame().as_i32(),
        sess2.confirmed_frame().as_i32(),
        sess3.confirmed_frame().as_i32(),
    ]
    .into_iter()
    .min()
    .unwrap_or(i32::MIN);
    let mut compared = 0_u32;
    let mut divergences: Vec<(&'static str, i32, StateStub, StateStub)> = Vec::new();
    for (pair_name, lhs, rhs) in [
        ("A-B", &states1, &states2),
        ("A-C", &states1, &states3),
        ("B-C", &states2, &states3),
    ] {
        for (&frame, &state_l) in lhs.iter() {
            if frame > confirmed_bound {
                continue;
            }
            if let Some(&state_r) = rhs.get(&frame) {
                compared += 1;
                if state_l != state_r {
                    divergences.push((pair_name, frame, state_l, state_r));
                }
            }
        }
    }
    assert!(
        compared > 0,
        "no confirmed frames were compared across survivors (bound={confirmed_bound}); \
         the repro did not exercise the drop path"
    );
    assert!(
        divergences.is_empty(),
        "N=4 mutual-mute: confirmed state diverged across survivors \
         (bound={confirmed_bound}, compared={compared}): {divergences:?}"
    );

    Ok(())
}

/// Liveness regression (round-3 F-NEW-A: the connect-status nudge must not
/// starve the receiver's pending-output retransmission timer): the clean
/// equal-receipt drop above, plus a single ~600ms `B -> A` blackout spanning
/// the nudge-arming window.
///
/// Choreography: C dies cleanly (zero stagger), both survivors burn their
/// whole `max_prediction = 1` window and auto-drop C while capped and
/// gossip-mute (nudges arm). The `B -> A` blackout begins immediately after
/// both detections, so the very first nudge volley is one-directional: A's
/// nudge reaches B (B mesh-agrees, releases, advances, and sends its fresh
/// post-agreement Input to A — LOST in the blackout, parked in B's
/// `pending_output`), while everything B-to-A is dropped (A never mesh-agrees
/// and keeps nudging).
///
/// The pin (pre-fix): after the link restores, A's nudges arrive at B on the
/// keepalive cadence (200ms, equal to `running_retry_interval`) and every
/// decodable Input packet — even one staging ZERO new frames — reset B's
/// `running_last_input_recv`. Messages are handled before the endpoint poll's
/// retry check inside the same `poll_remote_clients` call, so on the shared
/// 50ms poll grid every retry-eligible tick coincides with a fresh nudge
/// arrival: B's pending Input (the ONLY carrier of B's post-agreement
/// `disconnected@F` view — B no longer nudges once IT mesh-agrees) never
/// retransmits, A never mesh-agrees, A nudges forever, and both survivors pin
/// permanently (alive but zero frame progress).
///
/// The fix gates the `running_last_input_recv` reset in `on_input` on staged
/// progress: a zero-new-frames Input (nudge or duplicate retransmission) no
/// longer suppresses the retry, so B's pending Input retransmits within one
/// retry interval of the unblock, A mesh-agrees, and both run free.
///
/// FAILS before the gate (permanent pin: zero frame progress in the bounded
/// release loop) and PASSES after.
#[test]
fn p2p_n0_nudge_does_not_starve_pending_retransmission_after_blackout() -> Result<(), FortressError>
{
    const MAX_PREDICTION: usize = 1;
    // Survivor timeouts must EXCEED the 600ms blackout (or A would simply
    // auto-drop B mid-blackout and the repro would degrade into a different,
    // two-survivor scenario); C's timeout is irrelevant (it goes silent and is
    // never polled again).
    let (mut sess1, mut sess2, mut sess3, blocked, a1, a2, a3, clock) =
        build_filtered_three_player_sessions_with_timeouts_and_prediction(
            [
                Duration::from_secs(1),
                Duration::from_secs(1),
                Duration::from_secs(3),
            ],
            MAX_PREDICTION,
        )?;

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    let mut states1: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut states2: BTreeMap<i32, StateStub> = BTreeMap::new();
    let mut sink: BTreeMap<i32, StateStub> = BTreeMap::new();

    // --- Phase 1: warmup ladder, ALL LINKS OPEN (clean-network precondition).
    // Distinct C inputs (i + 3000) make divergent freeze frames visible as
    // divergent bytes. Near-zero clock steps keep all timeouts far away. ---
    for i in 0..6_u32 {
        sess1.poll_remote_clients();
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            i,
            &mut states1,
        )?;
        sess2.poll_remote_clients();
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1000,
            &mut states2,
        )?;
        sess3.poll_remote_clients();
        try_advance_recording(
            &mut sess3,
            &mut stub3,
            PlayerHandle::new(2),
            i + 3000,
            &mut sink,
        )?;
        clock.advance(Duration::from_millis(1));
    }

    // --- Phase 2: C dies CLEANLY — both survivor links cut at the same
    // instant (zero stagger, equal receipts). Both survivors burn their whole
    // one-frame prediction window against the frozen receipt. ---
    blocked.block(a3, a2);
    blocked.block(a3, a1);
    for i in 0..10_u32 {
        sess1.poll_remote_clients();
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            i + 20,
            &mut states1,
        )?;
        sess2.poll_remote_clients();
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            i + 1020,
            &mut states2,
        )?;
        clock.advance(Duration::from_millis(1));
    }
    let pinned_conf_a = sess1.confirmed_frame();
    let pinned_conf_b = sess2.confirmed_frame();
    // Choreography preconditions: equal receipts pin both survivors at the
    // same `F` with the whole window burned. If these fail the repro setup is
    // broken (not the code under test).
    assert_eq!(
        pinned_conf_a, pinned_conf_b,
        "choreography: zero stagger must pin both survivors at the same F"
    );
    assert_eq!(
        sess1.current_frame().as_i32(),
        pinned_conf_a.as_i32() + MAX_PREDICTION as i32,
        "choreography: A must have burned its whole prediction window pre-detection"
    );
    assert_eq!(
        sess2.current_frame().as_i32(),
        pinned_conf_b.as_i32() + MAX_PREDICTION as i32,
        "choreography: B must have burned its whole prediction window pre-detection"
    );

    // --- Phase 3: poll-only with full clock steps until BOTH survivors' own
    // 1s timeouts drop the silent C. Equal timeouts + equal silence start + A
    // polling first each iteration mean A detects no later than B, so when
    // the loop breaks NO nudge has fired yet (the nudge flag arms on the poll
    // AFTER a detection lands) — the blackout below therefore spans the whole
    // nudge-arming window. ---
    let mut sess1_dropped = false;
    let mut sess2_dropped = false;
    for _ in 0..60 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
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
        if sess1_dropped && sess2_dropped {
            break;
        }
    }
    assert!(
        sess1_dropped && sess2_dropped,
        "choreography: both survivors must auto-drop the silent C \
         (A dropped: {sess1_dropped}, B dropped: {sess2_dropped})"
    );

    // --- Phase 4: ~600ms B -> A blackout spanning the nudge-arming window.
    // A's first nudge reaches B (open direction): B mesh-agrees, releases,
    // advances, and its fresh post-agreement Input to A is lost — parked in
    // B's `pending_output` as the only carrier of B's `disconnected@F` view.
    // Every B -> A packet is dropped, so A stays unagreed and keeps nudging. ---
    blocked.block(a2, a1);
    for _ in 0..12 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        try_advance_recording(
            &mut sess1,
            &mut stub1,
            PlayerHandle::new(0),
            300,
            &mut states1,
        )?;
        try_advance_recording(
            &mut sess2,
            &mut stub2,
            PlayerHandle::new(1),
            1300,
            &mut states2,
        )?;
    }
    blocked.unblock(a2, a1);
    let stall_cur_a = sess1.current_frame();
    let stall_cur_b = sess2.current_frame();

    // --- Phase 5 (the liveness assertion target): BOUNDED release loop with
    // full clock steps. Pre-fix nothing moves, forever: A's nudges (one per
    // keepalive interval) keep resetting B's retry timer before the retry
    // check runs, B's pending Input never retransmits, A never mesh-agrees.
    // Post-fix B's retry fires within one interval of the unblock, its
    // retransmitted Input delivers `disconnected@F`, A mesh-agrees, and both
    // run free. ---
    for _ in 0..200 {
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

    // (i) Liveness: both survivors made frame progress after the blackout.
    assert!(
        sess1.current_frame() > stall_cur_a && sess2.current_frame() > stall_cur_b,
        "N0 nudge/retry starvation: both survivors must make frame progress after the \
         blackout lifts; conf_a={:?}/cur_a={:?}, conf_b={:?}/cur_b={:?} \
         (pinned_conf={pinned_conf_a:?}, stall_cur_a={stall_cur_a:?}, \
          stall_cur_b={stall_cur_b:?})",
        sess1.confirmed_frame(),
        sess1.current_frame(),
        sess2.confirmed_frame(),
        sess2.current_frame(),
    );

    // (ii) Mesh agreement: each survivor's confirmed frame advances past the
    // entire pre-detection stall window, only possible once the dropped slot
    // is mesh-agreed-excluded on both sides.
    assert!(
        sess1.confirmed_frame().as_i32() > pinned_conf_a.as_i32() + MAX_PREDICTION as i32,
        "sess1 confirmed_frame must advance past the stalled window after mesh agreement; \
         got {:?} (pinned at {pinned_conf_a:?})",
        sess1.confirmed_frame()
    );
    assert!(
        sess2.confirmed_frame().as_i32() > pinned_conf_b.as_i32() + MAX_PREDICTION as i32,
        "sess2 confirmed_frame must advance past the stalled window after mesh agreement; \
         got {:?} (pinned at {pinned_conf_b:?})",
        sess2.confirmed_frame()
    );

    // (iii) Oracle: byte-equality of recorded confirmed state across survivors.
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
        "N0 nudge/retry starvation: confirmed state diverged across survivors \
         (bound={confirmed_bound}, compared={compared}): {divergences:?}"
    );

    Ok(())
}

// ============================================================================
// Integration-level N-peer hot-join joiner lifecycle (toward fold-below-`S`)
// ============================================================================
//
// THE MECHANISM (traced, for context): an N-peer hot-join JOINER applies a
// snapshot at frame `S` and inherits a "carried-dead" slot (a peer that died
// before the snapshot) frozen at the coordinator's captured view
// `f_carried < S`. This sits stably. The fold-below-`S` fail-closed violation
// fires inside `disconnect_player_at_frames` (`src/sessions/p2p_session.rs`)
// ONLY when a relay delivers a freeze STRICTLY BELOW `f_carried` (hence below
// `S`) for the carried-dead slot: a LOWERING `update_player_disconnects` fold
// converges `converged < S = npeer_joiner_baseline`, and the joiner — whose
// entire history starts at `S` — structurally cannot re-roll or re-simulate
// below its only saved state, so it fails closed (leave `Running`, surface
// `NotSynchronized`, tear down terminally). The authoritative coverage of that
// fail-closed behavior is the in-crate unit guard
// `npeer_quad_committed_joiner_fold_below_snapshot_baseline_fails_closed_for_rejoin`,
// which injects the below-`S` claim via a private cache seam.
//
// WHAT THIS MODULE LANDS (the reusable harness + the two legs reachable over a
// genuine `FilterBusSocket` wire — every prune an explicit `remove_player`, no
// seam injection, no `set_peer_connect_status_for_tests`, no private-field
// writes): (1) a real `start_hot_join_session` N-peer joiner driven to
// `Running` over the wire (the first such integration test — the full N-peer
// joiner-drive lifecycle previously lived ONLY in `src/` unit tests on the
// in-process `MeshBus`); (2) that committed joiner genuinely CARRYING a
// dead slot disconnected BELOW its snapshot baseline `S` (the foundational leg
// of the corner), proven via the PUBLIC per-slot `InputStatus` threaded into
// each `AdvanceFrame` request (`local_connect_status` is private).
//
// WHY THE VIOLATION TRIGGER IS STRUCTURALLY UNREACHABLE OVER A GENUINE WIRE
// (Session 60 — the resolution of S58's "open question / roster tension"). The
// strongest natural attacker is an N>=5 double-failure relay aimed at the fresh
// joiner: A=h0 coordinator/HOST holding the dead slot HIGH and serving; B=h1
// RELAY; C=h2 the joiner's re-filled slot; D=h3 the carried-dead slot (dies
// asymmetrically); E=h4 the low-origin (the only peer to freeze D at the low
// receipt `g`). It cannot reach the violation, for three composing code reasons:
//   1. The coordinator serves at `S` only when `confirmed_frame() >= S` AND
//      `npeer_owed_freeze_readjust_at_or_below(S) == false` — and that gate folds
//      EVERY running non-reserved endpoint's claims, so the serve DEFERS (pre-
//      capture) / ABORTS (post-capture) while ANY folded endpoint reports a slot
//      freeze that would re-sim at/below `S`. So A never serves D@f_carried while
//      any folded peer (origin OR relay) holds D@`g` (`g < S`).
//   2. To carry D@f_carried > `g`, A must EXCLUDE the low-origin from its fold
//      (a connected remote reporting D@low also bounds A's confirmed low via
//      `remote_slot_confirmed_bound`), and the only exclusion that does not stall
//      A is to PRUNE it. Pruning gossips `origin@disconnected` to A's survivors.
//   3. A Disconnected protocol endpoint drops all `Input`
//      (`message_allowed_in_current_state` admits only `SyncRequest`), so any
//      survivor that folds `origin@disconnected` disconnects its origin endpoint
//      and can never adopt `g`. The relay that delivers `g` to the joiner MUST be
//      a coordinator-directed survivor (only a survivor reactivating the joiner's
//      slot gets a live endpoint to it) — hence A-folded, hence cascaded.
// So the joiner commits carrying D below `S` and STAYS Running: no wire ordering
// delivers a sub-baseline freeze. This is exercised end-to-end (over the wire,
// strongest-attacker staging) by
// `npeer_hot_join_fold_below_s_trigger_unreachable_over_wire` below, and the
// seam-injected `src/` unit guard proves the bytes WOULD fail closed if delivered
// — so that guard is the authoritative (and only-possible) coverage. Full
// argument + adversarial-review consensus: `progress/session-60-…`.
#[cfg(feature = "hot-join")]
mod npeer_hot_join_joiner_integration {
    use super::{protocol_config, FilterBusSocket};
    use crate::common::stubs::{GameStub, StubConfig, StubInput};
    use crate::common::{BlockedLinks, RoutingBus, TestClock, POLL_INTERVAL_DETERMINISTIC};
    use fortress_rollback::{
        DesyncDetection, DisconnectBehavior, FortressError, FortressEvent, FortressRequest, Frame,
        InputStatus, P2PSession, PlayerHandle, PlayerType, SessionBuilder, SessionState,
    };
    use std::collections::BTreeMap;
    use std::net::SocketAddr;
    use web_time::Duration;

    /// Handles, in roster order: A=host(0), B=relay(1), C=joiner-slot(2),
    /// D=carried-dead(3), E=survivor(4).
    const H_A: PlayerHandle = PlayerHandle::new(0);
    const H_B: PlayerHandle = PlayerHandle::new(1);
    const H_C: PlayerHandle = PlayerHandle::new(2);
    const H_D: PlayerHandle = PlayerHandle::new(3);
    const H_E: PlayerHandle = PlayerHandle::new(4);

    /// Returns the five mesh addresses (index-aligned to handles 0..5).
    fn mesh_addrs() -> [SocketAddr; 5] {
        [
            ([127, 0, 0, 1], 50001).into(),
            ([127, 0, 0, 1], 50002).into(),
            ([127, 0, 0, 1], 50003).into(),
            ([127, 0, 0, 1], 50004).into(),
            ([127, 0, 0, 1], 50005).into(),
        ]
    }

    /// The five live full-mesh sessions plus the wire handle and addresses.
    /// A is the hot-join coordinator (`with_hot_join(true)`); the rest are plain
    /// survivors. LONG symmetric timeouts so NO auto-timeout ever fires — every
    /// prune in the choreography is an explicit `remove_player`. Sockets are
    /// [`FilterBusSocket`]s over a shared [`RoutingBus`], so (a) directional loss
    /// is toggleable mid-run via `blocked`, and (b) a fresh joiner can re-attach at
    /// C's vacated address after C drops (the hot-join rejoin needs both — neither
    /// `FilterSocket`/`ChannelSocket` nor a plain `BusSocket` alone suffices).
    struct Mesh5 {
        a: P2PSession<StubConfig>,
        b: P2PSession<StubConfig>,
        e: P2PSession<StubConfig>,
        /// C is `Option` so the test can `drop` it (vacating its socket/address)
        /// before re-attaching the joiner at the same address.
        c: Option<P2PSession<StubConfig>>,
        /// D is `Option` so the test can `drop` it after D dies (vacating its
        /// address — D is never rejoined, it stays carried-dead).
        d: Option<P2PSession<StubConfig>>,
        stub_a: GameStub,
        stub_b: GameStub,
        stub_c: GameStub,
        stub_d: GameStub,
        stub_e: GameStub,
        bus: RoutingBus,
        blocked: BlockedLinks,
        addrs: [SocketAddr; 5],
        clock: TestClock,
    }

    /// Builds + synchronizes the five-player hot-join mesh. A=h0 is the
    /// coordinator; the snapshot serve uses `EveryFrame` saving (StubConfig
    /// default) and input delay 0 (default), so the public `start_hot_join_session`
    /// works for the joiner that re-fills C's slot.
    // The five sessions bind to single-char roster locals `a`..`e` (mirroring the
    // `Mesh5` fields and the `H_A`..`H_E` handles) — intentional and clearer than
    // `sess_a`..`sess_e` for a fixed five-peer roster.
    #[allow(clippy::many_single_char_names)]
    fn build_mesh5() -> Result<Mesh5, FortressError> {
        let clock = TestClock::new();
        let bus = RoutingBus::new();
        let blocked = BlockedLinks::new();
        let addrs = mesh_addrs();
        let pc = protocol_config(&clock);
        let long = Duration::from_secs(20);

        // The coordinator A: hot-join enabled.
        let a = {
            let mut builder = SessionBuilder::<StubConfig>::new()
                .with_protocol_config(pc.clone())
                .with_num_players(5)?
                .with_hot_join(true)
                .with_desync_detection_mode(DesyncDetection::On { interval: 2 })
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .with_disconnect_timeout(long)
                .with_disconnect_notify_delay(Duration::from_millis(100))
                .add_player(PlayerType::Local, H_A)?;
            for (h, idx) in [(H_B, 1), (H_C, 2), (H_D, 3), (H_E, 4)] {
                builder = builder.add_player(PlayerType::Remote(addrs[idx]), h)?;
            }
            builder.start_p2p_session(FilterBusSocket::new(&bus, addrs[0], blocked.clone()))?
        };

        let build_survivor = |local: PlayerHandle,
                              local_idx: usize,
                              remotes: [(PlayerHandle, usize); 4]|
         -> Result<P2PSession<StubConfig>, FortressError> {
            let mut builder = SessionBuilder::<StubConfig>::new()
                .with_protocol_config(pc.clone())
                .with_num_players(5)?
                .with_desync_detection_mode(DesyncDetection::On { interval: 2 })
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .with_disconnect_timeout(long)
                .with_disconnect_notify_delay(Duration::from_millis(100))
                .add_player(PlayerType::Local, local)?;
            for (h, idx) in remotes {
                builder = builder.add_player(PlayerType::Remote(addrs[idx]), h)?;
            }
            builder.start_p2p_session(FilterBusSocket::new(
                &bus,
                addrs[local_idx],
                blocked.clone(),
            ))
        };

        let b = build_survivor(H_B, 1, [(H_A, 0), (H_C, 2), (H_D, 3), (H_E, 4)])?;
        let c = build_survivor(H_C, 2, [(H_A, 0), (H_B, 1), (H_D, 3), (H_E, 4)])?;
        let d = build_survivor(H_D, 3, [(H_A, 0), (H_B, 1), (H_C, 2), (H_E, 4)])?;
        let e = build_survivor(H_E, 4, [(H_A, 0), (H_B, 1), (H_C, 2), (H_D, 3)])?;

        let mut mesh = Mesh5 {
            a,
            b,
            e,
            c: Some(c),
            d: Some(d),
            stub_a: GameStub::new(),
            stub_b: GameStub::new(),
            stub_c: GameStub::new(),
            stub_d: GameStub::new(),
            stub_e: GameStub::new(),
            bus,
            blocked,
            addrs,
            clock,
        };
        mesh.synchronize(1200);
        let _ = mesh.a.events().count();
        let _ = mesh.b.events().count();
        if let Some(c) = mesh.c.as_mut() {
            let _ = c.events().count();
        }
        if let Some(d) = mesh.d.as_mut() {
            let _ = d.events().count();
        }
        let _ = mesh.e.events().count();
        Ok(mesh)
    }

    impl Mesh5 {
        fn synchronize(&mut self, iterations: usize) {
            for _ in 0..iterations {
                self.poll_all(1);
                if self.a.current_state() == SessionState::Running
                    && self.b.current_state() == SessionState::Running
                    && self
                        .c
                        .as_ref()
                        .is_none_or(|c| c.current_state() == SessionState::Running)
                    && self
                        .d
                        .as_ref()
                        .is_none_or(|d| d.current_state() == SessionState::Running)
                    && self.e.current_state() == SessionState::Running
                {
                    return;
                }
            }
            panic!(
                "five hot-join mesh sessions failed to synchronize: A={:?} B={:?} C={:?} D={:?} E={:?}",
                self.a.current_state(),
                self.b.current_state(),
                self.c.as_ref().map(P2PSession::current_state),
                self.d.as_ref().map(P2PSession::current_state),
                self.e.current_state()
            );
        }

        /// Polls all currently-live sessions (A, B, D, E, and C if present)
        /// `iterations` times, advancing the clock each iteration.
        fn poll_all(&mut self, iterations: usize) {
            for _ in 0..iterations {
                self.a.poll_remote_clients();
                self.b.poll_remote_clients();
                self.e.poll_remote_clients();
                if let Some(c) = self.c.as_mut() {
                    c.poll_remote_clients();
                }
                if let Some(d) = self.d.as_mut() {
                    d.poll_remote_clients();
                }
                self.clock.advance(POLL_INTERVAL_DETERMINISTIC);
            }
        }

        /// Advances every currently-live survivor one frame on distinct,
        /// per-peer input streams (so any frozen value is frame-sensitive, never
        /// vacuously byte-equal). `base + handle` keeps the streams distinct.
        fn advance_all(&mut self, base: u32) {
            try_advance(&mut self.a, &mut self.stub_a, H_A, base);
            try_advance(&mut self.b, &mut self.stub_b, H_B, base + 100);
            try_advance(&mut self.e, &mut self.stub_e, H_E, base + 400);
            if let Some(c) = self.c.as_mut() {
                try_advance(c, &mut self.stub_c, H_C, base + 200);
            }
            if let Some(d) = self.d.as_mut() {
                try_advance(d, &mut self.stub_d, H_D, base + 300);
            }
        }

        /// Moves D out and drops it (vacating D's address; D never rejoins).
        fn drop_d(&mut self) {
            if let Some(d) = self.d.take() {
                drop(d);
            }
        }

        /// Settles in-flight inputs (poll-only rounds) so every live peer's
        /// receipt of every other converges — a subsequent drop then freezes the
        /// slot at ONE frame mesh-wide.
        fn settle(&mut self, polls: usize) {
            self.poll_all(polls);
        }
    }

    /// A real N-peer hot-join joiner re-filling C's slot (local handle 2),
    /// registering the full roster — including the carried-DEAD slot D at D's
    /// address (a rejoiner knows the session shape, not who is alive; its
    /// endpoint toward the dead address simply never synchronizes, and the
    /// snapshot's carried statuses tell it the slot is frozen). Built through the
    /// PUBLIC `start_hot_join_session` (S35 guard lift) — no test bypass.
    struct Joiner {
        session: P2PSession<StubConfig>,
        stub: GameStub,
        events: Vec<FortressEvent<StubConfig>>,
        /// Per-frame `(value, InputStatus)` the joiner surfaces for the
        /// carried-dead slot D in its `AdvanceFrame` requests (last write per
        /// frame is the post-rollback value). A public proxy for "the joiner
        /// carries D disconnected": `local_connect_status` is private, but the
        /// per-slot status threaded into every `AdvanceFrame` is observable.
        d_slot: BTreeMap<i32, (u32, InputStatus)>,
    }

    impl Joiner {
        /// Builds the joiner at C's (vacated) address over a fresh
        /// [`FilterBusSocket`] sharing the mesh bus + blocked-links handle.
        fn build(mesh: &Mesh5) -> Result<Self, FortressError> {
            let addrs = mesh.addrs;
            let socket = FilterBusSocket::new(&mesh.bus, addrs[2], mesh.blocked.clone());
            let session = SessionBuilder::<StubConfig>::new()
                .with_protocol_config(protocol_config(&mesh.clock))
                .with_num_players(5)?
                .with_desync_detection_mode(DesyncDetection::On { interval: 2 })
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .add_player(PlayerType::Remote(addrs[0]), H_A)?
                .add_player(PlayerType::Remote(addrs[1]), H_B)?
                .add_player(PlayerType::Local, H_C)?
                .add_player(PlayerType::Remote(addrs[3]), H_D)?
                .add_player(PlayerType::Remote(addrs[4]), H_E)?
                .start_hot_join_session(socket, addrs[0])?;
            Ok(Self {
                session,
                stub: GameStub::new(),
                events: Vec::new(),
                d_slot: BTreeMap::new(),
            })
        }

        fn poll(&mut self) {
            self.session.poll_remote_clients();
            self.events.extend(self.session.events());
        }

        /// Applies the joiner's advance requests into its stub, recording the
        /// D-slot `(value, status)` from each `AdvanceFrame` (re-simulations
        /// re-key the same frame, so the last write per frame is post-rollback).
        fn apply_recording(&mut self, requests: fortress_rollback::RequestVec<StubConfig>) {
            const D: usize = 3;
            for request in requests {
                match request {
                    FortressRequest::LoadGameState { cell, .. } => {
                        self.stub.gs = cell.load().expect("joiner load cell");
                    },
                    FortressRequest::SaveGameState { cell, frame } => {
                        let checksum = crate::common::calculate_hash(&self.stub.gs);
                        cell.save(frame, Some(self.stub.gs), Some(checksum as u128));
                    },
                    FortressRequest::AdvanceFrame { inputs } => {
                        let d = inputs.get(D).map(|&(input, status)| (input.inp, status));
                        self.stub.gs.advance_frame_pub(inputs);
                        if let Some(vs) = d {
                            self.d_slot.insert(self.stub.gs.frame, vs);
                        }
                    },
                }
            }
        }

        /// Drives one local-input + advance on the joiner, recording D's slot.
        /// Tolerates the prediction cap (the fold still ran).
        fn advance_recording(&mut self, value: u32) {
            match self.session.add_local_input(H_C, StubInput { inp: value }) {
                Ok(()) => {},
                Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => return,
                Err(other) => panic!("joiner add_local_input error: {other:?}"),
            }
            match self.session.advance_frame() {
                Ok(requests) => self.apply_recording(requests),
                Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
                Err(other) => panic!("joiner advance_frame error: {other:?}"),
            }
        }
    }

    /// Advances a live survivor one frame with a deterministic local input,
    /// applying the resulting requests into `stub`. Tolerates the prediction cap
    /// and `NotSynchronized` (the fold still ran inside `advance_frame`).
    fn try_advance(
        session: &mut P2PSession<StubConfig>,
        stub: &mut GameStub,
        handle: PlayerHandle,
        value: u32,
    ) {
        match session.add_local_input(handle, StubInput { inp: value }) {
            Ok(()) => {},
            Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => return,
            Err(other) => panic!("unexpected add_local_input error: {other:?}"),
        }
        match session.advance_frame() {
            Ok(requests) => stub.handle_requests(requests),
            Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
            Err(other) => panic!(
                "unexpected advance_frame error for handle {handle:?} at current {:?}: {other:?}",
                session.current_frame()
            ),
        }
    }

    /// Drops slot D mesh-wide (symmetric freeze) and frees D's session, leaving
    /// D carried-dead. The surviving mesh then advances so the eventual snapshot
    /// frame `S` sits strictly above D's freeze frame (so a joiner inherits D
    /// disconnected BELOW its baseline `S`). D is NEVER rejoined.
    ///
    /// Returns an UPPER BOUND on D's freeze frame: the coordinator A's
    /// `current_frame` at the moment of the drop. A froze D at its own received
    /// frame for D, which cannot exceed A's current frame — so any later
    /// snapshot frame `S` strictly above this witness is provably strictly above
    /// D's freeze. The caller asserts `S > witness` to make "below `S`" rigorous
    /// rather than merely staged.
    fn drop_d_symmetric(mesh: &mut Mesh5) -> Frame {
        mesh.settle(8);
        mesh.a.remove_player(H_D).expect("A removes D");
        mesh.b.remove_player(H_D).expect("B removes D");
        mesh.e.remove_player(H_D).expect("E removes D");
        if let Some(c) = mesh.c.as_mut() {
            c.remove_player(H_D).expect("C removes D");
        }
        // A froze D at <= A's current frame; capture that upper bound before the
        // mesh advances past it.
        let d_freeze_upper_bound = mesh.a.current_frame();
        // Free D's session: its address goes quiet, its slot stays frozen
        // mesh-wide. D never rejoins (it remains carried-dead).
        mesh.drop_d();
        let _ = mesh.a.events().count();
        let _ = mesh.b.events().count();
        let _ = mesh.e.events().count();

        // The surviving live mesh (A, B, C, E) keeps advancing with D frozen, so
        // S climbs strictly above D's freeze frame.
        for i in 0..5_u32 {
            mesh.poll_all(3);
            mesh.advance_all(40 + i);
        }
        d_freeze_upper_bound
    }

    /// Drops slot C cleanly on every live survivor (explicit `remove_player`),
    /// then frees C's session so its address is vacant for the rejoiner. The
    /// hot-join coordinator A re-reserves the slot. Asserts A re-armed the slot.
    fn drop_c_cleanly(mesh: &mut Mesh5) {
        mesh.settle(8);
        mesh.a.remove_player(H_C).expect("A removes C");
        mesh.b.remove_player(H_C).expect("B removes C");
        mesh.e.remove_player(H_C).expect("E removes C");
        if let Some(d) = mesh.d.as_mut() {
            d.remove_player(H_C).expect("D removes C");
        }
        let c = mesh.c.take().expect("C present before drop");
        drop(c);
        let _ = mesh.a.events().count();
        let _ = mesh.b.events().count();
        let _ = mesh.e.events().count();
        if let Some(d) = mesh.d.as_mut() {
            let _ = d.events().count();
        }
        // Let the drop converge mesh-wide.
        for _ in 0..6 {
            mesh.poll_all(3);
            mesh.advance_all(70);
        }
    }

    /// Drives a joiner from "constructed" to "Running real-from-F" over the wire,
    /// keeping the live survivors advancing so the serve can capture + commit.
    /// Returns the snapshot frame `S` (the frame the joiner loads). Asserts the
    /// first post-commit `advance_frame()` is exactly
    /// `[LoadGameState(S), AdvanceFrame(bridge)]`.
    fn drive_joiner_to_running(mesh: &mut Mesh5, joiner: &mut Joiner) -> Frame {
        let mut reached = false;
        for i in 0..1200_u32 {
            mesh.poll_all(1);
            joiner.poll();
            if i % 3 == 2 {
                mesh.advance_all(80 + (i % 8));
            }
            if joiner.session.current_state() == SessionState::Running {
                reached = true;
                break;
            }
        }
        assert!(
            reached,
            "joiner reached Running over the wire (state {:?})",
            joiner.session.current_state()
        );
        drive_first_advance(joiner)
    }

    /// The joiner's first post-commit `advance_frame()`: asserts it is exactly
    /// `[LoadGameState(S), AdvanceFrame(bridge)]`, applies it (recording D), and
    /// returns the snapshot frame `S`.
    fn drive_first_advance(joiner: &mut Joiner) -> Frame {
        let requests = joiner.session.advance_frame().expect("post-commit advance");
        let mut load_frame = None;
        let mut load_count = 0;
        let mut advance_count = 0;
        for request in &*requests {
            match request {
                FortressRequest::LoadGameState { frame, .. } => {
                    load_count += 1;
                    load_frame = Some(*frame);
                },
                FortressRequest::AdvanceFrame { .. } => advance_count += 1,
                FortressRequest::SaveGameState { .. } => {},
            }
        }
        assert_eq!(load_count, 1, "exactly one LoadGameState on first advance");
        assert_eq!(advance_count, 1, "exactly one bridge AdvanceFrame");
        let serve_s = load_frame.expect("a snapshot frame was loaded");
        joiner.apply_recording(requests);
        serve_s
    }

    /// Runs the live mesh + joiner forward together for `rounds`, advancing the
    /// joiner with distinct inputs (recording D's slot) so the joiner settles
    /// into deep post-join operation and its carried-dead D status is observable.
    fn run_mesh_and_joiner(mesh: &mut Mesh5, joiner: &mut Joiner, rounds: u32) {
        for k in 0..rounds {
            mesh.poll_all(2);
            joiner.poll();
            // Advance every round; the prediction cap self-throttles (advance
            // helpers early-return at the cap), and frequent advances let the
            // joiner confirm — and thus record D's carried status on — many
            // distinct post-join frames rather than re-simulating a tiny window.
            mesh.advance_all(90 + (k % 8));
            joiner.advance_recording(95 + (k % 8));
        }
    }

    #[test]
    fn npeer_hot_join_five_player_mesh_builds_and_synchronizes() -> Result<(), FortressError> {
        let mesh = build_mesh5()?;
        assert_eq!(mesh.a.num_players(), 5);
        assert_eq!(mesh.a.current_state(), SessionState::Running);
        assert_eq!(mesh.e.current_state(), SessionState::Running);
        assert_eq!(
            mesh.c.as_ref().map(P2PSession::current_state),
            Some(SessionState::Running)
        );
        // The wire handle is shared and directional toggling works.
        assert!(!mesh.blocked.is_blocked_pub(mesh.addrs[0], mesh.addrs[1]));
        Ok(())
    }

    /// The reusable integration joiner-drive. Warm up the 5-peer mesh, drop C
    /// cleanly, drive a real public `start_hot_join_session` joiner to `Running`
    /// over the wire, and assert the first post-commit advance is exactly the
    /// `[LoadGameState(S), AdvanceFrame(bridge)]` batch. The first integration
    /// test that exercises the N-peer joiner-drive lifecycle over a genuine wire
    /// (previously `src/`-unit-test / `MeshBus`-only).
    #[test]
    fn npeer_hot_join_joiner_drives_to_running_over_filtered_wire() -> Result<(), FortressError> {
        let mut mesh = build_mesh5()?;

        // Warm up so the mesh has confirmed history.
        for i in 0..8_u32 {
            mesh.poll_all(3);
            mesh.advance_all(i);
        }

        drop_c_cleanly(&mut mesh);
        assert!(mesh.c.is_none(), "C vacated its slot");

        let mut joiner = Joiner::build(&mesh)?;
        assert_eq!(joiner.session.current_state(), SessionState::HotJoining);

        let serve_s = drive_joiner_to_running(&mut mesh, &mut joiner);
        assert!(serve_s.as_i32() >= 0, "snapshot frame is real");
        assert_eq!(
            joiner.session.current_state(),
            SessionState::Running,
            "joiner is Running after the load+bridge"
        );
        assert!(
            joiner.session.current_frame().as_i32() > serve_s.as_i32(),
            "joiner advanced past the snapshot frame (real from F={serve_s:?})"
        );
        Ok(())
    }

    /// The foundational leg of the fold-below-`S` corner: stage the carried-dead
    /// slot D (died mesh-wide before the serve) so the committed joiner inherits
    /// it disconnected and frozen strictly BELOW its snapshot baseline `S`, over
    /// a genuine wire. Proven via the PUBLIC per-slot `InputStatus` threaded into
    /// the joiner's `AdvanceFrame` requests: every post-join frame surfaces D as
    /// `Disconnected` with a single constant (genuinely frozen) value. This is
    /// the stable carried-below-`S` state the (unlanded) violation trigger would
    /// then lower further — see the module header for why that delivery is the
    /// open future-work delta.
    #[test]
    fn npeer_hot_join_joiner_commits_carrying_dead_slot_below_snapshot_baseline(
    ) -> Result<(), FortressError> {
        let mut mesh = build_mesh5()?;
        for i in 0..6_u32 {
            mesh.poll_all(3);
            mesh.advance_all(i);
        }

        let d_freeze_upper_bound = drop_d_symmetric(&mut mesh);
        drop_c_cleanly(&mut mesh);

        let mut joiner = Joiner::build(&mesh)?;
        let serve_s = drive_joiner_to_running(&mut mesh, &mut joiner);
        run_mesh_and_joiner(&mut mesh, &mut joiner, 90);

        // "Below S" is rigorous, not merely staged: D's freeze is <= the witness
        // (A's frame when it froze D), and the snapshot baseline S is strictly
        // above that witness — so D is carried frozen strictly below S.
        assert!(
            serve_s.as_i32() > d_freeze_upper_bound.as_i32(),
            "snapshot baseline S={serve_s:?} must sit strictly above D's freeze \
             (upper bound {d_freeze_upper_bound:?}) — else 'below S' is unproven"
        );

        // The joiner must surface D as Disconnected on every frame it recorded,
        // and that frozen value must be constant (a genuine frozen slot). A floor
        // of >= 10 recorded frames keeps the constant-value check non-vacuous (a
        // single-frame sample could not distinguish frozen from coincidental).
        assert!(
            joiner.d_slot.len() >= 10,
            "joiner recorded too few D-slot statuses ({}) — vacuous staging",
            joiner.d_slot.len()
        );
        let frozen_values: std::collections::BTreeSet<u32> =
            joiner.d_slot.values().map(|&(v, _)| v).collect();
        assert_eq!(
            frozen_values.len(),
            1,
            "D's carried value must be frozen (constant) across all {} recorded post-join \
             frames, got {frozen_values:?}",
            joiner.d_slot.len()
        );
        for (frame, (value, status)) in &joiner.d_slot {
            assert_eq!(
                *status,
                InputStatus::Disconnected,
                "joiner must carry D Disconnected at frame {frame} (value {value})"
            );
        }
        // Sanity: the joiner is deep in post-join operation, advancing past S.
        assert!(
            joiner.session.current_frame().as_i32() > serve_s.as_i32(),
            "joiner advanced its current frame past S={serve_s:?} (deep post-join), got {:?} \
             (confirmed {:?})",
            joiner.session.current_frame(),
            joiner.session.confirmed_frame(),
        );
        Ok(())
    }

    /// The fold-below-`S` VIOLATION TRIGGER is **structurally unreachable over a
    /// genuine wire** — the resolution of Session 58's "open question / roster
    /// tension." This test drives the *strongest natural attacker construction* (a
    /// genuine N≥5 double-failure relay aimed at a freshly-committed joiner) and
    /// demonstrates the obstruction end-to-end over the `FilterBusSocket` mesh.
    ///
    /// **Roles:** A=h0 coordinator, B=h1 relay-survivor, C=h2 joiner-slot,
    /// D=h3 dropped slot, E=h4 low-origin (the only peer to freeze D at the low
    /// receipt `g`; its gossip to A/B/C is cut so `g` lives ONLY on E).
    ///
    /// **The obstruction (verified-code-grounded; the body exercises every step):**
    /// 1. The serve gate is `confirmed_frame() >= S` AND
    ///    `npeer_owed_freeze_readjust_at_or_below(S) == false` (the serve poll
    ///    defers pre-capture / aborts post-capture otherwise). The owed-readjust
    ///    dry-run folds EVERY running non-reserved endpoint, so A will not serve
    ///    while ANY folded peer reports a slot freeze that re-sims at/below `S`.
    ///    Separately, for A's confirmed to even climb to `S` above D's freeze, D
    ///    must be **mesh-agreed disconnected** on A — `remote_slot_confirmed_bound`
    ///    excludes a slot (`None`) ONLY in the `(true, false, _)` arm: locally
    ///    disconnected AND no running remote reports it connected. A running remote
    ///    reporting D at a low receipt instead bounds A low → A stalls below `S`.
    /// 2. The low `g` is a FREEZE: its holder (E) reports D disconnected@`g`. If E is
    ///    in A's fold, A's sticky-min merge adopts `g` → A captures D@`g`, the joiner
    ///    inherits `g`, and there is no later lowering (the benign converged case, no
    ///    violation). So to carry D@f_carried > `g`, A must EXCLUDE E — and the only
    ///    exclusion that doesn't stall A's confirmed on E is to **prune** E.
    /// 3. Pruning E gossips `E@disconnected` to B (the relay). A **Disconnected**
    ///    protocol endpoint drops all `Input` (`message_allowed_in_current_state`
    ///    admits only `SyncRequest`), so once B folds `E@disconnected` it disconnects
    ///    its E endpoint (asserted here via B's `PeerDropped{E}`) and can **never**
    ///    adopt `g` from E — even after the E→B link is re-opened post-commit.
    /// 4. The relay MUST be a coordinator-directed survivor: only a survivor that
    ///    reactivates the joiner's slot has a live endpoint to deliver `Input` to the
    ///    joiner. A directed survivor is A-running, so if it held `g` A would have
    ///    learned it (step 2). The same-poll fold-order race is unwinnable because A
    ///    prunes E *pre-serve* (necessary to make D mesh-agreed), so B has long since
    ///    disconnected E before any post-commit delivery window.
    ///
    /// So the joiner commits carrying D frozen strictly below `S` (the S58
    /// foundational leg), the relay link is opened, B→joiner is a live gossip path
    /// (the joiner confirms past `S` via its live remotes incl. B) — and yet the
    /// joiner STAYS Running: no genuine-wire ordering delivers a sub-baseline freeze.
    /// The seam-injected `src/` unit guard
    /// `npeer_quad_committed_joiner_fold_below_snapshot_baseline_fails_closed_for_rejoin`
    /// proves the violation MECHANISM fires on those exact bytes; this test proves the
    /// bytes have no wire preimage, so that guard is the authoritative (and
    /// only-possible) coverage. See `progress/session-60-*` for the full argument.
    ///
    /// **Scope:** unreachability for the committed N-peer JOINER specifically (no
    /// history below `S` → fails closed). A *survivor* CAN see a below-its-history
    /// lowering — that is the double-failure relay, which the S55 floor-round
    /// CONVERGES (not fail-closes). The joiner is unique because its only inbound
    /// relays are coordinator-directed survivors the coordinator necessarily folds.
    #[test]
    fn npeer_hot_join_fold_below_s_trigger_unreachable_over_wire() -> Result<(), FortressError> {
        let mut mesh = build_mesh5()?;
        let [a_addr, b_addr, _c_addr, _d_addr, e_addr] = mesh.addrs;

        // Phase 1: warmup, all links open.
        for i in 0..6_u32 {
            mesh.poll_all(3);
            mesh.advance_all(i);
        }

        // Phase 2: make E the low-origin. Cut E's gossip to A, B, C so E's low
        // D-view never reaches them (`g` lives ONLY on E), then freeze D LOW on E.
        // `e_g_upper` upper-bounds E's freeze frame `g` (E froze D at its own
        // last_frame <= its current frame).
        mesh.blocked.block(e_addr, a_addr);
        mesh.blocked.block(e_addr, b_addr);
        mesh.blocked.block(e_addr, mesh.addrs[2]);
        mesh.e.remove_player(H_D).expect("E freezes D low");
        let e_g_upper = mesh.e.current_frame();
        let _ = mesh.e.events().count();

        // Phase 3: prune E on A — the ONLY way to keep A's confirmed advancing past
        // E's stale low D-view (a still-connected low-origin bounds A's confirmed
        // for D, so without the prune A stalls and can never serve at S). The prune
        // gossips `E@disconnected` to B (and C), which fold it and disconnect their
        // E endpoint (the cascade — captured via B's `PeerDropped{E}`); this is the
        // step that makes the relay B structurally unable to adopt `g`. With E
        // excluded mesh-wide, A/B/C's confirmed for D now climbs HIGH (well above
        // `g`) as D keeps delivering — building a wide, verifiable f_carried > g gap.
        mesh.a.remove_player(H_E).expect("A prunes E");
        let _ = mesh.a.events().count();
        let mut b_dropped_e = false;
        for i in 0..12_u32 {
            mesh.poll_all(3);
            mesh.advance_all(40 + i);
            for e in mesh.b.events() {
                if let FortressEvent::PeerDropped { handle, .. } = e {
                    if handle == H_E {
                        b_dropped_e = true;
                    }
                }
            }
        }
        assert!(
            b_dropped_e,
            "the cascade did not fire: B never dropped E, so this staging does not \
             exercise the structural obstruction (A's prune of the low-origin must \
             disconnect the relay's endpoint to it)"
        );

        // Phase 4: freeze D HIGH (f_carried) on A, B, C; drop D's session.
        // `a_confirmed_at_freeze` lower-bounds A's D-freeze (A freezes D at its
        // last_frame for D, which is >= its session-wide `confirmed_frame()`), and
        // `f_carried_upper` upper-bounds it. The assert pins `g < f_carried`: E's
        // low freeze IS strictly below the value the joiner will carry, so a relay
        // that COULD deliver `g` would genuinely trip `converged < baseline`.
        let a_confirmed_at_freeze = mesh.a.confirmed_frame();
        let f_carried_upper = mesh.a.current_frame();
        assert!(
            a_confirmed_at_freeze.as_i32() > e_g_upper.as_i32(),
            "the gradient is not established: A's D-freeze lower bound \
             {a_confirmed_at_freeze:?} must sit strictly above E's `g` upper bound \
             {e_g_upper:?}, else E's `g` is not a genuine lowering of the carried value \
             (the test would be vacuous)"
        );
        mesh.a.remove_player(H_D).expect("A freezes D high");
        mesh.b.remove_player(H_D).expect("B freezes D high");
        if let Some(c) = mesh.c.as_mut() {
            c.remove_player(H_D).expect("C freezes D high");
        }
        mesh.drop_d();
        let _ = mesh.a.events().count();
        let _ = mesh.b.events().count();

        // Phase 5: rejoin C. Drop C cleanly; coordinator A re-reserves the slot.
        drop_c_cleanly(&mut mesh);
        let mut joiner = Joiner::build(&mesh)?;
        assert_eq!(joiner.session.current_state(), SessionState::HotJoining);

        // Phase 6: drive the joiner to Running over the wire. A serves carrying
        // D@f_carried (it never learned `g`).
        let mut reached = false;
        for i in 0..1500_u32 {
            mesh.poll_all(1);
            joiner.poll();
            if i % 3 == 2 {
                mesh.advance_all(80 + (i % 8));
            }
            if joiner.session.current_state() == SessionState::Running {
                reached = true;
                break;
            }
        }
        assert!(
            reached,
            "joiner reached Running over the wire (state {:?})",
            joiner.session.current_state()
        );
        let serve_s = drive_first_advance(&mut joiner);
        assert!(
            serve_s.as_i32() > f_carried_upper.as_i32(),
            "snapshot baseline S={serve_s:?} must sit strictly above D's freeze \
             (upper bound {f_carried_upper:?}) — else 'D carried below S' is unproven"
        );

        // Phase 7: open E->B post-commit — the relay's chance to adopt `g` and relay
        // it to the joiner. B's E endpoint is already terminal (Phase 3 cascade), so
        // B drops E's Input and never adopts `g`.
        mesh.blocked.unblock(e_addr, b_addr);
        joiner.d_slot.clear();
        run_mesh_and_joiner(&mut mesh, &mut joiner, 120);

        // The joiner confirmed deep past S using its live remotes (A AND B): B IS a
        // live gossip path, so a lowering WOULD reach the joiner if B held one. It
        // does not.
        assert!(
            joiner.session.confirmed_frame().as_i32() > serve_s.as_i32(),
            "joiner must confirm past S={serve_s:?} via its live remotes (incl. the \
             relay B) — else 'B is a live path' is unproven (confirmed {:?})",
            joiner.session.confirmed_frame()
        );
        // The violation never fired over the wire: the joiner stays Running, carrying
        // D frozen at a single constant value (never lowered to `g`).
        assert_eq!(
            joiner.session.current_state(),
            SessionState::Running,
            "the fold-below-S trigger fired over the wire (joiner left Running) — the \
             structural obstruction was bypassed"
        );
        assert!(
            joiner.d_slot.len() >= 10,
            "joiner recorded too few post-relay D-slot statuses ({}) — vacuous staging",
            joiner.d_slot.len()
        );
        let frozen_values: std::collections::BTreeSet<u32> =
            joiner.d_slot.values().map(|&(v, _)| v).collect();
        assert_eq!(
            frozen_values.len(),
            1,
            "D's carried value must stay frozen (never lowered to g) across all {} \
             post-relay frames, got {frozen_values:?}",
            joiner.d_slot.len()
        );
        for (frame, (value, status)) in &joiner.d_slot {
            assert_eq!(
                *status,
                InputStatus::Disconnected,
                "joiner must keep D Disconnected at frame {frame} (value {value})"
            );
        }
        Ok(())
    }
}
