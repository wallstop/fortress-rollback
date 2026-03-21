//! P2P session integration tests with enum-based inputs.
//!
//! These tests verify that enum-based input types work correctly with P2P sessions.
//! The actual test logic is implemented in generic helpers in `test_utils.rs`.
//!
//! # Deterministic Testing
//!
//! This test file uses `ChannelSocket` (in-memory sockets) and `TestClock`
//! (virtual time) for fully deterministic execution.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::ip_constant
)]

use crate::common::run_p2p_frame_advancement_test_deterministic;
use crate::common::stubs_enum::{EnumInput, GameStubEnum, StubEnumConfig};
use fortress_rollback::FortressError;

/// Test P2P frame advancement with enum-based inputs.
///
/// This test verifies that the P2P session correctly handles enum input types
/// by using the shared generic deterministic test helper with ChannelSocket
/// and TestClock for fully deterministic execution.
#[test]
fn test_advance_frame_p2p_sessions_enum() -> Result<(), FortressError> {
    run_p2p_frame_advancement_test_deterministic::<StubEnumConfig, GameStubEnum>(
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
