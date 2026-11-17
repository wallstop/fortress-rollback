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
- Finish [issue #446](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/446): scan
  every zero-assertion/ignored-result test and give each one a falsifiable assertion or delete it
  if it duplicates a stronger test. Sessions 228-229 fixed the RLE truncation, TimeSync negative
  frame, two empty P2P poll tests, the telemetry macro delivery tests, the reorder harvest
  history, `SyncLayer::reset_prediction`, and the P2P builder outcome assertions. Exit: a
  repo-wide assertion scan has no remaining vacuous tests.
