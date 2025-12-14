use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::FortressError;
use crate::frame_info::PlayerInput;
use crate::network::messages::ConnectionStatus;
use crate::report_violation;
use crate::sessions::builder::SaveMode;
use crate::sync_layer::SyncLayer;
use crate::telemetry::{ViolationKind, ViolationObserver, ViolationSeverity};
use crate::{Config, FortressRequest, Frame, PlayerHandle};

/// During a [`SyncTestSession`], Fortress Rollback will simulate a rollback every frame and resimulate the last n states, where n is the given check distance.
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
        let mut dummy_connect_status = Vec::new();
        for _ in 0..num_players {
            dummy_connect_status.push(ConnectionStatus::default());
        }

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
            return Err(FortressError::InvalidRequest {
                info: "The player handle you provided is not valid.".to_owned(),
            });
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
            return Err(FortressError::InvalidRequest {
                info: "Missing local input while calling advance_frame().".to_owned(),
            });
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
                return Err(FortressError::InternalError {
                    context: "Failed to get synchronized inputs".to_owned(),
                });
            }
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
                }
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
                    return Err(FortressError::InternalError {
                        context: "Failed to get synchronized inputs during resimulation".to_owned(),
                    });
                }
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
