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

### Test Structure (Arrange-Act-Assert)

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

- **Specs model production** — TLA+/Kani/Z3 specs represent real code behavior
- **When verification fails, assume production bug first** — Investigate before relaxing specs
- **Never "fix" specs just to make them pass** — That defeats the purpose
- **Invariants represent real safety properties** — Only relax with strong justification

### After Finding a Bug via Verification

Add comprehensive test coverage:

1. **Direct reproduction** — Cover the exact discovered scenario
2. **Edge cases** — Zero, max, boundary conditions
3. **Chained operations** — Sequential calls that might compound
4. **Lifecycle tests** — Create-use-modify-destroy cycles
5. **Negative tests** — Ensure violations are detected

---

## Defensive Programming Patterns

> **See also:** The complete guides in `.llm/skills/`:
>
> - [defensive-programming.md](.llm/skills/defensive-programming.md) — Zero-panic policy, error handling, safe patterns
> - [type-driven-design.md](.llm/skills/type-driven-design.md) — Parse don't validate, newtypes, typestate
> - [rust-pitfalls.md](.llm/skills/rust-pitfalls.md) — Common bugs that compile but cause problems

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
| **Frame** | Discrete time step in game simulation |
| **Rollback** | Restoring previous state when predictions are wrong |
| **Input Delay** | Buffer frames to smooth network jitter |
| **Prediction** | Continue simulation before inputs arrive |
| **Desync** | State divergence between peers (detected via checksums) |
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

- All new features must include tests
- Aim for >90% code coverage
- Include positive and negative test cases
- Test edge cases and error conditions
- Use integration tests for cross-component behavior

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

### Why This Matters

- **CI runs `cargo fmt --check`** — Any formatting differences will fail the build
- **CI runs clippy with warnings as errors** — Any clippy warning fails the build
- **Agents/sub-agents must also follow this** — All code changes, regardless of source, must be formatted and linted

### For Agents and Sub-Agents

When spawning sub-agents or using Task tools to make code changes:

1. The sub-agent MUST run `cargo fmt` on any files it modifies
2. The sub-agent MUST verify `cargo clippy --all-targets` passes
3. If the sub-agent cannot run these commands, the parent agent must run them after receiving the changes

### Additional Linters

For non-Rust files, the following linters are run in CI:

```bash
# Markdown files (CLAUDE.md, .llm/context.md, etc.)
npx markdownlint-cli <file.md>

# GitHub Actions workflows
actionlint  # or: ~/go/bin/actionlint
```

**Markdownlint rules to remember:**

- Lists must be surrounded by blank lines (MD032)
- No trailing spaces
- Proper heading hierarchy

---

*License: MIT OR Apache-2.0*
