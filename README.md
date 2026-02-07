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

Fortress Rollback is a fortified, correctness-first port of the original, phenomenal [ggrs crate](https://github.com/gschup/ggrs) and reimagination of the [GGPO network SDK](https://www.ggpo.net/), written in 100% safe [Rust 🦀](https://www.rust-lang.org/). This crate was primarily developed with AI assistance. The callback-style API from the original library has been replaced with a simple, request-driven control flow: instead of registering callback functions, Fortress Rollback returns a list of requests for the user to fulfill.

If you are interested in integrating rollback networking into your game or just want to chat with other rollback developers (not limited to Rust), check out the [GGPO Developers Discord](https://discord.com/invite/8FKKhCRCCE)!

## Interactive Examples

Fortress Rollback includes interactive game examples built with [macroquad](https://github.com/not-fl3/macroquad). Run them locally to see rollback networking in action:

```shell
# P2P session (run in two terminals with different ports)
cargo run --example ex_game_p2p --features graphical-examples -- --local-port 7000 --remote-addr 127.0.0.1:7001
cargo run --example ex_game_p2p --features graphical-examples -- --local-port 7001 --remote-addr 127.0.0.1:7000

# Sync test (determinism verification)
cargo run --example ex_game_synctest --features graphical-examples
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

For detailed configuration guidance, see the [User Guide](./docs/user-guide.md#network-requirements).

### Feature Flags

| Feature | Description |
|---------|-------------|
| `sync-send` | Adds `Send + Sync` bounds for multi-threaded game engines (e.g., Bevy) |
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

Fortress Rollback works in the browser! WASM support is **automatic** — no feature flags needed. The library detects `target_arch = "wasm32"` at compile time and uses browser-compatible APIs (`web_time` for timing, `js_sys::Date` for epoch timestamps).

For networking in the browser, use **[Matchbox](https://github.com/johanhelsing/matchbox)** — it provides WebRTC sockets that implement `NonBlockingSocket` and work seamlessly with Fortress Rollback:

```toml
[dependencies]
fortress-rollback = "0.5"
matchbox_socket = { version = "0.13", features = ["ggrs"] }
```

> **Note:** The `ggrs` feature of `matchbox_socket` implements the **original GGRS crate's** `NonBlockingSocket` trait, not Fortress Rollback's. You'll need a thin adapter wrapper to bridge the two traits. See the [custom socket example](./examples/custom_socket.rs) for the recommended approach to implementing your own `NonBlockingSocket`.

Matchbox handles:

- **WebRTC peer-to-peer connections** — direct data channels between browsers
- **Signaling server** — connection establishment (only needed during setup)
- **Cross-platform** — works on native and WASM with the same API

See the [custom socket example](./examples/custom_socket.rs) for implementing your own transport (WebSockets, custom protocols, etc.)

## Licensing

Fortress Rollback is dual-licensed under either

- [MIT License](./LICENSE-MIT): Also available [online](http://opensource.org/licenses/MIT)
- [Apache License, Version 2.0](./LICENSE-APACHE): Also available [online](http://www.apache.org/licenses/LICENSE-2.0)

at your option.
