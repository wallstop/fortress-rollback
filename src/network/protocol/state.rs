//! Protocol state machine for UDP communication.
//!
//! This module contains the state machine for the UDP protocol layer.
//!
//! # State Machine Diagram
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     UDP Protocol State Machine                          │
//! │                                                                         │
//! │   ┌──────────────┐     synchronize()     ┌───────────────┐              │
//! │   │ Initializing │ ─────────────────────►│ Synchronizing │              │
//! │   └──────────────┘                       └───────┬───────┘              │
//! │                                                  │                      │
//! │                                    sync complete │                      │
//! │                                  (roundtrips = 0)│                      │
//! │                                                  ▼                      │
//! │                                          ┌─────────────┐                │
//! │                                          │   Running   │◄──┐            │
//! │                                          └──────┬──────┘   │            │
//! │                                                 │          │ resume     │
//! │                               disconnect_timeout│          │            │
//! │                               or disconnect_req │   ┌──────┴──────┐     │
//! │                                                 │   │   Network   │     │
//! │                                                 │   │ Interrupted │     │
//! │                                                 │   └─────────────┘     │
//! │                                                 ▼                       │
//! │                                         ┌──────────────┐                │
//! │                                         │ Disconnected │                │
//! │                                         └───────┬──────┘                │
//! │                                                 │                       │
//! │                                   shutdown_delay│                       │
//! │                                                 ▼                       │
//! │                                          ┌──────────┐                   │
//! │                                          │ Shutdown │                   │
//! │                                          └──────────┘                   │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## State Transitions
//!
//! | From          | To            | Trigger                                    |
//! |---------------|---------------|--------------------------------------------|
//! | Initializing  | Synchronizing | `synchronize()` called                     |
//! | Synchronizing | Running       | All sync roundtrips completed              |
//! | Running       | Disconnected  | Disconnect timeout or peer disconnect req  |
//! | Disconnected  | Shutdown      | Shutdown delay elapsed                     |
//!
//! ## Events Emitted
//!
//! - **Synchronizing**: Progress during sync (total, count, elapsed_ms)
//! - **Synchronized**: Sync complete, entering Running state
//! - **NetworkInterrupted**: No packets received for `disconnect_notify_start`
//! - **NetworkResumed**: Packets received after interruption
//! - **Disconnected**: Connection lost, entering Disconnected state
//! - **SyncTimeout**: Sync took longer than `sync_timeout` (if configured)

/// Internal state machine for the UDP protocol.
///
/// # State Machine
///
/// The protocol progresses through states in a linear fashion:
///
/// ```text
/// Initializing ──► Synchronizing ──► Running ──► Disconnected ──► Shutdown
/// ```
///
/// ## State Descriptions
///
/// - **Initializing**: Protocol created but not yet started. Call `synchronize()` to begin.
/// - **Synchronizing**: Exchanging sync packets with peer to establish connection.
///   Emits `Synchronizing` events to track progress.
/// - **Running**: Normal operation. Exchanges game inputs, quality reports, and checksums.
///   Can emit `NetworkInterrupted` / `NetworkResumed` events.
/// - **Disconnected**: Peer connection lost. Waiting for shutdown delay before cleanup.
/// - **Shutdown**: Terminal state. Protocol is fully stopped, all messages dropped.
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
    ///
    /// In this state, the protocol is waiting for `synchronize()` to be called.
    /// No messages are sent or processed.
    ///
    /// **Transition**: `synchronize()` → `Synchronizing`
    Initializing,

    /// Currently synchronizing with the peer.
    ///
    /// The protocol exchanges `SyncRequest` / `SyncReply` packets to establish
    /// the connection. The number of required roundtrips is configured via
    /// `SyncConfig::num_sync_packets`.
    ///
    /// **Events emitted**:
    /// - `Synchronizing { total, count, elapsed_ms }` — progress updates
    /// - `SyncTimeout { elapsed_ms }` — if sync exceeds configured timeout
    ///
    /// **Transition**: All roundtrips complete → `Running`
    Synchronizing,

    /// Normal operation, exchanging game inputs.
    ///
    /// This is the main operational state where the protocol:
    /// - Sends and receives game inputs
    /// - Exchanges quality reports for frame advantage calculation
    /// - Sends checksum reports for desync detection
    /// - Monitors connection health
    ///
    /// **Events emitted**:
    /// - `NetworkInterrupted { disconnect_timeout }` — no packets for too long
    /// - `NetworkResumed` — packets received after interruption
    /// - `Disconnected` — disconnect timeout exceeded
    ///
    /// **Transition**: Disconnect timeout or peer request → `Disconnected`
    Running,

    /// Peer has disconnected.
    ///
    /// The protocol is waiting for `shutdown_delay` before transitioning to
    /// `Shutdown`. During this time, the protocol may still attempt to send
    /// final messages but will not process incoming messages normally.
    ///
    /// **Transition**: Shutdown delay elapsed → `Shutdown`
    Disconnected,

    /// Protocol has shut down.
    ///
    /// This is the terminal state. All queued messages are dropped and no
    /// further processing occurs. The protocol instance should be discarded.
    ///
    /// **Transition**: None (terminal state)
    Shutdown,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Variant Construction Tests
    // ==========================================================================

    #[test]
    fn protocol_state_initializing() {
        let state = ProtocolState::Initializing;
        assert!(matches!(state, ProtocolState::Initializing));
    }

    #[test]
    fn protocol_state_synchronizing() {
        let state = ProtocolState::Synchronizing;
        assert!(matches!(state, ProtocolState::Synchronizing));
    }

    #[test]
    fn protocol_state_running() {
        let state = ProtocolState::Running;
        assert!(matches!(state, ProtocolState::Running));
    }

    #[test]
    fn protocol_state_disconnected() {
        let state = ProtocolState::Disconnected;
        assert!(matches!(state, ProtocolState::Disconnected));
    }

    #[test]
    fn protocol_state_shutdown() {
        let state = ProtocolState::Shutdown;
        assert!(matches!(state, ProtocolState::Shutdown));
    }

    // ==========================================================================
    // Clone Trait Tests
    // ==========================================================================

    #[test]
    #[allow(clippy::redundant_clone)]
    fn protocol_state_clone() {
        let state = ProtocolState::Running;
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    // ==========================================================================
    // PartialEq Trait Tests
    // ==========================================================================

    #[test]
    fn protocol_state_equality_same_variant() {
        assert_eq!(ProtocolState::Initializing, ProtocolState::Initializing);
        assert_eq!(ProtocolState::Synchronizing, ProtocolState::Synchronizing);
        assert_eq!(ProtocolState::Running, ProtocolState::Running);
        assert_eq!(ProtocolState::Disconnected, ProtocolState::Disconnected);
        assert_eq!(ProtocolState::Shutdown, ProtocolState::Shutdown);
    }

    #[test]
    fn protocol_state_inequality_different_variants() {
        assert_ne!(ProtocolState::Initializing, ProtocolState::Synchronizing);
        assert_ne!(ProtocolState::Synchronizing, ProtocolState::Running);
        assert_ne!(ProtocolState::Running, ProtocolState::Disconnected);
        assert_ne!(ProtocolState::Disconnected, ProtocolState::Shutdown);
        assert_ne!(ProtocolState::Shutdown, ProtocolState::Initializing);
    }

    #[test]
    fn protocol_state_all_variants_distinct() {
        let variants = [
            ProtocolState::Initializing,
            ProtocolState::Synchronizing,
            ProtocolState::Running,
            ProtocolState::Disconnected,
            ProtocolState::Shutdown,
        ];

        // Each variant should be different from all others
        for i in 0..variants.len() {
            for j in 0..variants.len() {
                if i == j {
                    assert_eq!(variants[i], variants[j]);
                } else {
                    assert_ne!(variants[i], variants[j]);
                }
            }
        }
    }

    // ==========================================================================
    // Debug Trait Tests
    // ==========================================================================

    #[test]
    fn protocol_state_debug_format() {
        assert_eq!(format!("{:?}", ProtocolState::Initializing), "Initializing");
        assert_eq!(
            format!("{:?}", ProtocolState::Synchronizing),
            "Synchronizing"
        );
        assert_eq!(format!("{:?}", ProtocolState::Running), "Running");
        assert_eq!(format!("{:?}", ProtocolState::Disconnected), "Disconnected");
        assert_eq!(format!("{:?}", ProtocolState::Shutdown), "Shutdown");
    }

    // ==========================================================================
    // State Machine Transition Documentation Tests
    // ==========================================================================

    /// Documents the valid state transitions as per the TLA+ specification.
    #[test]
    fn protocol_state_transition_documentation() {
        // This test documents the valid state transitions.
        // The actual transition logic is in the Protocol struct.
        // Valid transitions:
        // Initializing -> Synchronizing (via synchronize())
        // Synchronizing -> Running (via complete_sync())
        // Running -> Disconnected (via disconnect())
        // Disconnected -> Shutdown (via poll() when disconnected)
        // Any state -> Shutdown (via explicit shutdown)

        // Verify all states can be constructed and matched
        let states = [
            ProtocolState::Initializing,
            ProtocolState::Synchronizing,
            ProtocolState::Running,
            ProtocolState::Disconnected,
            ProtocolState::Shutdown,
        ];

        // Each state should be in the array
        assert_eq!(states.len(), 5);
        assert!(matches!(states[0], ProtocolState::Initializing));
        assert!(matches!(states[4], ProtocolState::Shutdown));
    }
}
