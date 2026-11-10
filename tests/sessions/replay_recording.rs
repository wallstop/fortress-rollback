//! Replay recording fidelity across rollback corrections, and the checksum
//! evidence that desync detection consumes.
//!
//! A live session must publish and record its *confirmed* history, not the
//! speculative one. When a remote misprediction is repaired the affected frames
//! are re-simulated, so any checksum captured before the correction requests
//! execute is stale. Two user-visible failures follow from that:
//!
//! - A replay recorded by a correctly-played session reports
//!   [`FortressEvent::ReplayDesync`] on its own decoded playback.
//! - A stale local checksum sent to a peer triggers a false
//!   [`FortressEvent::DesyncDetected`] once the peer's canonical checksum arrives.

// In tests: panic/unwrap/expect/etc. are appropriate.
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use crate::common::stubs::{GameStub, StubConfig, StubInput};
use crate::common::{
    poll_with_advance, synchronize_sessions_deterministic, BlockedLinks, FilterSocket, SyncConfig,
    TestClock,
};
use fortress_rollback::sessions::replay_session::ReplaySession;
use fortress_rollback::{
    DesyncDetection, FortressError, FortressEvent, Frame, PlayerHandle, PlayerType, ProtocolConfig,
    SaveMode, SessionBuilder,
};

fn protocol_config(clock: &TestClock) -> ProtocolConfig {
    ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        ..ProtocolConfig::default()
    }
}

fn drive_lockstep(
    sess1: &mut fortress_rollback::P2PSession<StubConfig>,
    sess2: &mut fortress_rollback::P2PSession<StubConfig>,
    stub1: &mut GameStub,
    stub2: &mut GameStub,
    clock: &TestClock,
    from: u32,
    to: u32,
) -> Result<(), FortressError> {
    for i in from..to {
        sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: 7 * i + 1 })?;
        sess2.add_local_input(PlayerHandle::new(1), StubInput { inp: 13 * i + 2 })?;
        poll_with_advance(sess1, sess2, clock, 3);
        stub1.handle_requests(sess1.advance_frame()?);
        stub2.handle_requests(sess2.advance_frame()?);
        poll_with_advance(sess1, sess2, clock, 2);
    }
    Ok(())
}

/// Plays a recorded replay through a fresh session with checksum validation and
/// returns every frame that reported a replay desync.
fn replay_desync_frames(replay: fortress_rollback::replay::Replay<StubInput>) -> Vec<Frame> {
    let mut session = ReplaySession::<StubConfig>::new_with_validation(replay)
        .expect("recorded replay validates");
    let mut stub = GameStub::new();
    let mut desyncs = Vec::new();

    loop {
        let advanced = match session.advance_frame() {
            Ok(requests) => {
                stub.handle_requests(requests);
                true
            },
            Err(FortressError::InvalidFrameStructured { .. }) => false,
            Err(error) => panic!("unexpected replay error: {error:?}"),
        };
        for event in session.events() {
            if let FortressEvent::ReplayDesync { frame, .. } = event {
                desyncs.push(frame);
            }
        }
        if !advanced {
            break;
        }
    }
    desyncs.sort_unstable();
    desyncs
}

struct BurstRun {
    sess1: fortress_rollback::P2PSession<StubConfig>,
    events: Vec<FortressEvent<StubConfig>>,
}

/// Drives a two-peer session where peer 1's whole input backlog is withheld and
/// then restored at once. Peer 0 therefore predicts a run of peer 1 rows and
/// then repairs them all in a single `advance_frame`, so `confirmed_frame` jumps
/// across the entire wrong suffix while the correction is still only queued as
/// requests.
fn run_correction_burst(
    save_mode: SaveMode,
    recording: bool,
    desync: DesyncDetection,
) -> Result<BurstRun, FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2, links) = channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .with_input_delay(0)?
        .with_save_mode(save_mode)
        .with_desync_detection_mode(desync)
        .with_recording(recording)
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .with_input_delay(0)?
        .with_save_mode(save_mode)
        .with_desync_detection_mode(desync)
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("sessions synchronize");

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut events = Vec::new();

    // Agree on a confirmed prefix, then take peer 1's traffic away from the
    // recorder so peer 0 keeps running on predicted rows.
    drive_lockstep(&mut sess1, &mut sess2, &mut stub1, &mut stub2, &clock, 0, 5)?;
    links.block(a2, a1);

    for i in 5..12 {
        sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: 7 * i + 1 })?;
        sess2.add_local_input(PlayerHandle::new(1), StubInput { inp: 13 * i + 2 })?;
        stub1.handle_requests(sess1.advance_frame()?);
        stub2.handle_requests(sess2.advance_frame()?);
        events.extend(sess1.events());
        events.extend(sess2.events());
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);
    }

    // Restore the link: retransmissions deliver the withheld rows at once.
    links.unblock(a2, a1);
    poll_with_advance(&mut sess1, &mut sess2, &clock, 40);

    drive_lockstep(
        &mut sess1, &mut sess2, &mut stub1, &mut stub2, &clock, 12, 20,
    )?;
    poll_with_advance(&mut sess1, &mut sess2, &clock, 40);
    events.extend(sess1.events());
    events.extend(sess2.events());

    Ok(BurstRun { sess1, events })
}

#[test]
fn recorded_replay_of_corrected_session_plays_back_without_desync() -> Result<(), FortressError> {
    for save_mode in [SaveMode::EveryFrame, SaveMode::Sparse] {
        let mut run = run_correction_burst(save_mode, true, DesyncDetection::Off)?;
        let rollbacks = run.sess1.metrics().rollback_count;
        let replay = run.sess1.take_replay()?;
        replay.validate()?;
        let recorded_checksums = replay
            .checksums
            .iter()
            .filter(|checksum| checksum.is_some())
            .count();
        assert!(
            rollbacks > 0,
            "[{save_mode}] the witness must actually repair mispredictions"
        );
        assert!(
            recorded_checksums > 0,
            "[{save_mode}] the recording must contain checksums"
        );

        // Positive control: the playback harness must report a desync when a
        // recorded checksum is wrong, so the green result below is meaningful.
        let mut corrupted = replay.clone();
        let first = corrupted
            .checksums
            .iter_mut()
            .find(|checksum| checksum.is_some())
            .expect("a recorded checksum exists");
        *first = Some(first.unwrap_or(0) ^ 0xFFFF);
        assert!(
            !replay_desync_frames(corrupted).is_empty(),
            "[{save_mode}] the playback harness must detect a corrupted checksum"
        );

        assert_eq!(
            replay_desync_frames(replay),
            Vec::<Frame>::new(),
            "[{save_mode}] a replay of a correctly played-out session must not report ReplayDesync"
        );
    }
    Ok(())
}

#[test]
fn correction_burst_does_not_report_desync_detected() -> Result<(), FortressError> {
    let run = run_correction_burst(
        SaveMode::EveryFrame,
        false,
        DesyncDetection::On { interval: 4 },
    )?;
    assert!(
        run.sess1.metrics().rollback_count > 0,
        "the witness must actually repair mispredictions"
    );
    let reported: Vec<Frame> = run
        .events
        .iter()
        .filter_map(|event| match event {
            FortressEvent::DesyncDetected { frame, .. } => Some(*frame),
            _ => None,
        })
        .collect();
    assert_eq!(
        reported,
        Vec::<Frame>::new(),
        "repairing mispredictions must not be reported as a peer desync"
    );
    Ok(())
}

/// Builds a connected pair of directionally-blockable channel sockets.
fn channel_pair() -> (
    FilterSocket,
    FilterSocket,
    std::net::SocketAddr,
    std::net::SocketAddr,
    BlockedLinks,
) {
    let (s1, s2, a1, a2) = crate::common::create_channel_pair();
    let links = BlockedLinks::new();
    (
        FilterSocket::new(s1, links.clone()),
        FilterSocket::new(s2, links.clone()),
        a1,
        a2,
        links,
    )
}
