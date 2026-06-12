//! Deterministic hold-then-release "reordering" socket for in-process meshes.
//!
//! [`ReorderSocket`] delivers messages through shared in-memory inboxes (like
//! the `DelaySocket` mesh in `tests/sessions/desync_harvest.rs`), but adds
//! per-directed-link **HOLD / RELEASE** control: while a `(src, dst)` link is
//! held, every message sent on it is queued instead of delivered; releasing
//! the link flushes the held messages into the destination inbox **in their
//! original FIFO order** and re-opens the link. Nothing is ever dropped
//! (0% loss), there is no wall-clock dependence, and all state lives behind a
//! single shared mutex, so a single-threaded test is fully deterministic.
//!
//! This is the "reordering" primitive the Session-29 F17 analysis showed a
//! plain FIFO delay socket cannot express: traffic on *other* links keeps
//! flowing while one link is held, so a peer can observe a *later* frame's
//! input from one remote before an *earlier* frame's input from another —
//! exactly the cross-link arrival inversion the F17 red test needs. Within a
//! single link FIFO order is preserved (hold-then-release, not shuffling),
//! which the cumulative input encoding of the protocol tolerates natively.
//!
//! The shared [`HeldLinks`] handle mirrors the [`BlockedLinks`] pattern from
//! [`filter_socket`](super::filter_socket): it is `Clone` (all clones share
//! state) and is retained by the test to toggle links mid-run after the
//! sockets have been moved into their sessions.

use fortress_rollback::{Message, NonBlockingSocket};
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Interior state shared by every [`ReorderSocket`] on a mesh and by the
/// test-held [`HeldLinks`] handle. One mutex guards everything so that a
/// `release` (flush + unhold) is atomic with respect to concurrent sends.
#[derive(Default)]
struct ReorderState {
    /// Directed links currently holding outbound messages.
    held_links: HashSet<(SocketAddr, SocketAddr)>,
    /// Held messages per directed link, in original send (FIFO) order.
    held_messages: BTreeMap<(SocketAddr, SocketAddr), VecDeque<Message>>,
    /// Per-destination delivery inboxes shared by every socket on the mesh.
    inboxes: BTreeMap<SocketAddr, VecDeque<(SocketAddr, Message)>>,
}

/// Shared, cloneable handle controlling which directed links are currently
/// holding messages. Cloning shares the same underlying state (the
/// [`BlockedLinks`](super::filter_socket::BlockedLinks)-style pattern).
#[allow(dead_code)]
#[derive(Clone, Default)]
pub struct HeldLinks {
    inner: Arc<Mutex<ReorderState>>,
}

#[allow(dead_code)]
#[allow(clippy::expect_used)] // Test infrastructure — poisoned mutex is a test bug.
impl HeldLinks {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts holding the directed link `src -> dst`: messages sent on it are
    /// queued (never dropped) until [`Self::release`].
    pub fn hold(&self, src: SocketAddr, dst: SocketAddr) {
        self.inner
            .lock()
            .expect("HeldLinks mutex poisoned")
            .held_links
            .insert((src, dst));
    }

    /// Releases the directed link `src -> dst`: flushes every held message
    /// into `dst`'s inbox in original FIFO order, then re-opens the link so
    /// subsequent sends deliver immediately. Flush and unhold are atomic.
    pub fn release(&self, src: SocketAddr, dst: SocketAddr) {
        let mut state = self.inner.lock().expect("HeldLinks mutex poisoned");
        if let Some(mut held) = state.held_messages.remove(&(src, dst)) {
            let inbox = state.inboxes.entry(dst).or_default();
            while let Some(msg) = held.pop_front() {
                inbox.push_back((src, msg));
            }
        }
        state.held_links.remove(&(src, dst));
    }

    /// Number of messages currently held on the directed link `src -> dst`.
    /// Useful for premise assertions (e.g. "the hold actually captured
    /// traffic") in tests.
    #[must_use]
    pub fn held_len(&self, src: SocketAddr, dst: SocketAddr) -> usize {
        self.inner
            .lock()
            .expect("HeldLinks mutex poisoned")
            .held_messages
            .get(&(src, dst))
            .map_or(0, VecDeque::len)
    }
}

/// An in-memory [`NonBlockingSocket`] whose outbound messages can be held and
/// later released per directed link via a shared [`HeldLinks`] handle. See the
/// module docs for semantics.
#[allow(dead_code)]
pub struct ReorderSocket {
    local_addr: SocketAddr,
    links: HeldLinks,
}

#[allow(clippy::expect_used)] // Test infrastructure — poisoned mutex is a test bug.
impl NonBlockingSocket<SocketAddr> for ReorderSocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        let mut state = self.links.inner.lock().expect("HeldLinks mutex poisoned");
        if state.held_links.contains(&(self.local_addr, *addr)) {
            state
                .held_messages
                .entry((self.local_addr, *addr))
                .or_default()
                .push_back(msg.clone());
        } else {
            state
                .inboxes
                .entry(*addr)
                .or_default()
                .push_back((self.local_addr, msg.clone()));
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        let mut state = self.links.inner.lock().expect("HeldLinks mutex poisoned");
        match state.inboxes.get_mut(&self.local_addr) {
            Some(queue) => queue.drain(..).collect(),
            None => Vec::new(),
        }
    }
}

/// Builds three [`ReorderSocket`]s on a shared mesh (one per address) plus the
/// shared [`HeldLinks`] handle the test uses to hold/release directed links
/// mid-run. All links start open (immediate delivery).
#[allow(dead_code)]
#[must_use]
pub fn create_reorder_mesh_triple(addrs: [SocketAddr; 3]) -> ([ReorderSocket; 3], HeldLinks) {
    let links = HeldLinks::new();
    let mk = |i: usize| ReorderSocket {
        local_addr: addrs[i],
        links: links.clone(),
    };
    ([mk(0), mk(1), mk(2)], links)
}
