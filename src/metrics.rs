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
/// # Example
///
/// ```
/// # use fortress_rollback::metrics::SessionMetrics;
/// let metrics = SessionMetrics::default();
/// assert_eq!(metrics.events_discarded_total, 0);
/// ```
///
/// [`P2PSession::metrics`]: crate::P2PSession::metrics
/// [`SpectatorSession::metrics`]: crate::SpectatorSession::metrics
#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[must_use = "SessionMetrics should be inspected after being queried"]
pub struct SessionMetrics {
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
