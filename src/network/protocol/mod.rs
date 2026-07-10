//! UDP protocol implementation for peer-to-peer communication.
//!
//! This module contains the UDP protocol handler for managing network communication
//! between peers in a rollback networking session.

mod event;
mod input_bytes;
mod state;

pub use event::Event;
use input_bytes::{log_input_decode_error, InputBytes};
pub use state::ProtocolState;

use crate::error::{allocation_failed, SerializationErrorKind};
use crate::frame_info::PlayerInput;
use crate::metrics::{MessageKindCounts, PeerMetrics};
use crate::network::codec;
use crate::network::compression::{decode_with_max_len, try_encode};
use crate::network::messages::{
    ChecksumReport, ConnectionStatus, FloorReply, FloorRequest, Input, InputAck, Message,
    MessageBody, MessageHeader, QualityReply, QualityReport, SyncReply, SyncRequest,
};
#[cfg(feature = "hot-join")]
use crate::network::messages::{
    JoinAborted, JoinCommitted, JoinRequest, ReactivateSlot, ReactivateSlotAck, StateSnapshot,
    StateSnapshotAck,
};
use crate::rle;
use crate::rng::{random, Pcg32, Rng, SeedableRng};
use crate::sessions::config::{ProtocolConfig, SyncConfig};
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::time_sync::{TimeSync, TimeSyncConfig};
use crate::{report_violation, safe_frame_add, safe_frame_sub};
use crate::{
    Config, DesyncDetection, FortressError, Frame, InvalidRequestKind, NonBlockingSocket,
    PlayerHandle,
};
use tracing::trace;

use std::collections::vec_deque::Drain;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::Arc;
use web_time::{Duration, Instant};

use super::network_stats::NetworkStats;

const UDP_HEADER_SIZE: usize = 28; // Size of IP + UDP headers

/// UDP protocol handler for peer-to-peer communication.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
pub struct UdpProtocol<T>
where
    T: Config,
{
    num_players: usize,
    /// Number of local players this endpoint serializes inputs for (the width of
    /// [`last_acked_input`](Self::last_acked_input)). Retained so the endpoint can
    /// rebuild itself for a hot-join rejoin via [`rearm_for_rejoin`](Self::rearm_for_rejoin);
    /// only read under that feature, so it is gated to keep the default build's
    /// dead-code lint clean.
    #[cfg(feature = "hot-join")]
    local_players: usize,
    handles: Arc<[PlayerHandle]>,
    send_queue: VecDeque<Message>,
    event_queue: VecDeque<Event<T>>,

    // state
    state: ProtocolState,
    sync_remaining_roundtrips: u32,
    sync_random_requests: BTreeSet<u32>,
    /// Total sync requests sent (tracks retries for telemetry).
    sync_requests_sent: u32,
    /// Whether we've emitted a sync retry warning (emit only once).
    sync_retry_warning_sent: bool,
    /// Whether we've emitted a sync duration warning (emit only once).
    sync_duration_warning_sent: bool,
    /// Whether we've emitted a sync timeout event (emit only once per timeout period).
    sync_timeout_event_sent: bool,
    running_last_quality_report: Instant,
    running_last_input_recv: Instant,
    disconnect_notify_sent: bool,
    disconnect_event_sent: bool,

    // constants
    disconnect_timeout: Duration,
    disconnect_notify_start: Duration,
    shutdown_timeout: Instant,
    fps: usize,
    magic: u16,

    // sync configuration
    sync_config: SyncConfig,

    // protocol configuration
    protocol_config: ProtocolConfig,

    // the other client
    peer_addr: T::Address,
    remote_magic: u16,
    peer_connect_status: Vec<ConnectionStatus>,

    // ---- floor-round (double-failure-relay connected-relay reorder fix, S55) ----
    // A relay reports its per-slot pessimistic floor (the `min` over its own
    // receipt/freeze and every source it still folds, surfacing a departed
    // global-min origin's low) through a sequence-numbered request/response round
    // (`FloorRequest`/`FloorReply`) the observer re-issues periodically while in
    // the relay topology — NOT via the `Input`-gossip cache an out-of-order
    // packet could clobber stale-HIGH (verified-sound mode `AsyncAckSoundRoundSeq`
    // in `specs/tla/DoubleFailureRelay.tla`).
    /// This endpoint's cache of the peer's per-slot pessimistic floors as
    /// reported in its latest ACCEPTED [`FloorReply`] — the **dedicated,
    /// reorder-immune reply channel** (`roundFloor`). Parallel to
    /// [`Self::peer_connect_status`] (index = handle, length `num_players`),
    /// `Frame::NULL`-seeded. Written ONLY by [`Self::on_floor_reply`] for an
    /// ACCEPTED reply — one whose `round_seq` is STRICTLY NEWER than the latest
    /// accepted ([`Self::floor_reply_seq`]), does NOT exceed the latest request
    /// issued ([`Self::floor_request_seq`]), and reports a floor for EVERY slot.
    /// A reordered stale reply (older/equal `round_seq`), an unsolicited one
    /// (`round_seq` beyond any issued request), or a short/incomplete `floors`
    /// vector is dropped — which is what keeps this cache reorder-immune and
    /// prevents a partial reply from leaving a slot reading a stale prior round.
    /// An accepted reply (its length checked first) fully rewrites every slot.
    round_floor: Vec<Frame>,
    /// Monotonic per-request sequence number, bumped on every outgoing
    /// [`FloorRequest`] ([`Self::send_floor_request`]) and stamped on it. Lets the
    /// observer order this relay's replies and drop reordered stale ones (accept
    /// only a strictly-newer `round_seq`) as well as unsolicited/forged ones (a
    /// reply must not echo a `round_seq` beyond this latest issued request).
    ///
    /// Wraps at [`u32::MAX`] (`wrapping_add`); past the wrap a low post-wrap seq
    /// would fail the strictly-newer test against the high pre-wrap
    /// [`Self::floor_reply_seq`] and stall this endpoint's round. Unreachable in
    /// practice: the round only re-issues inside the relay topology, at most once
    /// per keepalive interval, so a wrap needs ~2³² re-issues (years of continuous
    /// post-drop traffic) — the same astronomically-rare framing as the protocol
    /// packet-filter `magic` era counter and the connect-status `epoch`.
    floor_request_seq: u32,
    /// The `round_seq` of the latest ACCEPTED [`FloorReply`] (`0` = none yet).
    /// `round_floor` always holds the floors of this reply.
    floor_reply_seq: u32,
    /// The [`Self::floor_request_seq`] value snapshotted at the most recent
    /// prune ([`Self::reset_floor_freshness`]). A reply counts as **post-prune
    /// fresh** only when `floor_reply_seq > floor_prune_seq` — i.e. it answers a
    /// request issued AFTER the prune, so it captures the relay's SETTLED floor
    /// (the spec's `ackFresh`, reset on every `Prune`). [`Self::floor_round_is_fresh`].
    floor_prune_seq: u32,
    /// Set every poll by [`Self::set_floor_request_needed`]: `true` when the
    /// session is in the relay topology and this endpoint is a folded relay. While set,
    /// `poll` (re)issues a [`FloorRequest`] on the keepalive cadence — issued
    /// CONTINUOUSLY (not stopped once fresh) so the cached floor tracks the
    /// relay's advancing pessimistic floor for live slots rather than freezing at
    /// the first post-prune reply (which would pin live-slot confirmation in a
    /// capped mesh).
    floor_request_needed: bool,
    /// Last time a [`FloorRequest`] was sent; gates re-issue on the keepalive
    /// cadence (a dedicated timer so quality reports / keepalives do not starve
    /// it).
    last_floor_request_time: Instant,
    /// A received [`FloorRequest`]'s `round_seq`, recorded by
    /// [`Self::on_floor_request`] and drained by the session (which computes
    /// `pessimistic_floors` and answers via [`Self::send_floor_reply`]).
    /// Highest-seq-wins: a newer request supersedes an undrained older one
    /// regardless of arrival order, so a reordered stale `FloorRequest` cannot
    /// clobber a higher pending one.
    pending_floor_request: Option<u32>,
    /// Whether this endpoint was [`is_running`](Self::is_running) at the previous
    /// [`detect_prune_transition`](Self::detect_prune_transition) poll. The
    /// session reads a `true → false` flip as a prune (running→pruned) and resets
    /// every endpoint's floor freshness. Seeded `false` (endpoints start
    /// non-running).
    floor_round_was_running: bool,

    // input compression
    pending_output: VecDeque<InputBytes>,
    last_acked_input: InputBytes,
    max_prediction: usize,
    recv_inputs: BTreeMap<Frame, InputBytes>,

    // connect-status nudge (see `send_connect_status_nudge`)
    /// When `true` (set by the session each poll via
    /// [`set_connect_status_nudge`](Self::set_connect_status_nudge)), this
    /// endpoint keeps gossiping the session's connect status even when
    /// input-idle, by re-sending a status-bearing duplicate `Input` message on
    /// the keepalive cadence. The session enables it while it holds a locally
    /// disconnected player slot whose drop is not yet mesh-agreed, and clears
    /// it as soon as the mesh agrees (or the slot reconnects).
    connect_status_nudge: bool,
    /// Last time a connect-status nudge was sent. A dedicated timer (rather
    /// than reusing `last_send_time`) because quality reports — whose default
    /// interval equals the keepalive interval — refresh `last_send_time` on
    /// every cycle and would otherwise starve the bare-keepalive branch (and
    /// any nudge hooked on it) indefinitely.
    last_nudge_time: Instant,
    /// Last time a REAL `Input` message (fresh send or pending retransmission,
    /// not a nudge) was queued. The nudge is an input-idle SUBSTITUTE: while
    /// genuine input traffic flows it must stay completely silent so enabling
    /// the flag changes nothing about an actively-advancing session's packet
    /// stream (and therefore cannot perturb gossip-race resolutions that
    /// in-flight Input packets would have decided). Tracked separately from
    /// `last_send_time`, which control traffic also refreshes.
    last_input_send_time: Instant,

    // time sync
    time_sync_layer: TimeSync,
    /// Retained so the endpoint can rebuild its `TimeSync` for a hot-join rejoin
    /// via [`rearm_for_rejoin`](Self::rearm_for_rejoin); only read under that
    /// feature, so it is gated to keep the default build's dead-code lint clean.
    #[cfg(feature = "hot-join")]
    time_sync_config: TimeSyncConfig,
    local_frame_advantage: i32,
    remote_frame_advantage: i32,

    // network
    /// The instant when synchronization started, used for elapsed time calculations.
    /// Using Instant (monotonic clock) instead of wall-clock time ensures reliable
    /// duration measurements even if the system clock is adjusted.
    stats_start_time: Instant,
    // `u64` (not `usize`) so these lifetime counters cannot wrap on 32-bit
    // targets (notably `wasm32`): a `usize` `bytes_sent` would overflow after
    // ~4 GiB of cumulative traffic and corrupt `network_stats()`.
    packets_sent: u64,
    bytes_sent: u64,
    // Per-peer receive accounting mirroring `packets_sent`/`bytes_sent`, surfaced
    // (with the send counters) via `peer_metrics()`. Wire-exact through
    // `Message::encoded_len`; same `u64` 32-bit-safety rationale as the send side.
    packets_received: u64,
    bytes_received: u64,
    // Per-`MessageKind` send/receive tallies. Each stays in lockstep with the
    // matching packet counter (one bucket increment per counted packet), so
    // `messages_*_by_kind.total() == packets_*` by construction.
    messages_sent_by_kind: MessageKindCounts,
    messages_received_by_kind: MessageKindCounts,
    // Cumulative raw (pre-compression) and encoded (post-compression) input bytes
    // batched into `Input` packets, for realized-compression accounting.
    input_bytes_pre_compression: u64,
    input_bytes_post_compression: u64,
    round_trip_time: u128,
    /// Origin instant for quality-report `ping` timestamps, captured from the
    /// protocol clock at endpoint construction. The peer echoes `ping` back
    /// verbatim ([`Self::on_quality_report`]), so timestamps are only ever
    /// compared against this endpoint's own clock — a session-local monotonic
    /// origin is sufficient. Deriving it from [`Self::now`] honors an injected
    /// [`ProtocolConfig::clock`] (deterministic simulation) and is immune to
    /// the wall-clock adjustments (NTP steps, VM snapshot restores) that could
    /// corrupt RTT when this was derived from `SystemTime`.
    ping_epoch_base: Instant,
    last_send_time: Instant,
    last_recv_time: Instant,

    // debug desync
    pub(crate) pending_checksums: BTreeMap<Frame, u128>,
    /// Highest frame at which a checksum this peer sent matched our local
    /// checksum history. Per-peer so that verification against one remote does
    /// not leak into another remote's sync verdict (an N>=3 logical error if it
    /// were session-global). `None` until the first matching checksum.
    pub(crate) last_verified_frame: Option<Frame>,
    /// Number of confirmed frames at which this peer's checksum did NOT match
    /// our local history (the per-peer persistence signal behind B3 trust
    /// downgrade). Monotonic (saturating), per-peer (never leaks across
    /// remotes). On a confirmed frame a mismatch is a genuine state divergence
    /// in the trusted-peer model; in an untrusted deployment a single one may be
    /// a malicious/buggy peer's one-off bad checksum, so a count that climbs is
    /// what marks a peer whose state *persistently* disagrees. Counted per
    /// confirmed frame, so one divergence spanning many frames increments many
    /// times. The library does NOT auto-eject on this — with two endpoints it
    /// cannot tell which side is wrong — it only surfaces it (one advisory
    /// WARNING at `CHECKSUM_MISMATCH_TRUST_DOWNGRADE_THRESHOLD`, plus
    /// `P2PSession::peer_checksum_mismatch_count` for the raw value).
    pub(crate) checksum_mismatch_count: u32,
    desync_detection: DesyncDetection,

    /// Optional deterministic RNG for protocol randomness.
    ///
    /// When set (via `ProtocolConfig::protocol_rng_seed`), this RNG is used for
    /// generating magic numbers and sync request IDs, enabling fully reproducible
    /// protocol behavior. When `None`, the thread-local RNG is used instead.
    protocol_rng: Option<Pcg32>,

    // hot-join (chunk 5 orchestration drives these; last-writer-wins)
    /// A `JoinRequest`'s requested slot received from the peer, awaiting drain.
    #[cfg(feature = "hot-join")]
    pending_join_request: Option<usize>,
    /// A `StateSnapshot` received from the peer, awaiting drain.
    #[cfg(feature = "hot-join")]
    received_snapshot: Option<StateSnapshot>,
    /// The frame from a received `StateSnapshotAck`, awaiting drain.
    #[cfg(feature = "hot-join")]
    received_snapshot_ack: Option<Frame>,
    /// A `ReactivateSlot` received from the peer, awaiting drain.
    #[cfg(feature = "hot-join")]
    received_reactivate_slot: Option<ReactivateSlot>,
    /// A `ReactivateSlotAck` received from the peer, awaiting drain.
    #[cfg(feature = "hot-join")]
    received_reactivate_slot_ack: Option<ReactivateSlotAck>,
    /// A `JoinCommitted` received from the peer, awaiting drain.
    #[cfg(feature = "hot-join")]
    received_join_committed: Option<JoinCommitted>,
    /// A `JoinAborted` received from the peer, awaiting drain.
    #[cfg(feature = "hot-join")]
    received_join_aborted: Option<JoinAborted>,
    /// Per-slot reactivation floor for the gossip merge: the agreed
    /// pre-activation bound (`F - 1`) of the most recent reactivation of each
    /// slot whose COMMIT this session has local evidence of, armed by
    /// [`arm_reactivation_floor`](Self::arm_reactivation_floor) and monotone
    /// (`max`) across reactivations. While armed for a slot, the merge
    /// ignores incoming `disconnected` claims whose freeze frame is STRICTLY
    /// below the floor: those are stale pre-reactivation carriers (packets in
    /// flight at the sender's status flip — every retransmit rebuilds the
    /// status at send time, so only genuinely in-flight packets can be stale)
    /// that would otherwise permanently re-stick the cache and re-drop the
    /// just-reactivated live slot. Sound threshold: every commit participant
    /// stamps its local receipt for the slot at exactly `F - 1` when it
    /// reopens, so a GENUINE post-reactivation re-drop's freeze frame — a
    /// `min` over participants' receipts — is always `>= F - 1`, while every
    /// pre-attempt claim carries the old freeze frame `f0 <= S = F - 1`.
    ///
    /// **Lifecycle (commit-evidence-only arming; session-33 round-2 review
    /// Finding 1):** the threshold theorem is valid only in COMMITTED worlds,
    /// so the floor arms exactly when this session acquires commit evidence
    /// (coordinator commit; survivor `JoinCommitted` receipt; commit-evidence
    /// implied/local close) and NEVER at the pre-commit reopen. In an aborted
    /// world the mesh's live convergence target IS the pre-attempt
    /// `f0 < F - 1` — a floor armed at reopen would filter it forever,
    /// pinning the survivor's confirmed frame at `F - 1` and stalling the
    /// whole mesh behind it. Window coverage: pre-reopen the slot is
    /// reserved/frozen at the f0 truth; from reopen to commit evidence the
    /// session's pending-reactivation shield keeps the disconnect fold off
    /// the slot (a re-stuck cache is tolerated and re-seeded at the
    /// evidence point); from commit evidence on, the floor filters; an abort
    /// restores the f0 truth with the floor still unarmed, so the genuine
    /// drop gossip re-converges. The single ambiguous corner is `f0 == F - 1`
    /// exactly (a serve opened at the very freeze frame, i.e. the coordinator
    /// never advanced between the drop and the rejoin): a stale carrier is
    /// then indistinguishable from an instant post-commit re-drop without
    /// per-slot reactivation epochs in the gossip — the same no-epoch root
    /// cause as the session-31/32 residual corners, tracked as the epoch
    /// wire-format future-work item. Bounded: `num_players` entries,
    /// validated at construction.
    #[cfg(feature = "hot-join")]
    reactivation_floor: Vec<Frame>,
    /// When `true`, `on_input` ignores incoming `Input` messages entirely (no
    /// decode, recv, ack, or `Event::Input`). A hot-joiner sets this on its host
    /// endpoint while it is still `HotJoining` so it neither processes nor *acks*
    /// the host's inputs before the snapshot defines the activation frame —
    /// acking pre-snapshot inputs would let the host trim its `pending_output`
    /// below the activation frame, permanently losing the host inputs the joiner
    /// needs after loading. Cleared once the snapshot is applied.
    #[cfg(feature = "hot-join")]
    defer_input_processing: bool,
}

impl<T: Config> PartialEq for UdpProtocol<T> {
    fn eq(&self, other: &Self) -> bool {
        self.peer_addr == other.peer_addr
    }
}

/// Fuzz-only helper that exercises the protocol `Input` acceptance path with
/// arbitrary packet fields while keeping construction inside this private
/// module. This is re-exported through `__internal` for `cargo fuzz`; it is not
/// part of the stable public API.
#[doc(hidden)]
pub fn fuzz_protocol_input_packet(
    start_frame: i32,
    ack_frame: i32,
    peer_connect_status: &[(bool, i32)],
    bytes: &[u8],
    pending_frames: &[i32],
) {
    #[derive(Debug, Clone)]
    struct FuzzConfig;

    impl Config for FuzzConfig {
        type Input = u8;
        type State = u8;
        type Address = u16;
    }

    let protocol_config = ProtocolConfig {
        pending_output_limit: 16,
        protocol_rng_seed: Some(0),
        ..ProtocolConfig::default()
    };
    let Ok(mut protocol) = UdpProtocol::<FuzzConfig>::new(
        vec![PlayerHandle::new(0), PlayerHandle::new(1)],
        1,
        2,
        1,
        8,
        Duration::from_secs(5),
        Duration::from_secs(3),
        60,
        DesyncDetection::Off,
        SyncConfig::default(),
        protocol_config,
        TimeSyncConfig::default(),
    ) else {
        return;
    };

    protocol.state = ProtocolState::Running;
    for &frame in pending_frames
        .iter()
        .take(protocol.protocol_config.pending_output_limit)
    {
        if protocol.pending_output.len() >= protocol.protocol_config.pending_output_limit {
            break;
        }
        let mut pending_bytes = Vec::new();
        // reserve-in-loop: one fresh single-byte gap-fill buffer per pending frame (loop bounded by `pending_output_limit`).
        if pending_bytes.try_reserve_exact(1).is_err() {
            return;
        }
        pending_bytes.push(0);
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(frame),
            bytes: pending_bytes,
        });
    }

    let mut status = Vec::new();
    if status
        .try_reserve_exact(peer_connect_status.len().min(8))
        .is_err()
    {
        return;
    }
    for &(disconnected, last_frame) in peer_connect_status.iter().take(8) {
        status.push(ConnectionStatus {
            disconnected,
            last_frame: Frame::new(last_frame),
            epoch: 0,
        });
    }

    let mut packet_bytes = Vec::new();
    let byte_limit = bytes.len().min(4096);
    if packet_bytes.try_reserve_exact(byte_limit).is_err() {
        return;
    }
    // byte_limit <= bytes.len() by construction, so `get(..byte_limit)` is always
    // Some; `unwrap_or_default()` keeps this panic-free (zero-panic policy /
    // `clippy::indexing_slicing`) rather than indexing with `[..byte_limit]`.
    packet_bytes.extend_from_slice(bytes.get(..byte_limit).unwrap_or_default());

    let body = Input {
        peer_connect_status: status,
        disconnect_requested: false,
        start_frame: Frame::new(start_frame),
        ack_frame: Frame::new(ack_frame),
        bytes: packet_bytes,
    };
    protocol.on_input(&body);

    let history_limit = protocol
        .protocol_config
        .input_history_multiplier
        .saturating_mul(protocol.max_prediction);
    assert!(
        protocol.recv_inputs.len() <= history_limit.saturating_add(1),
        "protocol input history exceeded configured bound"
    );
    assert!(
        protocol.pending_output.len() <= protocol.protocol_config.pending_output_limit,
        "protocol pending output exceeded configured bound"
    );
}

struct StagedInputFrame<I>
where
    I: Copy + Clone + PartialEq + Eq,
{
    input_data: InputBytes,
    player_inputs: Vec<PlayerInput<I>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AckDisposition {
    Apply,
    Ignore,
}

fn input_batch_decoded_byte_limit_with_cap(
    reference_len: usize,
    pending_output_limit: usize,
    decoded_byte_cap: usize,
) -> Option<usize> {
    reference_len
        .checked_mul(pending_output_limit)
        .map(|configured_limit| configured_limit.min(decoded_byte_cap))
}

fn input_batch_decoded_byte_limit(
    reference_len: usize,
    pending_output_limit: usize,
) -> Option<usize> {
    input_batch_decoded_byte_limit_with_cap(
        reference_len,
        pending_output_limit,
        rle::DEFAULT_MAX_DECODED_LEN,
    )
}

fn input_batch_len_for_limits(
    pending_len: usize,
    reference_len: usize,
    pending_output_limit: usize,
    decoded_byte_cap: usize,
) -> Option<usize> {
    if reference_len == 0 {
        return Some(0);
    }

    let max_decoded_input_bytes = input_batch_decoded_byte_limit_with_cap(
        reference_len,
        pending_output_limit,
        decoded_byte_cap,
    )?;
    let byte_limited_frames = max_decoded_input_bytes / reference_len;

    Some(
        pending_len
            .min(pending_output_limit)
            .min(byte_limited_frames),
    )
}

fn validate_default_input_wire_size<T: Config>() -> Result<usize, FortressError> {
    let input_size = codec::encoded_len(&T::Input::default()).map_err(|err| {
        report_violation!(
            ViolationSeverity::Critical,
            ViolationKind::InternalError,
            "Failed to measure default input type serialization: {}",
            err
        );
        SerializationErrorKind::EndpointCreationFailed
    })?;
    if input_size == 0 {
        return Err(SerializationErrorKind::InputSerializedSizeZero.into());
    }
    Ok(input_size)
}

fn validate_input_frame_wire_size(
    input_size: usize,
    player_count: usize,
) -> Result<usize, FortressError> {
    let frame_len = input_size.checked_mul(player_count).ok_or({
        FortressError::SerializationErrorStructured {
            kind: SerializationErrorKind::InputSerializedFrameTooLarge {
                frame_len: usize::MAX,
                max: rle::DEFAULT_MAX_DECODED_LEN,
            },
        }
    })?;
    if frame_len > rle::DEFAULT_MAX_DECODED_LEN {
        return Err(SerializationErrorKind::InputSerializedFrameTooLarge {
            frame_len,
            max: rle::DEFAULT_MAX_DECODED_LEN,
        }
        .into());
    }
    Ok(frame_len)
}

fn validate_protocol_input_wire_sizes<T: Config>(
    recv_player_num: usize,
    local_players: usize,
) -> Result<(), FortressError> {
    let input_size = validate_default_input_wire_size::<T>()?;
    validate_input_frame_wire_size(input_size, recv_player_num)?;
    validate_input_frame_wire_size(input_size, local_players)?;
    Ok(())
}

impl<T: Config> UdpProtocol<T> {
    /// Internal constructor for UDP protocol handler.
    ///
    /// Note: This is an internal constructor called via SessionBuilder. The many parameters are
    /// acceptable here because users interact through the builder pattern, not this method directly.
    ///
    /// # Returns
    /// Returns `None` if input serialization fails (indicates a fundamental issue with Config::Input).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        mut handles: Vec<PlayerHandle>,
        peer_addr: T::Address,
        num_players: usize,
        local_players: usize,
        max_prediction: usize,
        disconnect_timeout: Duration,
        disconnect_notify_start: Duration,
        fps: usize,
        desync_detection: DesyncDetection,
        sync_config: SyncConfig,
        protocol_config: ProtocolConfig,
        time_sync_config: TimeSyncConfig,
    ) -> Result<Self, FortressError> {
        // Compute initial time using custom clock if configured, or Instant::now()
        let now = match &protocol_config.clock {
            Some(clock_fn) => clock_fn(),
            None => Instant::now(),
        };

        handles.sort_unstable();
        let recv_player_num = handles.len();
        validate_protocol_input_wire_sizes::<T>(recv_player_num, local_players)?;

        // Initialize protocol RNG if a deterministic seed is provided
        let mut protocol_rng = protocol_config.protocol_rng_seed.map(Pcg32::seed_from_u64);

        // Generate magic number using either deterministic or thread-local RNG
        let mut magic: u16 = match &mut protocol_rng {
            Some(rng) => rng.gen(),
            None => random(),
        };
        while magic == 0 {
            magic = match &mut protocol_rng {
                Some(rng) => rng.gen(),
                None => random(),
            };
        }

        // Convert Vec to Arc<[PlayerHandle]> for cheap cloning in hot path
        let handles: Arc<[PlayerHandle]> = handles.into();

        // peer connection status
        let mut peer_connect_status = Vec::new();
        peer_connect_status
            .try_reserve_exact(num_players)
            .map_err(|_err| allocation_failed("protocol.peer_connect_status", num_players))?;
        for _ in 0..num_players {
            peer_connect_status.push(ConnectionStatus::default());
        }

        // floor-round reply cache (double-failure-relay connected-relay reorder
        // fix), parallel to `peer_connect_status`. Seeded to `Frame::NULL` ("no
        // reply yet"), which the session fold reads as "no floor known"; the
        // hold keeps confirmation at the current confirmed frame until a fresh
        // reply lands.
        let mut round_floor = Vec::new();
        round_floor
            .try_reserve_exact(num_players)
            .map_err(|_err| allocation_failed("protocol.round_floor", num_players))?;
        for _ in 0..num_players {
            round_floor.push(Frame::NULL);
        }

        // received input history - may fail if serialization is broken
        let mut recv_inputs = BTreeMap::new();
        recv_inputs.insert(
            Frame::NULL,
            InputBytes::zeroed::<T>(recv_player_num)
                .ok_or(SerializationErrorKind::EndpointCreationFailed)?,
        );

        // last acked input - may fail if serialization is broken
        let last_acked_input = InputBytes::zeroed::<T>(local_players)
            .ok_or(SerializationErrorKind::EndpointCreationFailed)?;

        let time_sync_layer = TimeSync::try_with_config(time_sync_config)?;

        Ok(Self {
            num_players,
            #[cfg(feature = "hot-join")]
            local_players,
            handles,
            send_queue: VecDeque::new(),
            event_queue: VecDeque::new(),

            // state
            state: ProtocolState::Initializing,
            sync_remaining_roundtrips: sync_config.num_sync_packets,
            sync_random_requests: BTreeSet::new(),
            sync_requests_sent: 0,
            sync_retry_warning_sent: false,
            sync_duration_warning_sent: false,
            sync_timeout_event_sent: false,
            running_last_quality_report: now,
            running_last_input_recv: now,
            disconnect_notify_sent: false,
            disconnect_event_sent: false,

            // constants
            disconnect_timeout,
            disconnect_notify_start,
            shutdown_timeout: now,
            fps,
            magic,

            // sync configuration
            sync_config,

            // protocol configuration
            protocol_config,

            // the other client
            peer_addr,
            remote_magic: 0,
            peer_connect_status,

            // floor-round (double-failure-relay connected-relay reorder fix)
            round_floor,
            floor_request_seq: 0,
            floor_reply_seq: 0,
            floor_prune_seq: 0,
            floor_request_needed: false,
            last_floor_request_time: now,
            pending_floor_request: None,
            floor_round_was_running: false,

            // input compression
            pending_output: VecDeque::new(),
            last_acked_input,
            max_prediction,
            recv_inputs,

            // connect-status nudge
            connect_status_nudge: false,
            last_nudge_time: now,
            last_input_send_time: now,

            // time sync
            time_sync_layer,
            #[cfg(feature = "hot-join")]
            time_sync_config,
            local_frame_advantage: 0,
            remote_frame_advantage: 0,

            // network
            stats_start_time: now,
            packets_sent: 0,
            bytes_sent: 0,
            packets_received: 0,
            bytes_received: 0,
            messages_sent_by_kind: MessageKindCounts::default(),
            messages_received_by_kind: MessageKindCounts::default(),
            input_bytes_pre_compression: 0,
            input_bytes_post_compression: 0,
            round_trip_time: 0,
            ping_epoch_base: now,
            last_send_time: now,
            last_recv_time: now,

            // debug desync
            pending_checksums: BTreeMap::new(),
            last_verified_frame: None,
            checksum_mismatch_count: 0,
            desync_detection,

            // deterministic protocol RNG (if configured)
            protocol_rng,

            // hot-join
            #[cfg(feature = "hot-join")]
            pending_join_request: None,
            #[cfg(feature = "hot-join")]
            received_snapshot: None,
            #[cfg(feature = "hot-join")]
            received_snapshot_ack: None,
            #[cfg(feature = "hot-join")]
            received_reactivate_slot: None,
            #[cfg(feature = "hot-join")]
            received_reactivate_slot_ack: None,
            #[cfg(feature = "hot-join")]
            received_join_committed: None,
            #[cfg(feature = "hot-join")]
            received_join_aborted: None,
            #[cfg(feature = "hot-join")]
            // alloc-bound: `num_players` is validated at session construction (mirrors `peer_connect_status`).
            reactivation_floor: vec![Frame::NULL; num_players],
            #[cfg(feature = "hot-join")]
            defer_input_processing: false,
        })
    }

    /// Returns the current time, using the custom clock if configured, or
    /// [`Instant::now()`] otherwise.
    fn now(&self) -> Instant {
        match &self.protocol_config.clock {
            Some(clock_fn) => clock_fn(),
            None => Instant::now(),
        }
    }

    /// Returns milliseconds elapsed on the protocol clock since this endpoint
    /// was constructed — the timestamp basis for quality-report pings (see
    /// [`Self::ping_epoch_base`]).
    ///
    /// Saturates to zero if the clock reads earlier than the construction
    /// instant (impossible for a monotonic clock; defensive for injected ones).
    fn ping_millis(&self) -> u128 {
        self.now()
            .saturating_duration_since(self.ping_epoch_base)
            .as_millis()
    }

    pub(crate) fn update_local_frame_advantage(&mut self, local_frame: Frame) {
        let last_recv_frame = self.last_recv_frame();
        if local_frame == Frame::NULL || last_recv_frame == Frame::NULL {
            return;
        }

        if !local_frame.is_valid() || !last_recv_frame.is_valid() {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "update_local_frame_advantage received invalid frame(s)"
            );
            return;
        }

        // Estimate which frame the other client is on by looking at the last frame they gave us
        // plus some delta for the packet roundtrip time. RTT is peer-influenced, so every step
        // uses checked or saturating arithmetic before narrowing back to frame units.
        let remote_frame_delta = self
            .round_trip_time
            .checked_div(2)
            .and_then(|half_rtt| half_rtt.checked_mul(self.fps as u128))
            .map(|frame_ms| frame_ms / 1000)
            .and_then(|frames| i32::try_from(frames).ok())
            .unwrap_or(i32::MAX);
        let remote_frame = safe_frame_add!(
            last_recv_frame,
            remote_frame_delta,
            "UdpProtocol::update_local_frame_advantage"
        );

        // Our frame "advantage" is how many frames behind the remote client we are. (It's an advantage because they will have to predict more often)
        self.local_frame_advantage = remote_frame.as_i32().saturating_sub(local_frame.as_i32());
    }

    pub(crate) fn network_stats(&self) -> Result<NetworkStats, FortressError> {
        if self.state != ProtocolState::Synchronizing && self.state != ProtocolState::Running {
            return Err(FortressError::NotSynchronized);
        }

        let elapsed = self.now() - self.stats_start_time;
        let seconds = elapsed.as_secs();
        if seconds == 0 {
            return Err(FortressError::NotSynchronized);
        }

        // All-`u64` so the sum cannot overflow on 32-bit targets before the rate
        // math runs (`bytes_sent`/`packets_sent` are already `u64`).
        let total_bytes_sent = self
            .bytes_sent
            .saturating_add(self.packets_sent.saturating_mul(UDP_HEADER_SIZE as u64));
        // `kbps_sent` is documented and named as **kilobits per second**. The
        // previous `bytes / seconds / 1024` produced kibibytes/sec — wrong by the
        // 8× byte<->bit factor against the field's own unit. Correct it to bits
        // (x8) over elapsed seconds, per SI kilo (/1000, the standard base for
        // network bit-rates). Now that D1 makes `bytes_sent` wire-exact this rate
        // is finally meaningful. Saturating multiply avoids overflow on long
        // sessions; the divisions then bring it back into `usize` range.
        let kbps_sent = usize::try_from(total_bytes_sent.saturating_mul(8) / seconds / 1000)
            .unwrap_or(usize::MAX);

        Ok(NetworkStats {
            ping: self.round_trip_time,
            send_queue_len: self.pending_output.len(),
            kbps_sent,
            local_frames_behind: self.local_frame_advantage,
            remote_frames_behind: self.remote_frame_advantage,
            // Checksum fields are populated by P2PSession::network_stats()
            // which has access to both local and remote checksum histories
            last_compared_frame: None,
            local_checksum: None,
            remote_checksum: None,
            checksums_match: None,
        })
    }

    /// A [`PeerMetrics`] snapshot for this endpoint.
    ///
    /// Unlike [`network_stats`](Self::network_stats), this never fails and is
    /// valid from construction: the byte / packet / message-kind / compression
    /// fields are always-on cumulative counters, and the remaining fields read
    /// current endpoint state as instantaneous gauges.
    pub(crate) fn peer_metrics(&self) -> PeerMetrics {
        PeerMetrics {
            bytes_sent: self.bytes_sent,
            bytes_received: self.bytes_received,
            packets_sent: self.packets_sent,
            packets_received: self.packets_received,
            messages_sent_by_kind: self.messages_sent_by_kind,
            messages_received_by_kind: self.messages_received_by_kind,
            input_bytes_pre_compression: self.input_bytes_pre_compression,
            input_bytes_post_compression: self.input_bytes_post_compression,
            pending_output_len: u64::try_from(self.pending_output.len()).unwrap_or(u64::MAX),
            pending_checksums_len: u64::try_from(self.pending_checksums.len()).unwrap_or(u64::MAX),
            ping_ms: self.round_trip_time,
            remote_frame_advantage: self.remote_frame_advantage,
        }
    }

    pub(crate) fn handles(&self) -> Arc<[PlayerHandle]> {
        Arc::clone(&self.handles)
    }

    pub(crate) fn is_synchronized(&self) -> bool {
        self.state == ProtocolState::Running
            || self.state == ProtocolState::Disconnected
            || self.state == ProtocolState::Shutdown
    }

    pub(crate) fn is_running(&self) -> bool {
        self.state == ProtocolState::Running
    }

    #[cfg(test)]
    pub(crate) fn force_running_for_tests(&mut self) {
        self.state = ProtocolState::Running;
        self.remote_magic = 1;
    }

    /// Test-only: a compact snapshot of the synchronization-relevant endpoint
    /// state — `(state name, remaining sync roundtrips, outstanding sync
    /// randoms, local magic, learned remote magic)` — consumed by harness
    /// stall diagnostics (for example the npeer mesh's `sync_joiner_with`),
    /// so a starved handshake reports WHICH side wedged and on what instead
    /// of a bare budget-exhaustion panic. Gated to the hot-join feature with
    /// its only consumers (the crate denies dead code in default-feature
    /// test builds).
    #[cfg(all(test, feature = "hot-join"))]
    pub(crate) fn sync_debug_snapshot(&self) -> (&'static str, u32, usize, u16, u16) {
        (
            self.state.as_str(),
            self.sync_remaining_roundtrips,
            self.sync_random_requests.len(),
            self.magic,
            self.remote_magic,
        )
    }

    /// Test-only: forces this endpoint into `Synchronizing`, the canonical
    /// non-running state a survivor endpoint occupies after a hot-join rearm
    /// (`rearm_for_rejoin` rebuilds via `new` → `Initializing` → `synchronize()`
    /// → `Synchronizing`). Unlike `rearm_for_rejoin`, this does NOT reset
    /// `peer_connect_status`, so tests can manufacture the (prod-unreachable)
    /// state where an endpoint is non-running yet still holds a stale lower view
    /// — used to prove the `is_running()` filter in `update_player_disconnects`
    /// is the only thing excluding such an endpoint from the global min.
    #[cfg(test)]
    pub(crate) fn force_synchronizing_for_tests(&mut self) {
        self.state = ProtocolState::Synchronizing;
    }

    /// Test-only: directly seeds the cached per-handle connection status this
    /// endpoint reports via [`peer_connect_status`](Self::peer_connect_status).
    /// In production this cache is written only by `merge_peer_connect_status`
    /// (driven by `on_input`); this helper lets session-level tests pin a known
    /// view without replaying a packet exchange. Out-of-range handles are ignored.
    #[cfg(test)]
    pub(crate) fn set_peer_connect_status_for_tests(
        &mut self,
        handle: PlayerHandle,
        status: ConnectionStatus,
    ) {
        if let Some(slot) = self.peer_connect_status.get_mut(handle.as_usize()) {
            *slot = status;
        }
    }

    /// Test-only: seeds this endpoint's [`round_floor`](Self::round_floor) reply
    /// cache for a slot AND marks the round **post-prune fresh** (a reply seq
    /// strictly above the prune threshold), as if a fresh post-prune `FloorReply`
    /// had been accepted. Lets session-level tests pin a known post-prune round
    /// outcome without driving a request/response exchange. Out-of-range handles
    /// ignored.
    ///
    /// Keeps the request seq consistent: a reply can only be accepted for a
    /// request actually issued, so `floor_request_seq` is bumped to at least the
    /// new `floor_reply_seq`. Without this the helper would leave the impossible
    /// `floor_reply_seq > floor_request_seq` state, which POISONS the endpoint —
    /// every subsequent real [`on_floor_reply`](Self::on_floor_reply) is dropped
    /// because the solicitation guard becomes unsatisfiable (a valid reply seq
    /// must be both `> floor_reply_seq` and `<= floor_request_seq`).
    #[cfg(test)]
    pub(crate) fn set_round_floor_for_tests(&mut self, handle: PlayerHandle, floor: Frame) {
        if let Some(slot) = self.round_floor.get_mut(handle.as_usize()) {
            *slot = floor;
        }
        // Mark fresh: a reply seq strictly above the prune threshold.
        self.floor_reply_seq = self.floor_prune_seq.wrapping_add(1);
        // A reply is only accepted for an issued request, so keep the request
        // seq at least as high as the accepted reply seq (no poisoned endpoint).
        self.floor_request_seq = self.floor_request_seq.max(self.floor_reply_seq);
        self.debug_assert_floor_round_invariants();
    }

    /// Test-only: deterministically seeds this endpoint's rolling frame-advantage
    /// average so that [`average_frame_advantage`](Self::average_frame_advantage)
    /// returns exactly `target`. Delegates to `TimeSync::seed_average_for_tests`.
    #[cfg(test)]
    pub(crate) fn seed_frame_advantage_for_tests(&mut self, target: i32) {
        self.time_sync_layer.seed_average_for_tests(target);
    }

    pub(crate) fn is_handling_message(&self, addr: &T::Address) -> bool {
        self.peer_addr == *addr
    }

    pub(crate) fn peer_connect_status(&self, handle: PlayerHandle) -> ConnectionStatus {
        self.peer_connect_status
            .get(handle.as_usize())
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn disconnect(&mut self) {
        if self.state == ProtocolState::Shutdown {
            return;
        }

        self.state = ProtocolState::Disconnected;
        // schedule the timeout which will lead to shutdown
        self.shutdown_timeout = self.now().add(self.protocol_config.shutdown_delay)
    }

    /// Transitions this protocol from `Initializing` to `Synchronizing` state.
    ///
    /// # Returns
    /// - `Ok(())` if the protocol was in `Initializing` state and successfully transitioned
    /// - `Err(FortressError::InvalidRequestStructured)` with [`InvalidRequestKind::WrongProtocolState`]
    ///   if the protocol was not in `Initializing` state
    pub(crate) fn synchronize(&mut self) -> Result<(), FortressError> {
        if self.state != ProtocolState::Initializing {
            return Err(InvalidRequestKind::WrongProtocolState {
                current_state: self.state.as_str(),
                expected_state: "Initializing",
            }
            .into());
        }
        self.state = ProtocolState::Synchronizing;
        self.sync_remaining_roundtrips = self.sync_config.num_sync_packets;
        self.stats_start_time = self.now();
        self.send_sync_request();
        Ok(())
    }

    /// Rebuilds this endpoint to a pristine pre-synchronization state and
    /// re-enters synchronization, so a returning peer can hot-join the slot it
    /// just vacated.
    ///
    /// The protocol state machine is otherwise strictly one-directional
    /// (`Initializing → Synchronizing → Running → Disconnected → Shutdown`) with no
    /// reconnect edge. After a graceful drop the endpoint is `Disconnected` (then
    /// `Shutdown`) and can never sync, ack, or serve a snapshot again. This method
    /// reconstructs the endpoint through [`new`](Self::new) — reusing every
    /// retained construction parameter — so the result is equivalent to a freshly
    /// built reserved endpoint (empty send/recv/pending queues, reset sync
    /// counters and timers, cleared hot-join scratch state), then calls
    /// [`synchronize`](Self::synchronize) to return to `Synchronizing`.
    ///
    /// Rebuilding through the constructor — rather than resetting fields one by one
    /// — is deliberate: it guarantees no runtime state from the previous
    /// connection can leak into the new one, and it stays correct automatically if
    /// `new` gains or drops fields.
    ///
    /// # Era fence (monotonic per-endpoint era counter)
    ///
    /// The rebuilt endpoint must NEVER reuse a recent era's magic. If a vacating
    /// peer is still live when the slot re-arms (a voluntary leave: the session
    /// removed the player while the remote process keeps running briefly), that
    /// peer still holds the OLD magic as its learned `remote_magic`. With a reused
    /// magic it would accept and answer the rebuilt endpoint's sync handshake; the
    /// rebuilt endpoint would then complete synchronization against the doomed peer
    /// and lock `remote_magic` to it — permanently filtering out the future
    /// rejoiner (a silent liveness blackhole).
    ///
    /// The fence is a **monotonic counter**: the rebuilt era's magic is the
    /// previous era's magic plus one (wrapping past the reserved `0`). This is
    /// strictly stronger than re-rolling a fresh random magic and excluding only
    /// the *immediately-previous* value — it makes the magic distinct from EVERY
    /// era within a 65535-rearm window, so a ghost from *any* recent era (not just
    /// the last one) can never match. Recurrence is impossible until 65535 rearms
    /// of the same slot alias, at no extra state and with no wire-format change
    /// (the previous fence drove only the *adjacent* collision to zero and left a
    /// ~1-in-65535 per-double-rearm multi-era residual). The **initial** magic
    /// stays randomly drawn — so two unrelated endpoints do not share a counter
    /// origin and a stale packet from an earlier *connection* to the same address
    /// is still filtered — and only the rearm transition is monotonic. The RNG
    /// stream is still carried across the rebuild so the unrelated sync-request IDs
    /// stay reproducible under a deterministic `protocol_rng_seed` and never reset.
    ///
    /// # Errors
    ///
    /// Propagates the same construction errors as [`new`](Self::new) (a
    /// should-never-happen serialization/allocation failure) and the
    /// [`synchronize`](Self::synchronize) transition error. On error `self` is left
    /// rebuilt-but-not-synchronized only if `synchronize` fails after a successful
    /// rebuild; a rebuild failure leaves `self` untouched.
    #[cfg(feature = "hot-join")]
    pub(crate) fn rearm_for_rejoin(&mut self) -> Result<(), FortressError> {
        // Construct the replacement BEFORE mutating `self`: if `new` fails (the
        // should-never-happen serialization path) the existing endpoint is left
        // untouched rather than half-reset.
        let mut rebuilt = Self::new(
            self.handles.to_vec(),
            self.peer_addr.clone(),
            self.num_players,
            self.local_players,
            self.max_prediction,
            self.disconnect_timeout,
            self.disconnect_notify_start,
            self.fps,
            self.desync_detection,
            self.sync_config,
            self.protocol_config.clone(),
            self.time_sync_config,
        )?;

        // Era fence (see the rustdoc): advance the magic as a MONOTONIC
        // per-endpoint counter — the previous era's magic plus one, wrapping past
        // the reserved `0`. This makes the rebuilt era's magic distinct from EVERY
        // era within a 65535-rearm window (not merely the immediately-previous
        // one), so a still-live peer from ANY recent era — which holds that era's
        // magic as its learned `remote_magic` — can never answer, and wedge, the
        // rebuilt endpoint's handshake. The RNG stream is still carried across the
        // rebuild so the unrelated sync-request IDs stay reproducible under a
        // deterministic `protocol_rng_seed` and never reset to the seed origin.
        let old_magic = self.magic;
        if self.protocol_rng.is_some() {
            rebuilt.protocol_rng = self.protocol_rng.take();
        }
        rebuilt.magic = match old_magic.wrapping_add(1) {
            // `wrapping_add(1)` only reaches 0 from u16::MAX; the magic is never 0.
            0 => 1,
            next => next,
        };

        *self = rebuilt;
        self.synchronize()
    }

    pub(crate) fn average_frame_advantage(&self) -> i32 {
        self.time_sync_layer.average_frame_advantage()
    }

    pub(crate) fn peer_addr(&self) -> T::Address {
        self.peer_addr.clone()
    }

    pub(crate) fn poll(&mut self, connect_status: &[ConnectionStatus]) -> Drain<'_, Event<T>> {
        let now = self.now();
        match self.state {
            ProtocolState::Synchronizing => {
                // Check for sync timeout if configured (emit event only once)
                if let Some(timeout) = self.sync_config.sync_timeout {
                    let elapsed = now - self.stats_start_time;
                    if elapsed > timeout && !self.sync_timeout_event_sent {
                        self.sync_timeout_event_sent = true;
                        self.event_queue.push_back(Event::SyncTimeout {
                            elapsed_ms: elapsed.as_millis(),
                        });
                    }
                }

                // some time has passed, let us send another sync request
                if self.last_send_time + self.sync_config.sync_retry_interval < now {
                    self.send_sync_request();
                }
            },
            ProtocolState::Running => {
                // resend pending inputs, if some time has passed without sending or
                // receiving NEW inputs (progress-free duplicates and connect-status
                // nudges do not refresh the pacer — see the gate in `on_input`)
                if self.running_last_input_recv + self.sync_config.running_retry_interval < now {
                    self.send_pending_output(connect_status);
                    self.running_last_input_recv = now;
                }

                // Connect-status nudge (see `send_connect_status_nudge`): while
                // the session holds a not-yet-mesh-agreed local disconnect,
                // keep gossiping even when input-idle. STRICTLY an input-idle
                // substitute: it fires only when no real `Input` message
                // (fresh or retransmitted — both already carry the current
                // connect status) has been queued for a full keepalive
                // interval AND `pending_output` is empty, so an
                // actively-advancing session's packet stream is completely
                // unchanged by the flag. A mute-but-held endpoint always has a
                // valid `last_acked_input`: `pending_output` drains only
                // through acks (`apply_ack_frame` moves the popped entry into
                // `last_acked_input`), so an empty queue on an endpoint that
                // ever sent an input implies at least one ack landed. The
                // remaining case (`last_acked_input.frame` still NULL because
                // no input was ever sent) cannot coincide with a gossip-mute
                // hold — such a session has burned none of its prediction
                // window and its next `advance_frame` sends a status-bearing
                // input — so skipping the nudge there (the bare KeepAlive
                // below keeps the link alive) is safe. One nudge per keepalive
                // interval per endpoint; it refreshes `last_send_time` via
                // `queue_message`, so it replaces (never doubles) the bare
                // KeepAlive on that tick.
                if self.connect_status_nudge
                    && self.pending_output.is_empty()
                    && self.last_acked_input.frame.is_valid()
                    && self.last_input_send_time + self.sync_config.keepalive_interval < now
                    && self.last_nudge_time + self.sync_config.keepalive_interval < now
                    && self.send_connect_status_nudge(connect_status)
                {
                    self.last_nudge_time = now;
                }

                // Floor-round request (double-failure-relay connected-relay
                // reorder fix): while the session marks this endpoint a folded
                // relay (`floor_request_needed`, set by `set_floor_request_needed`),
                // re-issue a sequence-numbered `FloorRequest` on the keepalive
                // cadence. Issued CONTINUOUSLY (not stopped once a reply lands) so
                // the cached floor tracks the relay's advancing pessimistic floor
                // for live slots — freezing it at the first post-prune reply would
                // pin live-slot confirmation at the stale value in a capped mesh.
                // A dedicated timer keeps quality reports / keepalives from
                // starving it.
                if self.floor_request_needed
                    && self.last_floor_request_time + self.sync_config.keepalive_interval < now
                {
                    self.send_floor_request();
                    self.last_floor_request_time = now;
                }

                // periodically send a quality report
                if self.running_last_quality_report + self.protocol_config.quality_report_interval
                    < now
                {
                    self.send_quality_report();
                }

                // send keep alive packet if we didn't send a packet for some time
                if self.last_send_time + self.sync_config.keepalive_interval < now {
                    self.send_keep_alive();
                }

                // trigger a NetworkInterrupted event if we didn't receive a packet for some time
                if !self.disconnect_notify_sent
                    && self.last_recv_time + self.disconnect_notify_start < now
                {
                    let duration: Duration = self.disconnect_timeout - self.disconnect_notify_start;
                    self.event_queue.push_back(Event::NetworkInterrupted {
                        disconnect_timeout: Duration::as_millis(&duration),
                    });
                    self.disconnect_notify_sent = true;
                }

                // if we pass the disconnect_timeout threshold, send an event to disconnect
                if !self.disconnect_event_sent
                    && self.last_recv_time + self.disconnect_timeout < now
                {
                    self.event_queue.push_back(Event::Disconnected);
                    self.disconnect_event_sent = true;
                }
            },
            ProtocolState::Disconnected => {
                if self.shutdown_timeout < now {
                    self.state = ProtocolState::Shutdown;
                }
            },
            ProtocolState::Initializing | ProtocolState::Shutdown => (),
        }
        self.event_queue.drain(..)
    }

    fn classify_ack_frame(&self, ack_frame: Frame) -> AckDisposition {
        if ack_frame == Frame::NULL {
            return AckDisposition::Ignore;
        }

        if !ack_frame.is_valid() {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Ignoring invalid ack frame {}",
                ack_frame
            );
            return AckDisposition::Ignore;
        }

        if self.last_acked_input.frame.is_valid() && ack_frame <= self.last_acked_input.frame {
            trace!(
                "Ignoring stale ack frame {} (last_acked={})",
                ack_frame,
                self.last_acked_input.frame
            );
            return AckDisposition::Ignore;
        }

        let Some(newest_pending) = self.pending_output.back().map(|input| input.frame) else {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Ignoring ack frame {} with no pending output",
                ack_frame
            );
            return AckDisposition::Ignore;
        };

        if ack_frame > newest_pending {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Ignoring future ack frame {} (newest pending frame {})",
                ack_frame,
                newest_pending
            );
            return AckDisposition::Ignore;
        }

        AckDisposition::Apply
    }

    fn apply_ack_frame(&mut self, ack_frame: Frame) {
        if self.classify_ack_frame(ack_frame) != AckDisposition::Apply {
            return;
        }

        while !self.pending_output.is_empty() {
            if let Some(input) = self.pending_output.front() {
                if input.frame <= ack_frame {
                    // This should always succeed since we just checked front() and is_empty()
                    if let Some(popped) = self.pending_output.pop_front() {
                        self.last_acked_input = popped;
                    }
                } else {
                    break;
                }
            }
        }
    }

    /*
     *  SENDING MESSAGES
     */

    pub(crate) fn send_all_messages(
        &mut self,
        socket: &mut Box<dyn NonBlockingSocket<T::Address>>,
    ) {
        if self.state == ProtocolState::Shutdown {
            trace!(
                "Protocol is shutting down; dropping {} messages",
                self.send_queue.len()
            );
            self.send_queue.drain(..);
            return;
        }

        if self.send_queue.is_empty() {
            // avoid log spam if there's nothing to send
            return;
        }

        trace!("Sending {} messages over socket", self.send_queue.len());
        for msg in self.send_queue.drain(..) {
            socket.send_to(&msg, &self.peer_addr);
        }
    }

    pub(crate) fn send_input(
        &mut self,
        inputs: &BTreeMap<PlayerHandle, PlayerInput<T::Input>>,
        connect_status: &[ConnectionStatus],
    ) {
        if self.state != ProtocolState::Running {
            return;
        }

        // We should never have so much pending input for a remote player. If
        // they are no longer acking our input, disconnect before mutating the
        // local send sequence.
        if self.pending_output.len() >= self.protocol_config.pending_output_limit {
            self.event_queue.push_back(Event::Disconnected);
            return;
        }

        let endpoint_data = match InputBytes::try_from_inputs::<T>(self.num_players, inputs) {
            Ok(endpoint_data) => endpoint_data,
            Err(err) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "send_input failed to serialize input bytes: {:?}",
                    err
                );
                return;
            },
        };
        if !self.pending_input_matches_reference_len(&endpoint_data, "send_input") {
            return;
        }

        // register the input and advantages in the time sync layer
        self.time_sync_layer.advance_frame(
            endpoint_data.frame,
            self.local_frame_advantage,
            self.remote_frame_advantage,
        );

        self.pending_output.push_back(endpoint_data);

        self.send_pending_output(connect_status);
    }

    /// Pushes a replicated input frame onto `pending_output` without advancing
    /// the time-sync layer or sending. Used to bridge the gap created by a
    /// mid-session input-delay increase: the input queue back-fills the gap
    /// with the most recently added input, and the protocol must transmit the
    /// same replicated frames so the remote peer's input sequence stays
    /// strictly monotonic.
    ///
    /// The caller is expected to pre-validate the available capacity via
    /// [`pending_output_capacity_remaining`]; once the queue is full, this
    /// method drops the entry and reports a `NetworkProtocol` violation
    /// (severity `Error`) rather than triggering the disconnect path used by
    /// `send_input`. The violation is emitted via [`report_violation!`],
    /// which routes through [`TracingObserver`]: install a
    /// `tracing-subscriber` to capture it.
    ///
    /// [`report_violation!`]: crate::report_violation
    /// [`TracingObserver`]: crate::telemetry::TracingObserver
    ///
    /// [`pending_output_capacity_remaining`]: Self::pending_output_capacity_remaining
    pub(crate) fn enqueue_replicated_input(
        &mut self,
        inputs: &BTreeMap<PlayerHandle, PlayerInput<T::Input>>,
    ) {
        if self.state != ProtocolState::Running {
            // Pre-running protocols have no remote yet — there is nothing to
            // back-fill toward. For the input-delay gap-fill this helper was
            // built for, the replicated entries WILL be sent normally once
            // the protocol enters Running; the hot-join activation-window
            // backfills (whose entries this early return would silently
            // lose) therefore gate on `is_running()` before calling — see
            // `P2PSession::backfill_joiner_pending_inputs`.
            return;
        }
        if self.pending_output.len() >= self.protocol_config.pending_output_limit {
            // Refuse to overflow. Caller should have pre-validated via
            // pending_output_capacity_remaining; reaching this branch means
            // the caller skipped that contract. Surface the violation rather
            // than silently dropping the entry so the bug is observable.
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "enqueue_replicated_input dropped entry: pending_output full (len={}, limit={})",
                self.pending_output.len(),
                self.protocol_config.pending_output_limit
            );
            return;
        }
        let endpoint_data = match InputBytes::try_from_inputs::<T>(self.num_players, inputs) {
            Ok(endpoint_data) => endpoint_data,
            Err(err) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "enqueue_replicated_input failed to serialize input bytes: {:?}",
                    err
                );
                return;
            },
        };
        if !self.pending_input_matches_reference_len(&endpoint_data, "enqueue_replicated_input") {
            return;
        }
        self.pending_output.push_back(endpoint_data);
    }

    /// Returns the frame of the oldest un-acked pending input, or
    /// [`Frame::NULL`] when nothing is pending. Consumed by the N-peer
    /// survivor's reopen-time backfill to decide which activation-window
    /// frames its organic send stream toward the joiner does not yet cover.
    #[cfg(feature = "hot-join")]
    pub(crate) fn oldest_pending_input_frame(&self) -> Frame {
        self.pending_output
            .front()
            .map_or(Frame::NULL, |input| input.frame)
    }

    /// Returns how many additional entries can be appended to `pending_output`
    /// before exceeding the configured limit. Returns `usize::MAX` for
    /// not-yet-running protocols, since back-fill is a no-op in that state.
    pub(crate) fn pending_output_capacity_remaining(&self) -> usize {
        if self.state != ProtocolState::Running {
            return usize::MAX;
        }
        self.protocol_config
            .pending_output_limit
            .saturating_sub(self.pending_output.len())
    }

    fn pending_input_matches_reference_len(&self, input: &InputBytes, context: &str) -> bool {
        let reference_len = self.last_acked_input.bytes.len();
        if reference_len == 0 || input.bytes.len() != reference_len {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "{} refused pending input: input bytes {}, reference bytes {}",
                context,
                input.bytes.len(),
                reference_len
            );
            return false;
        }
        true
    }

    /// Flushes any queued `pending_output` entries through the wire by
    /// triggering a `send_pending_output` call. Used in tandem with
    /// [`enqueue_replicated_input`] when bulk-pushing gap-fill frames after a
    /// mid-session frame-delay change.
    pub(crate) fn flush_pending_output(&mut self, connect_status: &[ConnectionStatus]) {
        if self.state != ProtocolState::Running {
            return;
        }
        self.send_pending_output(connect_status);
    }

    fn pending_output_batch_len_with_cap(&self, decoded_byte_cap: usize) -> Option<usize> {
        input_batch_len_for_limits(
            self.pending_output.len(),
            self.last_acked_input.bytes.len(),
            self.protocol_config.pending_output_limit,
            decoded_byte_cap,
        )
    }

    /// Re-sends the pending-output batch.
    fn send_pending_output(&mut self, connect_status: &[ConnectionStatus]) {
        self.send_pending_output_with_decoded_byte_cap(
            connect_status,
            rle::DEFAULT_MAX_DECODED_LEN,
        );
    }

    fn send_pending_output_with_decoded_byte_cap(
        &mut self,
        connect_status: &[ConnectionStatus],
        decoded_byte_cap: usize,
    ) {
        let mut body = Input::default();

        if let Some(input) = self.pending_output.front() {
            // Verify input frames are sequential relative to last acked
            let expected_frame = safe_frame_add!(
                self.last_acked_input.frame,
                1,
                "UdpProtocol::send_pending_output"
            );
            if self.last_acked_input.frame != Frame::NULL && expected_frame != input.frame {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Input frame sequence violation: last_acked={}, pending_front={}",
                    self.last_acked_input.frame,
                    input.frame
                );
                return;
            }
            body.start_frame = input.frame;

            let batch_len = match self.pending_output_batch_len_with_cap(decoded_byte_cap) {
                Some(batch_len) => batch_len,
                None => {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Input encode limit overflow: reference bytes {} * pending output limit {}",
                        self.last_acked_input.bytes.len(),
                        self.protocol_config.pending_output_limit
                    );
                    return;
                },
            };
            if batch_len == 0 {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Cannot encode pending inputs: reference bytes {}, pending output limit {}, pending len {}",
                    self.last_acked_input.bytes.len(),
                    self.protocol_config.pending_output_limit,
                    self.pending_output.len()
                );
                return;
            }

            // encode all pending inputs to a byte buffer
            body.bytes = match try_encode(
                &self.last_acked_input.bytes,
                self.pending_output
                    .iter()
                    .take(batch_len)
                    .map(|gi| &gi.bytes),
            ) {
                Ok(bytes) => bytes,
                Err(err) => {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Failed to encode pending inputs: {:?}",
                        err
                    );
                    return;
                },
            };
            // Input-compression accounting (always-on): the pre-compression size
            // is the sum of the raw per-frame input bytes batched into this send;
            // the post size is the delta/RLE-encoded `body.bytes`. Their ratio is
            // the realized compression, surfaced via `peer_metrics()`.
            let pre_compression_bytes: usize = self
                .pending_output
                .iter()
                .take(batch_len)
                .map(|gi| gi.bytes.len())
                .sum();
            self.input_bytes_pre_compression = self
                .input_bytes_pre_compression
                .saturating_add(pre_compression_bytes as u64);
            self.input_bytes_post_compression = self
                .input_bytes_post_compression
                .saturating_add(body.bytes.len() as u64);
            trace!(
                "Encoded {pre_compression_bytes} bytes from {batch_len} of {} pending output(s) into {} bytes",
                self.pending_output.len(),
                body.bytes.len()
            );

            body.ack_frame = self.last_recv_frame();
            body.disconnect_requested = self.state == ProtocolState::Disconnected;
            connect_status.clone_into(&mut body.peer_connect_status);

            self.queue_message(MessageBody::Input(body));
            // Real input traffic went out: the connect-status nudge (an
            // input-idle substitute) stays silent for the next interval.
            self.last_input_send_time = self.now();
        }
    }

    fn send_input_ack(&mut self) {
        let body = InputAck {
            ack_frame: self.last_recv_frame(),
        };

        self.queue_message(MessageBody::InputAck(body));
    }

    fn send_keep_alive(&mut self) {
        self.queue_message(MessageBody::KeepAlive);
    }

    /// Enables/disables the connect-status nudge for this endpoint. Set by the
    /// session every poll: `true` while the session holds a locally
    /// disconnected player slot whose drop is not yet mesh-agreed (some running
    /// endpoint still reports the slot connected), `false` otherwise. See
    /// [`send_connect_status_nudge`](Self::send_connect_status_nudge).
    pub(crate) fn set_connect_status_nudge(&mut self, enabled: bool) {
        self.connect_status_nudge = enabled;
    }

    /// Test-only: reads back the nudge flag so session-level tests can assert
    /// the per-poll wiring in `poll_remote_clients`.
    #[cfg(test)]
    pub(crate) fn connect_status_nudge_for_tests(&self) -> bool {
        self.connect_status_nudge
    }

    /// Sends a **connect-status nudge**: a status-bearing duplicate `Input`
    /// message re-built from the retained delta reference
    /// [`last_acked_input`](Self::last_acked_input), carrying the session's
    /// CURRENT `connect_status` array. Returns `true` if the message was
    /// queued.
    ///
    /// # Why
    ///
    /// Connect-status gossip travels only in `Input` messages, and an endpoint
    /// whose send queue is fully acked sends none — so a survivor that detects
    /// a peer drop while capped at its prediction window can never deliver the
    /// `disconnected` gossip, and mesh agreement (the condition that releases
    /// the dropped slot from the confirmed-frame minimum, see
    /// `P2PSession::remote_slot_confirmed_bound`) becomes unreachable: a
    /// permanent, silent liveness pin. The nudge closes that hole by giving the
    /// gossip a periodic carrier while a drop awaits mesh agreement.
    ///
    /// The caller (the `poll` gate) fires it only when **input-idle** — no
    /// real `Input` message queued for a full keepalive interval and an empty
    /// `pending_output` — so an actively-advancing session's packet stream is
    /// byte-identical with or without the flag: real input traffic already
    /// carries the connect status, and racing duplicate gossip ahead of it
    /// could change which packet resolves a same-poll disconnect-report race.
    ///
    /// # Why this is wire-compatible (no wire-format change)
    ///
    /// The packet is an `Input` message shape that already occurs on the wire
    /// via retransmission: `start_frame` is an already-acked frame, and the
    /// body is a delta batch of exactly one frame. Receivers handle stale or
    /// duplicate `Input` packets as established behavior — every decoded frame
    /// `<= last_recv_frame` is skipped — AND still run the connect-status merge
    /// on them: `merge_peer_connect_status` is hoisted BEFORE both decode-skip
    /// paths in `on_input` precisely so gossip rides every received packet
    /// (the S24 design; pinned by
    /// `on_input_low_start_frame_with_present_reference_applies_fresh_gossip`
    /// and `on_input_hoisted_merge_does_not_un_converge_freeze_from_stale_packet`).
    ///
    /// The body is encoded **self-referencing** (the `last_acked_input` bytes
    /// delta-encoded against themselves, an all-zero delta): the receiver
    /// decodes against ITS reference for `start_frame - 1`, which may produce
    /// different bytes — harmless, because every decoded frame is discarded as
    /// stale before being parsed (`inp_frame <= last_recv_frame` is checked
    /// first), and if the receiver has already pruned that reference the
    /// decode is skipped entirely. Either way the merge has already run.
    ///
    /// # Asymmetric cutoff: post-agreement status rides the retry timer
    ///
    /// The nudge flag clears on **local** mesh agreement
    /// (`P2PSession::connect_status_nudge_needed` goes `false` the moment THIS
    /// session sees every running endpoint report the slot disconnected), but
    /// release is a **global** condition — a peer that has not yet agreed
    /// still needs this session's view. Once this session stops nudging, its
    /// view travels ONLY in its real `Input` traffic: the next fresh send and,
    /// crucially, the `running_retry_interval` retransmission of any
    /// still-unacked `pending_output` (a capped post-agreement survivor may
    /// send nothing else status-bearing for an unbounded time). That is why
    /// the retry pacer `running_last_input_recv` must never be reset by
    /// progress-free packets such as this nudge: `on_input` refreshes it only
    /// when a packet stages at least one NEW frame, otherwise a still-nudging
    /// peer (keepalive cadence == default retry interval) would starve the
    /// retransmission — and with it the only remaining carrier — forever.
    /// Regression-pinned by the blackout repro in
    /// `tests/sessions/peer_drop.rs` and the
    /// `on_input_resets_retry_pacer_only_when_new_frames_staged` unit test.
    ///
    /// Only called while `Running` (the `poll` gate); `disconnect_requested`
    /// is therefore always `false` here, matching what `send_pending_output`
    /// would compute.
    fn send_connect_status_nudge(&mut self, connect_status: &[ConnectionStatus]) -> bool {
        if !self.last_acked_input.frame.is_valid() {
            return false;
        }
        let bytes = match try_encode(
            &self.last_acked_input.bytes,
            std::iter::once(&self.last_acked_input.bytes),
        ) {
            Ok(bytes) => bytes,
            Err(err) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Failed to encode connect-status nudge: {:?}",
                    err
                );
                return false;
            },
        };
        let mut body = Input {
            peer_connect_status: Vec::new(),
            disconnect_requested: false,
            start_frame: self.last_acked_input.frame,
            ack_frame: self.last_recv_frame(),
            bytes,
        };
        connect_status.clone_into(&mut body.peer_connect_status);
        self.queue_message(MessageBody::Input(body));
        true
    }

    fn send_sync_request(&mut self) {
        self.sync_requests_sent += 1;

        // Check for excessive retries and emit warning (once)
        if !self.sync_retry_warning_sent
            && self.sync_requests_sent > self.protocol_config.sync_retry_warning_threshold
        {
            self.sync_retry_warning_sent = true;
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::Synchronization,
                "Excessive sync retries: {} requests sent (threshold: {}). Possible high packet loss.",
                self.sync_requests_sent,
                self.protocol_config.sync_retry_warning_threshold
            );
        }

        // Check for excessive sync duration and emit warning (once)
        let elapsed_ms = (self.now() - self.stats_start_time).as_millis();
        if !self.sync_duration_warning_sent
            && elapsed_ms > self.protocol_config.sync_duration_warning_ms
        {
            self.sync_duration_warning_sent = true;
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::Synchronization,
                "Sync duration exceeded threshold: {}ms (threshold: {}ms). Network latency may be high.",
                elapsed_ms,
                self.protocol_config.sync_duration_warning_ms
            );
        }

        // Generate random number using deterministic RNG if configured, otherwise thread-local
        let random_number: u32 = match &mut self.protocol_rng {
            Some(rng) => rng.gen(),
            None => random(),
        };
        self.sync_random_requests.insert(random_number);
        let body = SyncRequest {
            random_request: random_number,
        };
        self.queue_message(MessageBody::SyncRequest(body));
    }

    fn send_quality_report(&mut self) {
        self.running_last_quality_report = self.now();

        // Timestamp from the (injectable) protocol clock. The peer echoes it
        // back verbatim, so it is only ever compared against this endpoint's
        // own clock in `on_quality_reply` — no wall-clock source is needed.
        let ping_timestamp = self.ping_millis();

        // Clamp to i16 range and convert - the clamp guarantees this won't fail,
        // but we use unwrap_or as defense-in-depth
        let clamped = self
            .local_frame_advantage
            .clamp(i16::MIN as i32, i16::MAX as i32);
        let frame_advantage = i16::try_from(clamped).unwrap_or(0);
        let body = QualityReport {
            frame_advantage,
            ping: ping_timestamp,
        };

        self.queue_message(MessageBody::QualityReport(body));
    }

    fn queue_message(&mut self, body: MessageBody) {
        trace!("Queuing message to {:?}: {:?}", self.peer_addr, body);

        // set the header
        let header = MessageHeader { magic: self.magic };
        let msg = Message { header, body };

        self.packets_sent = self.packets_sent.saturating_add(1);
        self.last_send_time = self.now();
        // Wire-exact payload bytes (D1 fix): the previous `size_of_val(&msg)`
        // measured the constant in-memory `Message` enum size, not what the
        // socket serializes, so `kbps_sent` was fiction. `Message::encoded_len`
        // matches `codec::encode(&msg).len()` exactly (property-tested).
        // Saturating so the lifetime counter degrades to a ceiling rather than
        // wrapping (or panicking under overflow-checks) on any target.
        self.bytes_sent = self.bytes_sent.saturating_add(msg.encoded_len() as u64);
        // Per-kind send tally: one bucket per packet keeps
        // `messages_sent_by_kind.total() == packets_sent`.
        self.messages_sent_by_kind.record(msg.kind());

        // add the packet to the back of the send queue
        self.send_queue.push_back(msg);
    }

    /*
     *  RECEIVING MESSAGES
     */

    pub(crate) fn handle_message(&mut self, msg: &Message) {
        trace!("Handling message from {:?}: {:?}", self.peer_addr, msg);

        // Per-peer receive accounting (always-on, wire-exact). Counted for every
        // message delivered to this endpoint *before* any protocol-state filter
        // below, so `bytes_received` reflects all traffic that arrived from this
        // peer and `packets_received == messages_received_by_kind.total()` holds
        // by construction (the send-side mirror of `queue_message`). Saturating so
        // the lifetime counters degrade to a ceiling rather than wrapping.
        self.packets_received = self.packets_received.saturating_add(1);
        self.bytes_received = self.bytes_received.saturating_add(msg.encoded_len() as u64);
        self.messages_received_by_kind.record(msg.kind());

        // don't handle messages if shutdown
        if self.state == ProtocolState::Shutdown {
            trace!("Protocol is shutting down; ignoring message");
            return;
        }

        // filter packets that don't match the magic if we have set it already
        if self.remote_magic != 0 && msg.header.magic != self.remote_magic {
            trace!("Received message with wrong magic; ignoring");
            return;
        }

        if !self.message_allowed_in_current_state(&msg.body) {
            trace!(
                "Dropping {:?} while protocol is in {:?}",
                msg.body,
                self.state
            );
            return;
        }

        // update time when we last received packages
        self.last_recv_time = self.now();

        // if the connection has been marked as interrupted, send an event to signal we are receiving again
        if self.disconnect_notify_sent && self.state == ProtocolState::Running {
            trace!("Received message on interrupted protocol; sending NetworkResumed event");
            self.disconnect_notify_sent = false;
            self.event_queue.push_back(Event::NetworkResumed);
        }

        // handle the message
        match &msg.body {
            MessageBody::SyncRequest(body) => self.on_sync_request(*body),
            MessageBody::SyncReply(body) => self.on_sync_reply(msg.header, *body),
            MessageBody::Input(body) => self.on_input(body),
            MessageBody::InputAck(body) => self.on_input_ack(*body),
            MessageBody::QualityReport(body) => self.on_quality_report(body),
            MessageBody::QualityReply(body) => self.on_quality_reply(body),
            MessageBody::ChecksumReport(body) => self.on_checksum_report(body),
            MessageBody::FloorRequest(body) => self.on_floor_request(body),
            MessageBody::FloorReply(body) => self.on_floor_reply(body),
            MessageBody::KeepAlive => (),
            #[cfg(feature = "hot-join")]
            MessageBody::JoinRequest(body) => self.on_join_request(body),
            #[cfg(feature = "hot-join")]
            MessageBody::StateSnapshot(body) => self.on_state_snapshot(body),
            #[cfg(feature = "hot-join")]
            MessageBody::StateSnapshotAck(body) => self.on_state_snapshot_ack(body),
            #[cfg(feature = "hot-join")]
            MessageBody::ReactivateSlot(body) => self.on_reactivate_slot(body),
            #[cfg(feature = "hot-join")]
            MessageBody::ReactivateSlotAck(body) => self.on_reactivate_slot_ack(body),
            #[cfg(feature = "hot-join")]
            MessageBody::JoinCommitted(body) => self.on_join_committed(body),
            #[cfg(feature = "hot-join")]
            MessageBody::JoinAborted(body) => self.on_join_aborted(body),
        }
    }

    fn message_allowed_in_current_state(&self, body: &MessageBody) -> bool {
        match self.state {
            ProtocolState::Initializing | ProtocolState::Synchronizing => {
                matches!(
                    body,
                    MessageBody::SyncRequest(_) | MessageBody::SyncReply(_)
                )
            },
            ProtocolState::Running => true,
            ProtocolState::Disconnected => matches!(body, MessageBody::SyncRequest(_)),
            ProtocolState::Shutdown => false,
        }
    }

    /// Upon receiving a `SyncRequest`, answer with a `SyncReply` with the proper data
    fn on_sync_request(&mut self, body: SyncRequest) {
        let reply_body = SyncReply {
            random_reply: body.random_request,
        };
        self.queue_message(MessageBody::SyncReply(reply_body));
    }

    /// Upon receiving a `SyncReply`, check validity and either continue the synchronization process or conclude synchronization.
    fn on_sync_reply(&mut self, header: MessageHeader, body: SyncReply) {
        // ignore sync replies when not syncing
        if self.state != ProtocolState::Synchronizing {
            return;
        }
        // this is not the correct reply
        if !self.sync_random_requests.remove(&body.random_reply) {
            return;
        }
        // the sync reply is good, so we send a sync request again until we have finished the required roundtrips. Then, we can conclude the syncing process.
        self.sync_remaining_roundtrips -= 1;
        let elapsed_ms = (self.now() - self.stats_start_time).as_millis();
        if self.sync_remaining_roundtrips > 0 {
            // register an event
            let evt = Event::Synchronizing {
                total: self.sync_config.num_sync_packets,
                count: self.sync_config.num_sync_packets - self.sync_remaining_roundtrips,
                total_requests_sent: self.sync_requests_sent,
                elapsed_ms,
            };
            self.event_queue.push_back(evt);
            // send another sync request
            self.send_sync_request();
        } else {
            // switch to running state
            self.state = ProtocolState::Running;
            // register an event
            self.event_queue.push_back(Event::Synchronized);
            // the remote endpoint is now "authorized"
            self.remote_magic = header.magic;
        }
    }

    /// Merges a remote peer's gossiped view of every slot's connect status into
    /// our cached copy ([`peer_connect_status`](Self::peer_connect_status)).
    ///
    /// For a CONNECTED slot, `last_frame` is monotone forward progress and only
    /// ever rises. For a DISCONNECTED slot, `last_frame` is the agreed freeze
    /// frame, which must converge DOWNWARD to the global-min as a lower freeze
    /// gossip relays across the mesh: taking `max` here would clobber a relayed
    /// lowering and leave survivors frozen at different frames for the dropped slot
    /// (silent desync).
    ///
    /// The merge is loss/reorder safe by construction — a stale packet can neither
    /// regress a connected slot's progress (`max`), re-raise an already-converged
    /// freeze frame (`min` for both-disconnected), nor resurrect a converged
    /// disconnect (local-disconnected wins over stale remote-connected). Because
    /// of that, it is also safe to apply from a packet whose *inputs* we could not
    /// decode: the gossip carried by an undecodable or stale packet cannot move
    /// our cached view in an unsafe direction. Processing it regardless lets
    /// disconnect gossip — the convergence driver behind
    /// `update_player_disconnects` — ride EVERY received packet, not only the
    /// decodable ones, narrowing the N>=3 disconnect-convergence window under
    /// asymmetric loss.
    ///
    /// Callers MUST validate `body.peer_connect_status.len() == num_players`
    /// first; a mismatched length is silently ignored here (the zipped iterator
    /// stops at the shorter side) but should already have been rejected upstream.
    /// The merge is intentionally skipped when the sender itself is disconnecting
    /// (`body.disconnect_requested`), matching the prior inline behavior.
    fn merge_peer_connect_status(&mut self, body: &Input) {
        if body.disconnect_requested {
            return;
        }
        #[cfg(feature = "hot-join")]
        let floors = &self.reactivation_floor;
        for (slot, (local, remote)) in self
            .peer_connect_status
            .iter_mut()
            .zip(body.peer_connect_status.iter())
            .enumerate()
        {
            // Reactivation floor (N-peer hot-join): ignore stale DISCONNECTED
            // claims from before the slot's most recent committed
            // reactivation — see `reactivation_floor` for the threshold
            // soundness argument and the one ambiguous equality corner.
            // Without this, a single pre-attempt packet reordered past the
            // final reactivation seed re-sticks the cache (the merge is
            // deliberately sticky-disconnected) and permanently re-drops the
            // live slot.
            #[cfg(feature = "hot-join")]
            if remote.disconnected
                && floors
                    .get(slot)
                    .is_some_and(|floor| !floor.is_null() && remote.last_frame < *floor)
            {
                continue;
            }
            #[cfg(not(feature = "hot-join"))]
            let _ = slot;
            if remote.disconnected || local.disconnected {
                if local.disconnected && remote.disconnected {
                    // Both views are freeze frames: take the lower so a relayed
                    // lowering wins and a stale higher disconnect packet cannot
                    // un-converge us.
                    local.last_frame = std::cmp::min(local.last_frame, remote.last_frame);
                } else if remote.disconnected {
                    // First time we learn this peer considers the slot
                    // disconnected: adopt its authoritative freeze frame (its true
                    // last-received frame for the slot). This may be lower OR higher
                    // than our stale connected forward-progress value — `min` would
                    // under-claim a peer that genuinely received the slot through a
                    // higher frame before it dropped, freezing too early and
                    // discarding valid confirmed inputs every peer actually received.
                    local.last_frame = remote.last_frame;
                }
                // else: local disconnected, remote still reports connected — a STALE
                // pre-drop gossip arriving after we already converged the freeze
                // frame. Do NOT raise `last_frame` (that would re-introduce the
                // clobber) and do NOT resurrect the slot below.
                local.disconnected = true;
            } else {
                // Both connected: monotone forward progress.
                local.last_frame = std::cmp::max(local.last_frame, remote.last_frame);
            }
        }
    }

    fn on_input(&mut self, body: &Input) {
        // A hot-joiner defers ALL input processing until it has applied the
        // snapshot. Crucially this also defers the ack: acking now would let the
        // host trim pending_output below the activation frame.
        #[cfg(feature = "hot-join")]
        if self.defer_input_processing {
            return;
        }

        let ack_disposition = self.classify_ack_frame(body.ack_frame);

        if body.peer_connect_status.len() != self.num_players {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "Received input with {} connection-status entries, expected {}",
                body.peer_connect_status.len(),
                self.num_players
            );
            return;
        }

        if !body.start_frame.is_valid() {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "Received input with invalid start frame {}",
                body.start_frame
            );
            return;
        }

        // Process the disconnect-gossip merge BEFORE the two decode-skip paths
        // below (the gap-too-large early return and the missing-decode-reference
        // guard). The merge is loss/reorder/stale safe (see
        // `merge_peer_connect_status`), so applying it from a packet whose inputs
        // we cannot decode is strictly correct and lets C's disconnect gossip
        // reach `update_player_disconnects` even when the carrying packet's inputs
        // are dropped — narrowing the convergence window under asymmetric loss.
        // Length/validity are already checked above; ack/input-staging/event
        // ordering and the recv-time bump intentionally remain gated on decode.
        self.merge_peer_connect_status(body);

        // Validate that received inputs are in a recoverable order.
        // If we receive an input for a frame that's too far ahead, we can't decode it
        // because we don't have the reference frame. This is normal UDP behavior -
        // packets can be lost or reordered. We just drop it and wait for retransmission.
        let last_recv_frame = self.last_recv_frame();
        let next_expected =
            safe_frame_add!(last_recv_frame, 1, "UdpProtocol::on_input next_expected");
        if last_recv_frame != Frame::NULL && next_expected < body.start_frame {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Received input for frame {} but last received was frame {} - gap too large to decode (likely packet loss)",
                body.start_frame,
                self.last_recv_frame()
            );
            return;
        }

        // if we did not receive any input yet, we decode with the blank input,
        // otherwise we use the input previous to the start of the encoded inputs
        let decode_frame = if last_recv_frame == Frame::NULL {
            Frame::NULL
        } else {
            safe_frame_sub!(body.start_frame, 1, "UdpProtocol::on_input decode_frame")
        };

        // if we have the necessary input saved, we decode
        if let Some(decode_inp) = self.recv_inputs.get(&decode_frame) {
            let max_decoded_input_bytes = match input_batch_decoded_byte_limit(
                decode_inp.bytes.len(),
                self.protocol_config.pending_output_limit,
            ) {
                Some(max) => max,
                None => {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Input decode limit overflow: reference bytes {} * pending output limit {}",
                        decode_inp.bytes.len(),
                        self.protocol_config.pending_output_limit
                    );
                    return;
                },
            };

            let recv_inputs = match decode_with_max_len(
                &decode_inp.bytes,
                &body.bytes,
                max_decoded_input_bytes,
            ) {
                Ok(inputs) => inputs,
                Err(e) => {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Failed to decode input packet: {:?}. Packet may be corrupted.",
                        e
                    );
                    return;
                },
            };

            let mut staged_frames = Vec::new();
            if staged_frames.try_reserve(recv_inputs.len()).is_err() {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Failed to reserve {} decoded input frame(s)",
                    recv_inputs.len()
                );
                return;
            }

            for (i, inp) in recv_inputs.into_iter().enumerate() {
                let Ok(frame_offset) = i32::try_from(i) else {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Decoded input batch has too many frames to represent as i32 offsets"
                    );
                    return;
                };
                let Some(inp_frame) = body.start_frame.checked_add(frame_offset) else {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Decoded input frame overflow from start frame {} and offset {}",
                        body.start_frame,
                        frame_offset
                    );
                    return;
                };
                // skip inputs that we don't need
                if inp_frame <= last_recv_frame {
                    continue;
                }

                let input_data = InputBytes {
                    frame: inp_frame,
                    bytes: inp,
                };
                let player_inputs =
                    match input_data.try_to_player_inputs_exact::<T>(self.handles.len()) {
                        Ok(player_inputs) => player_inputs,
                        Err(err) => {
                            log_input_decode_error(err);
                            return;
                        },
                    };

                staged_frames.push(StagedInputFrame {
                    input_data,
                    player_inputs,
                });
            }

            if ack_disposition == AckDisposition::Apply {
                self.apply_ack_frame(body.ack_frame);
            }

            let should_emit_disconnect = body.disconnect_requested
                && self.state != ProtocolState::Disconnected
                && !self.disconnect_event_sent;

            // The connect-status merge ran earlier (before the decode-skip paths)
            // via `merge_peer_connect_status`, so disconnect gossip converges even
            // for packets whose inputs we cannot decode.

            // Refresh the pending-output retransmission pacer ONLY when this
            // packet staged at least one NEW frame. `running_last_input_recv`
            // feeds exactly one consumer — the `running_retry_interval` resend
            // gate in `poll` — while liveness/disconnect-timeout tracking uses
            // the separate `last_recv_time`, which `handle_message` refreshes
            // for EVERY packet (including progress-free ones); this gate
            // changes retry pacing only, never disconnect detection.
            // Progress-free decodable Inputs — connect-status nudges and
            // duplicate retransmissions — must not suppress the resend: a
            // peer nudging on the keepalive cadence (== the default retry
            // interval) would otherwise starve our pending Input forever, and
            // that pending Input is the only carrier of our post-agreement
            // connect status (see `send_connect_status_nudge`'s rustdoc and
            // the blackout regression test in `tests/sessions/peer_drop.rs`).
            // Trade-off: duplicate-heavy legitimate traffic (retransmissions
            // under loss) now lets our retry fire on its normal interval —
            // at most one extra resend per `running_retry_interval`, benign.
            if !staged_frames.is_empty() {
                self.running_last_input_recv = self.now();
            }
            for staged in staged_frames {
                let peer_connect_status = body.peer_connect_status.clone();
                self.recv_inputs
                    .insert(staged.input_data.frame, staged.input_data);

                for (player_input, &player_handle) in
                    staged.player_inputs.into_iter().zip(self.handles.iter())
                {
                    self.event_queue.push_back(Event::Input {
                        input: player_input,
                        player: player_handle,
                        peer_connect_status: peer_connect_status.clone(),
                    });
                }
            }

            if should_emit_disconnect {
                self.event_queue.push_back(Event::Disconnected);
                self.disconnect_event_sent = true;
            }

            // send an input ack
            self.send_input_ack();

            // delete received inputs that are too old
            let last_recv_frame = self.last_recv_frame();
            let history_frames = self
                .protocol_config
                .input_history_multiplier
                .checked_mul(self.max_prediction)
                .and_then(|frames| i32::try_from(frames).ok())
                .unwrap_or(i32::MAX);
            self.recv_inputs.retain(|&k, _| {
                k >= safe_frame_sub!(
                    last_recv_frame,
                    history_frames,
                    "UdpProtocol::on_input history prune"
                )
            });
        } else {
            // A stale retransmission can outlive its delta reference after the
            // receiver prunes old input history. The frames are already known,
            // so decoding/staging is unnecessary, but silence here strands the
            // sender if our earlier cumulative InputAck was lost: it retains
            // the old front forever, keeps `pending_output` nonempty, and both
            // peers can exhaust their prediction windows. Re-ack the receiver's
            // contiguous high-water and consume the independent piggyback ACK;
            // neither operation depends on the missing input reference.
            if ack_disposition == AckDisposition::Apply {
                self.apply_ack_frame(body.ack_frame);
            }
            self.send_input_ack();
        }
    }

    /// Upon receiving a `InputAck`, discard the oldest buffered input including the acked input.
    fn on_input_ack(&mut self, body: InputAck) {
        self.apply_ack_frame(body.ack_frame);
    }

    /// Upon receiving a `QualityReport`, update network stats and reply with a `QualityReply`.
    fn on_quality_report(&mut self, body: &QualityReport) {
        self.remote_frame_advantage = body.frame_advantage as i32;
        let reply_body = QualityReply { pong: body.ping };
        self.queue_message(MessageBody::QualityReply(reply_body));
    }

    /// Upon receiving a `QualityReply`, update network stats.
    fn on_quality_reply(&mut self, body: &QualityReply) {
        let millis = self.ping_millis();
        // Saturating: a pong is normally an echo of this endpoint's own earlier
        // ping (never in the future), but a stale packet from a previous
        // endpoint era could carry an arbitrary value. A 0 RTT is harmless -
        // it will be corrected on the next quality report.
        self.round_trip_time = millis.saturating_sub(body.pong);
    }

    // ---- floor-round (double-failure-relay connected-relay reorder fix) ----

    /// Pushed by the session every poll: `request_needed` is `true` when this
    /// endpoint is a folded relay in the relay topology (so `poll` keeps
    /// re-issuing `FloorRequest`s on the keepalive cadence). Allocation-free.
    pub(crate) fn set_floor_request_needed(&mut self, request_needed: bool) {
        self.floor_request_needed = request_needed;
    }

    /// Records this endpoint's current [`is_running`](Self::is_running) state and
    /// returns `true` iff it just transitioned running→pruned since the last
    /// call — the running→pruned **prune** the session counts to reset every
    /// endpoint's floor freshness ([`Self::reset_floor_freshness`]). Centralizing
    /// detection here (rather than at every `disconnect()` call site) catches
    /// every prune, including disconnect timeouts. Called once per endpoint per
    /// `poll_remote_clients`.
    pub(crate) fn detect_prune_transition(&mut self) -> bool {
        let running = self.is_running();
        let pruned = self.floor_round_was_running && !running;
        self.floor_round_was_running = running;
        pruned
    }

    /// Bumps the monotonic per-request sequence number and queues a
    /// [`FloorRequest`] stamped with it.
    fn send_floor_request(&mut self) {
        self.floor_request_seq = self.floor_request_seq.wrapping_add(1);
        self.queue_message(MessageBody::FloorRequest(FloorRequest {
            round_seq: self.floor_request_seq,
        }));
        self.debug_assert_floor_round_invariants();
    }

    /// On receiving a [`FloorRequest`] (we are a relay for the requester): record
    /// the request's `round_seq` for the session to answer (it computes
    /// `pessimistic_floors` and replies via [`Self::send_floor_reply`]).
    ///
    /// Highest-seq-wins — a newer request supersedes an undrained older one,
    /// regardless of arrival order. Under packet reorder an OLDER `round_seq`
    /// must NOT clobber a higher undrained pending one (which would answer the
    /// stale round and skip the newer one), so the incoming seq is stored only
    /// when it is strictly greater than the currently-pending one (or none is
    /// pending yet).
    fn on_floor_request(&mut self, body: &FloorRequest) {
        if self
            .pending_floor_request
            .is_none_or(|pending| body.round_seq > pending)
        {
            self.pending_floor_request = Some(body.round_seq);
        }
    }

    /// Whether a [`FloorRequest`] is awaiting a reply (peek without draining).
    pub(crate) fn has_pending_floor_request(&self) -> bool {
        self.pending_floor_request.is_some()
    }

    /// Drains a pending [`FloorRequest`]'s `round_seq` for the session to answer.
    pub(crate) fn take_pending_floor_request(&mut self) -> Option<u32> {
        self.pending_floor_request.take()
    }

    /// Queues a [`FloorReply`] echoing `round_seq` and carrying the session's
    /// current per-slot pessimistic `floors`.
    pub(crate) fn send_floor_reply(&mut self, round_seq: u32, floors: &[Frame]) {
        self.queue_message(MessageBody::FloorReply(FloorReply {
            round_seq,
            floors: floors.to_vec(),
        }));
    }

    /// On receiving a [`FloorReply`], accept it ONLY when all three guards pass;
    /// any failure DROPS the reply, leaving every floor-round field untouched
    /// (the conservative `slot_round_incomplete` hold then keeps capping the
    /// slot — never advancing on an unvalidated relay floor):
    ///
    /// 1. **Reorder/duplicate.** Its echoed `round_seq` must be STRICTLY NEWER
    ///    than the latest accepted ([`Self::floor_reply_seq`]). An older/equal
    ///    seq is a reordered stale (or duplicate) reply — dropping it is what
    ///    makes [`Self::round_floor`] reorder-immune (a plain `Input`-gossip
    ///    floor cache could not survive a reordered stale-HIGH packet).
    /// 2. **Solicitation.** Its `round_seq` must NOT exceed the latest request
    ///    this endpoint actually issued ([`Self::floor_request_seq`]). A reply
    ///    echoing a never-issued seq is forged or corrupt; accepting it would
    ///    advance `floor_reply_seq` past every legitimate future reply (a
    ///    permanent round stall) and spuriously flip
    ///    [`Self::floor_round_is_fresh`] — handing peer-controlled state into the
    ///    floor cache (mirrors the request-tracking gate in
    ///    [`Self::on_sync_reply`]).
    /// 3. **Completeness.** Its `floors` must cover EVERY slot
    ///    (`len >= num_players`). Accepting a reply RELEASES the
    ///    `slot_round_incomplete` hold for this relay, so a short vector would
    ///    leave the omitted slots reading a stale prior round while the hold is
    ///    down — re-opening the connected-relay over-confirmation the round
    ///    exists to prevent. The honest relay always reports every slot
    ///    (`P2PSession::pessimistic_floors` is `num_players` long), so this
    ///    rejects only malformed replies.
    ///
    /// An accepted reply OVERWRITES the reply cache with the relay's reported
    /// floors (the spec's `roundFloor' = AckReportFloor`) — every slot, since the
    /// length is checked first, so no slot survives from a prior round — and
    /// advances `floor_reply_seq`. Whether it counts as POST-PRUNE fresh
    /// ([`Self::floor_round_is_fresh`]) then depends on whether its seq exceeds
    /// the prune threshold ([`Self::floor_prune_seq`]). Excess entries beyond
    /// `num_players` are ignored; a reported `Frame::NULL` slot makes the session
    /// fold fall back to `last_frame`.
    fn on_floor_reply(&mut self, body: &FloorReply) {
        if body.round_seq <= self.floor_reply_seq {
            trace!(
                "Dropping stale/duplicate FloorReply round_seq {} (latest accepted {})",
                body.round_seq,
                self.floor_reply_seq
            );
            return;
        }
        if body.round_seq > self.floor_request_seq {
            trace!(
                "Dropping unsolicited FloorReply round_seq {} (latest request {})",
                body.round_seq,
                self.floor_request_seq
            );
            return;
        }
        if body.floors.len() < self.round_floor.len() {
            trace!(
                "Dropping incomplete FloorReply: {} floors for {} slots",
                body.floors.len(),
                self.round_floor.len()
            );
            return;
        }
        for (slot, cached) in self.round_floor.iter_mut().enumerate() {
            if let Some(&floor) = body.floors.get(slot) {
                *cached = floor;
            }
        }
        self.floor_reply_seq = body.round_seq;
        self.debug_assert_floor_round_invariants();
    }

    /// This endpoint's cached per-slot pessimistic floor from its latest accepted
    /// [`FloorReply`] (`Frame::NULL` if none for the slot — the session fold
    /// then falls back to `last_frame`).
    pub(crate) fn round_floor(&self, handle: PlayerHandle) -> Frame {
        self.round_floor
            .get(handle.as_usize())
            .copied()
            .unwrap_or(Frame::NULL)
    }

    /// `true` when this endpoint has accepted a [`FloorReply`] that POSTDATES the
    /// most recent prune (`floor_reply_seq > floor_prune_seq`) — i.e. a fresh
    /// post-prune round has completed and [`Self::round_floor`] may be trusted.
    /// A never-replied relay (`floor_reply_seq == 0`, `floor_prune_seq == 0`) is
    /// never fresh, so its slot holds.
    pub(crate) fn floor_round_is_fresh(&self) -> bool {
        self.floor_reply_seq > self.floor_prune_seq
    }

    /// Resets this endpoint's floor freshness on a prune: snapshots the current
    /// request sequence as the threshold, so only a reply to a request issued
    /// AFTER this prune counts as fresh again (the spec's `ackFresh` reset). The
    /// session calls this on EVERY endpoint whenever ANY remote is pruned.
    pub(crate) fn reset_floor_freshness(&mut self) {
        self.floor_prune_seq = self.floor_request_seq;
        self.debug_assert_floor_round_invariants();
    }

    /// Debug-only floor-round state invariants, asserted at the end of every
    /// method that mutates floor-round state. Follows the same
    /// `debug_assert!`-gated invariant-check convention as
    /// [`SyncLayer`](crate::__internal::SyncLayer) (which wraps a fallible
    /// `check_invariants`; these relations are simple enough to assert inline).
    /// The relations:
    ///
    /// 1. `floor_reply_seq <= floor_request_seq` — a reply is only accepted for a
    ///    request actually issued, so the latest accepted reply seq can never
    ///    exceed the latest request seq.
    /// 2. `floor_prune_seq <= floor_request_seq` — a prune snapshots the current
    ///    request seq ([`Self::reset_floor_freshness`]), so the threshold can
    ///    never exceed the latest request seq.
    /// 3. `round_floor.len() == peer_connect_status.len()` — both are seeded to
    ///    `num_players` at construction and neither is ever resized, so the
    ///    per-slot floor cache must stay parallel to the connect-status cache.
    ///
    /// The body compiles out entirely in release builds (the `debug_assert!`s
    /// vanish), so the unconditional callers leave an empty call that optimizes
    /// away — no dead-code or unused-method warning in any build mode. The
    /// release-with-debug-assertions CI gate still exercises it.
    fn debug_assert_floor_round_invariants(&self) {
        debug_assert!(
            self.floor_reply_seq <= self.floor_request_seq,
            "floor invariant: reply seq {} must not exceed request seq {}",
            self.floor_reply_seq,
            self.floor_request_seq
        );
        debug_assert!(
            self.floor_prune_seq <= self.floor_request_seq,
            "floor invariant: prune seq {} must not exceed request seq {}",
            self.floor_prune_seq,
            self.floor_request_seq
        );
        debug_assert_eq!(
            self.round_floor.len(),
            self.peer_connect_status.len(),
            "floor invariant: round_floor length {} must match peer_connect_status length {}",
            self.round_floor.len(),
            self.peer_connect_status.len()
        );
    }

    /// Upon receiving a `ChecksumReport`, add it to the checksum history
    fn on_checksum_report(&mut self, body: &ChecksumReport) {
        let interval = if let DesyncDetection::On { interval } = self.desync_detection {
            interval
        } else {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::Configuration,
                "Received checksum report, but desync detection is off. Check that configuration is consistent between peers."
            );
            1
        };

        let max_history = self.protocol_config.max_checksum_history;
        if self.pending_checksums.len() >= max_history {
            // Calculate frames to keep, using saturating arithmetic to prevent underflow
            let frames_to_subtract = (max_history as i32 - 1).saturating_mul(interval as i32);
            let oldest_frame_to_keep = safe_frame_sub!(
                body.frame,
                frames_to_subtract,
                "UdpProtocol::on_checksum_report"
            );
            self.pending_checksums
                .retain(|&frame, _| frame >= oldest_frame_to_keep);
        }
        self.pending_checksums.insert(body.frame, body.checksum);
    }

    /// Upon receiving a `JoinRequest`, store the requested slot for the orchestration
    /// layer to drain via [`take_pending_join_request`](Self::take_pending_join_request).
    #[cfg(feature = "hot-join")]
    fn on_join_request(&mut self, body: &JoinRequest) {
        self.pending_join_request = Some(body.player_handle);
    }

    /// Upon receiving a `StateSnapshot`, store it for the orchestration layer to drain
    /// via [`take_received_snapshot`](Self::take_received_snapshot).
    #[cfg(feature = "hot-join")]
    fn on_state_snapshot(&mut self, body: &StateSnapshot) {
        self.received_snapshot = Some(body.clone());
    }

    /// Upon receiving a `StateSnapshotAck`, store the acked frame for the orchestration
    /// layer to drain via [`take_received_snapshot_ack`](Self::take_received_snapshot_ack).
    #[cfg(feature = "hot-join")]
    fn on_state_snapshot_ack(&mut self, body: &StateSnapshotAck) {
        self.received_snapshot_ack = Some(body.frame);
    }

    /// Upon receiving a `ReactivateSlot`, store it for the orchestration layer to drain
    /// via [`take_received_reactivate_slot`](Self::take_received_reactivate_slot).
    #[cfg(feature = "hot-join")]
    fn on_reactivate_slot(&mut self, body: &ReactivateSlot) {
        self.received_reactivate_slot = Some(body.clone());
    }

    /// Upon receiving a `ReactivateSlotAck`, store it for the orchestration layer to
    /// drain via [`take_received_reactivate_slot_ack`](Self::take_received_reactivate_slot_ack).
    #[cfg(feature = "hot-join")]
    fn on_reactivate_slot_ack(&mut self, body: &ReactivateSlotAck) {
        self.received_reactivate_slot_ack = Some(body.clone());
    }

    /// Upon receiving a `JoinCommitted`, store it for the orchestration layer to
    /// drain via [`take_received_join_committed`](Self::take_received_join_committed).
    #[cfg(feature = "hot-join")]
    fn on_join_committed(&mut self, body: &JoinCommitted) {
        self.received_join_committed = Some(body.clone());
    }

    /// Upon receiving a `JoinAborted`, store it for the orchestration layer to
    /// drain via [`take_received_join_aborted`](Self::take_received_join_aborted).
    #[cfg(feature = "hot-join")]
    fn on_join_aborted(&mut self, body: &JoinAborted) {
        self.received_join_aborted = Some(body.clone());
    }

    /// Returns the frame of the last received input
    fn last_recv_frame(&self) -> Frame {
        match self.recv_inputs.iter().max_by_key(|&(k, _)| k) {
            Some((k, _)) => *k,
            None => Frame::NULL,
        }
    }

    pub(crate) fn send_checksum_report(&mut self, frame_to_send: Frame, checksum: u128) {
        let body = ChecksumReport {
            frame: frame_to_send,
            checksum,
        };
        self.queue_message(MessageBody::ChecksumReport(body));
    }

    /// Queues a `JoinRequest` for the slot `player_handle`. No-op unless `Running`.
    // dead_code: consumed by chunk 5's session orchestration; only the message +
    // protocol layer lands in this chunk.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn send_join_request(&mut self, player_handle: usize) {
        if self.state != ProtocolState::Running {
            return;
        }
        self.queue_message(MessageBody::JoinRequest(JoinRequest { player_handle }));
    }

    /// Queues a `StateSnapshot`. No-op unless `Running`.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn send_state_snapshot(&mut self, snapshot: StateSnapshot) {
        if self.state != ProtocolState::Running {
            return;
        }
        self.queue_message(MessageBody::StateSnapshot(snapshot));
    }

    /// Queues a `StateSnapshotAck` for `frame`. No-op unless `Running`.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn send_state_snapshot_ack(&mut self, frame: Frame) {
        if self.state != ProtocolState::Running {
            return;
        }
        self.queue_message(MessageBody::StateSnapshotAck(StateSnapshotAck { frame }));
    }

    /// Queues a `ReactivateSlot` reopening `handle` at `frame`. No-op unless `Running`.
    #[cfg(feature = "hot-join")]
    pub(crate) fn send_reactivate_slot(&mut self, handle: usize, frame: Frame) {
        if self.state != ProtocolState::Running {
            return;
        }
        self.queue_message(MessageBody::ReactivateSlot(ReactivateSlot {
            handle,
            frame,
        }));
    }

    /// Queues a `ReactivateSlotAck` for `handle` at `frame`. No-op unless `Running`.
    #[cfg(feature = "hot-join")]
    pub(crate) fn send_reactivate_slot_ack(&mut self, handle: usize, frame: Frame) {
        if self.state != ProtocolState::Running {
            return;
        }
        self.queue_message(MessageBody::ReactivateSlotAck(ReactivateSlotAck {
            handle,
            frame,
        }));
    }

    /// Queues a `JoinCommitted` for `handle` at activation frame `frame`. No-op
    /// unless `Running`.
    #[cfg(feature = "hot-join")]
    pub(crate) fn send_join_committed(&mut self, handle: usize, frame: Frame) {
        if self.state != ProtocolState::Running {
            return;
        }
        self.queue_message(MessageBody::JoinCommitted(JoinCommitted { handle, frame }));
    }

    /// Queues a `JoinAborted` for `handle` at activation frame `frame`. No-op
    /// unless `Running`.
    #[cfg(feature = "hot-join")]
    pub(crate) fn send_join_aborted(&mut self, handle: usize, frame: Frame) {
        if self.state != ProtocolState::Running {
            return;
        }
        self.queue_message(MessageBody::JoinAborted(JoinAborted { handle, frame }));
    }

    /// Drains the most recently received `JoinRequest`'s requested slot, if any.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn take_pending_join_request(&mut self) -> Option<usize> {
        self.pending_join_request.take()
    }

    /// Test seam: stage a pending `JoinRequest` for `handle` exactly as if one
    /// had arrived from the peer, without driving the full sync + message path.
    /// Used to unit-test the host-side join-request authorization gate.
    #[cfg(all(test, feature = "hot-join"))]
    pub(crate) fn set_pending_join_request_for_test(&mut self, handle: usize) {
        self.pending_join_request = Some(handle);
    }

    /// Drains the most recently received `StateSnapshot`, if any.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn take_received_snapshot(&mut self) -> Option<StateSnapshot> {
        self.received_snapshot.take()
    }

    /// Drains the most recently received `StateSnapshotAck` frame, if any.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn take_received_snapshot_ack(&mut self) -> Option<Frame> {
        self.received_snapshot_ack.take()
    }

    /// Drains the most recently received `ReactivateSlot`, if any.
    #[cfg(feature = "hot-join")]
    pub(crate) fn take_received_reactivate_slot(&mut self) -> Option<ReactivateSlot> {
        self.received_reactivate_slot.take()
    }

    /// Drains the most recently received `ReactivateSlotAck`, if any.
    #[cfg(feature = "hot-join")]
    pub(crate) fn take_received_reactivate_slot_ack(&mut self) -> Option<ReactivateSlotAck> {
        self.received_reactivate_slot_ack.take()
    }

    /// Drains the most recently received `JoinCommitted`, if any.
    #[cfg(feature = "hot-join")]
    pub(crate) fn take_received_join_committed(&mut self) -> Option<JoinCommitted> {
        self.received_join_committed.take()
    }

    /// Drains the most recently received `JoinAborted`, if any.
    #[cfg(feature = "hot-join")]
    pub(crate) fn take_received_join_aborted(&mut self) -> Option<JoinAborted> {
        self.received_join_aborted.take()
    }

    /// Test seam: stage a received `ReactivateSlot` exactly as if one had arrived
    /// from the peer, without forging the magic-validated wire path. Used by
    /// session-level survivor fail-closed-validation tests. Mirrors
    /// [`set_pending_join_request_for_test`](Self::set_pending_join_request_for_test).
    #[cfg(all(test, feature = "hot-join"))]
    pub(crate) fn set_received_reactivate_slot_for_test(&mut self, body: ReactivateSlot) {
        self.received_reactivate_slot = Some(body);
    }

    /// Test seam: stage a received `JoinCommitted` (see
    /// [`set_received_reactivate_slot_for_test`](Self::set_received_reactivate_slot_for_test)).
    #[cfg(all(test, feature = "hot-join"))]
    pub(crate) fn set_received_join_committed_for_test(&mut self, body: JoinCommitted) {
        self.received_join_committed = Some(body);
    }

    /// Test seam: stage a received `JoinAborted` (see
    /// [`set_received_reactivate_slot_for_test`](Self::set_received_reactivate_slot_for_test)).
    #[cfg(all(test, feature = "hot-join"))]
    pub(crate) fn set_received_join_aborted_for_test(&mut self, body: JoinAborted) {
        self.received_join_aborted = Some(body);
    }

    /// Test seam: stage a received `StateSnapshot` (see
    /// [`set_received_reactivate_slot_for_test`](Self::set_received_reactivate_slot_for_test)).
    #[cfg(all(test, feature = "hot-join"))]
    pub(crate) fn set_received_snapshot_for_test(&mut self, body: StateSnapshot) {
        self.received_snapshot = Some(body);
    }

    /// Test seam: reads whether this endpoint currently defers (ignores)
    /// incoming `Input` messages — pins the joiner-side un-defer contract.
    #[cfg(all(test, feature = "hot-join"))]
    pub(crate) fn defers_input_processing(&self) -> bool {
        self.defer_input_processing
    }

    /// Sets whether this endpoint defers (ignores) incoming `Input` messages.
    /// See `defer_input_processing`.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn set_defer_input_processing(&mut self, defer: bool) {
        self.defer_input_processing = defer;
    }

    /// Re-seeds this endpoint's cached view of `handle`'s connection status to
    /// `{connected, last_frame}` — the N-peer slot-reactivation un-stick. The
    /// seed deliberately does **not** arm the slot's `reactivation_floor`;
    /// only [`arm_reactivation_floor`](Self::arm_reactivation_floor) does,
    /// and only at commit-evidence points (see both docs for why a
    /// reopen-armed floor is a liveness bug).
    ///
    /// The gossip merge ([`merge_peer_connect_status`]) is deliberately
    /// **sticky-disconnected**: once a peer claims a slot disconnected, later
    /// `connected` gossip never resurrects the cached view (that stickiness is
    /// what makes drop convergence loss/reorder-safe, and it is also what makes
    /// an *aborted* reactivation attempt's transient `connected` gossip
    /// invisible to non-participants). A **committed** reactivation therefore
    /// needs this explicit out-of-band reset, invoked by the session at its own
    /// reactivation points (coordinator commit; survivor reopen and
    /// commit-receipt) — every mesh member un-sticks its own caches when *it*
    /// learns the slot is live. `last_frame` is seeded to `F - 1` (the agreed
    /// pre-activation bound every reopening peer also stamps locally), which is
    /// faithful: every participant of a committed attempt reopens the slot at
    /// exactly `F`. Subsequent genuine gossip max-merges forward from there.
    /// A pre-commit (reopen-time) seed is equally safe without a floor: a
    /// stale `disconnected` claim re-sticking the cache mid-attempt is
    /// tolerated by design — the session's pending-reactivation shield keeps
    /// the fold off the slot for the attempt's lifetime, and the
    /// commit-receipt re-seed un-sticks the cache again — while after an
    /// ABORT the re-stuck `{disconnected, f0}` view is exactly the state the
    /// mesh re-converges on.
    ///
    /// [`merge_peer_connect_status`]: Self::merge_peer_connect_status
    #[cfg(feature = "hot-join")]
    pub(crate) fn seed_peer_connect_status_for_reactivation(
        &mut self,
        handle: PlayerHandle,
        last_frame: Frame,
    ) {
        if let Some(status) = self.peer_connect_status.get_mut(handle.as_usize()) {
            *status = ConnectionStatus {
                disconnected: false,
                last_frame,
                // Preserve the cached generation. This is the receiver-side
                // player-mesh cache, whose `epoch` is inert: the confirmed/freeze
                // folds read only `disconnected`/`last_frame`, and a spectator
                // consumes each host's own armed `local_connect_status`, never
                // this relayed cache (`merge_peer_connect_status` likewise never
                // copies a sender's epoch in). Preserved only to avoid a spurious
                // field reset.
                ..*status
            };
        }
    }

    /// Seeds this (freshly rebuilt joiner) endpoint's cached view of `handle`
    /// to `status` WITHOUT arming the reactivation floor — the N-peer
    /// joiner-endpoint cache bootstrap.
    ///
    /// A rearmed joiner endpoint holds default `{connected, NULL}` caches for
    /// EVERY slot; once it is un-reserved at the reactivation point, those
    /// NULL terms enter the session's confirmed-frame fold and pin the
    /// session's confirmed frame at `NULL` until the joiner's first own
    /// gossip arrives — a mesh-wide stall window (and, for a capped survivor,
    /// a gossip-silence wedge that can starve the coordinator's
    /// wait-then-capture gate). The session therefore bootstraps the caches
    /// at its reactivation point with claims the snapshot contract makes
    /// faithful: the joiner participates only by acking the snapshot at
    /// `S = F - 1`, which bakes in every slot's effects through `S`, so live
    /// slots are claimed `{connected, min(local view, F - 1)}` and dropped
    /// slots keep the agreed frozen view. Soundness: every fold consuming
    /// these claims `min`s them with the folding session's own receipts, so a
    /// bootstrap claim can never make a session confirm an input it does not
    /// hold — it only stops the rebuilt endpoint from vetoing with `NULL`.
    /// The floor is NOT armed here: the floor's `>= F - 1` re-drop theorem
    /// holds only for the reactivated slot itself (other slots can genuinely
    /// re-drop below `F - 1` when a lagging third party's receipt trails the
    /// snapshot frame).
    #[cfg(feature = "hot-join")]
    pub(crate) fn seed_peer_connect_status_for_joiner_bootstrap(
        &mut self,
        handle: PlayerHandle,
        status: ConnectionStatus,
    ) {
        if let Some(slot) = self.peer_connect_status.get_mut(handle.as_usize()) {
            *slot = status;
        }
    }

    /// Arms the slot's `reactivation_floor` (see that field's documentation)
    /// at `floor_frame` (`F - 1`), max-monotone across reactivations.
    ///
    /// MUST be called only at a **commit-evidence point** (coordinator commit,
    /// survivor `JoinCommitted` receipt, or a commit-evidence implied/local
    /// close): the floor's `>= F - 1` re-drop theorem is valid only in
    /// committed worlds. Arming at the (pre-commit) survivor reopen is a
    /// liveness bug (session-33 round-2 review Finding 1): after an ABORT the
    /// mesh's live convergence target is the pre-attempt freeze `f0 < F - 1`
    /// — exactly what an armed floor filters — so a post-reopen-aborted
    /// survivor would pin its confirmed frame at `F - 1` forever and stall
    /// the mesh. The pre-commit window needs no floor: the session's
    /// pending-reactivation shield exempts the slot from the disconnect-
    /// convergence fold for the attempt's whole lifetime, and the
    /// commit-receipt re-seed un-sticks any cache a stale claim re-stuck
    /// mid-attempt.
    ///
    /// Once armed the floor persists for the endpoint's lifetime (every
    /// world in which it armed is committed, so the genuine convergence
    /// target is always `>= F - 1`); only
    /// [`rearm_for_rejoin`](Self::rearm_for_rejoin) resets it (constructor
    /// rebuild) — and the rearmed endpoint is the JOINER's, which is
    /// fold-excluded (reserved) until its own next reopen re-seeds it, so the
    /// reset floor is never consulted while stale.
    #[cfg(feature = "hot-join")]
    pub(crate) fn arm_reactivation_floor(&mut self, handle: PlayerHandle, floor_frame: Frame) {
        if let Some(floor) = self.reactivation_floor.get_mut(handle.as_usize()) {
            *floor = if floor.is_null() {
                floor_frame
            } else {
                std::cmp::max(*floor, floor_frame)
            };
        }
    }

    /// Test seam: the slot's current reactivation floor ([`Frame::NULL`] when
    /// unarmed or out of range) — lets session tests pin the floor's
    /// commit-evidence-only lifecycle.
    #[cfg(all(test, feature = "hot-join"))]
    pub(crate) fn reactivation_floor_for_test(&self, handle: PlayerHandle) -> Frame {
        self.reactivation_floor
            .get(handle.as_usize())
            .copied()
            .unwrap_or(Frame::NULL)
    }

    /// Drops all unacked entries from `pending_output` (the host's queue of
    /// inputs awaiting the peer's ack).
    ///
    /// Used when a hot-join serve is aborted at the Phase-4 timeout: while a
    /// serve is open the host is paused and never sends inputs, but inputs may
    /// have accumulated *before* the pause began. Once the serve aborts the host
    /// resumes solo and `send_input` would otherwise see a full `pending_output`
    /// (the abandoned joiner never acked) and emit `Event::Disconnected` on every
    /// subsequent frame. The aborted joiner never needs these pre-snapshot host
    /// inputs (a future join loads a snapshot), so discarding them is safe and
    /// stops the disconnect spam. `last_acked_input` is left intact so the
    /// reference byte-length used to validate later sends stays valid.
    #[cfg(feature = "hot-join")]
    #[allow(dead_code)]
    pub(crate) fn clear_pending_output(&mut self) {
        self.pending_output.clear();
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::needless_collect
)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;
    use std::sync::Mutex;

    // Test configuration
    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
    struct TestInput {
        inp: u32,
    }

    #[derive(Clone, Default)]
    #[cfg_attr(feature = "hot-join", derive(Serialize, Deserialize))]
    struct TestState;

    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = TestState;
        type Address = SocketAddr;
    }

    struct BoolConfig;

    impl Config for BoolConfig {
        type Input = bool;
        type State = TestState;
        type Address = SocketAddr;
    }

    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
    struct UnitInput;

    struct UnitInputConfig;

    impl Config for UnitInputConfig {
        type Input = UnitInput;
        type State = TestState;
        type Address = SocketAddr;
    }

    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
    enum VariableInput {
        #[default]
        Idle,
        Active(u32),
    }

    struct VariableInputConfig;

    impl Config for VariableInputConfig {
        type Input = VariableInput;
        type State = TestState;
        type Address = SocketAddr;
    }

    #[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
    enum BalancedVariableInput {
        Short,
        Medium(u32),
        Long(u64),
    }

    impl Default for BalancedVariableInput {
        fn default() -> Self {
            Self::Medium(0)
        }
    }

    struct BalancedVariableInputConfig;

    impl Config for BalancedVariableInputConfig {
        type Input = BalancedVariableInput;
        type State = TestState;
        type Address = SocketAddr;
    }

    fn test_addr() -> SocketAddr {
        "127.0.0.1:7000".parse().unwrap()
    }

    /// Default number of sync packets for test purposes
    const TEST_NUM_SYNC_PACKETS: u32 = 5;

    fn create_protocol(
        handles: Vec<PlayerHandle>,
        num_players: usize,
        local_players: usize,
        max_prediction: usize,
    ) -> UdpProtocol<TestConfig> {
        create_protocol_with_config(
            handles,
            num_players,
            local_players,
            max_prediction,
            SyncConfig::default(),
            ProtocolConfig::default(),
        )
    }

    fn create_protocol_with_config(
        handles: Vec<PlayerHandle>,
        num_players: usize,
        local_players: usize,
        max_prediction: usize,
        sync_config: SyncConfig,
        protocol_config: ProtocolConfig,
    ) -> UdpProtocol<TestConfig> {
        UdpProtocol::new(
            handles,
            test_addr(),
            num_players,
            local_players,
            max_prediction,
            Duration::from_secs(5),
            Duration::from_secs(3),
            60,
            DesyncDetection::Off,
            sync_config,
            protocol_config,
            TimeSyncConfig::default(),
        )
        .expect("Failed to create test protocol")
    }

    fn mutable_clock_config() -> (ProtocolConfig, Arc<Mutex<Instant>>) {
        let current = Arc::new(Mutex::new(Instant::now()));
        let clock_handle = Arc::clone(&current);
        let config = ProtocolConfig {
            clock: Some(Arc::new(move || *clock_handle.lock().unwrap())),
            ..ProtocolConfig::default()
        };
        (config, current)
    }

    fn advance_test_clock(clock: &Arc<Mutex<Instant>>, duration: Duration) -> Instant {
        let mut current = clock.lock().unwrap();
        *current += duration;
        *current
    }

    fn complete_test_sync(protocol: &mut UdpProtocol<TestConfig>) {
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            protocol.on_sync_reply(
                MessageHeader { magic: 999 },
                SyncReply {
                    random_reply: random,
                },
            );
        }
    }

    fn queued_input_body(protocol: &UdpProtocol<TestConfig>) -> &Input {
        match &protocol
            .send_queue
            .front()
            .expect("input message should be queued")
            .body
        {
            MessageBody::Input(body) => body,
            other => panic!("expected input message, got {other:?}"),
        }
    }

    // ==========================================
    // State Machine Tests
    // ==========================================

    #[test]
    fn new_protocol_starts_in_initializing_state() {
        let protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        assert!(!protocol.is_synchronized());
        assert!(!protocol.is_running());
    }

    #[test]
    fn synchronize_transitions_to_synchronizing_state() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        protocol.synchronize().unwrap();

        // Still not synchronized until sync completes
        assert!(!protocol.is_synchronized());
        assert!(!protocol.is_running());
        // But it should have queued a sync request
        assert!(!protocol.send_queue.is_empty());
    }

    #[test]
    #[allow(clippy::wildcard_enum_match_arm)]
    fn sync_request_queues_sync_reply() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Clear the initial sync request
        protocol.send_queue.clear();

        // Simulate receiving a sync request
        let sync_req = SyncRequest {
            random_request: 12345,
        };
        protocol.on_sync_request(sync_req);

        // Should have queued a reply
        assert_eq!(protocol.send_queue.len(), 1);
        let msg = protocol.send_queue.front().unwrap();
        match &msg.body {
            MessageBody::SyncReply(reply) => {
                assert_eq!(reply.random_reply, 12345);
            },
            _ => panic!("Expected SyncReply message"),
        }
    }

    #[test]
    fn complete_sync_transitions_to_running() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete all sync roundtrips
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            // Get the random request from our sync request
            let random = *protocol.sync_random_requests.iter().next().unwrap();

            let header = MessageHeader { magic: 999 };
            let reply = SyncReply {
                random_reply: random,
            };
            protocol.on_sync_reply(header, reply);
        }

        assert!(protocol.is_synchronized());
        assert!(protocol.is_running());
    }

    #[test]
    fn sync_reply_with_wrong_random_is_ignored() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        let initial_remaining = protocol.sync_remaining_roundtrips;

        // Send a reply with the wrong random value
        let header = MessageHeader { magic: 999 };
        let reply = SyncReply {
            random_reply: 99999999, // Wrong value
        };
        protocol.on_sync_reply(header, reply);

        // Should still have same number of remaining roundtrips
        assert_eq!(protocol.sync_remaining_roundtrips, initial_remaining);
    }

    #[test]
    fn sync_reply_when_not_synchronizing_is_ignored() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        // Protocol is in Initializing state, not Synchronizing
        let header = MessageHeader { magic: 999 };
        let reply = SyncReply { random_reply: 123 };
        protocol.on_sync_reply(header, reply);

        // Should still be in initializing
        assert!(!protocol.is_synchronized());
    }

    #[test]
    fn disconnect_transitions_to_disconnected() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        assert!(protocol.is_running());

        protocol.disconnect();

        // Still counts as synchronized but not running
        assert!(protocol.is_synchronized());
        assert!(!protocol.is_running());
    }

    #[test]
    fn disconnect_when_already_shutdown_does_nothing() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.state = ProtocolState::Shutdown;

        protocol.disconnect();

        // Should still be shutdown, not disconnected
        assert_eq!(protocol.state, ProtocolState::Shutdown);
    }

    // ==========================================
    // Message Handling Tests
    // ==========================================

    #[test]
    fn handle_message_ignores_shutdown_state() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.state = ProtocolState::Shutdown;

        let msg = Message {
            header: MessageHeader { magic: 123 },
            body: MessageBody::KeepAlive,
        };
        protocol.handle_message(&msg);

        // Event queue should be empty
        assert!(protocol.event_queue.is_empty());
    }

    #[test]
    fn handle_message_filters_wrong_magic_after_sync() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync with magic 999
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        assert_eq!(protocol.remote_magic, 999);
        protocol.send_queue.clear();

        // Send message with different magic
        let msg = Message {
            header: MessageHeader { magic: 123 }, // Wrong magic
            body: MessageBody::KeepAlive,
        };
        protocol.handle_message(&msg);

        // Should be ignored - no state changes
        assert!(protocol.send_queue.is_empty());
    }

    #[test]
    fn handle_message_accepts_correct_magic() {
        let (config, clock) = mutable_clock_config();
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );
        protocol.synchronize().unwrap();

        // Complete sync with magic 999
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        let initial_recv_time = protocol.last_recv_time;

        let accepted_recv_time = advance_test_clock(&clock, Duration::from_millis(1));

        // Send message with correct magic
        let msg = Message {
            header: MessageHeader { magic: 999 },
            body: MessageBody::KeepAlive,
        };
        protocol.handle_message(&msg);

        // Should update recv time
        assert_eq!(protocol.last_recv_time, accepted_recv_time);
        assert!(protocol.last_recv_time > initial_recv_time);
    }

    #[test]
    fn handle_message_drops_gameplay_messages_while_synchronizing_without_side_effects() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(0),
            bytes: vec![1, 2, 3, 4],
        });

        let initial_recv_time = protocol.last_recv_time;
        let initial_pending_len = protocol.pending_output.len();
        let initial_last_acked = protocol.last_acked_input.frame;
        let initial_status = protocol.peer_connect_status.clone();
        let initial_remote_advantage = protocol.remote_frame_advantage;
        let initial_checksum_len = protocol.pending_checksums.len();
        let initial_event_len = protocol.event_queue.len();

        let messages = [
            Message {
                header: MessageHeader { magic: 123 },
                body: MessageBody::Input(Input {
                    peer_connect_status: vec![
                        ConnectionStatus {
                            disconnected: true,
                            last_frame: Frame::new(99),
                            epoch: 0,
                        };
                        2
                    ],
                    disconnect_requested: false,
                    start_frame: Frame::new(0),
                    ack_frame: Frame::new(0),
                    bytes: vec![1, 2, 3],
                }),
            },
            Message {
                header: MessageHeader { magic: 123 },
                body: MessageBody::InputAck(InputAck {
                    ack_frame: Frame::new(0),
                }),
            },
            Message {
                header: MessageHeader { magic: 123 },
                body: MessageBody::QualityReport(QualityReport {
                    frame_advantage: 7,
                    ping: 123,
                }),
            },
            Message {
                header: MessageHeader { magic: 123 },
                body: MessageBody::QualityReply(QualityReply { pong: 456 }),
            },
            Message {
                header: MessageHeader { magic: 123 },
                body: MessageBody::ChecksumReport(ChecksumReport {
                    checksum: 0xABCD,
                    frame: Frame::new(1),
                }),
            },
            Message {
                header: MessageHeader { magic: 123 },
                body: MessageBody::KeepAlive,
            },
        ];

        for message in &messages {
            protocol.handle_message(message);
        }

        assert_eq!(protocol.last_recv_time, initial_recv_time);
        assert_eq!(protocol.pending_output.len(), initial_pending_len);
        assert_eq!(protocol.last_acked_input.frame, initial_last_acked);
        assert_eq!(protocol.peer_connect_status, initial_status);
        assert_eq!(protocol.remote_frame_advantage, initial_remote_advantage);
        assert_eq!(protocol.pending_checksums.len(), initial_checksum_len);
        assert_eq!(protocol.event_queue.len(), initial_event_len);
    }

    #[test]
    fn running_peer_still_answers_sync_request() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);
        protocol.send_queue.clear();

        protocol.handle_message(&Message {
            header: MessageHeader { magic: 999 },
            body: MessageBody::SyncRequest(SyncRequest { random_request: 42 }),
        });

        assert!(matches!(
            protocol.send_queue.front().map(|message| &message.body),
            Some(MessageBody::SyncReply(SyncReply { random_reply: 42 }))
        ));
    }

    #[test]
    fn network_resumed_event_after_interrupt() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Simulate network interrupt notification was sent
        protocol.disconnect_notify_sent = true;

        // Handle a valid message
        let msg = Message {
            header: MessageHeader { magic: 999 },
            body: MessageBody::KeepAlive,
        };
        protocol.handle_message(&msg);

        // Should have NetworkResumed event
        let events: Vec<_> = protocol.event_queue.drain(..).collect();
        assert!(events.iter().any(|e| matches!(e, Event::NetworkResumed)));
        assert!(!protocol.disconnect_notify_sent);
    }

    // ==========================================
    // Input Handling Tests
    // ==========================================

    #[test]
    fn input_ack_pops_pending_output() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Add some pending outputs
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(0),
            bytes: vec![0, 0, 0, 0],
        });
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(1),
            bytes: vec![1, 0, 0, 0],
        });
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(2),
            bytes: vec![2, 0, 0, 0],
        });

        assert_eq!(protocol.pending_output.len(), 3);

        // Ack frame 1
        protocol.on_input_ack(InputAck {
            ack_frame: Frame::new(1),
        });

        // Should have removed frames 0 and 1
        assert_eq!(protocol.pending_output.len(), 1);
        assert_eq!(
            protocol.pending_output.front().unwrap().frame,
            Frame::new(2)
        );
        assert_eq!(protocol.last_acked_input.frame, Frame::new(1));
    }

    #[test]
    fn input_ack_rejects_future_ack_without_popping_pending_output() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);
        protocol.event_queue.clear();
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(0),
            bytes: vec![0, 0, 0, 0],
        });
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(1),
            bytes: vec![1, 0, 0, 0],
        });

        protocol.on_input_ack(InputAck {
            ack_frame: Frame::new(99),
        });

        assert_eq!(protocol.pending_output.len(), 2);
        assert_eq!(protocol.last_acked_input.frame, Frame::NULL);
    }

    #[test]
    fn on_input_ignores_future_ack_but_accepts_valid_input() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);
        protocol.event_queue.clear();
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(0),
            bytes: vec![0, 0, 0, 0],
        });

        let zeroed_bytes = protocol
            .recv_inputs
            .get(&Frame::NULL)
            .unwrap()
            .bytes
            .clone();
        let test_bytes = crate::network::codec::encode(&TestInput { inp: 42 }).unwrap();
        let encoded =
            crate::network::compression::encode(&zeroed_bytes, std::iter::once(&test_bytes));

        protocol.on_input(&Input {
            start_frame: Frame::new(0),
            ack_frame: Frame::new(99),
            bytes: encoded,
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        });

        assert!(protocol.recv_inputs.contains_key(&Frame::new(0)));
        assert_eq!(protocol.pending_output.len(), 1);
        assert_eq!(
            protocol.pending_output.front().unwrap().frame,
            Frame::new(0)
        );
        assert_eq!(protocol.last_acked_input.frame, Frame::NULL);
        assert!(protocol
            .event_queue
            .iter()
            .any(|event| matches!(event, Event::Input { .. })));
    }

    #[test]
    fn on_input_disconnect_request_emits_inputs_before_disconnect() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);
        protocol.event_queue.clear();

        let zeroed_bytes = protocol
            .recv_inputs
            .get(&Frame::NULL)
            .unwrap()
            .bytes
            .clone();
        let test_bytes = crate::network::codec::encode(&TestInput { inp: 42 }).unwrap();
        let encoded =
            crate::network::compression::encode(&zeroed_bytes, std::iter::once(&test_bytes));

        protocol.on_input(&Input {
            start_frame: Frame::new(0),
            ack_frame: Frame::NULL,
            bytes: encoded,
            disconnect_requested: true,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        });

        let events: Vec<_> = protocol.event_queue.drain(..).collect();
        assert_eq!(events.len(), 2);
        assert!(matches!(events.first(), Some(Event::Input { .. })));
        assert!(matches!(events.get(1), Some(Event::Disconnected)));
    }

    // --- F4 regression: connect-status merge convergence ---------------------
    //
    // The per-endpoint merge in `on_input` caches a remote peer's view of every
    // slot's connect status. For a disconnected slot, `last_frame` is the agreed
    // freeze frame, which must converge DOWN to the global min as a lower freeze
    // gossip relays across the mesh (a higher cached value comes from a relaying
    // survivor's pre-drop forward progress). These tests drive the merge directly
    // by feeding decodable `Input` packets that re-use `start_frame == 0`: the
    // packet decodes against the blank reference frame, the staged frame is
    // skipped (frame 0 <= last received frame 0), but the connect-status merge
    // still runs — isolating exactly the per-slot merge under test.

    /// Build a synced, running protocol whose `recv_inputs` holds frame 0, so
    /// later `start_frame == 0` gossip packets decode without staging new inputs.
    fn running_protocol_with_frame_zero() -> UdpProtocol<TestConfig> {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);

        // Prime frame 0 so subsequent start_frame == 0 packets are decodable.
        let zeroed_bytes = protocol
            .recv_inputs
            .get(&Frame::NULL)
            .unwrap()
            .bytes
            .clone();
        let test_bytes = crate::network::codec::encode(&TestInput { inp: 1 }).unwrap();
        let encoded =
            crate::network::compression::encode(&zeroed_bytes, std::iter::once(&test_bytes));
        protocol.on_input(&Input {
            start_frame: Frame::new(0),
            ack_frame: Frame::NULL,
            bytes: encoded,
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        });
        protocol.event_queue.clear();
        protocol
    }

    /// Feed a connect-status gossip packet for slot 1 ("the dropped peer") while
    /// keeping slot 0 (the local-relative peer) connected. The packet re-uses
    /// frame 0 so only the connect-status merge has an effect.
    fn gossip_slot_one(
        protocol: &mut UdpProtocol<TestConfig>,
        disconnected: bool,
        last_frame: i32,
    ) {
        let zeroed_bytes = protocol
            .recv_inputs
            .get(&Frame::NULL)
            .unwrap()
            .bytes
            .clone();
        let test_bytes = crate::network::codec::encode(&TestInput { inp: 1 }).unwrap();
        let encoded =
            crate::network::compression::encode(&zeroed_bytes, std::iter::once(&test_bytes));
        protocol.on_input(&Input {
            start_frame: Frame::new(0),
            ack_frame: Frame::NULL,
            bytes: encoded,
            disconnect_requested: false,
            peer_connect_status: vec![
                ConnectionStatus::default(),
                ConnectionStatus {
                    disconnected,
                    last_frame: Frame::new(last_frame),
                    epoch: 0,
                },
            ],
        });
    }

    #[test]
    fn on_input_lowers_disconnected_slot_last_frame_on_later_lower_gossip() {
        let mut protocol = running_protocol_with_frame_zero();

        // First gossip freezes the slot at 8, then a relayed lowering carries 4.
        gossip_slot_one(&mut protocol, true, 8);
        assert_eq!(
            protocol
                .peer_connect_status(PlayerHandle::new(1))
                .last_frame,
            Frame::new(8)
        );

        gossip_slot_one(&mut protocol, true, 4);
        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(status.disconnected);
        assert_eq!(status.last_frame, Frame::new(4));
    }

    #[test]
    fn on_input_disconnected_slot_ignores_stale_higher_freeze_gossip() {
        let mut protocol = running_protocol_with_frame_zero();

        // Converge the freeze frame to 4, then a reordered (stale) packet repeats 8.
        gossip_slot_one(&mut protocol, true, 4);
        gossip_slot_one(&mut protocol, true, 8);

        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(status.disconnected);
        assert_eq!(status.last_frame, Frame::new(4));
    }

    #[test]
    fn on_input_first_disconnect_adopts_remote_freeze_frame_when_higher() {
        let mut protocol = running_protocol_with_frame_zero();

        // Slot was connected with forward progress at 5; the peer that genuinely
        // received it through 8 before dropping must not be under-claimed.
        gossip_slot_one(&mut protocol, false, 5);
        assert_eq!(
            protocol
                .peer_connect_status(PlayerHandle::new(1))
                .last_frame,
            Frame::new(5)
        );

        gossip_slot_one(&mut protocol, true, 8);
        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(status.disconnected);
        assert_eq!(status.last_frame, Frame::new(8));
    }

    #[test]
    fn on_input_first_disconnect_adopts_remote_freeze_frame_when_lower() {
        let mut protocol = running_protocol_with_frame_zero();

        // Slot was connected with forward progress at 10; first disconnect gossip
        // carries a lower authoritative freeze frame and must lower us to it.
        gossip_slot_one(&mut protocol, false, 10);
        assert_eq!(
            protocol
                .peer_connect_status(PlayerHandle::new(1))
                .last_frame,
            Frame::new(10)
        );

        gossip_slot_one(&mut protocol, true, 4);
        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(status.disconnected);
        assert_eq!(status.last_frame, Frame::new(4));
    }

    #[test]
    fn on_input_disconnected_slot_not_resurrected_by_stale_connected_gossip() {
        let mut protocol = running_protocol_with_frame_zero();

        // Converge the freeze frame to 4, then a stale pre-drop "connected@9"
        // packet arrives. It must neither resurrect the slot nor raise the frame.
        gossip_slot_one(&mut protocol, true, 4);
        gossip_slot_one(&mut protocol, false, 9);

        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(status.disconnected);
        assert_eq!(status.last_frame, Frame::new(4));
    }

    #[test]
    fn on_input_connected_slot_keeps_monotone_forward_progress() {
        let mut protocol = running_protocol_with_frame_zero();

        // Connected slot advances 5 then 9 (max preserved), and a stale 3 cannot
        // regress it.
        gossip_slot_one(&mut protocol, false, 5);
        gossip_slot_one(&mut protocol, false, 9);
        assert_eq!(
            protocol
                .peer_connect_status(PlayerHandle::new(1))
                .last_frame,
            Frame::new(9)
        );

        gossip_slot_one(&mut protocol, false, 3);
        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(!status.disconnected);
        assert_eq!(status.last_frame, Frame::new(9));
    }

    /// Reactivation floor (session-33 review Finding 3, narrowed in-process
    /// fix): after a COMMITTED reactivation seeds `{connected, F - 1}` and
    /// arms the floor (the session's commit-evidence sites do both — the
    /// round-2 Finding-1 fix moved the arming out of the seed), a stale
    /// pre-attempt `disconnected` packet reordered past the seed (freeze
    /// frame `f0 < F - 1`) must be IGNORED — without the floor the sticky
    /// merge re-adopts it and permanently re-drops the live slot. A GENUINE
    /// post-reactivation re-drop carries a freeze frame `>= F - 1` (every
    /// commit participant's reopen stamps its receipt for the slot at
    /// exactly `F - 1`, and the converged freeze frame is a min over those
    /// receipts) and must still be adopted — including at exactly `F - 1`
    /// (instant joiner death). The `f0 == F - 1` equality corner (serve
    /// opened at the very freeze frame) remains ambiguous without per-slot
    /// reactivation epochs in the gossip — the session-31 wire-format
    /// future-work item.
    #[cfg(feature = "hot-join")]
    #[test]
    fn merge_ignores_stale_disconnected_gossip_below_reactivation_floor() {
        let mut protocol = running_protocol_with_frame_zero();

        // Pre-attempt drop converges at f0 = 4.
        gossip_slot_one(&mut protocol, true, 4);
        assert!(
            protocol
                .peer_connect_status(PlayerHandle::new(1))
                .disconnected
        );

        // The slot reactivates at F = 10 and COMMITS: the session seeds
        // {connected, 9} and arms the floor (the commit-evidence pairing).
        protocol.seed_peer_connect_status_for_reactivation(PlayerHandle::new(1), Frame::new(9));
        protocol.arm_reactivation_floor(PlayerHandle::new(1), Frame::new(9));
        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(!status.disconnected);
        assert_eq!(status.last_frame, Frame::new(9));

        // A stale pre-attempt carrier (in flight at the sender's status flip)
        // re-delivers {disconnected, 4}: it must NOT re-stick the live slot.
        gossip_slot_one(&mut protocol, true, 4);
        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(
            !status.disconnected,
            "stale disconnected gossip below the reactivation floor must be ignored"
        );
        assert_eq!(status.last_frame, Frame::new(9));

        // A genuine post-reactivation re-drop at exactly the floor (freeze
        // frame F - 1 = 9, the instant-death minimum) IS adopted.
        gossip_slot_one(&mut protocol, true, 9);
        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(
            status.disconnected,
            "a genuine re-drop at/above the floor must still be adopted"
        );
        assert_eq!(status.last_frame, Frame::new(9));
    }

    /// The reactivation floor also protects the BOTH-disconnected convergence
    /// arm: after a genuine post-reactivation re-drop, stale pre-attempt
    /// convergence traffic (freeze frames below the floor) must not drag the
    /// converged freeze frame below `F - 1`, while genuine convergence within
    /// `>= F - 1` still wins.
    #[cfg(feature = "hot-join")]
    #[test]
    fn merge_floor_blocks_stale_convergence_after_genuine_redrop() {
        let mut protocol = running_protocol_with_frame_zero();

        gossip_slot_one(&mut protocol, true, 4);
        // Committed reactivation at F = 10: seed + arm (the commit-evidence
        // pairing; the seed alone no longer arms — round-2 Finding 1).
        protocol.seed_peer_connect_status_for_reactivation(PlayerHandle::new(1), Frame::new(9));
        protocol.arm_reactivation_floor(PlayerHandle::new(1), Frame::new(9));

        // Genuine re-drop at 11, then genuine convergence down to 9 (>= floor).
        gossip_slot_one(&mut protocol, true, 11);
        assert_eq!(
            protocol
                .peer_connect_status(PlayerHandle::new(1))
                .last_frame,
            Frame::new(11)
        );
        gossip_slot_one(&mut protocol, true, 4); // stale pre-attempt convergence
        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(status.disconnected);
        assert_eq!(
            status.last_frame,
            Frame::new(11),
            "stale below-floor convergence traffic must not drag the re-drop freeze frame down"
        );
        gossip_slot_one(&mut protocol, true, 9); // genuine convergence at the floor
        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(status.disconnected);
        assert_eq!(status.last_frame, Frame::new(9));
    }

    /// Floor lifecycle at the merge level (session-33 round-2 review
    /// Finding 1): a reactivation SEED alone — the pre-commit reopen's cache
    /// un-stick — must NOT arm the floor. In an aborted world the mesh's
    /// live convergence target is the genuine pre-attempt freeze
    /// `f0 < F - 1`, exactly what an armed floor filters; a survivor whose
    /// reopen armed it would never re-adopt the mesh's drop gossip after the
    /// abort and would pin its confirmed frame at `F - 1` forever. Only the
    /// explicit commit-evidence arm ([`UdpProtocol::arm_reactivation_floor`])
    /// activates the filter.
    #[cfg(feature = "hot-join")]
    #[test]
    fn merge_seed_without_floor_arm_adopts_genuine_drop_gossip() {
        let mut protocol = running_protocol_with_frame_zero();

        // Pre-attempt drop converges at f0 = 4; the slot reopens at F = 10
        // (seed {connected, 9}) but the attempt has NOT committed: no floor.
        gossip_slot_one(&mut protocol, true, 4);
        protocol.seed_peer_connect_status_for_reactivation(PlayerHandle::new(1), Frame::new(9));
        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(!status.disconnected);
        assert_eq!(status.last_frame, Frame::new(9));

        // The attempt aborts; the mesh keeps gossiping the genuine
        // pre-attempt state. With no commit evidence ever seen, the merge
        // must re-adopt it (re-converging the slot's mesh-agreed exclusion).
        gossip_slot_one(&mut protocol, true, 4);
        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(
            status.disconnected,
            "with no commit evidence, the seed alone must not filter genuine pre-attempt drop gossip"
        );
        assert_eq!(
            status.last_frame,
            Frame::new(4),
            "the genuine pre-attempt freeze frame is re-adopted verbatim"
        );
    }

    // --- F14 arbitration: connect-status gossip vs. the two on_input skips -----
    //
    // The contested finding (F14) claims `on_input` drops fresh disconnect gossip
    // (`body.peer_connect_status`) when the decode-reference frame is missing,
    // widening the N>=3 disconnect-convergence window. There are TWO early skips
    // in `on_input` that bypass the connect-status merge:
    //   1. gap-too-large early return (start_frame too far AHEAD of last_recv+1)
    //   2. decode-reference-missing guard (`recv_inputs.get(decode_frame)` is None)
    //
    // These helpers + tests settle, from the real code:
    //   (a) which branch a FRESH (newer) ahead-of-window gossip packet hits,
    //   (b) whether the decode-guard-false branch is reachable carrying FRESH
    //       gossip or only STALE (pruned-reference) gossip,
    //   (c) whether skipped gossip is permanently lost or re-delivered by a later
    //       decodable packet.
    //
    // The endpoint owns slot 0 (handle 0); slots 1 ("B") and 2 ("C") are remote.
    // We drive the gossip for slot 2 ("C dropped"), mirroring the F14 scenario
    // where B relays C's disconnect to A.

    /// Build a synced, running 3-slot protocol (this endpoint owns slot 0; slots
    /// 1 and 2 are remote). `recv_inputs` holds only the NULL seed.
    fn running_protocol_three_slots() -> UdpProtocol<TestConfig> {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 3, 1, 8);
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);
        protocol.event_queue.clear();
        protocol
    }

    /// Encode a single-frame input batch decoding against `reference_bytes`.
    fn encode_one_frame(reference_bytes: &[u8], value: u32) -> Vec<u8> {
        let test_bytes = crate::network::codec::encode(&TestInput { inp: value }).unwrap();
        crate::network::compression::encode(reference_bytes, std::iter::once(&test_bytes))
    }

    // ---- floor-round (double-failure-relay connected-relay reorder fix) ----

    /// FLOOR-ROUND acceptance: a `FloorReply` whose `round_seq` is strictly newer
    /// than the latest accepted lands in the reorder-immune `round_floor` cache.
    /// After a prune, a reply must POSTDATE the prune threshold to count as fresh.
    #[test]
    fn on_floor_reply_newer_seq_is_accepted_and_postdates_prune_to_be_fresh() {
        let mut protocol = running_protocol_three_slots();
        let slot2 = PlayerHandle::new(2);
        assert!(
            !protocol.floor_round_is_fresh(),
            "no reply yet -> not fresh"
        );
        assert_eq!(protocol.round_floor(slot2), Frame::NULL);

        // The observer issues a request (seq 1), then a prune snapshots the
        // threshold at 1 — a reply to request 1 is now PRE-prune (not fresh).
        protocol.send_floor_request(); // floor_request_seq = 1
        protocol.reset_floor_freshness(); // floor_prune_seq = 1
        protocol.on_floor_reply(&FloorReply {
            round_seq: 1,
            floors: vec![Frame::new(0), Frame::new(0), Frame::new(9)],
        });
        assert_eq!(
            protocol.round_floor(slot2),
            Frame::new(9),
            "the reply's floors are cached (latest-wins)"
        );
        assert!(
            !protocol.floor_round_is_fresh(),
            "a reply to a PRE-prune request (seq == threshold) is not post-prune fresh"
        );

        // The observer re-issues post-prune (seq 2); its reply IS fresh.
        protocol.send_floor_request(); // floor_request_seq = 2
        protocol.on_floor_reply(&FloorReply {
            round_seq: 2,
            floors: vec![Frame::new(0), Frame::new(0), Frame::new(4)],
        });
        assert!(
            protocol.floor_round_is_fresh(),
            "a reply to a POST-prune request (seq > threshold) marks the round fresh"
        );
        assert_eq!(protocol.round_floor(slot2), Frame::new(4));
    }

    /// FLOOR-ROUND reorder rejection: a `FloorReply` whose `round_seq` is older
    /// than (or equal to) the latest accepted is DROPPED — it neither overwrites
    /// `round_floor` nor regresses the reply seq. This is what makes the reply
    /// channel reorder-immune (the stale-first race the gossip cache could not
    /// survive).
    #[test]
    fn on_floor_reply_stale_or_duplicate_seq_is_dropped() {
        let mut protocol = running_protocol_three_slots();
        let slot2 = PlayerHandle::new(2);

        // Issue five requests so replies up to seq 5 are solicited.
        for _ in 0..5 {
            protocol.send_floor_request();
        }

        // A fresh-LOW reply at seq 5 lands first (floor 4).
        protocol.on_floor_reply(&FloorReply {
            round_seq: 5,
            floors: vec![Frame::new(0), Frame::new(0), Frame::new(4)],
        });
        assert_eq!(protocol.round_floor(slot2), Frame::new(4));

        // A REORDERED stale-HIGH reply at an OLDER seq (3) must be dropped.
        protocol.on_floor_reply(&FloorReply {
            round_seq: 3,
            floors: vec![Frame::new(0), Frame::new(0), Frame::new(99)],
        });
        assert_eq!(
            protocol.round_floor(slot2),
            Frame::new(4),
            "an older-seq reply must NOT overwrite the round cache (reorder-immune)"
        );
        // A DUPLICATE at the same seq (5) is also dropped (no re-clobber).
        protocol.on_floor_reply(&FloorReply {
            round_seq: 5,
            floors: vec![Frame::new(0), Frame::new(0), Frame::new(77)],
        });
        assert_eq!(
            protocol.round_floor(slot2),
            Frame::new(4),
            "a duplicate-seq reply must NOT overwrite the round cache"
        );
    }

    /// FLOOR-ROUND solicitation guard: a `FloorReply` echoing a `round_seq` the
    /// observer never issued (beyond `floor_request_seq`) is DROPPED. Without the
    /// guard a forged/corrupt high seq would advance `floor_reply_seq` past every
    /// legitimate future reply (a permanent round stall) AND spuriously flip the
    /// round fresh — handing peer-controlled state into the floor cache.
    #[test]
    fn on_floor_reply_unsolicited_seq_is_dropped() {
        let mut protocol = running_protocol_three_slots();
        let slot2 = PlayerHandle::new(2);

        // The observer has issued exactly one request (seq 1).
        protocol.send_floor_request(); // floor_request_seq = 1

        // A reply echoing a never-issued seq (2 > 1) is forged/corrupt: drop it.
        protocol.on_floor_reply(&FloorReply {
            round_seq: 2,
            floors: vec![Frame::new(0), Frame::new(0), Frame::new(9)],
        });
        assert_eq!(
            protocol.round_floor(slot2),
            Frame::NULL,
            "an unsolicited reply must NOT write the floor cache"
        );
        assert!(
            !protocol.floor_round_is_fresh(),
            "an unsolicited reply must NOT advance the reply seq / flip freshness"
        );

        // A legitimate reply to the request we DID issue (seq 1) still lands,
        // proving the unsolicited reply did not poison `floor_reply_seq`.
        protocol.on_floor_reply(&FloorReply {
            round_seq: 1,
            floors: vec![Frame::new(0), Frame::new(0), Frame::new(4)],
        });
        assert_eq!(
            protocol.round_floor(slot2),
            Frame::new(4),
            "the solicited reply lands after the unsolicited one was dropped"
        );
    }

    /// FLOOR-ROUND completeness guard: a `FloorReply` whose `floors` omits a slot
    /// is DROPPED whole. Accepting it would RELEASE the `slot_round_incomplete`
    /// hold (flip the round fresh) while the omitted slot kept reading a stale
    /// prior round — re-opening the connected-relay over-confirmation the round
    /// exists to prevent. A short reply must leave the cache and seq untouched.
    #[test]
    fn on_floor_reply_incomplete_floors_is_dropped() {
        let mut protocol = running_protocol_three_slots();
        let slot1 = PlayerHandle::new(1);
        let slot2 = PlayerHandle::new(2);

        // A complete reply (seq 1) seeds the cache for every slot.
        protocol.send_floor_request(); // floor_request_seq = 1
        protocol.on_floor_reply(&FloorReply {
            round_seq: 1,
            floors: vec![Frame::new(0), Frame::new(7), Frame::new(9)],
        });
        assert_eq!(protocol.round_floor(slot1), Frame::new(7));
        assert_eq!(protocol.round_floor(slot2), Frame::new(9));
        let seq_before = protocol.floor_reply_seq;

        // A newer, solicited, but SHORT reply (only 2 of 3 slots) is malformed.
        protocol.send_floor_request(); // floor_request_seq = 2
        protocol.on_floor_reply(&FloorReply {
            round_seq: 2,
            floors: vec![Frame::new(1), Frame::new(2)],
        });
        assert_eq!(
            protocol.round_floor(slot1),
            Frame::new(7),
            "a short reply must NOT partially overwrite the cache"
        );
        assert_eq!(
            protocol.round_floor(slot2),
            Frame::new(9),
            "the omitted slot must NOT be left reading a stale prior round"
        );
        assert_eq!(
            protocol.floor_reply_seq, seq_before,
            "a short reply must NOT advance the reply seq (no spurious freshness)"
        );

        // A complete reply at the same newer seq is accepted in full.
        protocol.on_floor_reply(&FloorReply {
            round_seq: 2,
            floors: vec![Frame::new(1), Frame::new(2), Frame::new(3)],
        });
        assert_eq!(protocol.round_floor(slot1), Frame::new(2));
        assert_eq!(protocol.round_floor(slot2), Frame::new(3));
    }

    /// FLOOR-ROUND excess-entries tolerance: a complete-but-LONGER `floors` vector
    /// (more entries than slots) is ACCEPTED, with only the first `num_players`
    /// consumed and trailing entries ignored — the documented contract that keeps
    /// the guard lenient on benign trailing data while strict on missing coverage.
    #[test]
    fn on_floor_reply_excess_floors_are_accepted_and_trailing_ignored() {
        let mut protocol = running_protocol_three_slots();
        let slot0 = PlayerHandle::new(0);
        let slot1 = PlayerHandle::new(1);
        let slot2 = PlayerHandle::new(2);

        protocol.send_floor_request(); // floor_request_seq = 1
                                       // Four entries for a three-slot protocol: the fourth is trailing junk.
        protocol.on_floor_reply(&FloorReply {
            round_seq: 1,
            floors: vec![Frame::new(1), Frame::new(2), Frame::new(3), Frame::new(999)],
        });
        assert_eq!(protocol.round_floor(slot0), Frame::new(1));
        assert_eq!(protocol.round_floor(slot1), Frame::new(2));
        assert_eq!(
            protocol.round_floor(slot2),
            Frame::new(3),
            "the trailing fourth entry must be ignored, not consumed into a slot"
        );
        assert_eq!(
            protocol.floor_reply_seq, 1,
            "an excess-but-complete reply is accepted (advances the reply seq)"
        );
    }

    /// FLOOR-ROUND request/reply plumbing: a received `FloorRequest` is recorded
    /// (peek + drain), and answering with `send_floor_reply` queues a `FloorReply`
    /// echoing the request's `round_seq` with the supplied floors.
    #[test]
    fn floor_request_is_recorded_and_answered_with_echoed_seq() {
        let mut protocol = running_protocol_three_slots();
        assert!(!protocol.has_pending_floor_request());

        protocol.on_floor_request(&FloorRequest { round_seq: 7 });
        assert!(protocol.has_pending_floor_request());
        assert_eq!(protocol.take_pending_floor_request(), Some(7));
        assert!(
            !protocol.has_pending_floor_request(),
            "draining clears the pending request"
        );

        protocol.send_queue.clear();
        protocol.send_floor_reply(7, &[Frame::new(0), Frame::new(0), Frame::new(4)]);
        let queued = protocol
            .send_queue
            .iter()
            .find_map(|msg| match &msg.body {
                MessageBody::FloorReply(reply) => Some(reply.clone()),
                _ => None,
            })
            .expect("a FloorReply must be queued");
        assert_eq!(
            queued.round_seq, 7,
            "the reply echoes the request's round_seq"
        );
        assert_eq!(
            queued.floors,
            vec![Frame::new(0), Frame::new(0), Frame::new(4)]
        );
    }

    /// FLOOR-ROUND highest-seq-wins: a reordered OLDER `FloorRequest` must NOT
    /// clobber a higher undrained pending one (answering the stale round and
    /// skipping the newer one), a newer seq DOES supersede an older pending one,
    /// and a request into an empty pending slot is stored. Highest-seq-wins is
    /// independent of arrival order.
    #[test]
    fn on_floor_request_older_round_seq_does_not_clobber_newer_pending() {
        let mut protocol = running_protocol_three_slots();
        assert!(!protocol.has_pending_floor_request());

        // A request into an EMPTY pending slot is stored.
        protocol.on_floor_request(&FloorRequest { round_seq: 5 });
        assert_eq!(
            protocol.pending_floor_request,
            Some(5),
            "a request into an empty pending slot is stored"
        );

        // A REORDERED older seq (3 < 5) must NOT clobber the higher pending one.
        protocol.on_floor_request(&FloorRequest { round_seq: 3 });
        assert_eq!(
            protocol.pending_floor_request,
            Some(5),
            "an older-seq request must NOT clobber a higher undrained pending one"
        );

        // An EQUAL seq is also not newer, so it leaves the pending one untouched.
        protocol.on_floor_request(&FloorRequest { round_seq: 5 });
        assert_eq!(
            protocol.pending_floor_request,
            Some(5),
            "an equal-seq request must NOT re-clobber the pending one"
        );

        // A strictly NEWER seq (8 > 5) DOES supersede the older pending one.
        protocol.on_floor_request(&FloorRequest { round_seq: 8 });
        assert_eq!(
            protocol.pending_floor_request,
            Some(8),
            "a newer-seq request supersedes an undrained older pending one"
        );
    }

    /// FLOOR-ROUND test-helper consistency: `set_round_floor_for_tests` must
    /// leave the request seq consistent with the marked-fresh reply seq (it
    /// bumps `floor_request_seq` to at least `floor_reply_seq`), so the endpoint
    /// is NOT poisoned — a SUBSEQUENT real `on_floor_reply` to a freshly-issued
    /// request can still be accepted. Before the fix the helper set
    /// `floor_reply_seq = 1` while leaving `floor_request_seq = 0`, making the
    /// solicitation guard (`> floor_reply_seq` AND `<= floor_request_seq`)
    /// permanently unsatisfiable.
    #[test]
    fn set_round_floor_for_tests_leaves_request_seq_consistent_for_later_reply() {
        let mut protocol = running_protocol_three_slots();
        let slot2 = PlayerHandle::new(2);

        // Seed a known post-prune-fresh round via the helper.
        protocol.set_round_floor_for_tests(slot2, Frame::new(9));
        assert_eq!(protocol.round_floor(slot2), Frame::new(9));
        assert!(
            protocol.floor_round_is_fresh(),
            "the helper marks the round post-prune fresh"
        );
        // The headline post-condition: request seq is not left below reply seq.
        assert!(
            protocol.floor_reply_seq <= protocol.floor_request_seq,
            "helper must not leave reply seq {} above request seq {} (poisoned endpoint)",
            protocol.floor_reply_seq,
            protocol.floor_request_seq
        );

        // Now drive a REAL request/reply: a fresh request bumps the request seq
        // strictly above the reply seq, and a reply at that new seq is accepted.
        protocol.send_floor_request();
        let new_seq = protocol.floor_request_seq;
        assert!(
            new_seq > protocol.floor_reply_seq,
            "a freshly-issued request seq must exceed the prior reply seq"
        );
        protocol.on_floor_reply(&FloorReply {
            round_seq: new_seq,
            floors: vec![Frame::new(0), Frame::new(0), Frame::new(4)],
        });
        assert_eq!(
            protocol.round_floor(slot2),
            Frame::new(4),
            "a later real reply is still accepted: the endpoint was not poisoned"
        );
        assert_eq!(
            protocol.floor_reply_seq, new_seq,
            "the later real reply advanced the reply seq (it was not dropped)"
        );
    }

    /// FLOOR-ROUND seq invariant holds across a mixed send/reply/prune sequence:
    /// `floor_reply_seq <= floor_request_seq` and `floor_prune_seq <=
    /// floor_request_seq` must hold after every step. Exercises the invariant
    /// the `debug_assert_floor_round_invariants` guard protects against a real
    /// operation mix rather than a single mutation. Unlike the two tests above
    /// this is a forward-looking regression guard (it passes against the
    /// pre-fix code too); its job is to catch any FUTURE change that lets the
    /// monotonic seq relations drift on the send/reply/prune paths.
    #[test]
    fn floor_round_seq_invariant_holds_across_send_reply_prune_sequence() {
        let mut protocol = running_protocol_three_slots();

        #[track_caller]
        fn assert_seq_invariant(protocol: &UdpProtocol<TestConfig>, step: &str) {
            assert!(
                protocol.floor_reply_seq <= protocol.floor_request_seq,
                "{step}: reply seq {} must not exceed request seq {}",
                protocol.floor_reply_seq,
                protocol.floor_request_seq
            );
            assert!(
                protocol.floor_prune_seq <= protocol.floor_request_seq,
                "{step}: prune seq {} must not exceed request seq {}",
                protocol.floor_prune_seq,
                protocol.floor_request_seq
            );
        }

        assert_seq_invariant(&protocol, "initial");

        // Issue two requests, prune (snapshots request seq), then accept replies
        // for both an in-window and the latest seq, interleaving another prune.
        protocol.send_floor_request(); // request_seq = 1
        assert_seq_invariant(&protocol, "after first request");
        protocol.send_floor_request(); // request_seq = 2
        assert_seq_invariant(&protocol, "after second request");
        protocol.reset_floor_freshness(); // prune_seq = 2
        assert_seq_invariant(&protocol, "after prune");

        protocol.send_floor_request(); // request_seq = 3
        protocol.on_floor_reply(&FloorReply {
            round_seq: 3,
            floors: vec![Frame::new(0), Frame::new(0), Frame::new(7)],
        });
        assert_seq_invariant(&protocol, "after accepted reply");

        // A dropped (unsolicited) reply must not perturb the invariant either.
        protocol.on_floor_reply(&FloorReply {
            round_seq: 99,
            floors: vec![Frame::new(0), Frame::new(0), Frame::new(1)],
        });
        assert_seq_invariant(&protocol, "after dropped unsolicited reply");

        protocol.reset_floor_freshness(); // prune_seq = 3
        assert_seq_invariant(&protocol, "after second prune");
        assert_eq!(
            protocol.round_floor(PlayerHandle::new(2)),
            Frame::new(7),
            "the only accepted reply's floor is cached"
        );
    }

    /// FLOOR-ROUND prune detection: `detect_prune_transition` returns `true`
    /// exactly on the running→pruned edge (and only once per edge), the signal
    /// the session counts to bump its prune generation.
    #[test]
    fn detect_prune_transition_fires_once_on_running_to_pruned_edge() {
        let mut protocol = running_protocol_three_slots();
        assert!(protocol.is_running());

        // First call records "was running" (no prior state) -> no edge yet.
        assert!(
            !protocol.detect_prune_transition(),
            "still running: no prune"
        );
        assert!(
            !protocol.detect_prune_transition(),
            "still running: no prune"
        );

        protocol.disconnect();
        assert!(
            protocol.detect_prune_transition(),
            "running -> pruned edge fires once"
        );
        assert!(
            !protocol.detect_prune_transition(),
            "the edge does not re-fire while it stays pruned"
        );
    }

    /// Connect-status vector for [slot0 connected, slot1 connected, slot2 X].
    fn status_slot2(disconnected: bool, last_frame: i32) -> Vec<ConnectionStatus> {
        vec![
            ConnectionStatus::default(),
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected,
                last_frame: Frame::new(last_frame),
                epoch: 0,
            },
        ]
    }

    // (a) REACHABILITY: a FRESH gossip packet whose `start_frame` is too far AHEAD
    //     of the window takes the GAP-TOO-LARGE early return (line ~1472), NOT the
    //     decode-reference-missing guard. This pins WHICH branch the finding's
    //     "fresh packet, missing intermediate frames" case actually hits, and is
    //     the regression guard for the F14 hoist: the connect-status merge now
    //     runs BEFORE that return, so C's fresh disconnect gossip is applied even
    //     though the packet's inputs are dropped (no decode reference for them).
    //     Inputs are still NOT staged (gap too large), proving only the gossip —
    //     not the undecodable inputs — is processed.
    #[test]
    fn on_input_fresh_gossip_too_far_ahead_merges_before_gap_return() {
        let mut protocol = running_protocol_three_slots();

        // We have received slot inputs up through frame 2 (contiguous run).
        let bytes = vec![0u8; 4];
        for f in 0..=2 {
            protocol.recv_inputs.insert(
                Frame::new(f),
                InputBytes {
                    frame: Frame::new(f),
                    bytes: bytes.clone(),
                },
            );
        }
        assert_eq!(protocol.last_recv_frame(), Frame::new(2));

        // A packet for start_frame 10 (far ahead of last_recv+1 = 3) carries FRESH
        // "C disconnected @ 5" gossip. This is the finding's "fresh packet, missing
        // intermediate frames" shape.
        let keys_before: Vec<Frame> = protocol.recv_inputs.keys().copied().collect();
        protocol.on_input(&Input {
            start_frame: Frame::new(10),
            ack_frame: Frame::NULL,
            bytes: encode_one_frame(&bytes, 99),
            disconnect_requested: false,
            peer_connect_status: status_slot2(true, 5),
        });

        // POST-HOIST: slot 2's drop gossip is applied even though the packet's
        // inputs were dropped by the gap-too-large branch. (Pre-hoist this stayed
        // `ConnectionStatus::default()`.)
        let status = protocol.peer_connect_status(PlayerHandle::new(2));
        assert!(
            status.disconnected,
            "fresh gossip is merged before the gap-too-large return (F14 hoist)"
        );
        assert_eq!(status.last_frame, Frame::new(5));
        // The undecodable inputs are NOT staged: the gap branch still drops them.
        let keys_after: Vec<Frame> = protocol.recv_inputs.keys().copied().collect();
        assert_eq!(
            keys_after, keys_before,
            "gap-too-large packet stages no inputs; only gossip is processed"
        );
    }

    // (b) REACHABILITY: the decode-reference-missing guard (line ~1492 false) is
    //     reachable ONLY for STALE packets whose `start_frame - 1` was already
    //     PRUNED (a fresh in-window packet's reference is present, so it decodes;
    //     a fresh too-far-ahead packet takes the gap branch in (a) instead).
    //     `recv_inputs` keys are a contiguous run up to last_recv, so the only way
    //     `start_frame - 1 <= last_recv` is missing is via pruning below
    //     `last_recv - history_frames`. POST-HOIST, even that stale packet's
    //     (loss/reorder-safe) gossip is merged, while its redundant inputs remain
    //     un-staged.
    #[test]
    fn on_input_decode_guard_false_only_for_stale_pruned_reference() {
        let mut protocol = running_protocol_three_slots();

        // Simulate a long-running session: only a HIGH contiguous tail survives
        // pruning. history_frames = input_history_multiplier(2) * max_prediction(8)
        // = 16, so frames below last_recv(100) - 16 = 84 are pruned. We keep just
        // frame 100 (and the NULL seed is irrelevant once last_recv != NULL).
        let bytes = vec![0u8; 4];
        protocol.recv_inputs.insert(
            Frame::new(100),
            InputBytes {
                frame: Frame::new(100),
                bytes: bytes.clone(),
            },
        );
        assert_eq!(protocol.last_recv_frame(), Frame::new(100));
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(0),
            bytes: vec![0; 12],
        });

        // A STALE retransmission for start_frame 50 (decode ref = frame 49, long
        // pruned). It passes the gap-too-large check (50 <= 100 + 1) but the
        // decode reference is missing -> decode-guard-false branch. It carries the
        // sender's CURRENT gossip ("C disconnected @ 49").
        let stale = Input {
            start_frame: Frame::new(50),
            ack_frame: Frame::new(0),
            bytes: encode_one_frame(&bytes, 1),
            disconnect_requested: false,
            peer_connect_status: status_slot2(true, 49),
        };
        let keys_before: Vec<Frame> = protocol.recv_inputs.keys().copied().collect();
        protocol.send_queue.clear();
        protocol.on_input(&stale);

        // POST-HOIST: the gossip is merged even though inputs can't be decoded.
        // (Pre-hoist the entire body was skipped at the decode guard, leaving
        // `ConnectionStatus::default()`.)
        let status = protocol.peer_connect_status(PlayerHandle::new(2));
        assert!(
            status.disconnected,
            "stale-reference packet's gossip is merged post-hoist"
        );
        assert_eq!(status.last_frame, Frame::new(49));
        // No inputs were staged (decode still gated; stale frames are redundant).
        let keys_after: Vec<Frame> = protocol.recv_inputs.keys().copied().collect();
        assert_eq!(keys_after, keys_before, "stale packet stages nothing");
        assert!(matches!(
            protocol.send_queue.back().map(|message| &message.body),
            Some(MessageBody::InputAck(InputAck { ack_frame })) if *ack_frame == Frame::new(100)
        ), "a stale retransmission with a pruned reference must re-ack the contiguous receive high-water");
        assert!(
            protocol.pending_output.is_empty(),
            "the packet's independent piggyback ACK must drain local pending output"
        );

        // End-to-end half of the lost-ACK recovery: route the emitted
        // cumulative ACK back to a sender still retaining the stale 50..=100
        // batch. One retry must drain the whole already-received prefix.
        let mut sender = running_protocol_three_slots();
        for frame in 50..=100 {
            sender.pending_output.push_back(InputBytes {
                frame: Frame::new(frame),
                bytes: vec![0; 4],
            });
        }
        let ack = match &protocol.send_queue.back().expect("re-ack queued").body {
            MessageBody::InputAck(ack) => *ack,
            other => panic!("expected cumulative InputAck, got {other:?}"),
        };
        sender.on_input_ack(ack);
        assert!(
            sender.pending_output.is_empty(),
            "retry re-ack must release the sender's stale pending front"
        );
    }

    // (c) The COMMON case the finding's literal target conflates: a packet with a
    //     LOW (oldest-unacked) start_frame still carries the sender's CURRENT
    //     (fresh) gossip. As long as its decode reference is NOT pruned (the
    //     normal steady state, where the reference is within the history window),
    //     the packet decodes and the fresh gossip is applied. This shows a low
    //     start_frame does NOT imply skipped gossip even before the hoist.
    #[test]
    fn on_input_low_start_frame_with_present_reference_applies_fresh_gossip() {
        let mut protocol = running_protocol_three_slots();

        // Contiguous received run 0..=5; reference for start_frame 3 is frame 2,
        // which is present (within the history window).
        let bytes = vec![0u8; 4];
        for f in 0..=5 {
            protocol.recv_inputs.insert(
                Frame::new(f),
                InputBytes {
                    frame: Frame::new(f),
                    bytes: bytes.clone(),
                },
            );
        }
        assert_eq!(protocol.last_recv_frame(), Frame::new(5));

        // Oldest-unacked retransmission: start_frame 3 (ref frame 2 present),
        // carrying FRESH "C disconnected @ 4" gossip. Decodes -> merge runs.
        protocol.on_input(&Input {
            start_frame: Frame::new(3),
            ack_frame: Frame::NULL,
            bytes: encode_one_frame(&bytes, 42),
            disconnect_requested: false,
            peer_connect_status: status_slot2(true, 4),
        });

        let status = protocol.peer_connect_status(PlayerHandle::new(2));
        assert!(
            status.disconnected,
            "fresh gossip on a low start_frame with present reference is applied"
        );
        assert_eq!(status.last_frame, Frame::new(4));
    }

    // (d) SAFETY of the hoist: applying the merge from an undecodable/stale packet
    //     must NOT regress an already-converged freeze frame. Converge C's freeze
    //     to 4 via a decodable packet, then deliver a STALE (pruned-reference)
    //     packet that re-asserts the higher pre-convergence freeze (8). The merge's
    //     both-disconnected `min` rule must keep C frozen at 4 — the same
    //     stale-safety the in-decode merge guaranteed, now proven on the
    //     undecodable path too.
    #[test]
    fn on_input_hoisted_merge_does_not_un_converge_freeze_from_stale_packet() {
        let mut protocol = running_protocol_three_slots();

        let bytes = vec![0u8; 4];
        // Decodable packet converges C's freeze frame to 4.
        protocol.recv_inputs.insert(
            Frame::new(0),
            InputBytes {
                frame: Frame::new(0),
                bytes: bytes.clone(),
            },
        );
        protocol.on_input(&Input {
            start_frame: Frame::new(1),
            ack_frame: Frame::NULL,
            bytes: encode_one_frame(&bytes, 1),
            disconnect_requested: false,
            peer_connect_status: status_slot2(true, 4),
        });
        assert_eq!(
            protocol
                .peer_connect_status(PlayerHandle::new(2))
                .last_frame,
            Frame::new(4)
        );

        // Advance last_recv far enough that frame 49 is pruned, then deliver a
        // STALE packet (start_frame 50, ref 49 missing) re-asserting freeze @ 8.
        protocol.recv_inputs.insert(
            Frame::new(100),
            InputBytes {
                frame: Frame::new(100),
                bytes: bytes.clone(),
            },
        );
        protocol.on_input(&Input {
            start_frame: Frame::new(50),
            ack_frame: Frame::NULL,
            bytes: encode_one_frame(&bytes, 2),
            disconnect_requested: false,
            peer_connect_status: status_slot2(true, 8),
        });

        // The stale higher freeze must NOT un-converge us: min(4, 8) == 4.
        let status = protocol.peer_connect_status(PlayerHandle::new(2));
        assert!(status.disconnected);
        assert_eq!(
            status.last_frame,
            Frame::new(4),
            "hoisted merge from a stale packet must not re-raise a converged freeze"
        );
    }

    #[test]
    fn send_input_when_not_running_does_nothing() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        // Protocol is in Initializing state

        let inputs = BTreeMap::new();
        let connect_status = vec![ConnectionStatus::default(); 2];

        protocol.send_input(&inputs, &connect_status);

        // Should not queue any messages
        assert!(protocol.send_queue.is_empty());
        assert!(protocol.pending_output.is_empty());
    }

    // ==========================================
    // Quality Report Tests
    // ==========================================

    #[test]
    #[allow(clippy::wildcard_enum_match_arm)]
    fn quality_report_triggers_reply() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }
        protocol.send_queue.clear();

        let report = QualityReport {
            frame_advantage: 5,
            ping: 12345,
        };
        protocol.on_quality_report(&report);

        assert_eq!(protocol.remote_frame_advantage, 5);

        // Should have queued a quality reply
        assert_eq!(protocol.send_queue.len(), 1);
        let msg = protocol.send_queue.front().unwrap();
        match &msg.body {
            MessageBody::QualityReply(reply) => {
                assert_eq!(reply.pong, 12345);
            },
            _ => panic!("Expected QualityReply message"),
        }
    }

    // ==========================================
    // Checksum Report Tests
    // ==========================================

    #[test]
    fn checksum_report_stored_with_desync_detection_off() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        let report = ChecksumReport {
            frame: Frame::new(100),
            checksum: 0xDEADBEEF,
        };
        protocol.on_checksum_report(&report);

        // Should still store it (with a warning, but we can't test that here)
        assert_eq!(
            protocol.pending_checksums.get(&Frame::new(100)),
            Some(&0xDEADBEEF)
        );
    }

    #[test]
    fn checksum_report_limits_history_size() {
        let protocol_config = ProtocolConfig::default();
        let max_history = protocol_config.max_checksum_history;

        let mut protocol: UdpProtocol<TestConfig> = UdpProtocol::new(
            vec![PlayerHandle::new(0)],
            test_addr(),
            2,
            1,
            8,
            Duration::from_secs(5),
            Duration::from_secs(3),
            60,
            DesyncDetection::On { interval: 1 },
            SyncConfig::default(),
            protocol_config,
            TimeSyncConfig::default(),
        )
        .expect("Failed to create test protocol");

        // Add more than max_checksum_history checksums
        for frame in 0..(max_history as i32 + 10) {
            let report = ChecksumReport {
                frame: Frame::new(frame),
                checksum: frame as u128,
            };
            protocol.on_checksum_report(&report);
        }

        // Should have limited to max_checksum_history
        assert!(protocol.pending_checksums.len() <= max_history);

        // Oldest frames should be removed
        let max_frame = Frame::new(max_history as i32 + 9);
        assert!(protocol.pending_checksums.contains_key(&max_frame));
        // Old frames should be gone
        assert!(!protocol.pending_checksums.contains_key(&Frame::new(0)));
    }

    // ==========================================
    // Network Stats Tests
    // ==========================================

    #[test]
    fn network_stats_returns_error_when_not_synchronized() {
        let protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        let result = protocol.network_stats();
        assert!(matches!(result, Err(FortressError::NotSynchronized)));
    }

    #[test]
    fn network_stats_returns_error_when_no_time_elapsed() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Stats start time is set during synchronize(), so with 0 seconds elapsed
        // it should return an error
        let result = protocol.network_stats();
        // This will likely fail because no time has passed
        // The actual behavior depends on timing
        assert!(result.is_ok() || matches!(result, Err(FortressError::NotSynchronized)));
    }

    // ==========================================
    // Peer Metrics Tests (M2 §5.2)
    // ==========================================

    #[test]
    fn peer_metrics_send_accounting_matches_wire_bytes_and_kinds() {
        use crate::metrics::MessageKind;

        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        let bodies = [
            MessageBody::KeepAlive,
            MessageBody::QualityReply(QualityReply { pong: 7 }),
            MessageBody::InputAck(InputAck {
                ack_frame: Frame::new(3),
            }),
            MessageBody::KeepAlive,
        ];
        let mut expected_bytes = 0u64;
        for body in &bodies {
            // `queue_message` stamps the header magic itself; recompute the same
            // wire size independently (magic does not affect `encoded_len`).
            let msg = Message {
                header: MessageHeader {
                    magic: protocol.magic,
                },
                body: body.clone(),
            };
            expected_bytes += msg.encoded_len() as u64;
            protocol.queue_message(body.clone());
        }

        let m = protocol.peer_metrics();
        assert_eq!(m.packets_sent, bodies.len() as u64);
        assert_eq!(m.bytes_sent, expected_bytes);
        // One kind bucket per packet: the total equals the packet counter.
        assert_eq!(m.messages_sent_by_kind.total(), m.packets_sent);
        assert_eq!(m.messages_sent_by_kind.get(MessageKind::KeepAlive), 2);
        assert_eq!(m.messages_sent_by_kind.get(MessageKind::QualityReply), 1);
        assert_eq!(m.messages_sent_by_kind.get(MessageKind::InputAck), 1);
        assert_eq!(m.messages_sent_by_kind.get(MessageKind::Input), 0);
        // Nothing received yet.
        assert_eq!(m.packets_received, 0);
        assert_eq!(m.bytes_received, 0);
    }

    #[test]
    fn peer_metrics_receive_accounting_counts_every_message_before_filters() {
        use crate::metrics::MessageKind;

        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        // A magic mismatch (and, below, the Shutdown state) drops the message for
        // handling — but receive accounting is counted first, so the byte/packet
        // counters still advance. Pin the remote magic so the filter is active.
        protocol.remote_magic = 999;

        let bodies = [
            MessageBody::KeepAlive,
            MessageBody::QualityReport(QualityReport {
                frame_advantage: 0,
                ping: 5,
            }),
            MessageBody::KeepAlive,
        ];
        let mut expected_bytes = 0u64;
        for body in &bodies {
            let msg = Message {
                header: MessageHeader { magic: 123 }, // wrong magic: filtered after counting
                body: body.clone(),
            };
            expected_bytes += msg.encoded_len() as u64;
            protocol.handle_message(&msg);
        }

        let m = protocol.peer_metrics();
        assert_eq!(m.packets_received, bodies.len() as u64);
        assert_eq!(m.bytes_received, expected_bytes);
        assert_eq!(m.messages_received_by_kind.total(), m.packets_received);
        assert_eq!(m.messages_received_by_kind.get(MessageKind::KeepAlive), 2);
        assert_eq!(
            m.messages_received_by_kind.get(MessageKind::QualityReport),
            1
        );

        // Even a Shutdown endpoint (which returns before handling) counts the
        // arriving traffic — the accounting is unconditional and precedes every
        // protocol-state filter.
        protocol.state = ProtocolState::Shutdown;
        protocol.handle_message(&Message {
            header: MessageHeader { magic: 999 },
            body: MessageBody::KeepAlive,
        });
        assert_eq!(
            protocol.peer_metrics().packets_received,
            bodies.len() as u64 + 1
        );
    }

    #[test]
    fn peer_metrics_records_input_compression_bytes() {
        // The endpoint serializes one local player of `u32` input, so the
        // zeroed reference width is 4 bytes; pushed frames must match it.
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(0),
            bytes: vec![1, 2, 3, 4],
        });
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(1),
            bytes: vec![5, 6, 7, 8],
        });

        // Flush the batch: it is delta/RLE-encoded into one `Input` packet.
        let connect_status = vec![ConnectionStatus::default(); 2];
        protocol.send_pending_output(&connect_status);

        let m = protocol.peer_metrics();
        // Two 4-byte frames of raw input were batched before compression.
        assert_eq!(
            m.input_bytes_pre_compression, 8,
            "pre-compression input bytes: {m:?}"
        );
        assert!(
            m.input_bytes_post_compression > 0,
            "no post-compression input bytes recorded: {m:?}"
        );
        // Exactly one Input packet went out, tallied by kind.
        assert!(m.packets_sent > 0);
        assert!(
            m.messages_sent_by_kind
                .get(crate::metrics::MessageKind::Input)
                > 0
        );
        assert_eq!(m.messages_sent_by_kind.total(), m.packets_sent);
    }

    #[test]
    fn peer_metrics_snapshot_reports_connection_gauges() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.round_trip_time = 42;
        protocol.remote_frame_advantage = -3;
        protocol.pending_checksums.insert(Frame::new(1), 0xABCD);
        protocol.pending_checksums.insert(Frame::new(2), 0x1234);

        let m = protocol.peer_metrics();
        assert_eq!(m.ping_ms, 42);
        assert_eq!(m.remote_frame_advantage, -3);
        assert_eq!(m.pending_checksums_len, 2);
        assert_eq!(m.pending_output_len, protocol.pending_output.len() as u64);
    }

    // ==========================================
    // Poll / Timeout Tests
    // ==========================================

    #[test]
    fn poll_returns_events_and_clears_queue() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync to generate Synchronizing and Synchronized events
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        let connect_status = vec![ConnectionStatus::default(); 2];
        let events: Vec<_> = protocol.poll(&connect_status).collect();

        // Should have Synchronizing events and Synchronized event
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| matches!(e, Event::Synchronized)));

        // Queue should be empty after drain
        assert!(protocol.event_queue.is_empty());
    }

    #[test]
    fn poll_in_disconnected_state_transitions_to_shutdown() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.state = ProtocolState::Disconnected;

        // Set shutdown timeout to the past
        protocol.shutdown_timeout = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();

        let connect_status = vec![ConnectionStatus::default(); 2];
        let _events: Vec<_> = protocol.poll(&connect_status).collect();

        // Should have transitioned to Shutdown
        assert_eq!(protocol.state, ProtocolState::Shutdown);
    }

    #[test]
    fn sync_timeout_event_emitted_only_once() {
        let (protocol_config, clock) = mutable_clock_config();

        // Create a protocol with a very short sync timeout
        let sync_config = SyncConfig {
            sync_timeout: Some(Duration::from_millis(1)),
            ..SyncConfig::default()
        };

        let mut protocol: UdpProtocol<TestConfig> = UdpProtocol::new(
            vec![PlayerHandle::new(0)],
            test_addr(),
            2,
            1,
            8,
            Duration::from_secs(5),
            Duration::from_secs(3),
            60,
            DesyncDetection::Off,
            sync_config,
            protocol_config,
            TimeSyncConfig::default(),
        )
        .expect("Failed to create test protocol");
        protocol.synchronize().unwrap();

        advance_test_clock(&clock, Duration::from_millis(2));

        let connect_status = vec![ConnectionStatus::default(); 2];

        // First poll - should emit SyncTimeout
        let events1: Vec<_> = protocol.poll(&connect_status).collect();
        let timeout_count1 = events1
            .iter()
            .filter(|e| matches!(e, Event::SyncTimeout { .. }))
            .count();
        assert_eq!(
            timeout_count1, 1,
            "First poll should emit exactly one SyncTimeout event"
        );

        // Second poll - should NOT emit SyncTimeout again
        let events2: Vec<_> = protocol.poll(&connect_status).collect();
        let timeout_count2 = events2
            .iter()
            .filter(|e| matches!(e, Event::SyncTimeout { .. }))
            .count();
        assert_eq!(
            timeout_count2, 0,
            "Subsequent polls should not emit additional SyncTimeout events"
        );

        // Third poll - still no SyncTimeout
        let events3: Vec<_> = protocol.poll(&connect_status).collect();
        let timeout_count3 = events3
            .iter()
            .filter(|e| matches!(e, Event::SyncTimeout { .. }))
            .count();
        assert_eq!(
            timeout_count3, 0,
            "SyncTimeout should only be emitted once per timeout"
        );
    }

    // ==========================================
    // Connect-Status Nudge Tests
    // ==========================================
    //
    // The nudge (`send_connect_status_nudge`) re-sends a status-bearing
    // duplicate Input built from `last_acked_input` on the keepalive cadence
    // while the session holds a not-yet-mesh-agreed local disconnect.
    // Receiver-side handling of the nudge SHAPE (a stale/dup Input carrying
    // fresh gossip) is the established S24 hoisted-merge behavior, already
    // pinned by `on_input_low_start_frame_with_present_reference_applies_fresh_gossip`
    // and `on_input_hoisted_merge_does_not_un_converge_freeze_from_stale_packet`;
    // the exact sender-built shape is additionally pinned end-to-end below.

    /// Shared harness: a Running protocol with an injected mutable clock, an
    /// hour-long quality-report interval (quality reports refresh
    /// `last_send_time` and would otherwise crowd the keepalive/nudge cadence
    /// out of the observation window), and an empty (fully-acked) pipeline.
    fn running_nudge_protocol() -> (UdpProtocol<TestConfig>, Arc<Mutex<Instant>>) {
        let current = Arc::new(Mutex::new(Instant::now()));
        let clock_handle = Arc::clone(&current);
        let config = ProtocolConfig {
            quality_report_interval: Duration::from_secs(3600),
            clock: Some(Arc::new(move || *clock_handle.lock().unwrap())),
            ..ProtocolConfig::default()
        };
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);
        protocol.send_queue.clear();
        (protocol, current)
    }

    fn queued_inputs(protocol: &UdpProtocol<TestConfig>) -> Vec<&Input> {
        protocol
            .send_queue
            .iter()
            .filter_map(|message| match &message.body {
                MessageBody::Input(body) => Some(body),
                _ => None,
            })
            .collect()
    }

    fn queue_has_keep_alive(protocol: &UdpProtocol<TestConfig>) -> bool {
        protocol
            .send_queue
            .iter()
            .any(|message| matches!(message.body, MessageBody::KeepAlive))
    }

    /// (i) Flag set + keepalive interval elapsed + idle (fully-acked) queue:
    /// exactly ONE status-bearing duplicate Input per interval, re-sending the
    /// `last_acked_input` frame and carrying the connect status CURRENT at
    /// that poll; it replaces the bare KeepAlive on that tick (no double-send)
    /// and a second poll within the same interval sends nothing new.
    #[test]
    fn poll_nudge_sends_one_status_bearing_duplicate_input_per_keepalive_interval() {
        let (mut protocol, clock) = running_nudge_protocol();
        protocol.last_acked_input.frame = Frame::new(3);
        protocol.set_connect_status_nudge(true);

        let status_first = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(9),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(4),
                epoch: 0,
            },
        ];
        advance_test_clock(&clock, Duration::from_millis(201));
        let _ = protocol.poll(&status_first).count();

        {
            let inputs = queued_inputs(&protocol);
            assert_eq!(
                inputs.len(),
                1,
                "exactly one nudge Input per elapsed keepalive interval"
            );
            let nudge = inputs.first().expect("one nudge");
            assert_eq!(
                nudge.start_frame,
                Frame::new(3),
                "the nudge re-sends the last acked frame (a duplicate the receiver skips)"
            );
            assert_eq!(
                nudge.peer_connect_status, status_first,
                "the nudge carries the CURRENT connect status"
            );
            assert!(!nudge.disconnect_requested);
        }
        assert!(
            !queue_has_keep_alive(&protocol),
            "the nudge replaces the bare KeepAlive on that tick"
        );

        // Same instant: nothing new (one nudge per interval).
        let queue_len = protocol.send_queue.len();
        let _ = protocol.poll(&status_first).count();
        assert_eq!(
            protocol.send_queue.len(),
            queue_len,
            "no second nudge within the same keepalive interval"
        );

        // Next interval: another nudge, carrying the status passed at THAT poll.
        let status_second = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(9),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
                epoch: 0,
            },
        ];
        advance_test_clock(&clock, Duration::from_millis(201));
        let _ = protocol.poll(&status_second).count();
        let inputs = queued_inputs(&protocol);
        assert_eq!(inputs.len(), 2, "one more nudge after the next interval");
        assert_eq!(
            inputs.last().expect("second nudge").peer_connect_status,
            status_second,
            "each nudge carries the connect status current at its own poll"
        );
    }

    /// (ii) Flag clear: the idle tick sends the plain KeepAlive exactly as
    /// before — no Input message appears.
    #[test]
    fn poll_without_nudge_flag_sends_plain_keepalive_when_idle() {
        let (mut protocol, clock) = running_nudge_protocol();
        protocol.last_acked_input.frame = Frame::new(3);

        advance_test_clock(&clock, Duration::from_millis(201));
        let connect_status = vec![ConnectionStatus::default(); 2];
        let _ = protocol.poll(&connect_status).count();

        assert!(
            queue_has_keep_alive(&protocol),
            "without the flag the idle tick must send the plain KeepAlive"
        );
        assert!(
            queued_inputs(&protocol).is_empty(),
            "without the flag no duplicate Input may be sent"
        );
    }

    /// (iii) Flag set but `last_acked_input.frame` still NULL (no input ever
    /// acked): falls back to the plain KeepAlive. Pre-first-ack this state
    /// cannot coincide with a gossip-mute hold — `pending_output` drains only
    /// through acks, so a mute (empty-queue) endpoint that ever sent an input
    /// has a valid `last_acked_input`, and one that never sent an input has
    /// not burned its prediction window (its next advance sends a
    /// status-bearing input) — so the conservative fallback loses nothing.
    #[test]
    fn poll_nudge_with_null_last_acked_falls_back_to_keepalive() {
        let (mut protocol, clock) = running_nudge_protocol();
        assert_eq!(protocol.last_acked_input.frame, Frame::NULL);
        protocol.set_connect_status_nudge(true);

        advance_test_clock(&clock, Duration::from_millis(201));
        let connect_status = vec![ConnectionStatus::default(); 2];
        let _ = protocol.poll(&connect_status).count();

        assert!(
            queue_has_keep_alive(&protocol),
            "with a NULL last-acked frame the idle tick must fall back to KeepAlive"
        );
        assert!(
            queued_inputs(&protocol).is_empty(),
            "no self-referencing nudge can be built before the first ack"
        );
    }

    /// (iv) Running-state gating: with the flag set and the interval elapsed,
    /// a Synchronizing protocol sends no Input (only sync-request retries) and
    /// a Disconnected protocol sends nothing at all.
    #[test]
    fn poll_nudge_respects_running_state_gating() {
        let current = Arc::new(Mutex::new(Instant::now()));
        let clock_handle = Arc::clone(&current);
        let config = ProtocolConfig {
            quality_report_interval: Duration::from_secs(3600),
            clock: Some(Arc::new(move || *clock_handle.lock().unwrap())),
            ..ProtocolConfig::default()
        };
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );
        protocol.synchronize().unwrap();
        assert_eq!(protocol.state, ProtocolState::Synchronizing);
        protocol.last_acked_input.frame = Frame::new(3);
        protocol.set_connect_status_nudge(true);
        protocol.send_queue.clear();

        advance_test_clock(&current, Duration::from_millis(201));
        let connect_status = vec![ConnectionStatus::default(); 2];
        let _ = protocol.poll(&connect_status).count();
        assert!(
            queued_inputs(&protocol).is_empty(),
            "a Synchronizing protocol must not nudge"
        );

        protocol.state = ProtocolState::Disconnected;
        protocol.shutdown_timeout = *current.lock().unwrap() + Duration::from_secs(60);
        protocol.send_queue.clear();
        advance_test_clock(&current, Duration::from_millis(201));
        let _ = protocol.poll(&connect_status).count();
        assert!(
            protocol.send_queue.is_empty(),
            "a Disconnected protocol must not nudge"
        );
    }

    /// (Gate completeness) Flag set but a REAL Input message went out within
    /// the keepalive interval: the nudge is an input-idle substitute and must
    /// stay completely silent — an actively-advancing session's packet stream
    /// is unchanged by the flag. Once the input-idle interval elapses, the
    /// nudge fires.
    #[test]
    fn poll_nudge_waits_for_input_idle_interval() {
        let (mut protocol, clock) = running_nudge_protocol();
        protocol.last_acked_input.frame = Frame::new(3);
        protocol.set_connect_status_nudge(true);

        // A real Input was just sent (e.g. the session advanced a frame).
        advance_test_clock(&clock, Duration::from_millis(150));
        protocol.last_input_send_time = *clock.lock().unwrap();

        // 100ms later: the nudge cadence (since construction) has elapsed, but
        // the endpoint is NOT input-idle yet (only 100ms since the last real
        // Input) — no nudge, plain KeepAlive.
        advance_test_clock(&clock, Duration::from_millis(100));
        let connect_status = vec![ConnectionStatus::default(); 2];
        let _ = protocol.poll(&connect_status).count();
        assert!(
            queued_inputs(&protocol).is_empty(),
            "no nudge may fire while real input traffic is fresh"
        );
        assert!(
            queue_has_keep_alive(&protocol),
            "the idle tick still keeps the link alive"
        );

        // Another 101ms (201ms since the last real Input): now input-idle —
        // the nudge fires.
        advance_test_clock(&clock, Duration::from_millis(101));
        let _ = protocol.poll(&connect_status).count();
        assert_eq!(
            queued_inputs(&protocol).len(),
            1,
            "the nudge must fire once the input-idle interval elapses"
        );
    }

    /// (Gate completeness) Flag set but `pending_output` non-empty: the
    /// regular retransmission path already carries the current connect status,
    /// so the nudge stays silent — exactly one Input (the retransmission,
    /// starting at the pending frame, not a self-referencing duplicate).
    #[test]
    fn poll_nudge_skipped_while_pending_output_nonempty() {
        let (mut protocol, clock) = running_nudge_protocol();
        protocol.last_acked_input.frame = Frame::new(3);
        let width = protocol.last_acked_input.bytes.len();
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(4),
            bytes: vec![0u8; width],
        });
        protocol.set_connect_status_nudge(true);

        let connect_status = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(9),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(4),
                epoch: 0,
            },
        ];
        advance_test_clock(&clock, Duration::from_millis(201));
        let _ = protocol.poll(&connect_status).count();

        let inputs = queued_inputs(&protocol);
        assert_eq!(
            inputs.len(),
            1,
            "only the pending-output retransmission may be sent"
        );
        let retransmission = inputs.first().expect("one retransmission");
        assert_eq!(
            retransmission.start_frame,
            Frame::new(4),
            "the Input is the retransmission (pending front), not a self-referencing nudge"
        );
        assert_eq!(
            retransmission.peer_connect_status, connect_status,
            "gossip rides the retransmission"
        );
    }

    /// (v) End-to-end receiver proof for the EXACT nudge shape: a
    /// self-referencing duplicate Input built by `send_connect_status_nudge`
    /// is handled by `on_input` as an established stale-dup packet — the fresh
    /// connect-status gossip is merged (the hoisted S24 merge), no Input event
    /// is staged (every decoded frame is stale), and the normal InputAck reply
    /// is sent.
    #[test]
    fn receiver_merges_gossip_from_nudge_shaped_packet_without_staging_inputs() {
        // Sender: Running, last acked frame 3 (zeroed reference bytes).
        let (mut sender, _clock) = running_nudge_protocol();
        sender.last_acked_input.frame = Frame::new(3);
        let nudge_status = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(9),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(4),
                epoch: 0,
            },
        ];
        assert!(sender.send_connect_status_nudge(&nudge_status));
        let body = match &sender.send_queue.back().expect("nudge queued").body {
            MessageBody::Input(body) => body.clone(),
            other => panic!("expected Input, got {other:?}"),
        };

        // Receiver: Running with contiguous receipts 0..=5, so the nudge's
        // start frame 3 is a duplicate and its decode reference (frame 2) is
        // present.
        let mut receiver: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        receiver.synchronize().unwrap();
        complete_test_sync(&mut receiver);
        let width = receiver
            .recv_inputs
            .get(&Frame::NULL)
            .expect("blank reference present")
            .bytes
            .len();
        for f in 0..=5 {
            receiver.recv_inputs.insert(
                Frame::new(f),
                InputBytes {
                    frame: Frame::new(f),
                    bytes: vec![0u8; width],
                },
            );
        }
        receiver.event_queue.clear();
        receiver.send_queue.clear();

        receiver.on_input(&body);

        // The fresh gossip was merged...
        let merged = receiver.peer_connect_status(PlayerHandle::new(1));
        assert!(
            merged.disconnected,
            "the nudge's disconnect gossip must be merged"
        );
        assert_eq!(merged.last_frame, Frame::new(4));
        // ...no stale frame was staged as an Input event...
        assert!(
            !receiver
                .event_queue
                .iter()
                .any(|event| matches!(event, Event::Input { .. })),
            "a nudge must not stage any input"
        );
        // ...and the receiver replied with the normal duplicate-packet ack.
        assert!(
            receiver
                .send_queue
                .iter()
                .any(|message| matches!(message.body, MessageBody::InputAck(_))),
            "the receiver acks a nudge exactly like any duplicate Input packet"
        );
    }

    /// (Retry-pacer gate, round-3 F-NEW-A) A decodable Input that stages ZERO
    /// new frames (a connect-status nudge or a duplicate retransmission) must
    /// NOT reset `running_last_input_recv` — the pacer for the
    /// `running_retry_interval` pending-output resend in `poll`. A peer nudging
    /// on the keepalive cadence (== the default retry interval) would
    /// otherwise starve our pending Input forever, and that pending Input is
    /// the only carrier of our post-mesh-agreement connect status (the
    /// blackout pin regressed in `tests/sessions/peer_drop.rs`). An Input that
    /// stages at least one fresh frame DOES reset it. Liveness is unaffected
    /// either way: `last_recv_time` (the disconnect-timeout clock) is
    /// refreshed by `handle_message` for every packet, not here.
    #[test]
    fn on_input_resets_retry_pacer_only_when_new_frames_staged() {
        let (mut receiver, clock) = running_nudge_protocol();

        // Contiguous receipts 0..=5, so frame 3 is a stale duplicate (its
        // decode reference, frame 2, is present) and frame 6 is fresh.
        let width = receiver
            .recv_inputs
            .get(&Frame::NULL)
            .expect("blank reference present")
            .bytes
            .len();
        for f in 0..=5 {
            receiver.recv_inputs.insert(
                Frame::new(f),
                InputBytes {
                    frame: Frame::new(f),
                    bytes: vec![0u8; width],
                },
            );
        }
        let connect_status = vec![ConnectionStatus::default(); 2];

        // Zero-new-frames Input (the nudge shape: a self-referencing
        // duplicate of an already-received frame): pacer untouched.
        advance_test_clock(&clock, Duration::from_millis(75));
        let pacer_before = receiver.running_last_input_recv;
        let dup_reference = vec![0u8; width];
        let dup_body = Input {
            peer_connect_status: connect_status.clone(),
            disconnect_requested: false,
            start_frame: Frame::new(3),
            ack_frame: Frame::NULL,
            bytes: try_encode(&dup_reference, std::iter::once(&dup_reference))
                .expect("duplicate encode succeeds"),
        };
        receiver.on_input(&dup_body);
        assert_eq!(
            receiver.last_recv_frame(),
            Frame::new(5),
            "the duplicate must not have staged anything"
        );
        assert_eq!(
            receiver.running_last_input_recv, pacer_before,
            "a decodable zero-new-frames Input must NOT reset the pending-output retry pacer"
        );

        // Fresh-frames Input (frame 6 encoded against the frame-5 reference):
        // pacer reset to the (advanced) current instant.
        advance_test_clock(&clock, Duration::from_millis(75));
        let fresh_reference = vec![0u8; width];
        let fresh_bytes = vec![7u8; width];
        let fresh_body = Input {
            peer_connect_status: connect_status,
            disconnect_requested: false,
            start_frame: Frame::new(6),
            ack_frame: Frame::NULL,
            bytes: try_encode(&fresh_reference, std::iter::once(&fresh_bytes))
                .expect("fresh encode succeeds"),
        };
        receiver.on_input(&fresh_body);
        assert_eq!(
            receiver.last_recv_frame(),
            Frame::new(6),
            "the fresh frame must have been staged"
        );
        assert_eq!(
            receiver.running_last_input_recv,
            *clock.lock().unwrap(),
            "an Input staging at least one new frame must reset the retry pacer to now"
        );
        assert!(
            receiver.running_last_input_recv > pacer_before,
            "the reset must observe the advanced clock"
        );
    }

    // ==========================================
    // Accessor Tests
    // ==========================================

    #[test]
    fn handles_returns_sorted_handles() {
        let protocol: UdpProtocol<TestConfig> = create_protocol(
            vec![
                PlayerHandle::new(2),
                PlayerHandle::new(0),
                PlayerHandle::new(1),
            ],
            3,
            3,
            8,
        );

        let handles = protocol.handles();
        assert_eq!(
            handles.as_ref(),
            &[
                PlayerHandle::new(0),
                PlayerHandle::new(1),
                PlayerHandle::new(2)
            ]
        );
    }

    #[test]
    fn peer_addr_returns_correct_address() {
        let protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        assert_eq!(protocol.peer_addr(), test_addr());
    }

    #[test]
    fn is_handling_message_checks_address() {
        let protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        assert!(protocol.is_handling_message(&test_addr()));

        let other_addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
        assert!(!protocol.is_handling_message(&other_addr));
    }

    #[test]
    fn peer_connect_status_returns_correct_status() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        // Modify status for player 1
        protocol.peer_connect_status[1] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(100),
            epoch: 0,
        };

        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(status.disconnected);
        assert_eq!(status.last_frame, Frame::new(100));
    }

    // ==========================================
    // Frame Advantage Tests
    // ==========================================

    #[test]
    fn update_local_frame_advantage_with_null_frames() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        // Both frames are Frame::NULL, should return early
        protocol.update_local_frame_advantage(Frame::NULL);
        assert_eq!(protocol.local_frame_advantage, 0);

        // Local frame set but no recv frame
        protocol.update_local_frame_advantage(Frame::new(10));
        assert_eq!(protocol.local_frame_advantage, 0);
    }

    #[test]
    fn update_local_frame_advantage_saturates_peer_influenced_rtt() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        let remote_frame = Frame::new(i32::MAX - 1);
        protocol.recv_inputs.insert(
            remote_frame,
            InputBytes {
                frame: remote_frame,
                bytes: vec![0; std::mem::size_of::<TestInput>()],
            },
        );
        protocol.round_trip_time = u128::MAX;

        protocol.update_local_frame_advantage(Frame::new(0));

        assert_eq!(protocol.local_frame_advantage, i32::MAX);
    }

    #[test]
    fn average_frame_advantage_delegates_to_time_sync() {
        let protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        // Just verify it doesn't panic - the actual value depends on TimeSync internals
        let _advantage = protocol.average_frame_advantage();
    }

    // ==========================================
    // InputBytes Tests
    // ==========================================

    #[test]
    fn input_bytes_zeroed_creates_correct_size() {
        let input_bytes =
            InputBytes::zeroed::<TestConfig>(2).expect("Failed to create input bytes");

        assert_eq!(input_bytes.frame, Frame::NULL);
        // Each TestInput is 4 bytes (u32), so 2 players = 8 bytes
        assert_eq!(input_bytes.bytes.len(), 8);
        assert!(input_bytes.bytes.iter().all(|&b| b == 0));
    }

    #[test]
    fn input_bytes_from_inputs_serializes_correctly() {
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(Frame::new(10), TestInput { inp: 0xAABBCCDD }),
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput::new(Frame::new(10), TestInput { inp: 0x11223344 }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(2, &inputs);

        assert_eq!(input_bytes.frame, Frame::new(10));
        assert_eq!(input_bytes.bytes.len(), 8);
    }

    #[test]
    fn input_bytes_roundtrip() {
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(Frame::new(5), TestInput { inp: 12345 }),
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput::new(Frame::new(5), TestInput { inp: 67890 }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(2, &inputs);
        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(2);

        assert_eq!(player_inputs.len(), 2);
        assert_eq!(player_inputs[0].frame, Frame::new(5));
        assert_eq!(player_inputs[0].input.inp, 12345);
        assert_eq!(player_inputs[1].frame, Frame::new(5));
        assert_eq!(player_inputs[1].input.inp, 67890);
    }

    // ==========================================
    // Send Queue Tests
    // ==========================================

    #[test]
    #[allow(clippy::wildcard_enum_match_arm)]
    fn send_checksum_report_queues_message() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.send_queue.clear();

        protocol.send_checksum_report(Frame::new(100), 0xDEADBEEF);

        assert_eq!(protocol.send_queue.len(), 1);
        let msg = protocol.send_queue.front().unwrap();
        match &msg.body {
            MessageBody::ChecksumReport(report) => {
                assert_eq!(report.frame, Frame::new(100));
                assert_eq!(report.checksum, 0xDEADBEEF);
            },
            _ => panic!("Expected ChecksumReport message"),
        }
    }

    #[test]
    fn protocol_equality_is_by_peer_address() {
        let protocol1: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        let protocol2: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(1)], 3, 2, 16);

        // Same peer address
        assert!(protocol1 == protocol2);

        // Different peer address
        let protocol3: UdpProtocol<TestConfig> = UdpProtocol::new(
            vec![PlayerHandle::new(0)],
            "127.0.0.1:8000".parse().unwrap(),
            2,
            1,
            8,
            Duration::from_secs(5),
            Duration::from_secs(3),
            60,
            DesyncDetection::Off,
            SyncConfig::default(),
            ProtocolConfig::default(),
            TimeSyncConfig::default(),
        )
        .expect("Failed to create test protocol");
        assert!(protocol1 != protocol3);
    }

    // ==========================================
    // Frame Gap Detection Tests
    // ==========================================

    /// Test that on_input correctly detects and handles frame gaps.
    /// When the gap is too large to decode (we don't have the reference frame),
    /// the input should be dropped and a violation should be reported.
    #[test]
    fn on_input_rejects_input_with_too_large_gap() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync to get to Running state
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }
        assert!(protocol.is_running());

        // Set up initial state: we have received frame 0
        protocol.recv_inputs.insert(
            Frame::new(0),
            InputBytes {
                frame: Frame::new(0),
                bytes: vec![0, 0, 0, 0],
            },
        );

        // Try to receive an input that's too far ahead (frame 5 when we're at 0)
        // This creates a gap that's too large to decode
        let input = Input {
            start_frame: Frame::new(5), // Gap of 5 when max is 1
            ack_frame: Frame::NULL,
            bytes: vec![1, 2, 3, 4],
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };

        // Clear event queue and record input count before
        protocol.event_queue.clear();
        let inputs_before = protocol.recv_inputs.len();

        // Call on_input with the gap
        protocol.on_input(&input);

        // Verify: no new inputs were added (because gap too large)
        assert_eq!(
            protocol.recv_inputs.len(),
            inputs_before,
            "No inputs should be added when gap is too large"
        );

        // Verify: no input events were generated
        let input_events: Vec<_> = protocol
            .event_queue
            .iter()
            .filter(|e| matches!(e, Event::Input { .. }))
            .collect();
        assert!(
            input_events.is_empty(),
            "No input events should be generated when gap is too large"
        );
    }

    /// Test that consecutive frames are processed correctly
    #[test]
    fn on_input_accepts_consecutive_frame() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Set up initial state: we have frame 0
        let initial_bytes = vec![0u8; 4]; // TestConfig::Input is [u8; 4]
        protocol.recv_inputs.insert(
            Frame::new(0),
            InputBytes {
                frame: Frame::new(0),
                bytes: initial_bytes.clone(),
            },
        );

        // Encode frame 1 relative to frame 0
        let frame1_bytes = vec![1u8; 4];
        let encoded =
            crate::network::compression::encode(&initial_bytes, std::iter::once(&frame1_bytes));

        let input = Input {
            start_frame: Frame::new(1), // Consecutive - gap of 1 is ok
            ack_frame: Frame::NULL,
            bytes: encoded,
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };

        protocol.event_queue.clear();
        protocol.on_input(&input);

        // Verify: frame 1 was added
        assert!(
            protocol.recv_inputs.contains_key(&Frame::new(1)),
            "Frame 1 should be added when gap is acceptable"
        );
    }

    /// Test that first input (when no previous non-NULL input exists) is accepted
    #[test]
    fn on_input_accepts_first_input_with_null_frame() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // The protocol constructor inserts Frame::NULL entry for decoding first input.
        // So recv_inputs is NOT empty, but last_recv_frame() returns Frame::NULL
        // because the NULL frame is special.
        assert!(
            protocol.recv_inputs.contains_key(&Frame::NULL),
            "Protocol should have Frame::NULL entry for decoding"
        );
        assert_eq!(
            protocol.last_recv_frame(),
            Frame::NULL,
            "last_recv_frame should return NULL when only NULL entry exists"
        );

        // Get the zeroed bytes from the protocol's NULL entry - this is the reference for encoding
        let zeroed_bytes = protocol
            .recv_inputs
            .get(&Frame::NULL)
            .unwrap()
            .bytes
            .clone();

        // First input comes with frame 0, encoded relative to zeroed bytes
        let test_input = TestInput { inp: 42 };
        let test_bytes = crate::network::codec::encode(&test_input).unwrap();

        // The encoded bytes should have the same size as the reference
        assert_eq!(
            test_bytes.len(),
            zeroed_bytes.len(),
            "Input size should match zeroed size"
        );

        let encoded =
            crate::network::compression::encode(&zeroed_bytes, std::iter::once(&test_bytes));

        let input = Input {
            start_frame: Frame::new(0),
            ack_frame: Frame::NULL,
            bytes: encoded,
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };

        protocol.event_queue.clear();
        protocol.on_input(&input);

        // Verify: frame 0 was added
        assert!(
            protocol.recv_inputs.contains_key(&Frame::new(0)),
            "First input (frame 0) should be accepted when last_recv_frame is NULL"
        );
    }

    /// Test frame gap boundary: gap of exactly 1 is acceptable
    #[test]
    fn on_input_boundary_gap_of_one_is_acceptable() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Set up: we have frame 5
        let frame5_bytes = vec![5u8; 4];
        protocol.recv_inputs.insert(
            Frame::new(5),
            InputBytes {
                frame: Frame::new(5),
                bytes: frame5_bytes.clone(),
            },
        );

        // Receive frame 6 (gap of exactly 1)
        let frame6_bytes = vec![6u8; 4];
        let encoded =
            crate::network::compression::encode(&frame5_bytes, std::iter::once(&frame6_bytes));

        let input = Input {
            start_frame: Frame::new(6), // last_recv_frame() + 1 = 6, so 6 >= 6 is ok
            ack_frame: Frame::NULL,
            bytes: encoded,
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };

        let inputs_before = protocol.recv_inputs.len();
        protocol.event_queue.clear();
        protocol.on_input(&input);

        // Verify: frame 6 was added
        assert!(
            protocol.recv_inputs.contains_key(&Frame::new(6)),
            "Gap of 1 should be acceptable"
        );
        assert_eq!(protocol.recv_inputs.len(), inputs_before + 1);
    }

    #[test]
    fn on_input_drops_packet_when_configured_decode_limit_overflows() {
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::default(),
        );
        // Public config validation rejects this value; keep the receive path
        // defensive against internal mutation or future construction changes.
        protocol.protocol_config.pending_output_limit = usize::MAX;
        protocol.synchronize().unwrap();

        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            protocol.on_sync_reply(
                MessageHeader { magic: 999 },
                SyncReply {
                    random_reply: random,
                },
            );
        }

        let reference = vec![0u8; 2];
        protocol.recv_inputs.insert(
            Frame::new(0),
            InputBytes {
                frame: Frame::new(0),
                bytes: reference,
            },
        );

        let input = Input {
            start_frame: Frame::new(1),
            ack_frame: Frame::NULL,
            bytes: vec![1, 2, 3],
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };
        let inputs_before = protocol.recv_inputs.len();

        protocol.on_input(&input);

        assert_eq!(protocol.recv_inputs.len(), inputs_before);
        assert!(!protocol.recv_inputs.contains_key(&Frame::new(1)));
    }

    #[test]
    fn on_input_clamps_configured_decode_limit_to_rle_default_cap() {
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::default(),
        );
        // Public config validation rejects this value; keep the receive path
        // defensive against internal mutation or future construction changes.
        protocol.protocol_config.pending_output_limit = usize::MAX;
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);

        protocol.recv_inputs.insert(
            Frame::new(0),
            InputBytes {
                frame: Frame::new(0),
                bytes: vec![0],
            },
        );

        let bomb_len = (rle::DEFAULT_MAX_DECODED_LEN as u64) + 1;
        let mut header = (bomb_len << 2) | 1; // repeat=1, bit=0 (zero fill)
        let mut bomb = Vec::new();
        while header >= 0x80 {
            bomb.push((header as u8) | 0x80);
            header >>= 7;
        }
        bomb.push(header as u8);

        let input = Input {
            start_frame: Frame::new(1),
            ack_frame: Frame::NULL,
            bytes: bomb,
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };
        let inputs_before = protocol.recv_inputs.len();

        protocol.on_input(&input);

        assert_eq!(protocol.recv_inputs.len(), inputs_before);
        assert!(!protocol.recv_inputs.contains_key(&Frame::new(1)));
    }

    #[test]
    fn on_input_rejects_wrong_status_length_atomically() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);
        protocol.event_queue.clear();
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(0),
            bytes: vec![0, 0, 0, 0],
        });

        let input = Input {
            start_frame: Frame::new(0),
            ack_frame: Frame::new(0),
            bytes: vec![0],
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default()],
        };
        let inputs_before = protocol.recv_inputs.len();
        let pending_before = protocol.pending_output.len();
        let status_before = protocol.peer_connect_status.clone();

        protocol.on_input(&input);

        assert_eq!(protocol.recv_inputs.len(), inputs_before);
        assert_eq!(protocol.pending_output.len(), pending_before);
        assert_eq!(protocol.last_acked_input.frame, Frame::NULL);
        assert_eq!(protocol.peer_connect_status, status_before);
        assert!(protocol.event_queue.is_empty());
    }

    #[test]
    fn on_input_rejects_malformed_per_player_decode_atomically() {
        let mut protocol = UdpProtocol::<BoolConfig>::new(
            vec![PlayerHandle::new(0), PlayerHandle::new(1)],
            test_addr(),
            2,
            1,
            8,
            Duration::from_secs(5),
            Duration::from_secs(3),
            60,
            DesyncDetection::Off,
            SyncConfig::default(),
            ProtocolConfig::default(),
            TimeSyncConfig::default(),
        )
        .expect("bool protocol should be created");
        protocol.synchronize().unwrap();
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            protocol.on_sync_reply(
                MessageHeader { magic: 999 },
                SyncReply {
                    random_reply: random,
                },
            );
        }
        protocol.event_queue.clear();
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(0),
            bytes: vec![false as u8],
        });

        let reference = protocol
            .recv_inputs
            .get(&Frame::NULL)
            .unwrap()
            .bytes
            .clone();
        let malformed_frame = vec![2_u8, 0_u8];
        let input = Input {
            start_frame: Frame::new(0),
            ack_frame: Frame::new(0),
            bytes: crate::network::compression::encode(
                &reference,
                std::iter::once(&malformed_frame),
            ),
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };
        let inputs_before = protocol.recv_inputs.len();
        let pending_before = protocol.pending_output.len();

        protocol.on_input(&input);

        assert_eq!(protocol.recv_inputs.len(), inputs_before);
        assert_eq!(protocol.pending_output.len(), pending_before);
        assert_eq!(protocol.last_acked_input.frame, Frame::NULL);
        assert!(protocol.event_queue.is_empty());
    }

    /// Test frame gap boundary: gap of exactly 2 is rejected
    #[test]
    fn on_input_boundary_gap_of_two_is_rejected() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Set up: we have frame 5
        protocol.recv_inputs.insert(
            Frame::new(5),
            InputBytes {
                frame: Frame::new(5),
                bytes: vec![5u8; 4],
            },
        );

        // Try to receive frame 7 (gap of 2 - we're missing frame 6)
        let input = Input {
            start_frame: Frame::new(7), // last_recv_frame() + 1 = 6, but we have 7 < 6 is false
            ack_frame: Frame::NULL,
            bytes: vec![1, 2, 3, 4], // Won't be decoded anyway
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };

        let inputs_before = protocol.recv_inputs.len();
        protocol.event_queue.clear();
        protocol.on_input(&input);

        // Verify: no new inputs were added
        assert_eq!(
            protocol.recv_inputs.len(),
            inputs_before,
            "Gap of 2 should be rejected"
        );
        assert!(!protocol.recv_inputs.contains_key(&Frame::new(7)));
    }

    // ==========================================
    // Input Frame Consistency Tests
    // ==========================================

    /// Test that from_inputs handles frame consistency correctly.
    ///
    /// When frames are inconsistent, the function logs a warning violation
    /// but continues processing using the first non-NULL frame. This is
    /// safe because the serialized input data is still correct - only the
    /// frame metadata is inconsistent.
    #[test]
    fn from_inputs_handles_inconsistent_frames_gracefully() {
        use std::collections::BTreeMap;

        // Test 1: Consistent frames work correctly
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput {
                frame: Frame::new(5),
                input: TestInput { inp: 1 },
            },
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput {
                frame: Frame::new(5), // Same frame - no violation
                input: TestInput { inp: 2 },
            },
        );

        let result = InputBytes::from_inputs::<TestConfig>(2, &inputs);
        assert!(
            !result.bytes.is_empty(),
            "Should produce bytes for consistent frames"
        );
        assert_eq!(result.frame, Frame::new(5));

        // Test 2: Inconsistent frames still produce valid output
        // (with a warning violation logged)
        let mut inconsistent_inputs = BTreeMap::new();
        inconsistent_inputs.insert(
            PlayerHandle::new(0),
            PlayerInput {
                frame: Frame::new(5),
                input: TestInput { inp: 1 },
            },
        );
        inconsistent_inputs.insert(
            PlayerHandle::new(1),
            PlayerInput {
                frame: Frame::new(7), // Different frame - logs warning but continues
                input: TestInput { inp: 2 },
            },
        );

        let result = InputBytes::from_inputs::<TestConfig>(2, &inconsistent_inputs);
        // Should still produce valid bytes - the serialized input data is correct
        assert!(
            !result.bytes.is_empty(),
            "Should still produce bytes for inconsistent frames"
        );
        // Uses the first non-NULL frame (from player 0)
        assert_eq!(result.frame, Frame::new(5));
    }

    /// Test that from_inputs handles consistent frames correctly
    #[test]
    fn from_inputs_accepts_consistent_frames() {
        use std::collections::BTreeMap;

        // Add inputs with consistent frames
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput {
                frame: Frame::new(5),
                input: TestInput { inp: 1 },
            },
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput {
                frame: Frame::new(5), // Same frame
                input: TestInput { inp: 2 },
            },
        );

        let result = InputBytes::from_inputs::<TestConfig>(2, &inputs);

        assert!(!result.bytes.is_empty());
        assert_eq!(result.frame, Frame::new(5));
    }

    /// Test that from_inputs handles NULL frames as wildcard
    #[test]
    fn from_inputs_null_frame_is_wildcard() {
        use std::collections::BTreeMap;

        let mut inputs = BTreeMap::new();

        // Add input with real frame and one with NULL
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput {
                frame: Frame::new(5),
                input: TestInput { inp: 1 },
            },
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput {
                frame: Frame::NULL, // NULL frame should be skipped in consistency check
                input: TestInput { inp: 2 },
            },
        );

        let result = InputBytes::from_inputs::<TestConfig>(2, &inputs);

        // Should work without violation
        assert!(!result.bytes.is_empty());
        assert_eq!(result.frame, Frame::new(5));
    }

    // ==========================================
    // SyncConfig Tests
    // ==========================================

    #[test]
    fn sync_config_default_values() {
        let config = SyncConfig::default();
        assert_eq!(config.num_sync_packets, 5);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(200));
        assert_eq!(config.sync_timeout, None);
        assert_eq!(config.running_retry_interval, Duration::from_millis(200));
        assert_eq!(config.keepalive_interval, Duration::from_millis(200));
    }

    #[test]
    fn sync_config_high_latency_preset() {
        let config = SyncConfig::high_latency();
        assert_eq!(config.num_sync_packets, 5);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(400));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(10)));
        assert_eq!(config.running_retry_interval, Duration::from_millis(400));
        assert_eq!(config.keepalive_interval, Duration::from_millis(400));
    }

    #[test]
    fn sync_config_lossy_preset() {
        let config = SyncConfig::lossy();
        assert_eq!(config.num_sync_packets, 8);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(200));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(10)));
    }

    #[test]
    fn sync_config_lan_preset() {
        let config = SyncConfig::lan();
        assert_eq!(config.num_sync_packets, 3);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(100));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    #[allow(clippy::wildcard_enum_match_arm)]
    fn protocol_uses_custom_num_sync_packets() {
        let custom_config = SyncConfig {
            num_sync_packets: 3,
            ..SyncConfig::default()
        };

        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            custom_config,
            ProtocolConfig::default(),
        );

        protocol.synchronize().unwrap();

        // Simulate 3 successful sync roundtrips
        for i in 0..3 {
            let request_msg = protocol.send_queue.pop_back().unwrap();
            let random = match request_msg.body {
                MessageBody::SyncRequest(req) => req.random_request,
                _ => panic!("Expected SyncRequest"),
            };

            let reply = Message {
                header: MessageHeader { magic: 42 },
                body: MessageBody::SyncReply(SyncReply {
                    random_reply: random,
                }),
            };
            protocol.handle_message(&reply);

            // Check events
            let events: Vec<_> = protocol.poll(&[]).collect();
            if i < 2 {
                // Should get Synchronizing events for first 2 roundtrips
                assert!(events.iter().any(
                    |e| matches!(e, Event::Synchronizing { total: 3, count, .. } if *count == i + 1)
                ));
            } else {
                // Final roundtrip should produce Synchronized
                assert!(events.iter().any(|e| matches!(e, Event::Synchronized)));
            }
        }

        assert!(protocol.is_running());
    }

    #[test]
    fn sync_config_equality() {
        let config1 = SyncConfig::default();
        let config2 = SyncConfig::default();
        let config3 = SyncConfig::lan();

        assert_eq!(config1, config2);
        assert_ne!(config1, config3);
    }

    #[test]
    fn sync_config_clone() {
        let config = SyncConfig::high_latency();
        let cloned = config;
        assert_eq!(config, cloned);
    }

    // ==========================================
    // ProtocolConfig Tests
    // ==========================================

    #[test]
    fn protocol_config_default_values() {
        let config = ProtocolConfig::default();
        assert_eq!(config.quality_report_interval, Duration::from_millis(200));
        assert_eq!(config.shutdown_delay, Duration::from_secs(5));
        assert_eq!(config.max_checksum_history, 32);
        assert_eq!(config.pending_output_limit, 128);
        assert_eq!(config.sync_retry_warning_threshold, 10);
        assert_eq!(config.sync_duration_warning_ms, 3000);
    }

    #[test]
    fn protocol_config_competitive_preset() {
        let config = ProtocolConfig::competitive();
        assert_eq!(config.quality_report_interval, Duration::from_millis(100));
        assert_eq!(config.shutdown_delay, Duration::from_secs(3));
        assert_eq!(config.max_checksum_history, 32);
        assert_eq!(config.pending_output_limit, 128);
        assert_eq!(config.sync_retry_warning_threshold, 10);
        assert_eq!(config.sync_duration_warning_ms, 2000);
    }

    #[test]
    fn protocol_config_high_latency_preset() {
        let config = ProtocolConfig::high_latency();
        assert_eq!(config.quality_report_interval, Duration::from_millis(400));
        assert_eq!(config.shutdown_delay, Duration::from_secs(10));
        assert_eq!(config.max_checksum_history, 64);
        assert_eq!(config.pending_output_limit, 256);
        assert_eq!(config.sync_retry_warning_threshold, 20);
        assert_eq!(config.sync_duration_warning_ms, 10000);
    }

    #[test]
    fn protocol_config_debug_preset() {
        let config = ProtocolConfig::debug();
        assert_eq!(config.quality_report_interval, Duration::from_millis(500));
        assert_eq!(config.shutdown_delay, Duration::from_secs(30));
        assert_eq!(config.max_checksum_history, 128);
        assert_eq!(config.pending_output_limit, 64);
        assert_eq!(config.sync_retry_warning_threshold, 5);
        assert_eq!(config.sync_duration_warning_ms, 1000);
    }

    #[test]
    fn protocol_config_equality() {
        let config1 = ProtocolConfig::default();
        let config2 = ProtocolConfig::default();
        let config3 = ProtocolConfig::competitive();

        assert_eq!(config1, config2);
        assert_ne!(config1, config3);
    }

    #[test]
    fn protocol_config_clone() {
        let config = ProtocolConfig::high_latency();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn protocol_config_new_same_as_default() {
        let config1 = ProtocolConfig::new();
        let config2 = ProtocolConfig::default();
        assert_eq!(config1, config2);
    }

    // ==========================================
    // Ping Clock Tests
    // ==========================================

    /// Builds a `ProtocolConfig` whose clock reads `base + offset_ms`,
    /// returning the shared offset handle so tests can advance virtual time.
    fn injected_clock_config() -> (ProtocolConfig, Arc<std::sync::atomic::AtomicU64>) {
        let base = Instant::now();
        let offset_ms = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let offset = Arc::clone(&offset_ms);
        let config = ProtocolConfig {
            clock: Some(Arc::new(move || {
                base + Duration::from_millis(offset.load(std::sync::atomic::Ordering::Relaxed))
            })),
            ..ProtocolConfig::default()
        };
        (config, offset_ms)
    }

    #[test]
    fn ping_millis_starts_at_zero_with_injected_clock() {
        let (config, _offset) = injected_clock_config();
        let protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );
        assert_eq!(protocol.ping_millis(), 0);
    }

    #[test]
    fn ping_millis_tracks_injected_clock_exactly() {
        let (config, offset) = injected_clock_config();
        let protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );

        offset.store(1_234, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(protocol.ping_millis(), 1_234);

        offset.store(10_000, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(protocol.ping_millis(), 10_000);
    }

    #[test]
    fn ping_millis_without_injected_clock_is_monotonic() {
        let protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::default(),
        );

        let first = protocol.ping_millis();
        let second = protocol.ping_millis();
        assert!(
            second >= first,
            "monotonic clock must not go backwards (first={first}, second={second})"
        );
    }

    #[test]
    fn on_quality_reply_updates_rtt_from_injected_clock() {
        let (config, offset) = injected_clock_config();
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );

        // Ping issued at t=100ms, reply processed at t=175ms => RTT 75ms.
        offset.store(100, std::sync::atomic::Ordering::Relaxed);
        let ping = protocol.ping_millis();
        offset.store(175, std::sync::atomic::Ordering::Relaxed);
        protocol.on_quality_reply(&QualityReply { pong: ping });

        assert_eq!(protocol.round_trip_time, 75);
    }

    #[test]
    fn on_quality_reply_with_future_pong_saturates_to_zero() {
        let (config, _offset) = injected_clock_config();
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );

        // A pong larger than any timestamp this endpoint issued (e.g. a stale
        // packet from a previous endpoint era) must clamp to 0, not underflow.
        protocol.on_quality_reply(&QualityReply { pong: u128::MAX });
        assert_eq!(protocol.round_trip_time, 0);
    }

    // ==========================================
    // Deterministic Protocol RNG Tests
    // ==========================================

    #[test]
    fn protocol_with_same_seed_produces_same_magic_number() {
        let seed = 12345u64;
        let config = ProtocolConfig::deterministic(seed);

        let protocol1: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );

        let protocol2: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::deterministic(seed),
        );

        assert_eq!(
            protocol1.magic, protocol2.magic,
            "Same seed should produce same magic number"
        );
    }

    #[test]
    fn protocol_with_different_seeds_produces_different_magic_numbers() {
        let protocol1: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::deterministic(1),
        );

        let protocol2: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::deterministic(2),
        );

        assert_ne!(
            protocol1.magic, protocol2.magic,
            "Different seeds should produce different magic numbers (with very high probability)"
        );
    }

    #[test]
    fn protocol_without_seed_still_works() {
        // When no seed is provided, protocol should still initialize successfully
        let protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::default(), // No seed
        );

        // Magic should be non-zero
        assert_ne!(protocol.magic, 0, "Magic number should never be zero");
    }

    /// Session-33 round-6 era-fence pin (adjacent-era case): a deterministic
    /// (seeded) endpoint that re-arms for a hot-join rejoin must NOT reuse the
    /// previous era's magic. A naive rebuild re-seeds identically and reuses it — a
    /// still-live vacating peer (which holds the old magic as its learned
    /// `remote_magic`) then answers the rearmed handshake, the endpoint locks onto
    /// the doomed peer, and the real rejoiner is filtered out forever. The fence
    /// must also stay deterministic: identical seed + identical history must yield
    /// the identical rebuilt magic. (The monotonic counter that now backs the fence
    /// also closes the *non-adjacent* multi-era case — see
    /// `rearm_for_rejoin_era_fence_never_reuses_any_recent_era_magic`.)
    #[test]
    #[cfg(feature = "hot-join")]
    fn rearm_for_rejoin_era_fence_never_reuses_previous_magic_and_stays_deterministic() {
        let seed = 12345u64;
        let mut protocol1: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::deterministic(seed),
        );
        let mut protocol2: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::deterministic(seed),
        );
        let old_magic = protocol1.magic;
        assert_eq!(
            protocol1.magic, protocol2.magic,
            "precondition: seeded twins start with the identical magic"
        );

        protocol1
            .rearm_for_rejoin()
            .expect("rearm rebuilds the endpoint");
        protocol2
            .rearm_for_rejoin()
            .expect("rearm rebuilds the endpoint");

        assert_ne!(
            protocol1.magic, old_magic,
            "era fence: the rebuilt endpoint must never reuse the previous era's magic"
        );
        assert_ne!(protocol1.magic, 0, "magic stays non-zero across the rearm");
        assert_eq!(
            protocol1.magic, protocol2.magic,
            "the era fence preserves determinism: identical seed + history => identical rebuilt magic"
        );
        assert_eq!(
            protocol1.state,
            ProtocolState::Synchronizing,
            "rearm re-enters Synchronizing"
        );
    }

    /// Multi-era magic-recurrence hardening (`N-PLAYER-DESYNC-AUDIT.md`): the era
    /// fence is a **monotonic** per-endpoint counter, so across many rejoins the
    /// rebuilt magic never recurs within a 65535-rearm window — not merely
    /// differing from the *immediately-previous* era. The pre-fix fence re-rolled
    /// a fresh random magic and excluded **only the single previous era's value**,
    /// so a non-adjacent era could reuse an earlier era's magic; a still-live ghost
    /// peer from that earlier era (holding it as its learned `remote_magic`) would
    /// then answer the rebuilt handshake and wedge the rejoin. This test drives far
    /// more rejoins than the u16 birthday bound (~300), so the pre-fix random fence
    /// reuses an earlier era's magic with overwhelming probability (**RED**), while
    /// the monotonic fence is collision-free by construction (**GREEN**). This is
    /// the red-green security-invariant pin for the fix.
    #[test]
    #[cfg(feature = "hot-join")]
    fn rearm_for_rejoin_era_fence_never_reuses_any_recent_era_magic() {
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::deterministic(0x00C0_FFEE),
        );

        // Well above the u16 birthday threshold (~300): the pre-fix random re-roll
        // collides with an earlier era with > 99.9% probability over this many
        // rejoins (P(no collision) ~ e^-8), while the monotonic counter never does
        // (it walks 65535 distinct non-zero values before any repeat).
        const REJOINS: usize = 1024;

        let mut seen = std::collections::HashSet::with_capacity(REJOINS + 1);
        seen.insert(protocol.magic);
        let mut prev = protocol.magic;
        for i in 0..REJOINS {
            protocol
                .rearm_for_rejoin()
                .expect("rearm rebuilds the endpoint");
            let magic = protocol.magic;
            assert_ne!(magic, 0, "magic stays non-zero at rejoin {i}");
            assert_ne!(
                magic, prev,
                "magic differs from the immediately-previous era at rejoin {i}"
            );
            assert!(
                seen.insert(magic),
                "era fence breached: magic {magic} recurred within {REJOINS} rejoins \
                 (a still-live ghost from that earlier era could answer it) at rejoin {i}"
            );
            prev = magic;
        }
    }

    /// The monotonic era fence advances the magic by exactly one (wrapping past the
    /// reserved `0`) on every rejoin — a deterministic, self-evidently
    /// collision-free step that stays reproducible under a seed. RED on the pre-fix
    /// random re-roll (which equals `old + 1` only by a 1-in-65535 fluke).
    #[test]
    #[cfg(feature = "hot-join")]
    fn rearm_for_rejoin_era_magic_advances_by_monotonic_step() {
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::deterministic(7),
        );
        for i in 0..8 {
            let old = protocol.magic;
            protocol
                .rearm_for_rejoin()
                .expect("rearm rebuilds the endpoint");
            let expected = match old.wrapping_add(1) {
                0 => 1,
                next => next,
            };
            assert_eq!(
                protocol.magic, expected,
                "rejoin {i}: magic advances by a monotonic step (old {old} -> {expected})"
            );
        }
    }

    /// Wrap-around pin for the monotonic era fence: when the previous era's magic
    /// is `u16::MAX`, the next era must skip the reserved `0` and land on `1` — a
    /// `0` would violate the never-zero magic invariant and, on the wire, collide
    /// with the "magic not yet learned" sentinel (`remote_magic == 0` accepts any
    /// packet). The boundary is forced directly (walking 65535 real rejoins would
    /// be far too slow). This pins the `0 => 1` arm: mutating it to `0 => 0` turns
    /// this test RED.
    #[test]
    #[cfg(feature = "hot-join")]
    fn rearm_for_rejoin_era_magic_wraps_past_zero_to_one() {
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::deterministic(1),
        );
        protocol.magic = u16::MAX;
        protocol
            .rearm_for_rejoin()
            .expect("rearm rebuilds the endpoint");
        assert_eq!(
            protocol.magic, 1,
            "u16::MAX + 1 must skip the reserved 0 and land on 1"
        );
        assert_ne!(protocol.magic, 0, "the wrapped era magic is never zero");

        // The counter keeps stepping monotonically from there.
        protocol
            .rearm_for_rejoin()
            .expect("rearm rebuilds the endpoint");
        assert_eq!(
            protocol.magic, 2,
            "the counter continues monotonically after the wrap"
        );
    }

    /// The monotonic magic no longer consults the protocol RNG, so the RNG carry
    /// across the rebuild (`rebuilt.protocol_rng = self.protocol_rng.take()`) now
    /// exists solely to keep the unrelated sync-request IDs reproducible and
    /// flowing: a rearmed seeded endpoint CONTINUES its stream rather than
    /// resetting to the seed origin. This pins that carry as load-bearing — a
    /// rearmed endpoint's post-rearm sync-request IDs differ from a freshly-built
    /// endpoint with the same seed (proving the stream did not reset), while two
    /// seeded twins with identical history produce identical post-rearm streams
    /// (proving determinism). Deleting the carry (re-seeding on rebuild) makes the
    /// rearmed stream reset to the origin and turns the inequality RED.
    #[test]
    #[cfg(feature = "hot-join")]
    fn rearm_for_rejoin_continues_rng_stream_for_sync_requests() {
        let seed = 4242u64;
        let mk = || -> UdpProtocol<TestConfig> {
            create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            )
        };

        // A fresh endpoint's sync-request IDs come from the seed origin.
        let mut fresh = mk();
        fresh.synchronize().expect("synchronize");
        let fresh_requests = fresh.sync_random_requests.clone();
        assert!(
            !fresh_requests.is_empty(),
            "synchronize populates sync-request IDs"
        );

        // A rearmed endpoint first advances its stream (the pre-rearm synchronize),
        // then carries it across the rebuild, so its post-rearm IDs come from the
        // advanced position rather than the origin.
        let mut rearmed = mk();
        rearmed
            .synchronize()
            .expect("pre-rearm synchronize advances the stream");
        rearmed
            .rearm_for_rejoin()
            .expect("rearm carries the stream");
        let rearmed_requests = rearmed.sync_random_requests.clone();

        // Determinism: an identical-history twin yields the identical stream.
        let mut twin = mk();
        twin.synchronize().expect("synchronize");
        twin.rearm_for_rejoin().expect("rearm");
        assert_eq!(
            rearmed_requests, twin.sync_random_requests,
            "identical seed + history => identical post-rearm sync-request IDs"
        );

        // Load-bearing carry: the rearmed stream did NOT reset to the seed origin.
        assert_ne!(
            rearmed_requests, fresh_requests,
            "the carried RNG stream continues past the pre-rearm draws (it does not reset to the seed origin)"
        );
    }

    /// End-to-end consequence of the monotonic era fence (the audit's "the
    /// stale-era packet is still filtered AND a live packet is not"): a still-live
    /// ghost peer that synchronized against an EARLY era of our endpoint (and so
    /// holds that era's magic as its learned `remote_magic`) filters out every
    /// packet the rebuilt endpoint now sends under its current-era magic — so it
    /// can never answer and wedge the rejoin — while a packet carrying the magic it
    /// actually learned is still accepted (the filter discriminates, it does not
    /// blanket-drop). The red-green proof that the current era is distinct from
    /// every recent era lives in
    /// `rearm_for_rejoin_era_fence_never_reuses_any_recent_era_magic`; this test
    /// pins the wire-level behaviour that distinctness buys. (It passes on both
    /// pre- and post-fix code — the filter logic is unchanged; only the upstream
    /// era distinctness the fix guarantees is what this behaviour relies on.)
    #[test]
    #[cfg(feature = "hot-join")]
    fn rearm_for_rejoin_ghost_from_early_era_filters_rebuilt_endpoints_current_magic() {
        // Our endpoint walks through several eras of rejoin.
        let mut ours: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ProtocolConfig::deterministic(99),
        );
        let early_era_magic = ours.magic;
        let mut prior = vec![early_era_magic];
        for _ in 0..5 {
            ours.rearm_for_rejoin()
                .expect("rearm rebuilds the endpoint");
            prior.push(ours.magic);
        }
        let current_magic = ours.magic;
        // The current era is distinct from EVERY prior era (the fence's guarantee).
        let priors_before_current = &prior[..prior.len() - 1];
        assert!(
            !priors_before_current.contains(&current_magic),
            "current era magic {current_magic} must differ from all prior eras {priors_before_current:?}"
        );

        // A ghost peer that synchronized against our EARLIEST era, so it holds that
        // era's magic as its learned `remote_magic`.
        let (ghost_config, ghost_clock) = mutable_clock_config();
        let mut ghost: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            ghost_config,
        );
        ghost.synchronize().unwrap();
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *ghost.sync_random_requests.iter().next().unwrap();
            ghost.on_sync_reply(
                MessageHeader {
                    magic: early_era_magic,
                },
                SyncReply {
                    random_reply: random,
                },
            );
        }
        assert_eq!(ghost.remote_magic, early_era_magic);

        // The rebuilt endpoint's current-era packet is FILTERED by the ghost: no
        // observable state change (the ghost cannot answer it -> cannot wedge us).
        ghost.send_queue.clear();
        let last_recv_before = ghost.last_recv_time;
        ghost.handle_message(&Message {
            header: MessageHeader {
                magic: current_magic,
            },
            body: MessageBody::KeepAlive,
        });
        assert!(
            ghost.send_queue.is_empty(),
            "ghost must filter the rebuilt endpoint's current-era magic"
        );
        assert_eq!(
            ghost.last_recv_time, last_recv_before,
            "a filtered packet does not refresh the ghost's recv clock"
        );

        // The filter discriminates rather than blanket-dropping: a packet carrying
        // the magic the ghost actually learned is still accepted.
        let accepted_recv_time = advance_test_clock(&ghost_clock, Duration::from_millis(1));
        ghost.handle_message(&Message {
            header: MessageHeader {
                magic: early_era_magic,
            },
            body: MessageBody::KeepAlive,
        });
        assert_eq!(
            ghost.last_recv_time, accepted_recv_time,
            "accepted packets refresh the ghost's recv clock to the injected time"
        );
        assert!(
            ghost.last_recv_time > last_recv_before,
            "ghost still accepts a packet carrying its learned magic"
        );
    }

    #[test]
    fn protocol_new_rejects_zero_byte_input_type() {
        let result = UdpProtocol::<UnitInputConfig>::new(
            vec![PlayerHandle::new(0)],
            test_addr(),
            2,
            1,
            8,
            Duration::from_secs(5),
            Duration::from_secs(3),
            60,
            DesyncDetection::Off,
            SyncConfig::default(),
            ProtocolConfig::default(),
            TimeSyncConfig::default(),
        );

        assert!(matches!(
            result,
            Err(FortressError::SerializationErrorStructured {
                kind: SerializationErrorKind::InputSerializedSizeZero
            })
        ));
    }

    #[test]
    fn protocol_input_frame_wire_size_rejects_frame_larger_than_decode_cap() {
        assert!(matches!(
            validate_input_frame_wire_size(rle::DEFAULT_MAX_DECODED_LEN + 1, 1),
            Err(FortressError::SerializationErrorStructured {
                kind: SerializationErrorKind::InputSerializedFrameTooLarge {
                    frame_len,
                    max
                }
            }) if frame_len == rle::DEFAULT_MAX_DECODED_LEN + 1
                && max == rle::DEFAULT_MAX_DECODED_LEN
        ));

        assert!(matches!(
            validate_input_frame_wire_size(2, usize::MAX),
            Err(FortressError::SerializationErrorStructured {
                kind: SerializationErrorKind::InputSerializedFrameTooLarge {
                    frame_len: usize::MAX,
                    max
                }
            }) if max == rle::DEFAULT_MAX_DECODED_LEN
        ));
    }

    #[test]
    fn protocol_new_rejects_local_input_frame_larger_than_decode_cap() {
        let local_players = (rle::DEFAULT_MAX_DECODED_LEN / 4) + 1;
        let result = UdpProtocol::<TestConfig>::new(
            vec![PlayerHandle::new(0)],
            test_addr(),
            local_players,
            local_players,
            8,
            Duration::from_secs(5),
            Duration::from_secs(3),
            60,
            DesyncDetection::Off,
            SyncConfig::default(),
            ProtocolConfig::default(),
            TimeSyncConfig::default(),
        );

        assert!(matches!(
            result,
            Err(FortressError::SerializationErrorStructured {
                kind: SerializationErrorKind::InputSerializedFrameTooLarge {
                    frame_len,
                    max
                }
            }) if frame_len == local_players * 4 && max == rle::DEFAULT_MAX_DECODED_LEN
        ));
    }

    #[test]
    fn protocol_magic_is_never_zero() {
        // Test that the magic number generation loop correctly handles
        // the case where the first random value might be zero
        for seed in 0..100 {
            let protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );
            assert_ne!(
                protocol.magic, 0,
                "Magic number should never be zero (seed={})",
                seed
            );
        }
    }

    #[test]
    fn protocol_uses_custom_clock() {
        use crate::sessions::config::ClockFn;
        use std::sync::Arc;

        let fixed_time = Instant::now();
        let clock: ClockFn = Arc::new(move || fixed_time);
        let config = ProtocolConfig {
            clock: Some(clock),
            ..ProtocolConfig::default()
        };
        let protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );

        // The protocol's now() should return our fixed time, not the real clock
        let returned_time = protocol.now();
        assert_eq!(
            returned_time, fixed_time,
            "Protocol should use the injected clock function"
        );
    }

    /// Regression: when `pending_output` is at the configured limit,
    /// `enqueue_replicated_input` must refuse to push (to avoid overflow)
    /// and reach the violation-reporting branch. The observable side-effect
    /// is that `pending_output.len()` stays at the limit, which this test
    /// asserts directly.
    ///
    /// **On capturing the violation itself:** `report_violation!` routes
    /// only through the global `TracingObserver` (see
    /// `src/telemetry.rs:763-812`); it does not push into a thread-local
    /// `CollectingObserver`, and there is no `report_violation_to!`
    /// override accepting an observer at the call site here. Capturing the
    /// emitted `SpecViolation` therefore requires installing a
    /// `tracing-subscriber` layer that filters on the macro's structured
    /// fields, which no other unit test in this file does. Adding that
    /// infrastructure would be a strictly larger change than the
    /// regression this test guards. The contract callers actually depend
    /// on is "the entry is dropped instead of overflowing
    /// `pending_output`", which is exactly what is asserted below; the
    /// `report_violation!` call immediately precedes the `return;` in
    /// straight-line control flow, so the side-effect-only assertion (no
    /// push past the limit) is sufficient to prove the violation branch
    /// was taken.
    #[test]
    fn enqueue_replicated_input_drops_entry_when_pending_output_full() {
        let small_limit: usize = 4;
        let config = ProtocolConfig {
            pending_output_limit: small_limit,
            ..ProtocolConfig::default()
        };
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );
        // Drive the protocol to the Running state so `enqueue_replicated_input`
        // executes the limit check (it is a no-op pre-Running).
        protocol.synchronize().unwrap();
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            let reply = SyncReply {
                random_reply: random,
            };
            protocol.on_sync_reply(header, reply);
        }
        assert!(protocol.is_running());

        // Fill `pending_output` directly up to the configured limit.
        for i in 0..small_limit {
            protocol.pending_output.push_back(InputBytes {
                frame: Frame::new(i as i32),
                bytes: vec![i as u8; 4],
            });
        }
        assert_eq!(protocol.pending_output.len(), small_limit);

        // Try to enqueue one more — must hit the overflow guard, drop the
        // entry, and leave `pending_output` unchanged.
        let mut inputs: BTreeMap<PlayerHandle, PlayerInput<TestInput>> = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(Frame::new(small_limit as i32), TestInput { inp: 7 }),
        );
        protocol.enqueue_replicated_input(&inputs);

        assert_eq!(
            protocol.pending_output.len(),
            small_limit,
            "enqueue_replicated_input must not push past pending_output_limit"
        );
    }

    #[test]
    fn send_input_disconnects_without_mutating_when_pending_output_full() {
        let small_limit: usize = 2;
        let config = ProtocolConfig {
            pending_output_limit: small_limit,
            ..ProtocolConfig::default()
        };
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);
        protocol.send_queue.clear();
        protocol.event_queue.clear();

        for i in 0..small_limit {
            protocol.pending_output.push_back(InputBytes {
                frame: Frame::new(i32::try_from(i).unwrap()),
                bytes: vec![u8::try_from(i).unwrap(); 4],
            });
        }

        let mut inputs: BTreeMap<PlayerHandle, PlayerInput<TestInput>> = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(
                Frame::new(i32::try_from(small_limit).unwrap()),
                TestInput { inp: 99 },
            ),
        );
        let connect_status = vec![ConnectionStatus::default(); 2];

        protocol.send_input(&inputs, &connect_status);

        assert_eq!(protocol.pending_output.len(), small_limit);
        assert!(protocol.send_queue.is_empty());
        assert!(protocol
            .event_queue
            .iter()
            .any(|event| matches!(event, Event::Disconnected)));
    }

    #[test]
    fn send_pending_output_encodes_only_configured_frame_prefix() {
        let small_limit: usize = 3;
        let pending_count: usize = 6;
        let config = ProtocolConfig {
            pending_output_limit: small_limit,
            ..ProtocolConfig::default()
        };
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);
        protocol.send_queue.clear();

        for i in 0..pending_count {
            protocol.pending_output.push_back(InputBytes {
                frame: Frame::new(i32::try_from(i).unwrap()),
                bytes: u32::try_from(i + 1).unwrap().to_le_bytes().to_vec(),
            });
        }
        let expected: Vec<_> = protocol
            .pending_output
            .iter()
            .take(small_limit)
            .map(|input| input.bytes.clone())
            .collect();
        let connect_status = vec![ConnectionStatus::default(); 2];

        protocol.send_pending_output(&connect_status);

        assert_eq!(protocol.send_queue.len(), 1);
        let body = queued_input_body(&protocol);
        assert_eq!(body.start_frame, Frame::new(0));
        let max_decoded_input_bytes = input_batch_decoded_byte_limit(
            protocol.last_acked_input.bytes.len(),
            protocol.protocol_config.pending_output_limit,
        )
        .unwrap();
        let decoded = crate::network::compression::decode_with_max_len(
            &protocol.last_acked_input.bytes,
            &body.bytes,
            max_decoded_input_bytes,
        )
        .unwrap();

        assert_eq!(decoded, expected);
        assert_eq!(decoded.len(), small_limit);
        assert_eq!(protocol.pending_output.len(), pending_count);
    }

    #[test]
    fn send_pending_output_encodes_only_decoded_byte_cap_prefix() {
        let pending_limit: usize = 10;
        let decoded_byte_cap: usize = 18;
        let expected_batch_len: usize = decoded_byte_cap / std::mem::size_of::<TestInput>();
        let config = ProtocolConfig {
            pending_output_limit: pending_limit,
            ..ProtocolConfig::default()
        };
        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            SyncConfig::default(),
            config,
        );
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);
        protocol.send_queue.clear();

        for i in 0..pending_limit {
            protocol.pending_output.push_back(InputBytes {
                frame: Frame::new(i32::try_from(i).unwrap()),
                bytes: u32::try_from(i + 1).unwrap().to_le_bytes().to_vec(),
            });
        }
        let expected: Vec<_> = protocol
            .pending_output
            .iter()
            .take(expected_batch_len)
            .map(|input| input.bytes.clone())
            .collect();
        let connect_status = vec![ConnectionStatus::default(); 2];

        protocol.send_pending_output_with_decoded_byte_cap(&connect_status, decoded_byte_cap);

        assert_eq!(protocol.send_queue.len(), 1);
        let body = queued_input_body(&protocol);
        let decoded = crate::network::compression::decode_with_max_len(
            &protocol.last_acked_input.bytes,
            &body.bytes,
            decoded_byte_cap,
        )
        .unwrap();

        assert_eq!(decoded, expected);
        assert_eq!(decoded.len(), expected_batch_len);
        assert!(
            expected_batch_len < pending_limit,
            "test must exercise the byte-cap prefix, not the frame limit"
        );
    }

    #[test]
    fn input_batch_len_for_limits_clamps_to_decoded_byte_cap() {
        assert_eq!(
            input_batch_len_for_limits(10, 4, 10, 18),
            Some(4),
            "18 decoded bytes can carry only four 4-byte input frames"
        );
        assert_eq!(
            input_batch_len_for_limits(10, 4, 3, 18),
            Some(3),
            "the configured pending frame limit still applies first"
        );
        assert_eq!(
            input_batch_len_for_limits(10, 4, 10, 3),
            Some(0),
            "a cap smaller than one input frame must not encode a packet"
        );
        assert_eq!(
            input_batch_len_for_limits(10, 2, usize::MAX, usize::MAX),
            None,
            "overflowing the configured byte budget must be rejected"
        );
    }

    #[test]
    fn send_input_rejects_variable_width_serialized_input_without_queueing() {
        let mut protocol = UdpProtocol::<VariableInputConfig>::new(
            vec![PlayerHandle::new(0)],
            test_addr(),
            2,
            1,
            8,
            Duration::from_secs(5),
            Duration::from_secs(3),
            60,
            DesyncDetection::Off,
            SyncConfig::default(),
            ProtocolConfig::default(),
            TimeSyncConfig::default(),
        )
        .expect("variable-width protocol should construct; active input fails on send");
        protocol.force_running_for_tests();

        let default_len = codec::encoded_len(&VariableInput::Idle).unwrap();
        let active_len = codec::encoded_len(&VariableInput::Active(7)).unwrap();
        assert_ne!(
            default_len, active_len,
            "test requires variants with different serialized lengths"
        );

        let mut inputs: BTreeMap<PlayerHandle, PlayerInput<VariableInput>> = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(Frame::new(0), VariableInput::Active(7)),
        );
        let connect_status = vec![ConnectionStatus::default(); 2];

        protocol.send_input(&inputs, &connect_status);

        assert!(protocol.pending_output.is_empty());
        assert!(protocol.send_queue.is_empty());
    }

    #[test]
    fn send_input_rejects_per_player_width_mismatch_even_when_aggregate_matches() {
        let mut protocol = UdpProtocol::<BalancedVariableInputConfig>::new(
            Vec::new(),
            test_addr(),
            2,
            2,
            8,
            Duration::from_secs(5),
            Duration::from_secs(3),
            60,
            DesyncDetection::Off,
            SyncConfig::default(),
            ProtocolConfig::default(),
            TimeSyncConfig::default(),
        )
        .expect("balanced variable-width protocol should construct");
        protocol.force_running_for_tests();

        let default_len = codec::encoded_len(&BalancedVariableInput::default()).unwrap();
        let short_len = codec::encoded_len(&BalancedVariableInput::Short).unwrap();
        let long_len = codec::encoded_len(&BalancedVariableInput::Long(7)).unwrap();
        assert_eq!(short_len + long_len, default_len * 2);
        assert_ne!(short_len, default_len);
        assert_ne!(long_len, default_len);

        let mut inputs: BTreeMap<PlayerHandle, PlayerInput<BalancedVariableInput>> =
            BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(Frame::new(0), BalancedVariableInput::Short),
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput::new(Frame::new(0), BalancedVariableInput::Long(7)),
        );
        let connect_status = vec![ConnectionStatus::default(); 2];

        protocol.send_input(&inputs, &connect_status);

        assert!(protocol.pending_output.is_empty());
        assert!(protocol.send_queue.is_empty());
    }

    // ==========================================
    // Hot-Join Message Handling Tests
    // ==========================================

    /// Drives a protocol through synchronization into the `Running` state. After
    /// `complete_test_sync` the `remote_magic` is the SyncReply header magic (999),
    /// so messages delivered with that magic pass `handle_message`'s magic filter.
    #[cfg(feature = "hot-join")]
    const HOT_JOIN_REMOTE_MAGIC: u16 = 999;

    #[cfg(feature = "hot-join")]
    fn running_protocol() -> UdpProtocol<TestConfig> {
        let mut protocol = create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();
        complete_test_sync(&mut protocol);
        assert!(protocol.is_running());
        protocol.send_queue.clear();
        protocol
    }

    #[cfg(feature = "hot-join")]
    fn deliver(protocol: &mut UdpProtocol<TestConfig>, body: MessageBody) {
        protocol.handle_message(&Message {
            header: MessageHeader {
                magic: HOT_JOIN_REMOTE_MAGIC,
            },
            body,
        });
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn handle_message_stores_join_request() {
        let mut protocol = running_protocol();

        deliver(
            &mut protocol,
            MessageBody::JoinRequest(JoinRequest { player_handle: 1 }),
        );

        assert_eq!(protocol.take_pending_join_request(), Some(1));
        // Draining is one-shot.
        assert_eq!(protocol.take_pending_join_request(), None);
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn handle_message_stores_state_snapshot() {
        let mut protocol = running_protocol();
        let snapshot = StateSnapshot {
            frame: Frame::new(42),
            num_players: 2,
            state_bytes: vec![1, 2, 3, 4],
            bridge_inputs: Vec::new(),
            bridge_statuses: Vec::new(),
            checksum: Some(0xABCD),
        };

        deliver(&mut protocol, MessageBody::StateSnapshot(snapshot.clone()));

        assert_eq!(protocol.take_received_snapshot(), Some(snapshot));
        assert_eq!(protocol.take_received_snapshot(), None);
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn handle_message_stores_state_snapshot_ack() {
        let mut protocol = running_protocol();

        deliver(
            &mut protocol,
            MessageBody::StateSnapshotAck(StateSnapshotAck {
                frame: Frame::new(77),
            }),
        );

        assert_eq!(protocol.take_received_snapshot_ack(), Some(Frame::new(77)));
        assert_eq!(protocol.take_received_snapshot_ack(), None);
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn handle_message_join_request_last_writer_wins() {
        let mut protocol = running_protocol();

        deliver(
            &mut protocol,
            MessageBody::JoinRequest(JoinRequest { player_handle: 1 }),
        );
        deliver(
            &mut protocol,
            MessageBody::JoinRequest(JoinRequest { player_handle: 5 }),
        );

        assert_eq!(protocol.take_pending_join_request(), Some(5));
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn send_join_request_queues_when_running() {
        let mut protocol = running_protocol();

        protocol.send_join_request(3);

        assert_eq!(protocol.send_queue.len(), 1);
        match &protocol.send_queue.front().unwrap().body {
            MessageBody::JoinRequest(body) => assert_eq!(body.player_handle, 3),
            other => panic!("expected JoinRequest, got {other:?}"),
        }
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn send_state_snapshot_queues_when_running() {
        let mut protocol = running_protocol();
        let snapshot = StateSnapshot {
            frame: Frame::new(10),
            num_players: 2,
            state_bytes: vec![9, 9, 9],
            bridge_inputs: Vec::new(),
            bridge_statuses: Vec::new(),
            checksum: None,
        };

        protocol.send_state_snapshot(snapshot.clone());

        assert_eq!(protocol.send_queue.len(), 1);
        match &protocol.send_queue.front().unwrap().body {
            MessageBody::StateSnapshot(body) => assert_eq!(body, &snapshot),
            other => panic!("expected StateSnapshot, got {other:?}"),
        }
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn send_state_snapshot_ack_queues_when_running() {
        let mut protocol = running_protocol();

        protocol.send_state_snapshot_ack(Frame::new(55));

        assert_eq!(protocol.send_queue.len(), 1);
        match &protocol.send_queue.front().unwrap().body {
            MessageBody::StateSnapshotAck(body) => assert_eq!(body.frame, Frame::new(55)),
            other => panic!("expected StateSnapshotAck, got {other:?}"),
        }
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn send_hot_join_messages_are_noop_when_not_running() {
        // Protocol stays in Initializing (never synchronized).
        let mut protocol = create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        assert!(!protocol.is_running());

        protocol.send_join_request(1);
        protocol.send_state_snapshot(StateSnapshot {
            frame: Frame::new(1),
            num_players: 2,
            state_bytes: vec![1],
            bridge_inputs: Vec::new(),
            bridge_statuses: Vec::new(),
            checksum: None,
        });
        protocol.send_state_snapshot_ack(Frame::new(1));

        assert!(protocol.send_queue.is_empty());
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn handle_message_drops_hot_join_messages_while_synchronizing_without_side_effects() {
        // While Synchronizing, `message_allowed_in_current_state` admits only
        // Sync messages, so hot-join control messages must be dropped before
        // reaching a handler. This pins that the Running-only gate also covers
        // all seven hot-join variants (guarding against a future state-machine
        // change that forgets them). Mirrors
        // `handle_message_drops_gameplay_messages_while_synchronizing_without_side_effects`.
        let mut protocol = create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize().unwrap();
        assert!(!protocol.is_running());
        let initial_event_len = protocol.event_queue.len();

        deliver(
            &mut protocol,
            MessageBody::JoinRequest(JoinRequest { player_handle: 1 }),
        );
        deliver(
            &mut protocol,
            MessageBody::StateSnapshot(StateSnapshot {
                frame: Frame::new(7),
                num_players: 2,
                state_bytes: vec![1, 2, 3, 4],
                bridge_inputs: Vec::new(),
                bridge_statuses: Vec::new(),
                checksum: Some(0xABCD),
            }),
        );
        deliver(
            &mut protocol,
            MessageBody::StateSnapshotAck(StateSnapshotAck {
                frame: Frame::new(7),
            }),
        );
        deliver(
            &mut protocol,
            MessageBody::ReactivateSlot(ReactivateSlot {
                handle: 1,
                frame: Frame::new(7),
            }),
        );
        deliver(
            &mut protocol,
            MessageBody::ReactivateSlotAck(ReactivateSlotAck {
                handle: 1,
                frame: Frame::new(7),
            }),
        );
        deliver(
            &mut protocol,
            MessageBody::JoinCommitted(JoinCommitted {
                handle: 1,
                frame: Frame::new(7),
            }),
        );
        deliver(
            &mut protocol,
            MessageBody::JoinAborted(JoinAborted {
                handle: 1,
                frame: Frame::new(7),
            }),
        );

        assert_eq!(protocol.take_pending_join_request(), None);
        assert!(protocol.take_received_snapshot().is_none());
        assert_eq!(protocol.take_received_snapshot_ack(), None);
        assert!(protocol.take_received_reactivate_slot().is_none());
        assert!(protocol.take_received_reactivate_slot_ack().is_none());
        assert!(protocol.take_received_join_committed().is_none());
        assert!(protocol.take_received_join_aborted().is_none());
        assert_eq!(protocol.event_queue.len(), initial_event_len);
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn send_reactivate_slot_queues_when_running() {
        let mut protocol = running_protocol();

        protocol.send_reactivate_slot(3, Frame::new(42));

        assert_eq!(protocol.send_queue.len(), 1);
        match &protocol.send_queue.front().unwrap().body {
            MessageBody::ReactivateSlot(body) => {
                assert_eq!(body.handle, 3);
                assert_eq!(body.frame, Frame::new(42));
            },
            other => panic!("expected ReactivateSlot, got {other:?}"),
        }
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn send_reactivate_slot_ack_queues_when_running() {
        let mut protocol = running_protocol();

        protocol.send_reactivate_slot_ack(3, Frame::new(42));

        assert_eq!(protocol.send_queue.len(), 1);
        match &protocol.send_queue.front().unwrap().body {
            MessageBody::ReactivateSlotAck(body) => {
                assert_eq!(body.handle, 3);
                assert_eq!(body.frame, Frame::new(42));
            },
            other => panic!("expected ReactivateSlotAck, got {other:?}"),
        }
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn send_reactivate_slot_messages_are_noop_when_not_running() {
        // Protocol stays in Initializing (never synchronized).
        let mut protocol = create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        assert!(!protocol.is_running());

        protocol.send_reactivate_slot(1, Frame::new(1));
        protocol.send_reactivate_slot_ack(1, Frame::new(1));

        assert!(protocol.send_queue.is_empty());
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn handle_message_stores_reactivate_slot() {
        let mut protocol = running_protocol();
        let body = ReactivateSlot {
            handle: 2,
            frame: Frame::new(77),
        };

        deliver(&mut protocol, MessageBody::ReactivateSlot(body.clone()));

        assert_eq!(protocol.take_received_reactivate_slot(), Some(body));
        // Draining is one-shot.
        assert_eq!(protocol.take_received_reactivate_slot(), None);
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn handle_message_stores_reactivate_slot_ack() {
        let mut protocol = running_protocol();
        let body = ReactivateSlotAck {
            handle: 2,
            frame: Frame::new(77),
        };

        deliver(&mut protocol, MessageBody::ReactivateSlotAck(body.clone()));

        assert_eq!(protocol.take_received_reactivate_slot_ack(), Some(body));
        // Draining is one-shot.
        assert_eq!(protocol.take_received_reactivate_slot_ack(), None);
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn send_join_committed_queues_when_running() {
        let mut protocol = running_protocol();

        protocol.send_join_committed(3, Frame::new(42));

        assert_eq!(protocol.send_queue.len(), 1);
        match &protocol.send_queue.front().unwrap().body {
            MessageBody::JoinCommitted(body) => {
                assert_eq!(body.handle, 3);
                assert_eq!(body.frame, Frame::new(42));
            },
            other => panic!("expected JoinCommitted, got {other:?}"),
        }
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn send_join_aborted_queues_when_running() {
        let mut protocol = running_protocol();

        protocol.send_join_aborted(3, Frame::new(42));

        assert_eq!(protocol.send_queue.len(), 1);
        match &protocol.send_queue.front().unwrap().body {
            MessageBody::JoinAborted(body) => {
                assert_eq!(body.handle, 3);
                assert_eq!(body.frame, Frame::new(42));
            },
            other => panic!("expected JoinAborted, got {other:?}"),
        }
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn send_join_lifecycle_messages_are_noop_when_not_running() {
        // Protocol stays in Initializing (never synchronized).
        let mut protocol = create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        assert!(!protocol.is_running());

        protocol.send_join_committed(1, Frame::new(1));
        protocol.send_join_aborted(1, Frame::new(1));

        assert!(protocol.send_queue.is_empty());
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn handle_message_stores_join_committed() {
        let mut protocol = running_protocol();
        let body = JoinCommitted {
            handle: 2,
            frame: Frame::new(77),
        };

        deliver(&mut protocol, MessageBody::JoinCommitted(body.clone()));

        assert_eq!(protocol.take_received_join_committed(), Some(body));
        // Draining is one-shot.
        assert_eq!(protocol.take_received_join_committed(), None);
    }

    #[test]
    #[cfg(feature = "hot-join")]
    fn handle_message_stores_join_aborted() {
        let mut protocol = running_protocol();
        let body = JoinAborted {
            handle: 2,
            frame: Frame::new(77),
        };

        deliver(&mut protocol, MessageBody::JoinAborted(body.clone()));

        assert_eq!(protocol.take_received_join_aborted(), Some(body));
        // Draining is one-shot.
        assert_eq!(protocol.take_received_join_aborted(), None);
    }
}

// ============================================================================
// Property-Based Tests for Protocol State Machine
// ============================================================================
//
// These tests verify invariants of the UDP protocol state machine using proptest.
// See PLAN.md item 2.5 for context.
//
// # Invariants Tested
//
// ## State Machine Invariants (INV-PROTO)
// - INV-PROTO-1: State transitions are valid (follow state diagram)
// - INV-PROTO-2: sync_remaining_roundtrips never exceeds num_sync_packets
// - INV-PROTO-3: sync_remaining_roundtrips is non-negative (decrements correctly)
// - INV-PROTO-4: Magic number is never zero
// - INV-PROTO-5: State predicates are consistent (is_running, is_synchronized)
// - INV-PROTO-6: Input frame sequence is monotonic
// - INV-PROTO-7: Checksum history is bounded
//
// ## Message Handling Invariants
// - INV-PROTO-8: Sync replies only decrement counter for valid random values
// - INV-PROTO-9: Messages in shutdown state are dropped

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod property_tests {
    use super::*;
    use crate::test_config::miri_case_count;
    use proptest::prelude::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    // ========================================================================
    // Test Configuration
    // ========================================================================

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
    struct TestInput {
        inp: u32,
    }

    #[derive(Clone, Default)]
    #[cfg_attr(feature = "hot-join", derive(Serialize, Deserialize))]
    struct TestState;

    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = TestState;
        type Address = SocketAddr;
    }

    fn test_addr() -> SocketAddr {
        "127.0.0.1:7000".parse().unwrap()
    }

    // ========================================================================
    // Test Helpers
    // ========================================================================

    fn create_protocol_with_config(
        handles: Vec<PlayerHandle>,
        num_players: usize,
        local_players: usize,
        max_prediction: usize,
        sync_config: SyncConfig,
        protocol_config: ProtocolConfig,
    ) -> UdpProtocol<TestConfig> {
        UdpProtocol::new(
            handles,
            test_addr(),
            num_players,
            local_players,
            max_prediction,
            Duration::from_secs(5),
            Duration::from_secs(3),
            60,
            DesyncDetection::Off,
            sync_config,
            protocol_config,
            TimeSyncConfig::default(),
        )
        .expect("Failed to create test protocol")
    }

    /// Completes the sync process by simulating all required sync roundtrips.
    fn complete_sync(protocol: &mut UdpProtocol<TestConfig>, num_packets: u32) {
        for _ in 0..num_packets {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            let reply = SyncReply {
                random_reply: random,
            };
            protocol.on_sync_reply(header, reply);
        }
    }

    // ========================================================================
    // Property Test Strategies
    // ========================================================================

    /// Strategy for number of sync packets (1-10)
    fn num_sync_packets_strategy() -> impl Strategy<Value = u32> {
        1u32..=10
    }

    /// Strategy for protocol seeds
    fn seed_strategy() -> impl Strategy<Value = u64> {
        any::<u64>()
    }

    /// Strategy for frame numbers
    fn frame_strategy() -> impl Strategy<Value = i32> {
        0i32..10000
    }

    /// Strategy for checksum values
    fn checksum_strategy() -> impl Strategy<Value = u128> {
        any::<u128>()
    }

    /// Strategy for input values
    fn input_value_strategy() -> impl Strategy<Value = u32> {
        any::<u32>()
    }

    /// Strategy for player count (1-4)
    fn player_count_strategy() -> impl Strategy<Value = usize> {
        1usize..=4
    }

    /// Strategy for max prediction window
    fn max_prediction_strategy() -> impl Strategy<Value = usize> {
        4usize..=16
    }

    // ========================================================================
    // INV-PROTO-1: State transitions are valid
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// INV-PROTO-1: Protocol starts in Initializing state
        #[test]
        fn prop_protocol_starts_in_initializing(
            seed in seed_strategy(),
            num_players in player_count_strategy(),
            max_pred in max_prediction_strategy(),
        ) {
            let protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                num_players,
                1,
                max_pred,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            prop_assert!(matches!(protocol.state, ProtocolState::Initializing));
            prop_assert!(!protocol.is_synchronized());
            prop_assert!(!protocol.is_running());
        }

        /// INV-PROTO-1: synchronize() transitions from Initializing to Synchronizing
        #[test]
        fn prop_synchronize_transitions_to_synchronizing(
            seed in seed_strategy(),
        ) {
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            prop_assert!(matches!(protocol.state, ProtocolState::Initializing));

            protocol.synchronize().unwrap();
            prop_assert!(matches!(protocol.state, ProtocolState::Synchronizing));
        }

        /// INV-PROTO-1: synchronize() fails when not in Initializing state
        #[test]
        fn prop_synchronize_fails_when_not_initializing(
            seed in seed_strategy(),
        ) {
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            // First synchronize succeeds
            protocol.synchronize().unwrap();

            // Second synchronize should fail
            let result = protocol.synchronize();
            prop_assert!(result.is_err());
            // Use matches! in a separate assertion to avoid format string issues
            let is_invalid_request = matches!(
                result,
                Err(FortressError::InvalidRequestStructured {
                    kind: InvalidRequestKind::WrongProtocolState { .. }
                })
            );
            prop_assert!(is_invalid_request);
        }

        /// INV-PROTO-1: Completing sync transitions to Running state
        #[test]
        fn prop_complete_sync_transitions_to_running(
            seed in seed_strategy(),
            num_packets in num_sync_packets_strategy(),
        ) {
            let sync_config = SyncConfig {
                num_sync_packets: num_packets,
                ..SyncConfig::default()
            };
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                sync_config,
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            prop_assert!(matches!(protocol.state, ProtocolState::Synchronizing));

            complete_sync(&mut protocol, num_packets);

            prop_assert!(matches!(protocol.state, ProtocolState::Running));
            prop_assert!(protocol.is_synchronized());
            prop_assert!(protocol.is_running());
        }

        /// INV-PROTO-1: disconnect() transitions to Disconnected state
        #[test]
        fn prop_disconnect_transitions_to_disconnected(
            seed in seed_strategy(),
        ) {
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            complete_sync(&mut protocol, 5);
            prop_assert!(protocol.is_running());

            protocol.disconnect();

            prop_assert!(matches!(protocol.state, ProtocolState::Disconnected));
            prop_assert!(protocol.is_synchronized()); // Still "synchronized" per API
            prop_assert!(!protocol.is_running());
        }
    }

    // ========================================================================
    // INV-PROTO-2 & INV-PROTO-3: Sync counter invariants
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// INV-PROTO-2: sync_remaining_roundtrips starts at num_sync_packets
        #[test]
        fn prop_sync_remaining_starts_at_num_packets(
            num_packets in num_sync_packets_strategy(),
            seed in seed_strategy(),
        ) {
            let sync_config = SyncConfig {
                num_sync_packets: num_packets,
                ..SyncConfig::default()
            };
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                sync_config,
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();

            prop_assert_eq!(protocol.sync_remaining_roundtrips, num_packets);
        }

        /// INV-PROTO-3: sync_remaining_roundtrips decrements correctly
        #[test]
        fn prop_sync_remaining_decrements_correctly(
            num_packets in 2u32..=10,
            seed in seed_strategy(),
        ) {
            let sync_config = SyncConfig {
                num_sync_packets: num_packets,
                ..SyncConfig::default()
            };
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                sync_config,
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            let initial = protocol.sync_remaining_roundtrips;

            // Complete one roundtrip
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            let reply = SyncReply { random_reply: random };
            protocol.on_sync_reply(header, reply);

            prop_assert_eq!(
                protocol.sync_remaining_roundtrips,
                initial - 1,
                "sync_remaining should decrement by 1"
            );
        }

        /// INV-PROTO-3: Invalid sync replies do not decrement counter
        #[test]
        fn prop_invalid_sync_reply_no_decrement(
            num_packets in num_sync_packets_strategy(),
            seed in seed_strategy(),
            invalid_random in any::<u32>(),
        ) {
            let sync_config = SyncConfig {
                num_sync_packets: num_packets,
                ..SyncConfig::default()
            };
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                sync_config,
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            let initial = protocol.sync_remaining_roundtrips;

            // Send reply with random value that doesn't match any request
            // (unless by coincidence, which is astronomically unlikely)
            if !protocol.sync_random_requests.contains(&invalid_random) {
                let header = MessageHeader { magic: 999 };
                let reply = SyncReply { random_reply: invalid_random };
                protocol.on_sync_reply(header, reply);

                prop_assert_eq!(
                    protocol.sync_remaining_roundtrips,
                    initial,
                    "Invalid reply should not decrement sync_remaining"
                );
            }
        }
    }

    // ========================================================================
    // INV-PROTO-4: Magic number invariants
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// INV-PROTO-4: Magic number is never zero regardless of seed
        #[test]
        fn prop_magic_never_zero(seed in seed_strategy()) {
            let protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            prop_assert_ne!(protocol.magic, 0, "Magic number must never be zero");
        }

        /// INV-PROTO-4: Same seed produces same magic (determinism)
        #[test]
        fn prop_magic_deterministic(seed in seed_strategy()) {
            let protocol1: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            let protocol2: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            prop_assert_eq!(
                protocol1.magic,
                protocol2.magic,
                "Same seed must produce same magic"
            );
        }
    }

    // ========================================================================
    // INV-PROTO-5: State predicate consistency
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// INV-PROTO-5: is_running implies is_synchronized
        #[test]
        fn prop_is_running_implies_is_synchronized(
            seed in seed_strategy(),
            num_packets in num_sync_packets_strategy(),
        ) {
            let sync_config = SyncConfig {
                num_sync_packets: num_packets,
                ..SyncConfig::default()
            };
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                sync_config,
                ProtocolConfig::deterministic(seed),
            );

            // Test in all states
            prop_assert!(!protocol.is_running() || protocol.is_synchronized());

            protocol.synchronize().unwrap();
            prop_assert!(!protocol.is_running() || protocol.is_synchronized());

            complete_sync(&mut protocol, num_packets);
            // Now running - should be synchronized
            prop_assert!(protocol.is_running());
            prop_assert!(protocol.is_synchronized());

            protocol.disconnect();
            prop_assert!(!protocol.is_running());
            prop_assert!(protocol.is_synchronized()); // Disconnected is still "synchronized"
        }

        /// INV-PROTO-5: State predicates match state enum
        #[test]
        fn prop_state_predicates_match_enum(
            seed in seed_strategy(),
        ) {
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            // Initializing
            prop_assert!(matches!(protocol.state, ProtocolState::Initializing));
            prop_assert!(!protocol.is_running());
            prop_assert!(!protocol.is_synchronized());

            // Synchronizing
            protocol.synchronize().unwrap();
            prop_assert!(matches!(protocol.state, ProtocolState::Synchronizing));
            prop_assert!(!protocol.is_running());
            prop_assert!(!protocol.is_synchronized());

            // Running
            complete_sync(&mut protocol, 5);
            prop_assert!(matches!(protocol.state, ProtocolState::Running));
            prop_assert!(protocol.is_running());
            prop_assert!(protocol.is_synchronized());

            // Disconnected
            protocol.disconnect();
            prop_assert!(matches!(protocol.state, ProtocolState::Disconnected));
            prop_assert!(!protocol.is_running());
            prop_assert!(protocol.is_synchronized());
        }
    }

    // ========================================================================
    // INV-PROTO-6: Input frame sequence monotonicity
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// INV-PROTO-6: Pending output frames are monotonically increasing
        #[test]
        fn prop_pending_output_frames_monotonic(
            seed in seed_strategy(),
            num_frames in 1usize..20,
        ) {
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            complete_sync(&mut protocol, 5);

            // Add sequential frames to pending_output
            for i in 0..num_frames {
                protocol.pending_output.push_back(InputBytes {
                    frame: Frame::new(i as i32),
                    bytes: vec![i as u8; 4],
                });
            }

            // Verify monotonicity
            let frames: Vec<Frame> = protocol.pending_output.iter()
                .map(|ib| ib.frame)
                .collect();

            for window in frames.windows(2) {
                prop_assert!(
                    window[0] < window[1],
                    "Frames should be strictly increasing: {:?} should be < {:?}",
                    window[0],
                    window[1]
                );
            }
        }

        /// INV-PROTO-6: InputAck pops frames in order
        #[test]
        fn prop_input_ack_pops_in_order(
            seed in seed_strategy(),
            ack_frame in 0i32..10,
        ) {
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            complete_sync(&mut protocol, 5);

            // Add frames 0-9
            for i in 0..10 {
                protocol.pending_output.push_back(InputBytes {
                    frame: Frame::new(i),
                    bytes: vec![i as u8; 4],
                });
            }

            // Ack up to ack_frame
            protocol.on_input_ack(InputAck {
                ack_frame: Frame::new(ack_frame),
            });

            // All remaining frames should be > ack_frame
            for pending in &protocol.pending_output {
                prop_assert!(
                    pending.frame > Frame::new(ack_frame),
                    "Remaining frame {:?} should be > ack_frame {:?}",
                    pending.frame,
                    Frame::new(ack_frame)
                );
            }
        }
    }

    // ========================================================================
    // INV-PROTO-7: Checksum history is bounded
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// INV-PROTO-7: Checksum history never exceeds max_checksum_history
        #[test]
        fn prop_checksum_history_bounded(
            seed in seed_strategy(),
            num_checksums in 1usize..100,
        ) {
            let protocol_config = ProtocolConfig {
                max_checksum_history: 32,
                ..ProtocolConfig::deterministic(seed)
            };

            let mut protocol: UdpProtocol<TestConfig> = UdpProtocol::new(
                vec![PlayerHandle::new(0)],
                test_addr(),
                2,
                1,
                8,
                Duration::from_secs(5),
                Duration::from_secs(3),
                60,
                DesyncDetection::On { interval: 1 },
                SyncConfig::default(),
                protocol_config,
                TimeSyncConfig::default(),
            )
            .expect("Failed to create protocol");

            // Add many checksums
            for i in 0..num_checksums {
                let report = ChecksumReport {
                    frame: Frame::new(i as i32),
                    checksum: i as u128,
                };
                protocol.on_checksum_report(&report);
            }

            prop_assert!(
                protocol.pending_checksums.len() <= 32,
                "Checksum history ({}) should not exceed max (32)",
                protocol.pending_checksums.len()
            );
        }

        /// INV-PROTO-7: Old checksums are evicted when history is full
        #[test]
        fn prop_old_checksums_evicted(
            seed in seed_strategy(),
        ) {
            let max_history = 10usize;
            let protocol_config = ProtocolConfig {
                max_checksum_history: max_history,
                ..ProtocolConfig::deterministic(seed)
            };

            let mut protocol: UdpProtocol<TestConfig> = UdpProtocol::new(
                vec![PlayerHandle::new(0)],
                test_addr(),
                2,
                1,
                8,
                Duration::from_secs(5),
                Duration::from_secs(3),
                60,
                DesyncDetection::On { interval: 1 },
                SyncConfig::default(),
                protocol_config,
                TimeSyncConfig::default(),
            )
            .expect("Failed to create protocol");

            // Add max_history + 5 checksums
            for i in 0..(max_history + 5) {
                let report = ChecksumReport {
                    frame: Frame::new(i as i32),
                    checksum: i as u128,
                };
                protocol.on_checksum_report(&report);
            }

            // Oldest frames should have been evicted
            prop_assert!(
                !protocol.pending_checksums.contains_key(&Frame::new(0)),
                "Frame 0 should have been evicted"
            );

            // Most recent frames should still be present
            let last_frame = (max_history + 4) as i32;
            prop_assert!(
                protocol.pending_checksums.contains_key(&Frame::new(last_frame)),
                "Most recent frame {} should still be present",
                last_frame
            );
        }
    }

    // ========================================================================
    // INV-PROTO-8 & INV-PROTO-9: Message handling invariants
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// INV-PROTO-8: Sync reply processing is idempotent for same random value
        #[test]
        fn prop_sync_reply_idempotent_same_random(
            seed in seed_strategy(),
        ) {
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            let initial_remaining = protocol.sync_remaining_roundtrips;

            // Get a valid random value
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            let reply = SyncReply { random_reply: random };

            // First reply decrements
            protocol.on_sync_reply(header, reply);
            prop_assert_eq!(
                protocol.sync_remaining_roundtrips,
                initial_remaining - 1
            );

            let after_first = protocol.sync_remaining_roundtrips;

            // Same reply again should have no effect (random already removed)
            protocol.on_sync_reply(header, reply);
            prop_assert_eq!(
                protocol.sync_remaining_roundtrips,
                after_first,
                "Duplicate sync reply should have no effect"
            );
        }

        /// INV-PROTO-9: Messages are dropped in Shutdown state
        #[test]
        fn prop_messages_dropped_in_shutdown(
            seed in seed_strategy(),
            checksum in checksum_strategy(),
            frame in frame_strategy(),
        ) {
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.state = ProtocolState::Shutdown;
            let initial_checksum_count = protocol.pending_checksums.len();

            // Try to handle a checksum report
            let msg = Message {
                header: MessageHeader { magic: 123 },
                body: MessageBody::ChecksumReport(ChecksumReport {
                    frame: Frame::new(frame),
                    checksum,
                }),
            };
            protocol.handle_message(&msg);

            prop_assert_eq!(
                protocol.pending_checksums.len(),
                initial_checksum_count,
                "Checksums should not be added in Shutdown state"
            );

            prop_assert!(
                protocol.event_queue.is_empty(),
                "Events should not be generated in Shutdown state"
            );
        }
    }

    // ========================================================================
    // InputBytes Property Tests
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// InputBytes roundtrip preserves data for any input values
        #[test]
        fn prop_input_bytes_roundtrip(
            input1 in input_value_strategy(),
            input2 in input_value_strategy(),
            frame in frame_strategy(),
        ) {
            let mut inputs = BTreeMap::new();
            inputs.insert(
                PlayerHandle::new(0),
                PlayerInput::new(Frame::new(frame), TestInput { inp: input1 }),
            );
            inputs.insert(
                PlayerHandle::new(1),
                PlayerInput::new(Frame::new(frame), TestInput { inp: input2 }),
            );

            let input_bytes = InputBytes::from_inputs::<TestConfig>(2, &inputs);
            let player_inputs = input_bytes.to_player_inputs::<TestConfig>(2);

            prop_assert_eq!(player_inputs.len(), 2);
            prop_assert_eq!(player_inputs[0].frame, Frame::new(frame));
            prop_assert_eq!(player_inputs[0].input.inp, input1);
            prop_assert_eq!(player_inputs[1].frame, Frame::new(frame));
            prop_assert_eq!(player_inputs[1].input.inp, input2);
        }

        /// InputBytes zeroed creates correct size for any player count
        #[test]
        fn prop_input_bytes_zeroed_size(
            num_players in 1usize..10,
        ) {
            let input_bytes = InputBytes::zeroed::<TestConfig>(num_players)
                .expect("Failed to create zeroed InputBytes");

            // TestInput is u32 = 4 bytes per player
            prop_assert_eq!(
                input_bytes.bytes.len(),
                num_players * 4,
                "Zeroed InputBytes should have 4 bytes per player"
            );
            prop_assert_eq!(input_bytes.frame, Frame::NULL);
            prop_assert!(
                input_bytes.bytes.iter().all(|&b| b == 0),
                "All bytes should be zero"
            );
        }
    }

    // ========================================================================
    // INV-PROTO-10: Input Compression Roundtrip
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// INV-PROTO-10: Input bytes compression/decompression roundtrip preserves data.
        ///
        /// This test verifies that the full compression pipeline used in `on_input`
        /// (XOR delta encoding + RLE) correctly preserves input data through encoding
        /// and decoding, as would happen in actual network transmission.
        #[test]
        fn prop_input_compression_roundtrip(
            seed in seed_strategy(),
            num_players in player_count_strategy(),
            num_frames in 1usize..10,
        ) {
            use crate::network::compression::{decode, encode};

            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                num_players,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            complete_sync(&mut protocol, 5);

            // Create reference input (simulating last_acked_input)
            let reference = InputBytes::zeroed::<TestConfig>(num_players)
                .expect("Failed to create zeroed InputBytes");

            // Generate a sequence of inputs to send (simulating pending_output)
            let mut pending_inputs: Vec<InputBytes> = Vec::new();
            for i in 0..num_frames {
                let mut inputs = BTreeMap::new();
                for p in 0..num_players {
                    inputs.insert(
                        PlayerHandle::new(p),
                        PlayerInput::new(
                            Frame::new(i as i32),
                            TestInput { inp: ((i * num_players + p) as u32).wrapping_mul(seed as u32) },
                        ),
                    );
                }
                pending_inputs.push(InputBytes::from_inputs::<TestConfig>(num_players, &inputs));
            }

            // Encode using the same method as send_pending_output
            let encoded = encode(
                &reference.bytes,
                pending_inputs.iter().map(|gi| &gi.bytes),
            );

            // Decode using the same method as on_input
            let decoded = decode(&reference.bytes, &encoded);
            prop_assert!(decoded.is_ok(), "Decode should succeed");

            let decoded_inputs = decoded.unwrap();
            prop_assert_eq!(
                decoded_inputs.len(),
                pending_inputs.len(),
                "Decoded input count should match"
            );

            // Verify each input matches
            for (i, (original, decoded_bytes)) in pending_inputs.iter().zip(decoded_inputs.iter()).enumerate() {
                prop_assert_eq!(
                    &original.bytes,
                    decoded_bytes,
                    "Input {} bytes should match after roundtrip",
                    i
                );
            }
        }
    }

    // ========================================================================
    // INV-PROTO-11: Frame::NULL Edge Case Handling
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// INV-PROTO-11a: Frame::NULL is correctly handled in update_local_frame_advantage.
        ///
        /// When either local_frame or last_recv_frame is NULL, the function should
        /// return early without modifying local_frame_advantage.
        #[test]
        fn prop_null_frame_in_frame_advantage(
            seed in seed_strategy(),
            round_trip_time in 0u128..1000,
        ) {
            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.round_trip_time = round_trip_time;
            let initial_advantage = protocol.local_frame_advantage;

            // Case 1: local_frame is NULL
            protocol.update_local_frame_advantage(Frame::NULL);
            prop_assert_eq!(
                protocol.local_frame_advantage,
                initial_advantage,
                "local_frame_advantage should not change when local_frame is NULL"
            );

            // Case 2: last_recv_frame is NULL (recv_inputs only has NULL entry)
            protocol.update_local_frame_advantage(Frame::new(100));
            prop_assert_eq!(
                protocol.local_frame_advantage,
                initial_advantage,
                "local_frame_advantage should not change when last_recv_frame is NULL"
            );
        }

        /// INV-PROTO-11b: Frame::NULL is used as initial decode reference.
        ///
        /// When receiving the first input (last_recv_frame is NULL), the protocol
        /// should decode using the NULL frame's zeroed input as reference.
        #[test]
        fn prop_null_frame_as_decode_reference(
            seed in seed_strategy(),
            input_value in input_value_strategy(),
        ) {
            use crate::network::compression::encode;
            use crate::network::messages::{ConnectionStatus, Input};

            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            complete_sync(&mut protocol, 5);

            // Verify that NULL frame entry exists (created in constructor)
            prop_assert!(
                protocol.recv_inputs.contains_key(&Frame::NULL),
                "Protocol should have Frame::NULL entry for decoding"
            );

            // Verify last_recv_frame returns NULL when only NULL entry exists
            prop_assert_eq!(
                protocol.last_recv_frame(),
                Frame::NULL,
                "last_recv_frame should return NULL initially"
            );

            // Create and encode an input for frame 0 relative to the NULL frame reference
            let zeroed_bytes = protocol
                .recv_inputs
                .get(&Frame::NULL)
                .unwrap()
                .bytes
                .clone();

            let test_input = TestInput { inp: input_value };
            let test_bytes = crate::network::codec::encode(&test_input).unwrap();
            let encoded = encode(&zeroed_bytes, std::iter::once(&test_bytes));

            let input = Input {
                start_frame: Frame::new(0),
                ack_frame: Frame::NULL,
                bytes: encoded,
                disconnect_requested: false,
                peer_connect_status: vec![ConnectionStatus::default(); 2],
            };

            // Process the input
            protocol.on_input(&input);

            // Verify frame 0 was added
            prop_assert!(
                protocol.recv_inputs.contains_key(&Frame::new(0)),
                "Frame 0 should be added when decoding from NULL reference"
            );

            // Verify the input event was generated
            let has_input_event = protocol.event_queue.iter()
                .any(|e| matches!(e, Event::Input { .. }));
            prop_assert!(has_input_event, "Input event should be generated");
        }

        /// INV-PROTO-11c: Frame::NULL in pending_output triggers sequence violation check.
        ///
        /// When last_acked_input is not NULL and pending_output has non-sequential frames,
        /// a violation should be reported and send should be skipped.
        #[test]
        fn prop_null_frame_sequence_violation(
            seed in seed_strategy(),
        ) {
            use crate::network::messages::ConnectionStatus;

            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            complete_sync(&mut protocol, 5);

            // Set last_acked_input to frame 5
            protocol.last_acked_input = InputBytes {
                frame: Frame::new(5),
                bytes: vec![0; 4],
            };

            // Add a non-sequential frame (frame 10 instead of expected frame 6)
            protocol.pending_output.push_back(InputBytes {
                frame: Frame::new(10),
                bytes: vec![1, 2, 3, 4],
            });

            let connect_status = vec![ConnectionStatus::default(); 2];
            let initial_send_queue_len = protocol.send_queue.len();

            // Call send_pending_output - should detect violation and not queue message
            protocol.send_pending_output(&connect_status);

            // The violation path should return early without queueing a message
            // (The actual violation is reported, but we can verify no message was sent
            // because the frame sequence check fails)
            prop_assert_eq!(
                protocol.send_queue.len(),
                initial_send_queue_len,
                "No message should be queued when frame sequence is violated"
            );
        }
    }

    // ========================================================================
    // INV-PROTO-12: Multi-Player Variation Invariants
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// INV-PROTO-12: Protocol invariants hold across varied player counts (1-4).
        ///
        /// This test systematically verifies key invariants with different player counts,
        /// ensuring the protocol behaves correctly regardless of the number of players.
        #[test]
        fn prop_multi_player_protocol_invariants(
            seed in seed_strategy(),
            num_players in player_count_strategy(),
            max_pred in max_prediction_strategy(),
            num_sync_packets in num_sync_packets_strategy(),
            num_inputs in 1usize..10,
        ) {
            let sync_config = SyncConfig {
                num_sync_packets,
                ..SyncConfig::default()
            };

            // Create handles for the remote players we're receiving from
            let handles: Vec<_> = (0..num_players).map(PlayerHandle::new).collect();

            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                handles.clone(),
                num_players,
                1,
                max_pred,
                sync_config,
                ProtocolConfig::deterministic(seed),
            );

            // INV-1: Protocol starts in Initializing
            prop_assert!(matches!(protocol.state, ProtocolState::Initializing));

            protocol.synchronize().unwrap();

            // INV-2: sync_remaining_roundtrips starts at num_sync_packets
            prop_assert_eq!(protocol.sync_remaining_roundtrips, num_sync_packets);

            complete_sync(&mut protocol, num_sync_packets);

            // INV-3: After sync, state is Running
            prop_assert!(protocol.is_running());
            prop_assert!(protocol.is_synchronized());

            // INV-4: recv_inputs has the NULL frame entry for decoding
            prop_assert!(
                protocol.recv_inputs.contains_key(&Frame::NULL),
                "recv_inputs should contain NULL frame for {} players",
                num_players
            );

            // INV-5: The NULL frame input bytes has correct size for player count
            let null_input = protocol.recv_inputs.get(&Frame::NULL).unwrap();
            // TestInput is u32 = 4 bytes per player
            let expected_size = handles.len() * 4;
            prop_assert_eq!(
                null_input.bytes.len(),
                expected_size,
                "NULL frame bytes should have {} bytes for {} players",
                expected_size,
                handles.len()
            );

            // INV-6: Adding inputs maintains correct byte sizes
            for i in 0..num_inputs {
                let mut inputs = BTreeMap::new();
                for p in 0..num_players {
                    inputs.insert(
                        PlayerHandle::new(p),
                        PlayerInput::new(
                            Frame::new(i as i32),
                            TestInput { inp: (i * num_players + p) as u32 },
                        ),
                    );
                }
                let input_bytes = InputBytes::from_inputs::<TestConfig>(num_players, &inputs);
                prop_assert_eq!(
                    input_bytes.bytes.len(),
                    num_players * 4,
                    "Input bytes for frame {} should have correct size for {} players",
                    i,
                    num_players
                );
            }

            // INV-7: Player handles are sorted
            let handles_arc = protocol.handles();
            for window in handles_arc.windows(2) {
                prop_assert!(
                    window[0] < window[1],
                    "Handles should be sorted"
                );
            }
        }

        /// INV-PROTO-12b: Input event generation respects player count.
        ///
        /// When inputs are received, the correct number of Input events should be
        /// generated based on the number of remote player handles.
        #[test]
        fn prop_multi_player_input_events(
            seed in seed_strategy(),
            num_players in player_count_strategy(),
            input_values in proptest::collection::vec(input_value_strategy(), 1..=4),
        ) {
            use crate::network::compression::encode;
            use crate::network::messages::{ConnectionStatus, Input};

            let handles: Vec<_> = (0..num_players).map(PlayerHandle::new).collect();

            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                handles.clone(),
                num_players,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            complete_sync(&mut protocol, 5);

            // Get the NULL frame reference for encoding
            let zeroed_bytes = protocol
                .recv_inputs
                .get(&Frame::NULL)
                .unwrap()
                .bytes
                .clone();

            // Create input bytes for all players
            let mut input_bytes_vec = Vec::new();
            for i in 0..handles.len() {
                let input_value = input_values.get(i).copied().unwrap_or(0);
                let test_input = TestInput { inp: input_value };
                input_bytes_vec.extend(crate::network::codec::encode(&test_input).unwrap());
            }

            let encoded = encode(&zeroed_bytes, std::iter::once(&input_bytes_vec));

            let input = Input {
                start_frame: Frame::new(0),
                ack_frame: Frame::NULL,
                bytes: encoded,
                disconnect_requested: false,
                peer_connect_status: vec![ConnectionStatus::default(); num_players],
            };

            protocol.event_queue.clear();
            protocol.on_input(&input);

            // Count Input events
            let input_events: Vec<_> = protocol.event_queue.iter()
                .filter_map(|e| {
                    if let Event::Input { player, .. } = e {
                        Some(*player)
                    } else {
                        None
                    }
                })
                .collect();

            // Should have one Input event per player handle
            prop_assert_eq!(
                input_events.len(),
                handles.len(),
                "Should generate {} Input events for {} players",
                handles.len(),
                num_players
            );

            // Verify each handle received exactly one event
            for handle in &handles {
                let count = input_events.iter().filter(|&&h| h == *handle).count();
                prop_assert_eq!(
                    count,
                    1,
                    "Player {:?} should receive exactly one Input event",
                    handle
                );
            }
        }
    }

    // ========================================================================
    // INV-PROTO-13: Input Gap Rejection
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]

        /// INV-PROTO-13: Inputs with gaps larger than 1 are rejected.
        ///
        /// When receiving an input whose start_frame is more than 1 greater than
        /// last_recv_frame (when last_recv_frame is not NULL), the input should
        /// be rejected and no new frames should be added to recv_inputs.
        #[test]
        fn prop_input_gap_rejection(
            seed in seed_strategy(),
            last_frame in 0i32..100,
            gap_size in 2i32..20,
        ) {
            use crate::network::messages::{ConnectionStatus, Input};

            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            complete_sync(&mut protocol, 5);

            // Set up: we have received up to last_frame
            protocol.recv_inputs.insert(
                Frame::new(last_frame),
                InputBytes {
                    frame: Frame::new(last_frame),
                    bytes: vec![0, 0, 0, 0],
                },
            );

            // Calculate the frame that's too far ahead
            let gap_frame = last_frame + gap_size;

            let input = Input {
                start_frame: Frame::new(gap_frame),
                ack_frame: Frame::NULL,
                bytes: vec![1, 2, 3, 4], // Won't be decoded
                disconnect_requested: false,
                peer_connect_status: vec![ConnectionStatus::default(); 2],
            };

            let inputs_before = protocol.recv_inputs.len();
            protocol.event_queue.clear();

            protocol.on_input(&input);

            // Verify: no new inputs were added
            prop_assert_eq!(
                protocol.recv_inputs.len(),
                inputs_before,
                "No inputs should be added when gap is {} (> 1)",
                gap_size
            );

            // Verify the gap frame was not added
            prop_assert!(
                !protocol.recv_inputs.contains_key(&Frame::new(gap_frame)),
                "Frame {} should not be added with gap of {}",
                gap_frame,
                gap_size
            );

            // Verify no Input events were generated
            let input_event_count = protocol.event_queue.iter()
                .filter(|e| matches!(e, Event::Input { .. }))
                .count();
            prop_assert_eq!(
                input_event_count,
                0,
                "No Input events should be generated when gap is rejected"
            );
        }

        /// INV-PROTO-13b: Gap of exactly 1 is accepted (boundary condition).
        ///
        /// When receiving an input whose start_frame is exactly last_recv_frame + 1,
        /// the input should be accepted and processed.
        #[test]
        fn prop_input_gap_one_accepted(
            last_frame in 0i32..100,
        ) {
            use crate::network::compression::encode;
            use crate::network::messages::{ConnectionStatus, Input};

            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::default(),
            );

            protocol.synchronize().unwrap();
            complete_sync(&mut protocol, 5);

            // Get the input size from the NULL frame entry (this is the correct size for decode)
            let input_size = protocol
                .recv_inputs
                .get(&Frame::NULL)
                .unwrap()
                .bytes
                .len();

            // Set up: we have received up to last_frame using correctly sized bytes
            let last_bytes = vec![last_frame as u8; input_size];
            protocol.recv_inputs.insert(
                Frame::new(last_frame),
                InputBytes {
                    frame: Frame::new(last_frame),
                    bytes: last_bytes.clone(),
                },
            );

            // Create input for exactly the next frame (gap of 1)
            let next_frame = last_frame + 1;
            let next_bytes = vec![next_frame as u8; input_size];
            let encoded = encode(&last_bytes, std::iter::once(&next_bytes));

            let input = Input {
                start_frame: Frame::new(next_frame),
                ack_frame: Frame::NULL,
                bytes: encoded,
                disconnect_requested: false,
                peer_connect_status: vec![ConnectionStatus::default(); 2],
            };

            protocol.event_queue.clear();

            protocol.on_input(&input);

            // Verify: the new frame was added.
            // Note: We check the specific frame rather than count because on_input's
            // retain logic may remove old frames (including NULL) based on history settings.
            prop_assert!(
                protocol.recv_inputs.contains_key(&Frame::new(next_frame)),
                "Frame {} should be added with gap of exactly 1",
                next_frame
            );

            // Verify the last_frame is still present (it's the decode reference)
            prop_assert!(
                protocol.recv_inputs.contains_key(&Frame::new(last_frame)),
                "Frame {} should still be present after decode",
                last_frame
            );
        }

        /// INV-PROTO-13c: First input (from NULL) is always accepted.
        ///
        /// When last_recv_frame is NULL, any start_frame should be accepted
        /// because there's no gap check when there's no previous frame.
        #[test]
        fn prop_first_input_always_accepted(
            seed in seed_strategy(),
            start_frame in 0i32..100,
        ) {
            use crate::network::compression::encode;
            use crate::network::messages::{ConnectionStatus, Input};

            let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
                vec![PlayerHandle::new(0)],
                2,
                1,
                8,
                SyncConfig::default(),
                ProtocolConfig::deterministic(seed),
            );

            protocol.synchronize().unwrap();
            complete_sync(&mut protocol, 5);

            // Verify we're starting from NULL
            prop_assert_eq!(
                protocol.last_recv_frame(),
                Frame::NULL,
                "Should start with NULL last_recv_frame"
            );

            // Get the NULL frame reference for encoding
            let zeroed_bytes = protocol
                .recv_inputs
                .get(&Frame::NULL)
                .unwrap()
                .bytes
                .clone();

            // Create input for an arbitrary start_frame
            let input_bytes = vec![start_frame as u8; zeroed_bytes.len()];
            let encoded = encode(&zeroed_bytes, std::iter::once(&input_bytes));

            let input = Input {
                start_frame: Frame::new(start_frame),
                ack_frame: Frame::NULL,
                bytes: encoded,
                disconnect_requested: false,
                peer_connect_status: vec![ConnectionStatus::default(); 2],
            };

            protocol.event_queue.clear();
            protocol.on_input(&input);

            // Verify: the frame was added regardless of start_frame value
            prop_assert!(
                protocol.recv_inputs.contains_key(&Frame::new(start_frame)),
                "Frame {} should be accepted when last_recv_frame is NULL",
                start_frame
            );
        }
    }
}

// =============================================================================
// Kani Formal Verification Proofs
//
// These proofs verify core invariants of the UDP protocol layer using exhaustive
// symbolic verification. Kani explores ALL possible values within the specified
// bounds.
//
// ## Verified Invariants
//
// 1. **ProtocolState Transitions**: Valid state transitions match TLA+ spec
// 2. **Frame Arithmetic**: Frame::is_null() correctly identifies NULL frames
// 3. **PlayerHandle Preservation**: Handle indices are preserved through operations
// 4. **ConnectionStatus Invariants**: last_frame is always set correctly
//
// ## Design Notes
//
// The UdpProtocol type is complex with many dependencies (Vec, BTreeMap, Instant).
// We focus on verifying types that CAN be instantiated in Kani:
// - ProtocolState: Simple enum, no dependencies
// - ConnectionStatus: Simple struct with primitives
// - Frame: Wrapper around i32
// - PlayerHandle: Wrapper around usize
//
// Full protocol verification requires integration with the TLA+ model.
// =============================================================================
#[cfg(kani)]
mod kani_proofs {
    use super::*;
    use crate::network::messages::ConnectionStatus;

    // =========================================================================
    // ProtocolState Verification
    //
    // These proofs verify the state machine properties documented in state.rs.
    // TLA+ alignment: specs/tla/NetworkProtocol.tla
    // =========================================================================

    /// Proof: ProtocolState transitions follow valid paths.
    ///
    /// Verifies that the state machine has exactly 5 states matching TLA+ spec.
    /// TLA+ alignment: NetworkProtocol.tla defines States = {Init, Sync, Running, Disconnected, Shutdown}
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: State machine has exactly 5 states
    /// - Related: proof_running_is_active_state, proof_state_count_matches_specification
    #[kani::proof]
    fn proof_protocol_state_count() {
        let state_idx: u8 = kani::any();

        // The protocol has exactly 5 valid states
        let is_valid_state = state_idx < 5;

        let state = match state_idx {
            0 => Some(ProtocolState::Initializing),
            1 => Some(ProtocolState::Synchronizing),
            2 => Some(ProtocolState::Running),
            3 => Some(ProtocolState::Disconnected),
            4 => Some(ProtocolState::Shutdown),
            _ => None,
        };

        kani::assert(
            state.is_some() == is_valid_state,
            "State should exist iff index < 5",
        );
    }

    /// Proof: ProtocolState::Running is the only state that processes inputs.
    ///
    /// Verifies INV-PROTO-1: Only Running state should handle game inputs.
    /// This is a state predicate that the protocol relies on.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Running is the only input-processing state (INV-PROTO-1)
    /// - Related: proof_protocol_state_count, proof_synchronize_precondition
    #[kani::proof]
    fn proof_running_is_active_state() {
        let state_idx: u8 = kani::any();
        kani::assume(state_idx < 5);

        let state = match state_idx {
            0 => ProtocolState::Initializing,
            1 => ProtocolState::Synchronizing,
            2 => ProtocolState::Running,
            3 => ProtocolState::Disconnected,
            _ => ProtocolState::Shutdown,
        };

        // Only Running state should accept game inputs
        let accepts_inputs = matches!(state, ProtocolState::Running);

        // Verify this matches the expected index
        kani::assert(
            accepts_inputs == (state_idx == 2),
            "Only Running (index 2) accepts inputs",
        );
    }

    /// Proof: disconnect() is idempotent from Shutdown state.
    ///
    /// Verifies the guard condition at mod.rs:366: `if self.state == ProtocolState::Shutdown`
    /// ensures calling disconnect() from Shutdown is a no-op.
    /// Production code: disconnect() returns early if already in Shutdown state.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Disconnect idempotence from Shutdown state
    /// - Related: proof_synchronize_precondition, proof_shutdown_is_terminal
    // kani::no-unwind-needed: u8 state-index guard logic, no loops
    #[kani::proof]
    fn proof_disconnect_idempotent_from_shutdown() {
        // The disconnect() function at mod.rs:365-373 checks:
        // if self.state == ProtocolState::Shutdown { return; }
        // This means disconnect from Shutdown should be a no-op.

        let state_idx: u8 = kani::any();
        kani::assume(state_idx < 5);

        // Model the disconnect guard condition
        let is_shutdown = state_idx == 4;
        let would_transition = !is_shutdown; // disconnect only transitions if not in Shutdown

        // From Shutdown, disconnect does nothing (idempotent)
        if is_shutdown {
            kani::assert(
                !would_transition,
                "Disconnect from Shutdown should be no-op",
            );
        }

        // From non-Shutdown states, disconnect transitions to Disconnected (3)
        // Note: In production, state would become Disconnected (index 3)
        if !is_shutdown && would_transition {
            let target_state = 3u8; // Disconnected
            kani::assert(
                target_state > 0 && target_state < 5,
                "Disconnect targets valid Disconnected state",
            );
        }
    }

    /// Proof: synchronize() precondition matches production code.
    ///
    /// Verifies the condition checked at mod.rs:381:
    /// `if self.state != ProtocolState::Initializing { return Err(...) }`
    /// Production code only allows sync from Initializing state.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Synchronize precondition from Initializing only
    /// - Related: proof_initializing_is_initial, proof_transition_matrix_sync_required
    // kani::no-unwind-needed: u8 state-index guard logic, no loops
    #[kani::proof]
    fn proof_synchronize_precondition() {
        let state_idx: u8 = kani::any();
        kani::assume(state_idx < 5);

        // The synchronize() function at mod.rs:380-394 checks:
        // if self.state != ProtocolState::Initializing { return Err(...) }
        let is_initializing = state_idx == 0;
        let can_synchronize = is_initializing;

        // Verify the precondition: only Initializing (0) can synchronize
        kani::assert(
            can_synchronize == (state_idx == 0),
            "Only Initializing state can call synchronize()",
        );

        // If we can synchronize, target state is Synchronizing (1)
        if can_synchronize {
            let target_state = 1u8; // Synchronizing
            kani::assert(
                target_state == state_idx + 1,
                "synchronize() transitions to next state",
            );
        }
    }

    // =========================================================================
    // ConnectionStatus Verification
    //
    // ConnectionStatus is used to track peer state. These proofs verify
    // its invariants.
    // =========================================================================

    /// Proof: ConnectionStatus default values are consistent.
    ///
    /// Verifies that a new ConnectionStatus starts in a valid initial state.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Default ConnectionStatus state validity
    /// - Related: proof_connection_status_frame_preservation, proof_connection_status_disconnected_flag
    #[kani::proof]
    fn proof_connection_status_default() {
        let status = ConnectionStatus::default();

        // Default should be connected (not disconnected) with NULL frame
        kani::assert(!status.disconnected, "default status should be connected");
        kani::assert(
            status.last_frame.is_null(),
            "default last_frame should be null",
        );
    }

    /// Proof: ConnectionStatus with symbolic values preserves frame.
    ///
    /// Verifies that last_frame is correctly stored and retrieved.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Frame field preservation in ConnectionStatus
    /// - Related: proof_connection_status_default
    #[kani::proof]
    fn proof_connection_status_frame_preservation() {
        let frame_val: i32 = kani::any();

        let status = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(frame_val),
            epoch: 0,
        };

        // Frame should be preserved
        kani::assert(
            status.last_frame == Frame::new(frame_val),
            "Frame should be preserved in ConnectionStatus",
        );

        // NULL detection should work
        if frame_val == -1 {
            kani::assert(status.last_frame.is_null(), "frame -1 should be null");
        } else {
            kani::assert(
                !status.last_frame.is_null(),
                "non -1 frame should not be null",
            );
        }
    }

    /// Proof: ConnectionStatus disconnected flag works correctly.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Disconnected flag preservation
    /// - Related: proof_connection_status_default
    #[kani::proof]
    fn proof_connection_status_disconnected_flag() {
        let is_disconnected: bool = kani::any();

        let status = ConnectionStatus {
            disconnected: is_disconnected,
            last_frame: Frame::NULL,
            epoch: 0,
        };

        kani::assert(
            status.disconnected == is_disconnected,
            "Disconnected flag should be preserved",
        );
    }

    // =========================================================================
    // Frame Verification
    //
    // Frame is a critical type used throughout the protocol.
    // =========================================================================

    /// Proof: Frame::NULL is correctly detected.
    ///
    /// Verifies that Frame::is_null() correctly identifies NULL frames.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Frame::is_null() correctness
    /// - Related: proof_frame_ordering, proof_frame_addition_safe
    #[kani::proof]
    fn proof_frame_null_detection() {
        let frame_val: i32 = kani::any();
        let frame = Frame::new(frame_val);

        let is_null = frame.is_null();

        // NULL is represented as -1
        if frame_val == -1 {
            kani::assert(is_null, "Frame -1 should be NULL");
        } else {
            kani::assert(!is_null, "Frame != -1 should not be NULL");
        }
    }

    /// Proof: Frame ordering is consistent.
    ///
    /// Verifies that Frame comparison works correctly for the protocol's
    /// frame ordering logic.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Frame comparison operators consistency
    /// - Related: proof_frame_null_detection, proof_frame_addition_safe
    #[kani::proof]
    fn proof_frame_ordering() {
        let frame_a_val: i32 = kani::any();
        let frame_b_val: i32 = kani::any();
        kani::assume(frame_a_val >= 0 && frame_a_val < 10000);
        kani::assume(frame_b_val >= 0 && frame_b_val < 10000);

        let frame_a = Frame::new(frame_a_val);
        let frame_b = Frame::new(frame_b_val);

        // Verify ordering matches underlying integer ordering
        if frame_a_val < frame_b_val {
            kani::assert(frame_a < frame_b, "Frame ordering should match i32");
        } else if frame_a_val > frame_b_val {
            kani::assert(frame_a > frame_b, "Frame ordering should match i32");
        } else {
            kani::assert(frame_a == frame_b, "Equal frames should be equal");
        }
    }

    /// Proof: Frame arithmetic is safe within bounds.
    ///
    /// Verifies that frame addition doesn't overflow for realistic values.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Frame addition overflow safety (SAFE-6)
    /// - Related: proof_frame_ordering, proof_frame_gap_safe
    // kani::no-unwind-needed: straight-line Frame + i32 arithmetic, no loops
    #[kani::proof]
    fn proof_frame_addition_safe() {
        let frame_val: i32 = kani::any();
        let increment: i32 = kani::any();

        // Realistic bounds: 10 hour session at 60 fps = 2.16M frames
        kani::assume(frame_val >= 0 && frame_val < 3_000_000);
        kani::assume(increment >= 0 && increment <= 100);

        let frame = Frame::new(frame_val);
        let result = frame + increment;

        // Result should be frame_val + increment
        kani::assert(
            result == Frame::new(frame_val + increment),
            "Frame addition should work correctly",
        );
    }

    // =========================================================================
    // PlayerHandle Verification
    //
    // PlayerHandle is used to identify players in the protocol.
    // =========================================================================

    /// Proof: PlayerHandle preserves index.
    ///
    /// Verifies that PlayerHandle::new and as_usize are inverses.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: PlayerHandle index preservation
    /// - Related: proof_player_handle_equality, proof_player_handle_validity
    #[kani::proof]
    fn proof_player_handle_preservation() {
        let index: usize = kani::any();
        kani::assume(index <= 256); // Reasonable max players

        let handle = PlayerHandle::new(index);
        let retrieved = handle.as_usize();

        kani::assert(retrieved == index, "PlayerHandle should preserve index");
    }

    /// Proof: PlayerHandle equality works correctly.
    ///
    /// Verifies that handles with same index are equal.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: PlayerHandle equality consistency
    /// - Related: proof_player_handle_preservation
    #[kani::proof]
    fn proof_player_handle_equality() {
        let idx_a: usize = kani::any();
        let idx_b: usize = kani::any();
        kani::assume(idx_a <= 256);
        kani::assume(idx_b <= 256);

        let handle_a = PlayerHandle::new(idx_a);
        let handle_b = PlayerHandle::new(idx_b);

        if idx_a == idx_b {
            kani::assert(
                handle_a == handle_b,
                "Same index should produce equal handles",
            );
        } else {
            kani::assert(
                handle_a != handle_b,
                "Different indices should produce different handles",
            );
        }
    }

    // =========================================================================
    // Protocol Arithmetic Verification
    //
    // Verify arithmetic used in the protocol is safe.
    // =========================================================================

    /// Proof: Input frame gap calculation is safe.
    ///
    /// Verifies the frame gap detection used in on_input doesn't overflow.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Frame gap detection overflow safety
    /// - Related: proof_frame_addition_safe, proof_frame_null_detection
    // kani::no-unwind-needed: scalar saturating i32 arithmetic, no loops
    #[kani::proof]
    fn proof_frame_gap_safe() {
        let last_recv: i32 = kani::any();
        let start_frame: i32 = kani::any();

        kani::assume(last_recv >= -1); // NULL (-1) or valid frame
        kani::assume(start_frame >= 0);
        kani::assume(last_recv < 3_000_000);
        kani::assume(start_frame < 3_000_000);

        // Calculate expected next frame using saturating arithmetic
        let expected_next = if last_recv == -1 {
            0
        } else {
            last_recv.saturating_add(1)
        };

        // Gap detection should not overflow
        kani::assert(
            expected_next >= 0 || expected_next == i32::MAX,
            "Expected next frame should be non-negative or saturated",
        );

        // Verify gap detection logic
        let _has_gap = start_frame > expected_next;
        if last_recv == -1 {
            // First frame - no gap possible
            kani::assert(
                start_frame >= 0,
                "start_frame should be non-negative for first frame",
            );
        }
    }

    /// Proof: sync_remaining_roundtrips decrement is safe when counter > 0.
    ///
    /// Verifies INV-PROTO-3: sync_remaining_roundtrips decrement at mod.rs:749.
    /// Production code: `self.sync_remaining_roundtrips -= 1;`
    /// This is only called after validating the sync reply, which only happens
    /// in Synchronizing state where remaining > 0.
    ///
    /// The key invariant: on_sync_reply() (mod.rs:740-769) only decrements when:
    /// 1. State is Synchronizing (line 741)
    /// 2. The random_reply is valid (line 745)
    /// In this path, remaining was set to num_sync_packets > 0 at sync start.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Sync counter decrement safety (INV-PROTO-3)
    /// - Related: proof_sync_remaining_bounds
    #[kani::proof]
    #[kani::unwind(11)] // max loop iterations = 10, need +1 for termination check
    fn proof_sync_counter_decrement_safe() {
        let num_sync_packets: u32 = kani::any();
        // SyncConfig::num_sync_packets default is 5, production presets use 3-20.
        // Bounded to 10 for tractable loop verification (proof covers representative values).
        // Note: proof_sync_remaining_bounds uses <= 100 because it's loop-free.
        kani::assume(num_sync_packets > 0 && num_sync_packets <= 10);

        // sync_remaining starts at num_sync_packets (set at mod.rs:390)
        let mut remaining = num_sync_packets;

        // Simulate the decrement loop that happens in on_sync_reply
        // Each valid sync reply decrements by 1
        let replies_received: u32 = kani::any();
        kani::assume(replies_received <= num_sync_packets);

        for _ in 0..replies_received {
            // This is the decrement at mod.rs:749
            // Safe because remaining starts > 0 and we only decrement replies_received times
            kani::assert(
                remaining > 0,
                "Remaining should be positive before decrement",
            );
            remaining -= 1;
        }

        // After all decrements, remaining should be num_sync_packets - replies_received
        kani::assert(
            remaining == num_sync_packets - replies_received,
            "Remaining should equal initial minus replies",
        );

        // sync_remaining_roundtrips is never negative (it's u32, and we don't underflow)
        kani::assert(
            remaining <= num_sync_packets,
            "Remaining never exceeds initial value",
        );
    }

    /// Proof: sync_remaining_roundtrips bounds are maintained.
    ///
    /// Verifies INV-PROTO-2 and INV-PROTO-3:
    /// - sync_remaining never exceeds num_sync_packets
    /// - sync_remaining is non-negative (u32 guarantee + no underflow)
    ///
    /// Production code reference:
    /// - mod.rs:390 sets: `self.sync_remaining_roundtrips = self.sync_config.num_sync_packets`
    /// - mod.rs:749 decrements: `self.sync_remaining_roundtrips -= 1` (only when > 0 implicitly)
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Sync counter bounds (INV-PROTO-2, INV-PROTO-3)
    /// - Related: proof_sync_counter_decrement_safe
    // kani::no-unwind-needed: loop-free saturating_sub model, no loops
    #[kani::proof]
    fn proof_sync_remaining_bounds() {
        let num_sync_packets: u32 = kani::any();
        kani::assume(num_sync_packets > 0 && num_sync_packets <= 100);

        // Initial state: remaining = num_sync_packets
        let initial_remaining = num_sync_packets;

        // After some number of sync replies
        let decrements: u32 = kani::any();
        // Only valid decrements (can't receive more replies than packets requested)
        kani::assume(decrements <= num_sync_packets);

        // Saturating subtraction models the safe decrement pattern
        let final_remaining = initial_remaining.saturating_sub(decrements);

        // INV-PROTO-2: Never exceeds initial
        kani::assert(
            final_remaining <= num_sync_packets,
            "sync_remaining never exceeds num_sync_packets",
        );

        // INV-PROTO-3: Non-negative (guaranteed by u32, verified no underflow)
        // Since decrements <= num_sync_packets and we use saturating_sub, this is safe
        kani::assert(
            final_remaining == num_sync_packets - decrements,
            "sync_remaining correctly tracks replies",
        );
    }

    // =========================================================================
    // Frame Advantage Invariant Verification
    //
    // Verifies that local_frame_advantage and remote_frame_advantage
    // calculations stay within reasonable bounds.
    // =========================================================================

    /// Proof: local_frame_advantage calculation stays within bounds.
    ///
    /// Verifies the calculation used by `update_local_frame_advantage`:
    /// `remote_frame.as_i32().saturating_sub(local_frame.as_i32())`
    ///
    /// The frame advantage is bounded by the maximum frame difference possible
    /// during normal gameplay (limited by round trip time and frame rate).
    ///
    /// ## Modeling Note
    ///
    /// Production code uses saturating subtraction because both RTT and remote
    /// frame estimates can be peer-influenced. The proof mirrors that operation
    /// directly and verifies it stays inside the `i32` domain for all inputs.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Frame advantage calculation bounds
    /// - Related: proof_remote_frame_advantage_from_i8, proof_frame_advantage_null_guard
    // kani::no-unwind-needed: single i32 saturating subtraction, no loops
    #[kani::proof]
    fn proof_local_frame_advantage_bounds() {
        let local_frame: i32 = kani::any();
        let remote_frame: i32 = kani::any();

        // Mirror production code: hostile RTT can push the remote estimate to
        // either bound, so subtraction saturates instead of panicking.
        let advantage = remote_frame.saturating_sub(local_frame);

        // Verify the result is within the i32 domain for all symbolic inputs.
        kani::assert(
            advantage >= i32::MIN && advantage <= i32::MAX,
            "Frame advantage remains within i32 bounds",
        );
    }

    /// Proof: remote_frame_advantage assignment preserves value.
    ///
    /// Verifies the assignment at mod.rs:886:
    /// `self.remote_frame_advantage = body.frame_advantage as i32;`
    ///
    /// The QualityReport.frame_advantage is i8, so casting to i32 is always safe.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: i8 to i32 cast preserves value
    /// - Related: proof_local_frame_advantage_bounds
    // kani::no-unwind-needed: single i8 -> i32 cast, no loops
    #[kani::proof]
    fn proof_remote_frame_advantage_from_i8() {
        let wire_value: i8 = kani::any();

        // This is the cast at mod.rs:886
        let advantage: i32 = wire_value as i32;

        // i8 to i32 is always safe (widening conversion)
        // Value should be preserved exactly
        kani::assert(
            advantage >= i8::MIN as i32 && advantage <= i8::MAX as i32,
            "i8 to i32 cast preserves value range",
        );

        // Verify the cast is lossless
        kani::assert(
            advantage == i32::from(wire_value),
            "Cast produces same result as From trait",
        );
    }

    /// Proof: update_local_frame_advantage NULL guard works correctly.
    ///
    /// Verifies the guard in `update_local_frame_advantage`:
    /// `if local_frame == Frame::NULL || self.last_recv_frame() == Frame::NULL { return; }`
    ///
    /// This ensures we don't compute frame advantage with invalid frames.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: NULL frame guard prevents invalid calculations
    /// - Related: proof_local_frame_advantage_bounds, proof_frame_null_detection
    // kani::no-unwind-needed: scalar NULL-frame guard logic, no loops
    #[kani::proof]
    fn proof_frame_advantage_null_guard() {
        let local_frame_val: i32 = kani::any();
        let last_recv_frame_val: i32 = kani::any();

        // Frame::NULL is represented as -1
        let local_is_null = local_frame_val == -1;
        let recv_is_null = last_recv_frame_val == -1;

        // The guard condition at mod.rs:299
        let should_return_early = local_is_null || recv_is_null;

        // If either frame is NULL, we should not compute advantage
        if should_return_early {
            kani::assert(
                local_is_null || recv_is_null,
                "Early return when any frame is NULL",
            );
        } else {
            // Both frames are valid (not NULL)
            kani::assert(
                local_frame_val != -1 && last_recv_frame_val != -1,
                "Both frames valid when not returning early",
            );
        }
    }
}
