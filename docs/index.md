---
title: Fortress Rollback
description: A correctness-first peer-to-peer rollback networking library for deterministic multiplayer games in Rust.
---

<!-- SYNC: This source doc syncs to wiki/Home.md. -->

<p align="center">
  <img src="assets/logo.svg" alt="Fortress Rollback" width="160">
</p>

# Fortress Rollback

## Deterministic rollback netcode built on correctness

Fortress Rollback is a safe Rust library for peer-to-peer rollback networking. Its request-driven
API tells your game when to save state, load state, and simulate a frame. The library handles input
prediction, rollback, synchronization, and transport protocol details.

New to rollback netcode? Start locally. A `SyncTestSession` exercises the same save/load/advance
contract without adding network setup, so determinism mistakes surface before connection problems.

[Build your first session](getting-started.md){ .md-button .md-button--primary }
[Browse maintained examples](https://github.com/wallstop/fortress-rollback/tree/main/examples){ .md-button }

## Start here

| What you want to do | Read |
|---------------------|------|
| Install the crate and run one deterministic loop | [Getting Started](getting-started.md) |
| Integrate inputs, requests, events, and session types | [User Guide](user-guide.md) |
| Choose latency, prediction, and input-delay settings | [Network Tuning](tuning.md) |
| Validate a game before release | [Production Checklist](production-checklist.md) |
| Look up a type or method | [API documentation](https://docs.rs/fortress-rollback) |

## Common paths

- **Connect real peers:** finish Getting Started, then use the
  [P2P session guide](user-guide.md#setting-up-a-p2p-session) and
  [configuration example](https://github.com/wallstop/fortress-rollback/blob/main/examples/configuration.rs).
- **Handle rollback requests:** read [The Game Loop](user-guide.md#the-game-loop) and the
  [request-handling example](https://github.com/wallstop/fortress-rollback/blob/main/examples/request_handling.rs).
- **Use another transport or platform:** see
  [Custom Sockets](user-guide.md#custom-sockets) and
  [Feature Flags](user-guide.md#feature-flags).
- **Add spectators or hot-join:** start with [Spectator Sessions](user-guide.md#spectator-sessions)
  or the [hot-join feature reference](user-guide.md#feature-flag-reference).
- **Migrate from GGRS:** follow the [Migration Guide](migration.md) and compare behavior in
  [Fortress vs GGRS](fortress-vs-ggrs.md).

## Advanced reference

The [architecture](architecture.md) explains internal data flow and protocol design. The
[determinism model](specs/determinism-model.md), [API contracts](specs/api-contracts.md), and
[threat model](threat-model.md) define the guarantees and caller responsibilities. Operational
guides cover [replay](replay.md), [telemetry](telemetry.md),
[desync response](desync-playbook.md), and [disconnect/rejoin response](reconnect-playbook.md).

> [!NOTE]
> Fortress Rollback is alpha software. Review the [changelog](changelog.md), test upgrades against
> your game, and use the [production checklist](production-checklist.md) before release.
