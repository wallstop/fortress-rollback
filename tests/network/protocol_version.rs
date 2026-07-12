//! Protocol-version refusal through the real raw-UDP receive boundary.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use crate::common::stubs::StubConfig;
use crate::common::TestClock;
use fortress_rollback::telemetry::{CollectingObserver, ViolationKind, ViolationSeverity};
use fortress_rollback::{
    FortressEvent, PlayerHandle, PlayerType, ProtocolConfig, SessionBuilder, SessionState,
    SyncConfig, UdpNonBlockingSocket, PROTOCOL_VERSION,
};
use std::net::{Ipv4Addr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;

#[path = "wire_golden_legacy_0_9.rs"]
mod legacy;

#[test]
#[cfg(not(miri))]
fn unsupported_wire_version_reports_violation_and_never_synchronizes() {
    let clock = TestClock::new();
    let receiver = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    receiver.set_nonblocking(true).unwrap();
    let receiver_addr = receiver.local_addr().unwrap();
    let socket = UdpNonBlockingSocket::from_socket_with_buffer_sizes(receiver, 4096, 1024)
        .expect("valid non-zero socket buffers");
    let raw_peer = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let raw_peer_addr = raw_peer.local_addr().unwrap();
    let observer = Arc::new(CollectingObserver::new());
    let protocol_config = ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        ..ProtocolConfig::default()
    };
    let mut session = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config)
        .with_violation_observer(observer.clone())
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .unwrap()
        .add_player(PlayerType::Remote(raw_peer_addr), PlayerHandle::new(1))
        .unwrap()
        .start_p2p_session(socket)
        .unwrap();

    let unsupported = [
        0xF5,
        0x52,
        PROTOCOL_VERSION.saturating_add(1),
        0,
        1,
        0,
        0,
        0,
        7,
        0,
        0,
        0,
    ];
    assert_eq!(raw_peer.send_to(&unsupported, receiver_addr).unwrap(), 12);

    let mut events = Vec::new();
    for _ in 0..128 {
        session.poll_remote_clients();
        events.extend(session.events());
        if !observer.is_empty() {
            break;
        }
        clock.advance(Duration::from_millis(1));
        std::thread::yield_now();
    }

    assert!(events.is_empty(), "rejected bytes must not create events");
    assert_eq!(session.current_state(), SessionState::Synchronizing);
    let violations = observer.violations();
    assert_eq!(violations.len(), 1, "violations={violations:?}");
    let violation = &violations[0];
    assert_eq!(violation.severity, ViolationSeverity::Warning);
    assert_eq!(violation.kind, ViolationKind::NetworkProtocol);
    assert!(violation.message.contains("unsupported protocol version"));
    assert!(violation.message.contains(&raw_peer_addr.to_string()));
}

#[test]
#[cfg(not(miri))]
fn legacy_handshake_is_classified_and_times_out_without_synchronizing() {
    let clock = TestClock::new();
    let receiver = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    receiver.set_nonblocking(true).unwrap();
    let receiver_addr = receiver.local_addr().unwrap();
    let socket = UdpNonBlockingSocket::from_socket_with_buffer_sizes(receiver, 4096, 1024)
        .expect("valid non-zero socket buffers");
    let raw_peer = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let raw_peer_addr = raw_peer.local_addr().unwrap();
    let observer = Arc::new(CollectingObserver::new());
    let sync_config = SyncConfig::default();
    let sync_timeout = sync_config
        .sync_timeout
        .expect("the default sync config must bound synchronization");
    let protocol_config = ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        ..ProtocolConfig::default()
    };
    let mut session = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config)
        .with_sync_config(sync_config)
        .with_violation_observer(observer.clone())
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .unwrap()
        .add_player(PlayerType::Remote(raw_peer_addr), PlayerHandle::new(1))
        .unwrap()
        .start_p2p_session(socket)
        .unwrap();

    assert_eq!(
        raw_peer
            .send_to(legacy::LEGACY_SYNC_REQUEST, receiver_addr)
            .unwrap(),
        legacy::LEGACY_SYNC_REQUEST.len()
    );
    for _ in 0..128 {
        session.poll_remote_clients();
        if !observer.is_empty() {
            break;
        }
        clock.advance(Duration::from_millis(1));
        std::thread::yield_now();
    }

    let violations = observer.violations();
    assert_eq!(violations.len(), 1, "violations={violations:?}");
    assert_eq!(violations[0].severity, ViolationSeverity::Warning);
    assert_eq!(violations[0].kind, ViolationKind::NetworkProtocol);
    assert!(violations[0]
        .message
        .contains("suspected legacy unversioned packet"));
    assert_eq!(session.current_state(), SessionState::Synchronizing);

    clock.advance(sync_timeout + Duration::from_millis(1));
    session.poll_remote_clients();
    let events: Vec<_> = session.events().collect();
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, FortressEvent::SyncTimeout { .. }))
            .count(),
        1,
        "events={events:?}"
    );
    assert!(events
        .iter()
        .all(|event| !matches!(event, FortressEvent::Synchronized { .. })));
    assert_eq!(session.current_state(), SessionState::Synchronizing);
}
