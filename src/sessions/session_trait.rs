use crate::{
    Config, EventDrain, FortressResult, InvalidRequestKind, PlayerHandle, RequestVec, SessionState,
};

/// A unified interface for all Fortress Rollback session types.
///
/// The `Session` trait provides a common API surface that all session types
/// ([`P2PSession`], [`SpectatorSession`], [`SyncTestSession`]) implement.
/// This enables writing generic code that works with any session type,
/// such as a game loop that doesn't care whether it's running a local
/// sync test or a networked P2P match.
///
/// # Method Override Table
///
/// Not all session types override every method. Methods not overridden use
/// sensible defaults (e.g., returning a "not supported" error or a no-op).
///
/// | Method | [`P2PSession`] | [`SpectatorSession`] | [`SyncTestSession`] |
/// |--------|:-:|:-:|:-:|
/// | [`advance_frame`](Session::advance_frame) | ✅ Override | ✅ Override | ✅ Override |
/// | [`local_player_handle_required`](Session::local_player_handle_required) | ✅ Override | ✅ Override (error) | ✅ Override |
/// | [`add_local_input`](Session::add_local_input) | ✅ Override | ✅ Override (error) | ✅ Override |
/// | [`events`](Session::events) | ✅ Override | ✅ Override | ✅ Override |
/// | [`current_state`](Session::current_state) | ✅ Override | ✅ Override | ❌ Default (`Running`) |
/// | [`poll_remote_clients`](Session::poll_remote_clients) | ✅ Override | ✅ Override | ❌ Default (no-op) |
///
/// # Example
///
/// ```no_run
/// use fortress_rollback::prelude::*;
///
/// fn run_frame<T: Config>(session: &mut impl Session<T>, input: T::Input) -> FortressResult<()> {
///     let player = session.local_player_handle_required()?;
///     session.add_local_input(player, input)?;
///     let requests = session.advance_frame()?;
///     for request in requests {
///         // handle requests...
///     }
///     Ok(())
/// }
/// ```
///
/// [`P2PSession`]: crate::P2PSession
/// [`SpectatorSession`]: crate::SpectatorSession
/// [`SyncTestSession`]: crate::SyncTestSession
pub trait Session<T: Config> {
    /// Advances the session by one frame, returning any requests the
    /// application must fulfill (save state, load state, advance game).
    ///
    /// This is the primary driver of the rollback loop.
    ///
    /// # Errors
    ///
    /// Returns a [`FortressError`] if the session is in a state that
    /// cannot advance (e.g., not synchronized, disconnected).
    ///
    /// [`FortressError`]: crate::FortressError
    #[must_use = "FortressRequests must be processed to advance the game state"]
    fn advance_frame(&mut self) -> FortressResult<RequestVec<T>>;

    /// Returns the [`PlayerHandle`] for the local player.
    ///
    /// Session types that do not have a local player (e.g., spectators)
    /// should return a "not supported" error from their implementation.
    ///
    /// # Errors
    ///
    /// Returns a [`FortressError`] if there is no local player or the
    /// operation is not supported by this session type.
    ///
    /// [`FortressError`]: crate::FortressError
    /// [`PlayerHandle`]: crate::PlayerHandle
    #[must_use = "returns the local player handle which should be used"]
    fn local_player_handle_required(&self) -> FortressResult<PlayerHandle>;

    /// Adds local input for the given player handle.
    ///
    /// The default implementation returns a "not supported" error, which is
    /// the correct behavior for session types that do not accept local input
    /// (e.g., spectators).
    ///
    /// # Errors
    ///
    /// Returns a [`FortressError`] if the input cannot be added or the
    /// operation is not supported by this session type.
    ///
    /// [`FortressError`]: crate::FortressError
    fn add_local_input(
        &mut self,
        _player_handle: PlayerHandle,
        _input: T::Input,
    ) -> FortressResult<()> {
        Err(InvalidRequestKind::NotSupported {
            operation: "add_local_input",
        }
        .into())
    }

    /// Drains pending events from the session.
    ///
    /// The default implementation returns an empty [`EventDrain`].
    ///
    /// [`EventDrain`]: crate::EventDrain
    #[must_use = "events should be handled to react to session state changes"]
    fn events(&mut self) -> EventDrain<'_, T> {
        EventDrain::empty()
    }

    /// Returns the current state of the session.
    ///
    /// The default implementation returns [`SessionState::Running`].
    ///
    /// [`SessionState::Running`]: crate::SessionState::Running
    fn current_state(&self) -> SessionState {
        SessionState::Running
    }

    /// Polls remote clients for incoming data.
    ///
    /// The default implementation is a no-op, suitable for session types
    /// without network communication (e.g., sync test sessions).
    fn poll_remote_clients(&mut self) {}
}

#[cfg(test)]
#[allow(clippy::panic, clippy::unwrap_used, clippy::expect_used, dead_code)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestConfig;

    impl Config for TestConfig {
        type Input = u8;
        type State = Vec<u8>;
        type Address = SocketAddr;
    }

    /// Compile-time assertion that `Session` is object-safe.
    ///
    /// If someone adds a method with `Self` return or method-level generics,
    /// this will fail to compile.
    fn _assert_object_safe(_: &dyn Session<TestConfig>) {}
}
