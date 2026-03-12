<!-- CATEGORY: Rust Language -->
<!-- WHEN: Designing type-safe APIs, using newtypes, encoding state in types -->
# Type-Driven Design -- Parse, Don't Validate

**Core insight:** Validation checks data and discards the proof; parsing checks data and encodes the proof in the type system.

## Parse at Boundaries

```rust
// Validation: proof thrown away -- caller still has &[T]
validate_non_empty(&items)?;
process(&items); // nothing prevents calling without validation

// Parsing: proof preserved in type
let items = parse_non_empty(items)?; // returns NonEmpty<T>
process(&items); // guaranteed non-empty
```

**Principles:** Parse at boundaries, strengthen arguments, let types inform code, avoid shotgun parsing.

## The `let-else` Pattern

```rust
let Some(config) = load_config() else {
    return Err(FortressError::ConfigNotFound);
};
```

## Newtypes for Domain Concepts

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame(u32);

impl Frame {
    pub const ZERO: Frame = Frame(0);
    pub fn new(value: u32) -> Self { Frame(value) }
    pub fn checked_add(self, delta: u32) -> Option<Frame> {
        self.0.checked_add(delta).map(Frame)
    }
}
```

## NonEmpty Collections

```rust
pub struct NonEmpty<T> { first: T, rest: Vec<T> }

impl<T> NonEmpty<T> {
    pub fn from_vec(vec: Vec<T>) -> Option<Self> {
        let mut iter = vec.into_iter();
        iter.next().map(|first| Self { first, rest: iter.collect() })
    }
    pub fn first(&self) -> &T { &self.first } // always succeeds
}
```

## Enums for Finite States

```rust
enum ConnectionState {
    Disconnected,
    Connecting { attempt: u32 },
    Connected { since: Instant },
    Reconnecting { attempts: u32, last_error: String },
}
```

## Validated Types with Serde

```rust
impl TryFrom<String> for Username {
    type Error = UsernameError;
    fn try_from(value: String) -> Result<Self, Self::Error> { Username::new(&value) }
}

impl<'de> Deserialize<'de> for Username {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Username::try_from(s).map_err(serde::de::Error::custom)
    }
}
```

## Typestate Pattern

```rust
struct Disconnected;
struct Connected;

struct Connection<State> { inner: ConnectionInner, _state: PhantomData<State> }

impl Connection<Disconnected> {
    pub fn connect(self) -> Connection<Connecting> { /* ... */ }
}
impl Connection<Connected> {
    pub fn send(&mut self, data: &[u8]) -> Result<(), SendError> { /* ... */ }
    pub fn disconnect(self) -> Connection<Disconnected> { /* ... */ }
}
```

**Use typestate when:** Linear transitions, wrong-state calls are serious bugs, want compile-time guarantees.
**Use enums when:** Heterogeneous collections, dynamic state changes, complex transition graphs.

## Builder Pattern with Validation

```rust
#[must_use = "builders do nothing until .build() is called"]
pub struct SessionBuilder { num_players: Option<usize>, /* ... */ }

impl SessionBuilder {
    pub fn build(self) -> Result<Session, SessionBuildError> {
        let num_players = self.num_players
            .ok_or(SessionBuildError::MissingField { field: "num_players" })?;
        if num_players == 0 || num_players > MAX_PLAYERS {
            return Err(SessionBuildError::InvalidPlayerCount { count: num_players });
        }
        Ok(Session { num_players, /* ... */ })
    }
}
```

## Error Type Design

| | Internal (within crate) | At Boundary (API edge) |
|---|---|---|
| Control Flow | Match on variants | Status codes, enums |
| Reporting | Logs, traces | Response bodies |

**thiserror:** Libraries, callers match on variants. **anyhow:** Applications, opaque errors.

## Common Pitfalls

- **`Path::join` with absolute paths** replaces the base entirely. Validate relative.
- **Blanket `#[derive(Default)]`** may produce invalid state (port=0, timeout=0). Use explicit defaults.
- **Empty string defaults in serde:** Validate via `TryFrom` or custom deserializer.

## Principles Summary

1. Parse at boundaries -- convert raw data to validated types immediately
2. Make invalid states unrepresentable -- newtypes, enums, NonEmpty
3. Use `let-else` -- cleanest Option handling without panic
4. Typestate for state machines -- compile-time transition enforcement
5. Validate in constructors -- types always valid after construction
6. Rich error types -- enough context to debug
