//! P2P session integration tests with enum-based inputs.
//!
//! These tests verify that enum-based input types work correctly with P2P sessions.
//! The actual test logic is implemented in generic helpers in `test_utils.rs`.
//!
//! # Port Allocation
//!
//! This test file uses `PortAllocator` for thread-safe port allocation.
//! All ports are dynamically allocated to avoid conflicts with other tests.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::ip_constant
)]

use crate::common::run_p2p_frame_advancement_test;
use crate::common::stubs_enum::{EnumInput, GameStubEnum, StubEnumConfig};
use crate::common::PortAllocator;
use fortress_rollback::FortressError;
use serial_test::serial;

/// Test P2P frame advancement with enum-based inputs.
///
/// This test verifies that the P2P session correctly handles enum input types
/// by using the shared generic test helper.
#[test]
#[serial]
fn test_advance_frame_p2p_sessions_enum() -> Result<(), FortressError> {
    let (port1, port2) = PortAllocator::next_pair();
    run_p2p_frame_advancement_test::<StubEnumConfig, GameStubEnum>(
        port1,
        port2,
        |i| {
            if i % 2 == 0 {
                EnumInput::Val1
            } else {
                EnumInput::Val2
            }
        },
        10,
    )
}
