# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

1. **P2 — Publish seven post-redesign nightly windows and enable the organic presence ratchet.**
   The small-ring coverage cell makes 4/5 IDs organic-eligible. The sparse-earlier row is
   targeted-only (`SOMETIMES_STATE_ORGANIC_ELIGIBLE`, #402). The checked-in
   `evaluate_seven_window_ratchet` validates seven merged artifacts, counts only organic
   `schedules_hit >= 1` presence, and retains at least three qualifying IDs. **Exit:** seven-window
   artifact evidence evaluated by the ratchet and enabled, or a recorded rejection. Scope:
   [#403](https://github.com/wallstop/fortress-rollback/issues/403).

2. **P2 — Violation-path coverage census.** The checked-in production `report_violation!`
   baseline assigns line-independent IDs and rejects unapproved site drift
   (`scripts/simulation/violation_sites.py --check`, enforced in preflight and CI). The nightly
   `Violation-Site Exercise Census` joins the baseline with main test coverage and fleet
   observations and publishes the unexercised set. Next: review the first published census and cut
   the unexercised set with focused tests or deletion of unreachable assertions. **Exit:** a
   reviewed unexercised-set reduction recorded against the nightly report. Scope: stage 2 of the
   [fleet-search roadmap](.agents/skills/dst/references/fleet-search-roadmap.md).

## Future

1. **P2 — Compare remaining fleet-search techniques against fresh-seed search.** Use the
   [fleet-search roadmap](.agents/skills/dst/references/fleet-search-roadmap.md) to compare PCT,
   prefix branching, and LDFI-lite at equal compute. **Exit:** retain a technique only when it
   materially increases unique useful states or defect discovery per compute, preserves exact
   replay and bounded artifacts, and contributes a minimized regression; otherwise record the
   rejected experiment and keep the simpler fleet.
