# Mutation Testing — Improving Test Quality in Rust

> **This document provides comprehensive guidance for mutation testing in Rust.**
> Mutation testing verifies that tests actually check behavior, not just execute code paths.

## TL;DR — Quick Start

```bash
# Install cargo-mutants
cargo install --locked cargo-mutants

# Run on entire project
cargo mutants

# Run on specific file (recommended for efficiency)
cargo mutants -f src/some_module.rs

# List mutations without running
cargo mutants --list

# Fast mode: skip doctests, use nextest
cargo mutants -- --all-targets
```

**Key insight**: Code coverage tells you what code runs. Mutation testing tells you if your tests would notice if that code broke.

---

## What is Mutation Testing?

Mutation testing automatically injects small bugs ("mutants") into your code and verifies tests catch them.

### Mutation Outcomes

| Outcome | Meaning | Action |
|---------|---------|--------|
| **Caught** ✅ | Test failed → mutant killed | Good coverage |
| **Missed** ⚠️ | Tests still pass → mutant survived | Gap in test quality |
| **Timeout** ⏱️ | Test hung (usually infinite loop) | Usually acceptable |
| **Unviable** 🔨 | Mutation doesn't compile | Inconclusive |

### Example: Why Coverage Isn't Enough

```rust
fn frobb(i: i32) -> i32 {
    if i == 0 { 1 } else { 2 }
}

#[test]
fn test_frobb_0() { assert!(frobb(0) > 0); }  // Covers "if" branch

#[test]
fn test_frobb_1() { assert!(frobb(1) > 0); }  // Covers "else" branch
```

**100% line coverage!** But mutation testing reveals the tests are weak:

- Mutating `frobb` to always return `1` → **Tests still pass!**
- Mutating `i == 0` to `i != 0` → **Tests still pass!**

**Better tests:**

```rust
#[test]
fn test_frobb_0() { assert_eq!(frobb(0), 1); }  // Catches return value mutations

#[test]
fn test_frobb_1() { assert_eq!(frobb(1), 2); }  // Catches branch mutations
```

---

## Common Mutations

cargo-mutants applies these mutations:

| Category | Original | Mutated |
|----------|----------|---------|
| **Comparison** | `==` | `!=` |
| | `<` | `==`, `>`, `<=` |
| | `>` | `==`, `<`, `>=` |
| **Logical** | `&&` | `\|\|` |
| | `\|\|` | `&&` |
| **Arithmetic** | `+` | `-`, `*` |
| | `-` | `+`, `*` |
| **Returns** | `return x` | `return Default::default()` |
| | `true` | `false` |
| | `false` | `true` |
| **Statements** | statement | (deleted) |

---

## Installation & Configuration

### Installation

```bash
# Standard installation
cargo install --locked cargo-mutants

# Faster: use cargo-binstall
cargo binstall cargo-mutants
```

### Configuration File (`.cargo/mutants.toml`)

```toml
# Use nextest for faster test execution
test_tool = "nextest"

# Additional cargo test arguments
additional_cargo_test_args = ["--all-targets"]

# Exclude files from mutation testing
exclude_globs = [
    "examples/**/*.rs",
    "benches/**/*.rs",
    "tests/**/common/**/*.rs",
]

# Exclude by regex (function names, impl blocks)
exclude_re = [
    "impl Debug",
    "impl Display",
]

# Include by regex (focus on specific areas)
# examine_re = ["impl Serialize"]

# Timeout multiplier (default: 5x baseline)
timeout_multiplier = 10.0

# Optimize builds for mutation testing
profile = "mutants"

# Feature configuration
all_features = false
features = ["tokio"]
```

### Build Profile for Faster Mutations

Add to `Cargo.toml`:

```toml
[profile.mutants]
inherits = "test"
debug = "none"  # Faster builds, simpler failure output
```

---

## Command Reference

### Basic Usage

```bash
# Run on entire project
cargo mutants

# Verbose output with timing
cargo mutants -vV

# Target specific file
cargo mutants -f src/path/to/file.rs

# Target specific directory
cargo mutants -f src/network/*.rs

# Exclude file
cargo mutants -e src/main.rs

# List mutations without running tests
cargo mutants --list
```

### Filtering by Function/Name

```bash
# Only test functions matching regex
cargo mutants -F 'encode|decode'

# Exclude functions matching regex
cargo mutants -E 'impl Debug|impl Display'
```

### Performance Optimization

```bash
# Skip doctests (they're slow)
cargo mutants -- --all-targets

# Use in-place mode (CI, faster)
cargo mutants --in-place

# Skip baseline (when tests already passed)
cargo mutants --baseline=skip --timeout 300

# Parallel jobs (start conservative, watch memory)
cargo mutants -j2
```

### Incremental Testing (PR diffs)

```bash
# Generate diff from base branch
git diff origin/main.. > changes.diff

# Test only changed code
cargo mutants --in-diff changes.diff
```

### Sharding for CI

```bash
# Run shard 0 of 8 total shards
cargo mutants --shard 0/8 --baseline=skip --timeout 300
```

---

## Source Code Annotations

### Skip Functions

Add the `mutants` dependency:

```toml
[dev-dependencies]
mutants = "0.0.3"
```

```rust
#[mutants::skip]  // Skip this function from mutation testing
fn known_timeout_function() -> bool {
    // This function causes infinite loops when mutated
    loop_until_condition()
}

// Conditional skip (no runtime dependency)
#[cfg_attr(test, mutants::skip)]
fn might_hang() -> bool {
    // ...
}
```

**When to use `#[mutants::skip]`:**

| Situation | Skip? | Alternative |
|-----------|-------|-------------|
| Infinite loops when mutated | ✅ Yes | N/A |
| Debug/Display implementations | Maybe | `exclude_re` in config |
| Performance-critical hot paths | Maybe | Run selectively |
| Trivial getters/setters | Maybe | Usually test anyway |
| FFI/unsafe code | ✅ Yes | Manual review |

---

## Understanding Results

### Output Files (`mutants.out/`)

| File | Contents |
|------|----------|
| `mutants.json` | All generated mutants |
| `outcomes.json` | Test results for each mutant |
| `caught.txt` | Successfully caught mutants |
| `missed.txt` | **Review these!** Surviving mutants |
| `timeout.txt` | Mutants causing test timeouts |
| `diff/` | Diffs for each mutation |
| `logs/` | Build/test logs per mutant |

### Analyzing Missed Mutants

```bash
# View missed mutations
cat mutants.out/missed.txt

# View specific mutation diff
cat mutants.out/diff/src_lib_rs_123.diff

# View test output for a missed mutation
cat mutants.out/logs/src_lib_rs_123.log
```

### Categories of Missed Mutants

| Category | Example | Resolution |
|----------|---------|------------|
| **Missing assertion** | Return value not checked | Add `assert_eq!` |
| **Boundary condition** | `<` vs `<=` not tested | Add boundary tests |
| **Equivalent mutant** | `x * 1` → `x / 1` | Usually acceptable |
| **Dead code** | Unreachable branch mutated | Remove dead code |
| **Complex logic** | Nested conditions | Add specific tests |

---

## Writing Tests That Catch Mutants

### Principle: Assert Specific Values

```rust
// ❌ Weak: Only checks type/existence
assert!(result.is_ok());
assert!(value > 0);

// ✅ Strong: Checks exact values
assert_eq!(result, Ok(expected_value));
assert_eq!(value, 42);
```

### Principle: Test Boundaries

```rust
// ❌ Weak: Only tests one case
#[test]
fn test_is_valid_index() {
    assert!(is_valid_index(5, 10));  // index < len
}

// ✅ Strong: Tests boundaries
#[test]
fn test_is_valid_index_boundaries() {
    // At boundary
    assert!(is_valid_index(9, 10));   // index == len - 1
    assert!(!is_valid_index(10, 10)); // index == len
    
    // Zero case
    assert!(is_valid_index(0, 10));
    
    // Edge cases
    assert!(!is_valid_index(0, 0));   // empty collection
}
```

### Principle: Test Both Branches

```rust
// ❌ Weak: Only tests success path
#[test]
fn test_divide() {
    assert_eq!(divide(10, 2), Ok(5));
}

// ✅ Strong: Tests both branches
#[test]
fn test_divide_success() {
    assert_eq!(divide(10, 2), Ok(5));
}

#[test]
fn test_divide_by_zero() {
    assert_eq!(divide(10, 0), Err(DivideError::DivisionByZero));
}
```

### Principle: Test Operators Explicitly

```rust
// ❌ Weak: Doesn't distinguish operators
#[test]
fn test_calculation() {
    assert!(calculate(5, 3) > 0);  // True for +, *, -
}

// ✅ Strong: Tests exact result
#[test]
fn test_add() {
    assert_eq!(add(5, 3), 8);      // Would fail if + mutated to -
}

#[test]
fn test_multiply() {
    assert_eq!(multiply(5, 3), 15); // Would fail if * mutated to +
}
```

### Principle: Test State Changes

```rust
// ❌ Weak: Doesn't verify mutation occurred
#[test]
fn test_increment() {
    let mut counter = Counter::new();
    counter.increment();
    // No assertion!
}

// ✅ Strong: Verifies the effect
#[test]
fn test_increment() {
    let mut counter = Counter::new();
    assert_eq!(counter.value(), 0);
    counter.increment();
    assert_eq!(counter.value(), 1);
}
```

---

## CI/CD Integration

### GitHub Actions: Basic Setup

```yaml
name: Mutation Testing

on:
  push:
    branches: [main]
  schedule:
    - cron: '0 2 * * 1'  # Weekly, Monday 2am

jobs:
  mutants:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - uses: dtolnay/rust-toolchain@stable
      
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-mutants,cargo-nextest
      
      - name: Run mutation testing
        run: cargo mutants -vV --in-place --timeout 300
      
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: mutants-results
          path: mutants.out/
```

### GitHub Actions: Incremental PR Testing

```yaml
jobs:
  incremental-mutants:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-mutants,cargo-nextest
      
      - name: Generate diff
        run: git diff origin/${{ github.base_ref }}.. > changes.diff
      
      - name: Run incremental mutation testing
        run: cargo mutants --no-shuffle -vV --in-diff changes.diff --in-place
      
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: mutants-pr
          path: mutants.out/
```

### GitHub Actions: Sharded Full Testing

```yaml
jobs:
  # Run regular tests first
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --all-features

  # Shard mutation testing across 8 workers
  mutants:
    runs-on: ubuntu-latest
    needs: [test]
    strategy:
      fail-fast: false
      matrix:
        shard: [0, 1, 2, 3, 4, 5, 6, 7]
    steps:
      - uses: actions/checkout@v4
      
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-mutants,cargo-nextest
      
      - name: Run shard ${{ matrix.shard }}/8
        run: |
          cargo mutants --no-shuffle -vV \
            --shard ${{ matrix.shard }}/8 \
            --baseline=skip \
            --timeout 300 \
            --in-place
      
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: mutants-shard-${{ matrix.shard }}
          path: mutants.out/
```

---

## Performance Optimization

### Local Development

| Optimization | Command/Config | Speedup |
|--------------|----------------|---------|
| Target specific files | `cargo mutants -f src/module.rs` | 10-100x |
| Skip doctests | `-- --all-targets` | 2-5x |
| Use nextest | `test_tool = "nextest"` | 2-12x |
| Disable debug symbols | `debug = "none"` in profile | 20-30% |
| Use faster linker | Install `mold` or `wild` | 20-50% |
| Parallel jobs | `-j2` (careful with memory) | 1.5-2x |

### CI Optimization

| Optimization | How | Impact |
|--------------|-----|--------|
| In-place mode | `--in-place` | No tree copying |
| Skip baseline | `--baseline=skip` | No redundant test run |
| Sharding | `--shard N/M` | Distributed across workers |
| Incremental | `--in-diff file.diff` | Only test changed code |
| Ramdisk | `TMPDIR=/ram` | Faster I/O |

### Memory Considerations

Each parallel mutation job needs:

- ~2GB+ for cargo build directory
- Full test suite memory requirements

```bash
# Watch memory usage
htop &
cargo mutants -j2  # Start conservative

# Increase only if resources allow
cargo mutants -j3
```

---

## Workflow: Adapting Code for Mutation Testing

### Phase 1: Establish Baseline

```bash
# Install tooling
cargo install --locked cargo-mutants cargo-nextest

# Run initial scan
cargo mutants --list | head -50

# Run on small module first
cargo mutants -f src/simplest_module.rs
```

### Phase 2: Analyze Results

```bash
# Check missed mutations
cat mutants.out/missed.txt

# Categorize misses:
# - Missing assertions (easy fix)
# - Boundary conditions (medium fix)
# - Equivalent mutants (usually OK)
# - Complex logic (needs analysis)
```

### Phase 3: Improve Tests

Focus on **highest-value improvements**:

1. **Return value assertions** — Add `assert_eq!` for all return values
2. **Boundary tests** — Add tests at `0`, `max-1`, `max`, `max+1`
3. **Branch coverage** — Ensure both `if` and `else` tested
4. **Error paths** — Test all `Err` variants explicitly

### Phase 4: Configure Exclusions

After improving tests, exclude remaining acceptable misses:

```toml
# In .cargo/mutants.toml
exclude_re = [
    # Display/Debug impls - usually OK
    "impl Debug",
    "impl Display",
    
    # Known equivalent mutants
    "specific_function_name",
]
```

### Phase 5: CI Integration

1. Add weekly scheduled mutation testing
2. Add incremental PR testing
3. Monitor and investigate regressions

---

## Common Patterns for Fortress Rollback

### Testing Rollback Logic

```rust
#[test]
fn test_rollback_restores_exact_state() {
    let mut session = create_test_session();
    let original_state = session.current_state().clone();
    
    // Advance and save
    session.advance_frame();
    session.advance_frame();
    
    // Rollback
    session.rollback_to(Frame(0));
    
    // Verify EXACT state, not just "some state"
    assert_eq!(session.current_state(), &original_state);
}
```

### Testing Network Protocol

```rust
#[test]
fn test_input_acknowledged_exactly() {
    let (mut peer1, mut peer2) = create_connected_peers();
    
    // Send specific input
    peer1.add_local_input(INPUT_VALUE);
    exchange_messages(&mut peer1, &mut peer2);
    
    // Verify exact acknowledgment, not just "acked something"
    assert_eq!(peer2.received_inputs(), vec![(Frame(0), INPUT_VALUE)]);
}
```

### Testing Checksums

```rust
#[test]
fn test_checksum_detects_single_bit_flip() {
    let state1 = GameState::new();
    let state2 = state1.with_bit_flipped(42);
    
    // Checksums MUST differ
    assert_ne!(checksum(&state1), checksum(&state2));
}
```

---

## Troubleshooting

### "Too many timeouts"

**Cause**: Mutations cause infinite loops.

**Solutions**:

1. Add `#[mutants::skip]` to problematic functions
2. Increase `timeout_multiplier` in config
3. Investigate if loops have proper termination tests

### "All tests pass but shouldn't"

**Cause**: Assertions are too weak.

**Solutions**:

1. Review `missed.txt` for patterns
2. Strengthen assertions to check exact values
3. Add more specific test cases

### "Takes too long"

**Cause**: Rust compilation overhead × number of mutations.

**Solutions**:

1. Target specific files: `-f src/module.rs`
2. Skip doctests: `-- --all-targets`
3. Use faster linker (mold, wild)
4. Shard across CI workers

### "Out of memory"

**Cause**: Too many parallel jobs.

**Solutions**:

1. Reduce parallelism: `-j1` or `-j2`
2. Use `--in-place` mode
3. Increase swap space

---

## Resources

| Resource | Link |
|----------|------|
| cargo-mutants docs | <https://mutants.rs/> |
| GitHub repo | <https://github.com/sourcefrog/cargo-mutants> |
| RustConf 2024 talk | <https://www.youtube.com/watch?v=PjDHe-PkOy8> |

---

## Summary: Mutation Testing Best Practices

### Do ✅

- Use specific assertions (`assert_eq!` over `assert!`)
- Test boundary conditions explicitly
- Test both success and error paths
- Run mutation testing on critical modules first
- Configure exclusions for known-acceptable gaps
- Use incremental testing in PRs

### Don't ❌

- Rely only on code coverage metrics
- Ignore missed mutations without analysis
- Run full mutation testing on every commit
- Use weak assertions like `assert!(result.is_ok())`
- Skip test quality verification for critical paths

---

*Mutation testing validates that your tests catch bugs. Use it to find weak tests, improve coverage quality, and gain confidence that your test suite actually protects against regressions.*
