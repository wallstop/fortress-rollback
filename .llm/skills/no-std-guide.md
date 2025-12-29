# no_std Rust Development Guide

A comprehensive guide to writing Rust code that works across std, no_std+alloc, and bare-metal no_std environments—essential for WebAssembly, embedded systems, and maximum portability.

## Why `#![no_std]` Matters

### WebAssembly Compatibility

WebAssembly doesn't have access to OS-level functionality. While `wasm32-unknown-unknown` technically supports std, many std features panic at runtime:

```rust
// These compile but PANIC in WASM:
std::time::Instant::now()      // No system clock
std::net::TcpStream::connect() // No networking
std::fs::read_to_string()      // No filesystem
std::thread::spawn()           // No threads (without wasm-threads)
```

A `no_std` library forces you to design around these limitations upfront.

### Embedded Systems

Microcontrollers have no operating system—no heap allocator, no filesystem, no threads. `no_std` code runs on everything from Arduino to space probes.

### Binary Size

std pulls in significant code. For WASM deployments where every kilobyte matters, `no_std` can dramatically reduce bundle size.

### Determinism

For rollback networking and replays, `no_std` helps enforce determinism by preventing accidental use of non-deterministic std features (system time, random, threads).

---

## The std vs core vs alloc Split

Rust's standard library is layered:

```
┌─────────────────────────────────────────────────────────┐
│                        std                               │
│  (OS features: fs, net, thread, time, env, process)     │
├─────────────────────────────────────────────────────────┤
│                       alloc                              │
│  (Heap allocation: Vec, String, Box, Rc, Arc, BTreeMap) │
├─────────────────────────────────────────────────────────┤
│                        core                              │
│  (No dependencies: primitives, Option, Result, Iterator)│
└─────────────────────────────────────────────────────────┘
```

### `core` - Always Available

```rust
// Everything in core works everywhere
use core::mem;
use core::ptr;
use core::slice;
use core::str;
use core::fmt;
use core::ops::{Add, Deref};
use core::cmp::{Ord, PartialOrd};
use core::iter::Iterator;
use core::option::Option;
use core::result::Result;
use core::marker::{Copy, Send, Sync, PhantomData};
use core::cell::{Cell, RefCell};
use core::num::NonZeroU32;
use core::convert::{From, Into, TryFrom, TryInto};
```

### `alloc` - Requires a Global Allocator

```rust
extern crate alloc;

use alloc::vec::Vec;
use alloc::vec;              // vec![] macro
use alloc::string::String;
use alloc::string::ToString; // .to_string() trait
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::sync::Arc;
use alloc::collections::{BTreeMap, BTreeSet, VecDeque, BinaryHeap};
use alloc::format;           // format!() macro
use alloc::borrow::Cow;
```

**Note:** `HashMap`/`HashSet` are NOT in alloc—they require randomness for DOS protection. Use `BTreeMap`/`BTreeSet` or `hashbrown` crate.

### `std` - Requires an Operating System

```rust
use std::fs;
use std::net;
use std::thread;
use std::time::{Instant, SystemTime};
use std::sync::{Mutex, RwLock, mpsc};
use std::collections::{HashMap, HashSet}; // Includes RandomState
use std::io::{Read, Write, BufReader};
use std::env;
use std::process;
use std::path::PathBuf;
```

---

## Writing Code That Works With and Without std

### Basic Structure

```rust
// lib.rs
#![cfg_attr(not(feature = "std"), no_std)]

// Only needed if using alloc without std
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Re-export from the right place
#[cfg(feature = "std")]
use std::vec::Vec;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::vec::Vec;

// For truly no-heap code, don't use Vec at all
```

### The Prelude Pattern

Create a private prelude module that re-exports types from the correct location:

```rust
// src/lib.rs
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod prelude;
mod session;
mod input_queue;

// src/prelude.rs
//! Internal prelude for std/no_std compatibility

// Core types - always available
pub use core::cmp::{max, min, Ordering};
pub use core::fmt;
pub use core::mem;
pub use core::ops::Range;

// Alloc types - when heap is available
#[cfg(feature = "alloc")]
pub use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec,
    vec::Vec,
    collections::VecDeque,
};

// std types - only with full std
#[cfg(feature = "std")]
pub use std::collections::{HashMap, HashSet};

// Replacement for HashMap when no std
#[cfg(all(feature = "alloc", not(feature = "std")))]
pub use hashbrown::{HashMap, HashSet};
```

Then use it everywhere:

```rust
// src/session.rs
use crate::prelude::*;

pub struct Session {
    players: Vec<Player>,
    state_checksums: HashMap<Frame, u64>,
}
```

---

## Feature Flag Patterns

### Cargo.toml Setup

```toml
[package]
name = "my-netcode"
version = "0.1.0"

[features]
default = ["std"]

# Full standard library support
std = ["alloc", "dep:parking_lot"]

# Heap allocation without OS
alloc = []

# Optional integrations that require std
tokio = ["std", "dep:tokio"]
async-std = ["std", "dep:async-std"]

[dependencies]
# Always available (no_std compatible)
bytemuck = { version = "1", default-features = false }
bitflags = { version = "2", default-features = false }

# Only with alloc
hashbrown = { version = "0.14", optional = true, default-features = false, features = ["alloc"] }

# Only with std
parking_lot = { version = "0.12", optional = true }
tokio = { version = "1", optional = true, features = ["net", "time"] }

[dev-dependencies]
# Tests can use std even if library is no_std
proptest = "1"
```

### Default-Features = False

**Critical:** Always disable default features for dependencies in no_std libraries:

```toml
# WRONG - pulls in std transitively
serde = "1"

# CORRECT - explicitly opt into features
serde = { version = "1", default-features = false, features = ["derive", "alloc"] }
```

### Feature Hygiene

```rust
// Ensure features are additive and don't conflict
#[cfg(all(feature = "std", not(feature = "alloc")))]
compile_error!("The `std` feature requires `alloc`");

// Ensure at least some allocation strategy when using complex types
#[cfg(not(any(feature = "std", feature = "alloc")))]
compile_error!("This crate requires either `std` or `alloc` feature");
```

---

## Common std Replacements

### Collections

| std | no_std + alloc | no_std (no heap) |
|-----|----------------|------------------|
| `Vec<T>` | `alloc::vec::Vec<T>` | `heapless::Vec<T, N>` or `[T; N]` |
| `String` | `alloc::string::String` | `heapless::String<N>` or `&str` |
| `HashMap` | `hashbrown::HashMap` | `heapless::FnvIndexMap` |
| `HashSet` | `hashbrown::HashSet` | `heapless::FnvIndexSet` |
| `Box<T>` | `alloc::boxed::Box<T>` | Stack allocation or arena |
| `VecDeque` | `alloc::collections::VecDeque` | `heapless::Deque<T, N>` |

### Time

```rust
// std - NOT deterministic, NOT available in WASM
let now = std::time::Instant::now();
let elapsed = now.elapsed();

// no_std pattern: Accept time from outside
pub struct Session {
    frame: u32,
    frame_duration_micros: u64,
}

impl Session {
    /// Advance simulation by one frame
    /// Caller is responsible for timing
    pub fn advance_frame(&mut self) {
        self.frame += 1;
    }

    /// Get simulation time (deterministic!)
    pub fn elapsed_micros(&self) -> u64 {
        u64::from(self.frame) * self.frame_duration_micros
    }
}
```

### Random Numbers

```rust
// std - non-deterministic
use std::collections::HashMap; // Uses RandomState internally

// no_std deterministic pattern: Seeded PRNG
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_core::{RngCore, SeedableRng};

pub struct GameState {
    rng: Xoshiro256PlusPlus,
}

impl GameState {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Xoshiro256PlusPlus::seed_from_u64(seed),
        }
    }

    pub fn random_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }
}

// For HashMap without randomness (deterministic iteration order):
use hashbrown::HashMap;
// Or use BTreeMap which is ordered
use alloc::collections::BTreeMap;
```

### Synchronization Primitives

```rust
// std
use std::sync::{Mutex, RwLock, Arc};

// no_std + alloc (single-threaded or spin-locks)
use spin::Mutex;              // Spin-lock based
use spin::RwLock;
use alloc::sync::Arc;         // Arc is in alloc!

// For no_std multi-threading (rare), use:
// - critical-section crate for interrupt-safe locks
// - portable-atomic for atomic operations
```

### I/O Traits

```rust
// std
use std::io::{Read, Write, Error as IoError};

// no_std - define your own or use embedded-io
pub trait Read {
    type Error;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

pub trait Write {
    type Error;
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
}

// Or use the embedded-io crate which provides these
```

---

## alloc Crate Deep Dive

### Enabling alloc

```rust
// At crate root
#![no_std]

extern crate alloc;

// Now you can use:
use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;
```

### Global Allocator

In no_std environments, you must provide a global allocator:

```rust
// For WASM (usually automatic with wasm-bindgen)
// For embedded:
use embedded_alloc::Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

fn main() {
    // Initialize heap with memory region
    {
        const HEAP_SIZE: usize = 1024 * 64; // 64KB
        static mut HEAP_MEM: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_mut_ptr(), HEAP_SIZE) }
    }

    // Now Vec, Box, etc. work
    let v = vec![1, 2, 3];
}
```

### Fallible Allocation

```rust
// Standard allocation panics on OOM
let v: Vec<u8> = vec![0; 1_000_000_000]; // May panic!

// Fallible allocation (Rust 1.57+)
let v: Vec<u8> = Vec::new();
v.try_reserve(1_000_000_000)?; // Returns Result

// Or use try_* methods
let b = Box::try_new(expensive_value)?;
```

---

## Error Handling Without std

### The Problem

`std::error::Error` is NOT in core! This is the biggest pain point for no_std libraries.

```rust
// This doesn't work in no_std:
impl std::error::Error for MyError {}
```

### Solution 1: Conditional std::error::Error

```rust
#[derive(Debug)]
pub enum Error {
    InvalidInput,
    BufferTooSmall { needed: usize, available: usize },
    Disconnected,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput => write!(f, "invalid input"),
            Self::BufferTooSmall { needed, available } => {
                write!(f, "buffer too small: needed {needed}, got {available}")
            }
            Self::Disconnected => write!(f, "disconnected"),
        }
    }
}

// Only implement std::error::Error when std is available
#[cfg(feature = "std")]
impl std::error::Error for Error {}
```

### Solution 2: Use `core::error::Error` (Rust 1.81+)

As of Rust 1.81, `core::error::Error` exists!

```rust
#[derive(Debug)]
pub struct MyError;

impl core::fmt::Display for MyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "my error")
    }
}

// Works in no_std on Rust 1.81+
impl core::error::Error for MyError {}
```

### Solution 3: Error Source Chain

```rust
#[derive(Debug)]
pub enum Error {
    Io(IoErrorKind),
    Parse(ParseError),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(kind) => write!(f, "I/O error: {kind:?}"),
            Self::Parse(e) => write!(f, "parse error: {e}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            _ => None,
        }
    }
}
```

---

## Testing no_std Code

### Run Tests with std

Even for no_std libraries, tests typically run with std:

```rust
// src/lib.rs
#![cfg_attr(not(feature = "std"), no_std)]

// Tests always have std available
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Can use std here!
        let start = std::time::Instant::now();
        // ...
        println!("Elapsed: {:?}", start.elapsed());
    }
}
```

### Verify no_std Compilation

```bash
# Check that library compiles without std
cargo check --no-default-features --features alloc

# Check bare no_std (no heap)
cargo check --no-default-features

# Cross-compile to WASM to verify
cargo check --target wasm32-unknown-unknown --no-default-features --features alloc
```

### CI Configuration

```yaml
# .github/workflows/ci.yml
jobs:
  no-std:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: thumbv7em-none-eabihf, wasm32-unknown-unknown

      - name: Check no_std + alloc
        run: cargo check --no-default-features --features alloc

      - name: Check bare no_std
        run: cargo check --no-default-features

      - name: Check WASM
        run: cargo check --target wasm32-unknown-unknown --no-default-features --features alloc

      - name: Check embedded target
        run: cargo check --target thumbv7em-none-eabihf --no-default-features
```

### Testing Allocator Behavior

```rust
#[test]
fn test_no_unexpected_allocations() {
    // Use allocation tracking in tests
    let stats_before = allocation_counter::count();

    // Your code that shouldn't allocate
    let result = process_fixed_buffer(&mut buffer);

    let stats_after = allocation_counter::count();
    assert_eq!(stats_before, stats_after, "unexpected allocation");
}
```

---

## Popular no_std Compatible Crates

### Serialization

```toml
# Serde - the standard
serde = { version = "1", default-features = false, features = ["derive", "alloc"] }

# Binary formats
postcard = "1"          # Optimized for embedded
bincode = { version = "2.0.0-rc", default-features = false, features = ["alloc"] }
rkyv = { version = "0.8", default-features = false }  # Zero-copy

# No-alloc serialization
ssmarshal = "1"         # Fixed-size only
```

### Data Structures

```toml
heapless = "0.8"        # Fixed-capacity collections
arrayvec = "0.7"        # Array-backed Vec
smallvec = { version = "1", default-features = false }
hashbrown = { version = "0.14", default-features = false, features = ["alloc"] }
indexmap = { version = "2", default-features = false, features = ["alloc"] }
```

### Utilities

```toml
bitflags = { version = "2", default-features = false }
bytemuck = { version = "1", default-features = false }
byteorder = { version = "1", default-features = false }
num-traits = { version = "0.2", default-features = false }
either = { version = "1", default-features = false }
static_assertions = "1"
```

### Hashing & Crypto

```toml
# Non-cryptographic (fast)
ahash = { version = "0.8", default-features = false }
rustc-hash = "1"
fnv = { version = "1", default-features = false }

# Cryptographic
sha2 = { version = "0.10", default-features = false }
blake3 = { version = "1", default-features = false }
```

### Random Numbers

```toml
rand_core = { version = "0.6", default-features = false }
rand_xoshiro = "0.6"    # Fast, seedable PRNG
rand_chacha = { version = "0.3", default-features = false }
```

### Async (with alloc)

```toml
futures-core = { version = "0.3", default-features = false, features = ["alloc"] }
futures-util = { version = "0.3", default-features = false, features = ["alloc"] }
```

### Embedded-Specific

```toml
embedded-hal = "1"      # Hardware abstraction
critical-section = "1"  # Interrupt-safe primitives
portable-atomic = "1"   # Atomic ops on all platforms
defmt = "0.3"           # Efficient logging
```

---

## Conditional Compilation Patterns

### Basic Feature Checks

```rust
// Include module only with feature
#[cfg(feature = "std")]
mod std_networking;

#[cfg(feature = "std")]
pub use std_networking::TcpSocket;

// Conditional implementation
impl Session {
    #[cfg(feature = "std")]
    pub fn connect(addr: &str) -> std::io::Result<Self> {
        // Uses std::net
    }

    // Always available version
    pub fn from_socket<S: Socket>(socket: S) -> Self {
        // Works with any socket implementation
    }
}
```

### Platform-Specific Code

```rust
// WASM-specific
#[cfg(target_arch = "wasm32")]
fn get_timestamp() -> u64 {
    // Use js_sys or web_sys
}

#[cfg(not(target_arch = "wasm32"))]
fn get_timestamp() -> u64 {
    // Use std::time
}
```

### Combining Conditions

```rust
// std on non-WASM platforms
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
fn spawn_thread() { /* ... */ }

// Alloc but not std
#[cfg(all(feature = "alloc", not(feature = "std")))]
use hashbrown::HashMap;

// Either std or alloc
#[cfg(any(feature = "std", feature = "alloc"))]
use vec::Vec;

// Complex condition
#[cfg(all(
    feature = "alloc",
    any(feature = "std", target_arch = "wasm32"),
    not(feature = "no-threads")
))]
fn complex_feature() { /* ... */ }
```

### Compile-Time Assertions

```rust
// Ensure configuration is valid
#[cfg(all(feature = "tokio", not(feature = "std")))]
compile_error!("tokio feature requires std");

// Ensure target compatibility
#[cfg(all(target_arch = "wasm32", feature = "threads"))]
compile_error!("threads not supported on wasm32 without atomics");
```

---

## Complete Example: Game Networking Library

Here's a realistic structure for a no_std-compatible networking library:

```rust
// src/lib.rs
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

//! A rollback networking library for games.
//!
//! # Features
//!
//! - `std` (default): Full standard library support
//! - `alloc`: Heap allocation without std (for WASM/embedded)
//! - `tokio`: Async networking with Tokio runtime

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Feature validation
#[cfg(all(feature = "tokio", not(feature = "std")))]
compile_error!("The `tokio` feature requires `std`");

mod prelude;
mod error;
mod frame;
mod input;
mod session;

pub use error::{Error, Result};
pub use frame::Frame;
pub use input::{Input, InputBuffer};
pub use session::{Session, SessionConfig};

// Optional std-only exports
#[cfg(feature = "std")]
mod network;
#[cfg(feature = "std")]
pub use network::{UdpSocket, TcpSocket};
```

```rust
// src/prelude.rs
pub use core::cmp::{max, min, Ordering};
pub use core::convert::{TryFrom, TryInto};
pub use core::fmt::{self, Debug, Display};
pub use core::mem;
pub use core::num::NonZeroU32;
pub use core::ops::Range;
pub use core::result::Result as CoreResult;

#[cfg(feature = "alloc")]
pub use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec,
    vec::Vec,
    collections::VecDeque,
};

#[cfg(feature = "std")]
pub use std::collections::{HashMap, HashSet};

#[cfg(all(feature = "alloc", not(feature = "std")))]
pub use hashbrown::{HashMap, HashSet};
```

```rust
// src/error.rs
use crate::prelude::*;

/// Result type for this crate
pub type Result<T> = CoreResult<T, Error>;

/// Errors that can occur during session operation
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Input was received for an invalid player
    InvalidPlayer { player: usize, max: usize },
    /// Frame is too far in the past to process
    FrameTooOld { frame: u32, oldest: u32 },
    /// Input buffer is full
    InputBufferFull,
    /// Session has been disconnected
    Disconnected,
    /// Desync detected between players
    DesyncDetected { frame: u32 },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPlayer { player, max } => {
                write!(f, "invalid player {player}, max is {max}")
            }
            Self::FrameTooOld { frame, oldest } => {
                write!(f, "frame {frame} too old, oldest is {oldest}")
            }
            Self::InputBufferFull => write!(f, "input buffer full"),
            Self::Disconnected => write!(f, "session disconnected"),
            Self::DesyncDetected { frame } => {
                write!(f, "desync detected at frame {frame}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
```

```rust
// src/input.rs
use crate::prelude::*;
use crate::frame::Frame;

/// A game input that can be serialized
pub trait Input: Clone + Default + PartialEq {
    /// Serialize input to bytes
    fn to_bytes(&self, buf: &mut [u8]) -> usize;

    /// Deserialize input from bytes
    fn from_bytes(buf: &[u8]) -> Option<Self>;
}

/// Fixed-size input buffer for rollback
#[cfg(feature = "alloc")]
pub struct InputBuffer<I: Input> {
    inputs: VecDeque<(Frame, I)>,
    capacity: usize,
}

#[cfg(feature = "alloc")]
impl<I: Input> InputBuffer<I> {
    /// Create a new input buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            inputs: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Add input for a frame
    pub fn add(&mut self, frame: Frame, input: I) -> crate::Result<()> {
        if self.inputs.len() >= self.capacity {
            return Err(crate::Error::InputBufferFull);
        }
        self.inputs.push_back((frame, input));
        Ok(())
    }

    /// Get input for a frame
    pub fn get(&self, frame: Frame) -> Option<&I> {
        self.inputs
            .iter()
            .find(|(f, _)| *f == frame)
            .map(|(_, i)| i)
    }
}
```

```rust
// src/session.rs
use crate::prelude::*;
use crate::error::Result;
use crate::frame::Frame;
use crate::input::Input;

/// Configuration for a session
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Number of players
    pub num_players: usize,
    /// Maximum prediction frames
    pub max_prediction: u32,
    /// Input delay in frames
    pub input_delay: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            num_players: 2,
            max_prediction: 8,
            input_delay: 2,
        }
    }
}

/// A rollback networking session
#[cfg(feature = "alloc")]
pub struct Session<I: Input> {
    config: SessionConfig,
    current_frame: Frame,
    confirmed_frame: Frame,
    #[cfg(feature = "alloc")]
    local_inputs: Vec<I>,
    #[cfg(feature = "alloc")]
    pending_checksums: HashMap<Frame, u64>,
}

#[cfg(feature = "alloc")]
impl<I: Input> Session<I> {
    /// Create a new session
    pub fn new(config: SessionConfig) -> Result<Self> {
        Ok(Self {
            local_inputs: vec![I::default(); config.num_players],
            pending_checksums: HashMap::new(),
            current_frame: Frame::default(),
            confirmed_frame: Frame::default(),
            config,
        })
    }

    /// Advance to the next frame
    pub fn advance_frame(&mut self, checksum: Option<u64>) -> Result<Frame> {
        if let Some(cs) = checksum {
            self.pending_checksums.insert(self.current_frame, cs);
        }
        self.current_frame = self.current_frame.saturating_add(1);
        Ok(self.current_frame)
    }

    /// Get current frame number
    pub fn current_frame(&self) -> Frame {
        self.current_frame
    }
}
```

### Cargo.toml

```toml
[package]
name = "fortress-netcode"
version = "0.1.0"
edition = "2021"
rust-version = "1.81"
license = "MIT OR Apache-2.0"
description = "Rollback networking for games"
categories = ["game-development", "network-programming", "no-std"]
keywords = ["rollback", "netcode", "ggpo", "no_std"]

[features]
default = ["std"]
std = ["alloc", "hashbrown/default"]
alloc = ["hashbrown/alloc"]
tokio = ["std", "dep:tokio"]

[dependencies]
hashbrown = { version = "0.14", default-features = false, optional = true }
tokio = { version = "1", optional = true, features = ["net", "time", "sync"] }

[dev-dependencies]
proptest = "1"
criterion = "0.5"

[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.2", features = ["js"] }
```

---

## Checklist for no_std Support

- [ ] Add `#![cfg_attr(not(feature = "std"), no_std)]` to lib.rs
- [ ] Add `extern crate alloc;` when using alloc feature
- [ ] Create prelude module for consistent imports
- [ ] Replace `std::` imports with `core::` or `alloc::` equivalents
- [ ] Use `hashbrown` instead of `std::collections::HashMap`
- [ ] Make `std::error::Error` impl conditional
- [ ] Set `default-features = false` for all dependencies
- [ ] Add CI job to verify no_std compilation
- [ ] Test on wasm32-unknown-unknown target
- [ ] Document which features require std vs alloc vs nothing
- [ ] Avoid `std::time`, `std::thread`, `std::net` in core logic
- [ ] Use deterministic alternatives for random/time

---

## Further Reading

- [The Embedded Rust Book](https://docs.rust-embedded.org/book/)
- [Rust WASM Book](https://rustwasm.github.io/docs/book/)
- [The `core` crate documentation](https://doc.rust-lang.org/core/)
- [The `alloc` crate documentation](https://doc.rust-lang.org/alloc/)
