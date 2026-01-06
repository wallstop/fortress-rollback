//! SyncTest session integration tests with enum-based inputs.
//!
//! These tests verify that enum-based input types work correctly with SyncTest sessions.
//! The actual test logic is implemented in generic helpers in `test_utils.rs`.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use crate::common::run_synctest_with_delayed_input;
use crate::common::stubs_enum::{EnumInput, GameStubEnum, StubEnumConfig};
use fortress_rollback::FortressError;

/// Test SyncTest frame advancement with enum-based inputs and delayed input.
///
/// This test verifies that the SyncTest session correctly handles enum input types
/// with input delay by using the shared generic test helper.
#[test]
fn test_enum_advance_frames_with_delayed_input() -> Result<(), FortressError> {
    let inputs = [EnumInput::Val1, EnumInput::Val2];
    run_synctest_with_delayed_input::<StubEnumConfig, GameStubEnum>(
        7, // check_distance
        2, // input_delay
        |i| inputs[i as usize % inputs.len()],
        200, // num_frames
    )
}
