# Type-Driven Design — Parse, Don't Validate

> **This document covers type-driven design patterns that complement defensive programming.**
> These patterns use Rust's type system to eliminate entire classes of bugs at compile time.

## Core Philosophy: Parse, Don't Validate

The key insight: **validation checks data and discards the proof; parsing checks data and encodes the proof in the type system.**

```rust
// ❌ Validation: proof is thrown away
fn validate_non_empty(list: &[T]) -> Result<(), Error> {
    if list.is_empty() { return Err(Error::Empty); }
    Ok(())  // Caller still has &[T], must remember it's non-empty
}

// Caller:
validate_non_empty(&items)?;
process(&items);  // Nothing prevents calling process() without validation!

// ✅ Parsing: proof is preserved in type
fn parse_non_empty<T>(list: Vec<T>) -> Result<NonEmpty<T>, Error> {
    NonEmpty::try_from(list)  // Caller gets NonEmpty<T> - proof embedded
}

// Caller:
let items = parse_non_empty(items)?;
process(&items);  // items is NonEmpty<T> - guaranteed non-empty!
```

### Key Principles

1. **Parse at boundaries** — Validate/parse at system edges, work with strong types internally
2. **Strengthen arguments** — Instead of `head(&[T]) -> Option<T>`, use `head(&NonEmpty<T>) -> T`
3. **Let types inform code** — If a function can't fail, its signature shouldn't include `Result`
4. **Avoid shotgun parsing** — Don't spread validation; parse once, up front

---

## The `let-else` Pattern for Option Handling

The `let-else` pattern is the **preferred way to handle `Option`** without panicking:

```rust
// ✅ PREFERRED - Clear, concise, idiomatic
let Some(config) = load_config() else {
    return Err(FortressError::ConfigNotFound);
};
// config is now guaranteed to exist, unwrapped

// ✅ OK - When you need error context
let config = load_config()
    .ok_or_else(|| FortressError::ConfigNotFound { path: config_path.clone() })?;

// ✅ OK - Match when logic is complex
let config = match load_config() {
    Some(c) if c.is_valid() => c,
    Some(_) => return Err(FortressError::InvalidConfig),
    None => return Err(FortressError::ConfigNotFound),
};

// ❌ FORBIDDEN - Panic
let config = load_config().unwrap();
let config = load_config().expect("config should exist");
```

---

## Make Invalid States Unrepresentable

### Newtypes for Domain Concepts

Wrap primitives to add meaning and validation:

```rust
// ❌ Primitives allow any value
struct Session {
    frame: i32,       // Could be negative!
    player: usize,    // Could be out of bounds!
}

// ✅ Newtypes constrain to valid values
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame(u32);  // Can never be negative

impl Frame {
    pub const ZERO: Frame = Frame(0);
    pub const MAX: Frame = Frame(u32::MAX);

    pub fn new(value: u32) -> Self {
        Frame(value)
    }

    pub fn checked_add(self, delta: u32) -> Option<Frame> {
        self.0.checked_add(delta).map(Frame)
    }

    pub fn saturating_sub(self, delta: u32) -> Frame {
        Frame(self.0.saturating_sub(delta))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerHandle(usize);  // Only created through validated constructor
```

### NonEmpty Collections

Guarantee at least one element:

```rust
/// A vector guaranteed to contain at least one element.
pub struct NonEmpty<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> NonEmpty<T> {
    pub fn new(first: T) -> Self {
        Self { first, rest: Vec::new() }
    }

    pub fn from_vec(vec: Vec<T>) -> Option<Self> {
        let mut iter = vec.into_iter();
        iter.next().map(|first| Self {
            first,
            rest: iter.collect(),
        })
    }

    /// Always succeeds - we guarantee at least one element
    pub fn first(&self) -> &T {
        &self.first
    }

    pub fn len(&self) -> usize {
        1 + self.rest.len()
    }
}
```

### Enums for Finite States

Use enums instead of stringly-typed or numeric state:

```rust
// ❌ String/integer state allows invalid values
struct Connection {
    state: String,  // "connected", "disconnected", "conected" (typo!)
    status_code: i32,  // What does 42 mean?
}

// ✅ Enum makes invalid states impossible
enum ConnectionState {
    Disconnected,
    Connecting { attempt: u32 },
    Connected { since: Instant },
    Reconnecting { attempts: u32, last_error: String },
}

struct Connection {
    state: ConnectionState,
}
```

---

## Validated Types at Boundaries

### Serde Integration

Use `#[serde(try_from)]` to validate during deserialization:

```rust
use serde::{Deserialize, Serialize};

// The internal validated type
#[derive(Clone, Debug, Serialize)]
pub struct Username(String);

impl Username {
    pub fn new(name: &str) -> Result<Self, UsernameError> {
        if name.is_empty() {
            return Err(UsernameError::Empty);
        }
        if name.len() > 32 {
            return Err(UsernameError::TooLong { len: name.len() });
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(UsernameError::InvalidCharacters);
        }
        Ok(Self(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Username {
    type Error = UsernameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Username::new(&value)
    }
}

// Serde automatically validates on deserialize
impl<'de> Deserialize<'de> for Username {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Username::try_from(s).map_err(serde::de::Error::custom)
    }
}
```

---

## Typestate Pattern

Encode state machine transitions in the type system:

```rust
use std::marker::PhantomData;

// States as zero-sized types
struct Disconnected;
struct Connecting;
struct Connected;

struct Connection<State> {
    inner: ConnectionInner,
    _state: PhantomData<State>,
}

// Methods only available in specific states
impl Connection<Disconnected> {
    pub fn connect(self) -> Connection<Connecting> {
        // ... initiate connection
        Connection {
            inner: self.inner,
            _state: PhantomData,
        }
    }
}

impl Connection<Connecting> {
    pub fn wait_connected(self) -> Result<Connection<Connected>, ConnectionError> {
        // ... wait for connection
        Ok(Connection {
            inner: self.inner,
            _state: PhantomData,
        })
    }

    pub fn cancel(self) -> Connection<Disconnected> {
        Connection {
            inner: self.inner,
            _state: PhantomData,
        }
    }
}

impl Connection<Connected> {
    // send() only exists on Connected - impossible to call on Disconnected!
    pub fn send(&mut self, data: &[u8]) -> Result<(), SendError> {
        // ...
        Ok(())
    }

    pub fn disconnect(self) -> Connection<Disconnected> {
        Connection {
            inner: self.inner,
            _state: PhantomData,
        }
    }
}
```

### When to Use Typestate

**Use typestate when:**

- State transitions are linear or have clear rules
- Calling methods in wrong state is a serious bug
- You want compile-time guarantees

**Use enums instead when:**

- You need to store connections in heterogeneous collections
- State can change dynamically based on external events
- The state machine is complex with many valid transitions

---

## Builder Pattern with Validation

Use builders that validate and return `Result`:

```rust
#[must_use = "builders do nothing until .build() is called"]
pub struct SessionBuilder<Config> {
    num_players: Option<usize>,
    input_delay: Option<usize>,
    max_prediction: Option<usize>,
    _config: PhantomData<Config>,
}

impl<Config> SessionBuilder<Config> {
    pub fn new() -> Self {
        Self {
            num_players: None,
            input_delay: None,
            max_prediction: None,
            _config: PhantomData,
        }
    }

    pub fn with_num_players(mut self, count: usize) -> Self {
        self.num_players = Some(count);
        self
    }

    pub fn with_input_delay(mut self, frames: usize) -> Self {
        self.input_delay = Some(frames);
        self
    }

    pub fn with_max_prediction(mut self, frames: usize) -> Self {
        self.max_prediction = Some(frames);
        self
    }

    pub fn build(self) -> Result<Session<Config>, SessionBuildError> {
        let num_players = self.num_players
            .ok_or(SessionBuildError::MissingField { field: "num_players" })?;
        let input_delay = self.input_delay
            .ok_or(SessionBuildError::MissingField { field: "input_delay" })?;
        let max_prediction = self.max_prediction.unwrap_or(8);

        // Validation
        if num_players == 0 || num_players > MAX_PLAYERS {
            return Err(SessionBuildError::InvalidPlayerCount { count: num_players });
        }
        if input_delay > max_prediction {
            return Err(SessionBuildError::InputDelayTooHigh {
                input_delay,
                max_prediction,
            });
        }

        Ok(Session {
            num_players,
            input_delay,
            max_prediction,
            _config: PhantomData,
        })
    }
}
```

---

## Error Type Design

### The Error Handling Matrix

|               | **Internal** (within crate) | **At Boundary** (API edge) |
|---------------|------------------------------|----------------------------|
| **Control Flow** | Match on error variants | Status codes, enums |
| **Reporting** | Logs, traces, metrics | Response bodies, messages |

### When to Use `thiserror` vs `anyhow`

**Use `thiserror` when:**

- Building a library
- Callers need to match on error variants
- You want structured, inspectable errors

**Use `anyhow` when:**

- Building an application
- Errors are opaque (just report them)
- You want convenience over structure

### Rich Error Context

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("player {index} not found (session has {count} players)")]
    PlayerNotFound { index: usize, count: usize },

    #[error("cannot advance frame {requested} (current frame is {current})")]
    InvalidFrameAdvance { requested: Frame, current: Frame },

    #[error("network operation '{operation}' failed")]
    NetworkError {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },

    #[error("invariant violated: {invariant}")]
    InvariantViolation {
        invariant: &'static str,
        context: String,
    },
}
```

---

## Common Pitfalls to Avoid

### Path::join with Absolute Paths

```rust
// ⚠️ Surprising behavior - absolute path replaces base!
let base = Path::new("/home/user");
let result = base.join("/etc/passwd");  // Returns "/etc/passwd", not "/home/user/etc/passwd"!

// ✅ Strip leading slash or validate
fn safe_join(base: &Path, relative: &Path) -> Result<PathBuf, Error> {
    if relative.is_absolute() {
        return Err(Error::AbsolutePathNotAllowed);
    }
    Ok(base.join(relative))
}
```

### Blanket Default Implementations

```rust
// ❌ Avoid: May create invalid/unexpected state
#[derive(Default)]
struct Config {
    port: u16,      // Default is 0 - probably wrong!
    timeout_ms: u64, // Default is 0 - infinite timeout?
}

// ✅ Prefer: Explicit, meaningful defaults
impl Config {
    pub fn new() -> Self {
        Self {
            port: 8080,
            timeout_ms: 30_000,
        }
    }
}

// Or use builder pattern for required fields
```

### Empty String Defaults in Serde

```rust
// ❌ Accepts empty strings silently
#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    api_key: String,  // "" is probably not valid!
}

// ✅ Validate via TryFrom
#[derive(Deserialize)]
struct Config {
    #[serde(deserialize_with = "deserialize_non_empty")]
    api_key: String,
}

fn deserialize_non_empty<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        return Err(serde::de::Error::custom("value cannot be empty"));
    }
    Ok(s)
}
```

---

## Summary

Type-driven design principles:

1. **Parse at boundaries** — Convert raw data to validated types immediately
2. **Make invalid states unrepresentable** — Use newtypes, enums, NonEmpty
3. **Use `let-else`** — The cleanest way to handle Option without panic
4. **Typestate for state machines** — When state transitions must be enforced
5. **Validate in constructors** — Types should always be valid after construction
6. **Rich error types** — Enough context to debug without guessing

---

*See also: [defensive-programming.md](defensive-programming.md) for zero-panic policy and error handling patterns.*
