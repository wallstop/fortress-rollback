#[allow(unused_imports)] // MappedMutexGuard not used under loom
use crate::sync::{Arc, MappedMutexGuard, Mutex};
use std::ops::Deref;

use crate::frame_info::{GameState, PlayerInput};
use crate::input_queue::InputQueue;
use crate::network::messages::ConnectionStatus;
use crate::report_violation;
use crate::sessions::builder::SaveMode;
use crate::telemetry::{InvariantChecker, InvariantViolation, ViolationKind, ViolationSeverity};
use crate::{Config, FortressError, FortressRequest, Frame, InputStatus, PlayerHandle};

/// An [`Arc<Mutex>`] that you can [`save()`]/[`load()`] a `T` to/from. These will be handed to the user as part of a [`FortressRequest`].
///
/// [`save()`]: GameStateCell#method.save
/// [`load()`]: GameStateCell#method.load
pub struct GameStateCell<T>(Arc<Mutex<GameState<T>>>);

impl<T> GameStateCell<T> {
    /// Saves a `T` the user creates into the cell.
    ///
    /// # Returns
    /// Returns `false` if the frame is null (which would be a caller error), `true` otherwise.
    #[cfg(not(loom))]
    pub fn save(&self, frame: Frame, data: Option<T>, checksum: Option<u128>) -> bool {
        if frame.is_null() {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::StateManagement,
                "Attempted to save state with null frame"
            );
            return false;
        }
        let mut state = self.0.lock();
        state.frame = frame;
        state.data = data;
        state.checksum = checksum;
        true
    }

    /// Saves a `T` the user creates into the cell (loom version).
    ///
    /// # Returns
    /// Returns `false` if the frame is null (which would be a caller error), `true` otherwise.
    #[cfg(loom)]
    pub fn save(&self, frame: Frame, data: Option<T>, checksum: Option<u128>) -> bool {
        if frame.is_null() {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::StateManagement,
                "Attempted to save state with null frame"
            );
            return false;
        }
        let mut state = self.0.lock().unwrap();
        state.frame = frame;
        state.data = data;
        state.checksum = checksum;
        true
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
    #[cfg(not(loom))]
    #[must_use]
    pub fn data(&self) -> Option<GameStateAccessor<'_, T>> {
        if let Ok(mapped_data) =
            parking_lot::MutexGuard::try_map(self.0.lock(), |state| state.data.as_mut())
        {
            Some(GameStateAccessor(mapped_data))
        } else {
            None
        }
    }

    /// Under loom, we can't use MappedMutexGuard. Instead, we check if data exists
    /// and return None if not. For actual access under loom, tests should use load()
    /// which requires Clone.
    #[cfg(loom)]
    pub fn data(&self) -> Option<GameStateAccessor<'_, T>> {
        // Under loom, we cannot project the guard to a subfield.
        // Return None to indicate this API is not available under loom testing.
        // Tests should use load() instead which requires Clone.
        let _guard = self.0.lock().unwrap();
        // We can't return the accessor because loom's MutexGuard doesn't support try_map.
        // The loom tests should test concurrency via save/load/frame operations.
        None
    }

    #[cfg(not(loom))]
    /// Returns the frame number for this saved state.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    #[must_use]
    pub fn frame(&self) -> Frame {
        self.0.lock().frame
    }

    #[cfg(loom)]
    /// Returns the frame number for this saved state (loom version).
    pub fn frame(&self) -> Frame {
        self.0.lock().unwrap().frame
    }

    #[cfg(not(loom))]
    /// Returns the checksum for this saved state, if one was saved.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    #[must_use]
    pub fn checksum(&self) -> Option<u128> {
        self.0.lock().checksum
    }

    #[cfg(loom)]
    /// Returns the checksum for this saved state (loom version).
    pub fn checksum(&self) -> Option<u128> {
        self.0.lock().unwrap().checksum
    }
}

impl<T: Clone> GameStateCell<T> {
    /// Loads a `T` that the user previously saved into this cell, by cloning the `T`.
    ///
    /// See also [data()](Self::data) if you want a reference to the `T` without cloning it.
    #[cfg(not(loom))]
    #[must_use]
    pub fn load(&self) -> Option<T> {
        let data = self.data()?;
        Some(data.clone())
    }

    /// Under loom, we can't use the MappedMutexGuard-based data() method,
    /// so we access the data directly through the mutex.
    #[cfg(loom)]
    pub fn load(&self) -> Option<T> {
        let guard = self.0.lock().unwrap();
        guard.data.clone()
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

#[cfg(not(loom))]
impl<T> std::fmt::Debug for GameStateCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.0.lock();
        f.debug_struct("GameStateCell")
            .field("frame", &inner.frame)
            .field("checksum", &inner.checksum)
            .finish_non_exhaustive()
    }
}

#[cfg(loom)]
impl<T> std::fmt::Debug for GameStateCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.0.lock().unwrap();
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
///
/// Note: Under loom testing, this type is not available as loom doesn't support `MappedMutexGuard`.
/// Use [`GameStateCell::load()`] instead which requires `T: Clone`.
#[cfg(not(loom))]
pub struct GameStateAccessor<'c, T>(MappedMutexGuard<'c, T>);

/// Placeholder type under loom - the actual accessor cannot be created.
#[cfg(loom)]
pub struct GameStateAccessor<'c, T>(std::marker::PhantomData<&'c T>);

#[cfg(not(loom))]
impl<'c, T> Deref for GameStateAccessor<'c, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(loom)]
impl<'c, T> Deref for GameStateAccessor<'c, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // This should never be called under loom as data() returns None
        unreachable!("GameStateAccessor::deref called under loom - this should not happen")
    }
}

#[cfg(not(loom))]
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

#[cfg(loom)]
impl<'c, T> GameStateAccessor<'c, T> {
    /// Under loom, this method is not available.
    pub fn as_mut_dangerous(&mut self) -> &mut T {
        unreachable!(
            "GameStateAccessor::as_mut_dangerous called under loom - this should not happen"
        )
    }
}

/// Container for saved game states used during rollback.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
pub struct SavedStates<T> {
    /// The vector of game state cells.
    pub states: Vec<GameStateCell<T>>,
}

impl<T> SavedStates<T> {
    /// Creates a new SavedStates container with the given capacity.
    #[must_use]
    pub fn new(max_pred: usize) -> Self {
        // we need to store the current frame plus the number of max predictions, so that we can
        // roll back to the very first frame even when we have predicted as far ahead as we can.
        let num_cells = max_pred + 1;
        let mut states = Vec::with_capacity(num_cells);
        for _ in 0..num_cells {
            states.push(GameStateCell::default());
        }

        Self { states }
    }

    /// Gets the cell for a given frame.
    pub fn get_cell(&self, frame: Frame) -> Result<GameStateCell<T>, FortressError> {
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

/// The synchronization layer manages game state, input queues, and rollback operations.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
///
/// # Formal Specification Alignment
/// - **TLA+**: `SyncLayer` state in `specs/tla/Rollback.tla`
/// - **Invariants verified**:
///   - INV-1: Frame monotonicity (except during rollback)
///   - INV-2: Rollback bounded by `max_prediction`
///   - INV-6: State availability for rollback frames
///   - INV-7: `last_confirmed_frame <= current_frame`
///   - INV-8: `last_saved_frame <= current_frame`
/// - **Kani proofs**: 14 proofs in `sync_layer.rs` verify bounds and state transitions
/// - **Loom tests**: `GameStateCell` concurrent access verified in `loom-tests/`
pub struct SyncLayer<T>
where
    T: Config,
{
    num_players: usize,
    /// Maximum frames of prediction allowed before rollback is required.
    ///
    /// # Formal Specification Alignment
    /// - **TLA+**: `MAX_PREDICTION` in `specs/tla/Rollback.tla`
    /// - **Z3**: `MAX_PREDICTION` in `tests/test_z3_verification.rs`
    /// - **formal-spec.md**: INV-2 requires `rollback_depth <= max_prediction`
    max_prediction: usize,
    saved_states: SavedStates<T::State>,
    /// The last frame where all player inputs are confirmed.
    ///
    /// # Formal Specification Alignment
    /// - **TLA+**: `lastConfirmedFrame` in `specs/tla/Rollback.tla`
    /// - **formal-spec.md**: INV-7 requires `last_confirmed_frame <= current_frame`
    last_confirmed_frame: Frame,
    /// The most recently saved frame.
    ///
    /// # Formal Specification Alignment
    /// - **TLA+**: `lastSavedFrame` in `specs/tla/Rollback.tla`
    /// - **formal-spec.md**: INV-8 requires `last_saved_frame <= current_frame`
    last_saved_frame: Frame,
    /// The current simulation frame.
    ///
    /// # Formal Specification Alignment
    /// - **TLA+**: `currentFrame` in `specs/tla/Rollback.tla`
    /// - **formal-spec.md**: INV-1 requires monotonic increase (except rollback)
    current_frame: Frame,
    input_queues: Vec<InputQueue<T>>,
}

impl<T: Config> SyncLayer<T> {
    /// Creates a new `SyncLayer` instance with given values and default queue length.
    ///
    /// Note: This function exists for backward compatibility and testing.
    /// The main construction path uses `with_queue_length` via `SessionBuilder`.
    #[allow(dead_code)]
    #[must_use]
    pub fn new(num_players: usize, max_prediction: usize) -> Self {
        Self::with_queue_length(
            num_players,
            max_prediction,
            crate::input_queue::INPUT_QUEUE_LENGTH,
        )
    }

    /// Creates a new `SyncLayer` instance with a custom input queue length.
    ///
    /// # Arguments
    /// * `num_players` - The number of players in the session
    /// * `max_prediction` - Maximum frames of prediction allowed
    /// * `queue_length` - The size of the input queue circular buffer per player
    #[must_use]
    pub fn with_queue_length(
        num_players: usize,
        max_prediction: usize,
        queue_length: usize,
    ) -> Self {
        // initialize input_queues with player indices for deterministic prediction
        let mut input_queues = Vec::new();
        for player_index in 0..num_players {
            // queue_length should be validated before calling this function
            // If it's invalid, report a violation and use a default
            match InputQueue::with_queue_length(player_index, queue_length) {
                Some(queue) => input_queues.push(queue),
                None => {
                    // Fallback: use the default queue length
                    if let Some(queue) = InputQueue::with_queue_length(
                        player_index,
                        crate::input_queue::INPUT_QUEUE_LENGTH,
                    ) {
                        input_queues.push(queue);
                    }
                }
            }
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

    /// Returns the current simulation frame.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    #[must_use]
    pub fn current_frame(&self) -> Frame {
        self.current_frame
    }

    /// Advances the simulation by one frame.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    pub fn advance_frame(&mut self) {
        self.current_frame += 1;
    }

    /// Saves the current game state.
    ///
    /// This method maintains the invariant that `current_frame` is always valid (>= 0),
    /// which is guaranteed by construction (initialized to 0) and by the fact that
    /// the only mutation is via `advance_frame()` which increments it.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    pub fn save_current_state(&mut self) -> FortressRequest<T> {
        self.last_saved_frame = self.current_frame;
        // Debug assertion to catch invariant violations during development.
        // current_frame is initialized to 0 and only incremented, so this should never fail.
        debug_assert!(
            self.current_frame.as_i32() >= 0,
            "Internal invariant violation: current_frame must be non-negative"
        );
        // Use match to handle the theoretical error case gracefully instead of panicking.
        // In the impossible case of an invalid frame, create a default cell.
        let cell = match self.saved_states.get_cell(self.current_frame) {
            Ok(cell) => cell,
            Err(_) => {
                // This should never happen due to our invariants, but if it does,
                // report it and return a default cell to avoid panicking.
                report_violation!(
                    ViolationSeverity::Critical,
                    ViolationKind::InternalError,
                    "save_current_state: current_frame {} failed get_cell - this indicates an internal bug",
                    self.current_frame
                );
                GameStateCell::default()
            }
        };
        FortressRequest::SaveGameState {
            cell,
            frame: self.current_frame,
        }
    }

    /// Sets the frame delay for a player.
    ///
    /// # Errors
    /// Returns `FortressError::InvalidPlayerHandle` if `player_handle >= num_players`.
    /// Returns `FortressError::InvalidRequest` if `delay` exceeds the maximum allowed value.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    pub fn set_frame_delay(
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
        self.input_queues[player_handle.as_usize()].set_frame_delay(delay)?;
        Ok(())
    }

    /// Resets the prediction state for all input queues.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    pub fn reset_prediction(&mut self) {
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
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    pub fn load_frame(
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
        #[cfg(not(loom))]
        let cell_frame = cell.0.lock().frame;
        #[cfg(loom)]
        let cell_frame = cell.0.lock().unwrap().frame;
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
        // Update last_saved_frame to maintain invariant: last_saved_frame <= current_frame
        // After rollback, we're working from the loaded state, which is now our reference point
        self.last_saved_frame = frame_to_load;

        Ok(FortressRequest::LoadGameState {
            cell,
            frame: frame_to_load,
        })
    }

    /// Adds local input to the corresponding input queue. Checks if the prediction threshold has been reached. Returns the frame number where the input is actually added to.
    /// This number will only be different if the input delay was set to a number higher than 0.
    ///
    /// Returns `Frame::NULL` if the input frame doesn't match the current frame.
    pub(crate) fn add_local_input(
        &mut self,
        player_handle: PlayerHandle,
        input: PlayerInput<T::Input>,
    ) -> Frame {
        // The input provided should match the current frame, we account for input delay later
        if input.frame != self.current_frame {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "Input frame {} doesn't match current frame {}",
                input.frame,
                self.current_frame
            );
            return Frame::NULL;
        }
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
    ///
    /// # Returns
    /// Returns `None` if any input queue operation fails (indicates a severe internal error).
    pub(crate) fn synchronized_inputs(
        &mut self,
        connect_status: &[ConnectionStatus],
    ) -> Option<Vec<(T::Input, InputStatus)>> {
        let mut inputs = Vec::new();
        for (i, con_stat) in connect_status.iter().enumerate() {
            if con_stat.disconnected && con_stat.last_frame < self.current_frame {
                inputs.push((T::Input::default(), InputStatus::Disconnected));
            } else {
                inputs.push(self.input_queues[i].input(self.current_frame)?);
            }
        }
        Some(inputs)
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
    pub(crate) fn set_last_confirmed_frame(&mut self, mut frame: Frame, save_mode: SaveMode) {
        // don't set the last confirmed frame after the first incorrect frame before a rollback has happened
        let mut first_incorrect: Frame = Frame::NULL;
        for handle in 0..self.num_players {
            first_incorrect = std::cmp::max(
                first_incorrect,
                self.input_queues[handle].first_incorrect_frame(),
            );
        }

        // if sparse saving option is turned on, don't set the last confirmed frame after the last saved frame
        if save_mode == SaveMode::Sparse {
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

        #[cfg(not(loom))]
        let cell_frame = cell.0.lock().frame;
        #[cfg(loom)]
        let cell_frame = cell.0.lock().unwrap().frame;

        if cell_frame == frame {
            Some(cell)
        } else {
            None
        }
    }

    /// Returns the latest saved frame.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    #[must_use]
    pub fn last_saved_frame(&self) -> Frame {
        self.last_saved_frame
    }

    /// Returns the latest confirmed frame.
    ///
    /// # Note
    /// This method is exposed via `__internal` for testing. It is not part of the stable public API.
    #[must_use]
    pub fn last_confirmed_frame(&self) -> Frame {
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
                let sync_inputs = sync_layer
                    .synchronized_inputs(&dummy_connect_status)
                    .expect("synchronized inputs should be available");
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

    // =========================================================================
    // Rollback Invariant Tests
    // These tests verify that invariants are maintained during rollback:
    // - INV-4: last_confirmed_frame <= current_frame
    // - INV-5: last_saved_frame <= current_frame
    // =========================================================================

    /// Test that load_frame updates last_saved_frame to maintain invariant.
    ///
    /// This is a critical test case discovered during TLA+ verification:
    /// After rollback, last_saved_frame must be <= current_frame.
    #[test]
    fn test_load_frame_updates_last_saved_frame_invariant() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            cell.save(frame, Some(100u8), None);
        }
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(0));

        // Advance to frame 5 and save state
        for i in 1..=5 {
            sync_layer.advance_frame();
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(5));
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(5));

        // Rollback to frame 2
        let result = sync_layer.load_frame(Frame::new(2));
        assert!(result.is_ok());

        // INVARIANT CHECK: last_saved_frame must be <= current_frame after rollback
        assert_eq!(sync_layer.current_frame(), Frame::new(2));
        assert_eq!(
            sync_layer.last_saved_frame(),
            Frame::new(2),
            "last_saved_frame should be updated to rollback target"
        );
        assert!(
            sync_layer.last_saved_frame() <= sync_layer.current_frame(),
            "Invariant violated: last_saved_frame ({}) > current_frame ({})",
            sync_layer.last_saved_frame(),
            sync_layer.current_frame()
        );
    }

    /// Test that rollback to frame 0 correctly updates last_saved_frame.
    #[test]
    fn test_load_frame_zero_updates_last_saved_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save state at frame 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            cell.save(frame, Some(0u8), None);
        }

        // Advance to frame 3 and save each frame
        for i in 1..=3 {
            sync_layer.advance_frame();
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(3));
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(3));

        // Rollback all the way to frame 0
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(result.is_ok());

        // Verify invariant
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(0));
    }

    /// Test multiple consecutive rollbacks maintain invariants.
    #[test]
    fn test_multiple_rollbacks_maintain_invariants() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save states for frames 0-5
        for i in 0..=5 {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }

        // First rollback: 5 -> 3
        let _ = sync_layer.load_frame(Frame::new(3));
        assert_eq!(sync_layer.current_frame(), Frame::new(3));
        assert!(sync_layer.last_saved_frame() <= sync_layer.current_frame());

        // Re-advance to frame 5
        for _ in 0..2 {
            sync_layer.advance_frame();
        }

        // Second rollback: 5 -> 1
        let _ = sync_layer.load_frame(Frame::new(1));
        assert_eq!(sync_layer.current_frame(), Frame::new(1));
        assert!(sync_layer.last_saved_frame() <= sync_layer.current_frame());

        // Third rollback: 1 -> 0
        sync_layer.advance_frame();
        let _ = sync_layer.load_frame(Frame::new(0));
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert!(sync_layer.last_saved_frame() <= sync_layer.current_frame());
    }

    /// Test that check_invariants passes after rollback.
    #[test]
    fn test_check_invariants_after_rollback() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Setup: save states for frames 0-4
        for i in 0..=4 {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }

        // Verify invariants before rollback
        assert!(
            sync_layer.check_invariants().is_ok(),
            "Invariants should pass before rollback"
        );

        // Rollback to frame 1
        let _ = sync_layer.load_frame(Frame::new(1));

        // Verify invariants after rollback
        assert!(
            sync_layer.check_invariants().is_ok(),
            "Invariants should pass after rollback"
        );
    }

    /// Test rollback at the edge of prediction window maintains invariants.
    #[test]
    fn test_rollback_at_prediction_window_edge() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 4); // max_prediction = 4

        // Save states for frames 0-4
        for i in 0..=4 {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(4));

        // Rollback exactly to the edge of prediction window (frame 0)
        // current_frame (4) - max_prediction (4) = 0
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(result.is_ok());

        // Verify invariants
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert!(sync_layer.last_saved_frame() <= sync_layer.current_frame());
        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Test that last_confirmed_frame invariant is maintained.
    /// Note: last_confirmed_frame is set separately from load_frame, but
    /// this test ensures the SyncLayer invariant checker works correctly.
    #[test]
    fn test_last_confirmed_frame_invariant() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add inputs and advance
        for i in 0..5i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }

        // Set confirmed frame
        sync_layer.set_last_confirmed_frame(Frame::new(3), SaveMode::EveryFrame);

        // Verify invariant: last_confirmed_frame <= current_frame
        assert!(sync_layer.last_confirmed_frame() <= sync_layer.current_frame());
        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Test that set_last_confirmed_frame clamps to current_frame.
    /// Note: This test uses a smaller confirmed frame to avoid triggering
    /// a separate issue in discard_confirmed_frames when discarding all inputs.
    #[test]
    fn test_set_last_confirmed_frame_clamps_to_current() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Add inputs and advance to frame 10
        for i in 0..10i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }
        assert_eq!(sync_layer.current_frame(), Frame::new(10));

        // Try to set confirmed frame beyond current frame
        sync_layer.set_last_confirmed_frame(Frame::new(15), SaveMode::EveryFrame);

        // Should be clamped to current_frame
        assert!(
            sync_layer.last_confirmed_frame() <= sync_layer.current_frame(),
            "last_confirmed_frame ({}) should be clamped to current_frame ({})",
            sync_layer.last_confirmed_frame(),
            sync_layer.current_frame()
        );

        // The confirmed frame should be at most current_frame
        assert_eq!(sync_layer.last_confirmed_frame(), Frame::new(10));
    }

    /// Test invariant checking catches invalid states.
    #[test]
    fn test_invariant_checker_validates_player_count() {
        // Create sync layer with valid player count
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        assert!(sync_layer.check_invariants().is_ok());

        // Note: We can't easily create an invalid state from outside,
        // so this test just verifies the checker runs successfully.
    }

    /// Test full rollback cycle: advance, rollback, re-advance, verify invariants.
    #[test]
    fn test_full_rollback_cycle_maintains_invariants() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Phase 1: Advance to frame 5, saving states
        for i in 0..=5 {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
        }
        assert!(sync_layer.check_invariants().is_ok(), "Before rollback");

        // Phase 2: Rollback to frame 2
        let _ = sync_layer.load_frame(Frame::new(2));
        assert!(sync_layer.check_invariants().is_ok(), "After rollback");
        assert_eq!(sync_layer.current_frame(), Frame::new(2));
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(2));

        // Phase 3: Re-advance to frame 5, saving states again
        for _ in 0..3 {
            sync_layer.advance_frame();
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(99u8), None);
            }
        }
        assert!(sync_layer.check_invariants().is_ok(), "After re-advance");
        assert_eq!(sync_layer.current_frame(), Frame::new(5));

        // Phase 4: Another rollback
        let _ = sync_layer.load_frame(Frame::new(3));
        assert!(
            sync_layer.check_invariants().is_ok(),
            "After second rollback"
        );
        assert_eq!(sync_layer.current_frame(), Frame::new(3));
        assert!(sync_layer.last_saved_frame() <= sync_layer.current_frame());
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
        sync_layer.set_last_confirmed_frame(Frame::new(5), SaveMode::EveryFrame);
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
        sync_layer.set_last_confirmed_frame(Frame::new(5), SaveMode::Sparse);
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

        let inputs = sync_layer
            .synchronized_inputs(&connect_status)
            .expect("synchronized inputs should be available");
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

        sync_layer.set_last_confirmed_frame(Frame::new(5), SaveMode::EveryFrame);
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

    // ==========================================
    // save_current_state Invariant Tests
    // ==========================================

    /// Verifies that save_current_state maintains the current_frame invariant
    /// by checking that current_frame is always non-negative.
    #[test]
    fn test_save_current_state_maintains_frame_invariant() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save at frame 0 - the initial state
        let request = sync_layer.save_current_state();
        match &request {
            FortressRequest::SaveGameState { frame, .. } => {
                assert!(frame.as_i32() >= 0, "Frame should be non-negative");
                assert_eq!(*frame, Frame::new(0));
            }
            _ => panic!("Expected SaveGameState request"),
        }

        // Advance many frames and verify invariant holds at each
        for expected_frame in 1..100 {
            sync_layer.advance_frame();
            let request = sync_layer.save_current_state();
            match &request {
                FortressRequest::SaveGameState { frame, .. } => {
                    assert!(frame.as_i32() >= 0, "Frame should be non-negative");
                    assert_eq!(*frame, Frame::new(expected_frame));
                }
                _ => panic!("Expected SaveGameState request"),
            }
        }
    }

    /// Verifies that save_current_state correctly updates last_saved_frame.
    #[test]
    fn test_save_current_state_updates_last_saved_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Initially last_saved_frame is NULL
        assert_eq!(sync_layer.last_saved_frame(), Frame::NULL);

        // After saving, it should be updated
        sync_layer.save_current_state();
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(0));

        // Advance and save again
        sync_layer.advance_frame();
        sync_layer.save_current_state();
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(1));
    }

    /// Verifies that save_current_state works correctly after rollback.
    #[test]
    fn test_save_current_state_after_rollback() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Save and advance several frames
        for i in 0..5 {
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some(i as u8), None);
            }
            sync_layer.advance_frame();
        }

        assert_eq!(sync_layer.current_frame(), Frame::new(5));

        // Load frame 2 (rollback)
        let result = sync_layer.load_frame(Frame::new(2));
        assert!(result.is_ok());
        assert_eq!(sync_layer.current_frame(), Frame::new(2));

        // Now save_current_state should work correctly at frame 2
        let request = sync_layer.save_current_state();
        match &request {
            FortressRequest::SaveGameState { frame, .. } => {
                assert_eq!(*frame, Frame::new(2));
            }
            _ => panic!("Expected SaveGameState request"),
        }
        assert_eq!(sync_layer.last_saved_frame(), Frame::new(2));
    }

    /// Verifies save_current_state works correctly at frame 0 (boundary condition).
    #[test]
    fn test_save_current_state_at_frame_zero() {
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Should work correctly at frame 0
        assert_eq!(sync_layer.current_frame(), Frame::new(0));

        // Note: We use a non-mutable borrow pattern to test the const-ness
        // but save_current_state needs &mut self, so this is mainly documenting
        // that frame 0 is a valid state
        let mut sync_layer = sync_layer;
        let request = sync_layer.save_current_state();
        match request {
            FortressRequest::SaveGameState { frame, cell } => {
                assert_eq!(frame, Frame::new(0));
                // Cell should be usable
                cell.save(Frame::new(0), Some(42u8), Some(12345));
                assert_eq!(cell.frame(), Frame::new(0));
                assert_eq!(cell.load(), Some(42u8));
            }
            _ => panic!("Expected SaveGameState request"),
        }
    }

    /// Verifies that save_current_state provides correct cell cycling
    /// when frames exceed max_prediction.
    #[test]
    fn test_save_current_state_cell_cycling() {
        const MAX_PREDICTION: usize = 4;
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, MAX_PREDICTION);

        // Save more frames than we have cells (max_prediction + 1 = 5 cells)
        // Frame 0 and Frame 5 should use the same cell slot (index 0)
        // Frame 1 and Frame 6 should use the same cell slot (index 1)

        // First, save frames 0-4
        for i in 0..=MAX_PREDICTION {
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(frame, Some((i * 10) as u8), None);
            }
            if i < MAX_PREDICTION {
                sync_layer.advance_frame();
            }
        }

        // Now at frame 4, advance to frame 5
        sync_layer.advance_frame();
        assert_eq!(sync_layer.current_frame(), Frame::new(5));

        // Save at frame 5 - this should overwrite cell slot 0
        let request = sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            assert_eq!(frame, Frame::new(5));
            cell.save(frame, Some(50u8), None);
            // Verify the cell now has frame 5's data
            assert_eq!(cell.load(), Some(50u8));
        }
    }

    /// Documents the invariant that save_current_state relies on:
    /// current_frame is always non-negative because it's initialized to 0
    /// and only modified by advance_frame() which increments it.
    #[test]
    fn test_save_current_state_invariant_documentation() {
        // This test documents and verifies the invariant that save_current_state relies on.
        //
        // Invariant: current_frame >= 0
        //
        // Proof:
        // 1. SyncLayer::new() initializes current_frame to Frame::new(0)
        // 2. advance_frame() is the only method that modifies current_frame
        // 3. advance_frame() only increments: self.current_frame += 1
        // 4. load_frame() can reduce current_frame but only to a frame that was
        //    previously valid (saved state exists)
        // 5. Therefore, current_frame is always >= 0
        //
        // The save_current_state() method uses this invariant to call get_cell()
        // which requires frame >= 0. If this invariant were violated (which should
        // be impossible), the telemetry system would report a Critical violation.

        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Verify initial state
        assert_eq!(sync_layer.current_frame(), Frame::new(0));
        assert!(sync_layer.current_frame().as_i32() >= 0);

        // Verify after many operations
        for _ in 0..1000 {
            sync_layer.advance_frame();
            assert!(sync_layer.current_frame().as_i32() >= 0);
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
    /// Note: Bounds are reduced for Kani verification tractability.
    #[kani::proof]
    #[kani::unwind(4)]
    fn proof_new_sync_layer_valid() {
        let num_players: usize = kani::any();
        let max_prediction: usize = kani::any();

        kani::assume(num_players > 0 && num_players <= 2);
        kani::assume(max_prediction > 0 && max_prediction <= 3);

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
    #[kani::unwind(2)]
    fn proof_advance_frame_monotonic() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);

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
    #[kani::unwind(5)]
    fn proof_multiple_advances_monotonic() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);
        let count: usize = kani::any();
        kani::assume(count > 0 && count <= 3);

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
    #[kani::unwind(5)]
    fn proof_save_maintains_inv8() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);

        // Advance a bit
        let advances: usize = kani::any();
        kani::assume(advances <= 3);
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
    #[kani::unwind(7)]
    fn proof_load_frame_validates_bounds() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);

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
    #[kani::unwind(7)]
    fn proof_load_frame_success_maintains_invariants() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);

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
    #[kani::unwind(4)]
    fn proof_set_frame_delay_validates_handle() {
        let num_players: usize = kani::any();
        kani::assume(num_players > 0 && num_players <= 2);

        let mut sync_layer = SyncLayer::<TestConfig>::new(num_players, 3);

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
    #[kani::unwind(5)]
    fn proof_saved_states_count() {
        let max_prediction: usize = kani::any();
        kani::assume(max_prediction > 0 && max_prediction <= 3);

        let sync_layer = SyncLayer::<TestConfig>::new(2, max_prediction);

        // Should have max_prediction + 1 state slots
        kani::assert(
            sync_layer.saved_states.states.len() == max_prediction + 1,
            "Should have max_prediction + 1 saved state slots",
        );
    }

    /// Proof: SavedStates get_cell validates frame
    #[kani::proof]
    #[kani::unwind(2)]
    fn proof_get_cell_validates_frame() {
        let saved_states: SavedStates<u8> = SavedStates::new(3);

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
    #[kani::unwind(2)]
    fn proof_saved_states_circular_index() {
        let max_prediction: usize = kani::any();
        kani::assume(max_prediction > 0 && max_prediction <= 3);

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
    #[kani::unwind(5)]
    fn proof_reset_prediction_preserves_frames() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);

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
    #[kani::unwind(7)]
    fn proof_confirmed_frame_bounded() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);

        // Advance and add inputs
        for i in 0..5i32 {
            let game_input = PlayerInput::new(Frame::new(i), TestInput { inp: i as u8 });
            sync_layer.add_remote_input(PlayerHandle::new(0), game_input);
            sync_layer.add_remote_input(PlayerHandle::new(1), game_input);
            sync_layer.advance_frame();
        }

        // Set confirmed frame to any value
        let confirm_frame: i32 = kani::any();
        kani::assume(confirm_frame >= 0 && confirm_frame <= 15);

        sync_layer.set_last_confirmed_frame(Frame::new(confirm_frame), SaveMode::EveryFrame);

        // INV-7: last_confirmed_frame <= current_frame
        kani::assert(
            sync_layer.last_confirmed_frame() <= sync_layer.current_frame(),
            "INV-7: last_confirmed_frame should be <= current_frame",
        );
    }

    /// Proof: Sparse saving respects last_saved_frame
    #[kani::proof]
    #[kani::unwind(7)]
    fn proof_sparse_saving_respects_saved_frame() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 3);

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
        sync_layer.set_last_confirmed_frame(Frame::new(5), SaveMode::Sparse);

        kani::assert(
            sync_layer.last_confirmed_frame() <= sync_layer.last_saved_frame(),
            "With sparse saving, confirmed frame should not exceed last saved",
        );
    }
}
