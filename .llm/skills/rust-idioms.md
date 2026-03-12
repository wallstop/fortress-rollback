<!-- CATEGORY: Rust Language -->
<!-- WHEN: Writing idiomatic Rust, implementing traits, error handling patterns -->
# Rust Idioms

## Borrowed Types for Arguments

Accept `&str` or `&[T]` instead of `&String` or `&Vec<T>`:

```rust
fn process(data: &str) -> bool { /* ... */ }
// Works with: "literal", &String, &some_string
```

## Constructors and Default

```rust
impl Config {
    pub fn new() -> Self { Self { timeout: Duration::from_secs(30), retries: 3 } }
}
impl Default for Config {
    fn default() -> Self { Self::new() }
}
```

## String Building

Use `format!()` for readability; use `String::with_capacity()` + `push_str` in hot loops.

## `mem::take()` and `mem::replace()`

Move values out of `&mut` references without cloning:

```rust
let owned_data = mem::take(data);           // replaces with Default
let old = mem::replace(buffer, Vec::with_capacity(1024)); // replaces with specific value
```

## RAII / Finalisation in Destructors

Put cleanup in `Drop` to ensure it runs regardless of exit path. Destructors must not panic.

## On-Stack Dynamic Dispatch (Rust 1.79+)

```rust
let reader: &mut dyn Read = if use_stdin { &mut io::stdin() } else { &mut file };
```

## `#[non_exhaustive]` for Extensibility

Use on public enums/structs that may gain variants/fields in semver-compatible releases.

## Explicit Closure Captures

```rust
let closure = {
    let data = Rc::clone(&data);
    move || { println!("{:?}", data); }
};
```

## Return Consumed Argument on Error

```rust
pub struct SendError<T>(pub T);
pub fn send<T>(value: T) -> Result<(), SendError<T>> { /* ... */ }
```

## Naming Conventions (RFC 430)

| Item | Convention | Example |
|------|------------|---------|
| Types, Traits | `UpperCamelCase` | `HashMap` |
| Functions, Methods | `snake_case` | `get_value` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_SIZE` |

**Conversion prefixes:**

| Prefix | Cost | Ownership |
|--------|------|-----------|
| `as_` | Free | `&T` -> `&U` |
| `to_` | Expensive | `&T` -> `U` |
| `into_` | Variable | `T` -> `U` (consumes) |

## Implement Common Traits Eagerly

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct PlayerId(u32);
```

Do not duplicate trait functionality with standalone methods.

## Meaningful Error Types

Never use `()` as error type. Error messages: lowercase, no trailing punctuation.

## Sealed Traits

```rust
mod private { pub trait Sealed {} }
pub trait Protocol: private::Sealed { /* ... */ }
```

## Generic Functions (C-GENERIC)

Accept `impl AsRef<Path>` instead of `&PathBuf`.

## Iteration Patterns for Copy Types

```rust
// Prefer consuming iteration for Copy types
for handle in handles { session.add_local_input(handle, input)?; }

// Use .into_iter() when chaining
for (i, handle) in handles.into_iter().enumerate() { /* ... */ }

// Use .iter() only when you need the collection afterward or for non-Copy types
for &handle in handles.iter() { process(handle); }
let count = handles.len();
```

| Pattern | Loop Variable | Use When |
|---------|--------------|----------|
| `for x in collection` | Owned `T` | Copy types, consuming OK |
| `for x in collection.into_iter()` | Owned `T` | Chaining `.enumerate()`, `.filter()` |
| `for x in &collection` / `.iter()` | `&T` | Need collection after loop, non-Copy |
| `for &x in collection.iter()` | Owned `T` | Copy types when borrowing collection |

## Option `.copied()` Pattern

Use `.copied()` to convert `Option<&T>` to `Option<T>` for `Copy` types:

```rust
let byte: u8 = data.get(index).copied().ok_or(Error::OutOfBounds)?;
let owned_values: Vec<u8> = slice.iter().copied().collect();
```

| Situation | Pattern |
|-----------|---------|
| Safe indexing with `?` | `.get(i).copied().ok_or(err)?` |
| Default on missing | `.get(i).copied().unwrap_or(default)` |
| Match with value | `Some(&val) => ...` |
| Iterator of values | `.iter().copied()` |
| Non-Copy types | `.get(i).cloned()` |

## Quick Decision Guide

| Situation | Pattern |
|-----------|---------|
| Many optional constructor params | Builder |
| Wrap primitive for type safety | Newtype |
| Ensure cleanup on scope exit | RAII Guard |
| Interchangeable algorithms | Strategy (trait or closure) |
| Queueable/undoable operations | Command |
| Operations on heterogeneous tree | Visitor |
| Compile-time state machine | Type State |
| Need `&str` not `&String` | Borrowed Types |
| Fields need independent borrowing | Struct Decomposition |
