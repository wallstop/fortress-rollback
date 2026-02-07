use std::collections::vec_deque::Drain;
use std::iter::FusedIterator;

use crate::{Config, FortressEvent};

/// A zero-allocation opaque iterator that drains events from a session.
///
/// This type wraps the internal event queue drain, providing a stable public API
/// that doesn't expose `std::collections::vec_deque::Drain` directly. It implements
/// [`Iterator`], [`DoubleEndedIterator`], [`ExactSizeIterator`], and [`FusedIterator`].
///
/// Obtain an `EventDrain` by calling [`P2PSession::events()`],
/// [`SpectatorSession::events()`], [`SyncTestSession::events()`], or the
/// [`Session::events()`] trait method.
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
