# Formal Specifications

This directory contains formal specification documents for the Fortress Rollback library.

## Documents

| Document | Description |
|----------|-------------|
| [formal-spec.md](Formal-Specification) | Core formal specifications using TLA+ notation |
| [api-contracts.md](API-Contracts) | API preconditions, postconditions, and invariants |
| [determinism-model.md](Determinism-Model) | Determinism requirements and verification |
| [spec-divergences.md](Spec-Divergences) | Documented differences between spec and implementation |

## Related Resources

- [TLA+ Specifications](https://github.com/wallstop/fortress-rollback/blob/main/specs/tla/README.md) - Runnable TLA+ model checking specifications
- [Z3 Verification Tests](https://github.com/wallstop/fortress-rollback/blob/main/tests/verification/z3.rs) - Z3 SMT solver verification tests
