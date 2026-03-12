<!-- CATEGORY: Testing -->
<!-- WHEN: Running mutation tests, verifying test quality, cargo-mutants -->

# Mutation Testing

## Quick Start

```bash
cargo mutants                           # Run on entire project
cargo mutants -f src/some_module.rs     # Target specific file
cargo mutants --list                    # List mutations without running
cargo mutants -- --all-targets          # Skip doctests (faster)
```

## Mutation Outcomes

| Outcome | Meaning | Action |
|---------|---------|--------|
| **Caught** | Test failed = mutant killed | Good coverage |
| **Missed** | Tests still pass = mutant survived | Improve tests |
| **Timeout** | Test hung (usually infinite loop) | Usually acceptable |
| **Unviable** | Doesn't compile | Inconclusive |

## Common Mutations Applied

| Category | Original | Mutated |
|----------|----------|---------|
| Comparison | `==` | `!=` |
| Comparison | `<` | `==`, `>`, `<=` |
| Logical | `&&` | `\|\|` |
| Arithmetic | `+` | `-`, `*` |
| Returns | `return x` | `return Default::default()` |
| Boolean | `true` | `false` |
| Statements | statement | (deleted) |

## Configuration (`.cargo/mutants.toml`)

```toml
test_tool = "nextest"
additional_cargo_test_args = ["--all-targets"]

exclude_globs = ["examples/**/*.rs", "benches/**/*.rs", "tests/**/common/**/*.rs"]
exclude_re = ["impl Debug", "impl Display"]

timeout_multiplier = 10.0
profile = "mutants"
```

Add to `Cargo.toml`:

```toml
[profile.mutants]
inherits = "test"
debug = "none"
```

## Command Reference

```bash
# Filter by function name
cargo mutants -F 'encode|decode'

# Exclude functions
cargo mutants -E 'impl Debug|impl Display'

# Skip baseline (when tests already passed)
cargo mutants --baseline=skip --timeout 300

# Incremental PR testing
git diff origin/main.. > changes.diff
cargo mutants --in-diff changes.diff

# Sharding for CI
cargo mutants --shard 0/8 --baseline=skip --timeout 300

# Parallel jobs (watch memory)
cargo mutants -j2
```

## Source Code Annotations

```rust
#[mutants::skip]  // Skip from mutation testing
fn known_timeout_function() -> bool { loop_until_condition() }
```

Use `#[mutants::skip]` for: infinite loops when mutated, FFI/unsafe code.

## Analyzing Results

```bash
cat mutants.out/missed.txt              # Review missed mutations
cat mutants.out/diff/src_lib_rs_123.diff  # View specific mutation
```

| Category | Example | Resolution |
|----------|---------|------------|
| Missing assertion | Return value not checked | Add `assert_eq!` |
| Boundary condition | `<` vs `<=` not tested | Add boundary tests |
| Equivalent mutant | `x * 1` -> `x / 1` | Usually acceptable |
| Dead code | Unreachable branch mutated | Remove dead code |

## Writing Tests That Catch Mutants

```rust
// Weak: Only checks type/existence
assert!(result.is_ok());
assert!(value > 0);

// Strong: Checks exact values
assert_eq!(result, Ok(expected_value));
assert_eq!(value, 42);
```

Key principles:
- Assert specific values, not just types
- Test boundary conditions explicitly (`0`, `max-1`, `max`, `max+1`)
- Test both branches of conditionals
- Test exact operators (`assert_eq!(add(5, 3), 8)`)
- Verify state changes (before/after assertions)

## CI Integration

### GitHub Actions (Incremental PR)

```yaml
jobs:
  incremental-mutants:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - uses: taiki-e/install-action@v2
        with: { tool: cargo-mutants,cargo-nextest }
      - run: git diff origin/${{ github.base_ref }}.. > changes.diff
      - run: cargo mutants --no-shuffle -vV --in-diff changes.diff --in-place
      - uses: actions/upload-artifact@v4
        if: always()
        with: { name: mutants-pr, path: mutants.out/ }
```

### Sharded Full Testing

```yaml
strategy:
  fail-fast: false
  matrix:
    shard: [0, 1, 2, 3, 4, 5, 6, 7]
steps:
  - run: cargo mutants --no-shuffle -vV --shard ${{ matrix.shard }}/8 --baseline=skip --timeout 300 --in-place
```

## Performance Optimization

| Optimization | Impact |
|-------------|--------|
| Target specific files (`-f src/module.rs`) | 10-100x |
| Skip doctests (`-- --all-targets`) | 2-5x |
| Use nextest (`test_tool = "nextest"`) | 2-12x |
| Disable debug symbols (`debug = "none"`) | 20-30% |
| Faster linker (mold/wild) | 20-50% |

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Too many timeouts | Add `#[mutants::skip]`, increase `timeout_multiplier` |
| All tests pass but shouldn't | Strengthen assertions to check exact values |
| Takes too long | Target specific files, skip doctests, shard |
| Out of memory | Reduce parallelism (`-j1`), use `--in-place` |
