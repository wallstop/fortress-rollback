<!-- SYNC: This source doc syncs to wiki/Production-Checklist.md. -->

# Production Checklist

Use this checklist before shipping a networked build and repeat it whenever the simulation,
transport, or session configuration changes.

## Determinism

- Run representative matches through `SyncTestSession` in CI, including rollback-heavy inputs.
- Keep simulation inputs fixed-width and deterministic; never derive simulation state from wall
  time, unordered collection iteration, unseeded randomness, rendering, or network arrival order.
- Audit enabled dependency features with `cargo tree -e features`. In particular, investigate
  `ahash` or `const-random` in code that can influence simulation order or state.
- Test every supported target. Floating-point and platform APIs can differ even when native tests
  agree.

See the [determinism model](specs/determinism-model.md) for the complete contract.

## Detection and diagnostics

- Leave desync detection enabled. The default interval of 60 checks once per second at 60 FPS;
  shorter intervals detect faults sooner but add checksum traffic and work.
- Install a `ViolationObserver` and export every `Error` or `Critical` violation. Treat either as
  a degraded session that needs investigation.
- Poll `SessionMetrics` and each remote's `PeerMetrics`. Alert on checksum mismatches, discarded
  events, repeated stalls, rising confirmation lag, large pending-output queues, MTU-risk
  messages, and unknown-source packets.
- Drain `events()` after every network poll. A non-zero `events_discarded_total` proves the
  application lost at least one notification.
- Enable replay recording for diagnosable builds and persist the replay with the build ID,
  session configuration, player/address mapping, and metrics snapshot.

The [telemetry guide](telemetry.md) explains the counters and the
[desync playbook](desync-playbook.md) covers incident capture.

## Network and lifecycle policy

- Configure the same player count, input width, FPS, desync interval, save mode, prediction
  window, and input delay on every peer. Protocol v2 rejects incompatible peers during sync.
- Pick a disconnect policy deliberately. `Halt` ends progress after a drop;
  `ContinueWithout` coordinates a graceful removal and lets survivors continue. Configure the
  same policy on every peer.
- Set disconnect notification and timeout values from measured packet-loss bursts, not only
  average RTT. Test them under the worst supported network preset.
- Handle `NetworkInterrupted`, `NetworkResumed`, `PeerDropped`, and `Disconnected` as distinct
  lifecycle signals. See the [reconnect playbook](reconnect-playbook.md).
- Treat `WaitRecommendation` as bounded backpressure: skip the requested number of application
  simulation opportunities while continuing to poll the network and drain events.

## Capacity and frame lifetime

- Size the event queue, input queue, checksum history, and prediction window from measured load.
  Confirm their high-water marks plateau during a release soak.
- Keep encoded input packets below the path budget. `PeerMetrics` separately counts messages at
  or above the portable 1,200-byte warning and common 1,472-byte IPv4 fragmentation threshold.
- `Frame` uses a signed 32-bit counter. At 60 FPS, `i32::MAX` is about 414 days (1.14 years) from
  frame zero. End or migrate a continuously running session well before that boundary; checked
  arithmetic reports overflow, but rollover is not a supported operating mode.

## WASM targets

- Browser `wasm32-unknown-unknown` may use `wasm-bindgen`, `js-sys`, and `web-sys` for its clock
  or transport. Keep those dependencies behind the full browser target predicate.
- Godot Web `wasm32-unknown-emscripten` must not inherit browser JavaScript bridge crates. Use an
  application/Godot `NonBlockingSocket` adapter and the Emscripten monotonic clock.
- Browsers can throttle or suspend background tabs. Treat a suspension as a network interruption and
  test resume, timeout, and graceful-drop behavior explicitly.

## Release evidence

- Run the repository's full local validation command from `.llm/context.md`.
- Run the deterministic baseline sweep and compare bandwidth, rollback depth, confirmation lag,
  and stalls against the checked-in ledger.
- Run release-mode nightly chaos/soak lanes on every supported OS and keep their artifacts.
- Verify that every changelog behavior claim points to a regression test or measured artifact.
