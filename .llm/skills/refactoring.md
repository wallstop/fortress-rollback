<!-- CATEGORY: Rust Language -->
<!-- WHEN: Refactoring code, planning safe transformations, verification after changes -->
# Rust Refactoring Guide

## Three Rules of Safe Refactoring

1. **Preserve Semantics** -- behavior identical unless explicitly changing it
2. **Incremental Steps** -- one change at a time, verify, proceed
3. **Test Coverage First** -- ensure tests exist before refactoring

## When to Refactor

| Signal | Action |
|--------|--------|
| `unwrap()`/`expect()` in production | Convert to `Result` propagation |
| `.clone()` without clear need | Analyze ownership, use references |
| Manual loops with indexing | Convert to iterators |
| Primitive types for domain concepts | Wrap in newtypes |
| `Vec<T>` where `&[T]` suffices | Accept slices at boundaries |
| Large enum variants | Box the large variants |

## Verification Checklist

```bash
# Pre-refactoring
cargo nextest run && cargo clippy --all-targets --features tokio,json

# After each change
cargo check && cargo nextest run

# Post-refactoring
cargo fmt && cargo clippy --all-targets --features tokio,json && cargo nextest run
rg "unwrap\(\)|expect\(|panic!\(|todo!\(" src/
```

## Refactoring Patterns Summary

| # | Pattern | From | To | Clippy Lint |
|---|---------|------|----|-------------|
| 1 | Clone to Reference | `fn(x: String)` | `fn(x: &str)` | `needless_pass_by_value` |
| 2 | Unnecessary `to_string()` | `name.to_string()` for lookup | Direct `&str` | `unnecessary_to_owned` |
| 3 | Pre-allocate Collections | `Vec::new()` + push loop | `Vec::with_capacity(n)` | -- |
| 4 | Reuse Collections in Loops | New buffer each iteration | `.clear()` + reuse | -- |
| 5 | `clone_from` | `x = y.clone()` | `x.clone_from(&y)` | -- |
| 6 | Index Loop to Iterator | `for i in 0..len` | `.iter()` chain | -- |
| 7 | Avoid Intermediate `collect()` | `-> Vec<T>` | `-> impl Iterator` | -- |
| 8 | `chunks_exact` | `.chunks(n)` | `.chunks_exact(n)` | -- |
| 9 | `iter().copied()` | `.iter().map(\|&b\| b)` | `.iter().copied()` | -- |
| 10 | `swap_remove` | `.remove(i)` unordered | `.swap_remove(i)` | -- |
| 11 | `unwrap()` to `?` | `.unwrap()` | `.ok_or(err)?` | `unwrap_used` |
| 12 | `expect()` to Error | `.expect(msg)` | `.map_err(\|e\| Err)?` | `expect_used` |
| 13 | Index to `get()` | `collection[i]` | `.get(i)?` | `indexing_slicing` |
| 14 | Silent Swallow to Explicit | `let _ = result` | Propagate or handle | -- |
| 15 | `unwrap_or` to `unwrap_or_else` | `unwrap_or(expensive())` | `unwrap_or_else(\|\| ...)` | `or_fun_call` |
| 16 | `&String`/`&Vec` to slice | `fn(v: &Vec<T>)` | `fn(v: &[T])` | `ptr_arg` |
| 17 | Return Borrowed to Owned | Complex lifetime returns | `Cow<'_, str>` | -- |
| 18 | `mem::take` | Clone + clear | `mem::take(&mut x)` | -- |
| 19 | Struct Decomposition | `&mut self` conflicts | Direct field access / `parts_mut()` | -- |
| 20 | Cow for Conditional Ownership | Always clone | `Cow::Borrowed`/`Cow::Owned` | -- |
| 21 | Primitive to Newtype | `fn(from: u64, to: u64)` | Type-safe wrappers | -- |
| 22 | Bool to Enum | `fn(secure: bool)` | `Security::Secure` | -- |
| 23 | Option Fields to Types | `email: Option<String>` | `VerifiedUser { email: String }` | -- |
| 24 | String Status to Enum | `status: String` | `TaskStatus::Complete` | -- |
| 25 | Checked Arithmetic | `center - radius` | `center.checked_sub(radius)?` | `arithmetic_side_effects` |

## Common Pitfalls

1. **Over-optimizing cold paths** -- profile first, optimize hot paths
2. **Breaking trait bounds** -- don't add new bounds to existing generics
3. **Changing public return types** -- `Vec<T>` to `impl Iterator` is breaking
4. **Clone removal causing borrow conflicts** -- analyze why clone existed
5. **`mem::take` with invalid Default** -- use `mem::replace` with valid replacement
6. **Iterator refactoring changes order** -- sequential iterators if order matters
7. **Changing error variant structure** -- use `#[non_exhaustive]` on error enums

## Automated Transformation Checklist

### Pre-Refactoring
- [ ] Tests pass: `cargo nextest run`
- [ ] No clippy warnings: `cargo clippy --all-targets --features tokio,json`
- [ ] Git commit current changes

### Post-Refactoring
- [ ] All tests pass
- [ ] No new clippy warnings
- [ ] No new panicking patterns
- [ ] Benchmarks show expected improvement (if performance-related)
- [ ] Documentation updated if API changed
- [ ] CHANGELOG updated if user-facing change
