# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

_None._

## Future

- **Coordinated-drop coordinator failover ([#497](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/497)).**
  Let a reachable strict majority complete a graceful removal when the
  deterministic coordinator is isolated and the configured `disconnect_timeout`
  exceeds the two-second operation deadline. Exit: the 5s `3/1/1` partition row
  keeps the majority `Running`, confirms past the cut, and never forks; the
  failover transition is modeled in `CoordinatedPeerDrop.tla` before shipping;
  the default-timeout row in
  `tests/simulation/partition_sweep.rs::isolated_coordinator_removal_still_commits_on_reachable_majority`
  stays green.
