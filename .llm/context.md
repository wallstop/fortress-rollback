# Fortress Rollback — LLM Development Guide

> **This is the canonical source of truth** for project context. All other LLM instruction files (CLAUDE.md, AGENTS.md, copilot-instructions.md) point here.

## TL;DR — What You Need to Know

**Fortress Rollback** is a correctness-first fork of GGRS (Good Game Rollback System), written in 100% safe Rust. It provides peer-to-peer rollback networking for deterministic multiplayer games.

### The Five Pillars

1. **Zero-panic production code** — All errors returned as `Result`, never panic
2. **>90% test coverage** — All code must be thoroughly tested
3. **Formal verification** — TLA+, Z3, and Kani for critical components
4. **Enhanced usability** — Intuitive, type-safe, hard-to-misuse APIs
5. **Code clarity** — Readable, maintainable, well-documented

### Quick Commands

```bash
# Run after every change
cargo fmt && cargo clippy --all-targets && cargo nextest run

# Aliases from .cargo/config.toml
cargo c && cargo t

# Additional checks
typos                                    # Spell check (CI enforced)
cargo test --features z3-verification   # Z3 proofs (slow)
```

---

## CLI Tools (Dev Container)

Use modern tools instead of traditional counterparts. Shell aliases are pre-configured.

| Use | Instead of | Key flags |
|-----|------------|-----------|
| `rg` | `grep` | `-l` list files, `-C 3` context, `--type rust` |
| `fd` | `find` | `-e toml` extension, `--type d` dirs, `-x cmd {}` exec |
| `bat --paging=never` | `cat` | `-n` line numbers, `-r 10:20` range, `-l rust` language |
| `eza` | `ls` | `-la`, `--tree`, `--git` |
| `sd` | `sed` | `sd 'old' 'new' file`, `-F` literal |
| `dust` | `du` | `-d 2` depth, `-n 20` top entries |
| `procs` | `ps` | `--tree`, `procs cargo` filter |
| `tokei` | `wc -l` | Code stats by language |
| `hyperfine` | `time` | Statistical benchmarking |

**Critical rules:**

- Always use `bat --paging=never` (bare `bat` blocks)
- Never redirect to `/dev/null` — hides errors
- Use `rg` with `--no-ignore` to include gitignored files

---

## How to Approach Development

### Before Writing Any Code

1. **Understand the context** — Read relevant source files and tests
2. **Check for similar patterns** — See how existing code handles similar cases
3. **Consider the impact** — Will this change affect other components?
4. **Plan tests first** — What tests will verify correctness?

### When Implementing Features

1. **Write tests first** (TDD) — Define expected behavior before implementing
2. **Keep functions focused** — Single responsibility, clear intent
3. **Handle all errors** — No panics, use `Result<T, FortressError>`
4. **Document as you go** — Rustdoc with examples for all public items
5. **Consider edge cases** — Zero values, max values, empty collections
6. **Update changelog** — Only for user-facing changes (see Changelog Policy below)

### When Fixing Bugs

1. **Reproduce first** — Write a failing test that demonstrates the bug
2. **Root cause analysis** — Understand *why* it fails, not just *what* fails
3. **Fix at the right level** — Production bug vs test bug (see below)
4. **Add regression tests** — Ensure the bug can't return
5. **Check for similar issues** — Are there related bugs elsewhere?

---

## Code Quality Standards

### Non-Negotiable Requirements

- **100% safe Rust** — `#![forbid(unsafe_code)]`
- **ZERO-PANIC POLICY** — Production code must NEVER panic; all errors as `Result`
- **All clippy lints pass** — `clippy::all`, `clippy::pedantic`, `clippy::nursery`
- **No broken doc links** — All intra-doc links must resolve
- **Public items documented** — Rustdoc with examples
- **Overflow checks in release** — Integer overflow is caught at runtime
- **Deterministic behavior** — Same inputs must always produce same outputs

### Code Design Principles

These principles apply to **all code** — production, tests, CI/CD, documentation, and examples.

#### Minimal Comments

- **Rely on descriptive names** — Function, variable, and type names should be self-documenting
- **Comment only the "why"** — Explain non-obvious design decisions, not what the code does
- **Avoid redundant comments** — If the code is clear, don't add noise
- **Rustdoc is different** — Public API documentation is mandatory and valuable

```rust
// ❌ Avoid: Redundant comment
// Increment the frame counter
frame_counter += 1;

// ✅ Prefer: Self-documenting code, no comment needed
frame_counter += 1;

// ✅ Good: Explains non-obvious "why"
// Skip checksum validation for spectators to reduce bandwidth
if player.is_spectator() { return Ok(()); }
```

#### SOLID Principles

- **Single Responsibility** — Each module, struct, and function does one thing well
- **Open/Closed** — Extend behavior through traits and generics, not modification
- **Liskov Substitution** — Trait implementations must honor the trait's contract
- **Interface Segregation** — Prefer small, focused traits over large monolithic ones
- **Dependency Inversion** — Depend on abstractions (traits), not concrete types

#### DRY (Don't Repeat Yourself)

- **Extract common patterns** — If code appears twice, consider abstracting it
- **Prefer composition** — Build complex behavior from simple, reusable pieces
- **Centralize constants** — Magic numbers and strings belong in named constants
- **Share test utilities** — Common test setup belongs in shared modules

```rust
// ❌ Avoid: Duplicated validation logic
fn process_input(input: Input) -> Result<(), Error> {
    if input.frame < 0 { return Err(Error::InvalidFrame); }
    // ... process
}
fn validate_input(input: Input) -> Result<(), Error> {
    if input.frame < 0 { return Err(Error::InvalidFrame); }
    // ... validate
}

// ✅ Prefer: Single source of truth
impl Input {
    fn validate(&self) -> Result<(), Error> {
        if self.frame < 0 { return Err(Error::InvalidFrame); }
        Ok(())
    }
}
```

#### Clean Architecture

- **Separate concerns** — Keep business logic independent of I/O and frameworks
- **Layer dependencies inward** — Core logic shouldn't know about network or storage details
- **Define clear boundaries** — Use traits to define interfaces between layers

#### Design Patterns

Use established patterns where appropriate:

- **Builder** — For complex object construction (see `SessionBuilder`)
- **State Machine** — For protocol and connection state management
- **Strategy** — For swappable algorithms (e.g., input prediction)
- **Factory** — For creating related objects with consistent configuration
- **Iterator** — Leverage Rust's iterator combinators over manual loops

#### Lightweight Abstractions

When creating abstractions for common patterns:

- **Prefer value types** — Use `Copy` types and stack allocation when possible
- **Minimize allocations** — Avoid `Box`, `Vec`, `String` in hot paths unless necessary
- **Use zero-cost abstractions** — Generics and traits over dynamic dispatch
- **Function-based over object-based** — Simple functions often beat complex types

```rust
// ❌ Avoid: Unnecessary allocation for simple abstraction
struct FrameValidator {
    valid_range: Box<dyn Fn(Frame) -> bool>,
}

// ✅ Prefer: Zero-cost, value-typed abstraction
#[derive(Clone, Copy)]
struct FrameRange { min: Frame, max: Frame }

impl FrameRange {
    const fn contains(self, frame: Frame) -> bool {
        frame >= self.min && frame <= self.max
    }
}
```

#### Code Consolidation

- **Look for patterns first** — Before writing new code, search for similar existing code
- **Extract shared utilities** — Test helpers, validation logic, formatting
- **Avoid copy-paste** — If tempted to copy code, create a shared abstraction instead
- **Refactor proactively** — When adding features, improve structure of touched code

> **See also:** Performance and code quality guides in `.llm/skills/`:
>
> - [high-performance-rust.md](skills/high-performance-rust.md) — Performance optimization patterns and build configuration
> - [rust-refactoring-guide.md](skills/rust-refactoring-guide.md) — Safe code transformation patterns with verification
> - [rust-idioms-patterns.md](skills/rust-idioms-patterns.md) — Idiomatic Rust patterns and best practices
> - [clippy-configuration.md](skills/clippy-configuration.md) — Clippy lint configuration and enforcement
> - [zero-copy-memory-patterns.md](skills/zero-copy-memory-patterns.md) — Zero-copy and memory efficiency patterns
> - [async-rust-best-practices.md](skills/async-rust-best-practices.md) — Async Rust patterns for concurrent code
> - [rust-compile-time-optimization.md](skills/rust-compile-time-optimization.md) — Build and compile time optimization

### Safety-Focused CI Checks (ci-safety.yml)

The project runs comprehensive safety checks beyond standard linting:

| Check | Purpose |
|-------|---------|
| **Cargo Careful** | Extra runtime safety checks using nightly |
| **Overflow Checks** | Release builds with `-C overflow-checks=on` |
| **Debug Assertions** | Release builds with `-C debug-assertions=on` |
| **Panic Patterns** | Counts `unwrap`, `expect`, `panic!`, `todo!` usage |
| **Strict Clippy** | Nursery lints enabled for experimental checks |
| **Documentation** | Doc build with `-D warnings` (warnings as errors) |

See also: `ci-rust.yml` (Miri UB detection), `ci-security.yml` (cargo-geiger, cargo-deny)

### Documentation Template

```rust
/// Brief one-line description ending with a period.
///
/// Longer explanation if needed, explaining the "why" not just "what".
///
/// # Arguments
/// * `param1` - What this parameter represents
///
/// # Returns
/// What the function returns and when.
///
/// # Errors
/// * [`FortressError::Variant`] - When this specific error occurs
///
/// # Examples
/// ```
/// # use fortress_rollback::*;
/// let result = function(arg)?;
/// assert_eq!(result, expected);
/// # Ok::<(), FortressError>(())
/// ```
pub fn function(param1: Type) -> Result<ReturnType, FortressError> {
    // Implementation
}
```

### Test Writing Best Practices

> **See also:** Complete testing guides in `.llm/skills/`:
>
> - [rust-testing-guide.md](skills/rust-testing-guide.md) — Comprehensive testing best practices and patterns
> - [testing-tools-reference.md](skills/testing-tools-reference.md) — Tool ecosystem reference (nextest, proptest, mockall, etc.)
> - [property-testing.md](skills/property-testing.md) — Property-based testing to find edge cases automatically
> - [mutation-testing.md](skills/mutation-testing.md) — Mutation testing for test quality verification
> - [cross-platform-ci-cd.md](skills/cross-platform-ci-cd.md) — CI/CD workflows for multi-platform builds

#### Test Organization

| Location | Use Case |
|----------|----------|
| `src/*.rs` with `#[cfg(test)] mod tests` | Unit tests (access private functions) |
| `tests/it/*.rs` (single crate) | Integration tests (public API only) |
| `tests/common/mod.rs` | Shared test utilities |

**Critical:** Integration tests in `tests/` should be consolidated into a single crate (`tests/it/main.rs`) to avoid slow compilation.

#### Test Structure (Arrange-Act-Assert)

```rust
#[test]
fn descriptive_name_explaining_what_is_tested() {
    // Arrange: Set up test conditions
    let mut session = create_test_session();
    let input = prepare_test_input();

    // Act: Execute the behavior being tested
    let result = session.some_operation(input);

    // Assert: Verify expected outcomes
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected_value);
}
```

#### The `check` Helper Pattern (Recommended)

Decouple tests from API changes with helper functions:

```rust
#[track_caller]  // Shows actual test location on failure
fn check_parse(input: &str, expected: Option<Ast>) {
    let actual = parse(input).ok();
    assert_eq!(actual, expected, "parse({:?})", input);
}

#[test]
fn parse_empty_returns_none() {
    check_parse("", None);
}

#[test]
fn parse_valid_expression() {
    check_parse("1 + 2", Some(expected_ast()));
}
```

#### Test Naming Convention

Names should describe: **what** + **condition** + **expected behavior**

```rust
// ❌ BAD
fn test1() { }
fn it_works() { }

// ✅ GOOD
fn parse_empty_input_returns_none() { }
fn session_with_zero_players_returns_error() { }
fn rollback_preserves_confirmed_frames() { }
```

---

## Root Cause Analysis — When Tests Fail

**CRITICAL: The goal is NOT to make the test pass — it's to understand and fix the underlying issue.**

### Investigation Steps

1. **Reproduce** — Is it consistent or flaky? Under what conditions?
2. **Understand** — What property is the test verifying? Why should it hold?
3. **Trace** — Add logging, use debugger, examine state at failure
4. **Hypothesize** — What could cause this specific failure mode?
5. **Verify** — Confirm understanding before implementing any fix
6. **Scope** — Are there similar issues elsewhere?

### Production Bug vs Test Bug

**It's a production bug if:**

- Test expectations align with documented behavior
- Multiple tests depend on the same behavior
- The test logic is simple and clearly correct

**It's a test bug if:**

- Test makes assumptions not guaranteed by the API
- Test has inherent race conditions or timing issues
- Test expectations contradict documentation

### Strictly Forbidden "Fixes"

- ❌ Commenting out or weakening assertions
- ❌ Adding `Thread::sleep()` to "fix" timing
- ❌ Catching and swallowing errors
- ❌ `#[ignore]` without a documented fix plan
- ❌ Relaxing tolerances without understanding why
- ❌ Changing expected values to match actual without analysis

---

## Formal Verification Philosophy

> **See also:** Complete guides in `.llm/skills/`:
>
> - [tla-plus-modeling.md](skills/tla-plus-modeling.md) — TLA+ specification patterns
> - [kani-verification.md](skills/kani-verification.md) — Kani proof harnesses
> - [z3-verification.md](skills/z3-verification.md) — Z3 SMT solver proofs
> - [loom-testing.md](skills/loom-testing.md) — Loom concurrency testing
> - [miri-verification.md](skills/miri-verification.md) — Miri UB detection
> - [mutation-testing.md](skills/mutation-testing.md) — Test quality verification
> - [property-testing.md](skills/property-testing.md) — Property-based testing

**Core principles:**

- **Specs model production** — TLA+/Kani/Z3 specs represent real code behavior
- **When verification fails, assume production bug first** — Investigate before relaxing specs
- **Never "fix" specs just to make them pass** — That defeats the purpose
- **Invariants represent real safety properties** — Only relax with strong justification

### Quick Commands

```bash
# Kani proofs
cargo kani
cargo kani --harness verify_specific_function

# TLA+ verification
./scripts/verify-tla.sh

# Z3 proofs
cargo test --features z3-verification

# Loom concurrency tests
cd loom-tests && RUSTFLAGS="--cfg loom" cargo test --release

# Miri UB detection
cargo +nightly miri test

# Mutation testing
cargo mutants -f src/module.rs --timeout 30 --jobs 4
```

### After Finding a Bug via Verification

1. **Direct reproduction** — Cover the exact discovered scenario
2. **Edge cases** — Zero, max, boundary conditions
3. **Chained operations** — Sequential calls that might compound
4. **Lifecycle tests** — Create-use-modify-destroy cycles
5. **Negative tests** — Ensure violations are detected

---

## Rollback Netcode Development

> **See also:** The rollback netcode guides in `.llm/skills/`:
>
> - [rollback-netcode-conversion.md](skills/rollback-netcode-conversion.md) — Complete guide to converting games to rollback netcode
> - [rollback-engine-integration.md](skills/rollback-engine-integration.md) — Patterns for Bevy and custom engine integration
> - [determinism-guide.md](skills/determinism-guide.md) — Achieving and verifying determinism in Rust games (includes reproducible builds, WASM, float handling, crate recommendations)
> - [deterministic-simulation-testing.md](skills/deterministic-simulation-testing.md) — DST frameworks (madsim, turmoil), failure injection, controlled concurrency
> - [cross-platform-games.md](skills/cross-platform-games.md) — Cross-platform game development (WASM, mobile, desktop)
> - [cross-platform-rust.md](skills/cross-platform-rust.md) — Multi-platform project architecture and tooling
> - [wasm-rust-guide.md](skills/wasm-rust-guide.md) — Rust to WebAssembly compilation and toolchain
> - [no-std-guide.md](skills/no-std-guide.md) — `no_std` patterns for WASM and embedded
> - [wasm-threading.md](skills/wasm-threading.md) — Threading and concurrency in WebAssembly
> - [wasm-portability.md](skills/wasm-portability.md) — WASM determinism and sandboxing

### Essential Rollback Concepts

| Concept | Description |
|---------|-------------|
| **Determinism** | Same inputs MUST produce identical outputs on all machines |
| **State Serialization** | Must save/restore complete game state efficiently |
| **Input Prediction** | Guess remote inputs and continue simulation without waiting |
| **Rollback** | Restore saved state when prediction was wrong, resimulate |
| **Desync Detection** | Compare checksums between peers to catch divergence |
| **DST** | Deterministic Simulation Testing — control time, I/O, and concurrency for reproducible tests |

### Critical Determinism Rules

1. **No `HashMap` iteration** — Use `BTreeMap` or sort before iterating
2. **Control floating-point** — Use `libm` feature or fixed-point math
3. **Seeded RNG only** — `rand_pcg` or `rand_chacha` with shared seed
4. **Frame counters, not time** — Never use `Instant::now()` in simulation
5. **Sort ECS queries** — Bevy queries are non-deterministic; sort by stable ID
6. **Pin toolchain** — Use `rust-toolchain.toml` for reproducible builds
7. **Audit features** — Check for `ahash`, `const-random` feature leaks with `cargo tree -f "{p} {f}"`

---

## Defensive Programming Patterns

> **See [defensive-programming.md](skills/defensive-programming.md)** for the complete zero-panic guide with all patterns.

### Zero-Panic Policy (CRITICAL)

**Production code must NEVER panic.** This is non-negotiable.

```rust
// ❌ FORBIDDEN in production code
value.unwrap()                    // Panics on None
value.expect("msg")               // Panics with message
array[index]                      // Panics on out-of-bounds
panic!("something went wrong")   // Explicit panic
todo!()                           // Panics as placeholder

// ✅ REQUIRED - Return Results, let caller decide
value.ok_or(FortressError::MissingValue)?          // Convert Option to Result
array.get(index).ok_or(FortressError::OutOfBounds)?  // Safe indexing
if !valid { return Err(FortressError::InvalidState); }  // Explicit error
```

### Key Principles

- **Never swallow errors** — Use `?` to propagate, never `let _ = result`
- **Validate all inputs** — Don't assume internal state is valid
- **Prefer pattern matching** — Use `match` and `.get()` over indexing
- **Exhaustive matches** — Never use `_ =>` wildcards on enums
- **Enums over booleans** — `Compression::Enabled` not `true`
- **Type safety** — Make invalid states unrepresentable

**See also:** [type-driven-design.md](skills/type-driven-design.md), [rust-pitfalls.md](skills/rust-pitfalls.md)

---

## Project Architecture

### Repository Structure

```
src/
├── lib.rs                           # Public API entry point
├── error.rs                         # FortressError types
├── frame_info.rs                    # Frame metadata
├── hash.rs                          # Deterministic FNV-1a hashing
├── rle.rs                           # Run-length encoding
├── rng.rs                           # Deterministic PCG32 RNG
├── time_sync.rs                     # Time synchronization
├── sync.rs                          # Synchronization primitives (loom-compatible)
├── checksum.rs                      # State checksum utilities
├── telemetry.rs                     # Structured telemetry pipeline
│
├── input_queue/
│   ├── mod.rs                       # Input buffering
│   └── prediction.rs                # Input prediction strategies
│
├── sync_layer/
│   ├── mod.rs                       # Core synchronization (SyncLayer)
│   ├── game_state_cell.rs           # Thread-safe game state
│   └── saved_states.rs              # Circular buffer for rollback
│
├── network/
│   ├── compression.rs               # Message compression
│   ├── messages.rs                  # Protocol messages
│   ├── network_stats.rs             # Statistics tracking
│   ├── chaos_socket.rs              # Testing socket with chaos
│   ├── udp_socket.rs                # UDP abstraction
│   ├── codec.rs                     # Binary codec for serialization
│   ├── tokio_socket.rs              # Tokio async adapter
│   └── protocol/
│       ├── mod.rs                   # UDP protocol implementation
│       ├── event.rs                 # Protocol events
│       ├── input_bytes.rs           # Byte-encoded input data
│       └── state.rs                 # Protocol state machine
│
└── sessions/
    ├── builder.rs                   # SessionBuilder pattern
    ├── p2p_session.rs               # P2P gameplay
    ├── p2p_spectator_session.rs     # Spectator mode
    ├── sync_test_session.rs         # Determinism testing
    ├── config.rs                    # Session configuration presets
    ├── player_registry.rs           # Player tracking and connection states
    └── sync_health.rs               # Synchronization health status
```

### Key Concepts

| Concept | Description |
|---------|-------------|
| **Frame** | Discrete time step in game simulation (typically 60 FPS) |
| **Rollback** | Restoring previous state when predictions are wrong |
| **Input Delay** | Buffer frames to reduce network jitter (typically 2-3 frames) |
| **Prediction** | Continue simulation before remote inputs arrive |
| **Prediction Window** | Maximum frames ahead we'll predict (typically 6-8) |
| **Desync** | State divergence between peers (detected via checksums) |
| **Determinism** | Same inputs must always produce same outputs |
| **Checksum** | Hash of game state for desync detection |
| **Confirmed Frame** | Oldest frame where all inputs are known |
| **Resimulation** | Re-running frames with corrected inputs after rollback |

### Session Types

- **P2PSession** — Standard peer-to-peer gameplay
- **SpectatorSession** — Observe but don't participate
- **SyncTestSession** — Verify determinism by running simulation twice

---

## Common Code Patterns

### Session Builder

```rust
let session = SessionBuilder::<MyConfig>::new()
    .with_num_players(2)
    .with_input_delay(2)
    .with_max_prediction(8)
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(addr), PlayerHandle::new(1))?
    .start_p2p_session(socket)?;
```

### Request Handling Loop

```rust
for request in session.advance_frame()? {
    match request {
        FortressRequest::SaveGameState { frame, cell } => {
            cell.save(frame, Some(game_state.clone()), None);
        }
        FortressRequest::LoadGameState { cell, .. } => {
            game_state = cell.load().expect("State must exist");
        }
        FortressRequest::AdvanceFrame { inputs } => {
            game_state.update(&inputs);
        }
    }
}
```

### Player Types

```rust
PlayerType::Local              // Local player on this device
PlayerType::Remote(addr)       // Remote player (SocketAddr)
PlayerType::Spectator(addr)    // Observer (no input)
```

---

## Development Policies

### Breaking Changes Are Acceptable

- **API compatibility is NOT required** — This is a correctness-first fork
- **Safety and correctness trump compatibility** — Make breaking changes if they improve quality
- **Document all breaking changes** — Update `CHANGELOG.md` and `docs/migration.md`

### Test Coverage Requirements

> **See also:** [rust-testing-guide.md](skills/rust-testing-guide.md) for comprehensive testing patterns.

- All new features must include tests
- Aim for >90% code coverage
- Include positive and negative test cases
- Test edge cases and error conditions
- Use integration tests for cross-component behavior
- Use `cargo nextest run` for faster test execution
- Run mutation testing (`cargo mutants`) to verify test quality

**Testing anti-patterns to avoid:**

- `assert!(result.is_ok())` — Use `assert_eq!` with specific values
- Multiple assertions testing different behaviors in one test
- Sleep-based synchronization — Use proper channels/signals
- Testing implementation details instead of behavior
- Ignoring tests without documented fix plan

### Changelog Policy

The changelog (`CHANGELOG.md`) is for **users of the library**, not developers.

**Include in changelog:**

- New features, APIs, or configuration options
- Bug fixes that affect user-visible behavior
- Breaking changes (with migration guidance)
- Performance improvements users would notice
- Dependency updates that affect compatibility

**Do NOT include in changelog:**

- Internal refactoring (module splits, code reorganization)
- Test improvements or new tests
- Documentation-only changes
- CI/CD or tooling changes
- Code style or lint fixes

**Exception:** If a release contains *only* internal work (no user-facing changes), add a single summary line like:
> "Internal: Improved test coverage and code organization"

This keeps the changelog focused and useful for library consumers.

---

## Resources

| Resource | Link |
|----------|------|
| Original GGPO | <https://www.ggpo.net/> |
| GGPO Discord | <https://discord.com/invite/8FKKhCRCCE> |
| Bevy GGRS Plugin | <https://github.com/gschup/bevy_ggrs> |
| TLA+ Resources | <https://lamport.azurewebsites.net/tla/tla.html> |
| Z3 Prover | <https://github.com/Z3Prover/z3> |

---

## Quality Checklist

Before submitting code:

- [ ] `cargo fmt` run (no formatting changes)
- [ ] `cargo clippy --all-targets` passes with no warnings
- [ ] All tests pass (`cargo nextest run` or `cargo test`)
- [ ] Includes tests for new functionality
- [ ] Rustdoc comments with examples
- [ ] 100% safe Rust (no unsafe)
- [ ] Handles all error cases
- [ ] Changelog updated if user-facing

---

## Mandatory Pre-Commit Checks

**ALWAYS run these commands before committing ANY changes:**

```bash
# Format all code (REQUIRED - CI will fail without this)
cargo fmt

# Check for lint warnings (REQUIRED - CI will fail without this)
cargo clippy --all-targets

# Run tests
cargo nextest run  # or: cargo test
```

**Or use the single combined command:**

```bash
cargo fmt && cargo clippy --all-targets && cargo nextest run
```

**Or use the convenient aliases:**

```bash
cargo c && cargo t  # Defined in .cargo/config.toml
```

### Additional Checks

**Spell checking:** Run `typos` before committing. CI enforces this.

**Workflow files:** Run `actionlint` after ANY workflow changes — no exceptions.

```bash
typos                                    # Spell check
actionlint                               # GitHub Actions linting
```

### Markdown Formatting

**Always run markdownlint after editing any markdown file.** The `--fix` flag auto-fixes most issues:

```bash
# Fix a specific file
npx markdownlint 'docs/file.md' --config .markdownlint.json --fix

# Fix all markdown files in a directory
npx markdownlint 'docs/**/*.md' --config .markdownlint.json --fix

# Check all workspace markdown
npx markdownlint '**/*.md' --config .markdownlint.json --fix
```

**Pre-commit hook:** The hook automatically runs markdownlint with `--fix` on staged markdown files.

**Link validation:** Run `./scripts/check-links.sh` after editing markdown — CI will fail on broken links.

**See also:** [markdown-formatting.md](skills/markdown-formatting.md) for complete style rules, common fixes, and CI configuration.

### For Agents and Sub-Agents

When spawning sub-agents or using Task tools to make code changes:

1. The sub-agent MUST run `cargo fmt` on any files it modifies
2. The sub-agent MUST verify `cargo clippy --all-targets` passes
3. If the sub-agent cannot run these commands, the parent agent must run them after receiving the changes

**See also:** [GitHub Actions Best Practices](skills/github-actions-best-practices.md) and [Markdown Link Validation](skills/markdown-link-validation.md) for detailed guidance.

---

*License: MIT OR Apache-2.0*
