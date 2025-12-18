//! Tokio async runtime adapter for Fortress Rollback.
//!
//! This module provides [`TokioUdpSocket`], an adapter that wraps a Tokio async UDP socket
//! and implements the [`NonBlockingSocket`] trait for use with Fortress Rollback sessions.
//!
//! # Overview
//!
//! Fortress Rollback uses synchronous session APIs (`P2PSession`, `SpectatorSession`) that
//! expect a non-blocking socket. This adapter allows you to use Fortress Rollback within
//! an async Tokio application by:
//!
//! 1. Wrapping a [`tokio::net::UdpSocket`] in a non-blocking adapter
//! 2. Using the async helper methods for efficient I/O waiting
//! 3. Using the synchronous session APIs between async socket polls
//!
//! # Async vs Sync Methods
//!
//! This adapter provides both synchronous ([`NonBlockingSocket`] trait) and async methods:
//!
//! | Sync Method | Async Method | When to Use |
//! |-------------|--------------|-------------|
//! | `send_to()` | [`send_to_async()`](TokioUdpSocket::send_to_async) | Async is preferred for reliability |
//! | `receive_all_messages()` | [`recv_all()`](TokioUdpSocket::recv_all) | Async is preferred for efficiency |
//! | - | [`wait_readable()`](TokioUdpSocket::wait_readable) | Wait for socket readability |
//! | - | [`wait_writable()`](TokioUdpSocket::wait_writable) | Wait for socket writability |
//!
//! **Important**: The synchronous `send_to()` and `receive_all_messages()` methods from the
//! [`NonBlockingSocket`] trait may fail if the socket isn't ready:
//! - `send_to()` will report a violation and drop the packet if the socket would block
//! - `receive_all_messages()` will return an empty vector if no data is available
//!
//! For reliable operation in async contexts, use the async methods or call
//! `wait_readable()`/`wait_writable()` before the sync methods.
//!
//! # Usage Pattern
//!
//! The typical pattern for async game loops is:
//!
//! 1. Use [`recv_all()`](TokioUdpSocket::recv_all) to efficiently wait for and receive messages
//! 2. Process any received messages with the session
//! 3. Advance the game frame and handle requests
//! 4. Send messages using [`send_to_async()`](TokioUdpSocket::send_to_async) or the trait's `send_to()`
//!
//! # Example
//!
//! ```no_run
//! use fortress_rollback::{Config, SessionBuilder, PlayerType, PlayerHandle};
//! use fortress_rollback::tokio_socket::TokioUdpSocket;
//! use std::net::SocketAddr;
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//! struct MyConfig;
//!
//! impl Config for MyConfig {
//!     type Input = u32;
//!     type State = Vec<u8>;
//!     type Address = SocketAddr;
//! }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a Tokio UDP socket
//!     let tokio_socket = tokio::net::UdpSocket::bind("0.0.0.0:7000").await?;
//!
//!     // Wrap it in the adapter
//!     let socket = TokioUdpSocket::new(tokio_socket);
//!
//!     // Use with SessionBuilder as normal
//!     let mut session = SessionBuilder::<MyConfig>::new()
//!         .with_num_players(2)
//!         .add_player(PlayerType::Local, PlayerHandle::new(0))?
//!         .add_player(
//!             PlayerType::Remote("192.168.1.2:7000".parse()?),
//!             PlayerHandle::new(1)
//!         )?
//!         .start_p2p_session(socket)?;
//!
//!     // Game loop with async yielding
//!     loop {
//!         // Process inputs and advance frame...
//!         // session.add_local_input(handle, input)?;
//!         // let requests = session.advance_frame()?;
//!
//!         // Yield to other async tasks
//!         tokio::task::yield_now().await;
//!     }
//! }
//! ```
//!
//! # Performance Considerations
//!
//! - The adapter uses the same buffer sizes as [`UdpNonBlockingSocket`] for consistency
//! - Message serialization uses pre-allocated buffers to minimize allocations
//! - Use [`recv_all()`](TokioUdpSocket::recv_all) for efficient async waiting
//! - The sync `receive_all_messages()` uses `try_recv_from` which may need prior readability polling
//!
//! # Feature Flag
//!
//! This module requires the `tokio` feature flag:
//!
//! ```toml
//! [dependencies]
//! fortress-rollback = { version = "0.1", features = ["tokio"] }
//! ```
//!
//! [`UdpNonBlockingSocket`]: crate::UdpNonBlockingSocket
//! [`NonBlockingSocket`]: crate::NonBlockingSocket

use std::net::SocketAddr;

use tokio::net::UdpSocket;

use crate::network::codec;
use crate::report_violation;
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::{network::messages::Message, NonBlockingSocket};

/// Size of the receive buffer. Same as `UdpNonBlockingSocket` for consistency.
const RECV_BUFFER_SIZE: usize = 4096;

/// Size of the pre-allocated send buffer for serialization.
const SEND_BUFFER_SIZE: usize = 1024;

/// Ideal maximum UDP packet size to avoid fragmentation.
/// Source: <https://stackoverflow.com/a/35697810/775982>
const IDEAL_MAX_UDP_PACKET_SIZE: usize = 508;

/// A Tokio-compatible non-blocking UDP socket adapter for Fortress Rollback.
///
/// This adapter wraps a [`tokio::net::UdpSocket`] and implements the [`NonBlockingSocket`]
/// trait, allowing Fortress Rollback sessions to be used within async Tokio applications.
///
/// # Construction
///
/// Create a `TokioUdpSocket` by binding a Tokio UDP socket and wrapping it:
///
/// ```no_run
/// use fortress_rollback::tokio_socket::TokioUdpSocket;
///
/// # async fn example() -> std::io::Result<()> {
/// let tokio_socket = tokio::net::UdpSocket::bind("0.0.0.0:7000").await?;
/// let socket = TokioUdpSocket::new(tokio_socket);
/// # Ok(())
/// # }
/// ```
///
/// # Thread Safety
///
/// When the `sync-send` feature is enabled, `TokioUdpSocket` implements `Send + Sync`,
/// making it safe to share across threads and use in multi-threaded async runtimes.
///
/// # Buffer Management
///
/// The socket maintains internal buffers for both sending and receiving to minimize
/// allocations in the hot path:
/// - **Receive buffer**: 4KB, reused across `receive_all_messages` calls
/// - **Send buffer**: 1KB, reused across `send_to` calls for serialization
#[derive(Debug)]
pub struct TokioUdpSocket {
    socket: UdpSocket,
    /// Receive buffer - reused across recv_from calls
    recv_buffer: [u8; RECV_BUFFER_SIZE],
    /// Send buffer - reused across send_to calls to avoid allocation
    send_buffer: [u8; SEND_BUFFER_SIZE],
}

impl TokioUdpSocket {
    /// Creates a new `TokioUdpSocket` wrapping the provided Tokio UDP socket.
    ///
    /// # Arguments
    ///
    /// * `socket` - A bound [`tokio::net::UdpSocket`] ready for communication
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fortress_rollback::tokio_socket::TokioUdpSocket;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let tokio_socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    /// let socket = TokioUdpSocket::new(tokio_socket);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket,
            recv_buffer: [0; RECV_BUFFER_SIZE],
            send_buffer: [0; SEND_BUFFER_SIZE],
        }
    }

    /// Binds a new `TokioUdpSocket` to the specified port on all interfaces (0.0.0.0).
    ///
    /// This is a convenience method equivalent to:
    /// ```ignore
    /// let socket = tokio::net::UdpSocket::bind(("0.0.0.0", port)).await?;
    /// TokioUdpSocket::new(socket)
    /// ```
    ///
    /// # Arguments
    ///
    /// * `port` - The port number to bind to. Use 0 to let the OS assign an available port.
    ///
    /// # Errors
    ///
    /// Returns an error if the socket cannot be bound (e.g., port already in use).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fortress_rollback::tokio_socket::TokioUdpSocket;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// // Bind to a specific port
    /// let socket = TokioUdpSocket::bind_to_port(7000).await?;
    ///
    /// // Or let the OS choose a port
    /// let socket = TokioUdpSocket::bind_to_port(0).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bind_to_port(port: u16) -> Result<Self, std::io::Error> {
        let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), port);
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self::new(socket))
    }

    /// Returns the local address that this socket is bound to.
    ///
    /// # Errors
    ///
    /// Returns an error if the local address cannot be determined.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fortress_rollback::tokio_socket::TokioUdpSocket;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let socket = TokioUdpSocket::bind_to_port(0).await?;
    /// let local_addr = socket.local_addr()?;
    /// println!("Bound to: {}", local_addr);
    /// # Ok(())
    /// # }
    /// ```
    pub fn local_addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.socket.local_addr()
    }

    /// Returns a reference to the underlying Tokio UDP socket.
    ///
    /// This can be useful for advanced use cases like setting socket options
    /// or using Tokio-specific async methods directly.
    #[must_use]
    pub fn inner(&self) -> &UdpSocket {
        &self.socket
    }

    /// Waits until the socket is readable, then receives all available messages.
    ///
    /// This is the recommended way to receive messages in an async context.
    /// It efficiently yields to the runtime while waiting for messages to arrive,
    /// then returns all available messages in a single batch.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fortress_rollback::tokio_socket::TokioUdpSocket;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let mut socket = TokioUdpSocket::bind_to_port(7000).await?;
    ///
    /// // Wait for and receive messages
    /// let messages = socket.recv_all().await;
    /// for (from, msg) in messages {
    ///     println!("Received message from {}", from);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn recv_all(&mut self) -> Vec<(SocketAddr, Message)> {
        // Wait for the socket to become readable
        if self.socket.readable().await.is_err() {
            return Vec::new();
        }

        // Now try_recv_from will work without immediately returning WouldBlock
        self.receive_all_messages()
    }

    /// Waits until the socket is readable.
    ///
    /// After this returns successfully, [`NonBlockingSocket::receive_all_messages`]
    /// will be able to receive any pending messages without returning an empty vector
    /// due to the socket not being ready yet.
    ///
    /// This method is useful when you want more control over the receive loop,
    /// or when integrating with other async operations via `select!`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fortress_rollback::tokio_socket::TokioUdpSocket;
    /// use fortress_rollback::NonBlockingSocket;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let mut socket = TokioUdpSocket::bind_to_port(7000).await?;
    ///
    /// loop {
    ///     // Wait for socket to be readable
    ///     socket.wait_readable().await?;
    ///
    ///     // Now receive all pending messages
    ///     let messages = socket.receive_all_messages();
    ///     // Process messages...
    /// }
    /// # }
    /// ```
    pub async fn wait_readable(&self) -> Result<(), std::io::Error> {
        self.socket.readable().await
    }

    /// Waits until the socket is writable.
    ///
    /// After this returns successfully, [`NonBlockingSocket::send_to`]
    /// will be able to send messages without returning `WouldBlock`.
    ///
    /// This method is useful when you want to ensure sends succeed in
    /// your async game loop, especially when sending many messages.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fortress_rollback::tokio_socket::TokioUdpSocket;
    /// use fortress_rollback::NonBlockingSocket;
    /// use fortress_rollback::network::messages::{Message, MessageBody, MessageHeader};
    /// use std::net::SocketAddr;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let mut socket = TokioUdpSocket::bind_to_port(7000).await?;
    /// let target: SocketAddr = "192.168.1.2:7000".parse().unwrap();
    /// let msg = Message {
    ///     header: MessageHeader { magic: 0x1234 },
    ///     body: MessageBody::KeepAlive,
    /// };
    ///
    /// // Wait for socket to be writable before sending
    /// socket.wait_writable().await?;
    /// socket.send_to(&msg, &target);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_writable(&self) -> Result<(), std::io::Error> {
        self.socket.writable().await
    }

    /// Asynchronously sends a message to the specified address.
    ///
    /// This method waits for the socket to be writable before sending,
    /// making it suitable for use in async contexts where you want to
    /// ensure the send completes without `WouldBlock` errors.
    ///
    /// Note: This method is primarily used internally by the session APIs.
    /// Users typically don't need to call this directly.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use fortress_rollback::tokio_socket::TokioUdpSocket;
    /// use std::net::SocketAddr;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let mut socket = TokioUdpSocket::bind_to_port(7000).await?;
    /// let target: SocketAddr = "192.168.1.2:7000".parse().unwrap();
    ///
    /// // Messages are typically sent through the session API
    /// // socket.send_to_async(&msg, &target).await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_to_async(&mut self, msg: &Message, addr: &SocketAddr) {
        // Serialize into the pre-allocated send buffer to avoid allocation.
        let buf = match codec::encode_into(msg, &mut self.send_buffer) {
            Ok(len) => &self.send_buffer[..len],
            Err(codec::CodecError::BufferTooSmall { provided, .. }) => {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Message too large for send buffer ({} bytes), falling back to allocation.",
                    provided
                );
                match codec::encode(msg) {
                    Ok(buf) => {
                        self.send_encoded_packet_async(&buf, addr).await;
                        return;
                    },
                    Err(e) => {
                        report_violation!(
                            ViolationSeverity::Error,
                            ViolationKind::NetworkProtocol,
                            "Failed to serialize message: {}",
                            e
                        );
                        return;
                    },
                }
            },
            Err(e) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Failed to serialize message: {}",
                    e
                );
                return;
            },
        };

        self.send_encoded_packet_async(buf, addr).await;
    }

    /// Asynchronously sends an already-encoded packet to the given address.
    async fn send_encoded_packet_async(&self, buf: &[u8], addr: &SocketAddr) {
        if buf.len() > IDEAL_MAX_UDP_PACKET_SIZE {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Sending UDP packet of size {} bytes, which is larger than ideal ({})",
                buf.len(),
                IDEAL_MAX_UDP_PACKET_SIZE
            );
        }

        // Use async send_to which waits for the socket to be writable
        if let Err(e) = self.socket.send_to(buf, *addr).await {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Failed to send UDP packet to {}: {}",
                addr,
                e
            );
        }
    }

    /// Sends an already-encoded packet to the given address.
    ///
    /// # Important
    ///
    /// This uses `try_send_to` which may fail with `WouldBlock` if the socket
    /// isn't ready. In async contexts, call [`wait_writable()`](Self::wait_writable)
    /// first or use [`send_to_async()`](Self::send_to_async) instead.
    fn send_encoded_packet(&self, buf: &[u8], addr: &SocketAddr) {
        if buf.len() > IDEAL_MAX_UDP_PACKET_SIZE {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Sending UDP packet of size {} bytes, which is larger than ideal ({})",
                buf.len(),
                IDEAL_MAX_UDP_PACKET_SIZE
            );
        }

        // Use try_send_to for non-blocking send
        match self.socket.try_send_to(buf, *addr) {
            Ok(_) => {},
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Socket not ready - this is expected in non-blocking mode.
                // Report as warning since the packet will be dropped.
                // Users should call wait_writable() first or use send_to_async().
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Socket not ready (WouldBlock) when sending to {}. \
                     Packet dropped. Consider using wait_writable() or send_to_async().",
                    addr
                );
            },
            Err(e) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Failed to send UDP packet to {}: {}",
                    addr,
                    e
                );
            },
        }
    }
}

impl NonBlockingSocket<SocketAddr> for TokioUdpSocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        // Serialize into the pre-allocated send buffer to avoid allocation.
        let len = match codec::encode_into(msg, &mut self.send_buffer) {
            Ok(len) => len,
            Err(codec::CodecError::BufferTooSmall { provided, .. }) => {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Message too large for send buffer ({} bytes), falling back to allocation. Consider reducing input struct size.",
                    provided
                );
                // Fall back to allocating encode
                match codec::encode(msg) {
                    Ok(buf) => {
                        self.send_encoded_packet(&buf, addr);
                        return;
                    },
                    Err(e) => {
                        report_violation!(
                            ViolationSeverity::Error,
                            ViolationKind::NetworkProtocol,
                            "Failed to serialize message: {}",
                            e
                        );
                        return;
                    },
                }
            },
            Err(e) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Failed to serialize message: {}",
                    e
                );
                return;
            },
        };

        self.send_encoded_packet(&self.send_buffer[..len], addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        // Pre-allocate for typical case of 1-4 messages per poll
        let mut received_messages = Vec::with_capacity(4);

        loop {
            // Use try_recv_from for non-blocking receive
            match self.socket.try_recv_from(&mut self.recv_buffer) {
                Ok((number_of_bytes, src_addr)) => {
                    // Defensive check
                    if number_of_bytes > RECV_BUFFER_SIZE {
                        report_violation!(
                            ViolationSeverity::Error,
                            ViolationKind::NetworkProtocol,
                            "Received {} bytes but buffer is only {} bytes",
                            number_of_bytes,
                            RECV_BUFFER_SIZE
                        );
                        continue;
                    }
                    if let Ok(msg) = codec::decode_value(&self.recv_buffer[0..number_of_bytes]) {
                        received_messages.push((src_addr, msg));
                    }
                },
                // No more messages available (non-blocking behavior)
                Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    return received_messages
                },
                // Connection reset - continue trying
                Err(ref err) if err.kind() == std::io::ErrorKind::ConnectionReset => continue,
                // Other errors - log and stop
                Err(err) => {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Unexpected socket error: {:?}: {}",
                        err.kind(),
                        err
                    );
                    return received_messages;
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::messages::{MessageBody, MessageHeader};
    use std::net::{IpAddr, Ipv4Addr};

    // Helper function to wait for messages with retry logic using the async recv_all method.
    // This is necessary because UDP packet delivery timing can vary across platforms.
    async fn wait_for_messages(
        socket: &mut TokioUdpSocket,
        expected_count: usize,
        timeout: std::time::Duration,
    ) -> Vec<(SocketAddr, Message)> {
        let mut all_received = Vec::new();

        let result = tokio::time::timeout(timeout, async {
            while all_received.len() < expected_count {
                let messages = socket.recv_all().await;
                all_received.extend(messages);
            }
        })
        .await;

        // Ignore timeout error, just return what we got
        let _ = result;
        all_received
    }

    // Helper to convert a socket's local address to a loopback address for sending.
    // When a socket binds to 0.0.0.0:port, its local_addr() returns 0.0.0.0:port,
    // but on Windows (and some other platforms), you cannot send to 0.0.0.0 - you
    // must send to 127.0.0.1 for loopback communication to work correctly.
    fn to_loopback_addr(socket: &TokioUdpSocket) -> SocketAddr {
        let local = socket.local_addr().unwrap();
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), local.port())
    }

    #[test]
    fn test_buffer_size_constants() {
        // Verify constants match the standard UDP socket
        assert_eq!(RECV_BUFFER_SIZE, 4096);
        assert_eq!(SEND_BUFFER_SIZE, 1024);
        assert_eq!(IDEAL_MAX_UDP_PACKET_SIZE, 508);

        // Send buffer should be larger than ideal packet size
        // Use compile-time check instead of runtime assertion
        const _: () = assert!(SEND_BUFFER_SIZE > IDEAL_MAX_UDP_PACKET_SIZE);
    }

    #[tokio::test]
    async fn test_tokio_socket_bind_to_port() {
        let socket = TokioUdpSocket::bind_to_port(0).await;
        assert!(socket.is_ok());

        let socket = socket.unwrap();
        let local_addr = socket.local_addr();
        assert!(local_addr.is_ok());
        // OS should have assigned a non-zero port
        assert_ne!(local_addr.unwrap().port(), 0);
    }

    #[tokio::test]
    async fn test_tokio_socket_new() {
        let tokio_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let socket = TokioUdpSocket::new(tokio_socket);

        // Verify buffer initialization
        assert!(socket.recv_buffer.iter().all(|&b| b == 0));
        assert!(socket.send_buffer.iter().all(|&b| b == 0));
    }

    #[tokio::test]
    async fn test_tokio_socket_inner() {
        let tokio_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let expected_addr = tokio_socket.local_addr().unwrap();
        let socket = TokioUdpSocket::new(tokio_socket);

        // inner() should return the same socket
        assert_eq!(socket.inner().local_addr().unwrap(), expected_addr);
    }

    #[tokio::test]
    async fn test_tokio_socket_is_non_blocking() {
        let mut socket = TokioUdpSocket::bind_to_port(0).await.unwrap();
        // receive_all_messages should return immediately even with no messages
        let messages = socket.receive_all_messages();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_tokio_socket_send_and_receive() {
        let mut socket1 = TokioUdpSocket::bind_to_port(0).await.unwrap();
        let mut socket2 = TokioUdpSocket::bind_to_port(0).await.unwrap();

        // Use loopback addresses for cross-platform compatibility.
        // Sockets bind to 0.0.0.0:port, but on Windows you cannot send to 0.0.0.0.
        let addr1 = to_loopback_addr(&socket1);
        let addr2 = to_loopback_addr(&socket2);

        let msg = Message {
            header: MessageHeader { magic: 0x1234 },
            body: MessageBody::KeepAlive,
        };

        // Send from socket1 to socket2 using async method
        socket1.send_to_async(&msg, &addr2).await;

        // Wait for message with retry logic (UDP timing varies by platform)
        let received = wait_for_messages(&mut socket2, 1, std::time::Duration::from_secs(1)).await;

        assert_eq!(
            received.len(),
            1,
            "Expected 1 message but got {}",
            received.len()
        );
        // Check port matches (IP may differ between 0.0.0.0 and 127.0.0.1)
        assert_eq!(received[0].0.port(), addr1.port());
        assert_eq!(received[0].1, msg);
    }

    #[tokio::test]
    async fn test_tokio_socket_receive_multiple_messages() {
        let mut socket1 = TokioUdpSocket::bind_to_port(0).await.unwrap();
        let mut socket2 = TokioUdpSocket::bind_to_port(0).await.unwrap();

        // Use loopback address for cross-platform compatibility.
        // On Windows, sending to 0.0.0.0:port doesn't work - must use 127.0.0.1.
        let addr2 = to_loopback_addr(&socket2);

        let msg1 = Message {
            header: MessageHeader { magic: 0x1111 },
            body: MessageBody::KeepAlive,
        };
        let msg2 = Message {
            header: MessageHeader { magic: 0x2222 },
            body: MessageBody::KeepAlive,
        };

        // Send using async method
        socket1.send_to_async(&msg1, &addr2).await;
        socket1.send_to_async(&msg2, &addr2).await;

        // Wait for messages with retry logic (UDP timing varies by platform)
        let received = wait_for_messages(&mut socket2, 2, std::time::Duration::from_secs(1)).await;

        assert_eq!(
            received.len(),
            2,
            "Expected 2 messages but got {}",
            received.len()
        );
    }

    #[tokio::test]
    async fn test_tokio_socket_debug() {
        let socket = TokioUdpSocket::bind_to_port(0).await.unwrap();
        let debug = format!("{:?}", socket);
        assert!(debug.contains("TokioUdpSocket"));
    }

    #[tokio::test]
    async fn test_tokio_socket_send_to_invalid_address() {
        let mut socket = TokioUdpSocket::bind_to_port(0).await.unwrap();
        // Sending to an unreachable address should not panic
        let invalid_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let msg = Message {
            header: MessageHeader { magic: 0x1234 },
            body: MessageBody::KeepAlive,
        };
        // This should log an error but not panic
        socket.send_to(&msg, &invalid_addr);
    }

    #[tokio::test]
    async fn test_tokio_socket_wait_writable() {
        let socket = TokioUdpSocket::bind_to_port(0).await.unwrap();

        // A freshly bound socket should become writable quickly
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(1), socket.wait_writable()).await;

        assert!(result.is_ok(), "wait_writable should not timeout");
        assert!(
            result.unwrap().is_ok(),
            "wait_writable should succeed on a bound socket"
        );
    }

    #[tokio::test]
    async fn test_tokio_socket_send_after_wait_writable() {
        let mut socket1 = TokioUdpSocket::bind_to_port(0).await.unwrap();
        let mut socket2 = TokioUdpSocket::bind_to_port(0).await.unwrap();

        let addr2 = to_loopback_addr(&socket2);

        let msg = Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::KeepAlive,
        };

        // Wait for socket to be writable, then send using sync method
        socket1.wait_writable().await.unwrap();
        socket1.send_to(&msg, &addr2);

        // Verify message was received
        let received = wait_for_messages(&mut socket2, 1, std::time::Duration::from_secs(1)).await;

        assert_eq!(
            received.len(),
            1,
            "Message should be received after wait_writable + send_to"
        );
        assert_eq!(received[0].1, msg);
    }
}
