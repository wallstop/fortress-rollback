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
# Run after every change (use nextest for 12x faster tests)
cargo fmt && cargo clippy --all-targets && cargo nextest run

# Or use the convenient aliases defined in .cargo/config.toml
cargo c && cargo t

# Check for typos (CI will fail if typos are found)
typos

# Standard cargo test (slower, but useful for doc tests)
cargo test

# Z3 verification (slow — compiles Z3 from source)
cargo test --features z3-verification
```

---

## High-Performance CLI Tools

The dev container includes modern, high-performance CLI tools. **Always use these tools instead of their traditional counterparts** — they are faster, more intuitive, and provide better output.

### Tool Reference

| Modern Tool           | Replaces     | Why Better                                                              |
|-----------------------|--------------|-------------------------------------------------------------------------|
| `rg` (ripgrep)        | `grep`       | 10-100x faster, respects `.gitignore`, better regex, colored output     |
| `fd` / `fdfind`       | `find`       | 5x faster, intuitive syntax, respects `.gitignore`, colored output      |
| `bat` / `batcat`      | `cat`/`less` | Syntax highlighting, line numbers, git integration                      |
| `eza`                 | `ls`         | Icons, git status, tree view, better defaults                           |
| `delta`               | `diff`       | Side-by-side diffs, syntax highlighting (auto-configured for git)       |
| `sd`                  | `sed`        | Intuitive syntax, regex support, no escaping headaches                  |
| `dust`                | `du`         | Visual directory size with percentages, sorted output                   |
| `duf`                 | `df`         | Better disk usage display                                               |
| `procs`               | `ps`         | Colored output, tree view, searchable, better defaults                  |
| `htop` / `btm`        | `top`        | Interactive process viewer                                              |
| `hyperfine`           | `time`       | Statistical benchmarking, multiple runs                                 |
| `tokei`               | `cloc`/`wc`  | Fast code statistics by language, accurate line counts                  |
| `zoxide` (`z`)        | `cd`         | Learns your habits, jump to frequent directories                        |
| `fzf`                 | —            | Fuzzy finder for files, history, anything                               |
| `jq`                  | —            | JSON processor and pretty-printer                                       |
| `yq`                  | —            | YAML processor (like jq for YAML)                                       |
| `ncdu`                | `du`         | Interactive disk usage analyzer                                         |
| `tldr` (tealdeer)     | `man`        | Simplified man pages with examples (Rust-based)                         |
| `ag` (silversearcher) | `grep`       | Fast code search (alternative to rg)                                    |

### Shell Aliases (Pre-configured)

The following aliases are configured in the dev container, so traditional commands automatically use modern tools:

```bash
fd   → fdfind      # fd-find (Debian naming)
bat  → batcat      # bat (Debian naming)
ls   → eza         # eza with icons
cat  → bat         # bat with syntax highlighting (--paging=never)
diff → delta       # syntax highlighting
du   → dust        # visual disk usage
df   → duf         # disk usage
ps   → procs       # modern process viewer
top  → htop        # interactive
sed  → sd          # intuitive find-and-replace
z    → zoxide      # smart cd navigation
```

### Mandatory Tool Usage Rules

**ALWAYS use `rg` (ripgrep) instead of `grep`:**

```bash
# ✅ CORRECT - Use ripgrep
rg "FortressError"                              # Search all files
rg "impl.*Session" --type rust                  # Search only Rust files
rg -l "rollback"                                # List files containing match
rg "pattern" -C 3                               # Show 3 lines of context
rg "error" -i                                   # Case-insensitive search
rg "pattern" -A 5 -B 2                          # 5 lines after, 2 before
rg "pattern" --no-ignore                        # Include gitignored files

# ❌ NEVER use grep
grep -r "pattern" .                             # Slow, no syntax highlighting
grep -rn "pattern" --include="*.rs" .           # Verbose, slower
```

**ALWAYS use `fd` instead of `find`:**

```bash
# ✅ CORRECT - Use fd
fd "\.rs$"                                      # Find all Rust files
fd -e toml                                      # Find by extension
fd test src/                                    # Find files matching "test" in src/
fd "Tests" --type d                             # Find directories named Tests
fd "pattern" --hidden                           # Include hidden files
fd "pattern" --no-ignore                        # Include gitignored files
fd "pattern" -x echo {}                         # Execute command on results

# ❌ NEVER use find
find . -name "*.rs"                             # Slow, verbose syntax
find . -type d -name "Tests"                    # More typing, slower
```

**ALWAYS use `bat` with `--paging=never` instead of `cat`:**

```bash
# ✅ CORRECT - Use bat with --paging=never
bat --paging=never src/lib.rs                   # View with syntax highlighting
bat --paging=never -n src/lib.rs                # Show line numbers only
bat --paging=never -r 10:20 src/lib.rs          # Show lines 10-20
bat --paging=never -p src/lib.rs                # Plain output (no decorations)
bat --paging=never -l rust file.txt             # Force Rust syntax highlighting
bat --paging=never --style=plain src/lib.rs     # No line numbers or decorations

# ✅ CORRECT - Combining with other tools
head -n 50 src/lib.rs | bat --paging=never -l rust   # First 50 lines with highlighting
tail -n 50 src/lib.rs | bat --paging=never -l rust   # Last 50 lines with highlighting
rg "pattern" -C 3 | bat --paging=never -l rust       # Search results with highlighting

# ❌ NEVER use bare bat without --paging=never - it will block
bat src/lib.rs                                  # BLOCKS waiting for pager input!
bat -n src/lib.rs                               # BLOCKS!

# ❌ NEVER use cat - no syntax highlighting
cat src/lib.rs                                  # No highlighting, harder to read
```

**ALWAYS use `eza` instead of `ls`:**

```bash
# ✅ CORRECT - Use eza
eza -la                                         # List all with details
eza --tree                                      # Tree view
eza --tree --level=2                            # Tree with depth limit
eza -la --git                                   # Show git status
eza --icons                                     # Show file type icons

# ❌ NEVER use ls
ls -la                                          # No icons, no git status
```

**ALWAYS use `sd` instead of `sed` for find-and-replace:**

```bash
# ✅ CORRECT - Use sd
sd 'old_pattern' 'new_text' file.rs             # Simple replacement
sd 'foo(\d+)' 'bar$1' file.rs                   # Regex with capture groups
sd -F 'literal.string' 'replacement' file.rs   # Fixed string (no regex)
echo "hello world" | sd 'world' 'universe'      # Pipe support
fd -e rs | xargs sd 'OldStruct' 'NewStruct'     # Bulk replace in files

# ❌ NEVER use sed
sed -i 's/old/new/g' file.rs                    # Escape nightmare
sed -E 's/foo([0-9]+)/bar\1/g' file.rs          # Confusing syntax
```

**ALWAYS use `dust` instead of `du` for disk usage:**

```bash
# ✅ CORRECT - Use dust
dust                                            # Visual size breakdown of current dir
dust -r                                         # Reverse order (largest last)
dust -d 2                                       # Limit depth to 2 levels
dust -n 20 src/                                 # Show top 20 entries in src/

# ❌ NEVER use du
du -sh *                                        # No visual breakdown, unsorted
du -h --max-depth=2                             # Harder to read output
```

**ALWAYS use `procs` instead of `ps` for process viewing:**

```bash
# ✅ CORRECT - Use procs
procs                                           # List all processes (colored, sorted)
procs --tree                                    # Show process tree
procs cargo                                     # Filter by name
procs --sortd cpu                               # Sort by CPU descending
procs --watch                                   # Watch mode (auto-refresh)

# ❌ NEVER use ps
ps aux                                          # Hard to read, no colors
ps aux | grep cargo                             # Awkward filtering
```

**Use `tokei` for code statistics:**

```bash
# ✅ CORRECT - Use tokei
tokei                                           # Statistics for current project
tokei src/                                      # Statistics for specific directory
tokei -e tests                                  # Exclude directory
tokei -t Rust                                   # Only count Rust files
tokei --sort code                               # Sort by lines of code
```

**Use `hyperfine` for benchmarking:**

```bash
# ✅ CORRECT - Use hyperfine
hyperfine 'cargo test'                                # Benchmark with stats
hyperfine 'cargo test' 'cargo nextest run'            # Compare two commands
hyperfine --warmup 3 'cargo build --release'          # With warmup runs

# ❌ NEVER use time for benchmarking
time cargo test                                       # No statistics, single run
```

**NEVER redirect output to `/dev/null`:**

Suppressing output hides errors, warnings, and important diagnostic information. This makes debugging impossible and can mask real problems.

```bash
# ❌ NEVER suppress output - hides critical information
command 2>/dev/null                             # Hides all errors!
command >/dev/null 2>&1                         # Hides everything!
command 2>&1 >/dev/null                         # Still hides output!
curl -s url 2>/dev/null                         # Hides network errors!
some_script.sh 2>/dev/null || true              # Silently fails!

# ✅ CORRECT - Always preserve output for debugging
command                                         # See all output
command 2>&1 | head -20                         # Limit output but don't hide it
command || echo "Command failed with exit $?"   # Handle errors explicitly
curl -sf url || echo "curl failed"              # Use flags, handle errors
```

**Exception:** Only use `/dev/null` when you have explicitly documented *why* the output is irrelevant AND the command's success/failure is properly checked. This should be extremely rare.

**Use other modern tools:**

```bash
# duf - disk free space
duf                                             # Show disk usage beautifully

# jq - JSON processing
cat file.json | jq '.field'                     # Extract field
curl -s api/endpoint | jq '.'                   # Pretty-print JSON

# fzf - fuzzy finder
fd -e rs | fzf                                  # Fuzzy find Rust files
history | fzf                                   # Search command history

# z (zoxide) - smart directory jumping
z src                                           # Jump to most-used dir matching "src"
z network                                       # Jump to dir matching "network"
```

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
> - [tla-plus-modeling.md](skills/tla-plus-modeling.md) — TLA+ specification patterns and best practices
> - [kani-verification.md](skills/kani-verification.md) — Kani proof harnesses and Rust formal verification
> - [z3-verification.md](skills/z3-verification.md) — Z3 SMT solver proofs for algorithm correctness
> - [loom-testing.md](skills/loom-testing.md) — Loom concurrency permutation testing
> - [miri-verification.md](skills/miri-verification.md) — Miri undefined behavior detection
> - [miri-adaptation-guide.md](skills/miri-adaptation-guide.md) — Step-by-step Miri error fixes for agents
> - [mutation-testing.md](skills/mutation-testing.md) — Mutation testing for test quality verification
> - [property-testing.md](skills/property-testing.md) — Property-based testing to find edge cases automatically

- **Specs model production** — TLA+/Kani/Z3 specs represent real code behavior
- **When verification fails, assume production bug first** — Investigate before relaxing specs
- **Never "fix" specs just to make them pass** — That defeats the purpose
- **Invariants represent real safety properties** — Only relax with strong justification
- **Specs are abstract models, not code translations** — Model essential behavior, skip implementation details

### Kani Quick Reference

```bash
# Run all Kani proofs
cargo kani

# Run specific harness
cargo kani --harness verify_specific_function

# Verbose output
cargo kani -v
```

**Key Kani patterns:**

```rust
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    #[kani::unwind(10)]  // Set loop bound
    fn verify_no_panics() {
        let input: u32 = kani::any();           // Symbolic input (all values)
        kani::assume(input < 1000);             // Constrain state space
        let result = function_under_test(input);
        assert!(result.is_ok());                // Property to verify
    }
}
```

| Attribute | Purpose |
|-----------|---------|
| `#[kani::proof]` | Mark function as proof harness |
| `#[kani::unwind(N)]` | Set loop unwinding bound |
| `#[kani::stub(orig, repl)]` | Replace function for verification |
| `kani::any::<T>()` | Generate symbolic value (all possible T) |
| `kani::assume(cond)` | Narrow state space |

### TLA+ Quick Reference

```bash
# Run all TLA+ verification
./scripts/verify-tla.sh

# Run specific spec
./scripts/verify-tla.sh NetworkProtocol

# Quick verification (smaller bounds)
./scripts/verify-tla.sh --quick
```

**Key specs in `specs/tla/`:**

| Spec | Verifies |
|------|----------|
| `Rollback.tla` | Rollback mechanism (bounded depth, state availability) |
| `InputQueue.tla` | Input queue (FIFO order, bounded length) |
| `NetworkProtocol.tla` | Protocol state machine (valid transitions) |
| `Concurrency.tla` | Thread safety (mutual exclusion) |
| `ChecksumExchange.tla` | Desync detection |

### Z3 Quick Reference

```bash
# Run Z3 verification tests (requires system Z3: apt install libz3-dev)
cargo test --features z3-verification

# Bundled build (slow, ~30+ minutes - compiles Z3 from source)
cargo test --features z3-verification-bundled

# Run specific Z3 test
cargo test --features z3-verification -- z3_proof_circular_index
```

**Key Z3 patterns:**

```rust
#[cfg(feature = "z3-verification")]
use z3::{ast::Int, with_z3_config, Config, SatResult, Solver};

#[test]
fn z3_proof_property_holds() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();
        
        let x = Int::fresh_const("x");        // Symbolic variable
        solver.assert(x.ge(0));               // Precondition
        
        let result = &x % 128;                // Computation
        solver.assert(result.ge(128));        // Negate property to prove
        
        // UNSAT = property holds (no counterexample)
        assert_eq!(solver.check(), SatResult::Unsat);
    });
}
```

| Function | Purpose |
|----------|---------|
| `Int::fresh_const("name")` | Create symbolic integer variable |
| `solver.assert(constraint)` | Add constraint to solver |
| `solver.check()` | Check satisfiability |
| `x.ge(n)` / `x.lt(n)` | Comparisons (return Bool) |
| `&x + &y` / `&x % n` | Arithmetic operations |
| `SatResult::Unsat` | No solution exists (property proved!) |

**Key Z3 proofs in `tests/verification/z3.rs`:**

| Proof | Verifies |
|-------|----------|
| `z3_proof_circular_index_valid` | Modulo always produces valid index |
| `z3_proof_rollback_target_in_past` | Rollback target < current frame |
| `z3_proof_frame_delay_prevents_overflow` | Frame delay validation |
| `z3_proof_desync_detection_no_false_positives` | Desync only on mismatch |

### Loom Quick Reference

```bash
# Run loom tests (from loom-tests/ directory)
cd loom-tests
RUSTFLAGS="--cfg loom" cargo test --release

# Run specific loom test
RUSTFLAGS="--cfg loom" cargo test --release test_concurrent_saves

# With debugging output
LOOM_LOG=trace LOOM_LOCATION=1 RUSTFLAGS="--cfg loom" cargo test --release

# Limit state space for larger tests
LOOM_MAX_PREEMPTIONS=2 RUSTFLAGS="--cfg loom" cargo test --release
```

**Key loom patterns:**

```rust
#![cfg(loom)]  // Only compile under loom

use loom::sync::Arc;
use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::thread;

#[test]
fn test_concurrent_access() {
    loom::model(|| {
        let data = Arc::new(AtomicUsize::new(0));
        let data2 = data.clone();
        
        let t = thread::spawn(move || {
            data2.fetch_add(1, Ordering::SeqCst);
        });
        
        data.fetch_add(1, Ordering::SeqCst);
        t.join().unwrap();
        
        assert_eq!(data.load(Ordering::SeqCst), 2);
    });
}
```

| Environment Variable | Purpose |
|---------------------|---------|
| `LOOM_LOG=trace` | Enable detailed logging |
| `LOOM_LOCATION=1` | Include source locations in output |
| `LOOM_MAX_PREEMPTIONS=N` | Limit preemptions (for large tests) |
| `LOOM_CHECKPOINT_FILE=f.json` | Save/restore test progress |

**Key loom tests in `loom-tests/tests/`:**

| Test File | Verifies |
|-----------|----------|
| `game_state_cell.rs` | Thread-safe game state storage |
| `saved_states.rs` | Circular buffer concurrency |

### Miri Quick Reference

```bash
# Install Miri (requires nightly)
rustup +nightly component add miri

# Run Miri on tests
cargo +nightly miri test

# Run with isolation disabled (for env/file access)
MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test

# Test multiple random executions (find race conditions)
MIRIFLAGS="-Zmiri-many-seeds=0..16" cargo +nightly miri test

# Use Tree Borrows (more permissive aliasing model)
MIRIFLAGS="-Zmiri-tree-borrows" cargo +nightly miri test

# Cross-platform test (big-endian)
cargo +nightly miri test --target s390x-unknown-linux-gnu

# Debug specific allocation/pointer
MIRIFLAGS="-Zmiri-track-alloc-id=<id>" cargo +nightly miri test
```

**Key Miri flags:**

| Flag | Purpose |
|------|---------|
| `-Zmiri-disable-isolation` | Access host filesystem, env vars |
| `-Zmiri-tree-borrows` | Use Tree Borrows instead of Stacked Borrows |
| `-Zmiri-many-seeds=0..N` | Test N different random executions |
| `-Zmiri-strict-provenance` | Enforce strict pointer provenance |
| `-Zmiri-symbolic-alignment-check` | Stricter alignment checking |

**Adapting code for Miri:**

```rust
// Skip unsupported tests under Miri
#[test]
#[cfg_attr(miri, ignore)]
fn uses_ffi_or_networking() { /* ... */ }

// Reduce iterations for Miri (very slow interpreter)
let iterations = if cfg!(miri) { 10 } else { 10_000 };
```

**What Miri detects:**

- Out-of-bounds access, use-after-free
- Uninitialized memory reads
- Misaligned pointers/references
- Invalid type invariants (bad `bool`, enum discriminant)
- Data races
- Stacked Borrows / Tree Borrows aliasing violations
- Memory leaks

**What Miri does NOT detect:**

- All thread interleavings (use Loom)
- Complete weak memory behaviors (use Loom)
- FFI/foreign function UB
- Platform-specific API issues

### Mutation Testing Quick Reference

```bash
# Install cargo-mutants
cargo install --locked cargo-mutants

# Run on specific module (recommended)
cargo mutants -f src/rle.rs --timeout 30 --jobs 4 -- --lib

# List mutations without running
cargo mutants --list -f src/module.rs

# Run with nextest (faster)
cargo mutants -- --all-targets
```

**Understanding results:**

| Outcome | Meaning | Action |
|---------|---------|--------|
| **Caught** ✅ | Test failed → mutant killed | Good coverage |
| **Missed** ⚠️ | Tests still pass → gap | Improve tests |
| **Timeout** ⏱️ | Test hung (infinite loop) | Usually acceptable |
| **Unviable** 🔨 | Doesn't compile | Inconclusive |

**Key principle**: Code coverage shows code runs. Mutation testing shows tests would notice if code broke.

```rust
// ❌ Weak test — mutation can survive
assert!(result.is_ok());

// ✅ Strong test — catches mutations
assert_eq!(result, Ok(expected_value));
```

**Configuration:** See `.cargo/mutants.toml` for project settings.

### Property Testing Quick Reference

```bash
# Add to Cargo.toml [dev-dependencies]
proptest = "1.9"

# Run property tests
cargo test

# Run specific property test
cargo test prop_roundtrip
```

**Common patterns:**

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_roundtrip(data in any::<Vec<u8>>()) {
        let encoded = encode(&data)?;
        let decoded = decode(&encoded)?;
        prop_assert_eq!(data, decoded);
    }
    
    #[test]
    fn prop_invariant_maintained(ops in prop::collection::vec(any::<Op>(), 0..100)) {
        let mut state = State::new();
        for op in ops {
            state.apply(op)?;
            prop_assert!(state.check_invariants());
        }
    }
}
```

**Key patterns:**

| Pattern | Property | Example |
|---------|----------|---------|
| **Round-trip** | `decode(encode(x)) == x` | Serialization |
| **Commutativity** | `a + b == b + a` | Math operations |
| **Idempotency** | `f(f(x)) == f(x)` | Normalization |
| **Invariants** | Property always holds | Sorted order |
| **Oracle** | Compare with reference impl | Optimized vs naive |

**Strategy cheat sheet:**

| Need | Strategy |
|------|----------|
| Any value | `any::<T>()` |
| Range | `0..100i32` |
| Bounded vec | `prop::collection::vec(any::<T>(), 0..100)` |
| Regex string | `"[a-z]+"` |
| Choose variants | `prop_oneof![a, b, c]` |
| Transform | `strategy.prop_map(\|x\| f(x))` |

### After Finding a Bug via Verification

Add comprehensive test coverage:

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

> **See also:** The complete guides in `.llm/skills/`:
>
> - [defensive-programming.md](skills/defensive-programming.md) — Zero-panic policy, error handling, safe patterns
> - [type-driven-design.md](skills/type-driven-design.md) — Parse don't validate, newtypes, typestate
> - [rust-pitfalls.md](skills/rust-pitfalls.md) — Common bugs that compile but cause problems
> - [loom-testing.md](skills/loom-testing.md) — Loom concurrency permutation testing
> - [miri-verification.md](skills/miri-verification.md) — Miri undefined behavior detection
> - [concurrency-patterns.md](skills/concurrency-patterns.md) — Thread-safe Rust patterns
> - [mutation-testing.md](skills/mutation-testing.md) — Mutation testing to verify test quality
> - [property-testing.md](skills/property-testing.md) — Property-based testing for invariant verification

### Zero-Panic Policy (CRITICAL)

**Production code must NEVER panic.** This is non-negotiable.

- All errors must be returned as `Result<T, FortressError>`
- APIs must be robust and resilient to all possible inputs
- Internal state must remain consistent even when errors occur
- Callers must be forced to handle potential failures explicitly

```rust
// ❌ FORBIDDEN in production code
value.unwrap()                    // Panics on None
value.expect("msg")               // Panics with message
array[index]                      // Panics on out-of-bounds
panic!("something went wrong")   // Explicit panic
todo!()                           // Panics as placeholder
unreachable!()                    // Panics (use only when TRULY unreachable)
assert!(condition)                // Panics on false (tests only)

// ✅ REQUIRED - Return Results, let caller decide
value.ok_or(FortressError::MissingValue)?          // Convert Option to Result
array.get(index).ok_or(FortressError::OutOfBounds)?  // Safe indexing
if !valid { return Err(FortressError::InvalidState); }  // Explicit error
```

### Never Swallow Errors

Errors must be propagated to callers, not hidden:

```rust
// ❌ FORBIDDEN - Silently swallows errors
let _ = fallible_operation();           // Ignores Result
if let Ok(v) = operation() { use(v); }  // Silently ignores Err
match result { Ok(v) => v, Err(_) => default }  // Hides error

// ✅ REQUIRED - Propagate or explicitly handle
fallible_operation()?;                   // Propagate with ?
fallible_operation().map_err(|e| {       // Transform and propagate
    FortressError::Wrapped(e)
})?;
match result {
    Ok(v) => v,
    Err(e) => return Err(e.into()),      // Explicit propagation
}
```

### Assume Nothing, Validate Everything

Do not assume inputs or internal state are valid:

```rust
// ❌ Avoid: Assumes state is valid
fn process(&self) {
    let player = &self.players[self.current_player];  // May panic
    player.process();
}

// ✅ Prefer: Validate and return errors
fn process(&self) -> Result<(), FortressError> {
    let player = self.players
        .get(self.current_player)
        .ok_or(FortressError::InvalidPlayerIndex(self.current_player))?;
    player.process()
}
```

### Prefer Pattern Matching Over Indexing

```rust
// ❌ Avoid: Can panic
if !users.is_empty() { let first = &users[0]; }

// ✅ Prefer: Compiler-enforced safety
match users.as_slice() {
    [] => handle_empty(),
    [single] => handle_one(single),
    [first, rest @ ..] => handle_multiple(first, rest),
}
```

### Explicit Field Initialization

```rust
// ❌ Avoid: New fields silently use defaults
let config = Config { field1: value1, ..Default::default() };

// ✅ Prefer: Compiler forces handling new fields
let config = Config { field1, field2, field3 };
```

### Exhaustive Match Arms

```rust
// ❌ Avoid: Wildcard hides unhandled variants
match state { State::Ready => {}, _ => {} }

// ✅ Prefer: Explicit — compiler catches new variants
match state { State::Ready => {}, State::Running => {}, State::Paused => {} }
```

### Enums Over Booleans

```rust
// ❌ Avoid: What does true mean?
process_data(&data, true, false, true);

// ✅ Prefer: Self-documenting
process_data(&data, Compression::Enabled, Encryption::Disabled, Validation::Strict);
```

### Maintain Invariants

Internal state must always be consistent:

```rust
// ❌ Avoid: Partial updates can leave inconsistent state
fn update(&mut self, new_count: usize) {
    self.count = new_count;          // Updated
    self.items.resize(new_count, 0); // May fail, leaving count wrong
}

// ✅ Prefer: Atomic updates or rollback on failure
fn update(&mut self, new_count: usize) -> Result<(), FortressError> {
    let mut new_items = self.items.clone();
    new_items.resize(new_count, 0);  // Prepare new state
    // Only update if all operations succeed
    self.items = new_items;
    self.count = new_count;
    Ok(())
}
```

### Use Type System for Safety

```rust
// ❌ Avoid: Runtime checks for compile-time guarantees
fn set_frame(frame: i32) -> Result<(), Error> {
    if frame < 0 { return Err(Error::NegativeFrame); }
    // ...
}

// ✅ Prefer: Make invalid states unrepresentable
struct Frame(u32);  // Cannot be negative by construction

fn set_frame(frame: Frame) {
    // frame is guaranteed valid by the type system
}
```

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
| **Determinism** | Same inputs → same outputs (critical requirement) |

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

### Spell Checking with typos (REQUIRED)

**ALWAYS run `typos` locally before committing to catch spelling errors early:**

```bash
# Check for typos in the entire project
typos

# Or explicitly check from current directory
typos .

# Check specific files
typos src/lib.rs README.md
```

**Installation:**

```bash
cargo install typos-cli
```

**Configuration:**

- See [.typos.toml](../.typos.toml) for project-specific word exceptions
- Add project-specific terms (e.g., `ggrs`, `desync`, `resimulation`) to `[default.extend-words]`

**Why this matters:**

- **CI runs typos** — The `ci-quality.yml` workflow runs typos as an advisory check (non-blocking)
- **Catch errors early** — Running locally surfaces issues before code review
- **Fast** — `typos` is extremely fast, typically completing in under a second

### Why This Matters

- **CI runs `cargo fmt --check`** — Any formatting differences will fail the build
- **CI runs clippy with warnings as errors** — Any clippy warning fails the build
- **Agents/sub-agents must also follow this** — All code changes, regardless of source, must be formatted and linted

### For Agents and Sub-Agents

When spawning sub-agents or using Task tools to make code changes:

1. The sub-agent MUST run `cargo fmt` on any files it modifies
2. The sub-agent MUST verify `cargo clippy --all-targets` passes
3. If the sub-agent cannot run these commands, the parent agent must run them after receiving the changes

### Markdown Linting and Link Checking (REQUIRED)

**ALWAYS run these checks after modifying ANY markdown file:**

```bash
# Markdown linting (uses project config)
npx markdownlint '<file.md>' --config .markdownlint.json

# Example: lint this file
npx markdownlint '.llm/context.md' --config .markdownlint.json

# Link checking (validates all internal links) — RUN AFTER EVERY MARKDOWN CHANGE
./scripts/check-links.sh
```

> **⚠️ CI will fail on broken links.** The link checker runs in CI — always validate locally first.

**Key markdown rules to remember:**

| Rule  | Description                            | Fix                                      |
|-------|----------------------------------------|------------------------------------------|
| MD010 | Hard tabs                              | Use spaces, never hard tabs              |
| MD031 | Code blocks must have blank lines      | Add blank line before and after fences   |
| MD032 | Lists must have blank lines            | Add blank line before and after lists    |

**Relative link path rules (critical for avoiding broken links):**

Links resolve **from the directory containing the markdown file**, not from repo root.

| File Location | Linking To | Correct Path Syntax |
|---------------|------------|---------------------|
| Root (`README.md`) | `docs/user-guide.md` | `[Guide]` + `(docs/user-guide.md)` |
| `docs/user-guide.md` | Root `README.md` | `[README]` + `(../README.md)` |
| `.github/*.md` | `.llm/context.md` | `[Context]` + `(../.llm/context.md)` |
| `.github/*.md` | Root files | `[README]` + `(../README.md)` |
| `.llm/context.md` | Skills files | `[Skill]` + `(skills/defensive-programming.md)` |

**Common mistakes:**

- ❌ WRONG from `.github/`: `[Context]` + `(.llm/context.md)` — missing `../` prefix
- ❌ WRONG from `.llm/`: `[Skill]` + `(.llm/skills/foo.md)` — don't repeat current dir
- ✅ CORRECT from `.github/`: `[Context]` + `(../.llm/context.md)` — go up first
- ✅ CORRECT from `.llm/`: `[Skill]` + `(skills/foo.md)` — relative to current dir

**See also:** [Markdown Link Validation](skills/markdown-link-validation.md) — comprehensive guide with examples

### GitHub Actions Workflow Linting (REQUIRED)

**ALWAYS run `actionlint` after modifying ANY workflow file in `.github/workflows/`:**

```bash
# Lint all workflow files
actionlint

# Lint a specific workflow
actionlint .github/workflows/ci-security.yml
```

`actionlint` catches:

- YAML syntax errors
- Invalid GitHub Actions expressions (`${{ }}`)
- Shell script errors via embedded shellcheck
- Invalid runner labels, missing permissions, etc.

**Common shellcheck issues in workflows:**

| Issue | Problem | Fix |
|-------|---------|-----|
| SC2016 | Variables in single quotes won't expand | Use double quotes, or suppress if intentional |
| SC2086 | Unquoted variables | Quote: `"$VAR"` |
| SC2129 | Multiple `>> file` redirects | Use `{ cmd1; cmd2; } >> file` |
| SC2155 | `export x=$(cmd)` | Separate: `x=$(cmd); export x` |

**When shellcheck warnings are intentional — suppress with comment:**

```bash
# shellcheck disable=SC2016  # Intentional: function body expands at call time, not definition
git config --global credential.helper '!f() { echo "password=${GITHUB_TOKEN}"; }; f'
```

**Always explain WHY** the suppression is needed — future readers must understand the intent.

**Example — SC2129 fix for GITHUB_OUTPUT:**

```yaml
# ❌ WRONG: Individual redirects
echo "foo=bar" >> "$GITHUB_OUTPUT"
echo "baz=qux" >> "$GITHUB_OUTPUT"

# ✅ CORRECT: Grouped redirects
{
  echo "foo=bar"
  echo "baz=qux"
} >> "$GITHUB_OUTPUT"
```

**See also:** [GitHub Actions Best Practices](skills/github-actions-best-practices.md) for comprehensive guidance.

---

*License: MIT OR Apache-2.0*
