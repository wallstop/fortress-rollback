//! Game state cell types for saving and loading game states during rollback.
//!
//! This module provides [`GameStateCell`] and [`GameStateAccessor`] which are
//! the primary types users interact with when handling save/load requests from
//! the rollback system.

#[allow(unused_imports)] // MappedMutexGuard not used under loom
use crate::sync::{Arc, MappedMutexGuard, Mutex};
use std::ops::Deref;

use crate::frame_info::GameState;
use crate::report_violation;
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::Frame;

/// A thread-safe cell for saving and loading game states during rollback.
///
/// `GameStateCell` wraps your game state in an [`Arc<Mutex>`], allowing it to be shared
/// between the rollback system and your game code. You receive instances of this type
/// through [`FortressRequest::SaveGameState`] and [`FortressRequest::LoadGameState`]
/// requests when the session needs to save or restore game state.
///
/// # Thread Safety
///
/// This type is `Send + Sync` and uses interior mutability via a mutex. Multiple clones
/// of the same cell share the underlying state (see the [`Clone`] implementation).
///
/// # Typical Usage
///
/// ```ignore
/// // In your game loop, handle requests from the session:
/// for request in session.advance_frame()? {
///     match request {
///         FortressRequest::SaveGameState { frame, cell } => {
///             let checksum = compute_checksum(&game_state);
///             cell.save(frame, Some(game_state.clone()), Some(checksum));
///         }
///         FortressRequest::LoadGameState { cell, frame } => {
///             if let Some(loaded) = cell.load() {
///                 game_state = loaded;
///             }
///         }
///         // ... handle other requests
///     }
/// }
/// ```
///
/// [`save()`]: GameStateCell::save
/// [`load()`]: GameStateCell::load
/// [`FortressRequest::SaveGameState`]: crate::FortressRequest::SaveGameState
/// [`FortressRequest::LoadGameState`]: crate::FortressRequest::LoadGameState
pub struct GameStateCell<T>(pub(crate) Arc<Mutex<GameState<T>>>);

impl<T> GameStateCell<T> {
    /// Saves a game state into the cell.
    ///
    /// This method stores the provided game state, frame number, and optional checksum
    /// in the cell. The cell uses interior mutability, so this works with a shared reference.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame number this state corresponds to. Must not be [`Frame::NULL`].
    /// * `data` - The game state to save, or `None` to clear the cell.
    /// * `checksum` - Optional checksum for desync detection.
    ///
    /// # Returns
    ///
    /// Returns `true` if the save succeeded, `false` if the frame was null (a caller error).
    ///
    /// # Examples
    ///
    /// Basic save with state and checksum:
    ///
    /// ```
    /// use fortress_rollback::{Frame, GameStateCell};
    ///
    /// #[derive(Clone, Default)]
    /// struct GameState {
    ///     player_x: i32,
    ///     player_y: i32,
    /// }
    ///
    /// let cell = GameStateCell::<GameState>::default();
    /// let frame = Frame::new(42);
    /// let state = GameState { player_x: 100, player_y: 200 };
    /// let checksum = Some(0xDEADBEEF_u128);
    ///
    /// // Save succeeds and returns true
    /// assert!(cell.save(frame, Some(state), checksum));
    ///
    /// // Verify the data was saved
    /// assert_eq!(cell.frame(), frame);
    /// assert_eq!(cell.checksum(), checksum);
    /// ```
    ///
    /// Saving with a null frame returns `false`:
    ///
    /// ```
    /// use fortress_rollback::{Frame, GameStateCell};
    ///
    /// let cell = GameStateCell::<u32>::default();
    ///
    /// // Null frames are rejected
    /// assert!(!cell.save(Frame::NULL, Some(42), None));
    /// ```
    ///
    /// [`Frame::NULL`]: crate::Frame::NULL
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

    /// Saves a game state into the cell (loom version).
    ///
    /// See the non-loom version for full documentation and examples.
    ///
    /// # Returns
    ///
    /// Returns `true` if the save succeeded, `false` if the frame was null.
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
    /// {
    ///     let game_state_accessor = cell.data();
    ///     if let Some(accessor) = game_state_accessor {
    ///         assert_eq!(accessor.player_name, "alex");
    ///     }
    /// }
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

    /// Returns the frame number for this saved state.
    ///
    /// Returns [`Frame::NULL`] if no state has been saved yet.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::{Frame, GameStateCell};
    ///
    /// let cell = GameStateCell::<u32>::default();
    ///
    /// // Initially null
    /// assert!(cell.frame().is_null());
    ///
    /// // After saving, returns the saved frame
    /// cell.save(Frame::new(10), Some(42), None);
    /// assert_eq!(cell.frame(), Frame::new(10));
    /// ```
    ///
    /// [`Frame::NULL`]: crate::Frame::NULL
    #[cfg(not(loom))]
    #[must_use]
    pub fn frame(&self) -> Frame {
        self.0.lock().frame
    }

    #[cfg(loom)]
    /// Returns the frame number for this saved state (loom version).
    pub fn frame(&self) -> Frame {
        self.0.lock().unwrap().frame
    }

    /// Returns the checksum for this saved state, if one was saved.
    ///
    /// Returns `None` if no state has been saved, or if the state was saved without a checksum.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::{Frame, GameStateCell};
    ///
    /// let cell = GameStateCell::<u32>::default();
    ///
    /// // Initially no checksum
    /// assert!(cell.checksum().is_none());
    ///
    /// // Save with a checksum
    /// cell.save(Frame::new(5), Some(42), Some(0x12345678));
    /// assert_eq!(cell.checksum(), Some(0x12345678));
    ///
    /// // Save without a checksum
    /// cell.save(Frame::new(6), Some(99), None);
    /// assert!(cell.checksum().is_none());
    /// ```
    #[cfg(not(loom))]
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
    /// Loads a previously saved game state by cloning it.
    ///
    /// Returns `None` if no state has been saved, or if `None` was explicitly saved.
    /// This method clones the stored state, leaving the original in place for potential
    /// future loads (e.g., during multiple rollbacks to the same frame).
    ///
    /// See also [`data()`](Self::data) if you want a reference to the `T` without cloning it.
    ///
    /// # Examples
    ///
    /// Loading a saved state:
    ///
    /// ```
    /// use fortress_rollback::{Frame, GameStateCell};
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// struct GameState {
    ///     score: u32,
    /// }
    ///
    /// let cell = GameStateCell::<GameState>::default();
    ///
    /// // No state saved yet
    /// assert!(cell.load().is_none());
    ///
    /// // Save and load
    /// let original = GameState { score: 100 };
    /// cell.save(Frame::new(1), Some(original.clone()), None);
    ///
    /// let loaded = cell.load();
    /// assert_eq!(loaded, Some(original));
    /// ```
    ///
    /// Handling the `Option` defensively (as used in request handling):
    ///
    /// ```
    /// use fortress_rollback::{Frame, GameStateCell};
    ///
    /// #[derive(Clone, Default)]
    /// struct GameState { value: i32 }
    ///
    /// let cell = GameStateCell::<GameState>::default();
    /// let mut current_state = GameState { value: 42 };
    ///
    /// // Simulate a LoadGameState request handler
    /// // (in practice, LoadGameState is only requested for previously saved frames)
    /// if let Some(loaded) = cell.load() {
    ///     current_state = loaded;
    /// }
    /// // If load() returns None, current_state is unchanged
    /// ```
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

    /// Loads a previously saved state, returning an error if none exists.
    ///
    /// Use this when you expect a state to be present (e.g., during
    /// `LoadGameState` handling). For optional loads, use [`load()`].
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame number for error context. This should match
    ///   the `frame` from the [`FortressRequest::LoadGameState`] request.
    ///
    /// # Errors
    ///
    /// Returns [`FortressError::InvalidFrameStructured`] with
    /// [`InvalidFrameReason::MissingState`] if no state is saved in this cell.
    ///
    /// # Examples
    ///
    /// Basic usage with request handling:
    ///
    /// ```
    /// use fortress_rollback::{Frame, GameStateCell, FortressError, InvalidFrameReason};
    ///
    /// #[derive(Clone, PartialEq, Debug, Default)]
    /// struct GameState { score: u32 }
    ///
    /// let cell = GameStateCell::<GameState>::default();
    /// let frame = Frame::new(10);
    ///
    /// // Before saving, load_or_err returns an error
    /// let result = cell.load_or_err(frame);
    /// assert!(matches!(
    ///     result,
    ///     Err(FortressError::InvalidFrameStructured {
    ///         frame: f,
    ///         reason: InvalidFrameReason::MissingState
    ///     }) if f == frame
    /// ));
    ///
    /// // After saving, load_or_err succeeds
    /// cell.save(frame, Some(GameState { score: 100 }), None);
    /// let loaded = cell.load_or_err(frame)?;
    /// assert_eq!(loaded.score, 100);
    /// # Ok::<(), FortressError>(())
    /// ```
    ///
    /// Typical usage in request handling loop:
    ///
    /// ```ignore
    /// FortressRequest::LoadGameState { cell, frame } => {
    ///     *game_state = cell.load_or_err(frame)?;
    /// }
    /// ```
    ///
    /// [`load()`]: Self::load
    /// [`FortressRequest::LoadGameState`]: crate::FortressRequest::LoadGameState
    /// [`FortressError::InvalidFrameStructured`]: crate::FortressError::InvalidFrameStructured
    /// [`InvalidFrameReason::MissingState`]: crate::InvalidFrameReason::MissingState
    pub fn load_or_err(&self, frame: Frame) -> Result<T, crate::FortressError> {
        self.load()
            .ok_or(crate::FortressError::InvalidFrameStructured {
                frame,
                reason: crate::InvalidFrameReason::MissingState,
            })
    }
}

/// Creates an empty `GameStateCell` with no saved state.
///
/// The initial state has [`Frame::NULL`] and no data or checksum.
/// This is primarily used internally by the rollback system.
///
/// [`Frame::NULL`]: crate::Frame::NULL
impl<T> Default for GameStateCell<T> {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(GameState::default())))
    }
}

/// Clones this cell, creating a new handle to the **same** underlying state.
///
/// This is a shallow clone using [`Arc::clone`] — both the original and the clone
/// will see the same saved game state. Modifications via one cell (e.g., calling
/// [`save()`](Self::save)) are immediately visible through the other.
///
/// This is the intended behavior: the rollback system may hold multiple references
/// to the same cell across different frames.
impl<T> Clone for GameStateCell<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
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
impl<T> Deref for GameStateAccessor<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(loom)]
impl<T> Deref for GameStateAccessor<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // This should never be called under loom as data() returns None
        unreachable!("GameStateAccessor::deref called under loom - this should not happen")
    }
}

#[cfg(not(loom))]
impl<T> GameStateAccessor<'_, T> {
    /// Get mutable access to the `T` that the user previously saved into a [`GameStateCell`].
    ///
    /// You probably do not need this! It's safer to use [`Deref::deref()`] instead;
    /// see [`GameStateCell::data()`] for a usage example.
    ///
    /// # Safety Invariant
    ///
    /// **Danger**: the underlying `T` must _not_ be modified in any way that affects (or may ever
    /// in future affect) game logic. If this invariant is violated, you will almost certainly get
    /// desyncs.
    ///
    /// # Examples
    ///
    /// ## Safe: Updating non-gameplay metadata
    ///
    /// ```
    /// use fortress_rollback::{Frame, GameStateCell};
    ///
    /// #[derive(Clone)]
    /// struct GameState {
    ///     // Gameplay-affecting state (NEVER modify via as_mut_dangerous)
    ///     player_x: i32,
    ///     player_y: i32,
    ///     health: u32,
    ///
    ///     // Non-gameplay metadata (safe to modify)
    ///     debug_load_count: u32,
    ///     last_accessed_timestamp: u64,
    /// }
    ///
    /// let cell = GameStateCell::<GameState>::default();
    /// let state = GameState {
    ///     player_x: 100,
    ///     player_y: 200,
    ///     health: 100,
    ///     debug_load_count: 0,
    ///     last_accessed_timestamp: 0,
    /// };
    /// cell.save(Frame::new(1), Some(state), None);
    ///
    /// // SAFE: Updating debug/telemetry counters that don't affect gameplay
    /// if let Some(mut accessor) = cell.data() {
    ///     let state = accessor.as_mut_dangerous();
    ///     state.debug_load_count += 1;
    ///     state.last_accessed_timestamp = 1234567890;
    ///     // player_x, player_y, health remain unchanged
    /// };
    /// ```
    ///
    /// ## UNSAFE: Modifying gameplay state (DON'T DO THIS)
    ///
    /// ```ignore
    /// // ❌ WRONG: This WILL cause desyncs!
    /// if let Some(mut accessor) = cell.data() {
    ///     let state = accessor.as_mut_dangerous();
    ///     state.player_x += 10;  // NEVER modify gameplay state!
    ///     state.health = 50;      // This breaks determinism!
    /// }
    /// ```
    ///
    /// The correct approach for gameplay changes is to modify your current game state
    /// during `AdvanceFrame`, not the saved states in cells.
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

#[cfg(not(loom))]
impl<T: std::fmt::Debug> std::fmt::Debug for GameStateAccessor<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("GameStateAccessor").field(&*self.0).finish()
    }
}

#[cfg(loom)]
impl<T: std::fmt::Debug> std::fmt::Debug for GameStateAccessor<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("GameStateAccessor")
            .field(&"<unavailable under loom>")
            .finish()
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
    use crate::error::{FortressError, InvalidFrameReason};

    // ==========================================
    // GameStateCell Basic Tests
    // ==========================================

    #[test]
    fn game_state_cell_default_has_null_frame() {
        let cell = GameStateCell::<u8>::default();
        assert!(cell.frame().is_null());
    }

    #[test]
    fn game_state_cell_default_has_no_data() {
        let cell = GameStateCell::<String>::default();
        assert!(cell.load().is_none());
    }

    #[test]
    fn game_state_cell_default_has_no_checksum() {
        let cell = GameStateCell::<u32>::default();
        assert!(cell.checksum().is_none());
    }

    #[test]
    fn game_state_cell_save_and_load() {
        let cell = GameStateCell::<String>::default();
        let frame = Frame::new(42);
        let data = "test_data".to_string();

        let saved = cell.save(frame, Some(data.clone()), None);
        assert!(saved);

        let loaded = cell.load();
        assert_eq!(loaded, Some(data));
    }

    #[test]
    fn game_state_cell_save_updates_frame() {
        let cell = GameStateCell::<u8>::default();
        let frame = Frame::new(100);

        cell.save(frame, Some(42), None);

        assert_eq!(cell.frame(), frame);
    }

    #[test]
    fn game_state_cell_save_updates_checksum() {
        let cell = GameStateCell::<u8>::default();
        let frame = Frame::new(1);
        let checksum = Some(0xDEADBEEF_u128);

        cell.save(frame, Some(1), checksum);

        assert_eq!(cell.checksum(), checksum);
    }

    #[test]
    fn game_state_cell_save_with_null_frame_returns_false() {
        let cell = GameStateCell::<u8>::default();
        let result = cell.save(Frame::NULL, Some(42), None);
        assert!(!result);
    }

    #[test]
    fn game_state_cell_save_with_null_frame_does_not_modify_state() {
        let cell = GameStateCell::<u8>::default();
        let original_frame = Frame::new(10);
        cell.save(original_frame, Some(1), Some(100));

        // Try saving with null frame - should fail
        let result = cell.save(Frame::NULL, Some(99), Some(999));
        assert!(!result);

        // Original state should be preserved
        assert_eq!(cell.frame(), original_frame);
        assert_eq!(cell.load(), Some(1));
        assert_eq!(cell.checksum(), Some(100));
    }

    #[test]
    fn game_state_cell_save_none_data() {
        let cell = GameStateCell::<u8>::default();
        let frame = Frame::new(5);

        let saved = cell.save(frame, None, None);
        assert!(saved);

        assert_eq!(cell.frame(), frame);
        assert!(cell.load().is_none());
    }

    #[test]
    fn game_state_cell_overwrite() {
        let cell = GameStateCell::<String>::default();

        // First save
        cell.save(Frame::new(1), Some("first".to_string()), Some(1));
        assert_eq!(cell.load(), Some("first".to_string()));

        // Overwrite with new data
        cell.save(Frame::new(2), Some("second".to_string()), Some(2));
        assert_eq!(cell.load(), Some("second".to_string()));
        assert_eq!(cell.frame(), Frame::new(2));
        assert_eq!(cell.checksum(), Some(2));
    }

    // ==========================================
    // GameStateCell Clone Tests
    // ==========================================

    #[test]
    fn game_state_cell_clone_shares_underlying_data() {
        let cell1 = GameStateCell::<u32>::default();
        cell1.save(Frame::new(10), Some(42), None);

        let cell2 = cell1.clone();

        // Both cells should see the same data
        assert_eq!(cell1.load(), Some(42));
        assert_eq!(cell2.load(), Some(42));
    }

    #[test]
    fn game_state_cell_clone_modifications_visible() {
        let cell1 = GameStateCell::<u32>::default();
        let cell2 = cell1.clone();

        // Modify through cell1
        cell1.save(Frame::new(1), Some(100), Some(0xABC));

        // Should be visible through cell2
        assert_eq!(cell2.frame(), Frame::new(1));
        assert_eq!(cell2.load(), Some(100));
        assert_eq!(cell2.checksum(), Some(0xABC));
    }

    // ==========================================
    // GameStateCell Debug Tests
    // ==========================================

    #[test]
    fn game_state_cell_debug_format() {
        let cell = GameStateCell::<u8>::default();
        cell.save(Frame::new(42), Some(1), Some(0x123));

        let debug_str = format!("{:?}", cell);
        assert!(debug_str.contains("GameStateCell"));
        assert!(debug_str.contains("frame"));
        assert!(debug_str.contains("checksum"));
    }

    #[test]
    fn game_state_cell_debug_with_null_frame() {
        let cell = GameStateCell::<u8>::default();
        let debug_str = format!("{:?}", cell);
        assert!(debug_str.contains("GameStateCell"));
    }

    // ==========================================
    // GameStateCell Data Access Tests
    // ==========================================

    #[test]
    fn game_state_cell_data_returns_accessor() {
        let cell = GameStateCell::<String>::default();
        cell.save(Frame::new(1), Some("test".to_string()), None);

        let accessor = cell.data();
        assert!(accessor.is_some());

        let accessor = accessor.unwrap();
        assert_eq!(*accessor, "test");
    }

    #[test]
    fn game_state_cell_data_returns_none_when_empty() {
        let cell = GameStateCell::<String>::default();
        // Cell has no data saved

        let accessor = cell.data();
        assert!(accessor.is_none());
    }

    #[test]
    fn game_state_cell_data_returns_none_when_none_saved() {
        let cell = GameStateCell::<String>::default();
        cell.save(Frame::new(1), None, None);

        let accessor = cell.data();
        assert!(accessor.is_none());
    }

    #[test]
    fn game_state_accessor_deref() {
        let cell = GameStateCell::<Vec<i32>>::default();
        cell.save(Frame::new(1), Some(vec![1, 2, 3]), None);

        let accessor = cell.data().unwrap();
        // Deref to access the underlying Vec
        assert_eq!(accessor.len(), 3);
        assert_eq!(accessor[0], 1);
        assert_eq!(accessor[1], 2);
        assert_eq!(accessor[2], 3);
    }

    #[test]
    fn game_state_accessor_as_mut_dangerous() {
        let cell = GameStateCell::<Vec<i32>>::default();
        cell.save(Frame::new(1), Some(vec![1, 2, 3]), None);

        {
            let mut accessor = cell.data().unwrap();
            // Use dangerous mutable access
            let data = accessor.as_mut_dangerous();
            data.push(4);
        }

        // Verify mutation persisted
        let loaded = cell.load().unwrap();
        assert_eq!(loaded, vec![1, 2, 3, 4]);
    }

    // ==========================================
    // GameStateCell Edge Cases
    // ==========================================

    #[test]
    fn game_state_cell_frame_zero() {
        let cell = GameStateCell::<u8>::default();
        let frame = Frame::new(0);

        let saved = cell.save(frame, Some(42), None);
        assert!(saved);
        assert_eq!(cell.frame(), frame);
    }

    #[test]
    fn game_state_cell_large_frame() {
        let cell = GameStateCell::<u8>::default();
        let frame = Frame::new(i32::MAX);

        let saved = cell.save(frame, Some(1), None);
        assert!(saved);
        assert_eq!(cell.frame(), frame);
    }

    #[test]
    fn game_state_cell_large_checksum() {
        let cell = GameStateCell::<u8>::default();
        let checksum = Some(u128::MAX);

        cell.save(Frame::new(1), Some(1), checksum);
        assert_eq!(cell.checksum(), checksum);
    }

    #[test]
    fn game_state_cell_zero_checksum() {
        let cell = GameStateCell::<u8>::default();
        let checksum = Some(0_u128);

        cell.save(Frame::new(1), Some(1), checksum);
        assert_eq!(cell.checksum(), checksum);
    }

    #[test]
    fn game_state_cell_complex_type() {
        #[derive(Clone, Debug, PartialEq)]
        struct ComplexState {
            position: (f64, f64),
            velocity: (f64, f64),
            health: u32,
            name: String,
        }

        let cell = GameStateCell::<ComplexState>::default();
        let state = ComplexState {
            position: (10.5, 20.5),
            velocity: (-1.0, 0.5),
            health: 100,
            name: "Player1".to_string(),
        };

        cell.save(Frame::new(1), Some(state.clone()), Some(0xABC123));

        let loaded = cell.load().unwrap();
        assert_eq!(loaded, state);
    }

    #[test]
    fn game_state_cell_multiple_clones_all_share_state() {
        let cell1 = GameStateCell::<u32>::default();
        let cell2 = cell1.clone();
        let cell3 = cell1.clone();
        let cell4 = cell2.clone();

        // Save through cell3
        cell3.save(Frame::new(99), Some(12345), Some(0xFFFF));

        // All should see the same state
        assert_eq!(cell1.load(), Some(12345));
        assert_eq!(cell2.load(), Some(12345));
        assert_eq!(cell3.load(), Some(12345));
        assert_eq!(cell4.load(), Some(12345));

        assert_eq!(cell1.frame(), Frame::new(99));
        assert_eq!(cell4.checksum(), Some(0xFFFF));
    }

    #[test]
    fn game_state_cell_repeated_saves_same_frame() {
        let cell = GameStateCell::<u32>::default();
        let frame = Frame::new(10);

        // Multiple saves to same frame
        cell.save(frame, Some(1), None);
        cell.save(frame, Some(2), None);
        cell.save(frame, Some(3), None);

        // Last save wins
        assert_eq!(cell.load(), Some(3));
        assert_eq!(cell.frame(), frame);
    }

    // ==========================================
    // load_or_err() Tests
    // ==========================================

    #[test]
    fn load_or_err_returns_error_when_empty() {
        let cell = GameStateCell::<u32>::default();
        let frame = Frame::new(10);

        let result = cell.load_or_err(frame);
        assert!(matches!(
            result,
            Err(FortressError::InvalidFrameStructured {
                frame: f,
                reason: InvalidFrameReason::MissingState
            }) if f == frame
        ));
    }

    #[test]
    fn load_or_err_succeeds_after_save() {
        let cell = GameStateCell::<u32>::default();
        let frame = Frame::new(10);
        cell.save(frame, Some(42), None);

        let result = cell.load_or_err(frame);
        assert_eq!(result.unwrap(), 42);
    }
}
