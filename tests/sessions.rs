//! Session integration tests.
//!
//! This module contains integration tests for all session types:
//! - P2P sessions (with struct and enum inputs)
//! - Spectator sessions
//! - SyncTest sessions (with struct and enum inputs)

// Shared test infrastructure
#[path = "common/mod.rs"]
mod common;

// Session test modules
mod sessions {
    pub mod p2p;
    pub mod p2p_enum;
    pub mod spectator;
    pub mod synctest;
    pub mod synctest_enum;
}
