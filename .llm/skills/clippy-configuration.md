# Clippy Configuration Guide — Lint-Driven Code Quality

> **This document provides a comprehensive guide to configuring Clippy for maximum code quality.**
> Use these lints to catch bugs, enforce best practices, and improve performance automatically.

## TL;DR — Recommended Baseline Configuration

Add to `Cargo.toml`:

```toml
[lints.clippy]
# Correctness (bugs)
correctness = { level = "deny", priority = -1 }

# Suspicious patterns (likely bugs)  
suspicious = { level = "warn", priority = -1 }

# Zero-panic code (critical for production)
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
indexing_slicing = "deny"

# Performance
perf = { level = "warn", priority = -1 }

# Code quality
pedantic = { level = "warn", priority = -1 }
missing_errors_doc = "allow"  # Relax if too noisy
```

Run: `cargo clippy --all-targets`

---

## Understanding Clippy Lint Groups

### The Nine Lint Groups

| Group | Default | Description | When to Enable |
|-------|---------|-------------|----------------|
| **correctness** | deny | Actual bugs, always wrong | Always |
| **suspicious** | warn | Likely bugs, rarely correct | Always |
| **style** | warn | Idiomatic style violations | Always |
| **complexity** | warn | Overly complex code | Always |
| **perf** | warn | Performance issues | Always |
| **pedantic** | allow | Very strict, opinionated | Libraries |
| **restriction** | allow | Very restrictive, use selectively | Security-critical |
| **nursery** | allow | Unstable, may have false positives | Experimental |
| **cargo** | allow | Cargo.toml issues | CI/CD |

### Group Priority

Use priority to control lint group ordering:

```toml
[lints.clippy]
# Enable pedantic first (priority -1 = lower = applied first)
pedantic = { level = "warn", priority = -1 }
# Then allow specific lints (higher priority = applied later)
missing_errors_doc = "allow"
```

---

## Critical Correctness Lints (Always Enable)

These catch actual bugs — never disable them:

```toml
[lints.clippy]
# These are deny by default, but be explicit
correctness = { level = "deny", priority = -1 }

# Specific high-impact correctness lints
absurd_extreme_comparisons = "deny"   # x > MAX or x < MIN is always false
approx_constant = "deny"              # Using 3.14 instead of std::f64::consts::PI
bad_bit_mask = "deny"                 # if (x & 0b111) == 0b1000 { } // never true
deprecated = "deny"                   # Using deprecated items
derive_ord_xor_partial_ord = "deny"   # Ord without PartialOrd is wrong
drop_copy = "deny"                    # Dropping Copy types does nothing
eq_op = "deny"                        # x == x is always true
erasing_op = "deny"                   # x * 0 = 0, x & 0 = 0
infinite_loop = "deny"                # Loop that never exits
invalid_regex = "deny"                # Regex that won't compile
iter_skip_next = "deny"               # .skip(1).next() instead of .nth(1)
modulo_one = "deny"                   # x % 1 is always 0
never_loop = "deny"                   # Loop that always breaks on first iteration
nonsensical_open_options = "deny"     # File::create().read() is silly
not_unsafe_ptr_arg_deref = "deny"     # Deref raw ptr in safe fn without unsafe block
out_of_bounds_indexing = "deny"       # array[10] when array.len() < 10
suspicious_map = "deny"               # .map().unwrap() instead of .and_then()
uninit_assumed_init = "deny"          # MaybeUninit::assume_init without initialization
unit_cmp = "deny"                     # Comparing () values
```

---

## Zero-Panic Production Code Lints

For production code that must never panic:

```toml
[lints.clippy]
# Direct panic sources
panic = "deny"
panic_in_result_fn = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"

# Implicit panics
unwrap_used = "deny"
expect_used = "deny"
indexing_slicing = "deny"

# Arithmetic panics
integer_division = "deny"
modulo_arithmetic = "deny"

# Infallible conversions that could panic
fallible_impl_from = "deny"
```

**Handling false positives:**

```rust
// When you must use unwrap (rare cases with proof)
#[allow(clippy::unwrap_used, reason = "Vec always has at least one element after push")]
let first = vec.first().unwrap();

// Better: use expect with justification (if you allow expect_used)
let first = vec.first().expect("Vec non-empty after push");

// Best: restructure to avoid the need
let first = vec.first().ok_or(Error::EmptyVec)?;
```

---

## Performance Lints

```toml
[lints.clippy]
perf = { level = "warn", priority = -1 }

# Critical performance lints
box_collection = "warn"               # Box<Vec<T>> instead of Vec<T>
box_default = "warn"                  # Box::new(Default::default())
boxed_local = "warn"                  # Using Box for local variable
expect_fun_call = "warn"              # .expect(format!(...)) allocates even on Ok
inefficient_to_string = "warn"        # ToString impl that could use Display
iter_on_empty_collections = "warn"    # Iterating over known-empty collection
iter_on_single_items = "warn"         # Iterating over single item
large_const_arrays = "warn"           # Large arrays in const context
large_enum_variant = "warn"           # One variant much larger than others
large_types_passed_by_value = "warn"  # Passing large types by value
mutex_atomic = "warn"                 # Mutex<bool> instead of AtomicBool
naive_bytecount = "warn"              # Manual byte counting instead of bytecount
needless_collect = "warn"             # collect() when direct iteration works
or_fun_call = "warn"                  # .or(expensive()) instead of .or_else()
single_char_pattern = "warn"          # "x" instead of 'x' in string methods
slow_vector_initialization = "warn"   # vec![0; n] vs vec.resize()
unnecessary_to_owned = "warn"         # to_owned() when borrow suffices
useless_vec = "warn"                  # Vec when array works
vec_init_then_push = "warn"           # Vec::new() then push vs vec![]
```

### Examples

```rust
// ❌ box_collection: Box<Vec<T>> is double-indirection
fn get_data() -> Box<Vec<u8>> { ... }
// ✅ Just return Vec (already heap-allocated)
fn get_data() -> Vec<u8> { ... }

// ❌ large_enum_variant: Forces all variants to large size
enum Message {
    Ping,
    Data([u8; 1024]),  // 1024 bytes for all Messages!
}
// ✅ Box the large variant
enum Message {
    Ping,
    Data(Box<[u8; 1024]>),
}

// ❌ or_fun_call: compute_default() called even if Some
x.or(compute_default())
// ✅ Use or_else for lazy evaluation
x.or_else(|| compute_default())
```

---

## API Design Lints

For library authors:

```toml
[lints.clippy]
# Public API quality
missing_panics_doc = "warn"           # Document when functions can panic
missing_safety_doc = "warn"           # Document unsafe preconditions
missing_errors_doc = "warn"           # Document when Result can be Err
must_use_candidate = "warn"           # Functions that should be #[must_use]
return_self_not_must_use = "warn"     # Builder methods should be must_use
undocumented_unsafe_blocks = "warn"   # Unsafe blocks need // SAFETY:

# Type design
enum_variant_names = "warn"           # Variant names shouldn't repeat enum name
struct_excessive_bools = "warn"       # Multiple bools, use enum instead
```

---

## Code Quality Lints (Pedantic)

```toml
[lints.clippy]
pedantic = { level = "warn", priority = -1 }

# Often useful to relax these
missing_errors_doc = "allow"          # Can be noisy
module_name_repetitions = "allow"     # project::project_error is fine
must_use_candidate = "allow"          # Too noisy for internal code
similar_names = "allow"               # item vs items is clear enough
too_many_lines = "allow"              # Functions can be long if clear
```

### Most Valuable Pedantic Lints

```toml
# Keep these even if you relax pedantic overall
bool_to_int_with_if = "warn"          # if b { 1 } else { 0 } → u32::from(b)
case_sensitive_file_extension_comparisons = "warn"
cloned_instead_of_copied = "warn"     # .cloned() on Copy types
default_trait_access = "warn"         # Default::default() → T::default()
explicit_deref_methods = "warn"       # (*x).foo() → x.foo()
explicit_iter_loop = "warn"           # for x in v.iter() → for x in &v
filter_map_next = "warn"              # .filter_map().next() → .find_map()
flat_map_option = "warn"              # .flat_map(|x| x) on Option
from_iter_instead_of_collect = "warn" # FromIterator::from_iter → .collect()
if_not_else = "warn"                  # if !cond { a } else { b } → if cond { b } else { a }
implicit_clone = "warn"               # .clone() on reference when deref would copy
inefficient_to_string = "warn"        # Better ToString implementation available
items_after_statements = "warn"       # Items should be at module top
iter_without_into_iter = "warn"       # iter() without IntoIterator impl
manual_assert = "warn"                # if !cond { panic!() } → assert!(cond)
manual_instant_elapsed = "warn"       # Instant::now() - start → start.elapsed()
manual_is_ascii_check = "warn"        # Manual ASCII range checks
manual_let_else = "warn"              # let x = match expr { Some(x) => x, None => return }
manual_string_new = "warn"            # String::from("") → String::new()
map_unwrap_or = "warn"                # .map().unwrap_or() → .map_or()
match_bool = "warn"                   # match on bool → if/else
match_same_arms = "warn"              # Duplicate match arms
match_wildcard_for_single_variants = "warn"
needless_continue = "warn"            # Unnecessary continue at end of loop
needless_for_each = "warn"            # .for_each() when for loop is clearer
no_effect_underscore_binding = "warn" # let _ = expr; when expr has no side effect
range_plus_one = "warn"               # 0..n+1 → 0..=n
redundant_closure_for_method_calls = "warn"  # |x| x.foo() → T::foo
redundant_else = "warn"               # else after diverging if
semicolon_if_nothing_returned = "warn"
unnecessary_wraps = "warn"            # Returning Result when function never errors
unnested_or_patterns = "warn"         # Some(1) | Some(2) → Some(1 | 2)
unused_self = "warn"                  # Methods that don't use self
used_underscore_binding = "warn"      # Using _x after declaring it unused
verbose_bit_mask = "warn"             # x & (y - 1) when y is power of 2
```

---

## Restriction Lints (Use Selectively)

These are very strict — enable only what makes sense:

```toml
[lints.clippy]
# Safe defaults for most projects
absolute_paths = "allow"              # Too strict
as_conversions = "warn"               # Prefer TryFrom
clone_on_ref_ptr = "warn"             # Rc::clone(&x) is clearer
dbg_macro = "warn"                    # dbg!() shouldn't be in production
default_numeric_fallback = "warn"     # 42 without type annotation
deref_by_slicing = "warn"             # &vec[..] → &*vec
empty_drop = "warn"                   # Empty Drop impl
empty_structs_with_brackets = "warn"  # struct Empty {} → struct Empty;
exit = "warn"                         # std::process::exit is usually wrong
filetype_is_file = "warn"             # .is_file() misses symlinks
float_arithmetic = "allow"            # Too strict unless embedded
get_unwrap = "warn"                   # .get().unwrap() → indexing or ?
if_then_some_else_none = "warn"       # if c { Some(x) } else { None } → c.then(|| x)
impl_trait_in_params = "warn"         # fn f(x: impl Trait) → fn f<T: Trait>(x: T)
indexing_slicing = "warn"             # Already in zero-panic section
inline_asm_x86_att_syntax = "allow"   # Platform specific
inline_asm_x86_intel_syntax = "allow" # Platform specific
integer_division_remainder_used = "allow"  # Too strict
let_underscore_must_use = "warn"      # let _ = must_use_fn()
let_underscore_untyped = "warn"       # let _ = expr; might hide type error
lossy_float_literal = "warn"          # Float literals that lose precision
map_err_ignore = "warn"               # .map_err(|_| ...) loses error info
mem_forget = "warn"                   # mem::forget usually wrong
min_ident_chars = "allow"             # Too strict (i, x, n are fine)
missing_asserts_for_indexing = "warn" # Index without prior bounds check
mixed_read_write_in_expression = "warn"
multiple_unsafe_ops_per_block = "warn"
mutex_integer = "warn"                # Mutex<i32> → AtomicI32
needless_raw_strings = "warn"         # r"no special chars" → "no special chars"
panic_in_result_fn = "warn"           # Don't panic in fn returning Result
partial_pub_fields = "allow"          # Sometimes legitimate
print_stderr = "warn"                 # Use logging instead
print_stdout = "warn"                 # Use logging instead
pub_without_shorthand = "warn"        # pub(in self) → private
rc_buffer = "warn"                    # Rc<String> → Rc<str>
rc_mutex = "warn"                     # Rc<Mutex<T>> is usually wrong
redundant_type_annotations = "warn"   # let x: i32 = 5i32;
ref_patterns = "allow"                # &x in patterns is sometimes clearer
rest_pat_in_fully_bound_structs = "warn"
same_name_method = "warn"             # Method shadowing trait method
self_named_module_files = "allow"     # mod.rs vs folder/name.rs
semicolon_inside_block = "allow"      # Style preference
semicolon_outside_block = "allow"     # Style preference
shadow_reuse = "allow"                # Shadowing is fine in Rust
shadow_same = "allow"                 # let x = x; is fine
shadow_unrelated = "warn"             # Shadowing with unrelated type
single_call_fn = "allow"              # Single-use functions are fine
std_instead_of_alloc = "allow"        # Only for no_std
std_instead_of_core = "allow"         # Only for no_std
str_to_string = "warn"                # "".to_string() → String::new()
string_add = "warn"                   # s + "x" → s.push_str("x")
string_lit_as_bytes = "warn"          # "abc".as_bytes() → b"abc"
string_slice = "warn"                 # UTF-8 slicing can panic
string_to_string = "warn"             # String.to_string() → .clone()
suspicious_xor_used_as_pow = "warn"   # 2^10 (XOR) likely meant 2**10
try_err = "warn"                      # Err(x)? → return Err(x)
undocumented_unsafe_blocks = "deny"   # Unsafe needs // SAFETY:
unneeded_field_pattern = "warn"       # Foo { x: _, .. } → Foo { .. }
unnecessary_safety_comment = "warn"   # Safety comment on safe code
unnecessary_safety_doc = "warn"       # Safety doc on safe fn
unseparated_literal_suffix = "warn"   # 1usize → 1_usize
unwrap_in_result = "warn"             # .unwrap() in fn returning Result
unwrap_used = "deny"                  # Already in zero-panic
use_debug = "warn"                    # {:?} in user-facing output
verbose_file_reads = "warn"           # fs::read_to_string is simpler
wildcard_enum_match_arm = "warn"      # _ => ... misses new variants
```

---

## Cargo.toml Configuration Reference

### Full Production Configuration

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
# Lint groups
correctness = { level = "deny", priority = -1 }
suspicious = { level = "warn", priority = -1 }
style = { level = "warn", priority = -1 }
complexity = { level = "warn", priority = -1 }
perf = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }

# Zero-panic essentials
panic = "deny"
unwrap_used = "deny"
expect_used = "deny"
indexing_slicing = "deny"
todo = "deny"
unimplemented = "deny"

# Allow some pedantic lints
missing_errors_doc = "allow"
module_name_repetitions = "allow"
must_use_candidate = "allow"

# Additional strict lints
undocumented_unsafe_blocks = "deny"
```

### clippy.toml Configuration

```toml
# Project root: clippy.toml

# Type complexity threshold (default: 250)
type-complexity-threshold = 300

# Max lines per function (default: 100)
too-many-lines-threshold = 150

# Max arguments per function (default: 7)
too-many-arguments-threshold = 8

# Large type threshold for pass-by-value (default: 256)
trivial-copy-size-limit = 16
pass-by-value-size-limit = 256

# Enum variant size threshold (default: 200)
enum-variant-size-threshold = 400

# Struct field count threshold
struct-field-count-threshold = 10

# Cognitive complexity threshold (default: 25)
cognitive-complexity-threshold = 30

# Disallowed types (enforce consistent hashing)
disallowed-types = [
    { path = "std::collections::HashMap", reason = "Use FxHashMap for non-crypto hashing" },
    { path = "std::collections::HashSet", reason = "Use FxHashSet for non-crypto hashing" },
]

# Disallowed methods
disallowed-methods = [
    { path = "std::env::var", reason = "Use config module instead" },
]

# Allowed wildcard imports (usually none)
allowed-wildcard-imports = []

# Doc comment code block ignore (for incomplete examples)
doc-valid-idents = ["OpenGL", "WebGL", "SIMD"]
```

---

## Running Clippy

### Basic Commands

```bash
# Run on all targets (lib, tests, examples, benches)
cargo clippy --all-targets

# Include feature-gated code
cargo clippy --all-targets --all-features

# Fix automatically where possible
cargo clippy --fix --allow-dirty

# Treat warnings as errors (CI)
cargo clippy --all-targets -- -D warnings

# Check workspace
cargo clippy --workspace --all-targets
```

### CI Configuration

```yaml
# .github/workflows/lint.yml
name: Lint

on: [push, pull_request]

jobs:
  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - run: cargo clippy --all-targets --all-features -- -D warnings
```

---

## Handling Lint Violations

### Project-Wide Allowances

In `lib.rs` or `main.rs`:

```rust
// Allow throughout the crate
#![allow(clippy::module_name_repetitions)]

// Deny throughout the crate (stronger than Cargo.toml for sub-crates)
#![deny(clippy::unwrap_used)]
```

### Module-Level Allowances

```rust
// Allow for entire module
#![allow(clippy::too_many_arguments)]

mod internal_module {
    // ...
}
```

### Function/Item-Level Allowances

```rust
// With reason (Rust 1.81+)
#[allow(clippy::unwrap_used, reason = "Guaranteed Some by invariant")]
fn guaranteed_some() -> i32 {
    STATIC_OPTION.unwrap()
}

// Without reason (older Rust)
#[allow(clippy::unwrap_used)]  // OK: Guaranteed by invariant
fn guaranteed_some() -> i32 {
    STATIC_OPTION.unwrap()
}
```

### Inline Allowances

```rust
fn process() {
    #[allow(clippy::indexing_slicing)]  // Length checked on previous line
    let first = &slice[0];
}
```

---

## Common Anti-Patterns in Configuration

### ❌ Don't: Blanket Allow to Silence Warnings

```toml
# BAD: Hides real issues
[lints.clippy]
pedantic = "allow"
perf = "allow"
```

### ❌ Don't: Allow Without Reason

```rust
// BAD: Why is this allowed?
#[allow(clippy::unwrap_used)]
fn mystery() { ... }
```

### ❌ Don't: Global Deny on Noisy Lints

```toml
# BAD: Will frustrate developers
[lints.clippy]
single_char_lifetime_names = "deny"  # 'a is idiomatic!
```

### ✅ Do: Be Selective and Document

```rust
// GOOD: Explains the allowance
#[allow(
    clippy::unwrap_used,
    reason = "JSON schema guarantees this field exists"
)]
fn extract_required_field(json: &Value) -> &str {
    json["required_field"].as_str().unwrap()
}
```

---

## Quick Reference — Lint Categories

| Category | Lints | Use Case |
|----------|-------|----------|
| **Zero-Panic** | `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` | Production code |
| **Performance** | `perf` group, `large_enum_variant`, `needless_collect` | Hot paths |
| **Safety** | `undocumented_unsafe_blocks`, `missing_safety_doc` | Unsafe code |
| **API Quality** | `missing_panics_doc`, `must_use_candidate` | Public APIs |
| **Style** | `pedantic` group minus noisy ones | Code review |
| **Security** | `restriction` subset | Security-critical |

---

*Configure Clippy to match your project's requirements. Start strict, relax as needed with documented reasons.*
