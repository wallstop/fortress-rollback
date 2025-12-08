# GitHub Copilot Instructions for GGRS

> **Note**: For comprehensive project context, architecture details, and development guidelines, refer to [`.llm/context.md`](../.llm/context.md)

## Quick Reference

This is a **fork** of GGRS (Good Game Rollback System) focused on:
- **>90% test coverage** - All code must be thoroughly tested
- **Formal verification** - TLA+ and Z3 verification of critical components
- **Enhanced usability** - Simple, intuitive, hard-to-misuse APIs
- **Code clarity** - Easy to understand and maintain

## Core Principles When Assisting

### 1. Test-First Development
- Always include tests with new features
- Suggest test cases for edge conditions
- Help write property-based tests for complex logic
- Ensure determinism tests for game-critical code

### 2. Safe & Verified Code
- Maintain `#![forbid(unsafe_code)]` - suggest only safe Rust
- Consider formal verification when suggesting algorithms
- Document invariants and preconditions
- Suggest Z3 constraints for critical safety properties

**Formal Verification Philosophy:**
- **Specs model production** - TLA+/Kani/Z3 specs must accurately represent production code
- **When verification fails, assume production has a bug first** - investigate before relaxing specs
- **Never "fix" specs just to make them pass** - this defeats the purpose of verification
- **Document all spec changes** - explain what production behavior necessitates the change
- **Invariants represent real safety properties** - only relax with strong justification

**When Formal Verification or Analysis Reveals Issues:**
After fixing any bug discovered through formal verification, code review, or other analysis:
1. **Add comprehensive regression tests** - Cover the exact scenario that was discovered
2. **Test related edge cases** - Look for similar issues in related code paths
3. **Test boundary conditions** - Add tests at the edges of valid ranges
4. **Test invariant preservation** - Ensure invariants hold across state transitions
5. **Document the scenario** - Tests should explain what was discovered and why it matters

Example test categories to add after discovering a bug:
- Direct reproduction test for the discovered issue
- Edge case variants (zero values, max values, boundary conditions)
- Chained/sequential operations that might compound the issue
- Full lifecycle tests (create-use-modify-destroy cycles)
- Negative tests that verify detection of violations

### 3. Clear & Usable APIs
- Favor builder patterns for complex configuration
- Suggest type-safe interfaces over runtime validation
- Provide clear, actionable error messages
- Include usage examples in rustdoc comments

### 4. Documentation Standards
- All public items need rustdoc with examples
- Explain "why" not just "what" in comments
- Link to relevant formal specifications when applicable
- Keep CHANGELOG.md updated

## Code Generation Guidelines

### When Writing Code
```rust
// ✅ DO: Clear function signatures with documentation
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

// ❌ DON'T: Undocumented public APIs
pub fn advance_frame(&mut self) -> Result<(), FortressError> {
    // Implementation
}
```

### When Writing Tests
```rust
// ✅ DO: Descriptive test names and comprehensive coverage
#[test]
fn test_rollback_restores_correct_state_after_input_correction() {
    // Arrange
    let mut session = create_test_session();
    
    // Act
    session.add_local_input(player, input)?;
    let state_before = session.current_state();
    session.advance_frame()?;
    // ... rollback scenario
    
    // Assert
    assert_eq!(session.current_state(), expected_state);
}

// ❌ DON'T: Vague test names without clear intent
#[test]
fn test_rollback() {
    // Minimal test
}
```

### When Tests Fail or Are Flaky
**CRITICAL: Deep Root Cause Analysis Required**

When encountering a failing or flaky test, always perform thorough investigation:

#### Investigation Process
1. **Reproduce consistently** - Run the test multiple times, note any patterns
2. **Understand the assertion** - What is the test actually checking? What invariant should hold?
3. **Trace the failure path** - Add logging/debugging to understand the exact state at failure
4. **Identify the root cause** - Keep asking "why" until you reach the fundamental issue
5. **Verify your hypothesis** - Confirm your understanding before implementing a fix

#### Distinguishing Test Bug vs Production Bug
- **Production bug indicators**: Test expectations match documented behavior, other tests rely on same behavior, the test logic is straightforward
- **Test bug indicators**: Test makes assumptions not guaranteed by API, test has race conditions, test uses outdated API expectations

#### Providing Comprehensive Fixes
1. **Fix at the correct level**:
   - **Production bug**: Fix the library code AND verify fix doesn't break other tests
   - **Test bug**: Fix test's incorrect assumptions AND document why assumption was wrong
   - **Timing/race condition**: Add proper synchronization primitives (channels, barriers, condvars) NOT arbitrary sleeps
   - **Flakiness**: Find and eliminate non-determinism source, add determinism checks
2. **Consider ripple effects**: Does this fix impact other components? Are there similar issues elsewhere?
3. **Add regression protection**: If production bug, add test that would have caught it
4. **Update documentation**: If behavior was unclear, clarify it

#### Strictly Forbidden Practices
- ❌ Commenting out or weakening assertions that fail
- ❌ Adding `Thread::sleep()` or arbitrary delays to "fix" timing issues
- ❌ Catching and ignoring/swallowing errors
- ❌ Marking tests as `#[ignore]` without documented plan to fix
- ❌ Relaxing test tolerances without understanding why original was appropriate
- ❌ Changing expected values to match actual without understanding root cause
- ❌ Disabling features in tests that exist in production

```rust
// ✅ DO: Fix root cause when test reveals production bug
// Investigation: Test checksum validation failed randomly
// Root cause: checksums computed over all frames, but old frames get discarded
// Fix: Changed to window-based computation (last 64 frames)
// Verification: Test now passes consistently, added unit test for window logic

// ✅ DO: Fix test when test has incorrect assumptions  
// Investigation: Test expected immediate connection, but protocol has handshake
// Root cause: Test assumption didn't match documented async connection behavior
// Fix: Updated test to wait for connection established event

// ❌ DON'T: Band-aid patches that hide real issues
// - Commenting out assertions that fail
// - Adding Thread::sleep(5000) to "fix" timing
// - Catching and ignoring errors
// - Marking tests as #[ignore]
// - Increasing tolerances without justification
```

## Specific Assistance Areas

### Session Management
- Suggest appropriate session types (P2P, Spectator, SyncTest)
- Help configure session builders with sensible defaults
- Recommend input delay based on network conditions

### Network Protocol
- Ensure message ordering and delivery semantics
- Suggest compression strategies for bandwidth optimization
- Help with desync detection configuration

### State Management
- Guide serialization implementations for game state
- Suggest efficient rollback strategies
- Help maintain determinism across platforms

### Error Handling
- Provide context-rich error variants
- Suggest recovery strategies
- Include error documentation in rustdoc

## Quality Checklist

Before suggesting code, ensure it:
- [ ] Compiles with no warnings
- [ ] Includes tests (unit and/or integration)
- [ ] Has rustdoc comments for public items
- [ ] Follows Rust idioms and best practices
- [ ] Maintains 100% safe Rust
- [ ] Handles errors appropriately
- [ ] Considers performance implications
- [ ] Works toward >90% coverage goal

## Formal Verification Support

When working on critical components:
- Suggest documenting invariants as comments
- Help identify safety properties to verify
- Recommend TLA+ models for concurrent logic
- Propose Z3 constraints for algorithmic correctness

## Examples Location

Refer to and update these when making API changes:
- `examples/ex_game/ex_game_p2p.rs` - P2P session example
- `examples/ex_game/ex_game_spectator.rs` - Spectator example  
- `examples/ex_game/ex_game_synctest.rs` - Determinism testing

## Additional Resources

- Full context: `.llm/context.md`
- Contributing guide: `CONTRIBUTING.md`
- Changelog: `CHANGELOG.md`
- Original GGPO: https://www.ggpo.net/
