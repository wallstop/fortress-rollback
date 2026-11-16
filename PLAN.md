# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

_None._

## Future

- Reduce the remaining per-frame confirmation folds and send-side encode temporaries
  ([#432](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/432)): the receive
  staging and poll-queue allocations are gone; the suggested `poll_remote_clients` fold gate was
  measured and rejected because it shifted the loss-cell traffic baseline by +5% for a sub-µs
  saving. Exit with a fold-sharing design that leaves the deterministic sweep bytes unchanged.
