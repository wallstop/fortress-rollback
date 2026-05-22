<!-- CATEGORY: Rust Language -->
<!-- WHEN: Implementing error handling, ensuring zero-panic compliance, validating inputs -->
# Defensive Programming -- Zero-Panic Production Code

## Zero-Panic Policy (CRITICAL)

### Forbidden Patterns

```rust
panic!(); value.unwrap(); value.expect("..."); array[index]; todo!(); unimplemented!();
assert!(cond); // OK in tests only
unreachable!(); // Only when type system guarantees it
```

### Required Patterns

```rust
value.ok_or(FortressError::MissingValue)?;
array.get(index).ok_or(FortressError::IndexOutOfBounds { index, len: array.len() })?;
a.checked_add(b).ok_or(FortressError::ArithmeticOverflow)?;
operation().map_err(|e| FortressError::OperationFailed { cause: e.to_string() })?;
```

### Doc Examples Must Also Follow Zero-Panic

Use `?` with `# Ok::<(), FortressError>(())` pattern in doc tests.

| Scenario | Pattern |
|----------|---------|
| Teaching defensive handling | `if let Some(s) = cell.load() { state = s; }` |
| Happy path with proven state | `.expect("just saved")` (with justification) |
| Error propagation | `.ok_or(Error::Missing)?` |
| General fallible ops | `?` operator |

### Doc Example Verification

Always verify error variants, struct names, method names exist in source. Match on `#[non_exhaustive]` enums must include `_ =>` arm.

## Never Swallow Errors

```rust
// FORBIDDEN
let _ = fallible_operation();
let value = operation().unwrap_or_default(); // hides WHY

// REQUIRED
fallible_operation()?;
match fallible_operation() {
    Ok(value) => process(value),
    Err(Error::NotFound) => DEFAULT, // explicitly acceptable
    Err(e) => return Err(e.into()),
}
```

### No Silent Skips on Internal-Invariant Lookups

Internal-invariant collections (`input_queues`, `local_connect_status`,
`handles`, etc.) are sized by construction to cover every valid index. If a
lookup misses, the invariant is broken — that is a library bug, not a
recoverable condition. The fix depends on the function shape:

| Function shape | Required handling |
|----------------|-------------------|
| Returns `Result` | `.ok_or(FortressError::InternalErrorStructured { kind: ... })?` (e.g. `IndexOutOfBounds`, `ConnectionStatusIndexOutOfBounds`) |
| `&mut self`, returns `()` / `Frame` / etc. | `report_violation!(ViolationSeverity::Error, ViolationKind::InputQueue \| ::NetworkProtocol, "...");` then take a safe fallback |
| Loops `for i in 0..self.num_players { if let Some(q) = self.input_queues.get(i) {...} }` | Restructure to `for q in self.input_queues.iter() { ... }` — eliminates the silent-skip pattern by construction |

`if let Some(...) = self.collection.get(...)` followed by no `else` arm is a
red flag for invariant-backed collections: it converts a bug into a no-op.
The format string in `report_violation!` should include the index, the
collection name, and `collection.len()` so the diagnostic is actionable.

**Severity rubric** (use the highest applicable tier):

- `ViolationSeverity::Critical` — construction-time configuration breakage
  that leaves the structure in a permanently broken state (e.g., the
  per-player queue count is short of `num_players` after a constructor
  fallback fails — every subsequent operation that indexes by handle is
  unsafe by construction).
- `ViolationSeverity::Error` — runtime invariant violation where the call
  took a safe fallback or returned a structured error to its caller; the
  session as a whole keeps running, but this specific operation reflects a
  bug somewhere in the library.
- `ViolationSeverity::Warning` — recoverable conditions where the system
  corrected itself (clamping, automatic retries, transient gaps that the
  caller can ignore).

## Input Validation

All public APIs must validate inputs at the boundary:

```rust
pub fn set_player_count(&mut self, count: usize) -> Result<(), FortressError> {
    if count == 0 || count > MAX_PLAYERS {
        return Err(FortressError::InvalidPlayerCount { count, reason: "out of range" });
    }
    self.players = vec![Player::default(); count];
    Ok(())
}
```

## State Consistency

Operations must succeed completely or leave state unchanged (prepare-then-commit or rollback).

## Error Categorization

| Question | Category |
|----------|----------|
| Is invalid value from caller's argument? | `InvalidRequestStructured` |
| Is it from internal library state? | `InternalErrorStructured` |

**Quick test:** Can a user following docs correctly trigger this? YES -> `InvalidRequest`. NO -> `InternalError`.

### Unknown Fallback Variants

Include `Unknown` variant in error reason enums for safe fallback in mapping functions. Never use existing variants with placeholder values.

## Safe Collection Access

```rust
// Prefer iterators over indexing
for item in &items { process(item); }

// Pattern matching for first/last
let first = items.first().ok_or(Error::Empty)?;
match items.as_slice() {
    [] => Err(Error::Empty),
    [only] => process_single(only),
    [first, rest @ ..] => { /* guaranteed safe */ }
}
```

## Advanced Patterns

### `TryFrom` over `From` for Fallible Conversions

`From` must never panic. Use `TryFrom` if conversion can fail.

### Safe Numeric Conversions

Never `as` for lossy conversions. Use `i8::try_from(big)?.into()` or infallible widening `.into()`.

### Avoid `..Default::default()`

New fields silently get defaults. Prefer explicit field initialization or destructure-then-override.

### Exhaustive Destructuring in Trait Impls

```rust
impl PartialEq for PlayerState {
    fn eq(&self, other: &Self) -> bool {
        let Self { frame, input, checksum } = self;
        // Adding a field causes a compile error here, forcing you to update
        *frame == other.frame && *input == other.input && *checksum == other.checksum
    }
}
```

### `#[must_use]` on Important Types

```rust
#[must_use = "frame advance result contains requests that must be processed"]
pub struct FrameAdvanceResult { /* ... */ }
```

### Temporary Mutability

Shadow to freeze: `let config = config;` after setup.

### Parameter Structs

Replace many params with a config struct for self-documenting call sites.

## Recommended Clippy Lints

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
```

## Bounded Allocation

The default allocator **aborts the process** on allocation failure (uncatchable, cf. RUSTSEC-2022-0035). So any `Vec::with_capacity(n)`, `vec![x; n]`, or `reserve(n)` whose `n` comes from an unbounded source (a length read from the wire, an unvalidated config field) is a DoS/abort vector.

Two rules:

1. **Never trust a length from the wire.** Cap any size derived from network input against a hard maximum *before* allocating, and use `checked_add` while accumulating so a crafted packet cannot overflow `usize`. Return a structured error on the bomb path instead of allocating. Example: `rle::decode` caps the decoded length at `rle::MAX_DECODED_LEN` (64 MiB).
2. **Validate config at the boundary.** Range-check public config fields (e.g. in a `validate()` called by the builder) so a directly-constructed config cannot drive a huge allocation. Defense-in-depth: also clamp the size at the allocation site so the bound holds even if validation is bypassed.

Every dynamically-sized allocation in `src/` must carry an `// alloc-bound: <why>` comment (same line or the line above) stating why its size is bounded. Pure integer literals (`with_capacity(4)`, `vec![0u8; 16]`) and exact `.len()`/`.count()` sizes are exempt. The `check-unbounded-alloc` pre-commit hook (also run by agent-preflight) enforces this; tests/proofs in trailing `#[cfg(test)]` / `#[cfg(kani)]` modules are skipped.

```rust
// alloc-bound: num_players is the validated session player count fixed at construction
let mut queues = Vec::with_capacity(num_players);
```

## Checklist

- [ ] Dynamically-sized allocations bounded + `// alloc-bound:` justified
- [ ] No `unwrap()`, `expect()`, `panic!()`, `todo!()`
- [ ] No direct `[]` indexing -- use `.get()` with error handling
- [ ] No `as` for lossy numeric conversions
- [ ] All `Result` values handled
- [ ] Public functions validate inputs
- [ ] State changes atomic or rolled back
- [ ] Error types provide context
- [ ] No `..Default::default()` without field acknowledgment
- [ ] Custom trait impls use exhaustive destructuring
- [ ] `#[must_use]` on important return types
