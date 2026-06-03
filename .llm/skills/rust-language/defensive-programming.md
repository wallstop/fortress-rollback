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

### Fallible / Infallible Constructor Pairs

When a fallible `try_*` constructor is wrapped by an infallible `new`/`with_*` that reports a violation on error, the `try_*` layer returns the structured error **silently** — only the wrapper calls `report_violation!`. Logging in both layers emits duplicate telemetry for one invalid input. The infallible wrapper should also document its degraded fallback (the exact shape it returns on error) so the behavioral contract is explicit. Pattern source: `SavedStates`, `TimeSync`, `SyncLayer`, `InputQueue`. Also: when one error path must be preserved (e.g. propagating `AllocationFailed`), prefer propagating the original `FortressError` over collapsing every cause into a single opaque variant.

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

The default allocator **aborts the process** on allocation failure (uncatchable, cf. RUSTSEC-2022-0035). So any `Vec::with_capacity(n)`, `vec![x; n]`, or `reserve(n)` whose `n` comes from an unbounded source (a length read from the wire, a user-configurable size) is a DoS/abort vector unless the allocation is made fallible first.

Two rules:

1. **Never trust a length from the wire.** Bound any size derived from network input against a caller-configured limit or explicit default *before* allocating, and use `checked_add` while accumulating so a crafted packet cannot overflow `usize`. Return a structured error on the bomb path instead of allocating. When a wire length is decoded into a wider type (e.g. a `u64` varint), range-check it against the limit *while it is still `u64`*, before any `as usize` narrowing: on a 32-bit target (`usize == u32`) the cast truncates the high bits and could turn a huge declared length into a small one that slips past the check (see `rle::checked_run_len`). Example: protocol input decode derives its RLE decoded-length limit from `ProtocolConfig::pending_output_limit` and the reference input size; standalone `rle::decode` uses `rle::DEFAULT_MAX_DECODED_LEN`. Avoid generic serde/bincode container decoding for untrusted bytes when the schema has `Vec`, `String`, or similar length-prefixed fields; parse those lengths explicitly, validate them against the actual input/config, reserve fallibly, and then copy/decode the bounded payload.
2. **Do not invent arbitrary caps for user configuration.** Validate true semantic invariants at the boundary (for example `queue_length >= 2` or `stream_delay < buffer_size`), then use `try_reserve*`/checked arithmetic at construction so huge but user-requested sizes return a structured allocation error instead of aborting the process.
3. **A zero size is non-functional, not just abort-prone.** A size that is *too small* can silently break an object without any allocation failure: a zero-length socket receive buffer reads zero bytes forever (silently dropping every datagram and able to spin), and a zero-length send buffer can never encode. Reject a zero size at the boundary with `io::ErrorKind::InvalidInput` (or the structured equivalent), and centralize the check in one constructor helper so every entry point inherits it (e.g. `network::buffer::zeroed_buffer`, used by every socket-buffer constructor). Where an infallible constructor must degrade rather than fail, fall back to a *non-empty* buffer and guard the consumer against an empty one — never leave it to spin.

Every dynamically-sized infallible allocation in `src/` must carry an `// alloc-bound: <why>` comment (same line or the line above) stating why its size is concretely bounded. Pure integer literals (`with_capacity(4)`, `vec![0u8; 16]`), exact `.len()`/`.count()` sizes, and fallible `try_reserve*` reservations are exempt. Weak markers such as "trusted local config" or "no numeric cap" are rejected by the hook. Tests/proofs in trailing `#[cfg(test)]` / `#[cfg(kani)]` modules are skipped.

```rust
let mut queues = Vec::new();
queues.try_reserve_exact(num_players)?;
for player in 0..num_players {
    queues.push(InputQueue::try_with_queue_length(player, queue_length)?);
}
```

### Bulk-reserving from an untrusted `size_hint`

`Iterator::size_hint` is a performance hint only -- a buggy/adversarial iterator may report an upper bound that is too low, too high, or `None`. Never trust it for correctness. Use `error::try_reserve_hint(&mut vec, upper, per_item)` (the canonical helper) to bulk-reserve ONCE before a loop. It is **best-effort and infallible**: it multiplies with saturating arithmetic, reserves via `try_reserve`, and **silently ignores failure** (an over-large or dishonest hint simply leaves the vector unreserved). It never aborts and never returns an error.

Because a failed reservation here is a no-op, it is a pure optimization that **cannot change behavior** for any iterator (honest or adversarial). The load-bearing, panic-free growth path is the caller's per-iteration fallible `try_reserve`, which always runs and covers the `None`/under-reporting/over-large cases identically to pure incremental growth. The reservation is `upper * per_item` **additional** capacity (`Vec::try_reserve` semantics: reserves for `len + additional`, not an absolute target), so call it on an **empty/near-empty** vector before the loop — on a partially-filled vector it would under-reserve.

```rust
let (_lower, upper) = pending_input.size_hint();
try_reserve_hint(&mut bytes, upper, ref_bytes.len()); // best-effort; no `?`
for input in pending_input {
    // `Vec::try_reserve` is a documented no-op (returns `Ok` without
    // reallocating) when capacity already suffices, so the bulk reservation
    // above keeps this a single allocation in the common case; this call only
    // grows when the hint under-reported (or was absent). No hand-written
    // spare-capacity guard is needed -- std performs that check internally.
    let requested = bytes.len().saturating_add(input.len());
    // reserve-in-loop: guards an under-reporting/absent size_hint.
    let reserved = bytes.try_reserve(input.len());
    reserved.map_err(|_| allocation_failed("compression.delta_encode", requested))?;
    // ... push the XOR bytes ...
}
```

### The `// reserve-in-loop:` marker

A `try_reserve` / `try_reserve_exact` whose nearest enclosing block is a loop body (`for`/`while`/`loop`) is flagged by `check-unbounded-alloc` unless it carries a `// reserve-in-loop: <why>` justification (same line or the line above). Per-iteration fallible reserves are sometimes correct -- guarding an untrusted/absent `size_hint`, or allocating one fresh bounded buffer per iteration -- but must be a CONSCIOUS choice, not an accident: prefer a single bulk pre-reservation before the loop where the count is known. A closure or match arm LEXICALLY NESTED within a `for`/`while`/`loop` still counts as in-loop; a `fn` defined inside a loop body shields its own reserves. `impl Trait for Type {` is not treated as a loop. Known tested limitations (all defense-in-depth only, all false negative, never false positive; none occurs in `src/`): (1) a loop whose HEADER embeds a brace block (`for x in match v {} {`, `for x in vec![Foo { a: 1 }] {`) is classified as a non-loop body; (2) a closure whose signature embeds a bare `fn`-pointer TYPE (`|cb: fn()| { ... }`) is conservatively classified as a `fn` body, so an in-loop reserve inside it is shielded; (3) the iteration-IS-the-closure idiom (`items.iter().for_each(|x| { ... })`, `.map(|x| { ... }).collect()`) is NOT flagged — "closure inside a loop counts as in-loop" means a closure lexically nested in a `for`/`while`/`loop`, NOT an iterator-adapter closure that performs the iteration itself.

## Checklist

- [ ] Dynamically-sized allocations bounded + `// alloc-bound:` justified
- [ ] In-loop `try_reserve` / `try_reserve_exact` marked `// reserve-in-loop:`
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
