<!-- CATEGORY: Rust Language -->
<!-- WHEN: Reviewing code, debugging common Rust mistakes, avoiding pitfalls -->
# Rust Pitfalls -- Bugs That Compile

## Numeric Pitfalls

### Silent Truncation with `as`
```rust
let small: i8 = big as i8; // silently wraps!
// Use: i8::try_from(big).map_err(|_| Error::Overflow)?;
// OK when intentional: let low_byte: u8 = (value & 0xFF) as u8;
```
**Clippy:** `cast_possible_truncation`, `cast_sign_loss`, `cast_possible_wrap`

### Float Comparison
Use epsilon comparison or `float-cmp`/`approx` crate. Never `assert_eq!` on floats.

### Integer Division Truncates
`5 / 2` is `2`, not `2.5`. Be explicit: `(5 + 1) / 2` for round-up.

## String and Path Pitfalls

- **UTF-8 indexing:** `&s[0..2]` panics on multi-byte chars. Use `s.chars()`.
- **`Path::join` replaces base** if argument is absolute. Validate relative paths.
- **OsStr not UTF-8:** `path.file_name().and_then(|n| n.to_str())` returns `None` if invalid.

## Collection Pitfalls

- **HashMap iteration order** is non-deterministic. Use `BTreeMap` or sort keys.
- **`Vec::drain` range panics** if out of bounds. Clamp: `v.drain(0..v.len().min(10))`.
- **`collect::<Result<Vec<_>, _>>`** stops on first error but processes all items. Use explicit loop for short-circuit.

## Concurrency Pitfalls

### Mutex Poisoning
```rust
let data = mutex.lock().map_err(|_| Error::MutexPoisoned)?;
```

### RwLock Read-Before-Write Deadlock
Drop read lock before acquiring write lock. Re-check condition under write lock.

### Spin Loops Without Yield
```rust
while !flag.load(Ordering::Acquire) {
    #[cfg(loom)] loom::thread::yield_now();
    #[cfg(not(loom))] std::hint::spin_loop();
}
```

### Using std Types Under Loom
In loom tests, use `loom::sync::Arc`, `loom::thread`, `loom::sync::atomic::*` -- not `std::`.

### Arc Must Be Cloned BEFORE thread::spawn
```rust
let cell = Arc::new(data);
let cell_for_thread = cell.clone(); // clone BEFORE spawn
thread::spawn(move || { cell_for_thread.do_work(); });
assert!(cell.is_ok()); // original still available
```

## Pattern Matching and Ownership

### Use-After-Move in `if let` Fallthrough
```rust
// BUG: e moved by if let, unusable in fallback
if let MyError::Specific { field } = e { return mapped; }
log::warn!("{:?}", e); // ERROR: moved!

// FIX: Use match
match e {
    MyError::Specific { field } => OtherError::Mapped { field },
    other => { log::warn!("{:?}", other); OtherError::Unknown }
}
```

## Error Handling Pitfalls

- **Silent error conversion:** Preserve context with `.map_err()`.
- **`unwrap_or` hides errors:** Log or handle explicitly.
- **`?` in Drop:** Not allowed. Use `if let Err(e) = self.cleanup() { /* log */ }`.

## `ok_or` vs `ok_or_else`

Use `ok_or_else(|| ...)` when error construction allocates or is expensive. Use `ok_or(...)` for `Copy` error types (Clippy warns on unnecessary lazy evaluation).

## Result Type Alias Semver Hazard

`pub type Result<T, E = MyError>` shadows `std::result::Result` for glob importers. Use distinctive names: `FortressResult`.

## Trait Implementation Pitfalls

- **PartialEq without reflexivity:** NaN breaks HashMap/BTreeSet.
- **Deref to different type:** Method resolution may surprise.

## Serde Pitfalls

- **`#[serde(default)]` on required fields:** Silently accepts missing data.
- **Enum representation:** Default serializes as `{"Active": null}`. Use `#[serde(rename_all = "snake_case")]`.

## Testing Pitfalls

- **Tests pass on panic:** Function panics before assertion is reached. Use `#[should_panic]` or `catch_unwind`.
- **Flaky timing tests:** Mock time or use generous bounds.

## Iterator and Performance Pitfalls

- **`collect()` then iterate:** Chain iterators instead. Clippy: `needless_collect`.
- **`filter().map()` with unwrap:** Use `filter_map` instead.
- **`cloned()` vs `copied()`:** Use `copied()` for Copy types. Clippy: `cloned_instead_of_copied`.
- **`Vec::remove()` in loop:** O(n^2). Use `retain()` or `swap_remove()`.
- **`clone()` in hot loop:** Borrow instead when possible.
- **`push()` loop without capacity:** Use `Vec::with_capacity()` or `extend()`.

## Doc Comments

Describe WHAT, not HOW -- unless performance guarantees are part of the API contract. Implementation details in comments become stale.

## Kani Proof Naming

Proofs must verify what they claim. If name says "independent", actually test modification independence.

## Review Checklist

- [ ] `as` casts that might truncate
- [ ] Float equality comparisons
- [ ] `Path::join` with potentially absolute paths
- [ ] HashMap iteration assumed ordered
- [ ] `unwrap()` on mutex locks
- [ ] `unwrap_or_default()` hiding errors
- [ ] `?` in Drop or some closures
- [ ] Serde `default` on required fields
- [ ] RwLock read guards held while acquiring write
- [ ] Spin loops without yield
- [ ] `std::` types in loom tests
- [ ] `ok_or` vs `ok_or_else` for allocating errors
- [ ] Pattern matching: `match` not `if let` when fallback needs value
