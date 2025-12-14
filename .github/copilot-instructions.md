# GitHub Copilot Instructions for Fortress Rollback

> **Important**: Read [`.llm/context.md`](../.llm/context.md) for complete project context, policies, and guidelines.

## Quick Reference

This is a **fork** of GGRS (Good Game Rollback System) focused on:
- **>90% test coverage** - All code must be thoroughly tested
- **Formal verification** - TLA+ and Z3 verification of critical components
- **Enhanced usability** - Simple, intuitive, hard-to-misuse APIs
- **Code clarity** - Easy to understand and maintain

## Core Principles

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

### 3. Clear & Usable APIs
- Favor builder patterns for complex configuration
- Suggest type-safe interfaces over runtime validation
- Provide clear, actionable error messages
- Include usage examples in rustdoc comments

### 4. Documentation Standards
- All public items need rustdoc with examples
- Explain "why" not just "what" in comments
- Link to relevant formal specifications when applicable
- Keep docs/changelog.md updated

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
    
    // Act & Assert
    // ... complete test logic
}

// ❌ DON'T: Vague test names without clear intent
#[test]
fn test_rollback() {
    // Minimal test
}
```

## When Tests Fail

See [`.llm/context.md`](../.llm/context.md) for the complete Root Cause Analysis methodology.

**Key points:**
- Always investigate root cause before fixing
- Distinguish between test bugs and production bugs
- Never use arbitrary sleeps to "fix" timing issues
- Never comment out or weaken failing assertions

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

## Quality Checklist

Before suggesting code, ensure it:
- [ ] Compiles with no warnings
- [ ] Includes tests (unit and/or integration)
- [ ] Has rustdoc comments for public items
- [ ] Follows Rust idioms and best practices
- [ ] Maintains 100% safe Rust
- [ ] Handles errors appropriately
- [ ] Works toward >90% coverage goal

## Examples Location

Refer to and update these when making API changes:
- `examples/ex_game/ex_game_p2p.rs` - P2P session example
- `examples/ex_game/ex_game_spectator.rs` - Spectator example  
- `examples/ex_game/ex_game_synctest.rs` - Determinism testing

## Additional Resources

- Full context: `.llm/context.md`
- Contributing guide: `docs/contributing.md`
- Changelog: `docs/changelog.md`
- Original GGPO: https://www.ggpo.net/
