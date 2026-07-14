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
/// # Verification Coverage
/// - Kani proofs check the five-state enum representation.
/// - The `SyncHandshakeV1` TLA+ family models bounded two-peer, two-field configuration-handshake
///   safety and fair-delivery convergence.
/// - `PeerDrop.tla` models the halt-versus-continue peer-drop policy.
///
/// These artifacts are not a runtime trace-refinement proof; protocol and session tests exercise
/// the corresponding production transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl ProtocolState {
    /// Returns the state name as a static string slice.
    ///
    /// This is useful for error messages and logging.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Initializing => "Initializing",
            Self::Synchronizing => "Synchronizing",
            Self::Running => "Running",
            Self::Disconnected => "Disconnected",
            Self::Shutdown => "Shutdown",
        }
    }
}

impl std::fmt::Display for ProtocolState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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
    // Copy Trait Tests
    // ==========================================================================

    #[test]
    fn protocol_state_copy() {
        let state = ProtocolState::Running;
        let copied = state;
        assert_eq!(state, copied);
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
    // Display Trait Tests
    // ==========================================================================

    #[test]
    fn protocol_state_display_all_variants() {
        assert_eq!(format!("{}", ProtocolState::Initializing), "Initializing");
        assert_eq!(format!("{}", ProtocolState::Synchronizing), "Synchronizing");
        assert_eq!(format!("{}", ProtocolState::Running), "Running");
        assert_eq!(format!("{}", ProtocolState::Disconnected), "Disconnected");
        assert_eq!(format!("{}", ProtocolState::Shutdown), "Shutdown");
    }

    #[test]
    fn protocol_state_display_matches_as_str() {
        for state in [
            ProtocolState::Initializing,
            ProtocolState::Synchronizing,
            ProtocolState::Running,
            ProtocolState::Disconnected,
            ProtocolState::Shutdown,
        ] {
            assert_eq!(format!("{}", state), state.as_str());
        }
    }
}

// =============================================================================
// Kani Formal Verification Proofs
//
// These proofs verify fundamental properties of the ProtocolState enum using
// exhaustive symbolic verification. Kani explores ALL possible values and states.
//
// ## Verified Invariants
//
// 1. **Discriminant Uniqueness**: Each variant has a distinct discriminant value
// 2. **Exhaustive Matching**: All variants can be matched exhaustively
// 3. **State Index Domain**: The five current variants map to indices 0 through 4
// 4. **Clone Correctness**: Cloning preserves equality
// 5. **PartialEq Reflexivity**: Every state equals itself
//
// ## Unwind Bounds
//
// ProtocolState is a simple unit enum (no data), so no loop unwinding is needed.
// =============================================================================
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// Total number of current protocol-state variants.
    const PROTOCOL_STATE_COUNT: usize = 5;

    /// Helper to convert any u8 to a ProtocolState (for exhaustive testing).
    /// Returns None for values outside the valid range.
    fn state_from_index(index: u8) -> Option<ProtocolState> {
        match index {
            0 => Some(ProtocolState::Initializing),
            1 => Some(ProtocolState::Synchronizing),
            2 => Some(ProtocolState::Running),
            3 => Some(ProtocolState::Disconnected),
            4 => Some(ProtocolState::Shutdown),
            _ => None,
        }
    }

    /// Helper to convert a ProtocolState to its index.
    fn state_to_index(state: &ProtocolState) -> u8 {
        match state {
            ProtocolState::Initializing => 0,
            ProtocolState::Synchronizing => 1,
            ProtocolState::Running => 2,
            ProtocolState::Disconnected => 3,
            ProtocolState::Shutdown => 4,
        }
    }

    /// Proof: the index helper accepts exactly the five current variant indices.
    ///
    /// This proof keeps the helper's accepted index domain explicit. The exhaustive matches below
    /// make a newly added enum variant a compile error until the helper is updated.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: State-helper index domain
    /// - Related: proof_state_index_bijection, proof_exhaustive_match
    #[kani::proof]
    fn proof_state_index_domain() {
        let index: u8 = kani::any();

        // Only indices 0-4 should produce valid states
        if index < PROTOCOL_STATE_COUNT as u8 {
            let state = state_from_index(index);
            kani::assert(state.is_some(), "Valid index should produce a state");
        } else {
            let state = state_from_index(index);
            kani::assert(state.is_none(), "Invalid index should produce None");
        }
    }

    /// Proof: State-to-index conversion is bijective (one-to-one and onto).
    ///
    /// Verifies that state_to_index and state_from_index are inverses.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Index round-trip correctness
    /// - Related: proof_state_index_domain, proof_variants_distinct
    #[kani::proof]
    fn proof_state_index_bijection() {
        let index: u8 = kani::any();
        kani::assume(index < PROTOCOL_STATE_COUNT as u8);

        // Round-trip: index -> state -> index should preserve the value
        let state = state_from_index(index);
        kani::assert(state.is_some(), "Valid index should produce a state");

        if let Some(s) = state {
            let recovered_index = state_to_index(&s);
            kani::assert(
                recovered_index == index,
                "Round-trip through state should preserve index",
            );
        }
    }

    /// Proof: Clone produces equal state.
    ///
    /// Verifies that cloning a ProtocolState produces a value equal to the original.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Clone trait correctness
    /// - Related: proof_partial_eq_symmetric
    #[kani::proof]
    fn proof_clone_correctness() {
        let index: u8 = kani::any();
        kani::assume(index < PROTOCOL_STATE_COUNT as u8);

        if let Some(state) = state_from_index(index) {
            let cloned = state.clone();
            kani::assert(state == cloned, "Cloned state should equal original");
        }
    }

    /// Proof: PartialEq is symmetric for ProtocolState.
    ///
    /// Verifies that if state_a == state_b, then state_b == state_a.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Equality symmetry property
    /// - Related: proof_clone_correctness, proof_variants_distinct
    #[kani::proof]
    fn proof_protocol_state_partial_eq_symmetric() {
        let index_a: u8 = kani::any();
        let index_b: u8 = kani::any();
        kani::assume(index_a < PROTOCOL_STATE_COUNT as u8);
        kani::assume(index_b < PROTOCOL_STATE_COUNT as u8);

        if let (Some(state_a), Some(state_b)) =
            (state_from_index(index_a), state_from_index(index_b))
        {
            if state_a == state_b {
                kani::assert(state_b == state_a, "Equality should be symmetric");
            }
        }
    }

    /// Proof: Different indices produce unequal states.
    ///
    /// Verifies that each state variant is distinct from all others.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Variant distinctness
    /// - Related: proof_state_index_bijection, proof_partial_eq_symmetric
    #[kani::proof]
    fn proof_variants_distinct() {
        let index_a: u8 = kani::any();
        let index_b: u8 = kani::any();
        kani::assume(index_a < PROTOCOL_STATE_COUNT as u8);
        kani::assume(index_b < PROTOCOL_STATE_COUNT as u8);
        kani::assume(index_a != index_b);

        if let (Some(state_a), Some(state_b)) =
            (state_from_index(index_a), state_from_index(index_b))
        {
            kani::assert(
                state_a != state_b,
                "Different indices should produce different states",
            );
        }
    }

    /// Proof: Exhaustive pattern matching covers all variants.
    ///
    /// Verifies that every valid state can be matched. This proof implicitly
    /// verifies that no new variants have been added without updating the match.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Match exhaustiveness for all state variants
    /// - Related: proof_state_index_domain
    #[kani::proof]
    fn proof_exhaustive_match() {
        let index: u8 = kani::any();
        kani::assume(index < PROTOCOL_STATE_COUNT as u8);

        if let Some(state) = state_from_index(index) {
            // This match must be exhaustive - compiler will fail if variant is missing
            let matched_index = match state {
                ProtocolState::Initializing => 0u8,
                ProtocolState::Synchronizing => 1,
                ProtocolState::Running => 2,
                ProtocolState::Disconnected => 3,
                ProtocolState::Shutdown => 4,
            };

            kani::assert(
                matched_index == index,
                "Exhaustive match should return correct index",
            );
        }
    }

    /// Proof: Shutdown uses the documented helper index.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Shutdown helper index
    /// - Related: proof_initializing_variant_index
    #[kani::proof]
    fn proof_shutdown_variant_index() {
        let shutdown = ProtocolState::Shutdown;
        let index = state_to_index(&shutdown);

        kani::assert(index == 4, "Shutdown should use index 4");
    }

    /// Proof: Initializing uses the documented helper index.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Initializing helper index
    /// - Related: proof_shutdown_variant_index
    #[kani::proof]
    fn proof_initializing_variant_index() {
        let initializing = ProtocolState::Initializing;
        let index = state_to_index(&initializing);

        kani::assert(index == 0, "Initializing should use index 0");
    }
}
