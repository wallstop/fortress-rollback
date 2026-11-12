# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

_None._

## Future

1. **P2 — Improve search only after the preceding oracle is calibrated.** Start with the existing sometimes-state census:
   evaluate its multi-night stability before any ratchet. Then use the
   [fleet-search roadmap](.agents/skills/dst/references/fleet-search-roadmap.md) to compare violation-site coverage, PCT,
   prefix branching, and LDFI-lite against fresh-seed search at equal compute. **Exit:**
   retain a technique only when it materially increases unique useful states or defect discovery per compute, preserves
   exact replay and bounded artifacts, and contributes a minimized regression; otherwise record the rejected experiment
   and keep the simpler fleet.
