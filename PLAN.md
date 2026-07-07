# Fortress Rollback — Hardening Program Plan

> Development planning document (excluded from the published crate via `Cargo.toml` `exclude`).
> Approved by the maintainer on 2026-07-02. Branch: `dev/wallstop/hardening`.
> Goal: "rock solid and battle tested beyond belief in the most distributed of systems."

## 0. How to use this document

This is the canonical, self-contained action plan for the remaining hardening milestones. It
assumes no access to the conversation that produced it. Every cited `file:line` was verified on
the 2026-07-02 tree **with milestones M0–M1 applied**; line numbers drift, so re-locate with `rg`
before editing. Follow `.llm/context.md` for all project rules (zero-panic, bounded allocation,
clippy pedantic/nursery, changelog policy, test-output rules — never pipe test output through
`tail`/`head`; redirect to a file and read it).

**Part I** (§1–§10) is the original approved program (M0–M6). **Part II** (§11–§19, added by the
2026-07-03 distributed-systems audit) extends it: new defects D9–D12, a failure-mode taxonomy,
a falsifiable hypothesis registry, the 16-player / TCP-transport / clock-skew workstreams, and
research-backed search-power upgrades. Part II items are **integrated into Part I milestones**
via the "Part II amendments" blocks inside each milestone section — execute them together.
**Part III** (§20–§27, added by the 2026-07-03 second-pass execution session) records the first
**executed hypothesis verdicts with data** (H-16P, H-TCP, H-EVLOSS confirmed / H-DISC-RACE
falsified), the full TLA+ spec-coverage audit and the W-SPEC workstream, the reference-client
misuse audit, and a five-topic research digest that resolves several open Part II questions.

**Discipline for every task:** hypothesis → red test (quote the observed failing values) →
change → green + permanent regression artifact. Verify claims at `file:line` before acting on
them — two prior suspicions in this program were falsified by verification (see §3).

**Standard validation per task:**

```bash
cargo fmt && cargo clippy --workspace --all-targets --features tokio,json
cargo nextest run --no-capture > /tmp/test-results.txt 2>&1   # then READ the file
cargo nextest run --features hot-join --no-capture > /tmp/test-results-hj.txt 2>&1
python3 scripts/ci/agent-preflight.py --auto-fix
```

## 1. Program status

| Milestone | Status |
| --- | --- |
| Analysis (protocol/impl/client/test-infra maps, web research, defect registry) | **Done** |
| M0 — DST enablement: ping/RTT clock virtualization | **Done** (in working tree, uncommitted) |
| M1 — SimNet + simulation harness + oracle + PR smoke fleet | **Done** (in working tree, uncommitted) |
| M2 — Metrics layer + baseline sweep (fixes D1) | **Done** — §5. D1 ✅ (PR #189), D9 telemetry ✅ (#190), SessionMetrics ✅ (#191), PeerMetrics ✅ (#192), spectator peer_metrics ✅ (#193), HotJoinMetrics ✅ (#196) — **§5.2 metrics surface COMPLETE**; §5.3 sweep cell runner + PR gate ✅ (#194/#195), checked-in exact baseline `sweep-v2.json` ✅ (#197/session 80), nightly full-matrix CI wiring + N=3/8/12/16 spot rows ✅ (session 79), input-width axis {4,32}B ✅ (session 80). |
| M3 — Lifecycle vocabulary + nightly fleet + shrinker/corpus + residual & census encodings | **In progress** — §6. **§6.1 ops done:** `PeerStall` ✅ (PR #198, peer hang; schema v2; reusable mid-run confirmed probe + up-front schedule validation), `SetInputDelay` ✅ (PR #199, mid-run input-delay change; schema v3), `PeerKill` ✅ (PR #200, peer crash; schema v4; **introduced the oracle alive-mask** §6.2 liveness needs — dead peers excluded from (c-lite)/global_confirmed but pre-death states still checked; `NoLivePeers` guard; **data: ContinueWithout single-crash degrades gracefully, survivors byte-consistent**), `GracefulRemove` ✅ (session 76, schema v5; real `remove_player` on one survivor, target retired/detached, survivors byte-consistent; adversarial hardening: checksum-mismatch metrics feed the oracle under event starvation, materialized schedules fail-loud, overlapping stalls cannot shorten), `LegacyDisconnect` ✅ (session 77, schema v6; real `disconnect_player` on one survivor, target retired/detached; executable deterministic Halt/D13 probe reports post-heal non-recovery rather than graceful convergence), basic `SpectatorStart` ✅ (session 82, default-empty `spectator_hosts`, one preplanned multi-host spectator driven by the harness), `SpectatorHostKill` ✅ (session 001, schema v7; partition-only control, `dropped_blocked` premise, final spectator hosts 3→2, already-retired host fail-loud). **§6.2 oracle:** liveness (c) + metastability (i) ✅ (PR #205, bounded post-heal advance ≥ G=10 within derived B=250 steps; `Verdict::recovered_within_b`; always-on, ~25× measured margin); freeze-frame convergence (e) ✅ (session 78, applied-input/status recording with rollback last-write-wins; live `Running` survivors compare stable `Disconnected` frame/value for each retired slot); violation allowlist (f) ✅ (session 81, empty reviewed default, Critical hard-fail, validated non-overlapping prefixes, allowlist hit counts in `Verdict`, prefix-normalized pre-filter violation census in `RunReport`, ignored/manual 200-seed empty-allowlist census green with zero signatures); spectator convergence (d) ✅ (session 82, spectator input fingerprints match mesh canon, dropped-slot `Disconnected` statuses match, `SpectatorDivergence` event/error fails, planted graceful-remove schedule proves post-removal spectator progress; session 001 proves host-kill failover under partition). **Next:** `HotJoin`; witness-less spectator reactivation residual classification. |
| M4 — Fix campaign (contents determined by fleet findings) | Pending — §7 |
| M5 — Protocol v1: versioned wire + handshake config check (0.10.0, one atomic break) | Pending — §8 |
| M6 — Soak, docs, CI gates | Pending — §9 |

**Execution order (maintainer-chosen):** M2 → M3 → M4 → M5 → M6. Rationale: measure first
(M2 baselines are M5's cost ledger), maximize bug discovery (M3) before the one-time wire break
(M5) so every discovered wire-level need folds into a single version bump.

**Decisions locked by the maintainer (do not re-litigate):**

1. `SyncConfig::sync_timeout` default changes from `None` to `Some(Duration::from_secs(20))`.
2. Protocol-v1 handshake config mismatch **hard-fails on every field**.
3. Optional packet-auth is **deferred**; header `flags` bit 0 is reserved for a future auth tag.
4. Protocol v1 ships as **0.10.0** — one atomic wire break, never piecemeal.
5. Commit/push only when the maintainer asks. Suggested commit boundaries for the current tree:
   (a) M0 clock fix; (b) M1 harness + cold-start deadlock fix.

**Uncommitted working-tree inventory (M0+M1):** modified `CHANGELOG.md`, `Cargo.toml`,
`Cargo.lock`, `src/network/protocol/mod.rs`, `src/sessions/p2p_session.rs`,
`tests/common/mod.rs`, `tests/network.rs`; new `tests/common/sim_net.rs`,
`tests/network/deterministic_ping.rs`, `tests/simulation.rs`, `tests/simulation/harness/{mod,schedule,oracle}.rs`,
`tests/simulation/fleet.rs`. All suites green (2303 default / 2541 hot-join), clippy clean,
preflight passing, changelog entries written and hook-validated.

## 2. Defect registry

| # | Defect | Evidence (verified) | Fixed in |
| --- | --- | --- | --- |
| D1 | `NetworkStats.kbps_sent` accounts `std::mem::size_of_val(&msg)` — the constant in-memory enum size, not wire bytes. Bandwidth stats are fiction. | `src/network/protocol/mod.rs` `queue_message` (`bytes_sent += size_of_val`, was `:1814`), rate calc in `network_stats()` (was `:856-863`) | **M2 — §5.1 ✅** (`Message::encoded_len` + wire-exact `queue_message`; regression `codec::tests::encoded_len_matches_exact_wire_bytes`) |
| D2 | Undecodable packets silently dropped — no violation, no event; mixed-version/garbled peers spin in `Synchronizing` forever. | `src/network/socket_receive.rs:61` (`if let Ok(...) = codec::decode_message(...)`, no `Err` arm) | **M5** |
| D3 | Session-config mismatch (e.g. `num_players` 2 vs 3) passes the handshake, then every `Input` is dropped with thread-local violations only, until disconnect timeout. | `src/network/protocol/mod.rs:2039-2048`; handshake carries no config | **M5** |
| D4 | No protocol version anywhere on the wire; 0.9.0's `FloorRequest`/`FloorReply` "ride the same unversioned framing" (changelog admission, `CHANGELOG.md` 0.9.0). | `rg PROTOCOL_VERSION src/` → 0 hits | **M5** |
| D5 | `Input.disconnect_requested` is dead-on-send: set only in `Disconnected` state but the send path is gated to `Running`. 1 wasted byte/packet. | set at `protocol/mod.rs:1599`, gate comment `:1702`, recv-compat `:2191` | **M5** |
| D6 | Wall-clock `SystemTime::now()` for QualityReport ping bypassed the injectable clock (the only non-virtualizable timer). | was `protocol/mod.rs:64-100`, call sites `:1786`, `:2278` | **M0 ✅** |
| D7 | `sync_timeout` defaults to `None` → no event ever fires for a peer that cannot sync (with D2/D3 ⇒ infinite silent spinner). | `src/sessions/config.rs:122,142` | **M5** |
| D8 | Cold-start gossip-mute mesh deadlock at N≥3: permanent `confirmed == NULL` freeze on a healed network, every session `Running`. Found by the M1 fleet (seed 1, first 4-player run). | fix: `P2PSession::gossip_holds_confirmation_below_receipts` in `src/sessions/p2p_session.rs`; regression `tests/simulation/fleet.rs::regression_cold_start_gossip_stall_seed1_four_players` | **M1 ✅** |
| D9 | **Event-queue overflow silently discards safety-critical events** — `while event_queue.len() > max { pop_front() }` with no violation, no counter, no per-variant accounting. `Disconnected`, `DesyncDetected`, `Synchronized`, `SyncTimeout` all droppable. Same policy in the spectator session. Default size 100; a 16-peer churn wave can overflow within ~2 polls; even at N=2 an app draining events slower than it polls can lose a `DesyncDetected`. | `src/sessions/p2p_session.rs:9606-9608` (verified); `src/sessions/p2p_spectator_session.rs` ~`:1295-1298` | **M2 telemetry ✅** (session 64, §5.4: `SessionMetrics.events_discarded_total` + per-`EventKind` counters + one rate-limited `Warning` per overflow episode, both sessions; regression `event_queue_overflow_records_discard_telemetry` + rate-limit + spectator tests) + **M4** (retention policy) |
| D10 | `SavedStates` OOM fallback allocates a **1-cell** ring: every frame maps to the same cell, so any later rollback silently loads the wrong frame's state — guaranteed corruption if the path is ever hit (Error violation is emitted, then the session continues). Low reachability (allocation failure), catastrophic consequence. | `src/sync_layer/saved_states.rs:31-56` | **M4** (fail construction instead) |
| D11 | Frame-advantage estimation assumes symmetric one-way delay (`RTT/2`): on asymmetric paths (ADSL, satellite/terrestrial mix) the remote-frame estimate is systematically biased — e.g. 10ms/200ms split ⇒ ~6-frame overestimate at 60fps ⇒ chronic unnecessary `WaitRecommendation`s on one side. Inherent to ping-pong measurement (NTP-class limitation); needs bounding/damping + docs, not a naive "fix". | `src/network/protocol/mod.rs` `update_local_frame_advantage` (RTT/2 at ~`:828-834`) | **Part II §16 / M6 docs** |
| D12 | Wire-range-validation gaps for decodable-but-hostile values: `FloorReply.floors` entries accepted unchecked (any i32; only a `!= NULL` guard at the consumer), `ConnectionStatus.last_frame` extremes un-range-checked into gossip folds, checksum-report frame forwarded into `DesyncDetected` unvalidated. Bounded blast radius today (guarded consumers, checked arithmetic) but each is one refactor away from a real hole. | `protocol/mod.rs` `on_floor_reply` ~`:2356-2388`; `codec.rs` connection-status decode ~`:315-329`; `p2p_session.rs` ~`:9627` | **M5** (validate at decode) |
| D13 | `Halt` is not fully fail-closed: at disconnect, dropped slots leave the confirmation fold and the session confirms up to `max_prediction` further frames with fabricated default inputs, divergently across peers — contradicting the rustdoc "no further frames advance once any peer drops". Bounded (≤ max_prediction frames, session then halts; values ARE labeled `InputStatus::Disconnected` per-request) but the confirmed prefix (replays, results, checksums) forks — Halt-window replays diverge across peers. Found by the Part IV split-brain experiment (§28.2): peer 1 confirmed frames 96..=103 as `[3101, 3108, 0, 0]` while peer 3 confirmed `[0, 0, 3115, 3122]`. | red-doc test `tests/simulation/fleet.rs::partition_under_halt_confirms_fabricated_frames_divergently_d13`; `DisconnectBehavior::Halt` rustdoc in `src/sessions/config.rs`; mechanism (Part V §34): legacy-halt default branch of `synchronized_inputs`, `sync_layer/mod.rs:1316-1324` — Halt never engages the freeze machinery (`p2p_session.rs:7217-7238`), so no agreed value exists | **M4** (clamp Halt confirmation to the last globally-agreed frame; `**Pre-existing:**` changelog) |

## 3. Analysis findings that shape the work

- **The paradox:** 15 TLA+ specs, 30 Kani proofs, ~600 tests, 92% coverage — yet silent
  3+-player divergence shipped in 0.8.1 and residuals remain documented at 4+ players
  (`p2p_session.rs:7987-8146`). Bugs live in emergent whole-mesh schedule space; the DST fleet
  is the answer (proven by D8 on its first run).
- **Falsified suspicions (do not re-fix):** `local_checksum_history` is bounded
  (`max_checksum_history`, default 32; retain-prune at `p2p_session.rs` ~`:9618`, disconnect
  retain ~`:8709`) — only the stale-window question in §9.1 remains open. `disconnect_frame` is
  correctly min-folded at every arm site (`:1688,:2337,:2965,:3844,:4037,:7453-7461`).
- **Documented residuals to encode as tests (M3):** 4+-player staggered-drop family
  (stale-echo arbitrated NOTABUG; double-failure-relay closed by floor-round S55); spectator
  witness-less reactivation fails closed (`p2p_spectator_session.rs:2187-2199`).
- **M1 lessons baked into infrastructure:** serde_json needs `float_roundtrip` (ULP-lossy float
  parsing breaks corpus-replay determinism — already enabled in `Cargo.toml` dev-deps).
  A too-loose nudge condition wedged the hot-join handshake: any new liveness carrier must gate
  on the *deadlock signature*, not the steady state, and reserved hot-join endpoints must never
  be nudged.
- **Measured budgets (debug build):** 8-peer × 5000-step simulation = 11.7 s wall, 600k
  messages; PR smoke (600 steps, N≤4) ≈ 15 ms/run. Nightly fleet should build `--release`.
- **External reference points:** FoundationDB/TigerBeetle/Antithesis-style DST (this program);
  netcode.io for versioned secure UDP (adapted: version+nonce yes, crypto delegated to
  transport); CCF-style TLA+ trace validation (explicit follow-on, out of scope).

## 4. What M0/M1 built (foundation the rest depends on)

- `tests/common/sim_net.rs` — `SimNet<M>`: central seeded virtual-time message switch;
  per-directed-link `LinkPolicy` {drop, dup, base_delay, jitter, burst}, `set_blocked`
  (asymmetric), `set_holding` (capture/FIFO-release), `partition`, `heal_all`, `attach`/`detach`
  - `UnattachedPolicy::{Drop,Buffer}`, `SimNetStats` counters, 256-message per-poll cap.
  Payload-generic (unit-testable); `NonBlockingSocket<SocketAddr>` impl for `SimSocket<Message>`.
- `tests/simulation/harness/schedule.rs` — pure `generate(seed, SimConfig) -> Schedule`
  (schema-versioned, serde, fully materialized pre-run; separate `link_seed` so shrinking events
  never perturbs noise); `BackgroundNoise::{Clean,Mild,Rough}`; storyline segments
  (HeavyLossWindow / AsymmetricBlackhole / HoldRelease) confined to `(10%..90%)` of the pre-heal
  window, ≤60 steps each; `HealAll` + ~250-step drain always appended.
- `tests/simulation/harness/oracle.rs` — invariants: (a) confirmed-prefix agreement (incremental
  `confirmed_inputs_for_frame` sampling — inputs evict, so sample every step; never assert
  `confirmed_frame()` monotonicity), (b) end-of-run state agreement over the globally confirmed
  prefix, (b-cross) in-band `DesyncDetected` consistency, (g) advance-error capture,
  Error+-severity violation capture, (c-lite) end-progress (`MIN_END_CONFIRMED = 30`). Failure
  cap 64. Every invariant has a negative control.
- `tests/simulation/harness/mod.rs` — runner: N sessions (1 local + N−1 remote handles each),
  per-peer seeded `protocol_rng_seed = fnv1a(seed, i)`, injected `TestClock`, fixed step order
  (events → per peer: poll → events → add_local_input(pure fn) → advance → record → sample →
  trace-fold → clock.advance). `RunOptions{corrupt_state_from, corrupt_checksum_from}` for
  negative controls; `RunReport{verdict, trace_hash, final_confirmed, net_stats}`;
  `expect_pass` prints a `FORTRESS_SIM_REPRO seed=… n_players=… steps=… noise=…` line.
  `diagnose()` + `P2PSession::diagnostic_connect_status()` (`#[doc(hidden)]` in
  `src/sessions/p2p_session.rs`) dump per-endpoint gossip caches.
- `tests/simulation/fleet.rs` — `PR_SMOKE_SEEDS = [1,2,3,5,8,13,21,34]` × N∈{2,3,4};
  meta-determinism test (identical trace hashes); two negative controls; pinned D8 regression;
  `#[ignore]`d `budget_probe_eight_player_long_schedule` and `diagnose_repro`.
- Production changes shipped with M0/M1: `ping_epoch_base`/`ping_millis()` (D6),
  `gossip_holds_confirmation_below_receipts` + reserved-endpoint nudge exclusion (D8),
  `diagnostic_connect_status`. Changelog entries exist under `[Unreleased]`.

## 5. M2 — Metrics layer + baseline sweep

Goal: make every later claim measurable; fix D1. All library counters are plain integers,
always-on, pull-based snapshots (deterministic; WASM-safe — no `Instant` inside counters).

### 5.1 D1 fix: exact wire bytes (red first)

1. **✅ Done** (session 63). Red: property test — for arbitrary `Message`s,
   `msg.encoded_len() == codec::encode(&msg)?.len()` — landed as
   `codec::tests::encoded_len_matches_exact_wire_bytes` (proptest over `arb_message()` covering
   every variant, hot-join under `cfg`). `size_of_val` divergence documented by
   `codec::tests::size_of_val_is_constant_while_wire_size_is_not_d1`.
2. **✅ Done** (session 63). `pub(crate) fn Message::encoded_len(&self) -> usize` +
   `MessageBody::encoded_len` in `src/network/messages.rs` — arithmetic O(1)/alloc-free,
   wildcard-free `match`; exact against bincode standard/little-endian/fixed-int.
   `ConnectionStatus::WIRE_LEN = 7` (bool + i32 + u16). Header term is currently 2 B magic;
   becomes 12 B in M5 — the property test keeps it honest.
3. **✅ Partially done** (session 63). `queue_message` now does `bytes_sent += msg.encoded_len()`;
   changelog `**Pre-existing:**` fix (kbps semantics) written; `PeerMetrics.bytes_sent` payload-only
   / `kbps` keeps its `+ UDP_HEADER_SIZE` estimate decision recorded in the changelog.
   **Remaining (with §5.2):** `bytes_received` in `handle_message` (`protocol/mod.rs` ~`:1824`) and
   per-kind send/receive counters keyed on discriminant — deferred until §5.2's `PeerMetrics`
   surface exists to consume them (adding the fields now would be dead code under `deny(warnings)`).

### 5.2 Metrics types (additive API)

**Foundation landed (session 64):** the module is `src/metrics.rs` (re-exported). `SessionMetrics`
exists as `#[non_exhaustive]`, `Copy`, `Debug`, `serde::Serialize`, `to_json()`/`to_json_pretty()`
under `json`; `EventKind` + `FortressEvent::kind()` already ship as the events analogue of the
planned `MessageKind`.

**✅ `SessionMetrics` counters landed (session 65, PR #191):** every field in the `SessionMetrics`
bullet below now ships, wired to 8 P2PSession sites (all recorded outside the telemetry guards, so
always-on) with the `frames_advanced == visual_frames + resimulated_frames` and
`rollback_depth_histogram.total() == rollback_count` identities guaranteed by construction; new
public `RollbackDepthHistogram`. `stall_count` is rollback-mode only (lockstep waits excluded).
`RunReport` carries per-peer `SessionMetrics`; fleet test asserts the identities + non-zero
activity across the smoke fleet.

**✅ `PeerMetrics` + `peer_metrics()` accessor landed (session 66):** new public `PeerMetrics`
(`#[non_exhaustive]`, `Copy`, serde, `to_json`/`to_json_pretty` under `json`) surfacing per-peer
wire-exact `bytes_sent/received` + `packets_sent/received`, per-`MessageKind`
`messages_{sent,received}_by_kind` (new public `MessageKind` mirror + `MessageKindCounts` with
`total()`), `input_bytes_pre/post_compression`, and the `pending_output_len`/`pending_checksums_len`/
`ping_ms`/`remote_frame_advantage` gauges. Wired at `queue_message` (send by-kind),
`handle_message` (receive bytes/packets/by-kind, counted before every protocol-state filter so
`packets_received == messages_received_by_kind.total()`), and the `send_pending_output` compression
site. `MessageBody::kind()`/`Message::kind()` are wildcard-free (compile-guard on new variants).
Consuming sites: `P2PSession::peer_metrics(handle)` (same handle validation as `network_stats`) +
4 protocol unit tests (exact byte/kind identities, count-before-filter, compression, gauges) +
`tests/network/peer_metrics.rs` (end-to-end routing + accessor error paths + cross-run determinism).
Deliberately **dropped** the plan's `send_queue_len` field: it would collide in name with
`NetworkStats::send_queue_len` (which actually reports `pending_output.len()`) while exposing only the
transient internal flush buffer — `pending_output_len` is the meaningful backpressure gauge and is
kept. **✅ `SpectatorSession::peer_metrics(host_index)` landed (session 67, PR #193):** returns
`Option<PeerMetrics>` (à la `slice::get`; hosts are a dense `Vec` addressed by index in `0..num_hosts()`,
builder-priority order — no opaque handles, so out-of-range is the only failure mode → `Option`, not a
`Result` that would need a breaking `InvalidRequestKind` variant). Reads the counters the spectator's host
`UdpProtocol` endpoints already accumulate (send via `poll`→`queue_message`, receive via `handle_message`).
Failover caveat documented + tested: hosts are compacted (`retain_surviving_hosts`), never rebuilt, so
counters at a fixed index **jump** to the promoted survivor's running totals (never reset). 3 tests
(out-of-range, per-host isolation+counting, compaction index-shift).

**✅ `HotJoinMetrics` landed (session 67, PR #196).** Gated `hot-join` public type via
`P2PSession::hot_join_metrics() -> Option<HotJoinMetrics>` (`None` for a non-joiner): `completed`,
`polls_to_running`, `millis_to_running` (injectable-clock, DST-deterministic). Clock via the session's
stored `protocol_config.clock` (`clock_now`/`now()` helper). **Every-path completeness:** joiner→Running
at THREE sites — 2-peer apply (`:4464`), N-peer apply (`:5352`), AND `check_initial_sync` (`:7873`,
reachable when a joiner is fail-closed to `Synchronizing` mid-handshake then resumes) — all call an
idempotent `record_hot_join_activation()`. **The `check_initial_sync` site was found by adversarial
review** (my first pass instrumented only the 2 apply sites — the D9 every-path lesson bit again). 2-peer
integration test; N-peer + fail-closed sites reuse the same validated idempotent helper. This completes
the M2 §5.2 metrics surface. Original design notes (kept for reference):

**`HotJoinMetrics` — remaining M2 §5.2 item (design scoped session 67, ready to execute).**
Gated behind `hot-join`; measure the **joiner's** join latency. **Clock access is clean:** the
session already stores `protocol_config: ProtocolConfig` (`p2p_session.rs:217`) which holds the
injectable `clock: Option<ClockFn>` — add a `now()` helper mirroring the endpoint's
`map_or_else(Instant::now, |c| c())` so it's deterministic under the DST/TestClock harness. **Capture
points (audit ALL up front — this is the D9 "every-path" class):** (1) join start — session enters
`HotJoining` at construction (`p2p_session.rs:~821-830`); stamp `join_started_at` + arm a poll
counter; (2) the joiner poll loop (`poll_hot_join_joiner` ~:4205 / `_npeer` ~:4476) increments the
poll counter while `HotJoining`; (3) **BOTH** Running-transition sites — 2-peer at `:4401` and N-peer
at `:5288` (`self.state = SessionState::Running`) — stamp `became_running_at` and compute
`millis_to_running`/`polls_to_running` (use a single shared helper called at both sites, NOT
per-site edits, per the D9 wrapper meta-lesson). Type in `src/metrics.rs`: `#[cfg(feature =
"hot-join")]`, same derives as `SessionMetrics` (`#[non_exhaustive]` + Debug/Default/Clone/Copy/
PartialEq/Eq/Serialize), `to_json`/`to_json_pretty` under `json`. Consuming site: `pub fn
hot_join_metrics(&self) -> Option<HotJoinMetrics>` (None for a non-hot-join session) + a test driving
a full 2-peer AND N-peer join to `Running` (model on `tests/sessions/hot_join.rs` ~:865, TestClock
plus `advance`) asserting `millis_to_running` reflects the advanced virtual time and
`polls_to_running` > 1. Handle the never-completes / abort / retry (`ack_resends`) cases (metrics
stay partial, no panic).

New types (in `src/telemetry.rs` or new `src/metrics.rs`, re-exported): all `#[non_exhaustive]`,
`Clone + Copy`-where-possible, `Debug`, `serde::Serialize`, `to_json()` under `json` feature.

- `SessionMetrics`: `frames_advanced`, `visual_frames`, `resimulated_frames`, `rollback_count`,
  `rollback_depth_histogram: [u64; 17]` (buckets 1..=16 + overflow), `max_rollback_depth`,
  `prediction_miss_count`, `stall_count` (**new counter at the prediction-throttle site** — the
  `Ok(empty)` return in `advance_frame`, near the "Prediction Threshold reached" trace at
  `p2p_session.rs` ~`:1240`; not observable today), `wait_recommendations`,
  `confirmation_lag_current/max/sum` (sampled per advance), `checksums_compared/matches/mismatches`,
  container high-water marks (`event_queue`, `checksum_history`).
- `PeerMetrics`: `bytes_sent/received` (wire-exact via §5.1), `packets_sent/received`,
  `messages_{sent,received}_by_kind: [u64; MessageKind::COUNT]` + public `MessageKind` mirror
  enum with `as_str()`, `input_bytes_pre/post_compression` (hook exists — the `trace!` at
  `protocol/mod.rs:1566-1596` already computes both), `send_queue_len`, `pending_output_len`,
  `pending_checksums_len`, `ping_ms`, `remote_frame_advantage`.
- Accessors: `P2PSession::metrics()`, `P2PSession::peer_metrics(handle) -> Result<_,_>` (same
  handle validation as `network_stats`, `p2p_session.rs` ~`:5521`); `SpectatorSession::metrics()`
  subset (+ existing `frames_behind_host`). `NetworkStats` stays frozen (public fields;
  changing it is breaking). Hot-join phase timings (`HotJoinMetrics`) gated behind `hot-join`,
  timestamped via the injectable clock.
- Existing surfaces to build on, not duplicate: `SessionTelemetry` trait
  (`telemetry.rs` ~`:1699`, emit sites `p2p_session.rs` ~`:1113,:1236,:1320,:7718`),
  `CollectingTelemetry`, violation observers.

Overhead gate: criterion A/B on `benches/p2p_session.rs`
(`cargo bench --bench p2p_session -- --save-baseline pre` on the base commit, `--baseline pre`
after) — **≤1% regression** on `advance_frame_no_rollback/2` and `advance_frame_with_rollback/*`;
new micro-bench for `metrics()` (<100 ns) and `encoded_len` (<20 ns).

### 5.3 Baseline sweep

**Prep landed (session 67, PR #194):** the DST harness `RunReport` now carries
`peer_wire: Vec<PeerWireTotals>` — each peer's per-remote `PeerMetrics` folded into a per-player
bandwidth ledger (bytes/packets sent+received, per-`MessageKind`, input pre/post-compression) —
so the sweep can read wire bandwidth from `run()` without reaching into consumed sessions.
Consumer test `peer_wire_metrics_are_wired_across_smoke_fleet`.

**✅ Cell runner + PR gate landed (session 67, PR #195):** `tests/simulation/baseline_sweep.rs`
(placed in the simulation crate, not `tests/network.rs` — it reuses the M1 harness `run()`).
`run_cell(CellParams) -> CellReport` is a thin wrapper: uniform per-link `LinkPolicy`
(`drop_rate = loss%`, `base_delay = rtt_ms/2`, `jitter`, dup/burst 0), empty `events`, `heal_at`
past the end (constant conditions). `CellReport` (serde): per-player bytes/sec (pre/post
compression), messages-by-kind, rollbacks/100-frames, rollback-depth p50/p99/max (nearest-rank over
the merged histogram), confirmation-lag mean/max, stalls/min, wait-recs, `desync_incidents`.
`sweep_pr_gate` (5 cells) asserts desync==0 + liveness + bandwidth + percentile ordering + p99<=max
and bit-for-bit reproducibility; `full_matrix_sweep` (`#[ignore]`) runs the 64-cell base grid plus
scale spot rows to `FORTRESS_SWEEP_OUT` JSON Lines. **Measured: full matrix 48s, every cell desync=0, progress
2004-4945 at 5000 steps; bandwidth 4-16 KB/s/player, rollback depth ≤7 (max_prediction bound), lag
capped at 8.** All rate divisors NaN-guarded (incl. n==0) so the f64 report stays reflexive for the
replay assert.

**✅ Checked-in baseline / cost ledger landed (session 67, PR #197; schema v2 in session 80):**
`tests/simulation/baselines/sweep-v2.json` (blessed at 0.9.0 wire sizes with the {4,32}B input-width
axis) + `check_or_bless_baseline` in `sweep_pr_gate` — compares each gate cell within tolerance
(bytes ±5%, rollbacks ±10%, desync exact 0), `FORTRESS_SWEEP_BLESS=1` regenerates. Robust: metrics
deterministic (virtual time), 3-OS CI validates cross-platform determinism against the Linux-blessed
file. `BaselineCell` omits volatile version/git_sha for a stable diff; row identity includes input
width so 4B and 32B cells cannot compare against the wrong ledger entry.

**✅ Nightly full-matrix CI wiring landed (session 79):** `ci-network-nightly.yml` now runs the
ignored deterministic `full_matrix_sweep` on Linux with `FORTRESS_SWEEP_OUT` set and uploads the
JSONL artifact for 30 days. The full matrix also gained the deferred scale spot rows
N∈{3,8,12,16} now that the harness cap is 16; a cheap non-ignored regression test pins their
presence without running the expensive cells in PR CI.

**✅ Input-width axis {4,32}B landed (session 80):** the simulation harness now keeps `run()` as the
4-byte `StubInput` path and adds `run_with_input::<I>()` over a test-only `SimInput` trait. `WideStubInput`
is a fixed-width 32-byte serde type (`u32` logical lane + seven deterministic `u32` padding words), so
the sweep isolates wire cost while preserving identical game-state transitions. The oracle compares full
serialized input fingerprints (`logical`, encoded length, deterministic hash) for confirmed-input and
freeze-frame checks, preventing wide padding divergence from hiding behind the logical lane. The PR gate
and ignored full matrix now cover both widths with unique labels (`*-4b`, `*-32b`); the gate baseline is
`sweep-v2.json`.

Historical design sketch (superseded by `tests/simulation/baseline_sweep.rs`; keep only for
requirements not yet folded into the implementation):

- The current cell runner is deterministic, virtual-time, housed in
  `tests/simulation/baseline_sweep.rs`, and emits JSON Lines with per-`MessageKind` counters,
  rollback-depth p50/p99/max, lag mean/max, stalls, wait-recs, version, git SHA, desync incidents,
  and `input_width_bytes`. Checked-in gate baseline path is
  `tests/simulation/baselines/sweep-v2.json`; PR gate cells are 2p LAN/wifi/mobile and 4p
  wifi/loss15-rtt200 crossed with input widths {4,32}B. Full-matrix cells are the 64-cell N=2/4 grid
  plus N=3/8/12/16 scale spot rows, also crossed with {4,32}B.
- Nightly: ✅ full matrix behind `#[ignore]`, wired into `.github/workflows/ci-network-nightly.yml`
  (`--run-ignored ignored-only` pattern), artifact upload (30-day retention). Cells report
  `timed_out: true` at an iteration cap rather than hanging when the future timeout column lands.

### 5.4 Part II amendments (M2)

- **D9 telemetry (red first) — ✅ done (session 64).** Red-doc test flipped from "zero telemetry"
  to `event_queue_overflow_records_discard_telemetry`. Shipped: `SessionMetrics.events_discarded_total`
  - `events_discarded_by_kind` (per-`EventKind` array, custom labeled-map serde) in the new
  `src/metrics.rs`; `trim_event_queue` records every drop and emits **one** rate-limited
  `Warning`/`NetworkProtocol` per overflow *episode* (re-armed on each `events()` drain, so a churn
  burst warns once, not once per message) — both `P2PSession` and `SpectatorSession`. `EventKind` +
  `FortressEvent::kind()` are the events analogue of §5.2's planned `MessageKind`. Retention-policy
  change (never discard `Disconnected`/`DesyncDetected`; drop lowest-priority first) stays **M4** —
  telemetry first, policy with data.
- Extend the §5.3 sweep matrix with player spot rows {12, 16} once M3 raises the harness cap
  (§6.7) — ✅ landed in session 79 alongside N=3/8 rows; the N=16 bandwidth/confirmation-lag
  columns are the W-SCALE baseline (§14).
- Add `messages_by_kind` sweep columns for `FloorRequest`/`FloorReply` (relay-topology load,
  §14 cliff 7) — ✅ already covered by the all-`MessageKind` sweep map.

## 6. M3 — Lifecycle vocabulary, full oracle, nightly fleet, shrinker/corpus, residual & census encodings

### 6.1 Lifecycle `ScheduleEvent`s (extend `tests/simulation/harness/schedule.rs`)

**✅ `PeerStall{peer, steps}` landed (PR #198, `236e1f5`).** First lifecycle op:
a peer freezes for `steps` steps (local hang — GC/frame-spike/blocked save), skipping
poll+events+input+advance and emitting nothing on the wire, then resumes. Test-infra only
(no `src/`). Runner tracks a per-peer `stalled_until` deadline; the frozen peer still folds its
`(step, gs, confirmed)` tuple so the trace stays uniform + reproducible. Deliberately the one op
needing **no oracle change** (a hitch < disconnect timeout recovers). Deferred generator
integration (random storyline mix) to the §6.1 overhaul so existing seeds — incl. the D8 pin —
stay bit-identical. **New reusable harness infra it introduced:** `SCHEDULE_SCHEMA_VERSION` bump
1→2; `RunOptions.probe_confirmed_at` / `RunReport.probe_confirmed` (mid-run confirmed-frame
snapshot — end-of-run state hides recovery dynamics because a clean drain always converges);
up-front schedule-index + probe-range validation (malformed corpus schedules fail loudly).
Tests: `peer_hitch_recovers_with_consistent_mesh` asserts the freeze→recover arc (**measured**:
mid-stall clean `[244,245]` vs frozen `[195,195]` — the frozen peer gates the *whole* mesh's
confirmation; end-of-run within the prediction window), `oracle_catches_seeded_divergence_under_peer_hitch`
(oracle keeps teeth under a hitch), `run_rejects_out_of_range_peer_index` (fail-loud contract).
Reviewers: adversarial sub-agent (found the weak-teeth end-state assertion → mid-run probe) +
Copilot 2 rounds (assert-not-debug_assert, probe reuse, whole-class index/probe validation) +
Bugbot Low Risk. **Lifecycle op status below.**

| Op | Mechanism | Fire-time precondition (degrade to recorded no-op, never re-sample) |
| --- | --- | --- |
| `PeerStall{peer, steps}` | ✅ done (PR #198 + session 76 hardening): per-peer `stalled_until`; skip poll+advance; overlapping stalls keep the later deadline (`max(existing, new)`) | peer alive |
| `PeerKill{peer}` | ✅ done (PR #200): stop driving + `net.detach` (inbox discarded, default `UnattachedPolicy::Drop`); **oracle alive-mask** (`mark_peer_dead`) excludes dead/retired peers from (c-lite)/`global_confirmed` but still compares pre-death/pre-retirement states; `NoLivePeers` guard vs all-crashed vacuous pass. Scoped to `ContinueWithout` (Halt → D13, deferred to §6.2 (c)) | peer alive |
| `GracefulRemove{by, target}` | ✅ done (session 76, schema v5): calls `P2PSession::remove_player` on one survivor, then detaches/retires `target`; the rest of the mesh learns the drop by gossip; planted tests prove premise, determinism, survivor convergence, and post-removal oracle teeth | `by` alive; `target` remote & alive |
| `LegacyDisconnect{by, target}` | ✅ done (session 77, schema v6): calls `P2PSession::disconnect_player` on one survivor, then detaches/retires `target`; unlike `GracefulRemove`, this intentionally exercises the Halt-oriented legacy path and pins the current D13-adjacent observation (`recovered_within_b = Some(false)`, live-peer end-progress failures) instead of claiming survivor convergence | `by` alive; `target` remote & alive |
| `HotJoin{slot}` (cfg `hot-join`) | fresh `SimSocket` at vacated/reserved addr + `SessionBuilder::start_hot_join_session` (`builder.rs` ~`:1619`); host built with `with_hot_join(true)`/`add_reserved_player` | slot reserved or cleanly dropped; joiner constraints (input_delay 0, `max_prediction >= 1`, `SaveMode::EveryFrame` for N-peer) baked into config generation |
| `SpectatorStart{spec}` / `SpectatorHostKill{host}` | ✅ done (session 82 + session 001): default-empty `SimConfig::spectator_hosts` registers one pre-planned spectator handle on selected host peers, starts one multi-host spectator via `start_spectator_session_multi`, and drives it deterministically each step. `SpectatorHostKill` (schema v7) kills a configured redundant host under partition; planted coverage proves partition traffic was dropped, the kill changes execution beyond partition-only, the spectator compacts from 3 hosts to 2, and survivors keep confirming. | host configured with the spectator and not already retired |
| `SetInputDelay{peer, delay}` | ✅ done (PR #199): calls `set_input_delay` (actual `p2p_session.rs:6761`, PLAN's `~:6479` was stale); errors routed to the oracle; schema v3; premise decoupled via an increase-delta helper | exactly 1 local player for mid-run increases (harness always has 1); generator clamps (not emitted yet) |
| `HealAll` | existing | always; auto-appended before drain |

**Known seam to resolve with the generator overhaul (adversarial-review note, PR #199):** events apply
at step-top *before* the per-peer stall check, so a `SetInputDelay` firing on a peer inside its
`PeerStall` window would put gap-fill Input bytes on the wire while that peer is documented to "emit
nothing while frozen." Unreachable today (no planted schedule combines the two; the generator emits
neither). When the generator starts emitting both, either gate lifecycle ops on
`step >= stalled_until[peer]` or document lifecycle-ops-on-a-frozen-peer as undefined.

Generator: buggify-style storyline segments over the new ops (staggered multi-drop, asymmetric
flap, join-during-loss, spectator failover under partition) + background noise; assert a minimum
*effective-op* count post-generation (re-draw at generation time otherwise). Peers get per-peer
cadence (`poll_every`, `stall_until`) for fps-skew.

### 6.2 Oracle completion

**Alive-mask infra landed (PR #200, with `PeerKill`; generalized in session 76 with
`GracefulRemove`):** the oracle now tracks a retired/dead set (`mark_peer_dead`, fail-loud on
out-of-range); `finalize` excludes retired peers from (c-lite) end-progress and from the
`global_confirmed` prefix, but **still compares their pre-retirement recorded states in (b)** (a
peer that diverged before crashing or leaving is caught); a `NoLivePeers` failure guards against an
all-retired vacuous pass. This is the "which peers are live" foundation (c) below builds on.

**Harness hardening landed (session 76, adversarial-review driven):** (1) checksum-mismatch metrics
now feed the oracle directly, so a peer whose app starves `events()` cannot hide a checksum-only
desync by leaving/discarding `DesyncDetected`; regression
`checksum_mismatch_metric_catches_starved_desync_event`. (2) `run()` validates materialized/corpus
schedules, not just generator output: `n_players` 2..=16 before address construction, `steps >= 2`,
sorted in-run events, no post-heal non-`HealAll` faults, exact one-per-directed-link initial
coverage, no self/duplicate links, and no self link-fault events; data-driven regression
`run_rejects_invalid_materialized_schedule_invariants`. (3) overlapping `PeerStall`s cannot shorten
an existing long stall; regression `overlapping_peer_stalls_keep_the_later_deadline`.

- **(c) Liveness — ✅ landed (PR #205 GREEN, 5 commits `5db0162`→`adf5451`).** Post-`HealAll`, every live peer's
  `confirmed_frame` must advance ≥ `POST_HEAL_MIN_ADVANCE` (G=10) frames within B steps of the
  actual last `HealAll` event, per-peer (catches one-peer-pinned and mutual deadlock — the
  metastable stall that (c-lite)'s absolute end bar misses). **B is derived, not arbitrary:**
  `RECOVERY_WINDOW_MS = 4000` folds the documented compound worst-case recovery path —
  `disconnect_timeout` (2000ms) + a sync-retry burst (5×200ms) + gossip settle (3×200ms), rounded
  up — cited to `builder.rs:43-44` + `protocol/mod.rs:1237-1258,6431-6435`; converted per `step_dt`
  to `recovery_window_steps() = 250` at 16ms (the same drain budget the generator reserves). **G is
  bracketed on both sides:** `> max_prediction` (8) so a one-shot prediction-window refill does not
  clear it, `< MIN_END_CONFIRMED` (30) so (c) stays strictly complementary to (c-lite). **Measured
  margin ≈25×** (min legit advance 249 vs G=10), holds under Rough noise. Heal anchor + window
  derive from the ACTUAL last `HealAll` event step (not `schedule.heal_at`, per Copilot review), so
  a schedule whose `heal_at` drifts from where its event fires cannot misreport, and a `heal_at`-
  without-`HealAll` run (e.g. clock-skew) stays correctly inert. `window_steps` reports the real
  measured span (B, or B-1 at an exact-boundary drain). Runner owns "should (c) run" via
  `HealLiveness::ran` (heal fired AND ≥B-step window follows). **Always-on** (no SimConfig/schema
  field — anchors reuse the already-trace-folded per-`(step,peer)` confirmed read, so existing
  seeds replay bit-identically), so every healing schedule (smoke/tcp/hitch/split-brain fleets)
  gains it free. Four data-backed tests share one builder differing only by stall duration: 60-step
  hitch recovers (~191 frames, `Some(true)`); no-`HealAll` stays inert (`None`); a peer frozen
  across the whole window advances 0 while ContinueWithout survivors advance 90 → (c) fires on that
  peer only (`Some(false)`); a partition healed under Halt never recovers (`Some(false)`).
  **Adversarially reviewed** (session 75): every claimed number reproduced (pinned 0 / survivors 90
  / hitch 191-193 / healthy 249-250; B-derivation matches cited constants; Copilot event-anchor fix
  verified; no off-by-one at exact-boundary drains; ~25× margin holds over 40 Rough-noise seeds).
  One real finding fixed (`c060719`): an all-dead armed mesh reported `recovered_within_b =
  Some(true)` from the empty live-peer loop — now `None` (indeterminate), gated on ≥1 live peer
  checked; `NoLivePeers` still fails the verdict. Regression `recovered_within_b_is_none_when_
  every_peer_is_killed`. **PR reviewer rounds (Copilot/Cursor) all addressed:** doc self-
  contradictions on the B/B-1 window semantics + `saturating_sub` clarity (`f28648d`); and two
  latent `step_dt_ms` robustness gaps (`adf5451`) — reject `step_dt_ms == 0` up front, and gate (c)
  to indeterminate when the observed window span < G (a sub-G window can't score a healthy
  ~1-frame/step peer without a false `Some(false)`); regressions `run_rejects_zero_step_dt` +
  `post_heal_liveness_is_indeterminate_when_window_narrower_than_g`. **Still owed:** the Halt-mesh clean-halt inverse assertion (deferred — it
  intersects D13's Halt-fork behavior, an M4 fix, so "no error spew" is not a clean assertion yet).
- **(e) Freeze-frame convergence — ✅ landed (session 78).** The simulation harness now records each
  peer's applied `(input, InputStatus)` vector by simulated frame with rollback last-write-wins
  semantics (matching the existing post-advance state recorder). The oracle checks every retired
  slot against every live `Running` survivor's confirmed prefix: `F_applied` is the start of the final stable
  trailing `Disconnected` run, and both `F_applied` and the frozen input value must match. Missing
  stable freezes compare as `None`, so "one survivor never froze the slot" fails against a survivor
  that did; if every live `Running` survivor reports `None`, the retired slot fails as
  `FreezeFrameMissing` instead of false-greening. Negative controls
  `oracle_detects_freeze_frame_divergence` and
  `oracle_detects_missing_freeze_frame_for_running_survivors` prove both classes fail; adversarial
  review also added the inverse mixed `None`/`Some` diagnostic guard
  `oracle_does_not_report_missing_when_later_survivor_has_freeze_frame`. Existing
  `PeerKill`/`GracefulRemove` lifecycle schedules remain green under the always-on check.
- **(d) Spectator convergence — ✅ basic path landed (session 82).** The harness now supports one
  pre-planned multi-host spectator via default-empty `SimConfig::spectator_hosts`, so existing
  generated schedules remain byte-identical unless a planted schedule opts in. Config validation
  rejects out-of-range and duplicate hosts before sessions are built. Configured P2P hosts register
  one spectator handle, the runner starts one `SpectatorSession` with `start_spectator_session_multi`,
  polls/drains it in the deterministic step loop, records displayed `AdvanceFrame` input/status
  vectors, and includes spectator violations in the same census stream as peers.

  The oracle compares every displayed spectator frame inside the live mesh's confirmed prefix
  against the first live `Running` mesh peer with an applied-input record. Serialized-input
  fingerprints must agree; dropped-slot `Disconnected` status must agree exactly. Mesh records may
  still show a matching live slot as `Predicted` when no later rollback resimulated that early frame,
  so `Predicted` mesh vs `Confirmed` spectator is accepted only when the fingerprint matches.
  `SpectatorDivergence` events and non-lag spectator errors fail the run. For lifecycle schedules,
  the runner also supplies a required displayed-frame floor captured at successful retirement time
  (`slowest live survivor game frame + G`) so a spectator that only advances before a removal cannot
  false-green, without charging skipped schedule steps as game frames.

  Tests: planted `preplanned_spectator_matches_graceful_remove_mesh_canon` attaches a spectator to
  graceful-remove survivors and proves deterministic post-removal display through the required floor;
  paired stalled-host regressions prove the floor is in display-frame space and is wired into the
  oracle; negative controls corrupt spectator fingerprints and dropped-slot statuses; oracle units
  pin missing progress, missing required frame, status divergence, and the allowed
  `Predicted`/`Confirmed` live-slot asymmetry. Adversarial review found and fixed two real gaps
  (post-removal progress was originally only a count; dropped-slot status corruption was missing);
   PR review then fixed the step-vs-frame floor and fingerprint wording. Session 001 added the
   `SpectatorHostKill` failover-under-partition schedule with a partition-only no-op control,
   final-host-count proof, and fail-loud malformed schedule cases. **Still owed:** the documented
   witness-less-reactivation fail-closed residual (`p2p_spectator_session.rs:2187-2199`) classification.
- **(f) Violation allowlist — ✅ landed (session 81).** `Oracle` now owns telemetry severity and
  allowlist policy instead of receiving pre-filtered `Error`+ violations: exact `(severity, kind,
  location)` plus message-prefix matches can be tolerated and counted, while `Critical` remains
  fail-closed even when a matching entry exists. The reviewed default allowlist is intentionally
  empty after the first census: an explicit ignored/manual 200-seed generated four-player smoke
  census produced an empty ledger (zero signatures, zero allowlist hits). `RunReport::violation_census`
  preserves every observed signature before filtering (including warnings) using the same
  prefix-normalized signature model as allowlist review, and `Verdict::violation_allowlist_hits`
  exposes tolerated hit counts for future nightly spike reporting. Validation rejects Critical,
  empty-prefix, missing-rationale, malformed `file:line`, duplicate, and prefix-overlapping reviewed
  entries. Negative controls prove allowlisted `Error` suppression + counting, allowlisted `Warning`
  counting without failure, Critical hard-fail, and numeric-suffix census normalization.
- **Meta:** every fleet failure re-runs once (trace-hash equality) before being reported, to rule
  out harness nondeterminism.

### 6.3 Residuals (red-first, `tests/simulation/residuals.rs`)

1. 4+-player staggered-drop stale-echo pin: byte-identical confirmed state, confirmation never
   overruns the freeze (goes RED if S41/S55 behavior regresses). Source block
   `p2p_session.rs:7987-8146`; unit-level repros exist at `tests/sessions/peer_drop.rs:3106,:3413`.
2. Double-failure-relay end-to-end at session level (floor-round hold closes it).
3. Spectator witness-less reactivation: pins fail-closed-at-frozen-label; FAILs on any
   resurrect-with-wrong-value (complements `p2p_spectator_session.rs` ~`:6273`).

### 6.4 Census (premise-asserted schedules, `tests/simulation/census.rs`)

Multi-peer drops at different frames within one poll window (`disconnect_frame` min-fold probe);
`SaveMode::Sparse` × disconnect-rollback interplay; frozen-queue + `NetworkInterrupted`/
`NetworkResumed` blip; RTT ≫ `max_prediction` (assert throttle is `Ok(empty)` + liveness holds);
&gt;128-frame asymmetric-receipt stagger before a drop (`INPUT_QUEUE_LENGTH = 128` ring eviction).
Each schedule asserts its premise via `SimNetStats` (the fault actually hit traffic). The 1-cell
`SavedStates` OOM fallback (`src/sync_layer/saved_states.rs:31-56`) is not DST-reachable — cover
with an `__internal` unit test + doc note.

### 6.5 Shrinker, artifacts, corpus

- Artifact on FAIL: `target/sim-artifacts/<test>/<seed>.json` = `{schema_version, seed, config,
  explicit schedule, event-trace ring (last N steps), verdict, trace_hash}` + the existing
  `FORTRESS_SIM_REPRO` stdout line. Nightly uploads artifacts on failure (7-day retention).
- Shrinker (`tests/simulation/harness/shrink.rs`), domain-specific over explicit schedules; each
  accepted step re-verifies the same verdict class: (1) confirm ×2; (2) peer removal with
  handle/address re-mapping; (3) prefix binary-search on step count; (4) ddmin over schedule
  events; (5) field simplification (zero loss/jitter/dup, collapse delays, uniform cadence).
  Cap ~500 re-runs or 5 min; emit best-so-far continuously. Self-test: plant a known
  2-peer-sufficient failure in an 8-peer schedule; must reduce to ≤3 peers and ≤¼ steps.
- Corpus: minimized schedules promoted to `tests/simulation/corpus/NNN-slug.json` via
  `scripts/simulation/promote-artifact.sh` (validate schema + replay once; run
  `bash scripts/ci/check-shell-portability.sh`); `tests/simulation/corpus_replay.rs` replays every
  corpus file with the full oracle on PR CI (≤20 s total; split tests when a bucket exceeds 15 s).
  Migrate the D8 pinned regression into the corpus format once it exists.

### 6.6-pre Part II amendments (M3 harness upgrades — do these WITH 6.1)

1. **Raise the mesh cap to 16**: the `(2..=8)` assert in
   `tests/simulation/harness/schedule.rs` is an M1 artifact (not a design ceiling — do not treat
   it as evidence of known N>8 bugs). Raise to `2..=16`; add N∈{12,16} nightly rows and one
   N=16 PR smoke seed. Re-run the budget probe at N=16 (expect roughly 4× the measured N=8
   11.7 s/5000-step debug cost from O(N²) endpoints; record the number).
2. **Per-peer clock (rate) skew — ✅ landed (PR #202).** `TestClock::as_skewed_protocol_clock(num,
   den)` (skewed = `base_start + base_elapsed × num/den`, **u128 integer** — deterministic
   cross-platform, no float; unit-tested 2x/0.5x/1x/frozen); `SimConfig.clock_skew_ppm: Vec<i32>`
   (`#[serde(default)]` empty = no skew, existing seeds bit-identical, no schema bump; frozen at
   `-1_000_000`, lower rejected up front). Runner injects the exact base clock at 0 ppm
   (byte-identical to before), a skewed clock otherwise; SimNet keeps the base (network = real time).
   Test `clock_skew_is_tolerated_and_alters_execution` (peer 0 at +10% over a constant-40ms mesh):
   **mesh stays byte-consistent** (game logic is per-`(step,peer)`, not clock-driven) while the skew
   demonstrably alters timing-gated execution. Consistency extended to a long +0.1% run in PR #203
   (`clock_skew_holds_consistency_over_a_long_run`). **CRITICAL LIMITATION (adversarial finding, PR
   #203): this harness CANNOT test H-SKEW's rate-drift.** The runner advances every peer ONE frame
   per step (lockstep) with the clock read only for timestamps, so the H-SKEW mechanism (fast clock
   ⇒ faster frame-production ⇒ drift accumulation) is structurally absent — a lag/recs "falsification"
   here is a tautology. **Remaining (the real H-SKEW experiment): a SKEW-GATED FRAME MODEL** — advance
   each peer at a rate driven by *its own* skewed clock, then run hour-equivalent and measure
   confirmed-lag creep / one-sided recommendations. Also: clock *offset* (constant lead/lag) if
   needed; per-peer skew in the random generator. The skew infra still **unblocks the symmetric H-OSC
   case** (per-peer clock differences source the transient advantage symmetric *delay* alone doesn't
   produce).
3. **WaitRecommendation-obeying app model — ✅ landed (PR #201).** `SimConfig.app_model:
   AppModel {Ignore, Obey}` (`#[serde(default)] = Ignore`, so existing seeds replay
   bit-identical, no schema bump; missing-field deserialize test pins it). Runner: per-peer
   `wait_skip` counter accumulates `skip_frames` (max) from `WaitRecommendation` events under
   `Obey` and counts down **only on `Running` steps** (a peer that leaves `Running` mid-wait
   must not silently consume its owed skips). Test `app_model_obey_wait_recommendation_stays_consistent`
   over **asymmetric-delay** links (symmetric emits no recommendations — the D11 bias is the
   source). **H-OSC precondition data (asymmetric side-observation only, NOT the symmetric FM-3
   experiment which is still owed): the one-sided closed loop is well-damped — obeying costs
   ~0-2 frames of progress over 900 steps, never breaks consistency/liveness.** **Remaining:**
   `ObeyLate(k)`/skip-smearing variants + per-peer (asymmetric-obey) policy + advantage
   time-series recording for the full §16/A10 oscillation experiments (symmetric-delay Obey,
   cooldown ∈ {60,240}, skip-cap ∈ {none,9}).
4. **Reliable-ordered link profile** (TCP/WebRTC-reliable model): `BackgroundNoise::ReliableFifo`
   — 0 loss/dup, FIFO per link (SimNet already guarantees per-link FIFO at equal delay), plus a
   **retransmit-delay mode** approximating HOL: on a would-be drop, deliver the message *and
   everything sent after it on that link* no earlier than `drop_time + retransmit_timeout`
   (single-queue delay burst). Green expected (protocol tolerates ordering); pins H-TCP.
5. **MTU/fragmentation emulation** (unlocks H-FRAG): size-dependent loss in `SimNet` —
   `p_effective(size) = 1 − (1−p_frag)^ceil(size/1472)` when `size > 1472`; needs
   `Message::encoded_len()` from M2 (§5.1). Off by default; schema field.
6. **Oracle additions**: (h) *event-loss oracle* — ✅ **landed (PR #204).** `SimConfig`
   gained `starve_events: Vec<usize>` (peers whose app model never drains `events()`) +
   `event_queue_size: Option<usize>` (cap override, min 10), both `#[serde(default)]` (existing
   seeds bit-identical, no bump). A starved peer's bounded `event_queue` fills → the session trims
   → D9 `events_discarded_*` fires. `event_starvation_overflows_the_bounded_event_queue` (Clean
   N=8, 10-slot cap, peer 0 starved) asserts **fire** (peer 0 overflows, ≈25 discards; every
   draining peer 0) + **neutralize** (identical schedule, all draining → 0 — isolates starvation
   as the sole cause); both runs `expect_pass` (not servicing events must not desync/stall the
   mesh — a starved peer keeps the primary state/confirmed/liveness checks, and its divergence is
   still caught in-band by neighbors). N and the cap are load-bearing together (draining-peer
   high-water is the ≈N−1 cold-start sync burst, which must stay under the cap; documented).
   Adversarially reviewed (all tautology/vacuous/oracle-weakening probes refuted).
   (i) *metastability probe* — ✅ **landed (PR #205, with (c)).** `Verdict::recovered_within_b:
   Option<bool>` surfaces the explicit "recovered within B steps of heal: yes/no" signal:
   `Some(true)` = healed and every live peer advanced ≥ G within B; `Some(false)` = healed-but-
   pinned (the metastable case); `None` = inert (no heal) or indeterminate (window < B). Reported
   for free by every healing schedule (same anchors as (c)).

### 6.6 Nightly fleet

`#[ignore]`d `fleet::nightly_*` fns gated on `FORTRESS_SIM_TIER=nightly`;
`FORTRESS_SIM_SEED_BASE=${{ github.run_id }}`; ~1000 seeds sharded over ~8 test fns; N drawn
2..=8; ~5000-step schedules; build `--release` (measured: 11.7 s debug for one n=8×5000 run).
New nextest profile with `slow-timeout = { period = "120s", terminate-after = 8 }` mirroring
`profile.ci-network-nightly` (`.config/nextest.toml` ~`:86`); new job in
`ci-network-nightly.yml` (or sibling `ci-simulation-nightly.yml`):
`cargo nextest run --profile <p> --release --run-ignored ignored-only -E 'test(simulation::fleet::nightly)' --test simulation --features hot-join`.
Wall target ≤60 min. `scripts/hooks/check-network-timing-invariants.py` is scoped to
`test(network::multi_process::)` — verified unaffected. Run `actionlint`; verify via
`workflow_dispatch`.

## 7. M4 — Fix campaign

Process per finding: shrink → corpus entry (fails PR CI) → root-cause → fix at the right level →
corpus entry stays green as the permanent regression → changelog per the unreleased-code rule
(fixes to already-released behavior get `**Pre-existing:**` `### Fixed` entries). D8 is the
template — including its lesson that a first-cut fix can be too broad; let existing suites and
probe data tighten it.

**Decision point (return to maintainer with data):** if findings concentrate in
spectator-failover/hot-join convergence, propose subsystem simplification/redesign instead of a
fourth round of corner-case patches. Any fix requiring wire changes **queues for M5** (one break).

## 8. M5 — Protocol v1 (ships as 0.10.0)

One atomic wire break. Sequence into review-sized PRs on a feature branch; release together.

### 8.0 Red tests documenting today's failures (no production changes)

- T0.1 `tests/network/protocol_version.rs`: feed non-legacy-format bytes via a stub socket into a
  real session (TestClock-driven) → assert today's behavior: perpetual `Synchronizing`, zero
  events, zero violations (the D2 silent drop). Assertions flip after 8.2/8.3.
- T0.2 `tests/sessions/compat.rs`: `num_players` 2 vs 3 → assert both sessions emit
  `Synchronized` **then** stall to `Disconnected` (the D3 confusing sequence).

### 8.1 Wire format v1

Header grows 6 → 12 bytes; all header fields are serde fields of `MessageHeader`, so the generic
`codec::encode` path keeps sockets untouched:

```text
v1 packet (little-endian, bincode fixed-int):
offset size field              constraint
0      2    sentinel           [0xF5, 0x52]  (outside RFC 7983 demux ranges)
2      1    protocol_version   u8 == PROTOCOL_VERSION (1); else reject
3      1    flags              u8 == 0 (bit 0 reserved: future auth tag); nonzero rejected
4      4    conn_id            u32 LE; nonzero AND low-16 bits nonzero
8      4    body discriminant  u32 LE (existing numbering 0..=16, +17 Goodbye)
12..   var  body               unchanged per-field encodings

legacy 0.9.0 packet: [magic u16 LE][discriminant u32 LE ≤ 16][body]
```

- Constants: `pub const PROTOCOL_VERSION: u8 = 1` (crate root near `NULL_FRAME`),
  `pub(crate) WIRE_SENTINEL`, `pub(crate) MIN_SUPPORTED_PROTOCOL_VERSION`.
- **Legacy-interop is rejected unconditionally in both directions** given the conn_id
  constraints: a legacy peer reading a v1 packet sees discriminant
  `1 + (conn_id_low16 << 16)` > 16 (low-16 nonzero) → unknown-variant reject; a v1 peer reading a
  legacy packet fails sentinel (or, on the 1/65535 magic collision, fails the version/flags/
  conn_id-low16 checks; a 10-byte legacy SyncReply also under-runs the 12-byte v1 minimum).
  Enforce the low-16-nonzero constraint in generation (re-roll loop at `protocol/mod.rs`
  ~`:643-653`) and in decode.
- `conn_id` replaces `magic` everywhere: filter (~`:1833-1837`), remote learn at final sync
  roundtrip (~`:1939`), hot-join era fence `wrapping_add(1)` skipping forbidden values
  (~`:1092-1110`; update wrap-rarity docs to 2³²), `remote_magic` field/docs rename, and the
  `ConnectionStatus::epoch` cross-references in `messages.rs:34-37`.
- Size cost: +6 B/packet steady-state (−1 B from D5 removal). Watch the 4 KiB hot-join snapshot
  cap (`with_hot_join_max_snapshot_wire_bytes`) — tests sitting exactly at caps may need re-tuning.
  The M2 sweep baseline diff is the reviewed cost ledger.

### 8.2 Decode-side refusal UX (fixes D2)

- `decode_message` prelude validation + `pub enum WireRejectKind { LegacyUnversionedSuspected,
  UnsupportedVersion{seen}, UnknownFlags{seen}, BadSentinel, Malformed }` + a
  `classify_wire_bytes(&[u8]) -> WireRejectKind` helper (legacy heuristic:
  `len >= 6 && bytes[3..6] == [0,0,0] && bytes[2] <= 16`, documented best-effort).
- `src/network/socket_receive.rs:61`: stop swallowing — emit
  `report_violation!(Warning, NetworkProtocol, …)` with classification + source address,
  **rate-limited to one report per classification per poll** (reuse the per-poll cap pattern in
  that file).
- Keep all bounded-decode properties (no allocation before validation).

### 8.3 Handshake v1 (fixes D3; hard-fail-all per locked decision)

Fixed-width extensions to `SyncRequest`/`SyncReply` (manual decoders in `codec.rs` read fixed
widths only):

```rust
pub(crate) struct SessionConfigBlock {   // 14 bytes on wire
    num_players: u16,
    input_bytes_per_player: u16,  // from validate_default_input_wire_size::<T>()
    fps: u32,
    max_prediction: u16,
    desync_interval: u32,         // 0 = DesyncDetection::Off
}
// SyncRequest/SyncReply each additionally carry:
//   min_compat_version: u8, features: u32 (bit 0 = hot-join), config: SessionConfigBlock,
//   config_digest: u64
```

- Digest = `fnv1a_hash` (`src/hash.rs:45-49`) over the exact canonical bytes
  `b"FRv1-cfg" ‖ num_players:u16 LE ‖ input_bytes:u16 LE ‖ fps:u32 LE ‖ max_prediction:u16 LE ‖
  desync_interval:u32 LE ‖ features:u32 LE` — documented as a wire contract + golden test.
  Non-cryptographic is fine: it detects accidental mismatch; Byzantine peers are out of scope.
- Flow: the responder **always replies with its own block** (no new reject message → no spoof
  surface; both sides diagnose identically). On mismatch: set terminal
  `handshake_failed: Option<IncompatibleSessionReason>` on the endpoint (stop re-sending sync
  requests; keep answering incoming requests with the block), emit protocol
  `Event::Incompatible { reason }` once → translate to new
  `FortressEvent::IncompatibleSession { addr, reason }` in both session types.
  `IncompatibleSessionReason` is `Copy` with variants naming the divergent field
  (`ProtocolVersion/NumPlayers/InputWidth/Fps/MaxPrediction/DesyncInterval/Features/ConfigDigest`,
  each `{ ours, theirs }`). `FortressEvent` is exhaustive + `Copy` → **Breaking** changelog entry.
- Hard-fail every field including `max_prediction`/`desync_interval` (locked decision).
  `DisconnectBehavior` is deliberately excluded (local policy, never on the wire — document in
  threat model). Feature bit 0 mismatch hard-fails (a joiner whose host lacks hot-join otherwise
  waits forever). Floor-round messages are core in every v1 build — no sub-negotiation.
- **D7:** `SyncConfig::default().sync_timeout` → `Some(Duration::from_secs(20))`
  (`config.rs:122,142`; audit presets `:178-239`). Event-only (session keeps retrying).
- Sync-token echo validation and per-connection replay resistance: unchanged random-echo set
  (`protocol/mod.rs` ~`:1773,:1916`) + the conn_id's 2⁻³² filter (was 2⁻¹⁶).
- Flip T0.2; add one mismatch test per reason variant (feature-bit test needs a hot-join build
  with `with_hot_join(false)` vs `true`).

### 8.4 Goodbye + remove dead `disconnect_requested` (D5)

- `MessageBody::Goodbye(Goodbye { reason: u8 })`, discriminant **17** (above the hot-join block
  10..=16; v1 numbering is fixed independent of features — non-hot-join builds decode it and,
  for hot-join variants, reject-classified rather than unknown-variant). Send best-effort ×3
  from `disconnect()` (`protocol/mod.rs` ~`:989-997`); accept in `Synchronizing|Running`
  (~`:1887-1899`); receive → existing `Event::Disconnected` path (idempotent via
  `disconnect_event_sent`). Timing test: `remove_player`/`disconnect_player` produces the remote
  `Disconnected` event without waiting the full `disconnect_timeout` (TestClock).
- Remove `Input.disconnect_requested` — **first** add a test proving it is never transmitted
  (send path `Running`-gated; set only in `Disconnected` — sites: `messages.rs:88`,
  `protocol/mod.rs:1599,:2191`, merge-skip ~`:1970-1973`, decoder `codec.rs:394`). Expect wide
  mechanical churn in `protocol/mod.rs` tests (~7k lines construct `Input` literals) — land a
  test-local `msg(conn_id, body)`/input-literal helper FIRST to shrink the diff. Update the
  `disconnect_requested` doc references in `p2p_session.rs` (~`:8080,:8337`).

### 8.5 Compatibility machinery

- Golden wire suite `tests/network/wire_format.rs`: for **every** v1 variant (hot-join ones under
  cfg), a fully-populated message + `const EXPECTED: &[u8]` hex literal; assert
  `encode == EXPECTED`, `decode_message(EXPECTED) == (msg, len)`, generic-decode parity (extend
  the existing byte-exact seed test in `codec.rs` ~`:962`). Meta-guards:
  `assert_eq!(PROTOCOL_VERSION, 1, "wire bytes changed? create wire_golden_v2 and bump")` and a
  wildcard-free `match` over `MessageBody` (in a `messages.rs` unit test where the enum is
  visible) so adding a variant without a golden fails to compile.
- Legacy goldens `wire_golden_legacy_0_9.rs`: hex constants of real 0.9.0 packets (one per legacy
  variant + a recorded handshake), generated **once** from the `v0.9.0` git tag via a throwaway
  harness whose source is committed in a comment block (provenance). Tests: v1 rejects every
  legacy golden with `LegacyUnversionedSuspected`; a test-local reimplementation of the 6-byte
  legacy header read rejects every v1 golden (locks §8.1's argument both directions). Live
  mixed-version test: replay recorded legacy handshake bytes into a real session → never
  `Running`, classified violation captured, `SyncTimeout` fires. Flip T0.1.
- Pre-commit hook `scripts/hooks/check-wire-golden-immutable.py` (+ unit tests in
  `scripts/tests/`, register in `.pre-commit-config.yaml` and `scripts/ci/agent-preflight.py`,
  following the `check-changelog-unreleased.py` pattern): fail if a released `wire_golden_v*.rs`
  changes in a diff unless `PROTOCOL_VERSION` changed in the same diff. Keep <1 s.
- Versioning policy text (goes in the threat-model doc + `PROTOCOL_VERSION` rustdoc):
  any change to bytes any message can produce/accept bumps the version; appending a tail variant
  without a bump is permitted only if optional-for-correctness AND send-gated on a negotiated
  feature bit (the discipline 0.9.0's FloorRequest lacked); v1 acceptance is exact
  (`version == 1`), `min_compat_version` future-proofs speak-down.

### 8.5b Part II amendments (M5 — same wire break, decided while the wire is open)

- **D12 range validation at decode**: `FloorReply.floors` entries must be `NULL || (0..=i32::MAX)`
  with a sane upper sanity bound; `ConnectionStatus.last_frame` likewise; checksum-report frames
  validated before the `DesyncDetected` event is built. Reject-classified like other malformed
  input (no allocation, one violation per poll per peer).
- **Connect-status compaction (evaluate, decide with sweep data)**: `peer_connect_status` costs
  `7 × N` bytes per Input packet (112 B at N=16 — §14). Candidate v1 layout: 1-bit disconnected
  bitfield + per-slot `last_frame` varint-or-fixed + epoch only when nonzero; or gossip only
  *changed* slots with a full vector every k-th packet. Any variant must preserve the
  reorder-safe merge semantics (`merge_peer_connect_status` converge-down). Use the M2 sweep
  N=16 bandwidth column to accept/reject — if compaction ships it must be in THIS wire break.
- **TCP/stream framing helper + transport docs**: ship `codec::{encode_framed, FrameDecoder}`
  (u32-LE length prefix + `Message` bytes; `FrameDecoder` buffers partial reads and yields
  complete messages using `decode_message`'s consumed-length contract) plus
  `docs/` transport guidance: TCP requires `TCP_NODELAY` (Nagle ≈ +2.4 frames at 60fps),
  HOL quantified (one retransmit at a 200ms retransmit timeout ≈ 12 frames > the default 8-frame
  prediction window ⇒ stall-and-catchup, not divergence), WebRTC data channels must be
  **unordered+unreliable** for
  optimal play (matchbox channel-mode guidance), QUIC RFC 9221 datagrams / WebTransport
  recommended over raw TCP for web deployments. Note: TCP itself never surfaces duplicate
  payloads to the app (stack dedups) — no protocol change needed for duplicates.
- **MTU guard**: with `encoded_len()` available, emit a structured warning (and metrics counter)
  when an encoded message exceeds 1472 B (fragmentation threshold), not just the existing 508 B
  ideal-size log; document the loss-amplification math (4 KiB snapshot at 5% loss ⇒ ~14%
  effective loss).

### 8.6 Threat model — `docs/threat-model.md`

New page (mkdocs nav under Architecture; wiki mirror with `<!-- SYNC: -->` headers per
convention). Outline: semi-trusted peer model; **in scope**: malformed/hostile bytes (bounded
decode: length-vs-remaining pre-alloc checks, 64 MiB cap, depth 128, per-poll caps), version
mismatch (§8.1-8.2), config mismatch (§8.3), cross-session replay (conn_id 2⁻³² + handshake
random echo), protocol-level resource exhaustion; **delegated**: confidentiality/authenticity →
transport (WebRTC/DTLS via matchbox; include an HMAC/AEAD socket-adapter sketch over
`NonBlockingSocket` — apps can wrap today with zero library changes), on-path attackers, DDoS,
NAT traversal, Byzantine peers (advisory checksum trust-downgrade only). **Packet-auth
deferral rationale** (locked): flags bit 0 reserved; crypto deps bring unsafe/SIMD into a
`forbid(unsafe_code)` tree and expand deny/vet surface; dominant deployments already carry DTLS.

### 8.7 Release mechanics (Breaking Changes Checklist, `.llm/context.md`)

`Cargo.toml` → 0.10.0 + `bash scripts/sync-version.sh --check`; CHANGELOG `**Breaking:**`
entries (wire format, handshake, `FortressEvent` variant, `Input` field removal, sync-timeout
default) with migration guidance; `docs/migration.md` "0.9 → 0.10" incl. a mixed-version symptom
table (what each side observes) and a custom-socket note (API unchanged; raw-byte
recorders/relays must re-record); README/user-guide/architecture wire sections;
`rg 'magic' --type rust --type md` sweep; `cargo build --examples`; `cargo test --doc`; fuzz
targets updated (`fuzz_message_parsing` roundtrip logic unchanged, regenerate corpus seeds; audit
`fuzz_protocol_input_packet` header assumptions; clear stale `proptest-regressions/network`
entries only if they encode old-format expectations). TLA+ note: Goodbye only accelerates an
existing transition; cold-start-nudge spec extension recorded as follow-on.

## 9. M6 — Soak, docs, CI gates

### 9.1 Soak + boundary tests

- `tests/network/soak.rs`, `#[ignore]`d, nightly, `--release`: 2p + 4p sessions, mild chaos,
  spectator attached, periodic hot-join every 100k frames (hot-join build variant),
  `DesyncDetection::On{60}`, **4,000,000 confirmed frames** (~18.5 virtual hours; tune N from the
  measured ~11.7 s / 40k-frame-equivalent datum). Assert every 50k frames:
  container boundedness vs configured caps — `local_checksum_history ≤ max_checksum_history`
  (bound verified at `p2p_session.rs` ~`:9618`), `pending_checksums` (`protocol/mod.rs`
  ~`:2512-2524`), `event_queue ≤ max_event_queue_size`, `pending_output ≤ pending_output_limit`,
  high-water marks plateau post-warmup (the empirical audit of `// alloc-bound:` promises);
  Linux-only `/proc/self/statm` RSS growth <5% per virtual-hour post-warmup (cfg-gated; no
  global-allocator hacks — unsafe is forbidden); zero Error+ violations via `CollectingObserver`;
  monotone-progress and replay recording intact (`take_replay`).
- Stale-checksum-window red test (open question from analysis): entries older than every peer's
  compare cursor are removed only by the size cap. Write the failing test asserting pruning below
  `min(last_compared_frame across peers)` (interaction with `last_verified_frame`,
  `p2p_session.rs` ~`:6887`), then decide: prune-below-min-compared or document the cap as the
  contract. Also cover the skipped-insert/stale-prune arm (`check_checksum_send_interval` when
  the cell has no checksum).
- i32 frame-boundary unit test (in-src test module, where private state is reachable): start a
  `SyncLayer`/session near `i32::MAX - max_prediction - k`, advance, assert `safe_frame_add!`
  saturates with `ArithmeticOverflow` violations, no panic, no `SavedStates` modulo corruption,
  loud degraded behavior. Document the ~1.14-year @60fps headroom in the production checklist.

### 9.2 Docs (each: markdownlint --fix, link check, doc-claims check, wiki mirror + SYNC header, mkdocs nav)

- `docs/production-checklist.md`: determinism audit (SyncTestSession in CI, `cargo tree` feature
  audit for `ahash`/`const-random`), desync detection on + interval guidance, violation observer
  wired, metrics polled/exported, disconnect/timeout presets, frame headroom, replay recording,
  WASM caveats.
- `docs/desync-playbook.md`: on `DesyncDetected` capture `SyncHealth`,
  `peer_checksum_mismatch_count`, `SessionMetrics`/`PeerMetrics` JSON, violations JSON;
  `take_replay()` → `SyncTestSession`/replay-validation repro; one-off vs persistent mismatch
  triage; issue template.
- `docs/reconnect-playbook.md`: `NetworkInterrupted`/`NetworkResumed`/`Disconnected` handling,
  `DisconnectBehavior` options, hot-join rejoin flow + expected `HotJoinMetrics` timings.
- `docs/tuning.md`: sweep-data tables — (RTT, loss) → recommended (input_delay, max_prediction,
  preset) → expected rollbacks/100 frames, bandwidth/player, confirmation-lag p99; every table
  cites the sweep artifact (git SHA + schema).
- `docs/telemetry.md` + `NetworkStats` rustdoc corrections (kbps now wire-exact; ping cadence).
- `examples/ex_game` metrics overlay (macroquad `Game::render`, `graphical-examples` feature):
  ping, kbps, rollbacks/s, max depth, confirmation lag, stall count, per-peer sync health.

### 9.3 CI perf gating

Split `ci-benchmarks.yml`: µs-scale benches (`p2p_session`, `sync_layer` groups) →
`fail-on-alert: true` at 130–150% (currently `150%`/`fail-on-alert: false` at ~`:140-145`);
ns-scale (`input_queue`, `compression`) stay informational. The deterministic sweep-counter
baseline (§5.3) remains the hard gate. Verify the gate trips with an intentionally-regressing
draft PR. Explicitly defer iai/instruction-count (extra toolchain, misses socket/std paths);
revisit only if criterion proves too noisy over a month.

### 9.4 Recorded follow-ons (out of scope; do not start without maintainer approval)

TLA+ trace validation (CCF-style) and a spec extension covering the cold-start nudge;
`ChaosSocket`/`SimNet` consolidation (division of labor documented instead: `ChaosSocket` =
user-facing per-socket tool, `SimNet` = whole-mesh test fabric); spectator/hot-join subsystem
simplification (M4 decision point); packet-auth feature (flags bit 0).

## 10. Program-level acceptance criteria

- Nightly fleet green over a sustained window at **N ≤ 16** (raised from 8 by §6.6-pre), with
  residual pins documented — including runs with per-peer clock skew, WaitRecommendation-obeying
  app models, and the reliable-FIFO (TCP-model) link profile.
- Defects **D1–D12** all closed or explicitly dispositioned with regression tests
  (D6, D8 done; D11 may close as "bounded + documented" with the §16 experiments as evidence).
- Every §13 hypothesis executed to a verdict (confirmed → fixed+pinned; falsified → recorded).
- Corpus non-empty (fleet demonstrably finds real bugs — else run a planted-bug drill).
- Baselines + measured tuning tables published **including the N=16 columns**; protocol v1
  shipped as 0.10.0 with golden-suite + hook enforcement; threat model + transport guidance
  published.
- Every changelog claim backed by a test or a measured artifact.

---

# Part II — Distributed-systems audit (2026-07-03)

Adversarial second-pass audit: three targeted code audits (16-player scaling, error/degraded
paths, transport + timing loops), cross-verified, plus a literature sweep. Part II adds no new
standalone milestones except §17/§18 items explicitly marked "post-M6"; everything else is
integrated into Part I via the amendment blocks (§5.4, §6.6-pre, §8.5b).

## 11. Audit verification ledger (trust, but verify — both directions)

Claims from the audit agents that were **verified true** at file:line: D9 (event discard, both
sessions), D12 gaps, packet-size formula (§14), O(N²) folds, the `decode_message`
consumed-length/trailing-bytes contract, absent WaitRecommendation handling in harness+`ex_game`,
shared-TestClock skew gap, time-sync constants (window 30 / recommendation interval /
min-recommendation — re-verify exact names at impl).

Claims **falsified during cross-check** (do NOT act on these):

- "`pending_output_limit` default is 16" — that was a fuzz-harness override at
  `protocol/mod.rs` ~`:368`; the real default is **128** (`config.rs:439`).
- "`i16::try_from(clamped).unwrap_or(0)` wrongly clamps frame advantage" — the value is clamped
  to the i16 range immediately before, so the fallback is unreachable (verified benign).
- "NULL `start_frame` + offset decodes as frame 0" — `on_input` rejects invalid `start_frame`
  before any arithmetic (`protocol/mod.rs` ~`:2017`); premise contradicted by the same audit's
  own citation.
- "TCP retransmits deliver duplicate Inputs" — TCP dedups in the stack; applications never see
  duplicate payloads. (UDP duplicates are already an explicitly tolerated wire shape — the
  nudge relies on it.)
- "The harness N≤8 cap suggests developers knew of N>8 bugs" — the cap is an M1 artifact of
  this very program, three weeks old. Historical inference invalid.
- The 16-player audit's event-rate arithmetic used invented event names and a per-frame
  `DesyncDetected` rate; the *conclusion* (overflow within ~2 polls under churn) survives with
  corrected inputs, the specific numbers do not.

Method note for future audits: agent-reported `file:line` cites were ~90% accurate; every
load-bearing claim still gets an independent `rg`/read before entering the defect registry.

## 12. Failure-mode taxonomy v2 (with the metastability lens)

[Metastable failures (HotOS'21/OSDI'22)](https://sigops.org/s/conferences/hotos/2021/papers/hotos21-s11-bronson.pdf):
a trigger pushes the system into a degraded state that a **sustaining feedback loop** maintains
after the trigger is gone. D8 was exactly this (trigger: startup loss; sustaining effect:
input-idle ⇒ no gossip carrier). The heal+drain window in every fleet schedule is the
metastability probe: *any* failure to recover after `HealAll` is a candidate sustaining loop.

Candidate sustaining loops in Fortress (each is a §13 hypothesis):

| FM | Loop | Trigger | Sustaining effect |
| --- | --- | --- | --- |
| FM-1 | Rollback storm | RTT/loss spike | deeper rollbacks ⇒ more CPU/frame ⇒ longer poll gaps ⇒ inflated measured RTT ⇒ deeper prediction ⇒ more rollback work (capacity-degrading amplification) |
| FM-2 | Retransmit-batch fragmentation | loss burst | unacked batch grows ⇒ Input packet > MTU ⇒ IP fragmentation ⇒ higher effective loss ⇒ batch grows (workload amplification) |
| FM-3 | Mutual sleep oscillation | any advantage transient | symmetric undamped controllers with ~0.5 s measurement delay ⇒ both sleep ⇒ both ahead ⇒ repeat |
| FM-4 | Sync-storm starvation | N=16 cold start / mass reconnect | 240 endpoint handshakes × retries vs 256/poll receive cap ⇒ replies dropped ⇒ more retries |
| FM-5 | Event-queue flood blindness | churn wave | overflow discards `Disconnected`/`DesyncDetected` (D9) ⇒ app doesn't react ⇒ churn persists ⇒ more events |

Structural failure classes (beyond loops): silent-loss paths (D9, replay-partial), wrong-fallback
paths (D10; audit found the rest clean or guarded), sentinel-arithmetic hazards (audited: the
checked/valid-guard discipline holds; keep it a review checklist item), decodable-but-hostile
wire values (D12), stagger-vs-ring capacity (H-RING), and control-loop bias (D11).

## 13. Hypothesis registry (falsifiable; each gets a red/green verdict)

Execution home: M3 fleet/census unless noted. Every experiment states its falsifier.
**Executed verdicts to date are recorded in §21** (H-16P first data, H-TCP-lite green,
H-EVLOSS confirmed, H-DISC-RACE falsified).

| H | Hypothesis | Experiment (falsifier) |
| --- | --- | --- |
| H-OSC | The symmetric, undamped time-sync loop oscillates (~1 s period) once apps actually obey `WaitRecommendation`. | 2p+4p sims, symmetric 100 ms delay, app model `Obey` (§6.6-pre.3); record advantage time-series; fail if sustained periodic sleep/advance alternation above amplitude threshold after warmup. Green = loop is damped enough; document why. **Precondition infra ✅ (PR #201): `AppModel::Obey` closes the loop. First data (ASYMMETRIC delay, one-sided obey — NOT the symmetric case): well-damped, obeying costs ~0-2 frames/900 steps, no divergence/stall. SYMMETRIC-delay experiment (the real H-OSC/FM-3 mutual-sleep case) + advantage time-series still OWED.** |
| H-SKEW | 0.1% per-peer clock-rate skew causes unbounded confirmed-lag creep or chronic one-sided `WaitRecommendation`s (~43 frames/hour uncorrected). | Per-peer rate wrappers (§6.6-pre.2), hour-equivalent virtual run; measure drift absorption; fail if lag grows without bound or recommendations become one-sided permanent. **Precondition infra ✅ (PR #202). Consistency-under-skew ✅ (PR #203): a long +0.1% run stays byte-consistent + live. But H-SKEW itself is NOT YET EXECUTED — STILL OWED. CRITICAL (adversarial finding, PR #203): the DST harness advances every peer ONE frame per step (lockstep) and reads the clock only for timestamps, so the RATE-drift mechanism (fast clock ⇒ faster frame-production ⇒ ~43 frames/hour accumulation) is STRUCTURALLY ABSENT — `floor(half_rtt·fps/1000)` stays 1 from 0% through ~+11%, so `average_frame_advantage` never reaches `MIN_RECOMMENDATION` at any ppm. Testing H-SKEW REQUIRES a skew-gated frame model (advance each peer at a rate driven by ITS OWN skewed clock). Do NOT re-attempt a lag/recs "falsification" on the lockstep harness — it is a tautology.** |
| H-ASYM | Asymmetric one-way delay (10/200 ms) biases frame-advantage ≈ +6 frames on one side (D11) causing measurable throughput asymmetry. | SimNet asymmetric links + Obey app model; measure per-side stall totals vs symmetric control run. |
| H-META-RB | FM-1 rollback storm is self-sustaining after the RTT spike ends. | Inject 2 s RTT spike storyline; after heal, assert rollback-rate returns to baseline within B steps (uses M2 rollback histogram). Expected green (bounded window should self-limit) — proving that is the point. |
| H-FRAG | FM-2: at N=16 with 32 B inputs, loss ⇒ batch growth ⇒ >1472 B packets ⇒ fragmentation-amplified loss ⇒ runaway until stall. | Fragmentation emulation (§6.6-pre.5) + burst-loss storyline; watch packet-size + pending-len metrics; fail if pending never re-drains post-heal. |
| H-EVLOSS | D9 reachable in realistic churn: an 8-peer wave with default queue size loses a `Disconnected` or `DesyncDetected` with zero telemetry. | Deterministic churn schedule + slow-drain app model; assert loss observed (red today) then M2 telemetry fires (green). **✅ fleet coverage landed (PR #204, §6.6-pre.6h):** `starve_events` app model overflows the bounded queue → D9 fires (≈25 discards, N=8), draining stays clean; both directions + `expect_pass`. (A *churn*-driven variant — losing a specific `Disconnected`/`DesyncDetected` under a natural wave rather than cold-start sync — is the remaining nuance.) |
| H-RING | >128-frame receipt stagger before a drop silently corrupts the frozen value or desyncs (ring eviction), beyond the changelog's documented fail-safe. | Census schedule already planned (§6.4) — extend to N∈{8,16} and assert byte-level frozen-value agreement. |
| H-POLLCAP | FM-4: at N=16 mass-sync, the 256/poll receive caps starve some endpoints' handshakes (unfair drop ordering ⇒ long sync tail). | 16-peer cold-start schedule; measure per-endpoint time-to-Running distribution; fail if tail > k× median attributable to cap (instrument cap-hit counter). |
| H-TCP | Under a reliable-FIFO transport profile with HOL retransmit-delay bursts, the protocol stays correct (stalls, never diverges) and recovers within bound. | §6.6-pre.4 profile; expected green; pins the TCP story with data. |
| H-16P | A 16-player mesh under Mild noise sustains confirmation progress within frame budget on commodity CPU (O(N²) folds ≈ 0.25–1 ms/frame claim). | N=16 budget probe + sweep row; measure fold time via M2 metrics/criterion micro-bench; fail if frame budget share > agreed threshold (pick after first measurement). |
| H-DISC-RACE | The `event_count_before == len()` fallback-emission heuristic in failing disconnect paths can skip a `Disconnected` when the overflow trim fires concurrently (low priority: the path also enters fail-closed state). | Unit test forcing overflow during `disconnect_player_with_policy` failure; verify or falsify; fix trivially if real (`p2p_session.rs` ~`:9461-9516`). |

## 14. W-SCALE — 16-player readiness (quantified cliffs)

Verified formula: `Input packet = 23 + 7N + 8 + width × pending` bytes (header 2+4, bool+2
frames, status 7 N, length prefix 8). Key rows (worst-case incompressible delta):

| N | pending | width | bytes | vs 508 ideal | vs 1472 frag threshold |
| --- | --- | --- | --- | --- | --- |
| 2 | 128 | 4 | 557 | 110% | ok |
| 16 | 1 | 4 | 147 | 29% | ok |
| 16 | 128 | 4 | 655 | 129% | ok |
| 16 | 128 | 32 | 4231 | 833% | **~3 fragments** |

Cliffs, ranked (each → a task): (1) fragmentation at N=16 × wide inputs × loss (H-FRAG; MTU
guard §8.5b; consider batch-splitting to the frag threshold — sender already splits to the
receive-cap budget, extend to MTU); (2) O(N²) confirmed/disconnect folds ×2+ per poll
(H-16P; if measured hot: cache per-endpoint minima, incremental fold); (3) ring eviction under
stagger (H-RING); (4) event-queue overflow (D9); (5) 256/poll caps at sync storms (H-POLLCAP;
consider fair-share draining per endpoint if confirmed); (6) connect-status wire growth
(compaction, §8.5b); (7) floor-round hold latency at relay topologies (measure via new sweep
columns; accept or redesign with data). Spectator fanout is O(S×N) sends/frame — add an S×N
sweep row before optimizing. Prior art: full-mesh rollback is conventionally ≤8 players;
16 in mesh is beyond GGPO-lineage practice — if cliffs 1/2/5 resist mitigation, the honest
answer may be the §18 relay/star topology as a supported mode rather than heroic mesh tuning.

## 15. W-TRANSPORT — TCP/QUIC/WebTransport (summary; tasks live in §8.5b + §6.6-pre.4)

Contract stays datagram-shaped ("unordered, unreliable" — correct). Ship the framing helper +
`TCP_NODELAY`/HOL/Nagle docs + WebRTC channel-mode guidance + QUIC RFC 9221/WebTransport
recommendation; test via the reliable-FIFO profile (H-TCP). Explicit non-goals: exploiting
ordering for optimization; a bundled TCP transport implementation (adapter recipe + helper only,
unless demand proves otherwise).

## 16. W-TIME — control-loop hardening (data before design)

Order is deliberate: instrument (M2 metrics + §6.6-pre app model/skew clocks) → run H-OSC /
H-SKEW / H-ASYM → only then design damping. Candidate mechanisms to evaluate against the
recorded traces (NOT to implement blind): hysteresis (separate sleep/wake thresholds),
EWMA smoothing of advantage, asymmetry-breaking (only the peer with the *larger* measured
advantage sleeps; deterministic tiebreak by handle), recommendation-size cap, and clamping
advantage influence to the prediction window. `ex_game` gains a WaitRecommendation handler
(M6 docs milestone) so the reference client stops teaching the open-loop anti-pattern. D11
(RTT/2) is documented as a bounded measurement-model limitation with the H-ASYM data attached;
one-way-delay estimation (timestamp exchange) is a §18 idea, not a commitment.

## 17. W-SEARCH — search-power upgrades for the fleet (research-backed)

1. **Violation-path coverage** (Aspirator lens, [OSDI'14](https://www.usenix.org/conference/osdi14/technical-sessions/presentation/yuan):
   92% of catastrophic failures = mishandled non-fatal errors; 35% trivially wrong handlers):
   enumerate `report_violation!` sites (a build-script or test-time registry), record which fire
   across the whole test suite + fleet, publish the *unexercised* list — every unexercised
   violation site is an untested error handler. Gate: ratchet the count downward in CI.
   (M6-adjacent; cheap, high yield.)
2. **Sometimes-assertions** ([Antithesis practice](https://antithesis.com/docs/best_practices/sometimes_assertions/)):
   annotate rare-but-must-happen states (rollback depth = max, floor-round consumed, nudge
   fired, sparse checkpoint search, ring near-wrap) as fleet-visible "sometimes" probes; the
   nightly report asserts each fired at least once per week — coverage of *interesting* states,
   not lines. Implement as a tiny counter registry piggybacked on M2 metrics. (M3.)
3. **Model-state coverage feedback** ([model-guided fuzzing, ASPLOS'24](https://arxiv.org/html/2410.02307v2);
   [Mallory, CCS'23](https://dl.acm.org/doi/10.1145/3576915.3623097) — 54% more states than
   Jepsen): define abstract state = per-endpoint `ProtocolState` × per-slot
   (connected, disconnected-pending, mesh-agreed, frozen) × fold-binding-cause (receipt vs
   gossip vs floor-hold); hash into a fleet-wide seen-set; nightly biases new seeds toward
   schedule prefixes that discovered new abstract states (simple evolutionary loop over the
   corpus — no Q-learning needed initially). The TLA+ specs define the abstraction for free.
   (Post-M3 fleet upgrade.)
4. **LDFI-lite** ([lineage-driven fault injection, SIGMOD'15](https://dl.acm.org/doi/10.1145/2723372.2723711)):
   SimNet already journals every delivered copy; record which deliveries *supported* each
   confirmation (first input arrival per frame/slot, gossip merges that raised folds); generate
   targeted drop-sets that remove minimal support instead of random loss — order-of-magnitude
   fewer runs to find liveness holes like D8. Start greedy (drop the k critical deliveries),
   graduate to SAT only if greedy plateaus. (Post-M3.)
5. **Planted-bug drills**: quarterly, reintroduce a known-fixed bug (D8 revert) behind a cfg and
   verify the fleet finds it within the nightly budget — measures detector health, not the code.

## 18. Ideas parking lot (explicitly speculative — promote only with data)

- **Merkle/hierarchical state checksums**: user provides per-subsystem checksums; on desync the
  library bisects which subtree diverged first — turns "desync at frame N" into "entity system X
  diverged at frame N". API sketch: `Config::State: SubtreeChecksums` opt-in.
- **Adaptive input-delay controller**: auto-tune input delay from measured jitter/rollback-depth
  histograms (Skullgirls-style sliding scale, automated). Needs H-OSC data first — two adaptive
  controllers can fight.
- **Relay/star topology mode** for N>8: optional relay (dumb reflector with timestamps) so each
  peer sends once — kills O(N²) send fanout and most gossip-staleness classes; keeps rollback
  semantics. Big design doc; the honest path to 16+ if §14 cliffs resist.
- **Epidemic gossip overlay** for connect-status at large N (random peer subset per packet
  instead of full vector) — pairs with compaction (§8.5b).
- **Input FEC**: XOR parity of last k input frames in each packet — recovers single losses
  without retransmit RTT; bounded overhead; measure vs existing redundant-batch behavior first
  (pending_output already re-sends unacked history — quantify overlap before adding).
- **One-way-delay estimation**: piggyback send-timestamps + drift-corrected offset tracking
  (NTP-lite) to replace RTT/2 (addresses D11 properly; large validation burden).
- **TLC-trace → schedule compiler**: compile TLA+ counterexample/coverage traces into SimNet
  schedules — model-based test generation bridging specs and fleet (pairs with §17.3).
- **Spec extension**: TLA+ model of the gossip-carrier liveness property that D8 violated
  (currently only in code + tests).
- **jitter-buffer-style confirmed-input pacing for spectators** (smooth catch-up instead of
  `catchup_speed` bursts).

## 19. Part II bibliography

- [Metastable Failures in Distributed Systems (HotOS'21)](https://sigops.org/s/conferences/hotos/2021/papers/hotos21-s11-bronson.pdf) / [Metastable Failures in the Wild (OSDI'22)](https://www.usenix.org/system/files/osdi22-huang-lexiang.pdf)
- [Simple Testing Can Prevent Most Critical Failures (OSDI'14)](https://www.usenix.org/conference/osdi14/technical-sessions/presentation/yuan)
- [Lineage-driven Fault Injection (SIGMOD'15)](https://dl.acm.org/doi/10.1145/2723372.2723711)
- [Model-guided Fuzzing of Distributed Systems (2024)](https://arxiv.org/html/2410.02307v2); [Mallory: Greybox Fuzzing of Distributed Systems (CCS'23)](https://dl.acm.org/doi/10.1145/3576915.3623097)
- [Antithesis: Sometimes Assertions](https://antithesis.com/docs/best_practices/sometimes_assertions/); [How Antithesis Works](https://antithesis.com/product/how_antithesis_works/)
- [RFC 9221: An Unreliable Datagram Extension to QUIC](https://datatracker.ietf.org/doc/html/rfc9221); [WebTransport over HTTP/3](https://ietf-wg-webtrans.github.io/draft-ietf-webtrans-http3/draft-ietf-webtrans-http3.html)
- [SnapNet: Netcode Architectures — Rollback](https://www.snapnet.dev/blog/netcode-architectures-part-2-rollback/); [GGPO](https://www.ggpo.net/); [Choosing the right network model (Glenn Fiedler)](https://mas-bandwidth.com/choosing-the-right-network-model-for-your-multiplayer-game/)

---

# Part III — Second-pass execution session (2026-07-03)

Scope of this pass: execute the cheapest high-value Part II hypotheses to verdicts (with the
harness changes they required), audit TLA+ spec coverage properly (Part II left it as one
parking-lot line), audit the reference client / user-misuse surface, and run a five-topic
research sweep past Part II's bibliography. Everything below was verified or measured in this
session; agent-reported claims that failed verification are recorded in §20 (same discipline
as §11).

## 20. Verification-ledger additions

Re-verified before acting (all true): D9 discard site (`p2p_session.rs:9607-9608`), harness N
cap `(2..=8)` (`tests/simulation/harness/schedule.rs:197`, an M1 artifact), single shared
`TestClock` (`tests/simulation/harness/mod.rs:224`), harness address scheme is N-generic
(`peer_addr`, `harness/mod.rs:170-171`).

Claims from this session's client-audit agent **falsified during cross-check** (do not act):

- "Input type width has no runtime validation" — **false at the library level**. Construction
  validates the default value's encoded width (nonzero, frame-size bounds:
  `protocol/mod.rs:516-530`, `:532-552`), and the send path rejects **every** per-value width
  mismatch with a structured error (`input_bytes.rs:194-207`; regression tests
  `send_input_rejects_*` in `protocol/mod.rs`). Remaining gap is UX/docs only: rejection
  surfaces at send time, not at `SessionBuilder` time, and only the default value is measured
  up front. Folded into §23 as a docs item, not a defect.
- The same agent's `file:line` cites for lib.rs event definitions and several session internals
  did not resolve; its *behavioral* claims about ex_game were spot-verified instead (below).

Verified true: `examples/` contains **zero** matches for `WaitRecommendation` (events are
logged generically; the reference client never branches on the variant) — confirms Part II's
§16 finding independently. No production checklist doc exists yet (owned by M6 §9.2).

## 21. Executed hypothesis verdicts (red/green, with data)

| H | Verdict | Evidence |
| --- | --- | --- |
| H-16P (correctness half) | **GREEN so far** — no correctness cliff at N=16 under Mild noise | §21.1 |
| H-TCP (lite profile) | **GREEN** — reliable-FIFO + HOL stalls: stall-and-catch-up, zero divergence, N∈{2,4,16} | §21.2 |
| H-EVLOSS / D9 | **CONFIRMED** — unit level (safety-critical event lost with zero telemetry, pinned red-doc) **+ fleet level (PR #204):** a starved app model overflows the bounded queue and D9 `events_discarded_*` fires end-to-end, while draining stays clean — starvation is informational-only, mesh stays consistent | §21.3 |
| H-DISC-RACE | **FALSIFIED** — remove from registry | §21.4 |

### 21.1 H-16P first data (harness cap raised to 16)

Changes (this session, uncommitted): `schedule.rs` cap `2..=8` → `2..=16` (+doc, +coverage
test to 16); new `#[ignore]`d probes in `tests/simulation/fleet.rs`:
`probe_smoke_fleet_{twelve,sixteen}_player_mesh_holds_invariants`,
`budget_probe_sixteen_player_long_schedule`.

Measured (debug build, this machine, 2026-07-03):

- N=12 smoke fleet (8 seeds × 600 steps, Mild): **17.97 s** total, all green.
- N=16 smoke fleet: **35.86 s** total (~4.5 s/run vs ~15 ms/run at N≤4 — ~300×), all green.
- N=16 × 5,000-step budget probe (seed 99): **42.87 s** wall (vs 11.7 s at N=8 → **3.7×**,
  matching §6.6-pre.1's ~4× O(N²) prediction); `sent=2,567,360 delivered=2,555,279
  dropped_by_policy=24,256 duplicated=12,190`; **every one of the 16 peers confirmed to frame
  4883/4884** — full liveness through storylines + heal.

Honest attribution caveat: these are *harness* costs (16 sessions + SimNet + an oracle whose
per-step confirmed-input sampling is itself O(N²)), not per-session library cost. The §13
H-16P per-frame budget-share question (fold time on a commodity core) still requires M2
metrics / criterion isolation. What IS answered: no correctness/liveness cliff appears between
N=8 and N=16 on this seed set, so §14's cliffs 2–5 are cost/robustness questions, not
already-broken behavior. Promotion criteria for the probes → PR smoke: §25 A8.

### 21.2 H-TCP-lite (new tests, kept as permanent regressions)

`tests/simulation/fleet.rs::run_tcp_model_mesh`: zero loss, zero dup, fixed 30 ms delay,
**zero jitter** (SimNet per-link FIFO stays exact ⇒ in-order delivery — the TCP contract),
plus three 25-step capture-and-FIFO-release holds (≈400 ms virtual each ≈ 3× the 8-frame
prediction window) modeling head-of-line retransmit stalls.
`tcp_model_reliable_fifo_{two,four}_player_mesh_holds_invariants` run on every PR (0.45 s /
0.09 s); the N=16 variant is `#[ignore]`d (8.2 s, green). Verdict: the protocol never exploits
loss/reordering for correctness; HOL stalls degrade to stall-and-catch-up exactly as §15
predicted — now pinned by tests rather than argument. The full §6.6-pre.4 retransmit-delay
*mode* (drop-triggered burst delay on live noise) still lands in M3; this profile is the
correctness core of H-TCP.

### 21.3 H-EVLOSS / D9 red-documentation test

`src/sessions/p2p_session.rs::tests::event_queue_overflow_discards_disconnected_event_with_zero_telemetry`:
a queued `Disconnected` canary + `max_event_queue_size` benign `Synchronizing` events through
the real `handle_event` path ⇒ canary evicted by the `:9607` trim; a `CollectingObserver`
registered for the whole run captures **zero violations**. The defect is now executable, not
just documented. The final assertion is written to FLIP when M2's discard telemetry lands
(§5.4). Fleet-level reachability under realistic churn (slow-drain app model) remains the M3
oracle item (§6.6-pre.6h).

### 21.4 H-DISC-RACE falsified (structural census)

The hypothesized confounder cannot occur: sessions are single-threaded and the **only**
`event_queue.pop_front()` in the codebase (`p2p_session.rs:9608`) runs at the end of
`handle_event`, *after* the fallback heuristic, in the same invocation. Complete mutation
census: `push_back` ×5 (`:2376`, `:7051`, `:9332`, `:9627`, + `handle_event` arms), user-facing
`drain` (`:5834`), trim `pop_front` (`:9608`). Every error path of
`disconnect_player_with_policy` either pushes nothing before returning (snapshot failure →
fallback fires correctly) or unconditionally pushes `Disconnected` at `:7274` before returning
`Err` — and a fresh push lands at the queue *back*, which the front-evicting trim never
removes. Loss of *older* events is D9 proper, already covered. Removed from the registry.

## 22. W-SPEC — TLA+ spec-coverage audit and spec workstream

Answers Part II's open question "what specs are we missing" (previously one §18 line).

### 22.1 Inventory (verified against `specs/tla/`)

15 specs, 7,610 lines, ~67 safety invariants, 9 liveness properties. Liveness is checked only
in 7 specs (DoubleFailureRelay family, FreezeConvergence, NPeerReactivation,
NPeerServeFreezeConvergence, SpectatorFailover, SpectatorReactivationEpoch, ChecksumExchange's
terminal/monotonic props); **8 specs are safety-only** — notably `TimeSync.tla` (per-endpoint
window only; no control loop), `NetworkProtocol.tla` (1-roundtrip handshake, no loss/reorder/
timeout), `InputQueue.tla` (ring bounds QUEUE_LENGTH=3 vs production 128), `Rollback.tla`
(LIVE-4 disabled for CI budget). `ChecksumExchange` liveness is deliberately disabled
("premises unsound — late IntroduceDesync invalidates"). There is **no refinement or trace
linkage** between specs and code — specs are free-floating with a hand-maintained mapping in
`specs/tla/README.md`; mutation-tested `.cfg` negative variants (`_GateBlind`,
`_InheritedFloor`, …) are the one mechanism keeping properties load-bearing.

Depth is extremely uneven: `DoubleFailureRelay` alone is 1,725 lines + 16 cfgs (the S41–S55
residual), while the sync handshake — the first critical path of every session — has 307
lines with no loss model.

### 22.2 Coverage gaps, ranked

1. **v1 handshake config-check (M5 §8.3)** — a planned breaking wire change with zero
   pre-implementation spec. Highest risk: hard-fail semantics under packet loss/reorder.
2. **v0 sync handshake under loss/reorder/timeout** — `NetworkProtocol.tla` models none of it;
   sync-wedge diagnostics are unverified liveness.
3. **Cold-start nudge liveness** — D8's fix (`gossip_holds_confirmation_below_receipts`) is
   code+tests only; the D8 bug class was found by simulation precisely because no spec owns
   gossip-carrier liveness.
4. **Hot-join attempt/era isolation completeness** — lifecycle is modeled
   (`NPeerReactivation`), cross-attempt frame-value isolation is not.
5. **Connect-status merge under multi-cycle reordering + host failover jointly** —
   `SpectatorFailover` is deliberately single-cycle/in-order; the joint case is unspecified.
6. **Time-sync feedback-loop stability** — no spec closes the loop (advantage → recommendation
   → skip → advantage). Pairs with H-OSC (§13): run the experiments first, spec the loop with
   the observed dynamics.
7. **Event-queue delivery semantics** (D9's home), 8. **pending_output ack/trim liveness**,
8. **desync-detection latency/completeness** (liveness disabled), 10. **input-ring stagger
   >128 frames** (H-RING; spec bounds are toy-sized).

### 22.3 Spec proposals (execute top-down; each ≈ small-bounds TLC-checkable)

- **`SyncHandshakeV1.tla`** (gates M5 — spec BEFORE implementation, new M5 entrance item):
  peers × {phase, syncRemaining, local/learned config, network multiset with loss+reorder,
  timeout counters, terminal `handshake_failed`}. Safety: mismatch ⇒ both sides terminal-fail,
  no partial sync, no silent mismatch, fail-closed is sticky, learned-config injective.
  Liveness: eventually Synced or Failed under fair retransmission; loss never deadlocks.
  Bounds: 2 peers, 3 roundtrips, 2 config fields. Would catch: partial-handshake divergence,
  responder-always-replies-block protocol holes, timeout/wedge asymmetries — before the one
  atomic wire break ships.
- **`ReceiptNudgeProtocol.tla`** (D8 class): receipts/gossip folds/nudge predicate/partitions.
  Safety: fold converge-down never rewinds confirmation; nudge fires only when gossip binds
  below receipts. Liveness: post-heal, confirmation reaches receipts (the exact D8 deadlock
  shape becomes a checked property). Also encodes the §3 lesson (nudge must gate on the
  deadlock signature; reserved endpoints excluded).
- **`HotJoinAttemptEra.tla`**: era monotonicity, no cross-era frame-value re-confirmation,
  clamp-holds-during-pending, stale-directive rejection; liveness: attempts terminate.
- **`SpectatorFailoverExtended.tla`**: multi-cycle × reordered staging × host failover jointly;
  epoch high-water rejects cross-cycle re-arm even when the witness host died.
- **`TimeSyncControlLoop.tla`** (after H-OSC data): closed-loop advantage/recommendation/skip
  with an explicit no-sustained-oscillation temporal property and the §16 damping candidates
  as FIX_MODE variants (reuse the DoubleFailureRelay mutation-cfg pattern).

### 22.4 Trace validation (promoted from §9.4 follow-on to a costed pilot)

Research (§24.2 sources): the 2024–2025 consensus workflow is Merz/Kuppe/Cirstea-style —
instrument to log **variable-update deltas** at protocol-event granularity, write a
`Trace<Spec>.tla` constraining the base spec, run TLC (depth-first — CCF's DFS trick took a
consistency check from an hour to sub-second). Cost datapoint: CCF product engineers
self-served a second protocol's trace validation in ~1 engineer-week once the method existed;
6 bugs found, one *already triggered by existing tests* whose assertions were too weak — trace
validation is a universal strong oracle over tests you already run. Anti-pattern (MongoDB
eXtreme-modelling): trace-checking against a too-abstract spec dies on the abstraction gap —
spec at observable-protocol-event altitude. Fortress advantage: the DST harness is already
deterministic and journals everything; emitting ndjson per-step protocol events from the
harness is trivial compared to what etcd/CCF had to build. **Pilot (post-M3, decision point
with data): `NetworkProtocol.tla` (rewritten per 22.3 gap 2) + harness trace emission for
2-peer schedules.** Also noteworthy: no public mechanized spec of P2P rollback netcode exists
(the only academic formalization — OPODIS 2025 — is client-server, pen-and-paper, no
artifact); its property vocabulary (η-bounded divergence, "special actions") is reusable for
oracle design.

## 23. Reference-client & API-misuse audit (how users will actually hold it)

`ex_game` verdict: a **happy-path teaching artifact**. It correctly demonstrates the loop
skeleton (poll → drain events → add_local_input for every local → advance with matched errors),
save/load handling, `frames_ahead()`-driven fps slowdown, and lockstep-mode awareness. It
never branches on `WaitRecommendation` (verified §20), `DesyncDetected`,
`NetworkInterrupted/Resumed`, and never validates save/load round-trip faithfulness. Users
copy examples; the example teaches an open-loop time-sync client (H-OSC's precondition) and a
log-and-forget desync response.

Ranked user-side failure modes (verified subset; each mapped to an owner):

| # | User error | Library behavior today | Blast radius | Owner |
| --- | --- | --- | --- | --- |
| U1 | Ignores `WaitRecommendation` (as ex_game does) | event only; no telemetry on sustained ignore | rollback storms, stalls, one-sided lag | §16/M6: ex_game handler + docs; metrics `wait_recommendations` (M2) makes ignore measurable |
| U2 | Drains events slower than they arrive | silent overflow discard (D9) | missed `Disconnected`/`DesyncDetected` ⇒ compound divergence | M2 telemetry + M4 retention policy (already planned); §21.3 test pins it |
| U3 | Treats `DesyncDetected` as log-worthy, keeps playing | advisory only; trust-downgrade threshold never auto-ejects | permanent split-brain gameplay | M6 desync-playbook (§9.2) + evaluate `PeerUntrustworthy`-style escalation event (M4 decision) |
| U4 | Unfaithful save/load with `SaveMode::Sparse` | no round-trip validation; divergence surfaces frames later | delayed, hard-to-debug desync | M6 production checklist: round-trip validation pattern + consider `load` frame-match debug assert (parking lot) |
| U5 | Misreads `frames_ahead()` direction | correct but generically named | user speeds up instead of slowing | M6 docs: rustdoc + user-guide clarification (rename is breaking — note only) |
| U6 | Variable-width input values | **rejected at send with structured error** (verified §20) | none (correct) — but error surfaces mid-session | M6 docs: state the send-time contract; builder-time probe of a user-supplied sample set as a parking-lot nicety |
| U7 | `MissingLocalInput` doesn't name the missing handles | generic error | slow debugging in couch-co-op | M4 nicety: include handle list in the structured error |

New doc obligation for M6 §9.2 (production checklist): an explicit **misuse-surface section**
covering U1–U7 with the event-drain contract ("drain every poll"), WaitRecommendation
obligation, and DesyncDetected response recipe. No new library work is *created* by this audit
that Part I/II didn't already own — its value is confirming the priorities and adding U7 + the
checklist structure.

## 24. Research digest (what changed vs Part II's assumptions)

### 24.1 High-player-count practice: N>8 mesh rollback is unattested anywhere

GGPO hard-caps at 4 (`GGPO_MAX_PLAYERS`); Slippi doubles (4-peer mesh) is the largest attested
full-mesh rollback deployment; For Honor's 8-player P2P deterministic sim is the shipped P2P
ceiling — and Ubisoft retired it to dedicated servers; Photon Quantum's "128 players" is a
**server-star input topology** (authoritative input relay + deterministic client rollback),
attested at 32 players shipped (Stumble Guys); MultiVersus/RoA2 run rollback on servers even at
2–4 players. Nothing public at 12–16 mesh, anywhere. Consequences: (a) §21.1's green N=16 runs
put fortress **beyond public practice** — a differentiator only if the nightly fleet + sweep
prove it; docs must state the tested tier honestly (README + production checklist, §25 A7);
(b) §18's relay/star mode is confirmed as the industry-validated fallback if §14 cliffs resist;
(c) there is no prior art to crib from for O(N²) mesh health at 16 — our DST fleet is the only
safety net, which raises the priority of §17 search-power upgrades.

### 24.2 DST & formal methods frontier

Beyond §17's items, ranked by effort/value for our harness: (1) **swarm testing**
(TigerBeetle VOPR default): derive per-run fault-distribution parameters from the seed instead
of fixed noise-class ranges — a `BackgroundNoise::Swarm` variant whose loss/delay/jitter/dup
ranges are themselves seed-drawn (schema field, default off; M3 with §6.1); (2) statistical
scenario-coverage assertions ("this boundary case occurred ≥N times per fleet run") — merges
into §17.2 sometimes-assertions; (3) **buggify!-style code-site fault points**, sampled
per-run not per-call (FoundationDB): e.g. shrink queue caps, delay quality reports, force
spurious floor rounds under a sim-only cfg (M3/M4 decision); (4) **PCT-style message-priority
delivery** in SimNet (random priorities + d−1 inversion points — provable low-depth-bug
probability) as an alternative delivery policy (post-M3); (5) **prefix-replay branching**
(Antithesis-lite): replay a schedule to an interesting step (first deep rollback, first
violation), fork M fault continuations — cheap because schedules are fully deterministic
(post-M3, pairs with §17.4 LDFI). Validated by the research: our corpus stores materialized
*schedules* not seeds — exactly right (seeds go stale on any generator/code change; scenarios
replay); and our `same_schedule_produces_identical_trace` meta-test matches S2's
determinism-regression practice. Trace validation details in §22.4.

### 24.3 Transport 2024–2026

WebTransport is web-Baseline as of Safari 26.4 (2026-03) but **client-server only**; browser
P2P remains WebRTC data channels (`ordered:false, maxRetransmits:0`) indefinitely — the
matchbox path is the mainstream, not legacy. Native adapter priority by attested adoption:
Steam GameNetworkingSockets (transparent relay fallback; GGRS-socket precedent exists) >
iroh 1.0 (P2P QUIC, vendor-reported ~90% direct connections) > WebTransport (relayed
topologies only). Actionable amendments: (a) **universal safe payload budget ≈ 1,200 B**
(Chrome/WebTransport floor, SCTP practice, GNS sweet spot) — §8.5b's MTU guard becomes
dual-threshold: warn ≥1,200 (portability budget), alarm ≥1,472 (fragmentation math); (b) QUIC
datagrams are congestion-controlled and may be **dropped/delayed by the sender's own stack**
(RFC 9221 §5) — the `NonBlockingSocket` contract docs must say "send is best-effort and may
drop locally; the protocol's redundant-window tolerates this"; (c) 5–20% of real P2P sessions
relay at +20–80 ms RTT — add a relay row to the §9.2 tuning tables. TCP remains disqualified
as a *recommended* transport (HOL: freshness beats completeness) — consistent with §21.2:
correct-but-degraded, exactly what we now pin by test.

### 24.4 Input FEC idea: rejected with evidence (update §18)

The redundant unacked-input window **is** a repetition code and is strictly stronger against
burst loss than k+1 XOR parity (parity recovers one loss per group; the window survives any
pattern until its newest packet). Every attested implementation (GGPO, INVERSUS, Overwatch's
input window, Fiedler's lockstep, GGRS) converged on redundancy + delta coding; none ship
input FEC; Google removed XOR-parity FEC from QUIC in 2016 after it underperformed on bursty
loss. §18's "Input FEC" item is demoted to *rejected — do not implement*. The attested
headroom is finer **delta coding**: GGPO encodes ~1 marker bit per unchanged frame /
~6–7 bits per changed button vs our byte-aligned XOR+RLE — evaluate ONLY if M2 sweep data
shows input bytes dominating bandwidth (it feeds the same §8.5b wire-break window). Entropy
coding: unattested in any shipped input path; rejected here for decoder attack surface.

### 24.5 Time-sync oscillation: the fork inherited a loop measurably weaker than GGPO's

The strongest research finding of the pass. GGPO's timesync (source-verified) ships five
anti-oscillation safeguards; GGRS dropped or weakened four, and **fortress inherits GGRS's
values** (verified in-tree this session):

| Safeguard | GGPO | GGRS / fortress (verified) |
| --- | --- | --- |
| Averaging window | 40 frames | 30 (`time_sync.rs:7`) |
| Recommendation cooldown | **240 frames (~4 s)** | **60 frames (~1 s)** (`p2p_session.rs:39`) |
| Skip cap | `MAX_FRAME_ADVANTAGE = 9` | **uncapped** (`frames_ahead = max_frame_advantage()`, `p2p_session.rs:9138`; `time_sync.rs:327` test shows 40 passing through) |
| Dead-band | 3 frames | 3 (`MIN_RECOMMENDATION`, `p2p_session.rs:47`) — kept |
| Agreement guard ("only if both agree who's ahead") | explicit `if (advantage >= radvantage) return 0` | implicit only, via the `/2` formula sign |

Why the cooldown delta matters (inference, to be tested by H-OSC): a correction's effect is
observable only after RTT + ≤200 ms quality-report latency + a 30-frame window flush
(≈0.5–1 s) — GGRS's 1 s cooldown permits a second correction before the first is measurable;
GGPO's 4 s does not. The uncapped skip amplifies any such double-correction.

Production oscillation evidence & the attested mitigation toolbox: NRS GDC 2018 (slide 55,
verbatim): first MKX beta telemetry showed "most matches ended up constantly rolling back the
maximum … effectively a performance feedback loop", fixed by slowing **only the ahead player**
via continuous frame-lengthening (16.6→≤20 ms tick), because "net-pauses felt MUCH worse than
bogging". Slippi (in-code comments): dead-band deliberately at 60% of a frame *"because I was
worried about two systems continuously stalling for each other"*; asymmetric P-controller
(+1%/−0.5% speed saturation); and a warning that its min-across-peers aggregation *"biases
negative as the peer count grows"* — direct precedent that **the aggregation function is where
3+-player timesync goes wrong** (we take max across endpoints, same family of bias, opposite
sign). INVERSUS: plant gain is 2 (one stalled sim frame moves input advantage by two — the
principled reason for the `/2` damping), 0.75-frame threshold vs the theoretical 0.5 for
stability, decisions every 100 frames, stalls smeared one-at-a-time at escalating cadence.

**§16 amendments (A10):** (a) H-OSC's experiment matrix must include the two inherited deltas
as controlled variables: cooldown ∈ {60, 240} and skip cap ∈ {none, 9} — cheap constants to
sweep in the harness once the Obey app model exists; (b) the candidate-mechanism list gains
attested designs: continuous rate-modulation (NRS/Slippi) as the alternative to discrete
sleeps, explicit GGPO-style agreement guard, threshold-above-balance-point hysteresis
(Slippi 0.6 / INVERSUS 0.75), correction smearing (INVERSUS cadence); (c) H-OSC gains a
sub-hypothesis H-OSC-AGG: max-across-endpoints aggregation systematically over-corrects as N
grows (Slippi documents the min-side twin) — measure per-endpoint vs aggregated advantage
distributions at N∈{2,8,16} in the §16 traces; (d) `ex_game`'s future WaitRecommendation
handler should smear skips (1 frame per k) rather than sleep N at once — teaching the
INVERSUS pattern, not the net-pause pattern.

## 25. Amendments & new work items (integrated into Part I/II milestones)

- **A1 (done this session, uncommitted):** harness cap 2..=16 (`schedule.rs`), N∈{12,16}
  `#[ignore]`d probe fleets + N=16 budget probe, TCP-model tests (N∈{2,4} on PR, 16 probe),
  D9 red-doc unit test. Files: `tests/simulation/harness/schedule.rs`,
  `tests/simulation/fleet.rs`, `src/sessions/p2p_session.rs` (test module only). Full suite,
  clippy, fmt green. No changelog entries (test/infra only, no pub surface).
- **A2 (M2/M5):** MTU guard dual-threshold 1,200/1,472 (§24.3a supersedes §8.5b's single
  1,472 figure).
- **A3 (M3):** `BackgroundNoise::Swarm` seed-drawn fault distributions (§24.2.1; schema bump
  rules per §6.6-pre.2).
- **A4 (M3/M4 decision):** buggify-site facility under a sim-only cfg; start with 3 sites
  (queue-cap shrink, quality-report delay, forced floor round).
- **A5 (post-M3, with §17):** unified fleet search-power ordering: swarm → scenario-coverage
  stats (§17.2) → violation-path coverage (§17.1) → PCT delivery → prefix-replay branching →
  LDFI-lite (§17.4).
- **A6 (M5 entrance gate — NEW):** `SyncHandshakeV1.tla` written and TLC-green **before**
  §8.3 implementation starts; §8.0 red tests then encode the spec's terminal states.
- **A7 (M6):** docs additions — tested-player-tier statement (README + production checklist:
  "mesh validated to N=16 in simulation; ≥8-player meshes are beyond attested industry
  practice — measure your game"), relay-latency tuning row, `NonBlockingSocket` best-effort
  send clause, `frames_ahead()` direction clarification, misuse-surface checklist (U1–U7).
- **A8 (M3→M6):** promote `probe_smoke_fleet_sixteen_player…` + `tcp_model…sixteen…` to
  non-ignored once (i) nightly fleet green ≥1 week at N≤16 and (ii) a `--release` PR-budget
  measurement puts them under ~10 s combined; record both numbers in this file when done.
- **A9 (§18 updates):** Input-FEC entry → rejected (§24.4). Relay/star mode entry gains the
  §24.1 evidence line (industry-validated path >8). Add: per-changed-bit input delta encoding
  (GGPO-style) as a data-gated candidate inside the same M5 wire window.
- **A10 (§16 amendments):** time-sync research integration per §24.5 — H-OSC sweeps
  cooldown ∈ {60, 240} and skip-cap ∈ {none, 9}; H-OSC-AGG sub-hypothesis (aggregation bias
  at N≥3); attested mechanisms (continuous rate modulation, agreement guard,
  above-balance-point hysteresis, correction smearing) join the §16 candidate list;
  `ex_game` handler smears skips.

## 26. Program-level acceptance criteria deltas (extends §10)

- M5 may not start §8.3 implementation before A6's spec is TLC-green (spec-first for the one
  atomic wire break).
- Post-M3 decision point: trace-validation pilot go/no-go with the §22.4 cost data in hand.
- 0.10.0 release notes must state the tested topology/player tier per A7.
- §13 registry bookkeeping: H-DISC-RACE closed-falsified (§21.4); H-TCP's correctness core
  closed-green (§21.2, full noise-mode variant still M3); H-EVLOSS confirmed-red awaiting M2
  flip (§21.3); H-16P correctness half green, cost half open (§21.1).

## 27. Part III bibliography (additions)

- [Formalizing Rollback Netcodes (OPODIS 2025)](https://drops.dagstuhl.de/storage/00lipics/lipics-vol361-opodis2025/LIPIcs.OPODIS.2025.11/LIPIcs.OPODIS.2025.11.pdf)
- [Validating Traces of Distributed Programs Against TLA+ Specs (Cirstea/Kuppe/Loillier/Merz 2024)](https://arxiv.org/abs/2404.16075); [etcd-raft TLA+ trace validation](https://github.com/etcd-io/raft/tree/main/tla); [Smart Casual Verification of CCF (NSDI'25)](https://www.usenix.org/system/files/nsdi25-howard.pdf); [eXtreme Modelling in Practice (MongoDB, VLDB'20)](https://arxiv.org/abs/2006.00915); [Runtime Protocol Refinement Checking (NSDI'25 reproduction)](https://medium.com/princeton-systems-course/runtime-protocol-refinement-checking-for-etcd-raft-a2cb4710c3b4)
- [TigerBeetle VOPR](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/internals/vopr.md); [A Tale of Four Fuzzers (TigerBeetle 2025)](https://tigerbeetle.com/blog/2025-11-28-tale-of-four-fuzzers/); [FoundationDB BUGGIFY](https://apple.github.io/foundationdb/testing.html); [PCT (ASPLOS'10)](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/asplos277-pct.pdf); [S2 DST](https://s2.dev/blog/dst); [Eaton: DST notes](https://notes.eatonphil.com/2024-08-20-deterministic-simulation-testing.html)
- [GGPO input encoding (udp_proto.cpp)](https://github.com/pond3r/ggpo/blob/master/src/lib/ggpo/network/udp_proto.cpp); [Fiedler: Deterministic Lockstep](https://gafferongames.com/post/deterministic_lockstep/); [INVERSUS rollback](http://blog.hypersect.com/rollback-networking-in-inversus/); [QUIC-FEC (arXiv:1904.11326)](https://arxiv.org/abs/1904.11326)
- [For Honor P2P→cloud (AWS)](https://aws.amazon.com/blogs/gametech/for-honor-friday-the-13th-the-game-move-from-p2p-to-the-cloud-to-improve-player-experience/); [Photon Quantum server/client roles](https://doc.photonengine.com/quantum/v2/concepts-and-patterns/server-client-role); [Slippi doubles](https://www.gamespot.com/articles/super-smash-bros-melee-mod-slippi-now-supports-doubles-play-thanks-to-latest-update/1100-6490938/)
- [WebKit: Safari 26.4 (WebTransport)](https://webkit.org/blog/17862/webkit-features-for-safari-26-4/); [matchbox](https://github.com/johanhelsing/matchbox); [Steam Datagram Relay](https://partner.steamgames.com/doc/features/multiplayer/steamdatagramrelay); [iroh 1.0 / noq](https://www.iroh.computer/blog/noq-announcement); [RFC 9221 §5 (datagram CC)](https://www.rfc-editor.org/rfc/rfc9221.html)
- [GGPO timesync.cpp (safeguard constants)](https://github.com/pond3r/ggpo/blob/master/src/lib/ggpo/timesync.cpp); [NRS GDC 2018: 8 Frames in 16ms (slides)](https://www.gdcvault.com/play/1025471/8-Frames-in-16ms-Rollback); [Slippi EXI_DeviceSlippi.cpp (P-controller + comments)](https://github.com/project-slippi/Ishiiruka/blob/slippi/Source/Core/Core/HW/EXI_DeviceSlippi.cpp); [GGRS time_sync.rs](https://github.com/gschup/ggrs/blob/main/src/time_sync.rs)

---

# Part IV — Partition/CAP, Byzantine-lite, and degradation-model audit (2026-07-03, same session)

Prompted by the maintainer's question: network degradation modes, Byzantine behavior, and
CAP-class properties not yet covered. Method as before: verify at file:line, run the cheap
experiments now, register the rest as specified work.

## 28. Partition semantics — the CAP audit (new defect D13; experiments executed)

### 28.1 The framing

`DisconnectBehavior` **is the CAP dial**, per policy: `Halt` = consistency over availability
(fail closed on partition), `ContinueWithout` = availability over consistency (the mesh
*forks*). Neither had ever been exercised by a whole-mesh test: the M1 fleet's storylines
deliberately stay under the 2 s disconnect timeout, and the harness always built sessions with
the builder default (`Halt`) — **the fleet has never executed a disconnect** (verified: zero
`DisconnectBehavior`/`with_disconnect_behavior` references in `tests/simulation/harness/`
before this session). PACELC completes the picture: else (no partition), the system trades
latency for consistency via the prediction window — bounded staleness with rollback repair.

Harness change (this session): `SimConfig.disconnect_behavior: DropPolicy`
(`#[serde(default)]` = Halt — old corpus artifacts replay unchanged; test-side serde mirror of
the production enum), threaded to `SessionBuilder::with_disconnect_behavior`.

### 28.2 Experiment: symmetric split-brain (4-mesh → {0,1} × {2,3}, 8.8 s > 2 s timeout)

Two permanent tests in `tests/simulation/fleet.rs` (`split_brain_schedule`):

- **`ContinueWithout` — the fork, pinned as documented behavior**
  (`partition_under_continue_without_forks_into_divergent_halves`, green first run): each half
  drops the other, freezes their slots, keeps confirming on its own timeline
  (all four peers final-confirm > 200 — availability per partition); the halves diverge
  permanently (dropped endpoints are terminal — healing the network can never re-merge);
  and the fork is **invisible to in-band desync detection** (zero `InbandDesyncDetected` —
  ghost traffic from dropped endpoints is discarded). The application's only signal is the
  `Disconnected` events at drop time. Consequences: (a) the docs must say plainly that
  `ContinueWithout` under partition forks the match and the halves can never rejoin without
  a fresh session/hot-join; (b) **quorum awareness** becomes a real API idea — see 28.4.
- **`Halt` — NOT fully fail-closed: defect D13**
  (`partition_under_halt_confirms_fabricated_frames_divergently_d13`, red-documentation).
  Halt's rustdoc promises "no further frames advance once any peer drops"
  (`src/sessions/config.rs`, `DisconnectBehavior::Halt`). Observed: when the disconnect lands,
  dropped slots leave the confirmation fold and the session confirms up to `max_prediction`
  **further** frames with fabricated default inputs in the dropped slots — divergently:
  peer 1 confirmed frames 96..=103 as `[3101, 3108, 0, 0]`… while peer 3 confirmed the same
  frames as `[0, 0, 3115, 3122]`…; end confirmation asymmetric even within a half (95 vs 103,
  delta = exactly `max_prediction` = 8). Blast radius: bounded (≤ max_prediction frames;
  session then stalls in `Synchronizing` on all peers — that half of the contract holds), but
  the *confirmed prefix* — what applications trust for replays, results, checksums — forks
  silently at the end of a Halt session. Fix home: **M4** (Halt must clamp confirmation to the
  last globally-agreed frame; likely at the disconnected-slot exclusion in the confirmed fold
  when behavior is Halt), plus a `**Pre-existing:**` changelog entry (shipped behavior).
  Spec note: `PeerDrop.tla`'s `HaltFailsClosed` passed while this behavior shipped — the spec
  abstracts away the prediction-window confirmation at drop time; extend it (or the M4
  regression) to model the confirmed-prefix bound, a concrete instance of the §22 trace-gap.

### 28.3 D13's sibling class — asymmetric partitions and drop asymmetry (M3 census rows)

The symmetric split is the clean case. Registered for M3 census (hand-built schedules are now
cheap): (a) **asymmetric partition** — A→B blocked, B→A open, past timeout: B drops A while A
still hears B — one-sided drop; assert gossip converges the survivor set and no peer wedges
`Running`-but-stalled; (b) **minority partition** at N≥5 (1-peer island vs 4-peer majority) —
island halts/forks per policy, majority converges; (c) partition landing mid **floor round /
hot-join** (the agreement machinery mid-flight when the mesh splits).

### 28.4 Quorum awareness (new API idea, promoted from this audit)

The fork is by-design, but applications currently cannot distinguish "I dropped one flaky
peer" from "I am the minority half of a fork". Cheap, non-breaking addition (M4/M6 window):
expose the connected-survivor count (or include survivor-set size in the
`PeerDropped`/`Disconnected` context) and document the pattern: *if fewer than ⌈N/2⌉ peers
remain connected, treat the match as void/paused rather than continuing* — turning the CAP
consequence into an app-level policy decision instead of a silent fork. (A mesh-voted
disconnect protocol — consensus on membership — is explicitly NOT proposed: per-peer local
decisions + gossip convergence is the design; adding consensus would import the FLP/latency
problems the library deliberately avoids.)

## 29. Byzantine-lite: single-dishonest-peer capability taxonomy (W-BYZ)

Locked scope stands: full BFT out of scope; crypto delegated to transport (§8.6). But "what
can ONE dishonest or buggy peer do to honest peers" is a threat-model obligation, and several
rows are cheap to harden. Capability matrix (verified against current code):

| # | Capability | Current defense | Residual risk | Disposition |
| --- | --- | --- | --- | --- |
| B1 | **Equivocation**: different inputs for the same frame to different peers | none preventive; desync detection fires *eventually* (checksum divergence downstream) but cannot attribute blame | honest peers diverge, each blames the other | Document in threat model (M5 §8.6): detection-not-prevention is the chosen cost point at 60 Hz; input-hash gossip sketch recorded in §29.1 pending W-BYZ research |
| B2 | Lying gossip (`ConnectionStatus.last_frame` inflation/deflation about third parties) | converge-down merge bounds some damage; D12 range validation (M5) rejects extremes | pin a victim's slot low (stall amplification) or claim false progress | D12 (M5) + M3 census row: one peer's gossip lies by ±k; assert bounded effect |
| B3 | False checksum reports (desync accusation) | trust-downgrade threshold; advisory, never auto-ejects | reputation attack: honest peer flagged to the app | Stance already documented; add to threat model + desync playbook ("a mismatch is not proof of *local* fault") |
| B4 | Floor-round lying (hostile `FloorReply.floors`) | D12 range validation (M5); NULL/consumer guards | wedge or premature release of freeze holds | D12 + one census row |
| B5 | Flooding (packet/handshake storms) | 256/poll receive caps, bounded decode, 64 MiB cap, depth 128 | poll-cap starvation of honest endpoints (H-POLLCAP is the honest-traffic twin) | H-POLLCAP covers the mechanism; threat model names the caps as the defense |
| B6 | Hot-join snapshot poisoning (malicious coordinator serves fake state) | none — joiner fully trusts the snapshot | joiner starts on fabricated state; desync detection then fires against the mesh | Document as an explicit trust assumption (§8.6); parking lot: snapshot digest countersigned by a second survivor |
| B7 | Replay / cross-session injection | conn_id 2⁻³² (v1) + sync-token random echo | low | Already M5 |
| B8 | Source-address spoofing (UDP addr = identity) | none at library level until v1 conn_id | session interference for raw-UDP-without-DTLS deployments | Threat model states this plainly; ties to §30.3 address migration (which must NOT ship without packet auth) |

**Inherent limit to document (not fixable in-protocol):** determinism puts full game state on
every client — information exposure (wallhacks, input-reading bots) is structural; fog-of-war
secrecy is incompatible with client-side deterministic rollback. Anti-cheat is an
application/transport concern.

### 29.1 Research addendum (W-BYZ literature sweep — executed; posture confirmed)

- **Attack vocabulary for the threat model (adopt as-is):** the NEO (NOSSDAV'04) five
  protocol-level cheats — fixed-delay, timestamp, suppressed-update, **inconsistency
  (= equivocation, B1)**, collusion — plus Baughman & Levine's suppress-correct and lookahead.
  Rollback *concedes* the lookahead/fixed-delay class by design (prediction windows reward
  artificial delay — the known "one-sided rollback" fairness gripe); say so explicitly.
- **Commitment schemes (Lockstep/NEO/SEA/pipelined): confirmed impractical and unadopted.**
  Lockstep commit-reveal = 3× slowest-link playout latency; NEO (fixed 2d rounds + per-round
  sign/encrypt/vote) was **broken** by SEA (replay + equivocation still possible); a 2025
  review calls both "not practical due to the excessive use of cryptographic signatures."
  **No shipped game in 25 years adopted commit-reveal** (negative result, searched). The
  expected answer held: detection-oriented posture confirmed.
- **Equivocation (B1) refinement:** detection *with attribution* is cheap at our scale —
  PeerReview (SOSP'07) machinery specialized to fixed-schedule input broadcast: sign inputs
  (modern Ed25519 ≈ 50 µs — inside a 60 Hz budget), gossip received-input digests; two
  validly-signed conflicting inputs for the same (peer, frame) = a **transferable proof**.
  O(N²) overhead is trivial at N ≤ 8. Parking-lot entry (post-M6, needs packet-auth/flags
  bit 0 first): "signed-input equivocation proofs" — the only Byzantine defense that is both
  real and affordable here. Prevention remains BFT/trusted-hardware territory (TrInc) — out.
- **Shipped-practice confirmations:** Photon Quantum's docs explicitly warn against
  checksums-as-anticheat ("not recommended for live games" — a mismatch halts the session);
  our advisory trust-downgrade + never-auto-eject stance matches best practice. Slippi's 2021
  macro-cheating incident was adjudicated from `.slp` **input logs** — the rollback input
  stream doubles as the anti-cheat evidence log; our `take_replay()` is that artifact
  (mention in the desync playbook + threat model). Attested desync/equivocation cheats in
  GGPO-lineage games: none found (negative result).
- **Information leakage is a theorem, not a bug:** Kartograph (IEEE S&P'11) lifts fog-of-war
  by memory reading, undetectable remotely; the MPC fix (OpenConflict) costs 22 ms/core per
  sync — an order of magnitude off 60 Hz budgets, never shipped. Document as a designer-facing
  non-goal: **never put secrets in synchronized state**.
- **CAP/quorum precedent check (§28.4 stance confirmed):** no academic CAP/FLP treatment of
  lockstep meshes exists (our §28.1 framing is novel, not citable); the only mesh-votes
  precedent is NEO's per-round majority (broken, never shipped); shipped designs use
  deterministic election rules (DirectPlay 8 oldest-joined host migration), not votes.
  Exposing survivor count and letting the app decide remains the right call.

## 30. Degradation models the harness cannot express yet (W-DEGRADE)

Verified gaps in `SimNet`/`LinkPolicy` (today: drop/dup/base_delay/jitter/burst only):

1. **Bandwidth & queueing (bufferbloat)**: no rate limit exists — a 16-mesh peer on a
   constrained uplink is unsimulatable, yet §14's fragmentation cliff interacts with exactly
   that. Add per-directed-link token-bucket rate + bounded queue (tail-drop) with
   queueing-delay growth; a saturated link then produces the real-world signature (RTT rises
   *before* loss — currently unproducible). M3-adjacent; schema field, default off. New
   hypothesis **H-BLOAT** (feedback-loop candidate FM-6): the RTT/2 advantage estimator (D11)
   misreads bufferbloat-induced RTT growth as remote frame advantage and issues wrong
   `WaitRecommendation`s — slowing down *increases* queue drain time asymmetrically; pairs
   with the §16 trace instrumentation.
2. **Correlated (Gilbert-Elliott) loss**: `burst_rate/burst_len` is a crude stand-in; a
   two-state GE channel (good/bad states with transition probabilities) is the standard model
   for WiFi/cellular. Cheap `LinkPolicy` extension (schema field). The redundant window
   (§24.4) is provably strong against i.i.d. loss; GE bursts longer than the unacked window
   are its real failure mode — measure where that cliff sits (pairs with H-FRAG).
3. **NAT rebind / address migration**: verified — packets from an unknown source address are
   **silently dropped** (`p2p_session.rs:1254-1261`; no violation, no counter — the D2
   pattern one layer up). A mid-session NAT rebind (mobile/CGNAT mapping timeout) silently
   kills the peer until the disconnect timeout. Actions: (a) M2 telemetry: unknown-source
   packet counter + one rate-limited violation (distinguishes "rebind/ghost traffic" from
   pure silence in every future field report); (b) M6 docs: rebind failure mode + keepalive
   cadence guidance; (c) parking lot: **v1 conn_id enables QUIC-style address migration**,
   but migration without path validation = hijack-by-spoofed-source — gated on packet auth
   (flags bit 0) landing first; (d) M3 lifecycle op `Rebind{peer}`: re-attach a peer's
   `SimSocket` at a fresh address — today the mesh has zero coverage of ANY address change.
4. Duplicate storms and extreme reorder are already expressible (dup_rate, jitter,
   Hold/Release) — no gap.

## 31. Part IV work-item integration

- **D13** → §2 defect registry (fix **M4**; red-doc test
  `partition_under_halt_confirms_fabricated_frames_divergently_d13` flips at the fix;
  `**Pre-existing:**` changelog entry; PeerDrop.tla blind spot noted per §28.2).
- Fork semantics pin → permanent test `partition_under_continue_without_forks…`; M6 docs
  obligations (§28.2 wording; §28.4 quorum pattern).
- §28.3 partition census rows → M3 §6.4. §29 B2/B4 census rows → M3 §6.4.
- §29 matrix → M5 §8.6 threat-model page gains one subsection per row (B1–B8) plus the
  inherent-information-exposure statement.
- §30.1 token-bucket + H-BLOAT/FM-6, §30.2 GE loss, §30.3 `Rebind` op → M3 amendments;
  §30.3a unknown-source telemetry → M2 amendment (with D9 telemetry, §5.4).
- Harness `DropPolicy` plumbing unblocked M3 §6.1's lifecycle drop ops; `GracefulRemove` is now
  covered end-to-end (session 76), while `LegacyDisconnect` is covered as an executable Halt/D13
  probe (session 77), not as graceful survivor convergence.

## 32. Part IV bibliography

- [NEO: cheat-proof event ordering (NOSSDAV'04)](https://zappala.byu.edu/pubs/neo-nossdav-2004.pdf); [Baughman & Levine: cheat-proof playout (ToN'07)](http://forensics.umass.edu/pubs/baughman.ToN.pdf); [SEA breaks NEO (ARES'06)](https://ieeexplore.ieee.org/document/1625291); [Cronin et al.: pipelined lockstep cheating](https://www.cs.ubc.ca/~krasic/cpsc538a/papers/adcog03-cheat.pdf)
- [PeerReview: accountability for distributed systems (SOSP'07)](https://www.sigops.org/s/conferences/sosp/2007/papers/sosp118-haeberlen.pdf); [TrInc (NSDI'09)](https://www.usenix.org/legacy/event/nsdi09/tech/full_papers/levin/levin.pdf)
- [OpenConflict / Kartograph: information exposure in P2P games (IEEE S&P'11)](https://www.shiftleft.org/papers/openconflict/openconflict.pdf); [Chambers et al.: mitigating information exposure (NOSSDAV'05)](https://www.researchgate.net/publication/220937670_Mitigating_information_exposure_to_cheaters_in_real-time_strategy_games)
- [Photon Quantum: cheat protection manual](https://doc.photonengine.com/quantum/current/manual/cheat-protection); [Anti-cheat systematic review (arXiv 2512.21377)](https://arxiv.org/pdf/2512.21377); [DirectPlay 8 host migration (MC-DPL8CS)](https://learn.microsoft.com/en-us/openspecs/windows_protocols/mc-dpl8cs/c188116b-228c-4c39-9959-381845f3d1af)

---

# Part V — Fine-toothed final pass (2026-07-03, same session)

Method: attack this session's own conclusions first (oracle-integrity controls at N=16),
refine D13's mechanism to exact code, adversarially audit every source file no prior pass had
read, and census the frame-arithmetic discipline. One harness defect found and fixed
red-green; two clean bills with line-level verification; one suspicion falsified.

## 33. HD-1 — the oracle's own silent cap (found, fixed red-green this session)

**The "assume we're wrong" check paid off.** Part III's "green at N=16" verdict was only
meaningful if the oracle still has teeth at that scale, so a negative control was added:
corrupt peer 7's state in a 16-mesh and require the run to fail with `StateDivergence`.
**RED:** the run failed, but the report contained *exactly 64 `InbandDesyncDetected` entries
(including exact duplicates) and zero `StateDivergence`* — the global `failure_cap: 64`
(`oracle.rs:122`) with first-come-first-kept `push_failure` (`:127`) let the per-peer in-band
event flood evict the state-comparison class entirely. At N=2 both classes fit; at N=16 the
cap silently hid a whole failure class — the exact "silent cap" anti-pattern, inside the
instrument built to catch such things. Consequence had it shipped: an M3 nightly fleet failure
at high N could have been triaged from a report missing its most diagnostic class.

**GREEN:** `Oracle.per_class_cap = 8` — `push_failure` now caps per `std::mem::discriminant`
(8 classes × 8 ≤ the global 64), guaranteeing every failure class representation. The N=16
control passes (StateDivergence surfaces beside the capped in-band entries); the N=2 controls
are unaffected; full suite 2308/2308 green.

New permanent controls in `tests/simulation/fleet.rs` (both `#[ignore]`d probes, promoted
with A8): `oracle_catches_seeded_divergence_in_sixteen_player_mesh` and
`same_schedule_produces_identical_trace_at_sixteen_players` (bit-identical trace reproduction
verified at N=16, 12.7 s for the pair, debug).

**Meta-lesson (M3 §6.2 amendment):** every oracle invariant's negative control must run at
the LARGEST supported N, not only N=2 — detector coverage does not automatically scale with
the thing it watches. Add the N=16 rows for the checksum-lie control too when M3 lands.

## 34. D13 mechanism, refined to exact code (updates the §2 registry row)

The fabricated values are the **deliberate legacy-halt branch** of
`synchronized_inputs`: `src/sync_layer/mod.rs:1316-1324` — a disconnected slot past
`last_frame` surfaces the frozen agreed value **only if the queue was frozen via the graceful
path**; "for non-frozen disconnects (legacy halt path) keep returning the default to preserve
back-compat". Halt never engages the freeze-convergence machinery (freeze frames are captured
only when `behavior == ContinueWithout && Emit` — `p2p_session.rs:7217-7238`), so there is no
agreed value and each peer confirms its own local view — the observed cross-peer divergence.

Severity refinements, both directions:

- **Softer than first assessed:** the fabricated inputs are labeled
  `InputStatus::Disconnected` in `AdvanceFrame` requests (`sync_layer/mod.rs:1324`) — a
  status-inspecting application CAN distinguish them. "Silent" applies to the confirmed-frame
  *bookkeeping*, not the per-request API.
- **Wider than first assessed:** confirmed frames feed `take_replay()` — replay artifacts
  recorded through a Halt-disconnect window contain divergent fabricated tails (two peers'
  replays of the same session disagree about its final ≤ max_prediction frames). The desync
  playbook (M6 §9.2) must warn that replays from halted sessions are only canonical up to the
  last globally-agreed frame.

M4 fix decision sharpened: routing Halt drops through the freeze machinery would fix
*value agreement* but still violate the documented "no further frames advance" contract; the
right fix remains clamping confirmation at the last globally-agreed frame on the Halt path,
with the doc contract as the acceptance test (flip the §28.2 red-doc test).

## 35. Clean bills with line-level verification (two adversarial audits + one census)

Audit A — `compression.rs`, `rle.rs`, `input_queue/{mod,prediction}.rs`, `codec_depth.rs`,
`buffer.rs`: **zero defects.** Verified sound, each at file:line: delta-decode validates
length/frame-cap before any allocation with fallible reserves throughout; roundtrip +
XOR-self-inverse property tests exist; hostile `size_hint`s (absent/lying/`usize::MAX`)
covered by tests; RLE compares run lengths as u64 against the cap BEFORE narrowing (32-bit
truncation bypass impossible, tested); accumulation overflow checked; depth-limited
deserializer rejects deep nesting before recursion on every container path (tested at
limit−1/limit with a non-vacuity control); ring `confirmed_input` verifies the slot's stamped
frame so `% 128` aliasing is detected rather than silently served; prediction strategies use
only network-synchronized state.

Audit B — `sync_layer/{mod,saved_states,game_state_cell}.rs`, `replay.rs`,
`replay_session.rs`, spectator catchup, `sync_test_session.rs`, `frame_info.rs`, `rng.rs`:
**zero new defects.** Verified sound: rollback load validates past-ness, prediction-window
bound, and cell frame-stamp (`mod.rs:1195-1219`); `SavedStates` ring is `max_prediction + 1`
cells with stale-cell guards at every accessor; Sparse mode clamps `set_last_confirmed_frame`
to `last_saved_frame` (`mod.rs:1443-1498`) so rollback-reachable states can't be evicted;
`GameStateCell` is loom-verified; replay records confirmed inputs only with skipped-frame
gaps visible in metadata; spectator catchup errors (never silently skips) on buffer
underflow; sync-test checksum comparison catches deliberate corruption (tested); seeded-RNG
`gen_range` degrades to a violation + `range.start`, never panics. (One agent
mischaracterized D13 as "by-design" — corrected: it is registry defect D13, §34.)

Frame-arithmetic census — **suspicion falsified**: 31 `safe_frame_add!` sites; every raw
`+`/`-` on frame values outside the macro is in `#[cfg(test)]` modules (verified per hit:
`protocol/mod.rs:9212` > tests at `:2886`; spectator `:3109` > tests at `:2310`; hot_join
`:1338` > tests at `:800`) or a provably-in-range guard comparison
(`sync_layer/mod.rs:1185`, returns a structured error). `Frame` implements no `Add` — the
type cannot be accidentally bumped unchecked. Production frame arithmetic is disciplined;
the M6 §9.1 i32-boundary test remains the only planned work here.

## 36. Part V integration

- HD-1 fix shipped in-tree (`oracle.rs` per-class cap); §6.2 gains the scale-matched
  negative-control rule; the two N=16 controls promote together with A8.
- §2 D13 row updated with the §34 mechanism cite; M6 desync playbook gains the
  replay-canonicality warning; M4 fix acceptance = the doc contract.
- Audits A/B recorded as verified-sound coverage of the last unaudited source files — every
  file under `src/` has now been examined by at least one adversarial pass this program.
- Running totals for this session: defects found 2 (D13 production, HD-1 harness), hypotheses
  executed to verdicts 5 (H-16P correctness, H-TCP-lite, H-EVLOSS, H-DISC-RACE, oracle-teeth),
  suspicions falsified 4 (H-DISC-RACE, input-width validation, frame-arithmetic census,
  compression/sync-layer audit sweeps).
