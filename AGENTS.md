# OpenAI AGENTS Instructions for GGRS

## Overview

**Context File**: Refer to [`.llm/context.md`](.llm/context.md) for comprehensive project documentation.

GGRS is a Rust implementation of GGPO rollback networking. This fork focuses on:
- **>90% test coverage**
- **Formal verification** (TLA+, Z3)
- **Enhanced usability** (intuitive APIs, clear errors)
- **Code clarity** (simple, maintainable)

## Core Requirements

### Safety First
- 100% safe Rust code (`#![forbid(unsafe_code)]`)
- No panics in library code (use `Result`)
- Comprehensive error handling
- Type safety over runtime checks

### Test-Driven
- Write tests for all new code
- Cover edge cases and error paths
- Include property-based tests for complex logic
- Verify determinism across platforms
- Target >90% code coverage

### Well-Documented
- Rustdoc comments with examples for all public APIs
- Explain non-obvious design decisions
- Document invariants and safety properties
- Keep examples synchronized with code
- Maintain CHANGELOG.md

### Formally Verified
- Model critical components in TLA+
- Verify algorithms with Z3
- Document verification artifacts
- Prove safety properties for concurrent code

**Formal Verification Philosophy:**
- **Specs model production** - TLA+/Kani/Z3 specs must accurately represent production code
- **When verification fails, assume production has a bug first** - investigate before relaxing specs
- **Never "fix" specs just to make them pass** - this defeats the purpose of verification
- **Document all spec changes** - explain what production behavior necessitates the change
- **Invariants represent real safety properties** - only relax with strong justification
- **Review spec changes carefully** - ensure they don't hide real bugs

**When Formal Verification or Analysis Reveals Issues:**
After fixing any bug discovered through formal verification, code review, or other analysis:
1. **Add comprehensive regression tests** - Cover the exact scenario that was discovered
2. **Test related edge cases** - Look for similar issues in related code paths
3. **Test boundary conditions** - Add tests at the edges of valid ranges (zero, max, off-by-one)
4. **Test invariant preservation** - Ensure invariants hold across all state transitions
5. **Test full cycles** - Create-use-modify-destroy lifecycle tests
6. **Add negative tests** - Verify that violations are properly detected
7. **Document the discovery** - Tests should explain what was found and why it matters

```rust
// Example: After discovering load_frame() didn't update last_saved_frame
// Add these categories of tests:

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

## Code Standards

### Function Documentation Template
```rust
/// Brief one-line description.
///
/// More detailed explanation if needed, covering:
/// - What the function does
/// - When to use it
/// - Important considerations
///
/// # Arguments
/// * `param1` - Description
/// * `param2` - Description
///
/// # Errors
/// * `ErrorVariant1` - When this occurs
/// * `ErrorVariant2` - When this occurs
///
/// # Examples
/// ```
/// # use ggrs::*;
/// let result = function(arg1, arg2)?;
/// assert_eq!(result, expected);
/// ```
pub fn function(param1: Type1, param2: Type2) -> Result<ReturnType, FortressError> {
    // Implementation
}
```

### Test Structure Template
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

## Common Tasks

### Adding a New Feature
1. Write tests first (TDD)
2. Implement feature with full documentation
3. Update examples if API surface changes
4. Consider formal verification needs
5. Update CHANGELOG.md
6. Ensure all tests pass

### Fixing a Bug
1. Write failing test that reproduces the bug
2. **Root cause analysis** - Understand *why* it fails before fixing
3. Fix the implementation (not the test, unless the test is wrong)
4. Verify test passes
5. Check for similar issues elsewhere
6. Document the fix in CHANGELOG.md

### When Tests Fail or Are Flaky
**CRITICAL: Always perform deep Root Cause Analysis (RCA)**

The goal is never just to "make tests pass" — it's to understand and comprehensively fix the underlying issue.

#### Investigation Steps
1. **Reproduce the failure** - Run multiple times, note patterns and conditions
2. **Understand the test's intent** - What invariant or behavior is being verified?
3. **Trace the failure** - Add logging, examine state, understand the exact failure mode
4. **Identify root cause** - Keep asking "why" until you reach the fundamental issue
5. **Verify hypothesis** - Confirm your understanding before coding a fix
6. **Check for similar issues** - Are there related problems elsewhere?

#### Determining Where to Fix
- **Production bug indicators**: Test expectations match docs, logic is straightforward, other code depends on this behavior
- **Test bug indicators**: Test makes unguaranteed assumptions, has race conditions, or contradicts documentation

#### Comprehensive Fix Approach
- **Production bug** → Fix library code, verify other tests still pass, add regression test if missing
- **Test bug** → Fix test's incorrect assumptions, document why original was wrong
- **Timing issue** → Add proper synchronization primitives (channels, barriers, events) — NOT arbitrary sleeps
- **Flakiness** → Find and eliminate the source of non-determinism completely
- **Document everything** → Explain root cause and why the fix is correct

#### Strictly Forbidden "Fixes"
- ❌ Commenting out or weakening failing assertions
- ❌ Adding `Thread::sleep()` or arbitrary delays to "fix" timing
- ❌ Catching and ignoring errors in test code
- ❌ Marking tests as `#[ignore]` without documented fix plan
- ❌ Relaxing tolerances/thresholds without understanding why original was set
- ❌ Changing expected values to match actual without root cause analysis
- ❌ Removing test functionality that exists in production

### Improving Performance
1. Benchmark current performance
2. Identify bottleneck
3. Implement optimization
4. Verify correctness (tests still pass)
5. Benchmark improvement
6. Document performance characteristics

## Key Architecture Components

### Sessions
- **P2PSession**: Peer-to-peer gameplay
- **SpectatorSession**: Observation mode
- **SyncTestSession**: Determinism verification

### Network Layer
- Message protocol (compression, serialization)
- UDP socket abstraction
- Network statistics tracking
- Desync detection

### Synchronization
- Input queue management
- Frame synchronization
- Rollback state management
- Time synchronization between peers

### Error Handling
- `FortressError` enum with variants for all error cases
- `Result<T, FortressError>` return types
- Context-rich error messages
- Recovery suggestions where applicable

## Best Practices

### Code Quality
- Use clippy and fix all warnings
- Follow Rust naming conventions
- Keep functions focused (single responsibility)
- Prefer composition over complexity
- Make invalid states unrepresentable

### Testing
- Test public behavior, not implementation
- Use descriptive test names
- Avoid test interdependencies
- Make tests deterministic (no randomness/timing)
- Test error conditions explicitly

### Documentation
- Write docs for humans, not machines
- Include practical examples
- Explain the "why" behind decisions
- Link related concepts
- Keep docs synchronized with code

### Performance
- Profile before optimizing
- Document algorithmic complexity
- Avoid premature optimization
- Consider memory allocation patterns
- Benchmark critical paths

## Verification Approach

### TLA+ Models
Use for:
- State machine modeling
- Concurrency correctness
- Protocol verification
- Liveness properties

### Z3 Constraints
Use for:
- Algorithm correctness
- Invariant checking
- Safety properties
- Boundary conditions

### Testing Strategies
- Unit tests for isolated behavior
- Integration tests for component interaction
- Property-based tests for invariants
- Determinism tests for consistency
- Performance tests for regressions

## Project Navigation

**Source Code**
- `src/lib.rs` - Public API
- `src/sessions/` - Session implementations
- `src/network/` - Network protocol
- `src/sync_layer.rs` - Core sync logic
- `src/input_queue.rs` - Input management

**Tests & Examples**
- `tests/` - Integration tests
- `examples/ex_game/` - Working examples

**Documentation**
- `README.md` - Project overview
- `CONTRIBUTING.md` - Contribution guide
- `CHANGELOG.md` - Version history
- `.llm/context.md` - Complete context

## Quick Reference

### Player Types
```rust
PlayerType::Local              // Local player
PlayerType::Remote(addr)       // Remote player
PlayerType::Spectator(addr)    // Spectator (no input)
```

### Session Builder
```rust
SessionBuilder::new()
    .with_num_players(n)
    .with_input_delay(frames)
    .add_player(player_type, handle)?
    .start_p2p_session()?
```

### Request Pattern
```rust
for request in session.events() {
    match request {
        GgrsRequest::SaveGameState { frame, cell } => { /* ... */ }
        GgrsRequest::LoadGameState { frame, cell } => { /* ... */ }
        GgrsRequest::AdvanceFrame { inputs } => { /* ... */ }
    }
}
```

## Resources

- Full context: `.llm/context.md`
- Contributing: `CONTRIBUTING.md`
- Changelog: `CHANGELOG.md`
- Original GGPO: https://www.ggpo.net/
- Rust docs: https://docs.rs/ggrs/
- TLA+: https://lamport.azurewebsites.net/tla/tla.html
- Z3: https://github.com/Z3Prover/z3

## Quality Gates

Before suggesting code, verify:
- ✅ After every major change, run `cargo fmt`, `cargo clippy --all-targets --all-features`, and `cargo test`; fix all resulting issues before proceeding
- ✅ Compiles without warnings
- ✅ All tests pass
- ✅ Documentation is complete
- ✅ Error handling is comprehensive
- ✅ 100% safe Rust
- ✅ Performance is acceptable
- ✅ Examples work correctly
- ✅ CHANGELOG is updated
