//! Common test infrastructure shared across integration tests.
//!
//! This module provides:
//! - `stubs`: Game stub implementations with struct-based inputs
//! - `stubs_enum`: Game stub implementations with enum-based inputs
//! - `test_utils`: Shared constants, helpers, and synchronization utilities
//!
//! # Usage
//!
//! From any integration test file:
//! ```ignore
//! mod common;
//! use common::stubs::{GameStub, StubConfig, StubInput};
//! use common::test_utils::{synchronize_sessions, test_addr, MAX_SYNC_ITERATIONS};
//! // Or use the re-exported items:
//! use common::{synchronize_sessions, test_addr};
//! ```

pub mod stubs;
pub mod stubs_enum;
pub mod test_utils;

// Re-export commonly used items for convenience.
// These are public utilities for integration tests - allow unused until tests adopt them.
#[allow(unused_imports)]
pub use test_utils::{
    assert_spectator_synchronized, calculate_hash, create_chaos_socket, drain_sync_events,
    poll_with_sleep, run_p2p_frame_advancement_test, run_synctest_with_delayed_input,
    synchronize_sessions, synchronize_spectator, test_addr, GameStubHandler, PortAllocator,
    SyncConfig, SyncResult, MAX_SYNC_ITERATIONS, POLL_INTERVAL, SYNC_TIMEOUT,
};
