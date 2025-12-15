# OpenAI AGENTS Instructions for Fortress Rollback

> **Important**: Read [`.llm/context.md`](.llm/context.md) for complete project context, policies, and guidelines.

## Quick Reference

This is a **fork** of GGRS (Good Game Rollback System) focused on:

- **>90% test coverage** - All code must be thoroughly tested
- **Formal verification** - TLA+ and Z3 verification of critical components
- **Enhanced usability** - Simple, intuitive, hard-to-misuse APIs
- **Code clarity** - Easy to understand and maintain

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
- Maintain docs/changelog.md

## Code Standards

### Function Documentation Template

```rust
/// Brief one-line description.
///
/// # Arguments
/// * `param1` - Description
///
/// # Errors
/// * `ErrorVariant1` - When this occurs
///
/// # Examples
/// ```
/// # use fortress_rollback::*;
/// let result = function(arg1)?;
/// ```
pub fn function(param1: Type1) -> Result<ReturnType, FortressError> {
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
5. Update docs/changelog.md
6. Ensure all tests pass

### Fixing a Bug

1. Write failing test that reproduces the bug
2. **Root cause analysis** - Understand *why* it fails before fixing
3. Fix the implementation (not the test, unless the test is wrong)
4. Verify test passes
5. Check for similar issues elsewhere
6. Document the fix in docs/changelog.md

### When Tests Fail

See [`.llm/context.md`](.llm/context.md) for the complete Root Cause Analysis methodology.

**Key points:**

- Always investigate root cause before fixing
- Distinguish between test bugs and production bugs
- Never use arbitrary sleeps to "fix" timing issues
- Never comment out or weaken failing assertions

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

## Project Navigation

**Source Code**

- `src/lib.rs` - Public API
- `src/sessions/` - Session implementations
- `src/network/` - Network protocol (includes `protocol/` submodule)
- `src/sync_layer/` - Core sync logic and state management
- `src/input_queue.rs` - Input management

**Tests & Examples**

- `tests/` - Integration tests (organized by category)
- `examples/ex_game/` - Working examples

## Quick Reference

### Player Types

```rust,ignore
PlayerType::Local              // Local player
PlayerType::Remote(addr)       // Remote player
PlayerType::Spectator(addr)    // Spectator (no input)
```

### Session Builder

```rust,ignore
SessionBuilder::<MyConfig>::new()
    .with_num_players(n)
    .with_input_delay(frames)
    .add_player(player_type, PlayerHandle::new(handle))?
    .start_p2p_session(socket)?
```

### Request Pattern

```rust,ignore
for request in session.advance_frame()? {
    match request {
        FortressRequest::SaveGameState { frame, cell } => { /* ... */ }
        FortressRequest::LoadGameState { cell, .. } => { /* ... */ }
        FortressRequest::AdvanceFrame { inputs } => { /* ... */ }
    }
}
```

## Quality Gates

Before suggesting code, verify:

- ✅ Compiles without warnings
- ✅ All tests pass
- ✅ Documentation is complete
- ✅ Error handling is comprehensive
- ✅ 100% safe Rust
- ✅ Examples work correctly

After every major change:

```bash
cargo fmt
cargo clippy --all-targets
cargo test
```

## Resources

- Full context: `.llm/context.md`
- Contributing: `docs/contributing.md`
- Changelog: `docs/changelog.md`
- Original GGPO: <https://www.ggpo.net/>
