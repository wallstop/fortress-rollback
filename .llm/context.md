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
**CRITICAL: When tests fail or are flaky, always perform proper RCA**

Before fixing a failing test:
1. **Understand the failure** - Don't just make the test pass; understand *why* it fails
2. **Distinguish test bug vs production bug** - Is the test wrong, or is the production code wrong?
3. **Fix at the correct level**:
   - Production bug → Fix library code
   - Test bug → Fix test's incorrect assumptions
   - Timing issue → Add proper synchronization (not arbitrary sleeps)
   - Flakiness → Find and eliminate the source of non-determinism
4. **Never band-aid patch**: Disabling assertions, adding excessive timeouts, or commenting out checks are NOT fixes
5. **Document the fix**: Explain what was wrong and why the fix is correct

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
1. After every major change, run `cargo fmt`, `cargo clippy --all-targets --all-features`, and `cargo test`, and fix all resulting issues
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
