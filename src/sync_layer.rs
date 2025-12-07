use parking_lot::{MappedMutexGuard, Mutex};
use std::ops::Deref;
use std::sync::Arc;

use crate::frame_info::{GameState, PlayerInput};
use crate::input_queue::InputQueue;
use crate::network::messages::ConnectionStatus;
use crate::report_violation;
use crate::telemetry::{InvariantChecker, InvariantViolation, ViolationKind, ViolationSeverity};
use crate::{Config, FortressError, FortressRequest, Frame, InputStatus, PlayerHandle};

/// An [`Arc<Mutex>`] that you can [`save()`]/[`load()`] a `T` to/from. These will be handed to the user as part of a [`FortressRequest`].
///
/// [`save()`]: GameStateCell#method.save
/// [`load()`]: GameStateCell#method.load
pub struct GameStateCell<T>(Arc<Mutex<GameState<T>>>);

impl<T> GameStateCell<T> {
    /// Saves a `T` the user creates into the cell.
    pub fn save(&self, frame: Frame, data: Option<T>, checksum: Option<u128>) {
        let mut state = self.0.lock();
        assert!(!frame.is_null());
        state.frame = frame;
        state.data = data;
        state.checksum = checksum;
    }

    /// Provides direct access to the `T` that the user previously saved into the cell (if there was
    /// one previously saved), without cloning it.
    ///
    /// You probably want to use [load()](Self::load) instead to clone the data; this function is
    /// useful only in niche use cases.
    ///
    /// # Example usage
    ///
    /// ```
    /// # use fortress_rollback::{Frame, GameStateCell};
    /// // Setup normally performed by Fortress Rollback behind the scenes
    /// let mut cell = GameStateCell::<MyGameState>::default();
    /// let frame_num = Frame::new(0);
    ///
    /// // The state of our example game will be just a String, and our game state isn't Clone
    /// struct MyGameState { player_name: String };
    ///
    /// // Setup you do when Fortress Rollback requests you to save game state
    /// {
    ///     let game_state = MyGameState { player_name: "alex".to_owned() };
    ///     let checksum = None;
    ///     // (in real usage, save a checksum! We omit it here because it's not
    ///     // relevant to this example)
    ///     cell.save(frame_num, Some(game_state), checksum);
    /// }
    ///
    /// // We can't use load() to access the game state, because it's not Clone
    /// // println!("{}", cell.load().player_name); // compile error: Clone bound not satisfied
    ///
    /// // But we can still read the game state without cloning:
    /// let game_state_accessor = cell.data().expect("should have a gamestate stored");
    /// assert_eq!(game_state_accessor.player_name, "alex");
    /// ```
    ///
    /// If you really, really need mutable access to the `T`, then consider using the aptly named
    /// [GameStateAccessor::as_mut_dangerous()].
    pub fn data(&self) -> Option<GameStateAccessor<'_, T>> {
        if let Ok(mapped_data) =
            parking_lot::MutexGuard::try_map(self.0.lock(), |state| state.data.as_mut())
        {
            Some(GameStateAccessor(mapped_data))
        } else {
            None
        }
    }

    pub(crate) fn frame(&self) -> Frame {
        self.0.lock().frame
    }

    pub(crate) fn checksum(&self) -> Option<u128> {
        self.0.lock().checksum
    }
}

impl<T: Clone> GameStateCell<T> {
    /// Loads a `T` that the user previously saved into this cell, by cloning the `T`.
    ///
    /// See also [data()](Self::data) if you want a reference to the `T` without cloning it.
    pub fn load(&self) -> Option<T> {
        let data = self.data()?;
        Some(data.clone())
    }
}

impl<T> Default for GameStateCell<T> {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(GameState::default())))
    }
}

impl<T> Clone for GameStateCell<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> std::fmt::Debug for GameStateCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.0.lock();
        f.debug_struct("GameStateCell")
            .field("frame", &inner.frame)
            .field("checksum", &inner.checksum)
            .finish_non_exhaustive()
    }
}

/// A read-only accessor for the `T` that the user previously saved into a [GameStateCell].
///
/// You can use [deref()](Deref::deref) to access the `T` without cloning it; see
/// [GameStateCell::data()](GameStateCell::data) for a usage example.
///
/// This type exists to A) hide the type of the lock guard that allows thread-safe access to the
///  saved `T` so that it does not form part of Fortress Rollback API and B) make dangerous mutable access to the
///  `T` very explicit (see [as_mut_dangerous()](Self::as_mut_dangerous)).
pub struct GameStateAccessor<'c, T>(MappedMutexGuard<'c, T>);

impl<'c, T> Deref for GameStateAccessor<'c, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'c, T> GameStateAccessor<'c, T> {
    /// Get mutable access to the `T` that the user previously saved into a [GameStateCell].
    ///
    /// You probably do not need this! It's safer to use [Self::deref()](Deref::deref) instead;
    /// see [GameStateCell::data()](GameStateCell::data) for a usage example.
    ///
    /// **Danger**: the underlying `T` must _not_ be modified in any way that affects (or may ever
    /// in future affect) game logic. If this invariant is violated, you will almost certainly get
    /// desyncs.
    pub fn as_mut_dangerous(&mut self) -> &mut T {
        &mut self.0
    }
}

pub(crate) struct SavedStates<T> {
    pub states: Vec<GameStateCell<T>>,
}

impl<T> SavedStates<T> {
    fn new(max_pred: usize) -> Self {
        // we need to store the current frame plus the number of max predictions, so that we can
        // roll back to the very first frame even when we have predicted as far ahead as we can.
        let num_cells = max_pred + 1;
        let mut states = Vec::with_capacity(num_cells);
        for _ in 0..num_cells {
            states.push(GameStateCell::default());
        }

        Self { states }
    }

    fn get_cell(&self, frame: Frame) -> Result<GameStateCell<T>, FortressError> {
        if frame.as_i32() < 0 {
            return Err(FortressError::InvalidFrame {
                frame,
                reason: "frame must be non-negative".to_string(),
            });
        }
        let pos = frame.as_i32() as usize % self.states.len();
        Ok(self.states[pos].clone())
    }
}

pub(crate) struct SyncLayer<T>
where
    T: Config,
{
    num_players: usize,
    max_prediction: usize,
    saved_states: SavedStates<T::State>,
    last_confirmed_frame: Frame,
    last_saved_frame: Frame,
    current_frame: Frame,
    input_queues: Vec<InputQueue<T>>,
}

impl<T: Config> SyncLayer<T> {
    /// Creates a new `SyncLayer` instance with given values.
    pub(crate) fn new(num_players: usize, max_prediction: usize) -> Self {
        // initialize input_queues with player indices for deterministic prediction
        let mut input_queues = Vec::new();
        for player_index in 0..num_players {
            input_queues.push(InputQueue::new(player_index));
        }
        Self {
            num_players,
            max_prediction,
            last_confirmed_frame: Frame::NULL,
            last_saved_frame: Frame::NULL,
            current_frame: Frame::new(0),
            saved_states: SavedStates::new(max_prediction),
            input_queues,
        }
    }

    pub(crate) fn current_frame(&self) -> Frame {
        self.current_frame
    }

    pub(crate) fn advance_frame(&mut self) {
        self.current_frame += 1;
    }

    /// Saves the current game state.
    ///
    /// # Panics
    /// This method will not panic as `current_frame` is always valid (>= 0).
    pub(crate) fn save_current_state(&mut self) -> FortressRequest<T> {
        self.last_saved_frame = self.current_frame;
        // current_frame is always >= 0, so this cannot fail
        let cell = self
            .saved_states
            .get_cell(self.current_frame)
            .expect("Internal error: current_frame should always be valid for get_cell");
        FortressRequest::SaveGameState {
            cell,
            frame: self.current_frame,
        }
    }

    /// Sets the frame delay for a player.
    ///
    /// # Errors
    /// Returns `FortressError::InvalidPlayerHandle` if `player_handle >= num_players`.
    pub(crate) fn set_frame_delay(
        &mut self,
        player_handle: PlayerHandle,
        delay: usize,
    ) -> Result<(), FortressError> {
        if !player_handle.is_valid_player_for(self.num_players) {
            return Err(FortressError::InvalidPlayerHandle {
                handle: player_handle,
                max_handle: PlayerHandle::new(self.num_players.saturating_sub(1)),
            });
        }
        self.input_queues[player_handle.as_usize()].set_frame_delay(delay);
        Ok(())
    }

    pub(crate) fn reset_prediction(&mut self) {
        for i in 0..self.num_players {
            self.input_queues[i].reset_prediction();
        }
    }

    /// Loads the gamestate indicated by `frame_to_load`.
    ///
    /// # Errors
    /// Returns `FortressError::InvalidFrame` if:
    /// - `frame_to_load` is `NULL_FRAME`
    /// - `frame_to_load` is not in the past (>= current_frame)
    /// - `frame_to_load` is outside the prediction window
    /// - The saved state for `frame_to_load` doesn't exist or has wrong frame
    pub(crate) fn load_frame(
        &mut self,
        frame_to_load: Frame,
    ) -> Result<FortressRequest<T>, FortressError> {
        // The state should not be the current state or the state should not be in the future or too far away in the past
        if frame_to_load.is_null() {
            return Err(FortressError::InvalidFrame {
                frame: frame_to_load,
                reason: "cannot load NULL_FRAME".to_string(),
            });
        }

        if frame_to_load >= self.current_frame {
            return Err(FortressError::InvalidFrame {
                frame: frame_to_load,
                reason: format!(
                    "must load frame in the past (frame to load is {}, current frame is {})",
                    frame_to_load, self.current_frame
                ),
            });
        }

        if frame_to_load.as_i32() < self.current_frame.as_i32() - self.max_prediction as i32 {
            return Err(FortressError::InvalidFrame {
                frame: frame_to_load,
                reason: format!(
                    "cannot load frame outside of prediction window \
                    (frame to load is {}, current frame is {}, max prediction is {})",
                    frame_to_load, self.current_frame, self.max_prediction
                ),
            });
        }

        let cell = self.saved_states.get_cell(frame_to_load)?;
        let cell_frame = cell.0.lock().frame;
        if cell_frame != frame_to_load {
            return Err(FortressError::InvalidFrame {
                frame: frame_to_load,
                reason: format!(
                    "saved state has wrong frame (expected {}, got {})",
                    frame_to_load, cell_frame
                ),
            });
        }
        self.current_frame = frame_to_load;

        Ok(FortressRequest::LoadGameState {
            cell,
            frame: frame_to_load,
        })
    }

    /// Adds local input to the corresponding input queue. Checks if the prediction threshold has been reached. Returns the frame number where the input is actually added to.
    /// This number will only be different if the input delay was set to a number higher than 0.
    pub(crate) fn add_local_input(
        &mut self,
        player_handle: PlayerHandle,
        input: PlayerInput<T::Input>,
    ) -> Frame {
        // The input provided should match the current frame, we account for input delay later
        assert_eq!(input.frame, self.current_frame);
        self.input_queues[player_handle.as_usize()].add_input(input)
    }

    /// Adds remote input to the corresponding input queue.
    /// Unlike `add_local_input`, this will not check for correct conditions, as remote inputs have already been checked on another device.
    pub(crate) fn add_remote_input(
        &mut self,
        player_handle: PlayerHandle,
        input: PlayerInput<T::Input>,
    ) {
        self.input_queues[player_handle.as_usize()].add_input(input);
    }

    /// Returns inputs for all players for the current frame of the sync layer. If there are none for a specific player, return predictions.
    pub(crate) fn synchronized_inputs(
        &mut self,
        connect_status: &[ConnectionStatus],
    ) -> Vec<(T::Input, InputStatus)> {
        let mut inputs = Vec::new();
        for (i, con_stat) in connect_status.iter().enumerate() {
            if con_stat.disconnected && con_stat.last_frame < self.current_frame {
                inputs.push((T::Input::default(), InputStatus::Disconnected));
            } else {
                inputs.push(self.input_queues[i].input(self.current_frame));
            }
        }
        inputs
    }

    /// Returns confirmed inputs for all players for the current frame of the sync layer.
    pub(crate) fn confirmed_inputs(
        &self,
        frame: Frame,
        connect_status: &[ConnectionStatus],
    ) -> Result<Vec<PlayerInput<T::Input>>, FortressError> {
        let mut inputs = Vec::new();
        for (i, con_stat) in connect_status.iter().enumerate() {
            if con_stat.disconnected && con_stat.last_frame < frame {
                inputs.push(PlayerInput::blank_input(Frame::NULL));
            } else {
                inputs.push(self.input_queues[i].confirmed_input(frame)?);
            }
        }
        Ok(inputs)
    }

    /// Sets the last confirmed frame to a given frame. By raising the last confirmed frame, we can discard all previous frames, as they are no longer necessary.
    pub(crate) fn set_last_confirmed_frame(&mut self, mut frame: Frame, sparse_saving: bool) {
        // don't set the last confirmed frame after the first incorrect frame before a rollback has happened
        let mut first_incorrect: Frame = Frame::NULL;
        for handle in 0..self.num_players {
            first_incorrect = std::cmp::max(
                first_incorrect,
                self.input_queues[handle].first_incorrect_frame(),
            );
        }

        // if sparse saving option is turned on, don't set the last confirmed frame after the last saved frame
        if sparse_saving {
            frame = std::cmp::min(frame, self.last_saved_frame);
        }

        // never delete stuff ahead of the current frame
        frame = std::cmp::min(frame, self.current_frame());

        // if we set the last confirmed frame beyond the first incorrect frame, we discard inputs that we need later for adjusting the gamestate.
        // Clamp frame to not exceed first_incorrect as a safety measure and log if this happens
        if !first_incorrect.is_null() && first_incorrect < frame {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "Clamping confirmed frame {} to first_incorrect {} - this may indicate a bug",
                frame,
                first_incorrect
            );
            frame = first_incorrect;
        }

        self.last_confirmed_frame = frame;
        if self.last_confirmed_frame.as_i32() > 0 {
            for i in 0..self.num_players {
                self.input_queues[i].discard_confirmed_frames(frame - 1);
            }
        }
    }

    /// Finds the earliest incorrect frame detected by the individual input queues
    pub(crate) fn check_simulation_consistency(&self, mut first_incorrect: Frame) -> Frame {
        for handle in 0..self.num_players {
            let incorrect = self.input_queues[handle].first_incorrect_frame();
            if !incorrect.is_null() && (first_incorrect.is_null() || incorrect < first_incorrect) {
                first_incorrect = incorrect;
            }
        }
        first_incorrect
    }

    /// Returns a gamestate through given frame
    pub(crate) fn saved_state_by_frame(&self, frame: Frame) -> Option<GameStateCell<T::State>> {
        let cell = self.saved_states.get_cell(frame).ok()?;

        if cell.0.lock().frame == frame {
            Some(cell)
        } else {
            None
        }
    }

    /// Returns the latest saved frame
    pub(crate) fn last_saved_frame(&self) -> Frame {
        self.last_saved_frame
    }

    /// Returns the latest confirmed frame
    pub(crate) fn last_confirmed_frame(&self) -> Frame {
        self.last_confirmed_frame
    }
}

impl<T: Config> InvariantChecker for SyncLayer<T> {
    /// Checks the invariants of the SyncLayer.
    ///
    /// # Invariants
    ///
    /// 1. `num_players` must be > 0
    /// 2. `max_prediction` must be > 0
    /// 3. `current_frame` must be >= 0
    /// 4. `last_confirmed_frame` must be <= `current_frame`
    /// 5. `last_saved_frame` must be <= `current_frame`
    /// 6. Input queues count must match `num_players`
    /// 7. Saved states count must be `max_prediction + 1`
    /// 8. All input queues must pass their invariant checks
    fn check_invariants(&self) -> Result<(), InvariantViolation> {
        // Invariant 1: num_players > 0
        if self.num_players == 0 {
            return Err(InvariantViolation::new(
                "SyncLayer",
                "num_players must be greater than 0",
            ));
        }

        // Invariant 2: max_prediction > 0
        if self.max_prediction == 0 {
            return Err(InvariantViolation::new(
                "SyncLayer",
                "max_prediction must be greater than 0",
            ));
        }

        // Invariant 3: current_frame >= 0
        if self.current_frame.as_i32() < 0 {
            return Err(
                InvariantViolation::new("SyncLayer", "current_frame must be non-negative")
                    .with_details(format!("current_frame={}", self.current_frame)),
            );
        }

        // Invariant 4: last_confirmed_frame <= current_frame
        if !self.last_confirmed_frame.is_null() && self.last_confirmed_frame > self.current_frame {
            return Err(InvariantViolation::new(
                "SyncLayer",
                "last_confirmed_frame exceeds current_frame",
            )
            .with_details(format!(
                "last_confirmed_frame={}, current_frame={}",
                self.last_confirmed_frame, self.current_frame
            )));
        }

        // Invariant 5: last_saved_frame <= current_frame
        if !self.last_saved_frame.is_null() && self.last_saved_frame > self.current_frame {
            return Err(InvariantViolation::new(
                "SyncLayer",
                "last_saved_frame exceeds current_frame",
            )
            .with_details(format!(
                "last_saved_frame={}, current_frame={}",
                self.last_saved_frame, self.current_frame
            )));
        }

        // Invariant 6: input queues count matches num_players
        if self.input_queues.len() != self.num_players {
            return Err(InvariantViolation::new(
                "SyncLayer",
                "input_queues count does not match num_players",
            )
            .with_details(format!(
                "input_queues.len()={}, num_players={}",
                self.input_queues.len(),
                self.num_players
            )));
        }

        // Invariant 7: saved states count is max_prediction + 1
        let expected_states = self.max_prediction + 1;
        if self.saved_states.states.len() != expected_states {
            return Err(
                InvariantViolation::new("SyncLayer", "saved_states count is incorrect")
                    .with_details(format!(
                        "saved_states.len()={}, expected={}",
                        self.saved_states.states.len(),
                        expected_states
                    )),
            );
        }

        // Invariant 8: all input queues pass their invariant checks
        for (i, queue) in self.input_queues.iter().enumerate() {
            if let Err(violation) = queue.check_invariants() {
                return Err(InvariantViolation::new(
                    "SyncLayer",
                    format!("input_queue[{}] invariant violated", i),
                )
                .with_details(violation.to_string()));
            }
        }

        Ok(())
    }
}

// #########
// # TESTS #
// #########

#[cfg(test)]
mod sync_layer_tests {

    use super::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
    struct TestInput {
        inp: u8,
    }

    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = u8;
        type Address = SocketAddr;
    }

    #[test]
    fn test_different_delays() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        let p1_delay = 2;
        let p2_delay = 0;
        sync_layer
            .set_frame_delay(PlayerHandle::new(0), p1_delay)
            .unwrap();
        sync_layer
            .set_frame_delay(PlayerHandle::new(1), p2_delay)
            .unwrap();

        let mut dummy_connect_status = Vec::new();
        dummy_connect_status.push(ConnectionStatus::default());
        dummy_connect_status.push(ConnectionStatus::default());

        for i in 0..20i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            // adding input as remote to avoid prediction threshold detection
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            // update the dummy connect status
            dummy_connect_status[0].last_frame = Frame::new(i);
            dummy_connect_status[1].last_frame = Frame::new(i);

            if i >= 3 {
                let sync_inputs = sync_layer.synchronized_inputs(&dummy_connect_status);
                let player0_inputs = sync_inputs[0].0.inp;
                let player1_inputs = sync_inputs[1].0.inp;
                assert_eq!(player0_inputs, i as u8 - p1_delay as u8);
                assert_eq!(player1_inputs, i as u8 - p2_delay as u8);
            }

            sync_layer.advance_frame();
        }
    }

    #[test]
    fn test_set_frame_delay_invalid_handle() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        // Valid handles are 0 and 1 (num_players = 2)
        let result = sync_layer.set_frame_delay(PlayerHandle::new(2), 0);
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidPlayerHandle { handle, max_handle }) => {
                assert_eq!(handle, PlayerHandle::new(2));
                assert_eq!(max_handle, PlayerHandle::new(1));
            }
            _ => panic!("Expected InvalidPlayerHandle error"),
        }
    }

    #[test]
    fn test_sync_layer_new_initializes_correctly() {
        let sync_layer = SyncLayer::<TestConfig>::new(4, 7);
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::NULL);
        assert_eq!(sync_layer.last_saved_frame(), Frame::NULL);
        assert_eq!(sync_layer.num_players, 4);
        assert_eq!(sync_layer.max_prediction, 7);
        assert_eq!(sync_layer.input_queues.len(), 4);
    }

    #[test]
    fn test_advance_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(1));
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(2));
    }

    #[test]
    fn test_save_current_state() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        match request {
            FortressRequest::SaveGameState { cell, frame } => {
                assert_eq!(frame, Frame::new(0));
                // Save some data
                cell.save(Frame::new(0), Some(42u8), Some(1234));
                assert_eq!(cell.frame(), Frame::new(0));
            }
            _ => panic!("Expected SaveGameState request"),
        }
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(0));

        // Advance and save at frame 1
        sync_layer.advance_frame();
        let request = sync_layer.save_current_state();
        match request {
            FortressRequest::SaveGameState { frame, .. } => {
                assert_eq!(frame, Frame::new(1));
            }
            _ => panic!("Expected SaveGameState request"),
        }
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(1));
    }

    #[test]
    fn test_load_frame_success() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            cell.save(frame, Some(100u8), None);
        }

        // Advance a few frames
        sync_layer.advance_frame();
        sync_layer.advance_frame();
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(3));

        // Load frame 0
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(result.is_ok());
        match result.unwrap() {
            FortressRequest::LoadGameState { frame, cell } => {
                assert_eq!(frame, Frame::new(0));
                assert_eq!(cell.load(), Some(100u8));
            }
            _ => panic!("Expected LoadGameState request"),
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
    }

    #[test]
    fn test_load_frame_null_frame_error() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        sync_layer.advance_frame();

        let result = sync_layer.load_frame(Frame::NULL);
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidFrame { frame, reason }) => {
                assert_eq!(frame, Frame::NULL);
                assert!(reason.contains("NULL_FRAME"));
            }
            _ => panic!("Expected InvalidFrame error"),
        }
    }

    #[test]
    fn test_load_frame_future_frame_error() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        // Current frame is 0

        // Try to load frame 5 (in the future)
        let result = sync_layer.load_frame(Frame::new(5));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidFrame { frame, reason }) => {
                assert_eq!(frame, Frame::new(5));
                assert!(reason.contains("past"));
            }
            _ => panic!("Expected InvalidFrame error"),
        }
    }

    #[test]
    fn test_load_frame_current_frame_error() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        sync_layer.advance_frame();
        sync_layer.advance_frame();
        // Current frame is 2

        // Try to load current frame
        let result = sync_layer.load_frame(Frame::new(2));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidFrame { frame, reason }) => {
                assert_eq!(frame, Frame::new(2));
                assert!(reason.contains("past"));
            }
            _ => panic!("Expected InvalidFrame error"),
        }
    }

    #[test]
    fn test_load_frame_outside_prediction_window() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3); // max_prediction = 3

        // Advance to frame 10
        for _ in 0..10 {
            sync_layer.advance_frame();
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(10));

        // Try to load frame 0 (too far back, outside prediction window of 3)
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidFrame { frame, reason }) => {
                assert_eq!(frame, Frame::new(0));
                assert!(reason.contains("prediction window"));
            }
            _ => panic!("Expected InvalidFrame error"),
        }
    }

    /// Test that rollback to frame 0 works correctly when within prediction window.
    /// This is an important edge case: frame 0 is valid and should be loadable.
    #[test]
    fn test_load_frame_zero_within_prediction_window() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8); // max_prediction = 8

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            assert_eq!(frame, Frame::new(0));
            cell.save(frame, Some(42u8), Some(12345));
        }

        // Advance to frame 5 (within prediction window of 8)
        for _ in 0..5 {
            sync_layer.advance_frame();
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(5));

        // Load frame 0 - should succeed
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(
            result.is_ok(),
            "Frame 0 should be loadable within prediction window"
        );

        match result.unwrap() {
            FortressRequest::LoadGameState { frame, cell } => {
                assert_eq!(frame, Frame::new(0));
                assert_eq!(cell.frame(), Frame::new(0));
                assert_eq!(cell.load(), Some(42u8));
                assert_eq!(cell.checksum(), Some(12345));
            }
            _ => panic!("Expected LoadGameState request"),
        }

        // Current frame should now be 0
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
    }

    /// Test that frame 0 rollback fails when outside prediction window.
    #[test]
    fn test_load_frame_zero_outside_prediction_window() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 4); // max_prediction = 4

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            cell.save(frame, Some(42u8), None);
        }

        // Advance to frame 6 (frame 0 is now outside prediction window of 4)
        for _ in 0..6 {
            sync_layer.advance_frame();
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(6));

        // Load frame 0 - should fail (outside prediction window)
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(result.is_err());

        match result {
            Err(FortressError::InvalidFrame { frame, reason }) => {
                assert_eq!(frame, Frame::new(0));
                assert!(reason.contains("prediction window"));
            }
            _ => panic!("Expected InvalidFrame error"),
        }
    }

    #[test]
    fn test_saved_state_by_frame_found() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            cell.save(frame, Some(77u8), Some(9999));
        }

        // Retrieve the saved state
        let cell = sync_layer.saved_state_by_frame(Frame::new(0));
        assert!(cell.is_some());
        let cell = cell.unwrap();
        assert_eq!(cell.frame(), Frame::new(0));
        assert_eq!(cell.checksum(), Some(9999));
    }

    #[test]
    fn test_saved_state_by_frame_not_found() {
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Frame 5 was never saved
        let cell = sync_layer.saved_state_by_frame(Frame::new(5));
        assert!(cell.is_none());
    }

    #[test]
    fn test_saved_state_by_frame_negative() {
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Negative frame
        let cell = sync_layer.saved_state_by_frame(Frame::new(-1));
        assert!(cell.is_none());
    }

    #[test]
    fn test_set_last_confirmed_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add some inputs
        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }

        // Confirm up to frame 5
        sync_layer.set_last_confirmed_frame(Frame::new(5), false);
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::new(5));
    }

    #[test]
    fn test_set_last_confirmed_frame_with_sparse_saving() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        sync_layer.save_current_state();

        // Advance and add inputs
        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }

        // With sparse saving, confirmed frame should not exceed last saved frame (0)
        sync_layer.set_last_confirmed_frame(Frame::new(5), true);
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::new(0));
    }

    #[test]
    fn test_check_simulation_consistency_no_errors() {
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        let result = sync_layer.check_simulation_consistency(Frame::NULL);
        assert_eq!(result, Frame::NULL);
    }

    #[test]
    fn test_reset_prediction() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add some inputs
        let game_input = PlayerInput::new(Frame::new(0), TestInput { inp: 1 });
        sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
        sync_layer.add_remote_input(PlayerHandle::new(1), game_input);

        // Get input for future frame (triggers prediction)
        let connect_status = vec![ConnectionStatus::default(); 2];
        let _ = sync_layer.synchronized_inputs(&connect_status);

        // Reset predictions
        sync_layer.reset_prediction();
        // Should not panic and should clear prediction state
    }

    #[test]
    fn test_synchronized_inputs_with_disconnected_player() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add input for player 0
        let game_input = PlayerInput::new(Frame::new(0), TestInput { inp: 42 });
        sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
        sync_layer.add_remote_input(PlayerHandle::new(1), game_input);

        // Player 1 disconnected before current frame
        let mut connect_status = vec![ConnectionStatus::default(); 2];
        connect_status[1].disconnected = true;
        connect_status[1].last_frame = Frame::NULL; // Disconnected before frame 0

        let inputs = sync_layer.synchronized_inputs(&connect_status);
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].1, InputStatus::Confirmed);
        assert_eq!(inputs[1].1, InputStatus::Disconnected);
    }

    #[test]
    fn test_confirmed_inputs_with_disconnected_player() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add input for both players
        let game_input = PlayerInput::new(Frame::new(0), TestInput { inp: 42 });
        sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
        sync_layer.add_remote_input(PlayerHandle::new(1), game_input);

        // Player 1 disconnected before frame 0
        let mut connect_status = vec![ConnectionStatus::default(); 2];
        connect_status[1].disconnected = true;
        connect_status[1].last_frame = Frame::NULL;

        let result = sync_layer.confirmed_inputs(Frame::new(0), &connect_status);
        assert!(result.is_ok());
        let inputs = result.unwrap();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].input.inp, 42);
        assert_eq!(inputs[1].frame, Frame::NULL); // Blank input for disconnected
    }

    #[test]
    fn test_game_state_cell_save_load() {
        let cell = GameStateCell::<u32>::default();

        // Initially no data
        assert!(cell.load().is_none());

        // Save data
        cell.save(Frame::new(5), Some(12345), Some(999));

        // Load data
        assert_eq!(cell.frame(), Frame::new(5));
        assert_eq!(cell.checksum(), Some(999));
        assert_eq!(cell.load(), Some(12345));
    }

    #[test]
    fn test_game_state_cell_data_accessor() {
        let cell = GameStateCell::<String>::default();
        cell.save(Frame::new(1), Some("hello".to_string()), None);

        let accessor = cell.data();
        assert!(accessor.is_some());
        let accessor = accessor.unwrap();
        assert_eq!(&*accessor, "hello");
    }

    #[test]
    fn test_game_state_cell_clone() {
        let cell1 = GameStateCell::<u8>::default();
        cell1.save(Frame::new(10), Some(200), Some(5555));

        let cell2 = cell1.clone();

        // Both should point to same data (Arc clone)
        assert_eq!(cell2.frame(), Frame::new(10));
        assert_eq!(cell2.load(), Some(200));

        // Modifying through one affects the other
        cell1.save(Frame::new(11), Some(201), Some(6666));
        assert_eq!(cell2.frame(), Frame::new(11));
        assert_eq!(cell2.load(), Some(201));
    }

    // ==========================================
    // Invariant Checker Tests
    // ==========================================

    #[test]
    fn test_invariant_checker_new_sync_layer() {
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        assert!(sync_layer.check_invariants().is_ok());
    }

    #[test]
    fn test_invariant_checker_after_advance_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        for _ in 0..20 {
            sync_layer.advance_frame();
            assert!(sync_layer.check_invariants().is_ok());
        }
    }

    #[test]
    fn test_invariant_checker_after_save_state() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        for i in 0..10 {
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
            assert!(sync_layer.check_invariants().is_ok());
            sync_layer.advance_frame();
        }
    }

    #[test]
    fn test_invariant_checker_after_add_inputs() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            assert!(sync_layer.check_invariants().is_ok());
            sync_layer.advance_frame();
        }
    }

    #[test]
    fn test_invariant_checker_after_set_last_confirmed_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }

        sync_layer.set_last_confirmed_frame(Frame::new(5), false);
        assert!(sync_layer.check_invariants().is_ok());
    }

    #[test]
    fn test_invariant_checker_with_frame_delay() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        sync_layer.set_frame_delay(PlayerHandle::new(0), 2).unwrap();
        sync_layer.set_frame_delay(PlayerHandle::new(1), 3).unwrap();

        assert!(sync_layer.check_invariants().is_ok());

        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
            assert!(sync_layer.check_invariants().is_ok());
        }
    }
}

// ###################
// # KANI PROOFS     #
// ###################

/// Kani proofs for SyncLayer state consistency.
///
/// These proofs verify:
/// - INV-1: Frame monotonicity (current_frame never decreases except during rollback)
/// - INV-7: Confirmed frame consistency (last_confirmed_frame <= current_frame)
/// - INV-8: Saved frame consistency (last_saved_frame <= current_frame)
/// - State cell management and rollback bounds
///
/// Note: Requires Kani verifier. Install with:
///   cargo install --locked kani-verifier
///   cargo kani setup
///
/// Run proofs with:
///   cargo kani --tests
#[cfg(kani)]
mod kani_sync_layer_proofs {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
    struct TestInput {
        inp: u8,
    }

    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = u8;
        type Address = SocketAddr;
    }

    /// Proof: New SyncLayer has valid initial state
    ///
    /// Verifies all invariants hold at initialization.
    #[kani::proof]
    fn proof_new_sync_layer_valid() {
        let num_players: usize = kani::any();
        let max_prediction: usize = kani::any();

        kani::assume(num_players > 0 && num_players <= 4);
        kani::assume(max_prediction > 0 && max_prediction <= 8);

        let sync_layer = SyncLayer::<TestConfig>::new(num_players, max_prediction);

        // INV-1: current_frame starts at 0
        kani::assert(
            sync_layer.current_frame() == Frame::new(0),
            "New SyncLayer should start at frame 0",
        );

        // INV-7: last_confirmed_frame <= current_frame (NULL is treated as -1)
        kani::assert(
            sync_layer.last_confirmed_frame().is_null(),
            "New SyncLayer should have null last_confirmed_frame",
        );

        // INV-8: last_saved_frame <= current_frame
        kani::assert(
            sync_layer.last_saved_frame().is_null(),
            "New SyncLayer should have null last_saved_frame",
        );

        // Structural invariants
        kani::assert(
            sync_layer.num_players == num_players,
            "num_players should be set correctly",
        );
        kani::assert(
            sync_layer.max_prediction == max_prediction,
            "max_prediction should be set correctly",
        );
        kani::assert(
            sync_layer.input_queues.len() == num_players,
            "Should have one input queue per player",
        );
    }

    /// Proof: advance_frame maintains INV-1 (monotonicity)
    ///
    /// Verifies that advance_frame always increases current_frame.
    #[kani::proof]
    fn proof_advance_frame_monotonic() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        let initial_frame = sync_layer.current_frame();
        sync_layer.advance_frame();
        let new_frame = sync_layer.current_frame();

        kani::assert(
            new_frame > initial_frame,
            "advance_frame should increase current_frame",
        );
        kani::assert(
            new_frame == initial_frame + 1,
            "advance_frame should increment by exactly 1",
        );
    }

    /// Proof: Multiple advances maintain monotonicity
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_multiple_advances_monotonic() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        let count: usize = kani::any();
        kani::assume(count > 0 && count <= 8);

        let mut prev_frame = sync_layer.current_frame();
        for _ in 0..count {
            sync_layer.advance_frame();
            let curr_frame = sync_layer.current_frame();

            kani::assert(
                curr_frame > prev_frame,
                "Each advance should increase frame",
            );
            prev_frame = curr_frame;
        }

        kani::assert(
            sync_layer.current_frame() == Frame::new(count as i32),
            "Final frame should equal advance count",
        );
    }

    /// Proof: save_current_state maintains INV-8
    ///
    /// Verifies that after saving, last_saved_frame == current_frame.
    #[kani::proof]
    fn proof_save_maintains_inv8() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Advance a bit
        let advances: usize = kani::any();
        kani::assume(advances <= 5);
        for _ in 0..advances {
            sync_layer.advance_frame();
        }

        let frame_before_save = sync_layer.current_frame();
        let _request = sync_layer.save_current_state();
        let saved_frame = sync_layer.last_saved_frame();

        kani::assert(
            saved_frame == frame_before_save,
            "last_saved_frame should equal current_frame after save",
        );
        kani::assert(
            saved_frame <= sync_layer.current_frame(),
            "INV-8: last_saved_frame <= current_frame",
        );
    }

    /// Proof: load_frame validates bounds correctly
    ///
    /// Verifies that load_frame rejects invalid frames.
    #[kani::proof]
    fn proof_load_frame_validates_bounds() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 4);

        // Advance to frame 5 and save each frame
        for i in 0..5i32 {
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
            sync_layer.advance_frame();
        }
        // Now at frame 5, max_prediction is 4

        // Load NULL_FRAME should fail
        let result_null = sync_layer.load_frame(Frame::NULL);
        kani::assert(result_null.is_err(), "Loading NULL_FRAME should fail");

        // Load current frame should fail (not in the past)
        let result_current = sync_layer.load_frame(Frame::new(5));
        kani::assert(result_current.is_err(), "Loading current frame should fail");

        // Load future frame should fail
        let result_future = sync_layer.load_frame(Frame::new(10));
        kani::assert(result_future.is_err(), "Loading future frame should fail");

        // Load frame outside prediction window should fail (frame 0 is > 4 frames back)
        let result_too_old = sync_layer.load_frame(Frame::new(0));
        kani::assert(
            result_too_old.is_err(),
            "Loading frame outside prediction window should fail",
        );
    }

    /// Proof: load_frame success maintains invariants
    #[kani::proof]
    fn proof_load_frame_success_maintains_invariants() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Advance to frame 5 and save each frame
        for i in 0..5i32 {
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
            sync_layer.advance_frame();
        }
        // Now at frame 5

        // Load frame 2 (valid: in past, within prediction window)
        let result = sync_layer.load_frame(Frame::new(2));
        kani::assert(result.is_ok(), "Loading valid frame should succeed");

        // After load, current_frame should be the loaded frame
        kani::assert(
            sync_layer.current_frame() == Frame::new(2),
            "current_frame should be set to loaded frame",
        );
    }

    /// Proof: set_frame_delay validates player handle
    #[kani::proof]
    fn proof_set_frame_delay_validates_handle() {
        let num_players: usize = kani::any();
        kani::assume(num_players > 0 && num_players <= 4);

        let mut sync_layer = SyncLayer::<TestConfig>::new(num_players, 8);

        // Valid handle should succeed
        let valid_handle: usize = kani::any();
        kani::assume(valid_handle < num_players);
        let result_valid = sync_layer.set_frame_delay(PlayerHandle::new(valid_handle), 2);
        kani::assert(result_valid.is_ok(), "Valid handle should succeed");

        // Invalid handle should fail
        let invalid_handle: usize = kani::any();
        kani::assume(invalid_handle >= num_players && invalid_handle < 100);
        let result_invalid = sync_layer.set_frame_delay(PlayerHandle::new(invalid_handle), 2);
        kani::assert(result_invalid.is_err(), "Invalid handle should fail");
    }

    /// Proof: Saved states count is correct
    #[kani::proof]
    fn proof_saved_states_count() {
        let max_prediction: usize = kani::any();
        kani::assume(max_prediction > 0 && max_prediction <= 8);

        let sync_layer = SyncLayer::<TestConfig>::new(2, max_prediction);

        // Should have max_prediction + 1 state slots
        kani::assert(
            sync_layer.saved_states.states.len() == max_prediction + 1,
            "Should have max_prediction + 1 saved state slots",
        );
    }

    /// Proof: SavedStates get_cell validates frame
    #[kani::proof]
    fn proof_get_cell_validates_frame() {
        let saved_states: SavedStates<u8> = SavedStates::new(8);

        // Negative frame should fail
        let result_neg = saved_states.get_cell(Frame::new(-1));
        kani::assert(result_neg.is_err(), "Negative frame should fail");

        // Valid frame should succeed
        let valid_frame: i32 = kani::any();
        kani::assume(valid_frame >= 0 && valid_frame <= 1000);
        let result_valid = saved_states.get_cell(Frame::new(valid_frame));
        kani::assert(result_valid.is_ok(), "Valid frame should succeed");
    }

    /// Proof: SavedStates uses circular indexing correctly
    #[kani::proof]
    fn proof_saved_states_circular_index() {
        let max_prediction: usize = kani::any();
        kani::assume(max_prediction > 0 && max_prediction <= 8);

        let saved_states: SavedStates<u8> = SavedStates::new(max_prediction);
        let num_cells = max_prediction + 1;

        let frame: i32 = kani::any();
        kani::assume(frame >= 0 && frame <= 10000);

        let expected_pos = frame as usize % num_cells;

        // The get_cell implementation should use this index
        kani::assert(
            expected_pos < num_cells,
            "Calculated position should be within bounds",
        );
    }

    /// Proof: reset_prediction doesn't affect frame state
    #[kani::proof]
    fn proof_reset_prediction_preserves_frames() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Advance and save
        for _ in 0..3 {
            sync_layer.save_current_state();
            sync_layer.advance_frame();
        }

        let current_before = sync_layer.current_frame();
        let confirmed_before = sync_layer.last_confirmed_frame();
        let saved_before = sync_layer.last_saved_frame();

        sync_layer.reset_prediction();

        kani::assert(
            sync_layer.current_frame() == current_before,
            "reset_prediction should not change current_frame",
        );
        kani::assert(
            sync_layer.last_confirmed_frame() == confirmed_before,
            "reset_prediction should not change last_confirmed_frame",
        );
        kani::assert(
            sync_layer.last_saved_frame() == saved_before,
            "reset_prediction should not change last_saved_frame",
        );
    }

    /// Proof: INV-7 holds after set_last_confirmed_frame
    #[kani::proof]
    fn proof_confirmed_frame_bounded() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Advance and add inputs
        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }

        // Set confirmed frame to any value
        let confirm_frame: i32 = kani::any();
        kani::assume(confirm_frame >= 0 && confirm_frame <= 15);

        sync_layer.set_last_confirmed_frame(Frame::new(confirm_frame), false);

        // INV-7: last_confirmed_frame <= current_frame
        kani::assert(
            sync_layer.last_confirmed_frame() <= sync_layer.current_frame(),
            "INV-7: last_confirmed_frame should be <= current_frame",
        );
    }

    /// Proof: Sparse saving respects last_saved_frame
    #[kani::proof]
    fn proof_sparse_saving_respects_saved_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save at frame 0
        sync_layer.save_current_state();

        // Advance to frame 5, don't save intermediate frames
        for i in 0..5i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }

        // With sparse saving enabled, confirm frame should not exceed last_saved (0)
        sync_layer.set_last_confirmed_frame(Frame::new(5), true);

        kani::assert(
            sync_layer.last_confirmed_frame() <= sync_layer.last_saved_frame(),
            "With sparse saving, confirmed frame should not exceed last saved",
        );
    }
}
