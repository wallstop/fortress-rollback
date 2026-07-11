//! Deterministic whole-mesh simulation harness.
//!
//! Runs N real [`P2PSession`]s in one process over a [`SimNet`] fabric under
//! a single virtual clock, drives them with a materialized [`Schedule`], and
//! checks global invariants with the [`Oracle`]. Everything reproduces from
//! `(seed, SimConfig)`: sessions get seeded protocol RNGs, the network gets a
//! seeded fault stream, inputs are a pure function of `(step, peer)`, and the
//! step loop iterates peers in a fixed order.
//!
//! A run's [`RunReport::trace_hash`] folds every observable step artifact;
//! two runs of the same schedule must produce identical hashes (checked by
//! the meta-determinism test in the fleet).

// Test infrastructure: not every test binary uses every helper.
#![allow(dead_code)]

pub mod artifact;
pub mod oracle;
pub mod schedule;
pub mod shrink;

use crate::common::sim_net::{SimNet, SimPayloadMetadata, SimSocket, SimSocketBinding};
use crate::common::stubs::{StateStub, StubConfig, StubInput};
use crate::common::test_clock::TestClock;
use fortress_rollback::hash::{fnv1a_hash, DeterministicHasher};
#[cfg(feature = "hot-join")]
use fortress_rollback::rng::{Pcg32, SeedableRng};
use fortress_rollback::telemetry::CollectingObserver;
use fortress_rollback::{
    Config, DesyncDetection, EventKind, FortressError, FortressEvent, FortressRequest, Frame,
    GameStateCell, InputStatus, InputVec, Message, MessageKind, P2PSession, PeerMetrics,
    PlayerHandle, PlayerType, ProtocolConfig, RequestVec, SessionBuilder, SessionState,
    SpectatorSession,
};
use oracle::{
    validate_violation_allowlist, HealLiveness, InputFingerprint, Oracle, OracleFailure, Verdict,
    ViolationSignature, ViolationSource, DEFAULT_VIOLATION_ALLOWLIST, POST_HEAL_MIN_ADVANCE,
};
use schedule::{hot_join_host_for_slot, validate_schedule, AppModel, Schedule, ScheduleEvent};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::hash::Hasher as _;
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

/// Input contract used by the deterministic simulation harness.
///
/// The production library already supports arbitrary fixed-width `Config::Input`
/// types; this trait keeps the harness's game/oracle semantics stable while the
/// M2 sweep varies only serialized input width.
pub trait SimInput:
    Copy + Clone + PartialEq + Eq + Default + Serialize + DeserializeOwned + Send + Sync + 'static
{
    type SessionConfig: Config<Input = Self, State = StateStub, Address = SocketAddr>
        + std::fmt::Debug;

    /// Serialized byte width of one input value under the crate's fixed-int
    /// bincode codec.
    const WIDTH_BYTES: u32;

    /// Deterministic input for `(step, peer)`.
    fn from_word(word: u32, step: u32, peer: usize) -> Self;

    /// State-transition value used by the harness oracle. This intentionally
    /// stays 32-bit for every input width so wide-input sweep cells isolate wire
    /// cost instead of changing game behavior.
    fn value(self) -> u32;

    /// Full serialized input identity observed by the oracle.
    fn fingerprint(self) -> InputFingerprint;
}

impl SimInput for StubInput {
    type SessionConfig = StubConfig;

    const WIDTH_BYTES: u32 = 4;

    fn from_word(word: u32, _step: u32, _peer: usize) -> Self {
        Self { inp: word }
    }

    fn value(self) -> u32 {
        self.inp
    }

    fn fingerprint(self) -> InputFingerprint {
        InputFingerprint::from_bytes(self.inp, &self.inp.to_le_bytes())
    }
}

/// A 32-byte fixed-width input for the M2 sweep width axis.
///
/// The first word drives the same game-state transition as [`StubInput`]; the
/// seven padding words are deterministic, varying payload so the bandwidth
/// counters measure a real wide input stream rather than a zero-filled artifact.
#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WideStubInput {
    pub inp: u32,
    pub padding: [u32; 7],
}

#[derive(Debug)]
pub struct WideStubConfig;

impl Config for WideStubConfig {
    type Input = WideStubInput;
    type State = StateStub;
    type Address = SocketAddr;
}

impl SimInput for WideStubInput {
    type SessionConfig = WideStubConfig;

    const WIDTH_BYTES: u32 = 32;

    fn from_word(word: u32, step: u32, peer: usize) -> Self {
        let mut padding = [0u32; 7];
        let peer_word = u32::try_from(peer).unwrap_or(u32::MAX);
        for (i, slot) in padding.iter_mut().enumerate() {
            let salt = u32::try_from(i).unwrap_or(u32::MAX).wrapping_add(1);
            *slot = word
                .rotate_left(salt)
                .wrapping_add(step.wrapping_mul(17))
                .wrapping_add(peer_word.wrapping_mul(97))
                .wrapping_add(salt.wrapping_mul(0x9E37));
        }
        Self { inp: word, padding }
    }

    fn value(self) -> u32 {
        self.inp
    }

    fn fingerprint(self) -> InputFingerprint {
        let mut bytes = [0u8; 32];
        let words = [
            self.inp,
            self.padding[0],
            self.padding[1],
            self.padding[2],
            self.padding[3],
            self.padding[4],
            self.padding[5],
            self.padding[6],
        ];
        for (chunk, word) in bytes.chunks_exact_mut(4).zip(words) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        InputFingerprint::from_bytes(self.inp, &bytes)
    }
}

/// Options for fault-injection *inside the harness itself* — used by the
/// oracle's negative controls to prove the invariants actually fire.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunOptions {
    /// Corrupt `(peer, from_frame)`'s simulated **state** from that frame on
    /// (a real divergence: state, checksums, and downstream frames all split).
    pub corrupt_state_from: Option<(usize, i32)>,
    /// Corrupt `(peer, from_frame)`'s saved **checksums only** (states stay
    /// identical): exercises the in-band detector cross-check path.
    pub corrupt_checksum_from: Option<(usize, i32)>,
    /// If set, snapshot every peer's confirmed frame and every directed link's
    /// wire counters at the end of this step into [`RunReport::probe_confirmed`]
    /// and [`RunReport::probe_peer_wire_by_link`]. Lets a test observe mid-run
    /// confirmation and protocol traffic; end-of-run state alone hides those
    /// dynamics because a clean drain always converges. The probe contributes
    /// to trace/shrinker identity and must be within `0..steps`.
    pub probe_confirmed_at: Option<u32>,
    /// If set, sample the sender's exact protocol `pending_output_len` gauge
    /// for one directed `(from, to)` link after every simulation step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_output_probe_link: Option<(usize, usize)>,
    /// Corrupt the configured spectator's first input fingerprint in each
    /// displayed frame at or after this frame. Negative controls only need one
    /// planted spectator-only mismatch to prove the §6.2(d) oracle compares
    /// the spectator path, not only the mesh peers.
    pub corrupt_spectator_input_from: Option<i32>,
    /// Corrupt the first displayed `Disconnected` spectator slot from this
    /// frame onward by reporting it as `Confirmed`. Negative controls use this
    /// to pin the dropped-slot status half of §6.2(d).
    pub corrupt_spectator_status_from: Option<i32>,
}

/// End-of-step pending-output evidence for one directed protocol link.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingOutputProbe {
    pub from: usize,
    pub to: usize,
    /// Queue capacity used by every protocol endpoint in the harness.
    #[serde(default)]
    pub limit: u64,
    /// First complete end-of-step cut at which the queue reached `limit`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_reached_limit_at: Option<u32>,
    pub at_probe: Option<u64>,
    pub max: u64,
    pub at_heal: Option<u64>,
    pub after_recovery: Option<u64>,
    pub final_value: u64,
}

/// Payload identity for the endpoint-bearing peer events the harness records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PeerEventPayload {
    Addr(SocketAddr),
    PlayerAddr {
        handle: PlayerHandle,
        addr: SocketAddr,
    },
}

/// Key used by census rows that need to prove which recorded endpoint an event named.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeerEventKey {
    pub kind: EventKind,
    pub payload: PeerEventPayload,
}

/// One `LoadGameState` request observed while driving a peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadGameStateObservation {
    pub step: u32,
    pub peer: usize,
    pub frame: i32,
}

/// Maximum number of final simulation steps retained in a failure artifact.
pub const TRACE_TAIL_CAPACITY: usize = 64;
/// Maximum scheduled or observed event summaries retained per trace step.
pub const TRACE_STEP_EVENT_CAPACITY: usize = 32;
/// Maximum UTF-8 bytes retained for one event summary field.
pub const TRACE_EVENT_TEXT_CAPACITY: usize = 512;

/// Serializable mirror of [`SessionState`] for stable failure artifacts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceSessionState {
    Synchronizing,
    Running,
    #[cfg(feature = "hot-join")]
    HotJoining,
}

/// Serializable game state summary for one peer at one step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceGameState {
    pub frame: i32,
    pub value: i32,
}

/// Source of one drained event in a trace snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceEventSource {
    Peer(usize),
    Spectator,
}

/// Compact stable event summary retained at a fault/effect boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceObservedEvent {
    pub source: TraceEventSource,
    pub kind: String,
    pub details: String,
}

/// Serializable mirror of cumulative [`crate::common::sim_net::SimNetStats`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceNetStats {
    pub sent: u64,
    pub delivered: u64,
    pub dropped_by_policy: u64,
    pub retransmit_delayed: u64,
    pub dropped_blocked: u64,
    pub dropped_unattached: u64,
    pub duplicated: u64,
    pub held: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub gilbert_elliott_good_to_bad: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub gilbert_elliott_bad_to_good: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub gilbert_elliott_good_sends: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub gilbert_elliott_bad_sends: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub gilbert_elliott_loss_events: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub gilbert_elliott_max_loss_run: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub fragmentation_eligible_sends: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub fragmentation_fragments_modeled: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub fragmentation_lost_fragments: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub fragmentation_loss_events: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub fragmentation_input_eligible_sends: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub fragmentation_input_loss_events: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub fragmentation_max_packet_bytes: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub fragmentation_max_fragments_per_send: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub fragmentation_fragment_cap_hits: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub bandwidth_admitted_datagrams: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub bandwidth_admitted_bytes: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub bandwidth_queued_datagrams: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub bandwidth_tail_drops: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub bandwidth_oversize_drops: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub bandwidth_reservation_cap_drops: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub bandwidth_reservation_cap_dropped_bytes: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub bandwidth_tail_dropped_bytes: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub bandwidth_max_queue_bytes: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub bandwidth_max_queue_delay_ns: u64,
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

impl From<crate::common::sim_net::SimNetStats> for TraceNetStats {
    fn from(stats: crate::common::sim_net::SimNetStats) -> Self {
        Self {
            sent: stats.sent,
            delivered: stats.delivered,
            dropped_by_policy: stats.dropped_by_policy,
            retransmit_delayed: stats.retransmit_delayed,
            dropped_blocked: stats.dropped_blocked,
            dropped_unattached: stats.dropped_unattached,
            duplicated: stats.duplicated,
            held: stats.held,
            gilbert_elliott_good_to_bad: stats.gilbert_elliott_good_to_bad,
            gilbert_elliott_bad_to_good: stats.gilbert_elliott_bad_to_good,
            gilbert_elliott_good_sends: stats.gilbert_elliott_good_sends,
            gilbert_elliott_bad_sends: stats.gilbert_elliott_bad_sends,
            gilbert_elliott_loss_events: stats.gilbert_elliott_loss_events,
            gilbert_elliott_max_loss_run: stats.gilbert_elliott_max_loss_run,
            fragmentation_eligible_sends: stats.fragmentation_eligible_sends,
            fragmentation_fragments_modeled: stats.fragmentation_fragments_modeled,
            fragmentation_lost_fragments: stats.fragmentation_lost_fragments,
            fragmentation_loss_events: stats.fragmentation_loss_events,
            fragmentation_input_eligible_sends: stats.fragmentation_input_eligible_sends,
            fragmentation_input_loss_events: stats.fragmentation_input_loss_events,
            fragmentation_max_packet_bytes: stats.fragmentation_max_packet_bytes,
            fragmentation_max_fragments_per_send: stats.fragmentation_max_fragments_per_send,
            fragmentation_fragment_cap_hits: stats.fragmentation_fragment_cap_hits,
            bandwidth_admitted_datagrams: stats.bandwidth_admitted_datagrams,
            bandwidth_admitted_bytes: stats.bandwidth_admitted_bytes,
            bandwidth_queued_datagrams: stats.bandwidth_queued_datagrams,
            bandwidth_tail_drops: stats.bandwidth_tail_drops,
            bandwidth_oversize_drops: stats.bandwidth_oversize_drops,
            bandwidth_reservation_cap_drops: stats.bandwidth_reservation_cap_drops,
            bandwidth_reservation_cap_dropped_bytes: stats.bandwidth_reservation_cap_dropped_bytes,
            bandwidth_tail_dropped_bytes: stats.bandwidth_tail_dropped_bytes,
            bandwidth_max_queue_bytes: stats.bandwidth_max_queue_bytes,
            bandwidth_max_queue_delay_ns: stats.bandwidth_max_queue_delay_ns,
        }
    }
}

impl From<SessionState> for TraceSessionState {
    fn from(state: SessionState) -> Self {
        match state {
            SessionState::Synchronizing => Self::Synchronizing,
            SessionState::Running => Self::Running,
            #[cfg(feature = "hot-join")]
            SessionState::HotJoining => Self::HotJoining,
        }
    }
}

/// One stable, bounded end-of-step snapshot retained for failure diagnosis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceSnapshot {
    pub step: u32,
    pub confirmed_frames: Vec<i32>,
    pub session_states: Vec<TraceSessionState>,
    pub dead: Vec<bool>,
    pub game_states: Vec<TraceGameState>,
    pub scheduled_events: Vec<String>,
    pub scheduled_events_truncated: u32,
    pub observed_events: Vec<TraceObservedEvent>,
    pub observed_events_truncated: u32,
    pub net: TraceNetStats,
    pub spectator: Option<TraceSpectatorState>,
}

/// Stable per-step spectator progress included in trace identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceSpectatorState {
    pub current_frame: i32,
    pub num_hosts: usize,
    pub applied_frames: usize,
    pub max_applied_frame: Option<i32>,
}

#[derive(Serialize)]
struct TracePeerWireLink {
    from: usize,
    to: usize,
    totals: PeerWireTotals,
}

#[derive(Serialize)]
struct TraceLinkStats {
    from: usize,
    to: usize,
    stats: crate::common::sim_net::SimLinkStats,
}

#[derive(Serialize)]
struct TraceLinkCount {
    from: usize,
    to: usize,
    count: u64,
}

fn trace_fragmentation_drops(
    schema_version: u32,
    counts: &BTreeMap<(usize, usize), u64>,
) -> Vec<TraceLinkCount> {
    if schema_version < 13 {
        return Vec::new();
    }
    counts
        .iter()
        .map(|(&(from, to), &count)| TraceLinkCount { from, to, count })
        .collect()
}

fn trace_link_stats(
    schema_version: u32,
    stats: &BTreeMap<(usize, usize), crate::common::sim_net::SimLinkStats>,
) -> Vec<TraceLinkStats> {
    if schema_version < 13 {
        return Vec::new();
    }
    stats
        .iter()
        .map(|(&(from, to), &stats)| TraceLinkStats { from, to, stats })
        .collect()
}

#[derive(Serialize)]
struct TraceFinalSummary {
    failure_classes: Vec<&'static str>,
    final_confirmed: Vec<i32>,
    probe_confirmed: Vec<i32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    probe_peer_wire_by_link: Vec<TracePeerWireLink>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fragmentation_drops_by_link: Vec<TraceLinkCount>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    link_stats_by_link: Vec<TraceLinkStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending_output_probe: Option<PendingOutputProbe>,
    confirmed_at_heal: Vec<i32>,
    confirmed_after_recovery: Vec<i32>,
    recovered_within_b: Option<bool>,
    spectator_applied_frames: usize,
    spectator_max_frame: Option<i32>,
    spectator_final_hosts: Option<usize>,
    net: TraceNetStats,
}

/// Outcome of one simulation run.
#[derive(Clone, Debug)]
pub struct RunReport {
    /// Exact harness fault/probe options required to reproduce this run.
    pub replay_options: RunOptions,
    /// Serialized input width selecting the deterministic harness input type.
    pub replay_input_width_bytes: u32,
    pub verdict: Verdict,
    /// Deterministic digest of the run's observable trace.
    pub trace_hash: u64,
    /// Each peer's final confirmed frame.
    pub final_confirmed: Vec<i32>,
    /// Final [`TRACE_TAIL_CAPACITY`] end-of-step snapshots.
    pub trace_tail: Vec<TraceSnapshot>,
    /// Each peer's confirmed frame sampled at [`RunOptions::probe_confirmed_at`]
    /// (empty when no probe step was requested). Indexed by peer.
    pub probe_confirmed: Vec<i32>,
    /// Per-directed-link wire counters sampled after every peer has completed
    /// the step selected by [`RunOptions::probe_confirmed_at`]. Empty when no
    /// probe was requested. Keys are `(local_peer, remote_peer)`.
    pub probe_peer_wire_by_link: BTreeMap<(usize, usize), PeerWireTotals>,
    /// End-of-step protocol backlog evidence for the requested directed link.
    pub pending_output_probe: Option<PendingOutputProbe>,
    /// Network delivery/drop counters.
    pub net_stats: crate::common::sim_net::SimNetStats,
    /// Blocked-drop counts split by directed peer index pair `(from, to)`.
    pub blocked_drops_by_link: BTreeMap<(usize, usize), u64>,
    /// Fragmentation-loss counts split by directed peer index pair `(from, to)`.
    pub fragmentation_drops_by_link: BTreeMap<(usize, usize), u64>,
    /// Size and policy telemetry split by directed peer index pair `(from, to)`.
    pub link_stats_by_link: BTreeMap<(usize, usize), crate::common::sim_net::SimLinkStats>,
    /// `LoadGameState` requests observed while driving peers.
    pub load_game_state_observations: Vec<LoadGameStateObservation>,
    /// Each peer's final [`SessionMetrics`] snapshot (indexed by peer).
    pub metrics: Vec<fortress_rollback::SessionMetrics>,
    /// Each peer's wire-traffic totals, aggregated over all of that peer's
    /// remote links from the always-on `PeerMetrics` counters (indexed by peer).
    /// This is the per-player bandwidth ledger the M2 baseline sweep consumes.
    pub peer_wire: Vec<PeerWireTotals>,
    /// (c) each peer's confirmed frame sampled at the heal anchor — the step of
    /// the last actual `ScheduleEvent::HealAll` (derived from the event stream,
    /// not `schedule.heal_at`, which can drift or be set without a `HealAll`);
    /// empty when the schedule never heals. Indexed by peer.
    pub confirmed_at_heal: Vec<i32>,
    /// (c) each peer's confirmed frame sampled at the recovery anchor — B steps
    /// after the heal, or the run's last step when that lands past the end (an
    /// exact-boundary drain, span B-1); empty when the schedule never heals.
    /// Indexed by peer.
    pub confirmed_after_recovery: Vec<i32>,
    /// (i) metastability: `Some(true/false)` iff the (c) bounded post-heal
    /// liveness check ran — a `HealAll` fired and both anchors are observable (a
    /// full recovery window; span B, or B-1 at an exact-boundary drain). The
    /// explicit "recovered within B steps of heal: yes/no". `None` when (c) was
    /// inert (no heal), the window was too short to observe, or every peer was
    /// killed. Mirrors [`Verdict::recovered_within_b`].
    pub recovered_within_b: Option<bool>,
    /// All telemetry violations observed by every peer, before the oracle's
    /// severity/allowlist policy is applied. Used by the §6.2(f) violation
    /// census so warning-only signatures stay visible even though they do not
    /// fail the run.
    pub violation_census: BTreeMap<ViolationSignature, u64>,
    /// User-facing peer events drained by the harness, counted by category.
    /// Census rows use this to assert that a schedule exercised ordinary event
    /// surfaces (for example `NetworkInterrupted`/`NetworkResumed`) instead of
    /// only relying on end-state convergence.
    pub peer_event_counts: BTreeMap<EventKind, u64>,
    /// Same event counts split by observing peer. Indexed by peer.
    pub peer_event_counts_by_peer: Vec<BTreeMap<EventKind, u64>>,
    /// Same event counts split by observing peer and relevant event payload.
    /// Indexed by observing peer.
    pub peer_event_payload_counts_by_peer: Vec<BTreeMap<PeerEventKey, u64>>,
    /// Number of frames the configured spectator displayed and handed to the
    /// oracle. Zero when `SimConfig::spectator_hosts` is empty.
    pub spectator_applied_frames: usize,
    /// Highest frame the configured spectator displayed. `None` when
    /// `SimConfig::spectator_hosts` is empty or the spectator never advanced.
    pub spectator_max_frame: Option<i32>,
    /// Number of redundant hosts the configured spectator still had at the end of
    /// the run. `None` when `SimConfig::spectator_hosts` is empty.
    pub spectator_final_hosts: Option<usize>,
}

/// One peer's cumulative wire traffic, summed across every remote link it holds.
///
/// The mesh runner reads each peer session's per-remote [`PeerMetrics`] at
/// end-of-run and folds them into these totals, so a single value describes how
/// much a player put on / took off the wire regardless of mesh size. Byte counts
/// are wire-exact and payload-only (they match `PeerMetrics`, excluding UDP/IP
/// headers). The `messages_{sent,received}_by_kind` arrays are positional in
/// [`MessageKind::ALL`] order; read them by category with
/// [`sent_by_kind`](Self::sent_by_kind) / [`received_by_kind`](Self::received_by_kind).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PeerWireTotals {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub messages_sent_by_kind: [u64; MessageKind::COUNT],
    pub messages_received_by_kind: [u64; MessageKind::COUNT],
    pub input_bytes_pre_compression: u64,
    pub input_bytes_post_compression: u64,
}

impl PeerWireTotals {
    /// Folds one remote link's [`PeerMetrics`] snapshot into these totals.
    ///
    /// Cumulative counters add; the trailing gauges (`pending_*`, `ping_ms`,
    /// `remote_frame_advantage`) are deliberately dropped — an instantaneous
    /// gauge is not additive across links.
    fn add(&mut self, m: &PeerMetrics) {
        self.bytes_sent = self.bytes_sent.saturating_add(m.bytes_sent);
        self.bytes_received = self.bytes_received.saturating_add(m.bytes_received);
        self.packets_sent = self.packets_sent.saturating_add(m.packets_sent);
        self.packets_received = self.packets_received.saturating_add(m.packets_received);
        // Both arrays are laid out in `MessageKind::ALL` order (the same order
        // used to read them back), independent of the crate-private
        // `MessageKind::index`.
        for (slot, kind) in self.messages_sent_by_kind.iter_mut().zip(MessageKind::ALL) {
            *slot = slot.saturating_add(m.messages_sent_by_kind.get(kind));
        }
        for (slot, kind) in self
            .messages_received_by_kind
            .iter_mut()
            .zip(MessageKind::ALL)
        {
            *slot = slot.saturating_add(m.messages_received_by_kind.get(kind));
        }
        self.input_bytes_pre_compression = self
            .input_bytes_pre_compression
            .saturating_add(m.input_bytes_pre_compression);
        self.input_bytes_post_compression = self
            .input_bytes_post_compression
            .saturating_add(m.input_bytes_post_compression);
    }

    /// Messages of `kind` sent, summed across links.
    #[must_use]
    pub fn sent_by_kind(&self, kind: MessageKind) -> u64 {
        MessageKind::ALL
            .iter()
            .position(|k| *k == kind)
            .and_then(|i| self.messages_sent_by_kind.get(i).copied())
            .unwrap_or(0)
    }

    /// Messages of `kind` received, summed across links.
    #[must_use]
    pub fn received_by_kind(&self, kind: MessageKind) -> u64 {
        MessageKind::ALL
            .iter()
            .position(|k| *k == kind)
            .and_then(|i| self.messages_received_by_kind.get(i).copied())
            .unwrap_or(0)
    }

    /// Total messages sent across all kinds (equals [`Self::packets_sent`]).
    #[must_use]
    pub fn sent_by_kind_total(&self) -> u64 {
        self.messages_sent_by_kind
            .iter()
            .copied()
            .fold(0u64, u64::saturating_add)
    }

    /// Total messages received across all kinds (equals [`Self::packets_received`]).
    #[must_use]
    pub fn received_by_kind_total(&self) -> u64 {
        self.messages_received_by_kind
            .iter()
            .copied()
            .fold(0u64, u64::saturating_add)
    }
}

fn collect_peer_wire_by_link<I: SimInput>(
    peers: &[PeerSlot<I>],
    n_players: usize,
) -> BTreeMap<(usize, usize), PeerWireTotals> {
    let mut by_link = BTreeMap::new();
    for (local, slot) in peers.iter().enumerate() {
        for remote in 0..n_players {
            if remote == local {
                continue;
            }
            let mut totals = PeerWireTotals::default();
            // A lifecycle event may retire this handle from the local peer
            // before the probe cut. Preserve the complete directed-link key
            // set and report zero for that no-longer-remote edge; wire probing
            // is observational and must not turn valid retirement into a
            // harness panic.
            if let Ok(metrics) = slot.session.peer_metrics(PlayerHandle::new(remote)) {
                totals.add(&metrics);
            }
            by_link.insert((local, remote), totals);
        }
    }
    by_link
}

impl RunReport {
    /// Panics with a reproducible failure report if the run failed.
    #[track_caller]
    pub fn expect_pass(&self, schedule: &Schedule) {
        if self.verdict.passed() {
            return;
        }
        let test_name = std::thread::current()
            .name()
            .map_or_else(|| "unknown-test".to_owned(), ToOwned::to_owned);
        let artifact_status = match artifact::write_report_artifact(&test_name, schedule, self) {
            Ok(path) => format!("artifact={}", path.display()),
            Err(error) => format!("artifact_write_error={error}"),
        };
        panic!(
            "simulation failed — reproduce with:\n  FORTRESS_SIM_REPRO seed={} n_players={} steps={} noise={:?}\n{}\nfinal_confirmed={:?}\nrecovered_within_b={:?}\nspectator_applied_frames={}\nspectator_max_frame={:?}\nspectator_final_hosts={:?}\nallowlist_hits={:?}\nviolation_census={:?}\nnet={:?}\nfailures ({}):\n{}",
            schedule.seed,
            schedule.config.n_players,
            schedule.config.steps,
            schedule.config.noise,
            artifact_status,
            self.final_confirmed,
            self.recovered_within_b,
            self.spectator_applied_frames,
            self.spectator_max_frame,
            self.spectator_final_hosts,
            self.verdict.violation_allowlist_hits,
            self.violation_census,
            self.net_stats,
            self.verdict.failures.len(),
            self.verdict
                .failures
                .iter()
                .map(|f| format!("  - {f:?}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// The harness's game stub: `GameStub` semantics (shared `StateStub`
/// transition) plus recording and the negative-control corruption hooks.
struct SimGameStub<I: SimInput> {
    gs: StateStub,
    /// Post-advance state per frame; last write wins (rollback re-simulation
    /// overwrites), so confirmed frames hold their final state.
    recorded: BTreeMap<i32, StateStub>,
    /// Applied inputs per simulated frame; last write wins just like
    /// [`Self::recorded`], so a rollback re-simulation replaces stale transient
    /// statuses with the end-of-run truth. Used by the oracle's dropped-slot
    /// freeze-frame convergence check.
    applied_inputs: BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>,
    corrupt_state_from: Option<i32>,
    corrupt_checksum_from: Option<i32>,
    input_marker: PhantomData<I>,
}

impl<I: SimInput> SimGameStub<I> {
    fn new() -> Self {
        Self {
            gs: StateStub { frame: 0, state: 0 },
            recorded: BTreeMap::new(),
            applied_inputs: BTreeMap::new(),
            corrupt_state_from: None,
            corrupt_checksum_from: None,
            input_marker: PhantomData,
        }
    }

    fn prune_replacement_generation(&mut self, from_frame: i32) {
        self.recorded.retain(|frame, _| *frame < from_frame);
        self.applied_inputs.retain(|frame, _| *frame < from_frame);
    }

    fn loaded_frame(requests: &RequestVec<I::SessionConfig>) -> Option<Frame> {
        requests.iter().find_map(|request| match request {
            FortressRequest::LoadGameState { frame, .. } => Some(*frame),
            FortressRequest::SaveGameState { .. } | FortressRequest::AdvanceFrame { .. } => None,
        })
    }

    fn handle_replacement_handoff_requests(
        &mut self,
        requests: RequestVec<I::SessionConfig>,
        handoff_floor: i32,
    ) -> Option<Frame> {
        let loaded = Self::loaded_frame(&requests);
        if let Some(frame) = loaded {
            self.prune_replacement_generation(handoff_floor.min(frame.as_i32()));
        }
        self.handle_requests(requests);
        loaded
    }

    fn handle_requests(&mut self, requests: RequestVec<I::SessionConfig>) {
        for request in requests {
            match request {
                FortressRequest::LoadGameState { cell, .. } => {
                    self.load(&cell);
                },
                FortressRequest::SaveGameState { cell, frame } => self.save(&cell, frame),
                FortressRequest::AdvanceFrame { inputs } => self.advance(&inputs),
            }
        }
    }

    fn save(&self, cell: &GameStateCell<StateStub>, frame: Frame) {
        assert_eq!(self.gs.frame, frame.as_i32(), "save/state frame mismatch");
        let real_checksum = u128::from(fnv1a_hash(&self.gs));
        let checksum = match self.corrupt_checksum_from {
            Some(from) if frame.as_i32() >= from => real_checksum ^ 0xDEAD_BEEF_CAFE_BABE_u128,
            _ => real_checksum,
        };
        cell.save(frame, Some(self.gs), Some(checksum));
    }

    fn load(&mut self, cell: &GameStateCell<StateStub>) {
        self.gs = cell.load().expect("harness stub: missing saved state");
    }

    fn advance(&mut self, inputs: &InputVec<I>) {
        let frame = self.gs.frame;
        self.applied_inputs.insert(
            frame,
            inputs
                .iter()
                .map(|(input, status)| (input.fingerprint(), *status))
                .collect(),
        );

        // Same transition as GameStub/StateStub::advance_frame.
        let total: u32 = inputs.iter().map(|(input, _)| input.value()).sum();
        if total % 2 == 0 {
            self.gs.state += 2;
        } else {
            self.gs.state -= 1;
        }
        self.gs.frame += 1;

        if let Some(from) = self.corrupt_state_from {
            if self.gs.frame >= from {
                // A deterministic wrong turn: diverges the state transition
                // from this frame onward, exactly like a real determinism bug.
                self.gs.state ^= 1;
            }
        }
        self.recorded.insert(self.gs.frame, self.gs);
    }
}

struct PeerSlot<I: SimInput> {
    session: P2PSession<I::SessionConfig>,
    binding: SimSocketBinding<Message>,
    /// Canonical and rebound source addresses used by this logical peer.
    source_addrs: Vec<SocketAddr>,
    game: SimGameStub<I>,
    observer: Arc<CollectingObserver>,
    /// Highest frame whose confirmed inputs were sampled into the oracle.
    sampled_confirmed: i32,
    /// RNG seed used to construct this peer's current protocol generation.
    protocol_rng_seed: u64,
    /// A hot-joined replacement starts from a mid-game snapshot and cannot
    /// answer historical `confirmed_inputs_for_frame` queries below that
    /// snapshot. While armed, this stores the earliest frame the coordinator's
    /// clean-drop handoff may rewrite; the drive loop waits for the
    /// replacement's first `LoadGameState { frame }`, prunes the replacement
    /// generation from the earlier of those two boundaries, then resumes
    /// confirmed-input sampling above the loaded snapshot frame.
    pending_replacement_handoff_floor: Option<i32>,
}

struct SpectatorSlot<I: SimInput> {
    session: SpectatorSession<I::SessionConfig>,
    observer: Arc<CollectingObserver>,
    applied_inputs: BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>,
}

fn peer_protocol_config(schedule: &Schedule, peer: usize, clock: &TestClock) -> ProtocolConfig {
    // Per-peer clock: the exact base clock at 0 ppm (byte-identical to
    // before), or a rate-skewed clock modeling an unsynchronized local clock
    // (H-SKEW). A missing/short skew vector means "no skew".
    let ppm = schedule
        .config
        .clock_skew_ppm
        .get(peer)
        .copied()
        .unwrap_or(0);
    let peer_clock = if ppm == 0 {
        clock.as_protocol_clock()
    } else {
        // ratio (1e6 + ppm) / 1e6. `ppm == -1_000_000` (-100%) is a
        // frozen clock (num = 0); anything below that would run time
        // backwards and is rejected up front, so the fallback is unused.
        let num = u64::try_from(1_000_000_i64 + i64::from(ppm)).unwrap_or(0);
        clock.as_skewed_protocol_clock(num, 1_000_000)
    };

    ProtocolConfig {
        clock: Some(peer_clock),
        protocol_rng_seed: Some(peer_protocol_seed(schedule, peer)),
        ..ProtocolConfig::default()
    }
}

fn peer_protocol_seed(schedule: &Schedule, peer: usize) -> u64 {
    if schedule.schema_version < 12 {
        fnv1a_hash(&(schedule.seed, peer))
    } else {
        let peer = u64::try_from(peer).expect("validated peer index fits u64");
        fnv1a_hash(&(schedule.seed, peer))
    }
}

#[cfg(feature = "hot-join")]
fn protocol_magic_for_seed(seed: u64) -> u16 {
    // Keep this in lockstep with UdpProtocol::new: draw the low u16 from
    // Pcg32 and reject zero. The end-to-end hot-join census guards the
    // production side of this test-harness coupling.
    let mut rng = Pcg32::seed_from_u64(seed);
    loop {
        let magic = rng.next_u32() as u16;
        if magic != 0 {
            return magic;
        }
    }
}

#[cfg(feature = "hot-join")]
fn hot_join_protocol_seed_candidate(
    schedule: &Schedule,
    peer: usize,
    step: u32,
    previous_seed: u64,
    nonce: u32,
) -> u64 {
    let peer = u64::try_from(peer).expect("validated peer index fits u64");
    fnv1a_hash(&(
        "hot-join-replacement",
        schedule.seed,
        peer,
        step,
        previous_seed,
        nonce,
    ))
}

/// Derives a deterministic replacement-generation seed whose initial protocol
/// magic differs from the departing generation. Reusing the old seed makes a
/// replacement synchronize prematurely with survivor endpoints from the old
/// era; their later rearm advances its magic and the replacement then filters
/// the genuine new-era handshake forever.
#[cfg(feature = "hot-join")]
fn hot_join_protocol_seed(schedule: &Schedule, peer: usize, step: u32, previous_seed: u64) -> u64 {
    // Schema <=11 replays the original same-seed replacement semantics so
    // checked-in corpus traces keep their historical identity. Schema 12
    // gives each replacement generation a distinct initial protocol magic.
    if schedule.schema_version < 12 {
        return previous_seed;
    }
    let previous_magic = protocol_magic_for_seed(previous_seed);
    for nonce in 0_u32..=u32::from(u16::MAX) {
        let candidate =
            hot_join_protocol_seed_candidate(schedule, peer, step, previous_seed, nonce);
        if protocol_magic_for_seed(candidate) != previous_magic {
            return candidate;
        }
    }
    panic!("failed to derive a distinct protocol magic for hot-join peer {peer} at step {step}");
}

fn update_spectator_required_min_frame<I: SimInput>(
    peers: &[PeerSlot<I>],
    dead: &[bool],
    required_min_frame: &mut Option<i32>,
) {
    let Some(live_floor) = peers
        .iter()
        .enumerate()
        .filter(|(peer, _)| !dead[*peer])
        .map(|(_, slot)| slot.session.current_frame().as_i32())
        .min()
    else {
        return;
    };
    let required = live_floor.saturating_add(POST_HEAL_MIN_ADVANCE);
    *required_min_frame =
        Some(required_min_frame.map_or(required, |current| current.max(required)));
}

fn retire_peer_for_lifecycle<I: SimInput>(
    peer: usize,
    peers: &[PeerSlot<I>],
    dead: &mut [bool],
    oracle: &mut Oracle,
    spectator_required_min_frame: Option<&mut Option<i32>>,
) {
    if dead[peer] {
        return;
    }

    dead[peer] = true;
    peers[peer].binding.detach();
    oracle.mark_peer_dead(peer);
    if let Some(required_min_frame) = spectator_required_min_frame {
        update_spectator_required_min_frame(peers, dead, required_min_frame);
    }
}

/// Deterministic schema-v10 address for peer `peer`'s first NAT rebind.
///
/// TEST-NET-2 is disjoint from the harness's loopback canonical/spectator
/// addresses. Validation permits one rebind per peer, so the peer byte is a
/// complete collision-free generation identifier for the supported mesh cap.
fn rebound_peer_addr(peer: usize) -> SocketAddr {
    let host = u8::try_from(peer.saturating_add(1)).unwrap_or(u8::MAX);
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 51, 100, host)), 35_000)
}

#[cfg(feature = "hot-join")]
struct HotJoinRuntime<'a, I: SimInput> {
    schedule: &'a Schedule,
    clock: &'a TestClock,
    net: &'a SimNet<Message>,
    addrs: &'a [SocketAddr],
    peers: &'a mut [PeerSlot<I>],
    dead: &'a [bool],
    oracle: &'a mut Oracle,
}

#[cfg(feature = "hot-join")]
fn start_hot_join_for_slot<I: SimInput>(slot: usize, step: u32, ctx: &mut HotJoinRuntime<'_, I>) {
    if ctx.dead[slot] {
        ctx.oracle.observe_runner_error(
            "hot_join_slot_unavailable",
            slot,
            step,
            "slot is already retired",
        );
        return;
    }

    let Some(host) = hot_join_host_for_slot(ctx.schedule.config.n_players, slot) else {
        ctx.oracle.observe_runner_error(
            "hot_join_host_unavailable",
            slot,
            step,
            format!(
                "no deterministic coordinator exists for slot {slot} in a {}-peer mesh",
                ctx.schedule.config.n_players
            ),
        );
        return;
    };
    if ctx.dead[host] {
        ctx.oracle.observe_runner_error(
            "hot_join_host_unavailable",
            host,
            step,
            "deterministic coordinator is already retired",
        );
        return;
    }

    let socket: SimSocket<Message> = ctx.net.attach(ctx.addrs[slot]);
    let replacement_binding = socket.binding();
    let observer = Arc::clone(&ctx.peers[slot].observer);
    let replacement_protocol_seed =
        hot_join_protocol_seed(ctx.schedule, slot, step, ctx.peers[slot].protocol_rng_seed);
    let mut protocol_config = peer_protocol_config(ctx.schedule, slot, ctx.clock);
    protocol_config.protocol_rng_seed = Some(replacement_protocol_seed);
    let mut builder = SessionBuilder::<I::SessionConfig>::new()
        .with_num_players(ctx.schedule.config.n_players)
        .expect("valid player count")
        .with_max_prediction_window(ctx.schedule.config.max_prediction)
        .with_input_delay(0)
        .expect("hot-join input delay is fixed at zero")
        .with_save_mode(ctx.schedule.config.save_mode.into())
        .with_desync_detection_mode(DesyncDetection::On {
            interval: ctx.schedule.config.desync_interval,
        })
        .with_disconnect_behavior(ctx.schedule.config.disconnect_behavior.into())
        .with_protocol_config(protocol_config)
        .with_violation_observer(observer as Arc<_>);
    if let Some(size) = ctx.schedule.config.event_queue_size {
        builder = builder.with_event_queue_size(size).unwrap_or_else(|error| {
            panic!(
                "hot-join with_event_queue_size({size}) rejected a pre-validated \
                 size: {error:?}"
            )
        });
    }
    for (peer, addr) in ctx.addrs.iter().enumerate() {
        let player_type = if peer == slot {
            PlayerType::Local
        } else {
            PlayerType::Remote(*addr)
        };
        builder = builder
            .add_player(player_type, PlayerHandle::new(peer))
            .expect("valid hot-join player registration");
    }

    let replacement = match builder.start_hot_join_session(socket, ctx.addrs[host]) {
        Ok(session) => session,
        Err(error) => {
            ctx.oracle
                .observe_session_error("start_hot_join_session", slot, step, &error);
            return;
        },
    };

    let handle = PlayerHandle::new(slot);
    if let Err(error) = ctx.peers[host].session.remove_player(handle) {
        ctx.oracle
            .observe_session_error("hot_join_remove_player", host, step, &error);
        return;
    }

    let handoff_floor = ctx.peers[host]
        .session
        .confirmed_frame()
        .as_i32()
        .saturating_sub(i32::try_from(ctx.schedule.config.max_prediction).unwrap_or(i32::MAX));
    // `handoff_floor` is a post-advance state-frame boundary. Canonical
    // confirmed-input samples are keyed by the input frame that produces the
    // next state frame, so input frame `handoff_floor - 1` is the first sample
    // that can affect replacement-generation state at `handoff_floor`.
    let input_handoff_floor = handoff_floor.saturating_sub(1);
    ctx.oracle
        .begin_replacement_generation(slot, input_handoff_floor);
    // The replacement owns an address-backed `SimSocket`; reset the old peer's
    // inbox, then recreate an empty inbox at the same address for that socket.
    ctx.net.detach(ctx.addrs[slot]);
    let _replacement_inbox = ctx.net.attach(ctx.addrs[slot]);
    ctx.peers[slot].session = replacement;
    ctx.peers[slot].binding = replacement_binding;
    ctx.peers[slot].source_addrs = vec![ctx.addrs[slot]];
    ctx.peers[slot].protocol_rng_seed = replacement_protocol_seed;
    ctx.peers[slot].pending_replacement_handoff_floor = Some(handoff_floor);
}

/// Pure per-peer input function: any deterministic mapping works; this one
/// varies across both axes so prediction is frequently wrong (exercising
/// rollback) and per-peer streams never collide.
fn input_for<I: SimInput>(step: u32, peer: usize) -> I {
    let p = u32::try_from(peer).unwrap_or(0);
    let word = step
        .wrapping_mul(31)
        .wrapping_add(p.wrapping_mul(7))
        .wrapping_add(1);
    I::from_word(word, step, peer)
}

fn corrupt_fingerprint(fingerprint: InputFingerprint) -> InputFingerprint {
    InputFingerprint {
        logical: fingerprint.logical.wrapping_add(1),
        len: fingerprint.len,
        hash: fingerprint.hash ^ 0xA5A5_5A5A_D3C1_B2E0,
    }
}

fn record_spectator_requests<I: SimInput>(
    spectator: &mut SpectatorSlot<I>,
    requests: RequestVec<I::SessionConfig>,
    start_frame: i32,
    corrupt_from: Option<i32>,
    corrupt_status_from: Option<i32>,
) {
    let mut frame = start_frame;
    for request in requests {
        if let FortressRequest::AdvanceFrame { inputs } = request {
            let mut values: Vec<(InputFingerprint, InputStatus)> = inputs
                .iter()
                .map(|(input, status)| (input.fingerprint(), *status))
                .collect();
            if corrupt_from.is_some_and(|from| frame >= from) {
                if let Some((fingerprint, _)) = values.first_mut() {
                    *fingerprint = corrupt_fingerprint(*fingerprint);
                }
            }
            if corrupt_status_from.is_some_and(|from| frame >= from) {
                for (_, status) in &mut values {
                    if *status == InputStatus::Disconnected {
                        *status = InputStatus::Confirmed;
                        break;
                    }
                }
            }
            spectator.applied_inputs.insert(frame, values);
            frame += 1;
        }
    }
}

/// Synthetic mesh addresses (never bound): `127.0.0.1:(20001 + i)`.
pub(super) fn peer_addr(i: usize) -> SocketAddr {
    let port = 20001 + u16::try_from(i).expect("peer index fits in u16");
    ([127, 0, 0, 1], port).into()
}

/// Folds one schema-stable JSON representation into the running trace digest.
///
/// Rust's derived [`std::hash::Hash`] encoding is not a persistence format:
/// enum discriminants and `usize` writes can vary by target. Artifact DTO JSON
/// is the reviewed, cross-platform representation shared by trace replay.
fn fold_trace<T: Serialize>(hash: &mut u64, item: &T, scratch: &mut Vec<u8>) {
    scratch.clear();
    serde_json::to_writer(&mut *scratch, item).expect("trace DTO serializes");
    let mut hasher = DeterministicHasher::new();
    hasher.write(&hash.to_le_bytes());
    hasher.write(
        &u64::try_from(scratch.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    hasher.write(scratch);
    *hash = hasher.finish();
}

fn push_trace_summary<T>(items: &mut Vec<T>, truncated: &mut u32, item: T) {
    if items.len() < TRACE_STEP_EVENT_CAPACITY {
        items.push(item);
    } else {
        *truncated = truncated.saturating_add(1);
    }
}

fn bounded_trace_text(mut text: String) -> String {
    const SUFFIX: &str = "...<truncated>";
    if text.len() <= TRACE_EVENT_TEXT_CAPACITY {
        return text;
    }
    let mut end = TRACE_EVENT_TEXT_CAPACITY.saturating_sub(SUFFIX.len());
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.truncate(end);
    text.push_str(SUFFIX);
    text
}

fn stable_schedule_event_text(event: &ScheduleEvent) -> String {
    // `ScheduleEvent` is the schema-versioned corpus representation. Its JSON
    // form is therefore the stable trace identity; `Debug` is diagnostic-only
    // and may change without a schedule-schema bump.
    serde_json::to_string(event)
        .unwrap_or_else(|error| format!("schedule-event-serialization-error:{error}"))
}

fn stable_observed_event<I: SimInput>(
    source: TraceEventSource,
    event: &FortressEvent<I::SessionConfig>,
) -> TraceObservedEvent {
    TraceObservedEvent {
        source,
        kind: event.kind().as_str().to_owned(),
        // FortressEvent's Display implementation is an explicit, exhaustive,
        // payload-bearing representation. Keep Debug out of the digest.
        details: bounded_trace_text(event.to_string()),
    }
}

fn peer_event_key<I: SimInput>(event: &FortressEvent<I::SessionConfig>) -> Option<PeerEventKey> {
    let kind = event.kind();
    let payload = match event {
        FortressEvent::Synchronizing { addr, .. }
        | FortressEvent::Synchronized { addr }
        | FortressEvent::Disconnected { addr }
        | FortressEvent::NetworkInterrupted { addr, .. }
        | FortressEvent::NetworkResumed { addr }
        | FortressEvent::DesyncDetected { addr, .. }
        | FortressEvent::SyncTimeout { addr, .. } => PeerEventPayload::Addr(*addr),
        FortressEvent::PeerDropped { handle, addr } => PeerEventPayload::PlayerAddr {
            handle: *handle,
            addr: *addr,
        },
        #[cfg(feature = "hot-join")]
        FortressEvent::JoinRequested { handle, addr }
        | FortressEvent::PeerJoined { handle, addr } => PeerEventPayload::PlayerAddr {
            handle: *handle,
            addr: *addr,
        },
        FortressEvent::WaitRecommendation { .. }
        | FortressEvent::ReplayDesync { .. }
        | FortressEvent::SpectatorDivergence { .. }
        | FortressEvent::InputDelayRecommendation { .. } => return None,
    };
    Some(PeerEventKey { kind, payload })
}

/// Per-step progress dump for the diagnostic path.
// Deliberate diagnostic stdout: this path only runs under `--run-ignored`
// manual investigation, where print output IS the deliverable.
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn print_step_summary<I: SimInput>(step: u32, peers: &[PeerSlot<I>], net: &SimNet<Message>) {
    let summary: Vec<String> = peers
        .iter()
        .map(|slot| {
            format!(
                "{:?} game_frame={} confirmed={}",
                slot.session.current_state(),
                slot.game.gs.frame,
                slot.session.confirmed_frame().as_i32()
            )
        })
        .collect();
    println!("step {step}: {summary:?}\n  net={:?}", net.stats());
    for (i, slot) in peers.iter().enumerate() {
        println!("  peer{i}: {}", slot.session.diagnostic_connect_status());
    }
}

fn drive_spectator<I: SimInput>(
    spectator: &mut SpectatorSlot<I>,
    step: u32,
    options: &RunOptions,
    oracle: &mut Oracle,
    observed_events: &mut Vec<TraceObservedEvent>,
    observed_events_truncated: &mut u32,
) {
    let start_frame = spectator.session.current_frame().as_i32().saturating_add(1);
    match spectator.session.advance_frame() {
        Ok(requests) => record_spectator_requests(
            spectator,
            requests,
            start_frame,
            options.corrupt_spectator_input_from,
            options.corrupt_spectator_status_from,
        ),
        Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
        Err(error) => oracle.observe_spectator_error("advance_frame", step, &error),
    }

    let events: Vec<FortressEvent<I::SessionConfig>> = spectator.session.events().collect();
    for event in &events {
        push_trace_summary(
            observed_events,
            observed_events_truncated,
            stable_observed_event::<I>(TraceEventSource::Spectator, event),
        );
        if let FortressEvent::SpectatorDivergence { frame, player, .. } = event {
            oracle.observe_spectator_divergence_event(*frame, *player);
        }
    }
}

/// Diagnostic variant of [`run`]: prints per-peer progress every 50 steps.
/// For manual investigation of a repro seed (see `fleet::diagnose_repro`).
// Deliberate diagnostic stdout: this path only runs under `--run-ignored`
// manual investigation, where print output IS the deliverable.
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
pub fn diagnose(schedule: &Schedule) {
    let report = run_inner::<StubInput>(schedule, &RunOptions::default(), true);
    println!(
        "final: confirmed={:?} net={:?} failures={:#?}",
        report.final_confirmed, report.net_stats, report.verdict.failures
    );
}

/// Runs one schedule to completion and reports.
#[must_use]
pub fn run(schedule: &Schedule, options: &RunOptions) -> RunReport {
    run_with_input::<StubInput>(schedule, options)
}

/// Runs one schedule with a specific fixed-width harness input type.
#[must_use]
pub fn run_with_input<I: SimInput>(schedule: &Schedule, options: &RunOptions) -> RunReport {
    run_inner::<I>(schedule, options, false)
}

pub(super) fn validate_run_options(
    schedule: &Schedule,
    options: &RunOptions,
) -> Result<(), String> {
    let n = schedule.config.n_players;
    if options
        .probe_confirmed_at
        .is_some_and(|probe| probe >= schedule.config.steps)
    {
        return Err(format!(
            "probe_confirmed_at must be within 0..{}",
            schedule.config.steps
        ));
    }
    let Some((from, to)) = options.pending_output_probe_link else {
        return Ok(());
    };
    if from >= n || to >= n || from == to {
        return Err(format!(
            "pending_output_probe_link ({from}, {to}) must name two distinct peers within 0..{n}"
        ));
    }
    let endpoint_retires = schedule.events.iter().any(|(_, event)| {
        let retired = match event {
            ScheduleEvent::GracefulRemove { target, .. }
            | ScheduleEvent::LegacyDisconnect { target, .. } => Some(*target),
            ScheduleEvent::PeerKill { peer } => Some(*peer),
            ScheduleEvent::SpectatorHostKill { host } => Some(*host),
            #[cfg(feature = "hot-join")]
            ScheduleEvent::HotJoin { slot } => Some(*slot),
            _ => None,
        };
        retired.is_some_and(|peer| peer == from || peer == to)
    });
    if endpoint_retires {
        return Err(format!(
            "pending_output_probe_link ({from}, {to}) cannot target an endpoint retired or replaced during the run"
        ));
    }
    Ok(())
}

fn message_payload_metadata(message: &Message) -> SimPayloadMetadata {
    let (encoded_len, kind) = fortress_rollback::__internal::message_metadata(message);
    SimPayloadMetadata {
        encoded_len,
        is_input: kind == MessageKind::Input,
    }
}

fn run_inner<I: SimInput>(schedule: &Schedule, options: &RunOptions, diagnose: bool) -> RunReport {
    validate_schedule(schedule).unwrap_or_else(|error| {
        panic!(
            "invalid materialized schedule seed={} schema_version={}: {error}",
            schedule.seed, schedule.schema_version
        )
    });

    validate_run_options(schedule, options)
        .unwrap_or_else(|error| panic!("invalid run options: {error}"));
    let n = schedule.config.n_players;

    let clock = TestClock::new();
    let net: SimNet<Message> = SimNet::new_size_aware(
        schedule.link_seed,
        clock.as_protocol_clock(),
        message_payload_metadata,
    );
    let addrs: Vec<SocketAddr> = (0..n).map(peer_addr).collect();

    let mut spectator_host_enabled = vec![false; n];
    for &peer in &schedule.config.spectator_hosts {
        spectator_host_enabled[peer] = true;
    }
    let mut hot_join_host_enabled = vec![false; n];
    for (_, event) in &schedule.events {
        if let ScheduleEvent::HotJoin { slot } = event {
            if let Some(host) = hot_join_host_for_slot(n, *slot) {
                hot_join_host_enabled[host] = true;
            }
        }
    }
    #[cfg(not(feature = "hot-join"))]
    let _ = &hot_join_host_enabled;

    for (from, to, policy) in &schedule.initial_links {
        net.set_link(addrs[*from], addrs[*to], policy.clone());
    }

    // Build one session per peer. Handles: peer i is Local handle i, Remote
    // handle j at addrs[j] for j != i. Protocol RNG seeded per peer so magic
    // numbers/sync tokens are reproducible.
    let spectator_addr = peer_addr(n);
    let spectator_handle = PlayerHandle::new(n);
    let mut peers: Vec<PeerSlot<I>> = (0..n)
        .map(|i| {
            let socket: SimSocket<Message> = net.attach(addrs[i]);
            let binding = socket.binding();
            let observer = Arc::new(CollectingObserver::new());
            let protocol_config = peer_protocol_config(schedule, i, &clock);
            let mut builder = SessionBuilder::<I::SessionConfig>::new()
                .with_num_players(n)
                .expect("valid player count")
                .with_max_prediction_window(schedule.config.max_prediction)
                .with_input_delay(schedule.config.input_delay)
                .expect("valid input delay")
                .with_save_mode(schedule.config.save_mode.into())
                .with_desync_detection_mode(DesyncDetection::On {
                    interval: schedule.config.desync_interval,
                })
                .with_disconnect_behavior(schedule.config.disconnect_behavior.into())
                .with_protocol_config(protocol_config)
                .with_violation_observer(Arc::clone(&observer) as Arc<_>);
            #[cfg(feature = "hot-join")]
            if hot_join_host_enabled[i] {
                builder = builder.with_hot_join(true);
            }
            if let Some(size) = schedule.config.event_queue_size {
                // Validated `>= 10` up front, so the current min-cap check
                // cannot reject it; surface the real error (not a fixed string)
                // if the builder ever grows stricter validation.
                builder = builder.with_event_queue_size(size).unwrap_or_else(|error| {
                    panic!("with_event_queue_size({size}) rejected a pre-validated size: {error:?}")
                });
            }
            for (j, addr) in addrs.iter().enumerate() {
                let player_type = if j == i {
                    PlayerType::Local
                } else {
                    PlayerType::Remote(*addr)
                };
                builder = builder
                    .add_player(player_type, PlayerHandle::new(j))
                    .expect("valid player registration");
            }
            if spectator_host_enabled[i] {
                builder = builder
                    .add_player(PlayerType::Spectator(spectator_addr), spectator_handle)
                    .expect("valid spectator registration");
            }
            let session = builder.start_p2p_session(socket).expect("session starts");

            let mut game = SimGameStub::<I>::new();
            if let Some((peer, from)) = options.corrupt_state_from {
                if peer == i {
                    game.corrupt_state_from = Some(from);
                }
            }
            if let Some((peer, from)) = options.corrupt_checksum_from {
                if peer == i {
                    game.corrupt_checksum_from = Some(from);
                }
            }
            PeerSlot {
                session,
                binding,
                source_addrs: vec![addrs[i]],
                game,
                observer,
                sampled_confirmed: -1,
                protocol_rng_seed: peer_protocol_seed(schedule, i),
                pending_replacement_handoff_floor: None,
            }
        })
        .collect();

    let mut spectator: Option<SpectatorSlot<I>> = (!schedule.config.spectator_hosts.is_empty())
        .then(|| {
            let socket: SimSocket<Message> = net.attach(spectator_addr);
            let observer = Arc::new(CollectingObserver::new());
            let protocol_config = ProtocolConfig {
                clock: Some(clock.as_protocol_clock()),
                protocol_rng_seed: Some(fnv1a_hash(&(schedule.seed, "spectator"))),
                ..ProtocolConfig::default()
            };
            let host_addrs: Vec<SocketAddr> = schedule
                .config
                .spectator_hosts
                .iter()
                .map(|&peer| addrs[peer])
                .collect();
            let mut builder = SessionBuilder::<I::SessionConfig>::new()
                .with_num_players(n)
                .expect("valid player count")
                .with_protocol_config(protocol_config)
                .with_violation_observer(Arc::clone(&observer) as Arc<_>);
            if let Some(size) = schedule.config.event_queue_size {
                builder = builder.with_event_queue_size(size).unwrap_or_else(|error| {
                    panic!(
                        "spectator with_event_queue_size({size}) rejected a \
                         pre-validated size: {error:?}"
                    )
                });
            }
            let session = builder
                .start_spectator_session_multi(&host_addrs, socket)
                .expect("spectator session starts");
            SpectatorSlot {
                session,
                observer,
                applied_inputs: BTreeMap::new(),
            }
        });

    let mut oracle = Oracle::new(n);
    validate_violation_allowlist(DEFAULT_VIOLATION_ALLOWLIST)
        .expect("reviewed default violation allowlist must stay valid");
    let mut trace_hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    let mut trace_scratch = Vec::new();
    let mut trace_tail: VecDeque<TraceSnapshot> = VecDeque::with_capacity(TRACE_TAIL_CAPACITY);
    let mut peer_event_counts: BTreeMap<EventKind, u64> = BTreeMap::new();
    let mut peer_event_counts_by_peer = vec![BTreeMap::new(); n];
    let mut peer_event_payload_counts_by_peer = vec![BTreeMap::new(); n];
    let mut load_game_state_observations: Vec<LoadGameStateObservation> = Vec::new();
    let mut next_event = 0usize;
    // Per-peer stall deadline (exclusive step): peer `i` is frozen while
    // `step < stalled_until[i]`. `0` means never stalled; a `PeerStall` event
    // sets it to `step + steps`.
    let mut stalled_until: Vec<u32> = vec![0; n];
    // Peers retired by lifecycle events (`PeerKill`, `GracefulRemove`, or
    // `LegacyDisconnect`): no longer driven, detached from the fabric, and
    // excluded from the oracle's liveness checks (their pre-retirement
    // observations still count for agreement).
    let mut dead: Vec<bool> = vec![false; n];
    // Per-peer count of advances still owed to an obeyed `WaitRecommendation`
    // (only accumulates under `AppModel::Obey`). While > 0 the peer polls but
    // does not advance, letting the others catch up — the closed time-sync loop.
    let app_model = schedule.config.app_model;
    let mut wait_skip: Vec<u32> = vec![0; n];
    // Peers whose app model never drains the session event queue (models a
    // wedged event consumer). Their bounded `event_queue` fills and the session
    // trims oldest events, firing the D9 `events_discarded_*` telemetry. Because
    // the harness normally drains (and feeds) events per step, starvation is the
    // only fleet path that exercises that overflow. Built by direct index (peers
    // validated in-range above) — O(n + |starve_events|), no per-peer rescan.
    let mut starves = vec![false; n];
    for &peer in &schedule.config.starve_events {
        starves[peer] = true;
    }
    // Confirmed-frame snapshot taken at `options.probe_confirmed_at`, if any.
    let mut probe_confirmed: Vec<i32> = Vec::new();
    let mut probe_peer_wire_by_link: BTreeMap<(usize, usize), PeerWireTotals> = BTreeMap::new();
    let mut pending_output_probe =
        options
            .pending_output_probe_link
            .map(|(from, to)| PendingOutputProbe {
                from,
                to,
                limit: u64::try_from(ProtocolConfig::default().pending_output_limit)
                    .unwrap_or(u64::MAX),
                first_reached_limit_at: None,
                at_probe: None,
                max: 0,
                at_heal: None,
                after_recovery: None,
                final_value: 0,
            });

    // (c) bounded post-heal liveness anchors. The heal step is the ACTUAL last
    // `HealAll` event, not `schedule.heal_at` — a schedule can set `heal_at`
    // without emitting a heal (e.g. a no-fault clock-skew run sets it to `steps`
    // with no event), and a hand-authored schedule's `heal_at` field could drift
    // from where its event actually fires. Deriving both the anchor and the
    // window from the event keeps them consistent. (c) runs only when a heal
    // fired AND enough post-heal drain remains for both anchors to be observable
    // (`steps - heal_at >= B`, i.e. the recovery anchor `heal_at + B` is at most
    // the run's end); otherwise it is inert (no heal) or indeterminate (window
    // too short). The recovery anchor clamps to the last recorded step only at
    // the exact boundary `heal_at + B == steps`, giving a span of B-1 there and
    // exactly B otherwise — the runner reports that real span to the oracle, so
    // no case is silently mislabelled a full-B window.
    let heal_step = schedule
        .events
        .iter()
        .filter(|(_, event)| matches!(event, ScheduleEvent::HealAll))
        .map(|(step, _)| *step)
        .max();
    let b_steps = schedule.config.recovery_window_steps();
    let last_step = schedule.config.steps.saturating_sub(1);
    // A healthy peer confirms ~1 frame per step post-heal, so the observed
    // window (in steps) must be at least G wide for the G-frame floor to be
    // clearable at all; a narrower window cannot distinguish a pinned peer from
    // a healthy one, so (c) is indeterminate (`None`) rather than a false
    // `Some(false)` charged against every healthy run. Unreachable at the
    // default 16ms step_dt (span ~250 ≫ G=10); guards a pathologically coarse
    // step_dt / tiny B (and the exact-boundary B-1 span).
    let g_floor = u32::try_from(POST_HEAL_MIN_ADVANCE).unwrap_or(0);
    let (run_c, heal_anchor_at, recovery_anchor_at) = match heal_step {
        Some(heal_at) if schedule.config.steps.saturating_sub(heal_at) >= b_steps => {
            let heal_anchor = heal_at.min(last_step);
            let recovery_anchor = heal_at.saturating_add(b_steps).min(last_step);
            if recovery_anchor.saturating_sub(heal_anchor) >= g_floor {
                (true, heal_anchor, recovery_anchor)
            } else {
                (false, 0, 0)
            }
        },
        _ => (false, 0, 0),
    };
    let mut confirmed_at_heal: Vec<i32> = Vec::new();
    let mut confirmed_after_recovery: Vec<i32> = Vec::new();
    // A spectator floor is in displayed game-frame space, not schedule-step
    // space. Capture it only when a lifecycle event actually retires a peer,
    // using the slowest live survivor's current frame so stalled/waiting app
    // models are not charged for virtual time they never simulated.
    let spectator_enabled = spectator.is_some();
    let mut spectator_required_min_frame: Option<i32> = None;

    for step in 0..schedule.config.steps {
        let mut step_confirmed = Vec::with_capacity(n);
        let mut scheduled_events = Vec::new();
        let mut scheduled_events_truncated = 0u32;
        let mut observed_events = Vec::new();
        let mut observed_events_truncated = 0u32;
        // Apply control-plane events due at this step.
        while let Some((event_step, event)) = schedule.events.get(next_event) {
            if *event_step > step {
                break;
            }
            push_trace_summary(
                &mut scheduled_events,
                &mut scheduled_events_truncated,
                bounded_trace_text(stable_schedule_event_text(event)),
            );
            match event {
                ScheduleEvent::SetLink { from, to, policy } => {
                    net.set_link(
                        peers[*from].binding.local_addr(),
                        addrs[*to],
                        policy.clone(),
                    );
                },
                ScheduleEvent::Block { from, to, blocked } => {
                    net.set_blocked(peers[*from].binding.local_addr(), addrs[*to], *blocked);
                },
                ScheduleEvent::Hold { from, to, holding } => {
                    net.set_holding(peers[*from].binding.local_addr(), addrs[*to], *holding);
                },
                ScheduleEvent::PeerStall { peer, steps } => {
                    // `peer` in range and `steps > 0` are validated up front.
                    stalled_until[*peer] = stalled_until[*peer].max(step.saturating_add(*steps));
                },
                ScheduleEvent::SetInputDelay { peer, delay } => {
                    // Reconfigure the peer's own local input delay mid-run. A
                    // mid-session increase gap-fills and flushes to remotes; a
                    // failure (e.g. pending-output full) is a real error the
                    // oracle must surface, not swallow.
                    let handle = PlayerHandle::new(*peer);
                    if let Err(error) = peers[*peer].session.set_input_delay(handle, *delay) {
                        oracle.observe_session_error("set_input_delay", *peer, step, &error);
                    }
                },
                ScheduleEvent::GracefulRemove { by, target } => {
                    // User-driven graceful departure: one survivor explicitly
                    // removes the target and the target stops participating. The
                    // remaining live peers must learn the drop through gossip and
                    // keep a byte-consistent confirmed prefix.
                    if !dead[*by] && !dead[*target] {
                        let handle = PlayerHandle::new(*target);
                        if let Err(error) = peers[*by].session.remove_player(handle) {
                            oracle.observe_session_error("remove_player", *by, step, &error);
                        } else {
                            retire_peer_for_lifecycle(
                                *target,
                                &peers,
                                &mut dead,
                                &mut oracle,
                                spectator_enabled.then_some(&mut spectator_required_min_frame),
                            );
                        }
                    }
                },
                ScheduleEvent::LegacyDisconnect { by, target } => {
                    // User-driven legacy disconnect: one survivor explicitly
                    // kicks the target through the older Halt-oriented API. On
                    // success the target stops participating; on error it
                    // stays live and the oracle records the failed API call.
                    // This deliberately does not assert graceful convergence:
                    // Halt preserves the shared confirmed prefix but remains
                    // terminal, so the expected observation is non-recovery.
                    if !dead[*by] && !dead[*target] {
                        let handle = PlayerHandle::new(*target);
                        if let Err(error) = peers[*by].session.disconnect_player(handle) {
                            oracle.observe_session_error("disconnect_player", *by, step, &error);
                        } else {
                            retire_peer_for_lifecycle(
                                *target,
                                &peers,
                                &mut dead,
                                &mut oracle,
                                spectator_enabled.then_some(&mut spectator_required_min_frame),
                            );
                        }
                    }
                },
                ScheduleEvent::PeerKill { peer } => {
                    // Crash the peer: stop driving it, discard its inbox (so
                    // further traffic to it is dropped under the default
                    // `UnattachedPolicy::Drop`), and exclude it from the oracle's
                    // liveness checks. Its remaining mesh survives per the
                    // configured `DisconnectBehavior`. Idempotent.
                    retire_peer_for_lifecycle(
                        *peer,
                        &peers,
                        &mut dead,
                        &mut oracle,
                        spectator_enabled.then_some(&mut spectator_required_min_frame),
                    );
                },
                ScheduleEvent::SpectatorHostKill { host } => {
                    // Spectator-focused crash: validation guarantees this peer is
                    // one of the configured redundant spectator hosts. Killing it
                    // should force the spectator to continue from the remaining
                    // hosts while the mesh survives per `DisconnectBehavior`.
                    retire_peer_for_lifecycle(
                        *host,
                        &peers,
                        &mut dead,
                        &mut oracle,
                        spectator_enabled.then_some(&mut spectator_required_min_frame),
                    );
                },
                ScheduleEvent::Rebind { peer } => {
                    // Runtime-contingent retirement (for example, an earlier
                    // GracefulRemove) makes this fire-time precondition miss a
                    // deterministic no-op. Planted events are never re-sampled.
                    if !dead[*peer] {
                        let fresh = rebound_peer_addr(*peer);
                        match peers[*peer].binding.rebind(fresh) {
                            Ok(_) => peers[*peer].source_addrs.push(fresh),
                            Err(error) => oracle.observe_runner_error(
                                "rebind_failed",
                                *peer,
                                step,
                                format!("failed to move live socket to {fresh}: {error:?}"),
                            ),
                        }
                    }
                },
                #[cfg(feature = "hot-join")]
                ScheduleEvent::HotJoin { slot } => {
                    // Returning clean-drop path: build the replacement first so
                    // constructor failures leave the old slot intact; then one
                    // survivor removes the old slot, the runner resets the old
                    // inbox, and the replacement runs the public hot-join
                    // protocol. The slot is not marked dead: it remains part
                    // of the live oracle set and must end Running/confirming
                    // after reactivation.
                    let mut ctx = HotJoinRuntime {
                        schedule,
                        clock: &clock,
                        net: &net,
                        addrs: &addrs,
                        peers: &mut peers,
                        dead: &dead,
                        oracle: &mut oracle,
                    };
                    start_hot_join_for_slot(*slot, step, &mut ctx);
                },
                #[cfg(not(feature = "hot-join"))]
                ScheduleEvent::HotJoin { .. } => {
                    unreachable!("HotJoin schedules are rejected before sessions are built")
                },
                ScheduleEvent::HealAll => net.heal_all(),
            }
            next_event += 1;
        }

        // Drive every peer in fixed order. A peer that is stalled (a local hang:
        // frozen for its stall window) or dead (crashed by `PeerKill`) is not
        // driven — it does not poll, drain events, add input, or advance, and
        // puts nothing on the wire. A stalled peer resumes when its window ends;
        // a dead peer never does. Either way its state is still folded into the
        // trace below so the digest stays uniform and reproduces bit-for-bit.
        for (i, slot) in peers.iter_mut().enumerate() {
            // Confirmed frame after this step's drive (or the frozen value for a
            // stalled/dead peer): read exactly once and reused for both the
            // trace fold and any probe snapshot.
            let confirmed = if !dead[i] && step >= stalled_until[i] {
                slot.session.poll_remote_clients();

                // A starved peer never drains its event queue (models a wedged
                // event consumer): the session's bounded queue fills and trims,
                // firing D9. Skipping the drain forgoes only this peer's own
                // event signals — its self-observed `DesyncDetected` and its
                // event trace folds. The oracle keeps full teeth on it anyway:
                // the primary state-agreement check reads its recorded state
                // directly, and any real divergence is still caught in-band by
                // its neighbors' own desync detection over the wire.
                if !starves[i] {
                    let events: Vec<FortressEvent<I::SessionConfig>> =
                        slot.session.events().collect();
                    for event in &events {
                        let kind = event.kind();
                        push_trace_summary(
                            &mut observed_events,
                            &mut observed_events_truncated,
                            stable_observed_event::<I>(TraceEventSource::Peer(i), event),
                        );
                        *peer_event_counts.entry(kind).or_default() += 1;
                        *peer_event_counts_by_peer[i].entry(kind).or_default() += 1;
                        if let Some(key) = peer_event_key::<I>(event) {
                            *peer_event_payload_counts_by_peer[i].entry(key).or_default() += 1;
                        }
                        if let FortressEvent::DesyncDetected { frame, .. } = event {
                            oracle.observe_desync_event(i, *frame);
                        }
                        // Closed-loop app model: obey a `WaitRecommendation` by owing
                        // that many skipped advances (max so a stronger one wins).
                        if app_model == AppModel::Obey {
                            if let FortressEvent::WaitRecommendation { skip_frames } = event {
                                wait_skip[i] = wait_skip[i].max(*skip_frames);
                            }
                        }
                    }
                }

                if slot.session.current_state() == SessionState::Running {
                    if wait_skip[i] > 0 {
                        // Obeying a WaitRecommendation: poll/receive this step
                        // (done above) but skip the advance so the ahead peer
                        // lets the others catch up. Count down only on steps that
                        // would otherwise advance (i.e. while `Running`) — a peer
                        // that briefly leaves `Running` mid-wait (transient
                        // resync) must not silently consume its owed skips, or it
                        // would stop obeying once it resumes.
                        wait_skip[i] -= 1;
                    } else {
                        for handle in slot.session.local_player_handles() {
                            if let Err(error) = slot
                                .session
                                .add_local_input(handle, input_for::<I>(step, i))
                            {
                                oracle.observe_session_error("add_local_input", i, step, &error);
                            }
                        }
                        match slot.session.advance_frame() {
                            Ok(requests) => {
                                if let Some(frame) = SimGameStub::<I>::loaded_frame(&requests) {
                                    load_game_state_observations.push(LoadGameStateObservation {
                                        step,
                                        peer: i,
                                        frame: frame.as_i32(),
                                    });
                                }
                                if let Some(handoff_floor) = slot.pending_replacement_handoff_floor
                                {
                                    if let Some(frame) =
                                        slot.game.handle_replacement_handoff_requests(
                                            requests,
                                            handoff_floor,
                                        )
                                    {
                                        let snapshot_frame = frame.as_i32();
                                        slot.sampled_confirmed = snapshot_frame;
                                        slot.pending_replacement_handoff_floor = None;
                                    }
                                } else {
                                    slot.game.handle_requests(requests);
                                }
                            },
                            Err(error) => oracle.observe_advance_error(i, step, &error),
                        }
                    }
                }

                // Incrementally sample newly confirmed inputs (they evict).
                let confirmed = slot.session.confirmed_frame();
                if confirmed.is_valid() && slot.pending_replacement_handoff_floor.is_none() {
                    for frame in (slot.sampled_confirmed + 1)..=confirmed.as_i32() {
                        match slot.session.confirmed_inputs_for_frame(Frame::new(frame)) {
                            Ok(inputs) => {
                                let values: Vec<InputFingerprint> =
                                    inputs.iter().map(|input| input.fingerprint()).collect();
                                oracle.observe_confirmed_inputs(i, frame, values);
                            },
                            Err(error) => {
                                oracle.observe_confirmed_unavailable(
                                    i,
                                    frame,
                                    &format!("{error:?}"),
                                );
                            },
                        }
                        slot.sampled_confirmed = frame;
                    }
                }
                confirmed
            } else {
                // Stalled (local hang) or dead (crashed): no poll/advance, no
                // wire traffic — just its last confirmed frame, frozen.
                slot.session.confirmed_frame()
            };

            step_confirmed.push(confirmed.as_i32());

            // Optional mid-run confirmation snapshot for recovery-dynamics
            // tests, reusing the value already read for this peer above.
            if options.probe_confirmed_at == Some(step) {
                probe_confirmed.push(confirmed.as_i32());
            }

            // (c) heal-anchored snapshots, reusing the same confirmed value. The
            // peer loop runs in fixed order, so each vector ends up indexed by
            // peer. Only populated when (c) runs (a heal fired with a full
            // recovery window). Captured for every peer — a stalled/dead peer
            // falls through to here with its frozen confirmed frame, so the
            // vectors stay length-n and correctly peer-indexed.
            if run_c && step == heal_anchor_at {
                confirmed_at_heal.push(confirmed.as_i32());
            }
            if run_c && step == recovery_anchor_at {
                confirmed_after_recovery.push(confirmed.as_i32());
            }
        }

        // Sample link-specific wire counters only after the fixed-order peer
        // loop completes, so the probe describes one unambiguous end-of-step
        // cut rather than a mixture of before/after states across peers.
        if options.probe_confirmed_at == Some(step) {
            probe_peer_wire_by_link = collect_peer_wire_by_link(&peers, n);
        }
        if let Some(probe) = pending_output_probe.as_mut() {
            let value = peers[probe.from]
                .session
                .peer_metrics(PlayerHandle::new(probe.to))
                .unwrap_or_else(|error| {
                    panic!(
                        "peer {}: pending-output peer_metrics(handle={}) failed unexpectedly: {error:?}",
                        probe.from, probe.to
                    )
                })
                .pending_output_len;
            probe.max = probe.max.max(value);
            probe.final_value = value;
            if probe.first_reached_limit_at.is_none() && value >= probe.limit {
                probe.first_reached_limit_at = Some(step);
            }
            if options.probe_confirmed_at == Some(step) {
                probe.at_probe = Some(value);
            }
            if heal_step == Some(step) {
                probe.at_heal = Some(value);
            }
            if run_c && step == recovery_anchor_at {
                probe.after_recovery = Some(value);
            }
        }

        if let Some(spectator) = spectator.as_mut() {
            drive_spectator(
                spectator,
                step,
                options,
                &mut oracle,
                &mut observed_events,
                &mut observed_events_truncated,
            );
        }

        if trace_tail.len() == TRACE_TAIL_CAPACITY {
            let _ = trace_tail.pop_front();
        }
        let snapshot = TraceSnapshot {
            step,
            confirmed_frames: step_confirmed,
            session_states: peers
                .iter()
                .map(|slot| TraceSessionState::from(slot.session.current_state()))
                .collect(),
            dead: dead.clone(),
            game_states: peers
                .iter()
                .map(|slot| TraceGameState {
                    frame: slot.game.gs.frame,
                    value: slot.game.gs.state,
                })
                .collect(),
            scheduled_events,
            scheduled_events_truncated,
            observed_events,
            observed_events_truncated,
            net: TraceNetStats::from(net.stats()),
            spectator: spectator.as_ref().map(|slot| TraceSpectatorState {
                current_frame: slot.session.current_frame().as_i32(),
                num_hosts: slot.session.num_hosts(),
                applied_frames: slot.applied_inputs.len(),
                max_applied_frame: slot.applied_inputs.keys().next_back().copied(),
            }),
        };
        // The digest and the artifact tail share one stable representation, so
        // every captured step observable (including lifecycle/dead state,
        // event truncation, and network counters) participates here. Selected
        // final values are intentionally folded again in the final summary.
        fold_trace(&mut trace_hash, &snapshot, &mut trace_scratch);
        trace_tail.push_back(snapshot);

        if diagnose && step % 50 == 0 {
            print_step_summary(step, &peers, &net);
        }

        clock.advance(schedule.config.step_dt());
    }

    let metrics: Vec<fortress_rollback::SessionMetrics> =
        peers.iter().map(|slot| slot.session.metrics()).collect();

    // Final observations.
    let mut violation_census: BTreeMap<ViolationSignature, u64> = BTreeMap::new();
    for ((i, slot), metric) in peers.iter().enumerate().zip(metrics.iter()) {
        let violations = slot.observer.violations();
        for violation in &violations {
            let signature =
                ViolationSignature::from_violation(violation, DEFAULT_VIOLATION_ALLOWLIST);
            *violation_census.entry(signature).or_default() += 1;
        }
        oracle.observe_violations(ViolationSource::Peer(i), &violations);
        oracle.observe_checksum_mismatches(i, metric.checksums_mismatched);
    }
    if let Some(spectator) = &spectator {
        let violations = spectator.observer.violations();
        for violation in &violations {
            let signature =
                ViolationSignature::from_violation(violation, DEFAULT_VIOLATION_ALLOWLIST);
            *violation_census.entry(signature).or_default() += 1;
        }
        oracle.observe_violations(ViolationSource::Spectator, &violations);
    }
    let recorded: Vec<BTreeMap<i32, StateStub>> = peers
        .iter()
        .map(|slot| slot.game.recorded.clone())
        .collect();
    let applied_inputs: Vec<BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>> = peers
        .iter()
        .map(|slot| slot.game.applied_inputs.clone())
        .collect();
    let end_confirmed: Vec<Frame> = peers
        .iter()
        .map(|slot| slot.session.confirmed_frame())
        .collect();
    let end_state: Vec<SessionState> = peers
        .iter()
        .map(|slot| slot.session.current_state())
        .collect();
    let final_confirmed: Vec<i32> = end_confirmed.iter().map(|frame| frame.as_i32()).collect();

    // Aggregate each peer's per-remote wire metrics into one per-player total.
    // Peer `i` holds a remote handle `PlayerHandle::new(j)` for every `j != i`
    // (see the builder loop above); `peer_metrics` succeeds for any remote
    // handle regardless of sync state.
    let n_players = schedule.config.n_players;
    let peer_wire: Vec<PeerWireTotals> = peers
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let mut totals = PeerWireTotals::default();
            for j in 0..n_players {
                if j == i {
                    continue;
                }
                // Peer `i` registered `PlayerHandle::new(j)` as a remote for
                // every `j != i` (see the builder loop above), so this MUST
                // resolve. An error is a real invariant break — fail loudly
                // rather than silently under-counting a bandwidth regression.
                let pm = slot
                    .session
                    .peer_metrics(PlayerHandle::new(j))
                    .unwrap_or_else(|e| {
                        panic!("peer {i}: peer_metrics(handle={j}) failed unexpectedly: {e:?}")
                    });
                totals.add(&pm);
            }
            totals
        })
        .collect();

    // Hand the oracle the (c) bounded post-heal liveness inputs. `window_steps`
    // is the ACTUAL anchor span (B, or B-1 at an exact-boundary drain where the
    // recovery anchor clamped to the last step), so the failure reports the real
    // window rather than a nominal one.
    oracle.set_heal_liveness(HealLiveness {
        ran: run_c,
        window_steps: recovery_anchor_at.saturating_sub(heal_anchor_at),
        required_advance: POST_HEAL_MIN_ADVANCE,
        confirmed_at_heal: confirmed_at_heal.clone(),
        confirmed_after: confirmed_after_recovery.clone(),
    });
    let spectator_applied_inputs = spectator.as_ref().map(|slot| &slot.applied_inputs);
    let spectator_applied_frames =
        spectator_applied_inputs.map_or(0, std::collections::BTreeMap::len);
    let spectator_max_frame =
        spectator_applied_inputs.and_then(|records| records.keys().next_back().copied());
    let spectator_final_hosts = spectator.as_ref().map(|slot| slot.session.num_hosts());
    let verdict = oracle.finalize_with_applied_inputs_and_spectator(
        &recorded,
        &applied_inputs,
        &end_confirmed,
        &end_state,
        spectator_applied_inputs,
        spectator_required_min_frame,
    );
    let recovered_within_b = verdict.recovered_within_b;
    let mut blocked_drops_by_link = BTreeMap::new();
    let peer_by_source_addr: BTreeMap<SocketAddr, usize> = peers
        .iter()
        .enumerate()
        .flat_map(|(peer, slot)| {
            slot.source_addrs
                .iter()
                .copied()
                .map(move |addr| (addr, peer))
        })
        .collect();
    let peer_by_canonical_addr: BTreeMap<SocketAddr, usize> = addrs
        .iter()
        .copied()
        .enumerate()
        .map(|(peer, addr)| (addr, peer))
        .collect();
    for ((from_addr, to_addr), drops) in net.blocked_drop_counts() {
        if let (Some(from), Some(to)) = (
            peer_by_source_addr.get(&from_addr),
            peer_by_canonical_addr.get(&to_addr),
        ) {
            let total = blocked_drops_by_link.entry((*from, *to)).or_insert(0u64);
            *total = total.saturating_add(drops);
        }
    }
    let map_link = |from_addr: &SocketAddr, to_addr: &SocketAddr| {
        Some((
            *peer_by_source_addr.get(from_addr)?,
            *peer_by_canonical_addr.get(to_addr)?,
        ))
    };
    let mut fragmentation_drops_by_link = BTreeMap::new();
    for ((from_addr, to_addr), drops) in net.fragmentation_drop_counts() {
        if let Some(key) = map_link(&from_addr, &to_addr) {
            let total = fragmentation_drops_by_link.entry(key).or_insert(0_u64);
            *total = total.saturating_add(drops);
        }
    }
    let mut link_stats_by_link: BTreeMap<_, crate::common::sim_net::SimLinkStats> = BTreeMap::new();
    for ((from_addr, to_addr), stats) in net.link_stats() {
        if let Some(key) = map_link(&from_addr, &to_addr) {
            link_stats_by_link.entry(key).or_default().merge(stats);
        }
    }
    let final_trace_summary = TraceFinalSummary {
        failure_classes: verdict.failures.iter().map(OracleFailure::class).collect(),
        final_confirmed: final_confirmed.clone(),
        probe_confirmed: probe_confirmed.clone(),
        probe_peer_wire_by_link: probe_peer_wire_by_link
            .iter()
            .map(|(&(from, to), totals)| TracePeerWireLink {
                from,
                to,
                totals: totals.clone(),
            })
            .collect(),
        fragmentation_drops_by_link: trace_fragmentation_drops(
            schedule.schema_version,
            &fragmentation_drops_by_link,
        ),
        link_stats_by_link: trace_link_stats(schedule.schema_version, &link_stats_by_link),
        pending_output_probe,
        confirmed_at_heal: confirmed_at_heal.clone(),
        confirmed_after_recovery: confirmed_after_recovery.clone(),
        recovered_within_b,
        spectator_applied_frames,
        spectator_max_frame,
        spectator_final_hosts,
        net: TraceNetStats::from(net.stats()),
    };
    fold_trace(&mut trace_hash, &final_trace_summary, &mut trace_scratch);
    RunReport {
        replay_options: options.clone(),
        replay_input_width_bytes: I::WIDTH_BYTES,
        verdict,
        trace_hash,
        final_confirmed,
        trace_tail: trace_tail.into(),
        probe_confirmed,
        probe_peer_wire_by_link,
        pending_output_probe,
        net_stats: net.stats(),
        blocked_drops_by_link,
        fragmentation_drops_by_link,
        link_stats_by_link,
        load_game_state_observations,
        metrics,
        peer_wire,
        confirmed_at_heal,
        confirmed_after_recovery,
        recovered_within_b,
        violation_census,
        peer_event_counts,
        peer_event_counts_by_peer,
        peer_event_payload_counts_by_peer,
        spectator_applied_frames,
        spectator_max_frame,
        spectator_final_hosts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "hot-join")]
    use crate::simulation::harness::oracle::OracleFailure;
    use fortress_rollback::network::codec;

    #[test]
    fn schema_v10_rebind_addresses_are_deterministic_and_disjoint() {
        let addresses: Vec<_> = (0..16).map(rebound_peer_addr).collect();
        let unique: std::collections::BTreeSet<_> = addresses.iter().copied().collect();
        assert_eq!(unique.len(), 16);
        for (peer, address) in addresses.into_iter().enumerate() {
            assert_eq!(address, rebound_peer_addr(peer));
            assert!((0..=16).all(|canonical| address != peer_addr(canonical)));
        }
    }

    #[test]
    fn trace_event_text_is_utf8_safe_and_bounded() {
        let text = "🧱".repeat(TRACE_EVENT_TEXT_CAPACITY);
        let bounded = bounded_trace_text(text);
        assert!(bounded.len() <= TRACE_EVENT_TEXT_CAPACITY);
        assert!(bounded.ends_with("...<truncated>"));
    }

    fn snapshot_hash(snapshot: &TraceSnapshot) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325;
        fold_trace(&mut hash, snapshot, &mut Vec::new());
        hash
    }

    #[test]
    fn trace_hash_preserves_length_prefixed_json_identity() {
        let initial_hash = 0xcbf2_9ce4_8422_2325;
        let mut actual = initial_hash;
        let mut scratch = Vec::new();

        fold_trace(&mut actual, &vec![1_u8, 2], &mut scratch);
        fold_trace(&mut actual, &vec![3_u8, 4, 5], &mut scratch);

        let first_bytes = b"[1,2]";
        let mut first_expected = DeterministicHasher::new();
        first_expected.write(&initial_hash.to_le_bytes());
        first_expected.write(&(first_bytes.len() as u64).to_le_bytes());
        first_expected.write(first_bytes);

        let second_bytes = b"[3,4,5]";
        let mut second_expected = DeterministicHasher::new();
        second_expected.write(&first_expected.finish().to_le_bytes());
        second_expected.write(&(second_bytes.len() as u64).to_le_bytes());
        second_expected.write(second_bytes);
        assert_eq!(actual, second_expected.finish());
    }

    #[test]
    fn trace_hash_changes_when_previously_omitted_dead_state_changes() {
        let snapshot = TraceSnapshot {
            step: 7,
            confirmed_frames: vec![5, 5],
            session_states: vec![TraceSessionState::Running; 2],
            dead: vec![false, false],
            game_states: vec![TraceGameState { frame: 6, value: 9 }; 2],
            scheduled_events: vec!["\"HealAll\"".to_owned()],
            scheduled_events_truncated: 0,
            observed_events: Vec::new(),
            observed_events_truncated: 0,
            net: TraceNetStats::default(),
            spectator: None,
        };
        let mut changed = snapshot.clone();
        changed.dead[1] = true;

        assert_ne!(
            snapshot_hash(&snapshot),
            snapshot_hash(&changed),
            "dead/lifecycle state is part of the complete step identity"
        );
    }

    #[test]
    fn extended_trace_net_stats_are_backward_compatible_and_identity_bearing() {
        let legacy_json = serde_json::to_value(TraceNetStats::default()).unwrap();
        let object = legacy_json
            .as_object()
            .expect("TraceNetStats serializes as an object");
        assert!(!object.keys().any(|key| key.starts_with("gilbert_elliott")));
        assert!(!object.keys().any(|key| key.starts_with("fragmentation")));
        assert!(!object.keys().any(|key| key.starts_with("bandwidth")));
        let legacy_back: TraceNetStats = serde_json::from_value(legacy_json).unwrap();
        assert_eq!(legacy_back, TraceNetStats::default());

        let ge_stats = TraceNetStats {
            gilbert_elliott_good_to_bad: 2,
            gilbert_elliott_bad_to_good: 1,
            gilbert_elliott_good_sends: 7,
            gilbert_elliott_bad_sends: 5,
            gilbert_elliott_loss_events: 4,
            gilbert_elliott_max_loss_run: 3,
            ..TraceNetStats::default()
        };
        let ge_json = serde_json::to_vec(&ge_stats).unwrap();
        assert_eq!(
            serde_json::from_slice::<TraceNetStats>(&ge_json).unwrap(),
            ge_stats
        );

        let snapshot = TraceSnapshot {
            step: 7,
            confirmed_frames: vec![5, 5],
            session_states: vec![TraceSessionState::Running; 2],
            dead: vec![false, false],
            game_states: vec![TraceGameState { frame: 6, value: 9 }; 2],
            scheduled_events: Vec::new(),
            scheduled_events_truncated: 0,
            observed_events: Vec::new(),
            observed_events_truncated: 0,
            net: TraceNetStats::default(),
            spectator: None,
        };
        let mut changed = snapshot.clone();
        changed.net = ge_stats;
        assert_ne!(snapshot_hash(&snapshot), snapshot_hash(&changed));

        let fragmentation_stats = TraceNetStats {
            fragmentation_eligible_sends: 2,
            fragmentation_fragments_modeled: 5,
            fragmentation_lost_fragments: 1,
            fragmentation_loss_events: 1,
            fragmentation_input_eligible_sends: 2,
            fragmentation_input_loss_events: 1,
            fragmentation_max_packet_bytes: 2945,
            fragmentation_max_fragments_per_send: 3,
            fragmentation_fragment_cap_hits: 1,
            ..TraceNetStats::default()
        };
        let fragmentation_json = serde_json::to_vec(&fragmentation_stats).unwrap();
        assert_eq!(
            serde_json::from_slice::<TraceNetStats>(&fragmentation_json).unwrap(),
            fragmentation_stats
        );
        changed.net = fragmentation_stats;
        assert_ne!(snapshot_hash(&snapshot), snapshot_hash(&changed));

        let bandwidth_stats = TraceNetStats {
            bandwidth_admitted_datagrams: 3,
            bandwidth_admitted_bytes: 300,
            bandwidth_queued_datagrams: 2,
            bandwidth_tail_drops: 1,
            bandwidth_tail_dropped_bytes: 100,
            bandwidth_max_queue_bytes: 200,
            bandwidth_max_queue_delay_ns: 200_000_000,
            ..TraceNetStats::default()
        };
        let bandwidth_json = serde_json::to_vec(&bandwidth_stats).unwrap();
        assert_eq!(
            serde_json::from_slice::<TraceNetStats>(&bandwidth_json).unwrap(),
            bandwidth_stats
        );
        changed.net = bandwidth_stats;
        assert_ne!(snapshot_hash(&snapshot), snapshot_hash(&changed));
    }

    #[test]
    fn schema_v13_per_link_fragmentation_evidence_is_identity_bearing() {
        let stats = crate::common::sim_net::SimLinkStats {
            input_sends: 1,
            max_encoded_input_bytes: 2_000,
            fragmentation_input_losses: 1,
            ..crate::common::sim_net::SimLinkStats::default()
        };
        let left_stats = BTreeMap::from([((0, 1), stats)]);
        let right_stats = BTreeMap::from([((1, 0), stats)]);
        let left_drops = BTreeMap::from([((0, 1), 1)]);
        let right_drops = BTreeMap::from([((1, 0), 1)]);

        assert_eq!(
            serde_json::to_vec(&trace_link_stats(12, &left_stats)).unwrap(),
            serde_json::to_vec(&trace_link_stats(12, &right_stats)).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&trace_fragmentation_drops(12, &left_drops)).unwrap(),
            serde_json::to_vec(&trace_fragmentation_drops(12, &right_drops)).unwrap()
        );
        assert_ne!(
            serde_json::to_vec(&trace_link_stats(13, &left_stats)).unwrap(),
            serde_json::to_vec(&trace_link_stats(13, &right_stats)).unwrap()
        );
        assert_ne!(
            serde_json::to_vec(&trace_fragmentation_drops(13, &left_drops)).unwrap(),
            serde_json::to_vec(&trace_fragmentation_drops(13, &right_drops)).unwrap()
        );
    }

    #[test]
    fn schema_v14_per_link_bandwidth_evidence_is_identity_bearing() {
        let stats = crate::common::sim_net::SimLinkStats {
            bandwidth_admitted_datagrams: 1,
            bandwidth_admitted_bytes: 100,
            bandwidth_queued_datagrams: 1,
            bandwidth_max_queue_bytes: 100,
            bandwidth_max_queue_delay_ns: 100_000_000,
            ..crate::common::sim_net::SimLinkStats::default()
        };
        let left = BTreeMap::from([((0, 1), stats)]);
        let right = BTreeMap::from([((1, 0), stats)]);

        assert_ne!(
            serde_json::to_vec(&trace_link_stats(14, &left)).unwrap(),
            serde_json::to_vec(&trace_link_stats(14, &right)).unwrap()
        );
    }

    #[test]
    fn trace_hash_changes_when_spectator_progress_changes() {
        let mut snapshot = TraceSnapshot {
            step: 7,
            confirmed_frames: vec![5, 5],
            session_states: vec![TraceSessionState::Running; 2],
            dead: vec![false, false],
            game_states: vec![TraceGameState { frame: 6, value: 9 }; 2],
            scheduled_events: Vec::new(),
            scheduled_events_truncated: 0,
            observed_events: Vec::new(),
            observed_events_truncated: 0,
            net: TraceNetStats::default(),
            spectator: Some(TraceSpectatorState {
                current_frame: 4,
                num_hosts: 2,
                applied_frames: 4,
                max_applied_frame: Some(3),
            }),
        };
        let before = snapshot_hash(&snapshot);
        snapshot
            .spectator
            .as_mut()
            .expect("spectator exists")
            .current_frame = 5;
        assert_ne!(before, snapshot_hash(&snapshot));
    }

    fn assert_input_width<I: SimInput>(input: I) {
        let encoded = codec::encode(&input).expect("harness input serializes");
        assert_eq!(
            encoded.len(),
            usize::try_from(I::WIDTH_BYTES).expect("width fits usize"),
            "SimInput::WIDTH_BYTES must match codec width"
        );
        assert_eq!(
            input.fingerprint(),
            InputFingerprint::from_bytes(input.value(), &encoded),
            "SimInput::fingerprint must cover the full codec bytes"
        );
    }

    #[test]
    fn sim_input_widths_match_codec() {
        assert_input_width(input_for::<StubInput>(7, 1));
        assert_input_width(input_for::<WideStubInput>(7, 1));
    }

    #[test]
    fn replacement_generation_prunes_before_loading_snapshot_requests() {
        let mut game = SimGameStub::<StubInput>::new();
        game.recorded.insert(
            4,
            StateStub {
                frame: 4,
                state: 40,
            },
        );
        game.recorded.insert(
            6,
            StateStub {
                frame: 6,
                state: 60,
            },
        );
        game.applied_inputs.insert(5, Vec::new());

        let cell = GameStateCell::<StateStub>::default();
        cell.save(
            Frame::new(5),
            Some(StateStub {
                frame: 5,
                state: 10,
            }),
            Some(0),
        );
        let mut inputs = InputVec::<StubInput>::new();
        inputs.push((StubInput { inp: 1 }, InputStatus::Confirmed));
        let mut requests = RequestVec::<StubConfig>::new();
        requests.push(FortressRequest::LoadGameState {
            cell,
            frame: Frame::new(5),
        });
        requests.push(FortressRequest::AdvanceFrame { inputs });

        let loaded = game.handle_replacement_handoff_requests(requests, 5);

        assert_eq!(loaded, Some(Frame::new(5)));
        assert_eq!(
            game.recorded.get(&4),
            Some(&StateStub {
                frame: 4,
                state: 40
            }),
            "pre-handoff state must remain available for oracle comparison"
        );
        assert_eq!(
            game.recorded.get(&6),
            Some(&StateStub { frame: 6, state: 9 }),
            "same-tick replacement advance must survive handoff pruning"
        );
        assert_eq!(
            game.applied_inputs.get(&5),
            Some(&vec![(
                StubInput { inp: 1 }.fingerprint(),
                InputStatus::Confirmed
            )]),
            "same-tick replacement inputs must replace old-generation data"
        );
    }

    #[cfg(feature = "hot-join")]
    fn test_peer_slot(
        peer: usize,
        schedule: &Schedule,
        clock: &TestClock,
        net: &SimNet<Message>,
        addrs: &[SocketAddr],
    ) -> PeerSlot<StubInput> {
        let socket: SimSocket<Message> = net.attach(addrs[peer]);
        let binding = socket.binding();
        let observer = Arc::new(CollectingObserver::new());
        let protocol_config = peer_protocol_config(schedule, peer, clock);
        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_num_players(addrs.len())
            .expect("valid player count")
            .with_max_prediction_window(1)
            .with_input_delay(0)
            .expect("valid input delay")
            .with_save_mode(schedule.config.save_mode.into())
            .with_desync_detection_mode(DesyncDetection::On {
                interval: schedule.config.desync_interval,
            })
            .with_disconnect_behavior(schedule.config.disconnect_behavior.into())
            .with_protocol_config(protocol_config)
            .with_violation_observer(Arc::clone(&observer) as Arc<_>);
        for (slot, addr) in addrs.iter().enumerate() {
            let player_type = if slot == peer {
                PlayerType::Local
            } else {
                PlayerType::Remote(*addr)
            };
            builder = builder
                .add_player(player_type, PlayerHandle::new(slot))
                .expect("valid player registration");
        }
        PeerSlot {
            session: builder.start_p2p_session(socket).expect("session starts"),
            binding,
            source_addrs: vec![addrs[peer]],
            game: SimGameStub::<StubInput>::new(),
            observer,
            sampled_confirmed: -1,
            protocol_rng_seed: peer_protocol_seed(schedule, peer),
            pending_replacement_handoff_floor: None,
        }
    }

    #[cfg(feature = "hot-join")]
    #[test]
    fn hot_join_protocol_seed_is_deterministic_and_old_generation_distinct() {
        let schedule = schedule::generate(0x5EED_CAFE, schedule::SimConfig::smoke(4));
        let initial = peer_protocol_seed(&schedule, 3);
        let replacement = hot_join_protocol_seed(&schedule, 3, 140, initial);
        let repeated = hot_join_protocol_seed(&schedule, 3, 140, initial);
        let second_replacement = hot_join_protocol_seed(&schedule, 3, 240, replacement);
        let second_repeated = hot_join_protocol_seed(&schedule, 3, 240, replacement);

        assert_eq!(replacement, repeated);
        assert_ne!(replacement, initial);
        assert_ne!(
            protocol_magic_for_seed(replacement),
            protocol_magic_for_seed(initial)
        );
        assert_eq!(second_replacement, second_repeated);
        assert_ne!(second_replacement, replacement);
        assert_ne!(
            protocol_magic_for_seed(second_replacement),
            protocol_magic_for_seed(replacement)
        );
    }

    #[cfg(feature = "hot-join")]
    #[test]
    fn hot_join_protocol_seed_retries_when_nonce_zero_candidate_reuses_previous_magic() {
        let schedule = schedule::generate(0xB24D, schedule::SimConfig::smoke(4));
        let initial = peer_protocol_seed(&schedule, 3);
        let colliding = hot_join_protocol_seed_candidate(&schedule, 3, 140, initial, 0);
        let replacement = hot_join_protocol_seed(&schedule, 3, 140, initial);

        assert_eq!(
            protocol_magic_for_seed(colliding),
            protocol_magic_for_seed(initial)
        );
        assert_ne!(replacement, colliding);
        assert_ne!(
            protocol_magic_for_seed(replacement),
            protocol_magic_for_seed(initial)
        );
    }

    #[cfg(feature = "hot-join")]
    #[test]
    fn schema_v11_hot_join_preserves_same_seed_replay_semantics() {
        let mut schedule = schedule::generate(0x5EED_CAFE, schedule::SimConfig::smoke(4));
        schedule.schema_version = 11;
        let initial = peer_protocol_seed(&schedule, 3);

        assert_eq!(hot_join_protocol_seed(&schedule, 3, 140, initial), initial);
    }

    #[cfg(feature = "hot-join")]
    #[test]
    fn failed_hot_join_session_build_leaves_existing_slot_intact() {
        let n = 2;
        let clock = TestClock::new();
        let net: SimNet<Message> = SimNet::new(0, clock.as_protocol_clock());
        let addrs: Vec<SocketAddr> = (0..n).map(peer_addr).collect();
        let schedule = Schedule {
            schema_version: schedule::SCHEDULE_SCHEMA_VERSION,
            seed: 0,
            link_seed: 0,
            config: schedule::SimConfig {
                n_players: n,
                steps: 10,
                input_delay: 0,
                max_prediction: 0,
                disconnect_behavior: schedule::DropPolicy::ContinueWithout,
                noise: schedule::BackgroundNoise::Clean,
                ..schedule::SimConfig::smoke(n)
            },
            initial_links: Vec::new(),
            events: Vec::new(),
            heal_at: 0,
        };
        let mut peers: Vec<PeerSlot<StubInput>> = (0..n)
            .map(|peer| test_peer_slot(peer, &schedule, &clock, &net, &addrs))
            .collect();
        let dead = vec![false; n];
        let mut oracle = Oracle::new(n);

        {
            let mut ctx = HotJoinRuntime {
                schedule: &schedule,
                clock: &clock,
                net: &net,
                addrs: &addrs,
                peers: &mut peers,
                dead: &dead,
                oracle: &mut oracle,
            };
            start_hot_join_for_slot::<StubInput>(1, 3, &mut ctx);
        }

        assert!(
            !dead[1],
            "failed replacement construction must not retire the old slot"
        );
        assert!(
            peers[1].pending_replacement_handoff_floor.is_none(),
            "failed replacement construction must not arm handoff pruning"
        );
        peers[0]
            .session
            .remove_player(PlayerHandle::new(1))
            .expect("host must not have removed slot before replacement construction failed");

        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::NULL, Frame::NULL],
            &[SessionState::Running, SessionState::Running],
        );
        assert!(
            verdict.failures.iter().any(|failure| {
                matches!(
                    failure,
                    OracleFailure::SessionError {
                        operation: "start_hot_join_session",
                        peer: 1,
                        step: 3,
                        ..
                    }
                )
            }),
            "expected start_hot_join_session failure, got {:?}",
            verdict.failures
        );
    }

    #[test]
    fn default_run_matches_explicit_stub_input_run() {
        let schedule = schedule::generate(
            7,
            schedule::SimConfig {
                steps: 180,
                ..schedule::SimConfig::smoke(2)
            },
        );

        let implicit = run(&schedule, &RunOptions::default());
        let explicit = run_with_input::<StubInput>(&schedule, &RunOptions::default());

        assert_eq!(implicit.trace_hash, explicit.trace_hash);
        assert_eq!(implicit.final_confirmed, explicit.final_confirmed);
        assert_eq!(
            implicit.probe_peer_wire_by_link,
            explicit.probe_peer_wire_by_link
        );
        assert_eq!(implicit.net_stats, explicit.net_stats);
        assert_eq!(
            implicit.blocked_drops_by_link,
            explicit.blocked_drops_by_link
        );
        assert_eq!(
            implicit.fragmentation_drops_by_link,
            explicit.fragmentation_drops_by_link
        );
        assert_eq!(implicit.link_stats_by_link, explicit.link_stats_by_link);
        assert_eq!(implicit.pending_output_probe, explicit.pending_output_probe);
        assert_eq!(
            implicit.load_game_state_observations,
            explicit.load_game_state_observations
        );
        assert_eq!(implicit.recovered_within_b, explicit.recovered_within_b);
        assert_eq!(implicit.violation_census, explicit.violation_census);
        assert_eq!(implicit.peer_event_counts, explicit.peer_event_counts);
        assert_eq!(
            implicit.peer_event_counts_by_peer,
            explicit.peer_event_counts_by_peer
        );
        assert_eq!(
            implicit.peer_event_payload_counts_by_peer,
            explicit.peer_event_payload_counts_by_peer
        );
        assert_eq!(
            implicit.spectator_applied_frames,
            explicit.spectator_applied_frames
        );
        assert_eq!(implicit.spectator_max_frame, explicit.spectator_max_frame);
        assert_eq!(
            implicit.spectator_final_hosts,
            explicit.spectator_final_hosts
        );
        assert_eq!(implicit.verdict.passed(), explicit.verdict.passed());
    }

    #[test]
    fn pending_output_probe_tracks_milestones_and_replay_identity() {
        let mut config = schedule::SimConfig::smoke(2);
        config.steps = 400;
        config.noise = schedule::BackgroundNoise::Clean;
        let mut schedule = schedule::generate(0x5045_4e44, config);
        schedule.events = vec![
            (
                20,
                schedule::ScheduleEvent::Block {
                    from: 0,
                    to: 1,
                    blocked: true,
                },
            ),
            (
                80,
                schedule::ScheduleEvent::Block {
                    from: 0,
                    to: 1,
                    blocked: false,
                },
            ),
            (100, schedule::ScheduleEvent::HealAll),
        ];
        schedule.heal_at = 100;
        let options = RunOptions {
            probe_confirmed_at: Some(79),
            pending_output_probe_link: Some((0, 1)),
            ..RunOptions::default()
        };

        let first = run(&schedule, &options);
        let replay = run(&schedule, &options);
        let probe = first.pending_output_probe.expect("probe requested");
        assert_eq!((probe.from, probe.to), (0, 1));
        assert_eq!(probe.limit, 128);
        assert_eq!(probe.first_reached_limit_at, None);
        assert!(probe.at_probe.is_some_and(|value| value > 0));
        assert!(probe.max >= probe.at_probe.unwrap_or_default());
        assert!(probe.at_heal.is_some());
        assert!(probe.after_recovery.is_some());
        assert_eq!(first.trace_hash, replay.trace_hash);
        assert_eq!(first.pending_output_probe, replay.pending_output_probe);

        let reverse = run(
            &schedule,
            &RunOptions {
                pending_output_probe_link: Some((1, 0)),
                ..options
            },
        );
        assert_ne!(first.trace_hash, reverse.trace_hash);

        let mut retiring = schedule;
        retiring
            .events
            .push((120, schedule::ScheduleEvent::PeerKill { peer: 1 }));
        retiring.events.sort_by_key(|(step, _)| *step);
        assert!(validate_run_options(&retiring, &options).is_err());
    }

    #[test]
    fn end_of_step_wire_probe_is_complete_and_matches_final_cut() {
        let n = 3;
        let schedule = schedule::generate(
            0xA11C_E099,
            schedule::SimConfig {
                steps: 180,
                ..schedule::SimConfig::smoke(n)
            },
        );
        let without_probe = run(&schedule, &RunOptions::default());
        assert!(without_probe.probe_peer_wire_by_link.is_empty());

        let report = run(
            &schedule,
            &RunOptions {
                probe_confirmed_at: Some(schedule.config.steps - 1),
                ..RunOptions::default()
            },
        );
        assert_eq!(report.probe_peer_wire_by_link.len(), n * (n - 1));

        for local in 0..n {
            let links: Vec<&PeerWireTotals> = report
                .probe_peer_wire_by_link
                .iter()
                .filter_map(|(&(from, _), totals)| (from == local).then_some(totals))
                .collect();
            assert_eq!(links.len(), n - 1);
            let aggregate = PeerWireTotals {
                bytes_sent: links.iter().map(|totals| totals.bytes_sent).sum(),
                bytes_received: links.iter().map(|totals| totals.bytes_received).sum(),
                packets_sent: links.iter().map(|totals| totals.packets_sent).sum(),
                packets_received: links.iter().map(|totals| totals.packets_received).sum(),
                messages_sent_by_kind: std::array::from_fn(|index| {
                    links
                        .iter()
                        .map(|totals| totals.messages_sent_by_kind[index])
                        .sum()
                }),
                messages_received_by_kind: std::array::from_fn(|index| {
                    links
                        .iter()
                        .map(|totals| totals.messages_received_by_kind[index])
                        .sum()
                }),
                input_bytes_pre_compression: links
                    .iter()
                    .map(|totals| totals.input_bytes_pre_compression)
                    .sum(),
                input_bytes_post_compression: links
                    .iter()
                    .map(|totals| totals.input_bytes_post_compression)
                    .sum(),
            };
            assert_eq!(aggregate, report.peer_wire[local]);
        }
    }

    #[test]
    fn wire_probe_remains_complete_after_runtime_retirement() {
        let n = 3;
        let mut config = schedule::SimConfig::smoke(n);
        config.steps = 120;
        config.disconnect_behavior = schedule::DropPolicy::ContinueWithout;
        let mut schedule = schedule::generate(0xA11C_E100, config);
        schedule.events = vec![(
            30,
            schedule::ScheduleEvent::GracefulRemove { by: 0, target: 2 },
        )];

        let report = run(
            &schedule,
            &RunOptions {
                probe_confirmed_at: Some(60),
                ..RunOptions::default()
            },
        );
        assert_eq!(report.probe_peer_wire_by_link.len(), n * (n - 1));
        assert!(report.probe_peer_wire_by_link.contains_key(&(0, 2)));
    }
}
