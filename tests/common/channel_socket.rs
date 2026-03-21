//! In-memory socket implementation for deterministic testing.
//!
//! [`ChannelSocket`] implements [`NonBlockingSocket`] using `std::sync::mpsc` channels
//! for instant, deterministic message delivery. This eliminates all real UDP I/O from
//! tests, removing sources of non-determinism including:
//!
//! - Port conflicts (WSAEACCES, WSAEADDRINUSE on Windows)
//! - Packet delivery timing dependencies
//! - Platform-specific UDP behavior differences
//! - Need for `#[serial]` test attributes
//!
//! # Usage
//!
//! ```ignore
//! use common::channel_socket::create_channel_pair;
//!
//! let (socket1, socket2, addr1, addr2) = create_channel_pair();
//! // socket1 sends to addr2, socket2 sends to addr1
//! // Messages are instantly available via receive_all_messages()
//! ```

use fortress_rollback::{Message, NonBlockingSocket};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;

/// An in-memory socket for deterministic testing.
///
/// Messages sent via [`send_to()`](NonBlockingSocket::send_to) are immediately
/// available in the receiver's [`receive_all_messages()`](NonBlockingSocket::receive_all_messages).
/// No real UDP I/O, no timing dependency, no port conflicts.
///
/// The `Receiver` is wrapped in a [`Mutex`] to satisfy the `Sync` bound required
/// by [`NonBlockingSocket`] when the `sync-send` feature is enabled. Since
/// `ChannelSocket` is test infrastructure, the `Mutex` overhead is negligible.
///
/// # Type Parameters
///
/// Uses `SocketAddr` as the address type to match the common test setup pattern
/// where `Config::Address = SocketAddr`.
pub struct ChannelSocket {
    local_addr: SocketAddr,
    /// Senders to peer sockets, keyed by peer address.
    senders: HashMap<SocketAddr, Sender<(SocketAddr, Message)>>,
    /// Receiver for incoming messages from all peers.
    /// Wrapped in `Mutex` to provide `Sync` for the `sync-send` feature flag.
    receiver: Mutex<mpsc::Receiver<(SocketAddr, Message)>>,
}

// SAFETY: ChannelSocket is Send because all fields are Send.
// Sync is provided by the Mutex wrapper around Receiver.
// This satisfies the NonBlockingSocket<A>: Send + Sync bound
// required when the `sync-send` feature is enabled.

#[allow(clippy::expect_used)] // Test infrastructure — poisoned mutex is a test bug.
impl ChannelSocket {
    /// Returns the local address of this socket.
    ///
    /// This address is synthetic (not bound to a real port) and is used
    /// as the source address when sending messages to peers.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

#[allow(clippy::expect_used)] // Test infrastructure — poisoned mutex is a test bug.
impl NonBlockingSocket<SocketAddr> for ChannelSocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        if let Some(sender) = self.senders.get(addr) {
            // Clone message and send. Ignore errors (peer may have dropped).
            let _ = sender.send((self.local_addr, msg.clone()));
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        let receiver = self.receiver.lock().expect("ChannelSocket mutex poisoned");
        let mut messages = Vec::new();
        while let Ok(msg) = receiver.try_recv() {
            messages.push(msg);
        }
        messages
    }
}

/// Creates a single unconnected in-memory socket at a synthetic address.
///
/// Useful for tests that build a session but never actually communicate
/// (e.g., testing session construction, disconnect, handle queries).
/// The socket has no senders and a receiver that will never receive messages.
///
/// # Arguments
///
/// * `port` - The port number for the synthetic `127.0.0.1` address
///
/// # Returns
///
/// `(socket, addr)` where `socket` is an unconnected `ChannelSocket` and
/// `addr` is `127.0.0.1:port`.
#[allow(dead_code)]
#[must_use]
pub fn create_unconnected_socket(port: u16) -> (ChannelSocket, SocketAddr) {
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let (_tx, rx) = mpsc::channel();
    let socket = ChannelSocket {
        local_addr: addr,
        senders: HashMap::new(),
        receiver: Mutex::new(rx),
    };
    (socket, addr)
}

/// Creates a connected pair of in-memory sockets for 2-player P2P testing.
///
/// Each socket can send messages to the other via its address. Messages are
/// delivered instantly through channels — no real network I/O occurs.
///
/// # Returns
///
/// `(socket1, socket2, addr1, addr2)` where:
/// - `socket1` is at `addr1` and can send to `addr2`
/// - `socket2` is at `addr2` and can send to `addr1`
///
/// # Example
///
/// ```ignore
/// use common::channel_socket::create_channel_pair;
/// use fortress_rollback::SessionBuilder;
///
/// let (socket1, socket2, addr1, addr2) = create_channel_pair();
///
/// let sess1 = SessionBuilder::<MyConfig>::new()
///     .add_player(PlayerType::Local, PlayerHandle::new(0))?
///     .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
///     .start_p2p_session(socket1)?;
///
/// let sess2 = SessionBuilder::<MyConfig>::new()
///     .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
///     .add_player(PlayerType::Local, PlayerHandle::new(1))?
///     .start_p2p_session(socket2)?;
/// ```
#[allow(dead_code)]
#[must_use]
pub fn create_channel_pair() -> (ChannelSocket, ChannelSocket, SocketAddr, SocketAddr) {
    // Use synthetic addresses — these are never bound to real ports.
    let addr1: SocketAddr = ([127, 0, 0, 1], 10001).into();
    let addr2: SocketAddr = ([127, 0, 0, 1], 10002).into();

    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    let socket1 = ChannelSocket {
        local_addr: addr1,
        senders: std::iter::once((addr2, tx2)).collect(),
        receiver: Mutex::new(rx1),
    };

    let socket2 = ChannelSocket {
        local_addr: addr2,
        senders: std::iter::once((addr1, tx1)).collect(),
        receiver: Mutex::new(rx2),
    };

    (socket1, socket2, addr1, addr2)
}

/// Creates a connected triple of in-memory sockets for 3-player P2P testing.
///
/// Each socket can send messages to both other sockets. Messages are
/// delivered instantly through channels — no real network I/O occurs.
///
/// # Returns
///
/// `(socket1, socket2, socket3, addr1, addr2, addr3)` where each socket
/// can send to the other two.
#[allow(dead_code)]
#[must_use]
pub fn create_channel_triple() -> (
    ChannelSocket,
    ChannelSocket,
    ChannelSocket,
    SocketAddr,
    SocketAddr,
    SocketAddr,
) {
    let addr1: SocketAddr = ([127, 0, 0, 1], 10001).into();
    let addr2: SocketAddr = ([127, 0, 0, 1], 10002).into();
    let addr3: SocketAddr = ([127, 0, 0, 1], 10003).into();

    // Each socket has one receiver; multiple senders write to it.
    let (tx1a, rx1) = mpsc::channel();
    let tx1b = tx1a.clone();
    let (tx2a, rx2) = mpsc::channel();
    let tx2b = tx2a.clone();
    let (tx3a, rx3) = mpsc::channel();
    let tx3b = tx3a.clone();

    let socket1 = ChannelSocket {
        local_addr: addr1,
        senders: [(addr2, tx2a), (addr3, tx3a)].into_iter().collect(),
        receiver: Mutex::new(rx1),
    };

    let socket2 = ChannelSocket {
        local_addr: addr2,
        senders: [(addr1, tx1a), (addr3, tx3b)].into_iter().collect(),
        receiver: Mutex::new(rx2),
    };

    let socket3 = ChannelSocket {
        local_addr: addr3,
        senders: [(addr1, tx1b), (addr2, tx2b)].into_iter().collect(),
        receiver: Mutex::new(rx3),
    };

    (socket1, socket2, socket3, addr1, addr2, addr3)
}

/// Creates a connected quad of in-memory sockets for 4-player P2P testing.
///
/// Each socket can send messages to all three other sockets. Messages are
/// delivered instantly through channels — no real network I/O occurs.
///
/// # Returns
///
/// `(socket1, socket2, socket3, socket4, addr1, addr2, addr3, addr4)` where each socket
/// can send to the other three.
#[allow(dead_code)]
#[must_use]
pub fn create_channel_quad() -> (
    ChannelSocket,
    ChannelSocket,
    ChannelSocket,
    ChannelSocket,
    SocketAddr,
    SocketAddr,
    SocketAddr,
    SocketAddr,
) {
    let addr1: SocketAddr = ([127, 0, 0, 1], 10001).into();
    let addr2: SocketAddr = ([127, 0, 0, 1], 10002).into();
    let addr3: SocketAddr = ([127, 0, 0, 1], 10003).into();
    let addr4: SocketAddr = ([127, 0, 0, 1], 10004).into();

    // Each socket has one receiver; multiple senders write to it.
    let (tx1a, rx1) = mpsc::channel();
    let tx1b = tx1a.clone();
    let tx1c = tx1a.clone();

    let (tx2a, rx2) = mpsc::channel();
    let tx2b = tx2a.clone();
    let tx2c = tx2a.clone();

    let (tx3a, rx3) = mpsc::channel();
    let tx3b = tx3a.clone();
    let tx3c = tx3a.clone();

    let (tx4a, rx4) = mpsc::channel();
    let tx4b = tx4a.clone();
    let tx4c = tx4a.clone();

    let socket1 = ChannelSocket {
        local_addr: addr1,
        senders: [(addr2, tx2a), (addr3, tx3a), (addr4, tx4a)]
            .into_iter()
            .collect(),
        receiver: Mutex::new(rx1),
    };

    let socket2 = ChannelSocket {
        local_addr: addr2,
        senders: [(addr1, tx1a), (addr3, tx3b), (addr4, tx4b)]
            .into_iter()
            .collect(),
        receiver: Mutex::new(rx2),
    };

    let socket3 = ChannelSocket {
        local_addr: addr3,
        senders: [(addr1, tx1b), (addr2, tx2b), (addr4, tx4c)]
            .into_iter()
            .collect(),
        receiver: Mutex::new(rx3),
    };

    let socket4 = ChannelSocket {
        local_addr: addr4,
        senders: [(addr1, tx1c), (addr2, tx2c), (addr3, tx3c)]
            .into_iter()
            .collect(),
        receiver: Mutex::new(rx4),
    };

    (
        socket1, socket2, socket3, socket4, addr1, addr2, addr3, addr4,
    )
}

/// Creates a connected pair of [`ChaosSocket`](fortress_rollback::ChaosSocket)-wrapped
/// `ChannelSocket`s for deterministic network chaos testing.
///
/// This combines in-memory message delivery with configurable network fault injection
/// (latency, loss, jitter, etc.) and a shared virtual clock. The result is fully
/// deterministic chaos testing — no real I/O, no `thread::sleep()`, no flaky tests.
///
/// # Arguments
///
/// * `config1` - Chaos configuration for socket 1's network conditions
/// * `config2` - Chaos configuration for socket 2's network conditions
/// * `clock` - Shared test clock for deterministic time control
///
/// # Returns
///
/// `(chaos_socket1, chaos_socket2, addr1, addr2)`
///
/// # Example
///
/// ```ignore
/// use common::channel_socket::create_chaos_channel_pair;
/// use common::test_clock::TestClock;
/// use fortress_rollback::ChaosConfig;
///
/// let clock = TestClock::new();
/// let config = ChaosConfig::builder()
///     .latency_ms(50)
///     .packet_loss_rate(0.1)
///     .seed(42)
///     .build();
///
/// let (s1, s2, a1, a2) = create_chaos_channel_pair(config.clone(), config, &clock);
/// // Advance virtual time to trigger latency delivery:
/// clock.advance(Duration::from_millis(100));
/// ```
#[allow(dead_code)]
#[must_use]
pub fn create_chaos_channel_pair(
    config1: fortress_rollback::ChaosConfig,
    config2: fortress_rollback::ChaosConfig,
    clock: &super::test_clock::TestClock,
) -> (
    fortress_rollback::ChaosSocket<SocketAddr, ChannelSocket>,
    fortress_rollback::ChaosSocket<SocketAddr, ChannelSocket>,
    SocketAddr,
    SocketAddr,
) {
    use fortress_rollback::ChaosSocket;

    let (s1, s2, a1, a2) = create_channel_pair();
    let chaos1 = ChaosSocket::new(s1, config1).with_clock(clock.as_chaos_clock());
    let chaos2 = ChaosSocket::new(s2, config2).with_clock(clock.as_chaos_clock());
    (chaos1, chaos2, a1, a2)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    //! Tests for `ChannelSocket` and `create_channel_pair`.
    //!
    //! Since `Message` has `pub(crate)` fields, these tests verify channel mechanics
    //! (delivery counts, source addresses, synchronization) without inspecting message
    //! internals. The integration tests use real `SessionBuilder` + `P2PSession` to
    //! prove the infrastructure works end-to-end.

    use super::*;

    #[test]
    fn create_channel_pair_returns_distinct_addresses() {
        let (_s1, _s2, addr1, addr2) = create_channel_pair();
        assert_ne!(addr1, addr2, "Addresses should be distinct");
    }

    #[test]
    fn local_addr_matches_creation() {
        let (s1, s2, addr1, addr2) = create_channel_pair();
        assert_eq!(s1.local_addr(), addr1);
        assert_eq!(s2.local_addr(), addr2);
    }

    #[test]
    fn receive_empty_returns_empty_vec() {
        let (mut s1, _s2, _a1, _a2) = create_channel_pair();
        let received = s1.receive_all_messages();
        assert!(received.is_empty(), "No messages should be available");
    }

    /// Integration test: create P2P sessions over ChannelSockets and verify synchronization.
    ///
    /// This is the primary validation that ChannelSocket works correctly with the
    /// session layer — the actual use case for this infrastructure.
    #[test]
    fn sessions_synchronize_over_channel_sockets() {
        use super::super::stubs::{StubConfig, StubInput};
        use super::super::test_clock::TestClock;
        use fortress_rollback::{
            PlayerHandle, PlayerType, ProtocolConfig, SessionBuilder, SessionState,
        };
        use std::time::Duration;

        let clock = TestClock::new();

        let (s1, s2, a1, a2) = create_channel_pair();

        let protocol_config = ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            ..ProtocolConfig::default()
        };

        let mut sess1 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(protocol_config.clone())
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(s1)
            .unwrap();

        let mut sess2 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(protocol_config)
            .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(s2)
            .unwrap();

        assert_eq!(sess1.current_state(), SessionState::Synchronizing);
        assert_eq!(sess2.current_state(), SessionState::Synchronizing);

        // Synchronize using virtual time — no thread::sleep needed
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

            // Advance virtual time past sync retry interval
            clock.advance(Duration::from_millis(50));
        }

        assert!(
            synchronized,
            "Sessions should synchronize. sess1: {:?}, sess2: {:?}",
            sess1.current_state(),
            sess2.current_state()
        );

        // Advance a frame to verify the sessions actually work
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: 1 })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: 2 })
            .unwrap();

        let requests1 = sess1.advance_frame().unwrap();
        let requests2 = sess2.advance_frame().unwrap();

        // Verify we got requests (the actual request handling is tested elsewhere)
        assert!(
            !requests1.is_empty(),
            "Session 1 should produce frame advance requests"
        );
        assert!(
            !requests2.is_empty(),
            "Session 2 should produce frame advance requests"
        );
    }

    /// Integration test: sessions synchronize over ChaosSocket-wrapped ChannelSockets
    /// with deterministic latency controlled by TestClock.
    #[test]
    fn sessions_synchronize_over_chaos_channel_sockets() {
        use super::super::stubs::StubConfig;
        use super::super::test_clock::TestClock;
        use fortress_rollback::{
            ChaosConfig, PlayerHandle, PlayerType, ProtocolConfig, SessionBuilder, SessionState,
        };
        use std::time::Duration;

        let clock = TestClock::new();
        let chaos_config = ChaosConfig::builder().latency_ms(10).seed(42).build();

        let (cs1, cs2, a1, a2) =
            create_chaos_channel_pair(chaos_config.clone(), chaos_config, &clock);

        let protocol_config = ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            ..ProtocolConfig::default()
        };

        let mut sess1 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(protocol_config.clone())
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(cs1)
            .unwrap();

        let mut sess2 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(protocol_config)
            .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(cs2)
            .unwrap();

        // Synchronize with virtual time — latency is handled by the clock
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
            "Sessions should synchronize over chaos channels. sess1: {:?}, sess2: {:?}",
            sess1.current_state(),
            sess2.current_state()
        );
    }

    /// Verifies that dropping one end of a channel pair doesn't cause panics,
    /// including when attempting to send to the dropped peer.
    #[test]
    fn dropped_peer_send_does_not_panic() {
        use super::super::stubs::StubConfig;
        use super::super::test_clock::TestClock;
        use fortress_rollback::{PlayerHandle, PlayerType, ProtocolConfig, SessionBuilder};
        use std::time::Duration;

        let clock = TestClock::new();
        let (s1, s2, a1, a2) = create_channel_pair();

        let protocol_config = ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            ..ProtocolConfig::default()
        };

        // Build sess1 — this internally calls send_to during poll
        let mut sess1 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(protocol_config)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(s1)
            .unwrap();

        // Drop socket2 (and never build a session with it)
        drop(s2);

        // Polling should not panic even though peer socket is gone.
        // The channel send silently fails via `let _ = sender.send(...)`.
        for _ in 0..10 {
            sess1.poll_remote_clients();
            clock.advance(Duration::from_millis(50));
        }

        // s1 still exists and addr is valid
        assert_eq!(a1, ([127, 0, 0, 1], 10001).into());
    }

    /// Verifies that the chaos channel pair produces distinct addresses.
    #[test]
    fn chaos_channel_pair_addresses_are_distinct() {
        use super::super::test_clock::TestClock;
        use fortress_rollback::ChaosConfig;

        let clock = TestClock::new();
        let config = ChaosConfig::passthrough();
        let (_cs1, _cs2, a1, a2) = create_chaos_channel_pair(config.clone(), config, &clock);
        assert_ne!(a1, a2);
    }
}
