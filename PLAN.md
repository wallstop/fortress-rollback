# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

- **P1 — Qualify gameplay experience and performance with representative states.** The release
  representative-state ledger now publishes 2/4/16-player p50–p99.9 frame-service, rollback,
  allocation, bandwidth, and confirmation evidence plus reference budgets
  (`tests/representative_perf.rs`, `docs/tuning.md`). Remaining: a playable/reference game
  workload, visible correction magnitude, audio/presentation duplication, downstream queue
  behavior, and a profile-before-optimize pass on the target hardware. **Exit:** extend the
  published per-tier budgets and failure thresholds to that workload, reproduce the current
  mobile/4-player/16-player costs, and accept changes only when the correctness oracle stays green
  and a statistically useful end-to-end improvement exceeds run variance. Evaluate topology
  changes only if tuning cannot meet the chosen budget.

## Future

1. **P1 — Make the safest integration the easiest integration.** Exercise the public package as an external consumer,
   then mutate the getting-started/request-handling paths: missing or wrong-frame loads, checksum serialization failure,
   omitted local input, undrained events, ignored WaitRecommendation, variable-width input, and mismatched lifecycle
   configuration. Current examples silently continue on missing loads and often erase checksum errors. **Exit:** every
   preventable misuse fails early; every remaining responsibility has a runnable counterexample and explicit diagnostic,
   never silent corruption. Canonical examples return errors rather than warn-and-continue, cover rollback-sensitive side
   effects, and a clean two-process quickstart reaches confirmed play with enough telemetry to diagnose each failure.

2. **P1 — Verify portability and long-session determinism.** Replay identical captured corpora across supported native
   architectures, WASM/browser and Emscripten adapters, release/safe-release profiles, and mixed transport paths; include
   background-tab resume and extended frame/ring operation. **Exit:** under one captured replay/schedule, requests and
   artifacts are byte-identical; live mixed transports agree on confirmed checkpoints despite legal speculative timing
   differences. No panic or unexpected violation occurs; any platform limitation has a compatibility regression and narrowed
   support documentation. A simulator-only pass is not cross-platform evidence.

3. **P2 — Improve search only after the preceding oracle is calibrated.** Start with the existing sometimes-state census:
   evaluate its multi-night stability before any ratchet. Then use the
   [fleet-search roadmap](.agents/skills/dst/references/fleet-search-roadmap.md) to compare violation-site coverage, PCT,
   prefix branching, and LDFI-lite against fresh-seed search at equal compute. **Exit:**
   retain a technique only when it materially increases unique useful states or defect discovery per compute, preserves
   exact replay and bounded artifacts, and contributes a minimized regression; otherwise record the rejected experiment
   and keep the simpler fleet.
