# Fortress Rollback — LLM Development Guide

> **This is the canonical source of truth** for project context. All other LLM instruction files (CLAUDE.md, AGENTS.md, copilot-instructions.md) point here.

## TL;DR — What You Need to Know

**Fortress Rollback** is a correctness-first fork of GGRS (Good Game Rollback System), written in 100% safe Rust. It provides peer-to-peer rollback networking for deterministic multiplayer games.

### The Four Pillars

1. **>90% test coverage** — All code must be thoroughly tested
2. **Formal verification** — TLA+, Z3, and Kani for critical components
3. **Enhanced usability** — Intuitive, type-safe, hard-to-misuse APIs
4. **Code clarity** — Readable, maintainable, well-documented

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

The dev container includes modern, high-performance CLI tools. **Always prefer these over their traditional counterparts** for faster, more user-friendly results.

### Tool Mapping (Always Use Left Column)

| Use This | Instead Of | Why |
|----------|------------|-----|
| `rg` (ripgrep) | `grep` | 10-100x faster, respects .gitignore, better output |
| `fd` / `fdfind` | `find` | Much faster, intuitive syntax, respects .gitignore |
| `bat` / `batcat` | `cat` | Syntax highlighting, line numbers, git integration |
| `eza` | `ls` | Colors, git status, tree view built-in |
| `sd` | `sed` | Intuitive regex syntax, no escaping hell |
| `delta` | `diff` | Syntax highlighting, side-by-side, git integration |
| `dust` | `du` | Visual, sorted output, much faster |
| `duf` | `df` | Beautiful, colored disk usage |
| `procs` | `ps` | Colored, searchable, tree view |
| `htop` / `btm` | `top` | Interactive, visual, better UX |
| `hyperfine` | `time` | Statistical benchmarking, multiple runs |
| `tokei` | `cloc`/`wc -l` | Fast code statistics |
| `zoxide` (`z`) | `cd` | Smart directory jumping, learns your habits |

### Common Usage Examples

```bash
# Search for text in code (use rg, not grep)
rg "FortressError"                    # Search all files
rg "impl.*Session" --type rust        # Search only Rust files
rg -l "rollback"                      # List files containing match

# Find files (use fd, not find)
fd "\.rs$"                            # Find all Rust files
fd -e toml                            # Find by extension
fd test src/                          # Find files matching "test" in src/

# View files with syntax highlighting (use bat, not cat)
bat src/lib.rs                        # View with highlighting
bat -r 10:20 src/lib.rs               # View line range

# Replace text (use sd, not sed)
sd 'old_pattern' 'new_text' file.rs   # Simple replacement
sd -s 'literal[text]' 'new' file.rs   # Literal string mode

# Directory listing (use eza, not ls)
eza -la                               # Long listing with all files
eza --tree --level=2                  # Tree view, 2 levels deep
eza --git                             # Show git status

# Disk usage (use dust, not du)
dust                                  # Current directory usage
dust -d 2 src/                        # Depth-limited

# Benchmarking (use hyperfine, not time)
hyperfine 'cargo test'                # Benchmark with stats
hyperfine 'cargo test' 'cargo nextest run'  # Compare two commands

# Code statistics
tokei                                 # Lines of code by language
tokei src/                            # Specific directory
```

### Shell Aliases (Pre-configured)

The dev container configures these aliases automatically:
- `fd` → `fdfind`, `bat` → `batcat` (Debian naming)
- `ls` → `eza`, `cat` → `bat`, `diff` → `delta`
- `du` → `dust`, `df` → `duf`, `ps` → `procs`, `top` → `htop`
- `z` → zoxide (smart cd that learns your habits)

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
6. **Update changelog** — Document changes in `docs/changelog.md`

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
- **No panics in library code** — Always use `Result`
- **All clippy lints pass** — `clippy::all`, `clippy::pedantic`, `clippy::nursery`
- **No broken doc links** — All intra-doc links must resolve
- **Public items documented** — Rustdoc with examples

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

---

## Project Architecture

### Repository Structure

```
src/
├── lib.rs                    # Public API entry point
├── error.rs                  # FortressError types
├── frame_info.rs             # Frame metadata
├── input_queue.rs            # Input buffering
├── hash.rs                   # Deterministic FNV-1a hashing
├── rle.rs                    # Run-length encoding
├── rng.rs                    # Deterministic PCG32 RNG
├── time_sync.rs              # Time synchronization
├── sync_layer/
│   ├── mod.rs                # Core synchronization (SyncLayer)
│   ├── game_state_cell.rs    # Thread-safe game state
│   └── saved_states.rs       # Circular buffer for rollback
├── network/
│   ├── compression.rs        # Message compression
│   ├── messages.rs           # Protocol messages
│   ├── network_stats.rs      # Statistics tracking
│   ├── chaos_socket.rs       # Testing socket with chaos
│   ├── udp_socket.rs         # UDP abstraction
│   └── protocol/             # Network protocol state machine
└── sessions/
    ├── builder.rs            # SessionBuilder pattern
    ├── p2p_session.rs        # P2P gameplay
    ├── p2p_spectator_session.rs  # Spectator mode
    └── sync_test_session.rs  # Determinism testing
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
- **Document all breaking changes** — Update `docs/changelog.md` and `docs/migration.md`

### Test Coverage Requirements

- All new features must include tests
- Aim for >90% code coverage
- Include positive and negative test cases
- Test edge cases and error conditions
- Use integration tests for cross-component behavior

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

- [ ] Compiles with no warnings
- [ ] All tests pass
- [ ] Includes tests for new functionality
- [ ] Rustdoc comments with examples
- [ ] 100% safe Rust (no unsafe)
- [ ] Handles all error cases
- [ ] Changelog updated if user-facing

---

*License: MIT OR Apache-2.0*
