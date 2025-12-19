<p align="center">
  <img src="../assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# Example Instructions

Gathered here are some additional instructions on how to build and run the examples. Note that the examples are usually kept up-to-date with the most recent version of the code. If you are looking for example code compatible with a version published on crates.io, take a look at the [release tags](https://github.com/wallstop/fortress-rollback/tags).

## System Dependencies

The ExGame graphical examples use [macroquad](https://github.com/not-fl3/macroquad) for graphics and audio. These examples are **optional** and require the `graphical-examples` feature flag to build.

### Building Graphical Examples

To build the graphical examples, enable the feature:

```shell
cargo build --examples --features graphical-examples
```

### Linux (Debian/Ubuntu)

```shell
sudo apt-get install libasound2-dev libx11-dev libxi-dev libgl1-mesa-dev
```

### Linux (Fedora/RHEL)

```shell
sudo dnf install alsa-lib-devel libX11-devel libXi-devel mesa-libGL-devel
```

### macOS

No additional dependencies required (uses system frameworks).

### Windows

No additional dependencies required (uses system APIs).

---

## Configuration Example

Demonstrates all available configuration options including:

- Basic session setup
- Network presets (LAN, high-latency, lossy)
- Custom fine-tuned configurations
- Competitive and casual setups
- Spectator configuration

```shell
cargo run --example configuration
```

## Custom Socket Example

Demonstrates how to implement the `NonBlockingSocket` trait for custom networking transports:

- Channel-based socket for local testing
- WebSocket adapter skeleton
- Matchbox integration reference

This is essential reading if you're:
- Building a browser game (WebRTC/WebSockets)
- Integrating with a custom networking library
- Testing without real network connections

```shell
cargo run --example custom_socket
```

## Error Handling Example

Demonstrates proper error handling patterns:

- Configuration-time errors
- Session setup errors
- Runtime error handling
- Recovery strategies
- Comprehensive error matching

```shell
cargo run --example error_handling
```

## ExGame

ExGame is a very basic 2-4 player game example with each player controlling a coloured shape.
There is no real game, just movement with ice physics. Optionally, you can specify spectators.

- W to accelerate forwards
- S to accelerate backwards
- A to turn left
- D to turn right
- SPACE to move player 1 to (0, 0) locally (this will create a desync)

### Important Disclaimer - Determinism

Since ExGame is based on floats and uses floating-point sin, cos and sqrt,
I fully expect this example to desync when compiled on two different architectures/platforms.
This is intentional to see when and how that happens. If you plan to implement your own
deterministic game, make sure to take floating-point impresicions and non-deterministic results into consideration.

### Launching ExGame P2P and Spectator

The P2P example is launched by command-line arguments:

- `--local-port / -l`: local port the client is listening to
- `--players / -p`: a list of player addresses, with the local player being identified by `localhost`
- `--spectators / -s`: a list of spectator addresses. This client will act as a host for these spectators

For the spectator, the following command-line arguments exist:

- `--local-port / -l`: local port the client is listening to
- `--num-players / -n`: number of players that will participate in the game
- `--host / -h`: address of the host

For example, to run a two-player game on your local machine,
run these commands in separate terminals:

```shell
cargo run --example ex_game_p2p --features graphical-examples -- --local-port 7000 --players localhost 127.0.0.1:7001
cargo run --example ex_game_p2p --features graphical-examples -- --local-port 7001 --players 127.0.0.1:7000 localhost
```

In order to run a two-player game and a spectator on your local machine,
run these commands in separate terminals:

```shell
cargo run --example ex_game_p2p --features graphical-examples -- --local-port 7000 --players localhost 127.0.0.1:7001 --spectators 127.0.0.1:7002
cargo run --example ex_game_p2p --features graphical-examples -- --local-port 7001 --players 127.0.0.1:7000 localhost
cargo run --example ex_game_spectator --features graphical-examples -- --local-port 7002 --num-players 2 --host 127.0.0.1:7000 
```

In order to run a three-player game with two players playing in the same client and a third player playing on a second client,
run these commands in separate terminals:

```shell
cargo run --example ex_game_p2p --features graphical-examples -- --local-port 7000 --players localhost localhost 127.0.0.1:7001
cargo run --example ex_game_p2p --features graphical-examples -- --local-port 7001 --players 127.0.0.1:7000 127.0.0.1:7000 localhost
```

## ExGame SyncTest

The same game, but without network functionality.
Instead, the SyncTestSession focuses on simulating rollbacks and comparing checksums.
You can use the Arrow Keys in addition to WASD in order to move the second player.

### Launching ExGame SyncTest

ExGame SyncTest is launched by a single command-line argument:

- `--num-players / -n`: number of players that will participate in the game
- `--check-distance / -c`: number of frames that will be rolled back and resimulated each frame

```shell
cargo run --example ex_game_synctest --features graphical-examples -- --num-players 2 --check-distance 7
```
