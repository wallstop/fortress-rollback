use std::collections::{vec_deque::Drain, VecDeque};
use std::iter::FusedIterator;

use crate::{Config, EventKind, FortressEvent};

/// Whether an event should survive queue pressure ahead of routine updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventRetention {
    Routine,
    Durable,
}

/// Classifies every public event kind for bounded-queue retention.
///
/// Routine progress and advisory events may be superseded by newer observations.
/// Durable events describe lifecycle changes, link-state transitions, or faults
/// that applications commonly need to act on.
const fn event_retention(kind: EventKind) -> EventRetention {
    match kind {
        EventKind::Synchronizing
        | EventKind::WaitRecommendation
        | EventKind::InputDelayRecommendation => EventRetention::Routine,
        EventKind::Synchronized
        | EventKind::Disconnected
        | EventKind::NetworkInterrupted
        | EventKind::NetworkResumed
        | EventKind::DesyncDetected
        | EventKind::SyncTimeout
        | EventKind::IncompatibleSession
        | EventKind::ReplayDesync
        | EventKind::SpectatorDivergence
        | EventKind::PeerDropped => EventRetention::Durable,
        #[cfg(feature = "hot-join")]
        EventKind::JoinRequested => EventRetention::Routine,
        #[cfg(feature = "hot-join")]
        EventKind::PeerJoined => EventRetention::Durable,
    }
}

/// Removes one event to restore a bounded queue after overflow.
///
/// The oldest routine event is removed first, wherever it sits in the queue.
/// If the queue contains only durable events, the oldest durable event is
/// removed: bounded allocation remains mandatory even during a fault storm.
/// Relative order among all retained events is preserved.
#[cfg(test)]
pub(crate) fn remove_event_for_overflow<T: Config>(
    queue: &mut VecDeque<FortressEvent<T>>,
) -> Option<FortressEvent<T>> {
    let routine_index = queue
        .iter()
        .position(|event| event_retention(event.kind()) == EventRetention::Routine);
    match routine_index {
        Some(index) => queue.remove(index),
        None => queue.pop_front(),
    }
}

/// Inserts an event without ever letting the queue exceed its reserved cap.
///
/// Callers reserve `cap` elements fallibly during session construction. At
/// capacity, this removes the oldest queued routine event and inserts the new
/// event when one exists. Otherwise it rejects an incoming routine event, or
/// removes the oldest durable event before inserting an incoming durable event.
/// The returned event is exactly the rejected incoming event or removed queued
/// event. Thus every insertion stays within the reservation and cannot trigger
/// growth. A zero cap rejects every event.
pub(crate) fn enqueue_event_bounded<T: Config>(
    queue: &mut VecDeque<FortressEvent<T>>,
    cap: usize,
    event: FortressEvent<T>,
) -> Option<FortressEvent<T>> {
    if cap == 0 {
        return Some(event);
    }
    if queue.len() >= cap {
        if let Some(routine_index) = queue
            .iter()
            .position(|queued| event_retention(queued.kind()) == EventRetention::Routine)
        {
            let dropped = queue.remove(routine_index);
            queue.push_back(event);
            return dropped;
        }
        if event_retention(event.kind()) == EventRetention::Routine {
            return Some(event);
        }
        let dropped = queue.pop_front();
        queue.push_back(event);
        return dropped;
    }
    queue.push_back(event);
    None
}

/// A zero-allocation opaque iterator that drains events from a session.
///
/// This type wraps the internal event queue drain, providing a stable public API
/// that doesn't expose `std::collections::vec_deque::Drain` directly. It implements
/// [`Iterator`], [`DoubleEndedIterator`], [`ExactSizeIterator`], and [`FusedIterator`].
///
/// Obtain an `EventDrain` by calling [`P2PSession::events()`],
/// [`SpectatorSession::events()`], [`ReplaySession::events()`],
/// [`SyncTestSession::events()`], or the [`Session::events()`] trait method.
///
/// # Examples
///
/// ```ignore
/// for event in session.events() {
///     match event {
///         FortressEvent::WaitRecommendation { skip_frames } => {
///             println!("Should skip {skip_frames} frames");
///         }
///         _ => { /* handle other events */ }
///     }
/// }
/// ```
///
/// [`P2PSession::events()`]: crate::P2PSession::events
/// [`ReplaySession::events()`]: crate::ReplaySession::events
/// [`SpectatorSession::events()`]: crate::SpectatorSession::events
/// [`SyncTestSession::events()`]: crate::SyncTestSession::events
/// [`Session::events()`]: crate::Session::events
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct EventDrain<'a, T: Config> {
    inner: EventDrainInner<'a, T>,
}

enum EventDrainInner<'a, T: Config> {
    Queue(Drain<'a, FortressEvent<T>>),
    #[allow(dead_code)]
    Empty,
}

impl<'a, T: Config> EventDrain<'a, T> {
    pub(crate) fn from_drain(drain: Drain<'a, FortressEvent<T>>) -> Self {
        Self {
            inner: EventDrainInner::Queue(drain),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn empty() -> Self {
        Self {
            inner: EventDrainInner::Empty,
        }
    }
}

impl<T: Config> Iterator for EventDrain<'_, T> {
    type Item = FortressEvent<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            EventDrainInner::Queue(drain) => drain.next(),
            EventDrainInner::Empty => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            EventDrainInner::Queue(drain) => drain.size_hint(),
            EventDrainInner::Empty => (0, Some(0)),
        }
    }
}

impl<T: Config> DoubleEndedIterator for EventDrain<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            EventDrainInner::Queue(drain) => drain.next_back(),
            EventDrainInner::Empty => None,
        }
    }
}

impl<T: Config> ExactSizeIterator for EventDrain<'_, T> {
    fn len(&self) -> usize {
        match &self.inner {
            EventDrainInner::Queue(drain) => drain.len(),
            EventDrainInner::Empty => 0,
        }
    }
}

impl<T: Config> FusedIterator for EventDrain<'_, T> {}

impl<T: Config> std::fmt::Debug for EventDrain<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventDrain")
            .field("remaining", &self.len())
            .finish()
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::iter_with_drain
)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::net::SocketAddr;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestConfig;

    impl Config for TestConfig {
        type Input = u8;
        type State = Vec<u8>;
        type Address = SocketAddr;
    }

    fn make_event(skip: u32) -> FortressEvent<TestConfig> {
        FortressEvent::WaitRecommendation { skip_frames: skip }
    }

    fn addr(port: u16) -> SocketAddr {
        ([127, 0, 0, 1], port).into()
    }

    #[test]
    fn event_retention_classifies_every_kind() {
        let cases = [
            (EventKind::Synchronizing, EventRetention::Routine),
            (EventKind::Synchronized, EventRetention::Durable),
            (EventKind::Disconnected, EventRetention::Durable),
            (EventKind::NetworkInterrupted, EventRetention::Durable),
            (EventKind::NetworkResumed, EventRetention::Durable),
            (EventKind::WaitRecommendation, EventRetention::Routine),
            (EventKind::DesyncDetected, EventRetention::Durable),
            (EventKind::SyncTimeout, EventRetention::Durable),
            (EventKind::IncompatibleSession, EventRetention::Durable),
            (EventKind::ReplayDesync, EventRetention::Durable),
            (EventKind::SpectatorDivergence, EventRetention::Durable),
            (EventKind::InputDelayRecommendation, EventRetention::Routine),
            (EventKind::PeerDropped, EventRetention::Durable),
        ];
        assert_eq!(cases.len(), 13);
        for (kind, expected) in cases {
            assert_eq!(
                event_retention(kind),
                expected,
                "classification for {kind:?}"
            );
        }

        #[cfg(feature = "hot-join")]
        {
            assert_eq!(EventKind::COUNT, 15);
            assert_eq!(
                event_retention(EventKind::JoinRequested),
                EventRetention::Routine
            );
            assert_eq!(
                event_retention(EventKind::PeerJoined),
                EventRetention::Durable
            );
        }
    }

    #[test]
    fn overflow_removes_oldest_routine_and_preserves_relative_order() {
        let durable_a = FortressEvent::Synchronized { addr: addr(7001) };
        let routine_a = make_event(1);
        let durable_b = FortressEvent::Disconnected { addr: addr(7002) };
        let routine_b = make_event(2);
        let mut queue = VecDeque::from([durable_a, routine_a, durable_b, routine_b]);

        assert_eq!(remove_event_for_overflow(&mut queue), Some(routine_a));
        assert_eq!(
            queue,
            VecDeque::from([durable_a, durable_b, routine_b]),
            "removing a middle event must preserve FIFO order among survivors"
        );
        assert_eq!(remove_event_for_overflow(&mut queue), Some(routine_b));
        assert_eq!(queue, VecDeque::from([durable_a, durable_b]));
    }

    #[test]
    fn routine_arrival_cannot_displace_full_durable_queue() {
        let durable_a = FortressEvent::Synchronized { addr: addr(7011) };
        let durable_b = FortressEvent::Disconnected { addr: addr(7012) };
        let routine = make_event(3);
        let mut queue = VecDeque::from([durable_a, durable_b]);

        assert_eq!(enqueue_event_bounded(&mut queue, 2, routine), Some(routine));
        assert_eq!(queue, VecDeque::from([durable_a, durable_b]));
    }

    #[test]
    fn durable_arrival_to_full_durable_queue_drops_oldest_and_stays_bounded() {
        let peer_dropped: FortressEvent<TestConfig> = FortressEvent::PeerDropped {
            handle: crate::PlayerHandle::new(1),
            addr: addr(7021),
        };
        let disconnected = FortressEvent::Disconnected { addr: addr(7021) };
        let synchronized = FortressEvent::Synchronized { addr: addr(7022) };
        let mut queue = VecDeque::from([peer_dropped, disconnected]);

        assert_eq!(
            enqueue_event_bounded(&mut queue, 2, synchronized),
            Some(peer_dropped),
            "a durable-only queue must still stay bounded by dropping its oldest event"
        );
        assert_eq!(queue, VecDeque::from([disconnected, synchronized]));
    }

    #[test]
    fn bounded_enqueue_pre_evicts_without_exceeding_reserved_capacity() {
        let mut queue = VecDeque::new();
        queue.try_reserve_exact(2).expect("test queue reserves");
        let capacity = queue.capacity();
        let durable = FortressEvent::Disconnected { addr: addr(7031) };
        assert_eq!(enqueue_event_bounded(&mut queue, 2, durable), None);
        assert_eq!(enqueue_event_bounded(&mut queue, 2, make_event(1)), None);

        assert_eq!(
            enqueue_event_bounded(&mut queue, 2, make_event(2)),
            Some(make_event(1))
        );
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.capacity(), capacity, "bounded insert must not grow");
        assert_eq!(queue, VecDeque::from([durable, make_event(2)]));
    }

    #[test]
    fn empty_drain_returns_none() {
        let mut drain = EventDrain::<TestConfig>::empty();
        assert!(drain.next().is_none());
    }

    #[test]
    fn empty_drain_has_zero_len() {
        let drain = EventDrain::<TestConfig>::empty();
        assert_eq!(drain.len(), 0);
    }

    #[test]
    fn drain_from_queue_yields_all_events() {
        let mut queue: VecDeque<FortressEvent<TestConfig>> = VecDeque::new();
        queue.push_back(make_event(1));
        queue.push_back(make_event(2));
        queue.push_back(make_event(3));

        let drain = EventDrain::from_drain(queue.drain(..));
        let events: Vec<_> = drain.collect();

        assert_eq!(events.len(), 3);
        assert_eq!(events[0], make_event(1));
        assert_eq!(events[1], make_event(2));
        assert_eq!(events[2], make_event(3));
    }

    #[test]
    fn drain_is_fused() {
        let mut queue: VecDeque<FortressEvent<TestConfig>> = VecDeque::new();
        queue.push_back(make_event(1));

        let mut drain = EventDrain::from_drain(queue.drain(..));
        assert!(drain.next().is_some());
        assert!(drain.next().is_none());
        assert!(drain.next().is_none());
    }

    #[test]
    fn double_ended_iteration() {
        let mut queue: VecDeque<FortressEvent<TestConfig>> = VecDeque::new();
        queue.push_back(make_event(1));
        queue.push_back(make_event(2));
        queue.push_back(make_event(3));

        let mut drain = EventDrain::from_drain(queue.drain(..));
        assert_eq!(drain.next_back(), Some(make_event(3)));
        assert_eq!(drain.next(), Some(make_event(1)));
        assert_eq!(drain.next_back(), Some(make_event(2)));
        assert!(drain.next().is_none());
    }

    #[test]
    fn exact_size_is_accurate() {
        let mut queue: VecDeque<FortressEvent<TestConfig>> = VecDeque::new();
        queue.push_back(make_event(1));
        queue.push_back(make_event(2));

        let mut drain = EventDrain::from_drain(queue.drain(..));
        assert_eq!(drain.len(), 2);
        let _ = drain.next();
        assert_eq!(drain.len(), 1);
        let _ = drain.next();
        assert_eq!(drain.len(), 0);
    }

    #[test]
    fn debug_format_shows_remaining_count() {
        let mut queue: VecDeque<FortressEvent<TestConfig>> = VecDeque::new();
        queue.push_back(make_event(1));
        queue.push_back(make_event(2));
        let drain = EventDrain::from_drain(queue.drain(..));
        assert_eq!(format!("{drain:?}"), "EventDrain { remaining: 2 }");
    }

    #[test]
    fn debug_format_empty_drain() {
        let drain: EventDrain<'_, TestConfig> = EventDrain::empty();
        assert_eq!(format!("{drain:?}"), "EventDrain { remaining: 0 }");
    }

    #[test]
    fn size_hint_for_queue_drain() {
        let mut queue: VecDeque<FortressEvent<TestConfig>> = VecDeque::new();
        queue.push_back(make_event(1));
        queue.push_back(make_event(2));
        queue.push_back(make_event(3));
        let drain = EventDrain::from_drain(queue.drain(..));
        assert_eq!(drain.size_hint(), (3, Some(3)));
    }

    #[test]
    fn size_hint_for_empty_drain() {
        let drain: EventDrain<'_, TestConfig> = EventDrain::empty();
        assert_eq!(drain.size_hint(), (0, Some(0)));
    }
}
