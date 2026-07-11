//! Always-on, pull-based session metrics.
//!
//! [`SessionMetrics`] is a cheap, `Copy` snapshot of cumulative session
//! counters returned by [`P2PSession::metrics`] and
//! [`SpectatorSession::metrics`]. Counters are plain integers updated inline on
//! the paths they measure — no timers, no allocation, no `Instant` — so reading
//! them is deterministic and WASM-safe.
//!
//! [`PeerMetrics`] is the per-peer analogue, returned by
//! [`P2PSession::peer_metrics`]: wire-exact byte and packet counters, a
//! per-[`MessageKind`] breakdown of traffic in each direction, input-compression
//! totals, and a few instantaneous connection gauges for one remote endpoint.
//!
//! The first surface exposed here is **event-queue overflow accounting**: when
//! the bounded event queue discards an undrained [`FortressEvent`] the session
//! records it in [`SessionMetrics::events_discarded_total`] and the per-category
//! [`SessionMetrics::events_discarded_by_kind`] breakdown, so a lost
//! notification — possibly a safety-critical [`FortressEvent::Disconnected`] or
//! [`FortressEvent::DesyncDetected`] — is observable instead of silent.
//!
//! [`P2PSession::metrics`]: crate::P2PSession::metrics
//! [`P2PSession::peer_metrics`]: crate::P2PSession::peer_metrics
//! [`SpectatorSession::metrics`]: crate::SpectatorSession::metrics
//! [`FortressEvent`]: crate::FortressEvent
//! [`FortressEvent::Disconnected`]: crate::FortressEvent::Disconnected
//! [`FortressEvent::DesyncDetected`]: crate::FortressEvent::DesyncDetected

use serde::Serialize;

/// The category of a [`FortressEvent`], independent of its payload.
///
/// Mirrors the variants of [`FortressEvent`] one-to-one so events can be
/// counted, filtered, or labeled without matching on their payload fields.
/// Obtain one with [`FortressEvent::kind`].
///
/// The hot-join categories (`JoinRequested`, `PeerJoined`) exist only when the
/// `hot-join` feature is enabled, mirroring the corresponding [`FortressEvent`]
/// variants.
///
/// [`FortressEvent`]: crate::FortressEvent
/// [`FortressEvent::kind`]: crate::FortressEvent::kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// [`FortressEvent::Synchronizing`](crate::FortressEvent::Synchronizing).
    Synchronizing,
    /// [`FortressEvent::Synchronized`](crate::FortressEvent::Synchronized).
    Synchronized,
    /// [`FortressEvent::Disconnected`](crate::FortressEvent::Disconnected).
    Disconnected,
    /// [`FortressEvent::NetworkInterrupted`](crate::FortressEvent::NetworkInterrupted).
    NetworkInterrupted,
    /// [`FortressEvent::NetworkResumed`](crate::FortressEvent::NetworkResumed).
    NetworkResumed,
    /// [`FortressEvent::WaitRecommendation`](crate::FortressEvent::WaitRecommendation).
    WaitRecommendation,
    /// [`FortressEvent::DesyncDetected`](crate::FortressEvent::DesyncDetected).
    DesyncDetected,
    /// [`FortressEvent::SyncTimeout`](crate::FortressEvent::SyncTimeout).
    SyncTimeout,
    /// [`FortressEvent::ReplayDesync`](crate::FortressEvent::ReplayDesync).
    ReplayDesync,
    /// [`FortressEvent::SpectatorDivergence`](crate::FortressEvent::SpectatorDivergence).
    SpectatorDivergence,
    /// [`FortressEvent::InputDelayRecommendation`](crate::FortressEvent::InputDelayRecommendation).
    InputDelayRecommendation,
    /// [`FortressEvent::PeerDropped`](crate::FortressEvent::PeerDropped).
    PeerDropped,
    /// [`FortressEvent::JoinRequested`](crate::FortressEvent::JoinRequested).
    #[cfg(feature = "hot-join")]
    JoinRequested,
    /// [`FortressEvent::PeerJoined`](crate::FortressEvent::PeerJoined).
    #[cfg(feature = "hot-join")]
    PeerJoined,
}

impl EventKind {
    /// The number of event categories.
    ///
    /// Varies with enabled features: two additional categories exist when the
    /// `hot-join` feature is on.
    #[cfg(not(feature = "hot-join"))]
    pub const COUNT: usize = 12;
    /// The number of event categories.
    ///
    /// Varies with enabled features: two additional categories exist when the
    /// `hot-join` feature is on.
    #[cfg(feature = "hot-join")]
    pub const COUNT: usize = 14;

    /// Every category, in declaration order. Its length is [`Self::COUNT`].
    #[cfg(not(feature = "hot-join"))]
    pub const ALL: [Self; Self::COUNT] = [
        Self::Synchronizing,
        Self::Synchronized,
        Self::Disconnected,
        Self::NetworkInterrupted,
        Self::NetworkResumed,
        Self::WaitRecommendation,
        Self::DesyncDetected,
        Self::SyncTimeout,
        Self::ReplayDesync,
        Self::SpectatorDivergence,
        Self::InputDelayRecommendation,
        Self::PeerDropped,
    ];
    /// Every category, in declaration order. Its length is [`Self::COUNT`].
    #[cfg(feature = "hot-join")]
    pub const ALL: [Self; Self::COUNT] = [
        Self::Synchronizing,
        Self::Synchronized,
        Self::Disconnected,
        Self::NetworkInterrupted,
        Self::NetworkResumed,
        Self::WaitRecommendation,
        Self::DesyncDetected,
        Self::SyncTimeout,
        Self::ReplayDesync,
        Self::SpectatorDivergence,
        Self::InputDelayRecommendation,
        Self::PeerDropped,
        Self::JoinRequested,
        Self::PeerJoined,
    ];

    /// A stable snake_case label for this category, suitable for logging or as
    /// a metrics key. Matches the JSON key produced by serialization.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Synchronizing => "synchronizing",
            Self::Synchronized => "synchronized",
            Self::Disconnected => "disconnected",
            Self::NetworkInterrupted => "network_interrupted",
            Self::NetworkResumed => "network_resumed",
            Self::WaitRecommendation => "wait_recommendation",
            Self::DesyncDetected => "desync_detected",
            Self::SyncTimeout => "sync_timeout",
            Self::ReplayDesync => "replay_desync",
            Self::SpectatorDivergence => "spectator_divergence",
            Self::InputDelayRecommendation => "input_delay_recommendation",
            Self::PeerDropped => "peer_dropped",
            #[cfg(feature = "hot-join")]
            Self::JoinRequested => "join_requested",
            #[cfg(feature = "hot-join")]
            Self::PeerJoined => "peer_joined",
        }
    }

    /// The array index this category occupies in [`EventKindCounts`]. Always
    /// less than [`Self::COUNT`].
    const fn index(self) -> usize {
        match self {
            Self::Synchronizing => 0,
            Self::Synchronized => 1,
            Self::Disconnected => 2,
            Self::NetworkInterrupted => 3,
            Self::NetworkResumed => 4,
            Self::WaitRecommendation => 5,
            Self::DesyncDetected => 6,
            Self::SyncTimeout => 7,
            Self::ReplayDesync => 8,
            Self::SpectatorDivergence => 9,
            Self::InputDelayRecommendation => 10,
            Self::PeerDropped => 11,
            #[cfg(feature = "hot-join")]
            Self::JoinRequested => 12,
            #[cfg(feature = "hot-join")]
            Self::PeerJoined => 13,
        }
    }
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Per-[`EventKind`] counters, keyed by category.
///
/// Backed by a fixed-size array (one slot per [`EventKind`]); read individual
/// counts with [`get`](Self::get). Serializes as a JSON object keyed by each
/// category's [`EventKind::as_str`] label, so the wire form is self-describing
/// and stable across counter values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventKindCounts([u64; EventKind::COUNT]);

impl Default for EventKindCounts {
    fn default() -> Self {
        Self([0; EventKind::COUNT])
    }
}

impl EventKindCounts {
    /// The count recorded for `kind`.
    ///
    /// `kind.index()` is always in bounds, so the `unwrap_or` fallback is
    /// unreachable; it keeps the accessor panic-free without an index.
    #[must_use]
    pub fn get(&self, kind: EventKind) -> u64 {
        self.0.get(kind.index()).copied().unwrap_or(0)
    }

    /// Increments the counter for `kind` by one, saturating at [`u64::MAX`].
    fn record(&mut self, kind: EventKind) {
        if let Some(slot) = self.0.get_mut(kind.index()) {
            *slot = slot.saturating_add(1);
        }
    }
}

impl Serialize for EventKindCounts {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(EventKind::COUNT))?;
        for kind in EventKind::ALL {
            map.serialize_entry(kind.as_str(), &self.get(kind))?;
        }
        map.end()
    }
}

/// A histogram of rollback depths (the number of frames re-simulated per
/// rollback), bucketed by depth.
///
/// Backed by a fixed-size array of [`BUCKETS`](Self::BUCKETS) slots: slot `i`
/// (`0..=15`) counts rollbacks of depth `i + 1`, and the final slot counts every
/// rollback deeper than 16. Read individual buckets with [`bucket`](Self::bucket)
/// or the grand total with [`total`](Self::total). Serializes as a JSON object
/// keyed by each bucket's depth label (`"1"..="16"`, then `"17_plus"`), so the
/// wire form is self-describing and stable across counter values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollbackDepthHistogram([u64; Self::BUCKETS]);

impl RollbackDepthHistogram {
    /// The number of depth buckets: one per depth `1..=16` plus a final
    /// catch-all for every rollback deeper than 16.
    pub const BUCKETS: usize = 17;

    /// The self-describing label for each bucket, in slot order. The final label
    /// (`"17_plus"`) covers every rollback deeper than 16.
    const LABELS: [&'static str; Self::BUCKETS] = [
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
        "17_plus",
    ];

    /// The slot a rollback of `depth` frames falls into. Depths at or below 1 map
    /// to the first bucket; depths above 16 saturate into the final bucket.
    fn bucket_index(depth: usize) -> usize {
        // Clamp into `1..=BUCKETS`, then shift to a zero-based slot: depth 1 → 0,
        // depth 16 → 15, depth ≥ 17 → 16 (the "17_plus" catch-all).
        depth.clamp(1, Self::BUCKETS).saturating_sub(1)
    }

    /// Records one rollback of `depth` frames, saturating the bucket at
    /// [`u64::MAX`]. A `depth` of 0 (not a real rollback) is ignored.
    fn record(&mut self, depth: usize) {
        if depth == 0 {
            return;
        }
        if let Some(slot) = self.0.get_mut(Self::bucket_index(depth)) {
            *slot = slot.saturating_add(1);
        }
    }

    /// The count recorded in bucket `index` (see the type docs for the bucket
    /// layout). Returns 0 for an out-of-range index.
    #[must_use]
    pub fn bucket(&self, index: usize) -> u64 {
        self.0.get(index).copied().unwrap_or(0)
    }

    /// The total number of rollbacks recorded across every bucket.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.0.iter().copied().fold(0u64, u64::saturating_add)
    }
}

impl Default for RollbackDepthHistogram {
    fn default() -> Self {
        Self([0; Self::BUCKETS])
    }
}

impl Serialize for RollbackDepthHistogram {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(Self::BUCKETS))?;
        for (index, label) in Self::LABELS.into_iter().enumerate() {
            map.serialize_entry(label, &self.bucket(index))?;
        }
        map.end()
    }
}

/// A cumulative snapshot of session-level metrics.
///
/// In normal use you read a snapshot from [`P2PSession::metrics`],
/// [`SpectatorSession::metrics`], or [`ReplaySession::metrics`]. Cumulative
/// counters and high-water marks are monotonic for the life of the session;
/// current-value gauges may move in either direction. Every snapshot is cheap to
/// read (the type is `Copy`).
///
/// This type is `#[non_exhaustive]`: future library versions may add counters
/// without a breaking change, so it cannot be built with a struct literal
/// outside this crate — match with `..`.
///
/// # Which counters each session type populates
///
/// Rollback, pacing, checksum, and checksum-history counters are populated by
/// [`P2PSession`]. [`SpectatorSession`] and [`ReplaySession`] populate
/// `event_queue_high_water` plus the `events_discarded_*` counters; their other
/// fields stay at the default `0`. Spectators expose `frames_behind_host`
/// separately.
///
/// [`P2PSession`]: crate::P2PSession
/// [`ReplaySession`]: crate::ReplaySession
/// [`SpectatorSession`]: crate::SpectatorSession
///
/// # Example
///
/// ```
/// # use fortress_rollback::metrics::SessionMetrics;
/// let metrics = SessionMetrics::default();
/// assert_eq!(metrics.events_discarded_total, 0);
/// assert_eq!(metrics.frames_advanced, 0);
/// ```
///
/// [`P2PSession::metrics`]: crate::P2PSession::metrics
/// [`ReplaySession::metrics`]: crate::ReplaySession::metrics
/// [`SpectatorSession::metrics`]: crate::SpectatorSession::metrics
#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[must_use = "SessionMetrics should be inspected after being queried"]
pub struct SessionMetrics {
    /// Total frames the game state was stepped forward, counting **both** the
    /// forward advances that produce a rendered frame
    /// ([`visual_frames`](Self::visual_frames)) and the frames replayed during
    /// rollbacks ([`resimulated_frames`](Self::resimulated_frames)). By
    /// construction `frames_advanced == visual_frames + resimulated_frames`, so
    /// the ratio `resimulated_frames / frames_advanced` is the fraction of
    /// simulation work spent on rollback repair.
    ///
    /// The one-time `AdvanceFrame` a hot-join joiner runs to bridge a loaded
    /// snapshot up to its activation frame is a state-restoration step, not
    /// steady-state simulation, and is deliberately not counted here.
    pub frames_advanced: u64,

    /// The number of forward frame advances — one per
    /// [`P2PSession::advance_frame`](crate::P2PSession::advance_frame) call that
    /// steps the simulation to a new (never-before-simulated) frame. This is the
    /// count of frames the application renders.
    pub visual_frames: u64,

    /// The number of frames re-simulated during rollbacks. Each rollback of
    /// depth `d` re-simulates `d` frames, so this is the running sum of rollback
    /// depths (see [`rollback_depth_histogram`](Self::rollback_depth_histogram)).
    pub resimulated_frames: u64,

    /// The number of rollbacks performed (a load-and-resimulate episode),
    /// regardless of depth.
    pub rollback_count: u64,

    /// A histogram of rollback depths: how many rollbacks re-simulated 1 frame,
    /// 2 frames, …, 16 frames, and more than 16 frames. Its
    /// [`total`](RollbackDepthHistogram::total) equals
    /// [`rollback_count`](Self::rollback_count).
    pub rollback_depth_histogram: RollbackDepthHistogram,

    /// The deepest single rollback observed (its number of re-simulated frames).
    /// Stays 0 until the first rollback.
    pub max_rollback_depth: u32,

    /// The number of individual per-player prediction misses that triggered
    /// rollback repair. One misprediction-driven rollback can count several
    /// misses (one per player whose predicted input turned out wrong).
    pub prediction_miss_count: u64,

    /// The number of times an
    /// [`advance_frame`](crate::P2PSession::advance_frame) call could not step
    /// the simulation because the prediction window was already full (the session
    /// is waiting for a slow or unreachable peer to confirm inputs). A rising
    /// count means the local simulation is being throttled by the network.
    pub stall_count: u64,

    /// The number of [`FortressEvent::WaitRecommendation`] events emitted (the
    /// session asked the application to slow down to let a peer catch up).
    ///
    /// [`FortressEvent::WaitRecommendation`]: crate::FortressEvent::WaitRecommendation
    pub wait_recommendations: u64,

    /// The most recently sampled confirmation lag: how many frames ahead of the
    /// last confirmed frame the simulation was at the last forward advance. A
    /// gauge, not a monotonic counter.
    pub confirmation_lag_current: u64,

    /// The maximum [`confirmation_lag_current`](Self::confirmation_lag_current)
    /// observed over the life of the session.
    pub confirmation_lag_max: u64,

    /// The running sum of the per-advance confirmation lag samples. Divide by
    /// [`visual_frames`](Self::visual_frames) for the mean confirmation lag.
    pub confirmation_lag_sum: u64,

    /// The number of confirmed-frame checksums compared against a peer's
    /// checksum for desync detection. Zero unless
    /// [`DesyncDetection`](crate::DesyncDetection) is enabled. Equals
    /// [`checksums_matched`](Self::checksums_matched) `+`
    /// [`checksums_mismatched`](Self::checksums_mismatched).
    pub checksums_compared: u64,

    /// How many compared checksums matched their peer's value.
    pub checksums_matched: u64,

    /// How many compared checksums disagreed with their peer's value (each also
    /// emits a [`FortressEvent::DesyncDetected`]).
    ///
    /// [`FortressEvent::DesyncDetected`]: crate::FortressEvent::DesyncDetected
    pub checksums_mismatched: u64,

    /// The high-water mark of the event-queue length: the largest the bounded
    /// event queue grew before the application drained it. Compare against the
    /// configured event-queue size to see how close overflow (and the resulting
    /// [`events_discarded_total`](Self::events_discarded_total)) came.
    pub event_queue_high_water: u64,

    /// The high-water mark of the local desync-checksum history length.
    pub checksum_history_high_water: u64,

    /// Total number of queued [`FortressEvent`]s discarded because the bounded
    /// event queue overflowed before the application drained it.
    ///
    /// A non-zero value means events are being produced faster than the
    /// application drains them and notifications have been lost — possibly
    /// safety-critical ones such as [`FortressEvent::Disconnected`] or
    /// [`FortressEvent::DesyncDetected`]. Drain events every poll, or raise the
    /// event-queue size, to keep this at zero. See
    /// [`events_discarded_by_kind`](Self::events_discarded_by_kind) for the
    /// per-category breakdown.
    ///
    /// [`FortressEvent`]: crate::FortressEvent
    /// [`FortressEvent::Disconnected`]: crate::FortressEvent::Disconnected
    /// [`FortressEvent::DesyncDetected`]: crate::FortressEvent::DesyncDetected
    pub events_discarded_total: u64,

    /// Per-[`EventKind`] breakdown of
    /// [`events_discarded_total`](Self::events_discarded_total): how many
    /// discarded events fell into each category.
    pub events_discarded_by_kind: EventKindCounts,

    /// Number of decoded protocol messages received from an address that is
    /// not a configured endpoint for this networked session.
    ///
    /// A rising value can indicate stale traffic, source-address spoofing, or
    /// a peer whose NAT mapping changed during the session. Such packets are
    /// ignored because an address is the peer identity at this protocol layer.
    /// Malformed datagrams rejected by a socket before it yields a decoded
    /// [`Message`](crate::Message) are outside this counter's boundary. Replay
    /// and sync-test sessions have no receive socket, so this remains zero for
    /// them.
    pub unknown_source_packets: u64,
}

impl SessionMetrics {
    /// Creates a zeroed snapshot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one event-queue-overflow discard of an event of category `kind`.
    pub(crate) fn record_event_discard(&mut self, kind: EventKind) {
        self.events_discarded_total = self.events_discarded_total.saturating_add(1);
        self.events_discarded_by_kind.record(kind);
    }

    /// Records one decoded packet whose source address is not registered with
    /// the session.
    pub(crate) fn record_unknown_source_packet(&mut self) {
        self.unknown_source_packets = self.unknown_source_packets.saturating_add(1);
    }

    /// Records one forward frame advance (a rendered/visual frame) and samples
    /// the confirmation lag at that advance.
    pub(crate) fn record_forward_advance(&mut self, confirmation_lag: u64) {
        self.frames_advanced = self.frames_advanced.saturating_add(1);
        self.visual_frames = self.visual_frames.saturating_add(1);
        self.confirmation_lag_current = confirmation_lag;
        self.confirmation_lag_max = self.confirmation_lag_max.max(confirmation_lag);
        self.confirmation_lag_sum = self.confirmation_lag_sum.saturating_add(confirmation_lag);
    }

    /// Records one rollback that re-simulated `depth` frames.
    pub(crate) fn record_rollback(&mut self, depth: usize) {
        debug_assert!(
            depth > 0,
            "record_rollback called with depth 0 — not a real rollback"
        );
        if depth == 0 {
            return;
        }
        let depth_u64 = u64::try_from(depth).unwrap_or(u64::MAX);
        self.rollback_count = self.rollback_count.saturating_add(1);
        self.resimulated_frames = self.resimulated_frames.saturating_add(depth_u64);
        self.frames_advanced = self.frames_advanced.saturating_add(depth_u64);
        let depth_u32 = u32::try_from(depth).unwrap_or(u32::MAX);
        self.max_rollback_depth = self.max_rollback_depth.max(depth_u32);
        self.rollback_depth_histogram.record(depth);
    }

    /// Records `count` per-player prediction misses that triggered a rollback.
    pub(crate) fn record_prediction_misses(&mut self, count: u64) {
        self.prediction_miss_count = self.prediction_miss_count.saturating_add(count);
    }

    /// Records one prediction-window stall (an advance that could not step the
    /// simulation because the prediction window was full).
    pub(crate) fn record_stall(&mut self) {
        self.stall_count = self.stall_count.saturating_add(1);
    }

    /// Records one emitted [`FortressEvent::WaitRecommendation`].
    ///
    /// [`FortressEvent::WaitRecommendation`]: crate::FortressEvent::WaitRecommendation
    pub(crate) fn record_wait_recommendation(&mut self) {
        self.wait_recommendations = self.wait_recommendations.saturating_add(1);
    }

    /// Records one confirmed-frame checksum comparison against a peer.
    pub(crate) fn record_checksum_comparison(&mut self, matched: bool) {
        self.checksums_compared = self.checksums_compared.saturating_add(1);
        if matched {
            self.checksums_matched = self.checksums_matched.saturating_add(1);
        } else {
            self.checksums_mismatched = self.checksums_mismatched.saturating_add(1);
        }
    }

    /// Updates the event-queue high-water mark with an observed queue length.
    pub(crate) fn observe_event_queue_len(&mut self, len: usize) {
        let len = u64::try_from(len).unwrap_or(u64::MAX);
        self.event_queue_high_water = self.event_queue_high_water.max(len);
    }

    /// Updates the checksum-history high-water mark with an observed length.
    pub(crate) fn observe_checksum_history_len(&mut self, len: usize) {
        let len = u64::try_from(len).unwrap_or(u64::MAX);
        self.checksum_history_high_water = self.checksum_history_high_water.max(len);
    }

    /// Serializes this snapshot to a compact JSON string.
    ///
    /// Returns `None` if serialization fails. This type is a small set of
    /// integer counters, so failure is not expected in normal operation, but it
    /// is not impossible (for example, an allocation failure inside
    /// `serde_json`).
    #[cfg(feature = "json")]
    #[must_use]
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }

    /// Serializes this snapshot to a pretty-printed JSON string.
    ///
    /// Like [`to_json`](Self::to_json), but indented for readability.
    #[cfg(feature = "json")]
    #[must_use]
    pub fn to_json_pretty(&self) -> Option<String> {
        serde_json::to_string_pretty(self).ok()
    }
}

/// Hot-join handshake timing for a session that joined an in-progress game via
/// [`SessionBuilder::start_hot_join_session`](crate::SessionBuilder::start_hot_join_session).
///
/// Read one with
/// [`P2PSession::hot_join_metrics`](crate::P2PSession::hot_join_metrics), which
/// returns `None` for any session that did not hot-join (a host, or a normally
/// synchronized peer). Timings are measured on the session's injectable protocol
/// clock, so they are deterministic under the simulation harness (no wall clock).
///
/// Only available with the `hot-join` feature.
#[cfg(feature = "hot-join")]
#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[must_use = "HotJoinMetrics should be inspected after being queried"]
pub struct HotJoinMetrics {
    /// Whether the join has completed — the joiner applied the host snapshot and
    /// reached [`SessionState::Running`](crate::SessionState::Running). While
    /// `false`, [`polls_to_running`](Self::polls_to_running) is still climbing
    /// and [`millis_to_running`](Self::millis_to_running) stays `0`.
    pub completed: bool,

    /// [`poll_remote_clients`](crate::P2PSession::poll_remote_clients) iterations
    /// spent in [`HotJoining`](crate::SessionState::HotJoining) — how many polls
    /// the join took (or has taken so far, when not yet
    /// [`completed`](Self::completed)).
    pub polls_to_running: u64,

    /// Virtual milliseconds on the injected protocol clock from join start
    /// (session construction) to reaching `Running`. `0` while not yet
    /// [`completed`](Self::completed); once completed it is the measured latency,
    /// which may itself be `0` if the clock did not advance between construction
    /// and activation — read [`completed`](Self::completed), not this field, to
    /// tell whether the join has finished.
    pub millis_to_running: u64,
}

#[cfg(all(feature = "hot-join", feature = "json"))]
impl HotJoinMetrics {
    /// Serializes this snapshot to a compact JSON string.
    ///
    /// Returns `None` if serialization fails — not expected for a few integers,
    /// but possible (for example, an allocation failure inside `serde_json`).
    #[must_use]
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }

    /// Serializes this snapshot to a pretty-printed JSON string.
    ///
    /// Like [`to_json`](Self::to_json), but indented for readability.
    #[must_use]
    pub fn to_json_pretty(&self) -> Option<String> {
        serde_json::to_string_pretty(self).ok()
    }
}

/// The category of a protocol message, independent of its payload.
///
/// Mirrors the crate's internal wire-message variants one-to-one so per-peer
/// traffic can be counted and labeled by kind — see
/// [`PeerMetrics::messages_sent_by_kind`] and
/// [`PeerMetrics::messages_received_by_kind`] — without exposing the internal
/// message types.
///
/// The hot-join categories (`JoinRequest`, `StateSnapshot`, `StateSnapshotAck`,
/// `ReactivateSlot`, `ReactivateSlotAck`, `JoinCommitted`, `JoinAborted`) exist
/// only when the `hot-join` feature is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    /// A synchronization request (part of the connection handshake).
    SyncRequest,
    /// A synchronization reply (part of the connection handshake).
    SyncReply,
    /// A player-input packet — the steady-state gameplay message.
    Input,
    /// An acknowledgement of received input.
    InputAck,
    /// A quality report (round-trip-time / frame-advantage probe).
    QualityReport,
    /// A quality reply — the pong answering a [`QualityReport`](Self::QualityReport).
    QualityReply,
    /// A confirmed-frame checksum report (desync detection).
    ChecksumReport,
    /// A keep-alive, sent when no other traffic is otherwise due.
    KeepAlive,
    /// A floor-round request (the double-failure-relay reorder fix).
    FloorRequest,
    /// A floor-round reply (the double-failure-relay reorder fix).
    FloorReply,
    /// A hot-join slot-occupancy request.
    #[cfg(feature = "hot-join")]
    JoinRequest,
    /// A hot-join game-state snapshot.
    #[cfg(feature = "hot-join")]
    StateSnapshot,
    /// A hot-join snapshot acknowledgement.
    #[cfg(feature = "hot-join")]
    StateSnapshotAck,
    /// A hot-join slot-reactivation request.
    #[cfg(feature = "hot-join")]
    ReactivateSlot,
    /// A hot-join slot-reactivation acknowledgement.
    #[cfg(feature = "hot-join")]
    ReactivateSlotAck,
    /// A hot-join commit notification.
    #[cfg(feature = "hot-join")]
    JoinCommitted,
    /// A hot-join abort notification.
    #[cfg(feature = "hot-join")]
    JoinAborted,
}

impl MessageKind {
    /// The number of message categories.
    ///
    /// Varies with enabled features: seven additional categories exist when the
    /// `hot-join` feature is on.
    #[cfg(not(feature = "hot-join"))]
    pub const COUNT: usize = 10;
    /// The number of message categories.
    ///
    /// Varies with enabled features: seven additional categories exist when the
    /// `hot-join` feature is on.
    #[cfg(feature = "hot-join")]
    pub const COUNT: usize = 17;

    /// Every category, in declaration (wire-discriminant) order. Its length is
    /// [`Self::COUNT`].
    #[cfg(not(feature = "hot-join"))]
    pub const ALL: [Self; Self::COUNT] = [
        Self::SyncRequest,
        Self::SyncReply,
        Self::Input,
        Self::InputAck,
        Self::QualityReport,
        Self::QualityReply,
        Self::ChecksumReport,
        Self::KeepAlive,
        Self::FloorRequest,
        Self::FloorReply,
    ];
    /// Every category, in declaration (wire-discriminant) order. Its length is
    /// [`Self::COUNT`].
    #[cfg(feature = "hot-join")]
    pub const ALL: [Self; Self::COUNT] = [
        Self::SyncRequest,
        Self::SyncReply,
        Self::Input,
        Self::InputAck,
        Self::QualityReport,
        Self::QualityReply,
        Self::ChecksumReport,
        Self::KeepAlive,
        Self::FloorRequest,
        Self::FloorReply,
        Self::JoinRequest,
        Self::StateSnapshot,
        Self::StateSnapshotAck,
        Self::ReactivateSlot,
        Self::ReactivateSlotAck,
        Self::JoinCommitted,
        Self::JoinAborted,
    ];

    /// A stable snake_case label for this category, suitable for logging or as a
    /// metrics key. Matches the JSON key produced by serialization.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SyncRequest => "sync_request",
            Self::SyncReply => "sync_reply",
            Self::Input => "input",
            Self::InputAck => "input_ack",
            Self::QualityReport => "quality_report",
            Self::QualityReply => "quality_reply",
            Self::ChecksumReport => "checksum_report",
            Self::KeepAlive => "keep_alive",
            Self::FloorRequest => "floor_request",
            Self::FloorReply => "floor_reply",
            #[cfg(feature = "hot-join")]
            Self::JoinRequest => "join_request",
            #[cfg(feature = "hot-join")]
            Self::StateSnapshot => "state_snapshot",
            #[cfg(feature = "hot-join")]
            Self::StateSnapshotAck => "state_snapshot_ack",
            #[cfg(feature = "hot-join")]
            Self::ReactivateSlot => "reactivate_slot",
            #[cfg(feature = "hot-join")]
            Self::ReactivateSlotAck => "reactivate_slot_ack",
            #[cfg(feature = "hot-join")]
            Self::JoinCommitted => "join_committed",
            #[cfg(feature = "hot-join")]
            Self::JoinAborted => "join_aborted",
        }
    }

    /// The array index this category occupies in [`MessageKindCounts`]. Always
    /// less than [`Self::COUNT`].
    const fn index(self) -> usize {
        match self {
            Self::SyncRequest => 0,
            Self::SyncReply => 1,
            Self::Input => 2,
            Self::InputAck => 3,
            Self::QualityReport => 4,
            Self::QualityReply => 5,
            Self::ChecksumReport => 6,
            Self::KeepAlive => 7,
            Self::FloorRequest => 8,
            Self::FloorReply => 9,
            #[cfg(feature = "hot-join")]
            Self::JoinRequest => 10,
            #[cfg(feature = "hot-join")]
            Self::StateSnapshot => 11,
            #[cfg(feature = "hot-join")]
            Self::StateSnapshotAck => 12,
            #[cfg(feature = "hot-join")]
            Self::ReactivateSlot => 13,
            #[cfg(feature = "hot-join")]
            Self::ReactivateSlotAck => 14,
            #[cfg(feature = "hot-join")]
            Self::JoinCommitted => 15,
            #[cfg(feature = "hot-join")]
            Self::JoinAborted => 16,
        }
    }
}

impl std::fmt::Display for MessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Per-[`MessageKind`] counters, keyed by category.
///
/// Backed by a fixed-size array (one slot per [`MessageKind`]); read individual
/// counts with [`get`](Self::get) or the grand total with [`total`](Self::total).
/// Serializes as a JSON object keyed by each category's [`MessageKind::as_str`]
/// label, so the wire form is self-describing and stable across counter values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageKindCounts([u64; MessageKind::COUNT]);

impl Default for MessageKindCounts {
    fn default() -> Self {
        Self([0; MessageKind::COUNT])
    }
}

impl MessageKindCounts {
    /// The count recorded for `kind`.
    ///
    /// `kind.index()` is always in bounds, so the `unwrap_or` fallback is
    /// unreachable; it keeps the accessor panic-free without an index.
    #[must_use]
    pub fn get(&self, kind: MessageKind) -> u64 {
        self.0.get(kind.index()).copied().unwrap_or(0)
    }

    /// The total number of messages recorded across every category.
    ///
    /// By construction this equals the corresponding packet counter
    /// ([`PeerMetrics::packets_sent`] / [`PeerMetrics::packets_received`]): every
    /// counted packet increments exactly one category bucket.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.0.iter().copied().fold(0u64, u64::saturating_add)
    }

    /// Increments the counter for `kind` by one, saturating at [`u64::MAX`].
    pub(crate) fn record(&mut self, kind: MessageKind) {
        if let Some(slot) = self.0.get_mut(kind.index()) {
            *slot = slot.saturating_add(1);
        }
    }
}

impl Serialize for MessageKindCounts {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(MessageKind::COUNT))?;
        for kind in MessageKind::ALL {
            map.serialize_entry(kind.as_str(), &self.get(kind))?;
        }
        map.end()
    }
}

/// A per-peer snapshot of protocol-level traffic and connection metrics for one
/// remote endpoint.
///
/// Read one with [`P2PSession::peer_metrics`]. Like [`SessionMetrics`], this is a
/// cheap `Copy` snapshot updated inline on the paths it measures — no timers, no
/// allocation, no `Instant` — so reading it is deterministic and WASM-safe.
///
/// # Counters vs gauges
///
/// The byte, packet, message-kind, and input-compression fields are **cumulative
/// counters**, monotonic for the life of the endpoint. The trailing four fields —
/// [`pending_output_len`](Self::pending_output_len),
/// [`pending_checksums_len`](Self::pending_checksums_len),
/// [`ping_ms`](Self::ping_ms), and
/// [`remote_frame_advantage`](Self::remote_frame_advantage) — are **instantaneous
/// gauges** sampled at the moment of the snapshot.
///
/// Byte counts are wire-exact (the same arithmetic as the bandwidth accounting
/// behind [`NetworkStats`](crate::NetworkStats)) and count payload bytes only —
/// they exclude the per-packet UDP/IP header that
/// [`NetworkStats::kbps_sent`](crate::NetworkStats::kbps_sent) folds into its
/// estimate. Sent bytes/packets are tallied when a message is enqueued for the
/// socket (mirroring the pre-existing `bytes_sent` accounting), received ones
/// when a message is delivered to the endpoint.
///
/// # Per-endpoint attribution
///
/// Every counter is scoped to **one endpoint** (one remote peer or spectator).
/// In the unusual configuration where the same address is registered as both a
/// remote player and a spectator, each is a distinct endpoint with its own
/// snapshot, so a single datagram delivered to that address is counted once per
/// endpoint. Read per-peer values individually; do not sum `bytes_received`
/// across handles expecting a single "total bytes off the wire".
///
/// This type is `#[non_exhaustive]`: future library versions may add fields
/// without a breaking change, so match with `..`.
///
/// [`P2PSession::peer_metrics`]: crate::P2PSession::peer_metrics
/// [`SessionMetrics`]: crate::metrics::SessionMetrics
///
/// # Example
///
/// ```
/// # use fortress_rollback::metrics::PeerMetrics;
/// let metrics = PeerMetrics::default();
/// assert_eq!(metrics.bytes_sent, 0);
/// assert_eq!(metrics.packets_received, 0);
/// ```
#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[must_use = "PeerMetrics should be inspected after being queried"]
pub struct PeerMetrics {
    /// Total wire-exact payload bytes sent to this peer over the life of the
    /// endpoint (payload only; see the type docs).
    pub bytes_sent: u64,

    /// Total wire-exact payload bytes received from this peer, counted for every
    /// message delivered to the endpoint before any protocol-state filtering.
    pub bytes_received: u64,

    /// Total packets (messages) sent to this peer — equal to the
    /// [`messages_sent_by_kind`](Self::messages_sent_by_kind) total.
    pub packets_sent: u64,

    /// Total packets (messages) received from this peer — equal to the
    /// [`messages_received_by_kind`](Self::messages_received_by_kind) total.
    pub packets_received: u64,

    /// Per-[`MessageKind`] breakdown of [`packets_sent`](Self::packets_sent).
    pub messages_sent_by_kind: MessageKindCounts,

    /// Per-[`MessageKind`] breakdown of
    /// [`packets_received`](Self::packets_received).
    pub messages_received_by_kind: MessageKindCounts,

    /// Cumulative raw input bytes batched into `Input` packets **before**
    /// delta/RLE compression. When non-zero, dividing
    /// [`input_bytes_post_compression`](Self::input_bytes_post_compression) by
    /// this gives the realized compression ratio (it stays 0 until the first
    /// input is sent).
    pub input_bytes_pre_compression: u64,

    /// Cumulative encoded input bytes placed on the wire in `Input` packets
    /// **after** delta/RLE compression.
    pub input_bytes_post_compression: u64,

    /// **Gauge.** The number of input frames queued for (re)transmission that the
    /// peer has not yet acknowledged — the connection-backpressure signal also
    /// reported as
    /// [`NetworkStats::send_queue_len`](crate::NetworkStats::send_queue_len).
    pub pending_output_len: u64,

    /// **Gauge.** The number of this peer's confirmed-frame checksums buffered
    /// awaiting comparison against the local history.
    pub pending_checksums_len: u64,

    /// **Gauge.** The round-trip time to this peer in milliseconds, as most
    /// recently measured by a quality-report exchange (the same value as
    /// [`NetworkStats::ping`](crate::NetworkStats::ping)).
    pub ping_ms: u128,

    /// **Gauge.** The peer's most recently reported frame-advantage value — the
    /// same quantity [`NetworkStats::remote_frames_behind`] surfaces (the remote
    /// player's own estimate of the local↔remote frame gap; see that field for
    /// the sign convention).
    ///
    /// [`NetworkStats::remote_frames_behind`]: crate::NetworkStats::remote_frames_behind
    pub remote_frame_advantage: i32,
}

impl PeerMetrics {
    /// Serializes this snapshot to a compact JSON string.
    ///
    /// Returns `None` if serialization fails — not expected for this small set of
    /// integer counters, but not impossible (for example, an allocation failure
    /// inside `serde_json`).
    #[cfg(feature = "json")]
    #[must_use]
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }

    /// Serializes this snapshot to a pretty-printed JSON string.
    ///
    /// Like [`to_json`](Self::to_json), but indented for readability.
    #[cfg(feature = "json")]
    #[must_use]
    pub fn to_json_pretty(&self) -> Option<String> {
        serde_json::to_string_pretty(self).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Config, FortressEvent, Frame, PlayerHandle};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    struct TestConfig;
    impl Config for TestConfig {
        type Input = u8;
        type State = u8;
        type Address = SocketAddr;
    }

    fn addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1)
    }

    #[test]
    fn event_kind_all_len_matches_count() {
        assert_eq!(EventKind::ALL.len(), EventKind::COUNT);
    }

    #[test]
    fn event_kind_index_is_bijective_over_all() {
        for (i, kind) in EventKind::ALL.into_iter().enumerate() {
            assert_eq!(kind.index(), i, "index of {kind:?}");
        }
    }

    #[test]
    fn event_kind_as_str_labels_are_unique_and_nonempty() {
        let mut seen = std::collections::BTreeSet::new();
        for kind in EventKind::ALL {
            assert!(!kind.as_str().is_empty());
            assert!(
                seen.insert(kind.as_str()),
                "duplicate label {}",
                kind.as_str()
            );
        }
        assert_eq!(seen.len(), EventKind::COUNT);
    }

    #[test]
    fn event_kind_counts_default_is_zero_and_records_saturate() {
        let mut counts = EventKindCounts::default();
        for kind in EventKind::ALL {
            assert_eq!(counts.get(kind), 0);
        }
        counts.record(EventKind::Disconnected);
        counts.record(EventKind::Disconnected);
        assert_eq!(counts.get(EventKind::Disconnected), 2);
        assert_eq!(counts.get(EventKind::Synchronized), 0);
    }

    #[test]
    fn session_metrics_record_event_discard_bumps_total_and_kind() {
        let mut metrics = SessionMetrics::new();
        assert_eq!(metrics.events_discarded_total, 0);
        metrics.record_event_discard(EventKind::DesyncDetected);
        metrics.record_event_discard(EventKind::Disconnected);
        metrics.record_event_discard(EventKind::Disconnected);
        assert_eq!(metrics.events_discarded_total, 3);
        assert_eq!(
            metrics
                .events_discarded_by_kind
                .get(EventKind::DesyncDetected),
            1
        );
        assert_eq!(
            metrics
                .events_discarded_by_kind
                .get(EventKind::Disconnected),
            2
        );
        assert_eq!(
            metrics
                .events_discarded_by_kind
                .get(EventKind::Synchronized),
            0
        );
    }

    #[test]
    fn session_metrics_unknown_source_counter_saturates() {
        let mut metrics = SessionMetrics::new();
        metrics.record_unknown_source_packet();
        assert_eq!(metrics.unknown_source_packets, 1);

        metrics.unknown_source_packets = u64::MAX;
        metrics.record_unknown_source_packet();
        assert_eq!(metrics.unknown_source_packets, u64::MAX);
    }

    #[test]
    fn fortress_event_kind_maps_every_variant() {
        let a = addr();
        let cases: [(FortressEvent<TestConfig>, EventKind); 12] = [
            (
                FortressEvent::Synchronizing {
                    addr: a,
                    total: 1,
                    count: 0,
                    total_requests_sent: 0,
                    elapsed_ms: 0,
                },
                EventKind::Synchronizing,
            ),
            (
                FortressEvent::Synchronized { addr: a },
                EventKind::Synchronized,
            ),
            (
                FortressEvent::Disconnected { addr: a },
                EventKind::Disconnected,
            ),
            (
                FortressEvent::NetworkInterrupted {
                    addr: a,
                    disconnect_timeout: 0,
                },
                EventKind::NetworkInterrupted,
            ),
            (
                FortressEvent::NetworkResumed { addr: a },
                EventKind::NetworkResumed,
            ),
            (
                FortressEvent::WaitRecommendation { skip_frames: 0 },
                EventKind::WaitRecommendation,
            ),
            (
                FortressEvent::DesyncDetected {
                    frame: Frame::new(1),
                    local_checksum: 0,
                    remote_checksum: 0,
                    addr: a,
                },
                EventKind::DesyncDetected,
            ),
            (
                FortressEvent::SyncTimeout {
                    addr: a,
                    elapsed_ms: 0,
                },
                EventKind::SyncTimeout,
            ),
            (
                FortressEvent::ReplayDesync {
                    frame: Frame::new(1),
                    expected_checksum: 0,
                    actual_checksum: 0,
                },
                EventKind::ReplayDesync,
            ),
            (
                FortressEvent::SpectatorDivergence {
                    frame: Frame::new(1),
                    player: PlayerHandle::new(0),
                    primary_addr: a,
                    conflicting_addr: a,
                },
                EventKind::SpectatorDivergence,
            ),
            (
                FortressEvent::InputDelayRecommendation {
                    player_handle: PlayerHandle::new(0),
                    current_delay: 0,
                    suggested_delay: 0,
                },
                EventKind::InputDelayRecommendation,
            ),
            (
                FortressEvent::PeerDropped {
                    handle: PlayerHandle::new(0),
                    addr: a,
                },
                EventKind::PeerDropped,
            ),
        ];
        for (event, expected) in cases {
            assert_eq!(event.kind(), expected, "expected kind {expected:?}");
        }

        #[cfg(feature = "hot-join")]
        {
            assert_eq!(
                FortressEvent::<TestConfig>::JoinRequested {
                    handle: PlayerHandle::new(0),
                    addr: a,
                }
                .kind(),
                EventKind::JoinRequested
            );
            assert_eq!(
                FortressEvent::<TestConfig>::PeerJoined {
                    handle: PlayerHandle::new(0),
                    addr: a,
                }
                .kind(),
                EventKind::PeerJoined
            );
        }
    }

    #[test]
    fn rollback_histogram_buckets_depths_and_saturates_overflow() {
        let mut hist = RollbackDepthHistogram::default();
        assert_eq!(hist.total(), 0);
        // depth 0 is not a real rollback and is ignored
        hist.record(0);
        assert_eq!(hist.total(), 0);
        // depth d (1..=16) lands in slot d-1
        for depth in 1..=16usize {
            hist.record(depth);
            assert_eq!(hist.bucket(depth - 1), 1, "depth {depth}");
        }
        // every depth > 16 saturates into the final "17_plus" bucket (index 16)
        hist.record(17);
        hist.record(100);
        hist.record(usize::MAX);
        assert_eq!(hist.bucket(16), 3);
        assert_eq!(hist.total(), 16 + 3);
        // out-of-range index reads as zero, never panics
        assert_eq!(hist.bucket(RollbackDepthHistogram::BUCKETS), 0);
        assert_eq!(hist.bucket(999), 0);
    }

    #[test]
    fn record_rollback_keeps_frames_advanced_identity_and_max_depth() {
        let mut m = SessionMetrics::new();
        // Two forward advances and two rollbacks (depths 3 and 5).
        m.record_forward_advance(1);
        m.record_forward_advance(2);
        m.record_rollback(3);
        m.record_rollback(5);
        assert_eq!(m.visual_frames, 2);
        assert_eq!(m.resimulated_frames, 8);
        // The core invariant: total simulation work = visual + resimulated.
        assert_eq!(m.frames_advanced, m.visual_frames + m.resimulated_frames);
        assert_eq!(m.frames_advanced, 10);
        assert_eq!(m.rollback_count, 2);
        assert_eq!(m.rollback_depth_histogram.total(), m.rollback_count);
        assert_eq!(m.max_rollback_depth, 5);
        assert_eq!(m.rollback_depth_histogram.bucket(2), 1); // depth 3
        assert_eq!(m.rollback_depth_histogram.bucket(4), 1); // depth 5
    }

    #[test]
    fn record_forward_advance_tracks_confirmation_lag_gauge_max_and_sum() {
        let mut m = SessionMetrics::new();
        m.record_forward_advance(5);
        m.record_forward_advance(2);
        m.record_forward_advance(9);
        assert_eq!(m.confirmation_lag_current, 9); // gauge: last sample
        assert_eq!(m.confirmation_lag_max, 9);
        assert_eq!(m.confirmation_lag_sum, 16);
        assert_eq!(m.visual_frames, 3);
    }

    #[test]
    fn record_checksum_comparison_splits_matches_and_mismatches() {
        let mut m = SessionMetrics::new();
        m.record_checksum_comparison(true);
        m.record_checksum_comparison(false);
        m.record_checksum_comparison(true);
        assert_eq!(m.checksums_compared, 3);
        assert_eq!(m.checksums_matched, 2);
        assert_eq!(m.checksums_mismatched, 1);
        assert_eq!(
            m.checksums_compared,
            m.checksums_matched + m.checksums_mismatched
        );
    }

    #[test]
    fn high_water_marks_track_the_maximum_observed_length() {
        let mut m = SessionMetrics::new();
        m.observe_event_queue_len(3);
        m.observe_event_queue_len(7);
        m.observe_event_queue_len(4); // shrinking does not lower the mark
        assert_eq!(m.event_queue_high_water, 7);
        m.observe_checksum_history_len(10);
        m.observe_checksum_history_len(2);
        assert_eq!(m.checksum_history_high_water, 10);
    }

    #[test]
    fn stall_prediction_miss_and_wait_recommendation_counters_saturate_up() {
        let mut m = SessionMetrics::new();
        m.record_stall();
        m.record_stall();
        m.record_prediction_misses(4);
        m.record_prediction_misses(1);
        m.record_wait_recommendation();
        assert_eq!(m.stall_count, 2);
        assert_eq!(m.prediction_miss_count, 5);
        assert_eq!(m.wait_recommendations, 1);
    }

    #[cfg(feature = "json")]
    #[test]
    fn rollback_histogram_serializes_as_labeled_depth_map() {
        let mut m = SessionMetrics::new();
        m.record_rollback(1);
        m.record_rollback(17);
        let json = m.to_json().expect("json serialization succeeds");
        assert!(json.contains(r#""1":1"#), "{json}");
        assert!(json.contains(r#""17_plus":1"#), "{json}");
        assert!(json.contains(r#""frames_advanced":18"#), "{json}");
    }

    #[cfg(feature = "json")]
    #[test]
    fn session_metrics_serializes_kind_breakdown_as_labeled_map() {
        let mut metrics = SessionMetrics::new();
        metrics.record_event_discard(EventKind::Disconnected);
        metrics.record_unknown_source_packet();
        let json = metrics.to_json().expect("json serialization succeeds");
        assert!(json.contains(r#""events_discarded_total":1"#), "{json}");
        assert!(json.contains(r#""unknown_source_packets":1"#), "{json}");
        // The per-kind breakdown is a self-describing, snake_case-keyed map.
        assert!(json.contains(r#""disconnected":1"#), "{json}");
        assert!(json.contains(r#""synchronized":0"#), "{json}");
    }

    #[test]
    fn message_kind_all_len_matches_count() {
        assert_eq!(MessageKind::ALL.len(), MessageKind::COUNT);
    }

    #[test]
    fn message_kind_index_is_bijective_over_all() {
        for (i, kind) in MessageKind::ALL.into_iter().enumerate() {
            assert_eq!(kind.index(), i, "index of {kind:?}");
        }
    }

    #[test]
    fn message_kind_as_str_labels_are_unique_and_nonempty() {
        let mut seen = std::collections::BTreeSet::new();
        for kind in MessageKind::ALL {
            assert!(!kind.as_str().is_empty());
            assert!(
                seen.insert(kind.as_str()),
                "duplicate label {}",
                kind.as_str()
            );
        }
        assert_eq!(seen.len(), MessageKind::COUNT);
    }

    #[test]
    fn message_kind_counts_record_get_and_total_saturate() {
        let mut counts = MessageKindCounts::default();
        assert_eq!(counts.total(), 0);
        for kind in MessageKind::ALL {
            assert_eq!(counts.get(kind), 0);
        }
        counts.record(MessageKind::Input);
        counts.record(MessageKind::Input);
        counts.record(MessageKind::KeepAlive);
        assert_eq!(counts.get(MessageKind::Input), 2);
        assert_eq!(counts.get(MessageKind::KeepAlive), 1);
        assert_eq!(counts.get(MessageKind::SyncRequest), 0);
        // The total is the sum across every category.
        assert_eq!(counts.total(), 3);
    }

    #[test]
    fn peer_metrics_default_is_zero() {
        let m = PeerMetrics::default();
        assert_eq!(m.bytes_sent, 0);
        assert_eq!(m.bytes_received, 0);
        assert_eq!(m.packets_sent, 0);
        assert_eq!(m.packets_received, 0);
        assert_eq!(m.messages_sent_by_kind.total(), 0);
        assert_eq!(m.messages_received_by_kind.total(), 0);
        assert_eq!(m.input_bytes_pre_compression, 0);
        assert_eq!(m.input_bytes_post_compression, 0);
        assert_eq!(m.pending_output_len, 0);
        assert_eq!(m.pending_checksums_len, 0);
        assert_eq!(m.ping_ms, 0);
        assert_eq!(m.remote_frame_advantage, 0);
    }

    #[cfg(feature = "json")]
    #[test]
    fn peer_metrics_serializes_kind_breakdown_as_labeled_map() {
        let mut m = PeerMetrics {
            packets_sent: 1,
            ..Default::default()
        };
        m.messages_sent_by_kind.record(MessageKind::Input);
        let json = m.to_json().expect("json serialization succeeds");
        assert!(json.contains(r#""packets_sent":1"#), "{json}");
        // The per-kind breakdown is a self-describing, snake_case-keyed map.
        assert!(json.contains(r#""input":1"#), "{json}");
        assert!(json.contains(r#""keep_alive":0"#), "{json}");
    }
}
