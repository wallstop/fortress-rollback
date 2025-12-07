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
