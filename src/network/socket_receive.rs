use std::io::{self, ErrorKind};

use crate::network::codec;
use crate::network::messages::Message;
use crate::network::MAX_RECEIVE_MESSAGES_PER_POLL;
use crate::report_violation;
use crate::telemetry::{ViolationKind, ViolationSeverity};

pub(super) fn receive_all_messages_from<A>(
    recv_buffer: &mut [u8],
    adapter_name: &str,
    mut receive_next: impl FnMut(&mut [u8]) -> io::Result<(usize, A)>,
) -> Vec<(A, Message)> {
    if recv_buffer.is_empty() {
        return Vec::new();
    }

    // Pre-allocate for typical case of 1-4 messages per poll.
    let mut received_messages = Vec::new();
    if received_messages.try_reserve_exact(4).is_err() {
        report_violation!(
            ViolationSeverity::Error,
            ViolationKind::NetworkProtocol,
            "Failed to reserve {} receive batch",
            adapter_name
        );
        return received_messages;
    }

    let mut receive_attempts = 0usize;
    loop {
        if receive_attempts >= MAX_RECEIVE_MESSAGES_PER_POLL {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "{} receive attempts reached per-poll cap of {} datagram(s)",
                adapter_name,
                MAX_RECEIVE_MESSAGES_PER_POLL
            );
            return received_messages;
        }
        receive_attempts += 1;

        match receive_next(recv_buffer) {
            Ok((number_of_bytes, src_addr)) => {
                // Defensive check: if we received more bytes than buffer allows,
                // something is seriously wrong - skip this packet.
                if number_of_bytes > recv_buffer.len() {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "{} received {} bytes but buffer is only {} bytes",
                        adapter_name,
                        number_of_bytes,
                        recv_buffer.len()
                    );
                    continue;
                }

                if let Some(buf_slice) = recv_buffer.get(..number_of_bytes) {
                    if let Ok((msg, _consumed)) = codec::decode_message(buf_slice) {
                        if received_messages.len() >= MAX_RECEIVE_MESSAGES_PER_POLL {
                            report_violation!(
                                ViolationSeverity::Warning,
                                ViolationKind::NetworkProtocol,
                                "{} receive batch reached per-poll cap of {} message(s)",
                                adapter_name,
                                MAX_RECEIVE_MESSAGES_PER_POLL
                            );
                            return received_messages;
                        }
                        // reserve-in-loop: guarded by MAX_RECEIVE_MESSAGES_PER_POLL raw receive attempts and decoded-message cap.
                        if received_messages.try_reserve(1).is_err() {
                            report_violation!(
                                ViolationSeverity::Error,
                                ViolationKind::NetworkProtocol,
                                "Failed to reserve {} received message slot",
                                adapter_name
                            );
                            return received_messages;
                        }
                        received_messages.push((src_addr, msg));
                    }
                } else {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "{} recv_buffer slice [0..{}] out of bounds (buffer size: {})",
                        adapter_name,
                        number_of_bytes,
                        recv_buffer.len()
                    );
                }
            },
            // No more messages.
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => return received_messages,
            // Datagram sockets can report this after send_to; keep draining until
            // WouldBlock or the raw-attempt cap is reached.
            Err(ref err) if err.kind() == ErrorKind::ConnectionReset => continue,
            // For other errors, log and stop receiving.
            Err(err) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "{} unexpected socket error: {:?}: {}",
                    adapter_name,
                    err.kind(),
                    err
                );
                return received_messages;
            },
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
    use crate::network::messages::{MessageBody, MessageHeader};
    use std::collections::VecDeque;
    use std::net::SocketAddr;

    #[test]
    fn receive_all_messages_from_caps_raw_malformed_datagrams() {
        let addr: SocketAddr = "127.0.0.1:7000".parse().unwrap();
        let msg = Message {
            header: MessageHeader { magic: 0xCAFE },
            body: MessageBody::KeepAlive,
        };

        let mut packets = VecDeque::new();
        for _ in 0..MAX_RECEIVE_MESSAGES_PER_POLL {
            packets.push_back(vec![0xFF]);
        }
        packets.push_back(codec::encode(&msg).unwrap());

        let mut recv_buffer = vec![0; 64];
        let mut receive_next = |buffer: &mut [u8]| -> io::Result<(usize, SocketAddr)> {
            let packet = packets
                .pop_front()
                .ok_or_else(|| io::Error::from(ErrorKind::WouldBlock))?;
            let len = packet.len();
            buffer[..len].copy_from_slice(&packet);
            Ok((len, addr))
        };

        let first_poll = receive_all_messages_from(&mut recv_buffer, "test", &mut receive_next);
        assert!(
            first_poll.is_empty(),
            "malformed datagrams should count toward the raw receive-attempt cap without decoding"
        );

        let second_poll = receive_all_messages_from(&mut recv_buffer, "test", &mut receive_next);
        assert_eq!(second_poll, vec![(addr, msg)]);
    }
}
