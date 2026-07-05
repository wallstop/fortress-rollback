//! Always-on, pull-based session metrics.
//!
//! [`SessionMetrics`] is a cheap, `Copy` snapshot of cumulative session
//! counters returned by [`P2PSession::metrics`] and
//! [`SpectatorSession::metrics`]. Counters are plain integers updated inline on
//! the paths they measure — no timers, no allocation, no `Instant` — so reading
//! them is deterministic and WASM-safe.
//!
//! The first surface exposed here is **event-queue overflow accounting**: when
//! the bounded event queue discards an undrained [`FortressEvent`] the session
//! records it in [`SessionMetrics::events_discarded_total`] and the per-category
//! [`SessionMetrics::events_discarded_by_kind`] breakdown, so a lost
//! notification — possibly a safety-critical [`FortressEvent::Disconnected`] or
//! [`FortressEvent::DesyncDetected`] — is observable instead of silent.
//!
//! [`P2PSession::metrics`]: crate::P2PSession::metrics
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
/// In normal use you read a snapshot from [`P2PSession::metrics`] or
/// [`SpectatorSession::metrics`]; every counter is monotonic for the life of the
/// session and cheap to read (the type is `Copy`).
///
/// This type is `#[non_exhaustive]`: future library versions may add counters
/// without a breaking change, so it cannot be built with a struct literal
/// outside this crate — match with `..`.
///
/// # Which counters each session type populates
///
/// The rollback and pacing counters (everything except the
/// `events_discarded_*` family) are populated by [`P2PSession`]. A
/// [`SpectatorSession`] shares this type but currently records only the
/// event-discard counters (plus its own `frames_behind_host`, exposed
/// separately); its rollback/pacing counters stay at their default `0`.
///
/// [`P2PSession`]: crate::P2PSession
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
        let json = metrics.to_json().expect("json serialization succeeds");
        assert!(json.contains(r#""events_discarded_total":1"#), "{json}");
        // The per-kind breakdown is a self-describing, snake_case-keyed map.
        assert!(json.contains(r#""disconnected":1"#), "{json}");
        assert!(json.contains(r#""synchronized":0"#), "{json}");
    }
}
