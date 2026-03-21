<!-- CATEGORY: Platform -->
<!-- WHEN: WASM compilation, wasm-bindgen, web workers, WASM portability -->
# WebAssembly Development Guide

## Toolchain

| Tool | Purpose |
|------|---------|
| `wasm-pack` | Build, test, publish WASM packages |
| `wasm-bindgen` | Rust-JS bindings and type conversions |
| `js-sys` | Bindings to JS built-in objects |
| `web-sys` | Bindings to Web APIs (DOM, fetch, WebGL) |
| `console_error_panic_hook` | Better panic messages in browser |
| `wasm-bindgen-futures` | Async/await and Promise integration |
| `wasm-opt` | Post-build binary optimizer (Binaryen) |

## Cargo.toml Setup

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

## wasm-bindgen Patterns

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Counter { value: i32 }

#[wasm_bindgen]
impl Counter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Counter { Counter { value: 0 } }
    pub fn increment(&mut self) { self.value += 1; }
    pub fn value(&self) -> i32 { self.value }
}

// Returning Results to JS
#[wasm_bindgen]
pub fn divide(a: f64, b: f64) -> Result<f64, JsValue> {
    if b == 0.0 { Err(JsValue::from_str("Division by zero")) }
    else { Ok(a / b) }
}

// Importing JS functions
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
```

## Build Commands

```bash
wasm-pack build --target web --release      # Browser
wasm-pack build --target nodejs --release   # Node.js
wasm-pack build --target bundler --release  # Webpack etc.
wasm-opt -Oz input.wasm -o output.wasm      # Further optimization
```

## Binary Size Reduction

| Technique | Typical Savings |
|-----------|-----------------|
| `panic = "abort"` | 10-15% |
| `opt-level = "z"` | 20-30% |
| `lto = true` | 10-25% |
| `strip = true` | 30-50% |
| `wasm-opt -Oz` | 5-15% |
| `#![no_std]` | Varies (significant) |

Analyze with: `cargo install twiggy && twiggy top target/wasm32-unknown-unknown/release/game.wasm`

## Conditional Compilation

```rust
#[cfg(target_arch = "wasm32")]
mod wasm_impl { /* WASM-specific */ }

#[cfg(not(target_arch = "wasm32"))]
mod native_impl { /* Native */ }
```

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["console"] }
```

## WASM Limitations

| Limitation | Workaround |
|------------|------------|
| No threads | async/await, Web Workers |
| No filesystem | `include_bytes!`, IndexedDB |
| WebGL2 default | Target WebGL1 for compat or WebGPU for features |
| Audio restrictions | Start audio on first user interaction |
| No dynamic linking | Must statically link everything |

## Threading via Web Workers

### Requirements

| Component | Purpose |
|-----------|---------|
| `SharedArrayBuffer` | Shared memory between threads |
| Web Workers | Parallel execution contexts |
| Atomics | Synchronization primitives |
| COOP/COEP headers | Security requirement for `SharedArrayBuffer` |

Required HTTP headers:

```http
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Opener-Policy: same-origin
```

### Key Differences from Native

| Aspect | Native | WASM |
|--------|--------|------|
| Thread spawning | `std::thread::spawn` | Web Workers (expensive, async) |
| Blocking | Allowed anywhere | Cannot block main thread |
| Memory sharing | Automatic | Requires `SharedArrayBuffer` |

### Sequential Fallback Pattern

```rust
pub fn compute_heavy<T, F, R>(items: &[T], f: F) -> Vec<R>
where T: Sync, F: Fn(&T) -> R + Send + Sync, R: Send,
{
    #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
    { use rayon::prelude::*; items.par_iter().map(f).collect() }

    #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
    { items.iter().map(f).collect() }
}
```

### Rayon in WASM (wasm-bindgen-rayon)

```bash
# Requires nightly
RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' \
    cargo +nightly build --target wasm32-unknown-unknown \
    -Z build-std=std,panic_abort --release
```

```javascript
import init, { initThreadPool } from './pkg/index.js';
await init();
await initThreadPool(navigator.hardwareConcurrency);
```

### Runtime Feature Detection (JS)

```javascript
import { threads } from 'wasm-feature-detect';
const hasThreads = await threads();
if (hasThreads) { /* load threaded module */ }
else { /* load single-threaded module */ }
```

## Determinism Guarantees

### What WASM Guarantees

| Operation | Deterministic? |
|-----------|----------------|
| Integer arithmetic | Yes -- fully specified |
| Float arithmetic | Yes -- IEEE 754-2019 |
| Memory access | Yes -- same inputs same results |
| Control flow | Yes -- structured, no UB |

### Why This Matters for Rollback Netcode

Native float math varies across platforms (x87 vs SSE vs ARM FPU). WASM guarantees identical float results everywhere -- same binary, same results on all platforms.

### NaN Canonicalization

```rust
fn float_to_deterministic_bits(x: f32) -> u32 {
    if x.is_nan() { 0x7FC00000 } // Canonical quiet NaN
    else { x.to_bits() }
}
```

### Sources of Non-Determinism (Even in WASM)

| Source | Solution |
|--------|----------|
| Host-provided time (`Date.now()`) | Use frame counters |
| Threading schedule order | Single-threaded simulation |
| Random imports (system entropy) | Seeded PRNG in WASM |
| Network timing | Separate I/O from simulation |
| Hash with random seed | Use deterministic hash |

## Architecture for Deterministic Games

```rust
// Simulation: compile to WASM for determinism
mod simulation {
    pub fn advance_frame(state: &mut GameState, inputs: &[PlayerInput]) {
        for input in inputs { state.apply_input(input); }
        state.physics_tick();  // Float math is deterministic in WASM
    }
}
// Platform-specific: stays native
mod platform {
    pub fn render(state: &GameState) { /* GPU */ }
    pub fn poll_input() -> PlayerInput { /* Input system */ }
}
```

### Threading and Determinism

Keep simulation single-threaded. Only parallelize non-critical paths (I/O, checksums):

```rust
pub fn compute_checksum(state: &GameState) -> u64 {
    #[cfg(feature = "parallel")]
    { state.entities.par_iter().map(|e| e.hash()).sum() }
    #[cfg(not(feature = "parallel"))]
    { state.entities.iter().map(|e| e.hash()).sum() }
}
```

## WASI (Server-Side WASM)

```bash
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
wasmtime target/wasm32-wasip1/release/my_app.wasm
```

| Feature | Browser WASM | WASI |
|---------|--------------|------|
| File system | No | Yes |
| Network | Via JS fetch | Via WASI sockets |
| Environment vars | No | Yes |
| Sandboxing | Browser sandbox | Capability-based |

## Testing

```bash
cargo test                               # Native unit tests
wasm-pack test --headless --chrome       # Browser tests
wasm-pack test --node                    # Node.js tests
```

```rust
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);
    #[wasm_bindgen_test]
    fn test_in_browser() { assert_eq!(crate::add(1, 2), 3); }
}
```

## Memory Best Practices

1. Minimize JS/WASM boundary crossings -- batch operations
2. Use typed arrays (`Uint8Array`, `Float32Array`) for bulk data
3. Pre-allocate buffers -- avoid allocation in hot paths
4. Copy data across boundary rather than sharing pointers
