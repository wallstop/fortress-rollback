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

/// Fault policy for one directed link `(from, to)`.
///
/// All probabilities are in `0.0..=1.0` and are rolled per send, in a fixed
/// order (burst, drop, duplicate, jitter), from the net-wide seeded RNG.
/// When [`Self::retransmit_delay`] is nonzero, a burst/drop roll models a
/// reliable transport retransmission instead of packet loss: the would-be
/// dropped send is delayed, and subsequent sends on the same link are held
/// behind that retransmission deadline (TCP/WebRTC-reliable head-of-line
/// blocking).
/// Serializable so simulation schedules (which embed link policies) can be
/// stored as reproducible corpus artifacts.
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
    /// Copies dropped by drop-rate or burst rolls.
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
}

impl LinkState {
    fn with_policy(policy: LinkPolicy) -> Self {
        Self {
            policy,
            burst_remaining: 0,
            retransmit_blocked_until: None,
            blocked: false,
            holding: false,
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

    fn send(&mut self, from: SocketAddr, to: SocketAddr, payload: M) {
        self.stats.sent += 1;
        let key = (from, to);
        if !self.links.contains_key(&key) {
            let state = LinkState::with_policy(self.default_policy.clone());
            self.links.insert(key, state);
        }

        // Split borrows: read the link decision first, then roll RNG.
        let (blocked, holding) = match self.links.get(&key) {
            Some(link) => (link.blocked, link.holding),
            None => (false, false),
        };

        if blocked {
            self.stats.dropped_blocked += 1;
            return;
        }
        if holding {
            self.stats.held += 1;
            self.held.entry(key).or_default().push_back(payload);
            return;
        }

        let policy = self
            .links
            .get(&key)
            .map_or_else(LinkPolicy::clean, |link| link.policy.clone());

        // Burst state machine (mirrors the ChaosSocket semantics: the
        // triggering send and the following `burst_len - 1` sends drop).
        let in_burst = self
            .links
            .get(&key)
            .is_some_and(|link| link.burst_remaining > 0);
        if in_burst {
            if let Some(link) = self.links.get_mut(&key) {
                link.burst_remaining -= 1;
            }
            if !self.handle_policy_loss(key, policy.retransmit_delay) {
                return;
            }
        }

        let burst_roll_hit =
            !in_burst && policy.burst_rate > 0.0 && self.roll_unit() < policy.burst_rate;
        if burst_roll_hit {
            if let Some(link) = self.links.get_mut(&key) {
                link.burst_remaining = policy.burst_len.saturating_sub(1);
            }
            if !self.handle_policy_loss(key, policy.retransmit_delay) {
                return;
            }
        } else {
            let drop_roll_hit =
                !in_burst && policy.drop_rate > 0.0 && self.roll_unit() < policy.drop_rate;
            if drop_roll_hit && !self.handle_policy_loss(key, policy.retransmit_delay) {
                return;
            }
        }

        let copies = if policy.dup_rate > 0.0 && self.roll_unit() < policy.dup_rate {
            self.stats.duplicated += 1;
            2
        } else {
            1
        };

        let now = self.now();
        let blocked_until = self
            .links
            .get(&key)
            .and_then(|link| link.retransmit_blocked_until);
        for _ in 0..copies {
            let delay = policy.base_delay + self.roll_jitter(policy.jitter);
            let mut deliver_at = now + delay;
            if let Some(deadline) = blocked_until {
                deliver_at = deliver_at.max(deadline);
            }
            self.schedule(from, to, payload.clone(), deliver_at);
        }
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
            match self.inboxes.get_mut(&msg.to) {
                Some(queue) => {
                    queue.push_back((msg.from, msg.payload));
                    self.stats.delivered += 1;
                },
                None => match self.unattached {
                    UnattachedPolicy::Drop => self.stats.dropped_unattached += 1,
                    UnattachedPolicy::Buffer => {
                        self.inboxes
                            .entry(msg.to)
                            .or_default()
                            .push_back((msg.from, msg.payload));
                        self.stats.delivered += 1;
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
        Self {
            state: Arc::new(Mutex::new(SimNetState {
                clock,
                rng: Pcg32::seed_from_u64(seed),
                seq: 0,
                default_policy: LinkPolicy::clean(),
                links: BTreeMap::new(),
                held: BTreeMap::new(),
                in_flight: BinaryHeap::new(),
                inboxes: BTreeMap::new(),
                unattached: UnattachedPolicy::Drop,
                stats: SimNetStats::default(),
            })),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, SimNetState<M>> {
        self.state.lock().expect("SimNet mutex poisoned")
    }

    /// Attaches a socket at `addr`, creating an inbox for it.
    ///
    /// An existing inbox (from a previous attachment, or buffered
    /// unattached-policy traffic) is preserved, so a hot-join re-attach at a
    /// vacated address receives anything buffered for it.
    #[must_use]
    pub fn attach(&self, addr: SocketAddr) -> SimSocket<M> {
        self.lock().inboxes.entry(addr).or_default();
        SimSocket {
            addr,
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
        self.lock().default_policy = policy;
    }

    /// Sets (or replaces) the fault policy for the directed link `from → to`,
    /// preserving its blocked/holding toggles and any current retransmission
    /// head-of-line deadline, and resetting any burst in progress.
    pub fn set_link(&self, from: SocketAddr, to: SocketAddr, policy: LinkPolicy) {
        let mut state = self.lock();
        let link = state.links.entry((from, to)).or_default();
        link.policy = policy;
        link.burst_remaining = 0;
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
        let link = state.links.entry((from, to)).or_default();
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
        let link = state.links.entry(key).or_default();
        link.holding = holding;
        if !holding {
            state.release_held(key);
        }
    }

    /// Resets every link to clean/unblocked/released and the default policy
    /// to clean. Buffered held messages are delivered (FIFO), in-flight
    /// messages keep their scheduled delivery times.
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
}

/// A peer-side socket attached to a [`SimNet`] at a fixed address.
pub struct SimSocket<M = Message> {
    addr: SocketAddr,
    state: Arc<Mutex<SimNetState<M>>>,
}

// Test infrastructure — poisoned mutex is a test bug (see SimNet::lock).
#[allow(clippy::expect_used)]
impl<M: Clone> SimSocket<M> {
    /// The address this socket is attached at.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Sends a payload to `to` through the simulated network.
    pub fn send_payload(&self, to: SocketAddr, payload: M) {
        self.state
            .lock()
            .expect("SimNet mutex poisoned")
            .send(self.addr, to, payload);
    }

    /// Receives every payload due for this socket (bounded per call by
    /// [`MAX_RECEIVE_MESSAGES_PER_POLL`]; the remainder stays queued).
    #[must_use]
    pub fn recv_payloads(&self) -> Vec<(SocketAddr, M)> {
        self.state
            .lock()
            .expect("SimNet mutex poisoned")
            .receive(self.addr)
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
