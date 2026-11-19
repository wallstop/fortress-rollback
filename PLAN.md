# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

_None._

## Future

1. **Simulation coverage for announced-donor chunk installation.** Generated lossy lifecycle rows
   accept current-history donor announcements but never install announced chunks end-to-end: a
   120-seed x 3-size probe leaves
   `P2PSession::diagnostic_announced_backfill_chunks_installed` at zero. The harness needs a
   deterministic way to make an inventory-selected donor stop responding before the two-second
   operation deadline while a receiver still needs the range (for example, a planted row plus a
   `RunOptions::disconnect_timeout` shorter than the operation timeout that is not kept alive by
   in-flight donor traffic). Exit criterion: a green lossy row whose
   `SimReport::announced_backfill_chunks` is nonzero and whose removal still commits with one
   history.
