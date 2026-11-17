# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

_None._

## Future

- Fix [issue #444](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/444): a
  disconnected relay's fresh floor is ignored during a poll-only pause. Exit: a deterministic
  scenario reproduces the stale-disconnected-relay race, the consumer folds the fresh floor for
  that case without regressing the existing stale-high reorder tests, and the full default plus
  hot-join suites pass.
- Add the N>=8 warmed `advance_frame` benchmark and CI row from
  [issue #443](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/443). Exit: N=8
  and N=16 rows run in the benchmark gate with a documented budget or paired comparison.
