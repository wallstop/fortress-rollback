# Public API Design — Crafting Ergonomic and Stable Rust APIs

> **This document provides comprehensive guidance for designing public APIs in Rust crates.**
> A well-designed API is minimal, stable, ergonomic, and impossible to misuse.

## TL;DR — Quick Reference

```bash
# API stability checks
cargo install cargo-semver-checks
cargo semver-checks check-release

# Documentation coverage
cargo doc --no-deps --all-features
cargo test --doc  # Run doctests

# Lint for missing docs
# Add to lib.rs: #![warn(missing_docs)]
```

**Key Principles:**

1. **Minimize surface area** — Expose only what users truly need
2. **Hide implementation details** — Use newtypes and `pub(crate)`
3. **Design for stability** — Use `#[non_exhaustive]` judiciously
4. **Document everything** — Every public item needs rustdoc
5. **Re-export dependencies** — Users shouldn't hunt for your deps

---

## API Surface Minimization

### Core Philosophy

**Default to private.** Every public item is a commitment you must maintain forever.

```rust
// ❌ ANTI-PATTERN: Exposing too much
pub mod internal;           // Leaks implementation details
pub struct SessionInner;    // Naming hints it shouldn't be public
pub fn helper_function();   // Generic name suggests internal use

// ✅ CORRECT: Minimal public surface
mod internal;               // Private by default
pub struct Session;         // Clean public name
pub fn create_session();    // Specific, documented purpose
```

### Visibility Levels

Use the most restrictive visibility that works:

```rust
// ✅ Private by default - accessible only within this module
fn internal_helper() { }
struct InternalState { }

// ✅ pub(crate) - accessible anywhere in this crate
pub(crate) fn crate_internal_function() { }
pub(crate) struct CrateInternalType { }

// ✅ pub(super) - accessible in parent module
pub(super) fn parent_module_helper() { }

// ✅ pub(in crate::module) - accessible in specific module tree
pub(in crate::network) fn network_only_function() { }

// ⚠️  pub - accessible to everyone, forever
// Only use when INTENTIONALLY part of public API
pub fn documented_public_function() { }
```

### Struct Field Visibility

**Never expose struct fields directly** unless they're fundamental to the type's meaning:

```rust
// ❌ ANTI-PATTERN: Public fields leak implementation
pub struct Session {
    pub frame: u32,           // Can't change type later
    pub players: Vec<Player>, // Can't change representation
    pub state: SessionState,  // Exposes internal state machine
}

// ✅ CORRECT: Private fields with accessor methods
pub struct Session {
    frame: Frame,
    players: Players,
    state: SessionState,
}

impl Session {
    /// Returns the current frame number.
    pub fn current_frame(&self) -> Frame {
        self.frame
    }

    /// Returns the number of players in the session.
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    // No getter for `state` - truly internal
}
```

### Module Organization for Visibility

Structure modules to naturally limit visibility:

```rust
// src/lib.rs
pub mod session;      // Public module
mod internal;         // Private module - implementation details
mod protocol;         // Private module

pub use session::Session;  // Re-export only the public type

// src/session.rs
pub struct Session { /* ... */ }

// These are pub but only visible because parent module is pub
pub fn create_session() -> Session { /* ... */ }

// src/internal.rs
// Everything here is crate-private because module is private
pub(crate) struct InternalHelper { /* ... */ }
```

---

## Enum Design and Extensibility

### When to Use `#[non_exhaustive]`

Use `#[non_exhaustive]` for enums that **may gain variants** in future versions:

```rust
// ✅ CORRECT: Error types often grow
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SessionError {
    #[error("network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("protocol error: {message}")]
    Protocol { message: String },

    #[error("session full")]
    SessionFull,
}

// Future minor version can add:
// InvalidInput { reason: String },
// Timeout { duration: Duration },
```

### When NOT to Use `#[non_exhaustive]`

**Avoid** for enums where exhaustive matching is important:

```rust
// ✅ CORRECT: Fixed set of states - exhaustive matching catches bugs
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

// Users SHOULD match exhaustively:
match state {
    ConnectionState::Disconnected => handle_disconnected(),
    ConnectionState::Connecting => handle_connecting(),
    ConnectionState::Connected => handle_connected(),
    ConnectionState::Disconnecting => handle_disconnecting(),
    // No wildcard - compiler error if new state added
}
```

### The Trade-off

```rust
// With #[non_exhaustive], users MUST have catch-all:
match error {
    SessionError::Network(e) => /* ... */,
    SessionError::Protocol { .. } => /* ... */,
    SessionError::SessionFull => /* ... */,
    _ => /* catch-all hides new variants! */,
}

// ⚠️  WARNING: Catch-all patterns can hide bugs!
// New error variants won't get explicit handling.
// Only use #[non_exhaustive] if catch-all is acceptable.
```

### Non-Exhaustive Structs

For structs that may gain fields:

```rust
// ✅ Allows adding fields in minor versions
#[non_exhaustive]
pub struct SessionConfig {
    pub max_players: usize,
    pub prediction_window: u32,
    // Future: pub enable_spectators: bool,
}

// Users must use struct update syntax or builder
impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_players: 4,
            prediction_window: 8,
        }
    }
}

// Usage:
let config = SessionConfig {
    max_players: 2,
    ..Default::default()  // Required due to #[non_exhaustive]
};
```

---

## Newtype Pattern — Hiding Implementation Details

### Wrapping External Types

**Never expose dependency types directly in your public API:**

```rust
// ❌ ANTI-PATTERN: Exposes reqwest in public API
pub fn send_request(req: reqwest::Request) -> reqwest::Response { }

// If you change HTTP libraries, ALL users must update their code!

// ✅ CORRECT: Wrap external types
pub struct Request(reqwest::Request);
pub struct Response(reqwest::Response);

impl Request {
    pub fn new(url: &str) -> Result<Self, Error> {
        // Construction validates and wraps
        Ok(Self(reqwest::Request::new(
            reqwest::Method::GET,
            url.parse()?,
        )))
    }
}

pub fn send_request(req: Request) -> Result<Response, Error> {
    // Internal implementation can change freely
}
```

### Type Safety Through Newtypes

Create distinct types for values that shouldn't be mixed:

```rust
// ❌ ANTI-PATTERN: Easy to mix up arguments
pub fn create_session(
    max_players: usize,
    max_frames: usize,
    port: usize,
) -> Session { }

// Called incorrectly - compiles but bugs:
create_session(8080, 4, 60);  // Oops! Port, players, frames wrong order

// ✅ CORRECT: Distinct types prevent mistakes
#[derive(Clone, Copy, Debug)]
pub struct MaxPlayers(usize);

#[derive(Clone, Copy, Debug)]
pub struct MaxFrames(usize);

#[derive(Clone, Copy, Debug)]
pub struct Port(u16);

pub fn create_session(
    max_players: MaxPlayers,
    max_frames: MaxFrames,
    port: Port,
) -> Session { }

// Compiler catches mistakes:
// create_session(Port(8080), MaxPlayers(4), MaxFrames(60));  // Error!
create_session(MaxPlayers(4), MaxFrames(60), Port(8080));  // Correct!
```

### Newtype Implementation Patterns

```rust
/// A frame number in the simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Frame(u32);

impl Frame {
    /// The first frame of any session.
    pub const ZERO: Self = Self(0);

    /// Creates a new frame number.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw frame number.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Adds frames, returning `None` on overflow.
    pub fn checked_add(self, delta: u32) -> Option<Self> {
        self.0.checked_add(delta).map(Self)
    }

    /// Subtracts frames, saturating at zero.
    pub fn saturating_sub(self, delta: u32) -> Self {
        Self(self.0.saturating_sub(delta))
    }
}

// Implement traits for ergonomics
impl std::fmt::Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "frame {}", self.0)
    }
}

impl From<u32> for Frame {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
```

---

## Trait Design

### Avoid Overly Complex Bounds

```rust
// ❌ ANTI-PATTERN: Trait bound explosion
pub trait Config:
    Clone + Send + Sync + Default + Debug +
    Serialize + DeserializeOwned + PartialEq + Eq + Hash
{ }

// Users struggle to implement this!

// ✅ CORRECT: Minimal bounds, document why each is needed
pub trait Config: Clone + Send + Sync {
    // Clone: needed for copying config between threads
    // Send + Sync: config shared across threads
}

// If you need serialization, make it optional:
#[cfg(feature = "serde")]
impl<T: Config + Serialize + DeserializeOwned> SerializableConfig for T { }
```

### Single vs. Multiple Traits

```rust
// ⚠️  QUESTIONABLE: Many fine-grained traits
pub trait Read { fn read(&mut self, buf: &mut [u8]) -> Result<usize>; }
pub trait Write { fn write(&mut self, buf: &[u8]) -> Result<usize>; }
pub trait Seek { fn seek(&mut self, pos: SeekFrom) -> Result<u64>; }
pub trait Close { fn close(self) -> Result<()>; }

// Users need to import and bound on all of them:
fn process<T: Read + Write + Seek + Close>(io: T) { }

// ✅ BETTER FOR USABILITY: Combined trait with blanket impl
pub trait ReadWrite: Read + Write { }
impl<T: Read + Write> ReadWrite for T { }

// Or a single comprehensive trait if operations are always used together:
pub trait Stream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;
}
```

### Re-Export Traits Users Need

```rust
// If your API returns or accepts types that need traits:
// src/lib.rs

// ✅ Re-export so users don't need to add bytes to their Cargo.toml
pub use bytes::{Bytes, BytesMut, Buf, BufMut};

// Now users can:
// use my_crate::{Session, Bytes, BufMut};
```

### Associated Types for Flexibility

```rust
// ✅ Associated types let implementations choose their own types
pub trait Transport {
    type Error: std::error::Error + Send + Sync + 'static;
    type Address: Clone + Send + Sync;

    fn send(&mut self, addr: &Self::Address, data: &[u8]) -> Result<(), Self::Error>;
    fn recv(&mut self) -> Result<(Self::Address, Vec<u8>), Self::Error>;
}

// UDP implementation
impl Transport for UdpTransport {
    type Error = std::io::Error;
    type Address = SocketAddr;

    fn send(&mut self, addr: &SocketAddr, data: &[u8]) -> Result<(), std::io::Error> {
        self.socket.send_to(data, addr)?;
        Ok(())
    }

    fn recv(&mut self) -> Result<(SocketAddr, Vec<u8>), std::io::Error> {
        let mut buf = vec![0u8; 1500];
        let (len, addr) = self.socket.recv_from(&mut buf)?;
        buf.truncate(len);
        Ok((addr, buf))
    }
}
```

---

## Error Handling in Public APIs

### Distinct Error Types

```rust
// ❌ ANTI-PATTERN: One error type for everything
pub enum Error {
    Io(std::io::Error),
    Parse(ParseError),
    Network(NetworkError),
    Config(ConfigError),
    // 50 more variants...
}

// Users can't tell what errors a function might return

// ✅ CORRECT: Scoped error types
pub mod session {
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum CreateError {
        #[error("invalid configuration: {reason}")]
        InvalidConfig { reason: String },

        #[error("network binding failed: {0}")]
        NetworkBind(#[source] std::io::Error),
    }

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum AdvanceError {
        #[error("session not connected")]
        NotConnected,

        #[error("desync detected at frame {frame}")]
        Desync { frame: u32 },
    }

    pub fn create(config: Config) -> Result<Session, CreateError> { /* ... */ }

    impl Session {
        pub fn advance(&mut self) -> Result<(), AdvanceError> { /* ... */ }
    }
}
```

### Error Context

```rust
// ✅ Provide context for debugging
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("failed to read config file '{path}'")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid config at {path}:{line}:{column}")]
    Parse {
        path: PathBuf,
        line: usize,
        column: usize,
        #[source]
        source: ParseError,
    },
}
```

### Document Error Conditions

```rust
/// Creates a new session with the given configuration.
///
/// # Errors
///
/// Returns [`CreateError::InvalidConfig`] if:
/// - `config.max_players` is zero or exceeds 16
/// - `config.prediction_window` exceeds 64 frames
///
/// Returns [`CreateError::NetworkBind`] if the UDP socket
/// cannot bind to the specified port.
///
/// # Examples
///
/// ```
/// use my_crate::{Session, SessionConfig};
///
/// let config = SessionConfig::default();
/// let session = Session::create(config)?;
/// # Ok::<(), my_crate::CreateError>(())
/// ```
pub fn create(config: SessionConfig) -> Result<Session, CreateError> {
    // ...
}
```

---

## Documentation Best Practices

### Enforce Documentation Coverage

```rust
// src/lib.rs
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

//! # My Crate
//!
//! A brief description of what this crate does.
//!
//! ## Quick Start
//!
//! ```rust
//! use my_crate::Session;
//!
//! let session = Session::new()?;
//! session.start()?;
//! # Ok::<(), my_crate::Error>(())
//! ```
//!
//! ## Feature Flags
//!
//! - `serde`: Enable serialization support
//! - `async`: Enable async runtime support
```

### Document All Public Items

```rust
/// A multiplayer game session.
///
/// `Session` manages the connection state, input synchronization,
/// and rollback/resimulation logic for a multiplayer game.
///
/// # Thread Safety
///
/// `Session` is `Send` but not `Sync`. It must be accessed from
/// a single thread, but can be transferred between threads.
///
/// # Examples
///
/// Creating and running a basic session:
///
/// ```
/// use my_crate::{Session, SessionConfig, Input};
///
/// let config = SessionConfig {
///     max_players: 2,
///     ..Default::default()
/// };
///
/// let mut session = Session::create(config)?;
///
/// // Game loop
/// loop {
///     let local_input = get_local_input();
///     session.add_local_input(local_input)?;
///
///     match session.advance()? {
///         AdvanceResult::Run { inputs } => {
///             game.simulate(&inputs);
///         }
///         AdvanceResult::Wait => {
///             // Waiting for remote inputs
///         }
///     }
/// }
/// # Ok::<(), my_crate::Error>(())
/// ```
pub struct Session { /* ... */ }
```

### Use Intra-Doc Links

```rust
/// Advances the session by one frame.
///
/// This method should be called once per game tick. It handles:
/// - Sending local inputs to remote players
/// - Receiving and validating remote inputs
/// - Triggering rollback if needed (see [`Session::rollback_count`])
///
/// # Errors
///
/// Returns [`AdvanceError::NotConnected`] if the session hasn't
/// completed the handshake. Call [`Session::is_connected`] first.
///
/// Returns [`AdvanceError::Desync`] if checksum validation fails.
/// See the [desync detection guide](crate::desync) for details.
///
/// # See Also
///
/// - [`Session::add_local_input`] — must be called before `advance`
/// - [`AdvanceResult`] — the return type of this method
pub fn advance(&mut self) -> Result<AdvanceResult, AdvanceError> { }
```

---

## Re-exports and Preludes

### Re-export Public Dependencies

```rust
// Cargo.toml
[dependencies]
bytes = "1.0"
tokio = { version = "1.0", features = ["net"] }

// src/lib.rs
// ✅ Re-export types that appear in your public API
pub use bytes::{Bytes, BytesMut};

// For traits, re-export if users need them
pub use bytes::{Buf, BufMut};

// Now users don't need `bytes` in their Cargo.toml
// to use your API
```

### Creating a Prelude

```rust
// src/prelude.rs
//! Convenient re-exports for common usage.
//!
//! ```
//! use my_crate::prelude::*;
//! ```

pub use crate::{
    // Core types
    Session,
    SessionConfig,
    Input,

    // Common traits
    Transport,
    Checksum,

    // Error types
    SessionError,
    CreateError,
    AdvanceError,

    // Result aliases
    Result,
};

// src/lib.rs
pub mod prelude;
```

### Result Type Aliases

```rust
// ✅ Convenient result alias for the crate's error type
pub type Result<T, E = Error> = std::result::Result<T, E>;

// Users can write:
pub fn my_function() -> my_crate::Result<Session> { }

// Instead of:
pub fn my_function() -> Result<Session, my_crate::Error> { }
```

---

## Feature Flags

### Document All Features

```rust
// Cargo.toml
[features]
default = []

# Enable serde serialization/deserialization support.
# Adds Serialize/Deserialize implementations to public types.
serde = ["dep:serde", "dep:serde_derive"]

# Enable async I/O support with Tokio runtime.
async = ["dep:tokio"]

# Enable all optional features.
full = ["serde", "async"]

[dependencies]
serde = { version = "1.0", optional = true }
serde_derive = { version = "1.0", optional = true }
tokio = { version = "1.0", optional = true, features = ["net", "rt"] }
```

```rust
// src/lib.rs
//! ## Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `serde` | Serialization support for config and state |
//! | `async` | Async socket support via Tokio |
//! | `full`  | Enables all optional features |
```

### Features Must Be Additive

```rust
// ❌ ANTI-PATTERN: Mutually exclusive features
[features]
runtime-tokio = ["dep:tokio"]
runtime-async-std = ["dep:async-std"]
# User enables both → compilation error or undefined behavior

// ✅ CORRECT: Features are additive
[features]
tokio = ["dep:tokio"]
async-std = ["dep:async-std"]
# Both can be enabled simultaneously
# Code uses cfg to select implementation
```

```rust
// Handle multiple features being enabled
#[cfg(feature = "tokio")]
mod tokio_impl;

#[cfg(feature = "async-std")]
mod async_std_impl;

// Users choose which to use via type selection, not feature exclusion
```

---

## Versioning and Compatibility

### Follow Semantic Versioning Strictly

```text
MAJOR.MINOR.PATCH

MAJOR: Breaking API changes
MINOR: New features, backward compatible
PATCH: Bug fixes only

Pre-1.0.0:
- 0.MINOR.PATCH
- MINOR = breaking changes allowed
- PATCH = new features and fixes
```

### Breaking Changes Checklist

These are breaking changes requiring a MAJOR version bump (or MINOR pre-1.0):

- [ ] Removing public items (functions, types, modules)
- [ ] Changing function signatures
- [ ] Adding required fields to structs
- [ ] Adding required trait methods (without defaults)
- [ ] Changing enum variants (unless `#[non_exhaustive]`)
- [ ] Changing type of public fields
- [ ] Tightening trait bounds
- [ ] Removing trait implementations
- [ ] Changing MSRV (Minimum Supported Rust Version)

### Use cargo-semver-checks

```bash
# Install
cargo install cargo-semver-checks

# Check before release
cargo semver-checks check-release

# Example output showing detected breaking change:
# BREAKING: Function `Session::new` has been removed
# BREAKING: Struct `Config` has a new required field `timeout`
```

### Migration Guides

For major versions, provide a migration guide:

```markdown
# Migrating from v1.x to v2.0

## Breaking Changes

### Session::new() → Session::create()

The `new()` constructor has been renamed to `create()` and now
returns a `Result`:

```rust
// v1.x
let session = Session::new(config);

// v2.0
let session = Session::create(config)?;
```

### Config struct changes

`Config` now requires a `timeout` field:

```rust
// v1.x
let config = Config {
    max_players: 4,
};

// v2.0
let config = Config {
    max_players: 4,
    timeout: Duration::from_secs(10),
    ..Default::default()
};
```

```

---

## API Review Checklist

Use this checklist when designing or reviewing public APIs:

### Visibility

- [ ] Are all items private by default?
- [ ] Is `pub(crate)` used for crate-internal items?
- [ ] Are struct fields private with accessor methods?
- [ ] Are internal modules private?
- [ ] Are implementation details hidden behind newtypes?

### Types and Traits

- [ ] Do newtypes wrap external dependency types?
- [ ] Are similar primitives distinguished by type (e.g., `Frame`, `Player`)?
- [ ] Are trait bounds minimal and documented?
- [ ] Are commonly-used trait combinations provided?
- [ ] Are traits users need re-exported?

### Enums

- [ ] Is `#[non_exhaustive]` used only where catch-all is acceptable?
- [ ] Are fixed-set enums kept exhaustive?
- [ ] Are error enums scoped to specific operations?

### Errors

- [ ] Do distinct operations have distinct error types?
- [ ] Do errors include enough context for debugging?
- [ ] Are error conditions documented with `# Errors`?
- [ ] Are errors using `thiserror` for derives?

### Documentation

- [ ] Is `#![warn(missing_docs)]` enabled?
- [ ] Does every public item have documentation?
- [ ] Do docs include examples?
- [ ] Are examples tested via doctests?
- [ ] Do docs use intra-doc links to related items?
- [ ] Is there crate-level documentation with quick-start?

### Re-exports and Ergonomics

- [ ] Are public dependency types re-exported?
- [ ] Is there a prelude module for common imports?
- [ ] Is there a `Result` type alias?
- [ ] Are feature flags documented?

### Versioning

- [ ] Is semantic versioning followed strictly?
- [ ] Has `cargo semver-checks` been run?
- [ ] Are breaking changes documented?
- [ ] Is there a migration guide for major versions?

---

## Anti-Patterns Summary

| Anti-Pattern | Problem | Solution |
|--------------|---------|----------|
| Public fields | Can't change representation | Private fields + accessors |
| Exposing dep types | Coupling to dependencies | Newtype wrappers |
| One mega error type | Unclear what can fail | Scoped error types |
| Complex trait bounds | Hard to implement | Minimal bounds |
| Missing `#[non_exhaustive]` on errors | Can't add variants | Add annotation |
| `#[non_exhaustive]` on fixed enums | Forces catch-all patterns | Keep exhaustive |
| Missing re-exports | Users need your deps | Re-export public deps |
| Missing docs | Unusable API | `#![warn(missing_docs)]` |
| Breaking changes in minor | Violates semver | Use cargo-semver-checks |

---

## Example: Well-Designed Public API

```rust
//! # Fortress Session
//!
//! Rollback networking for multiplayer games.
//!
//! ## Quick Start
//!
//! ```no_run
//! use fortress::{Session, SessionConfig, Input};
//!
//! let config = SessionConfig::default();
//! let mut session = Session::create(config)?;
//! # Ok::<(), fortress::CreateError>(())
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

mod internal;
mod protocol;

pub mod prelude;

// Re-export public types
pub use self::session::{Session, SessionConfig};
pub use self::input::Input;
pub use self::error::{CreateError, AdvanceError};

// Re-export dependency types used in public API
pub use bytes::Bytes;

/// A type alias for results with the default error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A frame number in the game simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Frame(u32);

impl Frame {
    /// Frame zero - the start of any session.
    pub const ZERO: Self = Self(0);

    /// Creates a new frame number.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw frame number.
    pub const fn get(self) -> u32 {
        self.0
    }
}

mod session {
    use super::*;

    /// Configuration for creating a session.
    #[derive(Clone, Debug)]
    #[non_exhaustive]
    pub struct SessionConfig {
        /// Maximum number of players (1-16).
        pub max_players: usize,
        /// Frames of prediction before waiting.
        pub prediction_window: u32,
    }

    impl Default for SessionConfig {
        fn default() -> Self {
            Self {
                max_players: 2,
                prediction_window: 8,
            }
        }
    }

    /// A multiplayer game session.
    pub struct Session {
        config: SessionConfig,
        frame: Frame,
    }

    impl Session {
        /// Creates a new session with the given configuration.
        ///
        /// # Errors
        ///
        /// Returns [`CreateError::InvalidConfig`] if the configuration
        /// is invalid.
        pub fn create(config: SessionConfig) -> Result<Self, CreateError> {
            if config.max_players == 0 || config.max_players > 16 {
                return Err(CreateError::InvalidConfig {
                    reason: "max_players must be 1-16".into(),
                });
            }
            Ok(Self {
                config,
                frame: Frame::ZERO,
            })
        }

        /// Returns the current frame number.
        pub fn current_frame(&self) -> Frame {
            self.frame
        }
    }
}

mod error {
    use super::*;
    use thiserror::Error;

    /// Errors that can occur when creating a session.
    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum CreateError {
        /// The configuration is invalid.
        #[error("invalid configuration: {reason}")]
        InvalidConfig {
            /// Explanation of what's wrong.
            reason: String,
        },
    }

    /// Errors that can occur when advancing a session.
    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum AdvanceError {
        /// The session is not connected.
        #[error("session not connected")]
        NotConnected,

        /// A desync was detected.
        #[error("desync detected at frame {frame}")]
        Desync {
            /// The frame where desync occurred.
            frame: u32,
        },
    }
}

mod input {
    /// Player input for a single frame.
    #[derive(Clone, Debug, Default)]
    pub struct Input {
        data: Vec<u8>,
    }

    impl Input {
        /// Creates a new input with the given data.
        pub fn new(data: Vec<u8>) -> Self {
            Self { data }
        }

        /// Returns the input data.
        pub fn data(&self) -> &[u8] {
            &self.data
        }
    }
}
```

---

## References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Semver Compatibility](https://doc.rust-lang.org/cargo/reference/semver.html)
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)
- [The Rust RFC Book - API Evolution](https://rust-lang.github.io/rfcs/1105-api-evolution.html)
