# Fortress Rollback -- LLM Development Guide

> **Canonical source of truth** for project context. All other LLM instruction files point here.

## TL;DR

**Fortress Rollback** is a correctness-first fork of GGRS, written in 100% safe Rust. Peer-to-peer rollback networking for deterministic multiplayer games.

### Five Pillars

1. **Zero-panic production code** -- All errors returned as `Result`, never panic
2. **>90% test coverage** -- All code must be thoroughly tested
3. **Formal verification** -- TLA+, Z3, and Kani for critical components
4. **Enhanced usability** -- Intuitive, type-safe, hard-to-misuse APIs
5. **Code clarity** -- Readable, maintainable, well-documented

## Quick Commands

```bash
cargo fmt && cargo clippy --all-targets && cargo nextest run --no-capture  # Pre-commit
cargo c && cargo t                        # Aliases from .cargo/config.toml
typos                                     # Spell check (CI enforced)
cargo test --features z3-verification -- --nocapture  # Z3 proofs (slow)
```

Always use `--no-capture` (nextest) or `-- --nocapture` (cargo test) so test output is visible on failure.

## CLI Tools (Dev Container)

| Use | Instead of | Key flags |
|-----|------------|-----------|
| `rg` | `grep` | `-l` list files, `-C 3` context, `--type rust` |
| `fd` | `find` | `-e toml` extension, `--type d` dirs |
| `bat --paging=never` | `cat` | `-n` line numbers, `-r 10:20` range |
| `eza` | `ls` | `-la`, `--tree`, `--git` |
| `sd` | `sed` | `sd 'old' 'new' file`, `-F` literal |
| `dust` | `du` | `-d 2` depth |
| `tokei` | `wc -l` | Code stats by language |
| `hyperfine` | `time` | Statistical benchmarking |

Rules: always `bat --paging=never` (bare `bat` blocks); never redirect to `/dev/null`; use `rg --no-ignore` for gitignored files.

## Non-Negotiable Requirements

- **100% safe Rust** -- `#![forbid(unsafe_code)]`
- **ZERO-PANIC POLICY** -- Production code must NEVER panic; all errors as `Result`
- **All clippy lints pass** -- `clippy::all`, `clippy::pedantic`, `clippy::nursery`
- **No broken doc links** -- All intra-doc links must resolve
- **Public items documented** -- Rustdoc with examples
- **Overflow checks in release** -- Integer overflow caught at runtime
- **Deterministic behavior** -- Same inputs must always produce same outputs

```rust
// FORBIDDEN in production:  value.unwrap(), .expect(), array[i], panic!(), todo!()
// REQUIRED:  value.ok_or(FortressError::MissingValue)?, array.get(i).ok_or(...)?,
//            if !valid { return Err(FortressError::InvalidState); }
```

Zero-panic key principles: never swallow errors (use `?`), validate all inputs, prefer `.get()` over indexing, exhaustive `match` (no `_ =>` on enums), enums over booleans, doc examples must use `?` and `Result`. See [defensive-programming.md](skills/defensive-programming.md) for the complete guide.

## Code Design Principles

Follow SOLID, DRY, and Clean Architecture. Rely on descriptive names; comment only the "why." Prefer zero-cost abstractions (generics/traits over dynamic dispatch), value types, and minimal allocations.

Design patterns used: **Builder** (SessionBuilder), **State Machine** (protocol/connection states), **Strategy** (input prediction), **Iterator** (combinators over manual loops).

Before writing new code, search for similar existing patterns. Extract shared utilities; avoid copy-paste.

## Skills Reference

See [`.llm/skills/_index.md`](skills/_index.md) for the categorized index of deep-dive guides covering: defensive programming, testing (unit/property/mutation/fuzz/chaos), formal verification (Kani/TLA+/Z3/Loom/Miri), rollback netcode, determinism, performance, WASM, CI/CD, API design, documentation, and more.

## Project Architecture

### Repository Structure

```
src/
  lib.rs                           # Public API entry point
  error.rs                         # FortressError types
  frame_info.rs / hash.rs / rle.rs / rng.rs  # Core utilities
  time_sync.rs / sync.rs / checksum.rs / telemetry.rs
  input_queue/
    mod.rs                         # Input buffering
    prediction.rs                  # Input prediction strategies
  sync_layer/
    mod.rs                         # Core synchronization (SyncLayer)
    game_state_cell.rs             # Thread-safe game state
    saved_states.rs                # Circular buffer for rollback
  network/
    compression.rs / messages.rs / network_stats.rs
    chaos_socket.rs / udp_socket.rs / codec.rs / tokio_socket.rs
    protocol/
      mod.rs / event.rs / input_bytes.rs / state.rs
  sessions/
    builder.rs                     # SessionBuilder pattern
    p2p_session.rs                 # P2P gameplay
    p2p_spectator_session.rs       # Spectator mode
    sync_test_session.rs           # Determinism testing
    config.rs / player_registry.rs / sync_health.rs
```

### Session Types

- **P2PSession** -- Standard peer-to-peer gameplay
- **SpectatorSession** -- Observe but don't participate
- **SyncTestSession** -- Verify determinism by running simulation twice

### Critical Determinism Rules

1. **No `HashMap` iteration** -- Use `BTreeMap` or sort before iterating
2. **Control floating-point** -- Use `libm` feature or fixed-point math
3. **Seeded RNG only** -- `rand_pcg` or `rand_chacha` with shared seed
4. **Frame counters, not time** -- Never use `Instant::now()` in simulation
5. **Sort ECS queries** -- Bevy queries are non-deterministic; sort by stable ID
6. **Pin toolchain** -- Use `rust-toolchain.toml` for reproducible builds
7. **Audit features** -- Check for `ahash`, `const-random` leaks with `cargo tree -f "{p} {f}"`

## Kani Essentials

**The #1 cause of Kani CI failures:** All loops with symbolic bounds require `#[kani::unwind(N)]` where N = max_iterations + 1. CI uses `--default-unwind 8` via `--quick` mode.

**The #2 cause:** Proofs that assert the wrong thing (e.g., wrong enum variant).

**The #3 cause:** `format!()` inside macros (e.g., `report_violation!`) creating explosive CBMC state space. The `report_violation!` macro is already a no-op under `cfg(kani)` -- no additional gating needed when calling it. See [kani.md](skills/kani.md#common-timeout-causes) for details.

```bash
cargo kani --harness proof_function_name    # Run specific proof
./scripts/verify-kani.sh --tier 1 --quick   # Fast proofs (~15 min)
./scripts/verify-kani.sh --list             # List all proofs and tiers
./scripts/check-kani-coverage.sh            # Validate proof registration
```

New proofs must be registered in `scripts/verify-kani.sh`:

- **Tier 1:** Fast (<30s) -- simple property checks
- **Tier 2:** Medium (30s-2min) -- moderate complexity
- **Tier 3:** Slow (>2min) -- complex state verification

Pre-commit validates registration only, NOT that proofs pass. Run affected proofs locally before committing.

## Safety CI Checks

| Check | Purpose |
|-------|---------|
| **Cargo Careful** | Extra runtime safety checks (nightly) |
| **Overflow Checks** | Release builds with `-C overflow-checks=on` |
| **Debug Assertions** | Release builds with `-C debug-assertions=on` |
| **Panic Patterns** | Counts `unwrap`, `expect`, `panic!`, `todo!` |
| **Strict Clippy** | Nursery lints enabled |
| **Documentation** | Doc build with `-D warnings` |

Also: `ci-rust.yml` (Miri), `ci-security.yml` (cargo-geiger, cargo-deny).

**CI fails on:** unformatted code, clippy warnings, broken doc links, markdown lint errors, workflow syntax errors, unregistered Kani proofs.

## Development Workflow

### Before Writing Code

1. Read relevant source files and tests for context
2. Check existing patterns for consistency
3. Consider impact on other components
4. Plan tests first -- define expected behavior

### When Fixing Bugs

1. Write a failing test that reproduces the bug
2. Root-cause analysis -- understand *why*, not just *what*
3. Fix at the right level (production bug vs test bug)
4. Add regression tests; check for similar issues elsewhere

## Test Writing

Use **Arrange-Act-Assert** pattern. Name tests: `what_condition_expected_behavior` (e.g., `parse_empty_input_returns_none`).

```rust
#[track_caller]  // Shows actual test location on failure
fn check_parse(input: &str, expected: Option<Ast>) {
    let actual = parse(input).ok();
    assert_eq!(actual, expected, "parse({:?})", input);
}
```

Consolidate integration tests into a single crate (`tests/it/main.rs`). Anti-patterns: `assert!(result.is_ok())` (use `assert_eq!`), sleep-based synchronization, testing implementation details.

## Changelog Policy

**Quick decision:** "Does this affect `pub` items or user-observable behavior?"

- **YES** -- Add entry (use **Breaking:** prefix if API signature changed)
- **NO** (pub(crate), private, tests, CI) -- Skip

**Include:** new features/APIs, user-visible bug fixes, breaking changes (with migration guidance), performance improvements, dependency updates affecting compatibility.

**Exclude:** internal refactoring, test improvements, doc-only changes, CI/tooling, lint fixes.

## Mandatory Linting

- **After Rust changes:** `cargo fmt && cargo clippy --all-targets` (or `cargo c`)
- **After workflow changes:** `actionlint` (no exceptions)
- **After doc changes:** `cargo doc --no-deps`
- **After markdown changes:** `npx markdownlint 'file.md' --config .markdownlint.json --fix`
- **After `.llm/` changes:** All `.md` files under `.llm/` must be **300 lines or fewer** (enforced by pre-commit hook `llm-line-limit`)
- **Link validation:** `./scripts/check-links.sh`
- **Spell check:** `typos`
- **Vale (advisory):** `vale docs/` -- checks prose quality, non-blocking in CI
- **Full pre-commit:** `cargo fmt && cargo clippy --all-targets && cargo nextest run --no-capture`

## Breaking Changes Checklist

- [ ] `CHANGELOG.md` updated with **Breaking:** prefix and migration guidance
- [ ] `docs/migration.md` updated with before/after examples
- [ ] `README.md` and `docs/user-guide.md` updated if affected
- [ ] All `examples/*.rs` compile: `cargo build --examples`
- [ ] Rustdoc examples compile: `cargo test --doc`
- [ ] Search for old API usage: `rg 'old_name' --type rust --type md`

## Documentation Sync

When changing public APIs, update: rustdoc comments (source of truth), README.md, docs/user-guide.md, examples/, CHANGELOG.md. Search with `rg 'function_name|StructName' --type rust --type md`.

## Quality Checklist

- [ ] `cargo fmt` run
- [ ] `cargo clippy --all-targets` passes
- [ ] All tests pass (`cargo nextest run`)
- [ ] Tests for new functionality included
- [ ] Rustdoc comments with examples
- [ ] 100% safe Rust (no unsafe)
- [ ] All error cases handled
- [ ] No duplicate methods (e.g., don't add method duplicating `Display` impl)
- [ ] Feature-dependent APIs documented in rustdoc
- [ ] Changelog reviewed for pub/user-observable changes

## For Agents

When spawning sub-agents or using Task tools: the sub-agent MUST run `cargo fmt` and verify `cargo clippy --all-targets` passes on any modified files. If the sub-agent cannot run these, the parent agent must run them after receiving changes.

---

*License: MIT OR Apache-2.0*
