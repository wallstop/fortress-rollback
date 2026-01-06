use std::{
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
};

use crate::network::codec;
use crate::report_violation;
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::{network::messages::Message, NonBlockingSocket};

const RECV_BUFFER_SIZE: usize = 4096;
/// Size of the pre-allocated send buffer. This should be large enough to hold
/// any message we might send. 1KB is generous for typical network messages.
const SEND_BUFFER_SIZE: usize = 1024;
/// A packet larger than this may be fragmented, so ideally we wouldn't send packets larger than
/// this.
/// Source: <https://stackoverflow.com/a/35697810/775982>
const IDEAL_MAX_UDP_PACKET_SIZE: usize = 508;

/// A simple non-blocking UDP socket to use with Fortress Rollback Sessions. Listens to 0.0.0.0 on a given port.
///
/// # Performance
///
/// This socket maintains internal buffers for both sending and receiving to minimize
/// allocations in the hot path. The send buffer is reused across calls to [`send_to`],
/// and the receive buffer is sized to handle typical UDP MTU sizes.
///
/// [`send_to`]: NonBlockingSocket::send_to
#[derive(Debug)]
pub struct UdpNonBlockingSocket {
    socket: UdpSocket,
    /// Receive buffer - reused across recv_from calls
    recv_buffer: [u8; RECV_BUFFER_SIZE],
    /// Send buffer - reused across send_to calls to avoid allocation
    send_buffer: [u8; SEND_BUFFER_SIZE],
}

impl UdpNonBlockingSocket {
    /// Binds an UDP Socket to 0.0.0.0:port and set it to non-blocking mode.
    pub fn bind_to_port(port: u16) -> Result<Self, std::io::Error> {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            recv_buffer: [0; RECV_BUFFER_SIZE],
            send_buffer: [0; SEND_BUFFER_SIZE],
        })
    }
}

impl NonBlockingSocket<SocketAddr> for UdpNonBlockingSocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        // Serialize into the pre-allocated send buffer to avoid allocation.
        // This is the hot path for network sends.
        let len = match codec::encode_into(msg, &mut self.send_buffer) {
            Ok(len) => len,
            Err(codec::CodecError::BufferTooSmall { provided, .. }) => {
                // The message is larger than our send buffer. This is unusual but we can
                // handle it by falling back to allocation. Log a warning since this suggests
                // the message is unusually large (possibly large input structs).
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

        let buf_slice = self.send_buffer.get(..len).unwrap_or_else(|| {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "send_buffer slice [..{}] out of bounds (buffer size: {})",
                len,
                self.send_buffer.len()
            );
            &[]
        });
        self.send_encoded_packet(buf_slice, addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        // Pre-allocate for typical case of 1-4 messages per poll
        let mut received_messages = Vec::with_capacity(4);
        loop {
            match self.socket.recv_from(&mut self.recv_buffer) {
                Ok((number_of_bytes, src_addr)) => {
                    // Defensive check: if we received more bytes than buffer allows,
                    // something is seriously wrong - skip this packet
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
                    if let Some(buf_slice) = self.recv_buffer.get(0..number_of_bytes) {
                        if let Ok(msg) = codec::decode_value(buf_slice) {
                            received_messages.push((src_addr, msg));
                        }
                    } else {
                        report_violation!(
                            ViolationSeverity::Error,
                            ViolationKind::NetworkProtocol,
                            "recv_buffer slice [0..{}] out of bounds (buffer size: {})",
                            number_of_bytes,
                            RECV_BUFFER_SIZE
                        );
                    }
                },
                // there are no more messages
                Err(ref err) if err.kind() == ErrorKind::WouldBlock => return received_messages,
                // datagram socket sometimes get this error as a result of calling the send_to method
                Err(ref err) if err.kind() == ErrorKind::ConnectionReset => continue,
                // For other errors, log and stop receiving (don't panic)
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

impl UdpNonBlockingSocket {
    /// Sends an already-encoded packet to the given address.
    ///
    /// This is a helper that handles packet size warnings and send errors.
    fn send_encoded_packet(&self, buf: &[u8], addr: &SocketAddr) {
        // Overly large packets risk being fragmented, which can increase packet loss (any fragment
        // of a packet getting lost will cause the whole fragment to be lost), or increase latency
        // to be delayed (have to wait for all fragments to arrive).
        //
        // And if there's a large packet that's being sent, it's basically guaranteed that it's
        // because consuming code has submitted an input struct that is too large (and/or too large
        // a prediction window on too poor a connection, and/or the input struct did not delta
        // encode well). So we should let the user of fortress-rollback know about that, so they can fix it by
        // reducing the size of their input struct.
        //
        // On the other hand, the occasional large packet is kind of harmless - whether it gets
        // fragmented or not, the odds are that it will get through unless the connection is truly
        // horrible. So, we'll just log a warning.
        if buf.len() > IDEAL_MAX_UDP_PACKET_SIZE {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Sending UDP packet of size {} bytes, which is larger than ideal ({})",
                buf.len(),
                IDEAL_MAX_UDP_PACKET_SIZE
            );
        }

        // Send the packet; if this fails, log an error but don't panic.
        // UDP is best-effort, so dropped packets are expected behavior.
        if let Err(e) = self.socket.send_to(buf, addr) {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Failed to send UDP packet to {}: {}",
                addr,
                e
            );
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    #[cfg(not(miri))]
    use crate::network::messages::{MessageBody, MessageHeader};

    // Helper function to wait for messages with retry logic
    // This is necessary because UDP packet delivery timing can vary across platforms
    #[cfg(not(miri))]
    #[track_caller]
    fn wait_for_messages(
        socket: &mut UdpNonBlockingSocket,
        expected_count: usize,
        max_retries: u32,
    ) -> Vec<(SocketAddr, Message)> {
        let mut all_received = Vec::new();
        for _ in 0..max_retries {
            let received = socket.receive_all_messages();
            all_received.extend(received);
            if all_received.len() >= expected_count {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        all_received
    }

    // Helper to convert a socket's local address to a loopback address for sending.
    // When a socket binds to 0.0.0.0:port, its local_addr() returns 0.0.0.0:port,
    // but on Windows (and some other platforms), you cannot send to 0.0.0.0 - you
    // must send to 127.0.0.1 for loopback communication to work correctly.
    #[cfg(not(miri))]
    #[track_caller]
    fn to_loopback_addr(socket: &UdpNonBlockingSocket) -> SocketAddr {
        let local = socket.socket.local_addr().unwrap();
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), local.port())
    }

    #[test]
    #[cfg(not(miri))] // Miri cannot execute foreign functions like socket()
    fn test_udp_socket_bind_to_port() {
        // Bind to port 0 to let OS assign an available port
        UdpNonBlockingSocket::bind_to_port(0).unwrap();
    }

    #[test]
    #[cfg(not(miri))] // Miri cannot execute foreign functions like socket()
    fn test_udp_socket_is_non_blocking() {
        let mut socket = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        // receive_all_messages should return immediately even with no messages
        let messages = socket.receive_all_messages();
        assert!(messages.is_empty());
    }

    #[test]
    #[cfg(not(miri))] // Miri cannot execute foreign functions like socket()
    fn test_udp_socket_send_and_receive() {
        let mut socket1 = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        let mut socket2 = UdpNonBlockingSocket::bind_to_port(0).unwrap();

        // Use loopback addresses for cross-platform compatibility.
        // Sockets bind to 0.0.0.0:port, but on Windows you cannot send to 0.0.0.0.
        let addr1 = to_loopback_addr(&socket1);
        let addr2 = to_loopback_addr(&socket2);

        let msg = Message {
            header: MessageHeader { magic: 0x1234 },
            body: MessageBody::KeepAlive,
        };

        // Send from socket1 to socket2
        socket1.send_to(&msg, &addr2);

        // Wait for message with retry logic (UDP timing varies by platform)
        let received = wait_for_messages(&mut socket2, 1, 20);
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

    #[test]
    #[cfg(not(miri))] // Miri cannot execute foreign functions like socket()
    fn test_udp_socket_receive_multiple_messages() {
        let mut socket1 = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        let mut socket2 = UdpNonBlockingSocket::bind_to_port(0).unwrap();

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

        socket1.send_to(&msg1, &addr2);
        socket1.send_to(&msg2, &addr2);

        // Wait for messages with retry logic (UDP timing varies by platform)
        let received = wait_for_messages(&mut socket2, 2, 20);
        assert_eq!(
            received.len(),
            2,
            "Expected 2 messages but got {}",
            received.len()
        );
    }

    #[test]
    #[cfg(not(miri))] // Miri cannot execute foreign functions like socket()
    fn test_udp_socket_debug() {
        let socket = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        let debug = format!("{:?}", socket);
        assert!(debug.contains("UdpNonBlockingSocket"));
    }

    #[test]
    fn test_ideal_max_udp_packet_size_constant() {
        // Verify the constant is a reasonable value for UDP packets
        // 508 bytes is the safe payload size to avoid fragmentation
        assert_eq!(IDEAL_MAX_UDP_PACKET_SIZE, 508);
    }

    #[test]
    fn test_recv_buffer_size_constant() {
        // Verify the buffer size is 4KB
        assert_eq!(RECV_BUFFER_SIZE, 4096);
    }

    #[test]
    fn test_send_buffer_size_constant() {
        // Verify the send buffer is 1KB, which is generous for typical messages
        // This should be larger than IDEAL_MAX_UDP_PACKET_SIZE to handle edge cases
        assert_eq!(SEND_BUFFER_SIZE, 1024);
        // Compile-time assertion that send buffer is larger than ideal packet size
        const _: () = assert!(SEND_BUFFER_SIZE > IDEAL_MAX_UDP_PACKET_SIZE);
    }

    // ==========================================
    // Edge Case and Error Path Tests
    // ==========================================

    #[test]
    #[allow(clippy::assertions_on_constants)] // Intentional: verifying constant relationships
    fn test_buffer_sizes_relationship() {
        // SEND_BUFFER_SIZE should be at least as large as IDEAL_MAX_UDP_PACKET_SIZE
        // to handle normal messages without fallback allocation
        assert!(
            SEND_BUFFER_SIZE >= IDEAL_MAX_UDP_PACKET_SIZE,
            "SEND_BUFFER_SIZE must be >= IDEAL_MAX_UDP_PACKET_SIZE"
        );
        // RECV_BUFFER_SIZE should be larger to handle any incoming UDP packet
        assert!(
            RECV_BUFFER_SIZE >= SEND_BUFFER_SIZE,
            "RECV_BUFFER_SIZE must be >= SEND_BUFFER_SIZE"
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn test_udp_socket_receive_no_messages() {
        let mut socket = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        // Should return empty vec immediately (non-blocking)
        let messages = socket.receive_all_messages();
        assert!(messages.is_empty());
        // Call again - should still be empty and not panic
        let messages2 = socket.receive_all_messages();
        assert!(messages2.is_empty());
    }

    #[test]
    #[cfg(not(miri))]
    fn test_udp_socket_send_to_invalid_address() {
        let mut socket = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        // Sending to an unreachable address should not panic
        // Note: 0.0.0.0:0 is an invalid destination address
        let invalid_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let msg = Message {
            header: MessageHeader { magic: 0x1234 },
            body: MessageBody::KeepAlive,
        };
        // This should log an error but not panic
        socket.send_to(&msg, &invalid_addr);
    }

    #[test]
    #[cfg(not(miri))]
    fn test_udp_socket_bind_to_specific_port() {
        // Test binding to port 0 (let OS pick)
        let socket = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        let local_addr = socket.socket.local_addr().unwrap();
        // Port should be non-zero (OS assigned)
        assert_ne!(local_addr.port(), 0);
    }

    #[test]
    #[cfg(not(miri))]
    fn test_udp_socket_recv_buffer_initialized() {
        let socket = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        // Verify recv_buffer is initialized to zeros
        assert!(socket.recv_buffer.iter().all(|&b| b == 0));
    }

    #[test]
    #[cfg(not(miri))]
    fn test_udp_socket_send_buffer_initialized() {
        let socket = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        // Verify send_buffer is initialized to zeros
        assert!(socket.send_buffer.iter().all(|&b| b == 0));
    }

    #[test]
    #[cfg(not(miri))]
    fn test_udp_socket_local_addr_is_unspecified() {
        let socket = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        let local = socket.socket.local_addr().unwrap();
        // Should be bound to 0.0.0.0 (UNSPECIFIED)
        assert!(local.ip().is_unspecified());
    }

    #[test]
    #[cfg(not(miri))]
    fn test_udp_socket_multiple_sends_reuse_buffer() {
        let mut socket1 = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        let mut socket2 = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        let addr2 = to_loopback_addr(&socket2);

        // Send multiple messages - buffer should be reused
        for i in 0..5u16 {
            let msg = Message {
                header: MessageHeader { magic: i },
                body: MessageBody::KeepAlive,
            };
            socket1.send_to(&msg, &addr2);
        }

        // Receive all messages
        let received = wait_for_messages(&mut socket2, 5, 20);
        // Some messages might be lost (UDP), but we should get at least 1
        assert!(!received.is_empty(), "Should receive at least one message");
    }

    #[test]
    #[cfg(not(miri))]
    fn test_udp_socket_keepalive_message_roundtrip() {
        let mut socket1 = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        let mut socket2 = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        let addr2 = to_loopback_addr(&socket2);

        let msg = Message {
            header: MessageHeader { magic: 0xDEAD },
            body: MessageBody::KeepAlive,
        };

        socket1.send_to(&msg, &addr2);
        let received = wait_for_messages(&mut socket2, 1, 20);

        assert_eq!(received.len(), 1);
        assert_eq!(received[0].1.header.magic, 0xDEAD);
        assert!(matches!(received[0].1.body, MessageBody::KeepAlive));
    }

    #[test]
    #[cfg(not(miri))]
    fn test_udp_socket_handles_self_send() {
        let mut socket = UdpNonBlockingSocket::bind_to_port(0).unwrap();
        let self_addr = to_loopback_addr(&socket);

        let msg = Message {
            header: MessageHeader { magic: 0xBEEF },
            body: MessageBody::KeepAlive,
        };

        // Send to self
        socket.send_to(&msg, &self_addr);

        // Should be able to receive our own message
        let received = wait_for_messages(&mut socket, 1, 20);
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].1.header.magic, 0xBEEF);
    }
}
