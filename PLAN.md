# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

- Land the session-228 PR: mutation-survivor closure for #442's new paths, the #444 poll-only
  disconnected-relay floor fix, the #443 warmed N>=8 mesh `advance_frame` budget, and the first
  #446 vacuous-test slice. Exit: the single PR's full CI matrix is green and review feedback is
  addressed.

## Future

- Fix [issue #445](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/445): a
  permanent strict-majority partition with two isolated peers fail-closes the whole majority.
  A strict-majority certificate needs a proven participant reconciliation (shared required-witness
  set or majority certificate); local quorum weakening was explored in session 228 and reverted
  because concurrent disjoint removals expose incompatible cuts. Exit: a deterministic
  majority-continues regression plus an asymmetric-timeout A/B/C-vs-E reconciliation test pass
  without forks, and the default plus `hot-join` D14 suites stay green.
- Fix [issue #450](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/450): the
  #448 OpenAI Review caller fails at startup with zero jobs because the private shared-workflow
  repository is not reachable. Exit: a PR run produces at least one job/conclusion, or the caller
  is pinned to a reachable workflow, without changing org policy.
- Finish [issue #446](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/446): scan
  every zero-assertion/ignored-result test and give each one a falsifiable assertion or delete it
  if it duplicates a stronger test. Session 228 fixed the first four (RLE truncation, TimeSync
  negative frame, and two empty P2P poll tests). Exit: a repo-wide assertion scan has no
  remaining vacuous tests.
