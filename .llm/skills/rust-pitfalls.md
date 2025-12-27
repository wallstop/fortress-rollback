# Common Rust Pitfalls — Bugs That Compile

> **This document catalogs subtle Rust pitfalls that compile but cause bugs.**
> These are patterns where Rust's safety guarantees don't protect you.

## Overview

Rust prevents memory unsafety, but it doesn't prevent logic bugs. These pitfalls compile successfully but produce unexpected behavior. Know them to avoid them.

---

## Numeric Pitfalls

### Silent Truncation with `as`

The `as` keyword silently truncates or wraps values:

```rust
// ❌ DANGEROUS - Silent truncation
let big: i64 = 300;
let small: i8 = big as i8;  // small is 44, not 300!

let negative: i32 = -1;
let unsigned: u32 = negative as u32;  // unsigned is 4294967295!

// ✅ SAFE - Explicit handling
let small = i8::try_from(big)
    .map_err(|_| Error::Overflow { value: big })?;

// ✅ OK - When truncation is intentional and documented
let low_byte: u8 = (value & 0xFF) as u8;  // Clearly extracting low byte
```

**Clippy lints:** `clippy::cast_possible_truncation`, `clippy::cast_sign_loss`, `clippy::cast_possible_wrap`

### Float Comparison

Floating point equality is unreliable:

```rust
// ❌ DANGEROUS - May fail unexpectedly
let a = 0.1 + 0.2;
let b = 0.3;
assert_eq!(a, b);  // FAILS! a is 0.30000000000000004

// ✅ SAFE - Compare with epsilon
fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() < epsilon
}
assert!(approx_eq(a, b, 1e-10));

// ✅ BETTER - Use a dedicated crate like `float-cmp` or `approx`
```

### Integer Division Truncates

```rust
// ⚠️ May surprise you
let result = 5 / 2;  // result is 2, not 2.5

// ✅ Be explicit about intent
let truncated = 5 / 2;          // 2 - integer division
let rounded = (5 + 1) / 2;      // 3 - round up
let float_div = 5.0 / 2.0;      // 2.5 - floating point
```

---

## String and Path Pitfalls

### UTF-8 Indexing

Strings are UTF-8, not arrays of characters:

```rust
// ❌ PANICS - Byte index into multi-byte char
let s = "héllo";
let c = &s[0..2];  // Panics if slicing mid-character!

// ✅ SAFE - Use char iterators
let first_char = s.chars().next();
let first_n: String = s.chars().take(3).collect();

// ✅ SAFE - Use byte slicing only when you control the content
let ascii = "hello";
let slice = &ascii[0..3];  // OK - ASCII is single-byte
```

### Path::join Replaces Base

```rust
// ⚠️ SURPRISING - Absolute path replaces base entirely!
use std::path::Path;

let base = Path::new("/home/user/data");
let result = base.join("/etc/passwd");
// result is "/etc/passwd", NOT "/home/user/data/etc/passwd"!

// ✅ SAFE - Validate relative paths
fn safe_join(base: &Path, relative: &Path) -> Result<PathBuf, Error> {
    if relative.is_absolute() {
        return Err(Error::AbsolutePathRejected);
    }
    // Also check for path traversal
    if relative.components().any(|c| c == Component::ParentDir) {
        return Err(Error::PathTraversalRejected);
    }
    Ok(base.join(relative))
}
```

**Clippy lint:** `clippy::join_absolute_paths`

### OsStr/OsString Are Not UTF-8

```rust
// ⚠️ File paths may not be valid UTF-8
let path: &Path = get_path();
let name = path.file_name()
    .and_then(|n| n.to_str())  // Returns None if not UTF-8!
    .ok_or(Error::InvalidFileName)?;

// ✅ Handle non-UTF-8 explicitly
let name = path.file_name()
    .map(|n| n.to_string_lossy())  // Replaces invalid UTF-8 with �
    .ok_or(Error::NoFileName)?;
```

---

## Collection Pitfalls

### HashMap Iteration Order

`HashMap` iteration order is not deterministic:

```rust
// ❌ NONDETERMINISTIC - Order varies between runs
let mut map = HashMap::new();
map.insert("a", 1);
map.insert("b", 2);
for (k, v) in &map {
    println!("{}: {}", k, v);  // Order not guaranteed!
}

// ✅ DETERMINISTIC - Use BTreeMap or sort
use std::collections::BTreeMap;
let map: BTreeMap<_, _> = [("a", 1), ("b", 2)].into_iter().collect();
// BTreeMap iterates in key order

// ✅ Or sort HashMap keys
let mut keys: Vec<_> = map.keys().collect();
keys.sort();
for k in keys { /* ... */ }
```

### Vec::drain Range Panics

```rust
// ❌ PANICS - Range out of bounds
let mut v = vec![1, 2, 3];
let drained: Vec<_> = v.drain(0..10).collect();  // Panics!

// ✅ SAFE - Clamp to valid range
let end = v.len().min(10);
let drained: Vec<_> = v.drain(0..end).collect();
```

### Collecting into Result

```rust
// ✅ Elegant pattern - stops on first error
let results: Result<Vec<_>, _> = items
    .iter()
    .map(|item| process(item))
    .collect();

// But beware: all items are processed even if early one fails
// For short-circuit, use explicit loop or try_fold
```

---

## Concurrency Pitfalls

### Mutex Poisoning

A mutex becomes poisoned if a thread panics while holding it:

```rust
// ❌ PANICS if mutex was poisoned
let data = mutex.lock().unwrap();

// ✅ HANDLE poisoning explicitly
let data = mutex.lock().unwrap_or_else(|poisoned| {
    // Decide: recover the data or propagate error
    poisoned.into_inner()  // Get data anyway
});

// ✅ OR propagate as error
let data = mutex.lock()
    .map_err(|_| Error::MutexPoisoned)?;
```

### Arc::clone Is Cheap, Cloning Inner Is Not

```rust
// ✅ Cheap - just increments reference count
let arc2 = Arc::clone(&arc1);

// ❌ EXPENSIVE - clones the inner data
let arc2 = Arc::new((*arc1).clone());

// ⚠️ Watch out for this pattern
let arc: Arc<Mutex<Vec<Large>>> = /* ... */;
let guard = arc.lock().unwrap();
let cloned = guard.clone();  // Clones the Vec, not the Arc!
```

---

## Error Handling Pitfalls

### Silent Error Conversion

```rust
// ❌ DANGEROUS - Error context lost
fn process() -> Result<(), Error> {
    let data = read_file()?;  // io::Error converted to Error
    parse(data)?;              // ParseError converted to Error
    Ok(())
}
// If it fails, which operation failed?

// ✅ BETTER - Preserve context
fn process() -> Result<(), Error> {
    let data = read_file()
        .map_err(|e| Error::ReadFailed { path: path.clone(), source: e })?;
    parse(data)
        .map_err(|e| Error::ParseFailed { source: e })?;
    Ok(())
}
```

### unwrap_or Hides Errors

```rust
// ❌ SUSPICIOUS - Why did it fail?
let config = load_config().unwrap_or_default();

// ✅ EXPLICIT - Log or handle the error
let config = match load_config() {
    Ok(c) => c,
    Err(e) => {
        tracing::warn!("Config load failed, using defaults: {e}");
        Config::default()
    }
};
```

### `?` in Drop

The `?` operator doesn't work in `Drop::drop`:

```rust
impl Drop for Resource {
    fn drop(&mut self) {
        // ❌ CANNOT use ? - drop returns ()
        // self.cleanup()?;

        // ✅ Handle errors explicitly
        if let Err(e) = self.cleanup() {
            // Log, ignore, or store for later
            eprintln!("Cleanup failed: {e}");
        }
    }
}
```

---

## Trait Implementation Pitfalls

### PartialEq Without Reflexivity

```rust
// ❌ BROKEN PartialEq - violates reflexivity (a == a should be true)
impl PartialEq for FloatWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0  // NaN != NaN, so NaN wrapper != itself!
    }
}

// This breaks HashMap, BTreeSet, etc. that assume reflexivity
```

### Clone Without Copy Semantics

```rust
// ⚠️ CONFUSING - Clone is expensive, but type looks "small"
#[derive(Clone)]
struct Handle {
    inner: Arc<Mutex<HugeState>>,
}

// Cloning Handle is cheap (Arc clone), but users might not know that
// Consider documenting or providing explicit clone_ref() method
```

### Deref to Different Type

```rust
// ⚠️ SURPRISING - Method resolution may not do what you expect
struct Wrapper(String);

impl Deref for Wrapper {
    type Target = str;
    fn deref(&self) -> &str { &self.0 }
}

let w = Wrapper("hello".into());
w.len();  // Calls str::len, not String::len - they're the same, but...
```

---

## Macro Pitfalls

### Macro Hygiene Leaks

```rust
// ⚠️ Macros can capture variables unexpectedly
macro_rules! compute {
    ($x:expr) => {
        {
            let temp = expensive_computation();
            $x + temp  // temp might conflict with caller's variable
        }
    };
}

let temp = 5;
compute!(temp);  // Which temp?
```

### Debug Assertions in Release

```rust
// ⚠️ debug_assert! is removed in release builds
debug_assert!(important_invariant());  // Not checked in release!

// ✅ Use assert! for critical invariants, debug_assert! only for expensive checks
assert!(critical_safety_invariant());
debug_assert!(expensive_consistency_check());
```

---

## Serde Pitfalls

### Default for Missing Fields

```rust
// ❌ Accepts missing required fields silently
#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    api_key: String,  // Deserializes to "" if missing - is that valid?
}

// ✅ Make required fields required
#[derive(Deserialize)]
struct Config {
    api_key: String,  // Error if missing
    #[serde(default = "default_timeout")]
    timeout: Duration,  // Has sensible default
}
```

### Enum Representation

```rust
// ⚠️ Default enum serialization may not be what you want
#[derive(Serialize, Deserialize)]
enum Status { Active, Inactive }

// Serializes as {"Active": null} or {"Inactive": null}

// ✅ Use explicit representation
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Active,
    Inactive,
}
// Now serializes as "active" or "inactive"
```

---

## Testing Pitfalls

### Tests Pass on Panic

```rust
// ❌ Test passes because it panics before assertion!
#[test]
fn test_something() {
    let result = function_that_panics();
    assert!(result.is_ok());  // Never reached
}

// ✅ Use should_panic for panic tests
#[test]
#[should_panic(expected = "invalid input")]
fn test_panic_on_invalid() {
    function_that_panics();
}

// ✅ Or catch_unwind for more control
let result = std::panic::catch_unwind(|| function_that_panics());
assert!(result.is_err());
```

### Flaky Tests from Timing

```rust
// ❌ FLAKY - Depends on system timing
#[test]
fn test_timeout() {
    let start = Instant::now();
    operation_with_timeout();
    assert!(start.elapsed() < Duration::from_millis(100));  // May fail under load
}

// ✅ Mock time or use generous bounds
// Or use tokio::time::pause() for async tests
```

---

## Summary Checklist

When reviewing code, watch for:

- [ ] `as` casts that might truncate or change sign
- [ ] Float equality comparisons
- [ ] `Path::join` with potentially absolute paths
- [ ] HashMap/HashSet iteration assumed to be ordered
- [ ] `unwrap()` on mutex locks (poisoning)
- [ ] `unwrap_or_default()` hiding meaningful errors
- [ ] `?` in contexts where it doesn't work (Drop, some closures)
- [ ] PartialEq implementations that aren't reflexive
- [ ] Serde `#[serde(default)]` on fields that shouldn't be empty
- [ ] Tests that might pass due to panics

---

*See also: [defensive-programming.md](defensive-programming.md) for zero-panic patterns.*
