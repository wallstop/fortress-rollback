# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

_None._

## Future

- Fix [issue #445](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/445): a
  permanent strict-majority partition with two isolated peers fail-closes the whole majority.
  A strict-majority certificate needs a proven participant reconciliation (shared required-witness
  set or majority certificate); local quorum weakening was explored in session 228 and reverted
  because concurrent disjoint removals expose incompatible cuts. Exit: a deterministic
  majority-continues regression plus an asymmetric-timeout A/B/C-vs-E reconciliation test pass
  without forks, and the default plus `hot-join` D14 suites stay green.
