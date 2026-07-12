use std::fmt::Debug;
use std::io::{self, ErrorKind};

use crate::network::codec;
use crate::network::messages::Message;
use crate::network::MAX_RECEIVE_MESSAGES_PER_POLL;
use crate::report_violation;
use crate::telemetry::{ViolationKind, ViolationSeverity};

pub(super) fn receive_all_messages_from<A: Debug>(
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
    let mut reported_wire_rejects = 0u8;
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
                    match codec::decode_message(buf_slice) {
                        Ok((msg, _consumed)) => {
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
                        },
                        Err(_err) => {
                            let reject = codec::classify_wire_bytes(buf_slice);
                            let bit = reject.rate_limit_bit();
                            if reported_wire_rejects & bit == 0 {
                                reported_wire_rejects |= bit;
                                report_violation!(
                                    ViolationSeverity::Warning,
                                    ViolationKind::NetworkProtocol,
                                    "{} rejected datagram from {:?}: {}",
                                    adapter_name,
                                    src_addr,
                                    reject
                                );
                            }
                        },
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
    use crate::telemetry::{push_violation_observer, CollectingObserver};
    use std::collections::VecDeque;
    use std::net::SocketAddr;
    use std::sync::Arc;

    fn receive_packets(
        packets: &mut VecDeque<Vec<u8>>,
        addr: SocketAddr,
    ) -> Vec<(SocketAddr, Message)> {
        let mut recv_buffer = vec![0; 64];
        receive_all_messages_from(&mut recv_buffer, "test", |buffer| {
            let packet = packets
                .pop_front()
                .ok_or_else(|| io::Error::from(ErrorKind::WouldBlock))?;
            let len = packet.len();
            buffer[..len].copy_from_slice(&packet);
            Ok((len, addr))
        })
    }

    #[test]
    fn receive_all_messages_from_caps_raw_malformed_datagrams() {
        let addr: SocketAddr = "127.0.0.1:7000".parse().unwrap();
        let msg = Message {
            header: MessageHeader::new(0xCAFE),
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

    #[test]
    fn receive_all_messages_from_reports_each_reject_family_once_per_poll() {
        let addr: SocketAddr = "127.0.0.1:7001".parse().unwrap();
        let valid = Message {
            header: MessageHeader::new(1),
            body: MessageBody::KeepAlive,
        };
        let mut unsupported = codec::encode(&valid).unwrap();
        unsupported[2] = crate::PROTOCOL_VERSION.saturating_add(1);
        let mut flags = codec::encode(&valid).unwrap();
        flags[3] = 0x40;
        let mut bad_sentinel = codec::encode(&valid).unwrap();
        bad_sentinel[0] = 0;
        let families = [
            vec![0x34, 0x12, 0, 0, 0, 0],
            unsupported,
            flags,
            bad_sentinel,
            vec![0xF5],
        ];
        let mut packets = VecDeque::new();
        for family in families {
            packets.push_back(family.clone());
            packets.push_back(family);
        }
        packets.push_back(codec::encode(&valid).unwrap());

        let observer = Arc::new(CollectingObserver::new());
        let _guard = push_violation_observer(observer.clone());
        let received = receive_packets(&mut packets, addr);

        assert_eq!(received, vec![(addr, valid)]);
        let violations = observer.violations();
        assert_eq!(violations.len(), 5);
        for expected in [
            "legacy unversioned",
            "unsupported protocol version",
            "unknown protocol flags",
            "bad protocol sentinel",
            "malformed protocol packet",
        ] {
            assert_eq!(
                violations
                    .iter()
                    .filter(|violation| violation.message.contains(expected))
                    .count(),
                1,
                "reject family {expected:?} should report once: {violations:?}"
            );
        }
        assert!(violations
            .iter()
            .all(|violation| violation.message.contains(&addr.to_string())));
    }

    #[test]
    fn receive_all_messages_from_resets_reject_limit_each_poll() {
        let addr: SocketAddr = "127.0.0.1:7002".parse().unwrap();
        let observer = Arc::new(CollectingObserver::new());
        let _guard = push_violation_observer(observer.clone());

        for _ in 0..2 {
            let mut packets = VecDeque::from([vec![0xF5]]);
            assert!(receive_packets(&mut packets, addr).is_empty());
        }

        assert_eq!(observer.len(), 2);
    }
}
