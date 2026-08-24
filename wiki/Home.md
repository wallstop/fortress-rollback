<!-- SYNC: This wiki page is generated from docs/index.md. Edit docs source. -->

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

[Build your first session](Getting-Started)
[Browse maintained examples](https://github.com/wallstop/fortress-rollback/tree/main/examples)

## Start here

| What you want to do | Read |
|---------------------|------|
| Install the crate and run one deterministic loop | [Getting Started](Getting-Started) |
| Integrate inputs, requests, events, and session types | [User Guide](User-Guide) |
| Choose latency, prediction, and input-delay settings | [Network Tuning](Tuning) |
| Validate a game before release | [Production Checklist](Production-Checklist) |
| Look up a type or method | [API documentation](https://docs.rs/fortress-rollback) |

## Common paths

- **Connect real peers:** finish Getting Started, then use the
  [P2P session guide](User-Guide#setting-up-a-p2p-session) and
  [configuration example](https://github.com/wallstop/fortress-rollback/blob/main/examples/configuration.rs).
- **Handle rollback requests:** read [The Game Loop](User-Guide#the-game-loop) and the
  [request-handling example](https://github.com/wallstop/fortress-rollback/blob/main/examples/request_handling.rs).
- **Use another transport or platform:** see
  [Custom Sockets](User-Guide#custom-sockets) and
  [Feature Flags](User-Guide#feature-flags).
- **Add spectators or hot-join:** start with [Spectator Sessions](User-Guide#spectator-sessions)
  or the [hot-join feature reference](User-Guide#feature-flag-reference).
- **Migrate from GGRS:** follow the [Migration Guide](Migration) and compare behavior in
  [Fortress vs GGRS](Fortress-vs-GGRS).

## Advanced reference

The [architecture](Architecture) explains internal data flow and protocol design. The
[determinism model](Determinism-Model), [API contracts](API-Contracts), and
[threat model](Threat-Model) define the guarantees and caller responsibilities. Operational
guides cover [replay](Replay), [telemetry](Telemetry),
[desync response](Desync-Playbook), and [disconnect/rejoin response](Reconnect-Playbook).

> [!NOTE]
> Fortress Rollback is alpha software. Review the [changelog](Changelog), test upgrades against
> your game, and use the [production checklist](Production-Checklist) before release.
