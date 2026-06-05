//! Hot-join snapshot serialization and capture/apply helpers.
//!
//! This module is the bridge between a host's saved `Config::State` and the
//! `StateSnapshot` wire message: it serializes/deserializes `Config::State`,
//! captures a snapshot from a host `SyncLayer`, and applies a received
//! snapshot on a joiner `SyncLayer`. Chunk 5's orchestration drives these
//! helpers; until then everything carries `#[allow(dead_code)]`.
//!
//! All items are `pub(crate)` and gated behind the `hot-join` feature.
//!
//! # Bounded deserialization (allocation only)
//!
//! A received `StateSnapshot::state_bytes` is peer-controlled. The wire-level
//! [`decode_message`](crate::network::codec::decode_message) already bounds the
//! `state_bytes` *length prefix* to the packet, so the `&[u8]` reaching
//! `deserialize_state` is finite. But the bytes themselves still describe a
//! user `Config::State` whose `Deserialize` impl may contain length-prefixed
//! containers; a corrupt snapshot could claim an enormous embedded length.
//! `deserialize_state` therefore decodes through `codec::decode_bounded`, which
//! caps every container claim at `MAX_BOUNDED_DECODE_LEN` so a hostile length
//! prefix cannot drive an oversized *allocation*.
//!
//! This bounds allocation only — it does **not** bound *recursion depth*.
//! bincode decodes a recursive type by recursing, and a pathologically
//! deeply-nested `Config::State` (e.g. `enum Tree { Leaf(u8), Node(Box<Tree>) }`
//! nested thousands deep) can be encoded in far fewer bytes than the
//! `MAX_BOUNDED_DECODE_LEN` cap yet still exhaust the
//! call stack while decoding — an uncatchable abort, not a recoverable `Err`.
//! Reaching that requires a peer feeding such bytes through a custom socket
//! large enough to carry them. Consistent with the project's "the user owns
//! `Config::State`'s shape and determinism" stance, **save-state types should be
//! non-recursive / shallow**; the bounded decoder cannot make a deeply recursive
//! state safe to decode. See `decode_bounded` in the
//! [`codec`](crate::network::codec) module for the analysis of bincode's
//! *allocation* bounds.

use crate::network::codec;
use crate::report_violation;
use crate::sync_layer::SyncLayer;
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::{
    Config, FortressError, FortressRequest, Frame, InvalidFrameReason, InvalidRequestKind,
};

#[cfg(feature = "hot-join")]
use crate::network::messages::StateSnapshot;

/// Serializes a host's `Config::State` into bincode bytes for a `StateSnapshot`.
///
/// Mirrors how the protocol `InputBytes` type maps serialization failures: the
/// codec encode is the should-never-happen path (a `Config::State` whose
/// `Serialize` impl fails), so a failure is reported via `report_violation!`
/// and surfaced as a structured serialization error rather than panicking.
///
/// # Errors
///
/// Returns [`FortressError::SerializationErrorStructured`] if the codec fails to
/// encode `state` (indicates a bug in the user's `Config::State` serialization).
// dead_code: consumed by chunk 5's host snapshot orchestration.
#[cfg(feature = "hot-join")]
#[allow(dead_code)]
pub(crate) fn serialize_state<T: Config>(state: &T::State) -> Result<Vec<u8>, FortressError> {
    codec::encode(state).map_err(|err| {
        report_violation!(
            ViolationSeverity::Critical,
            ViolationKind::InternalError,
            "Failed to serialize Config::State for hot-join snapshot: {}. This likely indicates a bug in your Config::State serialization.",
            err
        );
        FortressError::SerializationErrorStructured {
            kind: crate::SerializationErrorKind::Custom("hot-join state serialization failed"),
        }
    })
}

/// Deserializes a `Config::State` from peer-controlled snapshot bytes.
///
/// Decodes through `codec::decode_bounded` so a corrupt or malicious snapshot
/// cannot trigger an oversized *allocation*: every length-prefixed container the
/// state declares is bounded to `MAX_BOUNDED_DECODE_LEN` (64 MiB) before
/// allocating, and `bytes` longer than that cap is rejected outright. See
/// `codec::decode_bounded` for the full bincode allocation-bound analysis. The
/// incoming `bytes` slice is itself already bounded to the packet by chunk-2's
/// [`decode_message`](crate::network::codec::decode_message).
///
/// # Allocation is bounded; recursion depth is not
///
/// The cap bounds memory, not stack depth. A `Config::State` that is recursive
/// (e.g. a linked/tree type holding `Box<Self>`) can be encoded deeply nested in
/// far fewer bytes than the cap, and bincode decodes it by recursing — so such
/// input can still overflow the stack (an uncatchable abort, not an `Err`).
/// `Config::State` save-state types should therefore be non-recursive / shallow;
/// see the [module docs](self#bounded-deserialization-allocation-only).
///
/// # Errors
///
/// Returns [`FortressError::SerializationErrorStructured`] if `bytes` are
/// truncated, malformed, exceed the bounded-decode cap, or declare a container
/// length larger than the cap. Does not over-allocate on hostile input (a
/// hostile *length prefix* yields `Err`, never an OOM); a deeply recursive
/// `Config::State` is the documented exception — see above.
// dead_code: consumed by chunk 5's joiner snapshot orchestration.
#[cfg(feature = "hot-join")]
#[allow(dead_code)]
pub(crate) fn deserialize_state<T: Config>(bytes: &[u8]) -> Result<T::State, FortressError> {
    // alloc-bound: delegated to codec::decode_bounded, which caps every decoded
    // container at MAX_BOUNDED_DECODE_LEN (64 MiB) and pre-rejects an over-cap
    // input length. A malicious huge-length prefix yields Err here, never an OOM.
    codec::decode_bounded::<T::State>(bytes).map_err(|err| {
        // Not a should-never-happen path: peer bytes are untrusted, so a decode
        // failure is an ordinary rejected-input outcome, reported at Warning.
        report_violation!(
            ViolationSeverity::Warning,
            ViolationKind::NetworkProtocol,
            "Failed to deserialize hot-join snapshot state ({} byte(s)): {}",
            bytes.len(),
            err
        );
        FortressError::SerializationErrorStructured {
            kind: crate::SerializationErrorKind::Custom("hot-join state deserialization failed"),
        }
    })
}

/// Captures a `StateSnapshot` from a host's saved state at `frame`.
///
/// Reads the saved state and checksum at `frame` via
/// `SyncLayer::capture_snapshot_state`, serializes the state, and builds a
/// `StateSnapshot { frame, num_players, state_bytes, checksum }`.
///
/// Returns `Ok(None)` when there is no valid saved state at `frame` (the slot is
/// empty or has wrapped to a different frame), so the host can wait or skip
/// rather than treat a not-yet-available frame as an error.
///
/// # Errors
///
/// Returns [`FortressError::SerializationErrorStructured`] only if serializing
/// the (present) saved state fails — see `serialize_state`.
// dead_code: consumed by chunk 5's host snapshot orchestration.
#[cfg(feature = "hot-join")]
#[allow(dead_code)]
pub(crate) fn capture_snapshot<T: Config>(
    sync_layer: &SyncLayer<T>,
    frame: Frame,
    num_players: usize,
) -> Result<Option<StateSnapshot>, FortressError>
where
    // `capture_snapshot_state` clones the saved state out of its cell.
    T::State: Clone,
{
    let Some((state, checksum)) = sync_layer.capture_snapshot_state(frame) else {
        return Ok(None);
    };
    let state_bytes = serialize_state::<T>(&state)?;
    Ok(Some(StateSnapshot {
        frame,
        num_players,
        state_bytes,
        checksum,
    }))
}

/// Applies a received `StateSnapshot` to a fresh joiner [`SyncLayer`] and
/// returns the [`FortressRequest::LoadGameState`] the joiner must emit.
///
/// Steps, in order:
/// 1. Validate `snapshot.num_players == expected_num_players` (the wire input
///    width is fixed by the reserved-slot model, so a count mismatch means the
///    snapshot is for a different session shape and must be rejected).
/// 2. Validate `snapshot.frame` is non-negative.
/// 3. Deserialize the state (bounded; see `deserialize_state`).
/// 4. `SyncLayer::seek_to_frame` — resets every input queue and the frame
///    counters to the activation frame.
/// 5. `SyncLayer::inject_snapshot_state` — writes the saved-states cell at the
///    frame and sets `last_saved_frame`.
///
/// **Order matters:** seek THEN inject. `seek_to_frame` repositions the queues
/// and overwrites the frame counters wholesale (it deliberately leaves
/// `last_saved_frame` alone); the subsequent inject writes the cell *and* sets
/// `last_saved_frame` to the activation frame. Injecting first would have the
/// seek leave `last_saved_frame` untouched at its pre-seek value, but the cell
/// write would still stand — running them in the reverse order would set
/// `last_saved_frame` and then immediately have seek reset the surrounding
/// state, leaving the layer's saved-frame bookkeeping inconsistent with the
/// freshly-reset queues.
///
/// # Errors
///
/// - [`FortressError::InvalidRequestStructured`] with
///   [`InvalidRequestKind::PlayerCountMismatch`] if the player counts differ (in
///   either direction).
/// - [`FortressError::InvalidFrameStructured`] with
///   [`InvalidFrameReason::MustBeNonNegative`] if `snapshot.frame` is negative or
///   [`Frame::NULL`].
/// - [`FortressError::SerializationErrorStructured`] if the state fails to
///   deserialize (see `deserialize_state`).
/// - Any error `SyncLayer::seek_to_frame` can return.
/// - [`FortressError::InvalidFrameStructured`] with
///   [`InvalidFrameReason::MissingState`] if injecting the snapshot cell fails
///   (an internal indexing fault for an already-validated frame).
// dead_code: consumed by chunk 5's joiner snapshot orchestration.
#[cfg(feature = "hot-join")]
#[allow(dead_code)]
pub(crate) fn apply_snapshot<T: Config>(
    sync_layer: &mut SyncLayer<T>,
    snapshot: &StateSnapshot,
    expected_num_players: usize,
) -> Result<FortressRequest<T>, FortressError> {
    if snapshot.num_players != expected_num_players {
        return Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::PlayerCountMismatch {
                expected: expected_num_players,
                actual: snapshot.num_players,
            },
        });
    }

    if snapshot.frame.as_i32() < 0 {
        return Err(FortressError::InvalidFrameStructured {
            frame: snapshot.frame,
            reason: InvalidFrameReason::MustBeNonNegative,
        });
    }

    // Deserialize BEFORE mutating the layer so a corrupt snapshot leaves the
    // joiner untouched (no partial seek/inject on a rejected snapshot).
    let state = deserialize_state::<T>(&snapshot.state_bytes)?;

    // Order: seek (resets queues + frame counters, leaves last_saved_frame) THEN
    // inject (writes the cell + sets last_saved_frame).
    sync_layer.seek_to_frame(snapshot.frame)?;
    sync_layer
        .inject_snapshot_state(snapshot.frame, state, snapshot.checksum)
        .ok_or(FortressError::InvalidFrameStructured {
            frame: snapshot.frame,
            reason: InvalidFrameReason::MissingState,
        })
}

#[cfg(all(test, feature = "hot-join"))]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::telemetry::InvariantChecker;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    /// A non-trivial state containing a `Vec` so we exercise variable-width
    /// state and the bounded-decode path. Derives the bounds the `hot-join`
    /// `Config::State` requires (and, under `sync-send`, `Send + Sync`, which a
    /// `Vec`/`u32`-only struct satisfies automatically).
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct VecState {
        counter: u32,
        items: Vec<u32>,
        label: String,
    }

    impl VecState {
        fn sample() -> Self {
            Self {
                counter: 0xDEAD_BEEF,
                items: vec![1, 2, 3, 4, 5, 0xFFFF_FFFF],
                label: "hot-join".to_owned(),
            }
        }
    }

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
    struct TestInput {
        inp: u8,
    }

    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = VecState;
        type Address = SocketAddr;
    }

    /// Builds a `SyncLayer` and saves `state` (with `checksum`) at frame `f` by
    /// advancing to `f` then driving `save_current_state` + `cell.save`, mirror
    /// of the sync_layer tests' save pattern.
    fn layer_with_saved_state(
        num_players: usize,
        frame: Frame,
        state: VecState,
        checksum: Option<u128>,
    ) -> SyncLayer<TestConfig> {
        let mut layer = SyncLayer::<TestConfig>::new(num_players, 8);
        for _ in 0..frame.as_i32() {
            layer.advance_frame();
        }
        assert_eq!(layer.current_frame(), frame);
        let request = layer.save_current_state();
        match request {
            FortressRequest::SaveGameState { cell, frame: f } => {
                assert_eq!(f, frame);
                assert!(cell.save(frame, Some(state), checksum));
            },
            other => panic!("expected SaveGameState, got {other}"),
        }
        layer
    }

    #[test]
    fn serialize_then_deserialize_roundtrips() {
        let original = VecState::sample();
        let bytes = serialize_state::<TestConfig>(&original).unwrap();
        let decoded = deserialize_state::<TestConfig>(&bytes).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn deserialize_rejects_oversized_length_without_oom() {
        // Bincode (fixed-int) encodes `VecState` as: u32 counter (4B),
        // u64 Vec len + elements, u64 String len + bytes. We hand-craft bytes
        // that decode the counter fine, then claim a `Vec<u32>` length of
        // u64::MAX. The bounded decoder must reject this WITHOUT attempting a
        // ~16 EiB allocation or panicking.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u32.to_le_bytes()); // counter
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // items Vec len (absurd)
                                                          // (no element bytes follow; the cap must trip before reading any)

        let result = deserialize_state::<TestConfig>(&bytes);

        assert!(
            matches!(
                result,
                Err(FortressError::SerializationErrorStructured { .. })
            ),
            "huge embedded length must be rejected as a serialization error, got {result:?}"
        );
    }

    #[test]
    fn deserialize_rejects_oversized_string_length_without_oom() {
        // Same guard for the native byte-buffer decode path (String/Vec<u8>),
        // which bincode would otherwise allocate up-front as vec![0u8; len].
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u32.to_le_bytes()); // counter
        bytes.extend_from_slice(&0u64.to_le_bytes()); // items: empty Vec
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // String len (absurd)

        let result = deserialize_state::<TestConfig>(&bytes);

        assert!(
            matches!(
                result,
                Err(FortressError::SerializationErrorStructured { .. })
            ),
            "huge embedded String length must be rejected, got {result:?}"
        );
    }

    #[test]
    fn deserialize_rejects_truncated_bytes() {
        let original = VecState::sample();
        let bytes = serialize_state::<TestConfig>(&original).unwrap();
        // Drop the trailing half so the decode runs out of input mid-state.
        let truncated = &bytes[..bytes.len() / 2];

        let result = deserialize_state::<TestConfig>(truncated);

        assert!(
            matches!(
                result,
                Err(FortressError::SerializationErrorStructured { .. })
            ),
            "truncated bytes must be rejected, got {result:?}"
        );
    }

    #[test]
    fn capture_snapshot_reads_saved_state() {
        let frame = Frame::new(3);
        let state = VecState::sample();
        let checksum = Some(0x1234_5678_9ABC_DEF0_u128);
        let layer = layer_with_saved_state(2, frame, state.clone(), checksum);

        let snapshot = capture_snapshot(&layer, frame, 2)
            .unwrap()
            .expect("a saved state exists at the frame");

        assert_eq!(snapshot.frame, frame);
        assert_eq!(snapshot.num_players, 2);
        assert_eq!(snapshot.checksum, checksum);
        // state_bytes must decode back to the saved state.
        let decoded = deserialize_state::<TestConfig>(&snapshot.state_bytes).unwrap();
        assert_eq!(decoded, state);
    }

    #[test]
    fn capture_snapshot_absent_frame_returns_none() {
        // Frame 0 has no saved state (nothing was ever saved).
        let layer = SyncLayer::<TestConfig>::new(2, 8);

        let snapshot = capture_snapshot(&layer, Frame::new(0), 2).unwrap();

        assert!(snapshot.is_none());
    }

    /// Saves `state`/`checksum` at the layer's *current* frame, asserting the
    /// emitted `SaveGameState` request targets that frame. Unlike
    /// `layer_with_saved_state`, it operates in place so a test can save at
    /// several frames in sequence.
    fn save_at_current_frame(
        layer: &mut SyncLayer<TestConfig>,
        state: VecState,
        checksum: Option<u128>,
    ) {
        let current = layer.current_frame();
        match layer.save_current_state() {
            FortressRequest::SaveGameState { cell, frame } => {
                assert_eq!(frame, current);
                assert!(cell.save(frame, Some(state), checksum));
            },
            other => panic!("expected SaveGameState, got {other}"),
        }
    }

    #[test]
    fn capture_snapshot_returns_none_after_wraparound() {
        // The saved-states buffer holds `max_prediction + 1` cells, indexed by
        // `frame % buffer_len`. With max_prediction = 2 the buffer is 3 cells, so
        // frame 0 and frame 3 (= buffer_len) collide on slot 0. Saving frame 3
        // overwrites frame 0's slot, and the `cell.frame() == frame` guard in
        // `capture_snapshot_state` must then reject a request for the now-stale
        // frame 0 rather than returning frame 3's (wrong) state.
        let buffer_len = 3; // max_prediction (2) + 1
        let mut layer = SyncLayer::<TestConfig>::new(2, buffer_len - 1);

        // Save a distinct state at frame 0.
        let frame0_state = VecState {
            counter: 0,
            items: vec![0],
            label: "frame-0".to_owned(),
        };
        save_at_current_frame(&mut layer, frame0_state, Some(0));
        // Frame 0 is initially recoverable.
        assert!(capture_snapshot(&layer, Frame::new(0), 2)
            .unwrap()
            .is_some());

        // Advance and save through frame `buffer_len`, whose slot collides with
        // frame 0's and overwrites it.
        let wrap_frame = Frame::new(buffer_len as i32);
        let mut saved_wrap = false;
        for _ in 0..buffer_len {
            layer.advance_frame();
            let frame = layer.current_frame();
            save_at_current_frame(
                &mut layer,
                VecState {
                    counter: frame.as_i32() as u32,
                    items: vec![frame.as_i32() as u32],
                    label: "later".to_owned(),
                },
                Some(frame.as_i32() as u128),
            );
            saved_wrap |= frame == wrap_frame;
        }
        assert!(saved_wrap, "must have saved at the wraparound frame");
        // Sanity: the colliding slot now holds the wraparound frame's state.
        assert!(capture_snapshot(&layer, wrap_frame, 2).unwrap().is_some());

        // Frame 0's slot is stale, so the guard rejects it with Ok(None).
        let result = capture_snapshot(&layer, Frame::new(0), 2);
        assert!(matches!(result, Ok(None)), "got {result:?}");
    }

    #[test]
    fn apply_snapshot_seeks_injects_and_returns_load() {
        let frame = Frame::new(4);
        let state = VecState::sample();
        let checksum = Some(0xABCD_u128);

        // Host captures at frame F.
        let host = layer_with_saved_state(2, frame, state.clone(), checksum);
        let snapshot = capture_snapshot(&host, frame, 2).unwrap().unwrap();

        // Fresh joiner applies it.
        let mut joiner = SyncLayer::<TestConfig>::new(2, 8);
        let request = apply_snapshot(&mut joiner, &snapshot, 2).unwrap();

        match request {
            FortressRequest::LoadGameState { cell, frame: f } => {
                assert_eq!(f, frame);
                assert_eq!(cell.load(), Some(state.clone()));
                assert_eq!(cell.checksum(), checksum);
            },
            other => panic!("expected LoadGameState, got {other}"),
        }

        // Joiner is positioned at the activation frame.
        assert_eq!(joiner.current_frame(), frame);
        assert_eq!(joiner.last_saved_frame(), frame);
        // The seek + inject must leave the sync layer internally consistent.
        assert!(joiner.check_invariants().is_ok());

        // Re-capturing on the joiner now round-trips the same state (proves the
        // inject wrote the cell at the right slot and the seek/cell wiring
        // cohere).
        let recaptured = capture_snapshot(&joiner, frame, 2).unwrap().unwrap();
        let recaptured_state = deserialize_state::<TestConfig>(&recaptured.state_bytes).unwrap();
        assert_eq!(recaptured_state, state);
        assert_eq!(recaptured.checksum, checksum);
    }

    #[test]
    fn apply_snapshot_num_players_mismatch_errs() {
        let frame = Frame::new(2);
        let state = VecState::sample();
        let host = layer_with_saved_state(2, frame, state, None);
        // Snapshot says num_players=2.
        let snapshot = capture_snapshot(&host, frame, 2).unwrap().unwrap();

        let mut joiner = SyncLayer::<TestConfig>::new(3, 8);
        // Joiner expects 3.
        let result = apply_snapshot(&mut joiner, &snapshot, 3);

        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::PlayerCountMismatch {
                    expected: 3,
                    actual: 2,
                }
            })
        ));
        // No state mutation: joiner is untouched.
        assert_eq!(joiner.current_frame(), Frame::new(0));
        assert_eq!(joiner.last_saved_frame(), Frame::NULL);
    }

    #[test]
    fn apply_snapshot_negative_frame_errs() {
        let snapshot = StateSnapshot {
            frame: Frame::new(-5),
            num_players: 2,
            state_bytes: serialize_state::<TestConfig>(&VecState::sample()).unwrap(),
            checksum: None,
        };

        let mut joiner = SyncLayer::<TestConfig>::new(2, 8);
        let result = apply_snapshot(&mut joiner, &snapshot, 2);

        assert!(matches!(
            result,
            Err(FortressError::InvalidFrameStructured {
                frame,
                reason: InvalidFrameReason::MustBeNonNegative,
            }) if frame == Frame::new(-5)
        ));
        // No mutation.
        assert_eq!(joiner.current_frame(), Frame::new(0));
        assert_eq!(joiner.last_saved_frame(), Frame::NULL);
    }

    #[test]
    fn apply_snapshot_corrupt_state_bytes_leaves_joiner_unmutated() {
        // Valid `num_players` and `frame` so the snapshot passes both up-front
        // validations and reaches the deserialize step, but `state_bytes` cannot
        // decode to `VecState` (three bytes is far too short for even the u32
        // counter + u64 Vec len). This pins the "deserialize BEFORE mutate"
        // ordering: the decode must fail and the fresh joiner must be untouched.
        let snapshot = StateSnapshot {
            frame: Frame::new(4),
            num_players: 2,
            state_bytes: vec![0xFF, 0xFF, 0xFF],
            checksum: None,
        };

        let mut joiner = SyncLayer::<TestConfig>::new(2, 8);
        let result = apply_snapshot(&mut joiner, &snapshot, 2);

        // The corrupt bytes surface as a serialization/decode error. (`result` is
        // not formatted: `FortressRequest<TestConfig>` is not `Debug`.)
        assert!(
            matches!(
                result,
                Err(FortressError::SerializationErrorStructured { .. })
            ),
            "corrupt state_bytes must be a serialization error"
        );
        // Critically, the joiner is still at its fresh-layer values: `seek_to_frame`
        // would have set `current_frame` to the snapshot frame (4) had deserialize
        // run after the seek, so these assertions fail if the ordering regresses.
        assert_eq!(joiner.current_frame(), Frame::new(0));
        assert_eq!(joiner.last_saved_frame(), Frame::NULL);
    }

    #[test]
    fn end_to_end_vec_contents_survive_the_bridge() {
        // Full bridge: host state with Vec contents -> capture_snapshot ->
        // serialize bytes (snapshot.state_bytes) -> deserialize/apply_snapshot
        // on joiner -> joiner cell holds the SAME Vec contents.
        let frame = Frame::new(6);
        let state = VecState {
            counter: 7,
            items: vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
            label: "end-to-end".to_owned(),
        };
        let checksum = Some(0x0F0F_0F0F_u128);

        let host = layer_with_saved_state(2, frame, state.clone(), checksum);
        let snapshot = capture_snapshot(&host, frame, 2).unwrap().unwrap();

        // The wire `state_bytes` decode back to the same Vec independently.
        let on_wire = deserialize_state::<TestConfig>(&snapshot.state_bytes).unwrap();
        assert_eq!(on_wire.items, state.items);

        let mut joiner = SyncLayer::<TestConfig>::new(2, 8);
        let request = apply_snapshot(&mut joiner, &snapshot, 2).unwrap();

        let loaded = match request {
            FortressRequest::LoadGameState { cell, frame: f } => {
                assert_eq!(f, frame);
                cell.load().expect("injected cell holds the state")
            },
            other => panic!("expected LoadGameState, got {other}"),
        };
        assert_eq!(loaded, state);
        assert_eq!(loaded.items, state.items);
    }
}
