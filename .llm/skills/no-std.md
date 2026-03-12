<!-- CATEGORY: Rust Language -->
<!-- WHEN: Writing no_std code, embedded targets, core vs alloc decisions -->
# no_std Guide

## The std / core / alloc Split

| Layer | Contains | Requires |
|-------|----------|----------|
| `core` | Primitives, Option, Result, Iterator, PhantomData | Nothing |
| `alloc` | Vec, String, Box, Rc, Arc, BTreeMap, format! | Global allocator |
| `std` | fs, net, thread, time, HashMap, Mutex, io | Operating system |

**Note:** `HashMap`/`HashSet` are NOT in alloc (need randomness). Use `BTreeMap` or `hashbrown`.

## Basic Structure

```rust
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;
```

## Prelude Pattern

```rust
// src/prelude.rs
pub use core::cmp::{max, min, Ordering};
pub use core::fmt;
pub use core::mem;

#[cfg(feature = "alloc")]
pub use alloc::{boxed::Box, string::{String, ToString}, vec, vec::Vec};

#[cfg(feature = "std")]
pub use std::collections::{HashMap, HashSet};

#[cfg(all(feature = "alloc", not(feature = "std")))]
pub use hashbrown::{HashMap, HashSet};
```

## Feature Flag Setup

```toml
[features]
default = ["std"]
std = ["alloc", "dep:parking_lot"]
alloc = []

[dependencies]
# Always: default-features = false
serde = { version = "1", default-features = false, features = ["derive", "alloc"] }
hashbrown = { version = "0.14", optional = true, default-features = false, features = ["alloc"] }
```

### Feature Hygiene

```rust
#[cfg(all(feature = "tokio", not(feature = "std")))]
compile_error!("tokio feature requires std");
```

## Common std Replacements

| std | no_std + alloc | no_std (no heap) |
|-----|----------------|------------------|
| `Vec<T>` | `alloc::vec::Vec<T>` | `heapless::Vec<T, N>` |
| `String` | `alloc::string::String` | `heapless::String<N>` |
| `HashMap` | `hashbrown::HashMap` | `heapless::FnvIndexMap` |
| `Box<T>` | `alloc::boxed::Box<T>` | Stack or arena |
| `Mutex` | `spin::Mutex` | `critical-section` |

### Time

Accept time from outside -- caller is responsible for timing. This enforces determinism.

### Random Numbers

Use seeded PRNG (`rand_xoshiro::Xoshiro256PlusPlus`). Use `BTreeMap` or `hashbrown` for deterministic iteration.

### Synchronization

```rust
// no_std: spin-locks or critical-section
use spin::Mutex;
use alloc::sync::Arc; // Arc is in alloc!
```

## Error Handling Without std

`std::error::Error` is NOT in core (before Rust 1.81).

```rust
impl core::fmt::Display for Error { /* ... */ }

#[cfg(feature = "std")]
impl std::error::Error for Error {}

// Rust 1.81+: core::error::Error exists
```

## Conditional Compilation

```rust
#[cfg(feature = "std")]
mod std_networking;

#[cfg(target_arch = "wasm32")]
fn get_timestamp() -> u64 { /* js_sys */ }

#[cfg(all(feature = "alloc", not(feature = "std")))]
use hashbrown::HashMap;
```

## Testing

```bash
# Verify no_std compilation
cargo check --no-default-features --features alloc
cargo check --no-default-features  # bare no_std
cargo check --target wasm32-unknown-unknown --no-default-features --features alloc
```

Tests always have std available (`#[cfg(test)]` runs with std).

## Popular no_std Crates

### Serialization
`serde` (default-features=false), `postcard`, `rkyv`

### Data Structures
`heapless`, `arrayvec`, `smallvec`, `hashbrown`, `indexmap`

### Utilities
`bitflags`, `bytemuck`, `byteorder`, `num-traits`, `static_assertions`

### Hashing
`ahash`, `rustc-hash`, `fnv`, `sha2`, `blake3`

### Random
`rand_core`, `rand_xoshiro`, `rand_chacha`

### Embedded
`embedded-hal`, `critical-section`, `portable-atomic`, `defmt`

## Checklist

- [ ] `#![cfg_attr(not(feature = "std"), no_std)]` in lib.rs
- [ ] `extern crate alloc;` when using alloc feature
- [ ] Prelude module for consistent imports
- [ ] `core::`/`alloc::` instead of `std::`
- [ ] `hashbrown` instead of `std::collections::HashMap`
- [ ] `std::error::Error` impl conditional on feature
- [ ] `default-features = false` for ALL dependencies
- [ ] CI job verifying no_std compilation
- [ ] Test on `wasm32-unknown-unknown`
- [ ] Avoid `std::time`, `std::thread`, `std::net` in core logic
- [ ] Document which features require std vs alloc
