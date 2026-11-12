//! One captured input schedule replayed identically on every supported target.
//!
//! The checked-in `fixtures/portable-replay.bin` artifact is a real
//! [`Replay`] recorded from a two-peer P2P session with rollback corrections.
//! Native, browser `wasm32-unknown-unknown`, and Godot `wasm32-unknown-emscripten`
//! tests all load the exact same bytes, run the same integer-only application
//! model, and assert the same frame count, request count, and digests.
//!
//! The captured session is 4,096 confirmed frames (68.3 seconds at 60 Hz),
//! which covers a multi-minute input history rather than only a short prefix.
//! Playback proves that the ordered request stream, the recorded state
//! checksums, and the canonical serialized artifact bytes are identical on
//! every target.
//!
//! Replay playback runs with checksum validation enabled, so the platform also
//! recomputes every recorded application checksum. A platform-specific replay
//! divergence surfaces as a [`FortressEvent::ReplayDesync`] event and changes
//! the digest instead of silently passing.
//!
//! Regenerate the artifact with:
//!
//! ```text
//! cargo test -p fortress-replay-fixture --features capture --lib \
//!   regenerate_captured_replay_fixture -- --ignored --nocapture
//! ```

// Test-support crate: capture regeneration prints its evidence to the test log.
#![allow(clippy::disallowed_macros)]

use fortress_rollback::replay::Replay;
use fortress_rollback::sessions::replay_session::ReplaySession;
use fortress_rollback::{Config, FortressEvent, FortressRequest, InputStatus};
use serde::{Deserialize, Serialize};

/// Checked-in replay bytes captured from a real two-peer session.
pub const FIXTURE_BYTES: &[u8] = include_bytes!("../fixtures/portable-replay.bin");

/// Frames the fixture must play back.
pub const EXPECTED_FRAMES: u32 = 4_096;
/// Digest of the replayed request stream. Captured natively; browser and
/// Emscripten lanes assert the same value.
pub const EXPECTED_REQUEST_DIGEST: u128 = 0xb195_35aa_fcc4_16fe_df10_56f7_8eca_cf18;
/// Digest of the replayed state-checksum stream. Captured natively; browser and
/// Emscripten lanes assert the same value.
pub const EXPECTED_DIGEST: u128 = 0x56cc_01bd_3e28_a54e_0956_a2fc_e302_6d0b;

/// Fixed-width input used by the captured schedule.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureInput {
    /// Horizontal movement axis.
    pub axis: u32,
    /// Button bitmask.
    pub buttons: u32,
}

/// Integer-only application state. No float, wall-clock, or platform-dependent
/// operation participates in the checksum.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureState {
    /// Completed frames.
    pub frame: i32,
    /// Rolling mix of every confirmed input.
    pub mix: u64,
}

/// Session configuration for the replay fixture.
#[derive(Debug)]
pub struct FixtureConfig;

impl Config for FixtureConfig {
    type Input = FixtureInput;
    type State = FixtureState;
    type Address = u8;
}

/// Stable 128-bit FNV-1a digest used for both application checksums and the
/// end-to-end replay evidence.
///
/// A zero `hash` starts a new digest at the FNV offset basis; pass a previous
/// result to continue the same stream.
#[must_use]
pub fn digest_bytes(mut hash: u128, bytes: &[u8]) -> u128 {
    const FNV_OFFSET_128: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    const FNV_PRIME_128: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;
    if hash == 0 {
        hash = FNV_OFFSET_128;
    }
    for byte in bytes {
        hash ^= u128::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME_128);
    }
    hash
}

/// Checksum of the application state at a frame boundary.
#[must_use]
pub fn state_checksum(state: &FixtureState) -> u128 {
    let hash = digest_bytes(0, &state.frame.to_le_bytes());
    digest_bytes(hash, &state.mix.to_le_bytes())
}

/// Advances the application model by one frame.
pub fn apply_inputs(state: &mut FixtureState, inputs: &[(FixtureInput, InputStatus)]) {
    let mut mix = state.mix ^ (u64::from(state.frame.unsigned_abs()) << 1);
    for (index, (input, _status)) in inputs.iter().enumerate() {
        let lane = u64::try_from(index).unwrap_or(u64::MAX);
        let value = (u64::from(input.axis).rotate_left(3) ^ (u64::from(input.buttons) << 32))
            .wrapping_add(lane);
        mix = mix
            .rotate_left(7)
            .wrapping_add(value)
            .wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    state.mix = mix;
    state.frame = state.frame.saturating_add(1);
}

/// Deterministic input for one frame and player.
#[must_use]
pub fn input_for(frame: u32, player: u32) -> FixtureInput {
    let seed = frame
        .wrapping_mul(0x9e37_79b9)
        .wrapping_add(player.wrapping_mul(0x85eb_ca6b))
        ^ 0xc2b2_ae35;
    FixtureInput {
        axis: seed % 5,
        buttons: (seed >> 8) & 0x0f,
    }
}

/// Observable result of replaying the captured schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayEvidence {
    /// Frames played back.
    pub frames: u32,
    /// Requests processed (one checksum save plus one advance per frame).
    pub requests: u64,
    /// Replay desync events reported by checksum validation.
    pub desyncs: u32,
    /// Digest of the ordered request stream (kind, frame, inputs, status).
    pub request_digest: u128,
    /// Digest of each played-back frame's post-advance application checksum.
    pub digest: u128,
    /// Final application checksum.
    pub final_checksum: u128,
}

/// Replays the checked-in fixture.
///
/// # Errors
///
/// Returns a diagnostic when the checked-in bytes do not decode, the session
/// cannot be constructed, or a request is outside the replay contract.
pub fn replay_fixture() -> Result<ReplayEvidence, String> {
    replay_bytes(FIXTURE_BYTES)
}

/// Replays replay bytes with checksum validation.
///
/// # Errors
///
/// Returns a diagnostic when the bytes do not decode, the session cannot be
/// constructed, or a request is outside the replay contract.
pub fn replay_bytes(bytes: &[u8]) -> Result<ReplayEvidence, String> {
    let replay = Replay::<FixtureInput>::from_bytes(bytes)
        .map_err(|error| format!("fixture replay decode failed: {error}"))?;
    let resealed = replay
        .to_bytes()
        .map_err(|error| format!("fixture replay re-encode failed: {error}"))?;
    if resealed.as_slice() != bytes {
        return Err(
            "fixture artifact is not canonical: re-encoded bytes differ from the checked-in artifact"
                .to_owned(),
        );
    }
    replay_captured(replay)
}

/// Replays an already-decoded captured schedule.
///
/// # Errors
///
/// Returns a diagnostic when the session cannot be constructed or a request is
/// outside the replay contract.
pub fn replay_captured(replay: Replay<FixtureInput>) -> Result<ReplayEvidence, String> {
    let mut session = ReplaySession::<FixtureConfig>::new_with_validation(replay)
        .map_err(|error| format!("fixture replay session failed: {error}"))?;
    let mut state = FixtureState::default();
    let mut frames = 0u32;
    let mut requests = 0u64;
    let mut desyncs = 0u32;
    let mut request_digest = digest_bytes(0, b"fortress-portable-replay-requests");
    let mut digest = digest_bytes(0, b"fortress-portable-replay");

    while !session.is_complete() {
        let frame_requests = session
            .advance_frame()
            .map_err(|error| format!("fixture advance failed at frame {frames}: {error}"))?;
        requests = requests.saturating_add(u64::try_from(frame_requests.len()).unwrap_or(u64::MAX));
        for request in frame_requests {
            request_digest = digest_request(request_digest, &request);
            process_request(&mut state, request)?;
        }
        digest = digest_bytes(digest, &state_checksum(&state).to_le_bytes());
        frames = frames.saturating_add(1);
        for event in session.events() {
            if matches!(event, FortressEvent::ReplayDesync { .. }) {
                desyncs = desyncs.saturating_add(1);
            }
        }
    }
    // The final frame's pending checksum resolves only when events are drained
    // or another advance is attempted.
    for event in session.events() {
        if matches!(event, FortressEvent::ReplayDesync { .. }) {
            desyncs = desyncs.saturating_add(1);
        }
    }

    Ok(ReplayEvidence {
        frames,
        requests,
        desyncs,
        request_digest,
        digest,
        final_checksum: state_checksum(&state),
    })
}

/// Folds one request into the ordered request-stream digest.
fn digest_request(mut hash: u128, request: &FortressRequest<FixtureConfig>) -> u128 {
    match request {
        FortressRequest::SaveGameState { frame, .. } => {
            digest_bytes(digest_bytes(hash, &[1]), &frame.as_i32().to_le_bytes())
        },
        FortressRequest::LoadGameState { frame, .. } => {
            digest_bytes(digest_bytes(hash, &[2]), &frame.as_i32().to_le_bytes())
        },
        FortressRequest::AdvanceFrame { inputs } => {
            hash = digest_bytes(hash, &[3]);
            for (input, status) in inputs.iter() {
                let status = match status {
                    InputStatus::Confirmed => 0u8,
                    InputStatus::Predicted => 1,
                    InputStatus::Disconnected => 2,
                };
                hash = digest_bytes(hash, &input.axis.to_le_bytes());
                hash = digest_bytes(hash, &input.buttons.to_le_bytes());
                hash = digest_bytes(hash, &[status]);
            }
            hash
        },
    }
}

fn process_request(
    state: &mut FixtureState,
    request: FortressRequest<FixtureConfig>,
) -> Result<(), String> {
    match request {
        FortressRequest::SaveGameState { cell, frame } => {
            if !cell.save(frame, Some(*state), Some(state_checksum(state))) {
                return Err(format!(
                    "fixture failed to save state for frame {}",
                    frame.as_i32()
                ));
            }
        },
        FortressRequest::LoadGameState { cell, frame } => {
            let loaded = cell.load().ok_or_else(|| {
                format!(
                    "fixture replay requested a missing state for frame {}",
                    frame.as_i32()
                )
            })?;
            *state = loaded;
        },
        FortressRequest::AdvanceFrame { inputs } => apply_inputs(state, &inputs),
    }
    Ok(())
}

#[cfg(feature = "capture")]
mod capture {
    // Capture helpers are exercised by the ignored regeneration tests only.
    #![allow(dead_code)]

    use super::{
        input_for, state_checksum, FixtureConfig, FixtureInput, FixtureState, EXPECTED_FRAMES,
    };
    use fortress_rollback::replay::Replay;
    use fortress_rollback::{
        ClockFn, FortressRequest, Message, NonBlockingSocket, PlayerHandle, PlayerType,
        ProtocolConfig, SessionBuilder,
    };
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use web_time::{Duration, Instant};

    const FIRST_ADDRESS: u8 = 1;
    const SECOND_ADDRESS: u8 = 2;
    const MAX_QUEUED_MESSAGES: usize = 4_096;
    const STEP_MILLIS: u64 = 16;
    /// One frame beyond the expected playback count: the last driven frame is
    /// not yet confirmed when the capture drains.
    const CAPTURE_FRAMES: u32 = EXPECTED_FRAMES + 1;
    const CAPTURE_MAX_PREDICTION: usize = 8;
    const SYNC_POLL_LIMIT: usize = 10_000;
    const DRAIN_POLLS: usize = 2_000;

    struct MemorySocket {
        local: u8,
        peer: u8,
        inbox: Rc<RefCell<VecDeque<(u8, Message)>>>,
        peer_inbox: Rc<RefCell<VecDeque<(u8, Message)>>>,
    }

    impl NonBlockingSocket<u8> for MemorySocket {
        fn send_to(&mut self, message: &Message, address: &u8) {
            if *address != self.peer {
                return;
            }
            let Ok(mut inbox) = self.peer_inbox.try_borrow_mut() else {
                return;
            };
            if inbox.len() < MAX_QUEUED_MESSAGES {
                inbox.push_back((self.local, message.clone()));
            }
        }

        fn receive_all_messages(&mut self) -> Vec<(u8, Message)> {
            let Ok(mut inbox) = self.inbox.try_borrow_mut() else {
                return Vec::new();
            };
            inbox.drain(..).collect()
        }
    }

    fn socket_pair() -> (MemorySocket, MemorySocket) {
        let first_inbox = Rc::new(RefCell::new(VecDeque::new()));
        let second_inbox = Rc::new(RefCell::new(VecDeque::new()));
        (
            MemorySocket {
                local: FIRST_ADDRESS,
                peer: SECOND_ADDRESS,
                inbox: Rc::clone(&first_inbox),
                peer_inbox: Rc::clone(&second_inbox),
            },
            MemorySocket {
                local: SECOND_ADDRESS,
                peer: FIRST_ADDRESS,
                inbox: second_inbox,
                peer_inbox: first_inbox,
            },
        )
    }

    fn apply_requests(state: &mut FixtureState, requests: &[FortressRequest<FixtureConfig>]) {
        for request in requests {
            match request {
                FortressRequest::SaveGameState { cell, frame } => {
                    assert!(
                        cell.save(*frame, Some(*state), Some(state_checksum(state))),
                        "fixture capture save must accept frame {}",
                        frame.as_i32()
                    );
                },
                FortressRequest::LoadGameState { cell, .. } => {
                    *state = cell.load().expect("captured rollback state must exist");
                },
                FortressRequest::AdvanceFrame { inputs } => super::apply_inputs(state, inputs),
            }
        }
    }

    /// Records the checked-in fixture from a real two-peer session.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when session construction or capture fails.
    pub fn capture() -> Result<Replay<FixtureInput>, String> {
        let base = Instant::now();
        let offset_ms = Arc::new(AtomicU64::new(0));
        let clock_offset = Arc::clone(&offset_ms);
        let clock: ClockFn =
            Arc::new(move || base + Duration::from_millis(clock_offset.load(Ordering::Relaxed)));
        let tick = || {
            offset_ms.fetch_add(STEP_MILLIS, Ordering::Relaxed);
        };
        let (first_socket, second_socket) = socket_pair();
        let protocol_config = |seed| ProtocolConfig {
            protocol_rng_seed: Some(seed),
            clock: Some(Arc::clone(&clock)),
            ..ProtocolConfig::default()
        };

        let mut first = SessionBuilder::<FixtureConfig>::new()
            .with_protocol_config(protocol_config(0x00c0_ffee))
            .with_max_prediction_window(CAPTURE_MAX_PREDICTION)
            .with_recording(true)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .map_err(|error| format!("first fixture player: {error}"))?
            .add_player(PlayerType::Remote(SECOND_ADDRESS), PlayerHandle::new(1))
            .map_err(|error| format!("first fixture remote: {error}"))?
            .start_p2p_session(first_socket)
            .map_err(|error| format!("first fixture session: {error}"))?;
        let mut second = SessionBuilder::<FixtureConfig>::new()
            .with_protocol_config(protocol_config(0x00c0_ffef))
            .with_max_prediction_window(CAPTURE_MAX_PREDICTION)
            .with_recording(true)
            .add_player(PlayerType::Remote(FIRST_ADDRESS), PlayerHandle::new(0))
            .map_err(|error| format!("second fixture remote: {error}"))?
            .add_player(PlayerType::Local, PlayerHandle::new(1))
            .map_err(|error| format!("second fixture player: {error}"))?
            .start_p2p_session(second_socket)
            .map_err(|error| format!("second fixture session: {error}"))?;

        let mut state_first = FixtureState::default();
        let mut state_second = FixtureState::default();
        let mut synchronized = false;
        for _ in 0..SYNC_POLL_LIMIT {
            first.poll_remote_clients();
            second.poll_remote_clients();
            tick();
            if first.current_state() == fortress_rollback::SessionState::Running
                && second.current_state() == fortress_rollback::SessionState::Running
            {
                synchronized = true;
                break;
            }
        }
        if !synchronized {
            return Err("fixture capture peers failed to synchronize".to_owned());
        }
        first.events().for_each(drop);
        second.events().for_each(drop);

        for frame in 0..CAPTURE_FRAMES {
            first
                .add_local_input(PlayerHandle::new(0), input_for(frame, 0))
                .map_err(|error| format!("first fixture input at {frame}: {error}"))?;
            second
                .add_local_input(PlayerHandle::new(1), input_for(frame, 1))
                .map_err(|error| format!("second fixture input at {frame}: {error}"))?;
            apply_requests(
                &mut state_first,
                &first
                    .advance_frame()
                    .map_err(|error| format!("first fixture advance at {frame}: {error}"))?,
            );
            apply_requests(
                &mut state_second,
                &second
                    .advance_frame()
                    .map_err(|error| format!("second fixture advance at {frame}: {error}"))?,
            );
            // Every fifth frame the first peer speculates one frame ahead. The
            // second peer's real input then forces a rollback correction, so the
            // recorded confirmed history is corrected rather than speculative.
            if frame % 5 == 4 {
                first
                    .add_local_input(PlayerHandle::new(0), input_for(frame + 1, 0))
                    .map_err(|error| format!("first speculative input at {frame}: {error}"))?;
                apply_requests(
                    &mut state_first,
                    &first.advance_frame().map_err(|error| {
                        format!("first speculative advance at {frame}: {error}")
                    })?,
                );
            }
            for _ in 0..3 {
                first.poll_remote_clients();
                second.poll_remote_clients();
                tick();
            }
        }
        for _ in 0..DRAIN_POLLS {
            first.poll_remote_clients();
            second.poll_remote_clients();
            tick();
        }
        first.events().for_each(drop);
        second.events().for_each(drop);

        let mut first_replay = first
            .take_replay()
            .map_err(|error| format!("first fixture replay: {error}"))?;
        let mut second_replay = second
            .take_replay()
            .map_err(|error| format!("second fixture replay: {error}"))?;
        // The peer that speculated ahead can record one extra confirmed frame.
        // Keep the common confirmed prefix, which both peers must agree on.
        let common_frames = first_replay.frames.len().min(second_replay.frames.len());
        let frames_agree = first_replay.frames[..common_frames]
            == second_replay.frames[..common_frames]
            && first_replay.checksums[..common_frames] == second_replay.checksums[..common_frames];
        if !frames_agree {
            return Err(
                "fixture capture peers disagree on their common confirmed prefix".to_owned(),
            );
        }
        first_replay.frames.truncate(common_frames);
        first_replay.checksums.truncate(common_frames);
        first_replay.metadata.total_frames = common_frames;
        second_replay.frames.truncate(common_frames);
        second_replay.checksums.truncate(common_frames);
        second_replay.metadata.total_frames = common_frames;
        if first_replay != second_replay {
            return Err(
                "fixture capture peers recorded different confirmed histories after truncation"
                    .to_owned(),
            );
        }
        Ok(first_replay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_fixture_replays_with_expected_evidence() {
        let evidence = replay_fixture().expect("checked-in fixture replays");
        assert_eq!(evidence.frames, EXPECTED_FRAMES);
        assert_eq!(
            evidence.requests,
            u64::from(EXPECTED_FRAMES).saturating_mul(2),
            "validating playback emits one save and one advance per frame"
        );
        assert_eq!(evidence.desyncs, 0, "checked-in fixture must not desync");
        assert_eq!(evidence.request_digest, EXPECTED_REQUEST_DIGEST);
        assert_eq!(evidence.digest, EXPECTED_DIGEST);
    }

    #[test]
    fn checked_in_fixture_covers_extended_frame_history() {
        let replay =
            Replay::<FixtureInput>::from_bytes(FIXTURE_BYTES).expect("checked-in fixture decodes");
        let checksummed_frames = replay
            .checksums
            .iter()
            .filter(|checksum| checksum.is_some())
            .count();
        assert_eq!(
            checksummed_frames,
            usize::try_from(EXPECTED_FRAMES).unwrap_or(usize::MAX),
            "every replayed frame must carry a recorded checksum"
        );
    }

    #[test]
    fn mutated_recorded_checksum_is_detected() {
        let mut corrupted =
            Replay::<FixtureInput>::from_bytes(FIXTURE_BYTES).expect("checked-in fixture decodes");
        let recorded = corrupted
            .checksums
            .iter_mut()
            .find(|checksum| checksum.is_some())
            .expect("fixture records at least one checksum");
        *recorded = Some(recorded.unwrap_or(0) ^ 1);
        let evidence = replay_captured(corrupted).expect("corrupted fixture still replays");
        assert!(
            evidence.desyncs > 0,
            "checksum validation must detect a corrupted recorded checksum"
        );
    }
}

#[cfg(feature = "capture")]
#[cfg(test)]
mod capture_tests {
    use super::capture::capture;
    use super::{replay_captured, EXPECTED_DIGEST, EXPECTED_FRAMES, EXPECTED_REQUEST_DIGEST};
    use std::path::Path;

    #[test]
    #[ignore = "regenerates the checked-in portable replay fixture"]
    fn regenerate_captured_replay_fixture() {
        let replay = capture().expect("capture succeeds");
        replay.validate().expect("captured replay validates");
        let bytes = replay.to_bytes().expect("captured replay serializes");
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/portable-replay.bin");
        std::fs::write(&path, &bytes).expect("fixture writes");

        let evidence = replay_captured(replay).expect("captured replay plays back");
        assert_eq!(evidence.desyncs, 0, "captured replay must not desync");
        eprintln!(
            "frames={} expected={} requests={} request_digest={:#034x} expected_request_digest={:#034x} digest={:#034x} expected_digest={:#034x} final={:#034x} bytes={}",
            evidence.frames,
            EXPECTED_FRAMES,
            evidence.requests,
            evidence.request_digest,
            EXPECTED_REQUEST_DIGEST,
            evidence.digest,
            EXPECTED_DIGEST,
            evidence.final_checksum,
            bytes.len()
        );
    }

    #[test]
    #[ignore = "capture determinism check; requires --features capture"]
    fn capture_is_deterministic_and_plays_back() {
        let first = capture().expect("first capture succeeds");
        let second = capture().expect("second capture succeeds");
        assert_eq!(first, second, "capture must be fully deterministic");
        assert!(
            first.frames.len() >= usize::try_from(EXPECTED_FRAMES).unwrap_or(usize::MAX),
            "capture must record the full extended session"
        );
        assert!(
            first
                .checksums
                .iter()
                .filter(|checksum| checksum.is_some())
                .count()
                > 0,
            "capture must record checksums"
        );
        let evidence = replay_captured(first).expect("captured replay plays back");
        assert_eq!(evidence.desyncs, 0, "captured replay must not desync");
    }
}
