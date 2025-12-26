# Fortress Rollback Test Organization

This document describes the test organization strategy for Fortress Rollback.

## Overview

Fortress Rollback uses a hybrid test organization following Rust best practices:

| Test Type | Location | Count | Purpose |
|-----------|----------|-------|---------|
| **Unit Tests** | `src/**/*.rs` (inline) | ~885 | Test private implementation details |
| **Integration Tests** | `tests/` | ~314 | Test public API and module interactions |
| **Property Tests** | `src/**/*.rs` + `tests/verification/` | ~150 | Random input verification |
| **Loom Tests** | `loom-tests/` | Separate | Concurrency correctness |
| **Fuzz Tests** | `fuzz/` | 7 targets | Security and edge cases |

## Why Hybrid Organization?

Rust's visibility rules require that tests accessing private (`pub(crate)` or non-public) items must be inline with `#[cfg(test)]`. Since most unit tests use `use super::*` to access implementation details, they cannot be moved to `tests/`.

Integration tests in `tests/` can only access the public API (items exported via `pub use` in `lib.rs`).

## Directory Structure

```
tests/
├── README.md              # This file
├── common/                # Shared test infrastructure
│   ├── mod.rs            # Re-exports stubs modules
│   ├── stubs.rs          # GameStub with struct-based inputs
│   └── stubs_enum.rs     # GameStub with enum-based inputs
├── network-peer/          # Separate crate for test binary
│   ├── Cargo.toml        # Avoids Cargo.lock in main crate
│   └── src/main.rs       # Multi-process network testing peer
├── config.rs              # Configuration struct integration tests
├── loom_concurrency.rs    # Loom integration tests (require loom feature)
├── network.rs             # Network test module root
├── network/               # Network integration tests
│   ├── multi_process.rs  # Real UDP multi-process tests
│   └── resilience.rs     # ChaosSocket resilience tests
├── sessions.rs            # Sessions test module root
├── sessions/              # Session integration tests
│   ├── macro_tests.rs    # handle_requests! macro tests
│   ├── p2p.rs            # P2P session tests (struct inputs)
│   ├── p2p_enum.rs       # P2P session tests (enum inputs)
│   ├── spectator.rs      # Spectator session tests
│   ├── synctest.rs       # SyncTest session tests (struct inputs)
│   └── synctest_enum.rs  # SyncTest session tests (enum inputs)
├── verification.rs        # Verification test module root
└── verification/          # Formal verification tests
    ├── determinism.rs    # Determinism verification
    ├── invariants.rs     # Internal invariant tests
    ├── metamorphic.rs    # Metamorphic testing
    ├── property.rs       # Property-based tests (proptest)
    └── z3.rs             # Z3 SMT solver proofs (requires z3-verification)
```

## Test Categories

### Unit Tests (Inline in `src/`)

Located within each source file under `#[cfg(test)] mod tests`.

**Purpose:** Test internal implementation details, edge cases, and private functions.

**Examples:**

- `src/hash.rs` - Hash function determinism, collision resistance
- `src/rle.rs` - RLE encoding/decoding edge cases
- `src/rng.rs` - PCG32 PRNG statistical properties
- `src/input_queue/mod.rs` - Queue operations, prediction strategies
- `src/sync_layer/mod.rs` - State save/load, rollback mechanics
- `src/network/protocol/mod.rs` - Protocol state machine transitions

### Integration Tests (`tests/`)

Test the public API as users would consume it.

#### `tests/sessions/`

End-to-end session lifecycle tests:

- Session creation via `SessionBuilder`
- Player management (add, remove, disconnect)
- Synchronization between peers
- Frame advancement and rollback
- Input handling with various input types

#### `tests/network/`

Network layer integration:

- Multi-process UDP communication
- Resilience under packet loss, latency, jitter
- Connection establishment and recovery

#### `tests/verification/`

Formal verification and property testing:

- **property.rs** - Proptest-based invariant verification
- **invariants.rs** - Internal invariant checking with `InvariantChecker`
- **determinism.rs** - Determinism verification across sessions
- **metamorphic.rs** - Metamorphic testing relationships
- **z3.rs** - Z3 SMT solver proofs (requires `z3-verification` feature)

#### `tests/config.rs`

Configuration struct tests:

- Default values
- Preset methods (`.high_latency()`, `.competitive()`, etc.)
- Configuration validation

### Property Tests

Split between inline and integration tests:

- **Inline** (`src/**/mod.rs#property_tests`): Access internal state
- **Integration** (`tests/verification/property.rs`): Use `__internal` module

### Loom Tests (`loom-tests/`)

Separate crate for Loom concurrency testing. Tests `GameStateCell` thread safety.

```bash
cd loom-tests && cargo test
```

### Fuzz Tests (`fuzz/`)

Cargo-fuzz targets for security testing:

- `fuzz_compression` - Message compression
- `fuzz_input_queue` - Input queue operations
- `fuzz_input_queue_direct` - Direct input queue fuzzing
- `fuzz_message_parsing` - Message parsing
- `fuzz_rle` - RLE encoding
- `fuzz_session_config` - Session configuration
- `fuzz_sync_layer_direct` - Direct sync layer fuzzing

```bash
cargo +nightly fuzz run <target>
```

## Shared Test Infrastructure

### `tests/common/stubs.rs`

Provides `GameStub` and related types for testing:

```rust
use crate::common::stubs::{GameStub, StubConfig, StubInput, StateStub};
```

- `StubConfig` - Implements `Config` trait for tests
- `StubInput` - Simple u32-based input
- `StateStub` - Simple game state with frame and state values
- `GameStub` - Handles `FortressRequest` for test sessions
- `RandomChecksumGameStub` - Variant that produces random checksums (for desync testing)

### `tests/common/stubs_enum.rs`

Same as above but with enum-based inputs:

```rust
use crate::common::stubs_enum::{GameStubEnum, StubConfigEnum, StubInputEnum};
```

## Running Tests

```bash
# Run all tests (recommended: uses nextest for 12x faster execution)
cargo nextest run

# Run standard cargo test
cargo test

# Run with specific features
cargo test --features z3-verification

# Run specific test category
cargo nextest run --test config          # Configuration tests
cargo nextest run --test sessions        # Session tests
cargo nextest run --test network         # Network tests
cargo nextest run --test verification    # Verification tests

# Run loom tests
cd loom-tests && cargo test

# Run fuzz tests
cargo +nightly fuzz run fuzz_rle -- -max_total_time=60
```

## Test Organization Philosophy

1. **Unit tests stay inline** when they:
   - Access private functions or types
   - Use `super::*` imports
   - Test implementation details

2. **Integration tests go in `tests/`** when they:
   - Only use the public API
   - Test module interactions
   - Simulate real-world usage

3. **Property tests** use proptest for randomized testing:
   - Inline for internal invariants
   - In `tests/verification/property.rs` for API-level properties

4. **Formal verification** in `tests/verification/`:
   - Z3 SMT solver proofs
   - Invariant checkers
   - Metamorphic relations

## Coverage Metrics

Current test coverage (as of December 2025):

- **Line Coverage:** 91.77%
- **Region Coverage:** 93.73%
- **Function Coverage:** 97.35%
- **Total Tests:** 1,131+

Coverage is tracked in CI and can be viewed on the repository's coverage dashboard.

## Adding New Tests

### Adding Unit Tests

Add to the existing `#[cfg(test)] mod tests` block in the relevant source file:

```rust
// In src/some_module.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_functionality() {
        // Test implementation
    }
}
```

### Adding Integration Tests

Add to the appropriate directory under `tests/`:

```rust
// In tests/sessions/new_test.rs
use crate::common::stubs::{GameStub, StubConfig};
use fortress_rollback::{SessionBuilder, PlayerType, PlayerHandle};

#[test]
fn test_new_session_feature() {
    // Test public API
}
```

Then add the module to the parent file (e.g., `tests/sessions.rs`):

```rust
mod sessions {
    pub mod new_test;
    // ... existing modules
}
```

### Adding Property Tests

For API-level properties, add to `tests/verification/property.rs`:

```rust
proptest! {
    #[test]
    fn prop_new_invariant(input in any::<u32>()) {
        // Property test
    }
}
```

For internal invariants, add to the `property_tests` module in the source file.
