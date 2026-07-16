<p align="center">
  <img src="docs/assets/logo-banner.svg" alt="Fortress Rollback" width="600">
</p>

<p align="center">
  <a href="https://crates.io/crates/fortress-rollback"><img src="https://img.shields.io/crates/v/fortress-rollback?style=for-the-badge" alt="crates.io"></a>
  <a href="https://wallstop.github.io/fortress-rollback/"><img src="https://img.shields.io/badge/docs-GitHub%20Pages-blue?style=for-the-badge" alt="Documentation"></a>
  <a href="https://github.com/wallstop/fortress-rollback/wiki"><img src="https://img.shields.io/badge/wiki-GitHub%20Wiki-green?style=for-the-badge" alt="Wiki"></a>
</p>

<p align="center">
  <a href="https://github.com/wallstop/fortress-rollback/actions/workflows/ci-rust.yml"><img src="https://img.shields.io/github/actions/workflow/status/wallstop/fortress-rollback/ci-rust.yml?branch=main&style=for-the-badge&label=CI" alt="CI"></a>
  <a href="https://github.com/wallstop/fortress-rollback/actions/workflows/publish.yml"><img src="https://img.shields.io/github/actions/workflow/status/wallstop/fortress-rollback/publish.yml?branch=main&style=for-the-badge&label=Publish" alt="Publish"></a>
  <a href="https://github.com/wallstop/fortress-rollback/actions/workflows/ci-coverage.yml"><img src="https://img.shields.io/github/actions/workflow/status/wallstop/fortress-rollback/ci-coverage.yml?branch=main&style=for-the-badge&label=Coverage" alt="Coverage"></a>
  <a href="https://github.com/wallstop/fortress-rollback/actions/workflows/ci-benchmarks.yml"><img src="https://img.shields.io/github/actions/workflow/status/wallstop/fortress-rollback/ci-benchmarks.yml?branch=main&style=for-the-badge&label=Benchmark" alt="Benchmark"></a>
</p>

---

> **🤖 AI-Assisted Development Notice**
>
> This project was developed with **substantial AI assistance**. The vast majority of the code, documentation, tests, and formal specifications were written with the help of **Claude Opus 4.5** and **Codex 5.1**. Human oversight was provided for code review, architectural decisions, and final approval, but the implementation work was heavily AI-driven. This transparency is provided so users can make informed decisions about using this crate.

---

---
> ⚠️⚠️⚠️
> WARNING
> ⚠️⚠️⚠️
>
> This crate is currently in alpha state as I start integrating it with some in-development game projects. I will utilize `semver` to the best of my ability to help guard against breaking changes. The main goal of this is to provide a robust, easy-to-use, 100% reliable rollback engine for games. As such, there may be anywhere between "zero" API changes and "complete overhauls". This readme and the version will be updated appropriately as things stabilize.
---

## P2P Rollback Networking in Rust

Fortress Rollback is a correctness-first peer-to-peer rollback networking library for deterministic multiplayer games, written in 100% safe [Rust 🦀](https://www.rust-lang.org/) (`#![forbid(unsafe_code)]`). It is engineered for reliability: zero panics in production code, determinism by construction, extensive property and fuzz testing, and formal verification (TLA+, Kani, Z3) of its trickiest invariants. The callback-style API common to rollback libraries is replaced with a simple, request-driven control flow — instead of registering callbacks, you advance the session and fulfill the list of requests it returns.

Fortress Rollback began as a fortified fork of the excellent [`ggrs`](https://github.com/gschup/ggrs) crate — itself inspired by the [GGPO](https://www.ggpo.net/) network SDK — and has since grown its own capabilities, including graceful peer drop, runtime input delay, redundant-host spectators, and hot-join. Migrating from `ggrs`? See [Migration from ggrs](#migration-from-ggrs) below.

Questions, bug reports, or feature ideas are welcome on [GitHub Issues](https://github.com/wallstop/fortress-rollback/issues).

## Interactive Examples

Fortress Rollback includes interactive game examples built with [macroquad](https://github.com/not-fl3/macroquad). Run them locally to see rollback networking in action:

```shell
# P2P session (run in two terminals with different ports)
cargo run --example ex_game_p2p --features graphical-examples -- --local-port 7000 --players localhost 127.0.0.1:7001
cargo run --example ex_game_p2p --features graphical-examples -- --local-port 7001 --players 127.0.0.1:7000 localhost

# Sync test (determinism verification)
cargo run --example ex_game_synctest --features graphical-examples -- --num-players 2 --check-distance 7
```

See the [examples README](./examples/README.md) for system dependencies and more options.

## Getting Started

To get started with Fortress Rollback, check out the following resources:

- 📖 **[User Guide](https://wallstop.github.io/fortress-rollback/)** — Full documentation site with guides, architecture, and API reference
- 📚 **[GitHub Wiki](https://github.com/wallstop/fortress-rollback/wiki)** — Quick reference and community-editable docs
- 💻 **[Examples](./examples/)** — Working code examples for common use cases
- 🎮 **[Request Handling Example](./examples/request_handling.rs)** — How to handle game loop requests with manual matching or the `handle_requests!` macro
- 📋 **[API Documentation](https://docs.rs/fortress-rollback/latest/fortress_rollback/)** — Auto-generated Rust docs on docs.rs

### System Dependencies for Examples

The interactive examples use [macroquad](https://github.com/not-fl3/macroquad), which requires system libraries:

**Linux (Debian/Ubuntu):**

```shell
sudo apt-get install libasound2-dev libx11-dev libxi-dev libgl1-mesa-dev
```

**Linux (Fedora/RHEL):**

```shell
sudo dnf install alsa-lib-devel libX11-devel libXi-devel mesa-libGL-devel
```

**macOS/Windows:** No additional dependencies required.

## Development Status

Alpha / experimental only.

### Key Improvements in Fortress Rollback

- **100% Deterministic**: All collections use `BTreeMap`/`BTreeSet` for guaranteed iteration order; new `hash` module provides FNV-1a deterministic hashing
- **Panic-Free API**: All public APIs return `Result` types instead of panicking — no unexpected crashes
- **Correctness-First**: Formally verified with TLA+ and Z3 proofs; ~1600 tests (~92% coverage) including multi-process network and resilience scenarios
- **Enhanced Desync Detection**: Built-in checksum validation with `P2PSession::confirmed_inputs_for_frame()` for debugging state divergence
- **`handle_requests!` Macro**: Eliminates boilerplate in the game loop — see [User Guide](./docs/user-guide.md#using-the-handle_requests-macro)
- **Config Presets**: `SyncConfig::lan()`, `ProtocolConfig::mobile()`, etc. for common network conditions
- **Player Handle Convenience Methods**: Easy access to local/remote handles for 1v1 games, player type checking, and iteration — see [User Guide](./docs/user-guide.md#player-handle-convenience-methods)
- **Unified Session Trait**: Write generic game loops via `Session<T>` — works with P2P, spectator, and sync test sessions — see [User Guide](./docs/user-guide.md#using-the-session-trait)
- **Runtime Input Delay**: Adjust per-player input delay mid-session via `P2PSession::set_input_delay` for hybrid delay+rollback netcode — see [User Guide](./docs/user-guide.md#adjusting-input-delay-at-runtime)
- **Graceful Peer Drop**: Configure `DisconnectBehavior::ContinueWithout` so the session keeps advancing for surviving peers, or remove a peer explicitly with `P2PSession::remove_player` (kick / surrender / leave-match) — see [User Guide](./docs/user-guide.md#disconnect-behavior-and-graceful-peer-drop)

📋 **[Complete comparison with GGRS →](./docs/fortress-vs-ggrs.md)** — See all differences, bug fixes, and migration steps

- [Changelog](./CHANGELOG.md)
- [Issues](https://github.com/wallstop/fortress-rollback/issues)
- [Contribution Guide](./docs/contributing.md)

### Network Requirements

| Condition | Supported | Optimal |
|-----------|-----------|---------|
| RTT | <200ms | <100ms |
| Packet Loss | <15% | <5% |
| Jitter | <50ms | <20ms |

The deterministic simulation fleet validates full-mesh correctness and liveness through `N=16`
under its documented profiles. That is not a production endorsement of a 16-player mesh: full
meshes above eight players exceed attested industry practice. Measure your own game-state cost,
input width, topology, bandwidth, stalls, and target hardware before choosing a supported tier.

For detailed configuration guidance, see the [User Guide](./docs/user-guide.md#network-requirements).

### Feature Flags

| Feature | Description |
|---------|-------------|
| `sync-send` | Adds `Send + Sync` bounds for multi-threaded game engines (e.g., Bevy) |
| `hot-join` | Allows a peer to join/rejoin a running session by filling a reserved or gracefully-dropped slot via a state snapshot (requires `Config::State: Serialize + DeserializeOwned`; player count stays fixed) |
| `tokio` | Enables `TokioUdpSocket` for async Tokio applications |
| `paranoid` | Runtime invariant checking in release builds |
| `graphical-examples` | Enables ex_game graphical examples (requires macroquad deps) |
| `loom` | Loom-compatible synchronization primitives for concurrency testing |
| `json` | JSON serialization for telemetry types (`to_json()` methods) |
| `z3-verification` | Enables Z3 SMT solver proofs (development/CI only — requires Z3 installed) |
| `z3-verification-bundled` | Like `z3-verification` but builds Z3 from source (slow, no system Z3 needed) |

For detailed feature documentation, see the [User Guide](./docs/user-guide.md#feature-flags).

## Migration from ggrs

Moving from the original `ggrs` crate? See the step-by-step guide in [migration.md](./docs/migration.md). It covers the crate rename (`fortress-rollback`), the new `Config::Address` `Ord` bound, and import changes (`fortress_rollback`).

### Web / WASM Support

Fortress Rollback's core logic needs no WASM feature flag, but the two Web/WASM integration paths covered here have different requirements:

- Browser `wasm32-unknown-unknown` uses `web_time`'s browser clock. Its dependency graph may legitimately include `wasm-bindgen`, `js-sys`, and `web-sys` through browser-only timing or transport crates.
- Godot Web GDExtensions use `wasm32-unknown-emscripten`. Their normal dependency graph must contain none of those JavaScript bridge crates because Godot's Web export loads the Rust output as an Emscripten side module, not through wasm-bindgen.

Native, browser, and Emscripten builds all measure protocol intervals and quality-report RTT with a local monotonic `web_time::Instant`; no shared epoch timestamp is required.

For browser networking, [Matchbox](https://github.com/johanhelsing/matchbox) 0.14 provides WebRTC data channels. Keep it behind the browser-only target gate so it cannot enter a Godot Web Emscripten build:

```toml
[target.'cfg(all(target_arch = "wasm32", target_os = "unknown"))'.dependencies]
matchbox_socket = "0.14"
```

Matchbox's optional `ggrs` feature implements the upstream GGRS trait only, not Fortress Rollback's `NonBlockingSocket`. Leave that feature disabled and wrap a raw Matchbox channel in a local adapter that uses `fortress_rollback::network::codec` to encode and decode messages. Split the channel and poll a fixed maximum from its receiver per call; `WebRtcChannel::receive()` drains the entire queue. The [custom socket example](./examples/custom_socket.rs) shows the generic codec and bounded-socket pattern such an adapter should follow.

Matchbox handles:

- **WebRTC peer-to-peer connections** — direct data channels between browsers
- **Signaling server** — connection establishment (only needed during setup)
- **Low-latency unreliable channels** — unordered delivery with no retransmits, matching rollback traffic; reserve reliable channels for separate application data

See the [custom socket example](./examples/custom_socket.rs) for implementing your own transport (WebSockets, custom protocols, etc.)

## Licensing

Fortress Rollback is dual-licensed under either

- [MIT License](./LICENSE-MIT): Also available [online](http://opensource.org/licenses/MIT)
- [Apache License, Version 2.0](./LICENSE-APACHE): Also available [online](http://www.apache.org/licenses/LICENSE-2.0)

at your option.
