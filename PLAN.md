# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

_None._

## Future

- Bound cumulative hot-join host pause across re-opened serves ([#433](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/433)): a peer that syncs but never acks must not hold the host paused indefinitely. Exit when a deterministic test bounds total stalled polls or enforces backoff.
- Retain final spectator host inputs arriving after the Goodbye poll ([#434](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/434)): choose a bounded tombstone or an explicit terminal-truncation state. Exit with a deterministic later-poll test.
- Remove duplicate per-frame confirmed folds and receive staging allocations ([#432](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/432)): exit with a benchmark plus tightened allocation contract and unchanged wire bytes.
