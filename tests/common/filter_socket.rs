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

    /// Public alias of [`Self::is_blocked`] for test-defined socket wrappers that
    /// live outside this module (e.g. the F9 spectator-convergence repro's
    /// `FilterBusSocket`, which needs a 4-node mesh `FilterSocket`/`ChannelSocket`
    /// cannot express).
    #[must_use]
    pub fn is_blocked_pub(&self, src: SocketAddr, dst: SocketAddr) -> bool {
        self.is_blocked(src, dst)
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

/// Creates a connected quad of [`FilterSocket`]-wrapped [`ChannelSocket`]s for
/// 4-player asymmetric-loss testing, all sharing one [`BlockedLinks`] handle.
///
/// This is the 4-node analog of [`create_filtered_channel_triple`]. It is needed
/// to reproduce *relay*-clobber disconnect-gossip desyncs (audit F4), which
/// require ≥3 survivors: a third survivor's lower view of a dropped slot can only
/// reach a higher-view survivor by transiting a relay peer when their direct link
/// is also lost — a topology a 3-node mesh (one dropped + two survivors, each
/// joined by a direct link that always carries truth) cannot express.
///
/// Returns the four sockets, their addresses, and the shared [`BlockedLinks`]
/// handle the test uses to toggle directional loss mid-run.
#[allow(dead_code)]
#[allow(clippy::type_complexity)]
#[must_use]
pub fn create_filtered_channel_quad() -> (
    FilterSocket,
    FilterSocket,
    FilterSocket,
    FilterSocket,
    SocketAddr,
    SocketAddr,
    SocketAddr,
    SocketAddr,
    BlockedLinks,
) {
    let (s1, s2, s3, s4, a1, a2, a3, a4) = super::channel_socket::create_channel_quad();
    let blocked = BlockedLinks::new();
    (
        FilterSocket::new(s1, blocked.clone()),
        FilterSocket::new(s2, blocked.clone()),
        FilterSocket::new(s3, blocked.clone()),
        FilterSocket::new(s4, blocked.clone()),
        a1,
        a2,
        a3,
        a4,
        blocked,
    )
}

/// Creates a fully-meshed set of `n` [`FilterSocket`]-wrapped [`ChannelSocket`]s
/// for N-player asymmetric-loss testing, all sharing one [`BlockedLinks`] handle.
///
/// This is the arbitrary-`n` generalization of [`create_filtered_channel_triple`]
/// / [`create_filtered_channel_quad`], built on the generic
/// [`create_channel_mesh`](super::channel_socket::create_channel_mesh). It returns
/// `Vec`s (rather than a fixed-arity tuple) because at `n >= 5` a positional tuple
/// becomes unwieldy and error-prone — index `sockets[i]` / `addrs[i]` instead.
///
/// It is needed for **N≥5** desync coverage that a 4-node mesh cannot express.
/// The first consumer is the genuine multi-relay *double-failure relay* repro,
/// where a receipt-bound lowering is folded as a `min` over **two** relay
/// survivors (not the single relay an N=4 mesh affords), so the multi-relay
/// fresh-ack round (S55) is exercised as a genuine many-element conjunction rather
/// than a degenerate one-relay echo. (A future live-wire repro of the hot-join
/// *fold-below-`S`* corner — which the S35 audit note flags as also needing N≥5 —
/// can reuse this same harness.)
///
/// Returns the `n` sockets, their `n` addresses (index-aligned), and the shared
/// [`BlockedLinks`] handle the test uses to toggle directional loss mid-run.
///
/// # Panics
///
/// Panics if `n` is outside `2..=1000` — this delegates to
/// [`create_channel_mesh`](super::channel_socket::create_channel_mesh), which
/// asserts that bound (both `n < 2` and `n > 1000` panic).
#[allow(dead_code)]
#[must_use]
pub fn create_filtered_channel_mesh(
    n: usize,
) -> (Vec<FilterSocket>, Vec<SocketAddr>, BlockedLinks) {
    let (sockets, addrs) = super::channel_socket::create_channel_mesh(n);
    let blocked = BlockedLinks::new();
    let filtered = sockets
        .into_iter()
        .map(|s| FilterSocket::new(s, blocked.clone()))
        .collect();
    (filtered, addrs, blocked)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn create_filtered_channel_mesh_sizes_and_shared_blocked_handle() {
        for n in [2_usize, 3, 4, 5, 8] {
            let (sockets, addrs, blocked) = create_filtered_channel_mesh(n);
            assert_eq!(sockets.len(), n, "mesh must have n filtered sockets");
            assert_eq!(addrs.len(), n, "mesh must have n addresses");
            let unique: std::collections::HashSet<_> = addrs.iter().collect();
            assert_eq!(unique.len(), n, "mesh addresses must be distinct (n={n})");
            // The returned handle and every socket's handle alias the SAME set:
            // a block toggled through the returned handle is observed by a socket.
            assert!(!blocked.is_blocked(addrs[0], addrs[1]));
            blocked.block(addrs[0], addrs[1]);
            assert!(
                blocked.is_blocked(addrs[0], addrs[1]),
                "the returned BlockedLinks handle must toggle the shared set (n={n})"
            );
            // Directional: the reverse link is unaffected.
            assert!(!blocked.is_blocked(addrs[1], addrs[0]));
        }
    }

    #[test]
    #[should_panic(expected = "at least 2")]
    fn create_filtered_channel_mesh_rejects_too_few_peers() {
        let _ = create_filtered_channel_mesh(1);
    }
}
