//! A shared in-memory routing-bus socket for deterministic multi-attach testing.
//!
//! [`ChannelSocket`](super::channel_socket::ChannelSocket) moves its mpsc
//! `Receiver` into the single session that owns it, so a *second* session can
//! never attach at an address a first session already used. That makes it
//! impossible to model a peer that drops and then **re-joins at the same
//! address** (the hot-join "graceful-drop rejoin" scenario).
//!
//! [`RoutingBus`] solves this by keeping all per-address inboxes in one shared
//! `Arc<Mutex<HashMap<...>>>`. Any number of [`BusSocket`]s can attach at the
//! same [`SocketAddr`] *over time*: when a socket is dropped and a fresh one is
//! created at the same address (sharing the same bus), the new socket reads from
//! the same inbox. Messages addressed to an address with no live socket are still
//! buffered, so a host can keep sending to a vacated slot and a returning joiner
//! attached later still receives the backlog — exactly what a real UDP endpoint
//! would observe.
//!
//! Delivery is instant and fully deterministic (no real I/O, no timing). The
//! `Mutex` provides the `Sync` bound required by
//! [`NonBlockingSocket`](fortress_rollback::NonBlockingSocket) under the
//! `sync-send` feature, mirroring `ChannelSocket`.
//!
//! # Usage
//!
//! ```ignore
//! use common::bus_socket::RoutingBus;
//!
//! let bus = RoutingBus::new();
//! let host_addr = ([127, 0, 0, 1], 20001).into();
//! let joiner_addr = ([127, 0, 0, 1], 20002).into();
//!
//! let host_socket = bus.socket(host_addr);
//! let joiner_socket = bus.socket(joiner_addr);
//! // ... later, after the first joiner drops, attach a fresh one at the SAME addr:
//! let rejoiner_socket = bus.socket(joiner_addr);
//! ```

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use fortress_rollback::{Message, NonBlockingSocket};

/// Per-address inbox map shared by every [`BusSocket`] attached to a bus.
type Inboxes = HashMap<SocketAddr, VecDeque<(SocketAddr, Message)>>;

/// A shared in-memory message bus that routes [`Message`]s between
/// [`BusSocket`]s by destination [`SocketAddr`].
///
/// Cloning a `RoutingBus` clones the `Arc`, so all clones share the same
/// underlying inboxes. This is what lets a fresh [`BusSocket`] re-attach at an
/// address a previous socket used.
#[derive(Clone)]
#[allow(dead_code)] // Used only by the hot-join rejoin integration tests.
pub struct RoutingBus {
    inboxes: Arc<Mutex<Inboxes>>,
}

#[allow(dead_code)] // Used only by the hot-join rejoin integration tests.
#[allow(clippy::expect_used)] // Test infrastructure — a poisoned mutex is a test bug.
impl RoutingBus {
    /// Creates a new, empty routing bus.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inboxes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Creates a fresh [`BusSocket`] bound to `addr` that shares this bus.
    ///
    /// Multiple sockets may be created at the same `addr` over the lifetime of
    /// the bus (one at a time, in practice): the new socket reads from and writes
    /// to the same shared inboxes, so a returning peer that re-attaches at a
    /// vacated address picks up where the previous socket left off.
    #[must_use]
    pub fn socket(&self, addr: SocketAddr) -> BusSocket {
        BusSocket {
            local_addr: addr,
            bus: self.clone(),
        }
    }
}

impl Default for RoutingBus {
    fn default() -> Self {
        Self::new()
    }
}

/// An in-memory socket attached to a [`RoutingBus`] at a single [`SocketAddr`].
///
/// `send_to(msg, dest)` enqueues `(local_addr, msg)` into `dest`'s shared inbox;
/// `receive_all_messages()` drains this socket's own inbox. Messages sent to an
/// address with no live socket are buffered until a socket attaches there.
#[allow(dead_code)] // Used only by the hot-join rejoin integration tests.
pub struct BusSocket {
    local_addr: SocketAddr,
    bus: RoutingBus,
}

#[allow(dead_code)] // Used only by the hot-join rejoin integration tests.
#[allow(clippy::expect_used)] // Test infrastructure — a poisoned mutex is a test bug.
impl BusSocket {
    /// Returns the local address this socket is bound to.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Attaches a fresh socket at `addr` sharing `bus` (alias for
    /// [`RoutingBus::socket`], provided for call-site readability).
    #[must_use]
    pub fn attach(bus: &RoutingBus, addr: SocketAddr) -> Self {
        bus.socket(addr)
    }
}

#[allow(clippy::expect_used)] // Test infrastructure — a poisoned mutex is a test bug.
impl NonBlockingSocket<SocketAddr> for BusSocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        let mut inboxes = self.bus.inboxes.lock().expect("RoutingBus mutex poisoned");
        // Buffer into the destination's inbox even if no socket is attached there
        // yet — a peer that joins later still receives the backlog.
        inboxes
            .entry(*addr)
            .or_default()
            .push_back((self.local_addr, msg.clone()));
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        let mut inboxes = self.bus.inboxes.lock().expect("RoutingBus mutex poisoned");
        match inboxes.get_mut(&self.local_addr) {
            Some(queue) => queue.drain(..).collect(),
            None => Vec::new(),
        }
    }
}
