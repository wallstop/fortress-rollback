# Code Review Lessons Learned

> **This document captures patterns and anti-patterns discovered through code review feedback.**
> Use these guidelines to prevent similar issues in future development.

---

## Eager vs Lazy Error Construction (`ok_or` vs `ok_or_else`)

### The Pattern

When using `Option::ok_or()`, the error value is constructed eagerly (every time), even when
the Option contains a value. This can be wasteful in hot paths.

```rust
// Eager construction — error built even when index is valid
value.ok_or(ExpensiveError { context: compute_context() })?

// Lazy construction — error only built on error path
value.ok_or_else(|| ExpensiveError { context: compute_context() })?
```

### When to Use `ok_or_else`

Use `ok_or_else(|| ...)` when:

- The error type allocates (contains `String`, `Vec`, `Box`, etc.)
- Computing error field values is expensive
- The code is in a hot path (inner loops, frequently called functions)
- The error construction has side effects

### When `ok_or` Is Fine

Use simple `ok_or(...)` when:

- The error type is `Copy` (no allocation, trivial construction)
- All field values are already computed or are `Copy`
- Clippy's `unnecessary_lazy_evaluations` lint would trigger

**Important:** Clippy will warn if you use `ok_or_else` with a `Copy` type. Trust the lint —
for `Copy` types, eager construction is actually more efficient than closure overhead.

```rust
// ❌ Clippy warning: unnecessary_lazy_evaluations
.ok_or_else(|| CopyError::IndexOutOfBounds { index, len })

// ✅ Correct for Copy types
.ok_or(CopyError::IndexOutOfBounds { index, len })
```

---

## Result Type Alias Semver Hazard

### The Problem

Exporting a `Result` type alias at the crate root can shadow `std::result::Result` for
downstream users who use glob imports:

```rust
// In your library
pub type Result<T, E = MyError> = std::result::Result<T, E>;

// Downstream user
use my_library::*;  // Now `Result` shadows std::result::Result!
```

### The Solution

Use a distinctive name that cannot shadow standard library types:

```rust
// ✅ Safe — cannot shadow std::result::Result
pub type FortressResult<T, E = FortressError> = std::result::Result<T, E>;
```

### Best Practices

1. **Use distinctive names** — `FortressResult`, `MyLibResult`, etc.
2. **Export from prelude only** — Don't export at crate root, only in a `prelude` module
3. **Document the pattern** — Show users how to alias locally if they prefer short names:

```rust
// Users can create a local alias
use fortress_rollback::FortressResult as Result;
```

---

## Test-Production Code Alignment

### The Problem

Tests that simulate internal implementation details can drift from production code:

```rust
// Production code (evolved)
fn decode(data: &[u8]) -> Result<Vec<u8>, MyError> {
    inner_decode(data).map_err(|e| match e {
        InnerError::Foo => MyError::Foo,
        InnerError::Bar => MyError::Bar,
    })?
}

// Tests (stuck on old implementation)
fn test_error_mapping() {
    let error: Box<dyn Error> = Box::new(SomeError);
    let result = error.downcast_ref::<InnerError>();  // Production doesn't do this!
    // ...
}
```

### The Solution

**Extract testable helpers.** When production code has error mapping logic, extract it:

```rust
// Extracted helper — testable in isolation
fn map_inner_error(e: InnerError) -> MyError {
    match e {
        InnerError::Foo => MyError::Foo,
        InnerError::Bar => MyError::Bar,
    }
}

// Production code uses the helper
fn decode(data: &[u8]) -> Result<Vec<u8>, MyError> {
    inner_decode(data).map_err(map_inner_error)?
}

// Tests test the helper directly
#[test]
fn test_map_inner_error_foo() {
    assert_eq!(map_inner_error(InnerError::Foo), MyError::Foo);
}
```

### Best Practices

1. **Test the actual code path** — Don't simulate patterns not in production
2. **Extract helpers for complex mappings** — Makes them unit-testable
3. **Add integration tests** — Verify end-to-end behavior with real inputs
4. **Review tests when refactoring** — Ensure tests still test the right thing

---

## Kani Proof Naming and Verification

### The Problem

Proof names and documentation can claim properties that the proof doesn't actually verify:

```rust
/// Proof: Clone creates independent copy.
/// Verifies that modifying one doesn't affect other.  // <-- claim
#[kani::proof]
fn proof_clone_is_independent() {
    let a = MyStruct::new();
    let b = a.clone();

    // Only checks equality, never modifies!
    kani::assert(a.field == b.field, "fields match");
}
```

### The Solution

**Proofs must verify what they claim.** If the name says "independent", actually test
modification independence:

```rust
#[kani::proof]
fn proof_clone_is_independent() {
    let a = MyStruct::new();
    let mut b = a.clone();

    let original_value = a.field;

    // Actually modify the clone
    b.field = different_value();

    // Verify original is unchanged (independence)
    kani::assert(a.field == original_value, "Original unchanged after modifying clone");

    // Verify clone has modification
    kani::assert(b.field != original_value, "Clone has new value");
}
```

### Best Practices

1. **Name proofs accurately** — `proof_clone_preserves_fields` vs `proof_clone_is_independent`
2. **Verify all claimed properties** — Read doc comments, ensure assertions match
3. **Consider renaming over extending** — If a proof tests X but claims Y, maybe rename to X
4. **Document proof scope clearly** — What exactly does this proof verify?

---

## Doc Comments and Implementation Details

### The Problem

Doc comments that describe *how* code works (implementation details) become stale when the
implementation changes:

```rust
/// Creates a violation with a unique ID.
/// Uses static string slices for zero-allocation performance.  // <-- LIE!
fn make_violation(id: u32) -> Violation {
    Violation::new(
        format!("test violation {}", id),  // Actually allocates!
    )
}
```

The doc comment claims "static string slices" but the code uses `format!()` which allocates.
This mismatch misleads readers and erodes trust in documentation.

### The Root Cause

Doc comments describing implementation details are inherently fragile because:

1. **Code changes, comments don't** — Refactoring updates code but forgets comments
2. **Performance claims age poorly** — Optimizations may be added or removed
3. **Allocation patterns shift** — Moving from `&'static str` to `String` is common
4. **Reviewers focus on code** — Comments are often skimmed, not verified against code

### The Solution

**Doc comments should describe WHAT, not HOW** — unless the HOW is part of the API contract.

```rust
// ❌ Describes implementation (fragile)
/// Creates a violation with a unique ID.
/// Uses static string slices for zero-allocation performance.

// ✅ Describes behavior (stable)
/// Creates a violation with a unique ID.

// ✅ OK if allocation IS the contract
/// Creates a violation with a unique ID.
///
/// # Performance
///
/// This function is allocation-free and suitable for hot paths.
/// (Note: This creates an API contract — changing it is breaking!)
```

### When Implementation Details ARE Appropriate

Include HOW only when it's part of the API contract:

- **Performance guarantees** — "O(1) lookup", "allocation-free"
- **Thread safety** — "Lock-free", "Uses interior mutability"
- **Determinism** — "Uses seeded RNG for reproducibility"
- **Resource management** — "Caches results", "Lazily initialized"

But remember: documenting these creates an implicit contract. Changing them becomes a
breaking change, even if the function signature doesn't change.

### Best Practices

1. **Focus on WHAT, not HOW** — Describe behavior and purpose
2. **Omit performance claims** — Unless they're API guarantees
3. **Review comments when refactoring** — Update or remove stale implementation details
4. **Use `# Performance` sections** — Makes performance contracts explicit and findable
5. **Avoid redundant phrases** — "for testing purposes" in test code is obvious

---

## GitHub Actions Permissions

### The Problem

Writing to system directories like `/usr/local/bin` requires elevated permissions on
GitHub-hosted runners:

```yaml
# ❌ Can fail with "permission denied"
run: |
  curl -sfL "$URL" | tar xz -C /usr/local/bin my_tool
```

### Solutions

**Option 1: Use sudo**

```yaml
# ✅ Works on GitHub-hosted runners
run: |
  curl -sfL "$URL" | sudo tar xz -C /usr/local/bin my_tool
```

**Option 2: Install to user directory**

```yaml
# ✅ No sudo needed
run: |
  mkdir -p "$HOME/.local/bin"
  curl -sfL "$URL" | tar xz -C "$HOME/.local/bin" my_tool
  echo "$HOME/.local/bin" >> "$GITHUB_PATH"
```

### Best Practices

1. **Be consistent** — Use the same pattern across all workflows
2. **Prefer sudo for /usr/local/bin** — It's simpler and widely understood
3. **Use GITHUB_PATH for custom directories** — Ensures tools are available to later steps
4. **Test on fresh runners** — Local dev containers may have different permissions

---

## Pattern Matching in Error Mappers

### The Problem

Using `if let` with a fallthrough path causes use-after-move bugs:

```rust
// ❌ BUG: `e` moved in if-let, unusable in fallback
fn map_error(e: MyError) -> OtherError {
    if let MyError::Specific { data } = e {
        return OtherError::Mapped { data };
    }
    log::warn!("unexpected: {:?}", e);  // ERROR: use of moved value!
    OtherError::Unknown
}
```

### The Solution

**Always use `match` for error mapping functions** that need to:

1. Handle a specific variant
2. Log/warn about unexpected variants

```rust
// ✅ CORRECT: Single match expression
fn map_error(e: MyError) -> OtherError {
    match e {
        MyError::Specific { data } => OtherError::Mapped { data },
        other => {
            log::warn!("unexpected: {:?}", other);
            OtherError::Unknown
        }
    }
}
```

### Why This Pattern Exists

Error mapping functions commonly need to:

- Extract fields from a specific error variant for the happy path
- Handle unexpected variants gracefully with logging/metrics
- Include the original error in the fallback for debugging

The `if let` + fallthrough pattern seems natural but moves ownership before the fallback.

### Best Practices

1. **Prefer `match` over `if let` + fallthrough** when you need the value in both paths
2. **Use `other` binding in catch-all arm** — gives access to the unmatched value
3. **Consider borrowing** — `if let Pattern = &value` if you don't need ownership
4. **Add `Unknown` variants to reason enums** — provides a fallback for error mapping

See also: [rust-pitfalls.md](rust-pitfalls.md#use-after-move-in-if-let-fallthrough)

---

## Summary Checklist

Before submitting code:

- [ ] `ok_or` vs `ok_or_else` — Used correctly based on error type (Copy vs allocating)
- [ ] Type aliases — Use distinctive names that can't shadow stdlib types
- [ ] Tests match production — No simulating patterns not in real code
- [ ] Kani proofs — Actually verify what their names/docs claim
- [ ] Doc comments — Describe WHAT, not HOW (unless HOW is API contract)
- [ ] CI permissions — Use sudo for system directories
- [ ] Pattern matching — Use `match` not `if let` when fallback needs the value

---

*This document should be updated as new patterns are discovered through code review.*
