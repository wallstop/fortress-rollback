<!-- SYNC: This wiki page is generated from docs/getting-started.md. Edit docs source. -->

# Build Your First Session

This guide builds a small deterministic simulation and runs it through real save, load, and
resimulation requests. It uses `SyncTestSession`, so you can verify the part your game controls
before adding sockets and another machine.

## Install

Add Fortress Rollback and Serde derives:

```toml
[dependencies]
fortress-rollback = "0.12"
serde = { version = "1", features = ["derive"] }
```

Fortress Rollback supports Rust 1.86 and newer.

## Run a maintained example

From a clone of this repository, run:

```bash
cargo run --example sync_test
```

The example drives 20 frames, forces rollback and resimulation, and fails with a structured error
if the same inputs produce a different checksum.

## Build the smallest useful loop

The following program is complete. Inputs use fixed-width integers because network sessions require
every input value to serialize to the same nonzero width. Game state uses integer arithmetic so the
same inputs produce the same result across targets.

```rust
use fortress_rollback::{
    compute_checksum, Config, FortressRequest, InputStatus, PlayerHandle, SessionBuilder,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
struct GameInput {
    direction: i8,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct GameState {
    frame: i32,
    position: i32,
}

struct GameConfig;

impl Config for GameConfig {
    type Input = GameInput;
    type State = GameState;
    type Address = SocketAddr;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = SessionBuilder::<GameConfig>::new()
        .with_num_players(1)?
        .with_check_distance(2)
        .with_max_prediction_window(8)
        .start_synctest_session()?;
    let mut state = GameState::default();

    for _ in 0..20 {
        session.add_local_input(
            PlayerHandle::new(0),
            GameInput { direction: 1 },
        )?;

        for request in session.advance_frame()? {
            match request {
                FortressRequest::SaveGameState { cell, frame } => {
                    let checksum = compute_checksum(&state)?;
                    cell.save(frame, Some(state.clone()), Some(checksum));
                },
                FortressRequest::LoadGameState { cell, .. } => {
                    if let Some(saved) = cell.load() {
                        state = saved;
                    }
                },
                FortressRequest::AdvanceFrame { inputs } => {
                    for (input, status) in inputs {
                        if status != InputStatus::Disconnected {
                            state.position = state
                                .position
                                .saturating_add(i32::from(input.direction));
                        }
                    }
                    state.frame = state.frame.saturating_add(1);
                },
            }
        }
    }

    Ok(())
}
```

Three rules matter:

1. Submit input for every local player before each `advance_frame` call.
2. Process every returned request once, in order.
3. Save every value that affects gameplay and restore the saved state on a load request.

Apply both confirmed and predicted inputs to the simulation. Skip an input only when its status is
`InputStatus::Disconnected`; rollback corrects a predicted value later if the peer sends a different
one.

Do not use render-only state, wall-clock time, unordered collection iteration, or unseeded random
values in the deterministic simulation. The [determinism model](Determinism-Model) explains
the full contract.

## Connect two peers

Once the local loop stays green, keep the same input and request code and change session setup:

1. Bind a `UdpNonBlockingSocket` or provide a custom `NonBlockingSocket`.
2. Add each participant with a stable `PlayerHandle`: one `PlayerType::Local` and one or more
   `PlayerType::Remote(address)` entries.
3. Call `start_p2p_session(socket)`.
4. Call `poll_remote_clients` every game tick, even while the session is synchronizing or stalled.
5. Submit local inputs and advance only while `current_state()` is `SessionState::Running`.

The maintained [configuration example](https://github.com/wallstop/fortress-rollback/blob/main/examples/configuration.rs)
shows session construction. The [request-handling example](https://github.com/wallstop/fortress-rollback/blob/main/examples/request_handling.rs)
shows both explicit matching and the `handle_requests!` macro. For engine or browser transports, use
the [custom socket example](https://github.com/wallstop/fortress-rollback/blob/main/examples/custom_socket.rs)
after the basic UDP path works.

## Before real playtests

- Run the SyncTest loop in CI with representative inputs and states.
- Read [The Game Loop](User-Guide#the-game-loop), especially polling, event draining, and frame
  pacing.
- Choose measured defaults from [Network Tuning](Tuning); do not guess at prediction windows.
- Decide how the game handles disconnects and desync evidence.
- Complete the [Production Checklist](Production-Checklist) before shipping.

Continue with the [User Guide](User-Guide) for session types, events, spectators, hot-join,
telemetry, and the complete configuration reference.
