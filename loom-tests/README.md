<p align="center">
  <img src="../assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# Fortress Rollback Loom Tests

This crate contains [loom](https://docs.rs/loom) concurrency tests for Fortress Rollback.

## Why a Separate Crate?

Loom tests need to be isolated from the main crate's dev-dependencies because:

1. **Incompatible dependencies**: Some dependencies (like `tokio`, `hyper-util`)
   use `#![cfg(not(loom))]` to disable modules under loom, causing compilation failures
   when building with `RUSTFLAGS="--cfg loom"`.

2. **Heavy dev-dependencies**: The main crate has many dev-dependencies (macroquad,
   criterion, z3, etc.) that don't need to be compiled for loom testing.

3. **Minimal test surface**: Loom tests focus specifically on concurrent primitives,
   so they only need the core library.

## Running Loom Tests

```bash
cd loom-tests
RUSTFLAGS="--cfg loom" cargo test --release
```

### Configuration Options

- `LOOM_MAX_PREEMPTIONS=N` - Limit thread preemptions (reduces state space)
- `LOOM_CHECKPOINT_FILE=file.json` - Save progress for long-running tests
- `LOOM_LOG=trace` - Enable detailed logging for debugging failures
- `LOOM_LOCATION=1` - Include source locations in panic messages

### Debugging Failures

If a test fails, save the checkpoint and replay:

```bash
# First run - save checkpoint
LOOM_CHECKPOINT_FILE=failure.json RUSTFLAGS="--cfg loom" cargo test --release failing_test

# Replay specific failure
LOOM_CHECKPOINT_INTERVAL=1 LOOM_LOG=trace LOOM_LOCATION=1 \
  LOOM_CHECKPOINT_FILE=failure.json \
  RUSTFLAGS="--cfg loom" cargo test --release failing_test
```

## Current Tests

### GameStateCell Tests (`loom_game_state_cell.rs`, 5 tests)

- `test_concurrent_saves` - Multiple threads saving concurrently
- `test_save_load_consistency` - Save and load never see partial state
- `test_multiple_readers_single_writer` - MRSW pattern verification
- `test_frame_advancement_pattern` - Rollback save pattern simulation
- `test_concurrent_access_bounded` - Bounded model checking with 3 threads

### SavedStates Tests (`loom_saved_states.rs`, 5 tests)

- `test_concurrent_save_access` - Multiple threads accessing saved states
- `test_save_overwrite_ordering` - Proper ordering when overwriting states
- `test_circular_buffer_wraparound` - Buffer wraparound under concurrency
- `test_frame_lookup_consistency` - Frame lookup returns correct states
- `test_concurrent_iteration` - Safe iteration under concurrent access

All tests verify that `GameStateCell` and `SavedStates` operations are atomic
and that no partial state can ever be observed.

## Architecture Notes

The main library (`fortress-rollback`) uses conditional compilation for loom:

- **Production**: Uses `parking_lot::Mutex` for performance
- **Loom testing**: Uses `loom::sync::Mutex` for exhaustive interleaving testing

Key implementation details:

- `src/sync.rs` - Abstraction module that re-exports the appropriate primitives
- `GameStateCell::data()` returns `None` under loom (no `MappedMutexGuard` equivalent)
- Tests should use `load()` which requires `T: Clone`

## References

- [Loom documentation](https://docs.rs/loom)
- [Loom GitHub](https://github.com/tokio-rs/loom)
- [CDSChecker paper](http://demsky.eecs.uci.edu/publications/c11modelcheck.pdf)
