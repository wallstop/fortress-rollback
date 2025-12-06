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
/// Returns `GgrsError::NotSynchronized` if session is still synchronizing.
///
/// # Examples
/// ```
/// session.advance_frame()?;
/// ```
pub fn advance_frame(&mut self) -> Result<(), GgrsError> {
    // Implementation
}

// ❌ DON'T: Undocumented public APIs
pub fn advance_frame(&mut self) -> Result<(), GgrsError> {
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
