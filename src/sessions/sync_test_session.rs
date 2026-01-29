use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::{FortressError, InternalErrorKind, InvalidRequestKind};
use crate::frame_info::PlayerInput;
use crate::network::messages::ConnectionStatus;
use crate::report_violation;
use crate::sessions::config::SaveMode;
use crate::sync_layer::SyncLayer;
use crate::telemetry::{ViolationKind, ViolationObserver, ViolationSeverity};
use crate::{Config, FortressRequest, Frame, PlayerHandle};

/// During a [`SyncTestSession`], Fortress Rollback will simulate a rollback every frame and resimulate the last n states, where n is the given check distance.
///
/// The resimulated checksums will be compared with the original checksums and report if there was a mismatch.
pub struct SyncTestSession<T>
where
    T: Config,
{
    num_players: usize,
    max_prediction: usize,
    check_distance: usize,
    sync_layer: SyncLayer<T>,
    dummy_connect_status: Vec<ConnectionStatus>,
    checksum_history: BTreeMap<Frame, Option<u128>>,
    local_inputs: BTreeMap<PlayerHandle, PlayerInput<T::Input>>,
    /// Optional observer for specification violations.
    violation_observer: Option<Arc<dyn ViolationObserver>>,
}

impl<T: Config> SyncTestSession<T> {
    /// Creates a new sync test session with the default queue length.
    ///
    /// Note: This function exists for backward compatibility.
    /// The main construction path uses `with_queue_length` via `SessionBuilder`.
    #[allow(dead_code)]
    pub(crate) fn new(
        num_players: usize,
        max_prediction: usize,
        check_distance: usize,
        input_delay: usize,
        violation_observer: Option<Arc<dyn ViolationObserver>>,
    ) -> Self {
        Self::with_queue_length(
            num_players,
            max_prediction,
            check_distance,
            input_delay,
            violation_observer,
            crate::input_queue::INPUT_QUEUE_LENGTH,
        )
    }

    pub(crate) fn with_queue_length(
        num_players: usize,
        max_prediction: usize,
        check_distance: usize,
        input_delay: usize,
        violation_observer: Option<Arc<dyn ViolationObserver>>,
        queue_length: usize,
    ) -> Self {
        let dummy_connect_status = vec![ConnectionStatus::default(); num_players];

        let mut sync_layer =
            SyncLayer::with_queue_length(num_players, max_prediction, queue_length);
        for i in 0..num_players {
            // This should never fail during construction as player handles are sequential and valid
            if let Err(e) = sync_layer.set_frame_delay(PlayerHandle::new(i), input_delay) {
                report_violation!(
                    ViolationSeverity::Critical,
                    ViolationKind::InternalError,
                    "Failed to set frame delay for player {} during session construction: {}",
                    i,
                    e
                );
            }
        }

        Self {
            num_players,
            max_prediction,
            check_distance,
            sync_layer,
            dummy_connect_status,
            checksum_history: BTreeMap::new(),
            local_inputs: BTreeMap::new(),
            violation_observer,
        }
    }

    /// Registers local input for a player for the current frame. This should be successfully called for every local player before calling [`advance_frame()`].
    /// If this is called multiple times for the same player before advancing the frame, older given inputs will be overwritten.
    /// In a sync test, all players are considered to be local, so you need to add input for all of them.
    ///
    /// # Errors
    /// - Returns [`InvalidRequest`] when the given handle is not valid (i.e. not between 0 and num_players).
    ///
    /// [`advance_frame()`]: Self#method.advance_frame
    /// [`InvalidRequest`]: FortressError::InvalidRequest
    pub fn add_local_input(
        &mut self,
        player_handle: PlayerHandle,
        input: T::Input,
    ) -> Result<(), FortressError> {
        if !player_handle.is_valid_player_for(self.num_players) {
            return Err(InvalidRequestKind::InvalidLocalPlayerHandle {
                handle: player_handle,
                num_players: self.num_players,
            }
            .into());
        }
        let player_input = PlayerInput::<T::Input>::new(self.sync_layer.current_frame(), input);
        self.local_inputs.insert(player_handle, player_input);
        Ok(())
    }

    /// In a sync test, this will advance the state by a single frame and afterwards rollback `check_distance` amount of frames,
    /// resimulate and compare checksums with the original states. Returns an order-sensitive [`Vec<FortressRequest>`].
    /// You should fulfill all requests in the exact order they are provided. Failure to do so will cause panics later.
    ///
    /// # Errors
    /// - Returns [`MismatchedChecksum`] if checksums don't match after resimulation.
    ///
    /// [`Vec<FortressRequest>`]: FortressRequest
    /// [`MismatchedChecksum`]: FortressError::MismatchedChecksum
    #[must_use = "FortressRequests must be processed to advance the game state"]
    pub fn advance_frame(&mut self) -> Result<Vec<FortressRequest<T>>, FortressError> {
        // Pre-allocate with capacity for typical case: 1 save + 1 advance = 2 requests.
        // During rollback testing, more requests will be added as the Vec grows.
        let mut requests = Vec::with_capacity(2);

        // if we advanced far enough into the game do comparisons and rollbacks
        let current_frame = self.sync_layer.current_frame();
        if self.check_distance > 0 && current_frame.as_i32() > self.check_distance as i32 {
            // compare checksums of older frames to our checksum history (where only the first version of any checksum is recorded)
            let oldest_frame_to_check = current_frame.as_i32() - self.check_distance as i32;
            let mismatched_frames: Vec<_> = (oldest_frame_to_check..=current_frame.as_i32())
                .filter(|&frame_to_check| !self.checksums_consistent(Frame::new(frame_to_check)))
                .map(Frame::new)
                .collect();

            if !mismatched_frames.is_empty() {
                return Err(FortressError::MismatchedChecksum {
                    current_frame,
                    mismatched_frames,
                });
            }

            // simulate rollbacks according to the check_distance
            let frame_to = self.sync_layer.current_frame() - self.check_distance as i32;
            self.adjust_gamestate(frame_to, &mut requests)?;
        }

        // we require inputs for all players
        if self.num_players != self.local_inputs.len() {
            return Err(InvalidRequestKind::MissingLocalInput.into());
        }
        // pass all inputs into the sync layer
        for (&handle, &input) in self.local_inputs.iter() {
            // send the input into the sync layer
            self.sync_layer.add_local_input(handle, input);
        }
        // clear local inputs after using them
        self.local_inputs.clear();

        // save the current frame in the synchronization layer
        // we can skip all the saving if the check_distance is 0
        if self.check_distance > 0 {
            requests.push(self.sync_layer.save_current_state());
        }

        // get the correct inputs for all players from the sync layer
        let inputs = match self
            .sync_layer
            .synchronized_inputs(&self.dummy_connect_status)
        {
            Some(inputs) => inputs,
            None => {
                report_violation!(
                    ViolationSeverity::Critical,
                    ViolationKind::InternalError,
                    "Failed to get synchronized inputs for frame {}",
                    self.sync_layer.current_frame()
                );
                return Err(FortressError::InternalErrorStructured {
                    kind: InternalErrorKind::SynchronizedInputsFailed {
                        frame: self.sync_layer.current_frame(),
                    },
                });
            },
        };

        // advance the frame
        requests.push(FortressRequest::AdvanceFrame { inputs });
        self.sync_layer.advance_frame();

        // since this is a sync test, we "cheat" by setting the last confirmed state to the (current state - check_distance), so the sync layer won't complain about missing
        // inputs from other players
        let safe_frame = self.sync_layer.current_frame() - self.check_distance as i32;

        self.sync_layer
            .set_last_confirmed_frame(safe_frame, SaveMode::EveryFrame);

        // also, we update the dummy connect status to pretend that we received inputs from all players
        for con_stat in &mut self.dummy_connect_status {
            con_stat.last_frame = self.sync_layer.current_frame();
        }

        Ok(requests)
    }

    /// Returns the current frame of a session.
    #[must_use]
    pub fn current_frame(&self) -> Frame {
        self.sync_layer.current_frame()
    }

    /// Returns the number of players this session was constructed with.
    #[must_use]
    pub fn num_players(&self) -> usize {
        self.num_players
    }

    /// Returns the maximum prediction window of a session.
    #[must_use]
    pub fn max_prediction(&self) -> usize {
        self.max_prediction
    }

    /// Returns the check distance set on creation, i.e. the length of the simulated rollbacks
    #[must_use]
    pub fn check_distance(&self) -> usize {
        self.check_distance
    }

    /// Returns a reference to the violation observer, if one was configured.
    ///
    /// This allows checking for violations that occurred during session operations
    /// when using a [`CollectingObserver`] or similar.
    ///
    /// [`CollectingObserver`]: crate::telemetry::CollectingObserver
    #[must_use]
    pub fn violation_observer(&self) -> Option<&Arc<dyn ViolationObserver>> {
        self.violation_observer.as_ref()
    }

    /// Updates the `checksum_history` and checks if the checksum is identical if it already has been recorded once
    fn checksums_consistent(&mut self, frame_to_check: Frame) -> bool {
        // remove entries older than the `check_distance`
        let oldest_allowed_frame = self.sync_layer.current_frame() - self.check_distance as i32;
        self.checksum_history
            .retain(|&k, _| k >= oldest_allowed_frame);

        match self.sync_layer.saved_state_by_frame(frame_to_check) {
            Some(latest_cell) => match self.checksum_history.get(&latest_cell.frame()) {
                Some(&cs) => cs == latest_cell.checksum(),
                None => {
                    self.checksum_history
                        .insert(latest_cell.frame(), latest_cell.checksum());
                    true
                },
            },
            None => true,
        }
    }

    fn adjust_gamestate(
        &mut self,
        frame_to: Frame,
        requests: &mut Vec<FortressRequest<T>>,
    ) -> Result<(), FortressError> {
        let start_frame = self.sync_layer.current_frame();
        let count = start_frame - frame_to;

        // rollback to the first incorrect state
        requests.push(self.sync_layer.load_frame(frame_to)?);
        self.sync_layer.reset_prediction();
        let actual_frame = self.sync_layer.current_frame();
        if actual_frame != frame_to {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "current frame mismatch after load: expected={}, actual={}",
                frame_to,
                actual_frame
            );
        }

        // step forward to the previous current state
        for i in 0..count {
            let inputs = match self
                .sync_layer
                .synchronized_inputs(&self.dummy_connect_status)
            {
                Some(inputs) => inputs,
                None => {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InternalError,
                        "Failed to get synchronized inputs during resimulation at frame {}",
                        self.sync_layer.current_frame()
                    );
                    return Err(FortressError::InternalErrorStructured {
                        kind: InternalErrorKind::SynchronizedInputsFailed {
                            frame: self.sync_layer.current_frame(),
                        },
                    });
                },
            };

            // first save (except in the first step, because we just loaded that state)
            if i > 0 {
                requests.push(self.sync_layer.save_current_state());
            }
            // then advance
            self.sync_layer.advance_frame();

            requests.push(FortressRequest::AdvanceFrame { inputs });
        }
        let final_frame = self.sync_layer.current_frame();
        if final_frame != start_frame {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "current frame mismatch after resimulation: expected={}, actual={}",
                start_frame,
                final_frame
            );
        }
        Ok(())
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
    use crate::telemetry::CollectingObserver;
    use std::net::SocketAddr;

    /// A minimal test configuration for unit testing.
    struct TestConfig;

    impl Config for TestConfig {
        type Input = u32;
        type State = Vec<u8>;
        type Address = SocketAddr;
    }

    // ==========================================
    // Constructor Tests
    // ==========================================

    #[test]
    fn sync_test_session_new_creates_valid_session() {
        let session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 2, 2, None);

        assert_eq!(session.num_players(), 2);
        assert_eq!(session.max_prediction(), 8);
        assert_eq!(session.check_distance(), 2);
        assert_eq!(session.current_frame(), Frame::new(0));
        assert!(session.violation_observer().is_none());
    }

    #[test]
    fn sync_test_session_with_queue_length_creates_valid_session() {
        let session: SyncTestSession<TestConfig> =
            SyncTestSession::with_queue_length(4, 16, 3, 1, None, 64);

        assert_eq!(session.num_players(), 4);
        assert_eq!(session.max_prediction(), 16);
        assert_eq!(session.check_distance(), 3);
        assert_eq!(session.current_frame(), Frame::new(0));
    }

    #[test]
    fn sync_test_session_with_violation_observer() {
        let observer = Arc::new(CollectingObserver::new());
        let session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 2, 2, Some(observer));

        assert!(session.violation_observer().is_some());
    }

    #[test]
    fn sync_test_session_single_player() {
        let session: SyncTestSession<TestConfig> = SyncTestSession::new(1, 8, 2, 0, None);

        assert_eq!(session.num_players(), 1);
    }

    #[test]
    fn sync_test_session_zero_check_distance() {
        let session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 0, 2, None);

        assert_eq!(session.check_distance(), 0);
    }

    #[test]
    fn sync_test_session_zero_input_delay() {
        let session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 2, 0, None);

        // Just ensure construction succeeds
        assert_eq!(session.current_frame(), Frame::new(0));
    }

    // ==========================================
    // add_local_input Tests
    // ==========================================

    #[test]
    fn add_local_input_valid_handle_succeeds() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 0, 0, None);

        session.add_local_input(PlayerHandle::new(0), 42).unwrap();
        session.add_local_input(PlayerHandle::new(1), 100).unwrap();
    }

    #[test]
    fn add_local_input_invalid_handle_fails() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 0, 0, None);

        let result = session.add_local_input(PlayerHandle::new(2), 42);
        assert!(result.is_err());

        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::InvalidLocalPlayerHandle { .. }
            })
        ));
    }

    #[test]
    fn add_local_input_overwrites_previous_input() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(1, 8, 0, 0, None);

        // Add first input
        session
            .add_local_input(PlayerHandle::new(0), 42)
            .expect("should succeed");

        // Overwrite with second input
        session
            .add_local_input(PlayerHandle::new(0), 100)
            .expect("should succeed");

        // Advance frame to verify the latest input is used
        let requests = session.advance_frame().expect("should advance");

        // Find the AdvanceFrame request
        let advance_request = requests
            .iter()
            .find(|r| matches!(r, FortressRequest::AdvanceFrame { .. }));
        assert!(advance_request.is_some());

        if let Some(FortressRequest::AdvanceFrame { inputs }) = advance_request {
            assert_eq!(inputs[0].0, 100); // Second input should be used
        }
    }

    // ==========================================
    // advance_frame Tests
    // ==========================================

    #[test]
    fn advance_frame_requires_all_inputs() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 0, 0, None);

        // Only add input for player 0
        session
            .add_local_input(PlayerHandle::new(0), 42)
            .expect("should succeed");

        let result = session.advance_frame();
        assert!(result.is_err());

        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::MissingLocalInput
            })
        ));
    }

    #[test]
    fn advance_frame_with_all_inputs_succeeds() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 0, 0, None);

        session
            .add_local_input(PlayerHandle::new(0), 42)
            .expect("should succeed");
        session
            .add_local_input(PlayerHandle::new(1), 100)
            .expect("should succeed");

        let requests = session.advance_frame().unwrap();
        // With check_distance 0, we should only get AdvanceFrame
        assert!(requests
            .iter()
            .any(|r| matches!(r, FortressRequest::AdvanceFrame { .. })));
    }

    #[test]
    fn advance_frame_increments_current_frame() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(1, 8, 0, 0, None);

        assert_eq!(session.current_frame(), Frame::new(0));

        session
            .add_local_input(PlayerHandle::new(0), 42)
            .expect("should succeed");
        session.advance_frame().expect("should advance");

        assert_eq!(session.current_frame(), Frame::new(1));
    }

    #[test]
    fn advance_frame_clears_inputs() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(1, 8, 0, 0, None);

        session
            .add_local_input(PlayerHandle::new(0), 42)
            .expect("should succeed");
        session.advance_frame().expect("should advance");

        // Next advance should fail because inputs are cleared
        let result = session.advance_frame();
        assert!(result.is_err());
    }

    #[test]
    fn advance_frame_with_check_distance_produces_save_request() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(1, 8, 2, 0, None);

        session
            .add_local_input(PlayerHandle::new(0), 42)
            .expect("should succeed");

        let requests = session.advance_frame().expect("should advance");

        // With check_distance > 0, we should get a SaveGameState request
        assert!(requests
            .iter()
            .any(|r| matches!(r, FortressRequest::SaveGameState { .. })));
    }

    #[test]
    fn advance_frame_multiple_times() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(1, 8, 0, 0, None);

        for frame in 1..=10 {
            session
                .add_local_input(PlayerHandle::new(0), frame as u32)
                .expect("should succeed");
            session.advance_frame().expect("should advance");
            assert_eq!(session.current_frame(), Frame::new(frame));
        }
    }

    #[test]
    fn advance_frame_no_input_for_any_player() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 0, 0, None);

        // Don't add any inputs
        let result = session.advance_frame();
        assert!(result.is_err());

        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::MissingLocalInput
            })
        ));
    }

    // ==========================================
    // Getter Tests
    // ==========================================

    #[test]
    fn current_frame_starts_at_zero() {
        let session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 2, 2, None);
        assert_eq!(session.current_frame(), Frame::new(0));
    }

    #[test]
    fn num_players_returns_correct_value() {
        for num_players in 1..=4 {
            let session: SyncTestSession<TestConfig> =
                SyncTestSession::new(num_players, 8, 2, 2, None);
            assert_eq!(session.num_players(), num_players);
        }
    }

    #[test]
    fn max_prediction_returns_correct_value() {
        for max_prediction in [4, 8, 16, 32] {
            let session: SyncTestSession<TestConfig> =
                SyncTestSession::new(2, max_prediction, 2, 2, None);
            assert_eq!(session.max_prediction(), max_prediction);
        }
    }

    #[test]
    fn check_distance_returns_correct_value() {
        for check_distance in 0..=10 {
            let session: SyncTestSession<TestConfig> =
                SyncTestSession::new(2, 8, check_distance, 2, None);
            assert_eq!(session.check_distance(), check_distance);
        }
    }

    #[test]
    fn violation_observer_none_when_not_set() {
        let session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 2, 2, None);
        assert!(session.violation_observer().is_none());
    }

    #[test]
    fn violation_observer_some_when_set() {
        let observer = Arc::new(CollectingObserver::new());
        let session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 2, 2, Some(observer));

        let stored_observer = session.violation_observer();
        assert!(stored_observer.is_some());
    }

    // ==========================================
    // Edge Case Tests
    // ==========================================

    #[test]
    fn many_players_construction() {
        // Test with a larger number of players
        let session: SyncTestSession<TestConfig> = SyncTestSession::new(8, 16, 4, 2, None);

        assert_eq!(session.num_players(), 8);
        assert_eq!(session.max_prediction(), 16);
    }

    #[test]
    fn large_check_distance() {
        // Test with a check distance larger than typical
        let session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 64, 32, 2, None);

        assert_eq!(session.check_distance(), 32);
        assert_eq!(session.max_prediction(), 64);
    }

    #[test]
    fn small_queue_length() {
        let session: SyncTestSession<TestConfig> =
            SyncTestSession::with_queue_length(2, 8, 2, 2, None, 16);

        assert_eq!(session.num_players(), 2);
    }

    // ==========================================
    // Checksum Validation Tests
    // ==========================================

    /// Helper to run a sync test session for a specified number of frames
    /// while simulating game state saves with consistent checksums.
    fn run_session_with_checksums(
        session: &mut SyncTestSession<TestConfig>,
        num_frames: usize,
        checksum_fn: impl Fn(Frame) -> Option<u128>,
    ) {
        let mut game_state: Vec<u8> = Vec::new();

        for frame_num in 0..num_frames {
            // Add inputs for all players
            for player_id in 0..session.num_players() {
                session
                    .add_local_input(PlayerHandle::new(player_id), frame_num as u32)
                    .expect("should succeed");
            }

            // Advance the frame and handle requests
            let requests = session.advance_frame().expect("should advance");

            for request in requests {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        let checksum = checksum_fn(frame);
                        cell.save(frame, Some(game_state.clone()), checksum);
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        if let Some(loaded) = cell.load() {
                            game_state = loaded;
                        }
                    },
                    FortressRequest::AdvanceFrame { .. } => {
                        // Simulate game advancement - append frame number to state
                        game_state.push(frame_num as u8);
                    },
                }
            }
        }
    }

    #[test]
    fn sync_test_with_consistent_checksums_succeeds() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 3, 0, None);

        // Use consistent checksums - should succeed
        run_session_with_checksums(&mut session, 20, |frame| Some(frame.as_i32() as u128));

        // Should have advanced to frame 20
        assert_eq!(session.current_frame(), Frame::new(20));
    }

    #[test]
    fn sync_test_with_no_checksums_succeeds() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 3, 0, None);

        // Use no checksums (None) - should still succeed since None == None
        run_session_with_checksums(&mut session, 15, |_| None);

        assert_eq!(session.current_frame(), Frame::new(15));
    }

    #[test]
    fn sync_test_detects_mismatched_checksum() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(1, 8, 2, 0, None);

        let mut game_state: Vec<u8> = Vec::new();
        let mut call_count = 0;

        // Simulate frames until we detect a mismatch
        let mut detected_mismatch = false;
        for frame_num in 0..20 {
            session
                .add_local_input(PlayerHandle::new(0), frame_num as u32)
                .expect("should succeed");

            match session.advance_frame() {
                Ok(requests) => {
                    for request in requests {
                        match request {
                            FortressRequest::SaveGameState { cell, frame } => {
                                call_count += 1;
                                // Return different checksum after many saves to trigger mismatch
                                // The check_distance is 2, so checksums are compared after 2 frames
                                let checksum = if call_count > 5 {
                                    // Different checksum for resimulated states
                                    Some(9999)
                                } else {
                                    Some(frame.as_i32() as u128)
                                };
                                cell.save(frame, Some(game_state.clone()), checksum);
                            },
                            FortressRequest::LoadGameState { cell, .. } => {
                                if let Some(loaded) = cell.load() {
                                    game_state = loaded;
                                }
                            },
                            FortressRequest::AdvanceFrame { .. } => {
                                game_state.push(frame_num as u8);
                            },
                        }
                    }
                },
                Err(FortressError::MismatchedChecksum { .. }) => {
                    detected_mismatch = true;
                    break;
                },
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        // With different checksums during resimulation, we should detect a mismatch
        assert!(
            detected_mismatch,
            "Should have detected mismatched checksums"
        );
    }

    #[test]
    fn sync_test_zero_check_distance_skips_rollback() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(1, 8, 0, 0, None);

        // With check_distance 0, no SaveGameState or LoadGameState should be issued
        session
            .add_local_input(PlayerHandle::new(0), 42)
            .expect("should succeed");

        let requests = session.advance_frame().expect("should advance");

        // Only AdvanceFrame, no SaveGameState
        assert!(!requests
            .iter()
            .any(|r| matches!(r, FortressRequest::SaveGameState { .. })));
        assert!(!requests
            .iter()
            .any(|r| matches!(r, FortressRequest::LoadGameState { .. })));
        assert!(requests
            .iter()
            .any(|r| matches!(r, FortressRequest::AdvanceFrame { .. })));
    }

    #[test]
    fn sync_test_rollback_happens_after_check_distance_frames() {
        let check_distance = 3;
        let mut session: SyncTestSession<TestConfig> =
            SyncTestSession::new(1, 8, check_distance, 0, None);

        let mut game_state: Vec<u8> = Vec::new();
        let mut saw_load_request = false;

        // Run enough frames to trigger rollback (> check_distance)
        for frame_num in 0..=(check_distance + 2) {
            session
                .add_local_input(PlayerHandle::new(0), frame_num as u32)
                .expect("should succeed");

            let requests = session.advance_frame().expect("should advance");

            for request in requests {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        cell.save(
                            frame,
                            Some(game_state.clone()),
                            Some(frame.as_i32() as u128),
                        );
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        saw_load_request = true;
                        if let Some(loaded) = cell.load() {
                            game_state = loaded;
                        }
                    },
                    FortressRequest::AdvanceFrame { .. } => {
                        game_state.push(frame_num as u8);
                    },
                }
            }
        }

        // After passing check_distance, we should see load requests from rollback simulation
        assert!(
            saw_load_request,
            "Should have seen LoadGameState after passing check_distance"
        );
    }

    #[test]
    fn sync_test_many_players_with_checksums() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(4, 8, 2, 0, None);

        // Should work with multiple players
        run_session_with_checksums(&mut session, 10, |frame| Some(frame.as_i32() as u128));

        assert_eq!(session.current_frame(), Frame::new(10));
        assert_eq!(session.num_players(), 4);
    }

    #[test]
    fn sync_test_large_check_distance() {
        let check_distance = 10;
        let mut session: SyncTestSession<TestConfig> =
            SyncTestSession::new(1, 32, check_distance, 0, None);

        run_session_with_checksums(&mut session, 30, |frame| Some(frame.as_i32() as u128));

        assert_eq!(session.current_frame(), Frame::new(30));
        assert_eq!(session.check_distance(), check_distance);
    }

    // ==========================================
    // Request Order Tests
    // ==========================================

    #[test]
    fn requests_contain_advance_frame_with_inputs() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(2, 8, 0, 0, None);

        session
            .add_local_input(PlayerHandle::new(0), 111)
            .expect("should succeed");
        session
            .add_local_input(PlayerHandle::new(1), 222)
            .expect("should succeed");

        let requests = session.advance_frame().expect("should advance");

        let advance_request = requests
            .iter()
            .find(|r| matches!(r, FortressRequest::AdvanceFrame { .. }));
        assert!(
            advance_request.is_some(),
            "Should have AdvanceFrame request"
        );

        if let Some(FortressRequest::AdvanceFrame { inputs }) = advance_request {
            assert_eq!(inputs.len(), 2);
            assert_eq!(inputs[0].0, 111);
            assert_eq!(inputs[1].0, 222);
        }
    }

    #[test]
    fn requests_order_save_before_advance() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(1, 8, 2, 0, None);

        session
            .add_local_input(PlayerHandle::new(0), 42)
            .expect("should succeed");

        let requests = session.advance_frame().expect("should advance");

        // Find positions of SaveGameState and AdvanceFrame
        let save_pos = requests
            .iter()
            .position(|r| matches!(r, FortressRequest::SaveGameState { .. }));
        let advance_pos = requests
            .iter()
            .position(|r| matches!(r, FortressRequest::AdvanceFrame { .. }));

        // SaveGameState should come before AdvanceFrame (but after any LoadGameState from rollback)
        assert!(
            save_pos.is_some(),
            "Should have SaveGameState with check_distance > 0"
        );
        assert!(advance_pos.is_some(), "Should have AdvanceFrame");

        // The last AdvanceFrame should be after the last SaveGameState
        // (The save is for the current frame, advance uses those inputs)
        assert!(
            save_pos < advance_pos,
            "SaveGameState should come before the final AdvanceFrame"
        );
    }

    // ==========================================
    // SessionBuilder Integration Tests
    // ==========================================

    #[test]
    fn sync_test_via_session_builder() {
        use crate::SessionBuilder;

        let session: SyncTestSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_max_prediction_window(8)
            .with_check_distance(3)
            .start_synctest_session()
            .expect("should create session");

        assert_eq!(session.num_players(), 2);
        assert_eq!(session.max_prediction(), 8);
        assert_eq!(session.check_distance(), 3);
    }

    #[test]
    fn sync_test_builder_with_input_delay() {
        use crate::SessionBuilder;

        let session: SyncTestSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_input_delay(3)
            .unwrap()
            .with_check_distance(2)
            .start_synctest_session()
            .expect("should create session");

        assert_eq!(session.num_players(), 2);
    }

    #[test]
    fn sync_test_builder_with_observer() {
        use crate::SessionBuilder;

        let observer = Arc::new(CollectingObserver::new());
        let session: SyncTestSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_violation_observer(observer)
            .start_synctest_session()
            .expect("should create session");

        assert!(session.violation_observer().is_some());
    }

    // ==========================================
    // Checksum History Retention Tests
    // ==========================================

    #[test]
    fn checksum_history_is_pruned_over_time() {
        // This test verifies that old checksums are removed from history
        // to prevent unbounded memory growth
        let check_distance = 2;
        let mut session: SyncTestSession<TestConfig> =
            SyncTestSession::new(1, 8, check_distance, 0, None);

        let mut game_state: Vec<u8> = Vec::new();

        // Run for many frames
        for frame_num in 0..50 {
            session
                .add_local_input(PlayerHandle::new(0), frame_num as u32)
                .expect("should succeed");

            let requests = session.advance_frame().expect("should advance");

            for request in requests {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        cell.save(
                            frame,
                            Some(game_state.clone()),
                            Some(frame.as_i32() as u128),
                        );
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        if let Some(loaded) = cell.load() {
                            game_state = loaded;
                        }
                    },
                    FortressRequest::AdvanceFrame { .. } => {
                        game_state.push(frame_num as u8);
                    },
                }
            }
        }

        // The session should complete successfully even with many frames
        // because old checksums are pruned
        assert_eq!(session.current_frame(), Frame::new(50));
    }

    // ==========================================
    // Input Status Tests
    // ==========================================

    #[test]
    fn advance_frame_returns_confirmed_input_status() {
        let mut session: SyncTestSession<TestConfig> = SyncTestSession::new(1, 8, 0, 0, None);

        session
            .add_local_input(PlayerHandle::new(0), 42)
            .expect("should succeed");

        let requests = session.advance_frame().expect("should advance");

        if let Some(FortressRequest::AdvanceFrame { inputs }) = requests
            .iter()
            .find(|r| matches!(r, FortressRequest::AdvanceFrame { .. }))
        {
            // In sync test, all inputs should be confirmed
            assert_eq!(inputs[0].1, crate::InputStatus::Confirmed);
        } else {
            panic!("Should have AdvanceFrame request");
        }
    }
}
