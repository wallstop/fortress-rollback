# Claude Instructions for Fortress Rollback

> **Important**: Read [`.llm/context.md`](.llm/context.md) for complete project context, policies, and guidelines.

## Quick Reference

This is a **fork** of GGRS (Good Game Rollback System) focused on:

- **>90% test coverage** - All code must be thoroughly tested
- **Formal verification** - TLA+ and Z3 verification of critical components
- **Enhanced usability** - Simple, intuitive, hard-to-misuse APIs
- **Code clarity** - Easy to understand and maintain

## When Assisting

### General Approach

- Prioritize correctness and safety over cleverness
- Write self-documenting code with clear intent
- Include tests with every code change
- Think about edge cases and error conditions
- Consider formal verification for critical logic

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

### When Tests Fail

See [`.llm/context.md`](.llm/context.md) for the complete Root Cause Analysis methodology.

**Key points:**

- Always investigate root cause before fixing
- Distinguish between test bugs and production bugs
- Never use arbitrary sleeps to "fix" timing issues
- Never comment out or weaken failing assertions

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
/// # use fortress_rollback::{P2PSession, PlayerType};
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

## Key Concepts

1. **Frames**: Discrete time steps in game simulation
2. **Rollback**: Restoring previous state when predictions are wrong
3. **Input Delay**: Buffer frames to smooth over network jitter
4. **Prediction**: Continuing simulation before inputs arrive
5. **Desync Detection**: Checksum verification between peers
6. **Determinism**: Same inputs → same outputs (critical requirement)

## Project Structure

- `src/lib.rs` - Public API and core types
- `src/sessions/` - Session implementations (P2P, Spectator, SyncTest)
- `src/network/` - Network protocol and communication
- `src/sync_layer/` - Core synchronization logic and state management
- `src/input_queue.rs` - Input buffering and management
- `examples/ex_game/` - Example implementations
- `tests/` - Integration tests (organized by category)

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

After every major change:

```bash
cargo fmt
cargo clippy --all-targets
cargo test
```

## Additional Resources

- Complete context: `.llm/context.md`
- User guide: `docs/user-guide.md`
- Architecture: `docs/architecture.md`
- Contributing: `docs/contributing.md`
- Changelog: `docs/changelog.md`
- Original GGPO: <https://www.ggpo.net/>
