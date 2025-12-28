# Floating-Point Determinism in Rust

This guide covers achieving cross-platform floating-point determinism in Rust, essential for networked games, simulations, and any application requiring reproducible results across different machines.

## Table of Contents

- [Why Floating-Point is Non-Deterministic](#why-floating-point-is-non-deterministic)
- [What IEEE-754 Actually Guarantees](#what-ieee-754-actually-guarantees)
- [Solutions for Determinism](#solutions-for-determinism)
- [Cross-Platform Considerations](#cross-platform-considerations)
- [Crate Recommendations](#crate-recommendations)
- [Common Pitfalls and How to Avoid Them](#common-pitfalls-and-how-to-avoid-them)
- [Testing Strategies](#testing-strategies)

---

## Why Floating-Point is Non-Deterministic

Floating-point non-determinism stems from multiple sources that can cause the same operations to produce different results on different platforms, compilers, or even different runs of the same binary.

### 1. Transcendental Functions Vary by Platform

**Problem:** Standard library math functions (`sin`, `cos`, `exp`, `ln`, `atan2`, etc.) use platform-specific implementations that can differ in the last few bits.

```rust
// These can produce different results on different platforms!
// The Rust std documentation explicitly states:
// "The precision of this function is non-deterministic.
//  This means it varies by platform, Rust version, and
//  can even differ within the same execution from one
//  invocation to the next."

let x = (1.0_f64).sin();      // ⚠️ NON-DETERMINISTIC
let y = (2.0_f64).exp();      // ⚠️ NON-DETERMINISTIC
let z = (3.0_f64).ln();       // ⚠️ NON-DETERMINISTIC
let w = (1.0_f64).atan2(1.0); // ⚠️ NON-DETERMINISTIC
```

**Non-deterministic functions in Rust's `std`:** `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`, `exp`, `exp2`, `exp_m1`, `ln`, `ln_1p`, `log`, `log2`, `log10`, `powf`, `powi`, `cbrt`, `hypot`, `sin_cos`, `to_degrees`, `to_radians`, `gamma`, `ln_gamma`, `erf`, `erfc`, `abs_sub`

### 2. LLVM Constant Folding Differs from Runtime

**Problem:** When LLVM sees a constant expression like `(3.14159_f64).sin()`, it may compute the result at compile time using its own math library, which can differ from the runtime library your program uses.

```rust
// Compile-time constant folding may use different precision
const SIN_PI: f64 = std::f64::consts::PI.sin();  // ⚠️ May differ from runtime

fn runtime_sin() -> f64 {
    let pi = std::f64::consts::PI;
    pi.sin()  // ⚠️ May differ from compile-time value
}
```

### 3. Parallel Summation Breaks Associativity

**Problem:** Floating-point addition is not associative due to rounding. `(a + b) + c` may not equal `a + (b + c)`.

```rust
// Order matters for floating-point!
let a = 1e16_f64;
let b = 1.0_f64;
let c = -1e16_f64;

let result1 = (a + b) + c;  // = 1.0
let result2 = a + (b + c);  // = 0.0 (b + c rounds to c due to magnitude difference)

assert_ne!(result1, result2);  // Different results!
```

When using `rayon` for parallel iteration:

```rust
use rayon::prelude::*;

// ⚠️ NON-DETERMINISTIC: Order of partial sums varies per run
let sum: f64 = values.par_iter().sum();

// ⚠️ NON-DETERMINISTIC: fold order depends on thread scheduling
let product: f64 = values.par_iter().fold(|| 1.0, |a, &b| a * b).sum();
```

### 4. FPU vs SSE2 Precision Differences

**Problem:** On x86, the legacy x87 FPU uses 80-bit extended precision internally, while SSE2 uses the exact precision of the operation (32 or 64 bit). This can cause different results.

```rust
// On 32-bit x86 with x87 FPU:
// Intermediate results may be computed at 80-bit precision,
// then rounded down to 64-bit for storage

let a: f64 = 1.0 + 1e-16;
let b: f64 = a - 1.0;
// x87: May preserve more precision in intermediate
// SSE2: Uses exactly 64-bit precision throughout
```

**x87 FPU Issues:**

- Uses per-thread precision settings (control word)
- Settings can be changed by any library (Direct3D, older DirectX)
- 80-bit internal precision causes "double rounding"

**SSE2/AVX:**

- Uses per-instruction precision
- More predictable, matches standard double/float
- Default on 64-bit x86

### 5. Fused Multiply-Add (FMA) Instruction Availability

**Problem:** FMA computes `a * b + c` with a single rounding, which is more accurate but different from separate multiply and add operations (two roundings).

```rust
// These may produce different results!
let fma_result = a.mul_add(b, c);  // One rounding
let separate = a * b + c;          // Two roundings

// Whether compiler uses FMA instruction varies by:
// - Target CPU features (-C target-cpu)
// - Optimization level
// - Compiler version
```

### 6. Estimate Instructions

**Problem:** Some CPUs have fast estimate instructions (`rcpss`, `rsqrtss`) that are intentionally imprecise. Compilers may use these with `-ffast-math` or similar flags.

```rust
// May use fast estimate on some platforms
let inv = 1.0 / x;      // Could use rcpss on x86 with fast-math
let rsqrt = 1.0 / x.sqrt();  // Could use rsqrtss
```

### 7. Denormal (Subnormal) Number Handling

**Problem:** Very small numbers near zero (denormals/subnormals) may be flushed to zero on some platforms for performance.

```rust
let tiny = 1e-310_f64;  // Subnormal number
let result = tiny * tiny;  // May be 0.0 or a tiny positive number
```

---

## What IEEE-754 Actually Guarantees

IEEE-754 guarantees **correctly rounded results** for only five basic operations:

| Operation | Guarantee |
|-----------|-----------|
| `+` (add) | ✅ Correctly rounded |
| `-` (subtract) | ✅ Correctly rounded |
| `*` (multiply) | ✅ Correctly rounded |
| `/` (divide) | ✅ Correctly rounded |
| `sqrt` | ✅ Correctly rounded |

**Everything else is NOT guaranteed by IEEE-754:**

- Transcendental functions (sin, cos, exp, log, etc.)
- Power functions
- Special functions

> **Key Insight:** If you're running the same binary on the same processor, the main determinism concerns are: (1) altered FPU settings mid-execution, (2) uninitialized variables, and (3) non-FP sources of non-determinism. Cross-platform determinism requires more care.

---

## Solutions for Determinism

### Solution 1: Use `libm` for Consistent Math Functions

The `libm` crate provides pure Rust implementations of math functions that produce identical results across all platforms.

```toml
# Cargo.toml
[dependencies]
libm = "0.2"
```

```rust
use libm::{sin, cos, atan2, exp, log, pow, sqrt};

// ✅ DETERMINISTIC: Same result on all platforms
let x = sin(1.0_f64);
let y = cos(std::f64::consts::PI);
let z = atan2(1.0, 1.0);
let w = exp(2.0);
let v = log(10.0);
let p = pow(2.0, 10.0);
let s = sqrt(2.0);

// For f32:
let x32 = libm::sinf(1.0_f32);
let y32 = libm::cosf(1.0_f32);
```

**libm Function Categories:**

```rust
// Trigonometric
libm::sin(x);   libm::cos(x);   libm::tan(x);
libm::asin(x);  libm::acos(x);  libm::atan(x);
libm::atan2(y, x);

// Hyperbolic
libm::sinh(x);  libm::cosh(x);  libm::tanh(x);
libm::asinh(x); libm::acosh(x); libm::atanh(x);

// Exponential/Logarithmic
libm::exp(x);   libm::exp2(x);  libm::expm1(x);
libm::log(x);   libm::log2(x);  libm::log10(x);
libm::log1p(x);

// Power
libm::pow(x, y);  libm::sqrt(x);  libm::cbrt(x);

// Other
libm::fma(x, y, z);  // Fused multiply-add (deterministic!)
libm::fabs(x);
libm::floor(x); libm::ceil(x); libm::round(x); libm::trunc(x);
```

### Solution 2: Fixed-Point Arithmetic

For maximum determinism, use fixed-point arithmetic which eliminates floating-point entirely.

```toml
# Cargo.toml
[dependencies]
fixed = "1.29"
```

```rust
use fixed::types::I20F12;  // 20 integer bits, 12 fractional bits

// ✅ DETERMINISTIC: Integer arithmetic, no FPU involved
let a = I20F12::from_num(3.5);
let b = I20F12::from_num(2.25);

let sum = a + b;           // 5.75
let product = a * b;       // 7.875
let quotient = a / b;      // ~1.555...

// Convert back if needed
let float_result: f64 = sum.to_num();

// For trigonometry, use lookup tables or CORDIC algorithms
// The `cordic` crate provides fixed-point trig functions
```

**Choosing Fixed-Point Types:**

| Type | Range | Precision | Use Case |
|------|-------|-----------|----------|
| `I8F8` | ±127 | 0.004 | Small values, embedded |
| `I16F16` | ±32767 | 0.00001 | General purpose |
| `I20F12` | ±524287 | 0.0002 | Game physics (recommended) |
| `I32F32` | ±2B | ~0.0000000002 | High precision |

### Solution 3: Compensated Summation (Kahan/Neumaier)

For accurate, deterministic summation that minimizes floating-point error:

```toml
# Cargo.toml
[dependencies]
accurate = "0.4"
```

```rust
use accurate::sum::Sum2;
use accurate::traits::*;

// ✅ DETERMINISTIC: Compensated summation with error tracking
let values = vec![1e16, 1.0, -1e16, 1.0, 1.0];

// Using Sum2 (Kahan-like summation)
let sum: f64 = values.iter().cloned().sum_with_accumulator::<Sum2<_>>();
// Result: 3.0 (correct!)

// Manual Kahan summation implementation
fn kahan_sum(values: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut compensation = 0.0;

    for &value in values {
        let y = value - compensation;
        let t = sum + y;
        compensation = (t - sum) - y;
        sum = t;
    }
    sum
}
```

### Solution 4: Control Iteration Order

Ensure summations always happen in the same order:

```rust
// ⚠️ NON-DETERMINISTIC with rayon
let sum: f64 = items.par_iter().map(|x| x.value).sum();

// ✅ DETERMINISTIC: Sequential iteration
let sum: f64 = items.iter().map(|x| x.value).sum();

// ✅ DETERMINISTIC: Sort first, then iterate
let mut sorted_items = items.clone();
sorted_items.sort_by_key(|x| x.id);
let sum: f64 = sorted_items.iter().map(|x| x.value).sum();

// ✅ DETERMINISTIC: Use BTreeMap instead of HashMap
use std::collections::BTreeMap;
let map: BTreeMap<i32, f64> = /* ... */;
let sum: f64 = map.values().sum();  // Deterministic iteration order
```

### Solution 5: Software Floating-Point (Maximum Portability)

For absolute cross-platform determinism, use software floating-point:

```toml
# Cargo.toml
[dependencies]
softfloat-wrapper = "0.3"  # Berkeley SoftFloat wrapper
```

```rust
use softfloat_wrapper::{Float, F64, RoundingMode};

// ✅ DETERMINISTIC: Pure software implementation
let a = F64::from_f64(3.14159);
let b = F64::from_f64(2.71828);

let sum = a.add(b, RoundingMode::TiesToEven);
let product = a.mul(b, RoundingMode::TiesToEven);

// Convert back to hardware float
let result: f64 = sum.to_f64();
```

> **Trade-off:** Software float is significantly slower than hardware float. Use only when absolute cross-platform determinism is required.

---

## Cross-Platform Considerations

### Platform Comparison Matrix

| Platform | FPU Type | Transcendental Lib | Determinism Risk |
|----------|----------|-------------------|------------------|
| x86_64 Linux | SSE2/AVX | glibc libm | Low (with same toolchain) |
| x86_64 Windows | SSE2/AVX | MSVC runtime | Medium (different libm) |
| x86_64 macOS | SSE2/AVX | libSystem | Medium (different libm) |
| aarch64 (ARM64) | NEON | varies | High (different rounding) |
| WASM | Software | varies | High (browser-dependent) |
| x86 (32-bit) | x87 FPU | varies | Very High (extended precision) |

### Achieving Cross-Platform Determinism

1. **Use `libm` for all transcendental functions**
2. **Avoid compiler optimizations that reorder FP operations:** Use explicit parentheses
3. **Test across target platforms regularly**
4. **Consider fixed-point for critical calculations**

```rust
// Explicit ordering with parentheses
let result = (a * b) + (c * d);  // Don't let compiler reorder

// Use libm consistently
let angle_result = libm::atan2(y, x);  // Not (y/x).atan()
```

### WASM-Specific Considerations

```rust
#[cfg(target_arch = "wasm32")]
fn deterministic_sin(x: f64) -> f64 {
    // libm works in WASM too
    libm::sin(x)
}

// Avoid:
// - Web APIs that return floats (can vary by browser)
// - Performance.now() for timing-based calculations
// - Audio worklet outputs
```

---

## Crate Recommendations

### Primary Crates

| Crate | Version | Purpose | When to Use |
|-------|---------|---------|-------------|
| [`libm`](https://crates.io/crates/libm) | 0.2 | Pure Rust math functions | Always, for transcendentals |
| [`fixed`](https://crates.io/crates/fixed) | 1.29 | Fixed-point arithmetic | Physics, positions, scores |
| [`accurate`](https://crates.io/crates/accurate) | 0.4 | Compensated summation | Large sums, dot products |

### Specialized Crates

| Crate | Version | Purpose | When to Use |
|-------|---------|---------|-------------|
| [`softfloat-wrapper`](https://crates.io/crates/softfloat-wrapper) | 0.3 | Software FP (Berkeley SoftFloat) | Maximum portability |
| [`cordic`](https://crates.io/crates/cordic) | 0.1 | CORDIC algorithms | Fixed-point trig |
| [`az`](https://crates.io/crates/az) | 1.2 | Safe numeric casting | Fixed/float conversion |

### Cargo.toml Example

```toml
[dependencies]
libm = "0.2"           # Deterministic math functions
fixed = "1.29"         # Fixed-point arithmetic (requires Rust 1.83+)
accurate = "0.4"       # Compensated summation

# Optional, for maximum portability
softfloat-wrapper = { version = "0.3", optional = true }
```

---

## Common Pitfalls and How to Avoid Them

### Pitfall 1: Using std Math Functions

```rust
// ❌ WRONG: Platform-dependent
let angle = position.y.atan2(position.x);

// ✅ CORRECT: Use libm
let angle = libm::atan2(position.y, position.x);
```

### Pitfall 2: Parallel Reduction

```rust
use rayon::prelude::*;

// ❌ WRONG: Non-deterministic order
let total_damage: f64 = enemies.par_iter().map(|e| e.damage).sum();

// ✅ CORRECT: Sequential, or sorted parallel
let total_damage: f64 = enemies.iter().map(|e| e.damage).sum();

// Or use deterministic parallel with explicit ordering
let mut damages: Vec<f64> = enemies.iter().map(|e| e.damage).collect();
let total_damage: f64 = damages.iter().sum();
```

### Pitfall 3: Accumulation in Loops

```rust
// ❌ PROBLEMATIC: Error accumulates differently based on order
let mut total = 0.0;
for item in items {
    total += item.value;  // Rounding error accumulates
}

// ✅ BETTER: Compensated summation
use accurate::sum::Sum2;
use accurate::traits::*;
let total: f64 = items.iter().map(|i| i.value).sum_with_accumulator::<Sum2<_>>();
```

### Pitfall 4: Mixing Float Precisions

```rust
// ❌ PROBLEMATIC: Precision loss in conversion
let high_precision: f64 = /* calculation */;
let low_precision: f32 = high_precision as f32;  // Loses precision
let back_to_high: f64 = low_precision as f64;    // Can't recover!

// ✅ CORRECT: Stick to one precision throughout
let calculation: f64 = /* use f64 consistently */;
```

### Pitfall 5: NaN Propagation

```rust
// ❌ DANGEROUS: NaN comparisons always false
let a = f64::NAN;
if a == a {  // Always false for NaN!
    // Never executes
}

// ✅ CORRECT: Explicit NaN checks
if a.is_nan() {
    // Handle NaN case
}

// ✅ CORRECT: Use total_cmp for deterministic ordering
values.sort_by(|a, b| a.total_cmp(b));  // NaN handled consistently
```

### Pitfall 6: Compiler Reordering

```rust
// ❌ PROBLEMATIC: Compiler may reorder
let result = a + b + c + d;  // Could be ((a+b)+c)+d or (a+(b+c))+d etc.

// ✅ EXPLICIT: Force order with parentheses
let result = ((a + b) + c) + d;
```

### Pitfall 7: Division by Values Near Zero

```rust
// ❌ DANGEROUS: Division by near-zero
let ratio = a / b;  // If b ≈ 0, result may be ±Inf or vary by platform

// ✅ SAFER: Check and clamp
const EPSILON: f64 = 1e-10;
let ratio = if b.abs() < EPSILON {
    0.0  // Or some defined behavior
} else {
    a / b
};
```

---

## Testing Strategies

### Strategy 1: Cross-Platform CI Testing

```yaml
# .github/workflows/determinism.yml
name: Determinism Tests
on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        arch: [x86_64]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - name: Run determinism tests
        run: cargo test --features determinism-tests
      - name: Output checksum for comparison
        run: cargo run --example compute_checksum > checksum-${{ matrix.os }}.txt
      - uses: actions/upload-artifact@v4
        with:
          name: checksums
          path: checksum-*.txt

  compare:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
      - name: Compare checksums
        run: |
          # All checksums should be identical
          diff checksum-ubuntu-latest.txt checksum-windows-latest.txt
          diff checksum-ubuntu-latest.txt checksum-macos-latest.txt
```

### Strategy 2: Sync Test Sessions

Use rollback netcode's sync testing:

```rust
use fortress_rollback::{SessionBuilder, DesyncDetection};

#[test]
fn test_game_determinism() {
    let mut session = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .with_desync_detection_mode(DesyncDetection::On { interval: 1 })
        .start_synctest_session()
        .expect("session");

    // Run simulation
    for frame in 0..1000 {
        let requests = session.advance_frame(inputs);
        for request in requests {
            handle_request(request);
        }
    }
    // Any desync would have panicked
}
```

### Strategy 3: Checksum Comparison

```rust
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Deterministic checksum of game state
fn compute_checksum(state: &GameState) -> u64 {
    let mut hasher = DefaultHasher::new();

    // Hash in deterministic order
    state.frame.hash(&mut hasher);
    for (id, entity) in state.entities.iter() {  // BTreeMap!
        id.hash(&mut hasher);
        // Convert floats to bits for deterministic hashing
        entity.x.to_bits().hash(&mut hasher);
        entity.y.to_bits().hash(&mut hasher);
    }

    hasher.finish()
}

#[test]
fn test_simulation_determinism() {
    let checksum1 = {
        let mut state = GameState::new();
        simulate(&mut state, &inputs);
        compute_checksum(&state)
    };

    let checksum2 = {
        let mut state = GameState::new();
        simulate(&mut state, &inputs);
        compute_checksum(&state)
    };

    assert_eq!(checksum1, checksum2, "Simulation is non-deterministic!");
}
```

### Strategy 4: Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn deterministic_physics(
        x in -1000.0..1000.0_f64,
        y in -1000.0..1000.0_f64,
        vx in -100.0..100.0_f64,
        vy in -100.0..100.0_f64,
    ) {
        let entity1 = Entity { x, y, vx, vy };
        let entity2 = Entity { x, y, vx, vy };

        let result1 = simulate_physics(entity1, 100);
        let result2 = simulate_physics(entity2, 100);

        // Compare bit-for-bit
        assert_eq!(result1.x.to_bits(), result2.x.to_bits());
        assert_eq!(result1.y.to_bits(), result2.y.to_bits());
    }
}
```

### Strategy 5: Record and Replay

```rust
/// Record all inputs during a game session
struct InputRecorder {
    inputs: Vec<(Frame, Vec<PlayerInput>)>,
}

impl InputRecorder {
    fn record(&mut self, frame: Frame, inputs: Vec<PlayerInput>) {
        self.inputs.push((frame, inputs));
    }

    fn save(&self, path: &Path) -> io::Result<()> {
        let encoded = bincode::serialize(&self.inputs)?;
        std::fs::write(path, encoded)
    }
}

/// Replay recorded inputs and compare final state
fn replay_and_verify(recording_path: &Path, expected_checksum: u64) {
    let data = std::fs::read(recording_path).unwrap();
    let inputs: Vec<(Frame, Vec<PlayerInput>)> = bincode::deserialize(&data).unwrap();

    let mut state = GameState::new();
    for (frame, frame_inputs) in inputs {
        state.advance(frame_inputs);
    }

    let actual_checksum = compute_checksum(&state);
    assert_eq!(actual_checksum, expected_checksum,
        "Replay produced different result!");
}
```

---

## Quick Reference: Deterministic vs Non-Deterministic Operations

| Operation | Deterministic? | Alternative |
|-----------|----------------|-------------|
| `a + b`, `a - b`, `a * b`, `a / b` | ✅ Yes (IEEE-754) | - |
| `a.sqrt()` | ✅ Yes (IEEE-754) | - |
| `a.sin()`, `a.cos()`, etc. | ❌ No | `libm::sin(a)` |
| `a.exp()`, `a.ln()` | ❌ No | `libm::exp(a)`, `libm::log(a)` |
| `a.powf(b)` | ❌ No | `libm::pow(a, b)` |
| `a.powi(n)` | ❌ No | Use manual multiplication or `libm::pow` |
| `a.mul_add(b, c)` | ⚠️ Platform-dependent | `libm::fma(a, b, c)` |
| `iter.sum()` | ✅ Yes (if order is fixed) | `Sum2` for accuracy |
| `par_iter.sum()` | ❌ No | Sequential sum or `accurate` crate |
| `HashMap` iteration | ❌ No | `BTreeMap` |
| `a.to_degrees()` | ❌ No | `a * (180.0 / PI)` manually |

---

## Summary

### For Maximum Determinism

1. **Replace all std math with libm:**

   ```rust
   use libm::{sin, cos, atan2, sqrt, pow, exp, log};
   ```

2. **Use BTreeMap instead of HashMap**

3. **Avoid parallel reductions on floats**

4. **Use compensated summation for large sums:**

   ```rust
   use accurate::sum::Sum2;
   ```

5. **Consider fixed-point for critical game state:**

   ```rust
   use fixed::types::I20F12;
   ```

6. **Test across all target platforms in CI**

7. **Use checksums and sync testing to catch desyncs early**

---

## References

- [IEEE 754-2019 Standard](https://ieeexplore.ieee.org/document/8766229)
- [Bruce Dawson: Floating-Point Determinism](https://randomascii.wordpress.com/2013/07/16/floating-point-determinism/)
- [Rust `libm` crate](https://crates.io/crates/libm)
- [Rust `fixed` crate](https://crates.io/crates/fixed)
- [Rust `accurate` crate](https://crates.io/crates/accurate)
- [What Every Computer Scientist Should Know About Floating-Point Arithmetic](https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html)
