<p align="center">
  <img src="docs/assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Note:** For historical changes from the original GGRS project (versions 0.2.0–0.11.0),
> see [docs/ggrs-changelog-archive.md](docs/ggrs-changelog-archive.md).

## [Unreleased]

### Added

- `GameStateCell::load_or_err()` method for strict state loading with proper error handling
- `InvalidFrameReason::MissingState` variant for clearer error messages when loading missing states
- `SessionBuilder::with_lan_defaults()` preset for low-latency LAN play
- `SessionBuilder::with_internet_defaults()` preset for typical online play
- `SessionBuilder::with_high_latency_defaults()` preset for mobile/unstable connections
- `Frame` ergonomic methods for safe arithmetic and conversion:
  - `as_usize()`, `try_as_usize()` — convert to usize with Option/Result
  - `buffer_index(size)`, `try_buffer_index(size)` — ring buffer index calculation
  - `try_add(i32)`, `try_sub(i32)` — Result-returning arithmetic
  - `next()`, `prev()` — Result-returning increment/decrement
  - `saturating_next()`, `saturating_prev()` — saturating increment/decrement
  - `from_usize(usize)`, `try_from_usize(usize)` — safe construction from usize
  - `distance_to(Frame)` — signed distance calculation
  - `is_within(window, reference)` — window proximity check
- `FortressError::FrameArithmeticOverflow` for overflow detection in frame arithmetic
- `FortressError::FrameValueTooLarge` for usize-to-Frame conversion failures
- `InternalErrorKind::DivisionByZero` for division by zero errors
- `Debug` impl for `P2PSession`, `SpectatorSession`, and `SyncTestSession` — enables logging session state for debugging
- `Debug` impl for `ChaosSocket` — shows config, stats, and packet queue length
- `Debug` impl for `GameStateAccessor` — delegates to inner `T` when `T: Debug`
- `PartialEq` derive for `ChaosConfig` — enables configuration comparison in tests
- `Hash` derive for `ChaosStats`, `NetworkStats`, and `Pcg32` — enables use as map keys
- `Copy`, `PartialEq`, `Eq`, and `Hash` derives for `TracingObserver` unit struct
- `Hash` derive for configuration types: `TimeSyncConfig`, `SyncConfig`, `ProtocolConfig`, `SpectatorConfig`, `InputQueueConfig` — enables use as map keys for configuration caching
- `PartialEq`, `Eq`, and `Hash` derives for `DeterministicHasher` and `DeterministicBuildHasher` — enables comparison and use as map keys

## [0.3.0]

### Added

- `SessionBuilder::add_local_player()` convenience method for adding local players
- `SessionBuilder::add_remote_player()` convenience method for adding remote players
- `P2PSession::local_player_handle()` for easily getting the first local player handle
- `ProtocolConfig` now re-exported in `fortress_rollback::prelude`
- `sync_test` example demonstrating `SyncTestSession` determinism verification
- `request_handling` example demonstrating both manual matching and the `handle_requests!` macro
- Structured error reason types for zero-allocation error construction and programmatic inspection:
  - `IndexOutOfBounds` struct for out-of-bounds errors with collection name, index, and length
  - `InvalidFrameReason`, `RleDecodeReason`, `DeltaDecodeReason` enums for specific failure modes
  - `InternalErrorKind`, `InvalidRequestKind`, `SerializationErrorKind`, `SocketErrorKind` enums
- New `FortressError` variants using structured types: `InvalidFrameStructured`, `InternalErrorStructured`, `InvalidRequestStructured`, `SerializationErrorStructured`, `SocketErrorStructured`
- `ChecksumAlgorithm` and `CodecOperation` enums for identifying operations in errors
- `CompressionError` enum for RLE and delta decode errors

### Changed

- **Breaking:** Removed `#[non_exhaustive]` from `FortressError`, `FortressEvent`, `FortressRequest`, `ViolationKind`, `CompressionError`, `CodecOperation`, `CodecError`, `ChecksumError`, `ChecksumAlgorithm`, `InvalidFrameReason`, `RleDecodeReason`, `DeltaDecodeReason`, `InternalErrorKind`, `InvalidRequestKind`, `SerializationErrorKind`, and `SocketErrorKind` — users can now write exhaustive matches without wildcard arms
- **Breaking:** `ChecksumError::SerializationFailed` now uses struct fields `{ algorithm, message }` instead of tuple
- **Breaking:** `CodecError::EncodeError` and `DecodeError` now use struct fields `{ message, operation }` instead of tuple

## [0.2.2] - 2026-01-22

### Changed

- **Breaking:** Renamed `Result` type alias to `FortressResult` to avoid shadowing `std::result::Result` when using glob imports (`use fortress_rollback::*`)

### Fixed

- Removed the possibility for an internal panic under debug builds

## [0.2.1] - 2025-12-26

### Added

- `ProtocolConfig::deterministic(seed)` preset for fully reproducible network sessions
- `ProtocolConfig::protocol_rng_seed` field for deterministic RNG seeding
- `SessionBuilder::with_event_queue_size()` for configurable event queue capacity
- `ProtocolConfig::input_history_multiplier` field with presets (competitive=2, high_latency=3)
- `ProtocolConfig::validate()` method for configuration validation
- `#[must_use]` attributes on key session methods (`advance_frame()`, `disconnect_player()`, etc.)

### Changed

- **Breaking:** `SessionBuilder::with_input_delay()` and `with_num_players()` now return `Result<Self, FortressError>` instead of silently clamping invalid values
- Replaced floating-point arithmetic with integer-only calculation in `TimeSync::average_frame_advantage()` to eliminate potential non-determinism
- Replaced `DefaultHasher` with `DeterministicHasher` (FNV-1a) in `timing_entropy_seed()` for cross-platform consistency
- Reduced cloning overhead in `poll_remote_clients()` by using `Arc<[PlayerHandle]>` instead of `Vec<PlayerHandle>`
- Pre-allocated compression buffers to reduce allocations in network hot paths

### Fixed

- Fixed sync timeout event flooding that could occur under certain conditions

## [0.2.0] - 2025-12-20

### Added

- Added optional `json` feature for JSON serialization of telemetry types
  - Provides `to_json()` and `to_json_pretty()` methods on `SpecViolation` and `InvariantViolation`
  - Enable with `features = ["json"]` in Cargo.toml
- Added `SyncConfig::extreme()` preset for very hostile network conditions
  - Sends 20 sync packets (vs 10 for mobile, 5 for default) with 250ms retry intervals
  - 30-second sync timeout to handle multiple burst loss events
  - Designed for scenarios with 10%+ burst loss probability and 8+ packet burst lengths
  - Not recommended for production use due to long timeouts
- Added burst loss recommendations to packet loss documentation table

### Changed

- **Breaking:** `serde_json` is now an optional dependency behind the `json` feature
  - Reduces default dependency count from 7 to 6 production dependencies
  - Users who need `to_json()` methods must enable the `json` feature
  - The telemetry types still implement `serde::Serialize` for use with any serializer
- Restructured test code to further reduce published crate size
  - Moved network peer binary to separate `tests/network-peer` crate
  - Excluded additional test infrastructure from published package

### Fixed

- Fixed flaky CI tests on macOS under high load by using more robust timing margins
- Improved test reliability for high packet loss scenarios using appropriate sync presets

## [0.1.2] - 2025-12-19

### Changed

- Reduced published crate size from 2.43 MiB to ~300 KiB (88% reduction)
  - Excluded `actionlint` binary that was accidentally committed
  - Excluded `tests/` directory (users can clone repo to run tests)
  - Excluded `Cargo.lock` (not needed for library crates)
  - Excluded development config files (`.cargo/`, `.config/`, `clippy.toml`, etc.)
  - Excluded LLM instruction files (`AGENTS.md`, `CLAUDE.md`, `.llm/`)
  - Excluded `supply-chain/` cargo-vet metadata

## [0.1.1] - 2025-12-19

### Changed

- Reduced crate size by excluding irrelevant artifacts from published package

## [0.1.0] - 2025-12-19

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
- Comprehensive test suite: 1100+ library and integration tests, multi-process network tests passing
- TLA+ `Concurrency.tla` specification for `GameStateCell` thread safety verification

### Changed

#### Determinism Improvements

- Replaced all `HashMap` with `BTreeMap` for guaranteed iteration order
- Replaced all `HashSet` with `BTreeSet` for deterministic iteration
- `Config::Address` now requires `Ord` + `PartialOrd` trait bounds (see Migration section)
- Test infrastructure uses `fnv1a_hash` instead of `DefaultHasher`

#### Default Behavior

- **Desync detection now enabled by default**: `DesyncDetection::On { interval: 60 }` (once per second at 60fps). This catches state divergence early. Users needing to disable detection can explicitly set `DesyncDetection::Off`.

#### Branding

- Crate renamed to `fortress-rollback` (import as `fortress_rollback`)
- All `Ggrs*` types renamed to `Fortress*`:
  - `GgrsError` → `FortressError`
  - `GgrsEvent<T>` → `FortressEvent<T>`
  - `GgrsRequest<T>` → `FortressRequest<T>`
- All documentation updated to reference "Fortress Rollback"

#### Performance Improvements

- `FortressRequest::AdvanceFrame { inputs }` now uses `InputVec<T::Input>` (a `SmallVec<[(T::Input, InputStatus); 4]>`) instead of `Vec`. This avoids heap allocations for games with 1-4 players.
- `synchronized_inputs()` returns `InputVec` for stack-allocated inputs in the common case

#### Safety Improvements

- `InputQueue::confirmed_input` now returns `Result` instead of panicking
- Spectator confirmed-input path bubbles `FortressError` on missing data

### Fixed

- Crash when misprediction detected at frame 0 (first frame): `adjust_gamestate()` no longer attempts to load the current frame when receiving early corrections
- Multi-process rollback desync (BUG-001): Window-based checksum computation using last 64 frames ensures frames are always available for comparison between peers
- `PlayerRegistry::spectator_handles()` incorrectly returned local player handles in addition to spectators
- All 35 multi-process network tests now pass reliably

### Removed

- Historical GGRS changelog entries moved to [docs/ggrs-changelog-archive.md](docs/ggrs-changelog-archive.md)

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
fortress-rollback = "0.2"
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

### Input Vector Type

The `inputs` field in `FortressRequest::AdvanceFrame` is now `InputVec<T::Input>` (a `SmallVec`) instead of `Vec`:

```rust
// If you have explicit type annotations:
// Before
fn handle_inputs(inputs: Vec<(MyInput, InputStatus)>) { ... }

// After
use fortress_rollback::InputVec;
fn handle_inputs(inputs: InputVec<MyInput>) { ... }
// Or accept a slice for flexibility:
fn handle_inputs(inputs: &[(MyInput, InputStatus)]) { ... }
```

### Behavioral Notes

- Session termination using `confirmed_frame()` alone is incorrect; use the new `SyncHealth` API for proper synchronization verification. See [docs/migration.md](docs/migration.md) for details.
- **Desync detection is now enabled by default** (`DesyncDetection::On { interval: 60 }`). GGRS defaulted to `Off`. Explicitly set `DesyncDetection::Off` if you need the old behavior.

---

For detailed migration instructions, see [docs/migration.md](docs/migration.md).

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/wallstop/fortress-rollback/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/wallstop/fortress-rollback/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/wallstop/fortress-rollback/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/wallstop/fortress-rollback/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/wallstop/fortress-rollback/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/wallstop/fortress-rollback/releases/tag/v0.1.0
