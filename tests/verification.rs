//! Verification integration tests.
//!
//! This module contains formal verification and property-based tests:
//! - Determinism tests - verifying deterministic behavior
//! - Invariant tests - internal invariant checking
//! - Property tests - property-based testing with proptest
//! - Metamorphic tests - metamorphic testing relationships
//! - Z3 verification - SMT solver verification (requires z3-verification feature)

// Verification test modules
mod verification {
    pub mod determinism;
    pub mod invariants;
    pub mod metamorphic;
    pub mod property;
    pub mod z3;
}
