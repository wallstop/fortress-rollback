//! Game stub implementations for testing with struct-based inputs.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::needless_pass_by_ref_mut,
    clippy::use_self,
    clippy::derive_partial_eq_without_eq
)]

use fortress_rollback::rng::{thread_rng, Rng, ThreadRng};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::net::SocketAddr;

use fortress_rollback::hash::fnv1a_hash;
use fortress_rollback::{Config, FortressRequest, Frame, GameStateCell, InputVec};

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    fnv1a_hash(t)
}

pub struct GameStub {
    pub gs: StateStub,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StubInput {
    pub inp: u32,
}

#[derive(Debug)]
pub struct StubConfig;

impl Config for StubConfig {
    type Input = StubInput;
    type State = StateStub;
    type Address = SocketAddr;
}

impl Default for GameStub {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStub {
    #[allow(dead_code)]
    #[must_use]
    pub fn new() -> GameStub {
        GameStub {
            gs: StateStub { frame: 0, state: 0 },
        }
    }

    #[allow(dead_code)]
    pub fn handle_requests(&mut self, requests: Vec<FortressRequest<StubConfig>>) {
        for request in requests {
            match request {
                FortressRequest::LoadGameState { cell, .. } => self.load_game_state(cell),
                FortressRequest::SaveGameState { cell, frame } => self.save_game_state(cell, frame),
                FortressRequest::AdvanceFrame { inputs } => self.advance_frame(inputs),
            }
        }
    }

    /// Returns the current frame number.
    #[allow(dead_code)]
    #[must_use]
    pub fn current_frame(&self) -> i32 {
        self.gs.frame
    }

    fn save_game_state(&mut self, cell: GameStateCell<StateStub>, frame: Frame) {
        assert_eq!(self.gs.frame, frame.as_i32());
        let checksum = calculate_hash(&self.gs);
        cell.save(frame, Some(self.gs), Some(checksum as u128));
    }

    fn load_game_state(&mut self, cell: GameStateCell<StateStub>) {
        self.gs = cell.load().unwrap();
    }

    fn advance_frame(&mut self, inputs: InputVec<StubInput>) {
        self.gs.advance_frame(inputs);
    }
}

pub struct RandomChecksumGameStub {
    pub gs: StateStub,
    rng: ThreadRng,
}

impl Default for RandomChecksumGameStub {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomChecksumGameStub {
    #[allow(dead_code)]
    #[must_use]
    pub fn new() -> RandomChecksumGameStub {
        RandomChecksumGameStub {
            gs: StateStub { frame: 0, state: 0 },
            rng: thread_rng(),
        }
    }

    #[allow(dead_code)]
    pub fn handle_requests(&mut self, requests: Vec<FortressRequest<StubConfig>>) {
        for request in requests {
            match request {
                FortressRequest::LoadGameState { cell, .. } => self.load_game_state(cell),
                FortressRequest::SaveGameState { cell, frame } => self.save_game_state(cell, frame),
                FortressRequest::AdvanceFrame { inputs } => self.advance_frame(inputs),
            }
        }
    }

    fn save_game_state(&mut self, cell: GameStateCell<StateStub>, frame: Frame) {
        assert_eq!(self.gs.frame, frame.as_i32());

        let random_checksum: u128 = self.rng.gen();
        cell.save(frame, Some(self.gs), Some(random_checksum));
    }

    fn load_game_state(&mut self, cell: GameStateCell<StateStub>) {
        self.gs = cell.load().expect("No data found.");
    }

    fn advance_frame(&mut self, inputs: InputVec<StubInput>) {
        self.gs.advance_frame(inputs);
    }
}

/// A game stub that corrupts checksums after a configurable frame threshold.
///
/// This is useful for testing desync detection in a way that survives rollback.
/// Unlike corrupting the state before `handle_requests`, this corruption happens
/// during save_game_state, which means it persists even if rollback resimulation
/// occurs.
pub struct CorruptibleGameStub {
    pub gs: StateStub,
    /// Frame at which checksum corruption begins (inclusive).
    /// Frames before this threshold have correct checksums.
    pub corrupt_checksum_from_frame: Option<i32>,
}

impl Default for CorruptibleGameStub {
    fn default() -> Self {
        Self::new()
    }
}

impl CorruptibleGameStub {
    #[allow(dead_code)]
    #[must_use]
    pub fn new() -> CorruptibleGameStub {
        CorruptibleGameStub {
            gs: StateStub { frame: 0, state: 0 },
            corrupt_checksum_from_frame: None,
        }
    }

    /// Creates a new stub that corrupts checksums from the given frame onwards.
    #[allow(dead_code)]
    #[must_use]
    pub fn with_corruption_from(frame: i32) -> CorruptibleGameStub {
        CorruptibleGameStub {
            gs: StateStub { frame: 0, state: 0 },
            corrupt_checksum_from_frame: Some(frame),
        }
    }

    /// Enables checksum corruption from the specified frame onwards.
    #[allow(dead_code)]
    pub fn enable_corruption_from(&mut self, frame: i32) {
        self.corrupt_checksum_from_frame = Some(frame);
    }

    #[allow(dead_code)]
    pub fn handle_requests(&mut self, requests: Vec<FortressRequest<StubConfig>>) {
        for request in requests {
            match request {
                FortressRequest::LoadGameState { cell, .. } => self.load_game_state(cell),
                FortressRequest::SaveGameState { cell, frame } => self.save_game_state(cell, frame),
                FortressRequest::AdvanceFrame { inputs } => self.advance_frame(inputs),
            }
        }
    }

    fn save_game_state(&mut self, cell: GameStateCell<StateStub>, frame: Frame) {
        assert_eq!(self.gs.frame, frame.as_i32());

        // Compute the real checksum
        let real_checksum = calculate_hash(&self.gs) as u128;

        // Corrupt checksum if we're at or past the corruption threshold
        let checksum = match self.corrupt_checksum_from_frame {
            Some(corrupt_from) if frame.as_i32() >= corrupt_from => {
                // Use a deterministic but incorrect checksum (XOR with magic value)
                // This ensures the same frame always produces the same wrong checksum
                real_checksum ^ 0xDEAD_BEEF_CAFE_BABE_u128
            },
            _ => real_checksum,
        };

        cell.save(frame, Some(self.gs), Some(checksum));
    }

    fn load_game_state(&mut self, cell: GameStateCell<StateStub>) {
        self.gs = cell.load().unwrap();
    }

    fn advance_frame(&mut self, inputs: InputVec<StubInput>) {
        self.gs.advance_frame(inputs);
    }
}

#[derive(Default, Copy, Clone, Hash)]
pub struct StateStub {
    pub frame: i32,
    pub state: i32,
}

impl StateStub {
    fn advance_frame(&mut self, inputs: InputVec<StubInput>) {
        // Sum all player inputs for deterministic state update
        let total_inputs: u32 = inputs.iter().map(|(input, _)| input.inp).sum();

        if total_inputs % 2 == 0 {
            self.state += 2;
        } else {
            self.state -= 1;
        }
        self.frame += 1;
    }
}

// ============================================================================
// GameStubHandler trait implementation
// ============================================================================

use super::test_utils::GameStubHandler;

impl GameStubHandler<StubConfig> for GameStub {
    type State = StateStub;

    fn new() -> Self {
        GameStub::new()
    }

    fn handle_requests(&mut self, requests: Vec<FortressRequest<StubConfig>>) {
        GameStub::handle_requests(self, requests);
    }

    fn current_frame(&self) -> i32 {
        self.gs.frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corruptible_stub_no_corruption_by_default() {
        let stub = CorruptibleGameStub::new();
        assert!(stub.corrupt_checksum_from_frame.is_none());
    }

    #[test]
    fn corruptible_stub_with_corruption_threshold() {
        let stub = CorruptibleGameStub::with_corruption_from(10);
        assert_eq!(stub.corrupt_checksum_from_frame, Some(10));
    }

    #[test]
    fn corruptible_stub_enable_corruption() {
        let mut stub = CorruptibleGameStub::new();
        stub.enable_corruption_from(5);
        assert_eq!(stub.corrupt_checksum_from_frame, Some(5));
    }

    #[test]
    fn corruptible_stub_checksum_differs_when_corrupted() {
        // Create two stubs with the same state
        let clean_stub = CorruptibleGameStub::new();
        let corrupt_stub = CorruptibleGameStub::with_corruption_from(0);

        // Both stubs have identical state
        assert_eq!(clean_stub.gs.frame, corrupt_stub.gs.frame);
        assert_eq!(clean_stub.gs.state, corrupt_stub.gs.state);

        // Manually compute checksums
        let clean_checksum = calculate_hash(&clean_stub.gs) as u128;
        let corrupt_checksum = clean_checksum ^ 0xDEAD_BEEF_CAFE_BABE_u128;

        // Verify they differ
        assert_ne!(clean_checksum, corrupt_checksum);
    }

    #[test]
    fn corruptible_stub_checksum_deterministic() {
        // Corruption should be deterministic (same input -> same corrupt output)
        let stub1 = CorruptibleGameStub::with_corruption_from(0);
        let stub2 = CorruptibleGameStub::with_corruption_from(0);

        let checksum1 = calculate_hash(&stub1.gs) as u128 ^ 0xDEAD_BEEF_CAFE_BABE_u128;
        let checksum2 = calculate_hash(&stub2.gs) as u128 ^ 0xDEAD_BEEF_CAFE_BABE_u128;

        assert_eq!(checksum1, checksum2);
    }
}
