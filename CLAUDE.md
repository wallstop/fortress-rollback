# Claude Instructions for GGRS

> **Important**: Read [`.llm/context.md`](.llm/context.md) for complete project context, architecture, and development guidelines.

## Project Summary

GGRS is a Rust implementation of GGPO rollback networking for multiplayer games. This fork emphasizes verification, correctness, and usability through:

1. **>90% test coverage** with comprehensive test suites
2. **Formal verification** using TLA+ and Z3
3. **Enhanced usability** with intuitive, type-safe APIs
4. **Code clarity** for easy understanding and maintenance

## When Assisting

### General Approach
- Prioritize correctness and safety over cleverness
- Write self-documenting code with clear intent
- Include tests with every code change
- Think about edge cases and error conditions
- Consider formal verification for critical logic

### Breaking Changes Policy
- **API compatibility is NOT required** - Breaking the public API is acceptable if it provides a safer or more ergonomic experience
- **Safety and correctness trump compatibility** - If a breaking change improves safety, determinism, or prevents misuse, make it
- **Document all breaking changes** - Update CHANGELOG.md and MIGRATION.md when APIs change
- **This fork prioritizes production-grade quality** - We are not a drop-in replacement for upstream GGRS

### Code Style
- Use 100% safe Rust (`#![forbid(unsafe_code)]`)
- Follow Rust idioms and conventions
- Write descriptive variable and function names
- Keep functions focused and single-purpose
- Prefer explicit over implicit

### Testing Philosophy
- Test behavior, not implementation details
- Include both happy path and error cases
- Write deterministic tests (no flakiness)
- Use property-based testing for complex logic
- Ensure tests are maintainable and readable

### Root Cause Analysis for Test Failures
**CRITICAL: When tests fail or are flaky, perform deep investigation and provide comprehensive fixes**

The goal is NOT to "make the test pass" — it's to understand and fix the underlying issue properly.

#### Investigation Methodology
1. **Reproduce and characterize** - Run test multiple times; is failure consistent or intermittent? What are the failure conditions?
2. **Understand the assertion** - What property or invariant is the test verifying? Why should it hold?
3. **Trace the failure path** - Add logging/debugging, examine state at point of failure
4. **Root cause analysis** - Keep asking "why" until you identify the fundamental issue
5. **Verify hypothesis** - Confirm your understanding before implementing any fix
6. **Assess scope** - Are there similar issues elsewhere in the codebase?

#### Distinguishing Test Bug vs Production Bug
- **Production bug indicators**:
   - Test expectations align with documented behavior
   - Test logic is straightforward and clearly correct
   - Other tests or users depend on the same behavior
- **Test bug indicators**:
   - Test makes assumptions not guaranteed by the API
   - Test has inherent race conditions or timing dependencies
   - Test expectations contradict documentation
   - Test setup is incorrect or incomplete

#### Providing Comprehensive Fixes
1. **Fix at the correct level**:
   - Production bug → Fix library code AND verify other tests still pass AND add regression test if missing
   - Test bug → Fix test's incorrect assumptions AND document why original assumption was wrong
   - Timing issue → Add proper synchronization (channels, barriers, condvars) NOT arbitrary sleeps
   - Flakiness → Find and eliminate non-determinism at its source
2. **Consider ripple effects** - Does this fix impact other components? Run full test suite
3. **Add regression protection** - If production bug, ensure test coverage prevents future regression
4. **Update documentation** - If behavior was unclear, clarify in docs/comments
5. **Document the fix** - Explain root cause and why the solution is correct

#### Strictly Forbidden "Fixes"
- ❌ Commenting out or weakening failing assertions
- ❌ Adding `Thread::sleep()` or arbitrary delays to "fix" timing
- ❌ Catching and ignoring errors in test code  
- ❌ Marking tests as `#[ignore]` without documented plan to fix
- ❌ Relaxing numeric tolerances without understanding why original was appropriate
- ❌ Changing expected values to match actual without root cause analysis
- ❌ Disabling test features that exist in production code

### When Formal Verification or Analysis Reveals Issues
**CRITICAL: After fixing any discovered bug, add comprehensive test coverage**

After fixing a bug found through formal verification (TLA+, Kani, Z3), code review, or analysis:
1. **Direct reproduction test** - Test the exact scenario that was discovered
2. **Edge case variants** - Zero values, max values, boundary conditions
3. **Chained operations** - Multiple sequential calls that might compound issues
4. **Full lifecycle tests** - Create-use-modify-destroy cycles
5. **Invariant preservation** - Verify invariants hold across state transitions
6. **Negative tests** - Ensure violations are properly detected
7. **Document in tests** - Explain what was discovered and why the test matters

```rust
// After discovering a bug, add comprehensive coverage like:
#[test]
fn test_exact_discovered_scenario() { /* Direct reproduction */ }

#[test]
fn test_edge_case_with_zero() { /* Boundary */ }

#[test]
fn test_edge_case_with_max() { /* Boundary */ }

#[test]
fn test_chained_operations_maintain_invariants() { /* Sequential */ }

#[test]
fn test_full_lifecycle_maintains_invariants() { /* Complete cycle */ }

#[test]
fn test_violation_detection() { /* Negative test */ }
```

### Documentation
- Every public item needs rustdoc with examples
- Explain the "why" behind non-obvious decisions
- Document invariants, preconditions, postconditions
- Keep examples up-to-date with API changes
- Link related concepts and components

## Code Examples

### Good Function Design
```rust
/// Adds local input for the current frame.
///
/// This must be called before `advance_frame()` for each local player.
/// The input will be buffered and synchronized with remote inputs.
///
/// # Arguments
/// * `player_handle` - The handle of the local player
/// * `input` - The input data for this frame
///
/// # Errors
/// * `InvalidPlayer` - If player_handle doesn't correspond to a local player
/// * `InvalidInput` - If input size doesn't match configured size
///
/// # Examples
/// ```
/// # use ggrs::{P2PSession, PlayerType};
/// # let mut session = P2PSession::new(2, 8)?;
/// let player = 0;
/// let input = 0b1010_0000u8; // Button states
/// session.add_local_input(player, input)?;
/// session.advance_frame()?;
/// ```
pub fn add_local_input(
    &mut self,
    player_handle: PlayerHandle,
    input: u64,
) -> Result<(), FortressError> {
    // Implementation
}
```

### Good Test Structure
```rust
#[test]
fn rollback_restores_state_correctly() {
    // Arrange: Set up the scenario
    let mut session = create_test_session(2, 8);
    let initial_state = GameState::new();
    session.load_game_state(&initial_state);
    
    // Act: Perform the operation
    session.advance_frame()?;
    session.advance_frame()?;
    let frame_2_state = session.current_state();
    
    // Simulate receiving corrected input for frame 1
    session.rollback_to(1)?;
    
    // Assert: Verify the outcome
    assert_eq!(session.current_frame(), 1);
    assert_ne!(session.current_state(), frame_2_state);
}
```

## Defensive Programming Patterns

Apply these patterns from [corrode.dev/blog/defensive-programming](https://corrode.dev/blog/defensive-programming/) for safer, more robust code:

### Prefer Slice Pattern Matching Over Indexing
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

### Explicit Field Initialization Over Default Spread
```rust
// ❌ Avoid: New fields silently use defaults
let config = Config {
    field1: value1,
    ..Default::default()
};

// ✅ Prefer: Compiler forces handling new fields
let config = Config {
    field1: value1,
    field2: value2,
    field3: value3,
};

// ✅ Alternative: Destructure default for explicit handling
let Config { field1, field2, field3 } = Config::default();
let config = Config {
    field1: custom_value,  // Override
    field2,                // Use default (explicit)
    field3,                // Use default (explicit)
};
```

### Destructuring in Trait Implementations
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

### Use TryFrom for Fallible Conversions
```rust
// ❌ Avoid: From that can fail hides errors
impl From<RawData> for ProcessedData {
    fn from(raw: RawData) -> Self {
        Self { value: raw.value.unwrap_or_default() }  // Hidden fallibility
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

### Exhaustive Match Arms
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
    State::Paused | State::Stopped => {}  // Explicitly grouped
}
```

### Named Placeholders in Patterns
```rust
// ❌ Avoid: Unclear which fields are ignored
match event {
    Event::Input { _, _, .. } => {}
}

// ✅ Prefer: Self-documenting ignored fields
match event {
    Event::Input { player: _, frame: _, data } => use_data(data),
}
```

### Temporary Mutability Pattern
```rust
// ✅ Confine mutability to initialization scope
let data = {
    let mut data = get_initial_data();
    data.sort();
    data.dedup();
    data  // Return immutable
};
// `data` is now immutable, temps don't leak
```

### Defensive Constructor Patterns
```rust
// For libraries: Prevent bypassing validation
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

// Alternative: Use #[non_exhaustive] for cross-crate protection
#[non_exhaustive]
pub struct Config {
    pub value: u32,
}
```

### Mark Important Types with #[must_use]
```rust
#[must_use = "Configuration must be applied to take effect"]
pub struct SessionConfig { /* ... */ }

#[must_use = "Errors must be handled"]
pub enum SessionError { /* ... */ }
```

### Enums Over Booleans for Clarity
```rust
// ❌ Avoid: Unclear at call site
process_data(&data, true, false, true);

// ✅ Prefer: Self-documenting call sites
process_data(&data, Compression::Enabled, Encryption::Disabled, Validation::Strict);
```

### Recommended Clippy Lints
Add to `Cargo.toml` for defensive programming enforcement:
```toml
[lints.clippy]
indexing_slicing = "warn"        # Prefer .get() or pattern matching
fallible_impl_from = "deny"      # Use TryFrom for fallible conversions
wildcard_enum_match_arm = "warn" # Prefer explicit match arms
must_use_candidate = "warn"      # Add #[must_use] where appropriate
```

## Verification Guidance

### When to Suggest Formal Methods
- Concurrent state management
- Network protocol correctness
- Input synchronization guarantees
- Safety-critical rollback logic
- Frame ordering and consistency

### TLA+ Use Cases
- Model session state machines
- Verify synchronization protocols
- Check for race conditions
- Prove liveness properties

### Z3 Use Cases
- Verify algorithmic invariants
- Check frame calculation correctness
- Validate input buffer logic
- Prove safety properties

## Common Patterns in GGRS

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

### Error Propagation
```rust
// Use ? for clean error propagation
session.add_local_input(player, input)?;
session.advance_frame()?;

// Provide context for errors
let result = session.advance_frame()
    .map_err(|e| format!("Failed to advance frame: {}", e))?;
```

## Project Structure Context

- `src/lib.rs` - Public API and core types
- `src/sessions/` - Session implementations (P2P, Spectator, SyncTest)
- `src/network/` - Network protocol and communication
- `src/sync_layer.rs` - Core synchronization logic
- `src/input_queue.rs` - Input buffering and management
- `examples/ex_game/` - Example implementations
- `tests/` - Integration tests

## Key Concepts to Understand

1. **Frames**: Discrete time steps in game simulation
2. **Rollback**: Restoring previous state when predictions are wrong
3. **Input Delay**: Buffer frames to smooth over network jitter
4. **Prediction**: Continuing simulation before inputs arrive
5. **Desync Detection**: Checksum verification between peers
6. **Determinism**: Same inputs → same outputs (critical requirement)

## Assistance Checklist

When providing code or suggestions:
- ✅ Includes appropriate tests
- ✅ Has complete rustdoc comments
- ✅ Handles all error cases
- ✅ Maintains 100% safe Rust
- ✅ Follows project code style
- ✅ Updates examples if API changes
- ✅ Considers performance impact
- ✅ Documents any invariants
- ✅ Works toward coverage goals

## Additional Resources

- Complete context: `.llm/context.md`
- Contributing: `CONTRIBUTING.md`
- Changelog: `CHANGELOG.md`
- Original GGPO: https://www.ggpo.net/
- Rust docs: https://docs.rs/ggrs/
