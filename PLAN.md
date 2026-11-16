# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

- Cross-architecture runtime determinism lanes: execute the deterministic suite on
  aarch64 (QEMU) and i686 (32-bit native) in `ci-rust.yml`, fix the test portability and
  virtual-clock defects they expose, and record the verified platform status in
  `docs/specs/determinism-model.md`. Exit: both lanes pass on main and the CI contract
  tests stay green.

## Future

_None._
