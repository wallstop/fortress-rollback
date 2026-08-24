<p align="center">
  <img src="https://raw.githubusercontent.com/wallstop/fortress-rollback/main/docs/assets/logo-banner.svg" alt="Fortress Rollback" width="600">
</p>

<p align="center">
  <a href="https://crates.io/crates/fortress-rollback"><img src="https://img.shields.io/crates/v/fortress-rollback?style=for-the-badge" alt="crates.io"></a>
  <a href="https://wallstop.github.io/fortress-rollback/"><img src="https://img.shields.io/badge/docs-GitHub%20Pages-blue?style=for-the-badge" alt="Documentation"></a>
  <a href="https://github.com/wallstop/fortress-rollback/actions/workflows/ci-rust.yml"><img src="https://img.shields.io/github/actions/workflow/status/wallstop/fortress-rollback/ci-rust.yml?branch=main&style=for-the-badge&label=CI" alt="CI"></a>
</p>

# Fortress Rollback

Fortress Rollback is a correctness-first peer-to-peer rollback networking library for
deterministic multiplayer games. It uses 100% safe Rust (`#![forbid(unsafe_code)]`), returns
recoverable errors instead of panicking in production code, and exposes a request-driven game
loop: advance the session, then fulfill the ordered save, load, and simulation requests it returns.

The project began as a hardened fork of [GGRS](https://github.com/gschup/ggrs) and now includes
graceful peer drop, runtime input delay, redundant-host spectators, and opt-in hot-join.

> [!WARNING]
> Fortress Rollback is alpha software. Expect API changes before 1.0 and test upgrades against
> your game before release. The project follows semantic versioning to the extent possible.

## Install and Try It

Add the library and Serde derives to your game:

```toml
[dependencies]
fortress-rollback = "0.12"
serde = { version = "1", features = ["derive"] }
```

To see a complete deterministic loop first, clone this repository and run:

```bash
cargo run --example sync_test
```

Then follow [Build Your First Session](https://wallstop.github.io/fortress-rollback/getting-started/). It starts without networking so
you can prove that your simulation survives rollback before debugging transport or peer setup.

## The Integration Loop

Every game follows the same four steps:

1. Define compact, fixed-width input and deterministic game-state types.
2. Build a `SyncTestSession` during development or a `P2PSession` for connected peers.
3. Poll the session, submit every local player's input, and call `advance_frame`.
4. Fulfill every returned `FortressRequest` in order: save, load, or advance game state.

The [first-session guide](https://wallstop.github.io/fortress-rollback/getting-started/) shows this
loop as a runnable program. The [request-handling example](https://github.com/wallstop/fortress-rollback/blob/main/examples/request_handling.rs)
shows the manual and macro forms.

## Choose the Right Guide

| Goal | Canonical page |
|------|----------------|
| Build a first deterministic session | [Getting Started](https://wallstop.github.io/fortress-rollback/getting-started/) |
| Integrate sessions, requests, and events | [User Guide](https://wallstop.github.io/fortress-rollback/user-guide/) |
| Run maintained examples | [Examples](https://github.com/wallstop/fortress-rollback/tree/main/examples) |
| Tune latency, prediction, and input delay | [Network Tuning](https://wallstop.github.io/fortress-rollback/tuning/) |
| Add browser, engine, or custom transport support | [Platforms and custom sockets](https://wallstop.github.io/fortress-rollback/user-guide/#custom-sockets) |
| Prepare a game for release | [Production Checklist](https://wallstop.github.io/fortress-rollback/production-checklist/) |
| Understand the public API | [docs.rs](https://docs.rs/fortress-rollback/latest/fortress_rollback/) |
| Migrate from GGRS | [Migration Guide](https://wallstop.github.io/fortress-rollback/migration/) |

For protocol internals and guarantees, see the
[architecture](https://wallstop.github.io/fortress-rollback/architecture/),
[determinism model](https://wallstop.github.io/fortress-rollback/specs/determinism-model/), and
[threat model](https://wallstop.github.io/fortress-rollback/threat-model/).

## Network Requirements

The deterministic simulation fleet validates full-mesh correctness and liveness through `N=16`
under documented profiles. This result is not a production endorsement: full meshes above eight
players exceed attested industry practice. Measure your game-state cost, input width, topology,
bandwidth, stalls, and target hardware before choosing a supported tier. Start with the measured
profiles in [Network Tuning](https://wallstop.github.io/fortress-rollback/tuning/).

## Project Notes

Questions and bug reports belong in [GitHub Issues](https://github.com/wallstop/fortress-rollback/issues).
The project uses substantial AI assistance for code, documentation, tests, and formal
specifications, with human review for architecture and final approval.

Fortress Rollback is dual-licensed under [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option.
