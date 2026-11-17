# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

- Continue [issue #446](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/446):
  session 229 strengthens `SyncLayer::reset_prediction` and the P2P builder tests, after session
  228 removed the first four zero-assertion tests and strengthened the telemetry macro and reorder
  harvest assertions. Exit for this slice: those tests assert observable behavior and the full
  suite stays green.

## Future

- Fix [issue #445](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/445): a
  permanent strict-majority partition with two isolated peers fail-closes the whole majority.
  A strict-majority certificate needs a proven participant reconciliation (shared required-witness
  set or majority certificate); local quorum weakening was explored in session 228 and reverted
  because concurrent disjoint removals expose incompatible cuts. Exit: a deterministic
  majority-continues regression plus an asymmetric-timeout A/B/C-vs-E reconciliation test pass
  without forks, and the default plus `hot-join` D14 suites stay green.
- Track [issue #454](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/454): the
  manual `Agent Metrics` dispatch of the shared `review-tools` workflow fails at startup with zero
  jobs while OpenAI Review succeeds. Exit: a manual dispatch starts a job and completes, or the
  shared workflow's checkout ref is made event-independent.
