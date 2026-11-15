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
