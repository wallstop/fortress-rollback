# Rust Testing Best Practices — Comprehensive Guide for Agentic Workflows

> **This document provides best practices for writing, organizing, and maintaining tests in Rust.**
> These patterns help AI agents and developers write high-quality tests that are maintainable, fast, and reliable.

## TL;DR — Quick Reference

```bash
# Run tests with nextest (12x faster) — ALWAYS use --no-capture
cargo nextest run --no-capture

# Run specific test
cargo nextest run test_name --no-capture

# Run tests matching pattern
cargo nextest run -E 'test(parse_) | test(validate_)' --no-capture

# Run with retries for flaky tests (NOT recommended — fix flakiness instead)
cargo nextest run --retries 2 --no-capture

# Run doc tests (nextest doesn't support them)
cargo test --doc -- --nocapture

# Run slow/ignored tests
cargo test -- --ignored --nocapture
```

> **CRITICAL: Always capture test output.** Use `--no-capture` (nextest) or `-- --nocapture` (cargo test) so that failure output is immediately visible without re-running.

**Key Principles:**

1. **Test features, not code** — Tests should survive refactors
2. **One assertion per test** — Clear failure messages
3. **Use descriptive names** — `test_parse_empty_input_returns_error` not `test1`
4. **Consolidate integration tests** — One crate in `tests/it/` for faster compilation
5. **Write `check` helpers** — Decouple tests from API changes

---

## Test Organization — Where Tests Go

### Unit Tests: Inline with `#[cfg(test)]`

Unit tests belong **in the same file** as the code they test:

```rust
// src/parser.rs
pub fn parse(input: &str) -> Result<Ast, ParseError> {
    // Implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_returns_error() {
        assert!(parse("").is_err());
    }

    #[test]
    fn parse_valid_expression() {
        let ast = parse("1 + 2").unwrap();
        assert_eq!(ast.kind, AstKind::BinaryOp);
    }
}
```

**Why inline tests:**

- Access to private functions via `use super::*`
- Tests live next to the code they verify
- Easy to see what's tested when reading code

### Separate Test File Pattern (Large Modules)

For large modules, move tests to a separate file to reduce noise:

```rust
// src/parser.rs
pub fn parse(input: &str) -> Result<Ast, ParseError> {
    // Implementation
}

#[cfg(test)]
mod tests;  // Tests in src/parser/tests.rs
```

```rust
// src/parser/tests.rs (or src/parser_tests.rs adjacent to module)
use super::*;

#[test]
fn parse_empty_returns_error() {
    assert!(parse("").is_err());
}
```

**Benefit:** When you modify only tests, Cargo doesn't recompile the library crate.

### Integration Tests: Single Crate Pattern (Recommended)

Each file in `tests/` compiles as a **separate crate**, causing slow incremental builds.

**❌ Anti-pattern — Multiple test crates:**

```text
tests/
  foo.rs       ← Compiled as separate crate
  bar.rs       ← Compiled as separate crate
  baz.rs       ← Compiled as separate crate
```

**✅ Best practice — Single test crate:**

```text
tests/
  it/
    main.rs    ← Single crate entry point
    foo.rs
    bar.rs
    baz.rs
```

```rust
// tests/it/main.rs
mod foo;
mod bar;
mod baz;
```

```rust
// tests/it/foo.rs
use my_crate::*;

#[test]
fn test_foo_functionality() {
    // Test public API
}
```

**Impact:** Cargo's own test suite saw **3x compile time reduction** after consolidation.

### Shared Test Utilities

**Never** put helpers directly in `tests/`:

```
// ❌ WRONG — tests/common.rs is treated as a test crate
tests/
  common.rs           ← Cargo runs this as a test!
  integration_test.rs
```

**Use `mod.rs` pattern:**

```
// ✅ CORRECT — Subdirectories are not treated as test crates
tests/
  common/
    mod.rs            ← Not a test crate
  it/
    main.rs
```

```rust
// tests/common/mod.rs
pub fn setup_test_environment() -> TestEnv {
    // Shared setup code
}

pub fn create_test_session() -> Session {
    // Reusable fixture
}
```

```rust
// tests/it/main.rs
mod common;
use common::*;

mod session_tests;
```

---

## Test Writing Patterns

### The `check` Helper Pattern (Critical for Maintainability)

**Problem:** Every test directly calls the function, making API changes painful.

```rust
// ❌ BAD: Changing API requires updating every test
#[test]
fn test_empty() {
    let result = binary_search(&[], &0);
    assert_eq!(result, false);
}

#[test]
fn test_found() {
    let result = binary_search(&[1, 2, 3], &2);
    assert_eq!(result, true);
}
```

**Solution:** Create a `check` helper that wraps the API:

```rust
// ✅ GOOD: Change only `check` when API changes
#[track_caller]  // Shows caller location in panic messages
fn check(haystack: &[i32], needle: i32, expected: bool) {
    let actual = binary_search(haystack, &needle);
    assert_eq!(
        actual, expected,
        "binary_search({:?}, {}) = {}, expected {}",
        haystack, needle, actual, expected
    );
}

#[test]
fn test_empty() {
    check(&[], 0, false);
}

#[test]
fn test_found() {
    check(&[1, 2, 3], 2, true);
}

#[test]
fn test_not_found() {
    check(&[1, 2, 3], 4, false);
}
```

**Benefits:**

- API changes only touch `check`
- Consistent error messages
- Lower cognitive barrier for writing tests
- `#[track_caller]` ensures failure shows actual test location

### Descriptive Test Names

Test names should describe:

1. **What** is being tested
2. **Under what conditions**
3. **Expected behavior**

```rust
// ❌ BAD: Unclear what's being tested
#[test]
fn test1() { }
#[test]
fn it_works() { }
#[test]
fn issue_123() { }  // What was issue 123?

// ✅ GOOD: Self-documenting
#[test]
fn parse_empty_input_returns_none() { }
#[test]
fn session_disconnect_timeout_triggers_after_5_seconds() { }
#[test]
fn rollback_preserves_confirmed_frames() { }
```

**Including issue numbers:** Add as comment, not name prefix:

```rust
/// Regression test for <https://github.com/org/repo/issues/123>
/// Session was panicking when player count was zero.
#[test]
fn session_with_zero_players_returns_error() {
    let result = Session::new(0);
    assert!(result.is_err());
}
```

### Arrange-Act-Assert Pattern

Structure tests consistently:

```rust
#[test]
fn player_join_updates_count() {
    // Arrange: Set up test conditions
    let mut session = Session::new(2).unwrap();
    let initial_count = session.player_count();

    // Act: Execute the behavior being tested
    session.add_player(PlayerHandle::new(0)).unwrap();

    // Assert: Verify expected outcomes
    assert_eq!(session.player_count(), initial_count + 1);
}
```

### Testing Error Cases

Always test error paths, not just happy paths:

```rust
#[test]
fn add_player_with_invalid_handle_returns_error() {
    let mut session = Session::new(2).unwrap();

    let result = session.add_player(PlayerHandle::new(999));

    assert!(matches!(
        result,
        Err(FortressError::InvalidPlayerHandle { handle: 999, .. })
    ));
}

#[test]
fn parse_malformed_input_provides_error_context() {
    let result = parse("invalid{{");

    let err = result.unwrap_err();
    assert!(err.to_string().contains("unexpected token"));
    assert_eq!(err.line(), 1);
    assert_eq!(err.column(), 8);
}
```

### Tests That Return `Result`

Use `Result` return type to leverage `?` operator:

```rust
#[test]
fn complex_setup_with_result() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_test_config()?;
    let session = Session::with_config(config)?;
    let player = session.add_player(PlayerHandle::new(0))?;

    assert_eq!(player.state(), PlayerState::Connected);
    Ok(())
}
```

**Note:** Can't use `#[should_panic]` with `Result` return type.

### Testing Panics

```rust
#[test]
#[should_panic(expected = "index out of bounds")]
fn access_invalid_index_panics() {
    let arr = [1, 2, 3];
    let _ = arr[10];  // Panics
}
```

**Prefer checking expected substring** to avoid brittle tests:

```rust
// ❌ Brittle: Exact message may change
#[should_panic(expected = "index out of bounds: the len is 3 but the index is 10")]

// ✅ Robust: Key phrase
#[should_panic(expected = "index out of bounds")]
```

---

## Test Performance

### Avoid IO in Tests

The biggest performance killer in tests is **IO**:

```rust
// ❌ SLOW: File IO in every test
#[test]
fn test_config_parsing() {
    let config = load_from_file("test_config.toml").unwrap();
    assert_eq!(config.timeout, 30);
}

// ✅ FAST: Parse string directly
#[test]
fn test_config_parsing() {
    let config = parse_config(r#"
        timeout = 30
        max_players = 4
    "#).unwrap();
    assert_eq!(config.timeout, 30);
}
```

### Mark Slow Tests

Gate slow tests behind environment variable:

```rust
#[test]
fn slow_network_integration_test() {
    if std::env::var("RUN_SLOW_TESTS").is_err() {
        eprintln!("Skipping slow test. Set RUN_SLOW_TESTS=1 to run.");
        return;
    }

    // Expensive test...
}
```

**Don't use `#[cfg(...)]`** — it hides tests from `cargo test --list`.

### Disable Doc Tests for Internal Libraries

Doc tests compile as **separate binaries** — extremely slow:

```toml
# Cargo.toml — For internal libraries
[lib]
doctest = false
```

### Use cargo-nextest

10-60% faster than `cargo test`:

```bash
cargo install cargo-nextest --locked
cargo nextest run
```

---

## Common Anti-Patterns

### ❌ Testing Probabilistic Properties

```rust
// ❌ BAD: Hash collisions are mathematically possible
#[test]
fn different_inputs_produce_different_hashes() {
    let hash1 = fnv1a_hash(&42u32);
    let hash2 = fnv1a_hash(&43u32);
    assert_ne!(hash1, hash2);  // Could fail due to collision!
}

// ✅ GOOD: Test determinism instead
#[test]
fn same_input_produces_same_hash() {
    let hash1 = fnv1a_hash(&42u32);
    let hash2 = fnv1a_hash(&42u32);
    assert_eq!(hash1, hash2);
}

// ✅ GOOD: Test known vectors
#[test]
fn fnv1a_matches_known_vectors() {
    assert_eq!(fnv1a_hash(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a_hash(b"a"), 0xaf63_dc4c_8601_ec8c);
}
```

See [property-testing.md](property-testing.md) for more on avoiding flaky property tests.

### ❌ Re-implementing Production Logic in Tests

```rust
// ❌ BAD: Test duplicates production match logic
#[test]
fn test_error_reason_mapping() {
    let error = create_error();

    // This duplicates production's map_error_to_reason()!
    let reason = match &error {
        Error::Specific { reason } => *reason,
        _ => Reason::Unknown,
    };

    assert_eq!(reason, Reason::Expected);
}

// ✅ GOOD: Call production code directly
#[test]
fn test_error_reason_mapping() {
    let error = create_error();
    let reason = map_error_to_reason(&error);  // Production function
    assert_eq!(reason, Reason::Expected);
}
```

### ❌ Thread Ownership Issues in Concurrent Tests

```rust
// ❌ BAD: Value moved into thread, can't use after
#[test]
fn test_concurrent_access() {
    let cell = GameStateCell::new();
    let handle = thread::spawn(move || {
        cell.save(Frame::new(1), Some(42), None);  // cell moved
    });
    handle.join().unwrap();
    assert_eq!(cell.load(), Some(42));  // ERROR: cell was moved!
}

// ✅ GOOD: Clone Arc before spawning threads
#[test]
fn test_concurrent_access() {
    let cell = Arc::new(GameStateCell::new());
    let cell_for_thread = cell.clone();  // Clone BEFORE spawn

    let handle = thread::spawn(move || {
        cell_for_thread.save(Frame::new(1), Some(42), None);
    });

    handle.join().unwrap();
    assert_eq!(cell.load(), Some(42));  // Original Arc still available
}
```

### ❌ Testing Implementation, Not Behavior

```rust
// ❌ BAD: Tests internal implementation detail
#[test]
fn uses_hashmap_internally() {
    let cache = Cache::new();
    // Somehow checks internal HashMap exists
}

// ✅ GOOD: Tests observable behavior
#[test]
fn cache_returns_stored_value() {
    let mut cache = Cache::new();
    cache.set("key", "value");
    assert_eq!(cache.get("key"), Some("value"));
}
```

### ❌ Weak Assertions

```rust
// ❌ BAD: Mutation testing will reveal this as weak
#[test]
fn test_add() {
    let result = add(2, 2);
    assert!(result > 0);  // Would pass for add(2, 2) = 1
}

// ✅ GOOD: Exact assertion
#[test]
fn test_add() {
    assert_eq!(add(2, 2), 4);
}
```

### ❌ Testing Multiple Things

```rust
// ❌ BAD: Multiple assertions testing different behaviors
#[test]
fn test_everything() {
    let mut session = Session::new(4).unwrap();
    assert_eq!(session.player_count(), 0);

    session.add_player(PlayerHandle::new(0)).unwrap();
    assert_eq!(session.player_count(), 1);

    session.start().unwrap();
    assert_eq!(session.state(), SessionState::Running);
}

// ✅ GOOD: Separate tests for each behavior
#[test]
fn new_session_has_zero_players() {
    let session = Session::new(4).unwrap();
    assert_eq!(session.player_count(), 0);
}

#[test]
fn add_player_increments_count() {
    let mut session = Session::new(4).unwrap();
    session.add_player(PlayerHandle::new(0)).unwrap();
    assert_eq!(session.player_count(), 1);
}

#[test]
fn start_changes_state_to_running() {
    let mut session = create_ready_session();
    session.start().unwrap();
    assert_eq!(session.state(), SessionState::Running);
}
```

### ❌ Sleep-Based Synchronization

```rust
// ❌ BAD: Flaky and slow
#[test]
fn async_operation_completes() {
    start_async_task();
    std::thread::sleep(Duration::from_secs(1));
    assert!(task_completed());
}

// ✅ GOOD: Use proper synchronization
#[test]
fn async_operation_completes() {
    let (tx, rx) = std::sync::mpsc::channel();
    start_async_task_with_callback(move || tx.send(()).unwrap());
    rx.recv_timeout(Duration::from_secs(5)).expect("Task should complete");
}
```

### ❌ Untracked Background Work

```rust
// ❌ BAD: No way to wait for completion
fn do_work() {
    std::thread::spawn(|| {
        // Work happens in background
    });
}

// ✅ GOOD: Return handle to wait on
fn do_work() -> JoinHandle<Result<(), Error>> {
    std::thread::spawn(|| {
        // Work happens in background
        Ok(())
    })
}
```

---

## Testing Tools Reference

### Core Tools

| Tool | Purpose | Usage |
|------|---------|-------|
| `cargo test` | Built-in test runner | `cargo test` |
| `cargo-nextest` | Faster test runner | `cargo nextest run` |
| `#[cfg(test)]` | Compile tests only in test mode | Module annotation |
| `#[test]` | Mark function as test | Function attribute |
| `#[should_panic]` | Test expects panic | Function attribute |
| `#[ignore]` | Skip test by default | Function attribute |

### Extended Testing Ecosystem

| Tool | Purpose | When to Use |
|------|---------|-------------|
| **proptest** | Property-based testing | Invariants, edge cases |
| **quickcheck** | Property testing (simpler) | Quick property tests |
| **rstest** | Fixtures, parameterized tests | Shared setup, test matrices |
| **test-case** | Parameterized tests | Table-driven testing |
| **mockall** | Mocking traits | Unit testing with dependencies |
| **wiremock** | HTTP mocking | Testing HTTP clients |
| **insta** | Snapshot testing | Complex output verification |
| **fake** | Test data generation | Realistic test data |
| **cargo-mutants** | Mutation testing | Verify test quality |

### Installation Commands

```bash
# Test runner
cargo install cargo-nextest --locked

# Mutation testing
cargo install --locked cargo-mutants

# Snapshot testing
cargo install cargo-insta
```

### Cargo.toml Dev Dependencies

```toml
[dev-dependencies]
# Property testing
proptest = "1.5"

# Parameterized tests
rstest = "0.22"

# Mocking
mockall = "0.13"

# Snapshot testing
insta = { version = "1.40", features = ["yaml"] }

# Test data generation
fake = { version = "3", features = ["derive"] }

# Better assertion diffs
pretty_assertions = "1.4"

# Temporary files
tempfile = "3.12"
```

---

## Parameterized Testing with rstest

For table-driven tests with shared setup:

```rust
use rstest::{rstest, fixture};

#[fixture]
fn session() -> Session {
    Session::new(4).unwrap()
}

#[rstest]
#[case(0, true)]
#[case(1, true)]
#[case(4, false)]  // Max players exceeded
#[case(999, false)]
fn add_player_validity(
    mut session: Session,
    #[case] player_id: usize,
    #[case] expected_valid: bool
) {
    let result = session.add_player(PlayerHandle::new(player_id));
    assert_eq!(result.is_ok(), expected_valid);
}
```

---

## Snapshot Testing with insta

For testing complex outputs:

```rust
use insta::{assert_snapshot, assert_yaml_snapshot};

#[test]
fn test_error_formatting() {
    let error = parse("invalid{{syntax").unwrap_err();
    assert_snapshot!(error.to_string());
}

#[test]
fn test_ast_structure() {
    let ast = parse("1 + 2 * 3").unwrap();
    assert_yaml_snapshot!(ast);
}
```

```bash
# Review snapshots interactively
cargo insta review

# Update all snapshots
INSTA_UPDATE=always cargo test
```

---

## Test Quality Verification

### Run Mutation Testing

```bash
# Test specific module
cargo mutants -f src/parser.rs --timeout 30

# See what mutations would be applied
cargo mutants --list -f src/parser.rs
```

### Check Coverage

```bash
# Generate coverage report
cargo llvm-cov --html

# View in browser
open target/llvm-cov/html/index.html
```

---

## Checklist for Writing Tests

Before committing tests, verify:

- [ ] Test names describe what is being tested
- [ ] Each test verifies ONE behavior
- [ ] Error cases are tested, not just happy paths
- [ ] Assertions are specific (use `assert_eq!`, not just `assert!`)
- [ ] No `sleep()` for synchronization
- [ ] Shared setup uses fixtures or helper functions
- [ ] Integration tests are in single crate (`tests/it/`)
- [ ] No IO unless testing IO functionality
- [ ] `#[track_caller]` on helper functions

---

## Summary: Key Principles

1. **Test organization:**
   - Unit tests inline with `#[cfg(test)]`
   - Integration tests in single `tests/it/` crate
   - Shared utilities in `tests/common/mod.rs`

2. **Test writing:**
   - Use `check` helpers to decouple from API
   - Descriptive names: what + condition + expected
   - One assertion per test when practical
   - Always test error paths

3. **Test performance:**
   - Avoid IO (parse strings, not files)
   - Use nextest for faster execution
   - Mark slow tests for optional execution
   - Disable doc tests for internal libraries

4. **Test quality:**
   - Use exact assertions (`assert_eq!` over `assert!`)
   - Run mutation testing to verify coverage
   - Don't test implementation details

---

*See also: [property-testing.md](property-testing.md), [mutation-testing.md](mutation-testing.md), [defensive-programming.md](defensive-programming.md)*
