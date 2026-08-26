# Fleet Search Roadmap

Use this reference only after the current product/API work is complete and the maintainer has not
reprioritized the queue. Every stage must preserve deterministic replay, bounded artifacts, and the
existing oracle.

## Order and entrance gates

1. **Sometimes-state coverage.** Define a small registry of rare states that must occur across a
   scheduled window, such as maximum rollback depth, floor-round consumption, nudges, sparse
   checkpoint search, and near-wrap ring use. First publish counts without gating. Add a ratchet
   only after the baseline proves the probes are stable and non-vacuous.
2. **Violation-path coverage.** Inventory stable IDs for production `report_violation!` sites,
   aggregate exercised IDs across tests and the fleet, and report the unexercised set. Ratchet only
   against a checked-in baseline; site moves must not silently reset identity.
3. **Probabilistic concurrency testing (PCT).** Bias deterministic delivery order with a bounded
   number of priority changes. Compare unique abstract states and failures per unit of compute with
   the current seeded scheduler before retaining it.
4. **Prefix-replay branching.** Resume from replay-safe corpus prefixes that reached rare abstract
   states, then explore alternative bounded suffixes. Require exact prefix identity and demonstrate
   better state discovery per compute budget than fresh-seed search.
5. **Lineage-driven fault injection (LDFI-lite).** Record the deliveries supporting confirmation,
   then greedily remove small support sets. Compare failure yield with random loss. Introduce a SAT
   dependency only if greedy search plateaus and a measured result justifies the added complexity.

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
