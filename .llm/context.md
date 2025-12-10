# Fortress Rollback (formerly GGRS)

## Project Overview

Fortress Rollback is the correctness-first fork of GGRS, a reimagination of the GGPO (Good Game Peace Out) network SDK, written in 100% safe Rust. It provides peer-to-peer rollback networking capabilities for games, enabling deterministic multiplayer experiences with low-latency input prediction and rollback mechanics.

### Key Features
- **100% Safe Rust**: No unsafe code, leveraging Rust's memory safety guarantees
- **Request-Based API**: Simplified control flow replacing callback-style API
- **P2P Rollback Networking**: Implements rollback netcode for real-time multiplayer games
- **Multiple Session Types**: P2P sessions, spectator sessions, and sync test sessions
- **Input Prediction & Rollback**: Handles network latency through prediction and state rollback
- **Desync Detection**: Checksum-based verification between peers

## Repository Structure

```
ggrs/
├── src/
│   ├── lib.rs                    # Main library entry point
│   ├── error.rs                  # Error types and handling
│   ├── frame_info.rs            # Frame metadata and tracking
│   ├── input_queue.rs           # Input buffering and management
│   ├── sync_layer.rs            # Core synchronization logic
│   ├── time_sync.rs             # Time synchronization between peers
│   ├── network/
│   │   ├── compression.rs       # Network message compression
│   │   ├── messages.rs          # Network protocol messages
│   │   ├── network_stats.rs     # Network statistics tracking
│   │   ├── protocol.rs          # Network protocol implementation
│   │   └── udp_socket.rs        # UDP socket abstraction
│   └── sessions/
│       ├── builder.rs           # Session builder pattern
│       ├── p2p_session.rs       # Peer-to-peer session
│       ├── p2p_spectator_session.rs  # Spectator session
│       └── sync_test_session.rs # Determinism testing session
├── examples/                     # Example implementations
├── tests/                        # Integration tests
└── Cargo.toml                    # Package manifest
```

## Core Concepts

### Session Types
1. **P2PSession**: Standard peer-to-peer gameplay session
2. **SpectatorSession**: Session for spectators who observe but don't participate
3. **SyncTestSession**: Determinism verification session for testing

### Player Types
- **Local**: Player on the current device
- **Remote**: Player on a remote device (identified by socket address)
- **Spectator**: Remote observer (doesn't contribute input)

### Frame Management
- Frames are the fundamental unit of game state progression
- Input prediction allows the game to advance before receiving remote inputs
- Rollback mechanism restores previous states when predictions are incorrect

## Fork Goals & Objectives

This fork aims to elevate GGRS to production-grade quality through rigorous verification, testing, and usability improvements.

### Primary Goals

#### 1. Extensive Test Coverage (>90%)
- **Unit Tests**: Cover all public APIs and internal logic
- **Integration Tests**: Test session lifecycle, network scenarios, edge cases
- **Property-Based Tests**: Use fuzzing and property testing for robustness
- **Determinism Tests**: Verify consistent behavior across platforms
- **Performance Tests**: Benchmark critical paths and identify regressions

#### 2. Formal Verification
- **TLA+ Specifications**: Model concurrent behavior, synchronization protocols, and state machines
- **Z3 SMT Solver**: Verify safety properties, invariants, and correctness conditions
- **Proof Coverage**: Focus on critical sections: input synchronization, rollback logic, network protocol
- **Verification Targets**:
  - State transition correctness
  - Input buffer consistency
  - Frame synchronization guarantees
  - No deadlocks or race conditions
  - Network message ordering and delivery properties

**Formal Verification Philosophy:**
- **Specs model production** - TLA+/Kani/Z3 specs must accurately represent production code behavior
- **When verification fails, assume production has a bug first** - investigate before relaxing specs
- **Never "fix" specs just to make them pass** - this defeats the purpose of verification
- **Document all spec changes** - explain what production behavior necessitates the change
- **Invariants represent real safety properties** - only relax with strong justification

**When Formal Verification or Analysis Reveals Issues:**
After fixing any bug discovered through formal verification, code review, or other analysis, add comprehensive test coverage:
1. **Direct reproduction test** - Cover the exact scenario that was discovered
2. **Edge case variants** - Zero values, max values, boundary conditions
3. **Chained operations** - Multiple sequential calls that might compound issues
4. **Full lifecycle tests** - Create-use-modify-destroy cycles  
5. **Invariant preservation** - Verify invariants hold across all state transitions
6. **Negative tests** - Ensure violations are properly detected
7. **Document in tests** - Explain what was discovered and why the test matters

#### 3. Enhanced Usability
- **Clear API Design**: Intuitive, hard-to-misuse interfaces
- **Comprehensive Documentation**: Detailed explanations, examples, and guides
- **Better Error Messages**: Actionable, context-rich error reporting
- **Type Safety**: Leverage Rust's type system to prevent misuse at compile time
- **Builder Patterns**: Fluent APIs for session configuration
- **Sensible Defaults**: Minimize required configuration for common use cases

#### 4. Simplicity & Understandability
- **Code Clarity**: Prioritize readability and maintainability
- **Architectural Documentation**: Clear explanations of design decisions
- **Inline Comments**: Document non-obvious logic and algorithms
- **Example-Driven**: Comprehensive examples demonstrating best practices
- **Minimal Abstractions**: Only introduce complexity when necessary

## Development Guidelines

### Breaking Changes Policy
- **API compatibility is NOT required** - Breaking the public API is acceptable if it provides a safer or more ergonomic experience
- **Safety and correctness trump compatibility** - If a breaking change improves safety, determinism, or prevents misuse, make it
- **Document all breaking changes** - Update CHANGELOG.md and MIGRATION.md when APIs change
- **This fork prioritizes production-grade quality** - We are not a drop-in replacement for upstream GGRS

### Testing Standards
- All new features must include tests
- Aim for >90% code coverage
- Include both positive and negative test cases
- Test edge cases and error conditions
- Use integration tests for cross-component behavior

### Formal Methods
- Document protocol invariants and safety properties
- Create TLA+ models for concurrent components
- Use Z3 for verifying algorithmic correctness
- Maintain verification artifacts alongside code

### Code Quality
- Follow Rust best practices and idioms
- Use `#![forbid(unsafe_code)]` - maintain 100% safe Rust
- Enable and satisfy clippy lints (all, pedantic, nursery)
- Ensure all documentation is accurate and up-to-date
- No broken intra-doc links

### Documentation Requirements
- Public APIs must have rustdoc comments with examples
- Complex algorithms require detailed explanations
- Include diagrams for architectural concepts
- Maintain CHANGELOG.md with all changes
- Update examples when APIs change

### Performance Considerations
- Profile and benchmark performance-critical code
- Optimize for common case, not edge cases
- Document performance characteristics (Big-O complexity)
- Avoid unnecessary allocations in hot paths
- Use appropriate data structures for access patterns

### Error Handling
- Use `Result<T, FortressError>` for fallible operations
- Provide context-rich error messages
- Include recovery suggestions where applicable
- Document error conditions in rustdoc
- Never panic in library code (prefer Result)

### Defensive Programming Patterns
Apply these patterns from [corrode.dev/blog/defensive-programming](https://corrode.dev/blog/defensive-programming/) for safer, more robust code:

#### Prefer Slice Pattern Matching Over Indexing
```rust
// ❌ Avoid: Decoupled length check and indexing can panic
if !users.is_empty() {
    let first = &users[0];
}

// ✅ Prefer: Compiler-enforced safe access
match users.as_slice() {
    [] => handle_empty(),
    [single] => handle_one(single),
    [first, rest @ ..] => handle_multiple(first, rest),
}
```

#### Explicit Field Initialization Over Default Spread
```rust
// ❌ Avoid: New fields silently use defaults
let config = Config { field1: value1, ..Default::default() };

// ✅ Prefer: Compiler forces handling new fields
let config = Config { field1: value1, field2: value2, field3: value3 };

// ✅ Alternative: Destructure default for explicit handling
let Config { field1, field2, field3 } = Config::default();
let config = Config { field1: custom_value, field2, field3 };
```

#### Destructuring in Trait Implementations
```rust
// ❌ Avoid: New fields won't be included in comparison
impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.price == other.price
    }
}

// ✅ Prefer: Compiler error when fields are added
impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        let Self { size, price, timestamp: _ } = self;
        let Self { size: other_size, price: other_price, timestamp: _ } = other;
        size == other_size && price == other_price
    }
}
```

#### Use TryFrom for Fallible Conversions
```rust
// ❌ Avoid: From that can fail hides errors
impl From<RawData> for ProcessedData {
    fn from(raw: RawData) -> Self {
        Self { value: raw.value.unwrap_or_default() }
    }
}

// ✅ Prefer: TryFrom makes fallibility explicit
impl TryFrom<RawData> for ProcessedData {
    type Error = ConversionError;
    fn try_from(raw: RawData) -> Result<Self, Self::Error> {
        Ok(Self { value: raw.value.ok_or(ConversionError::MissingValue)? })
    }
}
```

#### Exhaustive Match Arms
```rust
// ❌ Avoid: Wildcard hides unhandled variants
match state {
    State::Ready => handle_ready(),
    State::Running => handle_running(),
    _ => {}  // New variants silently fall through
}

// ✅ Prefer: Explicit variants catch additions
match state {
    State::Ready => handle_ready(),
    State::Running => handle_running(),
    State::Paused | State::Stopped => {}
}
```

#### Named Placeholders in Patterns
```rust
// ❌ Avoid: Unclear which fields are ignored
match event { Event::Input { _, _, .. } => {} }

// ✅ Prefer: Self-documenting ignored fields
match event { Event::Input { player: _, frame: _, data } => use_data(data) }
```

#### Temporary Mutability Pattern
```rust
// ✅ Confine mutability to initialization scope
let data = {
    let mut data = get_initial_data();
    data.sort();
    data.dedup();
    data  // Return immutable
};
```

#### Defensive Constructor Patterns (for libraries)
```rust
// Prevent bypassing validation with private field
pub struct ValidatedConfig {
    pub value: u32,
    _private: (),  // Prevents direct construction
}

impl ValidatedConfig {
    pub fn new(value: u32) -> Result<Self, ConfigError> {
        if value == 0 { return Err(ConfigError::ZeroNotAllowed); }
        Ok(Self { value, _private: () })
    }
}

// Alternative: #[non_exhaustive] for cross-crate protection
#[non_exhaustive]
pub struct Config { pub value: u32 }
```

#### Mark Important Types with #[must_use]
```rust
#[must_use = "Configuration must be applied to take effect"]
pub struct SessionConfig { /* ... */ }
```

#### Enums Over Booleans for Clarity
```rust
// ❌ Avoid: process_data(&data, true, false, true);
// ✅ Prefer: process_data(&data, Compression::Enabled, Encryption::Disabled, Validation::Strict);
```

#### Recommended Clippy Lints
```toml
[lints.clippy]
indexing_slicing = "warn"        # Prefer .get() or pattern matching
fallible_impl_from = "deny"      # Use TryFrom for fallible conversions
wildcard_enum_match_arm = "warn" # Prefer explicit match arms
must_use_candidate = "warn"      # Add #[must_use] where appropriate
```

## Key Technical Details

### Determinism
GGRS requires games to be deterministic - same inputs must produce same outputs. This is critical for rollback to work correctly.

### Serialization
Game state must be serializable (implement `Serialize` + `DeserializeOwned`) for save/load during rollback.

### Checksums
Periodic checksum verification detects desyncs between peers, with configurable intervals.

### Input Delay
Configurable input delay provides a buffer against network jitter, trading latency for stability.

## Testing Strategy

### Root Cause Analysis for Test Failures
**CRITICAL: When tests fail or are flaky, always perform deep investigation**

The goal is NOT to "make the test pass" — it's to understand and fix the underlying issue, whether in production code or the test itself.

#### Investigation Methodology
1. **Reproduce and characterize** - Run test multiple times; is it consistent or flaky? Under what conditions does it fail?
2. **Understand the assertion** - What property is the test verifying? Why should this property hold?
3. **Trace execution** - Add logging, use debugger, examine state at failure point
4. **Form hypothesis** - What could cause this specific failure mode?
5. **Verify hypothesis** - Confirm understanding before implementing fix
6. **Consider scope** - Are there similar issues elsewhere in the codebase?

#### Determining Fix Location
- **Production bug indicators**: 
  - Test expectations align with documented/intended behavior
  - Multiple tests or users depend on the same behavior
  - The test logic is simple and clearly correct
- **Test bug indicators**:
  - Test makes assumptions not guaranteed by the API contract
  - Test has inherent race conditions or timing dependencies  
  - Test uses mocking incorrectly or has setup errors
  - Test expectations contradict documentation

#### Comprehensive Fix Requirements
1. **Fix at the correct level**:
   - Production bug → Fix library code, add regression test if needed
   - Test bug → Fix test assumptions, document why original assumption was wrong
   - Timing issue → Use proper synchronization (channels, events, barriers) not sleeps
   - Flakiness → Eliminate non-determinism at its source
2. **Assess impact**: Does this fix affect other components? Run full test suite
3. **Add protection**: If production bug, ensure test coverage prevents regression
4. **Update docs**: If behavior was ambiguous, clarify documentation
5. **Document the fix**: Explain root cause and why the solution is correct

#### Strictly Forbidden "Fixes"
- ❌ Commenting out or weakening failing assertions
- ❌ Adding `Thread::sleep()` or arbitrary delays
- ❌ Catching and ignoring/swallowing errors in tests
- ❌ Marking tests as `#[ignore]` without a fix plan
- ❌ Relaxing numeric tolerances without justification
- ❌ Changing expected values to match actual without understanding why
- ❌ Disabling test features that exist in production code

### Unit Tests
- Test individual functions and methods in isolation
- Mock network interactions and time dependencies
- Verify error conditions and edge cases

### Integration Tests
- Test complete session lifecycles
- Simulate network conditions (latency, packet loss, reordering)
- Verify synchronization across multiple peers
- Test rollback and state restoration

### Property-Based Tests
- Use `proptest` or `quickcheck` for fuzz testing
- Verify invariants hold under random inputs
- Test serialization round-trips
- Ensure deterministic behavior

### Verification Tests
- Automated checks against TLA+ models
- Z3 constraint verification
- Invariant checking at runtime (in debug builds)

## Contributing to This Fork

When contributing, ensure:
1. After every major change, run `cargo fmt`, `cargo clippy --all-targets`, and `cargo test`, and fix all resulting issues
   - Note: Use `--features z3-verification` only when working on Z3 tests (compiles Z3 from source, which is slow)
2. Tests accompany all changes
3. Documentation is updated
4. Code passes all lints and checks
5. Performance impact is measured
6. Formal verification is maintained/extended where applicable
7. Examples demonstrate new features
8. CHANGELOG is updated

## Related Resources

- [Original GGPO SDK](https://www.ggpo.net/)
- [GGPO Developers Discord](https://discord.com/invite/8FKKhCRCCE)
- [Bevy GGRS Plugin](https://github.com/gschup/bevy_ggrs) (compatible, points to original GGRS lineage)
- [TLA+ Resources](https://lamport.azurewebsites.net/tla/tla.html)
- [Z3 Theorem Prover](https://github.com/Z3Prover/z3)

## License

Dual-licensed under MIT OR Apache-2.0
