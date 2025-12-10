use rand::{prelude::ThreadRng, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::net::SocketAddr;

use fortress_rollback::hash::fnv1a_hash;
use fortress_rollback::{Config, FortressRequest, Frame, GameStateCell, InputStatus};

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
                _ => unreachable!("Unknown request type"),
            }
        }
    }

    fn save_game_state(&mut self, cell: GameStateCell<StateStub>, frame: Frame) {
        assert_eq!(self.gs.frame, frame.as_i32());
        let checksum = calculate_hash(&self.gs);
        cell.save(frame, Some(self.gs), Some(checksum as u128));
    }

    fn load_game_state(&mut self, cell: GameStateCell<StateStub>) {
        self.gs = cell.load().unwrap();
    }

    fn advance_frame(&mut self, inputs: Vec<(StubInput, InputStatus)>) {
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
                _ => unreachable!("Unknown request type"),
            }
        }
    }

    fn save_game_state(&mut self, cell: GameStateCell<StateStub>, frame: Frame) {
        assert_eq!(self.gs.frame, frame.as_i32());

        let random_checksum: u128 = self.rng.r#gen();
        cell.save(frame, Some(self.gs), Some(random_checksum));
    }

    fn load_game_state(&mut self, cell: GameStateCell<StateStub>) {
        self.gs = cell.load().expect("No data found.");
    }

    fn advance_frame(&mut self, inputs: Vec<(StubInput, InputStatus)>) {
        self.gs.advance_frame(inputs);
    }
}

#[derive(Default, Copy, Clone, Hash)]
pub struct StateStub {
    pub frame: i32,
    pub state: i32,
}

impl StateStub {
    // Note: is_multiple_of() is nightly-only, so we use modulo
    #[allow(clippy::manual_is_multiple_of)]
    fn advance_frame(&mut self, inputs: Vec<(StubInput, InputStatus)>) {
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
