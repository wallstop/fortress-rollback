# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

_None._

## Future

1. **Exercise announced-donor chunk installation end-to-end** ([#488](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/488)).
   Generated lossy lifecycle rows accept current-history donor announcements but never install an
   announced chunk; the path is covered only by unit tests. Exit criterion: a green lossy row with
   `SimReport::announced_backfill_chunks` nonzero whose removal still commits with one history.
