# Rust Refactoring Guide — Performance and Quality Transformations

> **A comprehensive guide for automated code transformation** covering 25 refactoring patterns with before/after examples, verification techniques, and pitfall avoidance.

---

## Table of Contents

1. [Refactoring Philosophy](#refactoring-philosophy)
2. [Verification Framework](#verification-framework)
3. [Allocation Reduction Patterns (1-5)](#allocation-reduction-patterns)
4. [Iterator Optimization Patterns (6-10)](#iterator-optimization-patterns)
5. [Error Handling Patterns (11-15)](#error-handling-patterns)
6. [Lifetime and Borrowing Patterns (16-20)](#lifetime-and-borrowing-patterns)
7. [Type Safety Patterns (21-25)](#type-safety-patterns)
8. [Performance Measurement](#performance-measurement)
9. [Common Pitfalls](#common-pitfalls)
10. [Automated Transformation Checklist](#automated-transformation-checklist)

---

## Refactoring Philosophy

### The Three Rules of Safe Refactoring

1. **Preserve Semantics** — Behavior must remain identical unless explicitly changing it
2. **Incremental Steps** — Make one change at a time, verify, then proceed
3. **Test Coverage First** — Ensure tests exist before refactoring

### When to Refactor

| Signal | Action |
|--------|--------|
| `unwrap()`/`expect()` in production code | Convert to `Result` propagation |
| `.clone()` without clear necessity | Analyze ownership, use references |
| Manual loops with indexing | Convert to iterators |
| Primitive types for domain concepts | Wrap in newtypes |
| `Vec<T>` where `&[T]` suffices | Accept slices at boundaries |
| Large enum variants | Box the large variants |
| `HashMap`/`HashSet` without ordering needs | Consider faster hashers |

---

## Verification Framework

### Step-by-Step Verification Process

```bash
# 1. Ensure tests pass before refactoring
cargo nextest run

# 2. Make the refactoring change
# ... edit code ...

# 3. Verify compilation and type checking
cargo check

# 4. Run clippy to catch regressions
cargo clippy --all-targets

# 5. Run tests to verify behavior preservation
cargo nextest run

# 6. Run benchmarks if performance-related
cargo bench

# 7. For critical code, run property tests
cargo test --features proptest
```

### Verification Commands

```bash
# Quick check cycle (alias: cargo c)
cargo fmt && cargo clippy --all-targets && cargo nextest run

# Check for new clippy warnings
cargo clippy --all-targets -- -W clippy::pedantic 2>&1 | head -100

# Verify no new panicking code introduced
rg "unwrap\(\)|expect\(|panic!\(|todo!\(|unimplemented!\(" src/

# Check allocation patterns changed
cargo build --release && size target/release/your_binary
```

---

## Allocation Reduction Patterns

### Pattern 1: Clone to Reference

**Problem:** Unnecessary cloning when a reference suffices.

**Detection:** Clippy lint `clippy::needless_pass_by_value`

```rust
// ❌ BEFORE: Clones the string unnecessarily
fn process_name(name: String) -> usize {
    name.len()  // Only reads the string
}

// Caller must clone:
let result = process_name(my_string.clone());
```

```rust
// ✅ AFTER: Accepts reference, no allocation
fn process_name(name: &str) -> usize {
    name.len()
}

// Caller passes reference:
let result = process_name(&my_string);
```

**Transformation Steps:**
1. Change parameter type from `String` to `&str` (or `Vec<T>` to `&[T]`)
2. Update function body if it needs ownership (rarely)
3. Update all call sites to pass references
4. Remove `.clone()` calls at call sites

**Verification:** Compile, test, check no new `.clone()` needed at call sites.

---

### Pattern 2: Unnecessary `to_string()`/`to_owned()`

**Problem:** Converting to owned type when borrowed type works.

**Detection:** Clippy lints `clippy::unnecessary_to_owned`, `clippy::redundant_clone`

```rust
// ❌ BEFORE: Allocates string unnecessarily
fn find_user(name: &str) -> Option<User> {
    let owned = name.to_string();  // Unnecessary allocation
    users.get(&owned).cloned()
}

// ❌ BEFORE: Clone for comparison
fn matches(a: &String, b: &str) -> bool {
    a.clone() == b.to_string()  // Two allocations!
}
```

```rust
// ✅ AFTER: Direct comparison, no allocation
fn find_user(name: &str) -> Option<User> {
    users.get(name).cloned()  // HashMap<String, _> accepts &str via Borrow
}

// ✅ AFTER: Reference comparison
fn matches(a: &str, b: &str) -> bool {
    a == b  // Zero allocations
}
```

**Transformation Steps:**
1. Check if the owned value is actually needed
2. If only used for comparison/lookup, use the borrowed form directly
3. Leverage `Borrow` trait for HashMap/HashSet lookups

---

### Pattern 3: Pre-allocate Collections

**Problem:** Vec grows through multiple reallocations.

**Detection:** DHAT profiler, or pattern matching `Vec::new()` followed by push loops

```rust
// ❌ BEFORE: Multiple reallocations as vec grows
fn collect_items(count: usize) -> Vec<Item> {
    let mut items = Vec::new();
    for i in 0..count {
        items.push(create_item(i));  // May reallocate multiple times
    }
    items
}
```

```rust
// ✅ AFTER: Single allocation
fn collect_items(count: usize) -> Vec<Item> {
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        items.push(create_item(i));  // No reallocation
    }
    items
}

// ✅ EVEN BETTER: Use iterator collect
fn collect_items(count: usize) -> Vec<Item> {
    (0..count).map(create_item).collect()  // collect() uses size_hint
}
```

**Transformation Steps:**
1. Identify loops that push to a Vec
2. Determine if the final size is known or bounded
3. Use `with_capacity()` with the known/estimated size
4. Consider converting to iterator chain with `collect()`

---

### Pattern 4: Reuse Collections in Loops

**Problem:** Allocating new collections on each iteration.

**Detection:** Pattern `for ... { let mut vec = Vec::new(); ... }`

```rust
// ❌ BEFORE: Allocates new buffer each iteration
fn process_batches(batches: &[Batch]) -> Results {
    let mut results = Results::new();
    for batch in batches {
        let buffer: Vec<u8> = Vec::new();  // New allocation each time
        process_batch(batch, &mut buffer);
        results.add(&buffer);
    }
    results
}
```

```rust
// ✅ AFTER: Reuse buffer across iterations
fn process_batches(batches: &[Batch]) -> Results {
    let mut results = Results::new();
    let mut buffer: Vec<u8> = Vec::new();  // Allocate once
    for batch in batches {
        buffer.clear();  // Reuse capacity, clear content
        process_batch(batch, &mut buffer);
        results.add(&buffer);
    }
    results
}
```

**Transformation Steps:**
1. Move collection declaration outside the loop
2. Add `.clear()` at the start of each iteration
3. Optionally pre-size with `with_capacity()` based on typical size

---

### Pattern 5: `clone_from` Instead of `clone` Assignment

**Problem:** Assignment with clone discards existing allocation.

**Detection:** Pattern `x = y.clone()` where `x` already has capacity

```rust
// ❌ BEFORE: Discards x's existing allocation
let mut x: Vec<u32> = Vec::with_capacity(1000);
x.extend(0..100);
let y: Vec<u32> = (0..50).collect();

x = y.clone();  // Drops x's 1000-capacity buffer, allocates new ~50-capacity
```

```rust
// ✅ AFTER: Reuses x's allocation
let mut x: Vec<u32> = Vec::with_capacity(1000);
x.extend(0..100);
let y: Vec<u32> = (0..50).collect();

x.clone_from(&y);  // Reuses x's buffer if it has enough capacity
assert!(x.capacity() >= 1000);  // Capacity preserved!
```

**Transformation Steps:**
1. Identify `x = y.clone()` patterns
2. Replace with `x.clone_from(&y)`
3. Verify `x` implements `Clone` (which provides `clone_from`)

---

## Iterator Optimization Patterns

### Pattern 6: Index Loop to Iterator

**Problem:** Manual indexing is verbose and can panic.

**Detection:** Pattern `for i in 0..collection.len()` with `collection[i]`

```rust
// ❌ BEFORE: Manual indexing, can panic if modified
fn sum_values(items: &[Item]) -> i64 {
    let mut sum = 0;
    for i in 0..items.len() {
        sum += items[i].value;  // Bounds check on each access
    }
    sum
}
```

```rust
// ✅ AFTER: Iterator, no bounds checks, idiomatic
fn sum_values(items: &[Item]) -> i64 {
    items.iter().map(|item| item.value).sum()
}

// Or if you need the index:
fn sum_with_index(items: &[Item]) -> i64 {
    items.iter()
        .enumerate()
        .map(|(i, item)| item.value * i as i64)
        .sum()
}
```

**Transformation Steps:**
1. Replace `for i in 0..len` with `.iter()` or `.iter_mut()`
2. Replace `collection[i]` with the iterator variable
3. Use `.enumerate()` if index is needed
4. Chain with `.map()`, `.filter()`, `.fold()` as needed

---

### Pattern 7: Avoid Intermediate `collect()`

**Problem:** Collecting into Vec just to iterate again.

**Detection:** Pattern `.collect::<Vec<_>>()` followed by `.iter()` or `for`

```rust
// ❌ BEFORE: Unnecessary intermediate allocation
fn get_valid_items(items: &[Item]) -> Vec<&Item> {
    items.iter().filter(|i| i.is_valid()).collect()
}

// Later:
for item in get_valid_items(&items) {
    process(item);
}
```

```rust
// ✅ AFTER: Return iterator, no allocation
fn get_valid_items(items: &[Item]) -> impl Iterator<Item = &Item> {
    items.iter().filter(|i| i.is_valid())
}

// Or inline the iterator:
for item in items.iter().filter(|i| i.is_valid()) {
    process(item);
}
```

**Transformation Steps:**
1. Change return type from `Vec<T>` to `impl Iterator<Item = T>`
2. Remove `.collect()` from the function
3. If needed, add lifetime: `impl Iterator<Item = &'a T> + 'a`

**Caveat:** Cannot return `impl Iterator` from trait methods; use `Box<dyn Iterator>` or collect in those cases.

---

### Pattern 8: `chunks_exact` Over `chunks`

**Problem:** `chunks()` generates code to handle remainders on every iteration.

**Detection:** Usage of `.chunks(n)` in hot paths

```rust
// ❌ BEFORE: Compiler must handle variable-size chunks
fn process_pairs(data: &[u8]) {
    for chunk in data.chunks(2) {
        if chunk.len() == 2 {
            process_pair(chunk[0], chunk[1]);
        }
    }
}
```

```rust
// ✅ AFTER: Compiler knows exact size, better codegen
fn process_pairs(data: &[u8]) {
    for chunk in data.chunks_exact(2) {
        // chunk is guaranteed to be exactly 2 elements
        process_pair(chunk[0], chunk[1]);
    }
    // Handle remainder separately if needed
    let remainder = data.chunks_exact(2).remainder();
    if !remainder.is_empty() {
        handle_remainder(remainder);
    }
}
```

**Transformation Steps:**
1. Replace `.chunks(n)` with `.chunks_exact(n)`
2. Handle `.remainder()` separately if the remainder matters
3. If remainder should be an error, validate input length first

---

### Pattern 9: `iter().copied()` for Small Copy Types

**Problem:** Iterating references to small Copy types may generate suboptimal code.

**Detection:** Pattern `iter().map(|x| *x)` or iterator of `&u8`, `&u32`, etc.

```rust
// ❌ BEFORE: May dereference repeatedly
fn sum_bytes(data: &[u8]) -> u64 {
    data.iter().map(|&b| b as u64).sum()
}
```

```rust
// ✅ AFTER: Explicit copy, often better codegen
fn sum_bytes(data: &[u8]) -> u64 {
    data.iter().copied().map(|b| b as u64).sum()
}

// Or for simple cases:
fn sum_bytes(data: &[u8]) -> u64 {
    data.iter().map(|&b| u64::from(b)).sum()
}
```

**Transformation Steps:**
1. Add `.copied()` after `.iter()` for `Copy` types
2. Remove explicit dereference in subsequent closures
3. Use `|&x|` pattern if preferred for clarity

---

### Pattern 10: `swap_remove` for Unordered Removal

**Problem:** `remove()` is O(n), shifts all subsequent elements.

**Detection:** `.remove(index)` where order doesn't matter

```rust
// ❌ BEFORE: O(n) removal
fn remove_by_id(items: &mut Vec<Item>, id: u64) {
    if let Some(index) = items.iter().position(|i| i.id == id) {
        items.remove(index);  // Shifts all elements after index
    }
}
```

```rust
// ✅ AFTER: O(1) removal (order not preserved)
fn remove_by_id(items: &mut Vec<Item>, id: u64) {
    if let Some(index) = items.iter().position(|i| i.id == id) {
        items.swap_remove(index);  // Swaps with last, pops
    }
}
```

**Transformation Steps:**
1. Verify that element order doesn't matter
2. Replace `.remove(index)` with `.swap_remove(index)`
3. Document that order is not preserved if it's a public API

**Caveat:** Only use when order doesn't matter; test behavior if order was implicitly relied upon.

---

## Error Handling Patterns

### Pattern 11: `unwrap()` to `?` Propagation

**Problem:** `unwrap()` panics on None/Err.

**Detection:** Clippy lint `clippy::unwrap_used`

```rust
// ❌ BEFORE: Panics on None or Err
fn get_user_name(id: u64) -> String {
    let user = database.find_user(id).unwrap();  // Panics if not found
    user.name.clone()
}
```

```rust
// ✅ AFTER: Propagates error to caller
fn get_user_name(id: u64) -> Result<String, UserError> {
    let user = database.find_user(id)
        .ok_or(UserError::NotFound { id })?;
    Ok(user.name.clone())
}
```

**Transformation Steps:**
1. Change return type to `Result<T, E>` or `Option<T>`
2. Replace `.unwrap()` with `?` for propagation
3. Add `.ok_or()` or `.ok_or_else()` to convert `Option` to `Result` with context
4. Wrap success value in `Ok()`
5. Update all callers to handle the Result

---

### Pattern 12: `expect()` to Contextual Error

**Problem:** `expect()` panics with a message but doesn't allow recovery.

**Detection:** Clippy lint `clippy::expect_used`

```rust
// ❌ BEFORE: Panics with message
fn load_config() -> Config {
    let content = fs::read_to_string("config.toml")
        .expect("config file should exist");
    toml::from_str(&content)
        .expect("config should be valid TOML")
}
```

```rust
// ✅ AFTER: Rich error types with context
fn load_config() -> Result<Config, ConfigError> {
    let content = fs::read_to_string("config.toml")
        .map_err(|e| ConfigError::ReadFailed { 
            path: "config.toml".into(), 
            source: e 
        })?;
    toml::from_str(&content)
        .map_err(|e| ConfigError::ParseFailed { 
            path: "config.toml".into(), 
            source: e 
        })
}
```

**Transformation Steps:**
1. Define or use appropriate error types
2. Replace `.expect(msg)` with `.map_err(|e| Error::Variant { context })?`
3. Include relevant context (file paths, IDs, etc.) in error variants
4. Update return type to Result

---

### Pattern 13: Index Access to Safe `get()`

**Problem:** `collection[index]` panics on out-of-bounds.

**Detection:** Clippy lint `clippy::indexing_slicing`

```rust
// ❌ BEFORE: Can panic
fn get_player(players: &[Player], index: usize) -> &Player {
    &players[index]  // Panics if index >= players.len()
}
```

```rust
// ✅ AFTER Option: Returns Option
fn get_player(players: &[Player], index: usize) -> Option<&Player> {
    players.get(index)
}

// ✅ AFTER Result: Returns Result with context
fn get_player(players: &[Player], index: usize) -> Result<&Player, PlayerError> {
    players.get(index)
        .ok_or(PlayerError::InvalidIndex { 
            index, 
            player_count: players.len() 
        })
}
```

**Transformation Steps:**
1. Replace `collection[index]` with `collection.get(index)`
2. Handle the resulting `Option`:
   - Use `?` with `.ok_or()` for Result propagation
   - Use `.ok_or_else(|| ...)` for lazy error construction
   - Use `if let Some(x) = ...` for conditional handling

---

### Pattern 14: Silent Error Swallowing to Explicit Handling

**Problem:** Errors ignored with `let _ = ...` or `if let Ok(...)`.

**Detection:** Pattern `let _ = result` or `if let Ok(x) = ...` without else

```rust
// ❌ BEFORE: Silently ignores errors
fn try_save(data: &Data) {
    let _ = fs::write("data.json", serde_json::to_string(data).unwrap());
}

// ❌ BEFORE: Only handles success case
fn maybe_process(item: Result<Item, Error>) {
    if let Ok(item) = item {
        process(item);
    }
    // What about Err? Silently ignored!
}
```

```rust
// ✅ AFTER: Propagate or handle explicitly
fn try_save(data: &Data) -> Result<(), SaveError> {
    let json = serde_json::to_string(data)
        .map_err(|e| SaveError::Serialize(e))?;
    fs::write("data.json", json)
        .map_err(|e| SaveError::Write(e))?;
    Ok(())
}

// ✅ AFTER: Handle both cases
fn process_result(item: Result<Item, Error>) -> Result<(), Error> {
    match item {
        Ok(item) => {
            process(item);
            Ok(())
        }
        Err(e) => {
            log::warn!("Item processing failed: {e}");
            Err(e)  // Or handle differently
        }
    }
}
```

**Transformation Steps:**
1. Identify `let _ = result` patterns
2. Decide: propagate with `?`, handle explicitly, or log
3. If intentionally ignoring, document why: `let _ = result; // Intentionally ignored: reason`
4. For `if let Ok`, add an else branch that handles or propagates the error

---

### Pattern 15: `unwrap_or` to `unwrap_or_else` for Expensive Defaults

**Problem:** `unwrap_or(expensive())` always evaluates the default.

**Detection:** Clippy lint `clippy::or_fun_call`

```rust
// ❌ BEFORE: compute_default() called even when value exists
fn get_value(cache: &HashMap<Key, Value>, key: &Key) -> Value {
    cache.get(key).cloned().unwrap_or(compute_default())
}
```

```rust
// ✅ AFTER: compute_default() only called when needed
fn get_value(cache: &HashMap<Key, Value>, key: &Key) -> Value {
    cache.get(key).cloned().unwrap_or_else(|| compute_default())
}

// Same applies to:
// .ok_or() -> .ok_or_else()
// .map_or() -> .map_or_else()
// .or() -> .or_else()
```

**Transformation Steps:**
1. Identify expensive expressions in `unwrap_or()`, `ok_or()`, `map_or()`, `or()`
2. Wrap in closure: `unwrap_or(x)` → `unwrap_or_else(|| x)`
3. For cheap constants (literals, Copy types), `unwrap_or` is fine

---

## Lifetime and Borrowing Patterns

### Pattern 16: `&String`/`&Vec<T>` to `&str`/`&[T]`

**Problem:** Overly restrictive parameter types.

**Detection:** Clippy lint `clippy::ptr_arg`

```rust
// ❌ BEFORE: Only accepts &String
fn print_greeting(name: &String) {
    println!("Hello, {}!", name);
}

// ❌ BEFORE: Only accepts &Vec<i32>
fn sum(numbers: &Vec<i32>) -> i32 {
    numbers.iter().sum()
}
```

```rust
// ✅ AFTER: Accepts &String, &str, and string literals
fn print_greeting(name: &str) {
    println!("Hello, {}!", name);
}

// ✅ AFTER: Accepts &Vec<i32>, &[i32], and arrays
fn sum(numbers: &[i32]) -> i32 {
    numbers.iter().sum()
}

// Usage - all work:
print_greeting("literal");
print_greeting(&String::from("owned"));
sum(&vec![1, 2, 3]);
sum(&[1, 2, 3]);
```

**Transformation Steps:**
1. Change `&String` to `&str`
2. Change `&Vec<T>` to `&[T]`
3. Function body usually needs no changes (deref coercion)
4. Callers that pass `&String`/`&Vec<T>` continue to work

---

### Pattern 17: Return Borrowed to Return Owned

**Problem:** Returning reference requires lifetime, but ownership is clearer.

**Detection:** Functions returning references with complex lifetime bounds

```rust
// ❌ BEFORE: Complex lifetime, limits caller
fn get_name<'a>(&'a self) -> &'a str {
    &self.name
}

// Caller can't store it beyond self's lifetime
let name = obj.get_name();  // Borrows obj
drop(obj);  // Error: name still borrowed
```

```rust
// ✅ AFTER: Returns owned, caller controls lifetime
fn get_name(&self) -> String {
    self.name.clone()
}

// Or use Cow for efficiency
fn get_name(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.name)
}
```

**When to Apply:**
- Callers often need to store the result
- The clone is cheap (short strings, small types)
- Lifetime complexity hurts API ergonomics

**When NOT to Apply:**
- Hot paths where cloning is measurable
- Large data that shouldn't be copied
- Internal APIs where lifetime tracking is acceptable

---

### Pattern 18: `mem::take` for Ownership from `&mut`

**Problem:** Need to take ownership from `&mut` without clone.

**Detection:** Pattern involving `.clone()` just to get ownership from mutable reference

```rust
// ❌ BEFORE: Clones to get ownership
fn process_and_clear(data: &mut Vec<Item>) -> ProcessedData {
    let owned = data.clone();  // Unnecessary clone
    data.clear();
    ProcessedData::from(owned)
}
```

```rust
// ✅ AFTER: Takes ownership, replaces with default
use std::mem;

fn process_and_clear(data: &mut Vec<Item>) -> ProcessedData {
    let owned = mem::take(data);  // data becomes Vec::new()
    ProcessedData::from(owned)
}

// For non-Default types, use mem::replace:
fn process_and_replace(data: &mut State) -> State {
    mem::replace(data, State::Empty)
}
```

**Transformation Steps:**
1. Import `std::mem`
2. Replace `x.clone()` + clear pattern with `mem::take(&mut x)`
3. For non-Default types, use `mem::replace(&mut x, replacement_value)`

---

### Pattern 19: Struct Decomposition for Split Borrowing

**Problem:** Can't borrow multiple fields mutably through methods.

**Detection:** Borrow checker errors about conflicting borrows on `&mut self`

```rust
// ❌ BEFORE: Can't borrow fields independently through methods
struct Game {
    state: State,
    renderer: Renderer,
}

impl Game {
    fn update(&mut self) {
        // Error: can't call both, each takes &mut self
        self.update_state();
        self.render_state();
    }
    
    fn update_state(&mut self) { /* uses self.state */ }
    fn render_state(&mut self) { /* uses self.renderer and self.state */ }
}
```

```rust
// ✅ AFTER: Access fields directly for split borrowing
impl Game {
    fn update(&mut self) {
        // Direct field access allows split borrowing
        State::update(&mut self.state);
        Renderer::render(&mut self.renderer, &self.state);
    }
}

// Or use a helper method that returns mutable references to multiple fields
impl Game {
    fn parts_mut(&mut self) -> (&mut State, &mut Renderer) {
        (&mut self.state, &mut self.renderer)
    }
    
    fn update(&mut self) {
        let (state, renderer) = self.parts_mut();
        state.tick();
        renderer.draw(state);
    }
}
```

**Transformation Steps:**
1. Identify methods that need simultaneous access to multiple fields
2. Either access fields directly in the calling code
3. Or create a helper that returns a tuple of mutable references
4. Update methods to take specific fields rather than `&mut self`

---

### Pattern 20: Cow for Conditional Ownership

**Problem:** Sometimes need owned data, sometimes borrowed.

**Detection:** Functions that clone conditionally, or APIs that might return static or dynamic data

```rust
// ❌ BEFORE: Always allocates
fn normalize_path(path: &str) -> String {
    if path.contains("..") {
        resolve_path(path)  // Returns String
    } else {
        path.to_string()    // Unnecessary allocation
    }
}
```

```rust
// ✅ AFTER: Borrows when possible, owns when needed
use std::borrow::Cow;

fn normalize_path(path: &str) -> Cow<'_, str> {
    if path.contains("..") {
        Cow::Owned(resolve_path(path))
    } else {
        Cow::Borrowed(path)  // No allocation
    }
}

// Usage is transparent:
let normalized = normalize_path(input);
println!("{}", normalized);  // Works for both Borrowed and Owned
```

**Transformation Steps:**
1. Change return type from `String` to `Cow<'_, str>` (or `Vec<T>` to `Cow<'_, [T]>`)
2. Wrap borrowed returns in `Cow::Borrowed(x)`
3. Wrap owned returns in `Cow::Owned(x)`
4. Callers can use result as `&str` via deref, or call `.into_owned()` if they need ownership

---

## Type Safety Patterns

### Pattern 21: Primitive to Newtype

**Problem:** Primitives don't carry domain meaning; easy to mix up.

**Detection:** Functions with multiple parameters of same primitive type

```rust
// ❌ BEFORE: Easy to mix up parameters
fn transfer(from: u64, to: u64, amount: u64) { /* ... */ }

// Called with wrong order - compiles fine!
transfer(amount, from_account, to_account);  // Bug!
```

```rust
// ✅ AFTER: Type-safe, self-documenting
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AccountId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount(u64);

fn transfer(from: AccountId, to: AccountId, amount: Amount) { /* ... */ }

// Wrong order is now a compile error:
// transfer(amount, from_account, to_account);  // Error: type mismatch
```

**Transformation Steps:**
1. Create newtype: `struct TypeName(InnerType);`
2. Derive useful traits: `Clone, Copy, Debug, PartialEq, Eq, Hash`
3. Add constructor and accessor methods if needed
4. Update function signatures to use the newtype
5. Update call sites to wrap values in the newtype

---

### Pattern 22: Boolean Parameters to Enums

**Problem:** Boolean parameters are unclear at call sites.

**Detection:** Functions with multiple `bool` parameters

```rust
// ❌ BEFORE: What do these booleans mean?
fn connect(host: &str, secure: bool, verify: bool) { /* ... */ }

connect("example.com", true, false);  // Unclear!
```

```rust
// ✅ AFTER: Self-documenting enums
#[derive(Clone, Copy, Debug)]
pub enum Security { Secure, Insecure }

#[derive(Clone, Copy, Debug)]
pub enum CertVerification { Verify, Skip }

fn connect(host: &str, security: Security, verification: CertVerification) { /* ... */ }

connect("example.com", Security::Secure, CertVerification::Skip);  // Clear!
```

**Transformation Steps:**
1. Create an enum for each boolean with meaningful variant names
2. Replace `bool` parameters with the enum type
3. Update all call sites to use enum variants
4. Update function body: replace `if flag` with `match flag { ... }`

---

### Pattern 23: Option Fields to Separate Types (Parse, Don't Validate)

**Problem:** Option fields require checks throughout the codebase.

**Detection:** Repeated `if field.is_some()` or `field.as_ref().unwrap()` patterns

```rust
// ❌ BEFORE: Must check everywhere
struct User {
    name: String,
    email: Option<String>,  // Verified users have email
}

fn send_notification(user: &User) {
    if let Some(email) = &user.email {
        send_email(email, "Hello!");
    }
    // What if we forget the check somewhere?
}
```

```rust
// ✅ AFTER: Type encodes the invariant
struct UnverifiedUser {
    name: String,
}

struct VerifiedUser {
    name: String,
    email: String,  // Always present for verified users
}

fn send_notification(user: &VerifiedUser) {
    // No check needed - type guarantees email exists
    send_email(&user.email, "Hello!");
}

// Verification is explicit state transition
fn verify(user: UnverifiedUser, email: String) -> VerifiedUser {
    VerifiedUser { name: user.name, email }
}
```

**Transformation Steps:**
1. Identify Option fields that represent different states
2. Create separate types for each state
3. Make state transitions explicit functions
4. Update code to use the appropriate type at each point

---

### Pattern 24: String Status to Enum

**Problem:** String-typed status allows invalid values.

**Detection:** Fields like `status: String` with pattern matching on string values

```rust
// ❌ BEFORE: Any string is valid, typos compile
struct Task {
    status: String,
}

fn is_done(task: &Task) -> bool {
    task.status == "complete"  // What about "completed"? "COMPLETE"?
}
```

```rust
// ✅ AFTER: Only valid states exist
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Complete,
    Failed,
}

struct Task {
    status: TaskStatus,
}

fn is_done(task: &Task) -> bool {
    task.status == TaskStatus::Complete
}
```

**Transformation Steps:**
1. Identify all valid string values
2. Create enum with variants for each value
3. Replace `String` field with enum type
4. Update all comparisons to use enum variants
5. Add `From<&str>` or `TryFrom<&str>` for deserialization if needed

---

### Pattern 25: Unsafe Index Arithmetic to Checked Operations

**Problem:** Arithmetic on indices can overflow or underflow.

**Detection:** Clippy lint `clippy::arithmetic_side_effects`, or index arithmetic patterns

```rust
// ❌ BEFORE: Can overflow/underflow
fn window_around(data: &[u8], center: usize, radius: usize) -> &[u8] {
    let start = center - radius;  // Underflow if center < radius!
    let end = center + radius + 1;  // Overflow possible!
    &data[start..end]  // Can panic
}
```

```rust
// ✅ AFTER: Checked arithmetic, explicit error handling
fn window_around(data: &[u8], center: usize, radius: usize) -> Option<&[u8]> {
    let start = center.checked_sub(radius)?;
    let end = center.checked_add(radius)?.checked_add(1)?;
    data.get(start..end)
}

// Or with Result for better error messages:
fn window_around(data: &[u8], center: usize, radius: usize) -> Result<&[u8], WindowError> {
    let start = center.checked_sub(radius)
        .ok_or(WindowError::Underflow { center, radius })?;
    let end = center.checked_add(radius)
        .and_then(|x| x.checked_add(1))
        .ok_or(WindowError::Overflow { center, radius })?;
    data.get(start..end)
        .ok_or(WindowError::OutOfBounds { start, end, len: data.len() })
}
```

**Transformation Steps:**
1. Replace `a + b` with `a.checked_add(b)?` or `.ok_or(Error)?`
2. Replace `a - b` with `a.checked_sub(b)?` or `.ok_or(Error)?`
3. Replace `a * b` with `a.checked_mul(b)?` or `.ok_or(Error)?`
4. Chain with `.and_then()` for multiple operations
5. Replace index access with `.get()` and handle Option/Result

---

## Performance Measurement

### Before/After Benchmarking

```rust
// benches/refactoring_benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_before(c: &mut Criterion) {
    let data = generate_test_data();
    
    c.bench_function("operation_before", |b| {
        b.iter(|| operation_before(&data))
    });
}

fn benchmark_after(c: &mut Criterion) {
    let data = generate_test_data();
    
    c.bench_function("operation_after", |b| {
        b.iter(|| operation_after(&data))
    });
}

// Compare multiple input sizes
fn benchmark_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");
    
    for size in [100, 1000, 10000, 100000].iter() {
        let data = generate_data_of_size(*size);
        
        group.bench_with_input(BenchmarkId::new("before", size), &data, |b, data| {
            b.iter(|| operation_before(data))
        });
        
        group.bench_with_input(BenchmarkId::new("after", size), &data, |b, data| {
            b.iter(|| operation_after(data))
        });
    }
    
    group.finish();
}
```

### Memory Profiling

```bash
# Use DHAT for allocation profiling
cargo install dhat
DHAT_ARGS="--num-callers=20" cargo run --release --features dhat-heap

# Use heaptrack (Linux)
heaptrack ./target/release/your_binary

# Count allocations with custom allocator
cargo build --release --features count-alloc
```

### Quick Performance Checks

```bash
# Time command execution
hyperfine --warmup 3 './target/release/binary_before' './target/release/binary_after'

# Compare with statistical significance
hyperfine --warmup 3 --min-runs 20 \
    './before' './after' \
    --export-markdown results.md

# Profile with perf (Linux)
perf record -g ./target/release/binary
perf report

# Generate flamegraph
cargo flamegraph --bin your_binary
```

---

## Common Pitfalls

### Pitfall 1: Over-Optimizing Cold Paths

**Mistake:** Spending time optimizing rarely-executed code.

**Solution:** Profile first, optimize hot paths.

```bash
# Profile to find hot spots
cargo flamegraph --bin your_binary
# Focus on functions taking >5% of total time
```

### Pitfall 2: Breaking Trait Bounds

**Mistake:** Refactoring changes trait bounds, breaking downstream.

```rust
// ❌ BEFORE: T: Clone
fn process<T: Clone>(data: &[T]) -> Vec<T> { ... }

// AFTER change to T: Clone + Default breaks callers:
fn process<T: Clone + Default>(data: &[T]) -> Vec<T> { ... }
```

**Solution:** Check that trait bounds are not made more restrictive.

### Pitfall 3: Changing Public API Return Types

**Mistake:** Changing `Vec<T>` to `impl Iterator<Item=T>` in public API.

```rust
// This is a breaking change!
// Before: pub fn items(&self) -> Vec<Item>
// After: pub fn items(&self) -> impl Iterator<Item = &Item>
```

**Solution:** Add new method with different name, deprecate old one.

### Pitfall 4: Clone Removal Causing Borrow Conflicts

**Mistake:** Removing `.clone()` introduces borrow checker errors.

```rust
// The clone might have been hiding a borrow conflict:
let x = data.clone();  // Clone allows `data` to be borrowed again
process(&x, &data);    // Both references work

// After removing clone:
process(&data, &data);  // May or may not work depending on process signature
```

**Solution:** Analyze why the clone existed before removing.

### Pitfall 5: `mem::take` on Types Without Sensible Default

**Mistake:** Using `mem::take` leaves an invalid default value.

```rust
// If State::default() is invalid or expensive:
let state = mem::take(&mut self.state);  // self.state is now State::default()
// Code assuming self.state is valid will break
```

**Solution:** Use `mem::replace` with an explicit valid replacement, or restructure to use `Option<State>`.

### Pitfall 6: Iterator Refactoring Changes Order

**Mistake:** Iterator transformations may change processing order.

```rust
// Before: processes in index order
for i in 0..items.len() {
    if condition(i) {
        process(&items[i]);
    }
}

// After: parallel iterator changes order
items.par_iter().filter(condition).for_each(process);
```

**Solution:** If order matters, use sequential iterators or document the change.

### Pitfall 7: Changing Error Types Breaks Error Handling

**Mistake:** Changing error variant structure breaks match arms.

```rust
// Before
enum Error { NotFound }

// After - breaks existing matches
enum Error { NotFound { id: u64 } }
```

**Solution:** Use `#[non_exhaustive]` on error enums, add fields carefully.

---

## Automated Transformation Checklist

Use this checklist when applying refactoring patterns:

### Pre-Refactoring

- [ ] Tests pass: `cargo nextest run`
- [ ] No clippy warnings: `cargo clippy --all-targets`
- [ ] Benchmarks exist for performance-sensitive code
- [ ] Git commit/stash current changes

### During Refactoring

- [ ] Make one logical change at a time
- [ ] Compile after each change: `cargo check`
- [ ] Run quick tests frequently: `cargo nextest run`

### Post-Refactoring

- [ ] All tests pass: `cargo nextest run`
- [ ] No new clippy warnings: `cargo clippy --all-targets`
- [ ] No new panicking patterns: `rg "unwrap\(\)|expect\(" src/`
- [ ] Benchmarks show expected improvement (if performance-related)
- [ ] Documentation updated if API changed
- [ ] CHANGELOG updated if user-facing change

### Search Patterns for Common Issues

```bash
# Find remaining unwrap/expect calls
rg "\.unwrap\(\)|\.expect\(" src/

# Find remaining index access (potential panics)
rg "\[.*\]" --type rust src/ | rg -v "^\s*//" | head -50

# Find clone calls that might be unnecessary
rg "\.clone\(\)" src/

# Find potential allocation in loops
rg "for .* in .* \{" -A 5 src/ | rg "Vec::new|String::new|to_string|to_owned"

# Find functions returning Vec that could return iterator
rg "fn .* -> Vec<" src/
```

---

## Quick Reference Card

| Anti-Pattern | Refactoring | Clippy Lint |
|--------------|-------------|-------------|
| `.unwrap()` | `.ok_or()?` | `clippy::unwrap_used` |
| `.expect()` | `.map_err()?` | `clippy::expect_used` |
| `collection[i]` | `.get(i)?` | `clippy::indexing_slicing` |
| `a + b` (overflow) | `a.checked_add(b)?` | `clippy::arithmetic_side_effects` |
| `fn(x: String)` | `fn(x: &str)` | `clippy::needless_pass_by_value` |
| `fn(v: &Vec<T>)` | `fn(v: &[T])` | `clippy::ptr_arg` |
| `.clone()` unnecessary | Remove or use reference | `clippy::redundant_clone` |
| `x = y.clone()` | `x.clone_from(&y)` | — |
| `Vec::new()` + loop | `with_capacity()` | — |
| `.unwrap_or(expensive())` | `.unwrap_or_else(\|\| ...)` | `clippy::or_fun_call` |
| `.collect()` then iterate | Return `impl Iterator` | — |
| `.chunks(n)` | `.chunks_exact(n)` | — |
| `vec.remove(i)` (unordered) | `vec.swap_remove(i)` | — |
| `bool` parameters | Enum variants | — |
| `String` status field | Enum type | — |
| `Option<T>` for state | Separate types | — |

---

## Further Reading

- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Parse, Don't Validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
- [Clippy Lints Documentation](https://rust-lang.github.io/rust-clippy/master/)
