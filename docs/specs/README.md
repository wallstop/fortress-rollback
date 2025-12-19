# Formal Specifications

This directory contains formal specification documents for the Fortress Rollback library.

## Documents

| Document | Description |
|----------|-------------|
| [formal-spec.md](formal-spec.md) | Core formal specifications using TLA+ notation |
| [api-contracts.md](api-contracts.md) | API preconditions, postconditions, and invariants |
| [determinism-model.md](determinism-model.md) | Determinism requirements and verification |
| [spec-divergences.md](spec-divergences.md) | Documented differences between spec and implementation |

## Related Resources

- [TLA+ Specifications](../../specs/tla/README.md) - Runnable TLA+ model checking specifications
- [Z3 Verification Tests](../../tests/verification/z3.rs) - Z3 SMT solver verification tests
