# Fortress Rollback Documentation

This directory contains documentation for the Fortress Rollback library.

## Getting Started

| Document | Description |
|----------|-------------|
| [**User Guide**](user-guide.md) | **📘 Complete guide to integrating Fortress Rollback** |
| [Migration Guide](migration.md) | Migrating from GGRS to Fortress Rollback |
| [Fortress vs GGRS](fortress-vs-ggrs.md) | Comparison, bug fixes, and differences |

## Technical Reference

| Document | Description |
|----------|-------------|
| [Architecture](architecture.md) | Internal architecture, data flow, and design |
| [API Contracts](specs/api-contracts.md) | API preconditions, postconditions, and invariants |
| [Determinism Model](specs/determinism-model.md) | Determinism requirements for your game |
| [Changelog](../CHANGELOG.md) | Version history and breaking changes |

### Network Layer Reference

The architecture guide includes detailed documentation of the network serialization layer:

- **[Binary Codec](architecture.md#binary-codec-networkcodec)** — Centralized bincode serialization with zero-allocation options
- **[Input Compression](architecture.md#input-compression-networkcompression)** — XOR delta encoding + RLE compression pipeline
- **[Message Types](architecture.md#during-gameplay)** — Protocol message format and types

## Contributing

| Document | Description |
|----------|-------------|
| [Contributing](contributing.md) | Guidelines for contributors |
| [Code of Conduct](code-of-conduct.md) | Community standards |

### For Contributors: Formal Specifications

These documents are for contributors interested in the formal verification approach:

| Document | Description |
|----------|-------------|
| [Formal Spec](specs/formal-spec.md) | Core formal specifications in TLA+ notation |
| [TLA+ Specifications](../specs/tla/README.md) | Runnable TLA+ model checking specs |
| [Spec Divergences](specs/spec-divergences.md) | Documented spec-implementation differences |

## Related Resources

- [Examples](../examples/README.md) — Working code examples with build instructions
- [Root README](../README.md) — Project overview and quick start
