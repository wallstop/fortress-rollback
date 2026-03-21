<!-- CATEGORY: Testing -->
<!-- WHEN: Writing tests, choosing testing tools, test organization, nextest usage -->

# Rust Testing Guide

## Quick Reference

```bash
# Run tests with nextest (ALWAYS use --no-capture)
cargo nextest run --no-capture

# Run specific test / pattern
cargo nextest run test_name --no-capture
cargo nextest run -E 'test(parse_) | test(validate_)' --no-capture

# Run doc tests (nextest doesn't support them)
cargo test --doc -- --nocapture

# Run ignored/slow tests
cargo test -- --ignored --nocapture

# IMPORTANT: Never pipe test output through tail/head.
# Redirect to temp file and read it instead:
cargo nextest run --no-capture > /tmp/test-results.txt 2>&1
# For repeated runs (flakiness check), use a for loop:
for i in $(seq 1 10); do cargo nextest run --no-capture >> /tmp/flaky-check.txt 2>&1; done
```

## Test Organization

| Location | Purpose | Notes |
|----------|---------|-------|
| `src/foo.rs` with `#[cfg(test)] mod tests` | Unit tests | Access private fns via `use super::*` |
| `tests/it/main.rs` + submodules | Integration tests | Single crate = faster compile (3x improvement) |
| `tests/common/mod.rs` | Shared helpers | Never put helpers directly in `tests/` |

### Integration Test Structure (Recommended)

```text
tests/
  it/
    main.rs    # mod foo; mod bar;
    foo.rs
    bar.rs
  common/
    mod.rs     # Shared setup/fixtures
```

## Test Writing Patterns

### Arrange-Act-Assert + `check` Helpers

```rust
#[track_caller]
fn check(haystack: &[i32], needle: i32, expected: bool) {
    let actual = binary_search(haystack, &needle);
    assert_eq!(actual, expected, "binary_search({:?}, {}) = {}, expected {}", haystack, needle, actual, expected);
}

#[test]
fn test_found() { check(&[1, 2, 3], 2, true); }

#[test]
fn test_not_found() { check(&[1, 2, 3], 4, false); }
```

### Naming Convention

```rust
// Format: <what>_<condition>_<expected>
fn parse_empty_input_returns_none() { }
fn rollback_preserves_confirmed_frames() { }

/// Regression test for <https://github.com/org/repo/issues/123>
fn session_with_zero_players_returns_error() { }
```

### Testing Error Cases

```rust
#[test]
fn add_player_with_invalid_handle_returns_error() {
    // In tests: .unwrap() is idiomatic -- panics = test failure
    let mut session = Session::new(2).unwrap();
    let result = session.add_player(PlayerHandle::new(999));
    assert!(matches!(result, Err(FortressError::InvalidPlayerHandle { handle: 999, .. })));
}
```

## Tool Selection Guide

| Need | Tool | Usage |
|------|------|-------|
| Fast test execution | cargo-nextest | `cargo nextest run` |
| Find edge cases | proptest | `proptest! { }` |
| Table-driven tests | rstest / test-case | `#[rstest]` / `#[test_case]` |
| Mock dependencies | mockall | `#[automock]` |
| Mock HTTP APIs | wiremock | `MockServer::start()` |
| Snapshot testing | insta | `assert_snapshot!()` |
| Generate test data | fake | `Faker.fake()` |
| Verify test quality | cargo-mutants | `cargo mutants` |
| Fuzz testing | cargo-fuzz | `cargo fuzz run` |

### Recommended `[dev-dependencies]`

```toml
proptest = "1.5"
rstest = "0.22"
mockall = "0.13"
insta = { version = "1.40", features = ["yaml"] }
pretty_assertions = "1.4"
fake = { version = "3", features = ["derive"] }
tempfile = "3.12"
```

## Nextest Configuration

Create `.config/nextest.toml`:

```toml
[profile.default]
fail-fast = true
slow-timeout = { period = "60s", terminate-after = 2 }
retries = 0

[profile.ci]
fail-fast = false
retries = 2

[[profile.default.overrides]]
filter = "test(/integration/)"
threads-required = 2
```

### Filter Expressions

```bash
cargo nextest run -E 'test(parse)'           # Tests containing "parse"
cargo nextest run -E 'package(my_crate)'     # Tests in specific package
cargo nextest run -E 'not test(slow)'        # Exclude patterns
```

## Parameterized Testing (rstest)

```rust
use rstest::{rstest, fixture};

#[fixture]
fn session() -> Session { Session::new(4).unwrap() }

#[rstest]
#[case(0, true)]
#[case(1, true)]
#[case(4, false)]
fn add_player_validity(mut session: Session, #[case] player_id: usize, #[case] expected_valid: bool) {
    let result = session.add_player(PlayerHandle::new(player_id));
    assert_eq!(result.is_ok(), expected_valid);
}
```

### Test Matrix

```rust
#[rstest]
fn test_all_combinations(
    #[values(Compression::None, Compression::Zstd)] compression: Compression,
    #[values(true, false)] checksum: bool,
    #[values(1, 10, 100)] size: usize,
) {
    let result = process(&generate_data(size), compression, checksum);
    assert!(result.is_ok());
}
```

## Snapshot Testing (insta)

```rust
use insta::{assert_snapshot, assert_yaml_snapshot};

#[test]
fn test_error_formatting() {
    let error = parse("invalid{{syntax").unwrap_err();
    assert_snapshot!(error.to_string());
}
```

```bash
cargo insta review           # Review snapshots interactively
INSTA_UPDATE=always cargo test  # Accept all
```

## Common Anti-Patterns

| Anti-Pattern | Fix |
|-------------|-----|
| `assert!(result > 0)` (weak assertion) | `assert_eq!(result, 42)` |
| Testing multiple behaviors in one test | Split into focused tests |
| `thread::sleep()` for sync | Use channels/condvars |
| Re-implementing production logic in test | Call production function directly |
| Testing implementation, not behavior | Test observable behavior |
| Untracked background threads | Return `JoinHandle` |
| Value moved into thread | Clone `Arc` before `spawn` |
| Testing hash uniqueness | Test determinism + known vectors |

## Test Performance Tips

- Avoid IO in tests -- parse strings directly instead of files
- Gate slow tests behind `RUN_SLOW_TESTS` env var (not `#[cfg(...)]`)
- Disable doc tests for internal libraries: `[lib] doctest = false`
- Use nextest (10-60% faster than `cargo test`)

## Pre-Commit Checklist

- [ ] Descriptive test names (what + condition + expected)
- [ ] Each test verifies ONE behavior
- [ ] Error cases tested, not just happy paths
- [ ] Specific assertions (`assert_eq!`, not just `assert!`)
- [ ] No `sleep()` for synchronization
- [ ] Integration tests in `tests/it/` single crate
- [ ] `#[track_caller]` on helper functions
