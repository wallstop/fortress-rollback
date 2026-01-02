//! Game stub implementations for testing with enum-based inputs.

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

use std::hash::Hash;
use std::net::SocketAddr;

use fortress_rollback::hash::fnv1a_hash;
use fortress_rollback::{Config, FortressRequest, Frame, GameStateCell, InputVec};
use serde::{Deserialize, Serialize};

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    fnv1a_hash(t)
}

pub struct GameStubEnum {
    pub gs: StateStubEnum,
}

#[allow(dead_code)]
#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum EnumInput {
    #[default]
    Val1,
    Val2,
}

#[derive(Debug)]
pub struct StubEnumConfig;

impl Config for StubEnumConfig {
    type Input = EnumInput;
    type State = StateStubEnum;
    type Address = SocketAddr;
}

impl Default for GameStubEnum {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStubEnum {
    #[allow(dead_code)]
    #[must_use]
    pub fn new() -> GameStubEnum {
        GameStubEnum {
            gs: StateStubEnum { frame: 0, state: 0 },
        }
    }

    #[allow(dead_code)]
    pub fn handle_requests(&mut self, requests: Vec<FortressRequest<StubEnumConfig>>) {
        for request in requests {
            match request {
                FortressRequest::LoadGameState { cell, .. } => self.load_game_state(cell),
                FortressRequest::SaveGameState { cell, frame } => self.save_game_state(cell, frame),
                FortressRequest::AdvanceFrame { inputs } => self.advance_frame(inputs),
                _ => unreachable!("Unknown request type"),
            }
        }
    }

    /// Returns the current frame number.
    #[allow(dead_code)]
    #[must_use]
    pub fn current_frame(&self) -> i32 {
        self.gs.frame
    }

    fn save_game_state(&mut self, cell: GameStateCell<StateStubEnum>, frame: Frame) {
        assert_eq!(self.gs.frame, frame.as_i32());
        let checksum = calculate_hash(&self.gs);
        cell.save(frame, Some(self.gs), Some(checksum as u128));
    }

    fn load_game_state(&mut self, cell: GameStateCell<StateStubEnum>) {
        self.gs = cell.load().unwrap();
    }

    fn advance_frame(&mut self, inputs: InputVec<EnumInput>) {
        self.gs.advance_frame(inputs);
    }
}

#[derive(Default, Copy, Clone, Hash)]
pub struct StateStubEnum {
    pub frame: i32,
    pub state: i32,
}

impl StateStubEnum {
    fn advance_frame(&mut self, inputs: InputVec<EnumInput>) {
        let p0_inputs = inputs[0];
        let p1_inputs = inputs[1];

        if p0_inputs == p1_inputs {
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

impl GameStubHandler<StubEnumConfig> for GameStubEnum {
    type State = StateStubEnum;

    fn new() -> Self {
        GameStubEnum::new()
    }

    fn handle_requests(&mut self, requests: Vec<FortressRequest<StubEnumConfig>>) {
        GameStubEnum::handle_requests(self, requests);
    }

    fn current_frame(&self) -> i32 {
        self.gs.frame
    }
}
