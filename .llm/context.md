# Fortress Rollback

> **This is the canonical source of truth** for project context. Other LLM instruction files (CLAUDE.md, AGENTS.md, copilot-instructions.md) reference this document for shared guidelines.

## Project Overview

Fortress Rollback is the correctness-first fork of GGRS, a reimagination of the GGPO (Good Game Peace Out) network SDK, written in 100% safe Rust. It provides peer-to-peer rollback networking capabilities for games, enabling deterministic multiplayer experiences with low-latency input prediction and rollback mechanics.

### Key Features
- **100% Safe Rust**: No unsafe code, leveraging Rust's memory safety guarantees
- **Request-Based API**: Simplified control flow replacing callback-style API
- **P2P Rollback Networking**: Implements rollback netcode for real-time multiplayer games
- **Multiple Session Types**: P2P sessions, spectator sessions, and sync test sessions
- **Input Prediction & Rollback**: Handles network latency through prediction and state rollback
- **Desync Detection**: Checksum-based verification between peers

### Fork Goals
1. **>90% test coverage** - Comprehensive unit, integration, property-based, and determinism tests
2. **Formal verification** - TLA+ specifications, Z3 constraints, invariant proofs
3. **Enhanced usability** - Intuitive APIs, clear error messages, type safety
4. **Code clarity** - Readable, maintainable, well-documented

## Repository Structure

```
fortress-rollback/
├── src/
│   ├── lib.rs                    # Main library entry point & public API
│   ├── error.rs                  # Error types (FortressError)
│   ├── frame_info.rs             # Frame metadata and tracking
│   ├── input_queue.rs            # Input buffering and management
│   ├── sync_layer.rs             # Core synchronization logic
│   ├── time_sync.rs              # Time synchronization between peers
│   ├── network/
│   │   ├── compression.rs        # Network message compression
│   │   ├── messages.rs           # Network protocol messages
│   │   ├── network_stats.rs      # Network statistics tracking
│   │   ├── protocol.rs           # Network protocol implementation
│   │   └── udp_socket.rs         # UDP socket abstraction
│   └── sessions/
│       ├── builder.rs            # Session builder pattern
│       ├── p2p_session.rs        # Peer-to-peer session
│       ├── p2p_spectator_session.rs  # Spectator session
│       └── sync_test_session.rs  # Determinism testing session
├── specs/                        # Formal specifications (TLA+)
├── examples/                     # Example implementations
├── tests/                        # Integration tests
├── fuzz/                         # Fuzz testing targets
├── loom-tests/                   # Concurrency tests with Loom
└── Cargo.toml                    # Package manifest
```

## Core Concepts

### Session Types
- **P2PSession**: Standard peer-to-peer gameplay session
- **SpectatorSession**: Session for spectators who observe but don't participate
- **SyncTestSession**: Determinism verification session for testing

### Player Types
- **Local**: Player on the current device
- **Remote**: Player on a remote device (identified by socket address)
- **Spectator**: Remote observer (doesn't contribute input)

### Key Technical Concepts
- **Frames**: Discrete time steps in game simulation
- **Rollback**: Restoring previous state when predictions are wrong
- **Input Delay**: Buffer frames to smooth over network jitter
- **Prediction**: Continuing simulation before inputs arrive
- **Desync Detection**: Checksum verification between peers
- **Determinism**: Same inputs → same outputs (critical requirement)

---

## Development Policies

### Breaking Changes Policy
- **API compatibility is NOT required** - Breaking the public API is acceptable
- **Safety and correctness trump compatibility** - If a breaking change improves safety, determinism, or prevents misuse, make it
- **Document all breaking changes** - Update docs/changelog.md and docs/migration.md when APIs change
- **This fork prioritizes production-grade quality** - We are not a drop-in replacement for upstream GGRS

### Code Quality Standards
- 100% safe Rust (`#![forbid(unsafe_code)]`)
- Enable and satisfy clippy lints (all, pedantic, nursery)
- No panics in library code (use `Result<T, FortressError>`)
- No broken intra-doc links
- All public items have rustdoc with examples

### Build & Test Commands
After every major change, run:
```bash
cargo fmt
cargo clippy --all-targets
cargo test
```
Note: Use `--features z3-verification` only when working on Z3 tests (compiles Z3 from source, which is slow)

---

## Testing Guidelines

### Test Coverage Requirements
- All new features must include tests
- Aim for >90% code coverage
- Include both positive and negative test cases
- Test edge cases and error conditions
- Use integration tests for cross-component behavior

### Test Structure
```rust
#[test]
fn descriptive_test_name_explaining_scenario() {
    // Arrange: Set up test conditions
    let mut session = create_test_session();
    
    // Act: Execute the behavior being tested
    let result = session.some_operation(args);
    
    // Assert: Verify expected outcomes
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected_value);
}
```

### Root Cause Analysis for Test Failures
**CRITICAL: When tests fail or are flaky, always perform deep investigation**

The goal is NOT to "make the test pass" — it's to understand and fix the underlying issue.

#### Investigation Methodology
1. **Reproduce and characterize** - Run test multiple times; is it consistent or flaky? Under what conditions?
2. **Understand the assertion** - What property is the test verifying? Why should it hold?
3. **Trace execution** - Add logging, use debugger, examine state at failure point
4. **Form hypothesis** - What could cause this specific failure mode?
5. **Verify hypothesis** - Confirm understanding before implementing fix
6. **Consider scope** - Are there similar issues elsewhere in the codebase?

#### Distinguishing Test Bug vs Production Bug
**Production bug indicators:**
- Test expectations align with documented/intended behavior
- Multiple tests or users depend on the same behavior
- The test logic is simple and clearly correct

**Test bug indicators:**
- Test makes assumptions not guaranteed by the API contract
- Test has inherent race conditions or timing dependencies  
- Test uses mocking incorrectly or has setup errors
- Test expectations contradict documentation

#### Comprehensive Fix Requirements
1. **Fix at the correct level**:
   - Production bug → Fix library code, verify other tests still pass, add regression test if missing
   - Test bug → Fix test's incorrect assumptions, document why original was wrong
   - Timing issue → Use proper synchronization (channels, barriers, condvars) NOT sleeps
   - Flakiness → Find and eliminate non-determinism at its source
2. **Assess impact** - Does this fix affect other components? Run full test suite
3. **Add protection** - If production bug, ensure test coverage prevents regression
4. **Update docs** - If behavior was ambiguous, clarify documentation
5. **Document the fix** - Explain root cause and why the solution is correct

#### Strictly Forbidden "Fixes"
- ❌ Commenting out or weakening failing assertions
- ❌ Adding `Thread::sleep()` or arbitrary delays to "fix" timing
- ❌ Catching and ignoring/swallowing errors in tests
- ❌ Marking tests as `#[ignore]` without a documented fix plan
- ❌ Relaxing numeric tolerances without understanding why original was appropriate
- ❌ Changing expected values to match actual without root cause analysis
- ❌ Disabling test features that exist in production code

---

## Formal Verification

### Philosophy
- **Specs model production** - TLA+/Kani/Z3 specs must accurately represent production code behavior
- **When verification fails, assume production has a bug first** - investigate before relaxing specs
- **Never "fix" specs just to make them pass** - this defeats the purpose of verification
- **Document all spec changes** - explain what production behavior necessitates the change
- **Invariants represent real safety properties** - only relax with strong justification

### When Formal Verification Reveals Issues
After fixing any bug discovered through formal verification, code review, or other analysis, add comprehensive test coverage:

1. **Direct reproduction test** - Cover the exact scenario that was discovered
2. **Edge case variants** - Zero values, max values, boundary conditions
3. **Chained operations** - Multiple sequential calls that might compound issues
4. **Full lifecycle tests** - Create-use-modify-destroy cycles  
5. **Invariant preservation** - Verify invariants hold across all state transitions
6. **Negative tests** - Ensure violations are properly detected
7. **Document in tests** - Explain what was discovered and why the test matters

```rust
// Example: After discovering load_frame() didn't update last_saved_frame
#[test]
fn test_load_frame_updates_last_saved_frame_invariant() { /* Direct reproduction */ }

#[test]  
fn test_load_frame_zero_updates_last_saved_frame() { /* Edge case: frame 0 */ }

#[test]
fn test_multiple_rollbacks_maintain_invariants() { /* Chained operations */ }

#[test]
fn test_full_rollback_cycle_maintains_invariants() { /* Full lifecycle */ }

#[test]
fn test_invariant_checker_identifies_violations() { /* Negative test */ }
```

### Verification Tools
- **TLA+**: State machine modeling, concurrency correctness, protocol verification
- **Z3**: Algorithm correctness, invariant checking, safety properties
- **Loom**: Concurrency testing for Rust code

---

## Defensive Programming Patterns

Apply these patterns from [corrode.dev/blog/defensive-programming](https://corrode.dev/blog/defensive-programming/):

### Slice Pattern Matching Over Indexing
```rust
// ❌ Avoid: Decoupled length check and indexing can panic
if !users.is_empty() { let first = &users[0]; }

// ✅ Prefer: Compiler-enforced safe access
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
let config = Config { field1: value1, field2: value2, field3: value3 };
```

### Destructuring in Trait Implementations
```rust
// ✅ Prefer: Compiler error when fields are added
impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        let Self { size, price, timestamp: _ } = self;
        let Self { size: other_size, price: other_price, timestamp: _ } = other;
        size == other_size && price == other_price
    }
}
```

### TryFrom for Fallible Conversions
```rust
// ✅ Prefer: TryFrom makes fallibility explicit
impl TryFrom<RawData> for ProcessedData {
    type Error = ConversionError;
    fn try_from(raw: RawData) -> Result<Self, Self::Error> {
        Ok(Self { value: raw.value.ok_or(ConversionError::MissingValue)? })
    }
}
```

### Exhaustive Match Arms
```rust
// ❌ Avoid: Wildcard hides unhandled variants
match state { State::Ready => {}, _ => {} }

// ✅ Prefer: Explicit variants catch additions
match state { State::Ready => {}, State::Running | State::Paused => {} }
```

### Enums Over Booleans
```rust
// ❌ Avoid: process_data(&data, true, false, true);
// ✅ Prefer: process_data(&data, Compression::Enabled, Encryption::Disabled, Validation::Strict);
```

### Recommended Clippy Lints
```toml
[lints.clippy]
indexing_slicing = "warn"
fallible_impl_from = "deny"
wildcard_enum_match_arm = "warn"
must_use_candidate = "warn"
```

---

## Code Examples

### Session Builder Pattern
```rust
let session = SessionBuilder::new()
    .with_num_players(2)
    .with_input_delay(2)
    .with_max_prediction(8)
    .add_player(PlayerType::Local, 0)?
    .add_player(PlayerType::Remote(addr), 1)?
    .start_p2p_session()?;
```

### Request Handling
```rust
for request in session.events() {
    match request {
        GgrsRequest::SaveGameState { frame, cell } => {
            cell.save(frame, &game_state, None);
        }
        GgrsRequest::LoadGameState { frame, cell } => {
            game_state = cell.load(frame);
        }
        GgrsRequest::AdvanceFrame { inputs } => {
            game_state.update(&inputs);
        }
    }
}
```

### Function Documentation
```rust
/// Advances the session by one frame, processing inputs and handling rollback.
///
/// # Errors
/// Returns `FortressError::NotSynchronized` if session is still synchronizing.
///
/// # Examples
/// ```
/// session.advance_frame()?;
/// ```
pub fn advance_frame(&mut self) -> Result<(), FortressError> {
    // Implementation
}
```

---

## Quality Checklist

Before suggesting code, ensure it:
- [ ] Compiles with no warnings
- [ ] Includes tests (unit and/or integration)
- [ ] Has rustdoc comments for public items
- [ ] Follows Rust idioms and best practices
- [ ] Maintains 100% safe Rust
- [ ] Handles errors appropriately (no panics)
- [ ] Considers performance implications
- [ ] Works toward >90% coverage goal

---

## Resources

- [Original GGPO SDK](https://www.ggpo.net/)
- [GGPO Developers Discord](https://discord.com/invite/8FKKhCRCCE)
- [Bevy GGRS Plugin](https://github.com/gschup/bevy_ggrs)
- [TLA+ Resources](https://lamport.azurewebsites.net/tla/tla.html)
- [Z3 Theorem Prover](https://github.com/Z3Prover/z3)

## License

Dual-licensed under MIT OR Apache-2.0
