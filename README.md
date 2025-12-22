<p align="center">
  <img src="assets/logo-banner.svg" alt="Fortress Rollback" width="600">
</p>

<p align="center">
  <a href="https://crates.io/crates/fortress-rollback"><img src="https://img.shields.io/crates/v/fortress-rollback?style=for-the-badge" alt="crates.io"></a>
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

> crates.io publication is being prepared. Until the first release is live, depend on the git repository: `fortress-rollback = { git = "https://github.com/wallstop/fortress-rollback", branch = "main" }`.

If you are interested in integrating rollback networking into your game or just want to chat with other rollback developers (not limited to Rust), check out the [GGPO Developers Discord](https://discord.com/invite/8FKKhCRCCE)!

## Live Demonstrations

Fortress Rollback currently ships with the same demos you may know from GGRS. One is written with [macroquad](https://github.com/not-fl3/macroquad), the other with [bevy](https://bevyengine.org/). Both use [matchbox](https://github.com/johanhelsing/matchbox). Try it out with a friend! Just click the link and match with another player! (You can also open the link in two separate windows to play against yourself)

## Getting Started

To get started with Fortress Rollback, check out the following resources:

- [Examples](./examples/)
- [Documentation](https://docs.rs/fortress-rollback/newest/fortress_rollback/)

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
- **Correctness-First**: 1100+ tests including multi-process network tests, network resilience tests, and formal TLA+ specifications
- **Panic-Free API**: All public APIs return `Result` types instead of panicking
- **Enhanced Testing**: Full desync detection with confirmed input checksums via `P2PSession::confirmed_inputs_for_frame()`

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
| `z3-verification` | Z3 formal verification tests (requires system Z3) |

For detailed feature documentation, see the [User Guide](./docs/user-guide.md#feature-flags).

## Migration from ggrs

Moving from the original `ggrs` crate? See the step-by-step guide in [migration.md](./docs/migration.md). It covers the crate rename (`fortress-rollback`), the new `Config::Address` `Ord` bound, and import changes (`fortress_rollback`).

### Web / WASM Support

Fortress Rollback works in the browser! WASM support is **automatic** — no feature flags needed. The library detects `target_arch = "wasm32"` at compile time and uses browser-compatible APIs (`js-sys` for time).

For networking in the browser, use **[Matchbox](https://github.com/johanhelsing/matchbox)** — it provides WebRTC sockets that implement `NonBlockingSocket` and work seamlessly with Fortress Rollback:

```toml
[dependencies]
fortress-rollback = "0.1"
matchbox_socket = { version = "0.13", features = ["ggrs"] }
```

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
