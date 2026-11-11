# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

- **P1 — Bound transport work and validate real-path delivery.** Test receive-cap starvation followed by valid traffic,
  retransmission storms, bounded link bandwidth/queueing, MTU/fragmentation, malformed floods, NAT rebinding, and browser
  suspension using deterministic acceptance tests plus real-transport smoke tests. **Exit:** memory and per-poll work have
  measured bounds; under each supported finite load useful traffic recovers without silent queue growth, while overload
  is visibly refused or degraded; unsupported rebinding has an explicit replacement flow. Record actual socket
  throughput/loss and queue age, not only enqueued bytes. Replay protocol-valid lies from `docs/threat-model.md`; require
  only documented detection/fail-closed behavior, not unsupported prevention or attribution. The adapter-level
  `SessionMetrics::socket` counters now expose accepted sends, failures, raw ingress, rejects, and cap saturation;
  real-path throughput/loss sampling and queue-age evidence remain.

## Future

1. **P1 — Qualify gameplay experience and performance with representative states.** Extend beyond tiny stubs to realistic
   snapshot sizes, simulation costs, input widths, and a playable/reference workload. Measure input latency, visible
   correction frequency/magnitude, audio/presentation duplication, frame-time p50/p95/p99/p99.9, rollback CPU,
   allocations, memory plateau, and bandwidth across the supported tiers. **Exit:** publish per-tier budgets and failure
   thresholds; reproduce the current mobile/4-player/16-player costs, profile before optimizing, and accept changes only
   when the same correctness oracle stays green and statistically useful end-to-end improvement exceeds run variance.
   Evaluate topology changes only if tuning cannot meet the chosen budget.

2. **P1 — Make the safest integration the easiest integration.** Exercise the public package as an external consumer,
   then mutate the getting-started/request-handling paths: missing or wrong-frame loads, checksum serialization failure,
   omitted local input, undrained events, ignored WaitRecommendation, variable-width input, and mismatched lifecycle
   configuration. Current examples silently continue on missing loads and often erase checksum errors. **Exit:** every
   preventable misuse fails early; every remaining responsibility has a runnable counterexample and explicit diagnostic,
   never silent corruption. Canonical examples return errors rather than warn-and-continue, cover rollback-sensitive side
   effects, and a clean two-process quickstart reaches confirmed play with enough telemetry to diagnose each failure.

3. **P1 — Verify portability and long-session determinism.** Replay identical captured corpora across supported native
   architectures, WASM/browser and Emscripten adapters, release/safe-release profiles, and mixed transport paths; include
   background-tab resume and extended frame/ring operation. **Exit:** under one captured replay/schedule, requests and
   artifacts are byte-identical; live mixed transports agree on confirmed checkpoints despite legal speculative timing
   differences. No panic or unexpected violation occurs; any platform limitation has a compatibility regression and narrowed
   support documentation. A simulator-only pass is not cross-platform evidence.

4. **P2 — Improve search only after the preceding oracle is calibrated.** Start with the existing sometimes-state census:
   evaluate its multi-night stability before any ratchet. Then use the
   [fleet-search roadmap](.agents/skills/dst/references/fleet-search-roadmap.md) to compare violation-site coverage, PCT,
   prefix branching, and LDFI-lite against fresh-seed search at equal compute. **Exit:**
   retain a technique only when it materially increases unique useful states or defect discovery per compute, preserves
   exact replay and bounded artifacts, and contributes a minimized regression; otherwise record the rejected experiment
   and keep the simpler fleet.
