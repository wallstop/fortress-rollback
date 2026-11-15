# Fleet Search Roadmap

Use this reference only after the current product/API work is complete and the maintainer has not
reprioritized the queue. Every stage must preserve deterministic replay, bounded artifacts, and the
existing oracle.

## Order and entrance gates

1. **Sometimes-state coverage.** Define a small registry of rare states that must occur across a
   scheduled window, such as maximum rollback depth, floor-round consumption, nudges, sparse
   checkpoint search, and near-wrap ring use. First publish counts without gating. Add a ratchet
   only after the baseline proves the probes are stable and non-vacuous. **Enabled (2026-11-15):**
   seven post-redesign windows on `main` retained organic presence for the prediction-limit,
   floor-round, nudge, and input-ring IDs; the checked-in `RATCHET_RETAINED_IDS` set now fails the
   nightly merge when a retained ID records zero organic hits. The sparse-earlier row remains
   targeted-only (#402).
2. **Violation-path coverage.** Inventory stable IDs for production `report_violation!` sites,
   aggregate exercised IDs across tests and the fleet, and report the unexercised set. Ratchet only
   against a checked-in baseline; site moves must not silently reset identity. The inventory is
   `scripts/simulation/violation-site-baseline.tsv`: IDs are content hashes of `(kind, format
   literal)`, so line/file moves do not reset identity; `violation_sites.py --check` (agent
   preflight and `ci-quality.yml`) rejects unapproved drift. Regenerate with `--write` only when an
   assertion changes intentionally.
3. **Probabilistic concurrency testing (PCT).** Bias deterministic delivery order with a bounded
   number of priority changes. Compare unique abstract states and failures per unit of compute with
   the current seeded scheduler before retaining it. **Measured rejection (2026-11-14):** a matched
   128-schedule / 640,000-step window with 16 priority changes observed the same six schedule-level
   organic sometimes-state vectors, the same thirteen per-ID peer-count buckets, and the same 107
   bounded behavior signatures as the forward-order baseline, with zero failures in either arm;
   every trace hash changed, so the interleaving was applied. The comparison also exposed a
   freeze-frame oracle false positive for a survivor lagging behind the freeze frame, now fixed and
   pinned by `oracle_ignores_lagging_survivor_before_the_freeze_frame` and
   `pct_lagging_survivor_does_not_fake_a_freeze_divergence`. Keep the opt-in harness axis for
   order-sensitivity regressions; do not enable PCT in the nightly fleet without a new
   order-sensitive hypothesis.
4. **Prefix-replay branching.** Resume from replay-safe corpus prefixes that reached rare abstract
   states, then explore alternative bounded suffixes. Require exact prefix identity and demonstrate
   better state discovery per compute budget than fresh-seed search. **Measured rejection
   (2026-11-15):** four 250-step rare-state prefixes (`input_queue_length = 2`, `max_prediction =
   1`, clean network, `ContinueWithout`) were replayed with 64 matched fresh suffixes each. Every
   one of the 256 branches reproduced its prefix cumulative trace hash exactly. Across 256,000
   simulated steps the branch arm observed only the `0x15` prefix vector (prediction-limit, nudge,
   input-ring), added zero new organic vectors, and observed 3 per-ID peer-count buckets. The
   matched fresh-seed arm found the missing `floor_round.lower_floor_consumable.v1` vector (`0x17`)
   in 86 of its 256 schedules and 7 buckets over the same budget; both arms had zero failures.
   Replaying a prefix consumes suffix reach without a session-state snapshot API, so the technique
   adds no state discovery per compute and is rejected. The state-discovery metric does not consume
   oracle failures; the oracle's failure path is separately pinned by the existing
   `corrupt_state_from` negative control (`artifact_reproduction_rejects_mutated_progress_evidence`
   asserts a corrupted run fails). Do not add snapshot plumbing for this purpose without a measured
   coverage gap.
5. **Lineage-driven fault injection (LDFI-lite).** Record the deliveries supporting confirmation,
   then greedily remove small support sets. Compare failure yield with random loss. Introduce a SAT
   dependency only if greedy search plateaus and a measured result justifies the added complexity.
   **Measured rejection (2026-11-15):** across 16 matched 1,000-step green schedules, a bounded
   4,096-delivery log and greedy support removal found a 1-4 delivery support set in 15 schedules;
   the remaining schedule still confirmed after every candidate delivery was removed. Removing the
   support set lowered the target's final confirmed frame in 15/15 tested schedules (1 frame each)
   with zero oracle failures. Matched random drop sets of the same size lowered it in 0 of 240
   trials and also produced zero oracle failures; the baseline had zero failures. LDFI-lite is a
   stronger targeted confirmation perturbation (15/15 vs 0/240) but observed no oracle failure in
   the matched green arms, so the delivery-identity ring, drop-set plumbing, and greedy re-run
   search are not retained. The same oracle's known-failing `corrupt_state_from` control
   (`artifact_reproduction_rejects_mutated_progress_evidence`) proves the detector reports
   failures; the zero-failure result is not a claim that the fleet is defect-free.

## Shared experiment contract

For each stage:

- state one falsifiable hypothesis and a fixed compute/artifact budget;
- record the current baseline before implementation;
- include a negative control proving the detector can fail;
- preserve the seed, schedule, trace, and smallest useful regression for every confirmed defect;
- reject the technique if it adds complexity without a material discovery or coverage gain.

Background sources retained from the completed hardening audit include Antithesis
sometimes-assertions, PCT (ASPLOS 2010), model-guided distributed-systems fuzzing, and lineage-driven
fault injection (SIGMOD 2015).
