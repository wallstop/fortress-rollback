# Clippy Lints and Rust Code Quality Guide

> A comprehensive guide to Clippy lints, configuration, and best practices for production-quality Rust code.

## Quick Reference

| Lint Group | Default Level | Count | Purpose |
|------------|--------------|-------|---------|
| `correctness` | **deny** | ~60 | Code that is outright wrong |
| `suspicious` | warn | ~90 | Code that looks wrong but might be intentional |
| `style` | warn | ~150 | Idiomatic Rust code conventions |
| `complexity` | warn | ~100 | Simplifiable code patterns |
| `perf` | warn | ~50 | Performance improvements |
| `pedantic` | allow | ~100 | Stricter, opinionated lints |
| `restriction` | allow | ~100 | Highly restrictive lints |
| `nursery` | allow | ~50 | Experimental/incomplete lints |
| `cargo` | allow | ~5 | Cargo.toml quality checks |

---

## Part 1: Lint Groups Explained

### Correctness (deny-by-default)

**Never suppress these without extreme justification.** These lints catch actual bugs.

```rust
// Example: approx_constant - catches incorrect mathematical constants
// ❌ Triggers correctness lint
let pi = 3.14159;  // clippy::approx_constant
// ✅ Use the constant
let pi = std::f64::consts::PI;

// Example: infinite_iter - catches infinite iterators consumed completely
// ❌ This will hang forever
let sum: i32 = (0..).sum();  // clippy::infinite_iter
// ✅ Limit the iterator
let sum: i32 = (0..100).sum();
```

### Suspicious (warn-by-default)

Code that looks wrong but might be intentional. Review carefully.

```rust
// Example: suspicious_arithmetic_impl - catches odd operator implementations
impl Add for MyType {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        // ❌ Suspicious: subtraction in Add impl
        Self(self.0 - other.0)  // clippy::suspicious_arithmetic_impl
    }
}

// Example: await_holding_lock - catches deadlock potential
async fn bad() {
    let guard = mutex.lock().unwrap();
    // ❌ Holding lock across await point
    some_async_op().await;  // clippy::await_holding_lock
}
```

### Complexity (warn-by-default)

Code that can be simplified without changing behavior.

```rust
// Example: needless_bool - unnecessary boolean operations
// ❌ Overly complex
if condition { true } else { false }  // clippy::needless_bool
// ✅ Simplified
condition

// Example: redundant_closure - unnecessary closure wrapper
// ❌ Unnecessary closure
items.iter().map(|x| f(x))  // clippy::redundant_closure
// ✅ Direct function reference
items.iter().map(f)
```

### Performance (warn-by-default)

Code that could be more efficient.

```rust
// Example: box_collection - boxing already-heap-allocated types
// ❌ Double indirection
struct Bad {
    data: Box<Vec<u8>>,  // clippy::box_collection
}
// ✅ Single allocation
struct Good {
    data: Vec<u8>,
}

// Example: manual_memcpy - using loop instead of copy_from_slice
// ❌ Manual loop
for i in 0..len {
    dst[i] = src[i];  // clippy::manual_memcpy
}
// ✅ Use built-in
dst[..len].copy_from_slice(&src[..len]);
```

### Style (warn-by-default)

Idiomatic Rust conventions. Subjective but generally good advice.

```rust
// Example: len_zero - using .len() == 0 instead of .is_empty()
// ❌ Less idiomatic
if vec.len() == 0 { }  // clippy::len_zero
// ✅ More idiomatic
if vec.is_empty() { }

// Example: redundant_field_names - verbose struct initialization
// ❌ Redundant
let s = MyStruct { name: name, value: value };  // clippy::redundant_field_names
// ✅ Shorthand
let s = MyStruct { name, value };
```

### Pedantic (allow-by-default)

Stricter lints that may have false positives. Cherry-pick useful ones.

```rust
// Example: cast_lossless - using `as` instead of `into()` for safe casts
// ❌ Could be clearer about losslessness
let big: u64 = small_u32 as u64;  // clippy::cast_lossless
// ✅ Explicit safe conversion
let big: u64 = small_u32.into();

// Example: must_use_candidate - functions that should have #[must_use]
// ❌ Missing must_use
pub fn calculate(&self) -> Result<Value, Error> { }  // clippy::must_use_candidate
// ✅ With must_use
#[must_use]
pub fn calculate(&self) -> Result<Value, Error> { }
```

### Restriction (allow-by-default)

Highly restrictive lints. **Never enable the entire group.** Cherry-pick only.

```rust
// Example: unwrap_used - any use of .unwrap()
// ❌ Can panic
let value = option.unwrap();  // clippy::unwrap_used
// ✅ Handle the error
let value = option.ok_or(Error::Missing)?;

// Example: panic - any use of panic!()
// ❌ Crashes the program
panic!("something went wrong");  // clippy::panic
// ✅ Return error
return Err(Error::SomethingWrong);
```

### Nursery (allow-by-default)

Experimental lints that may have bugs. Cherry-pick stable ones.

```rust
// Example: significant_drop_in_scrutinee - drops in match scrutinees
// ❌ Drop timing may be surprising
match mutex.lock().unwrap().data.clone() {  // clippy::significant_drop_in_scrutinee
    Some(x) => use(x),
    None => {},
}
// ✅ Explicit drop timing
let data = mutex.lock().unwrap().data.clone();
drop(mutex);
match data { ... }
```

---

## Part 2: Top 40 Most Important Lints

### Critical Correctness Lints (Always Enable)

| Lint | Category | Why It Matters |
|------|----------|----------------|
| `invalid_regex` | correctness | Invalid regex causes runtime panic |
| `approx_constant` | correctness | Using imprecise mathematical constants |
| `infinite_iter` | correctness | Infinite iterator consumed completely |
| `iter_next_loop` | correctness | Calling `.next()` in a for loop |
| `out_of_bounds_indexing` | correctness | Compile-time detected OOB |
| `panicking_unwrap` | correctness | Unwrap on known-None value |
| `uninit_assumed_init` | correctness | Using uninitialized memory |
| `derive_ord_xor_partial_ord` | correctness | Inconsistent ordering impls |
| `if_let_mutex` | correctness | Deadlock from mutex in if let |

### Essential Safety Lints (For Production Code)

| Lint | Category | Why It Matters |
|------|----------|----------------|
| `unwrap_used` | restriction | Prevents panic from None/Err |
| `expect_used` | restriction | Prevents panic from None/Err |
| `panic` | restriction | Prevents explicit panics |
| `todo` | restriction | Catches incomplete code |
| `unimplemented` | restriction | Catches stub implementations |
| `indexing_slicing` | restriction | Prevents OOB panic |
| `arithmetic_side_effects` | restriction | Prevents overflow panic |

### Performance Lints (Enable for Hot Paths)

| Lint | Category | Why It Matters |
|------|----------|----------------|
| `box_collection` | perf | Unnecessary double indirection |
| `large_enum_variant` | perf | Enum size bloated by one variant |
| `manual_memcpy` | perf | Loop slower than memcpy |
| `useless_vec` | perf | Vec where array/slice suffices |
| `unnecessary_to_owned` | perf | Cloning when borrowing works |
| `redundant_clone` | nursery | Clone of already-owned value |
| `manual_str_repeat` | perf | Manual loop vs `.repeat()` |
| `map_entry` | perf | Inefficient HashMap access |
| `slow_vector_initialization` | perf | Pre-size vectors when possible |
| `cmp_owned` | perf | Unnecessary allocation for comparison |

### API Design Lints (For Public APIs)

| Lint | Category | Why It Matters |
|------|----------|----------------|
| `must_use_candidate` | pedantic | Functions should indicate useful return |
| `missing_errors_doc` | pedantic | Document error conditions |
| `missing_panics_doc` | pedantic | Document panic conditions |
| `missing_safety_doc` | style | Unsafe functions need documentation |
| `new_without_default` | style | Types with `new()` should impl Default |
| `len_without_is_empty` | style | Types with `.len()` should have `.is_empty()` |
| `result_unit_err` | style | `Result<T, ()>` loses error info |

### Code Quality Lints (Improve Maintainability)

| Lint | Category | Why It Matters |
|------|----------|----------------|
| `cognitive_complexity` | restriction | Functions too complex to understand |
| `too_many_arguments` | complexity | Functions with too many params |
| `needless_pass_by_value` | pedantic | Take references when ownership not needed |
| `implicit_clone` | pedantic | Make clones explicit |
| `clone_on_ref_ptr` | restriction | Clone Arc/Rc explicitly |
| `wildcard_imports` | pedantic | `use foo::*` hides dependencies |
| `enum_glob_use` | pedantic | `use Enum::*` obscures variants |
| `fallible_impl_from` | nursery | From should be infallible; use TryFrom |

---

## Part 3: Cargo.toml Configuration

### Basic Setup

```toml
[lints.rust]
# Enable warnings for unsafe code
unsafe_code = "warn"
# Or forbid entirely
# unsafe_code = "forbid"

[lints.clippy]
# Start with pedantic as base, then allow noisy ones
pedantic = { level = "warn", priority = -1 }
# Allow commonly noisy pedantic lints
module_name_repetitions = "allow"
similar_names = "allow"
too_many_lines = "allow"
```

### Production-Grade Configuration

```toml
[lints.rust]
# Require docs on public items
missing_docs = "warn"
# Warn about unused items
unused = "warn"
# Handle custom cfg flags
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(test)', 'cfg(feature, values(...))'] }

[lints.clippy]
# === Base: Enable pedantic ===
pedantic = { level = "warn", priority = -1 }

# === Defensive programming (deny these) ===
todo = "deny"
unimplemented = "deny"
fallible_impl_from = "deny"

# === No-panic enforcement (warn for gradual adoption, deny when clean) ===
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"

# === Debug/print prevention ===
dbg_macro = "warn"
print_stdout = "warn"
print_stderr = "warn"

# === Allow noisy pedantic lints ===
module_name_repetitions = "allow"
similar_names = "allow"
too_many_lines = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
cast_precision_loss = "allow"
cast_sign_loss = "allow"
cast_possible_truncation = "allow"
```

### Strict Library Configuration

```toml
[lints.clippy]
# Enable all major groups
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }

# Deny dangerous patterns
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"

# Allow specific noisy lints
multiple_crate_versions = "allow"
significant_drop_tightening = "allow"
redundant_pub_crate = "allow"
future_not_send = "allow"
cognitive_complexity = "allow"
missing_const_for_fn = "allow"
```

### Application Configuration (More Relaxed)

```toml
[lints.clippy]
# Enable pedantic but allow more flexibility
pedantic = { level = "warn", priority = -1 }

# Applications can panic in unrecoverable situations
unwrap_used = "warn"  # Not deny
panic = "allow"       # Applications may legitimately panic

# Still prevent incomplete code
todo = "deny"
unimplemented = "deny"

# Allow CLI output
print_stdout = "allow"
print_stderr = "allow"
```

---

## Part 4: clippy.toml Configuration

Create `clippy.toml` in project root for lint-specific settings:

```toml
# Minimum Supported Rust Version - affects which lints apply
msrv = "1.80"

# Cognitive complexity threshold (default: 25)
cognitive-complexity-threshold = 30

# Type complexity threshold (default: 250)
type-complexity-threshold = 350

# Trivially copy pass by ref threshold (bytes)
trivial-copy-size-limit = 16

# Pass by value threshold (bytes)
pass-by-value-size-limit = 256

# Allow short identifier names
allowed-idents-below-min-chars = ["x", "y", "z", "i", "j", "k", "n", "id"]

# Require docs on crate items
missing-docs-in-crate-items = true

# Disallow specific macros (prevents debug output in production)
disallowed-macros = [
    { path = "std::dbg", reason = "Use tracing macros instead" },
    { path = "std::print", reason = "Use tracing macros instead" },
    { path = "std::println", reason = "Use tracing macros instead" },
]

# Allow unwrap/expect in tests
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-indexing-slicing-in-tests = true
```

---

## Part 5: Handling False Positives

### Per-Item Allowances

```rust
// Allow on a single item with justification
#[allow(clippy::too_many_arguments)]
fn complex_but_necessary(
    a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32
) -> Result<(), Error> {
    // Justification: This matches the wire protocol format
    ...
}
```

### Scoped Allowances

```rust
fn process() {
    // Allow only for this specific line
    #[allow(clippy::unwrap_used)]
    let value = guaranteed_some.unwrap(); // SAFETY: validated above
}
```

### Module-Level Allowances

```rust
// In a module that intentionally uses unsafe patterns
#![allow(clippy::indexing_slicing)]
// Justification: This module handles bounds checking manually for performance
```

### Test-Specific Configuration

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]  // OK in tests
    #![allow(clippy::panic)]         // OK in tests
    
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
```

---

## Part 6: Lints by Use Case

### For Zero-Panic Code

```toml
[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"
indexing_slicing = "deny"  # Use .get() instead
arithmetic_side_effects = "deny"  # Use checked_* methods
```

### For Performance-Critical Code

```toml
[lints.clippy]
# Performance warnings
box_collection = "warn"
large_enum_variant = "warn"
redundant_clone = "warn"
unnecessary_to_owned = "warn"
useless_vec = "warn"
manual_memcpy = "warn"
slow_vector_initialization = "warn"
# More aggressive
large_stack_arrays = "warn"
large_futures = "warn"
```

### For Public Library APIs

```toml
[lints.clippy]
# API quality
must_use_candidate = "warn"
missing_errors_doc = "warn"
missing_panics_doc = "warn"
missing_safety_doc = "warn"
# Prevent breaking changes
exhaustive_enums = "warn"
exhaustive_structs = "warn"
```

### For Async Code

```toml
[lints.clippy]
# Async-specific
await_holding_lock = "deny"
await_holding_refcell_ref = "deny"
future_not_send = "warn"  # May need to allow
large_futures = "warn"
```

---

## Part 7: Before/After Examples

### Unwrap to Error Handling

```rust
// ❌ Before: Can panic
fn get_config(path: &str) -> Config {
    let content = std::fs::read_to_string(path).unwrap();
    toml::from_str(&content).unwrap()
}

// ✅ After: Returns Result
fn get_config(path: &str) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::Read { path: path.into(), source: e })?;
    toml::from_str(&content)
        .map_err(|e| ConfigError::Parse { path: path.into(), source: e })
}
```

### Indexing to Safe Access

```rust
// ❌ Before: Can panic on out of bounds
fn get_element(data: &[u8], index: usize) -> u8 {
    data[index]  // clippy::indexing_slicing
}

// ✅ After: Returns Option
fn get_element(data: &[u8], index: usize) -> Option<u8> {
    data.get(index).copied()
}

// Or with Result
fn get_element(data: &[u8], index: usize) -> Result<u8, Error> {
    data.get(index).copied().ok_or(Error::IndexOutOfBounds { index, len: data.len() })
}
```

### Clone to Borrow

```rust
// ❌ Before: Unnecessary allocation
fn process(data: String) {
    use_data(&data);  // Only borrowing
}
// Called with: process(my_string.clone())

// ✅ After: Accept reference
fn process(data: &str) {
    use_data(data);
}
// Called with: process(&my_string)
```

### Manual Loop to Iterator

```rust
// ❌ Before: Manual, error-prone
let mut results = Vec::new();
for item in items {
    if item.is_valid() {
        results.push(item.process());
    }
}

// ✅ After: Functional, clear intent
let results: Vec<_> = items
    .into_iter()
    .filter(|item| item.is_valid())
    .map(|item| item.process())
    .collect();
```

### Box Collection to Direct Collection

```rust
// ❌ Before: Double indirection
struct Cache {
    data: Box<HashMap<String, Value>>,  // clippy::box_collection
}

// ✅ After: Single allocation
struct Cache {
    data: HashMap<String, Value>,
}
```

### Large Enum Variant

```rust
// ❌ Before: All variants sized to largest
enum Message {
    Small(u8),
    Large([u8; 1024]),  // clippy::large_enum_variant
}

// ✅ After: Box the large variant
enum Message {
    Small(u8),
    Large(Box<[u8; 1024]>),
}
```

---

## Part 8: Running Clippy

### Basic Usage

```bash
# Run with default settings
cargo clippy

# Run on all targets (tests, benches, examples)
cargo clippy --all-targets

# Run with all features enabled
cargo clippy --all-features

# Fix automatically where possible
cargo clippy --fix

# Deny all warnings (for CI)
cargo clippy -- -D warnings
```

### With Feature Flags

```bash
# Check specific features
cargo clippy --features "feature1,feature2"

# Check without default features
cargo clippy --no-default-features

# Check all feature combinations
cargo clippy --all-features
```

### CI Configuration

```yaml
# GitHub Actions example
- name: Clippy
  run: cargo clippy --all-targets --all-features -- -D warnings
```

---

## Part 9: Common Anti-Patterns to Avoid

### Don't Enable All Restriction Lints

```toml
# ❌ NEVER do this
[lints.clippy]
restriction = "warn"  # Will cause hundreds of false positives

# ✅ Cherry-pick specific restriction lints
[lints.clippy]
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
```

### Don't Suppress Without Reason

```rust
// ❌ Bad: No explanation
#[allow(clippy::unwrap_used)]
fn foo() { ... }

// ✅ Good: Documented justification
#[allow(clippy::unwrap_used)]
// SAFETY: `value` is validated to be Some by `validate()` called above
fn foo() { ... }
```

### Don't Mix Allow and Deny Inconsistently

```toml
# ❌ Confusing: Group denied but member allowed
[lints.clippy]
pedantic = "deny"
similar_names = "allow"  # Still denied due to group

# ✅ Clear: Use priority
[lints.clippy]
pedantic = { level = "deny", priority = -1 }  # Lower priority
similar_names = "allow"  # Higher priority, overrides
```

---

## Part 10: Quick Troubleshooting

| Issue | Solution |
|-------|----------|
| "unknown lint" | Update Rust/Clippy version |
| Lint triggers in macro | Add `#[allow]` to macro invocation |
| Lint triggers in generated code | Use `#[cfg_attr(not(clippy), ...)]` |
| Too many warnings | Enable lints gradually |
| False positive | File issue, use `#[allow]` with comment |
| Lint not triggering | Check if lint is in enabled group |

---

## Summary: Recommended Baseline

For most production Rust projects:

```toml
[lints.clippy]
# Enable pedantic as baseline
pedantic = { level = "warn", priority = -1 }

# Zero-panic enforcement
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
todo = "deny"
unimplemented = "deny"

# Common noisy lints to allow
module_name_repetitions = "allow"
similar_names = "allow"
too_many_lines = "allow"
missing_errors_doc = "allow"
```

Combined with `clippy.toml`:

```toml
msrv = "1.80"
allow-unwrap-in-tests = true
allow-expect-in-tests = true
```

This provides excellent coverage without being overwhelming.
