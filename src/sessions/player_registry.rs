//! Player registry for tracking players and their connection states.
//!
//! This module provides the [`PlayerRegistry`] struct that manages all players
//! (local, remote, and spectators) and their protocol handlers.

use crate::error::{FortressError, InvalidRequestKind};
use crate::network::protocol::UdpProtocol;
use crate::{Config, HandleVec, PlayerHandle, PlayerType};
use std::collections::BTreeMap;

/// Registry tracking all players and their connection states.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
pub struct PlayerRegistry<T>
where
    T: Config,
{
    /// Map from player handles to their types.
    pub handles: BTreeMap<PlayerHandle, PlayerType<T::Address>>,
    /// Map from addresses to protocol handlers for remote players.
    pub remotes: BTreeMap<T::Address, UdpProtocol<T>>,
    /// Map from addresses to protocol handlers for spectators.
    pub spectators: BTreeMap<T::Address, UdpProtocol<T>>,
}

impl<T> std::fmt::Debug for PlayerRegistry<T>
where
    T: Config,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Destructure to ensure all fields are included when new fields are added.
        let Self {
            handles,
            remotes,
            spectators,
        } = self;

        f.debug_struct("PlayerRegistry")
            .field("handles", handles)
            .field("remotes", &remotes.keys())
            .field("spectators", &spectators.keys())
            .finish()
    }
}

impl<T: Config> PlayerRegistry<T> {
    /// Creates a new empty player registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            handles: BTreeMap::new(),
            remotes: BTreeMap::new(),
            spectators: BTreeMap::new(),
        }
    }

    /// Returns an iterator over local player handles.
    ///
    /// This is a zero-allocation alternative to [`local_player_handles`].
    /// Use this when you only need to iterate once or want to avoid allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::SocketAddr;
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// registry.handles.insert(PlayerHandle::new(0), PlayerType::Local);
    ///
    /// for handle in registry.local_player_handles_iter() {
    ///     assert_eq!(handle, PlayerHandle::new(0));
    /// }
    /// ```
    ///
    /// [`local_player_handles`]: Self::local_player_handles
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn local_player_handles_iter(&self) -> impl Iterator<Item = PlayerHandle> + '_ {
        self.handles
            .iter()
            .filter_map(|(k, v)| matches!(v, PlayerType::Local).then_some(*k))
    }

    /// Returns handles for all local players.
    ///
    /// For a zero-allocation alternative, see [`local_player_handles_iter`].
    ///
    /// [`local_player_handles_iter`]: Self::local_player_handles_iter
    #[must_use]
    pub fn local_player_handles(&self) -> HandleVec {
        self.local_player_handles_iter().collect()
    }

    /// Returns the single local player's handle, or an error if there isn't exactly one.
    ///
    /// This is a zero-allocation convenience method for the common case of games with exactly one
    /// local player (typical client in a networked game). It uses an iterator internally to avoid
    /// heap allocation. It returns an error if:
    /// - No local players are registered (`NoLocalPlayers`)
    /// - More than one local player is registered (`MultipleLocalPlayers`)
    ///
    /// For games with multiple local players (e.g., local co-op), use
    /// [`Self::local_player_handles`] instead.
    ///
    /// # Errors
    ///
    /// - [`InvalidRequestKind::NoLocalPlayers`] if no local players are registered.
    /// - [`InvalidRequestKind::MultipleLocalPlayers`] if more than one local player is registered.
    ///
    /// [`InvalidRequestKind::NoLocalPlayers`]: crate::InvalidRequestKind::NoLocalPlayers
    /// [`InvalidRequestKind::MultipleLocalPlayers`]: crate::InvalidRequestKind::MultipleLocalPlayers
    #[must_use = "returns the local player handle which should be used"]
    pub fn local_player_handle_required(&self) -> Result<PlayerHandle, FortressError> {
        let mut iter = self.local_player_handles_iter();
        match (iter.next(), iter.next()) {
            (None, _) => Err(InvalidRequestKind::NoLocalPlayers.into()),
            (Some(handle), None) => Ok(handle),
            (Some(_), Some(_)) => {
                let count = 2 + iter.count();
                Err(InvalidRequestKind::MultipleLocalPlayers { count }.into())
            },
        }
    }

    /// Returns an iterator over remote player handles.
    ///
    /// This is a zero-allocation alternative to [`remote_player_handles`].
    /// Use this when you only need to iterate once or want to avoid allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    /// registry.handles.insert(PlayerHandle::new(1), PlayerType::Remote(addr));
    ///
    /// for handle in registry.remote_player_handles_iter() {
    ///     assert_eq!(handle, PlayerHandle::new(1));
    /// }
    /// ```
    ///
    /// [`remote_player_handles`]: Self::remote_player_handles
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn remote_player_handles_iter(&self) -> impl Iterator<Item = PlayerHandle> + '_ {
        self.handles
            .iter()
            .filter_map(|(k, v)| matches!(v, PlayerType::Remote(_)).then_some(*k))
    }

    /// Returns handles for all remote players.
    ///
    /// For a zero-allocation alternative, see [`remote_player_handles_iter`].
    ///
    /// [`remote_player_handles_iter`]: Self::remote_player_handles_iter
    #[must_use]
    pub fn remote_player_handles(&self) -> HandleVec {
        self.remote_player_handles_iter().collect()
    }

    /// Returns the single remote player's handle, or an error if there isn't exactly one.
    ///
    /// This is a zero-allocation convenience method for the common case of 1v1 games
    /// with exactly one remote player. It uses an iterator internally to avoid heap
    /// allocation. It returns an error if:
    /// - No remote players are registered (`NoRemotePlayers`)
    /// - More than one remote player is registered (`MultipleRemotePlayers`)
    ///
    /// For games with multiple remote players, use [`Self::remote_player_handles`] instead.
    ///
    /// # Errors
    ///
    /// - [`InvalidRequestKind::NoRemotePlayers`] if no remote players are registered.
    /// - [`InvalidRequestKind::MultipleRemotePlayers`] if more than one remote player is registered.
    ///
    /// [`InvalidRequestKind::NoRemotePlayers`]: crate::InvalidRequestKind::NoRemotePlayers
    /// [`InvalidRequestKind::MultipleRemotePlayers`]: crate::InvalidRequestKind::MultipleRemotePlayers
    #[must_use = "returns the remote player handle which should be used"]
    pub fn remote_player_handle_required(&self) -> Result<PlayerHandle, FortressError> {
        let mut iter = self.remote_player_handles_iter();
        match (iter.next(), iter.next()) {
            (None, _) => Err(InvalidRequestKind::NoRemotePlayers.into()),
            (Some(handle), None) => Ok(handle),
            (Some(_), Some(_)) => {
                let count = 2 + iter.count();
                Err(InvalidRequestKind::MultipleRemotePlayers { count }.into())
            },
        }
    }

    /// Returns an iterator over spectator handles.
    ///
    /// This is a zero-allocation alternative to [`spectator_handles`].
    /// Use this when you only need to iterate once or want to avoid allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9090);
    /// registry.handles.insert(PlayerHandle::new(10), PlayerType::Spectator(addr));
    ///
    /// for handle in registry.spectator_handles_iter() {
    ///     assert_eq!(handle, PlayerHandle::new(10));
    /// }
    /// ```
    ///
    /// [`spectator_handles`]: Self::spectator_handles
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn spectator_handles_iter(&self) -> impl Iterator<Item = PlayerHandle> + '_ {
        self.handles
            .iter()
            .filter_map(|(k, v)| matches!(v, PlayerType::Spectator(_)).then_some(*k))
    }

    /// Returns handles for all spectators.
    ///
    /// For a zero-allocation alternative, see [`spectator_handles_iter`].
    ///
    /// [`spectator_handles_iter`]: Self::spectator_handles_iter
    #[must_use]
    pub fn spectator_handles(&self) -> HandleVec {
        self.spectator_handles_iter().collect()
    }

    /// Returns the number of players (local + remote, excluding spectators).
    #[must_use]
    pub fn num_players(&self) -> usize {
        self.handles
            .values()
            .filter(|v| matches!(v, PlayerType::Local | PlayerType::Remote(_)))
            .count()
    }

    /// Returns the number of spectators.
    #[must_use]
    pub fn num_spectators(&self) -> usize {
        self.spectator_handles_iter().count()
    }

    /// Returns an iterator over handles associated with a given address.
    ///
    /// This is a zero-allocation alternative to [`handles_by_address`].
    /// Use this when you only need to iterate once or want to avoid allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    /// registry.handles.insert(PlayerHandle::new(1), PlayerType::Remote(addr));
    ///
    /// for handle in registry.handles_by_address_iter(addr) {
    ///     assert_eq!(handle, PlayerHandle::new(1));
    /// }
    /// ```
    ///
    /// [`handles_by_address`]: Self::handles_by_address
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn handles_by_address_iter(
        &self,
        addr: T::Address,
    ) -> impl Iterator<Item = PlayerHandle> + '_ {
        self.handles
            .iter()
            .filter_map(move |(h, player_type)| match player_type {
                PlayerType::Local => None,
                PlayerType::Remote(a) | PlayerType::Spectator(a) => (*a == addr).then_some(*h),
            })
    }

    /// Returns all handles associated with a given address.
    ///
    /// For a zero-allocation alternative, see [`handles_by_address_iter`].
    ///
    /// [`handles_by_address_iter`]: Self::handles_by_address_iter
    #[must_use]
    pub fn handles_by_address(&self, addr: T::Address) -> HandleVec {
        self.handles_by_address_iter(addr).collect()
    }

    /// Returns `true` if the given handle refers to a local player.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::SocketAddr;
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// let handle = PlayerHandle::new(0);
    /// registry.handles.insert(handle, PlayerType::Local);
    ///
    /// assert!(registry.is_local_player(handle));
    /// assert!(!registry.is_local_player(PlayerHandle::new(1)));
    /// ```
    #[must_use]
    pub fn is_local_player(&self, handle: PlayerHandle) -> bool {
        matches!(self.handles.get(&handle), Some(PlayerType::Local))
    }

    /// Returns `true` if the given handle refers to a remote player.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// let handle = PlayerHandle::new(0);
    /// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    /// registry.handles.insert(handle, PlayerType::Remote(addr));
    ///
    /// assert!(registry.is_remote_player(handle));
    /// assert!(!registry.is_remote_player(PlayerHandle::new(1)));
    /// ```
    #[must_use]
    pub fn is_remote_player(&self, handle: PlayerHandle) -> bool {
        matches!(self.handles.get(&handle), Some(PlayerType::Remote(_)))
    }

    /// Returns `true` if the given handle refers to a spectator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// let handle = PlayerHandle::new(10);
    /// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9090);
    /// registry.handles.insert(handle, PlayerType::Spectator(addr));
    ///
    /// assert!(registry.is_spectator_handle(handle));
    /// assert!(!registry.is_spectator_handle(PlayerHandle::new(0)));
    /// ```
    #[must_use]
    pub fn is_spectator_handle(&self, handle: PlayerHandle) -> bool {
        matches!(self.handles.get(&handle), Some(PlayerType::Spectator(_)))
    }

    /// Returns the player type for the given handle, if registered.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// let handle = PlayerHandle::new(0);
    /// registry.handles.insert(handle, PlayerType::Local);
    ///
    /// assert_eq!(registry.player_type(handle), Some(PlayerType::Local));
    /// assert_eq!(registry.player_type(PlayerHandle::new(99)), None);
    /// ```
    #[must_use]
    pub fn player_type(&self, handle: PlayerHandle) -> Option<PlayerType<T::Address>> {
        self.handles.get(&handle).cloned()
    }

    /// Returns the number of local players.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::SocketAddr;
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// assert_eq!(registry.num_local_players(), 0);
    ///
    /// registry.handles.insert(PlayerHandle::new(0), PlayerType::Local);
    /// assert_eq!(registry.num_local_players(), 1);
    /// ```
    #[must_use]
    pub fn num_local_players(&self) -> usize {
        self.local_player_handles_iter().count()
    }

    /// Returns the number of remote players (excluding spectators).
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    /// registry.handles.insert(PlayerHandle::new(1), PlayerType::Remote(addr));
    ///
    /// assert_eq!(registry.num_remote_players(), 1);
    /// ```
    #[must_use]
    pub fn num_remote_players(&self) -> usize {
        self.remote_player_handles_iter().count()
    }

    /// Returns an iterator over all registered player handles.
    ///
    /// This is a zero-allocation alternative to [`all_player_handles`].
    /// Handles are returned in sorted order (BTreeMap iteration order).
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// registry.handles.insert(PlayerHandle::new(0), PlayerType::Local);
    /// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    /// registry.handles.insert(PlayerHandle::new(1), PlayerType::Remote(addr));
    ///
    /// let count = registry.all_player_handles_iter().count();
    /// assert_eq!(count, 2);
    /// ```
    ///
    /// [`all_player_handles`]: Self::all_player_handles
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn all_player_handles_iter(&self) -> impl Iterator<Item = PlayerHandle> + '_ {
        self.handles.keys().copied()
    }

    /// Returns all registered player handles in sorted order.
    ///
    /// This includes local players, remote players, and spectators.
    /// For a zero-allocation alternative, see [`all_player_handles_iter`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::sessions::player_registry::PlayerRegistry;
    /// # use fortress_rollback::{PlayerHandle, PlayerType};
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # struct TestConfig;
    /// # impl fortress_rollback::Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let mut registry = PlayerRegistry::<TestConfig>::new();
    /// registry.handles.insert(PlayerHandle::new(0), PlayerType::Local);
    /// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    /// registry.handles.insert(PlayerHandle::new(1), PlayerType::Remote(addr));
    ///
    /// let handles = registry.all_player_handles();
    /// assert_eq!(handles.len(), 2);
    /// ```
    ///
    /// [`all_player_handles_iter`]: Self::all_player_handles_iter
    #[must_use]
    pub fn all_player_handles(&self) -> HandleVec {
        self.all_player_handles_iter().collect()
    }
}

impl<T: Config> Default for PlayerRegistry<T> {
    fn default() -> Self {
        Self::new()
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
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    struct TestConfig;

    impl Config for TestConfig {
        type Input = u8;
        type State = u8;
        type Address = SocketAddr;
    }

    fn test_addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    #[test]
    fn player_registry_new_is_empty() {
        let registry = PlayerRegistry::<TestConfig>::new();
        assert_eq!(registry.num_players(), 0);
        assert_eq!(registry.num_spectators(), 0);
        assert!(registry.local_player_handles().is_empty());
        assert!(registry.remote_player_handles().is_empty());
        assert!(registry.spectator_handles().is_empty());
    }

    #[test]
    fn player_registry_default_is_empty() {
        let registry = PlayerRegistry::<TestConfig>::default();
        assert_eq!(registry.num_players(), 0);
        assert_eq!(registry.num_spectators(), 0);
    }

    #[test]
    fn player_registry_with_local_player() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let handle = PlayerHandle::new(0);
        registry.handles.insert(handle, PlayerType::Local);

        assert_eq!(registry.num_players(), 1);
        assert_eq!(registry.num_spectators(), 0);
        assert_eq!(
            registry.local_player_handles(),
            HandleVec::from_iter([handle])
        );
        assert!(registry.remote_player_handles().is_empty());
        assert!(registry.spectator_handles().is_empty());
    }

    #[test]
    fn player_registry_with_remote_player() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let handle = PlayerHandle::new(1);
        let addr = test_addr(8080);
        registry.handles.insert(handle, PlayerType::Remote(addr));

        assert_eq!(registry.num_players(), 1);
        assert_eq!(registry.num_spectators(), 0);
        assert!(registry.local_player_handles().is_empty());
        assert_eq!(
            registry.remote_player_handles(),
            HandleVec::from_iter([handle])
        );
        assert!(registry.spectator_handles().is_empty());
    }

    #[test]
    fn player_registry_with_spectator() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let handle = PlayerHandle::new(10);
        let addr = test_addr(9090);
        registry.handles.insert(handle, PlayerType::Spectator(addr));

        assert_eq!(registry.num_players(), 0);
        assert_eq!(registry.num_spectators(), 1);
        assert!(registry.local_player_handles().is_empty());
        assert!(registry.remote_player_handles().is_empty());
        let spectators = registry.spectator_handles();
        assert_eq!(spectators, HandleVec::from_iter([handle]));
    }

    #[test]
    fn player_registry_mixed_players() {
        let mut registry = PlayerRegistry::<TestConfig>::new();

        // Add local player
        let local_handle = PlayerHandle::new(0);
        registry.handles.insert(local_handle, PlayerType::Local);

        // Add remote player
        let remote_handle = PlayerHandle::new(1);
        let remote_addr = test_addr(8080);
        registry
            .handles
            .insert(remote_handle, PlayerType::Remote(remote_addr));

        // Add spectator
        let spec_handle = PlayerHandle::new(10);
        let spec_addr = test_addr(9090);
        registry
            .handles
            .insert(spec_handle, PlayerType::Spectator(spec_addr));

        assert_eq!(registry.num_players(), 2); // local + remote
        assert_eq!(registry.num_spectators(), 1);
        assert_eq!(
            registry.local_player_handles(),
            HandleVec::from_iter([local_handle])
        );
        assert_eq!(
            registry.remote_player_handles(),
            HandleVec::from_iter([remote_handle])
        );
    }

    #[test]
    fn player_registry_handles_by_address_remote() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let addr = test_addr(8080);

        let handle1 = PlayerHandle::new(1);
        registry.handles.insert(handle1, PlayerType::Remote(addr));

        // Look up by address
        let found = registry.handles_by_address(addr);
        assert_eq!(found.len(), 1);
        assert!(found.contains(&handle1));
    }

    #[test]
    fn player_registry_handles_by_address_spectator() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let addr = test_addr(9090);

        let handle = PlayerHandle::new(10);
        registry.handles.insert(handle, PlayerType::Spectator(addr));

        let found = registry.handles_by_address(addr);
        assert_eq!(found.len(), 1);
        assert!(found.contains(&handle));
    }

    #[test]
    fn player_registry_handles_by_address_not_found() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let addr = test_addr(8080);
        let other_addr = test_addr(9999);

        let handle = PlayerHandle::new(1);
        registry.handles.insert(handle, PlayerType::Remote(addr));

        // Look up different address
        let found = registry.handles_by_address(other_addr);
        assert!(found.is_empty());
    }

    #[test]
    fn player_registry_handles_by_address_excludes_local() {
        let mut registry = PlayerRegistry::<TestConfig>::new();

        let handle = PlayerHandle::new(0);
        registry.handles.insert(handle, PlayerType::Local);

        // Local players don't have addresses, so any address lookup returns empty
        let found = registry.handles_by_address(test_addr(1234));
        assert!(found.is_empty());
    }

    #[test]
    fn player_registry_multiple_handles_same_address() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let addr = test_addr(8080);

        // Two players at the same address (e.g., couch co-op on remote machine)
        let handle1 = PlayerHandle::new(1);
        let handle2 = PlayerHandle::new(2);
        registry.handles.insert(handle1, PlayerType::Remote(addr));
        registry.handles.insert(handle2, PlayerType::Remote(addr));

        let found = registry.handles_by_address(addr);
        assert_eq!(found.len(), 2);
        assert!(found.contains(&handle1));
        assert!(found.contains(&handle2));
    }

    #[test]
    fn player_registry_debug_format() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let addr = test_addr(8080);

        registry
            .handles
            .insert(PlayerHandle::new(0), PlayerType::Local);
        registry
            .handles
            .insert(PlayerHandle::new(1), PlayerType::Remote(addr));

        let debug_str = format!("{:?}", registry);
        assert!(debug_str.contains("PlayerRegistry"));
        assert!(debug_str.contains("handles"));
        assert!(debug_str.contains("remotes"));
        assert!(debug_str.contains("spectators"));
    }

    // ==========================================
    // New Helper Method Tests
    // ==========================================

    #[test]
    fn is_local_player_returns_true_for_local() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let handle = PlayerHandle::new(0);
        registry.handles.insert(handle, PlayerType::Local);

        assert!(registry.is_local_player(handle));
    }

    #[test]
    fn is_local_player_returns_false_for_remote() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let handle = PlayerHandle::new(0);
        registry
            .handles
            .insert(handle, PlayerType::Remote(test_addr(8080)));

        assert!(!registry.is_local_player(handle));
    }

    #[test]
    fn is_local_player_returns_false_for_unknown() {
        let registry = PlayerRegistry::<TestConfig>::new();
        assert!(!registry.is_local_player(PlayerHandle::new(99)));
    }

    #[test]
    fn is_remote_player_returns_true_for_remote() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let handle = PlayerHandle::new(1);
        registry
            .handles
            .insert(handle, PlayerType::Remote(test_addr(8080)));

        assert!(registry.is_remote_player(handle));
    }

    #[test]
    fn is_remote_player_returns_false_for_local() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let handle = PlayerHandle::new(0);
        registry.handles.insert(handle, PlayerType::Local);

        assert!(!registry.is_remote_player(handle));
    }

    #[test]
    fn is_remote_player_returns_false_for_spectator() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let handle = PlayerHandle::new(10);
        registry
            .handles
            .insert(handle, PlayerType::Spectator(test_addr(9090)));

        assert!(!registry.is_remote_player(handle));
    }

    #[test]
    fn is_spectator_handle_returns_true_for_spectator() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let handle = PlayerHandle::new(10);
        registry
            .handles
            .insert(handle, PlayerType::Spectator(test_addr(9090)));

        assert!(registry.is_spectator_handle(handle));
    }

    #[test]
    fn is_spectator_handle_returns_false_for_remote() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let handle = PlayerHandle::new(1);
        registry
            .handles
            .insert(handle, PlayerType::Remote(test_addr(8080)));

        assert!(!registry.is_spectator_handle(handle));
    }

    #[test]
    fn player_type_returns_correct_type() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let local_handle = PlayerHandle::new(0);
        let remote_handle = PlayerHandle::new(1);
        let spec_handle = PlayerHandle::new(10);
        let remote_addr = test_addr(8080);
        let spec_addr = test_addr(9090);

        registry.handles.insert(local_handle, PlayerType::Local);
        registry
            .handles
            .insert(remote_handle, PlayerType::Remote(remote_addr));
        registry
            .handles
            .insert(spec_handle, PlayerType::Spectator(spec_addr));

        assert_eq!(registry.player_type(local_handle), Some(PlayerType::Local));
        assert_eq!(
            registry.player_type(remote_handle),
            Some(PlayerType::Remote(remote_addr))
        );
        assert_eq!(
            registry.player_type(spec_handle),
            Some(PlayerType::Spectator(spec_addr))
        );
        assert_eq!(registry.player_type(PlayerHandle::new(99)), None);
    }

    #[test]
    fn num_local_players_counts_correctly() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        assert_eq!(registry.num_local_players(), 0);

        registry
            .handles
            .insert(PlayerHandle::new(0), PlayerType::Local);
        assert_eq!(registry.num_local_players(), 1);

        registry
            .handles
            .insert(PlayerHandle::new(1), PlayerType::Local);
        assert_eq!(registry.num_local_players(), 2);

        // Remote doesn't count
        registry
            .handles
            .insert(PlayerHandle::new(2), PlayerType::Remote(test_addr(8080)));
        assert_eq!(registry.num_local_players(), 2);
    }

    #[test]
    fn num_remote_players_counts_correctly() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        assert_eq!(registry.num_remote_players(), 0);

        registry
            .handles
            .insert(PlayerHandle::new(1), PlayerType::Remote(test_addr(8080)));
        assert_eq!(registry.num_remote_players(), 1);

        // Spectator doesn't count as remote
        registry.handles.insert(
            PlayerHandle::new(10),
            PlayerType::Spectator(test_addr(9090)),
        );
        assert_eq!(registry.num_remote_players(), 1);
    }

    #[test]
    fn all_player_handles_returns_all() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let h0 = PlayerHandle::new(0);
        let h1 = PlayerHandle::new(1);
        let h10 = PlayerHandle::new(10);

        registry.handles.insert(h0, PlayerType::Local);
        registry
            .handles
            .insert(h1, PlayerType::Remote(test_addr(8080)));
        registry
            .handles
            .insert(h10, PlayerType::Spectator(test_addr(9090)));

        let all = registry.all_player_handles();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&h0));
        assert!(all.contains(&h1));
        assert!(all.contains(&h10));
    }

    // ==========================================
    // remote_player_handle_required tests
    // ==========================================

    #[test]
    fn remote_player_handle_required_returns_handle_for_single_remote() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        let handle = PlayerHandle::new(1);
        registry
            .handles
            .insert(handle, PlayerType::Remote(test_addr(8080)));

        let result = registry.remote_player_handle_required();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), handle);
    }

    #[test]
    fn remote_player_handle_required_returns_error_for_no_remotes() {
        let registry = PlayerRegistry::<TestConfig>::new();

        let result = registry.remote_player_handle_required();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                FortressError::InvalidRequestStructured {
                    kind: InvalidRequestKind::NoRemotePlayers
                }
            ),
            "Expected NoRemotePlayers error, got: {:?}",
            err
        );
    }

    #[test]
    fn remote_player_handle_required_returns_error_for_multiple_remotes() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        registry
            .handles
            .insert(PlayerHandle::new(0), PlayerType::Remote(test_addr(8080)));
        registry
            .handles
            .insert(PlayerHandle::new(1), PlayerType::Remote(test_addr(8081)));

        let result = registry.remote_player_handle_required();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                FortressError::InvalidRequestStructured {
                    kind: InvalidRequestKind::MultipleRemotePlayers { count: 2 }
                }
            ),
            "Expected MultipleRemotePlayers error with count 2, got: {:?}",
            err
        );
    }

    #[test]
    fn remote_player_handle_required_ignores_local_and_spectators() {
        let mut registry = PlayerRegistry::<TestConfig>::new();
        // Add a local player and a spectator - these shouldn't count
        registry
            .handles
            .insert(PlayerHandle::new(0), PlayerType::Local);
        registry.handles.insert(
            PlayerHandle::new(10),
            PlayerType::Spectator(test_addr(9090)),
        );

        // No remote players, so should return error
        let result = registry.remote_player_handle_required();
        assert!(result.is_err());
    }
}
