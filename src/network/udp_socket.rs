use std::{
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
};

use crate::report_violation;
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::{network::messages::Message, NonBlockingSocket};

const RECV_BUFFER_SIZE: usize = 4096;
/// A packet larger than this may be fragmented, so ideally we wouldn't send packets larger than
/// this.
/// Source: <https://stackoverflow.com/a/35697810/775982>
const IDEAL_MAX_UDP_PACKET_SIZE: usize = 508;

/// A simple non-blocking UDP socket to use with Fortress Rollback Sessions. Listens to 0.0.0.0 on a given port.
#[derive(Debug)]
pub struct UdpNonBlockingSocket {
    socket: UdpSocket,
    buffer: [u8; RECV_BUFFER_SIZE],
}

impl UdpNonBlockingSocket {
    /// Binds an UDP Socket to 0.0.0.0:port and set it to non-blocking mode.
    pub fn bind_to_port(port: u16) -> Result<Self, std::io::Error> {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            buffer: [0; RECV_BUFFER_SIZE],
        })
    }
}

impl NonBlockingSocket<SocketAddr> for UdpNonBlockingSocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        // Serialize the message; if this fails, log an error and skip sending.
        // This should never happen with well-formed Message types, but we don't want to panic.
        let buf = match bincode::serialize(&msg) {
            Ok(buf) => buf,
            Err(e) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Failed to serialize message: {}",
                    e
                );
                return;
            }
        };

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
        // On the other hand, the occaisional large packet is kind of harmless - whether it gets
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
        if let Err(e) = self.socket.send_to(&buf, addr) {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Failed to send UDP packet to {}: {}",
                addr,
                e
            );
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        let mut received_messages = Vec::new();
        loop {
            match self.socket.recv_from(&mut self.buffer) {
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
                    if let Ok(msg) = bincode::deserialize(&self.buffer[0..number_of_bytes]) {
                        received_messages.push((src_addr, msg));
                    }
                }
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
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::messages::{MessageBody, MessageHeader};

    // Helper function to wait for messages with retry logic
    // This is necessary because UDP packet delivery timing can vary across platforms
    #[cfg(not(miri))]
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
    fn to_loopback_addr(socket: &UdpNonBlockingSocket) -> SocketAddr {
        let local = socket.socket.local_addr().unwrap();
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), local.port())
    }

    #[test]
    #[cfg(not(miri))] // Miri cannot execute foreign functions like socket()
    fn test_udp_socket_bind_to_port() {
        // Bind to port 0 to let OS assign an available port
        let socket = UdpNonBlockingSocket::bind_to_port(0);
        assert!(socket.is_ok());
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
}
