//! Replay playback session for deterministic match replay.
//!
//! [`ReplaySession`] plays back a recorded [`Replay`] frame by frame,
//! returning the confirmed inputs from each frame without requiring
//! network communication, save/load, or local input.
//!
//! # Example
//!
//! ```
//! use fortress_rollback::replay::{Replay, ReplayMetadata};
//! use fortress_rollback::sessions::replay_session::ReplaySession;
//! use fortress_rollback::Session;
//! use serde::{Deserialize, Serialize};
//! use std::net::SocketAddr;
//!
//! #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
//! struct MyInput { buttons: u8 }
//!
//! #[derive(Debug)]
//! struct ReplayConfig;
//! impl fortress_rollback::Config for ReplayConfig {
//!     type Input = MyInput;
//!     type State = Vec<u8>;
//!     type Address = SocketAddr;
//! }
//!
//! let replay = Replay {
//!     num_players: 2,
//!     frames: vec![
//!         vec![MyInput { buttons: 1 }, MyInput { buttons: 2 }],
//!         vec![MyInput { buttons: 3 }, MyInput { buttons: 4 }],
//!     ],
//!     checksums: vec![None; 2],
//!     metadata: ReplayMetadata {
//!         library_version: env!("CARGO_PKG_VERSION").to_string(),
//!         num_players: 2,
//!         total_frames: 2,
//!         skipped_frames: 0,
//!     },
//! };
//!
//! let mut session = ReplaySession::<ReplayConfig>::new(replay)?;
//! assert!(!session.is_complete());
//!
//! // Advance through each frame
//! let requests = session.advance_frame()?;
//! assert_eq!(requests.len(), 1); // One AdvanceFrame request
//! assert!(!session.is_complete());
//!
//! let requests = session.advance_frame()?;
//! assert_eq!(requests.len(), 1);
//! assert!(session.is_complete());
//! # Ok::<(), fortress_rollback::FortressError>(())
//! ```
//!
//! [`Replay`]: crate::replay::Replay
//! [`ReplaySession`]: crate::sessions::replay_session::ReplaySession

use std::collections::VecDeque;
use std::fmt;

use crate::replay::Replay;
use crate::sessions::session_trait::Session;
use crate::sync_layer::GameStateCell;
use crate::{
    Config, EventDrain, FortressError, FortressEvent, FortressRequest, FortressResult, Frame,
    InputStatus, InputVec, InvalidRequestKind, PlayerHandle, RequestVec, SessionState,
};

/// A session that plays back a recorded [`Replay`] deterministically.
///
/// This session type reads pre-recorded inputs from a [`Replay`] and
/// returns them as [`FortressRequest::AdvanceFrame`] requests, one frame
/// at a time. It does not require network communication, save/load
/// operations, or local input.
///
/// # Not Supported
///
/// Since replay sessions play back pre-recorded data:
/// - [`add_local_input`](Session::add_local_input) returns a "not supported" error
/// - [`local_player_handle_required`](Session::local_player_handle_required) returns a "not supported" error
/// - [`poll_remote_clients`](Session::poll_remote_clients) is a no-op
///
/// # Example
///
/// ```
/// use fortress_rollback::replay::{Replay, ReplayMetadata};
/// use fortress_rollback::sessions::replay_session::ReplaySession;
/// use fortress_rollback::{Config, Session, Frame};
/// use serde::{Deserialize, Serialize};
/// use std::net::SocketAddr;
///
/// #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
/// struct Input(u8);
///
/// #[derive(Debug)]
/// struct Cfg;
/// impl Config for Cfg {
///     type Input = Input;
///     type State = ();
///     type Address = SocketAddr;
/// }
///
/// let replay = Replay {
///     num_players: 1,
///     frames: vec![vec![Input(42)]],
///     checksums: vec![None],
///     metadata: ReplayMetadata {
///         library_version: String::new(),
///         num_players: 1,
///         total_frames: 1,
///         skipped_frames: 0,
///     },
/// };
///
/// let mut session = ReplaySession::<Cfg>::new(replay)?;
/// assert_eq!(session.current_frame(), Frame::NULL);
/// assert_eq!(session.total_frames(), 1);
///
/// let requests = session.advance_frame()?;
/// assert_eq!(session.current_frame(), Frame::new(0));
/// assert!(session.is_complete());
/// # Ok::<(), fortress_rollback::FortressError>(())
/// ```
///
/// [`Replay`]: crate::replay::Replay
pub struct ReplaySession<T>
where
    T: Config,
{
    /// The replay data being played back.
    replay: Replay<T::Input>,
    /// The current frame index. Starts at NULL (-1) and increments on advance.
    current_frame: Frame,
    /// Event queue for desync detection and other events.
    event_queue: VecDeque<FortressEvent<T>>,
    /// Whether checksum validation is enabled.
    validate_checksums: bool,
    /// Pending validation cell from the previous frame's `SaveGameState` request.
    /// Stored as `(frame, cell)` so we can compare the checksum after the user
    /// has filled the cell.
    pending_validation: Option<(Frame, GameStateCell<T::State>)>,
}

impl<T: Config> ReplaySession<T> {
    /// Creates a new [`ReplaySession`] from a recorded [`Replay`].
    ///
    /// The session starts at [`Frame::NULL`] and will advance through each
    /// recorded frame when [`advance_frame`](Session::advance_frame) is called.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::replay::{Replay, ReplayMetadata};
    /// use fortress_rollback::sessions::replay_session::ReplaySession;
    /// use fortress_rollback::{Config, Frame};
    /// use serde::{Deserialize, Serialize};
    /// use std::net::SocketAddr;
    ///
    /// #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
    /// struct Input(u8);
    ///
    /// #[derive(Debug)]
    /// struct Cfg;
    /// impl Config for Cfg {
    ///     type Input = Input;
    ///     type State = ();
    ///     type Address = SocketAddr;
    /// }
    ///
    /// let replay = Replay {
    ///     num_players: 1,
    ///     frames: vec![vec![Input(0)]],
    ///     checksums: vec![None],
    ///     metadata: ReplayMetadata {
    ///         library_version: String::new(),
    ///         num_players: 1,
    ///         total_frames: 1,
    ///         skipped_frames: 0,
    ///     },
    /// };
    ///
    /// let session = ReplaySession::<Cfg>::new(replay)?;
    /// assert_eq!(session.current_frame(), Frame::NULL);
    /// # Ok::<(), fortress_rollback::FortressError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the replay fails internal consistency validation
    /// (see [`Replay::validate`]).
    pub fn new(replay: Replay<T::Input>) -> FortressResult<Self> {
        replay.validate()?;
        Ok(Self {
            replay,
            current_frame: Frame::NULL,
            event_queue: VecDeque::new(),
            validate_checksums: false,
            pending_validation: None,
        })
    }

    /// Creates a new [`ReplaySession`] with checksum validation enabled.
    ///
    /// When validation is enabled, the session emits [`FortressRequest::SaveGameState`]
    /// requests before each [`FortressRequest::AdvanceFrame`], allowing the application
    /// to compute checksums. These checksums are compared against the checksums stored
    /// in the replay to detect non-determinism.
    ///
    /// If a mismatch is detected, a [`FortressEvent::ReplayDesync`] event is emitted
    /// with the frame number and both checksums.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::replay::{Replay, ReplayMetadata};
    /// use fortress_rollback::sessions::replay_session::ReplaySession;
    /// use fortress_rollback::{Config, Frame, Session, FortressRequest};
    /// use serde::{Deserialize, Serialize};
    /// use std::net::SocketAddr;
    ///
    /// #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
    /// struct Input(u8);
    ///
    /// #[derive(Debug)]
    /// struct Cfg;
    /// impl Config for Cfg {
    ///     type Input = Input;
    ///     type State = ();
    ///     type Address = SocketAddr;
    /// }
    ///
    /// let replay = Replay {
    ///     num_players: 1,
    ///     frames: vec![vec![Input(0)]],
    ///     checksums: vec![Some(0xABCD)],
    ///     metadata: ReplayMetadata {
    ///         library_version: String::new(),
    ///         num_players: 1,
    ///         total_frames: 1,
    ///         skipped_frames: 0,
    ///     },
    /// };
    ///
    /// let mut session = ReplaySession::<Cfg>::new_with_validation(replay)?;
    /// let requests = session.advance_frame()?;
    /// // With validation, SaveGameState is emitted before AdvanceFrame
    /// assert_eq!(requests.len(), 2);
    /// assert!(matches!(requests[0], FortressRequest::SaveGameState { .. }));
    /// assert!(matches!(requests[1], FortressRequest::AdvanceFrame { .. }));
    /// # Ok::<(), fortress_rollback::FortressError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the replay fails internal consistency validation
    /// (see [`Replay::validate`]).
    pub fn new_with_validation(replay: Replay<T::Input>) -> FortressResult<Self> {
        replay.validate()?;
        Ok(Self {
            replay,
            current_frame: Frame::NULL,
            event_queue: VecDeque::new(),
            validate_checksums: true,
            pending_validation: None,
        })
    }

    /// Checks and resolves any pending validation from the previous frame.
    ///
    /// If a [`FortressRequest::SaveGameState`] was issued on the previous frame,
    /// this compares the checksum stored in the cell by the application against
    /// the replay's recorded checksum. On mismatch, a [`FortressEvent::ReplayDesync`]
    /// event is enqueued.
    fn check_pending_validation(&mut self) {
        if let Some((prev_frame, cell)) = self.pending_validation.take() {
            let prev_index = prev_frame.try_as_usize().ok();
            let replay_checksum = prev_index
                .and_then(|idx| self.replay.checksums.get(idx).copied())
                .flatten();
            let actual_checksum = cell.checksum();

            if let (Some(expected), Some(actual)) = (replay_checksum, actual_checksum) {
                if expected != actual {
                    self.event_queue.push_back(FortressEvent::ReplayDesync {
                        frame: prev_frame,
                        expected_checksum: expected,
                        actual_checksum: actual,
                    });
                }
            }
        }
    }

    /// Returns `true` if checksum validation mode is enabled.
    ///
    /// When validating, [`advance_frame`](Session::advance_frame) emits
    /// [`FortressRequest::SaveGameState`] requests so the application can
    /// provide checksums for comparison against the replay recording.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::replay::{Replay, ReplayMetadata};
    /// # use fortress_rollback::sessions::replay_session::ReplaySession;
    /// # use fortress_rollback::Config;
    /// # use serde::{Deserialize, Serialize};
    /// # use std::net::SocketAddr;
    /// # #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
    /// # struct Input(u8);
    /// # #[derive(Debug)]
    /// # struct Cfg;
    /// # impl Config for Cfg {
    /// #     type Input = Input;
    /// #     type State = ();
    /// #     type Address = SocketAddr;
    /// # }
    /// let replay = Replay {
    ///     num_players: 1,
    ///     frames: vec![vec![Input(0)]],
    ///     checksums: vec![Some(0x1234)],
    ///     metadata: ReplayMetadata {
    ///         library_version: String::new(),
    ///         num_players: 1,
    ///         total_frames: 1,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// let normal = ReplaySession::<Cfg>::new(replay.clone())?;
    /// assert!(!normal.is_validating());
    ///
    /// let validating = ReplaySession::<Cfg>::new_with_validation(replay)?;
    /// assert!(validating.is_validating());
    /// # Ok::<(), fortress_rollback::FortressError>(())
    /// ```
    #[must_use]
    pub fn is_validating(&self) -> bool {
        self.validate_checksums
    }

    /// Returns the current frame of the replay session.
    ///
    /// Starts at [`Frame::NULL`] before the first [`advance_frame`](Session::advance_frame)
    /// call. After the first advance, it will be `Frame::new(0)`.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::replay::{Replay, ReplayMetadata};
    /// # use fortress_rollback::sessions::replay_session::ReplaySession;
    /// # use fortress_rollback::{Config, Frame, Session};
    /// # use serde::{Deserialize, Serialize};
    /// # use std::net::SocketAddr;
    /// # #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
    /// # struct Input(u8);
    /// # #[derive(Debug)]
    /// # struct Cfg;
    /// # impl Config for Cfg {
    /// #     type Input = Input;
    /// #     type State = ();
    /// #     type Address = SocketAddr;
    /// # }
    /// let replay = Replay {
    ///     num_players: 1,
    ///     frames: vec![vec![Input(0)]],
    ///     checksums: vec![None],
    ///     metadata: ReplayMetadata {
    ///         library_version: String::new(),
    ///         num_players: 1,
    ///         total_frames: 1,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// let mut session = ReplaySession::<Cfg>::new(replay)?;
    /// assert_eq!(session.current_frame(), Frame::NULL);
    /// let _ = session.advance_frame()?;
    /// assert_eq!(session.current_frame(), Frame::new(0));
    /// # Ok::<(), fortress_rollback::FortressError>(())
    /// ```
    #[must_use]
    pub fn current_frame(&self) -> Frame {
        self.current_frame
    }

    /// Returns the total number of frames in the replay.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::replay::{Replay, ReplayMetadata};
    /// # use fortress_rollback::sessions::replay_session::ReplaySession;
    /// # use fortress_rollback::Config;
    /// # use serde::{Deserialize, Serialize};
    /// # use std::net::SocketAddr;
    /// # #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
    /// # struct Input(u8);
    /// # #[derive(Debug)]
    /// # struct Cfg;
    /// # impl Config for Cfg {
    /// #     type Input = Input;
    /// #     type State = ();
    /// #     type Address = SocketAddr;
    /// # }
    /// let replay = Replay {
    ///     num_players: 1,
    ///     frames: vec![vec![Input(0)]; 100],
    ///     checksums: vec![None; 100],
    ///     metadata: ReplayMetadata {
    ///         library_version: String::new(),
    ///         num_players: 1,
    ///         total_frames: 100,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// let session = ReplaySession::<Cfg>::new(replay)?;
    /// assert_eq!(session.total_frames(), 100);
    /// # Ok::<(), fortress_rollback::FortressError>(())
    /// ```
    #[must_use]
    pub fn total_frames(&self) -> usize {
        self.replay.total_frames()
    }

    /// Returns `true` if all frames in the replay have been played back.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::replay::{Replay, ReplayMetadata};
    /// # use fortress_rollback::sessions::replay_session::ReplaySession;
    /// # use fortress_rollback::{Config, Session};
    /// # use serde::{Deserialize, Serialize};
    /// # use std::net::SocketAddr;
    /// # #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
    /// # struct Input(u8);
    /// # #[derive(Debug)]
    /// # struct Cfg;
    /// # impl Config for Cfg {
    /// #     type Input = Input;
    /// #     type State = ();
    /// #     type Address = SocketAddr;
    /// # }
    /// let replay = Replay {
    ///     num_players: 1,
    ///     frames: vec![vec![Input(0)]],
    ///     checksums: vec![None],
    ///     metadata: ReplayMetadata {
    ///         library_version: String::new(),
    ///         num_players: 1,
    ///         total_frames: 1,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// let mut session = ReplaySession::<Cfg>::new(replay)?;
    /// assert!(!session.is_complete());
    /// let _ = session.advance_frame()?;
    /// assert!(session.is_complete());
    /// # Ok::<(), fortress_rollback::FortressError>(())
    /// ```
    #[must_use]
    pub fn is_complete(&self) -> bool {
        // current_frame starts at NULL (-1). After advancing through all frames,
        // current_frame will be total_frames - 1 (0-indexed).
        let total = self.replay.total_frames();
        if total == 0 {
            return true;
        }
        // current_frame is the last frame we advanced to (0-indexed).
        // If it equals total_frames - 1, we have played all frames.
        self.current_frame.as_i32() >= 0
            && self
                .current_frame
                .try_as_usize()
                .is_ok_and(|f| f + 1 >= total)
    }

    /// Returns a reference to the underlying [`Replay`].
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::replay::{Replay, ReplayMetadata};
    /// # use fortress_rollback::sessions::replay_session::ReplaySession;
    /// # use fortress_rollback::Config;
    /// # use serde::{Deserialize, Serialize};
    /// # use std::net::SocketAddr;
    /// # #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
    /// # struct Input(u8);
    /// # #[derive(Debug)]
    /// # struct Cfg;
    /// # impl Config for Cfg {
    /// #     type Input = Input;
    /// #     type State = ();
    /// #     type Address = SocketAddr;
    /// # }
    /// let replay = Replay {
    ///     num_players: 2,
    ///     frames: vec![],
    ///     checksums: vec![],
    ///     metadata: ReplayMetadata {
    ///         library_version: String::new(),
    ///         num_players: 2,
    ///         total_frames: 0,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// let session = ReplaySession::<Cfg>::new(replay)?;
    /// assert_eq!(session.replay().num_players, 2);
    /// # Ok::<(), fortress_rollback::FortressError>(())
    /// ```
    #[must_use]
    pub fn replay(&self) -> &Replay<T::Input> {
        &self.replay
    }

    /// Returns the local player handle.
    ///
    /// Replay sessions do not have a local player, so this always returns a
    /// "not supported" error.
    ///
    /// # Errors
    ///
    /// Always returns [`InvalidRequestKind::NotSupported`].
    #[must_use = "returns the local player handle which should be used"]
    pub fn local_player_handle_required(&self) -> FortressResult<PlayerHandle> {
        Err(InvalidRequestKind::NotSupported {
            operation: "local_player_handle_required",
        }
        .into())
    }

    /// Adds local input for the given player.
    ///
    /// Replay sessions play back pre-recorded data, so this always returns a
    /// "not supported" error.
    ///
    /// # Errors
    ///
    /// Always returns [`InvalidRequestKind::NotSupported`].
    #[must_use = "error should be handled"]
    pub fn add_local_input(
        &mut self,
        _player_handle: PlayerHandle,
        _input: T::Input,
    ) -> FortressResult<()> {
        Err(InvalidRequestKind::NotSupported {
            operation: "add_local_input",
        }
        .into())
    }

    /// Returns all events that happened since last queried for events.
    #[must_use = "events should be handled to react to session state changes"]
    pub fn events(&mut self) -> EventDrain<'_, T> {
        EventDrain::from_drain(self.event_queue.drain(..))
    }

    /// Returns the current session state.
    ///
    /// Always returns [`SessionState::Running`] since replay sessions
    /// do not require synchronization.
    #[must_use]
    pub fn current_state(&self) -> SessionState {
        SessionState::Running
    }

    /// Advances the replay by one frame, returning the recorded inputs.
    ///
    /// Returns a single [`FortressRequest::AdvanceFrame`] containing the
    /// confirmed inputs for the next frame. Returns an error if there are
    /// no more frames to play back.
    ///
    /// # Errors
    ///
    /// Returns [`FortressError::InvalidFrameStructured`] if the replay
    /// has been fully played back.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::replay::{Replay, ReplayMetadata};
    /// # use fortress_rollback::sessions::replay_session::ReplaySession;
    /// # use fortress_rollback::{Config, Session, FortressRequest};
    /// # use serde::{Deserialize, Serialize};
    /// # use std::net::SocketAddr;
    /// # #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
    /// # struct Input(u8);
    /// # #[derive(Debug)]
    /// # struct Cfg;
    /// # impl Config for Cfg {
    /// #     type Input = Input;
    /// #     type State = ();
    /// #     type Address = SocketAddr;
    /// # }
    /// let replay = Replay {
    ///     num_players: 1,
    ///     frames: vec![vec![Input(42)]],
    ///     checksums: vec![None],
    ///     metadata: ReplayMetadata {
    ///         library_version: String::new(),
    ///         num_players: 1,
    ///         total_frames: 1,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// let mut session = ReplaySession::<Cfg>::new(replay)?;
    /// let requests = session.advance_frame()?;
    /// assert_eq!(requests.len(), 1);
    /// # Ok::<(), fortress_rollback::FortressError>(())
    /// ```
    #[must_use = "FortressRequests must be processed to advance the game state"]
    pub fn advance_frame(&mut self) -> FortressResult<RequestVec<T>> {
        // Always check pending validation from the previous frame first,
        // even if the replay is exhausted. This ensures the last frame's
        // checksum is validated when the user makes the final advance_frame()
        // call (which will return an error after validation runs).
        self.check_pending_validation();

        let next_frame = self.current_frame.next()?;
        let frame_index = next_frame.try_as_usize()?;

        let frame_inputs =
            self.replay
                .frames
                .get(frame_index)
                .ok_or(FortressError::InvalidFrameStructured {
                    frame: next_frame,
                    reason: crate::InvalidFrameReason::NotConfirmed {
                        confirmed_frame: self.current_frame,
                    },
                })?;

        let mut inputs = InputVec::with_capacity(frame_inputs.len());
        for input in frame_inputs {
            inputs.push((*input, InputStatus::Confirmed));
        }

        self.current_frame = next_frame;

        let mut requests = RequestVec::new();

        if self.validate_checksums {
            let cell = GameStateCell::<T::State>::default();
            requests.push(FortressRequest::SaveGameState {
                cell: cell.clone(),
                frame: next_frame,
            });
            self.pending_validation = Some((next_frame, cell));
        }

        requests.push(FortressRequest::AdvanceFrame { inputs });
        Ok(requests)
    }
}

impl<T: Config> Session<T> for ReplaySession<T> {
    fn advance_frame(&mut self) -> FortressResult<RequestVec<T>> {
        Self::advance_frame(self)
    }

    fn local_player_handle_required(&self) -> FortressResult<PlayerHandle> {
        Self::local_player_handle_required(self)
    }

    fn add_local_input(
        &mut self,
        player_handle: PlayerHandle,
        input: T::Input,
    ) -> FortressResult<()> {
        Self::add_local_input(self, player_handle, input)
    }

    fn events(&mut self) -> EventDrain<'_, T> {
        Self::events(self)
    }

    fn current_state(&self) -> SessionState {
        Self::current_state(self)
    }
}

impl<T: Config> fmt::Debug for ReplaySession<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReplaySession")
            .field("current_frame", &self.current_frame)
            .field("total_frames", &self.replay.total_frames())
            .field("is_complete", &self.is_complete())
            .field("num_players", &self.replay.num_players)
            .field("validate_checksums", &self.validate_checksums)
            .finish_non_exhaustive()
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
    use crate::replay::ReplayMetadata;
    use std::net::SocketAddr;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestConfig;

    impl Config for TestConfig {
        type Input = u8;
        type State = Vec<u8>;
        type Address = SocketAddr;
    }

    fn make_replay(num_frames: usize, num_players: usize) -> Replay<u8> {
        let frames: Vec<Vec<u8>> = (0..num_frames)
            .map(|f| {
                (0..num_players)
                    .map(|p| (f * num_players + p) as u8)
                    .collect()
            })
            .collect();
        Replay {
            num_players,
            frames,
            checksums: vec![None; num_frames],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players,
                total_frames: num_frames,
                skipped_frames: 0,
            },
        }
    }

    #[test]
    fn new_session_starts_at_null_frame() {
        let session = ReplaySession::<TestConfig>::new(make_replay(5, 2)).unwrap();
        assert_eq!(session.current_frame(), Frame::NULL);
        assert!(!session.is_complete());
    }

    #[test]
    fn advance_frame_returns_correct_inputs() {
        let mut session = ReplaySession::<TestConfig>::new(make_replay(3, 2)).unwrap();

        // Frame 0
        let requests = session.advance_frame().unwrap();
        assert_eq!(requests.len(), 1);
        match &requests[0] {
            FortressRequest::AdvanceFrame { inputs } => {
                assert_eq!(inputs.len(), 2);
                assert_eq!(inputs[0], (0, InputStatus::Confirmed));
                assert_eq!(inputs[1], (1, InputStatus::Confirmed));
            },
            _ => panic!("Expected AdvanceFrame request"),
        }
        assert_eq!(session.current_frame(), Frame::new(0));

        // Frame 1
        let requests = session.advance_frame().unwrap();
        match &requests[0] {
            FortressRequest::AdvanceFrame { inputs } => {
                assert_eq!(inputs[0], (2, InputStatus::Confirmed));
                assert_eq!(inputs[1], (3, InputStatus::Confirmed));
            },
            _ => panic!("Expected AdvanceFrame request"),
        }
        assert_eq!(session.current_frame(), Frame::new(1));
    }

    #[test]
    fn advance_past_end_returns_error() {
        let mut session = ReplaySession::<TestConfig>::new(make_replay(1, 1)).unwrap();
        session.advance_frame().unwrap();
        assert!(session.is_complete());

        let result = session.advance_frame();
        assert!(result.is_err());
    }

    #[test]
    fn is_complete_empty_replay() {
        let session = ReplaySession::<TestConfig>::new(make_replay(0, 1)).unwrap();
        assert!(session.is_complete());
    }

    #[test]
    fn is_complete_after_all_frames() {
        let mut session = ReplaySession::<TestConfig>::new(make_replay(3, 1)).unwrap();
        for _ in 0..3 {
            assert!(!session.is_complete());
            session.advance_frame().unwrap();
        }
        assert!(session.is_complete());
    }

    #[test]
    fn total_frames_matches_replay() {
        let session = ReplaySession::<TestConfig>::new(make_replay(42, 2)).unwrap();
        assert_eq!(session.total_frames(), 42);
    }

    #[test]
    fn local_player_handle_required_not_supported() {
        let session = ReplaySession::<TestConfig>::new(make_replay(1, 1)).unwrap();
        let result = session.local_player_handle_required();
        assert!(result.is_err());
    }

    #[test]
    fn add_local_input_not_supported() {
        let mut session = ReplaySession::<TestConfig>::new(make_replay(1, 1)).unwrap();
        let result = session.add_local_input(PlayerHandle::new(0), 42);
        assert!(result.is_err());
    }

    #[test]
    fn events_returns_empty_drain() {
        let mut session = ReplaySession::<TestConfig>::new(make_replay(1, 1)).unwrap();
        assert!(session.events().next().is_none());
    }

    #[test]
    fn current_state_always_running() {
        let session = ReplaySession::<TestConfig>::new(make_replay(1, 1)).unwrap();
        assert_eq!(session.current_state(), SessionState::Running);
    }

    #[test]
    fn replay_accessor() {
        let session = ReplaySession::<TestConfig>::new(make_replay(5, 3)).unwrap();
        assert_eq!(session.replay().num_players, 3);
        assert_eq!(session.replay().total_frames(), 5);
    }

    #[test]
    fn debug_format() {
        let session = ReplaySession::<TestConfig>::new(make_replay(10, 2)).unwrap();
        let debug_str = format!("{:?}", session);
        assert!(debug_str.contains("ReplaySession"));
        assert!(debug_str.contains("total_frames"));
    }

    #[test]
    fn session_trait_advance_frame() {
        let mut session: Box<dyn Session<TestConfig>> =
            Box::new(ReplaySession::<TestConfig>::new(make_replay(2, 1)).unwrap());
        let requests = session.advance_frame().unwrap();
        assert_eq!(requests.len(), 1);
    }

    #[test]
    fn full_playback_single_player() {
        let num_frames = 10;
        let mut session = ReplaySession::<TestConfig>::new(make_replay(num_frames, 1)).unwrap();

        for expected_frame in 0..num_frames {
            let requests = session.advance_frame().unwrap();
            assert_eq!(requests.len(), 1);
            match &requests[0] {
                FortressRequest::AdvanceFrame { inputs } => {
                    assert_eq!(inputs.len(), 1);
                    assert_eq!(inputs[0].0, expected_frame as u8);
                    assert_eq!(inputs[0].1, InputStatus::Confirmed);
                },
                _ => panic!("Expected AdvanceFrame"),
            }
            assert_eq!(session.current_frame(), Frame::new(expected_frame as i32));
        }
        assert!(session.is_complete());
    }

    fn make_replay_with_checksums(
        num_frames: usize,
        num_players: usize,
        checksums: Vec<Option<u128>>,
    ) -> Replay<u8> {
        let frames: Vec<Vec<u8>> = (0..num_frames)
            .map(|f| {
                (0..num_players)
                    .map(|p| (f * num_players + p) as u8)
                    .collect()
            })
            .collect();
        Replay {
            num_players,
            frames,
            checksums,
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players,
                total_frames: num_frames,
                skipped_frames: 0,
            },
        }
    }

    #[test]
    fn validation_mode_emits_save_game_state() {
        let replay = make_replay_with_checksums(3, 1, vec![Some(100), Some(200), Some(300)]);
        let mut session = ReplaySession::<TestConfig>::new_with_validation(replay).unwrap();

        let requests = session.advance_frame().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(
            matches!(&requests[0], FortressRequest::SaveGameState { frame, .. } if *frame == Frame::new(0))
        );
        assert!(matches!(&requests[1], FortressRequest::AdvanceFrame { .. }));
    }

    #[test]
    fn validation_mode_detects_checksum_mismatch() {
        let replay = make_replay_with_checksums(2, 1, vec![Some(0xAAAA), Some(0xBBBB)]);
        let mut session = ReplaySession::<TestConfig>::new_with_validation(replay).unwrap();

        // Frame 0: get SaveGameState, fill with wrong checksum
        let requests = session.advance_frame().unwrap();
        if let FortressRequest::SaveGameState { cell, frame } = &requests[0] {
            cell.save(*frame, Some(vec![1u8]), Some(0xDEAD));
        } else {
            panic!("Expected SaveGameState");
        }

        // Frame 1: triggers validation of frame 0
        let _requests = session.advance_frame().unwrap();

        // Check for desync event
        let events: Vec<_> = session.events().collect();
        assert_eq!(events.len(), 1);
        match &events[0] {
            FortressEvent::ReplayDesync {
                frame,
                expected_checksum,
                actual_checksum,
            } => {
                assert_eq!(*frame, Frame::new(0));
                assert_eq!(*expected_checksum, 0xAAAA);
                assert_eq!(*actual_checksum, 0xDEAD);
            },
            other => panic!("Expected ReplayDesync, got {:?}", other),
        }
    }

    #[test]
    fn validation_mode_no_event_on_matching_checksums() {
        let replay = make_replay_with_checksums(2, 1, vec![Some(0x1234), Some(0x5678)]);
        let mut session = ReplaySession::<TestConfig>::new_with_validation(replay).unwrap();

        // Frame 0: fill with matching checksum
        let requests = session.advance_frame().unwrap();
        if let FortressRequest::SaveGameState { cell, frame } = &requests[0] {
            cell.save(*frame, Some(vec![1u8]), Some(0x1234));
        }

        // Frame 1: triggers validation of frame 0
        let _requests = session.advance_frame().unwrap();

        // No desync events should be emitted
        assert!(session.events().next().is_none());
    }

    #[test]
    fn validation_mode_skips_frames_without_checksums() {
        let replay = make_replay_with_checksums(2, 1, vec![None, None]);
        let mut session = ReplaySession::<TestConfig>::new_with_validation(replay).unwrap();

        // Frame 0: fill with a checksum (but replay has None)
        let requests = session.advance_frame().unwrap();
        if let FortressRequest::SaveGameState { cell, frame } = &requests[0] {
            cell.save(*frame, Some(vec![1u8]), Some(0xBEEF));
        }

        // Frame 1: triggers validation of frame 0, but replay checksum is None
        let _requests = session.advance_frame().unwrap();

        // No desync events -- replay has no checksum to compare against
        assert!(session.events().next().is_none());
    }

    #[test]
    fn non_validation_mode_no_save_requests() {
        let replay = make_replay_with_checksums(3, 1, vec![Some(100), Some(200), Some(300)]);
        let mut session = ReplaySession::<TestConfig>::new(replay).unwrap();

        for _ in 0..3 {
            let requests = session.advance_frame().unwrap();
            assert_eq!(requests.len(), 1);
            assert!(matches!(&requests[0], FortressRequest::AdvanceFrame { .. }));
        }
        assert!(session.events().next().is_none());
    }

    #[test]
    fn validation_mode_skips_when_actual_checksum_is_none() {
        let replay = make_replay_with_checksums(2, 1, vec![Some(0x1234), Some(0x5678)]);
        let mut session = ReplaySession::<TestConfig>::new_with_validation(replay).unwrap();

        // Frame 0: save without providing a checksum
        let requests = session.advance_frame().unwrap();
        if let FortressRequest::SaveGameState { cell, frame } = &requests[0] {
            cell.save(*frame, Some(vec![1u8]), None);
        }

        // Frame 1: triggers validation of frame 0, but actual checksum is None
        let _requests = session.advance_frame().unwrap();

        // No desync events -- actual checksum is None
        assert!(session.events().next().is_none());
    }

    #[test]
    fn single_frame_replay_validation_detects_last_frame_desync() {
        // MAJOR #4: Single-frame replay with validation.
        // The last (and only) frame's checksum must be validated when
        // the user calls advance_frame() again (which returns an error).
        let replay = make_replay_with_checksums(1, 1, vec![Some(0xCAFE)]);
        let mut session = ReplaySession::<TestConfig>::new_with_validation(replay).unwrap();
        assert!(!session.is_complete());

        // Frame 0: get SaveGameState + AdvanceFrame
        let requests = session.advance_frame().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(matches!(
            &requests[0],
            FortressRequest::SaveGameState { .. }
        ));
        assert!(matches!(&requests[1], FortressRequest::AdvanceFrame { .. }));

        // Fill with a mismatched checksum
        if let FortressRequest::SaveGameState { cell, frame } = &requests[0] {
            cell.save(*frame, Some(vec![1u8]), Some(0xBAD));
        }
        assert!(session.is_complete());

        // Next advance_frame() should return error (no more frames),
        // but first it validates the pending checksum from frame 0.
        let result = session.advance_frame();
        assert!(result.is_err());

        // The desync event should be available
        let events: Vec<_> = session.events().collect();
        assert_eq!(events.len(), 1);
        match &events[0] {
            FortressEvent::ReplayDesync {
                frame,
                expected_checksum,
                actual_checksum,
            } => {
                assert_eq!(*frame, Frame::new(0));
                assert_eq!(*expected_checksum, 0xCAFE);
                assert_eq!(*actual_checksum, 0xBAD);
            },
            other => panic!("Expected ReplayDesync, got {:?}", other),
        }
    }

    #[test]
    fn single_frame_replay_validation_matching_checksum() {
        // Verify no desync event when last-frame checksum matches.
        let replay = make_replay_with_checksums(1, 1, vec![Some(0xCAFE)]);
        let mut session = ReplaySession::<TestConfig>::new_with_validation(replay).unwrap();

        let requests = session.advance_frame().unwrap();
        if let FortressRequest::SaveGameState { cell, frame } = &requests[0] {
            cell.save(*frame, Some(vec![1u8]), Some(0xCAFE));
        }

        // Next call returns error but no desync event
        let result = session.advance_frame();
        assert!(result.is_err());
        assert!(session.events().next().is_none());
    }

    #[test]
    fn empty_replay_with_validation() {
        // MAJOR #5: 0-frame replay with validation.
        let replay = make_replay_with_checksums(0, 1, vec![]);
        let session = ReplaySession::<TestConfig>::new_with_validation(replay).unwrap();
        assert!(session.is_complete());
        assert!(session.is_validating());
        assert_eq!(session.total_frames(), 0);
        assert_eq!(session.current_frame(), Frame::NULL);
    }

    #[test]
    fn empty_replay_with_validation_advance_returns_error() {
        let replay = make_replay_with_checksums(0, 1, vec![]);
        let mut session = ReplaySession::<TestConfig>::new_with_validation(replay).unwrap();

        // Advancing past the empty replay should error
        let result = session.advance_frame();
        assert!(result.is_err());
        // No events since there were no frames to validate
        assert!(session.events().next().is_none());
    }

    #[test]
    fn is_validating_returns_correct_value() {
        let replay = make_replay(3, 1);
        let normal = ReplaySession::<TestConfig>::new(replay).unwrap();
        assert!(!normal.is_validating());

        let replay = make_replay(3, 1);
        let validating = ReplaySession::<TestConfig>::new_with_validation(replay).unwrap();
        assert!(validating.is_validating());
    }
}
