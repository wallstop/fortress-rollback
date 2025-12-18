<p align="center">
  <img src="assets/logo-banner.svg" alt="Fortress Rollback" width="600">
</p>

<p align="center">
  <a href="https://crates.io/crates/fortress-rollback"><img src="https://img.shields.io/crates/v/fortress-rollback?style=for-the-badge" alt="crates.io"></a>
  <a href="https://github.com/wallstop/fortress-rollback/actions/workflows/rust.yml"><img src="https://img.shields.io/github/actions/workflow/status/wallstop/fortress-rollback/rust.yml?branch=main&style=for-the-badge&label=CI" alt="CI"></a>
  <a href="https://github.com/wallstop/fortress-rollback/actions/workflows/publish.yml"><img src="https://img.shields.io/github/actions/workflow/status/wallstop/fortress-rollback/publish.yml?branch=main&style=for-the-badge&label=Publish" alt="Publish"></a>
  <a href="https://github.com/wallstop/fortress-rollback/actions/workflows/coverage.yml"><img src="https://img.shields.io/github/actions/workflow/status/wallstop/fortress-rollback/coverage.yml?branch=main&style=for-the-badge&label=Coverage" alt="Coverage"></a>
  <a href="https://github.com/wallstop/fortress-rollback/actions/workflows/benchmark.yml"><img src="https://img.shields.io/github/actions/workflow/status/wallstop/fortress-rollback/benchmark.yml?branch=main&style=for-the-badge&label=Benchmark" alt="Benchmark"></a>
</p>

---

> **🤖 AI-Assisted Development Notice**
>
> This project was developed with **substantial AI assistance**. The vast majority of the code, documentation, tests, and formal specifications were written with the help of **Claude Opus 4.5** and **Codex 5.1**. Human oversight was provided for code review, architectural decisions, and final approval, but the implementation work was heavily AI-driven. This transparency is provided so users can make informed decisions about using this crate.

---

## P2P Rollback Networking in Rust

Fortress Rollback (a fork of GGRS, the good game rollback system) is a fortified, correctness-first port of the original [ggrs crate](https://github.com/gschup/ggrs) and reimagination of the [GGPO network SDK](https://www.ggpo.net/), written in 100% safe [Rust 🦀](https://www.rust-lang.org/). The callback-style API from the original library has been replaced with a simple, request-driven control flow: instead of registering callback functions, Fortress Rollback returns a list of requests for the user to fulfill.

> crates.io publication is being prepared. Until the first release is live, depend on the git repository: `fortress-rollback = { git = "https://github.com/wallstop/fortress-rollback", branch = "main" }`.

If you are interested in integrating rollback networking into your game or just want to chat with other rollback developers (not limited to Rust), check out the [GGPO Developers Discord](https://discord.com/invite/8FKKhCRCCE)!

## Live Demonstrations

Fortress Rollback currently ships with the same demos you may know from GGRS. One is written with [macroquad](https://github.com/not-fl3/macroquad), the other with [bevy](https://bevyengine.org/). Both use [matchbox](https://github.com/johanhelsing/matchbox). Try it out with a friend! Just click the link and match with another player! (You can also open the link in two separate windows to play against yourself)

🚧 MATCHMAKING CURRENTLY OFFLINE! 🚧

- [Bevy Demo](https://gschup.github.io/bevy_ggrs_demo/) ([Repository](https://github.com/gschup/bevy_ggrs_demo))
- [Macroquad Demo](https://gschup.github.io/ggrs_demo/) ([Repository](https://github.com/gschup/ggrs_demo))

## Getting Started

To get started with Fortress Rollback, check out the following resources:

- [Wiki](https://github.com/wallstop/fortress-rollback/wiki)
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
- **Correctness-First**: 620+ tests including multi-process network tests, network resilience tests, and formal TLA+ specifications
- **Panic-Free API**: All public APIs return `Result` types instead of panicking
- **Enhanced Testing**: Full desync detection with confirmed input checksums via `P2PSession::confirmed_inputs_for_frame()`

📋 **[Complete comparison with GGRS →](./docs/fortress-vs-ggrs.md)** — See all differences, bug fixes, and migration steps

- [Changelog](./docs/changelog.md)
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
| `wasm-bindgen` | WASM compatibility for browser games |
| `paranoid` | Runtime invariant checking in release builds |
| `graphical-examples` | Enables ex_game graphical examples (requires macroquad deps) |
| `z3-verification` | Z3 formal verification tests (requires system Z3) |

For detailed feature documentation, see the [User Guide](./docs/user-guide.md#feature-flags).

## Migration from ggrs

Moving from the original `ggrs` crate? See the step-by-step guide in [migration.md](./docs/migration.md). It covers the crate rename (`fortress-rollback`), the new `Config::Address` `Ord` bound, and import changes (`fortress_rollback`).

## Useful Links

### Bevy Plugin

The authors of the original GGRS are maintaining a [bevy](https://bevyengine.org/) plugin. Check it out:

- [Bevy GGRS](https://github.com/gschup/bevy_ggrs)

### WASM support through WebRTC sockets

If you are interested to run a GGRS application in your browser, check the amazing Matchbox project!
The matchbox sockets are compatible with GGRS through a feature flag:

- [Matchbox](https://github.com/johanhelsing/matchbox)

### Godot Wrapper

[Godot](https://godotengine.org/) is a popular open-source game engine. marcello505 is developing a wrapper for GGRS/Fortress Rollback.
Find the repository here:

- [Godot GGRS Wrapper](https://github.com/marcello505/godot-ggrs-wrapper)

### Other Rollback Implementations in Rust

Also take a look at the awesome backroll project, a completely async rollback library. Special thanks to james7132 for a lot of inspiration and fruitful discussion.

- [backroll-rs](https://github.com/HouraiTeahouse/backroll-rs/)

## Licensing

Fortress Rollback is dual-licensed under either

- [MIT License](./LICENSE-MIT): Also available [online](http://opensource.org/licenses/MIT)
- [Apache License, Version 2.0](./LICENSE-APACHE): Also available [online](http://www.apache.org/licenses/LICENSE-2.0)

at your option.
