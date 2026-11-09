# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

1. **P0 — Differentially prove rollback equivalence and oracle fidelity.** Hypothesis: for one fixed committed input/membership
   history, arbitrary legal delivery/session interleavings produce the same confirmed trajectory as a non-networked reference,
   including N-peer hot-join and redundant-spectator failover ([remaining closure](https://github.com/wallstop/fortress-rollback/issues/377)). Explore queue/ring boundaries,
   repeated mispredictions, state-only divergence, and near-overflow frames; compare implementation traces with the
   relevant TLA+/Kani assumptions rather than treating a model pass as implementation proof. **Exit:** mutation controls
   demonstrate detection of input, state, freeze, false-positive, and silent-desync failures; every red case minimizes
   to a deterministic regression and is fixed at its root, or the green claim is explicitly bounded to explored states.

## Future

1. **P0 — Close the partition/membership CAP matrix.** Using [the reconnect contract](docs/reconnect-playbook.md),
   test symmetric/asymmetric partitions, majority and minority components, concurrent drops, timeout-versus-slow-peer
   ambiguity, coordinator/origin loss, repeated heal/reflap, and rejoin during another membership change. Include Halt,
   ContinueWithout, redundant spectators, and hot-join. Add component-scoped confirmed-state comparison before making
   majority/minority progress claims; the global live-peer minimum cannot cover states beyond a minority that failed
   closed.
   **Exit:** no two components rewrite or fork an exposed confirmed
   prefix; each scenario either converges to one survivor history within a measured post-heal bound, emits the documented
   terminal/fail-closed outcome, or exposes an explicit split-match decision—never two silent histories presented as one
   session. Any quorum, split-brain, or recovery limitation becomes an explicit contract, not an inferred guarantee.

2. **P1 — Explain and improve measured pacing/liveness cliffs.** Start from `docs/tuning.md`: mobile 2-player stalls,
   severe 4-player stalls, the confirmed clock-skew phase sawtooth, and unresolved H-BLOAT drain behavior. Hypothesis:
   symmetric RTT/2 estimation, dead bands, retry cadence, and slowest-link confirmation interact to cause avoidable
   skew/stalls. Sweep clock skew, directional delay, Gilbert–Elliott loss, constrained queues, wait compliance, and poll
   frequency. **Exit:**
   identify the dominant mechanism with ablations; retain a controller/configuration change only if it improves stall
   time and visual skew without oscillation, divergence, or worse tail latency, with before/after distributions.

3. **P1 — Bound transport work and validate real-path delivery.** Test receive-cap starvation followed by valid traffic,
   retransmission storms, bounded link bandwidth/queueing, MTU/fragmentation, malformed floods, NAT rebinding, and browser
   suspension using deterministic acceptance tests plus real-transport smoke tests. **Exit:** memory and per-poll work have
   measured bounds; under each supported finite load useful traffic recovers without silent queue growth, while overload
   is visibly refused or degraded; unsupported rebinding has an explicit replacement flow. Record actual socket
   throughput/loss and queue age, not only enqueued bytes. Replay protocol-valid lies from `docs/threat-model.md`; require
   only documented detection/fail-closed behavior, not unsupported prevention or attribution.

4. **P1 — Qualify gameplay experience and performance with representative states.** Extend beyond tiny stubs to realistic
   snapshot sizes, simulation costs, input widths, and a playable/reference workload. Measure input latency, visible
   correction frequency/magnitude, audio/presentation duplication, frame-time p50/p95/p99/p99.9, rollback CPU,
   allocations, memory plateau, and bandwidth across the supported tiers. **Exit:** publish per-tier budgets and failure
   thresholds; reproduce the current mobile/4-player/16-player costs, profile before optimizing, and accept changes only
   when the same correctness oracle stays green and statistically useful end-to-end improvement exceeds run variance.
   Evaluate topology changes only if tuning cannot meet the chosen budget.

5. **P1 — Make the safest integration the easiest integration.** Exercise the public package as an external consumer,
   then mutate the getting-started/request-handling paths: missing or wrong-frame loads, checksum serialization failure,
   omitted local input, undrained events, ignored WaitRecommendation, variable-width input, and mismatched lifecycle
   configuration. Current examples silently continue on missing loads and often erase checksum errors. **Exit:** every
   preventable misuse fails early; every remaining responsibility has a runnable counterexample and explicit diagnostic,
   never silent corruption. Canonical examples return errors rather than warn-and-continue, cover rollback-sensitive side
   effects, and a clean two-process quickstart reaches confirmed play with enough telemetry to diagnose each failure.

6. **P1 — Verify portability and long-session determinism.** Replay identical captured corpora across supported native
   architectures, WASM/browser and Emscripten adapters, release/safe-release profiles, and mixed transport paths; include
   background-tab resume and extended frame/ring operation. **Exit:** under one captured replay/schedule, requests and
   artifacts are byte-identical; live mixed transports agree on confirmed checkpoints despite legal speculative timing
   differences. No panic or unexpected violation occurs; any platform limitation has a compatibility regression and narrowed
   support documentation. A simulator-only pass is not cross-platform evidence.

7. **P2 — Evaluate source-linked Lean proofs against Kani.** Follow [#365](https://github.com/wallstop/fortress-rollback/issues/365).
   After the preceding correctness/oracle work, baseline full-domain Kani varint contracts, then timebox Hax/Aeneas extraction
   of production `Frame::buffer_index` and `rle` varint. Compare arbitrary-slice assurance, mutation detection, assumptions,
   maintenance, CI time, and cache cost. **Exit:** retain Lean only for valuable proofs beyond Kani with fail-closed regeneration,
   axiom audits, compiling-mutant rejection, restored-green evidence, and measured budgets; otherwise record the rejected experiment.

8. **P2 — Improve search only after the preceding oracle is calibrated.** Start with the existing sometimes-state census:
   evaluate its multi-night stability before any ratchet. Then use the
   [fleet-search roadmap](.agents/skills/dst/references/fleet-search-roadmap.md) to compare violation-site coverage, PCT,
   prefix branching, and LDFI-lite against fresh-seed search at equal compute. **Exit:**
   retain a technique only when it materially increases unique useful states or defect discovery per compute, preserves
   exact replay and bounded artifacts, and contributes a minimized regression; otherwise record the rejected experiment
   and keep the simpler fleet.
