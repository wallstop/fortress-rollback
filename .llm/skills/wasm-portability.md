# WebAssembly Portability and Determinism

> **Understanding how WebAssembly achieves cross-platform determinism and sandboxed execution.**

## Why WASM is Portable

WebAssembly achieves portability through several key design decisions:

### 1. Virtual Stack Machine

WASM uses a **stack-based virtual machine** rather than targeting hardware registers:

```wat
;; WebAssembly Text Format (WAT)
(func $add (param $a i32) (param $b i32) (result i32)
  local.get $a    ;; Push $a onto stack
  local.get $b    ;; Push $b onto stack
  i32.add         ;; Pop two values, push sum
)
```

This abstraction means:
- No assumptions about CPU register count
- No platform-specific calling conventions
- Runtime translates to native instructions

### 2. Fixed-Width Types

WASM defines exactly four core numeric types (expanded in recent proposals):

| Type | Description |
|------|-------------|
| `i32` | 32-bit integer |
| `i64` | 64-bit integer |
| `f32` | 32-bit IEEE 754 float |
| `f64` | 64-bit IEEE 754 float |

No platform-dependent types like C's `int` or `long` — sizes are guaranteed.

### 3. Structured Control Flow

Unlike native assembly with arbitrary jumps, WASM has **structured control flow**:

```wat
(block $outer
  (block $inner
    br_if $inner   ;; Conditional branch to end of $inner
    br $outer      ;; Unconditional branch to end of $outer
  )
)
```

---

## WASM vs Native Assembly

| Aspect | Native Assembly | WebAssembly |
|--------|-----------------|-------------|
| **Register model** | Fixed hardware registers | Virtual stack + locals |
| **Memory model** | Flat address space | Linear memory, bounds-checked |
| **System calls** | Platform-specific (syscall) | Imported functions from host |
| **Endianness** | Architecture-dependent | Little-endian (specified) |
| **Instruction set** | ISA-specific | Universal bytecode |
| **ABI** | Platform calling conventions | Single canonical ABI |
| **Undefined behavior** | Varies by platform | Fully specified |

---

## Determinism Guarantees

### What's Guaranteed Deterministic

| Operation | Deterministic? | Notes |
|-----------|----------------|-------|
| Integer arithmetic | ✅ Yes | Fully specified |
| Float arithmetic | ✅ Yes | IEEE 754-2019 compliant |
| Memory access | ✅ Yes | Same inputs → same results |
| Control flow | ✅ Yes | Structured, no UB |
| Function calls | ✅ Yes | Deterministic dispatch |

### Why This Matters for Rollback Netcode

Native code has determinism challenges:

```rust
// Native: May differ across platforms
let result = (0.1_f32 + 0.2_f32) * 0.3_f32;
// x87: 80-bit intermediate precision
// SSE: 64-bit precision
// ARM: Different FPU behavior
```

WASM solves this:

```rust
// WASM: Guaranteed identical everywhere
// Same binary → same float results on all platforms
let result = (0.1_f32 + 0.2_f32) * 0.3_f32;
```

### NaN Handling

WASM specifies canonical NaN values:

```rust
// Canonical NaN handling for serialization
fn canonicalize_nan(x: f32) -> f32 {
    if x.is_nan() {
        f32::NAN  // Use canonical NaN
    } else {
        x
    }
}

// For checksum computation
fn float_to_deterministic_bits(x: f32) -> u32 {
    if x.is_nan() {
        0x7FC00000  // Canonical quiet NaN
    } else {
        x.to_bits()
    }
}
```

---

## Linear Memory Model

WASM uses a single, contiguous linear memory:

```
┌─────────────────────────────────────────┐
│ Linear Memory (grows upward, in pages)  │
├─────────────────────────────────────────┤
│ 0x00000000  Data Section (static data)  │
├─────────────────────────────────────────┤
│             Heap (grows downward)       │
├─────────────────────────────────────────┤
│             Stack (grows upward)        │
│ End of memory (page boundary)           │
└─────────────────────────────────────────┘
```

### Memory Safety

All memory accesses are bounds-checked:

```rust
// In native code: UB, possible security vulnerability
unsafe {
    let ptr = 0xDEADBEEF as *const u32;
    let _ = *ptr;  // Arbitrary memory access
}

// In WASM: Guaranteed trap, safe
// Out-of-bounds access → runtime trap, not UB
```

### Memory Configuration

```javascript
// JavaScript: Configure memory
const memory = new WebAssembly.Memory({
    initial: 256,  // 256 pages = 16MB initial
    maximum: 1024, // 1024 pages = 64MB maximum
});
```

---

## Sandboxing and Security

WASM's security is based on **capability-based sandboxing**:

### Zero Capabilities by Default

WASM modules have **no capabilities** unless explicitly granted:

```rust
// Host grants specific capabilities
let wasi = WasiCtxBuilder::new()
    .preopened_dir(Dir::open_ambient_dir("/data")?, "/")  // Only /data
    .env("APP_ENV", "production")  // Specific env var
    .build();

// Module cannot access anything outside granted capabilities
```

### Security Properties

1. **No ambient authority** — Must be explicitly granted access
2. **Memory safety** — Bounds-checked access, no buffer overflows
3. **Control flow integrity** — Structured control flow, validated at load
4. **Type safety** — Statically typed, validated before execution
5. **No raw syscalls** — All I/O through imported functions

---

## WASI (WebAssembly System Interface)

### Overview

WASI provides standardized APIs for WebAssembly outside browsers:

| Capability | Description |
|------------|-------------|
| File system | Read/write files (within capability grants) |
| Network | Sockets (Preview 2+) |
| Clocks | Monotonic and wall-clock time |
| Random | Secure random number generation |
| Environment | Args and environment variables |

### WASI vs Browser WASM

| Feature | Browser WASM | WASI |
|---------|--------------|------|
| File system | ❌ No | ✅ Yes |
| Network | Via JS fetch | Via WASI sockets |
| Environment vars | ❌ No | ✅ Yes |
| CLI args | ❌ No | ✅ Yes |
| Sandboxing | Browser sandbox | Capability-based |

### WASI Targets

```bash
# Browser/embedded (no OS)
cargo build --target wasm32-unknown-unknown

# WASI Preview 1 (most compatible)
cargo build --target wasm32-wasip1

# WASI Preview 2 (newer, component model)
cargo build --target wasm32-wasip2
```

---

## Runtime Environments

### Browser Runtimes

Browsers embed WASM runtimes with JavaScript interop:

```javascript
// Browser instantiation
const { instance } = await WebAssembly.instantiateStreaming(
    fetch('module.wasm'),
    { env: { log: (x) => console.log(x) } }
);

const result = instance.exports.compute(42);
```

### Server-Side Runtimes

| Runtime | Language | Key Features |
|---------|----------|--------------|
| **Wasmtime** | Rust | Reference implementation, fast JIT |
| **Wasmer** | Rust | Multiple backends, package registry |
| **wasm3** | C | Interpreter, tiny footprint |
| **WasmEdge** | C++ | Edge computing, AI inference |
| **WAMR** | C | IoT and embedded |

### Wasmtime Example

```rust
use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, "module.wasm")?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let compute = instance.get_typed_func::<i32, i32>(&mut store, "compute")?;
    let result = compute.call(&mut store, 42)?;

    println!("Result: {}", result);
    Ok(())
}
```

---

## Limitations and Proposals

### Current Limitations

| Limitation | Impact | Workaround |
|------------|--------|------------|
| 32-bit memory | 4GB max | Memory64 proposal |
| No native threads | Single-threaded default | Atomics + Web Workers |
| No SIMD (older) | Performance | SIMD proposal (shipped) |
| No GC | Manual memory | GC proposal (shipping) |
| No exceptions | Error handling | Exception handling proposal |

### Active Proposals (2024-2025)

| Proposal | Status | Description |
|----------|--------|-------------|
| **GC** | Phase 4 | Garbage-collected types |
| **Exception Handling** | Phase 4 | try/catch/throw |
| **Component Model** | Phase 3 | High-level composition |
| **Threads** | Phase 4 | SharedArrayBuffer, atomics |
| **SIMD** | Phase 5 | 128-bit SIMD operations |
| **Memory64** | Phase 3 | 64-bit memory addresses |
| **Tail Calls** | Phase 4 | Guaranteed TCO |

---

## Architecture for Deterministic Games

### Recommended Pattern

```rust
// Core game logic - compile to WASM for determinism
mod simulation {
    /// Advance game state by one frame.
    ///
    /// This is fully deterministic when compiled to WASM.
    /// Same inputs → same outputs on all platforms.
    pub fn advance_frame(state: &mut GameState, inputs: &[PlayerInput]) {
        for input in inputs {
            state.apply_input(input);
        }
        state.physics_tick();  // Float math is deterministic!
        state.resolve_collisions();
    }

    pub fn compute_checksum(state: &GameState) -> u64 {
        state.deterministic_hash()
    }
}

// Platform-specific code - stays native
mod platform {
    pub fn render(state: &GameState) { /* GPU */ }
    pub fn play_audio(events: &[AudioEvent]) { /* Audio API */ }
    pub fn poll_input() -> PlayerInput { /* Input system */ }
}
```

### Embedding WASM in Native App

```rust
use wasmtime::*;

struct GameEngine {
    store: Store<()>,
    advance_frame: TypedFunc<(i32, i32), ()>,
    memory: Memory,
}

impl GameEngine {
    pub fn tick(&mut self, inputs: &[u8]) -> Result<()> {
        // Copy inputs to WASM linear memory
        self.memory.write(&mut self.store, INPUT_OFFSET, inputs)?;

        // Run deterministic simulation in WASM
        self.advance_frame.call(
            &mut self.store,
            (INPUT_OFFSET as i32, inputs.len() as i32)
        )?;

        Ok(())
    }
}
```

---

## Sources of Non-Determinism (Even in WASM)

⚠️ These break determinism even in WASM:

| Source | Problem | Solution |
|--------|---------|----------|
| Host-provided time | `Date.now()` varies | Use frame counters |
| Threading | Schedule order varies | Single-threaded simulation |
| Random imports | System entropy | Seeded PRNG in WASM |
| Network | Timing varies | Separate I/O from simulation |
| Hash with random seed | Different per run | Use deterministic hash |

---

## Summary

WebAssembly provides:

1. **Portable bytecode** — Same binary runs everywhere
2. **Deterministic execution** — IEEE 754 floats, no UB
3. **Memory safety** — Bounds-checked, sandboxed
4. **Security** — Capability-based, no ambient authority

For rollback netcode, consider:

- Compiling game simulation to WASM for cross-platform determinism
- Keeping I/O and rendering in native code
- Single-threaded simulation (threading breaks determinism)
- Seeded RNG and frame counters instead of system time

---

## Related Guides

- [wasm-rust-guide.md](wasm-rust-guide.md) — Rust to WASM compilation
- [wasm-threading.md](wasm-threading.md) — Threading in WASM
- [cross-platform-rust.md](cross-platform-rust.md) — Multi-platform architecture
- [no-std-guide.md](no-std-guide.md) — `no_std` patterns
- [determinism-guide.md](determinism-guide.md) — Determinism best practices

---

*WebAssembly's determinism guarantees make it an excellent choice for cross-platform game simulation.*
