//! Integration tests for the `Session` trait.
//!
//! These tests verify that all session types correctly implement the [`Session`]
//! trait and that generic code can work uniformly with any session type.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::ip_constant
)]

use crate::common::stubs::{StubConfig, StubInput};
use crate::common::{bind_socket_ephemeral, PortAllocator};
use fortress_rollback::{
    Config, FortressError, InvalidRequestKind, PlayerHandle, PlayerType, Session, SessionBuilder,
    SessionState,
};
use serial_test::serial;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

// ============================================================================
// Helper: generic function that operates on any Session<T>
// ============================================================================

/// Calls trait methods through a generic `&mut impl Session<T>` reference.
///
/// This function exercises the trait interface without knowing the concrete type,
/// proving that generic code can work uniformly with any session implementation.
fn call_via_trait<T: Config>(session: &mut impl Session<T>) -> SessionState {
    session.poll_remote_clients();
    // Exercise events() — consume the drain to release the mutable borrow
    let _event_count = session.events().count();
    session.current_state()
}

// ============================================================================
// P2PSession implements Session
// ============================================================================

#[test]
#[serial]
fn p2p_session_implements_session_trait() -> Result<(), FortressError> {
    // Arrange
    let (socket, _addr0) = bind_socket_ephemeral()?;
    let [port1] = PortAllocator::next_ports::<1>();
    let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port1);
    let mut sess = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    // Act — call through the trait-generic helper
    let state = call_via_trait(&mut sess);

    // Assert — P2P starts in Synchronizing (waiting for remote peer)
    assert_eq!(state, SessionState::Synchronizing);
    Ok(())
}

#[test]
#[serial]
fn p2p_local_player_handle_required_returns_handle_via_trait() -> Result<(), FortressError> {
    // Arrange
    let (socket, _addr0) = bind_socket_ephemeral()?;
    let [port1] = PortAllocator::next_ports::<1>();
    let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port1);
    let sess = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    // Act — call through the trait interface
    let handle = Session::local_player_handle_required(&sess)?;

    // Assert — handle should be player 0 (the local player)
    assert_eq!(handle, PlayerHandle::new(0));
    Ok(())
}

// ============================================================================
// SpectatorSession implements Session
// ============================================================================

#[test]
#[serial]
fn spectator_session_implements_session_trait() -> Result<(), FortressError> {
    // Arrange
    let host_port = PortAllocator::next_port();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let (socket, _addr) = bind_socket_ephemeral()?;
    let mut sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Act — call through the trait-generic helper
    let state = call_via_trait(&mut sess);

    // Assert — Spectator starts in Synchronizing (waiting for host)
    assert_eq!(state, SessionState::Synchronizing);
    Ok(())
}

// ============================================================================
// SyncTestSession implements Session
// ============================================================================

#[test]
fn synctest_session_implements_session_trait() -> Result<(), FortressError> {
    // Arrange
    let mut sess = SessionBuilder::<StubConfig>::new().start_synctest_session()?;

    // Act — call through the trait-generic helper
    let state = call_via_trait(&mut sess);

    // Assert — SyncTest uses the default `current_state` which returns Running
    assert_eq!(state, SessionState::Running);
    Ok(())
}

#[test]
fn synctest_advance_frame_via_trait() -> Result<(), FortressError> {
    // Arrange
    let mut sess = SessionBuilder::<StubConfig>::new().start_synctest_session()?;

    // Act — add inputs and advance through the trait interface
    Session::add_local_input(&mut sess, PlayerHandle::new(0), StubInput::default())?;
    Session::add_local_input(&mut sess, PlayerHandle::new(1), StubInput::default())?;
    let requests = Session::advance_frame(&mut sess)?;

    // Assert — should have at least one request (advance frame)
    assert!(!requests.is_empty());
    Ok(())
}

// ============================================================================
// Generic function taking `&mut impl Session<T>` works with all types
// ============================================================================

#[test]
#[serial]
fn generic_function_works_with_all_session_types() -> Result<(), FortressError> {
    // P2P
    let (socket, _addr0) = bind_socket_ephemeral()?;
    let [port1] = PortAllocator::next_ports::<1>();
    let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port1);
    let mut p2p = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;
    let p2p_state = call_via_trait(&mut p2p);
    assert_eq!(p2p_state, SessionState::Synchronizing);

    // Spectator
    let host_port = PortAllocator::next_port();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let (socket, _addr) = bind_socket_ephemeral()?;
    let mut spectator = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");
    let spec_state = call_via_trait(&mut spectator);
    assert_eq!(spec_state, SessionState::Synchronizing);

    // SyncTest
    let mut synctest = SessionBuilder::<StubConfig>::new().start_synctest_session()?;
    let sync_state = call_via_trait(&mut synctest);
    assert_eq!(sync_state, SessionState::Running);

    Ok(())
}

// ============================================================================
// Spectator add_local_input returns NotSupported (via trait)
// ============================================================================

#[test]
#[serial]
fn spectator_add_local_input_returns_not_supported_via_trait() -> Result<(), FortressError> {
    // Arrange
    let host_port = PortAllocator::next_port();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let (socket, _addr) = bind_socket_ephemeral()?;
    let mut sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Act — call add_local_input through the trait interface
    let result = Session::add_local_input(&mut sess, PlayerHandle::new(0), StubInput::default());

    // Assert — should be NotSupported
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::NotSupported {
                    operation: "add_local_input"
                }
            }
        ),
        "expected NotSupported, got: {err:?}"
    );
    Ok(())
}

// ============================================================================
// Default current_state returns Running for SyncTest (via trait)
// ============================================================================

#[test]
fn synctest_current_state_returns_running_via_trait() -> Result<(), FortressError> {
    // Arrange
    let sess = SessionBuilder::<StubConfig>::new().start_synctest_session()?;

    // Act — call current_state through the trait interface
    let state = Session::current_state(&sess);

    // Assert — SyncTest uses the default implementation: Running
    assert_eq!(state, SessionState::Running);
    Ok(())
}

// ============================================================================
// Default poll_remote_clients is no-op for SyncTest (via trait)
// ============================================================================

#[test]
fn synctest_poll_remote_clients_is_noop_via_trait() -> Result<(), FortressError> {
    // Arrange
    let mut sess = SessionBuilder::<StubConfig>::new().start_synctest_session()?;

    // Act — calling poll_remote_clients should not panic or change state
    Session::poll_remote_clients(&mut sess);

    // Assert — state is unchanged (still Running)
    assert_eq!(Session::current_state(&sess), SessionState::Running);
    Ok(())
}

// ============================================================================
// events() returns empty for SyncTest (via trait)
// ============================================================================

#[test]
fn synctest_events_returns_empty_via_trait() -> Result<(), FortressError> {
    // Arrange
    let mut sess = SessionBuilder::<StubConfig>::new().start_synctest_session()?;

    // Act — drain events through the trait interface
    let event_count = Session::events(&mut sess).count();

    // Assert — no events have been queued
    assert_eq!(event_count, 0);
    Ok(())
}

// ============================================================================
// local_player_handle_required returns error for spectator (via trait)
// ============================================================================

#[test]
#[serial]
fn spectator_local_player_handle_required_returns_error_via_trait() -> Result<(), FortressError> {
    // Arrange
    let host_port = PortAllocator::next_port();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let (socket, _addr) = bind_socket_ephemeral()?;
    let sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Act — call local_player_handle_required through the trait interface
    let result = Session::local_player_handle_required(&sess);

    // Assert — spectators have no local player
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::NotSupported {
                    operation: "local_player_handle_required"
                }
            }
        ),
        "expected NotSupported, got: {err:?}"
    );
    Ok(())
}

// ============================================================================
// Compile-time assertion: Session is object-safe
// ============================================================================

/// This function exists solely to verify at compile time that the `Session`
/// trait is object-safe (can be used with `dyn Session<StubConfig>`).
/// If someone adds a method with `Self` return or method-level generics,
/// this file will fail to compile.
#[allow(dead_code)]
fn _assert_object_safe(_: &dyn Session<StubConfig>) {}
