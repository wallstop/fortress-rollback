//! Deterministic whole-mesh simulation network for DST-style testing.
//!
//! [`SimNet`] is the scheduler-controlled network fabric for the simulation
//! harness (`tests/simulation.rs`): one central, seeded, virtual-time message
//! switch that every peer's [`SimSocket`] sends through. It subsumes the
//! per-socket test utilities for whole-mesh use:
//!
//! - [`ChannelSocket`](super::channel_socket::ChannelSocket): instant in-memory
//!   delivery — `SimNet` adds per-directed-link faults on top.
//! - `ChaosSocket`: per-socket loss/latency/jitter with its own RNG — `SimNet`
//!   instead scopes every fault to a *directed link* `(from, to)` and rolls all
//!   faults from **one** seeded RNG, so a whole mesh reproduces from a single
//!   seed and asymmetric conditions (A→B lossy, B→A clean) are first-class.
//! - `FilterSocket`/`BlockedLinks`: directional black-holes — `SimNet`'s
//!   `set_blocked` covers this, mid-run, without wrapping sockets.
//! - `ReorderSocket`/`HeldLinks`: hold-then-FIFO-release — `SimNet`'s
//!   `set_holding` covers this per directed link.
//! - `RoutingBus`/`BusSocket`: re-attach at a vacated address (hot-join) —
//!   `SimNet`'s [`SimNet::attach`]/[`SimNet::detach`] plus
//!   [`UnattachedPolicy::Buffer`] cover this.
//!
//! # Determinism contract
//!
//! With a fixed seed, a fixed virtual clock schedule, and a fixed sequence of
//! `send`/`receive`/control calls, `SimNet` produces a byte-identical delivery
//! trace. All internal maps are `BTreeMap`s, in-flight messages are ordered by
//! `(deliver_at, unique sequence number)` (a total order), and every random
//! roll comes from one seeded [`Pcg32`] consumed in call order. There is no
//! wall-clock read anywhere: time comes exclusively from the injected clock.
//!
//! The payload type is generic so the fault machinery is unit-testable with
//! plain values (`Message` has `pub(crate)` fields and cannot be constructed
//! by integration tests); sessions use the [`Message`] instantiation via the
//! [`NonBlockingSocket`] impl.

// Test infrastructure: not every test binary uses every helper.
#![allow(dead_code)]

use fortress_rollback::hash::fnv1a_hash;
use fortress_rollback::rng::{Pcg32, SeedableRng};
use fortress_rollback::{Message, NonBlockingSocket};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use web_time::Instant;

/// Clock function shared with the sessions under test (same shape as
/// `TestClock::as_protocol_clock`).
pub type SimClockFn = Arc<dyn Fn() -> Instant + Send + Sync>;

/// Upper bound on messages returned by a single `receive_all_messages` call,
/// mirroring the built-in sockets' per-poll decode cap. Excess messages stay
/// queued in the inbox for the next poll (never dropped).
pub const MAX_RECEIVE_MESSAGES_PER_POLL: usize = 256;

/// Modeled UDP payload bytes per IPv4 fragment under a conventional 1500-byte
/// path MTU (20-byte IPv4 header + 8-byte UDP header).
pub const FRAGMENT_PAYLOAD_BYTES: usize = 1472;
/// Defensive ceiling for generic metadata providers. This exceeds the fragment
/// count of the protocol's maximum 64 MiB encoded message.
pub const MAX_MODELED_FRAGMENTS: usize = 65_536;
/// Per-directed-link ceiling for queued bandwidth reservations.
pub const MAX_BANDWIDTH_RESERVATIONS_PER_LINK: usize = 4_096;
/// Whole-fabric ceiling for queued bandwidth reservations.
pub const MAX_BANDWIDTH_RESERVATIONS_TOTAL: usize = 65_536;
/// Maximum burst or queued-byte declaration accepted by direct SimNet users.
pub const MAX_BANDWIDTH_BYTES: u64 = 64 * 1024 * 1024;

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

/// Optional per-fragment loss applied only to oversized datagrams.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FragmentationPolicy {
    /// Independent probability that one modeled fragment is lost.
    pub fragment_drop_rate: f64,
}

/// Deterministic token-bucket shaper with a bounded per-directed-link queue.
///
/// Bytes covered by the burst bucket pass immediately. Excess bytes accrue as
/// service debt at `rate_bytes_per_second`; a send whose debt would exceed
/// `queue_capacity_bytes` is tail-dropped. This models the bufferbloat
/// signature (queueing delay grows before loss) with strictly capped queue
/// metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BandwidthPolicy {
    /// Sustained payload throughput for this directed link.
    pub rate_bytes_per_second: u64,
    /// Bytes that may pass immediately after an idle period.
    pub burst_bytes: u64,
    /// Maximum queued service debt before tail drop.
    pub queue_capacity_bytes: u64,
}

/// Metadata supplied by size-aware [`SimNet`] constructors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimPayloadMetadata {
    pub encoded_len: usize,
    pub is_input: bool,
}

/// Cumulative telemetry for one directed link.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SimLinkStats {
    pub input_sends: u64,
    pub input_delivered_copies: u64,
    pub input_policy_loss_decisions: u64,
    pub input_policy_drops: u64,
    pub max_encoded_input_bytes: u64,
    pub input_sends_over_1200_bytes: u64,
    pub input_sends_over_1472_bytes: u64,
    pub fragmentation_input_losses: u64,
    pub max_consecutive_input_policy_loss_run: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub bandwidth_admitted_datagrams: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub bandwidth_admitted_bytes: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub bandwidth_tail_dropped_bytes: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub bandwidth_tail_drops: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub bandwidth_oversize_drops: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub bandwidth_reservation_cap_drops: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub bandwidth_reservation_cap_dropped_bytes: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub bandwidth_time_overflow_drops: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub bandwidth_queued_datagrams: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub bandwidth_max_queue_bytes: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub bandwidth_max_queue_delay_ns: u64,
}

impl SimLinkStats {
    /// Merges another source-address generation of the same logical link.
    pub fn merge(&mut self, other: Self) {
        self.input_sends = self.input_sends.saturating_add(other.input_sends);
        self.input_delivered_copies = self
            .input_delivered_copies
            .saturating_add(other.input_delivered_copies);
        self.input_policy_loss_decisions = self
            .input_policy_loss_decisions
            .saturating_add(other.input_policy_loss_decisions);
        self.input_policy_drops = self
            .input_policy_drops
            .saturating_add(other.input_policy_drops);
        self.max_encoded_input_bytes = self
            .max_encoded_input_bytes
            .max(other.max_encoded_input_bytes);
        self.input_sends_over_1200_bytes = self
            .input_sends_over_1200_bytes
            .saturating_add(other.input_sends_over_1200_bytes);
        self.input_sends_over_1472_bytes = self
            .input_sends_over_1472_bytes
            .saturating_add(other.input_sends_over_1472_bytes);
        self.fragmentation_input_losses = self
            .fragmentation_input_losses
            .saturating_add(other.fragmentation_input_losses);
        self.max_consecutive_input_policy_loss_run = self
            .max_consecutive_input_policy_loss_run
            .max(other.max_consecutive_input_policy_loss_run);
        self.bandwidth_admitted_bytes = self
            .bandwidth_admitted_bytes
            .saturating_add(other.bandwidth_admitted_bytes);
        self.bandwidth_admitted_datagrams = self
            .bandwidth_admitted_datagrams
            .saturating_add(other.bandwidth_admitted_datagrams);
        self.bandwidth_tail_dropped_bytes = self
            .bandwidth_tail_dropped_bytes
            .saturating_add(other.bandwidth_tail_dropped_bytes);
        self.bandwidth_tail_drops = self
            .bandwidth_tail_drops
            .saturating_add(other.bandwidth_tail_drops);
        self.bandwidth_oversize_drops = self
            .bandwidth_oversize_drops
            .saturating_add(other.bandwidth_oversize_drops);
        self.bandwidth_reservation_cap_drops = self
            .bandwidth_reservation_cap_drops
            .saturating_add(other.bandwidth_reservation_cap_drops);
        self.bandwidth_reservation_cap_dropped_bytes = self
            .bandwidth_reservation_cap_dropped_bytes
            .saturating_add(other.bandwidth_reservation_cap_dropped_bytes);
        self.bandwidth_time_overflow_drops = self
            .bandwidth_time_overflow_drops
            .saturating_add(other.bandwidth_time_overflow_drops);
        self.bandwidth_queued_datagrams = self
            .bandwidth_queued_datagrams
            .saturating_add(other.bandwidth_queued_datagrams);
        self.bandwidth_max_queue_bytes = self
            .bandwidth_max_queue_bytes
            .max(other.bandwidth_max_queue_bytes);
        self.bandwidth_max_queue_delay_ns = self
            .bandwidth_max_queue_delay_ns
            .max(other.bandwidth_max_queue_delay_ns);
    }
}

/// Two-state Markov loss model for one directed link.
///
/// Every probability is validated in `0.0..=1.0` by the schedule boundary.
/// SimNet itself remains a lightweight test utility and assumes its callers
/// provide a valid policy.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GilbertElliottPolicy {
    /// Probability that a send in the good state transitions to bad for the
    /// next admitted send, after the current send's loss decision.
    pub good_to_bad: f64,
    /// Probability that a send in the bad state transitions to good for the
    /// next admitted send, after the current send's loss decision.
    pub bad_to_good: f64,
    /// Per-send loss probability while in the current good state.
    pub good_drop_rate: f64,
    /// Per-send loss probability while in the current bad state.
    pub bad_drop_rate: f64,
}

/// Fault policy for one directed link `(from, to)`.
///
/// Legacy loss rolls are ordered burst then independent drop. For valid
/// materialized schedules, [`Self::gilbert_elliott`] replaces those legacy
/// loss decisions; duplication and jitter still follow any surviving send.
/// When [`Self::retransmit_delay`] is nonzero, a would-be loss becomes a
/// reliable head-of-line delay instead of packet loss.
/// Serializable so schedules can be stored as reproducible corpus artifacts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinkPolicy {
    /// Probability an individual send is dropped.
    pub drop_rate: f64,
    /// Probability an individual send is delivered twice (each copy rolls its
    /// own jitter, so duplicates may arrive out of order).
    pub dup_rate: f64,
    /// Fixed one-way delivery delay.
    pub base_delay: Duration,
    /// Additional uniformly-random delay in `[0, jitter]` per delivered copy.
    pub jitter: Duration,
    /// Probability a send *starts* a loss burst (that send and the following
    /// `burst_len - 1` sends on this link are dropped).
    pub burst_rate: f64,
    /// Total sends dropped per burst, including the send that triggered it.
    pub burst_len: u32,
    /// Reliable retransmission delay for would-be drops. `Duration::ZERO`
    /// keeps UDP-like unreliable loss semantics.
    #[serde(default)]
    pub retransmit_delay: Duration,
    /// Optional two-state correlated-loss channel. A materialized link starts
    /// in the good state; each non-blocked/non-held send rolls its current
    /// state's loss probability first, then the transition for the next send.
    /// `None` preserves the pre-schema-v11 RNG stream exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gilbert_elliott: Option<GilbertElliottPolicy>,
    /// Optional IPv4-style fragmentation-amplified loss. `None` consumes no
    /// fragmentation RNG and preserves legacy traces exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragmentation: Option<FragmentationPolicy>,
    /// Optional token-bucket bandwidth and bounded-queue model. `None`
    /// consumes no time or state and preserves legacy traces exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth: Option<BandwidthPolicy>,
}

impl LinkPolicy {
    /// A perfect link: no loss, no duplication, no delay.
    #[must_use]
    pub fn clean() -> Self {
        Self {
            drop_rate: 0.0,
            dup_rate: 0.0,
            base_delay: Duration::ZERO,
            jitter: Duration::ZERO,
            burst_rate: 0.0,
            burst_len: 0,
            retransmit_delay: Duration::ZERO,
            gilbert_elliott: None,
            fragmentation: None,
            bandwidth: None,
        }
    }
}

impl Default for LinkPolicy {
    fn default() -> Self {
        Self::clean()
    }
}

/// What happens to a message delivered to an address with no attached socket.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnattachedPolicy {
    /// Silently discard, like real UDP to a dead port (counted in
    /// [`SimNetStats::dropped_unattached`]). The default.
    Drop,
    /// Buffer in the destination inbox; a later [`SimNet::attach`] at that
    /// address receives everything buffered while it was away (the
    /// `RoutingBus` hot-join semantics).
    Buffer,
}

/// Delivery/drop counters for premise assertions ("this schedule really did
/// drop traffic") and debugging. All counts are message copies.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SimNetStats {
    /// `send_to` calls observed (before any fault roll).
    pub sent: u64,
    /// Copies placed into an inbox (including buffered-unattached copies).
    pub delivered: u64,
    /// Copies dropped by independent, fixed-burst, or Gilbert-Elliott loss.
    pub dropped_by_policy: u64,
    /// Would-be policy drops converted into reliable retransmission delays.
    pub retransmit_delayed: u64,
    /// Copies dropped because the link was blocked.
    pub dropped_blocked: u64,
    /// Copies dropped on delivery because no socket was attached
    /// (only under [`UnattachedPolicy::Drop`]).
    pub dropped_unattached: u64,
    /// Sends that produced a second copy.
    pub duplicated: u64,
    /// Sends captured by a holding link (delivered later on release).
    pub held: u64,
    /// Good-to-bad state transitions made by Gilbert-Elliott links.
    pub gilbert_elliott_good_to_bad: u64,
    /// Bad-to-good state transitions made by Gilbert-Elliott links.
    pub gilbert_elliott_bad_to_good: u64,
    /// Sends whose Gilbert-Elliott loss decision used the bad state.
    pub gilbert_elliott_bad_sends: u64,
    /// Sends whose Gilbert-Elliott loss decision used the good state.
    pub gilbert_elliott_good_sends: u64,
    /// Loss decisions caused specifically by Gilbert-Elliott state.
    pub gilbert_elliott_loss_events: u64,
    /// Longest consecutive Gilbert-Elliott loss-decision run on any one link.
    pub gilbert_elliott_max_loss_run: u64,
    /// Oversized datagrams admitted to fragmentation modeling.
    pub fragmentation_eligible_sends: u64,
    /// Total fragments modeled across eligible datagrams.
    pub fragmentation_fragments_modeled: u64,
    /// Fragment rolls that selected loss (several may belong to one datagram).
    pub fragmentation_lost_fragments: u64,
    /// Datagrams whose fragmentation rolls selected at least one loss.
    pub fragmentation_loss_events: u64,
    /// Oversized Input datagrams admitted to fragmentation modeling.
    pub fragmentation_input_eligible_sends: u64,
    /// Input datagrams lost because at least one modeled fragment was lost.
    pub fragmentation_input_loss_events: u64,
    /// Largest eligible encoded datagram observed.
    pub fragmentation_max_packet_bytes: u64,
    /// Largest fragment count modeled for one datagram.
    pub fragmentation_max_fragments_per_send: u64,
    /// Metadata providers that exceeded [`MAX_MODELED_FRAGMENTS`] and were
    /// dropped fail-closed without running a truncated probability model.
    pub fragmentation_fragment_cap_hits: u64,
    /// Datagrams admitted to an enabled bandwidth model.
    pub bandwidth_admitted_datagrams: u64,
    /// Payload bytes admitted to enabled bandwidth models.
    pub bandwidth_admitted_bytes: u64,
    /// Datagrams delayed by bandwidth service debt.
    pub bandwidth_queued_datagrams: u64,
    /// Datagrams rejected by bounded-queue tail drop.
    pub bandwidth_tail_drops: u64,
    /// Datagrams larger than the bucket's maximum atomic burst.
    pub bandwidth_oversize_drops: u64,
    /// Datagrams refused by the reservation element-count safety ceiling.
    pub bandwidth_reservation_cap_drops: u64,
    /// Payload bytes refused by the reservation element-count ceiling.
    pub bandwidth_reservation_cap_dropped_bytes: u64,
    /// Datagrams refused because their shaped deadline was unrepresentable.
    pub bandwidth_time_overflow_drops: u64,
    /// Payload bytes rejected by bounded-queue tail drop.
    pub bandwidth_tail_dropped_bytes: u64,
    /// Largest queued service debt observed on any link.
    pub bandwidth_max_queue_bytes: u64,
    /// Largest bandwidth-induced delay observed on any link.
    pub bandwidth_max_queue_delay_ns: u64,
}

/// Runtime state of one directed link.
#[derive(Clone, Debug, Default)]
struct LinkState {
    policy: LinkPolicy,
    /// Remaining sends to drop in the current loss burst.
    burst_remaining: u32,
    /// Reliable transport head-of-line block deadline for this link.
    retransmit_blocked_until: Option<Instant>,
    /// Black-hole: every send is dropped.
    blocked: bool,
    /// Capture-and-hold: sends bypass fault rolls and queue until release.
    holding: bool,
    /// Current Gilbert-Elliott state. Every new/materially replaced link starts
    /// good, independent of whether the optional policy is enabled.
    gilbert_elliott_bad: bool,
    /// Current consecutive GE loss-decision run for this directed link.
    gilbert_elliott_loss_run: u64,
    /// Non-Input traffic deliberately does not reset this run.
    input_policy_loss_run: u64,
    /// Per-materialized-link fragmentation stream, initialized lazily.
    fragmentation_rng: Option<Pcg32>,
    /// Stable seed identity retained across source-address rebinds.
    fragmentation_stream_key: Option<(SocketAddr, SocketAddr)>,
    /// Available whole-byte burst credit at [`bandwidth_updated_at`].
    bandwidth_tokens: u64,
    /// Outstanding shaped bytes waiting for service.
    bandwidth_queued_bytes: u64,
    /// Last virtual-time instant at which token/queue state was settled.
    bandwidth_updated_at: Option<Instant>,
    /// Fractional byte numerator retained across integer refills (`< 1e9`).
    bandwidth_refill_remainder: u64,
    /// Last enabled shaper, retained until its admitted horizon drains even if
    /// the link policy is replaced with a clean one.
    bandwidth_drain_policy: Option<BandwidthPolicy>,
    /// Future shaped departures used only to retire queued-byte accounting.
    bandwidth_reservations: VecDeque<(Instant, u64)>,
    stats: SimLinkStats,
}

impl LinkState {
    fn with_policy(policy: LinkPolicy, key: (SocketAddr, SocketAddr)) -> Self {
        Self {
            policy,
            burst_remaining: 0,
            retransmit_blocked_until: None,
            blocked: false,
            holding: false,
            gilbert_elliott_bad: false,
            gilbert_elliott_loss_run: 0,
            input_policy_loss_run: 0,
            fragmentation_rng: None,
            fragmentation_stream_key: Some(key),
            bandwidth_tokens: 0,
            bandwidth_queued_bytes: 0,
            bandwidth_updated_at: None,
            bandwidth_refill_remainder: 0,
            bandwidth_drain_policy: None,
            bandwidth_reservations: VecDeque::new(),
            stats: SimLinkStats::default(),
        }
    }
}

/// An in-flight message copy, ordered by `(deliver_at, seq)`.
///
/// `seq` is unique per copy, so the ordering is total and same-instant copies
/// deliver in global send order — the determinism backbone.
struct InFlight<M> {
    deliver_at: Instant,
    seq: u64,
    from: SocketAddr,
    to: SocketAddr,
    payload: M,
}

impl<M> PartialEq for InFlight<M> {
    fn eq(&self, other: &Self) -> bool {
        self.deliver_at == other.deliver_at && self.seq == other.seq
    }
}

impl<M> Eq for InFlight<M> {}

impl<M> PartialOrd for InFlight<M> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<M> Ord for InFlight<M> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.deliver_at
            .cmp(&other.deliver_at)
            .then(self.seq.cmp(&other.seq))
    }
}

struct SimNetState<M> {
    clock: SimClockFn,
    rng: Pcg32,
    /// Domain seed for per-materialized-link fragmentation streams.
    fragmentation_seed: u64,
    /// Original source-address identity for fragmentation streams after a
    /// live socket rebind. Historical addresses remain valid for hot join.
    fragmentation_source_identities: BTreeMap<SocketAddr, SocketAddr>,
    payload_metadata: Option<fn(&M) -> SimPayloadMetadata>,
    /// Next unique in-flight sequence number.
    seq: u64,
    /// Policy applied to links that have no explicit state yet. Materialized
    /// into a `LinkState` on a link's first send, so later changes to the
    /// default do not retroactively affect links that already carried traffic
    /// — set the default before traffic flows, use `set_link` afterwards.
    default_policy: LinkPolicy,
    links: BTreeMap<(SocketAddr, SocketAddr), LinkState>,
    /// Messages captured by holding links, FIFO per directed link.
    held: BTreeMap<(SocketAddr, SocketAddr), VecDeque<M>>,
    /// Min-heap of scheduled deliveries (via `Reverse`).
    in_flight: BinaryHeap<std::cmp::Reverse<InFlight<M>>>,
    inboxes: BTreeMap<SocketAddr, VecDeque<(SocketAddr, M)>>,
    unattached: UnattachedPolicy,
    stats: SimNetStats,
    blocked_drop_counts: BTreeMap<(SocketAddr, SocketAddr), u64>,
    fragmentation_drop_counts: BTreeMap<(SocketAddr, SocketAddr), u64>,
    /// Total live entries across every link's bandwidth reservation queue.
    bandwidth_reservation_count: usize,
}

impl<M: Clone> SimNetState<M> {
    fn now(&self) -> Instant {
        (self.clock)()
    }

    /// Uniform roll in `[0, 1)`.
    fn roll_unit(&mut self) -> f64 {
        f64::from(self.rng.next_u32()) / (f64::from(u32::MAX) + 1.0)
    }

    /// Uniform duration in `[0, max]` at millisecond granularity.
    fn roll_jitter(&mut self, max: Duration) -> Duration {
        let max_ms = max.as_millis();
        if max_ms == 0 {
            return Duration::ZERO;
        }
        // Modulo bias is irrelevant for fault injection; determinism is what
        // matters. `max_ms + 1` keeps the range inclusive and fits u64 for
        // any sane jitter configuration.
        let bound = u64::try_from(max_ms + 1).unwrap_or(u64::MAX);
        let ms = self.rng.next_u64() % bound;
        Duration::from_millis(ms)
    }

    fn schedule(&mut self, from: SocketAddr, to: SocketAddr, payload: M, deliver_at: Instant) {
        let seq = self.seq;
        self.seq += 1;
        self.in_flight.push(std::cmp::Reverse(InFlight {
            deliver_at,
            seq,
            from,
            to,
            payload,
        }));
    }

    fn settle_bandwidth_reservations(&mut self, now: Instant) {
        if self.bandwidth_reservation_count == 0 {
            return;
        }
        let mut retired = 0_usize;
        for link in self.links.values_mut() {
            while link
                .bandwidth_reservations
                .front()
                .is_some_and(|(departure, _)| *departure <= now)
            {
                if let Some((_, bytes)) = link.bandwidth_reservations.pop_front() {
                    link.bandwidth_queued_bytes = link.bandwidth_queued_bytes.saturating_sub(bytes);
                    retired = retired.saturating_add(1);
                }
            }
        }
        self.bandwidth_reservation_count = self.bandwidth_reservation_count.saturating_sub(retired);
    }

    /// Applies one directed link's integer token bucket. Returns the earliest
    /// shaped departure, or `None` when the bounded queue drops the copy.
    fn bandwidth_departure(
        &mut self,
        key: (SocketAddr, SocketAddr),
        encoded_len: usize,
        policy: BandwidthPolicy,
    ) -> Option<Instant> {
        let now = self.now();
        let encoded_len = u64::try_from(encoded_len).unwrap_or(u64::MAX);
        let link = self.links.get_mut(&key)?;

        if encoded_len == 0 || encoded_len > policy.burst_bytes {
            self.stats.bandwidth_oversize_drops =
                self.stats.bandwidth_oversize_drops.saturating_add(1);
            link.stats.bandwidth_oversize_drops =
                link.stats.bandwidth_oversize_drops.saturating_add(1);
            return None;
        }

        let mut cursor = link.bandwidth_updated_at.unwrap_or(now);
        let mut tokens = if link.bandwidth_updated_at.is_some() {
            link.bandwidth_tokens
        } else {
            policy.burst_bytes
        }
        .min(policy.burst_bytes);
        let mut remainder = link.bandwidth_refill_remainder;
        if cursor < now {
            let elapsed_ns = now.duration_since(cursor).as_nanos();
            let numerator = elapsed_ns
                .saturating_mul(u128::from(policy.rate_bytes_per_second))
                .saturating_add(u128::from(remainder));
            let refilled = u64::try_from(numerator / 1_000_000_000).unwrap_or(u64::MAX);
            let uncapped = tokens.saturating_add(refilled);
            tokens = uncapped.min(policy.burst_bytes);
            remainder = if uncapped >= policy.burst_bytes {
                0
            } else {
                u64::try_from(numerator % 1_000_000_000).unwrap_or(0)
            };
            cursor = now;
        }

        if encoded_len <= tokens {
            tokens -= encoded_len;
        } else {
            let missing = encoded_len - tokens;
            let needed_numerator = u128::from(missing)
                .saturating_mul(1_000_000_000)
                .saturating_sub(u128::from(remainder));
            let rate = u128::from(policy.rate_bytes_per_second);
            let wait_ns = needed_numerator.div_ceil(rate);
            let wait_ns_u64 = u64::try_from(wait_ns).unwrap_or(u64::MAX);
            let refill_numerator = u128::from(wait_ns_u64)
                .saturating_mul(rate)
                .saturating_add(u128::from(remainder));
            let gained = u64::try_from(refill_numerator / 1_000_000_000).unwrap_or(u64::MAX);
            tokens = tokens.saturating_add(gained).saturating_sub(encoded_len);
            remainder = u64::try_from(refill_numerator % 1_000_000_000).unwrap_or(0);
            let Some(next_cursor) = cursor.checked_add(Duration::from_nanos(wait_ns_u64)) else {
                self.stats.bandwidth_time_overflow_drops =
                    self.stats.bandwidth_time_overflow_drops.saturating_add(1);
                link.stats.bandwidth_time_overflow_drops =
                    link.stats.bandwidth_time_overflow_drops.saturating_add(1);
                return None;
            };
            cursor = next_cursor;
        }
        let departure = cursor;

        let queued = departure > now;
        if queued
            && (link.bandwidth_reservations.len() >= MAX_BANDWIDTH_RESERVATIONS_PER_LINK
                || self.bandwidth_reservation_count >= MAX_BANDWIDTH_RESERVATIONS_TOTAL)
        {
            self.stats.bandwidth_reservation_cap_drops =
                self.stats.bandwidth_reservation_cap_drops.saturating_add(1);
            self.stats.bandwidth_reservation_cap_dropped_bytes = self
                .stats
                .bandwidth_reservation_cap_dropped_bytes
                .saturating_add(encoded_len);
            link.stats.bandwidth_reservation_cap_drops =
                link.stats.bandwidth_reservation_cap_drops.saturating_add(1);
            link.stats.bandwidth_reservation_cap_dropped_bytes = link
                .stats
                .bandwidth_reservation_cap_dropped_bytes
                .saturating_add(encoded_len);
            return None;
        }
        if queued
            && link.bandwidth_queued_bytes.saturating_add(encoded_len) > policy.queue_capacity_bytes
        {
            self.stats.bandwidth_tail_drops = self.stats.bandwidth_tail_drops.saturating_add(1);
            self.stats.bandwidth_tail_dropped_bytes = self
                .stats
                .bandwidth_tail_dropped_bytes
                .saturating_add(encoded_len);
            link.stats.bandwidth_tail_drops = link.stats.bandwidth_tail_drops.saturating_add(1);
            link.stats.bandwidth_tail_dropped_bytes = link
                .stats
                .bandwidth_tail_dropped_bytes
                .saturating_add(encoded_len);
            return None;
        }

        link.bandwidth_tokens = tokens.min(policy.burst_bytes);
        link.bandwidth_refill_remainder = remainder;
        link.bandwidth_updated_at = Some(cursor);
        link.bandwidth_drain_policy = Some(policy);
        self.stats.bandwidth_admitted_datagrams =
            self.stats.bandwidth_admitted_datagrams.saturating_add(1);
        self.stats.bandwidth_admitted_bytes = self
            .stats
            .bandwidth_admitted_bytes
            .saturating_add(encoded_len);
        link.stats.bandwidth_admitted_bytes = link
            .stats
            .bandwidth_admitted_bytes
            .saturating_add(encoded_len);
        link.stats.bandwidth_admitted_datagrams =
            link.stats.bandwidth_admitted_datagrams.saturating_add(1);
        if queued {
            let delay_ns =
                u64::try_from(departure.duration_since(now).as_nanos()).unwrap_or(u64::MAX);
            link.bandwidth_queued_bytes = link.bandwidth_queued_bytes.saturating_add(encoded_len);
            link.bandwidth_reservations
                .push_back((departure, encoded_len));
            self.bandwidth_reservation_count = self.bandwidth_reservation_count.saturating_add(1);
            self.stats.bandwidth_queued_datagrams =
                self.stats.bandwidth_queued_datagrams.saturating_add(1);
            self.stats.bandwidth_max_queue_bytes = self
                .stats
                .bandwidth_max_queue_bytes
                .max(link.bandwidth_queued_bytes);
            self.stats.bandwidth_max_queue_delay_ns =
                self.stats.bandwidth_max_queue_delay_ns.max(delay_ns);
            link.stats.bandwidth_queued_datagrams =
                link.stats.bandwidth_queued_datagrams.saturating_add(1);
            link.stats.bandwidth_max_queue_bytes = link
                .stats
                .bandwidth_max_queue_bytes
                .max(link.bandwidth_queued_bytes);
            link.stats.bandwidth_max_queue_delay_ns =
                link.stats.bandwidth_max_queue_delay_ns.max(delay_ns);
        }
        Some(departure)
    }

    /// Admits a datagram behind an already-shaped horizon after shaping has
    /// been disabled. Followers share the existing departure cut without
    /// extending the old rate limit, while retaining the same byte/element
    /// allocation bounds until that backlog drains.
    fn bandwidth_horizon_departure(
        &mut self,
        key: (SocketAddr, SocketAddr),
        encoded_len: usize,
        policy: BandwidthPolicy,
    ) -> Option<Instant> {
        let now = self.now();
        let encoded_len = u64::try_from(encoded_len).unwrap_or(u64::MAX);
        let link = self.links.get_mut(&key)?;
        let departure = link.bandwidth_updated_at.filter(|cursor| *cursor > now)?;

        if encoded_len == 0 || encoded_len > policy.burst_bytes {
            self.stats.bandwidth_oversize_drops =
                self.stats.bandwidth_oversize_drops.saturating_add(1);
            link.stats.bandwidth_oversize_drops =
                link.stats.bandwidth_oversize_drops.saturating_add(1);
            return None;
        }
        if link.bandwidth_reservations.len() >= MAX_BANDWIDTH_RESERVATIONS_PER_LINK
            || self.bandwidth_reservation_count >= MAX_BANDWIDTH_RESERVATIONS_TOTAL
        {
            self.stats.bandwidth_reservation_cap_drops =
                self.stats.bandwidth_reservation_cap_drops.saturating_add(1);
            self.stats.bandwidth_reservation_cap_dropped_bytes = self
                .stats
                .bandwidth_reservation_cap_dropped_bytes
                .saturating_add(encoded_len);
            link.stats.bandwidth_reservation_cap_drops =
                link.stats.bandwidth_reservation_cap_drops.saturating_add(1);
            link.stats.bandwidth_reservation_cap_dropped_bytes = link
                .stats
                .bandwidth_reservation_cap_dropped_bytes
                .saturating_add(encoded_len);
            return None;
        }
        if link.bandwidth_queued_bytes.saturating_add(encoded_len) > policy.queue_capacity_bytes {
            self.stats.bandwidth_tail_drops = self.stats.bandwidth_tail_drops.saturating_add(1);
            self.stats.bandwidth_tail_dropped_bytes = self
                .stats
                .bandwidth_tail_dropped_bytes
                .saturating_add(encoded_len);
            link.stats.bandwidth_tail_drops = link.stats.bandwidth_tail_drops.saturating_add(1);
            link.stats.bandwidth_tail_dropped_bytes = link
                .stats
                .bandwidth_tail_dropped_bytes
                .saturating_add(encoded_len);
            return None;
        }

        let delay_ns = u64::try_from(departure.duration_since(now).as_nanos()).unwrap_or(u64::MAX);
        link.bandwidth_queued_bytes = link.bandwidth_queued_bytes.saturating_add(encoded_len);
        link.bandwidth_reservations
            .push_back((departure, encoded_len));
        self.bandwidth_reservation_count = self.bandwidth_reservation_count.saturating_add(1);
        self.stats.bandwidth_admitted_datagrams =
            self.stats.bandwidth_admitted_datagrams.saturating_add(1);
        self.stats.bandwidth_admitted_bytes = self
            .stats
            .bandwidth_admitted_bytes
            .saturating_add(encoded_len);
        self.stats.bandwidth_queued_datagrams =
            self.stats.bandwidth_queued_datagrams.saturating_add(1);
        self.stats.bandwidth_max_queue_bytes = self
            .stats
            .bandwidth_max_queue_bytes
            .max(link.bandwidth_queued_bytes);
        self.stats.bandwidth_max_queue_delay_ns =
            self.stats.bandwidth_max_queue_delay_ns.max(delay_ns);
        link.stats.bandwidth_admitted_datagrams =
            link.stats.bandwidth_admitted_datagrams.saturating_add(1);
        link.stats.bandwidth_admitted_bytes = link
            .stats
            .bandwidth_admitted_bytes
            .saturating_add(encoded_len);
        link.stats.bandwidth_queued_datagrams =
            link.stats.bandwidth_queued_datagrams.saturating_add(1);
        link.stats.bandwidth_max_queue_bytes = link
            .stats
            .bandwidth_max_queue_bytes
            .max(link.bandwidth_queued_bytes);
        link.stats.bandwidth_max_queue_delay_ns =
            link.stats.bandwidth_max_queue_delay_ns.max(delay_ns);
        Some(departure)
    }

    /// Returns `true` when a policy loss was converted into a reliable
    /// retransmission and the payload should still be scheduled.
    fn handle_policy_loss(
        &mut self,
        key: (SocketAddr, SocketAddr),
        retransmit_delay: Duration,
    ) -> bool {
        if retransmit_delay.is_zero() {
            self.stats.dropped_by_policy += 1;
            return false;
        }

        self.stats.retransmit_delayed += 1;
        let deadline = self.now() + retransmit_delay;
        if let Some(link) = self.links.get_mut(&key) {
            link.retransmit_blocked_until = Some(
                link.retransmit_blocked_until
                    .map_or(deadline, |existing| existing.max(deadline)),
            );
        }
        true
    }

    fn record_input_send(&mut self, key: (SocketAddr, SocketAddr), metadata: SimPayloadMetadata) {
        if !metadata.is_input {
            return;
        }
        if let Some(link) = self.links.get_mut(&key) {
            link.stats.input_sends = link.stats.input_sends.saturating_add(1);
            link.stats.max_encoded_input_bytes = link
                .stats
                .max_encoded_input_bytes
                .max(u64::try_from(metadata.encoded_len).unwrap_or(u64::MAX));
            if metadata.encoded_len > 1200 {
                link.stats.input_sends_over_1200_bytes =
                    link.stats.input_sends_over_1200_bytes.saturating_add(1);
            }
            if metadata.encoded_len > FRAGMENT_PAYLOAD_BYTES {
                link.stats.input_sends_over_1472_bytes =
                    link.stats.input_sends_over_1472_bytes.saturating_add(1);
            }
        }
    }

    fn record_input_policy_outcome(
        &mut self,
        key: (SocketAddr, SocketAddr),
        is_input: bool,
        lost: bool,
        dropped: bool,
    ) {
        if !is_input {
            return;
        }
        if let Some(link) = self.links.get_mut(&key) {
            if lost {
                link.stats.input_policy_loss_decisions =
                    link.stats.input_policy_loss_decisions.saturating_add(1);
                link.input_policy_loss_run = link.input_policy_loss_run.saturating_add(1);
                link.stats.max_consecutive_input_policy_loss_run = link
                    .stats
                    .max_consecutive_input_policy_loss_run
                    .max(link.input_policy_loss_run);
                if dropped {
                    link.stats.input_policy_drops = link.stats.input_policy_drops.saturating_add(1);
                }
            } else {
                link.input_policy_loss_run = 0;
            }
        }
    }

    /// Evaluates one link's optional two-state channel and advances its state.
    /// Every admitted send consumes exactly two RNG rolls: loss in the current
    /// state, then transition for the next send. The fixed draw count makes GE
    /// traces easy to reproduce while the `None` path consumes no new draws.
    fn gilbert_elliott_loses(
        &mut self,
        key: (SocketAddr, SocketAddr),
        policy: &GilbertElliottPolicy,
    ) -> bool {
        let was_bad = self
            .links
            .get(&key)
            .is_some_and(|link| link.gilbert_elliott_bad);
        if was_bad {
            self.stats.gilbert_elliott_bad_sends += 1;
        } else {
            self.stats.gilbert_elliott_good_sends += 1;
        }
        let drop_rate = if was_bad {
            policy.bad_drop_rate
        } else {
            policy.good_drop_rate
        };
        let loses = self.roll_unit() < drop_rate;
        let transition_rate = if was_bad {
            policy.bad_to_good
        } else {
            policy.good_to_bad
        };
        let transitioned = self.roll_unit() < transition_rate;
        let next_bad = was_bad ^ transitioned;
        if transitioned {
            if was_bad {
                self.stats.gilbert_elliott_bad_to_good += 1;
            } else {
                self.stats.gilbert_elliott_good_to_bad += 1;
            }
        }
        let run = if let Some(link) = self.links.get_mut(&key) {
            link.gilbert_elliott_bad = next_bad;
            if loses {
                link.gilbert_elliott_loss_run += 1;
            } else {
                link.gilbert_elliott_loss_run = 0;
            }
            link.gilbert_elliott_loss_run
        } else {
            0
        };
        if loses {
            self.stats.gilbert_elliott_loss_events += 1;
            self.stats.gilbert_elliott_max_loss_run =
                self.stats.gilbert_elliott_max_loss_run.max(run);
        }
        loses
    }

    /// Rolls every modeled fragment, without short-circuiting, and returns
    /// whether loss of any fragment loses the whole datagram.
    fn fragmentation_loses(
        &mut self,
        key: (SocketAddr, SocketAddr),
        metadata: SimPayloadMetadata,
        policy: FragmentationPolicy,
    ) -> bool {
        if metadata.encoded_len <= FRAGMENT_PAYLOAD_BYTES {
            return false;
        }
        let requested_fragments = metadata.encoded_len.div_ceil(FRAGMENT_PAYLOAD_BYTES);
        let cap_exceeded = requested_fragments > MAX_MODELED_FRAGMENTS;
        let fragments = if cap_exceeded { 0 } else { requested_fragments };
        if cap_exceeded {
            self.stats.fragmentation_fragment_cap_hits =
                self.stats.fragmentation_fragment_cap_hits.saturating_add(1);
        }
        let fragments_u64 = u64::try_from(fragments).unwrap_or(u64::MAX);
        self.stats.fragmentation_eligible_sends =
            self.stats.fragmentation_eligible_sends.saturating_add(1);
        self.stats.fragmentation_fragments_modeled = self
            .stats
            .fragmentation_fragments_modeled
            .saturating_add(fragments_u64);
        self.stats.fragmentation_max_packet_bytes = self
            .stats
            .fragmentation_max_packet_bytes
            .max(u64::try_from(metadata.encoded_len).unwrap_or(u64::MAX));
        self.stats.fragmentation_max_fragments_per_send = self
            .stats
            .fragmentation_max_fragments_per_send
            .max(fragments_u64);
        if metadata.is_input {
            self.stats.fragmentation_input_eligible_sends = self
                .stats
                .fragmentation_input_eligible_sends
                .saturating_add(1);
        }

        let mut lost = cap_exceeded;
        let mut lost_fragments = 0_u64;
        if !cap_exceeded {
            let stream_key = self
                .links
                .get(&key)
                .and_then(|link| link.fragmentation_stream_key)
                .unwrap_or(key);
            let seed = fnv1a_hash(&(self.fragmentation_seed, stream_key));
            for _ in 0..fragments {
                let roll = self.links.get_mut(&key).map_or(1.0, |link| {
                    let rng = link
                        .fragmentation_rng
                        .get_or_insert_with(|| Pcg32::seed_from_u64(seed));
                    f64::from(rng.next_u32()) / (f64::from(u32::MAX) + 1.0)
                });
                if roll < policy.fragment_drop_rate {
                    lost = true;
                    lost_fragments = lost_fragments.saturating_add(1);
                }
            }
        }
        self.stats.fragmentation_lost_fragments = self
            .stats
            .fragmentation_lost_fragments
            .saturating_add(lost_fragments);
        if lost {
            self.stats.fragmentation_loss_events =
                self.stats.fragmentation_loss_events.saturating_add(1);
            if metadata.is_input {
                self.stats.fragmentation_input_loss_events =
                    self.stats.fragmentation_input_loss_events.saturating_add(1);
                if let Some(link) = self.links.get_mut(&key) {
                    link.stats.fragmentation_input_losses =
                        link.stats.fragmentation_input_losses.saturating_add(1);
                }
            }
            let count = self.fragmentation_drop_counts.entry(key).or_default();
            *count = count.saturating_add(1);
        }
        lost
    }

    fn send(&mut self, from: SocketAddr, to: SocketAddr, payload: M) {
        self.stats.sent += 1;
        let key = (from, to);
        if !self.links.contains_key(&key) {
            let stream_from = self
                .fragmentation_source_identities
                .get(&from)
                .copied()
                .unwrap_or(from);
            let state = LinkState::with_policy(self.default_policy.clone(), (stream_from, to));
            self.links.insert(key, state);
        }

        let metadata = self.payload_metadata.map(|metadata| metadata(&payload));
        if let Some(metadata) = metadata {
            self.record_input_send(key, metadata);
        }
        let policy = self
            .links
            .get(&key)
            .map_or_else(LinkPolicy::clean, |link| link.policy.clone());
        assert!(
            policy.fragmentation.is_none() || self.payload_metadata.is_some(),
            "fragmentation policy requires SimNet::new_size_aware metadata"
        );
        assert!(
            policy.bandwidth.is_none() || self.payload_metadata.is_some(),
            "bandwidth policy requires SimNet::new_size_aware metadata"
        );
        assert!(
            policy.bandwidth.is_none_or(|bandwidth| {
                bandwidth.rate_bytes_per_second > 0 && bandwidth.burst_bytes > 0
            }),
            "bandwidth policy requires nonzero rate_bytes_per_second and burst_bytes"
        );
        assert!(
            policy.bandwidth.is_none_or(|bandwidth| {
                bandwidth.burst_bytes <= MAX_BANDWIDTH_BYTES
                    && bandwidth.queue_capacity_bytes <= MAX_BANDWIDTH_BYTES
            }),
            "bandwidth policy burst_bytes and queue_capacity_bytes exceed the modeled bound"
        );
        assert!(
            policy.fragmentation.is_none() || policy.retransmit_delay.is_zero(),
            "fragmentation policy cannot be combined with reliable retransmission"
        );

        // Split borrows: read the link decision first, then roll RNG.
        let (blocked, holding) = match self.links.get(&key) {
            Some(link) => (link.blocked, link.holding),
            None => (false, false),
        };

        if blocked {
            self.stats.dropped_blocked += 1;
            *self.blocked_drop_counts.entry(key).or_default() += 1;
            return;
        }
        if holding {
            self.stats.held += 1;
            self.held.entry(key).or_default().push_back(payload);
            return;
        }

        let shaping_now = self.now();
        self.settle_bandwidth_reservations(shaping_now);

        // The shaper models the sender-side uplink. A datagram consumes that
        // link before downstream loss, fragmentation, or simulated network
        // duplication decides its fate.
        let draining_policy = self.links.get(&key).and_then(|link| {
            (link
                .bandwidth_updated_at
                .is_some_and(|cursor| cursor > shaping_now))
            .then_some(link.bandwidth_drain_policy)
            .flatten()
        });
        let shaped_departure = if let Some(bandwidth) = policy.bandwidth {
            let Some(metadata) = metadata else {
                return;
            };
            let Some(departure) = self.bandwidth_departure(key, metadata.encoded_len, bandwidth)
            else {
                return;
            };
            departure
        } else if let Some(bandwidth) = draining_policy {
            let Some(metadata) = metadata else {
                return;
            };
            let Some(departure) =
                self.bandwidth_horizon_departure(key, metadata.encoded_len, bandwidth)
            else {
                return;
            };
            departure
        } else {
            if let Some(link) = self.links.get_mut(&key) {
                link.bandwidth_drain_policy = None;
            }
            shaping_now
        };

        let is_input = metadata.is_some_and(|metadata| metadata.is_input);
        let mut input_policy_lost = false;

        let gilbert_elliott_lost = policy
            .gilbert_elliott
            .as_ref()
            .is_some_and(|ge| self.gilbert_elliott_loses(key, ge));
        if gilbert_elliott_lost {
            input_policy_lost = true;
            if !self.handle_policy_loss(key, policy.retransmit_delay) {
                self.record_input_policy_outcome(key, is_input, true, true);
                return;
            }
        }

        // Burst state machine (mirrors the ChaosSocket semantics: the
        // triggering send and the following `burst_len - 1` sends drop).
        let in_burst = !gilbert_elliott_lost
            && self
                .links
                .get(&key)
                .is_some_and(|link| link.burst_remaining > 0);
        if in_burst {
            if let Some(link) = self.links.get_mut(&key) {
                link.burst_remaining -= 1;
            }
            input_policy_lost = true;
            if !self.handle_policy_loss(key, policy.retransmit_delay) {
                self.record_input_policy_outcome(key, is_input, true, true);
                return;
            }
        }

        let burst_roll_hit = !gilbert_elliott_lost
            && !in_burst
            && policy.burst_rate > 0.0
            && self.roll_unit() < policy.burst_rate;
        if burst_roll_hit {
            if let Some(link) = self.links.get_mut(&key) {
                link.burst_remaining = policy.burst_len.saturating_sub(1);
            }
            input_policy_lost = true;
            if !self.handle_policy_loss(key, policy.retransmit_delay) {
                self.record_input_policy_outcome(key, is_input, true, true);
                return;
            }
        } else {
            let drop_roll_hit = !gilbert_elliott_lost
                && !in_burst
                && policy.drop_rate > 0.0
                && self.roll_unit() < policy.drop_rate;
            if drop_roll_hit {
                input_policy_lost = true;
                if !self.handle_policy_loss(key, policy.retransmit_delay) {
                    self.record_input_policy_outcome(key, is_input, true, true);
                    return;
                }
            }
        }

        // Consume the same legacy duplication/jitter draws regardless of the
        // independent fragmentation outcome, preserving the base fault story.
        let duplicated = policy.dup_rate > 0.0 && self.roll_unit() < policy.dup_rate;
        let copies = if duplicated { 2 } else { 1 };
        let mut delays = [Duration::ZERO; 2];
        for delay in delays.iter_mut().take(copies) {
            *delay = policy.base_delay + self.roll_jitter(policy.jitter);
        }

        let fragmentation_lost = metadata.is_some_and(|metadata| {
            policy
                .fragmentation
                .is_some_and(|fragmentation| self.fragmentation_loses(key, metadata, fragmentation))
        });
        input_policy_lost |= fragmentation_lost;
        if fragmentation_lost && !self.handle_policy_loss(key, policy.retransmit_delay) {
            self.record_input_policy_outcome(key, is_input, true, true);
            return;
        }

        if duplicated {
            self.stats.duplicated += 1;
        }

        let blocked_until = self
            .links
            .get(&key)
            .and_then(|link| link.retransmit_blocked_until);
        for delay in delays.into_iter().take(copies) {
            let mut deliver_at = shaped_departure + delay;
            if let Some(deadline) = blocked_until {
                deliver_at = deliver_at.max(deadline);
            }
            self.schedule(from, to, payload.clone(), deliver_at);
        }
        self.record_input_policy_outcome(key, is_input, input_policy_lost, false);
    }

    /// Moves every due in-flight copy into its destination inbox.
    fn pump(&mut self) {
        let now = self.now();
        while let Some(std::cmp::Reverse(head)) = self.in_flight.peek() {
            if head.deliver_at > now {
                break;
            }
            let Some(std::cmp::Reverse(msg)) = self.in_flight.pop() else {
                break;
            };
            let is_input = self
                .payload_metadata
                .is_some_and(|metadata| metadata(&msg.payload).is_input);
            match self.inboxes.get_mut(&msg.to) {
                Some(queue) => {
                    queue.push_back((msg.from, msg.payload));
                    self.stats.delivered += 1;
                    if is_input {
                        if let Some(link) = self.links.get_mut(&(msg.from, msg.to)) {
                            link.stats.input_delivered_copies =
                                link.stats.input_delivered_copies.saturating_add(1);
                        }
                    }
                },
                None => match self.unattached {
                    UnattachedPolicy::Drop => self.stats.dropped_unattached += 1,
                    UnattachedPolicy::Buffer => {
                        self.inboxes
                            .entry(msg.to)
                            .or_default()
                            .push_back((msg.from, msg.payload));
                        self.stats.delivered += 1;
                        if is_input {
                            if let Some(link) = self.links.get_mut(&(msg.from, msg.to)) {
                                link.stats.input_delivered_copies =
                                    link.stats.input_delivered_copies.saturating_add(1);
                            }
                        }
                    },
                },
            }
        }
    }

    fn receive(&mut self, addr: SocketAddr) -> Vec<(SocketAddr, M)> {
        self.pump();
        let Some(queue) = self.inboxes.get_mut(&addr) else {
            return Vec::new();
        };
        let take = queue.len().min(MAX_RECEIVE_MESSAGES_PER_POLL);
        queue.drain(..take).collect()
    }

    /// Flushes a link's held queue into delivery at the current instant,
    /// preserving capture order (fresh sequence numbers keep FIFO).
    fn release_held(&mut self, key: (SocketAddr, SocketAddr)) {
        let Some(mut queue) = self.held.remove(&key) else {
            return;
        };
        let now = self.now();
        while let Some(payload) = queue.pop_front() {
            self.schedule(key.0, key.1, payload, now);
        }
    }
}

/// Controller handle for the simulated network. Cheap to clone; all clones
/// (and all [`SimSocket`]s) share the same state.
pub struct SimNet<M = Message> {
    state: Arc<Mutex<SimNetState<M>>>,
}

impl<M> Clone for SimNet<M> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

// Test infrastructure — a poisoned mutex is a test bug, not a recoverable
// condition, so `.expect()` is appropriate here (ChannelSocket precedent).
#[allow(clippy::expect_used)]
impl<M: Clone> SimNet<M> {
    /// Creates a network with the given fault seed and virtual clock.
    ///
    /// Starts with a clean default policy and [`UnattachedPolicy::Drop`].
    #[must_use]
    pub fn new(seed: u64, clock: SimClockFn) -> Self {
        Self::new_inner(seed, clock, None)
    }

    /// Creates a network that can apply encoded-size-aware policies.
    #[must_use]
    pub fn new_size_aware(
        seed: u64,
        clock: SimClockFn,
        payload_metadata: fn(&M) -> SimPayloadMetadata,
    ) -> Self {
        Self::new_inner(seed, clock, Some(payload_metadata))
    }

    fn new_inner(
        seed: u64,
        clock: SimClockFn,
        payload_metadata: Option<fn(&M) -> SimPayloadMetadata>,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(SimNetState {
                clock,
                rng: Pcg32::seed_from_u64(seed),
                fragmentation_seed: seed ^ 0x4652_4147_4d45_4e54,
                fragmentation_source_identities: BTreeMap::new(),
                payload_metadata,
                seq: 0,
                default_policy: LinkPolicy::clean(),
                links: BTreeMap::new(),
                held: BTreeMap::new(),
                in_flight: BinaryHeap::new(),
                inboxes: BTreeMap::new(),
                unattached: UnattachedPolicy::Drop,
                stats: SimNetStats::default(),
                blocked_drop_counts: BTreeMap::new(),
                fragmentation_drop_counts: BTreeMap::new(),
                bandwidth_reservation_count: 0,
            })),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, SimNetState<M>> {
        self.state.lock().expect("SimNet mutex poisoned")
    }

    /// Attaches a socket at `addr`, creating an inbox for it.
    ///
    /// An existing inbox is preserved when a previous `SimSocket` was dropped
    /// without [`Self::detach`], or when unattached-policy traffic created the
    /// inbox. [`Self::detach`] explicitly removes the inbox, so a detach/attach
    /// cycle starts with an empty queue.
    ///
    /// The fabric routes through an address-owned inbox, so multiple live
    /// attachments at one address share that queue. The hot-join harness uses
    /// this briefly while constructing a replacement generation.
    #[must_use]
    pub fn attach(&self, addr: SocketAddr) -> SimSocket<M> {
        let mut state = self.lock();
        state.inboxes.entry(addr).or_default();
        state
            .fragmentation_source_identities
            .entry(addr)
            .or_insert(addr);
        drop(state);
        let binding = Arc::new(Mutex::new(SimSocketBindingState { addr, active: true }));
        SimSocket {
            binding,
            state: Arc::clone(&self.state),
        }
    }

    /// Detaches the socket at `addr`, discarding its inbox. In-flight copies
    /// destined for it follow the [`UnattachedPolicy`] at delivery time.
    pub fn detach(&self, addr: SocketAddr) {
        self.lock().inboxes.remove(&addr);
    }

    /// Sets the policy applied to links on their **first** send. Links that
    /// already carried traffic keep their materialized state — use
    /// [`Self::set_link`] to change those.
    pub fn set_default_policy(&self, policy: LinkPolicy) {
        let mut state = self.lock();
        assert!(
            policy.fragmentation.is_none() || state.payload_metadata.is_some(),
            "fragmentation policy requires SimNet::new_size_aware metadata"
        );
        assert!(
            policy.bandwidth.is_none() || state.payload_metadata.is_some(),
            "bandwidth policy requires SimNet::new_size_aware metadata"
        );
        assert!(
            policy.bandwidth.is_none_or(|bandwidth| {
                bandwidth.rate_bytes_per_second > 0 && bandwidth.burst_bytes > 0
            }),
            "bandwidth policy requires nonzero rate_bytes_per_second and burst_bytes"
        );
        assert!(
            policy.bandwidth.is_none_or(|bandwidth| {
                bandwidth.burst_bytes <= MAX_BANDWIDTH_BYTES
                    && bandwidth.queue_capacity_bytes <= MAX_BANDWIDTH_BYTES
            }),
            "bandwidth policy burst_bytes and queue_capacity_bytes exceed the modeled bound"
        );
        assert!(
            policy.fragmentation.is_none() || policy.retransmit_delay.is_zero(),
            "fragmentation policy cannot be combined with reliable retransmission"
        );
        state.default_policy = policy;
    }

    /// Sets (or replaces) the fault policy for the directed link `from → to`,
    /// preserving its blocked/holding toggles, retransmission head-of-line
    /// deadline, and any already-admitted bandwidth departure horizon.
    /// Replacing a policy resets fixed-burst progress
    /// and starts any Gilbert-Elliott channel in Good with an empty loss run.
    /// The per-link fragmentation RNG continues across policy epochs so a
    /// re-applied policy does not replay the same trial prefix.
    pub fn set_link(&self, from: SocketAddr, to: SocketAddr, policy: LinkPolicy) {
        let mut state = self.lock();
        assert!(
            policy.fragmentation.is_none() || state.payload_metadata.is_some(),
            "fragmentation policy requires SimNet::new_size_aware metadata"
        );
        assert!(
            policy.bandwidth.is_none() || state.payload_metadata.is_some(),
            "bandwidth policy requires SimNet::new_size_aware metadata"
        );
        assert!(
            policy.bandwidth.is_none_or(|bandwidth| {
                bandwidth.rate_bytes_per_second > 0 && bandwidth.burst_bytes > 0
            }),
            "bandwidth policy requires nonzero rate_bytes_per_second and burst_bytes"
        );
        assert!(
            policy.bandwidth.is_none_or(|bandwidth| {
                bandwidth.burst_bytes <= MAX_BANDWIDTH_BYTES
                    && bandwidth.queue_capacity_bytes <= MAX_BANDWIDTH_BYTES
            }),
            "bandwidth policy burst_bytes and queue_capacity_bytes exceed the modeled bound"
        );
        assert!(
            policy.fragmentation.is_none() || policy.retransmit_delay.is_zero(),
            "fragmentation policy cannot be combined with reliable retransmission"
        );
        let key = (from, to);
        let stream_from = state
            .fragmentation_source_identities
            .get(&from)
            .copied()
            .unwrap_or(from);
        let link = state.links.entry(key).or_default();
        link.fragmentation_stream_key
            .get_or_insert((stream_from, to));
        link.policy = policy;
        link.burst_remaining = 0;
        link.gilbert_elliott_bad = false;
        link.gilbert_elliott_loss_run = 0;
        link.input_policy_loss_run = 0;
    }

    /// Sets the same policy on both directions between `a` and `b`.
    pub fn set_link_symmetric(&self, a: SocketAddr, b: SocketAddr, policy: LinkPolicy) {
        self.set_link(a, b, policy.clone());
        self.set_link(b, a, policy);
    }

    /// Black-holes (or restores) the directed link `from → to`. Blocking is
    /// asymmetric by design: the reverse direction is untouched.
    pub fn set_blocked(&self, from: SocketAddr, to: SocketAddr, blocked: bool) {
        let mut state = self.lock();
        let key = (from, to);
        let stream_from = state
            .fragmentation_source_identities
            .get(&from)
            .copied()
            .unwrap_or(from);
        let link = state.links.entry(key).or_default();
        link.fragmentation_stream_key
            .get_or_insert((stream_from, to));
        link.blocked = blocked;
    }

    /// Blocks every directed link between the two groups, in both directions
    /// (a group partition / split brain). Restore with `set_blocked(.., false)`
    /// per link or [`Self::heal_all`].
    pub fn partition(&self, group_a: &[SocketAddr], group_b: &[SocketAddr]) {
        for &a in group_a {
            for &b in group_b {
                self.set_blocked(a, b, true);
                self.set_blocked(b, a, true);
            }
        }
    }

    /// Starts or stops capture-and-hold on the directed link `from → to`.
    /// While holding, sends on the link queue (bypassing fault rolls); on
    /// release they are delivered immediately, in capture order.
    pub fn set_holding(&self, from: SocketAddr, to: SocketAddr, holding: bool) {
        let mut state = self.lock();
        let key = (from, to);
        let stream_from = state
            .fragmentation_source_identities
            .get(&from)
            .copied()
            .unwrap_or(from);
        let link = state.links.entry(key).or_default();
        link.fragmentation_stream_key
            .get_or_insert((stream_from, to));
        link.holding = holding;
        if !holding {
            state.release_held(key);
        }
    }

    /// Resets every link to clean/unblocked/released and the default policy
    /// to clean. Buffered held messages are delivered (FIFO), in-flight
    /// messages keep their scheduled delivery times, and new sends cannot
    /// leapfrog an already-admitted bandwidth backlog.
    pub fn heal_all(&self) {
        let mut state = self.lock();
        state.default_policy = LinkPolicy::clean();
        let keys: Vec<(SocketAddr, SocketAddr)> = state.links.keys().copied().collect();
        for key in keys {
            if let Some(link) = state.links.get_mut(&key) {
                link.policy = LinkPolicy::clean();
                link.burst_remaining = 0;
                link.retransmit_blocked_until = None;
                link.blocked = false;
                link.holding = false;
                link.gilbert_elliott_bad = false;
                link.gilbert_elliott_loss_run = 0;
                link.input_policy_loss_run = 0;
            }
            state.release_held(key);
        }
    }

    /// Sets the policy for messages delivered to unattached addresses.
    pub fn set_unattached_policy(&self, policy: UnattachedPolicy) {
        self.lock().unattached = policy;
    }

    /// Snapshot of the delivery/drop counters.
    #[must_use]
    pub fn stats(&self) -> SimNetStats {
        self.lock().stats
    }

    /// Snapshot of per-directed-link blocked-drop counters.
    #[must_use]
    pub fn blocked_drop_counts(&self) -> BTreeMap<(SocketAddr, SocketAddr), u64> {
        self.lock().blocked_drop_counts.clone()
    }

    /// Snapshot of fragmentation loss events split by directed link.
    #[must_use]
    pub fn fragmentation_drop_counts(&self) -> BTreeMap<(SocketAddr, SocketAddr), u64> {
        self.lock().fragmentation_drop_counts.clone()
    }

    /// Snapshot of cumulative per-directed-link telemetry.
    #[must_use]
    pub fn link_stats(&self) -> BTreeMap<(SocketAddr, SocketAddr), SimLinkStats> {
        self.lock()
            .links
            .iter()
            .map(|(&key, link)| (key, link.stats))
            .collect()
    }
}

/// Why a simulated socket cannot move to a requested address.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SimRebindError {
    /// The requested address is already the socket's current address.
    SameAddress,
    /// Another socket or pre-existing outbound link state already uses the
    /// requested address.
    AddressInUse,
    /// The controlled socket has already been detached.
    Detached,
}

#[derive(Copy, Clone, Debug)]
struct SimSocketBindingState {
    addr: SocketAddr,
    active: bool,
}

/// Out-of-band controller for a live [`SimSocket`]'s local binding.
///
/// The simulation harness keeps this handle after moving the socket into a
/// session so it can model a NAT mapping change without rebuilding that
/// session or changing its peers' canonical destination addresses.
/// Rebinding is address-level: the caller must ensure no other live socket is
/// attached at the old address because its address-owned inbox moves with the
/// binding. Schema validation keeps that operation disjoint from hot join.
#[derive(Clone)]
pub struct SimSocketBinding<M = Message> {
    binding: Arc<Mutex<SimSocketBindingState>>,
    state: Arc<Mutex<SimNetState<M>>>,
}

/// A peer-side socket attached to a [`SimNet`] at a movable local address.
pub struct SimSocket<M = Message> {
    binding: Arc<Mutex<SimSocketBindingState>>,
    state: Arc<Mutex<SimNetState<M>>>,
}

// Test infrastructure — poisoned mutex is a test bug (see SimNet::lock).
#[allow(clippy::expect_used)]
impl<M: Clone> SimSocket<M> {
    /// The address this socket is attached at.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.binding
            .lock()
            .expect("SimSocket binding mutex poisoned")
            .addr
    }

    /// Returns an out-of-band controller for this socket's local binding.
    #[must_use]
    pub fn binding(&self) -> SimSocketBinding<M> {
        SimSocketBinding {
            binding: Arc::clone(&self.binding),
            state: Arc::clone(&self.state),
        }
    }

    /// Sends a payload to `to` through the simulated network.
    pub fn send_payload(&self, to: SocketAddr, payload: M) {
        let binding = self
            .binding
            .lock()
            .expect("SimSocket binding mutex poisoned");
        if !binding.active {
            return;
        }
        let from = binding.addr;
        drop(binding);
        self.state
            .lock()
            .expect("SimNet mutex poisoned")
            .send(from, to, payload);
    }

    /// Receives every payload due for this socket (bounded per call by
    /// [`MAX_RECEIVE_MESSAGES_PER_POLL`]; the remainder stays queued).
    #[must_use]
    pub fn recv_payloads(&self) -> Vec<(SocketAddr, M)> {
        let binding = self
            .binding
            .lock()
            .expect("SimSocket binding mutex poisoned");
        if !binding.active {
            return Vec::new();
        }
        let addr = binding.addr;
        drop(binding);
        self.state
            .lock()
            .expect("SimNet mutex poisoned")
            .receive(addr)
    }
}

// Test infrastructure — poisoned mutexes are test bugs (see SimNet::lock).
#[allow(clippy::expect_used)]
impl<M: Clone> SimSocketBinding<M> {
    /// The address currently used by the controlled socket.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.binding
            .lock()
            .expect("SimSocket binding mutex poisoned")
            .addr
    }

    /// Moves the live socket to `new_addr`, returning its previous address.
    ///
    /// Messages already queued to the live socket remain readable. Messages
    /// still in flight keep their original source and destination. Outbound
    /// link policy/state is cloned to the new source. The old generation is
    /// deliberately retained so pre-rebind held traffic remains discoverable
    /// and releasable by [`SimNet::heal_all`]. Destination policy remains on
    /// the canonical old address so peers that have not learned the new
    /// mapping continue sending into an unattached endpoint. Schema v10 bounds
    /// this history to one rebind per peer and excludes old-address hot join.
    pub fn rebind(&self, new_addr: SocketAddr) -> Result<SocketAddr, SimRebindError> {
        let mut binding = self
            .binding
            .lock()
            .expect("SimSocket binding mutex poisoned");
        if !binding.active {
            return Err(SimRebindError::Detached);
        }
        let old_addr = binding.addr;
        if old_addr == new_addr {
            return Err(SimRebindError::SameAddress);
        }

        let mut state = self.state.lock().expect("SimNet mutex poisoned");
        let new_source_in_use = state.links.keys().any(|(from, _)| *from == new_addr)
            || state.held.keys().any(|(from, _)| *from == new_addr);
        if state.inboxes.contains_key(&new_addr) || new_source_in_use {
            return Err(SimRebindError::AddressInUse);
        }

        let stream_source = state
            .fragmentation_source_identities
            .get(&old_addr)
            .copied()
            .unwrap_or(old_addr);
        state
            .fragmentation_source_identities
            .insert(new_addr, stream_source);

        let outbound_keys: Vec<(SocketAddr, SocketAddr)> = state
            .links
            .keys()
            .copied()
            .filter(|(from, _)| *from == old_addr)
            .collect();
        for old_key in outbound_keys {
            if let Some(mut link) = state.links.get(&old_key).cloned() {
                let new_key = (new_addr, old_key.1);
                link.fragmentation_stream_key.get_or_insert(old_key);
                let continued_run = link.input_policy_loss_run;
                link.stats = SimLinkStats {
                    max_consecutive_input_policy_loss_run: continued_run,
                    ..SimLinkStats::default()
                };
                // The NAT mapping changes, but the socket retains its physical
                // uplink. Move (rather than duplicate) shaping state so the new
                // source cannot leapfrog its pre-rebind backlog or gain a new
                // burst allowance.
                if let Some(old_link) = state.links.get_mut(&old_key) {
                    old_link.bandwidth_tokens = 0;
                    old_link.bandwidth_queued_bytes = 0;
                    old_link.bandwidth_updated_at = None;
                    old_link.bandwidth_refill_remainder = 0;
                    old_link.bandwidth_drain_policy = None;
                    old_link.bandwidth_reservations.clear();
                }
                // Retain `old_key`: `heal_all` discovers and releases held
                // pre-rebind traffic by iterating the historical link keys.
                state.links.insert(new_key, link);
            }
        }

        let queued = state.inboxes.remove(&old_addr).unwrap_or_default();
        state.inboxes.insert(new_addr, queued);
        binding.addr = new_addr;
        Ok(old_addr)
    }

    /// Detaches the controlled socket at its current address.
    pub fn detach(&self) {
        let mut binding = self
            .binding
            .lock()
            .expect("SimSocket binding mutex poisoned");
        if !binding.active {
            return;
        }
        let addr = binding.addr;
        self.state
            .lock()
            .expect("SimNet mutex poisoned")
            .inboxes
            .remove(&addr);
        binding.active = false;
    }
}

impl NonBlockingSocket<SocketAddr> for SimSocket<Message> {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        self.send_payload(*addr, msg.clone());
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        self.recv_payloads()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

    fn addr(port: u16) -> SocketAddr {
        ([127, 0, 0, 1], port).into()
    }

    /// A manually-advanceable clock (local, to avoid a dependency on
    /// `test_clock` from within unit tests of a sibling module).
    fn manual_clock() -> (SimClockFn, Arc<AtomicU64>) {
        let base = Instant::now();
        let offset_ms = Arc::new(AtomicU64::new(0));
        let offset = Arc::clone(&offset_ms);
        let clock: SimClockFn =
            Arc::new(move || base + Duration::from_millis(offset.load(AtomicOrdering::Relaxed)));
        (clock, offset_ms)
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct SizedPayload {
        id: u32,
        encoded_len: usize,
        is_input: bool,
    }

    fn sized_payload_metadata(payload: &SizedPayload) -> SimPayloadMetadata {
        SimPayloadMetadata {
            encoded_len: payload.encoded_len,
            is_input: payload.is_input,
        }
    }

    fn small_u32_metadata(_: &u32) -> SimPayloadMetadata {
        SimPayloadMetadata {
            encoded_len: 4,
            is_input: false,
        }
    }

    /// Drives `sends` identical send sequences through a fresh net and
    /// returns the (receiver-observed) delivery trace.
    fn run_trace(seed: u64, policy: LinkPolicy) -> Vec<(SocketAddr, u32)> {
        let (clock, offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(seed, clock);
        net.set_default_policy(policy);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));

        let mut trace = Vec::new();
        for step in 0..200u32 {
            a.send_payload(addr(2), step);
            b.send_payload(addr(1), 10_000 + step);
            trace.extend(a.recv_payloads());
            trace.extend(b.recv_payloads());
            offset.fetch_add(16, AtomicOrdering::Relaxed);
        }
        // Drain: advance well past any jitter and collect the tail.
        offset.fetch_add(10_000, AtomicOrdering::Relaxed);
        trace.extend(a.recv_payloads());
        trace.extend(b.recv_payloads());
        trace
    }

    #[test]
    fn same_seed_produces_identical_delivery_trace() {
        let policy = LinkPolicy {
            drop_rate: 0.2,
            dup_rate: 0.1,
            base_delay: Duration::from_millis(30),
            jitter: Duration::from_millis(25),
            burst_rate: 0.02,
            burst_len: 4,
            retransmit_delay: Duration::ZERO,
            gilbert_elliott: None,
            fragmentation: None,
            bandwidth: None,
        };
        let first = run_trace(42, policy.clone());
        let second = run_trace(42, policy.clone());
        assert_eq!(first, second, "same seed must reproduce the exact trace");

        let third = run_trace(43, policy);
        assert_ne!(
            first, third,
            "a different seed should perturb a lossy/jittery trace"
        );
    }

    #[test]
    fn clean_link_delivers_everything_in_order() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));

        for i in 0..10 {
            a.send_payload(addr(2), i);
        }
        let received = b.recv_payloads();
        let values: Vec<u32> = received.iter().map(|(_, v)| *v).collect();
        assert_eq!(values, (0..10).collect::<Vec<_>>());
        assert!(received.iter().all(|(from, _)| *from == addr(1)));

        let stats = net.stats();
        assert_eq!(stats.sent, 10);
        assert_eq!(stats.delivered, 10);
        assert_eq!(stats.dropped_by_policy, 0);
    }

    #[test]
    fn link_faults_are_isolated_per_direction() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));

        // A→B drops everything; B→A stays clean.
        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                drop_rate: 1.0,
                ..LinkPolicy::clean()
            },
        );

        for i in 0..5 {
            a.send_payload(addr(2), i);
            b.send_payload(addr(1), 100 + i);
        }
        assert!(b.recv_payloads().is_empty(), "A→B must be fully dropped");
        assert_eq!(
            a.recv_payloads().len(),
            5,
            "B→A must be unaffected by the A→B policy"
        );
    }

    #[test]
    fn blocked_link_is_asymmetric_and_reversible() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));

        net.set_blocked(addr(1), addr(2), true);
        a.send_payload(addr(2), 1);
        b.send_payload(addr(1), 2);
        assert!(b.recv_payloads().is_empty(), "blocked direction drops");
        assert_eq!(a.recv_payloads().len(), 1, "reverse direction flows");

        net.set_blocked(addr(1), addr(2), false);
        a.send_payload(addr(2), 3);
        assert_eq!(b.recv_payloads().len(), 1, "unblocked link flows again");
        assert_eq!(net.stats().dropped_blocked, 1);
    }

    #[test]
    fn delayed_message_arrives_only_after_clock_advance() {
        let (clock, offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        net.set_default_policy(LinkPolicy {
            base_delay: Duration::from_millis(100),
            ..LinkPolicy::clean()
        });
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));

        a.send_payload(addr(2), 1);
        assert!(b.recv_payloads().is_empty(), "not due yet");

        offset.fetch_add(99, AtomicOrdering::Relaxed);
        assert!(b.recv_payloads().is_empty(), "still 1ms early");

        offset.fetch_add(1, AtomicOrdering::Relaxed);
        assert_eq!(b.recv_payloads().len(), 1, "due exactly at base_delay");
    }

    #[test]
    fn retransmit_delay_delivers_would_drop_and_holds_later_sends() {
        let (clock, offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));

        let reliable_loss = LinkPolicy {
            drop_rate: 1.0,
            retransmit_delay: Duration::from_millis(100),
            ..LinkPolicy::clean()
        };
        let reliable_clean = LinkPolicy {
            drop_rate: 0.0,
            retransmit_delay: Duration::from_millis(100),
            ..LinkPolicy::clean()
        };

        net.set_link(addr(1), addr(2), reliable_loss);
        a.send_payload(addr(2), 1);
        assert!(
            b.recv_payloads().is_empty(),
            "would-drop payload is retransmission-delayed, not delivered immediately"
        );

        net.set_link(addr(1), addr(2), reliable_clean);
        offset.fetch_add(10, AtomicOrdering::Relaxed);
        a.send_payload(addr(2), 2);
        offset.fetch_add(89, AtomicOrdering::Relaxed);
        assert!(
            b.recv_payloads().is_empty(),
            "later clean sends stay behind the retransmission deadline"
        );

        offset.fetch_add(1, AtomicOrdering::Relaxed);
        let values: Vec<u32> = b.recv_payloads().into_iter().map(|(_, v)| v).collect();
        assert_eq!(
            values,
            vec![1, 2],
            "retransmitted and later clean sends deliver FIFO at the HOL deadline"
        );

        let stats = net.stats();
        assert_eq!(stats.sent, 2);
        assert_eq!(stats.retransmit_delayed, 1);
        assert_eq!(stats.dropped_by_policy, 0);
        assert_eq!(stats.delivered, 2);
    }

    #[test]
    fn duplication_delivers_exactly_two_copies() {
        let (clock, offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        net.set_default_policy(LinkPolicy {
            dup_rate: 1.0,
            ..LinkPolicy::clean()
        });
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));

        a.send_payload(addr(2), 9);
        offset.fetch_add(1, AtomicOrdering::Relaxed);
        let received = b.recv_payloads();
        assert_eq!(received.len(), 2, "dup_rate=1 must deliver two copies");
        assert!(received.iter().all(|(_, v)| *v == 9));
        assert_eq!(net.stats().duplicated, 1);
    }

    #[test]
    fn hold_then_release_preserves_fifo_order() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));

        net.set_holding(addr(1), addr(2), true);
        for i in 0..5 {
            a.send_payload(addr(2), i);
        }
        assert!(b.recv_payloads().is_empty(), "held messages must not leak");
        assert_eq!(net.stats().held, 5);

        net.set_holding(addr(1), addr(2), false);
        let values: Vec<u32> = b.recv_payloads().into_iter().map(|(_, v)| v).collect();
        assert_eq!(values, vec![0, 1, 2, 3, 4], "release must preserve order");
    }

    #[test]
    fn hold_can_invert_arrival_order_across_links() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));
        let c = net.attach(addr(3));

        // Hold A→C while A→B flows, then release: C sees "newer" traffic from
        // B (relayed) before A's older direct messages — the cross-link
        // reorder primitive.
        net.set_holding(addr(1), addr(3), true);
        a.send_payload(addr(3), 1); // old, held
        a.send_payload(addr(2), 2); // flows to B
        let via_b = b.recv_payloads();
        assert_eq!(via_b.len(), 1);
        b.send_payload(addr(3), 3); // newer, arrives first
        assert_eq!(
            c.recv_payloads()
                .into_iter()
                .map(|(_, v)| v)
                .collect::<Vec<_>>(),
            vec![3]
        );

        net.set_holding(addr(1), addr(3), false);
        assert_eq!(
            c.recv_payloads()
                .into_iter()
                .map(|(_, v)| v)
                .collect::<Vec<_>>(),
            vec![1],
            "held message arrives after the newer relayed one"
        );
    }

    #[test]
    fn detach_drops_and_reattach_receives_fresh_traffic() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let _b = net.attach(addr(2));

        net.detach(addr(2));
        a.send_payload(addr(2), 1);
        // Force delivery attempt while detached (Drop policy).
        let _ = a.recv_payloads();
        assert_eq!(net.stats().dropped_unattached, 1);

        let b2 = net.attach(addr(2));
        a.send_payload(addr(2), 2);
        assert_eq!(
            b2.recv_payloads()
                .into_iter()
                .map(|(_, v)| v)
                .collect::<Vec<_>>(),
            vec![2],
            "re-attached socket receives post-attach traffic only"
        );
    }

    #[test]
    fn rebind_moves_live_socket_and_preserves_outbound_policy() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let old = addr(1);
        let fresh = addr(3);
        let peer = addr(2);
        let socket = net.attach(old);
        let binding = socket.binding();
        let remote = net.attach(peer);

        net.set_link(
            old,
            peer,
            LinkPolicy {
                drop_rate: 1.0,
                ..LinkPolicy::clean()
            },
        );
        remote.send_payload(old, 9);
        assert!(remote.recv_payloads().is_empty()); // pump 9 into the live socket inbox
        remote.send_payload(old, 10); // remains in flight until after the rebind

        assert_eq!(binding.rebind(fresh), Ok(old));
        assert_eq!(socket.local_addr(), fresh);
        assert_eq!(binding.local_addr(), fresh);
        assert_eq!(
            socket.recv_payloads(),
            vec![(peer, 9)],
            "already-queued traffic belongs to the live socket and must survive the rebind"
        );
        assert_eq!(net.stats().dropped_unattached, 1);

        socket.send_payload(peer, 20);
        assert!(
            remote.recv_payloads().is_empty(),
            "the old outbound drop policy must follow the rebound sender"
        );

        net.heal_all();
        socket.send_payload(peer, 30);
        assert_eq!(remote.recv_payloads(), vec![(fresh, 30)]);
        remote.send_payload(fresh, 40);
        assert_eq!(socket.recv_payloads(), vec![(peer, 40)]);

        binding.detach();
        assert_eq!(binding.rebind(old), Err(SimRebindError::Detached));
        socket.send_payload(peer, 45);
        assert!(remote.recv_payloads().is_empty());
        remote.send_payload(fresh, 50);
        let _ = remote.recv_payloads();
        assert_eq!(net.stats().dropped_unattached, 2);
    }

    #[test]
    fn rebind_collision_rejects_without_partial_mutation() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let binding = a.binding();
        let occupied = net.attach(addr(2));

        assert_eq!(binding.rebind(addr(1)), Err(SimRebindError::SameAddress));
        assert_eq!(binding.rebind(addr(2)), Err(SimRebindError::AddressInUse));
        assert_eq!(a.local_addr(), addr(1));

        a.send_payload(addr(2), 7);
        assert_eq!(occupied.recv_payloads(), vec![(addr(1), 7)]);
    }

    #[test]
    fn rebind_keeps_pre_and_post_generation_held_traffic_releasable() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let old = addr(1);
        let fresh = addr(3);
        let peer = addr(2);
        let socket = net.attach(old);
        let binding = socket.binding();
        let remote = net.attach(peer);

        net.set_holding(old, peer, true);
        socket.send_payload(peer, 1);
        assert_eq!(binding.rebind(fresh), Ok(old));
        socket.send_payload(peer, 2);
        assert!(remote.recv_payloads().is_empty());

        net.heal_all();
        assert_eq!(remote.recv_payloads(), vec![(old, 1), (fresh, 2)]);
    }

    #[test]
    fn buffer_policy_preserves_traffic_across_reattach() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        net.set_unattached_policy(UnattachedPolicy::Buffer);
        let a = net.attach(addr(1));

        a.send_payload(addr(2), 1);
        let _ = a.recv_payloads(); // pump delivery into the buffered inbox

        let b = net.attach(addr(2));
        assert_eq!(
            b.recv_payloads()
                .into_iter()
                .map(|(_, v)| v)
                .collect::<Vec<_>>(),
            vec![1],
            "buffered unattached traffic must survive until attach"
        );
    }

    #[test]
    fn receive_is_bounded_per_poll_and_never_loses_the_remainder() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));

        let total = MAX_RECEIVE_MESSAGES_PER_POLL + 40;
        for i in 0..total {
            a.send_payload(addr(2), u32::try_from(i).unwrap());
        }
        let first = b.recv_payloads();
        assert_eq!(first.len(), MAX_RECEIVE_MESSAGES_PER_POLL);
        let second = b.recv_payloads();
        assert_eq!(second.len(), 40, "remainder must arrive on the next poll");
        assert_eq!(
            second.first().map(|(_, v)| *v),
            Some(u32::try_from(MAX_RECEIVE_MESSAGES_PER_POLL).unwrap()),
            "remainder must continue in order"
        );
    }

    #[test]
    fn burst_drops_consecutive_sends() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        net.set_default_policy(LinkPolicy {
            burst_rate: 1.0, // every non-burst send starts a new burst
            burst_len: 3,
            ..LinkPolicy::clean()
        });
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));

        for i in 0..9 {
            a.send_payload(addr(2), i);
        }
        assert!(
            b.recv_payloads().is_empty(),
            "burst_rate=1 with burst_len=3 drops every send (back-to-back bursts)"
        );
        assert_eq!(net.stats().dropped_by_policy, 9);
    }

    #[test]
    fn gilbert_elliott_rolls_loss_then_transition_and_tracks_per_link_runs() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            gilbert_elliott: Some(GilbertElliottPolicy {
                good_to_bad: 1.0,
                bad_to_good: 0.0,
                good_drop_rate: 0.0,
                bad_drop_rate: 1.0,
            }),
            ..LinkPolicy::clean()
        });

        for value in 0..5 {
            a.send_payload(addr(2), value);
        }
        assert_eq!(b.recv_payloads(), vec![(addr(1), 0)]);
        assert_eq!(
            net.stats(),
            SimNetStats {
                sent: 5,
                delivered: 1,
                dropped_by_policy: 4,
                gilbert_elliott_good_to_bad: 1,
                gilbert_elliott_bad_sends: 4,
                gilbert_elliott_good_sends: 1,
                gilbert_elliott_loss_events: 4,
                gilbert_elliott_max_loss_run: 4,
                ..SimNetStats::default()
            }
        );

        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                gilbert_elliott: Some(GilbertElliottPolicy {
                    good_to_bad: 1.0,
                    bad_to_good: 1.0,
                    good_drop_rate: 0.0,
                    bad_drop_rate: 1.0,
                }),
                ..LinkPolicy::clean()
            },
        );
        a.send_payload(addr(2), 10);
        a.send_payload(addr(2), 11);
        a.send_payload(addr(2), 12);
        assert_eq!(b.recv_payloads(), vec![(addr(1), 10), (addr(1), 12)]);
        let stats = net.stats();
        assert_eq!(stats.gilbert_elliott_good_to_bad, 3);
        assert_eq!(stats.gilbert_elliott_bad_to_good, 1);
        assert_eq!(stats.gilbert_elliott_loss_events, 5);
        assert_eq!(stats.gilbert_elliott_max_loss_run, 4);
    }

    #[test]
    fn gilbert_elliott_bypasses_blocked_and_held_sends_and_resets_per_link() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));
        let c = net.attach(addr(3));
        let ge = LinkPolicy {
            gilbert_elliott: Some(GilbertElliottPolicy {
                good_to_bad: 1.0,
                bad_to_good: 0.0,
                good_drop_rate: 0.0,
                bad_drop_rate: 1.0,
            }),
            ..LinkPolicy::clean()
        };
        net.set_link(addr(1), addr(2), ge.clone());
        net.set_link(addr(1), addr(3), ge.clone());

        net.set_blocked(addr(1), addr(2), true);
        a.send_payload(addr(2), 1);
        net.set_blocked(addr(1), addr(2), false);
        net.set_holding(addr(1), addr(3), true);
        a.send_payload(addr(3), 2);
        net.set_holding(addr(1), addr(3), false);
        assert_eq!(c.recv_payloads(), vec![(addr(1), 2)]);
        assert_eq!(net.stats().gilbert_elliott_good_sends, 0);

        a.send_payload(addr(2), 3);
        a.send_payload(addr(2), 4);
        a.send_payload(addr(3), 5);
        assert_eq!(b.recv_payloads(), vec![(addr(1), 3)]);
        assert_eq!(c.recv_payloads(), vec![(addr(1), 5)]);
        let before_reset = net.stats();
        assert_eq!(before_reset.gilbert_elliott_good_sends, 2);
        assert_eq!(before_reset.gilbert_elliott_bad_sends, 1);
        assert_eq!(before_reset.gilbert_elliott_loss_events, 1);

        net.set_link(addr(1), addr(2), ge);
        a.send_payload(addr(2), 6);
        assert_eq!(b.recv_payloads(), vec![(addr(1), 6)]);
        net.heal_all();
        a.send_payload(addr(2), 7);
        assert_eq!(b.recv_payloads(), vec![(addr(1), 7)]);
        assert_eq!(net.stats().gilbert_elliott_good_sends, 3);
    }

    #[test]
    fn gilbert_elliott_losses_become_reliable_fifo_retransmissions() {
        let (clock, offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));
        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                retransmit_delay: Duration::from_millis(100),
                gilbert_elliott: Some(GilbertElliottPolicy {
                    good_to_bad: 1.0,
                    bad_to_good: 1.0,
                    good_drop_rate: 1.0,
                    bad_drop_rate: 1.0,
                }),
                ..LinkPolicy::clean()
            },
        );

        a.send_payload(addr(2), 1);
        a.send_payload(addr(2), 2);
        assert!(b.recv_payloads().is_empty());
        offset.store(100, AtomicOrdering::Relaxed);
        assert_eq!(b.recv_payloads(), vec![(addr(1), 1), (addr(1), 2)]);
        let stats = net.stats();
        assert_eq!(stats.gilbert_elliott_good_sends, 1);
        assert_eq!(stats.gilbert_elliott_bad_sends, 1);
        assert_eq!(stats.gilbert_elliott_good_to_bad, 1);
        assert_eq!(stats.gilbert_elliott_bad_to_good, 1);
        assert_eq!(stats.gilbert_elliott_loss_events, 2);
        assert_eq!(stats.retransmit_delayed, 2);
        assert_eq!(stats.dropped_by_policy, 0);
    }

    #[test]
    fn gilbert_elliott_state_follows_rebound_source_generation() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let old = addr(1);
        let peer = addr(2);
        let fresh = addr(3);
        let socket = net.attach(old);
        let binding = socket.binding();
        let remote = net.attach(peer);
        net.set_link(
            old,
            peer,
            LinkPolicy {
                gilbert_elliott: Some(GilbertElliottPolicy {
                    good_to_bad: 1.0,
                    bad_to_good: 0.0,
                    good_drop_rate: 0.0,
                    bad_drop_rate: 1.0,
                }),
                ..LinkPolicy::clean()
            },
        );

        socket.send_payload(peer, 1);
        assert_eq!(remote.recv_payloads(), vec![(old, 1)]);
        assert_eq!(binding.rebind(fresh), Ok(old));
        socket.send_payload(peer, 2);
        assert!(remote.recv_payloads().is_empty());
        assert_eq!(net.stats().gilbert_elliott_bad_sends, 1);
        assert_eq!(net.stats().gilbert_elliott_loss_events, 1);
    }

    #[test]
    fn gilbert_elliott_long_run_matches_stationary_loss_envelope() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(0xCE45_0E11, clock);
        let a = net.attach(addr(1));
        let _b = net.attach(addr(2));
        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                gilbert_elliott: Some(GilbertElliottPolicy {
                    good_to_bad: 0.02,
                    bad_to_good: 0.08,
                    good_drop_rate: 0.01,
                    bad_drop_rate: 0.60,
                }),
                ..LinkPolicy::clean()
            },
        );

        for value in 0..20_000 {
            a.send_payload(addr(2), value);
        }
        let stats = net.stats();
        let bad_fraction = stats.gilbert_elliott_bad_sends as f64 / 20_000.0;
        let loss_fraction = stats.gilbert_elliott_loss_events as f64 / 20_000.0;
        assert!((0.15..=0.25).contains(&bad_fraction), "{stats:?}");
        assert!((0.09..=0.17).contains(&loss_fraction), "{stats:?}");
        assert!(stats.gilbert_elliott_good_to_bad > 100, "{stats:?}");
        assert!(stats.gilbert_elliott_bad_to_good > 100, "{stats:?}");
    }

    #[test]
    fn heal_all_restores_clean_links_and_flushes_held() {
        let (clock, _offset) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let a = net.attach(addr(1));
        let b = net.attach(addr(2));

        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                drop_rate: 1.0,
                ..LinkPolicy::clean()
            },
        );
        net.set_blocked(addr(2), addr(1), true);
        net.set_holding(addr(1), addr(2), false);
        a.send_payload(addr(2), 1); // dropped by policy
        assert!(b.recv_payloads().is_empty());

        net.heal_all();
        a.send_payload(addr(2), 2);
        b.send_payload(addr(1), 3);
        assert_eq!(b.recv_payloads().len(), 1, "healed link delivers");
        assert_eq!(a.recv_payloads().len(), 1, "healed block delivers");
    }

    #[test]
    fn fragmentation_uses_fixed_1472_boundary_and_rolls_every_fragment() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 1.0,
            }),
            ..LinkPolicy::clean()
        });

        for (id, encoded_len) in [(1, 1472), (2, 1473), (3, 2944), (4, 2945)] {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len,
                    is_input: true,
                },
            );
        }

        assert_eq!(receiver.recv_payloads().len(), 1);
        let stats = net.stats();
        assert_eq!(stats.fragmentation_eligible_sends, 3);
        assert_eq!(stats.fragmentation_fragments_modeled, 7);
        assert_eq!(stats.fragmentation_lost_fragments, 7);
        assert_eq!(stats.fragmentation_loss_events, 3);
        assert_eq!(stats.fragmentation_input_eligible_sends, 3);
        assert_eq!(stats.fragmentation_input_loss_events, 3);
        assert_eq!(stats.fragmentation_max_packet_bytes, 2945);
        assert_eq!(stats.fragmentation_max_fragments_per_send, 3);
        assert_eq!(net.fragmentation_drop_counts()[&(addr(1), addr(2))], 3);

        let link = net.link_stats()[&(addr(1), addr(2))];
        assert_eq!(link.input_sends, 4);
        assert_eq!(link.input_delivered_copies, 1);
        assert_eq!(link.input_policy_loss_decisions, 3);
        assert_eq!(link.max_consecutive_input_policy_loss_run, 3);
        assert_eq!(link.max_encoded_input_bytes, 2945);
        assert_eq!(link.input_sends_over_1200_bytes, 4);
        assert_eq!(link.input_sends_over_1472_bytes, 3);
        assert_eq!(link.fragmentation_input_losses, 3);
        assert_eq!(link.input_policy_drops, 3);
    }

    #[test]
    fn bandwidth_queue_delays_before_exact_capacity_tail_drop() {
        let (clock, offset) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 1_000,
                burst_bytes: 100,
                queue_capacity_bytes: 200,
            }),
            ..LinkPolicy::clean()
        });

        for id in 1..=4 {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 100,
                    is_input: false,
                },
            );
        }

        assert_eq!(
            receiver
                .recv_payloads()
                .into_iter()
                .map(|(_, payload)| payload.id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        offset.fetch_add(99, AtomicOrdering::Relaxed);
        assert!(receiver.recv_payloads().is_empty());
        offset.fetch_add(1, AtomicOrdering::Relaxed);
        assert_eq!(receiver.recv_payloads()[0].1.id, 2);
        offset.fetch_add(100, AtomicOrdering::Relaxed);
        assert_eq!(receiver.recv_payloads()[0].1.id, 3);

        let stats = net.stats();
        assert_eq!(stats.bandwidth_admitted_datagrams, 3);
        assert_eq!(stats.bandwidth_admitted_bytes, 300);
        assert_eq!(stats.bandwidth_queued_datagrams, 2);
        assert_eq!(stats.bandwidth_tail_drops, 1);
        assert_eq!(stats.bandwidth_tail_dropped_bytes, 100);
        assert_eq!(stats.bandwidth_oversize_drops, 0);
        assert_eq!(stats.bandwidth_max_queue_bytes, 200);
        assert_eq!(stats.bandwidth_max_queue_delay_ns, 200_000_000);

        let link = net.link_stats()[&(addr(1), addr(2))];
        assert_eq!(link.bandwidth_admitted_bytes, 300);
        assert_eq!(link.bandwidth_queued_datagrams, 2);
        assert_eq!(link.bandwidth_tail_drops, 1);
        assert_eq!(link.bandwidth_tail_dropped_bytes, 100);
        assert_eq!(link.bandwidth_max_queue_bytes, 200);
        assert_eq!(link.bandwidth_max_queue_delay_ns, 200_000_000);
    }

    #[test]
    fn bandwidth_refill_preserves_fractional_nanosecond_remainder() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let _receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 3,
                burst_bytes: 1,
                queue_capacity_bytes: 2,
            }),
            ..LinkPolicy::clean()
        });
        for id in 1..=3 {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 1,
                    is_input: false,
                },
            );
        }

        assert_eq!(net.stats().bandwidth_queued_datagrams, 2);
        assert_eq!(net.stats().bandwidth_max_queue_delay_ns, 666_666_667);
    }

    #[test]
    fn bandwidth_deadline_overflow_fails_closed_without_queue_mutation() {
        let base = Instant::now();
        let mut low = 0_u64;
        let mut high = u64::MAX;
        while low < high {
            let mid = low + (high - low) / 2 + 1;
            if base.checked_add(Duration::from_secs(mid)).is_some() {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        let near_limit = base
            .checked_add(Duration::from_secs(low))
            .expect("binary search retains a representable Instant");
        assert!(near_limit.checked_add(Duration::from_secs(1)).is_none());

        let clock: SimClockFn = Arc::new(move || near_limit);
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 1,
                burst_bytes: 1,
                queue_capacity_bytes: 1,
            }),
            ..LinkPolicy::clean()
        });
        for id in 1..=2 {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 1,
                    is_input: false,
                },
            );
        }

        assert_eq!(receiver.recv_payloads()[0].1.id, 1);
        assert_eq!(net.stats().bandwidth_admitted_datagrams, 1);
        assert_eq!(net.stats().bandwidth_time_overflow_drops, 1);
        assert_eq!(net.stats().bandwidth_queued_datagrams, 0);
        assert_eq!(net.lock().bandwidth_reservation_count, 0);
    }

    #[test]
    fn settling_one_send_retires_expired_reservations_on_idle_links() {
        let (clock, offset) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let a = net.attach(addr(1));
        let _b = net.attach(addr(2));
        let c = net.attach(addr(3));
        let _d = net.attach(addr(4));
        let policy = LinkPolicy {
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 1,
                burst_bytes: 1,
                queue_capacity_bytes: 1,
            }),
            ..LinkPolicy::clean()
        };
        net.set_link(addr(1), addr(2), policy.clone());
        net.set_link(addr(3), addr(4), policy);
        for id in 1..=2 {
            a.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 1,
                    is_input: false,
                },
            );
        }
        assert_eq!(net.lock().bandwidth_reservation_count, 1);

        offset.fetch_add(1_000, AtomicOrdering::Relaxed);
        for id in 3..=4 {
            c.send_payload(
                addr(4),
                SizedPayload {
                    id,
                    encoded_len: 1,
                    is_input: false,
                },
            );
        }
        let state = net.lock();
        assert_eq!(state.bandwidth_reservation_count, 1);
        assert!(state.links[&(addr(1), addr(2))]
            .bandwidth_reservations
            .is_empty());
    }

    #[test]
    fn bandwidth_rejects_atomic_payload_above_burst_without_wedging() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 1_000,
                burst_bytes: 99,
                queue_capacity_bytes: 10_000,
            }),
            ..LinkPolicy::clean()
        });
        sender.send_payload(
            addr(2),
            SizedPayload {
                id: 1,
                encoded_len: 100,
                is_input: false,
            },
        );

        assert!(receiver.recv_payloads().is_empty());
        assert_eq!(net.stats().bandwidth_oversize_drops, 1);
        assert_eq!(
            net.link_stats()[&(addr(1), addr(2))].bandwidth_oversize_drops,
            1
        );
    }

    #[test]
    fn downstream_duplication_consumes_one_uplink_admission() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            dup_rate: 1.0,
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 1_000,
                burst_bytes: 100,
                queue_capacity_bytes: 100,
            }),
            ..LinkPolicy::clean()
        });
        sender.send_payload(
            addr(2),
            SizedPayload {
                id: 1,
                encoded_len: 100,
                is_input: false,
            },
        );

        assert_eq!(receiver.recv_payloads().len(), 2);
        assert_eq!(net.stats().duplicated, 1);
        assert_eq!(net.stats().bandwidth_admitted_datagrams, 1);
        assert_eq!(net.stats().bandwidth_queued_datagrams, 0);
    }

    #[test]
    #[should_panic(expected = "bandwidth policy requires SimNet::new_size_aware metadata")]
    fn bandwidth_without_payload_metadata_fails_loudly() {
        let (clock, _) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        net.set_default_policy(LinkPolicy {
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 1_000,
                burst_bytes: 100,
                queue_capacity_bytes: 100,
            }),
            ..LinkPolicy::clean()
        });
    }

    #[test]
    #[should_panic(expected = "bandwidth policy burst_bytes and queue_capacity_bytes exceed")]
    fn direct_bandwidth_policy_rejects_unbounded_queue_declaration() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        net.set_default_policy(LinkPolicy {
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 1,
                burst_bytes: 1,
                queue_capacity_bytes: MAX_BANDWIDTH_BYTES + 1,
            }),
            ..LinkPolicy::clean()
        });
    }

    #[test]
    fn downstream_fragment_loss_still_consumes_uplink_bandwidth() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 1.0,
            }),
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 10_000,
                burst_bytes: 2_000,
                queue_capacity_bytes: 2_000,
            }),
            ..LinkPolicy::clean()
        });
        sender.send_payload(
            addr(2),
            SizedPayload {
                id: 1,
                encoded_len: 2_000,
                is_input: true,
            },
        );

        assert!(receiver.recv_payloads().is_empty());
        assert_eq!(net.stats().bandwidth_admitted_datagrams, 1);
        assert_eq!(net.stats().bandwidth_admitted_bytes, 2_000);
        assert_eq!(net.stats().fragmentation_loss_events, 1);
    }

    #[test]
    fn policy_replacement_preserves_existing_bandwidth_departure_horizon() {
        let (clock, offset) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        let shaped = LinkPolicy {
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 1_000,
                burst_bytes: 100,
                queue_capacity_bytes: 300,
            }),
            ..LinkPolicy::clean()
        };
        net.set_link(addr(1), addr(2), shaped);
        for id in 1..=3 {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 100,
                    is_input: false,
                },
            );
        }
        net.set_link(addr(1), addr(2), LinkPolicy::clean());
        sender.send_payload(
            addr(2),
            SizedPayload {
                id: 4,
                encoded_len: 100,
                is_input: false,
            },
        );

        assert_eq!(receiver.recv_payloads()[0].1.id, 1);
        offset.fetch_add(100, AtomicOrdering::Relaxed);
        assert_eq!(receiver.recv_payloads()[0].1.id, 2);
        offset.fetch_add(100, AtomicOrdering::Relaxed);
        assert_eq!(
            receiver
                .recv_payloads()
                .into_iter()
                .map(|(_, payload)| payload.id)
                .collect::<Vec<_>>(),
            vec![3, 4]
        );
    }

    #[test]
    fn smaller_replacement_burst_clamps_inherited_token_credit() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                bandwidth: Some(BandwidthPolicy {
                    rate_bytes_per_second: 1,
                    burst_bytes: 1_000,
                    queue_capacity_bytes: 1_000,
                }),
                ..LinkPolicy::clean()
            },
        );
        sender.send_payload(
            addr(2),
            SizedPayload {
                id: 1,
                encoded_len: 100,
                is_input: false,
            },
        );
        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                bandwidth: Some(BandwidthPolicy {
                    rate_bytes_per_second: 1,
                    burst_bytes: 100,
                    queue_capacity_bytes: 100,
                }),
                ..LinkPolicy::clean()
            },
        );
        for id in 2..=3 {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 100,
                    is_input: false,
                },
            );
        }

        assert_eq!(
            receiver
                .recv_payloads()
                .into_iter()
                .map(|(_, payload)| payload.id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(net.stats().bandwidth_queued_datagrams, 1);
    }

    #[test]
    fn heal_preserves_existing_bandwidth_departure_horizon() {
        let (clock, offset) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                bandwidth: Some(BandwidthPolicy {
                    rate_bytes_per_second: 1_000,
                    burst_bytes: 100,
                    queue_capacity_bytes: 200,
                }),
                ..LinkPolicy::clean()
            },
        );
        for id in 1..=2 {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 100,
                    is_input: false,
                },
            );
        }
        net.heal_all();
        sender.send_payload(
            addr(2),
            SizedPayload {
                id: 3,
                encoded_len: 100,
                is_input: false,
            },
        );

        assert_eq!(receiver.recv_payloads()[0].1.id, 1);
        offset.fetch_add(100, AtomicOrdering::Relaxed);
        assert_eq!(
            receiver
                .recv_payloads()
                .into_iter()
                .map(|(_, payload)| payload.id)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    fn clean_horizon_followers_are_bounded_without_extending_horizon() {
        let (clock, offset) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                bandwidth: Some(BandwidthPolicy {
                    rate_bytes_per_second: 1,
                    burst_bytes: 1,
                    queue_capacity_bytes: 3,
                }),
                ..LinkPolicy::clean()
            },
        );
        for id in 1..=2 {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 1,
                    is_input: false,
                },
            );
        }
        net.heal_all();
        for id in 3..=7 {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 1,
                    is_input: false,
                },
            );
        }
        sender.send_payload(
            addr(2),
            SizedPayload {
                id: 8,
                encoded_len: 2,
                is_input: false,
            },
        );

        assert_eq!(receiver.recv_payloads()[0].1.id, 1);
        assert_eq!(net.stats().bandwidth_tail_drops, 3);
        assert_eq!(net.stats().bandwidth_oversize_drops, 1);
        offset.fetch_add(1_000, AtomicOrdering::Relaxed);
        assert_eq!(
            receiver
                .recv_payloads()
                .into_iter()
                .map(|(_, payload)| payload.id)
                .collect::<Vec<_>>(),
            vec![2, 3, 4]
        );
    }

    #[test]
    fn rebind_moves_bandwidth_state_without_granting_a_fresh_burst() {
        let (clock, offset) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let binding = sender.binding();
        let receiver = net.attach(addr(2));
        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                bandwidth: Some(BandwidthPolicy {
                    rate_bytes_per_second: 1_000,
                    burst_bytes: 100,
                    queue_capacity_bytes: 200,
                }),
                ..LinkPolicy::clean()
            },
        );
        for id in 1..=2 {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 100,
                    is_input: false,
                },
            );
        }
        assert_eq!(net.lock().bandwidth_reservation_count, 1);
        assert_eq!(binding.rebind(addr(3)), Ok(addr(1)));
        {
            let state = net.lock();
            assert_eq!(state.bandwidth_reservation_count, 1);
            assert!(state.links[&(addr(1), addr(2))]
                .bandwidth_reservations
                .is_empty());
            assert_eq!(
                state.links[&(addr(3), addr(2))]
                    .bandwidth_reservations
                    .len(),
                1
            );
        }
        sender.send_payload(
            addr(2),
            SizedPayload {
                id: 3,
                encoded_len: 100,
                is_input: false,
            },
        );

        assert_eq!(receiver.recv_payloads()[0].1.id, 1);
        offset.fetch_add(100, AtomicOrdering::Relaxed);
        assert_eq!(receiver.recv_payloads()[0].1.id, 2);
        offset.fetch_add(100, AtomicOrdering::Relaxed);
        assert_eq!(receiver.recv_payloads()[0].1.id, 3);
        net.set_link(addr(3), addr(2), LinkPolicy::clean());
        sender.send_payload(
            addr(2),
            SizedPayload {
                id: 4,
                encoded_len: 100,
                is_input: false,
            },
        );
        assert_eq!(net.lock().bandwidth_reservation_count, 0);
        assert_eq!(receiver.recv_payloads()[0].1.id, 4);
    }

    #[test]
    fn bandwidth_reservation_element_cap_fails_closed() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let _receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 1,
                burst_bytes: 1,
                queue_capacity_bytes: 10_000,
            }),
            ..LinkPolicy::clean()
        });
        for id in 0..=u32::try_from(MAX_BANDWIDTH_RESERVATIONS_PER_LINK + 1).unwrap() {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 1,
                    is_input: false,
                },
            );
        }

        assert_eq!(
            net.stats().bandwidth_admitted_datagrams,
            u64::try_from(MAX_BANDWIDTH_RESERVATIONS_PER_LINK + 1).unwrap()
        );
        assert_eq!(net.stats().bandwidth_reservation_cap_drops, 1);
        assert_eq!(net.stats().bandwidth_reservation_cap_dropped_bytes, 1);
    }

    #[test]
    #[should_panic(expected = "fragmentation policy requires SimNet::new_size_aware metadata")]
    fn fragmentation_without_payload_metadata_fails_loudly() {
        let (clock, _) = manual_clock();
        let net: SimNet<u32> = SimNet::new(7, clock);
        let sender = net.attach(addr(1));
        let _receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 0.2,
            }),
            ..LinkPolicy::clean()
        });

        sender.send_payload(addr(2), 1);
    }

    #[test]
    #[should_panic(
        expected = "fragmentation policy cannot be combined with reliable retransmission"
    )]
    fn default_fragmentation_with_retransmission_fails_loudly() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        net.set_default_policy(LinkPolicy {
            retransmit_delay: Duration::from_millis(10),
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 0.2,
            }),
            ..LinkPolicy::clean()
        });
    }

    #[test]
    #[should_panic(
        expected = "fragmentation policy cannot be combined with reliable retransmission"
    )]
    fn directed_fragmentation_with_retransmission_fails_loudly() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                retransmit_delay: Duration::from_millis(10),
                fragmentation: Some(FragmentationPolicy {
                    fragment_drop_rate: 0.2,
                }),
                ..LinkPolicy::clean()
            },
        );
    }

    #[test]
    fn blocked_and_held_sends_bypass_fragmentation_modeling() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                fragmentation: Some(FragmentationPolicy {
                    fragment_drop_rate: 1.0,
                }),
                bandwidth: Some(BandwidthPolicy {
                    rate_bytes_per_second: 1,
                    burst_bytes: 1,
                    queue_capacity_bytes: 1,
                }),
                ..LinkPolicy::clean()
            },
        );
        let payload = SizedPayload {
            id: 1,
            encoded_len: 2000,
            is_input: true,
        };

        net.set_blocked(addr(1), addr(2), true);
        sender.send_payload(addr(2), payload.clone());
        net.set_blocked(addr(1), addr(2), false);
        net.set_holding(addr(1), addr(2), true);
        sender.send_payload(addr(2), payload);
        net.set_holding(addr(1), addr(2), false);

        assert_eq!(receiver.recv_payloads().len(), 1);
        assert_eq!(net.stats().fragmentation_eligible_sends, 0);
        assert_eq!(net.stats().bandwidth_admitted_datagrams, 0);
        assert_eq!(net.stats().bandwidth_oversize_drops, 0);
        let link = net.link_stats()[&(addr(1), addr(2))];
        assert_eq!(link.input_sends, 2);
        assert_eq!(link.input_delivered_copies, 1);
        assert_eq!(link.max_encoded_input_bytes, 2000);
        assert_eq!(link.input_policy_loss_decisions, 0);
    }

    #[test]
    fn base_loss_precedes_fragmentation_and_non_input_does_not_reset_input_run() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let _receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            drop_rate: 1.0,
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 1.0,
            }),
            ..LinkPolicy::clean()
        });
        for (id, is_input) in [(1, true), (2, false), (3, true)] {
            sender.send_payload(
                addr(2),
                SizedPayload {
                    id,
                    encoded_len: 2000,
                    is_input,
                },
            );
        }
        assert_eq!(net.stats().fragmentation_eligible_sends, 0);
        let link = net.link_stats()[&(addr(1), addr(2))];
        assert_eq!(link.input_policy_loss_decisions, 2);
        assert_eq!(link.input_policy_drops, 2);
        assert_eq!(link.max_consecutive_input_policy_loss_run, 2);
    }

    #[test]
    fn reliable_loss_decision_is_not_counted_as_an_input_drop() {
        let (clock, offset) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            drop_rate: 1.0,
            retransmit_delay: Duration::from_millis(10),
            ..LinkPolicy::clean()
        });
        sender.send_payload(
            addr(2),
            SizedPayload {
                id: 1,
                encoded_len: 100,
                is_input: true,
            },
        );
        offset.fetch_add(10, AtomicOrdering::Relaxed);

        assert_eq!(receiver.recv_payloads().len(), 1);
        let link = net.link_stats()[&(addr(1), addr(2))];
        assert_eq!(link.input_policy_loss_decisions, 1);
        assert_eq!(link.input_policy_drops, 0);
    }

    #[test]
    fn generic_fragment_count_above_bound_fails_closed() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let receiver = net.attach(addr(2));
        net.set_default_policy(LinkPolicy {
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 0.0,
            }),
            ..LinkPolicy::clean()
        });
        sender.send_payload(
            addr(2),
            SizedPayload {
                id: 1,
                encoded_len: usize::MAX,
                is_input: false,
            },
        );
        assert!(receiver.recv_payloads().is_empty());
        let stats = net.stats();
        assert_eq!(stats.fragmentation_fragments_modeled, 0);
        assert_eq!(stats.fragmentation_fragment_cap_hits, 1);
        assert_eq!(stats.fragmentation_loss_events, 1);
        assert_eq!(stats.dropped_by_policy, 1);
    }

    #[test]
    fn size_awareness_without_fragmentation_preserves_legacy_fault_trace() {
        let (clock, offset) = manual_clock();
        let legacy: SimNet<u32> = SimNet::new(42, Arc::clone(&clock));
        let aware: SimNet<u32> = SimNet::new_size_aware(42, clock, small_u32_metadata);
        let policy = LinkPolicy {
            drop_rate: 0.2,
            dup_rate: 0.1,
            jitter: Duration::from_millis(9),
            ..LinkPolicy::clean()
        };
        legacy.set_default_policy(policy.clone());
        aware.set_default_policy(policy);
        let legacy_sender = legacy.attach(addr(1));
        let legacy_receiver = legacy.attach(addr(2));
        let aware_sender = aware.attach(addr(1));
        let aware_receiver = aware.attach(addr(2));
        let mut legacy_trace = Vec::new();
        let mut aware_trace = Vec::new();
        for value in 0..200 {
            legacy_sender.send_payload(addr(2), value);
            aware_sender.send_payload(addr(2), value);
            offset.fetch_add(1, AtomicOrdering::Relaxed);
            legacy_trace.extend(legacy_receiver.recv_payloads());
            aware_trace.extend(aware_receiver.recv_payloads());
        }
        offset.fetch_add(100, AtomicOrdering::Relaxed);
        legacy_trace.extend(legacy_receiver.recv_payloads());
        aware_trace.extend(aware_receiver.recv_payloads());
        assert_eq!(legacy_trace, aware_trace);
        assert_eq!(legacy.stats(), aware.stats());
    }

    #[test]
    fn fragmentation_drop_preserves_later_legacy_rng_outcomes() {
        let (clock, offset) = manual_clock();
        let control = SimNet::new_size_aware(42, Arc::clone(&clock), sized_payload_metadata);
        let treatment = SimNet::new_size_aware(42, clock, sized_payload_metadata);
        let dup_jitter = || LinkPolicy {
            dup_rate: 0.4,
            jitter: Duration::from_millis(9),
            ..LinkPolicy::clean()
        };
        control.set_default_policy(LinkPolicy {
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 0.0,
            }),
            ..dup_jitter()
        });
        treatment.set_default_policy(LinkPolicy {
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 1.0,
            }),
            ..dup_jitter()
        });
        let control_sender = control.attach(addr(1));
        let control_receiver = control.attach(addr(2));
        let treatment_sender = treatment.attach(addr(1));
        let treatment_receiver = treatment.attach(addr(2));

        let oversized = SizedPayload {
            id: 0,
            encoded_len: 2000,
            is_input: true,
        };
        control_sender.send_payload(addr(2), oversized.clone());
        treatment_sender.send_payload(addr(2), oversized);
        let later_base = LinkPolicy {
            drop_rate: 0.2,
            ..dup_jitter()
        };
        control.set_link(addr(1), addr(2), later_base.clone());
        treatment.set_link(addr(1), addr(2), later_base);

        for id in 1..100 {
            let payload = SizedPayload {
                id,
                encoded_len: 100,
                is_input: true,
            };
            control_sender.send_payload(addr(2), payload.clone());
            treatment_sender.send_payload(addr(2), payload);
            offset.fetch_add(1, AtomicOrdering::Relaxed);
        }
        offset.fetch_add(100, AtomicOrdering::Relaxed);
        let control_trace: Vec<_> = control_receiver
            .recv_payloads()
            .into_iter()
            .filter(|(_, payload)| payload.id != 0)
            .collect();
        let treatment_trace: Vec<_> = treatment_receiver.recv_payloads();

        assert_eq!(control_trace, treatment_trace);
        assert_eq!(treatment.stats().fragmentation_loss_events, 1);
    }

    #[test]
    fn fragmentation_rng_is_isolated_per_directed_link() {
        let (clock, _) = manual_clock();
        let isolated = SimNet::new_size_aware(91, Arc::clone(&clock), sized_payload_metadata);
        let noisy = SimNet::new_size_aware(91, clock, sized_payload_metadata);
        let policy = LinkPolicy {
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 0.25,
            }),
            ..LinkPolicy::clean()
        };
        isolated.set_default_policy(policy.clone());
        noisy.set_default_policy(policy);
        let isolated_target = isolated.attach(addr(1));
        let noisy_target = noisy.attach(addr(1));
        let noisy_other = noisy.attach(addr(3));
        let _isolated_receiver = isolated.attach(addr(2));
        let _noisy_receiver = noisy.attach(addr(2));
        let _noisy_other_receiver = noisy.attach(addr(4));

        for id in 0..200 {
            let target = SizedPayload {
                id,
                encoded_len: 2000,
                is_input: true,
            };
            noisy_other.send_payload(
                addr(4),
                SizedPayload {
                    is_input: false,
                    ..target.clone()
                },
            );
            isolated_target.send_payload(addr(2), target.clone());
            noisy_target.send_payload(addr(2), target);
        }

        assert_eq!(
            isolated.fragmentation_drop_counts()[&(addr(1), addr(2))],
            noisy.fragmentation_drop_counts()[&(addr(1), addr(2))]
        );
        assert_eq!(
            isolated.link_stats()[&(addr(1), addr(2))],
            noisy.link_stats()[&(addr(1), addr(2))]
        );
    }

    #[test]
    fn fragmentation_rng_continues_across_policy_epochs() {
        let (clock, _) = manual_clock();
        let uninterrupted = SimNet::new_size_aware(91, Arc::clone(&clock), sized_payload_metadata);
        let replaced = SimNet::new_size_aware(91, clock, sized_payload_metadata);
        let policy = LinkPolicy {
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 0.25,
            }),
            ..LinkPolicy::clean()
        };
        uninterrupted.set_default_policy(policy.clone());
        replaced.set_default_policy(policy.clone());
        let uninterrupted_sender = uninterrupted.attach(addr(1));
        let replaced_sender = replaced.attach(addr(1));
        let _uninterrupted_receiver = uninterrupted.attach(addr(2));
        let _replaced_receiver = replaced.attach(addr(2));

        for id in 0..200 {
            if id == 100 {
                replaced.set_link(addr(1), addr(2), policy.clone());
            }
            let payload = SizedPayload {
                id,
                encoded_len: 2000,
                is_input: true,
            };
            uninterrupted_sender.send_payload(addr(2), payload.clone());
            replaced_sender.send_payload(addr(2), payload);
        }

        assert_eq!(
            uninterrupted.fragmentation_drop_counts(),
            replaced.fragmentation_drop_counts()
        );
    }

    #[test]
    fn rebind_before_first_fragmentation_trial_preserves_stream_identity() {
        let (clock, _) = manual_clock();
        let control = SimNet::new_size_aware(91, Arc::clone(&clock), sized_payload_metadata);
        let rebound = SimNet::new_size_aware(91, clock, sized_payload_metadata);
        let policy = LinkPolicy {
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 0.25,
            }),
            ..LinkPolicy::clean()
        };
        control.set_default_policy(policy.clone());
        rebound.set_default_policy(policy);
        let control_sender = control.attach(addr(1));
        let control_receiver = control.attach(addr(2));
        let rebound_sender = rebound.attach(addr(1));
        let rebound_binding = rebound_sender.binding();
        let rebound_receiver = rebound.attach(addr(2));
        assert_eq!(rebound_binding.rebind(addr(3)), Ok(addr(1)));

        for id in 0..200 {
            let payload = SizedPayload {
                id,
                encoded_len: 2000,
                is_input: true,
            };
            control_sender.send_payload(addr(2), payload.clone());
            rebound_sender.send_payload(addr(2), payload);
        }

        let control_payloads: Vec<_> = control_receiver
            .recv_payloads()
            .into_iter()
            .map(|(_, payload)| payload)
            .collect();
        let rebound_payloads: Vec<_> = rebound_receiver
            .recv_payloads()
            .into_iter()
            .map(|(_, payload)| payload)
            .collect();
        assert_eq!(control_payloads, rebound_payloads);
    }

    #[test]
    fn rebind_preserves_logical_input_loss_run() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let binding = sender.binding();
        let _receiver = net.attach(addr(2));
        net.set_link(
            addr(1),
            addr(2),
            LinkPolicy {
                drop_rate: 1.0,
                ..LinkPolicy::clean()
            },
        );
        let payload = SizedPayload {
            id: 1,
            encoded_len: 100,
            is_input: true,
        };
        sender.send_payload(addr(2), payload.clone());
        assert_eq!(binding.rebind(addr(3)), Ok(addr(1)));
        sender.send_payload(addr(2), payload);

        assert_eq!(
            net.link_stats()[&(addr(3), addr(2))].max_consecutive_input_policy_loss_run,
            2
        );
    }

    #[test]
    fn set_link_resets_current_input_policy_loss_run() {
        let (clock, _) = manual_clock();
        let net = SimNet::new_size_aware(7, clock, sized_payload_metadata);
        let sender = net.attach(addr(1));
        let _receiver = net.attach(addr(2));
        let lossy = LinkPolicy {
            drop_rate: 1.0,
            ..LinkPolicy::clean()
        };
        net.set_link(addr(1), addr(2), lossy.clone());
        let payload = SizedPayload {
            id: 1,
            encoded_len: 100,
            is_input: true,
        };
        sender.send_payload(addr(2), payload.clone());
        net.set_link(addr(1), addr(2), lossy);
        sender.send_payload(addr(2), payload);
        assert_eq!(
            net.link_stats()[&(addr(1), addr(2))].max_consecutive_input_policy_loss_run,
            1
        );
    }

    /// End-to-end sanity: real P2P sessions synchronize and advance over
    /// `SimSocket`s — the actual use case for this fabric.
    #[test]
    fn sessions_synchronize_over_sim_sockets() {
        use super::super::stubs::{StubConfig, StubInput};
        use super::super::test_clock::TestClock;
        use fortress_rollback::{
            PlayerHandle, PlayerType, ProtocolConfig, SessionBuilder, SessionState,
        };

        let clock = TestClock::new();
        let net: SimNet<Message> = SimNet::new(99, clock.as_protocol_clock());
        let a1 = addr(1);
        let a2 = addr(2);
        let s1 = net.attach(a1);
        let s2 = net.attach(a2);

        let protocol_config = |seed: u64| ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            protocol_rng_seed: Some(seed),
            ..ProtocolConfig::default()
        };

        let mut sess1 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(protocol_config(1))
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(s1)
            .unwrap();

        let mut sess2 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(protocol_config(2))
            .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(s2)
            .unwrap();

        let mut synchronized = false;
        for _ in 0..500 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
            if sess1.current_state() == SessionState::Running
                && sess2.current_state() == SessionState::Running
            {
                synchronized = true;
                break;
            }
            clock.advance(Duration::from_millis(50));
        }
        assert!(
            synchronized,
            "sessions must synchronize over SimNet. sess1: {:?}, sess2: {:?}",
            sess1.current_state(),
            sess2.current_state()
        );

        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: 1 })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: 2 })
            .unwrap();
        assert!(!sess1.advance_frame().unwrap().is_empty());
        assert!(!sess2.advance_frame().unwrap().is_empty());
    }
}
