# Option Copied Pattern — Working with `Option<&T>` and Value Types

> When retrieving values from collections via `.get()`, use `.copied()` to convert `Option<&T>` to `Option<T>` for `Copy` types. This improves code clarity and avoids subtle reference semantics.

## The Problem

Many collection methods return `Option<&T>`:

```rust
let bytes: &[u8] = &[1, 2, 3];
let value = bytes.get(0);  // Returns Option<&u8>, not Option<u8>
```

While Rust's auto-deref often handles this transparently, explicit `.copied()` makes intent clear and avoids potential confusion in expressions.

---

## When to Use `.copied()`

### ✅ Use for Value Retrieval with Error Handling

The canonical pattern for safe indexing with error propagation:

```rust
// ✅ RECOMMENDED: Clear value semantics
let byte: u8 = data
    .get(index)
    .copied()
    .ok_or(Error::IndexOutOfBounds { index, len: data.len() })?;

// Also good for simple unwrap_or patterns
let byte: u8 = data.get(index).copied().unwrap_or(0);
```

### ✅ Use When Operating on the Value

When you need the value for arithmetic, bitwise operations, or comparisons:

```rust
// ✅ RECOMMENDED: XOR with explicit value types
let ref_byte = ref_bytes.get(i).copied().ok_or(Error::OutOfBounds)?;
let data_byte = data.get(j).copied().ok_or(Error::OutOfBounds)?;
let result = ref_byte ^ data_byte;  // Both are u8, not &u8

// ✅ RECOMMENDED: Arithmetic operations
let sum = values.get(i).copied().unwrap_or(0) + offset;
```

### ✅ Use for Iterator Chains

When you want to work with owned values in an iterator:

```rust
// ✅ Returns Iterator<Item = u8> instead of Iterator<Item = &u8>
let owned_values: Vec<u8> = slice.iter().copied().collect();

// ✅ Filter and map with values
let result: Option<u8> = bytes
    .iter()
    .copied()
    .filter(|&b| b > 0)
    .max();
```

---

## Alternative Patterns

### Pattern Matching with `Some(&value)`

Destructuring in match/if-let avoids the method call:

```rust
// ✅ Equivalent to .copied() — pattern destructures the reference
match bytes.get(index) {
    Some(&byte) => process(byte),  // byte is u8, not &u8
    None => return Err(Error::NotFound),
}

// ✅ Also works with if-let
if let Some(&b) = slice.get(i) {
    // b is the value, not a reference
}
```

**Trade-off:** Pattern matching is good for control flow; `.copied()` chains better with combinators.

### Using `*` Dereference

Explicit dereference after the fact:

```rust
// ⚠️ Works but less idiomatic
let byte_ref = bytes.get(index).ok_or(Error::NotFound)?;
let byte = *byte_ref;

// ⚠️ Inline dereference — harder to read
let result = *bytes.get(i).ok_or(Error::NotFound)? ^ *bytes.get(j).ok_or(Error::NotFound)?;
```

**Trade-off:** More verbose and splits the operation across multiple steps.

---

## When `.copied()` Is NOT Needed

### ❌ Non-Copy Types

`.copied()` only works for `Copy` types. For other types, use `.cloned()`:

```rust
// ❌ Won't compile — String is not Copy
let s: Option<String> = strings.get(0).copied();

// ✅ Use .cloned() for Clone types
let s: Option<String> = strings.get(0).cloned();
```

### ❌ When You Need a Reference

If you're passing to a function that takes `&T`, keep the reference:

```rust
// ✅ No copy needed — function takes reference
fn process(data: &[u8]) { /* ... */ }

if let Some(slice) = matrix.get(row) {
    process(slice);  // Pass &[u8] directly
}
```

### ❌ For Complex or Large Types

Even for `Copy` types, copying large arrays may be undesirable:

```rust
// ⚠️ Copies 1KB of data — probably not what you want
let arr: Option<[u8; 1024]> = large_arrays.get(0).copied();

// ✅ Keep reference for large Copy types
let arr: Option<&[u8; 1024]> = large_arrays.get(0);
```

---

## Comparison: `.copied()` vs `.cloned()`

| Method | Trait Bound | Use Case |
|--------|-------------|----------|
| `.copied()` | `T: Copy` | Primitives, small value types |
| `.cloned()` | `T: Clone` | Strings, Vecs, any Clone type |

```rust
// Primitives: use .copied()
let num: Option<u32> = numbers.get(0).copied();

// Owned types: use .cloned()
let name: Option<String> = names.get(0).cloned();
```

---

## Summary

| Situation | Recommended Pattern |
|-----------|---------------------|
| Safe indexing with `?` | `.get(i).copied().ok_or(err)?` |
| Default on missing | `.get(i).copied().unwrap_or(default)` |
| Match with value | `Some(&val) => ...` |
| Iterator of values | `.iter().copied()` |
| Non-Copy types | `.get(i).cloned()` |
| Need reference | `.get(i)` (no copy) |

**Rule of thumb:** When you need the *value* from `Option<&T>` where `T: Copy`, use `.copied()` to make that intent explicit.
