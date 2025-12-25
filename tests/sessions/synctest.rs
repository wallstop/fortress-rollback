//! SyncTest session integration tests.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use crate::common::stubs::{GameStub, RandomChecksumGameStub, StubConfig, StubInput};
use fortress_rollback::{FortressError, FortressRequest, PlayerHandle, SessionBuilder};

#[test]
fn test_create_session() {
    assert!(SessionBuilder::<StubConfig>::new()
        .start_synctest_session()
        .is_ok());
}

#[test]
fn test_advance_frame_no_rollbacks() -> Result<(), FortressError> {
    let check_distance = 0;
    let mut stub = GameStub::new();
    let mut sess = SessionBuilder::new()
        .with_check_distance(check_distance)
        .start_synctest_session()?;

    for i in 0..200 {
        sess.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess.add_local_input(PlayerHandle::new(1), StubInput { inp: i })?;
        let requests = sess.advance_frame()?;
        assert_eq!(requests.len(), 1); // only advance
        stub.handle_requests(requests);
        assert_eq!(stub.gs.frame, i as i32 + 1); // frame should have advanced
    }

    Ok(())
}

#[test]
fn test_advance_frame_with_rollbacks() -> Result<(), FortressError> {
    let check_distance = 2;
    let mut stub = GameStub::new();
    let mut sess = SessionBuilder::new()
        .with_check_distance(check_distance)
        .start_synctest_session()?;

    for i in 0..200 {
        sess.add_local_input(PlayerHandle::new(0), StubInput { inp: i as u32 })?;
        sess.add_local_input(PlayerHandle::new(1), StubInput { inp: i as u32 })?;
        let requests = sess.advance_frame()?;
        if i <= check_distance {
            assert_eq!(requests.len(), 2); // save, advance
            assert!(matches!(requests[0], FortressRequest::SaveGameState { .. }));
            assert!(matches!(requests[1], FortressRequest::AdvanceFrame { .. }));
        } else {
            assert_eq!(requests.len(), 6); // load, advance, save, advance, save, advance
            assert!(matches!(requests[0], FortressRequest::LoadGameState { .. })); // rollback
            assert!(matches!(requests[1], FortressRequest::AdvanceFrame { .. })); // rollback
            assert!(matches!(requests[2], FortressRequest::SaveGameState { .. })); // rollback
            assert!(matches!(requests[3], FortressRequest::AdvanceFrame { .. })); // rollback
            assert!(matches!(requests[4], FortressRequest::SaveGameState { .. }));
            assert!(matches!(requests[5], FortressRequest::AdvanceFrame { .. }));
        }

        stub.handle_requests(requests);
        assert_eq!(stub.gs.frame, i as i32 + 1); // frame should have advanced
    }

    Ok(())
}

#[test]
fn test_advance_frames_with_delayed_input() -> Result<(), FortressError> {
    let check_distance = 7;
    let mut stub = GameStub::new();
    let mut sess = SessionBuilder::new()
        .with_check_distance(check_distance)
        .with_input_delay(2)
        .start_synctest_session()?;

    for i in 0..200 {
        sess.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess.add_local_input(PlayerHandle::new(1), StubInput { inp: i })?;
        let requests = sess.advance_frame()?;
        stub.handle_requests(requests);
        assert_eq!(stub.gs.frame, i as i32 + 1); // frame should have advanced
    }

    Ok(())
}

#[test]
#[should_panic(expected = "MismatchedChecksum")]
fn test_advance_frames_with_random_checksums() {
    let mut stub = RandomChecksumGameStub::new();
    let mut sess = SessionBuilder::new()
        .with_input_delay(2)
        .start_synctest_session()
        .unwrap();

    for i in 0..200 {
        sess.add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess.add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();
        let requests = sess.advance_frame().unwrap(); // this should give a MismatchedChecksum error
        stub.handle_requests(requests);
        assert_eq!(stub.gs.frame, i as i32 + 1);
    }
}

/// Test deep rollback scenario with maximum prediction window.
/// Uses check_distance = 7 to force rollbacks spanning many frames.
#[test]
fn test_deep_rollback_scenario() -> Result<(), FortressError> {
    let check_distance = 7; // Maximum typical prediction window
    let mut stub = GameStub::new();
    let mut sess = SessionBuilder::new()
        .with_check_distance(check_distance)
        .start_synctest_session()?;

    for i in 0..500 {
        sess.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess.add_local_input(PlayerHandle::new(1), StubInput { inp: i })?;
        let requests = sess.advance_frame()?;

        // After initial warmup, we should see rollback patterns
        if i > check_distance as u32 {
            // Should have: load, advance*check_distance, save, advance
            let load_count = requests
                .iter()
                .filter(|r| matches!(r, FortressRequest::LoadGameState { .. }))
                .count();
            let save_count = requests
                .iter()
                .filter(|r| matches!(r, FortressRequest::SaveGameState { .. }))
                .count();
            let advance_count = requests
                .iter()
                .filter(|r| matches!(r, FortressRequest::AdvanceFrame { .. }))
                .count();

            assert_eq!(load_count, 1, "Should load once per frame after warmup");
            assert!(save_count >= 1, "Should save at least once");
            assert!(
                advance_count >= check_distance,
                "Should advance at least check_distance times"
            );
        }

        stub.handle_requests(requests);
        assert_eq!(stub.gs.frame, i as i32 + 1);
    }

    Ok(())
}

/// Test frequent rollbacks with small check distance and high frame count.
/// Verifies rollback consistency over many iterations.
#[test]
fn test_frequent_rollback_consistency() -> Result<(), FortressError> {
    let check_distance = 1; // Minimum rollback distance for maximum frequency
    let mut stub = GameStub::new();
    let mut sess = SessionBuilder::new()
        .with_check_distance(check_distance)
        .start_synctest_session()?;

    let mut rollback_count = 0;
    for i in 0..1000 {
        sess.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess.add_local_input(PlayerHandle::new(1), StubInput { inp: i })?;
        let requests = sess.advance_frame()?;

        // Count rollbacks (LoadGameState requests)
        rollback_count += requests
            .iter()
            .filter(|r| matches!(r, FortressRequest::LoadGameState { .. }))
            .count();

        stub.handle_requests(requests);
        assert_eq!(stub.gs.frame, i as i32 + 1);
    }

    // With check_distance=1, we should have many rollbacks (one per frame after warmup)
    assert!(
        rollback_count >= 990,
        "Should have frequent rollbacks with check_distance=1, got {}",
        rollback_count
    );

    Ok(())
}

/// Test rollback with input delay combined.
/// This tests the interaction between input delay and rollback mechanics.
#[test]
fn test_rollback_with_varying_input_delay() -> Result<(), FortressError> {
    // Test multiple input delay values
    for input_delay in [0, 1, 2, 4] {
        let check_distance = 4;
        let mut stub = GameStub::new();
        let mut sess = SessionBuilder::new()
            .with_check_distance(check_distance)
            .with_input_delay(input_delay)
            .start_synctest_session()?;

        for i in 0..100 {
            sess.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
            sess.add_local_input(PlayerHandle::new(1), StubInput { inp: i })?;
            let requests = sess.advance_frame()?;
            stub.handle_requests(requests);
            assert_eq!(
                stub.gs.frame,
                i as i32 + 1,
                "Frame mismatch with input_delay={}",
                input_delay
            );
        }
    }

    Ok(())
}
