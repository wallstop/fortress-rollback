# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

1. **P2 — Add a presence-only sometimes-state ratchet after seven post-redesign nightly windows.**
   The original organic census was stable but only 2/5 IDs were non-vacuous at the default session
   configuration. The nightly fleet now reserves 40/1,000 rows for a two-slot input-ring and
   one-frame prediction cell; a local 1,000-schedule window covers 4/5 IDs. The remaining
   sparse-earlier row is tracked in [#402](https://github.com/wallstop/fortress-rollback/issues/402).
   Publish seven consecutive scheduled-main windows, then add a `schedules_hit >= 1` ratchet only
   for IDs present in all seven and retain it only if at least 3/5 qualify. **Exit:** checked-in
   ratchet evaluator plus seven-window artifact evidence, or a recorded rejection. Scope:
   [#403](https://github.com/wallstop/fortress-rollback/issues/403).

## Future

1. **P2 — Compare fleet-search techniques against fresh-seed search.** Use the
   [fleet-search roadmap](.agents/skills/dst/references/fleet-search-roadmap.md) to compare
   violation-site coverage, PCT, prefix branching, and LDFI-lite at equal compute. **Exit:**
   retain a technique only when it materially increases unique useful states or defect discovery
   per compute, preserves exact replay and bounded artifacts, and contributes a minimized
   regression; otherwise record the rejected experiment and keep the simpler fleet.
