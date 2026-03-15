<!-- CATEGORY: CI/CD & Tooling -->
<!-- WHEN: Debugging CI failures, reproducing CI issues locally -->
# CI/CD Debugging Guide

## Failure Category Quick Reference

| Error Pattern | Category | Quick Fix |
|---------------|----------|-----------|
| `cargo fmt --check` fails | Formatting | `cargo fmt` |
| Clippy warnings | Linting | `cargo clippy --all-targets --features tokio,json --fix --allow-dirty` |
| Test assertion failures | Test logic | `RUST_BACKTRACE=1 cargo test name -- --nocapture` |
| `VERIFICATION RESULT: FAILURE` | Kani | Verify assertion matches impl; add `#[kani::unwind(N)]` |
| `linker cc not found` | Cross-compilation | Check Cross.toml, avoid unstable image tags |
| `invalid linker name in argument '-fuse-ld=lld'` | Missing linker | Install lld, or use `cargo_linker.get_cargo_env()` fallback |
| Timeout / cancelled | Performance | Add `#[kani::unwind(N)]`; increase `timeout-minutes` |
| `actionlint` errors | Workflow syntax | Run `actionlint` locally |
| `unresolved link` | Documentation | Add link reference definition |
| Markdownlint errors | Markdown | `npx markdownlint --fix '**/*.md'` |

## Step 1: Read the Full Error

1. Scroll UP past the obvious error to find root causes
2. Check for cascading failures that obscure the original problem
3. Note the exact command and environment variables
4. Look for timestamps to distinguish timeouts from immediate failures

## Step 2: Reproduce Locally

Run the EXACT same command as CI:

```bash
# Check what CI ran
cat .github/workflows/ci-*.yml | grep "run:"

# Common reproductions
cargo fmt --check
cargo clippy --all-targets --features tokio,json -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo nextest run test_name --no-capture
cargo kani --harness proof_function_name
actionlint
```

## Specific Failure Types

### Kani Failures

```bash
cargo kani --harness proof_name --verbose
./scripts/check-kani-coverage.sh   # Check registration
```

Common causes:
1. Missing `#[kani::unwind(N)]` for loops (N = max_iterations + 1)
2. Proof assertions don't match implementation
3. Proof not registered in tier lists

Always verify the assertion itself is correct -- check what the code actually returns.

### Cross-Compilation Failures

```bash
cargo install cross --git https://github.com/cross-rs/cross
CROSS_DEBUG=1 cross build --target aarch64-unknown-linux-gnu
```

Never use `:main` or `:edge` image tags. Use environment variable passthrough:

```toml
# Cross.toml
[build.env]
passthrough = ["CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER"]
```

### Timeout Failures

```bash
timeout 600 cargo kani --harness proof_name
cargo build --timings   # Profile build time
```

### Test Failures

```bash
RUST_BACKTRACE=1 cargo test test_name -- --nocapture
cargo test -- --test-threads=1   # Eliminate race conditions
```

### Documentation Failures

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
vale sync && vale docs/
npx markdownlint '**/*.md' --config .markdownlint.json
```

## Environment Differences Checklist

| Factor | Check Command | Common Issue |
|--------|---------------|--------------|
| Rust version | `rustc --version` | Mismatch with `rust-toolchain.toml` |
| Toolchain | `rustup show` | Missing components |
| Env vars | `env \| grep CARGO` | CI sets `CARGO_TERM_COLOR` etc. |
| Cache | N/A | CI has clean cache, local has stale |
| Git state | `git status` | Uncommitted changes |

### Simulate Clean CI

```bash
cargo clean
CARGO_HOME=$(mktemp -d) cargo build
```

## CI Log Patterns

| Pattern | Meaning |
|---------|---------|
| `##[error]` | Explicit error from action |
| `error[E...]` | Rust compiler error code |
| `panicked at` | Rust panic occurred |
| `timeout` | Step exceeded time limit |
| `exit code 1` | Command failed |

Enable debug logging: set repo variable `ACTIONS_STEP_DEBUG=true`.

## Commands Before Every Commit

```bash
cargo c && cargo t                                        # Format + lint + test
cargo doc --no-deps                                       # Doc changes
actionlint                                                # Workflow changes
npx markdownlint '**/*.md' --config .markdownlint.json --fix  # Markdown changes
```
