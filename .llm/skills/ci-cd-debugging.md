# CI/CD Debugging Guide

> **A practical guide to reproducing and debugging CI failures locally.**

## Overview

CI failures are frustrating when you cannot reproduce them locally. This guide covers systematic approaches to debug common CI failure categories in this project.

---

## General Debugging Approach

### Step 1: Read the Full Error Message

Before attempting to reproduce locally, carefully read the entire CI error output:

1. **Scroll up** past the obvious error to find root causes
2. **Check for cascading failures** that obscure the original problem
3. **Note the exact command** that failed and any environment variables
4. **Look for timestamps** to identify timeouts vs. immediate failures

### Step 2: Identify the Failure Category

| Error Pattern | Category | See Section |
|---------------|----------|-------------|
| `cargo fmt --check` fails | Formatting | [Formatting Failures](#formatting-failures) |
| Clippy warnings | Linting | [Clippy Failures](#clippy-failures) |
| Test assertions fail | Test logic | [Test Failures](#test-failures) |
| `cargo kani` verification fails | Formal verification | [Kani Failures](#kani-failures) |
| Cross-compile errors | Cross-compilation | [Cross-Compilation Failures](#cross-compilation-failures) |
| Docker/container issues | Container builds | [Container Failures](#container-failures) |
| Link errors (undefined reference) | Linker | [Linker Failures](#linker-failures) |
| Timeout | Performance/hang | [Timeout Failures](#timeout-failures) |
| `actionlint` errors | Workflow syntax | [Workflow Failures](#workflow-failures) |
| Vale/markdownlint errors | Documentation | [Documentation Failures](#documentation-failures) |

### Step 3: Reproduce Locally

**Golden rule:** Run the EXACT same command as CI, with the same environment.

```bash
# Check what CI ran in the workflow YAML
cat .github/workflows/ci-*.yml | grep "run:"

# Run locally with same flags
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

---

## Formatting Failures

### Symptoms

```
error: diff in src/lib.rs
```

### Local Reproduction

```bash
# Check what needs formatting
cargo fmt --check

# Apply fixes
cargo fmt
```

### Common Causes

- Committed without running `cargo fmt`
- Editor auto-format with different settings
- Different Rust toolchain version

---

## Clippy Failures

### Symptoms

```
error: unused variable
  --> src/lib.rs:42:9
```

### Local Reproduction

```bash
# Run exactly as CI does
cargo clippy --all-targets -- -D warnings

# Or use the project alias
cargo c
```

### Common Causes

- New code triggering lints
- Clippy version difference between local and CI
- Missing `#[allow(...)]` for intentional patterns

### Fixing

```bash
# See all available lints for a warning
cargo clippy --explain LINT_NAME

# Allow specific lint locally if justified
#[allow(clippy::lint_name)]  // Reason why this is acceptable
```

---

## Test Failures

### Symptoms

```
thread 'test_name' panicked at src/lib.rs:42:9
assertion `left == right` failed
  left: 10
 right: 20
```

### Local Reproduction

```bash
# Run the specific failing test
cargo test test_name -- --nocapture

# Or with nextest for better output
cargo nextest run test_name --no-capture
```

### Common Causes

- Platform-specific behavior (timing, random order)
- Missing test fixtures or data
- Environment variable differences

### Debugging Tips

```bash
# Run tests with verbose output
RUST_BACKTRACE=1 cargo test test_name -- --nocapture

# Run tests in single-threaded mode (eliminates race conditions)
cargo test -- --test-threads=1

# Run with specific seed for deterministic failures
PROPTEST_CASES=100 cargo test
```

---

## Kani Failures

### Symptoms

```
VERIFICATION RESULT: FAILURE
Check 1: proof_my_function.assertion.1
 - Status: FAILURE
 - Description: assertion failed
```

### Local Reproduction

```bash
# Run the specific failing proof
cargo kani --harness proof_function_name

# Run all proofs
cargo kani
```

### Common Causes

1. **Missing `#[kani::unwind(N)]`** for loops with symbolic bounds
2. **Proof assertions don't match implementation** (most common)
3. **Overflow in arithmetic** not handled
4. **Proof not registered** in tier lists

### Debugging Tips

```bash
# Get verbose output
cargo kani --harness proof_name --verbose

# Generate HTML visualization
cargo kani --harness proof_name --visualize

# Check if proof is registered
./scripts/check-kani-coverage.sh
```

### Critical Check: Assertion Correctness

When a Kani proof fails, **first verify the assertion itself is correct**:

```rust
// Read the implementation
impl Default for MyType {
    fn default() -> Self {
        MyType::VariantA  // What does it ACTUALLY return?
    }
}

// Then check if the proof's assertion matches
#[kani::proof]
fn proof_default() {
    let x = MyType::default();
    // Is this assertion correct?
    assert!(matches!(x, MyType::VariantA));  // Must match impl!
}
```

---

## Cross-Compilation Failures

### Symptoms

```
error: linker `cc` not found
error: could not find native library `ssl`
```

### Local Reproduction

```bash
# Install cross-rs
cargo install cross --git https://github.com/cross-rs/cross

# Run same target as CI
cross build --target aarch64-unknown-linux-gnu
```

### Common Causes

1. **Unstable image tags** (`:main`, `:edge`) in Cross.toml
2. **Missing linker** for target architecture
3. **Environment variable passthrough** not configured

### Debugging Tips

```bash
# Check Cross.toml configuration
cat Cross.toml

# Run with verbose Docker output
CROSS_DEBUG=1 cross build --target aarch64-unknown-linux-gnu

# Override linker via environment
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  cross build --target aarch64-unknown-linux-gnu
```

### Image Tag Stability

**Never use `:main` or `:edge` tags.** Use environment variable passthrough instead:

```toml
# Cross.toml
[build.env]
passthrough = [
    "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER",
]
```

---

## Container Failures

### Symptoms

```
docker: Error response from daemon: pull access denied
error: failed to run custom build command
```

### Local Reproduction

```bash
# Ensure Docker is running
docker ps

# Pull the image CI uses
docker pull ghcr.io/cross-rs/aarch64-unknown-linux-gnu:latest

# Run build in container
cross build --target aarch64-unknown-linux-gnu
```

### Common Causes

- Docker not running locally
- Image tag changed upstream
- Rate limiting on container registry

---

## Linker Failures

### Symptoms

```
error: linking with `cc` failed
undefined reference to `some_symbol`
```

### Local Reproduction

```bash
# Check what linker is being used
cargo build -vv 2>&1 | grep "Running"

# Ensure linker is installed
which aarch64-linux-gnu-gcc
```

### Common Causes

- Wrong linker for target
- Missing system libraries
- `.cargo/config.toml` linker settings not matching CI

### Fixing

```toml
# .cargo/config.toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

---

## Timeout Failures

### Symptoms

```
Error: The operation was canceled.
##[error]The job running on runner ... has exceeded the maximum execution time
```

### Local Reproduction

```bash
# Run with timeout to simulate CI
timeout 600 cargo kani --harness proof_name

# Profile build time
cargo build --timings
```

### Common Causes

1. **Missing `#[kani::unwind(N)]`** causing Kani to hang
2. **Excessive test count** in integration tests
3. **Network timeouts** in tests that hit external services

### Debugging Tips

```bash
# For Kani hangs, add unwind bounds
#[kani::proof]
#[kani::unwind(11)]  # Max iterations + 1
fn proof_with_loop() {
    let n: usize = kani::any();
    kani::assume(n <= 10);
    for _ in 0..n { /* ... */ }
}
```

---

## Workflow Failures

### Symptoms

```
Error: .github/workflows/ci.yml: unexpected key "on" for mapping
```

### Local Reproduction

```bash
# Run actionlint on all workflows
actionlint

# Check specific workflow
actionlint .github/workflows/ci-rust.yml
```

### Common Causes

- YAML syntax errors
- Invalid GitHub Actions expressions
- Shellcheck warnings in run blocks

### Debugging Tips

```bash
# Test workflow locally with act
brew install act
act push -j build --dryrun

# Validate expressions
echo '${{ github.event_name }}' | yq
```

---

## Documentation Failures

### Symptoms

```
error: unresolved link to `SomeType`
warning: Passive voice
```

### Local Reproduction

```bash
# Rustdoc link check (exact CI command)
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Vale prose linting
vale sync
vale docs/

# Markdownlint
npx markdownlint '**/*.md' --config .markdownlint.json
```

### Common Causes

- Broken intra-doc links
- Missing link reference definitions
- Vale style rule violations

---

## Environment Differences Checklist

When CI fails but local passes, check these differences:

| Factor | Check Command | Common Issue |
|--------|---------------|--------------|
| Rust version | `rustc --version` | CI uses toolchain from `rust-toolchain.toml` |
| Toolchain | `rustup show` | Missing components (clippy, rustfmt) |
| Environment variables | `env \| grep CARGO` | CI sets `CARGO_TERM_COLOR`, etc. |
| Working directory | `pwd` | Tests assume specific cwd |
| File permissions | `ls -la` | Scripts need execute permission |
| Git state | `git status` | Uncommitted changes affect builds |
| Cache | N/A | CI has clean cache, local has stale |

### Simulate Clean CI Environment

```bash
# Clean all build artifacts
cargo clean

# Clean cargo registry cache (careful - slow to rebuild)
rm -rf ~/.cargo/registry/cache

# Run with no local config
CARGO_HOME=$(mktemp -d) cargo build
```

---

## CI Log Interpretation

### GitHub Actions Log Navigation

1. Click on failing job name in CI summary
2. Expand the failing step
3. Click "Search logs" (magnifying glass) to find specific errors
4. Look for `##[error]` prefixed lines

### Common Log Patterns

| Pattern | Meaning |
|---------|---------|
| `##[error]` | Explicit error from action |
| `error[E...]` | Rust compiler error code |
| `FAILED` | Test or verification failure |
| `panicked at` | Rust panic occurred |
| `timeout` | Step exceeded time limit |
| `exit code 1` | Command failed |

### Getting More Debug Output

Add to workflow for verbose logging:

```yaml
env:
  CARGO_TERM_VERBOSE: true
  RUST_BACKTRACE: 1
```

Or re-run with debug logging enabled in GitHub UI.

---

## Quick Reference

### Most Common CI Fixes

| CI Failure | Quick Fix |
|------------|-----------|
| Formatting | `cargo fmt` |
| Clippy | `cargo clippy --fix --allow-dirty` |
| Doc links | Add link reference definition |
| Kani timeout | Add `#[kani::unwind(N)]` |
| Kani assertion | Verify assertion matches implementation |
| Cross-compile | Check Cross.toml for unstable tags |
| actionlint | Run `actionlint` and fix syntax |

### Commands to Run Before Every Commit

```bash
# Format + lint + test (project aliases)
cargo c && cargo t

# For documentation changes
cargo doc --no-deps

# For workflow changes
actionlint

# For markdown changes
npx markdownlint '**/*.md' --config .markdownlint.json --fix
```

---

*Systematic debugging saves hours of frustration.*
