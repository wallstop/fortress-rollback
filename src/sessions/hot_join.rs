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
//! # Bounded deserialization (allocation and recursion depth)
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
//! The byte cap alone bounds *allocation* but not *recursion depth*: bincode
//! decodes a recursive type by recursing, and a pathologically deeply-nested
//! `Config::State` (e.g. `enum Tree { Leaf(u8), Node(Box<Tree>) }` nested
//! thousands deep) can be encoded in far fewer bytes than the
//! `MAX_BOUNDED_DECODE_LEN` cap yet still exhaust the call stack while decoding.
//! `codec::decode_bounded` therefore *also* bounds recursion depth: it decodes
//! through a depth-limited serde wrapper that **rejects** nesting beyond
//! `codec_depth::MAX_DECODE_DEPTH` with a recoverable `Err` instead of
//! overflowing the stack (B-codec). So a deeply-recursive snapshot blob from a
//! hostile peer is a clean decode error, not an uncatchable abort. (A
//! `Config::State` nested *within* that generous limit decodes normally; the
//! limit is far deeper than any realistic game state.) See `decode_bounded` /
//! `codec_depth` in the [`codec`](crate::network::codec) module for the full
//! allocation- and depth-bound analysis.

use crate::network::codec;
use crate::network::messages::{ConnectionStatus, Message, MessageBody, MessageHeader};
use crate::report_violation;
use crate::sync_layer::SyncLayer;
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::{
    Config, FortressError, FortressRequest, Frame, InvalidFrameReason, InvalidRequestKind,
    SerializationErrorKind,
};

#[cfg(feature = "hot-join")]
use crate::frame_info::PlayerInput;
#[cfg(feature = "hot-join")]
use crate::network::messages::StateSnapshot;
#[cfg(feature = "hot-join")]
use crate::{InputStatus, InputVec, PlayerHandle};

/// Default cap for a complete encoded hot-join `StateSnapshot` message.
///
/// The built-in UDP sockets use a 4 KiB receive buffer, so the default keeps
/// host snapshots within what those sockets can receive as one datagram. Users
/// with custom transports or larger socket buffers can raise this through
/// `SessionBuilder::with_hot_join_max_snapshot_wire_bytes`.
#[cfg(feature = "hot-join")]
pub(crate) const DEFAULT_HOT_JOIN_MAX_SNAPSHOT_WIRE_BYTES: usize = 4096;

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

#[cfg(feature = "hot-join")]
fn hot_join_serialization_error(message: &'static str) -> FortressError {
    FortressError::SerializationErrorStructured {
        kind: SerializationErrorKind::Custom(message),
    }
}

#[cfg(feature = "hot-join")]
fn serialized_state_len<T: Config>(state: &T::State) -> Result<usize, FortressError> {
    codec::encoded_len(state).map_err(|err| {
        report_violation!(
            ViolationSeverity::Critical,
            ViolationKind::InternalError,
            "Failed to measure Config::State for hot-join snapshot: {}. This likely indicates a bug in your Config::State serialization.",
            err
        );
        hot_join_serialization_error("hot-join state serialization failed")
    })
}

#[cfg(feature = "hot-join")]
fn snapshot_wire_len(
    frame: Frame,
    num_players: usize,
    checksum: Option<u128>,
    state_bytes_len: usize,
    bridge_inputs_len: usize,
    bridge_statuses: &[ConnectionStatus],
) -> Result<usize, FortressError> {
    // The skeleton carries the (small, num_players-bounded) per-slot statuses
    // verbatim so their exact encoded footprint is measured rather than
    // approximated; only the two byte blobs are added arithmetically below.
    let skeleton_snapshot = StateSnapshot {
        frame,
        num_players,
        state_bytes: Vec::new(),
        bridge_inputs: Vec::new(),
        // alloc-bound: at most the session's validated num_players entries
        // (the capture sites pass their own connection-status table).
        bridge_statuses: bridge_statuses.to_vec(),
        checksum,
    };
    let empty_message = Message {
        header: MessageHeader { magic: 0 },
        body: MessageBody::StateSnapshot(skeleton_snapshot),
    };

    let overhead = codec::encoded_len(&empty_message).map_err(|err| {
        report_violation!(
            ViolationSeverity::Critical,
            ViolationKind::InternalError,
            "Failed to measure hot-join StateSnapshot wire overhead: {}",
            err
        );
        hot_join_serialization_error("hot-join snapshot wire-size measurement failed")
    })?;

    overhead
        .checked_add(state_bytes_len)
        .and_then(|len| len.checked_add(bridge_inputs_len))
        .ok_or_else(|| {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "Hot-join snapshot wire length overflow: overhead {} + state {} + bridge {} bytes",
                overhead,
                state_bytes_len,
                bridge_inputs_len
            );
            hot_join_serialization_error("hot-join snapshot exceeds configured byte limit")
        })
}

#[cfg(feature = "hot-join")]
fn validate_snapshot_wire_size(
    frame: Frame,
    num_players: usize,
    checksum: Option<u128>,
    state_bytes_len: usize,
    bridge_inputs_len: usize,
    bridge_statuses: &[ConnectionStatus],
    max_snapshot_wire_bytes: usize,
) -> Result<(), FortressError> {
    if max_snapshot_wire_bytes == 0 {
        report_violation!(
            ViolationSeverity::Error,
            ViolationKind::Configuration,
            "Hot-join max snapshot wire bytes must be greater than zero"
        );
        return Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::ConfigValueOutOfRange {
                field: "hot_join_max_snapshot_wire_bytes",
                min: 1,
                max: u64::MAX,
                actual: 0,
            },
        });
    }

    let wire_len = snapshot_wire_len(
        frame,
        num_players,
        checksum,
        state_bytes_len,
        bridge_inputs_len,
        bridge_statuses,
    )?;
    if wire_len > max_snapshot_wire_bytes {
        report_violation!(
            ViolationSeverity::Error,
            ViolationKind::NetworkProtocol,
            "Hot-join snapshot at frame {} encodes to {} bytes (state {} bytes, bridge inputs {} bytes), exceeding configured max {}",
            frame,
            wire_len,
            state_bytes_len,
            bridge_inputs_len,
            max_snapshot_wire_bytes
        );
        return Err(hot_join_serialization_error(
            "hot-join snapshot exceeds configured byte limit",
        ));
    }

    Ok(())
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
/// # Allocation AND recursion depth are bounded
///
/// The byte cap bounds memory; `codec::decode_bounded` additionally bounds stack
/// depth. A `Config::State` that is recursive (e.g. a linked/tree type holding
/// `Box<Self>`) can be encoded deeply nested in far fewer bytes than the cap, and
/// bincode decodes it by recursing — so it decodes through a depth-limited serde
/// wrapper that **rejects** nesting beyond `codec_depth::MAX_DECODE_DEPTH` with a
/// recoverable `Err` rather than overflowing the stack. A state nested within
/// that (generous) limit decodes normally; see the
/// [module docs](self#bounded-deserialization-allocation-and-recursion-depth).
///
/// # Errors
///
/// Returns [`FortressError::SerializationErrorStructured`] if `bytes` are
/// truncated, malformed, exceed the bounded-decode cap, declare a container
/// length larger than the cap, or are nested past the recursion-depth limit.
/// Hostile input yields `Err`, never an OOM or a stack-overflow abort.
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
/// `StateSnapshot { frame, num_players, state_bytes, bridge_inputs: [], checksum }`
/// (the 2-peer serve shape: empty bridge inputs).
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
    capture_snapshot_with_max_wire_bytes(
        sync_layer,
        frame,
        num_players,
        DEFAULT_HOT_JOIN_MAX_SNAPSHOT_WIRE_BYTES,
    )
}

/// Captures a host snapshot after validating the complete encoded wire length.
///
/// This is the configurable form used by P2P hot-join orchestration. It measures
/// the serialized state before allocating `state_bytes` so an over-limit state is
/// rejected deterministically instead of being repeatedly encoded into packets
/// a built-in socket cannot receive.
#[cfg(feature = "hot-join")]
#[allow(dead_code)]
pub(crate) fn capture_snapshot_with_max_wire_bytes<T: Config>(
    sync_layer: &SyncLayer<T>,
    frame: Frame,
    num_players: usize,
    max_snapshot_wire_bytes: usize,
) -> Result<Option<StateSnapshot>, FortressError>
where
    // `capture_snapshot_state` clones the saved state out of its cell.
    T::State: Clone,
{
    let Some((state, checksum)) = sync_layer.capture_snapshot_state(frame) else {
        return Ok(None);
    };
    let state_len = serialized_state_len::<T>(&state)?;
    validate_snapshot_wire_size(
        frame,
        num_players,
        checksum,
        state_len,
        0,
        &[],
        max_snapshot_wire_bytes,
    )?;
    let state_bytes = serialize_state::<T>(&state)?;
    Ok(Some(StateSnapshot {
        frame,
        num_players,
        state_bytes,
        // 2-peer serve shape: snapshot.frame == F, no bridge and no per-slot
        // statuses. Emptiness (of both fields) is the wire discriminator the
        // joiner roles validate fail-closed.
        bridge_inputs: Vec::new(),
        bridge_statuses: Vec::new(),
        checksum,
    }))
}

/// Captures an **N-peer** serve snapshot at the snapshot frame `S`, embedding
/// the already-encoded bridge-input blob (the confirmed inputs at `S` for all
/// `num_players` slots — see [`encode_bridge_inputs`]) and the coordinator's
/// per-slot connection statuses at `S` (see
/// [`StateSnapshot::bridge_statuses`]). The complete wire length, *including*
/// the bridge inputs and statuses, is validated against
/// `max_snapshot_wire_bytes` before allocating `state_bytes`.
///
/// Producer-side fail-closed validation:
/// - `bridge_inputs` must be non-empty and `bridge_statuses` must hold
///   exactly `num_players` entries (the serve-shape discriminator — the
///   joiner must never invent either field);
/// - every disconnected status must satisfy `last_frame <= frame`: a slot
///   frozen ABOVE the snapshot frame has real ring history in
///   `(frame, last_frame]` that the snapshot + one-frame bridge cannot carry,
///   so the joiner could never reproduce it (an N>=4 corner; refusing the
///   capture defers the serve into its Phase-4 abort, and the retry's later
///   `S` clears the condition — honest, never wrong).
#[cfg(feature = "hot-join")]
pub(crate) fn capture_npeer_snapshot_with_max_wire_bytes<T: Config>(
    sync_layer: &SyncLayer<T>,
    frame: Frame,
    num_players: usize,
    max_snapshot_wire_bytes: usize,
    bridge_inputs: Vec<u8>,
    bridge_statuses: Vec<ConnectionStatus>,
) -> Result<Option<StateSnapshot>, FortressError>
where
    // `capture_snapshot_state` clones the saved state out of its cell.
    T::State: Clone,
{
    if bridge_inputs.is_empty() {
        // Fail closed at the producer: an N-peer snapshot without bridge
        // inputs would be indistinguishable from the 2-peer serve shape and
        // the joiner must never invent them. (A zero-width `Config::Input`
        // cannot reach this arm dishonestly: network sessions reject it at
        // endpoint construction — `SerializationErrorKind::InputSerializedSizeZero`
        // via `validate_default_input_wire_size` — so a non-zero player count
        // always encodes to a non-empty blob.)
        report_violation!(
            ViolationSeverity::Error,
            ViolationKind::InternalError,
            "N-peer hot-join snapshot at frame {} captured with empty bridge inputs",
            frame
        );
        return Err(hot_join_serialization_error(
            "hot-join N-peer snapshot requires bridge inputs",
        ));
    }
    if bridge_statuses.len() != num_players {
        report_violation!(
            ViolationSeverity::Error,
            ViolationKind::InternalError,
            "N-peer hot-join snapshot at frame {} captured with {} per-slot statuses for {} players",
            frame,
            bridge_statuses.len(),
            num_players
        );
        return Err(hot_join_serialization_error(
            "hot-join N-peer snapshot requires one connection status per slot",
        ));
    }
    if bridge_statuses
        .iter()
        .any(|status| status.disconnected && status.last_frame > frame)
    {
        report_violation!(
            ViolationSeverity::Warning,
            ViolationKind::NetworkProtocol,
            "Deferring the N-peer hot-join capture at frame {}: a slot is frozen above the snapshot frame ({:?}); the joiner cannot reproduce its post-snapshot ring history (the serve aborts/retries at a later S)",
            frame,
            bridge_statuses
        );
        return Err(hot_join_serialization_error(
            "hot-join N-peer snapshot cannot carry a slot frozen above the snapshot frame",
        ));
    }
    let Some((state, checksum)) = sync_layer.capture_snapshot_state(frame) else {
        return Ok(None);
    };
    let state_len = serialized_state_len::<T>(&state)?;
    validate_snapshot_wire_size(
        frame,
        num_players,
        checksum,
        state_len,
        bridge_inputs.len(),
        &bridge_statuses,
        max_snapshot_wire_bytes,
    )?;
    let state_bytes = serialize_state::<T>(&state)?;
    Ok(Some(StateSnapshot {
        frame,
        num_players,
        state_bytes,
        bridge_inputs,
        bridge_statuses,
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
/// **Order matters:** seek THEN inject. `seek_to_frame` is restricted to fresh
/// layers and repositions the queues plus frame counters while deliberately
/// leaving `last_saved_frame == Frame::NULL`. `inject_snapshot_state` then
/// requires `frame == current_frame`, writes the cell, and sets
/// `last_saved_frame` to the activation frame. Reversing the order is rejected
/// because it would otherwise advance the saved-frame watermark before the layer
/// is positioned at that frame.
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
///   [`InvalidFrameReason::Custom`] if the joiner's `last_saved_frame` is
///   already later than `snapshot.frame` (the helper is for fresh joiner layers;
///   applying an older snapshot to a running layer would violate sync-layer
///   frame ordering).
/// - [`FortressError::InvalidRequestStructured`] with
///   [`InvalidRequestKind::Custom`](crate::InvalidRequestKind::Custom) if the
///   joiner layer is otherwise non-fresh.
/// - Any error `SyncLayer::inject_snapshot_state` can return.
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

    // Order: seek (resets queues + frame counters, leaves last_saved_frame NULL)
    // THEN inject (writes the cell + sets last_saved_frame).
    sync_layer.seek_to_frame(snapshot.frame)?;
    sync_layer.inject_snapshot_state(snapshot.frame, state, snapshot.checksum)
}

/// Encodes the N-peer bridge inputs — the confirmed inputs at the snapshot
/// frame `S` for all slots, in handle order — into the
/// [`StateSnapshot::bridge_inputs`] wire blob: each `Config::Input` is
/// `codec`-encoded (fixed width by the network-input-width rule) and the
/// encodings are concatenated with no per-element framing.
///
/// # Errors
///
/// Returns [`FortressError::SerializationErrorStructured`] if any input fails
/// to encode (indicates a bug in the user's `Config::Input` serialization).
#[cfg(feature = "hot-join")]
pub(crate) fn encode_bridge_inputs<T: Config>(
    inputs: &[PlayerInput<T::Input>],
) -> Result<Vec<u8>, FortressError> {
    // alloc-bound: grown by `encode_append`'s fallible writer to exactly
    // `inputs.len()` (== the session's validated num_players) × the fixed
    // `Config::Input` encoded width.
    let mut bytes = Vec::new();
    for player_input in inputs {
        codec::encode_append(&player_input.input, &mut bytes).map_err(|err| {
            report_violation!(
                ViolationSeverity::Critical,
                ViolationKind::InternalError,
                "Failed to serialize a hot-join bridge input: {}. This likely indicates a bug in your Config::Input serialization.",
                err
            );
            hot_join_serialization_error("hot-join bridge input serialization failed")
        })?;
    }
    Ok(bytes)
}

/// Decodes a received [`StateSnapshot::bridge_inputs`] blob into exactly
/// `num_players` inputs, in handle order, requiring the blob to be consumed
/// completely (no trailing bytes). An empty blob with `num_players > 0` is
/// rejected — the N-peer joiner must never invent bridge inputs.
///
/// # Errors
///
/// Returns [`FortressError::SerializationErrorStructured`] if the blob is
/// truncated, malformed, has trailing bytes, or any element fails the bounded
/// decode (`codec::decode_bounded_with_consumed` caps every container claim,
/// so a hostile length prefix inside a non-conforming variable-width
/// `Config::Input` yields `Err`, never an oversized allocation).
#[cfg(feature = "hot-join")]
pub(crate) fn decode_bridge_inputs<T: Config>(
    bytes: &[u8],
    num_players: usize,
) -> Result<Vec<T::Input>, FortressError> {
    let mut inputs = Vec::new();
    // alloc-bound: `num_players` is the receiver's own session-validated
    // player count (the caller rejects a snapshot whose `num_players`
    // mismatches before decoding the blob), so this reserves at most
    // num_players entries of the fixed-width `Config::Input`.
    inputs.try_reserve_exact(num_players).map_err(|_err| {
        report_violation!(
            ViolationSeverity::Error,
            ViolationKind::InternalError,
            "Failed to reserve {} hot-join bridge inputs",
            num_players
        );
        hot_join_serialization_error("hot-join bridge input allocation failed")
    })?;
    let mut cursor = 0_usize;
    for _ in 0..num_players {
        let remaining = bytes
            .get(cursor..)
            .ok_or_else(|| hot_join_serialization_error("hot-join bridge inputs truncated"))?;
        let (input, consumed) = codec::decode_bounded_with_consumed::<T::Input>(remaining)
            .map_err(|err| {
                // Peer bytes are untrusted: a decode failure is an ordinary
                // rejected-input outcome, reported at Warning.
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Failed to deserialize a hot-join bridge input ({} byte(s) remaining): {}",
                    remaining.len(),
                    err
                );
                hot_join_serialization_error("hot-join bridge input deserialization failed")
            })?;
        cursor = cursor
            .checked_add(consumed)
            .ok_or_else(|| hot_join_serialization_error("hot-join bridge input cursor overflow"))?;
        inputs.push(input);
    }
    if cursor != bytes.len() {
        report_violation!(
            ViolationSeverity::Warning,
            ViolationKind::NetworkProtocol,
            "Hot-join bridge inputs have {} trailing byte(s) after {} decoded input(s)",
            bytes.len().saturating_sub(cursor),
            num_players
        );
        return Err(hot_join_serialization_error(
            "hot-join bridge inputs have trailing bytes",
        ));
    }
    Ok(inputs)
}

/// Applies a received **N-peer** `StateSnapshot` (snapshot frame `S`, carried
/// bridge inputs and per-slot connection statuses for all slots at `S`) to a
/// fresh joiner [`SyncLayer`] and simulates the one bridge frame, returning
/// the two requests the joiner must emit **in order**: `LoadGameState(S)`
/// then `AdvanceFrame` with the carried inputs (deterministically reproducing
/// every peer's state at the activation frame `F = S + 1`).
///
/// Steps, in order:
/// 1. Validate `bridge_inputs.len() == expected_num_players`, and the same
///    for `snapshot.bridge_statuses` (fail-closed: a snapshot without exactly
///    one carried input + status per slot must never be bridged — the joiner
///    cannot invent the missing values), plus the per-status frozen bound
///    (`disconnected ⇒ last_frame <= S`; a slot frozen above `S` has ring
///    history the bridge cannot carry).
/// 2. Delegate to [`apply_snapshot`] for the validated seek + inject core
///    (num_players/frame validation, bounded deserialize **before** any
///    mutation, `seek_to_frame(S)`, `inject_snapshot_state`). The reuse
///    boundary is deliberate: the 2-peer apply positions the layer at the
///    load frame and stops (the joiner is real from `F == snapshot.frame`);
///    the N-peer path must continue below — the design's "the 2-peer apply
///    must not be reused verbatim" point.
/// 3. Per slot, replicate exactly what every survivor's layer holds at `S`
///    (S34 fix round 1 — CRITICAL-2 + MAJOR-1):
///    - a slot carried `{disconnected, last_frame < S}` was served through
///      `synchronized_inputs`' frozen branch on every peer: do NOT seed its
///      ring (no peer has a real frame-`S` entry); instead freeze the queue
///      at the carried value (the agreed frozen input) so frames `>= F`
///      serve `(carried value, Disconnected)` exactly like every survivor;
///    - a slot carried `{disconnected, last_frame == S}` froze exactly AT
///      `S`: its real frame-`S` input exists in every peer's ring, so seed
///      the ring at `S` **then** freeze at the carried value;
///    - a live slot (and the joining slot itself, which goes live at `F`)
///      is seeded at `S` (the seek positioned each queue to accept `S`
///      next, and the add also seeds `last_confirmed_input`), so its first
///      **real** input lands at exactly `F`.
/// 4. Advance the layer's frame counter to `F` and build the bridge
///    `AdvanceFrame`, deriving each slot's [`InputStatus`] from the SAME
///    predicate `synchronized_inputs` evaluates at `S` on every survivor:
///    `disconnected && last_frame < S` ⇒ [`InputStatus::Disconnected`], else
///    [`InputStatus::Confirmed`]. In particular the `f0 == S` boundary (the
///    idle-lobby drop) presents the slot's real frame-`S` input `Confirmed`
///    mesh-wide; hardcoding the joining slot `Disconnected` desyncs any game
///    whose state folds the disconnected bit.
/// 5. Arm the joiner's own [`SyncLayer::set_reactivation_floor`] when the
///    own slot was carried `{disconnected, f0 < S}` (S34 fix round 2), so a
///    later re-simulation of the bridge frame `S` — possible for a
///    `SaveMode::Sparse` joiner whose first rollback reloads
///    `last_saved = S` — presents the own slot exactly as the original
///    bridge did (`(carried value, Disconnected)`), even though the session
///    stamps the own slot `{connected, S}` at apply. The `f0 == S` boundary
///    arms nothing: its floor window `(f0, F)` is empty and the seeded ring
///    entry already replays `Confirmed`, matching the original bridge.
///
/// # Errors
///
/// Everything [`apply_snapshot`] can return, plus
/// [`InvalidRequestKind::PlayerCountMismatch`] when the carried input or
/// status count differs from `expected_num_players`, and
/// [`FortressError::SerializationErrorStructured`] when a carried status is
/// frozen above the snapshot frame. Validation precedes every mutation: on a
/// validation `Err` the joiner layer is untouched. (The post-validation
/// freeze loop's [`SyncLayer::refreeze_player_with_value`] is infallible for
/// the already-validated handles; its error arm is defensively surfaced but
/// unreachable in correct code — the same holds for the post-bridge
/// [`SyncLayer::set_reactivation_floor`] arm.)
#[cfg(feature = "hot-join")]
pub(crate) fn apply_npeer_snapshot<T: Config>(
    sync_layer: &mut SyncLayer<T>,
    snapshot: &StateSnapshot,
    bridge_inputs: &[T::Input],
    expected_num_players: usize,
    local_handle: PlayerHandle,
) -> Result<(FortressRequest<T>, FortressRequest<T>), FortressError> {
    if bridge_inputs.len() != expected_num_players {
        return Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::PlayerCountMismatch {
                expected: expected_num_players,
                actual: bridge_inputs.len(),
            },
        });
    }
    if snapshot.bridge_statuses.len() != expected_num_players {
        return Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::PlayerCountMismatch {
                expected: expected_num_players,
                actual: snapshot.bridge_statuses.len(),
            },
        });
    }
    if snapshot
        .bridge_statuses
        .iter()
        .any(|status| status.disconnected && status.last_frame > snapshot.frame)
    {
        return Err(hot_join_serialization_error(
            "hot-join N-peer snapshot carries a slot frozen above the snapshot frame",
        ));
    }

    let load = apply_snapshot(sync_layer, snapshot, expected_num_players)?;

    let mut inputs = InputVec::<T::Input>::new();
    for (idx, (input, status)) in bridge_inputs
        .iter()
        .zip(snapshot.bridge_statuses.iter())
        .enumerate()
    {
        let handle = PlayerHandle::new(idx);
        // The survivors' frame-S presentation predicate (see step 4 above).
        let disconnected_below_s = status.disconnected && status.last_frame < snapshot.frame;
        if handle == local_handle || !disconnected_below_s {
            // Seed the ring at S: a live slot (or one frozen exactly AT S)
            // has its real frame-S input in every peer's ring — and the
            // JOINING slot itself must always be seeded regardless of its
            // carried freeze frame, because it goes live at F and the seed is
            // what positions its queue (and `last_confirmed_input`) so the
            // joiner's first real local input lands at exactly F.
            sync_layer.add_remote_input(handle, PlayerInput::new(snapshot.frame, *input));
        }
        if status.disconnected && handle != local_handle {
            // A (second) dead slot stays dead on the joiner: freeze its
            // queue at the carried value so frames >= F serve
            // (carried value, Disconnected) exactly like every survivor's
            // frozen branch. Infallible for the validated handle range; the
            // defensive arm surfaces (never silently swallows) the
            // impossible failure.
            sync_layer.refreeze_player_with_value(handle, Some(*input))?;
        }
        let input_status = if disconnected_below_s {
            InputStatus::Disconnected
        } else {
            InputStatus::Confirmed
        };
        inputs.push((*input, input_status));
    }
    sync_layer.advance_frame();

    // S34 fix round 2 (MAJOR-A): arm the joiner's own pre-activation serving
    // floor, making the joiner uniform with every survivor/coordinator reopen
    // site (which all call `SyncLayer::set_reactivation_floor`). The carried
    // statuses govern only the FIRST bridge presentation above; a later
    // re-simulation of the bridge frame `S` (e.g. a `SaveMode::Sparse`
    // rollback to `last_saved = S`) goes through `synchronized_inputs` with
    // the session-STAMPED table, where the own slot is `{connected, S}` —
    // without the floor the replay would present the own slot's carried
    // frozen input `Confirmed` where the original bridge (and every
    // survivor's history of `S`) presented it `Disconnected`. The floor
    // serves frames in `(f0, F)` as `(carried value, Disconnected)` — the
    // queue's `last_confirmed_input` is the just-seeded carried value — and
    // fails closed at `<= f0` exactly like the survivor sites. Armed only
    // for a carried `f0 < S`: at the `f0 == S` boundary the window
    // `(f0, F)` is empty and a replay of `S` must serve the seeded ring
    // entry `Confirmed` (arming would fail it closed).
    if let Some(own) = snapshot.bridge_statuses.get(local_handle.as_usize()) {
        if own.disconnected && own.last_frame < snapshot.frame {
            // Infallible for the validated handle range (the carried status
            // exists at the handle's index, and the status count was
            // validated against `expected_num_players` above); the defensive
            // arm surfaces (never silently swallows) the impossible failure.
            let activation_frame = sync_layer.current_frame();
            sync_layer.set_reactivation_floor(local_handle, activation_frame, own.last_frame)?;
        }
    }

    Ok((load, FortressRequest::AdvanceFrame { inputs }))
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
    fn capture_snapshot_rejects_oversized_wire_message_before_allocating_state_bytes() {
        let frame = Frame::new(3);
        let state = VecState {
            counter: 7,
            items: vec![42; 256],
            label: "too-large-for-test-cap".to_owned(),
        };
        let layer = layer_with_saved_state(2, frame, state, Some(0x1234));

        let result = capture_snapshot_with_max_wire_bytes(&layer, frame, 2, 64);

        assert!(
            matches!(
                result,
                Err(FortressError::SerializationErrorStructured {
                    kind: SerializationErrorKind::Custom(
                        "hot-join snapshot exceeds configured byte limit"
                    ),
                })
            ),
            "oversized snapshots must be rejected with a structured serialization error"
        );
    }

    #[test]
    fn capture_snapshot_with_zero_wire_limit_is_rejected() {
        let frame = Frame::new(1);
        let layer = layer_with_saved_state(2, frame, VecState::sample(), None);

        let result = capture_snapshot_with_max_wire_bytes(&layer, frame, 2, 0);

        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "hot_join_max_snapshot_wire_bytes",
                    min: 1,
                    actual: 0,
                    ..
                }
            })
        ));
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
            bridge_inputs: Vec::new(),
            bridge_statuses: Vec::new(),
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
            bridge_inputs: Vec::new(),
            bridge_statuses: Vec::new(),
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
    fn apply_snapshot_older_than_joiner_saved_frame_leaves_joiner_unmutated() {
        let snapshot_frame = Frame::new(3);
        let host = layer_with_saved_state(2, snapshot_frame, VecState::sample(), Some(0xAA));
        let snapshot = capture_snapshot(&host, snapshot_frame, 2)
            .unwrap()
            .expect("host saved the snapshot frame");

        let mut joiner = layer_with_saved_state(2, Frame::new(5), VecState::sample(), Some(0xBB));
        let current_before = joiner.current_frame();
        let saved_before = joiner.last_saved_frame();
        let confirmed_before = joiner.last_confirmed_frame();

        let result = apply_snapshot(&mut joiner, &snapshot, 2);

        match result {
            Err(FortressError::InvalidFrameStructured { frame, reason }) => {
                assert_eq!(frame, snapshot_frame);
                assert!(matches!(
                    reason,
                    InvalidFrameReason::Custom("seek target is older than last_saved_frame")
                ));
            },
            _ => panic!("Expected InvalidFrameStructured error"),
        }
        assert_eq!(joiner.current_frame(), current_before);
        assert_eq!(joiner.last_saved_frame(), saved_before);
        assert_eq!(joiner.last_confirmed_frame(), confirmed_before);
        assert!(joiner.check_invariants().is_ok());
    }

    #[test]
    fn apply_snapshot_on_non_fresh_same_frame_leaves_joiner_unmutated() {
        let snapshot_frame = Frame::new(5);
        let host = layer_with_saved_state(2, snapshot_frame, VecState::sample(), Some(0xAA));
        let snapshot = capture_snapshot(&host, snapshot_frame, 2)
            .unwrap()
            .expect("host saved the snapshot frame");

        let mut joiner = layer_with_saved_state(2, snapshot_frame, VecState::sample(), Some(0xBB));
        let current_before = joiner.current_frame();
        let saved_before = joiner.last_saved_frame();
        let confirmed_before = joiner.last_confirmed_frame();

        let result = apply_snapshot(&mut joiner, &snapshot, 2);

        match result {
            Err(FortressError::InvalidRequestStructured { kind }) => {
                assert!(matches!(
                    kind,
                    InvalidRequestKind::Custom("seek_to_frame requires a fresh SyncLayer")
                ));
            },
            _ => panic!("Expected InvalidRequestStructured error"),
        }
        assert_eq!(joiner.current_frame(), current_before);
        assert_eq!(joiner.last_saved_frame(), saved_before);
        assert_eq!(joiner.last_confirmed_frame(), confirmed_before);
        assert!(joiner.check_invariants().is_ok());
    }

    fn sample_bridge_wire(values: &[u8]) -> Vec<u8> {
        let inputs: Vec<PlayerInput<TestInput>> = values
            .iter()
            .map(|&inp| PlayerInput::new(Frame::new(0), TestInput { inp }))
            .collect();
        encode_bridge_inputs::<TestConfig>(&inputs).unwrap()
    }

    #[test]
    fn bridge_inputs_roundtrip_in_handle_order() {
        let wire = sample_bridge_wire(&[7, 9, 42]);
        // TestInput is one byte wide under the fixed-int codec.
        assert_eq!(wire.len(), 3);

        let decoded = decode_bridge_inputs::<TestConfig>(&wire, 3).unwrap();

        assert_eq!(
            decoded,
            vec![
                TestInput { inp: 7 },
                TestInput { inp: 9 },
                TestInput { inp: 42 }
            ]
        );
    }

    #[test]
    fn decode_bridge_inputs_rejects_trailing_bytes() {
        let mut wire = sample_bridge_wire(&[7, 9]);
        wire.push(0xFF); // one trailing byte after the two decoded inputs

        let result = decode_bridge_inputs::<TestConfig>(&wire, 2);

        assert!(matches!(
            result,
            Err(FortressError::SerializationErrorStructured { .. })
        ));
    }

    #[test]
    fn decode_bridge_inputs_rejects_truncated_blob() {
        let wire = sample_bridge_wire(&[7]); // one input present, two expected

        let result = decode_bridge_inputs::<TestConfig>(&wire, 2);

        assert!(matches!(
            result,
            Err(FortressError::SerializationErrorStructured { .. })
        ));
    }

    #[test]
    fn decode_bridge_inputs_rejects_empty_blob() {
        // The fail-closed core of the serve-shape discrimination: an N-peer
        // joiner must never invent bridge inputs from a bridge-less snapshot.
        let result = decode_bridge_inputs::<TestConfig>(&[], 2);

        assert!(matches!(
            result,
            Err(FortressError::SerializationErrorStructured { .. })
        ));
    }

    /// Two-player N-peer capture statuses: slot 0 live through `frame`,
    /// slot 1 (the joining slot) frozen strictly below it.
    fn sample_statuses(frame: Frame) -> Vec<ConnectionStatus> {
        vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: frame,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(frame.as_i32() - 2),
            },
        ]
    }

    #[test]
    fn capture_npeer_snapshot_includes_bridge_inputs_in_wire_budget() {
        let frame = Frame::new(3);
        let state = VecState::sample();
        let layer = layer_with_saved_state(2, frame, state, Some(0x55));
        let state_len = serialized_state_len::<TestConfig>(&VecState::sample()).unwrap();
        // The exact complete wire length of the bridge-less, status-less
        // message: the 2-peer capture fits this budget exactly ...
        let exact_budget = snapshot_wire_len(frame, 2, Some(0x55), state_len, 0, &[]).unwrap();
        assert!(
            capture_snapshot_with_max_wire_bytes(&layer, frame, 2, exact_budget)
                .unwrap()
                .is_some()
        );

        // ... so an N-peer capture carrying two bridge bytes plus two
        // per-slot statuses must overflow it (the budget check must count
        // both fields).
        let result = capture_npeer_snapshot_with_max_wire_bytes(
            &layer,
            frame,
            2,
            exact_budget,
            sample_bridge_wire(&[7, 9]),
            sample_statuses(frame),
        );

        assert!(matches!(
            result,
            Err(FortressError::SerializationErrorStructured {
                kind: SerializationErrorKind::Custom(
                    "hot-join snapshot exceeds configured byte limit"
                ),
            })
        ));
    }

    #[test]
    fn capture_npeer_snapshot_rejects_empty_bridge_inputs() {
        let frame = Frame::new(3);
        let layer = layer_with_saved_state(2, frame, VecState::sample(), None);

        let result = capture_npeer_snapshot_with_max_wire_bytes(
            &layer,
            frame,
            2,
            4096,
            Vec::new(),
            sample_statuses(frame),
        );

        assert!(matches!(
            result,
            Err(FortressError::SerializationErrorStructured { .. })
        ));
    }

    #[test]
    fn capture_npeer_snapshot_rejects_status_count_mismatch() {
        let frame = Frame::new(3);
        let layer = layer_with_saved_state(2, frame, VecState::sample(), None);

        // One status for a two-player capture: fail closed at the producer.
        let mut statuses = sample_statuses(frame);
        statuses.truncate(1);
        let result = capture_npeer_snapshot_with_max_wire_bytes(
            &layer,
            frame,
            2,
            4096,
            sample_bridge_wire(&[7, 9]),
            statuses,
        );

        assert!(matches!(
            result,
            Err(FortressError::SerializationErrorStructured { .. })
        ));
    }

    #[test]
    fn capture_npeer_snapshot_rejects_slot_frozen_above_snapshot_frame() {
        let frame = Frame::new(3);
        let layer = layer_with_saved_state(2, frame, VecState::sample(), None);

        // A slot frozen ABOVE the snapshot frame has real ring history in
        // (S, f0] that the snapshot + one-frame bridge cannot carry: the
        // capture must refuse (the serve retries at a later S).
        let mut statuses = sample_statuses(frame);
        if let Some(status) = statuses.get_mut(1) {
            status.last_frame = Frame::new(frame.as_i32() + 1);
        }
        let result = capture_npeer_snapshot_with_max_wire_bytes(
            &layer,
            frame,
            2,
            4096,
            sample_bridge_wire(&[7, 9]),
            statuses,
        );

        assert!(matches!(
            result,
            Err(FortressError::SerializationErrorStructured { .. })
        ));
    }

    #[test]
    fn apply_npeer_snapshot_bridges_one_frame_and_positions_real_from_f() {
        let snapshot_frame = Frame::new(4);
        let activation_frame = Frame::new(5);
        let state = VecState::sample();
        let checksum = Some(0xABCD_u128);
        let host = layer_with_saved_state(2, snapshot_frame, state.clone(), checksum);
        let bridge_wire = sample_bridge_wire(&[7, 9]);
        let snapshot = capture_npeer_snapshot_with_max_wire_bytes(
            &host,
            snapshot_frame,
            2,
            4096,
            bridge_wire,
            sample_statuses(snapshot_frame),
        )
        .unwrap()
        .expect("a saved state exists at S");
        assert_eq!(
            snapshot.frame, snapshot_frame,
            "the N-peer snapshot is at S"
        );
        let bridge_inputs = decode_bridge_inputs::<TestConfig>(&snapshot.bridge_inputs, 2).unwrap();

        let mut joiner = SyncLayer::<TestConfig>::new(2, 8);
        let (load, bridge) = apply_npeer_snapshot(
            &mut joiner,
            &snapshot,
            &bridge_inputs,
            2,
            PlayerHandle::new(1), // the joining slot
        )
        .unwrap();

        // Request 1: LoadGameState at exactly S.
        match load {
            FortressRequest::LoadGameState { cell, frame } => {
                assert_eq!(frame, snapshot_frame);
                assert_eq!(cell.load(), Some(state));
                assert_eq!(cell.checksum(), checksum);
            },
            other => panic!("expected LoadGameState, got {other}"),
        }
        // Request 2: the bridge AdvanceFrame carrying the inputs in handle
        // order — the joining slot presented Disconnected (it was frozen at S
        // on every peer), the others Confirmed.
        match bridge {
            FortressRequest::AdvanceFrame { inputs } => {
                assert_eq!(
                    inputs.as_slice(),
                    &[
                        (TestInput { inp: 7 }, crate::InputStatus::Confirmed),
                        (TestInput { inp: 9 }, crate::InputStatus::Disconnected),
                    ]
                );
            },
            other => panic!("expected AdvanceFrame, got {other}"),
        }

        // The layer simulated the bridge: current = F = S + 1 (the precise
        // point a verbatim 2-peer apply_snapshot reuse gets wrong — it stops
        // at S), with the snapshot still the last save.
        assert_eq!(joiner.current_frame(), activation_frame);
        assert_eq!(joiner.last_saved_frame(), snapshot_frame);
        assert!(joiner.check_invariants().is_ok());

        // The carried inputs are stored confirmed at S ...
        assert_eq!(
            joiner
                .confirmed_input(PlayerHandle::new(0), snapshot_frame)
                .unwrap()
                .input,
            TestInput { inp: 7 }
        );
        // ... and every queue accepts its first REAL input at exactly F —
        // including the JOINING slot's own queue (carried frozen below S,
        // but seeded anyway: it goes live at F).
        joiner.add_remote_input(
            PlayerHandle::new(0),
            PlayerInput::new(activation_frame, TestInput { inp: 11 }),
        );
        assert_eq!(
            joiner
                .confirmed_input(PlayerHandle::new(0), activation_frame)
                .unwrap()
                .input,
            TestInput { inp: 11 }
        );
        joiner.add_remote_input(
            PlayerHandle::new(1),
            PlayerInput::new(activation_frame, TestInput { inp: 13 }),
        );
        assert_eq!(
            joiner
                .confirmed_input(PlayerHandle::new(1), activation_frame)
                .unwrap()
                .input,
            TestInput { inp: 13 },
            "the joining slot's own queue accepts its first real input at F"
        );
    }

    #[test]
    fn apply_npeer_snapshot_bridge_count_mismatch_leaves_joiner_unmutated() {
        let snapshot_frame = Frame::new(4);
        let host = layer_with_saved_state(2, snapshot_frame, VecState::sample(), None);
        let snapshot = capture_npeer_snapshot_with_max_wire_bytes(
            &host,
            snapshot_frame,
            2,
            4096,
            sample_bridge_wire(&[7, 9]),
            sample_statuses(snapshot_frame),
        )
        .unwrap()
        .unwrap();

        let mut joiner = SyncLayer::<TestConfig>::new(2, 8);
        // Only one carried input for a two-player snapshot: fail closed.
        let result = apply_npeer_snapshot(
            &mut joiner,
            &snapshot,
            &[TestInput { inp: 7 }],
            2,
            PlayerHandle::new(1),
        );

        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::PlayerCountMismatch {
                    expected: 2,
                    actual: 1,
                }
            })
        ));
        assert_eq!(joiner.current_frame(), Frame::new(0));
        assert_eq!(joiner.last_saved_frame(), Frame::NULL);
    }

    /// S34 fix round 1, MAJOR-1 (unit half): a SECOND dead slot (carried
    /// `{disconnected, f0' < S}`) must be presented `Disconnected` on the
    /// bridge AND left frozen at its carried value — every survivor's
    /// simulation of `F` serves it through the frozen branch
    /// (`last_confirmed_input`, `Disconnected`), so a joiner that bridges it
    /// `Confirmed` and leaves the queue unfrozen serves the DEFAULT input
    /// with the right status afterwards: a silent value desync.
    #[test]
    fn apply_npeer_snapshot_freezes_carried_disconnected_slot_with_carried_value() {
        let snapshot_frame = Frame::new(4);
        let activation_frame = Frame::new(5);
        let host = layer_with_saved_state(3, snapshot_frame, VecState::sample(), None);
        let statuses = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: snapshot_frame,
            },
            // The second dead slot, frozen strictly below S.
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
            },
            // The joining slot itself.
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(3),
            },
        ];
        let snapshot = capture_npeer_snapshot_with_max_wire_bytes(
            &host,
            snapshot_frame,
            3,
            4096,
            sample_bridge_wire(&[7, 9, 11]),
            statuses,
        )
        .unwrap()
        .unwrap();
        let bridge_inputs = decode_bridge_inputs::<TestConfig>(&snapshot.bridge_inputs, 3).unwrap();

        let mut joiner = SyncLayer::<TestConfig>::new(3, 8);
        let (_load, bridge) = apply_npeer_snapshot(
            &mut joiner,
            &snapshot,
            &bridge_inputs,
            3,
            PlayerHandle::new(2), // the joining slot
        )
        .unwrap();

        // Bridge statuses from the carried predicate: the live slot
        // Confirmed, BOTH dead slots Disconnected.
        match bridge {
            FortressRequest::AdvanceFrame { inputs } => {
                assert_eq!(
                    inputs.as_slice(),
                    &[
                        (TestInput { inp: 7 }, crate::InputStatus::Confirmed),
                        (TestInput { inp: 9 }, crate::InputStatus::Disconnected),
                        (TestInput { inp: 11 }, crate::InputStatus::Disconnected),
                    ]
                );
            },
            other => panic!("expected AdvanceFrame, got {other}"),
        }

        // From F onward the dead slot must serve its CARRIED value through
        // the frozen branch, exactly as every survivor serves it. The
        // statuses below are what the joiner session stamps at apply: live
        // slots connected through S, the dead slot's carried status
        // verbatim, the (now live) own slot connected through S.
        let stamped = [
            ConnectionStatus {
                disconnected: false,
                last_frame: snapshot_frame,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
            },
            ConnectionStatus {
                disconnected: false,
                last_frame: snapshot_frame,
            },
        ];
        let at_f = joiner
            .synchronized_inputs(&stamped)
            .expect("synchronized inputs at F");
        assert_eq!(joiner.current_frame(), activation_frame);
        assert_eq!(
            at_f.get(1).copied(),
            Some((TestInput { inp: 9 }, crate::InputStatus::Disconnected)),
            "the second dead slot serves its carried frozen value (not the \
             default input) with Disconnected at F — the queue must be \
             frozen with the carried value at apply"
        );
    }

    /// S34 fix round 1, CRITICAL-2 (unit half): a slot whose carried freeze
    /// frame EQUALS the snapshot frame (`f0 == S`) is presented `Confirmed`
    /// on the bridge — `synchronized_inputs`' frozen branch requires
    /// `last_frame < S`, so every survivor's history of `S` serves the
    /// slot's real frame-`S` input with a non-`Disconnected` status.
    #[test]
    fn apply_npeer_snapshot_presents_slot_frozen_at_snapshot_frame_confirmed() {
        let snapshot_frame = Frame::new(4);
        let host = layer_with_saved_state(2, snapshot_frame, VecState::sample(), None);
        let statuses = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: snapshot_frame,
            },
            // The joining slot, frozen exactly AT the snapshot frame.
            ConnectionStatus {
                disconnected: true,
                last_frame: snapshot_frame,
            },
        ];
        let snapshot = capture_npeer_snapshot_with_max_wire_bytes(
            &host,
            snapshot_frame,
            2,
            4096,
            sample_bridge_wire(&[7, 9]),
            statuses,
        )
        .unwrap()
        .unwrap();
        let bridge_inputs = decode_bridge_inputs::<TestConfig>(&snapshot.bridge_inputs, 2).unwrap();

        let mut joiner = SyncLayer::<TestConfig>::new(2, 8);
        let (_load, bridge) = apply_npeer_snapshot(
            &mut joiner,
            &snapshot,
            &bridge_inputs,
            2,
            PlayerHandle::new(1),
        )
        .unwrap();

        match bridge {
            FortressRequest::AdvanceFrame { inputs } => {
                assert_eq!(
                    inputs.as_slice(),
                    &[
                        (TestInput { inp: 7 }, crate::InputStatus::Confirmed),
                        (TestInput { inp: 9 }, crate::InputStatus::Confirmed),
                    ],
                    "f0 == S presents the slot's real frame-S input Confirmed \
                     (the frozen branch requires last_frame < S)"
                );
            },
            other => panic!("expected AdvanceFrame, got {other}"),
        }
    }

    /// S34 fix round 1: status-count and frozen-bound validation precede any
    /// joiner-layer mutation.
    #[test]
    fn apply_npeer_snapshot_status_count_mismatch_leaves_joiner_unmutated() {
        let snapshot_frame = Frame::new(4);
        let host = layer_with_saved_state(2, snapshot_frame, VecState::sample(), None);
        let mut snapshot = capture_npeer_snapshot_with_max_wire_bytes(
            &host,
            snapshot_frame,
            2,
            4096,
            sample_bridge_wire(&[7, 9]),
            sample_statuses(snapshot_frame),
        )
        .unwrap()
        .unwrap();
        snapshot.bridge_statuses.truncate(1);
        let bridge_inputs = decode_bridge_inputs::<TestConfig>(&snapshot.bridge_inputs, 2).unwrap();

        let mut joiner = SyncLayer::<TestConfig>::new(2, 8);
        let result = apply_npeer_snapshot(
            &mut joiner,
            &snapshot,
            &bridge_inputs,
            2,
            PlayerHandle::new(1),
        );

        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::PlayerCountMismatch {
                    expected: 2,
                    actual: 1,
                }
            })
        ));
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

    /// S34 fix round 2, MAJOR-A (the round-2 reviewer's probe, promoted): a
    /// joiner that ever re-simulates the bridge frame — concretely, a
    /// `SaveMode::Sparse` joiner whose first rollback lands before its first
    /// post-apply sparse save has `frame_to_load = last_saved = S`
    /// (`adjust_gamestate`'s sparse arm) — replays `S -> F` through
    /// `synchronized_inputs(S)` with the connection statuses the session
    /// STAMPED at apply (`apply_buffered_npeer_snapshot`: own slot
    /// `{connected, S}`), not the CARRIED statuses the original bridge used.
    /// Without a reactivation floor on the joiner's own slot (every
    /// survivor/coordinator reopen site arms one), an own slot carried
    /// `{disconnected, f0 < S}` (the COMMON shape) originally presented
    /// `Disconnected` but replays `Confirmed` — a silent cross-peer desync
    /// for any disconnect-folding app. The fix arms the floor at apply; this
    /// test pins replay == original presentation.
    #[test]
    fn apply_npeer_snapshot_bridge_frame_replay_presents_original_statuses() {
        let snapshot_frame = Frame::new(4);
        let activation_frame = Frame::new(5);
        let host = layer_with_saved_state(2, snapshot_frame, VecState::sample(), Some(0xAB));
        let snapshot = capture_npeer_snapshot_with_max_wire_bytes(
            &host,
            snapshot_frame,
            2,
            4096,
            sample_bridge_wire(&[7, 9]),
            sample_statuses(snapshot_frame), // joining slot frozen at S - 2 < S
        )
        .unwrap()
        .unwrap();
        let bridge_inputs = decode_bridge_inputs::<TestConfig>(&snapshot.bridge_inputs, 2).unwrap();

        let mut joiner = SyncLayer::<TestConfig>::new(2, 8);
        let (_load, bridge) = apply_npeer_snapshot(
            &mut joiner,
            &snapshot,
            &bridge_inputs,
            2,
            PlayerHandle::new(1), // the joining slot
        )
        .unwrap();
        let original = match bridge {
            FortressRequest::AdvanceFrame { inputs } => inputs,
            other => panic!("expected AdvanceFrame, got {other}"),
        };
        assert_eq!(joiner.current_frame(), activation_frame);

        // Exactly what `apply_buffered_npeer_snapshot` stamps for this
        // snapshot: live slot {connected, min(carried, S)} = {connected, S},
        // own (joining) slot {connected, S}.
        let stamped = [
            ConnectionStatus {
                disconnected: false,
                last_frame: snapshot_frame,
            },
            ConnectionStatus {
                disconnected: false,
                last_frame: snapshot_frame,
            },
        ];

        // The Sparse rollback: load the only saved state (S), then re-request
        // the bridge frame the way `adjust_gamestate`'s re-simulation does.
        joiner
            .load_frame(snapshot_frame)
            .expect("S is saved and inside the prediction window");
        assert_eq!(joiner.current_frame(), snapshot_frame);
        let replayed = joiner
            .synchronized_inputs(&stamped)
            .expect("replay of the bridge frame");

        assert_eq!(
            replayed.as_slice(),
            original.as_slice(),
            "re-simulating the bridge frame S must reproduce the original \
             bridge presentation exactly (status included); a divergence \
             desyncs any app that folds the Disconnected bit after a \
             Sparse-mode rollback to S"
        );
    }

    /// S34 fix round 2, MAJOR-A boundary pin: an own slot carried frozen
    /// exactly AT `S` (`f0 == S`, the idle-lobby drop) has an EMPTY floor
    /// window `(f0, F)` — its real frame-`S` input was ring-seeded and both
    /// the original bridge and any replay of `S` must present it
    /// `Confirmed` from the ring. The fix must therefore NOT arm a floor for
    /// this shape (an over-armed floor would fail the replay closed at
    /// `S <= frozen_bound` even though the ring holds the real entry).
    #[test]
    fn apply_npeer_snapshot_replay_at_freeze_equals_snapshot_boundary_stays_confirmed() {
        let snapshot_frame = Frame::new(4);
        let host = layer_with_saved_state(2, snapshot_frame, VecState::sample(), Some(0xAC));
        let statuses = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: snapshot_frame,
            },
            // The joining slot, frozen exactly AT the snapshot frame.
            ConnectionStatus {
                disconnected: true,
                last_frame: snapshot_frame,
            },
        ];
        let snapshot = capture_npeer_snapshot_with_max_wire_bytes(
            &host,
            snapshot_frame,
            2,
            4096,
            sample_bridge_wire(&[7, 9]),
            statuses,
        )
        .unwrap()
        .unwrap();
        let bridge_inputs = decode_bridge_inputs::<TestConfig>(&snapshot.bridge_inputs, 2).unwrap();

        let mut joiner = SyncLayer::<TestConfig>::new(2, 8);
        let (_load, bridge) = apply_npeer_snapshot(
            &mut joiner,
            &snapshot,
            &bridge_inputs,
            2,
            PlayerHandle::new(1),
        )
        .unwrap();
        let original = match bridge {
            FortressRequest::AdvanceFrame { inputs } => inputs,
            other => panic!("expected AdvanceFrame, got {other}"),
        };
        // The boundary presents the real frame-S input Confirmed mesh-wide
        // (S34 fix round 1, CRITICAL-2).
        assert_eq!(
            original.as_slice(),
            &[
                (TestInput { inp: 7 }, InputStatus::Confirmed),
                (TestInput { inp: 9 }, InputStatus::Confirmed),
            ]
        );

        let stamped = [
            ConnectionStatus {
                disconnected: false,
                last_frame: snapshot_frame,
            },
            ConnectionStatus {
                disconnected: false,
                last_frame: snapshot_frame,
            },
        ];
        joiner
            .load_frame(snapshot_frame)
            .expect("S is saved and inside the prediction window");
        let replayed = joiner
            .synchronized_inputs(&stamped)
            .expect("replay of the bridge frame at the f0 == S boundary must serve the seeded ring entry, not fail closed");

        assert_eq!(
            replayed.as_slice(),
            original.as_slice(),
            "the f0 == S boundary must replay Confirmed from the seeded ring \
             (no floor window exists for an empty (f0, F))"
        );
    }
}
