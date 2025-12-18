//! Player registry for tracking players and their connection states.
//!
//! This module provides the [`PlayerRegistry`] struct that manages all players
//! (local, remote, and spectators) and their protocol handlers.

use crate::network::protocol::UdpProtocol;
use crate::{Config, PlayerHandle, PlayerType};
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

    /// Returns handles for all local players.
    #[must_use]
    pub fn local_player_handles(&self) -> Vec<PlayerHandle> {
        self.handles
            .iter()
            .filter_map(|(k, v)| match v {
                PlayerType::Local => Some(*k),
                PlayerType::Remote(_) => None,
                PlayerType::Spectator(_) => None,
            })
            .collect()
    }

    /// Returns handles for all remote players.
    #[must_use]
    pub fn remote_player_handles(&self) -> Vec<PlayerHandle> {
        self.handles
            .iter()
            .filter_map(|(k, v)| match v {
                PlayerType::Local => None,
                PlayerType::Remote(_) => Some(*k),
                PlayerType::Spectator(_) => None,
            })
            .collect()
    }

    /// Returns handles for all spectators.
    #[must_use]
    pub fn spectator_handles(&self) -> Vec<PlayerHandle> {
        self.handles
            .iter()
            .filter_map(|(k, v)| match v {
                PlayerType::Local => None,
                PlayerType::Remote(_) => None,
                PlayerType::Spectator(_) => Some(*k),
            })
            .collect()
    }

    /// Returns the number of players (local + remote, excluding spectators).
    #[must_use]
    pub fn num_players(&self) -> usize {
        self.handles
            .iter()
            .filter(|(_, v)| matches!(v, PlayerType::Local | PlayerType::Remote(_)))
            .count()
    }

    /// Returns the number of spectators.
    #[must_use]
    pub fn num_spectators(&self) -> usize {
        self.handles
            .iter()
            .filter(|(_, v)| matches!(v, PlayerType::Spectator(_)))
            .count()
    }

    /// Returns all handles associated with a given address.
    pub fn handles_by_address(&self, addr: T::Address) -> Vec<PlayerHandle> {
        let handles: Vec<PlayerHandle> = self
            .handles
            .iter()
            .filter_map(|(h, player_type)| match player_type {
                PlayerType::Local => None,
                PlayerType::Remote(a) => Some((h, a)),
                PlayerType::Spectator(a) => Some((h, a)),
            })
            .filter_map(|(h, a)| if addr == *a { Some(*h) } else { None })
            .collect();
        handles
    }
}

impl<T: Config> Default for PlayerRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
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
        assert_eq!(registry.local_player_handles(), vec![handle]);
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
        assert_eq!(registry.remote_player_handles(), vec![handle]);
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
        assert_eq!(spectators, vec![handle]);
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
        assert_eq!(registry.local_player_handles(), vec![local_handle]);
        assert_eq!(registry.remote_player_handles(), vec![remote_handle]);
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
}
