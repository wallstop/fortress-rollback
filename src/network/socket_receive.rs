use std::fmt::Debug;
use std::io::{self, ErrorKind};

use crate::metrics::SocketMetrics;
use crate::network::codec;
use crate::network::messages::Message;
use crate::network::MAX_RECEIVE_MESSAGES_PER_POLL;
use crate::report_violation;
use crate::telemetry::{ViolationKind, ViolationSeverity};

pub(super) fn receive_all_messages_from<A: Debug>(
    recv_buffer: &mut [u8],
    adapter_name: &str,
    metrics: &mut SocketMetrics,
    mut receive_next: impl FnMut(&mut [u8]) -> io::Result<(usize, A)>,
) -> Vec<(A, Message)> {
    if recv_buffer.is_empty() {
        return Vec::new();
    }

    metrics.record_receive_poll();

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
            metrics.record_receive_cap_saturation();
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
                metrics.record_datagram_received(number_of_bytes);
                // Defensive check: if we received more bytes than buffer allows,
                // something is seriously wrong - skip this packet.
                if number_of_bytes > recv_buffer.len() {
                    metrics.record_datagram_rejected();
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

                // The bounds check above guarantees this slice exists; the
                // loop-top attempt cap bounds the decoded batch to the same
                // per-poll maximum, so no separate batch-cap branch is needed.
                let Some(buf_slice) = recv_buffer.get(..number_of_bytes) else {
                    metrics.record_datagram_rejected();
                    continue;
                };
                match codec::decode_message(buf_slice) {
                    Ok((msg, _consumed)) => {
                        // reserve-in-loop: guarded by MAX_RECEIVE_MESSAGES_PER_POLL raw receive attempts.
                        if received_messages.try_reserve(1).is_err() {
                            report_violation!(
                                ViolationSeverity::Error,
                                ViolationKind::NetworkProtocol,
                                "Failed to reserve {} received message slot",
                                adapter_name
                            );
                            return received_messages;
                        }
                        metrics.record_message_decoded();
                        received_messages.push((src_addr, msg));
                    },
                    Err(_err) => {
                        metrics.record_datagram_rejected();
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
        let mut metrics = SocketMetrics::default();
        receive_all_messages_from(&mut recv_buffer, "test", &mut metrics, |buffer| {
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

        let mut metrics = SocketMetrics::default();
        let first_poll =
            receive_all_messages_from(&mut recv_buffer, "test", &mut metrics, &mut receive_next);
        assert!(
            first_poll.is_empty(),
            "malformed datagrams should count toward the raw receive-attempt cap without decoding"
        );
        assert_eq!(metrics.receive_polls, 1);
        assert_eq!(metrics.datagrams_received, 256);
        assert_eq!(metrics.datagrams_rejected, 256);
        assert_eq!(metrics.messages_decoded, 0);
        assert_eq!(metrics.receive_cap_saturations, 1);

        let second_poll =
            receive_all_messages_from(&mut recv_buffer, "test", &mut metrics, &mut receive_next);
        assert_eq!(second_poll, vec![(addr, msg)]);
        assert_eq!(metrics.receive_polls, 2);
        assert_eq!(metrics.datagrams_received, 257);
        assert_eq!(metrics.datagrams_rejected, 256);
        assert_eq!(metrics.messages_decoded, 1);
        assert_eq!(metrics.receive_cap_saturations, 1);
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
    fn receive_all_messages_from_reports_oversized_and_unexpected_receive_errors() {
        let addr: SocketAddr = "127.0.0.1:7003".parse().unwrap();
        let observer = Arc::new(CollectingObserver::new());
        let _guard = push_violation_observer(observer.clone());
        let mut recv_buffer = vec![0; 4];
        let mut metrics = SocketMetrics::default();

        let mut oversized_reported = false;
        let oversized =
            receive_all_messages_from(&mut recv_buffer, "test", &mut metrics, |_buffer| {
                if oversized_reported {
                    return Err(io::Error::from(ErrorKind::WouldBlock));
                }
                oversized_reported = true;
                Ok((5, addr))
            });
        assert!(oversized.is_empty());

        let errored = receive_all_messages_from(
            &mut recv_buffer,
            "test",
            &mut metrics,
            |_buffer| -> io::Result<(usize, SocketAddr)> { Err(io::Error::from(ErrorKind::Other)) },
        );
        assert!(errored.is_empty());
        assert_eq!(observer.len(), 2);
    }

    #[test]
    fn receive_all_messages_from_resets_reject_limit_each_poll() {
        let addr: SocketAddr = "127.0.0.1:7002".parse().unwrap();
        let observer = Arc::new(CollectingObserver::new());
        let _guard = push_violation_observer(observer.clone());

        for _ in 0..2 {
            let mut packets = VecDeque::from([vec![0xF5]]);
            assert!(
                receive_packets(&mut packets, addr).is_empty(),
                "receive_packets(&mut packets, addr) is expected to be empty"
            );
        }

        assert_eq!(observer.len(), 2);
    }
}
