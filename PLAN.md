# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

_None._

## Future

- **Bind coordinated-drop participant sets to the operation identity
  ([#504](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/504)).**
  Two requesters in a split view can currently mint the same `DropOperationId`
  with different participant vectors, letting disjoint strict-majority groups
  commit different cuts for one target. Exit: a deterministic two-requester
  split-view row asserts at most one committed cut, and
  `CoordinatorFailover.tla` models two disjoint participant groups.
