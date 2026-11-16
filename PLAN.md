# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

_None._

## Future

- Share the remaining per-frame confirmation fold
  ([#432](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/432)): receive staging,
  the poll queue, and the send-side delta scratch are now allocation-free/retained, and a cached
  delta encode method is byte-identical. The suggested `poll_remote_clients` fold gate was measured
  and rejected because it shifted the loss-cell traffic baseline by +5% for a sub-µs saving. Exit
  with a fold-sharing design that leaves the deterministic sweep bytes unchanged.
