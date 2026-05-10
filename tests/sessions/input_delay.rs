//! Integration tests for runtime input delay adjustment on `P2PSession`.
//!
//! Covers:
//! - `P2PSession::set_input_delay` and `P2PSession::input_delay` round-trip.
//! - Validation of error paths: non-local handles, oversized delays.
//! - Per-player isolation: changing one local player's delay does not affect
//!   another's.
//! - Mid-session **increase** of input delay succeeds end-to-end: the queue
//!   back-fills, the network protocol replays the gap-fill frames to the
//!   remote peer, and both peers continue to advance without disconnects,
//!   desyncs, or sequence violations.
//! - Stress: a large mid-session delta (0 -> 8) followed by 12 more frames.
//! - Mid-session **decrease** of input delay returns
//!   `InputDelayDecreaseUnsupported`.
//! - Mid-session increase with multiple local players returns
//!   `InputDelayMidSessionMultiLocalUnsupported`.
//! - `Display` format for `FortressEvent::InputDelayRecommendation`.

// In tests: panic/unwrap/expect/etc. are appropriate.
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use crate::common::stubs::{GameStub, StubConfig, StubInput};
use crate::common::{
    create_channel_pair, create_unconnected_socket, poll_with_advance,
    synchronize_sessions_deterministic, SyncConfig, TestClock,
};
use fortress_rollback::{
    FortressError, FortressEvent, InvalidRequestKind, PlayerHandle, PlayerType, ProtocolConfig,
    SessionBuilder, SessionState,
};
use std::net::SocketAddr;

fn protocol_config(clock: &TestClock) -> ProtocolConfig {
    ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        ..ProtocolConfig::default()
    }
}

#[test]
fn p2p_set_input_delay_changes_delay() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket, _addr) = create_unconnected_socket(11000);
    let remote_addr: SocketAddr = ([127, 0, 0, 1], 11001).into();

    let mut sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .with_input_delay(0)?
        .start_p2p_session(socket)?;

    // Initial delay should match the builder configuration.
    assert_eq!(sess.input_delay(PlayerHandle::new(0))?, 0);

    sess.set_input_delay(PlayerHandle::new(0), 2)?;
    assert_eq!(sess.input_delay(PlayerHandle::new(0))?, 2);

    // Setting again to a different (larger) value also works.
    sess.set_input_delay(PlayerHandle::new(0), 5)?;
    assert_eq!(sess.input_delay(PlayerHandle::new(0))?, 5);

    Ok(())
}

#[test]
fn p2p_set_input_delay_rejects_remote_player() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket, _addr) = create_unconnected_socket(11010);
    let remote_addr: SocketAddr = ([127, 0, 0, 1], 11011).into();

    let mut sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    let err = sess
        .set_input_delay(PlayerHandle::new(1), 2)
        .expect_err("setting delay on remote player should fail");
    assert!(
        matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::NotLocalPlayer { .. }
            }
        ),
        "expected NotLocalPlayer error, got {err:?}"
    );

    // input_delay() should also reject remote handles.
    let err = sess
        .input_delay(PlayerHandle::new(1))
        .expect_err("reading delay on remote player should fail");
    assert!(
        matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::NotLocalPlayer { .. }
            }
        ),
        "expected NotLocalPlayer error, got {err:?}"
    );

    Ok(())
}

#[test]
fn p2p_set_input_delay_rejects_too_large() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket, _addr) = create_unconnected_socket(11020);
    let remote_addr: SocketAddr = ([127, 0, 0, 1], 11021).into();

    let mut sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    // Use a clearly oversized delay - the default queue length is 128, max delay 127.
    let err = sess
        .set_input_delay(PlayerHandle::new(0), 10_000)
        .expect_err("oversized delay should fail");
    assert!(
        matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::FrameDelayTooLarge { .. }
            }
        ),
        "expected FrameDelayTooLarge error, got {err:?}"
    );

    Ok(())
}

#[test]
fn p2p_input_delay_unaffected_by_other_player() -> Result<(), FortressError> {
    // Both players are local in a single session.
    let clock = TestClock::new();
    let (socket, _addr) = create_unconnected_socket(11030);

    let mut sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .with_input_delay(1)?
        .start_p2p_session(socket)?;

    assert_eq!(sess.input_delay(PlayerHandle::new(0))?, 1);
    assert_eq!(sess.input_delay(PlayerHandle::new(1))?, 1);

    sess.set_input_delay(PlayerHandle::new(0), 4)?;

    assert_eq!(
        sess.input_delay(PlayerHandle::new(0))?,
        4,
        "player 0 delay should reflect the new value"
    );
    assert_eq!(
        sess.input_delay(PlayerHandle::new(1))?,
        1,
        "player 1 delay should be unaffected"
    );

    Ok(())
}

/// Asserts that `events` contains no event variants that indicate a desync,
/// disconnect, or input-related failure. Useful for end-to-end tests that
/// drive a session through a particular code path and want to verify that
/// nothing went wrong.
#[track_caller]
fn assert_no_failure_events(events: &[FortressEvent<StubConfig>], context: &str) {
    for event in events {
        match event {
            FortressEvent::Disconnected { .. }
            | FortressEvent::DesyncDetected { .. }
            | FortressEvent::ReplayDesync { .. } => {
                panic!("[{context}] unexpected failure event: {event:?}");
            },
            _ => {},
        }
    }
}

/// Drives a synchronized session pair forward by `count` frames using the
/// supplied input generators. Polls between frames, processes requests, and
/// asserts that no failure events occur. Returns the events drained from each
/// session for further inspection.
#[allow(clippy::too_many_arguments)]
#[track_caller]
fn drive_frames<F1, F2>(
    sess1: &mut fortress_rollback::P2PSession<StubConfig>,
    sess2: &mut fortress_rollback::P2PSession<StubConfig>,
    stub1: &mut GameStub,
    stub2: &mut GameStub,
    clock: &TestClock,
    count: u32,
    mut input1: F1,
    mut input2: F2,
    context: &str,
) -> (
    Vec<FortressEvent<StubConfig>>,
    Vec<FortressEvent<StubConfig>>,
)
where
    F1: FnMut(u32) -> StubInput,
    F2: FnMut(u32) -> StubInput,
{
    let mut events1: Vec<FortressEvent<StubConfig>> = Vec::new();
    let mut events2: Vec<FortressEvent<StubConfig>> = Vec::new();

    for i in 0..count {
        poll_with_advance(sess1, sess2, clock, 3);

        sess1
            .add_local_input(PlayerHandle::new(0), input1(i))
            .expect("add_local_input on sess1");
        sess2
            .add_local_input(PlayerHandle::new(1), input2(i))
            .expect("add_local_input on sess2");

        let req1 = sess1.advance_frame().expect("advance_frame on sess1");
        let req2 = sess2.advance_frame().expect("advance_frame on sess2");
        stub1.handle_requests(req1);
        stub2.handle_requests(req2);

        events1.extend(sess1.events());
        events2.extend(sess2.events());
    }

    // Final poll + drain of any straggler events emitted on the wire.
    poll_with_advance(sess1, sess2, clock, 5);
    events1.extend(sess1.events());
    events2.extend(sess2.events());

    assert_no_failure_events(&events1, &format!("{context}/sess1"));
    assert_no_failure_events(&events2, &format!("{context}/sess2"));

    (events1, events2)
}

#[test]
fn p2p_set_input_delay_mid_session_increase_succeeds() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .with_input_delay(0)?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .with_input_delay(0)?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("Sessions should synchronize");
    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // Phase 1: drive 5 frames at delay = 0 with mutually consistent inputs.
    drive_frames(
        &mut sess1,
        &mut sess2,
        &mut stub1,
        &mut stub2,
        &clock,
        5,
        |i| StubInput { inp: i },
        |i| StubInput { inp: i },
        "phase1@delay=0",
    );

    let frame_before_change_1 = sess1.current_frame();
    let frame_before_change_2 = sess2.current_frame();

    // Mid-session delay increase from 0 -> 3. This bumps `last_added_frame`
    // forward by 3 inside sess1's input queue and must keep both peers
    // consistent (no desync, no sequence violation, no disconnect).
    sess1.set_input_delay(PlayerHandle::new(0), 3)?;
    assert_eq!(sess1.input_delay(PlayerHandle::new(0))?, 3);

    // Phase 2: drive 10 more frames after the change. Inputs continue to be
    // consistent so the local prediction matches the remote input -- if the
    // protocol replays the gap-fill correctly there is no rollback either.
    drive_frames(
        &mut sess1,
        &mut sess2,
        &mut stub1,
        &mut stub2,
        &clock,
        10,
        |i| StubInput { inp: i + 100 },
        |i| StubInput { inp: i + 100 },
        "phase2@delay=3",
    );

    // Both sessions remained Running with no disconnect/desync events.
    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);

    // Both peers advanced past the change by exactly 10 frames.
    assert_eq!(
        sess1.current_frame(),
        frame_before_change_1 + 10,
        "sess1 should have advanced 10 frames after the delay change"
    );
    assert_eq!(
        sess2.current_frame(),
        frame_before_change_2 + 10,
        "sess2 should have advanced 10 frames after the delay change"
    );

    // sess2 must have observed sess1's inputs (otherwise we'd have stalled).
    // confirmed_frame is the min of all peers' last_frame values; with delay=3
    // on sess1 and delay=0 on sess2, sess2 lags by 3 in its view of sess1.
    assert!(
        sess2.confirmed_frame().as_i32() >= frame_before_change_2.as_i32(),
        "sess2.confirmed_frame() should have progressed past the mid-session change \
         (got {:?}, change at {:?})",
        sess2.confirmed_frame(),
        frame_before_change_2
    );

    // Final delay is still 3.
    assert_eq!(sess1.input_delay(PlayerHandle::new(0))?, 3);

    Ok(())
}

/// Stress test: transition from delay=0 to a large delay (8) mid-session and
/// drive 12 more frames, verifying both peers stay consistent.
#[test]
fn p2p_set_input_delay_mid_session_increase_works_with_large_delta() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .with_input_delay(0)?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .with_input_delay(0)?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("Sessions should synchronize");

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // Phase 1: 4 frames at delay=0.
    drive_frames(
        &mut sess1,
        &mut sess2,
        &mut stub1,
        &mut stub2,
        &clock,
        4,
        |i| StubInput { inp: i },
        |i| StubInput { inp: i },
        "phase1@delay=0",
    );

    let frame_before_change_1 = sess1.current_frame();
    let frame_before_change_2 = sess2.current_frame();

    // Large delta: 0 -> 8.
    sess1.set_input_delay(PlayerHandle::new(0), 8)?;
    assert_eq!(sess1.input_delay(PlayerHandle::new(0))?, 8);

    // Phase 2: 12 more frames.
    drive_frames(
        &mut sess1,
        &mut sess2,
        &mut stub1,
        &mut stub2,
        &clock,
        12,
        |i| StubInput { inp: i + 200 },
        |i| StubInput { inp: i + 200 },
        "phase2@delay=8",
    );

    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);
    assert_eq!(sess1.current_frame(), frame_before_change_1 + 12);
    assert_eq!(sess2.current_frame(), frame_before_change_2 + 12);
    assert_eq!(sess1.input_delay(PlayerHandle::new(0))?, 8);

    Ok(())
}

#[test]
fn p2p_set_input_delay_mid_session_decrease_returns_error() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .with_input_delay(2)?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .with_input_delay(2)?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("Sessions should synchronize");

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // Advance a few frames so the input queue has data; this triggers the
    // mid-session check in InputQueue::set_frame_delay.
    for i in 0..3u32 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);

        sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess2.add_local_input(PlayerHandle::new(1), StubInput { inp: i })?;

        let req1 = sess1.advance_frame()?;
        let req2 = sess2.advance_frame()?;
        stub1.handle_requests(req1);
        stub2.handle_requests(req2);
    }

    // Attempt to decrease the delay. This must be rejected.
    let err = sess1
        .set_input_delay(PlayerHandle::new(0), 0)
        .expect_err("decreasing input delay mid-session should return an error");

    assert!(
        matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::InputDelayDecreaseUnsupported {
                    current: 2,
                    requested: 0,
                }
            }
        ),
        "expected InputDelayDecreaseUnsupported error, got {err:?}"
    );

    // Delay stays unchanged.
    assert_eq!(sess1.input_delay(PlayerHandle::new(0))?, 2);

    Ok(())
}

/// Multi-local mid-session input-delay increases must be rejected with
/// `InputDelayMidSessionMultiLocalUnsupported` so we never silently emit
/// inconsistent gap-fill bytes for the unchanged local players. We use a
/// 2-local-player session (no remote, so the session reaches `Running`
/// trivially) and exercise the mid-session branch by manually populating
/// the queue via `__internal` test hooks indirectly through the public
/// API: each `add_local_input` + `advance_frame` adds one input to every
/// queue.
#[test]
fn p2p_set_input_delay_mid_session_multi_local_returns_error() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, _a1, a2) = create_channel_pair();

    // Use the standard 2-peer setup but place 2 local players on sess1.
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_num_players(3)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(2))?
        .with_input_delay(0)?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_num_players(3)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(_a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(_a1), PlayerHandle::new(1))?
        .add_player(PlayerType::Local, PlayerHandle::new(2))?
        .with_input_delay(0)?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("Sessions should synchronize");

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // Drive a few frames so we're "mid-session".
    for i in 0..3u32 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);
        sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess1.add_local_input(PlayerHandle::new(1), StubInput { inp: i })?;
        sess2.add_local_input(PlayerHandle::new(2), StubInput { inp: i })?;
        let req1 = sess1.advance_frame()?;
        let req2 = sess2.advance_frame()?;
        stub1.handle_requests(req1);
        stub2.handle_requests(req2);
    }

    // Now attempt to increase delay for a single local player on sess1 while
    // there are still 2 local players. Must be rejected.
    let err = sess1
        .set_input_delay(PlayerHandle::new(0), 2)
        .expect_err("multi-local mid-session increase should be rejected");

    assert!(
        matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::InputDelayMidSessionMultiLocalUnsupported {
                    local_players: 2,
                }
            }
        ),
        "expected InputDelayMidSessionMultiLocalUnsupported, got {err:?}"
    );

    // Delay stays at 0 after the rejected attempt.
    assert_eq!(sess1.input_delay(PlayerHandle::new(0))?, 0);
    assert_eq!(sess1.input_delay(PlayerHandle::new(1))?, 0);

    Ok(())
}

#[test]
fn input_delay_recommendation_event_display() {
    let event: FortressEvent<StubConfig> = FortressEvent::InputDelayRecommendation {
        player_handle: PlayerHandle::new(2),
        current_delay: 1,
        suggested_delay: 4,
    };
    let formatted = event.to_string();
    assert!(
        formatted.contains("InputDelayRecommendation"),
        "Display should include the variant name: {formatted}"
    );
    assert!(
        formatted.contains("player=PlayerHandle(2)"),
        "Display should include the player handle: {formatted}"
    );
    assert!(
        formatted.contains("current=1"),
        "Display should include the current delay: {formatted}"
    );
    assert!(
        formatted.contains("suggested=4"),
        "Display should include the suggested delay: {formatted}"
    );
}
