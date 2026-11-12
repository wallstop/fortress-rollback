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
  agree. Replay the checked-in portable fixture
  (`tests/replay-fixture/fixtures/portable-replay.bin`) on native, browser
  `wasm32-unknown-unknown`, and Godot `wasm32-unknown-emscripten`; all three lanes must report the
  same 4,096-frame count, request count, ordered request-stream digest, and state-checksum
  digest with zero replay desyncs.

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

## Semi-trusted peer policy

- Decide before launch whether a dishonest participant is in scope. If it is, authenticate the
  transport identity; protocol connection IDs and configuration digests are not authenticators.
- Define an equivocation policy: quarantine or void at the first proven divergent frame, preserve
  per-recipient input/packet evidence, and never claim checksum-only attribution.
- Define a checksum-accusation policy: use `peer_checksum_mismatch_count` as an advisory persistence
  signal, corroborate with other peers and deterministic replays, and never auto-eject from one
  peer's report alone.
- Put flood controls before the socket: rate-limit unauthenticated traffic, monitor kernel drops
  and unknown-source/cap/queue telemetry, and keep polling and draining events. Do not treat larger
  receive or queue limits as DDoS protection. Compare `SessionMetrics::socket` accepted
  sends/failures and cap saturations against protocol enqueue demand in `PeerMetrics`; the socket
  counters describe adapter acceptance, not network delivery.
- With `hot-join`, authenticate and record the coordinator that serves each snapshot. Capture the
  snapshot frame, the optional peer-supplied checksum, and an application-computed loaded-state
  fingerprint when available. These preserve evidence but do not prove provenance; abort and
  choose a separately trusted source when provenance is uncertain. Fortress does not obtain a
  second-survivor attestation.

The [threat model](threat-model.md) records the complete single-dishonest-peer capability matrix.

## Network and lifecycle policy

- Configure the same player count, input width, FPS, desync interval, save mode, prediction
  window, and input delay on every peer. Protocol v2 rejects incompatible peers during sync.
- Pick a disconnect policy deliberately. `Halt` ends progress after a drop;
  `ContinueWithout` coordinates a graceful removal when another live endpoint can witness the
  certificate and a strict majority of the agreed membership remains reachable; automatic
  timeouts without both fail closed. Configure the same policy on every peer.
- Set disconnect notification and timeout values from measured packet-loss bursts, not only
  average RTT. Test them under the worst supported network preset.
- Handle `NetworkInterrupted`, `NetworkResumed`, `PeerDropped`, and `Disconnected` as distinct
  lifecycle signals. See the [reconnect playbook](reconnect-playbook.md).
- Treat `WaitRecommendation` as bounded backpressure: skip the requested number of application
  simulation opportunities while continuing to poll the network and drain events.

## Application misuse surface

- **U1 — ignored pacing:** handle every `WaitRecommendation` as bounded simulation backpressure.
  Smear the skipped opportunities when appropriate, but keep polling the network and draining
  events while the simulation slows.
- **U2 — slow event draining:** drain `events()` after every network poll. Alert on the event-queue
  high-water mark and every per-kind discard counter; the retained advisory history may be
  incomplete after any discard.
- **U3 — log-and-continue desync:** treat `DesyncDetected` as a reason to stop or quarantine the
  affected match and preserve replay, input, checksum, build, and authenticated-packet evidence.
  A checksum disagreement proves divergence, not state correctness or culprit identity; never
  auto-eject solely from one peer's accusation.
- **U4 — unfaithful sparse saves:** when using `SaveMode::Sparse`, exercise a save/load round-trip
  in CI. Restore the recorded frame and assert canonical game state and checksum equality before
  trusting later replay or desync evidence.
- **U5 — inverted pacing direction:** a positive `frames_ahead()` means the local session is
  estimated ahead and should slow; a negative value means it is estimated behind. The estimate is
  advisory and assumes symmetric one-way delay, so do not turn a negative value into unbounded
  catch-up.
- **U6 — variable-width input:** require every input value to serialize to the same non-zero width
  as `Config::Input::default()`. Fortress rejects the transmission and reports an Error violation
  at send time, after a session may already be running; exercise representative values before
  shipping.
- **U7 — incomplete couch-co-op input:** call `add_local_input` for every local handle before each
  `advance_frame`. `MissingLocalInput` currently does not name the omitted handles, so compare and
  log `local_player_handles()` when diagnosing it.

Runnable counterexamples live in `tests/sessions/misuse.rs` (missing rollback save, checksum
serialization failure, omitted local input) and `tests/sessions/macro_tests.rs` (fallible macro
stops before later requests). Event-starvation, variable-width input, mismatched-configuration, and
NAT-rebinding failures have their own deterministic tests in the simulation, protocol,
compatibility, and census suites. Canonical examples use `try_handle_requests!` when they must
abort on the first request error.

## Capacity and frame lifetime

- Size the event queue, input queue, checksum history, and prediction window from measured load.
  Confirm their high-water marks plateau during a release soak.
- For custom asynchronous sockets, override `socket_metrics` to report adapter queue depth and
  oldest-message age, then compare `PeerMetrics::packets_sent` protocol-queue deltas with adapter
  admission/service rate. A normal session update can submit multiple packets; requiring the
  transport buffer to empty after each packet is an unstable stop-and-wait design.
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
- Keep a browser-run session oracle for raw custom transports: exercise handshake, confirmed frame
  exchange, malformed packet rejection, and the adapter's receive-poll cap. Keep its
  `wasm-bindgen` test dependencies behind the full browser target predicate so the Emscripten graph
  stays separate.
- Browsers can throttle or suspend background tabs. Treat a suspension as a network interruption and
  test resume, timeout, and graceful-drop behavior explicitly. The browser runtime lane injects a
  virtual clock, then verifies `NetworkInterrupted`/`NetworkResumed` and continued confirmed play
  after a sub-timeout suspension, and a fail-closed `Disconnected` after a suspension longer than
  the disconnect timeout.

## Release evidence

- Run the repository's full local validation command from `.agents/skills/fortress-development/SKILL.md`.
- Record the supported player tier honestly. The deterministic simulation fleet validates
  full-mesh correctness and liveness through `N=16` under documented profiles, but that is not a
  production endorsement. Full meshes above eight players exceed attested industry practice;
  measure your own game, topology, input width, state cost, bandwidth, stalls, and hardware.
- Run the deterministic baseline sweep and compare encoded byte/message demand, rollback depth,
  confirmation lag, and stalls against the checked-in ledger. The strict-build CI lane also replays
  the checked-in corpus under `-C overflow-checks=on -C debug-assertions=on`, so release-safety
  profiles must preserve artifact and trace identity. The corpus includes a mixed-transport schedule
  whose unreliable and reliable-retransmission links must still agree on every confirmed prefix, and
  an extended live schedule that confirms past 12,000 frames at the default prediction window under
  mixed reliable/unreliable links, repeatedly recycling the input-queue and saved-state rings under
  holds, blocks, stalls, and runtime input-delay increases. Replaying the extended schedule must
  reproduce the identical trace hash and confirmed prefixes.
- Run release-mode nightly chaos/soak lanes on every supported OS and keep their artifacts.
- Verify that every changelog behavior claim points to a regression test or measured artifact.
