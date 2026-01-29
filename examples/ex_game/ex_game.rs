use std::net::SocketAddr;

use fortress_rollback::{
    compute_checksum_fletcher16, fletcher16, handle_requests, Config, FortressRequest, Frame,
    GameStateCell, InputStatus, InputVec, PlayerHandle,
};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

const FPS: u64 = 60;
const CHECKSUM_PERIOD: i32 = 100;

const SHIP_HEIGHT: f32 = 50.;
const SHIP_BASE: f32 = 40.;
const WINDOW_HEIGHT: f32 = 800.0;
const WINDOW_WIDTH: f32 = 600.0;

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;

const MOVEMENT_SPEED: f32 = 15.0 / FPS as f32;
const ROTATION_SPEED: f32 = 2.5 / FPS as f32;
const MAX_SPEED: f32 = 7.0;
const FRICTION: f32 = 0.98;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Input {
    pub inp: u8,
}

/// `FortressConfig` holds all type parameters for Fortress Rollback sessions
#[derive(Debug)]
pub struct FortressConfig;
impl Config for FortressConfig {
    type Input = Input;
    type State = State;
    type Address = SocketAddr;
}

// BoxGame will handle rendering, gamestate, inputs and Fortress Rollback requests
pub struct Game {
    num_players: usize,
    game_state: State,
    local_handles: Vec<PlayerHandle>,
    last_checksum: (Frame, u64),
    periodic_checksum: (Frame, u64),
}

impl Game {
    pub fn new(num_players: usize) -> Self {
        assert!(num_players <= 4);
        Self {
            num_players,
            game_state: State::new(num_players),
            local_handles: Vec::new(),
            last_checksum: (Frame::NULL, 0),
            periodic_checksum: (Frame::NULL, 0),
        }
    }

    // for each request, call the appropriate function
    pub fn handle_requests(
        &mut self,
        requests: Vec<FortressRequest<FortressConfig>>,
        in_lockstep: bool,
    ) {
        for request in requests {
            match request {
                FortressRequest::LoadGameState { cell, frame } => {
                    if in_lockstep {
                        // In lockstep mode, load requests are unexpected but we handle gracefully
                        eprintln!(
                            "WARNING: Unexpected LoadGameState request in lockstep mode (frame {:?})",
                            frame
                        );
                    }
                    self.load_game_state(cell, frame);
                },
                FortressRequest::SaveGameState { cell, frame } => {
                    if in_lockstep {
                        // In lockstep mode, save requests are unexpected but we handle gracefully
                        eprintln!(
                            "WARNING: Unexpected SaveGameState request in lockstep mode (frame {:?})",
                            frame
                        );
                    }
                    self.save_game_state(cell, frame);
                },
                FortressRequest::AdvanceFrame { inputs } => self.advance_frame(inputs),
            }
        }
    }

    /// Alternative request handling using the `handle_requests!` macro.
    ///
    /// This method shows how to use the macro for cleaner code when you don't
    /// need special lockstep handling.
    ///
    /// # Example
    ///
    /// ```ignore
    /// match sess.advance_frame() {
    ///     Ok(requests) => game.handle_requests_with_macro(requests),
    ///     Err(e) => return Err(Box::new(e)),
    /// }
    /// ```
    #[allow(dead_code)]
    pub fn handle_requests_with_macro(&mut self, requests: Vec<FortressRequest<FortressConfig>>) {
        handle_requests!(
            requests,
            save: |cell: GameStateCell<State>, frame: Frame| {
                self.save_game_state(cell, frame);
            },
            load: |cell: GameStateCell<State>, frame: Frame| {
                self.load_game_state(cell, frame);
            },
            advance: |inputs: InputVec<Input>| {
                self.advance_frame(inputs);
            }
        );
    }

    // save current gamestate, create a checksum
    // creating a checksum here is only relevant for SyncTestSessions
    fn save_game_state(&mut self, cell: GameStateCell<State>, frame: Frame) {
        assert_eq!(self.game_state.frame, frame.as_i32());
        // Use the built-in checksum helper for deterministic serialization + hashing
        let checksum = compute_checksum_fletcher16(&self.game_state).ok();
        cell.save(frame, Some(self.game_state.clone()), checksum);
    }

    // load gamestate and overwrite
    fn load_game_state(&mut self, cell: GameStateCell<State>, frame: Frame) {
        // LoadGameState is only requested for previously saved frames.
        // Missing state indicates a library bug, but we handle gracefully.
        if let Some(loaded) = cell.load() {
            self.game_state = loaded;
        } else {
            // This should never happen - log for debugging
            eprintln!("WARNING: LoadGameState for frame {frame:?} but no state found");
        }
    }

    fn advance_frame(&mut self, inputs: InputVec<Input>) {
        // advance the game state
        self.game_state.advance(inputs);

        // remember checksum to render it later
        // Note: it's more efficient to only compute checksums periodically for display
        // For actual desync detection, use the checksum passed to cell.save() in SaveGameState
        if let Ok(buffer) = fortress_rollback::network::codec::encode(&self.game_state) {
            let checksum = u64::from(fletcher16(&buffer));
            self.last_checksum = (Frame::new(self.game_state.frame), checksum);
            if self.game_state.frame % CHECKSUM_PERIOD == 0 {
                self.periodic_checksum = (Frame::new(self.game_state.frame), checksum);
            }
        }
    }

    // renders the game to the window
    pub fn render(&self) {
        clear_background(BLACK);

        // render players
        for i in 0..self.num_players {
            let color = match i {
                0 => GOLD,
                1 => BLUE,
                2 => GREEN,
                3 => RED,
                _ => WHITE,
            };
            let (x, y) = self.game_state.positions[i];
            let rotation = self.game_state.rotations[i] + std::f32::consts::PI / 2.0;
            let v1 = Vec2::new(
                x + rotation.sin() * SHIP_HEIGHT / 2.,
                y - rotation.cos() * SHIP_HEIGHT / 2.,
            );
            let v2 = Vec2::new(
                x - rotation.cos() * SHIP_BASE / 2. - rotation.sin() * SHIP_HEIGHT / 2.,
                y - rotation.sin() * SHIP_BASE / 2. + rotation.cos() * SHIP_HEIGHT / 2.,
            );
            let v3 = Vec2::new(
                x + rotation.cos() * SHIP_BASE / 2. - rotation.sin() * SHIP_HEIGHT / 2.,
                y + rotation.sin() * SHIP_BASE / 2. + rotation.cos() * SHIP_HEIGHT / 2.,
            );
            draw_triangle(v1, v2, v3, color);
        }

        // render checksums
        let last_checksum_str = format!(
            "Frame {}: Checksum {}",
            self.last_checksum.0, self.last_checksum.1
        );
        let periodic_checksum_str = format!(
            "Frame {}: Checksum {}",
            self.periodic_checksum.0, self.periodic_checksum.1
        );
        let force_desync_info_str = "Press SPACE to trigger a desync";
        draw_text(&last_checksum_str, 20.0, 20.0, 30.0, WHITE);
        draw_text(&periodic_checksum_str, 20.0, 40.0, 30.0, WHITE);
        draw_text(
            force_desync_info_str,
            90.0,
            WINDOW_HEIGHT * 9.0 / 10.0,
            30.0,
            WHITE,
        );
    }

    #[allow(dead_code)]
    pub fn register_local_handles(&mut self, handles: Vec<PlayerHandle>) {
        self.local_handles = handles
    }

    #[allow(dead_code)]
    // creates a compact representation of currently pressed keys and serializes it
    pub fn local_input(&mut self, handle: PlayerHandle) -> Input {
        // manually teleport the player to the center of the screen, but not through a proper input
        // this will create a forced desync (unless player one is already at the center)
        if is_key_pressed(KeyCode::Space) {
            self.game_state.positions[handle.as_usize()] =
                (WINDOW_WIDTH * 0.5, WINDOW_HEIGHT * 0.5);
        }

        let mut inp: u8 = 0;

        if handle == self.local_handles[0] {
            // first local player with WASD
            if is_key_down(KeyCode::W) {
                inp |= INPUT_UP;
            }
            if is_key_down(KeyCode::A) {
                inp |= INPUT_LEFT;
            }
            if is_key_down(KeyCode::S) {
                inp |= INPUT_DOWN;
            }
            if is_key_down(KeyCode::D) {
                inp |= INPUT_RIGHT;
            }
        } else {
            // all other local players with arrow keys
            if is_key_down(KeyCode::Up) {
                inp |= INPUT_UP;
            }
            if is_key_down(KeyCode::Left) {
                inp |= INPUT_LEFT;
            }
            if is_key_down(KeyCode::Down) {
                inp |= INPUT_DOWN;
            }
            if is_key_down(KeyCode::Right) {
                inp |= INPUT_RIGHT;
            }
        }

        Input { inp }
    }

    #[allow(dead_code)]
    pub const fn current_frame(&self) -> i32 {
        self.game_state.frame
    }
}

// BoxGameState holds all relevant information about the game state
#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    pub frame: i32,
    pub num_players: usize,
    pub positions: Vec<(f32, f32)>,
    pub velocities: Vec<(f32, f32)>,
    pub rotations: Vec<f32>,
}

impl State {
    pub fn new(num_players: usize) -> Self {
        let mut positions = Vec::new();
        let mut velocities = Vec::new();
        let mut rotations = Vec::new();

        let r = WINDOW_WIDTH / 4.0;

        for i in 0..num_players as i32 {
            let rot = i as f32 / num_players as f32 * 2.0 * std::f32::consts::PI;
            let x = WINDOW_WIDTH / 2.0 + r * rot.cos();
            let y = WINDOW_HEIGHT / 2.0 + r * rot.sin();
            positions.push((x, y));
            velocities.push((0.0, 0.0));
            rotations.push((rot + std::f32::consts::PI) % (2.0 * std::f32::consts::PI));
        }

        Self {
            frame: 0,
            num_players,
            positions,
            velocities,
            rotations,
        }
    }

    pub fn advance(&mut self, inputs: InputVec<Input>) {
        // increase the frame counter
        self.frame += 1;

        for (i, (player_input, status)) in inputs.iter().enumerate().take(self.num_players) {
            // get input of that player
            let input = match status {
                InputStatus::Confirmed => player_input.inp,
                InputStatus::Predicted => player_input.inp,
                InputStatus::Disconnected => 4, // disconnected players spin
            };

            // old values
            let (old_x, old_y) = self.positions[i];
            let (old_vel_x, old_vel_y) = self.velocities[i];
            let mut rot = self.rotations[i];

            // slow down
            let mut vel_x = old_vel_x * FRICTION;
            let mut vel_y = old_vel_y * FRICTION;

            // thrust
            if input & INPUT_UP != 0 && input & INPUT_DOWN == 0 {
                vel_x += MOVEMENT_SPEED * rot.cos();
                vel_y += MOVEMENT_SPEED * rot.sin();
            }
            // break
            if input & INPUT_UP == 0 && input & INPUT_DOWN != 0 {
                vel_x -= MOVEMENT_SPEED * rot.cos();
                vel_y -= MOVEMENT_SPEED * rot.sin();
            }
            // turn left
            if input & INPUT_LEFT != 0 && input & INPUT_RIGHT == 0 {
                rot = (rot - ROTATION_SPEED).rem_euclid(2.0 * std::f32::consts::PI);
            }
            // turn right
            if input & INPUT_LEFT == 0 && input & INPUT_RIGHT != 0 {
                rot = (rot + ROTATION_SPEED).rem_euclid(2.0 * std::f32::consts::PI);
            }

            // limit speed
            let magnitude = (vel_x * vel_x + vel_y * vel_y).sqrt();
            if magnitude > MAX_SPEED {
                vel_x = (vel_x * MAX_SPEED) / magnitude;
                vel_y = (vel_y * MAX_SPEED) / magnitude;
            }

            // compute new position
            let mut x = old_x + vel_x;
            let mut y = old_y + vel_y;

            // constrain players to canvas borders
            x = x.max(0.0);
            x = x.min(WINDOW_WIDTH);
            y = y.max(0.0);
            y = y.min(WINDOW_HEIGHT);

            // update all state
            self.positions[i] = (x, y);
            self.velocities[i] = (vel_x, vel_y);
            self.rotations[i] = rot;
        }
    }
}
