# WebAssembly Threading and Concurrency Guide

> **A guide to multi-threading in WebAssembly, including browser constraints, Rust patterns, and cross-platform compatibility.**

## Overview

Threading in WebAssembly is **possible but complex**. It requires:
- Browser security headers (COOP/COEP)
- Shared memory (`SharedArrayBuffer`)
- Nightly Rust (for full support)
- Significant JavaScript glue code

This guide covers how to write Rust code that works both with and without WASM threading.

---

## Current State of WASM Threading

### Requirements

| Component | Purpose |
|-----------|---------|
| `SharedArrayBuffer` | Shared memory between threads |
| Web Workers | Parallel execution contexts |
| Atomics | Synchronization primitives |
| COOP/COEP headers | Security requirement |

### Browser Support

| Browser | Threading Support |
|---------|-------------------|
| Chrome 68+ | ✅ (with headers) |
| Firefox 79+ | ✅ (with headers) |
| Safari 15.2+ | ✅ (with headers) |
| Edge (Chromium) | ✅ (with headers) |

### Required HTTP Headers

Your server **must** send these headers for `SharedArrayBuffer` to work:

```http
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Opener-Policy: same-origin
```

Without these headers, `SharedArrayBuffer` is disabled (Spectre mitigation).

---

## Conditional Compilation Patterns

### Basic Target Detection

```rust
#[cfg(target_arch = "wasm32")]
mod wasm_impl {
    // WASM-specific code (no std::thread)
}

#[cfg(not(target_arch = "wasm32"))]
mod native_impl {
    // Native code using std::thread
}
```

### Feature-Based Threading

```toml
# Cargo.toml
[features]
default = []
parallel = ["rayon"]

[dependencies]
rayon = { version = "1.10", optional = true }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-rayon = { version = "1.2", optional = true }
```

```rust
pub fn process_parallel<T, F, R>(items: &[T], f: F) -> Vec<R>
where
    T: Sync,
    F: Fn(&T) -> R + Send + Sync,
    R: Send,
{
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        items.par_iter().map(f).collect()
    }
    
    #[cfg(not(feature = "parallel"))]
    {
        items.iter().map(f).collect()
    }
}
```

---

## Making Code Work With AND Without Threading

### Pattern 1: Sequential Fallback

```rust
/// Process items, using parallelism when available
pub fn compute_heavy<T, F, R>(items: &[T], f: F) -> Vec<R>
where
    T: Sync,
    F: Fn(&T) -> R + Send + Sync,
    R: Send,
{
    #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
    {
        use rayon::prelude::*;
        items.par_iter().map(f).collect()
    }
    
    #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
    {
        items.iter().map(f).collect()
    }
}
```

### Pattern 2: Trait-Based Abstraction

```rust
pub trait ParallelIterator {
    type Item;
    fn process<F, R>(self, f: F) -> Vec<R>
    where
        F: Fn(Self::Item) -> R + Send + Sync,
        R: Send;
}

// Native parallel implementation
#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
impl<I> ParallelIterator for I
where
    I: rayon::iter::IntoParallelIterator,
{
    type Item = I::Item;
    
    fn process<F, R>(self, f: F) -> Vec<R>
    where
        F: Fn(Self::Item) -> R + Send + Sync,
        R: Send,
    {
        use rayon::prelude::*;
        self.into_par_iter().map(f).collect()
    }
}

// Sequential fallback
#[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
impl<I> ParallelIterator for I
where
    I: IntoIterator,
{
    type Item = I::Item;
    
    fn process<F, R>(self, f: F) -> Vec<R>
    where
        F: Fn(Self::Item) -> R,
        R: Send,
    {
        self.into_iter().map(f).collect()
    }
}
```

### Pattern 3: Runtime Feature Detection

```javascript
// JavaScript: Check if threading is available
import { threads } from 'wasm-feature-detect';

async function loadModule() {
    const hasThreads = await threads();
    
    if (hasThreads) {
        const { default: init, initThreadPool } = await import('./pkg-threaded/index.js');
        await init();
        await initThreadPool(navigator.hardwareConcurrency);
        return import('./pkg-threaded/index.js');
    } else {
        const { default: init } = await import('./pkg-single/index.js');
        await init();
        return import('./pkg-single/index.js');
    }
}
```

---

## Using Rayon in WASM

### With `wasm-bindgen-rayon`

```toml
# Cargo.toml
[dependencies]
rayon = "1.10"
wasm-bindgen = "0.2"
wasm-bindgen-rayon = "1.2"

[lib]
crate-type = ["cdylib"]
```

```rust
use wasm_bindgen::prelude::*;
use rayon::prelude::*;

// Re-export the thread pool initializer
pub use wasm_bindgen_rayon::init_thread_pool;

#[wasm_bindgen]
pub fn parallel_sum(numbers: &[i32]) -> i64 {
    numbers.par_iter().map(|&x| x as i64).sum()
}
```

```javascript
// JavaScript usage
import init, { initThreadPool, parallel_sum } from './pkg/index.js';

async function main() {
    await init();
    await initThreadPool(navigator.hardwareConcurrency);
    
    const numbers = new Int32Array([1, 2, 3, 4, 5]);
    console.log(parallel_sum(numbers)); // 15
}
```

### Building with Threading

```bash
# Requires nightly Rust
RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' \
    cargo +nightly build \
    --target wasm32-unknown-unknown \
    -Z build-std=std,panic_abort \
    --release
```

---

## Synchronization Primitives

### What Works in WASM

```rust
use std::sync::atomic::{AtomicU32, Ordering};

// Atomics work with +atomics feature
static COUNTER: AtomicU32 = AtomicU32::new(0);

pub fn increment() -> u32 {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}
```

### `std::sync` with Atomics

```rust
// These work when compiled with +atomics
use std::sync::{Arc, Mutex, RwLock};

let shared_data = Arc::new(Mutex::new(Vec::new()));
```

### Memory Model

WASM atomics follow the same memory model as C++11/Rust:

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

static VALUE: AtomicUsize = AtomicUsize::new(0);

// All orderings work the same as native
pub fn store_release(val: usize) {
    VALUE.store(val, Ordering::Release);
}

pub fn load_acquire() -> usize {
    VALUE.load(Ordering::Acquire)
}

pub fn compare_exchange(expected: usize, new: usize) -> Result<usize, usize> {
    VALUE.compare_exchange(expected, new, Ordering::SeqCst, Ordering::Relaxed)
}
```

---

## Key Differences from Native

| Aspect | Native | WASM |
|--------|--------|------|
| Thread spawning | `std::thread::spawn` | Web Workers (expensive, async) |
| Blocking | Allowed anywhere | **Cannot block main thread** |
| Thread pool | Optional | **Required** |
| Memory sharing | Automatic | Requires `SharedArrayBuffer` |
| Thread local storage | `thread_local!` | Limited support |

### Main Thread Blocking

**⚠️ You cannot block the main thread in WASM!**

```rust
// ❌ BAD: Will deadlock in browser
fn main() {
    let handle = thread::spawn(|| { /* work */ });
    handle.join().unwrap(); // Blocks main thread!
}

// ✅ GOOD: Use async or run in Worker
async fn process() {
    let result = spawn_and_await(|| { /* work */ }).await;
}
```

---

## Alternative Threading Approaches

### `wasm_thread` Crate

```rust
use wasm_thread as thread;

thread::spawn(|| {
    println!("Hello from a WASM thread!");
});
```

### Manual Web Worker Pool

```rust
use wasm_bindgen::prelude::*;
use web_sys::Worker;

#[wasm_bindgen]
pub struct ThreadPool {
    workers: Vec<Worker>,
}

#[wasm_bindgen]
impl ThreadPool {
    #[wasm_bindgen(constructor)]
    pub fn new(size: usize) -> Result<ThreadPool, JsValue> {
        let workers = (0..size)
            .map(|_| Worker::new("./worker.js"))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ThreadPool { workers })
    }
    
    pub fn execute(&self, task: JsValue) {
        // Round-robin task distribution
        let worker = &self.workers[0];
        worker.post_message(&task).unwrap();
    }
}
```

---

## Threading and Determinism

**⚠️ Important for rollback netcode:**

Threading introduces non-determinism due to:
- Scheduling order variations
- Race conditions
- Memory ordering differences

### Safe Patterns for Deterministic Code

```rust
// ✅ SAFE: Single-threaded simulation, parallel I/O
pub struct GameSession {
    // Simulation runs single-threaded (deterministic)
    simulation: SingleThreadedSimulation,
    
    // I/O can be parallel (doesn't affect game state)
    network_pool: ThreadPool,
}

impl GameSession {
    pub fn tick(&mut self, inputs: &[Input]) {
        // Deterministic simulation
        self.simulation.advance(inputs);
        
        // Non-deterministic I/O (doesn't affect simulation)
        self.network_pool.send_state(self.simulation.state());
    }
}
```

### Avoid in Game Logic

```rust
// ❌ AVOID: Non-deterministic parallel computation
fn update_entities_bad(entities: &mut [Entity]) {
    entities.par_iter_mut().for_each(|e| {
        e.update(); // Order-dependent results!
    });
}

// ✅ SAFE: Collect results, apply deterministically
fn update_entities_good(entities: &mut [Entity]) {
    let updates: Vec<_> = entities
        .par_iter()
        .map(|e| e.compute_update())
        .collect();
    
    // Apply in deterministic order
    for (entity, update) in entities.iter_mut().zip(updates) {
        entity.apply(update);
    }
}
```

---

## Build Configurations

### Without Threading (Default)

```bash
# Standard WASM build (no threading)
wasm-pack build --target web --release
```

### With Threading (Nightly)

```bash
# Threading requires nightly and special flags
RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' \
    wasm-pack build \
    --target web \
    --release \
    -- \
    -Z build-std=std,panic_abort
```

### Build Script for Both

```bash
#!/bin/bash
# build-wasm.sh

# Build without threading
wasm-pack build --target web --out-dir pkg-single --release

# Build with threading (requires nightly)
RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' \
    wasm-pack build \
    --target web \
    --out-dir pkg-threaded \
    --release \
    -- \
    -Z build-std=std,panic_abort
```

---

## Common Pitfalls

### 1. Missing COOP/COEP Headers

```javascript
// Check if SharedArrayBuffer is available
if (typeof SharedArrayBuffer === 'undefined') {
    console.warn('Threading not available - check COOP/COEP headers');
}
```

### 2. Thread Pool Not Initialized

```javascript
// ❌ Forgot to initialize
parallel_computation(data);

// ✅ Initialize first
await initThreadPool(navigator.hardwareConcurrency);
parallel_computation(data);
```

### 3. Blocking Main Thread

```rust
// ❌ Deadlock
std::thread::spawn(|| {}).join();

// ✅ Use async
wasm_bindgen_futures::spawn_local(async { /* work */ });
```

### 4. Using std::thread Directly

```rust
// ❌ Won't compile for wasm32-unknown-unknown
use std::thread;
thread::spawn(|| {});

// ✅ Use conditional compilation
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;
```

---

## Recommendations for This Project

For a rollback netcode library like Fortress/GGRS:

### 1. Keep Simulation Single-Threaded

```rust
// Game simulation must be deterministic
pub fn advance_frame(state: &mut GameState, inputs: &[Input]) {
    // Single-threaded, deterministic
    for input in inputs {
        state.apply_input(input);
    }
    state.physics_tick();
}
```

### 2. Parallelize Non-Critical Paths (Optional)

```rust
// Checksum computation can be parallel (result is same either way)
pub fn compute_checksum(state: &GameState) -> u64 {
    #[cfg(feature = "parallel")]
    {
        state.entities.par_iter().map(|e| e.hash()).sum()
    }
    
    #[cfg(not(feature = "parallel"))]
    {
        state.entities.iter().map(|e| e.hash()).sum()
    }
}
```

### 3. Default to No Threading

```toml
[features]
default = []  # No parallel by default
parallel = ["rayon"]  # Opt-in
```

### 4. Document Threading Implications

```rust
/// Processes inputs for all players.
/// 
/// # Determinism
/// This function is deterministic regardless of threading.
/// Parallel execution only affects performance, not results.
pub fn process_inputs(inputs: &[Input]) -> Vec<Effect> {
    // Implementation
}
```

---

## Quick Reference

### WASM Threading Summary

| Feature | Status | Requirement |
|---------|--------|-------------|
| Basic atomics | Stable | `+atomics` target feature |
| SharedArrayBuffer | Needs headers | COOP/COEP HTTP headers |
| Rayon | Works | `wasm-bindgen-rayon` crate |
| std::thread | Doesn't work | Use `wasm_thread` crate |
| Web Workers | Works | JavaScript glue code |

### Build Command Cheat Sheet

```bash
# Standard build (no threads)
wasm-pack build --target web --release

# With threads (nightly)
RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' \
    cargo +nightly build --target wasm32-unknown-unknown -Z build-std=std,panic_abort

# Check if threading is available (JS)
const hasThreads = typeof SharedArrayBuffer !== 'undefined';
```

---

*Threading in WASM is possible but optional. For deterministic applications like rollback netcode, single-threaded simulation with optional parallel I/O is recommended.*
