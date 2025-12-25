//! SyncTest session integration tests with enum-based inputs.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use crate::common::stubs_enum::{EnumInput, GameStubEnum, StubEnumConfig};
use fortress_rollback::{FortressError, PlayerHandle, SessionBuilder};

#[test]
fn test_enum_advance_frames_with_delayed_input() -> Result<(), FortressError> {
    let check_distance = 7;
    let mut stub = GameStubEnum::new();
    let mut sess = SessionBuilder::<StubEnumConfig>::new()
        .with_check_distance(check_distance)
        .with_input_delay(2)
        .start_synctest_session()?;

    let inputs = [EnumInput::Val1, EnumInput::Val2];
    for i in 0..200 {
        let input = inputs[i % inputs.len()];
        sess.add_local_input(PlayerHandle::new(0), input)?;
        sess.add_local_input(PlayerHandle::new(1), input)?;
        let requests = sess.advance_frame()?;
        stub.handle_requests(requests);
        assert_eq!(stub.gs.frame, i as i32 + 1); // frame should have advanced
    }

    Ok(())
}
