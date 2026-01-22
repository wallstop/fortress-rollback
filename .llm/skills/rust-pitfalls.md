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

### RwLock Read-Before-Write Deadlock

```rust
// ❌ DEADLOCK - Holding read lock while acquiring write lock
fn update_if_needed(lock: &RwLock<Data>) {
    let guard = lock.read().unwrap();
    if needs_update(&guard) {
        // Still holding read lock!
        let mut write_guard = lock.write().unwrap();  // DEADLOCK
        update(&mut write_guard);
    }
}

// ✅ SAFE - Drop read lock first
fn update_if_needed(lock: &RwLock<Data>) {
    let needs_write = {
        let guard = lock.read().unwrap();
        needs_update(&guard)
    };  // Read lock dropped here

    if needs_write {
        let mut guard = lock.write().unwrap();
        // Re-check under write lock (another thread may have updated)
        if needs_update(&guard) {
            update(&mut guard);
        }
    }
}
```

### Spin Loops Without Yield

```rust
// ❌ DANGEROUS - Burns CPU, unfair to other threads, breaks loom testing
fn wait_for_flag(flag: &AtomicBool) {
    while !flag.load(Ordering::Acquire) {
        // Hot loop consuming 100% CPU
    }
}

// ✅ BETTER - CPU hint for spin loops
fn wait_for_flag(flag: &AtomicBool) {
    while !flag.load(Ordering::Acquire) {
        std::hint::spin_loop();  // Hints to CPU to save power
    }
}

// ✅ BEST for loom testing - explicit yield
fn wait_for_flag_loom_compatible(flag: &AtomicBool) {
    while !flag.load(Ordering::Acquire) {
        #[cfg(loom)]
        loom::thread::yield_now();  // Required for loom to make progress

        #[cfg(not(loom))]
        std::hint::spin_loop();
    }
}

// ✅ BEST for production - Use proper synchronization
use parking_lot::{Mutex, Condvar};

fn wait_for_condition(pair: &(Mutex<bool>, Condvar)) {
    let (lock, cvar) = pair;
    let mut started = lock.lock();
    while !*started {
        cvar.wait(&mut started);  // Sleeps until signaled
    }
}
```

### Using std Types Under Loom

```rust
// ❌ INVISIBLE TO LOOM - Operations not tracked
#[cfg(loom)]
fn broken_loom_test() {
    use std::sync::Arc;  // Wrong! Should use loom::sync::Arc
    use std::thread;     // Wrong! Should use loom::thread

    loom::model(|| {
        let data = Arc::new(AtomicUsize::new(0));
        thread::spawn(/* ... */);  // Loom can't see this thread!
    });
}

// ✅ CORRECT - Use loom types in loom tests
#[cfg(loom)]
fn correct_loom_test() {
    use loom::sync::Arc;
    use loom::sync::atomic::AtomicUsize;
    use loom::thread;

    loom::model(|| {
        let data = Arc::new(AtomicUsize::new(0));
        let d = data.clone();
        thread::spawn(move || {
            d.fetch_add(1, Ordering::SeqCst);
        });
    });
}
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

### Arc Must Be Cloned BEFORE thread::spawn

When moving an `Arc` into a spawned thread, clone it BEFORE the `spawn` call:

```rust
// ❌ BAD: Arc moved into thread, original cannot be used after
let cell = Arc::new(GameStateCell::new());
let handle = thread::spawn(move || {
    cell.save(Frame::new(1), Some(42), None);  // cell moved here
});
handle.join().unwrap();
assert_eq!(cell.load(), Some(42));  // ERROR: cell was moved!

// ✅ GOOD: Clone Arc before spawning, keep original for later use
let cell = Arc::new(GameStateCell::new());
let cell_for_thread = cell.clone();  // Clone BEFORE spawn

let handle = thread::spawn(move || {
    cell_for_thread.save(Frame::new(1), Some(42), None);
});
handle.join().unwrap();
assert_eq!(cell.load(), Some(42));  // Original still available

// ✅ GOOD: Multiple threads, each gets its own clone
let data = Arc::new(Mutex::new(vec![]));
let data1 = data.clone();
let data2 = data.clone();

let t1 = thread::spawn(move || data1.lock().unwrap().push(1));
let t2 = thread::spawn(move || data2.lock().unwrap().push(2));

t1.join().unwrap();
t2.join().unwrap();

// Original Arc still accessible for verification
assert_eq!(data.lock().unwrap().len(), 2);
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

## Iterator and Performance Pitfalls

### collect() Then Iterate

```rust
// ❌ WASTEFUL - Allocates intermediate Vec
let result: Vec<_> = items.iter()
    .filter(|x| x.is_valid())
    .collect();
for item in result {
    process(item);
}

// ✅ EFFICIENT - Chain iterators
for item in items.iter().filter(|x| x.is_valid()) {
    process(item);
}
```

**Clippy lint:** `clippy::needless_collect`

### filter_map Instead of filter().map()

```rust
// ⚠️ Two passes
let valid: Vec<_> = items.iter()
    .filter(|x| x.value.is_some())
    .map(|x| x.value.unwrap())  // unwrap after filter - still a code smell
    .collect();

// ✅ Single pass, no unwrap
let valid: Vec<_> = items.iter()
    .filter_map(|x| x.value)
    .collect();
```

### find() Then get()

```rust
// ❌ REDUNDANT - find already returns the value
let idx = items.iter().position(|x| x.id == target_id);
let item = idx.map(|i| &items[i]);

// ✅ DIRECT
let item = items.iter().find(|x| x.id == target_id);
```

### cloned() vs copied()

```rust
// ⚠️ cloned() works but implies Clone trait (may be expensive)
let bytes: Vec<u8> = source.iter().cloned().collect();

// ✅ copied() is explicit about Copy semantics (cheap)
let bytes: Vec<u8> = source.iter().copied().collect();
```

**Clippy lint:** `clippy::cloned_instead_of_copied`

### Inefficient String Building

```rust
// ❌ SLOW - Many allocations
let mut result = String::new();
for item in items {
    result = result + &item.name + ", ";
}

// ✅ FAST - Pre-allocate and push
let mut result = String::with_capacity(items.len() * 20);
for item in items {
    result.push_str(&item.name);
    result.push_str(", ");
}

// ✅ BETTER - Use join
let result = items.iter()
    .map(|item| item.name.as_str())
    .collect::<Vec<_>>()
    .join(", ");
```

### Vec::remove() in Loop

```rust
// ❌ O(n²) - Each remove shifts remaining elements
fn remove_invalids(items: &mut Vec<Item>) {
    let mut i = 0;
    while i < items.len() {
        if !items[i].valid {
            items.remove(i);  // O(n) shift
        } else {
            i += 1;
        }
    }
}

// ✅ O(n) - retain is optimized
fn remove_invalids(items: &mut Vec<Item>) {
    items.retain(|item| item.valid);
}

// ✅ O(1) per removal if order doesn't matter
fn remove_invalids_unordered(items: &mut Vec<Item>) {
    let mut i = 0;
    while i < items.len() {
        if !items[i].valid {
            items.swap_remove(i);  // O(1) - swaps with last
        } else {
            i += 1;
        }
    }
}
```

### clone() in Hot Loop

```rust
// ❌ EXPENSIVE - Clones String each iteration
for item in &items {
    let name = item.name.clone();
    if cache.contains_key(&name) {  // Could borrow instead
        // ...
    }
}

// ✅ EFFICIENT - Borrow where possible
for item in &items {
    if cache.contains_key(&item.name) {  // Borrow, no clone
        // ...
    }
}
```

### extend() vs push() Loop

```rust
// ❌ MANY REALLOCATIONS - Each push may resize
let mut result = Vec::new();
for item in source {
    result.push(item.clone());
}

// ✅ SINGLE ALLOCATION
let mut result = Vec::with_capacity(source.len());
result.extend(source.iter().cloned());

// ✅ OR collect directly
let result: Vec<_> = source.iter().cloned().collect();
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
- [ ] RwLock read guards held while acquiring write lock (deadlock)
- [ ] Spin loops without yield (breaks loom, wastes CPU)
- [ ] Using `std::sync`/`std::thread` instead of `loom::` in loom tests

---

*See also: [defensive-programming.md](defensive-programming.md) for zero-panic patterns, [loom-testing.md](loom-testing.md) for concurrency testing, [concurrency-patterns.md](concurrency-patterns.md) for thread-safe patterns.*
