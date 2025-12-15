//! Protocol state machine for UDP communication.
//!
//! This module contains the state machine for the UDP protocol layer.

/// Internal state machine for the UDP protocol.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
///
/// # Formal Specification Alignment
/// - **TLA+**: State machine modeled in `specs/tla/NetworkProtocol.tla`
/// - **Verified properties**:
///   - Valid transitions: Initializing -> Synchronizing -> Running -> Disconnected -> Shutdown
///   - `sync_remaining` counter never negative (SyncRemainingNonNegative invariant)
///   - Only Running state processes game inputs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolState {
    /// Initial state before any communication.
    Initializing,
    /// Currently synchronizing with the peer.
    Synchronizing,
    /// Normal operation, exchanging game inputs.
    Running,
    /// Peer has disconnected.
    Disconnected,
    /// Protocol has shut down.
    Shutdown,
}
