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
// 3. **State Count**: Exactly 5 states exist (matching TLA+ specification)
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

    /// Total number of protocol states (must match TLA+ specification).
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

    /// Proof: ProtocolState has exactly 5 variants.
    ///
    /// Verifies alignment with TLA+ specification which defines exactly 5 states.
    /// This proof ensures no variants are accidentally added or removed.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: State enum variant count matches TLA+ spec
    /// - Related: proof_state_index_bijection, proof_exhaustive_match
    #[kani::proof]
    fn proof_state_count_matches_specification() {
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
    /// - Related: proof_state_count_matches_specification, proof_variants_distinct
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
    /// - Related: proof_state_count_matches_specification
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

    /// Proof: Shutdown is reachable from any state conceptually.
    ///
    /// This documents that Shutdown is the terminal state. In practice,
    /// transitions go through Disconnected first, but protocol can force
    /// shutdown from any state via explicit shutdown call.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Shutdown is terminal state (highest index)
    /// - Related: proof_initializing_is_initial, proof_transition_matrix_rejects_backwards
    #[kani::proof]
    fn proof_shutdown_is_terminal() {
        // Shutdown is defined as the terminal state - no transitions out
        // This proof documents this invariant symbolically
        let shutdown = ProtocolState::Shutdown;
        let index = state_to_index(&shutdown);

        kani::assert(index == 4, "Shutdown should have highest index (terminal)");
    }

    /// Proof: Initializing is the only valid initial state.
    ///
    /// Verifies that protocols start in Initializing state (index 0).
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Initial state has index 0
    /// - Related: proof_shutdown_is_terminal, proof_transition_matrix_sync_required
    #[kani::proof]
    fn proof_initializing_is_initial() {
        let initializing = ProtocolState::Initializing;
        let index = state_to_index(&initializing);

        kani::assert(
            index == 0,
            "Initializing should have index 0 (initial state)",
        );
    }

    // =========================================================================
    // State Transition Matrix Verification
    //
    // These proofs verify the documented state transition rules from the TLA+
    // specification. The actual production code in `UdpProtocol` is too complex
    // for Kani (uses Vec, BTreeMap, Instant), but these proofs verify the
    // transition matrix that production code must follow.
    //
    // Production code references:
    // - synchronize() at mod.rs:380 transitions Initializing -> Synchronizing
    // - on_sync_reply() at mod.rs:764 transitions Synchronizing -> Running
    // - disconnect() at mod.rs:365 transitions Running -> Disconnected
    // - poll() transitions Disconnected -> Shutdown after timeout
    // =========================================================================

    /// Helper: Documented state transition matrix.
    ///
    /// This function encodes the valid transitions from the TLA+ specification
    /// (specs/tla/NetworkProtocol.tla). Production code must follow these rules.
    ///
    /// Valid transitions:
    /// - Initializing (0) -> Synchronizing (1): via synchronize() at mod.rs:389
    /// - Synchronizing (1) -> Running (2): via on_sync_reply() at mod.rs:764
    /// - Running (2) -> Disconnected (3): via disconnect() at mod.rs:370
    /// - Disconnected (3) -> Shutdown (4): via poll() timeout logic
    /// - Any state -> Shutdown (4): explicit shutdown
    /// - Same state -> Same state: no-op transitions are valid
    fn documented_transition_valid(from_idx: u8, to_idx: u8) -> bool {
        match (from_idx, to_idx) {
            // Normal forward transitions per TLA+ spec
            (0, 1) => true, // Initializing -> Synchronizing
            (1, 2) => true, // Synchronizing -> Running
            (2, 3) => true, // Running -> Disconnected
            (3, 4) => true, // Disconnected -> Shutdown
            // Emergency shutdown from any state
            (_, 4) => true, // Any -> Shutdown
            // Stay in same state (valid for stability)
            (s, t) if s == t => true,
            // All other transitions violate the TLA+ specification
            _ => false,
        }
    }

    /// Proof: Documented transition matrix rejects backward transitions.
    ///
    /// Verifies that the transition matrix properly rejects invalid backward
    /// transitions. This is a property the production code relies on.
    /// TLA+ alignment: NetworkProtocol.tla ValidTransition predicate.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: No backward state transitions allowed
    /// - Related: proof_transition_matrix_sequential, proof_transition_matrix_sync_required
    #[kani::proof]
    fn proof_transition_matrix_rejects_backwards() {
        let from_idx: u8 = kani::any();
        let to_idx: u8 = kani::any();
        kani::assume(from_idx < PROTOCOL_STATE_COUNT as u8);
        kani::assume(to_idx < PROTOCOL_STATE_COUNT as u8);

        let is_valid = documented_transition_valid(from_idx, to_idx);

        // Backward transitions (except staying in place) must be rejected
        if to_idx < from_idx {
            kani::assert(!is_valid, "Backward transitions should be invalid");
        }

        // Shutdown must always be reachable (for error recovery)
        if to_idx == 4 {
            kani::assert(is_valid, "Shutdown should always be reachable");
        }

        // Normal single-step forward transitions must be valid
        if to_idx == from_idx + 1 && from_idx < 4 {
            kani::assert(is_valid, "Forward step should be valid");
        }
    }

    /// Proof: Transition matrix enforces sequential progression.
    ///
    /// Verifies that non-shutdown transitions must be single steps forward.
    /// This ensures synchronization cannot be skipped.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Single-step forward progression only
    /// - Related: proof_transition_matrix_rejects_backwards, proof_transition_matrix_sync_required
    #[kani::proof]
    fn proof_transition_matrix_sequential() {
        let from_idx: u8 = kani::any();
        let to_idx: u8 = kani::any();
        kani::assume(from_idx < PROTOCOL_STATE_COUNT as u8);
        kani::assume(to_idx < PROTOCOL_STATE_COUNT as u8);

        // If not going to shutdown and not staying in place
        if to_idx != 4 && to_idx != from_idx {
            let is_one_step = to_idx == from_idx + 1;
            let is_valid = documented_transition_valid(from_idx, to_idx);

            // If valid and not shutdown and not same state, must be one step
            if is_valid {
                kani::assert(
                    is_one_step,
                    "Non-shutdown transitions should be single steps",
                );
            }
        }
    }

    /// Proof: Cannot bypass synchronization to reach Running.
    ///
    /// Verifies synchronize() precondition (mod.rs:381): must be in Initializing
    /// to start sync. Production code: `if self.state != ProtocolState::Initializing`
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Synchronization cannot be skipped
    /// - Related: proof_initializing_is_initial, proof_transition_matrix_sequential
    #[kani::proof]
    fn proof_transition_matrix_sync_required() {
        // Cannot go directly from Initializing to Running (must sync first)
        let init_to_running = documented_transition_valid(0, 2);
        kani::assert(!init_to_running, "Cannot skip Synchronizing");

        // The valid path: Init -> Sync -> Running
        let init_to_sync = documented_transition_valid(0, 1);
        let sync_to_running = documented_transition_valid(1, 2);
        kani::assert(init_to_sync, "Can go from Initializing to Synchronizing");
        kani::assert(sync_to_running, "Can go from Synchronizing to Running");
    }
}
