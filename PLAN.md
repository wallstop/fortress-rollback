# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

_None._

## Future

1. **Exercise N-peer hot-join membership in the browser and Godot probes**
   ([#493](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/493)).
   The native fleet and cross-arch trace cover the generated N-peer reactivation
   schedule, but the wasm runtime probes cover only synchronization, replay, and
   explicit removal. Exit criterion: a browser `wasm32-unknown-unknown` test and both
   Godot Playwright rows activate a hot-join slot and confirm frames on it without
   sleeps.
