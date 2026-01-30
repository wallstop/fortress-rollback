# Defensive Programming — Zero-Panic Production Code

> **This document defines the defensive programming standards for Fortress Rollback.**
> All production code, including library code and any editor/tooling code, MUST follow these practices.
> **This also applies to documentation examples** — rustdoc examples are compiled and should demonstrate correct error handling.

## Core Philosophy

**Production code must be 100% safe, predictable, and deterministic.** We achieve this through:

1. **Zero-Panic Policy** — Never panic in production; return `Result` for all fallible operations
2. **Assume Nothing** — Validate all inputs and internal state; trust no assumptions
3. **Expose Errors, Don't Swallow** — Callers must handle potential failures explicitly
4. **Maintain Invariants** — Internal state must remain consistent even during error recovery
5. **Type-Safe APIs** — Use the type system to make invalid states unrepresentable

---

## Zero-Panic Policy (CRITICAL)

### Forbidden Panic Patterns

The following patterns are **STRICTLY FORBIDDEN** in production code:

```rust
// ❌ FORBIDDEN - Direct panic
panic!("something went wrong");
panic!("{}", message);

// ❌ FORBIDDEN - Implicit panics via unwrap/expect
value.unwrap();
value.expect("should never be None");
result.unwrap();
result.expect("operation must succeed");

// ❌ FORBIDDEN - Index operations that can panic
array[index];
slice[range];
vec[index];
string.chars().nth(i).unwrap();

// ❌ FORBIDDEN - Placeholder panics
todo!();
todo!("implement later");
unimplemented!();
unimplemented!("not yet implemented");

// ❌ FORBIDDEN - Assertions in production paths (OK in tests)
assert!(condition);
assert_eq!(a, b);
assert_ne!(a, b);
debug_assert!(condition);  // OK in debug builds, but prefer explicit handling

// ❌ FORBIDDEN - unreachable! (unless TRULY unreachable by type system)
unreachable!();
unreachable!("this should never happen");
```

### Documentation Examples Must Also Follow Zero-Panic

**Rustdoc examples are compiled code.** They must demonstrate proper error handling, not panic shortcuts:

```rust
// ❌ FORBIDDEN in doc examples — teaches bad habits
/// # Examples
///
/// ```
/// let session = SessionBuilder::new().build().unwrap();
/// let result = session.advance_frame();
/// if result.is_err() {
///     panic!("frame advance failed");  // NEVER show panic! as error handling
/// }
/// ```

// ✅ REQUIRED in doc examples — demonstrates proper patterns
/// # Examples
///
/// ```
/// # use fortress_rollback::*;
/// let session = SessionBuilder::new().build()?;
/// let requests = session.advance_frame()?;
/// for request in requests {
///     // Handle each request...
/// }
/// # Ok::<(), FortressError>(())
/// ```
```

**Why this matters:**

- Doc examples are often copy-pasted by users
- Examples with `panic!` or `unwrap()` teach incorrect patterns
- Users learn from examples — show them the RIGHT way
- The `# Ok::<(), Error>(())` pattern enables `?` in doc tests

### When Different Patterns Are Acceptable in Doc Examples

While the general rule is "no panics in examples," there are nuanced cases where different patterns are appropriate based on what the example is teaching:

#### Use `if let Some` for Defensive Fallback Patterns

When demonstrating how users should handle optional state in their game loops:

```rust
/// // Simulate a LoadGameState request handler
/// // LoadGameState is only requested for previously saved frames,
/// // but we handle None defensively to avoid crashing on library bugs
/// if let Some(loaded) = cell.load() {
///     current_state = loaded;
/// }
/// // If load() returns None, current_state is unchanged
```

**When to use:** Teaching defensive programming where the caller should gracefully handle missing data rather than crash.

#### Use `.expect("reason")` for Provably-Present State

When the example demonstrates a successful path where the state is guaranteed to exist by prior operations in the same example:

```rust
/// // We just saved the state above, so we know it exists
/// let accessor = cell.data().expect("state was just saved");
/// assert_eq!(accessor.player_name, "alex");
```

**When to use:** When the example's control flow proves the value exists, AND the focus is on demonstrating the happy path. The `.expect()` message must explain WHY it's safe.

**Important:** This is acceptable ONLY when:

- The example itself proves the value exists (e.g., save then load)
- The message clearly explains the invariant
- The example demonstrates API usage, not error handling

#### Use `.ok_or(Error)?` for Error Propagation Patterns

When demonstrating how users should handle missing state as an error condition:

```rust
/// let loaded = cell.load_or_err(frame)?;
/// // Or manually:
/// let loaded = cell.load()
///     .ok_or(FortressError::InvalidFrameStructured {
///         frame,
///         reason: InvalidFrameReason::MissingState,
///     })?;
```

**When to use:** Teaching error propagation where missing data is a genuine error that should be returned to the caller.

#### Decision Guide for Doc Examples

| Scenario | Pattern | Example |
|----------|---------|---------|
| Teaching defensive game loop handling | `if let Some` | `if let Some(s) = cell.load() { state = s; }` |
| Demonstrating happy path with proven state | `.expect("why")` | `cell.data().expect("just saved")` |
| Teaching error propagation | `.ok_or()?` | `cell.load().ok_or(Error::Missing)?` |
| General fallible operations | `?` operator | `session.advance_frame()?` |

### Documentation Example Verification (CRITICAL)

**ALWAYS verify that types, methods, and error variants used in documentation examples actually exist in the source code.** Fabricated examples that don't compile erode trust and waste users' time.

#### Before Writing Doc Examples

```bash
# Verify error variants exist
rg 'enum FortressError' -A 100 src/error.rs | head -120

# Verify a specific variant exists
rg 'DesyncDetected|InvalidFrame|NetworkError' src/error.rs

# Verify struct/method exists
rg 'pub fn method_name|pub struct TypeName' --type rust
```

#### Common Doc Example Mistakes

```rust
// ❌ FORBIDDEN: Using non-existent error variants
/// ```
/// match result {
///     Err(FortressError::DesyncDetected) => { /* ... */ }  // Does this exist?
/// }
/// ```

// ❌ FORBIDDEN: Incomplete match on #[non_exhaustive] enums
/// ```
/// match event {
///     FortressEvent::Synchronizing { total, count, .. } => { /* ... */ }
///     FortressEvent::Disconnected { .. } => { /* ... */ }
///     // Missing other variants AND missing `_ =>` fallback!
/// }
/// ```

// ✅ REQUIRED: Verify variants exist, handle exhaustiveness
/// ```
/// match event {
///     FortressEvent::Synchronizing { total, count, .. } => { /* ... */ }
///     FortressEvent::Disconnected { addr, .. } => { /* ... */ }
///     FortressEvent::NetworkInterrupted { addr, .. } => { /* ... */ }
///     // ... all other variants ...
///     _ => { /* Handle future variants gracefully */ }
/// }
/// ```
```

#### Matching on `#[non_exhaustive]` Enums in Examples

When demonstrating match statements on `#[non_exhaustive]` enums (like `FortressEvent`), you **must** include a wildcard arm:

```rust
// ✅ REQUIRED for #[non_exhaustive] enums in doc examples
/// ```
/// for event in session.events()? {
///     match event {
///         FortressEvent::Synchronizing { total, count, .. } => {
///             println!("Sync progress: {count}/{total}");
///         }
///         FortressEvent::Disconnected { addr, .. } => {
///             println!("Player at {addr} disconnected");
///         }
///         _ => {
///             // Handle other/future event types
///         }
///     }
/// }
/// ```
```

**Why this matters:**

- `#[non_exhaustive]` means new variants may be added without a breaking change
- Examples without `_ =>` won't compile (teaching users broken patterns)
- The wildcard arm shows users how to future-proof their code

#### Verification Checklist for Doc Examples

Before committing documentation with code examples:

- [ ] All error variants used actually exist in `FortressError`
- [ ] All struct/method names are spelled correctly and exist
- [ ] Match statements on `#[non_exhaustive]` enums include `_ =>` arm
- [ ] Examples compile: `cargo test --doc`
- [ ] Examples follow zero-panic policy (no `unwrap()` without justification)

### Required Patterns

All fallible operations MUST return `Result`:

```rust
// ✅ REQUIRED - Convert Option to Result
value.ok_or(FortressError::MissingValue)?
value.ok_or_else(|| FortressError::NotFound { key: key.clone() })?

// ✅ REQUIRED - Safe indexing
array.get(index).ok_or(FortressError::IndexOutOfBounds { index, len: array.len() })?
slice.get(range).ok_or(FortressError::RangeOutOfBounds)?

// ✅ REQUIRED - Explicit error returns
if !valid {
    return Err(FortressError::InvalidState { reason: "precondition violated" });
}

// ✅ REQUIRED - Transform and propagate errors
operation().map_err(|e| FortressError::OperationFailed { cause: e.to_string() })?

// ✅ REQUIRED - Checked arithmetic (instead of panicking overflow)
a.checked_add(b).ok_or(FortressError::ArithmeticOverflow)?
a.checked_sub(b).ok_or(FortressError::ArithmeticUnderflow)?
a.checked_mul(b).ok_or(FortressError::ArithmeticOverflow)?
```

### When `unreachable!()` Is Acceptable

Only use `unreachable!()` when the type system GUARANTEES it cannot be reached:

```rust
// ✅ OK - Type system guarantees this arm is unreachable
enum State { A, B }
match state {
    State::A => handle_a(),
    State::B => handle_b(),
    // No wildcard needed - all variants covered
}

// ✅ OK - After exhaustive validation that changes types
let positive: NonZeroU32 = match value {
    0 => return Err(Error::ZeroNotAllowed),
    n => NonZeroU32::new(n).expect("n is non-zero"), // OK: proven by match
};

// ❌ NOT OK - Runtime assumption, not type-guaranteed
match self.state {
    State::Connected => { /* ... */ }
    _ => unreachable!(), // State could be anything!
}
```

---

## Never Swallow Errors

### Forbidden Error-Hiding Patterns

```rust
// ❌ FORBIDDEN - Ignoring Result entirely
let _ = fallible_operation();
fallible_operation();  // Warning: unused Result

// ❌ FORBIDDEN - Silent fallback on error
let value = operation().unwrap_or(default);  // Hides WHY it failed
let value = operation().unwrap_or_default();

// ❌ FORBIDDEN - Conditional success, silent failure
if let Ok(value) = operation() {
    use(value);
}
// What happens on Err? Nothing? That's a bug.

// ❌ FORBIDDEN - Matching away errors
match result {
    Ok(v) => v,
    Err(_) => return,  // Where did the error go?
}
```

### Required Error Handling Patterns

```rust
// ✅ REQUIRED - Propagate with ?
fallible_operation()?;

// ✅ REQUIRED - Transform and propagate
fallible_operation()
    .map_err(|e| FortressError::Wrapped { source: e })?;

// ✅ REQUIRED - Handle OR propagate, never ignore
match fallible_operation() {
    Ok(value) => process(value),
    Err(e) => {
        // Either handle it meaningfully...
        log::warn!("Operation failed: {e}, using fallback");
        use_fallback()
        // ...OR propagate it
        // return Err(e.into());
    }
}

// ✅ REQUIRED - If using unwrap_or, document WHY it's safe
let value = operation()
    .unwrap_or(DEFAULT);  // OK only if DEFAULT is semantically correct for ALL errors

// ✅ BETTER - Be explicit about acceptable errors
let value = match operation() {
    Ok(v) => v,
    Err(Error::NotFound) => DEFAULT, // Explicitly acceptable
    Err(e) => return Err(e.into()),  // Other errors propagate
};
```

---

## Validate Everything, Assume Nothing

### Input Validation

All public APIs must validate inputs at the boundary:

```rust
// ❌ Avoid: Trusts caller
pub fn set_player_count(&mut self, count: usize) {
    self.players = vec![Player::default(); count];
}

// ✅ Prefer: Validates at boundary
pub fn set_player_count(&mut self, count: usize) -> Result<(), FortressError> {
    if count == 0 {
        return Err(FortressError::InvalidPlayerCount {
            count,
            reason: "must have at least one player",
        });
    }
    if count > MAX_PLAYERS {
        return Err(FortressError::InvalidPlayerCount {
            count,
            reason: "exceeds maximum player limit",
        });
    }
    self.players = vec![Player::default(); count];
    Ok(())
}
```

### Internal State Validation

Don't assume internal state is valid — verify before use:

```rust
// ❌ Avoid: Assumes index is valid
fn current_player(&self) -> &Player {
    &self.players[self.current_player_index]
}

// ✅ Prefer: Returns Result, validates state
fn current_player(&self) -> Result<&Player, FortressError> {
    self.players
        .get(self.current_player_index)
        .ok_or(FortressError::InvalidPlayerIndex {
            index: self.current_player_index,
            count: self.players.len(),
        })
}

// ✅ Alternative: Debug assertion + safe access (for internal hot paths)
fn current_player(&self) -> Option<&Player> {
    debug_assert!(
        self.current_player_index < self.players.len(),
        "invariant violated: index {} >= len {}",
        self.current_player_index,
        self.players.len()
    );
    self.players.get(self.current_player_index)
}
```

---

## Maintain Invariants

### State Consistency

Operations must either succeed completely or leave state unchanged:

```rust
// ❌ Avoid: Partial update on failure leaves inconsistent state
fn add_player(&mut self, player: Player) -> Result<(), Error> {
    self.count += 1;  // Updated
    self.players.push(player);  // What if this fails?
    self.update_network()?;  // Now count is wrong if this fails!
    Ok(())
}

// ✅ Prefer: Prepare, then commit atomically
fn add_player(&mut self, player: Player) -> Result<(), Error> {
    // Validate first
    if self.count >= MAX_PLAYERS {
        return Err(Error::TooManyPlayers);
    }

    // Prepare the update (may fail)
    self.prepare_network_update(&player)?;

    // Commit atomically (infallible operations only)
    self.players.push(player);
    self.count += 1;
    Ok(())
}

// ✅ Alternative: Rollback on failure
fn add_player(&mut self, player: Player) -> Result<(), Error> {
    self.players.push(player);
    self.count += 1;

    if let Err(e) = self.update_network() {
        // Rollback
        self.players.pop();
        self.count -= 1;
        return Err(e);
    }
    Ok(())
}
```

### RAII for Cleanup

Use Drop traits and guards for cleanup that MUST happen:

```rust
// ✅ RAII guard ensures cleanup
struct ConnectionGuard<'a> {
    session: &'a mut Session,
    connection_id: ConnectionId,
}

impl Drop for ConnectionGuard<'_> {
    fn drop(&mut self) {
        // Cleanup runs even if the operation panics (in tests)
        // or returns early with an error
        self.session.release_connection(self.connection_id);
    }
}

fn use_connection(session: &mut Session) -> Result<(), Error> {
    let id = session.acquire_connection()?;
    let _guard = ConnectionGuard { session, connection_id: id };

    // If any of this fails, guard ensures cleanup
    do_something()?;
    do_another_thing()?;

    Ok(())
    // Guard dropped here, releasing connection
}
```

---

## Type-Safe API Design

### Make Invalid States Unrepresentable

```rust
// ❌ Avoid: Runtime validation needed
struct Session {
    players: Vec<Player>,  // Could be empty!
    frame: i32,            // Could be negative!
}

impl Session {
    fn current_frame(&self) -> Result<u32, Error> {
        if self.frame < 0 {
            return Err(Error::InvalidFrame);
        }
        Ok(self.frame as u32)
    }
}

// ✅ Prefer: Invalid states impossible by construction
struct Session {
    players: NonEmpty<Player>,  // At least one player
    frame: Frame,               // Newtype wrapper, always valid
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Frame(u32);  // Can never be negative

impl Frame {
    pub const ZERO: Frame = Frame(0);

    pub fn new(value: u32) -> Self {
        Frame(value)
    }

    pub fn checked_add(self, delta: u32) -> Option<Frame> {
        self.0.checked_add(delta).map(Frame)
    }
}
```

### Use Enums Over Booleans

```rust
// ❌ Avoid: What does true mean?
fn connect(addr: SocketAddr, encrypted: bool, compressed: bool) { }
connect(addr, true, false);  // Unclear

// ✅ Prefer: Self-documenting
enum Encryption { Enabled, Disabled }
enum Compression { Enabled, Disabled }

fn connect(addr: SocketAddr, encryption: Encryption, compression: Compression) { }
connect(addr, Encryption::Enabled, Compression::Disabled);  // Crystal clear
```

### Phantom Types for State Machines

```rust
// ✅ Compile-time state machine validation
struct Connection<S: ConnectionState> {
    inner: ConnectionInner,
    _state: PhantomData<S>,
}

trait ConnectionState {}
struct Disconnected;
struct Connecting;
struct Connected;

impl ConnectionState for Disconnected {}
impl ConnectionState for Connecting {}
impl ConnectionState for Connected {}

impl Connection<Disconnected> {
    fn connect(self) -> Connection<Connecting> {
        // ... initiate connection
        Connection { inner: self.inner, _state: PhantomData }
    }
}

impl Connection<Connecting> {
    fn wait_connected(self) -> Result<Connection<Connected>, Error> {
        // ... wait for connection
        Ok(Connection { inner: self.inner, _state: PhantomData })
    }
}

impl Connection<Connected> {
    fn send(&mut self, data: &[u8]) -> Result<(), Error> {
        // Only available when connected!
        Ok(())
    }
}
```

---

## Safe Collection Access Patterns

### Prefer Iterators Over Indexing

```rust
// ❌ Avoid: Index-based loops
for i in 0..items.len() {
    process(&items[i]);  // Can panic if items modified
}

// ✅ Prefer: Iterator-based
for item in &items {
    process(item);
}

// ✅ Prefer: With index if needed
for (i, item) in items.iter().enumerate() {
    process_with_index(i, item);
}
```

### Pattern Matching for Collection Access

```rust
// ❌ Avoid: Can panic
let first = &items[0];
let last = &items[items.len() - 1];

// ✅ Prefer: Pattern matching
let first = items.first().ok_or(Error::EmptyCollection)?;
let last = items.last().ok_or(Error::EmptyCollection)?;

// ✅ Prefer: Destructuring
match items.as_slice() {
    [] => return Err(Error::EmptyCollection),
    [only] => process_single(only),
    [first, .., last] => process_range(first, last),
}

// ✅ Prefer: Split operations
let (head, tail) = items.split_first().ok_or(Error::EmptyCollection)?;
```

---

## Error Design Guidelines

### Rich Error Types

```rust
// ✅ Good error design
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FortressError {
    /// Player index is out of bounds
    InvalidPlayerIndex {
        index: usize,
        player_count: usize,
    },

    /// Frame number is invalid for the current state
    InvalidFrame {
        requested: Frame,
        current: Frame,
        reason: &'static str,
    },

    /// Network operation failed
    NetworkError {
        operation: &'static str,
        details: String,
    },

    /// Internal invariant violated (indicates a bug)
    InvariantViolation {
        invariant: &'static str,
        context: String,
    },
}
```

### Error Context

Always provide enough context to debug:

```rust
// ❌ Avoid: No context
return Err(Error::InvalidIndex);

// ✅ Prefer: Full context
return Err(FortressError::InvalidPlayerIndex {
    index: player_idx,
    player_count: self.players.len(),
});
```

### Error Categorization: `InvalidRequest*` vs `InternalError*`

Choosing the correct error category is critical for debugging and API contracts.

**`InvalidRequestStructured` / `InvalidRequest`**: Use for **caller-provided invalid arguments**.
The caller made a mistake; this is expected and recoverable.

**`InternalErrorStructured` / `InternalError`**: Use for **library bugs or invariant violations**.
Something went wrong inside the library that should never happen under normal API usage.

**Decision tree:**

```
Is the invalid value provided by the caller as an argument?
├─ YES → InvalidRequestStructured (caller's responsibility)
└─ NO → Is it derived from internal library state?
        ├─ YES → InternalErrorStructured (library bug)
        └─ NO → Trace back: who created this value?
                └─ Usually leads to one of the above
```

**Example: Division by zero**

```rust
// Function signature: pub fn try_buffer_index(&self, buffer_size: usize) -> Result<...>

// ❌ WRONG: buffer_size == 0 is caller's fault, not a library bug
if buffer_size == 0 {
    return Err(FortressError::InternalErrorStructured {
        kind: InternalErrorKind::DivisionByZero,  // Wrong category!
    });
}

// ✅ CORRECT: Caller passed invalid argument
if buffer_size == 0 {
    return Err(FortressError::InvalidRequestStructured {
        kind: InvalidRequestKind::ZeroBufferSize,
    });
}
```

**Why this matters:**

- `InternalError` tells users: "Report this as a bug" — wrong if it's their fault
- `InvalidRequest` tells users: "Fix your input" — actionable guidance
- Incorrect categorization erodes trust and wastes debugging time

**Quick test:** Ask "If a user follows the documented API correctly, can they ever trigger this error?"

- **NO** (impossible with correct usage) → `InternalError` (indicates library bug)
- **YES** (possible with incorrect arguments) → `InvalidRequest` (user error)

### Unknown/Fallback Variants in Error Reason Enums

When creating "reason" enums for structured errors, include an `Unknown` or fallback variant
for cases where error mapping might not have complete information.

```rust
// ❌ Avoid: No fallback for unexpected cases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeltaDecodeReason {
    EmptyReference,
    DataLengthMismatch { data_len: usize, reference_len: usize },
    ReferenceIndexOutOfBounds { index: usize, length: usize },
    DataIndexOutOfBounds { index: usize, length: usize },
    // What happens if we need to map an unexpected error type?
}

// ✅ Prefer: Explicit Unknown variant for fallback cases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RleDecodeReason {
    BitfieldIndexOutOfBounds,
    DestinationSliceOutOfBounds,
    SourceSliceOutOfBounds,
    TruncatedData { offset: usize, buffer_len: usize },
    /// An unknown or unexpected error occurred.
    ///
    /// This variant is used as a fallback when the underlying error cannot be
    /// mapped to a more specific reason (e.g., when downcasting fails).
    Unknown,
}
```

**Why `Unknown` is important:**

1. **Error mapping functions** need a fallback when source errors don't match expected patterns
2. **Non-exhaustive enums** may have new variants added; code mapping them needs safety valves
3. **Defensive error handling** shouldn't panic when encountering unexpected error types

**Anti-pattern: Misleading fallback values**

Never use an existing variant with nonsensical placeholder values as a fallback — it misleads
debugging and masks the true cause:

```rust
// ❌ FORBIDDEN: Misleading placeholder values
fn map_error(error: &FortressError) -> RleDecodeReason {
    match error {
        // ... specific mappings ...
        _ => RleDecodeReason::TruncatedData { offset: 0, buffer_len: 0 },  // MISLEADING!
        // This looks like a real truncation error but contains no useful info
    }
}

// ✅ REQUIRED: Explicit Unknown variant
fn map_error(error: &FortressError) -> RleDecodeReason {
    match error {
        // ... specific mappings ...
        _ => RleDecodeReason::Unknown,  // Honest about not knowing the cause
    }
}
```

```rust
// ✅ Usage: Error mapping with Unknown fallback
fn map_rle_error_to_reason(error: &FortressError) -> RleDecodeReason {
    match error {
        FortressError::InternalErrorStructured {
            kind: InternalErrorKind::RleDecodeError { reason },
        } => *reason,
        _ => RleDecodeReason::Unknown,  // Safe fallback for unexpected types
    }
}
```

**When to add Unknown:**

- Error reason enums used in error mapping/conversion functions
- Enums that might receive values from external sources (deserialization, FFI)
- Enums marked `#[non_exhaustive]` where exhaustive matching isn't possible

**When Unknown may not be needed:**

- Internal enums where all variants are explicitly constructed in known code paths
- Enums where an existing `Custom(&'static str)` variant serves as the fallback

---

## Advanced Defensive Patterns

### Use `TryFrom` Instead of `From` for Fallible Conversions

`From` implementations must never panic. If a conversion can fail, use `TryFrom`:

```rust
// ❌ FORBIDDEN - From that panics violates zero-panic policy
impl From<i32> for Frame {
    fn from(value: i32) -> Self {
        if value < 0 {
            panic!("Frame cannot be negative");  // NEVER DO THIS
        }
        Frame(value as u32)
    }
}

// ✅ REQUIRED - TryFrom for fallible conversions
impl TryFrom<i32> for Frame {
    type Error = FortressError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value < 0 {
            return Err(FortressError::InvalidFrame {
                reason: "frame cannot be negative",
            });
        }
        Ok(Frame(value as u32))
    }
}
```

### Safe Numeric Conversions

Never use `as` for numeric conversions that can truncate or lose sign:

```rust
// ❌ FORBIDDEN - Silent truncation/overflow
let small: i8 = big_number as i8;
let unsigned: u32 = signed_value as u32;

// ✅ REQUIRED - Explicit conversion with error handling
let small = i8::try_from(big_number)
    .map_err(|_| FortressError::NumericOverflow { value: big_number })?;

// ✅ OK - Infallible widening conversions
let big: i64 = small_number.into();
let wider: u64 = u32::from(narrow);
```

### Avoid `..Default::default()` — Use Explicit Field Initialization

Using `..Default::default()` hides new fields when structs evolve:

```rust
// ❌ Avoid: New fields silently get defaults
struct SessionConfig {
    max_prediction_frames: usize,
    input_delay: usize,
    disconnect_timeout_ms: u64,  // Added later - silently becomes 0!
}

let config = SessionConfig {
    max_prediction_frames: 8,
    input_delay: 2,
    ..Default::default()  // Hides disconnect_timeout_ms
};

// ✅ Prefer: Explicit initialization — compiler errors on new fields
let config = SessionConfig {
    max_prediction_frames: 8,
    input_delay: 2,
    disconnect_timeout_ms: 5000,
};

// ✅ Alternative: Destructure-then-override (when defaults are appropriate)
let SessionConfig {
    max_prediction_frames: _,
    input_delay: _,
    disconnect_timeout_ms: _,  // Compiler errors if field added
} = SessionConfig::default();

let config = SessionConfig {
    max_prediction_frames: 8,
    input_delay: 2,
    ..SessionConfig::default()  // Now safe - all fields acknowledged
};
```

### Exhaustive Destructuring in Trait Implementations

When implementing `PartialEq`, `Hash`, `Debug`, etc., destructure to catch new fields:

```rust
// ❌ Avoid: New fields silently excluded from comparison
impl PartialEq for PlayerState {
    fn eq(&self, other: &Self) -> bool {
        self.frame == other.frame && self.input == other.input
        // prediction_count added later - silently ignored!
    }
}

// ✅ Prefer: Destructure forces handling all fields
impl PartialEq for PlayerState {
    fn eq(&self, other: &Self) -> bool {
        let Self { frame, input, checksum } = self;
        let Self {
            frame: other_frame,
            input: other_input,
            checksum: other_checksum,
        } = other;
        // Adding a field causes compile error here

        frame == other_frame && input == other_input && checksum == other_checksum
    }
}
```

### Named Placeholders for Ignored Fields

Use `field_name: _` instead of just `_` or `..`:

```rust
// ❌ Avoid: No warning if field removed
let NetworkMessage { sequence, payload, .. } = msg;

// ✅ Prefer: Explicit acknowledgment of ignored fields
let NetworkMessage {
    sequence,
    payload,
    timestamp: _,  // Compiler error if timestamp removed
} = msg;
```

### Defensive Constructors

Prevent invalid construction with private fields or `#[non_exhaustive]`:

```rust
// ❌ Avoid: Public fields allow invalid construction
pub struct PlayerHandle {
    pub index: usize,  // Anyone can create PlayerHandle { index: 999 }
}

// ✅ Prefer: Private fields + validated constructor
pub struct PlayerHandle {
    index: usize,
    _private: (),  // Prevents construction outside module
}

impl PlayerHandle {
    pub fn new(index: usize, session: &Session) -> Result<Self, FortressError> {
        if index >= session.player_count() {
            return Err(FortressError::InvalidPlayerIndex {
                index,
                count: session.player_count(),
            });
        }
        Ok(Self { index, _private: () })
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

// ✅ For library enums that may grow: #[non_exhaustive]
#[non_exhaustive]
pub enum SessionEvent {
    PlayerJoined { player: PlayerHandle },
    PlayerLeft { player: PlayerHandle },
    // New variants won't break downstream matches
}
```

### `#[must_use]` on Important Types

Prevent accidental ignoring of critical values:

```rust
// ✅ Force callers to handle important return values
#[must_use = "frame advance result contains requests that must be processed"]
pub struct FrameAdvanceResult {
    pub requests: Vec<FortressRequest>,
    pub skip_frame: bool,
}

// ✅ On methods returning important values
impl Session {
    #[must_use]
    pub fn advance_frame(&mut self) -> FrameAdvanceResult {
        // ...
    }
}

// ✅ On builders
#[must_use = "builders do nothing until .build() is called"]
pub struct SessionBuilder { /* ... */ }
```

### Temporary Mutability Pattern

Shadow to freeze values after initialization:

```rust
// ✅ Prevent accidental mutation after setup
fn build_config() -> SessionConfig {
    let mut config = SessionConfig::default();
    config.max_prediction_frames = 8;
    config.input_delay = calculate_delay();

    let config = config;  // Shadow: now immutable
    // config.input_delay = 0;  // Compile error!

    validate(&config);
    config
}

// ✅ Scope block variant for complex initialization
let config = {
    let mut config = SessionConfig::default();
    let temp = compute_settings();
    config.apply_settings(temp);
    config  // Returned immutable; temp not accessible
};
```

### Parameter Structs for Many Options

Replace multiple parameters with a configuration struct:

```rust
// ❌ Avoid: Easy to mix up parameter order
fn start_session(
    num_players: usize,
    input_delay: usize,
    max_prediction: usize,
    disconnect_timeout: Duration,
) -> Result<Session, Error> { /* ... */ }

start_session(4, 2, 8, Duration::from_secs(5))?;  // Which is which?

// ✅ Prefer: Self-documenting struct
pub struct SessionConfig {
    pub num_players: usize,
    pub input_delay: usize,
    pub max_prediction: usize,
    pub disconnect_timeout: Duration,
}

fn start_session(config: SessionConfig) -> Result<Session, Error> { /* ... */ }

start_session(SessionConfig {
    num_players: 4,
    input_delay: 2,
    max_prediction: 8,
    disconnect_timeout: Duration::from_secs(5),
})?;
```

### Enhanced Slice Pattern Matching

Use full slice patterns instead of check-then-index:

```rust
// ❌ Avoid: Check and index are separate operations
fn process_inputs(inputs: &[Input]) -> Result<(), Error> {
    if inputs.is_empty() {
        return Err(Error::NoInputs);
    }
    let first = inputs[0];  // Decoupled from check
    let rest = &inputs[1..];  // Can panic with 1 element
    // ...
}

// ✅ Prefer: Compiler-enforced patterns
fn process_inputs(inputs: &[Input]) -> Result<(), Error> {
    match inputs {
        [] => Err(Error::NoInputs),
        [single] => process_single(single),
        [first, second] => process_pair(first, second),
        [first, rest @ ..] => {
            // first: &Input, rest: &[Input] - guaranteed safe
            process_first(first);
            for item in rest {
                process_rest(item)?;
            }
            Ok(())
        }
    }
}
```

### Safe Debug for Sensitive Data

Redact sensitive fields in Debug implementations:

```rust
// ✅ REQUIRED - Destructure to catch new sensitive fields
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { username, password: _, api_key: _ } = self;
        f.debug_struct("Credentials")
            .field("username", username)
            .field("password", &"[REDACTED]")
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}
```

---

## Recommended Clippy Lints

Enable these lints for automated enforcement:

```rust
// In lib.rs or main.rs
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::fallible_impl_from,
)]

#![warn(
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::wildcard_enum_match_arm,
    clippy::default_trait_access,
)]
```

Or in `Cargo.toml`:

```toml
[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
indexing_slicing = "deny"
arithmetic_side_effects = "deny"
cast_possible_truncation = "deny"
cast_sign_loss = "deny"
fallible_impl_from = "deny"
must_use_candidate = "warn"
return_self_not_must_use = "warn"
wildcard_enum_match_arm = "warn"
```

---

## Summary Checklist

Before committing any production code, verify:

- [ ] No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`
- [ ] No direct index access `[]` — use `.get()` with error handling
- [ ] No `as` for lossy numeric conversions — use `TryFrom`
- [ ] All `Result` values are handled (not ignored with `let _ =`)
- [ ] All public functions validate inputs at the boundary
- [ ] State changes are atomic or rolled back on failure
- [ ] Error types provide sufficient context for debugging
- [ ] Types make invalid states unrepresentable where possible
- [ ] Assertions only in test code, not production paths
- [ ] No `..Default::default()` without explicit field acknowledgment
- [ ] Custom trait impls use exhaustive destructuring
- [ ] `#[must_use]` on important return types
- [ ] Sensitive data redacted in `Debug` implementations
- [ ] `cargo doc --no-deps` passes — no broken intra-doc links

---

*This policy applies to all code: library, examples, tools, and editor integrations.*
