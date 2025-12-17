<p align="center">
  <img src="../assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Note:** For historical changes from the original GGRS project (versions 0.2.0–0.11.0),
> see [ggrs-changelog-archive.md](ggrs-changelog-archive.md).

## [Unreleased]

## [0.1.0] - 2024-12-XX

Initial release of Fortress Rollback, a correctness-first fork of GGRS v0.11.0.

### Added

#### Desync Detection API

- `SyncHealth` enum for synchronization status reporting (`InSync`, `Pending`, `DesyncDetected`)
- `P2PSession::sync_health(peer)` to query synchronization status with a specific peer
- `P2PSession::is_synchronized()` to check if all peers are in sync
- `P2PSession::all_sync_health()` to get sync status for all remote peers
- `P2PSession::last_verified_frame()` to get the highest frame with successful checksum verification
- `NetworkStats` fields for desync monitoring: `last_compared_frame`, `local_checksum`, `remote_checksum`, `checksums_match`
- `InvariantChecker` implementation for `P2PSession` to validate session health

#### Deterministic Hashing

- `fortress_rollback::hash` module with deterministic FNV-1a hashing utilities
- `DeterministicHasher` for consistent cross-process checksums
- `fnv1a_hash` convenience function for computing deterministic hashes
- `DeterministicBuildHasher` for use with collections requiring deterministic hashing

#### Configuration APIs

- Structured configuration: `SyncConfig`, `ProtocolConfig`, `TimeSyncConfig`, `SpectatorConfig`, `InputQueueConfig`
- Preset-based configuration methods (e.g., `SyncConfig::high_latency()`, `ProtocolConfig::competitive()`)
- `SaveMode` enum replacing deprecated `with_sparse_saving_mode()`
- `ViolationObserver` trait and `CollectingObserver` for monitoring internal invariant violations

#### Session APIs

- `P2PSession::confirmed_inputs_for_frame(frame)` for computing deterministic checksums over confirmed state
- `InputQueue` now tracks player index for player-specific prediction strategies

#### Development Infrastructure

- Pre-commit hooks configuration for code quality automation (markdownlint, link validation, cargo fmt/clippy)
- `docs.yml` CI workflow for documentation and link validation
- `scripts/check-links.sh` for local file reference validation
- Comprehensive test suite: 180+ library tests, 35/35 multi-process network tests passing
- TLA+ `Concurrency.tla` specification for `GameStateCell` thread safety verification

### Changed

#### Determinism Improvements

- Replaced all `HashMap` with `BTreeMap` for guaranteed iteration order
- Replaced all `HashSet` with `BTreeSet` for deterministic iteration
- `Config::Address` now requires `Ord` + `PartialOrd` trait bounds (see Migration section)
- Test infrastructure uses `fnv1a_hash` instead of `DefaultHasher`

#### Branding

- Crate renamed to `fortress-rollback` (import as `fortress_rollback`)
- All `Ggrs*` types renamed to `Fortress*`:
  - `GgrsError` → `FortressError`
  - `GgrsEvent<T>` → `FortressEvent<T>`
  - `GgrsRequest<T>` → `FortressRequest<T>`
- All documentation updated to reference "Fortress Rollback"

#### Safety Improvements

- `InputQueue::confirmed_input` now returns `Result` instead of panicking
- Spectator confirmed-input path bubbles `FortressError` on missing data

### Fixed

- Crash when misprediction detected at frame 0 (first frame): `adjust_gamestate()` no longer attempts to load the current frame when receiving early corrections
- Multi-process rollback desync (BUG-001): Window-based checksum computation using last 64 frames ensures frames are always available for comparison between peers
- All 35 multi-process network tests now pass reliably

### Removed

- Historical GGRS changelog entries moved to [ggrs-changelog-archive.md](ggrs-changelog-archive.md)

---

## Breaking Changes from GGRS

This section summarizes breaking changes for users migrating from GGRS v0.11.0.

### Dependency Change

```toml
# Before
[dependencies]
ggrs = "0.11"

# After
[dependencies]
fortress-rollback = "0.1"
```

### Import Path Change

```rust
// Before
use ggrs::{SessionBuilder, P2PSession, GgrsError};

// After
use fortress_rollback::{SessionBuilder, P2PSession, FortressError};
```

### Type Renames

| Old Name           | New Name             |
|--------------------|----------------------|
| `GgrsError`        | `FortressError`      |
| `GgrsEvent<T>`     | `FortressEvent<T>`   |
| `GgrsRequest<T>`   | `FortressRequest<T>` |

### Address Trait Bounds

`Config::Address` now requires `Ord` + `PartialOrd`:

```rust
// Before
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct MyAddress { /* ... */ }

// After
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct MyAddress { /* ... */ }
```

### Behavioral Notes

- Session termination using `confirmed_frame()` alone is incorrect; use the new `SyncHealth` API for proper synchronization verification. See [migration.md](migration.md) for details.
- Desync detection remains disabled by default; enable it explicitly for production use.

---

For detailed migration instructions, see [migration.md](migration.md).

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/wallstop/fortress-rollback/releases/tag/v0.1.0
