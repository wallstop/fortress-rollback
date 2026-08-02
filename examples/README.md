<p align="center">
  <img src="../docs/assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# Example Instructions

Gathered here are some additional instructions on how to build and run the examples. Note that the examples are usually kept up-to-date with the most recent version of the code. If you are looking for example code compatible with a version published on crates.io, take a look at the [release tags](https://github.com/wallstop/fortress-rollback/tags).

All examples are headless and require no graphics or audio system libraries. Build the full set
with `cargo build --examples`.

## Configuration Example

Demonstrates all available configuration options including:

- Basic session setup
- Network presets (LAN, high-latency, lossy)
- Custom fine-tuned configurations
- Competitive and casual setups
- Spectator configuration
- Disconnect behavior selection: `DisconnectBehavior::Halt` (default, legacy halt-on-drop) vs `DisconnectBehavior::ContinueWithout` (graceful peer drop) via `SessionBuilder::with_disconnect_behavior`
- Runtime input-delay adjustment via `P2PSession::set_input_delay` / `P2PSession::input_delay` for hybrid delay+rollback netcode (constraints and error variants documented in the [User Guide](../docs/user-guide.md#adjusting-input-delay-at-runtime))

```shell
cargo run --example configuration
```

## Custom Socket Example

Demonstrates how to implement the `NonBlockingSocket` trait for custom networking transports:

- Channel-based socket for local testing
- WebSocket adapter skeleton
- Matchbox 0.14 browser adapter guidance using a raw channel and the Fortress codec pattern

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
- Comprehensive error matching, including the runtime-input-delay variants (`InputDelayDecreaseUnsupported`, `InputDelayMidSessionMultiLocalUnsupported`, `InputDelayMidSessionPendingOutputFull`), the `PlayerAlreadyRemoved` variant returned by `P2PSession::remove_player`, and the `InternalErrorKind::InputQueueGapFillFailed` library-bug variant

```shell
cargo run --example error_handling
```

## Request Handling Example

Demonstrates the different ways to handle `FortressRequest` in your game loop:

- **Manual matching** — Full control with explicit `match` statements
- **`handle_requests!` macro** — Less boilerplate, same semantics
- Using `compute_checksum()` for desync detection
- Understanding that `FortressRequest` is exhaustively matchable (no wildcard `_ =>` needed)

This is the recommended starting point for understanding request handling patterns.

```shell
cargo run --example request_handling
```

## Sync Test Example

Demonstrates using `SyncTestSession` to verify determinism in game logic:

- Setting up a SyncTestSession with SessionBuilder
- Running a simulation with known inputs
- Handling all FortressRequest variants (Save, Load, Advance)
- Computing and verifying checksums
- Understanding what causes MismatchedChecksum errors

This is a standalone example with no graphical dependencies, making it ideal for:

- CI/CD integration testing
- Understanding the sync test workflow
- Debugging determinism issues

```shell
cargo run --example sync_test
```
