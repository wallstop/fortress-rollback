//! Deterministic directional-loss socket wrapper for testing asymmetric packet
//! loss.
//!
//! [`FilterSocket`] wraps a [`ChannelSocket`] and drops outbound
//! [`send_to()`](NonBlockingSocket::send_to) calls whose `(source, destination)`
//! link is currently in a shared blocked-set. This lets a test simulate
//! *directional* loss on a single link (e.g. "P3 -> P2 is down, but P3 -> P1 is
//! fine") deterministically, without timing or randomness.
//!
//! The blocked-set is shared via `Arc<Mutex<HashSet<(SocketAddr, SocketAddr)>>>`
//! so a test can toggle which links are blocked mid-run (after sockets have been
//! moved into their sessions). `Arc<Mutex<..>>` (rather than `Rc<RefCell<..>>`)
//! is required so the socket stays `Send + Sync` under the `sync-send` feature,
//! the same bound [`NonBlockingSocket`] imposes on `ChannelSocket`.
//!
//! Each [`FilterSocket`] knows its own local address, so blocking is keyed on the
//! ordered `(source, destination)` pair — blocking `P3 -> P2` does not affect
//! `P1 -> P2`. Receiving is never filtered: a blocked send simply never reaches
//! the peer, so the peer's queue for that link stops advancing — exactly
//! mirroring real packet loss from a now-quiet endpoint.

use super::channel_socket::ChannelSocket;
use fortress_rollback::{Message, NonBlockingSocket};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Shared, cloneable handle to the set of currently-blocked directional links,
/// keyed by `(source, destination)`. Cloning shares the same underlying set.
#[allow(dead_code)]
#[derive(Clone, Default)]
pub struct BlockedLinks {
    inner: Arc<Mutex<HashSet<(SocketAddr, SocketAddr)>>>,
}

#[allow(dead_code)]
#[allow(clippy::expect_used)] // Test infrastructure — poisoned mutex is a test bug.
impl BlockedLinks {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Blocks the directional link `src -> dst` until [`Self::unblock`].
    pub fn block(&self, src: SocketAddr, dst: SocketAddr) {
        self.inner
            .lock()
            .expect("BlockedLinks mutex poisoned")
            .insert((src, dst));
    }

    /// Re-enables the directional link `src -> dst`.
    pub fn unblock(&self, src: SocketAddr, dst: SocketAddr) {
        self.inner
            .lock()
            .expect("BlockedLinks mutex poisoned")
            .remove(&(src, dst));
    }

    /// Returns whether the directional link `src -> dst` is currently blocked.
    #[must_use]
    fn is_blocked(&self, src: SocketAddr, dst: SocketAddr) -> bool {
        self.inner
            .lock()
            .expect("BlockedLinks mutex poisoned")
            .contains(&(src, dst))
    }
}

/// A [`ChannelSocket`] wrapper that drops sends on currently-blocked directional
/// links, driven by a shared [`BlockedLinks`] handle.
#[allow(dead_code)]
pub struct FilterSocket {
    inner: ChannelSocket,
    local_addr: SocketAddr,
    blocked: BlockedLinks,
}

#[allow(dead_code)]
impl FilterSocket {
    #[must_use]
    pub fn new(inner: ChannelSocket, blocked: BlockedLinks) -> Self {
        let local_addr = inner.local_addr();
        Self {
            inner,
            local_addr,
            blocked,
        }
    }
}

impl NonBlockingSocket<SocketAddr> for FilterSocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        // Directional drop: if this socket's link to `addr` is currently
        // blocked, the packet is silently lost (no delivery), exactly like real
        // packet loss.
        if self.blocked.is_blocked(self.local_addr, *addr) {
            return;
        }
        self.inner.send_to(msg, addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        self.inner.receive_all_messages()
    }
}

/// Creates a connected triple of [`FilterSocket`]-wrapped [`ChannelSocket`]s for
/// 3-player asymmetric-loss testing, all sharing one [`BlockedLinks`] handle.
///
/// Returns the three sockets, their addresses, and the shared [`BlockedLinks`]
/// handle the test uses to toggle directional loss mid-run.
#[allow(dead_code)]
#[allow(clippy::type_complexity)]
#[must_use]
pub fn create_filtered_channel_triple() -> (
    FilterSocket,
    FilterSocket,
    FilterSocket,
    SocketAddr,
    SocketAddr,
    SocketAddr,
    BlockedLinks,
) {
    let (s1, s2, s3, a1, a2, a3) = super::channel_socket::create_channel_triple();
    let blocked = BlockedLinks::new();
    (
        FilterSocket::new(s1, blocked.clone()),
        FilterSocket::new(s2, blocked.clone()),
        FilterSocket::new(s3, blocked.clone()),
        a1,
        a2,
        a3,
        blocked,
    )
}
