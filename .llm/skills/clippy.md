<!-- CATEGORY: Performance & Quality -->
<!-- WHEN: Configuring Clippy, lint levels, fortress-specific lint rules -->

# Clippy Configuration

---

## Lint Groups

| Group | Default | Purpose | Enable |
|-------|---------|---------|--------|
| `correctness` | deny | Actual bugs | Always |
| `suspicious` | warn | Likely bugs | Always |
| `style` | warn | Idiomatic style | Always |
| `complexity` | warn | Overly complex code | Always |
| `perf` | warn | Performance issues | Always |
| `pedantic` | allow | Strict, opinionated | Libraries |
| `restriction` | allow | Very restrictive | Cherry-pick only |
| `nursery` | allow | Unstable | Cherry-pick only |
| `cargo` | allow | Cargo.toml issues | CI |

---

## Recommended Baseline (Cargo.toml)

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
# Lint groups (lower priority = applied first)
correctness = { level = "deny", priority = -1 }
suspicious = { level = "warn", priority = -1 }
style = { level = "warn", priority = -1 }
complexity = { level = "warn", priority = -1 }
perf = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }

# Zero-panic enforcement
panic = "deny"
unwrap_used = "deny"
expect_used = "deny"
indexing_slicing = "deny"
todo = "deny"
unimplemented = "deny"

# Additional strict lints
undocumented_unsafe_blocks = "deny"

# Relax noisy pedantic lints
missing_errors_doc = "allow"
module_name_repetitions = "allow"
must_use_candidate = "allow"
similar_names = "allow"
too_many_lines = "allow"
```

---

## Zero-Panic Lints

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
fallible_impl_from = "deny"
```

Handling false positives:

```rust
// Best: restructure to avoid the need
let first = vec.first().ok_or(Error::EmptyVec)?;

// When unavoidable, allow with reason
#[allow(clippy::unwrap_used, reason = "Vec non-empty after push")]
let first = vec.first().unwrap();
```

---

## Key Performance Lints

| Lint | Catches |
|------|---------|
| `box_collection` | `Box<Vec<T>>` double indirection |
| `large_enum_variant` | One variant bloating enum size |
| `needless_collect` | `collect()` when direct iteration works |
| `or_fun_call` | `.or(expensive())` instead of `.or_else()` |
| `unnecessary_to_owned` | `.to_owned()` when borrow suffices |
| `useless_vec` | Vec when array works |
| `expect_fun_call` | `.expect(format!())` allocates even on Ok |
| `large_types_passed_by_value` | Pass large types by reference |
| `mutex_atomic` | `Mutex<bool>` instead of `AtomicBool` |

---

## API Design Lints

```toml
[lints.clippy]
missing_panics_doc = "warn"
missing_safety_doc = "warn"
missing_errors_doc = "warn"
must_use_candidate = "warn"
return_self_not_must_use = "warn"
undocumented_unsafe_blocks = "warn"
```

---

## Most Valuable Pedantic Lints

Keep these even when relaxing pedantic overall:

```toml
cloned_instead_of_copied = "warn"    # .cloned() on Copy types
explicit_iter_loop = "warn"          # for x in v.iter() -> for x in &v
manual_let_else = "warn"             # match-to-return -> let-else
manual_string_new = "warn"           # String::from("") -> String::new()
redundant_closure_for_method_calls = "warn"  # |x| x.foo() -> T::foo
unnecessary_wraps = "warn"           # Result when never errors
```

---

## clippy.toml Configuration

```toml
# clippy.toml in project root
type-complexity-threshold = 300
too-many-lines-threshold = 150
too-many-arguments-threshold = 8
pass-by-value-size-limit = 256
enum-variant-size-threshold = 400
cognitive-complexity-threshold = 30

disallowed-types = [
    { path = "std::collections::HashMap", reason = "Use BTreeMap for determinism" },
    { path = "std::collections::HashSet", reason = "Use BTreeSet for determinism" },
]

allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-indexing-slicing-in-tests = true
```

---

## Running Clippy

```bash
cargo clippy --all-targets                  # All targets
cargo clippy --all-targets --all-features   # With all features
cargo clippy --fix --allow-dirty            # Auto-fix
cargo clippy --all-targets -- -D warnings   # CI (deny warnings)
cargo clippy --workspace --all-targets      # Workspace
```

---

## Handling Violations

### Scope Levels

```rust
// Crate-wide (lib.rs / main.rs)
#![allow(clippy::module_name_repetitions)]

// Item-level with reason (Rust 1.81+)
#[allow(clippy::unwrap_used, reason = "Guaranteed Some by invariant")]
fn guaranteed_some() -> i32 { STATIC_OPTION.unwrap() }

// Inline
fn process() {
    #[allow(clippy::indexing_slicing)]  // Length checked above
    let first = &slice[0];
}

// Tests
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::panic)]
}
```

---

## Restriction Lints (Cherry-Pick)

Never enable the entire `restriction` group. Useful individual lints:

```toml
dbg_macro = "warn"               # dbg!() in production
print_stdout = "warn"            # Use logging instead
print_stderr = "warn"
shadow_unrelated = "warn"        # Shadowing with unrelated type
string_add = "warn"              # s + "x" -> s.push_str("x")
wildcard_enum_match_arm = "warn" # _ => misses new variants
as_conversions = "warn"          # Prefer TryFrom
clone_on_ref_ptr = "warn"        # Rc::clone(&x) is clearer
mem_forget = "warn"              # mem::forget usually wrong
exit = "warn"                    # std::process::exit is usually wrong
```

---

## Anti-Patterns

| Anti-Pattern | Why |
|--------------|-----|
| `restriction = "warn"` | Hundreds of false positives |
| `#[allow]` without reason | Hides real issues |
| `pedantic = "deny"` | Will frustrate developers |
| `perf = "allow"` | Hides performance issues |

---

## CI Integration

```yaml
- name: Clippy
  run: cargo clippy --all-targets --all-features -- -D warnings
```

---

## Lint Category Quick Reference

| Category | Key Lints | Use Case |
|----------|-----------|----------|
| Zero-Panic | `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` | Production |
| Performance | `perf` group, `large_enum_variant`, `needless_collect` | Hot paths |
| Safety | `undocumented_unsafe_blocks`, `missing_safety_doc` | Unsafe code |
| API Quality | `missing_panics_doc`, `must_use_candidate` | Public APIs |
