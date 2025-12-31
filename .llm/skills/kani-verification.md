# Kani Verification — Formal Proofs for Rust Code

> **This document provides guidance for writing Kani proof harnesses and adapting code for formal verification.**
> Kani is a bit-precise model checker that exhaustively verifies properties about Rust code — unlike testing, it explores *all* possible inputs.

## Core Philosophy: Exhaustive Verification

The key insight: **Kani doesn't sample inputs like fuzzing — it symbolically explores ALL possible execution paths within bounds.**

```rust
// ❌ Testing: Samples a finite number of inputs
#[test]
fn test_transfer() {
    let balance = 100;
    assert!(transfer(balance, 50).is_ok());  // Only tests ONE case
}

// ✅ Kani: Verifies ALL possible inputs exhaustively
#[kani::proof]
fn verify_transfer() {
    let balance: u32 = kani::any();  // ALL 2^32 values
    let amount: u32 = kani::any();   // ALL 2^32 values
    let _ = transfer(balance, amount);  // Kani checks EVERY combination
}
```

### Key Principles

1. **Symbolic inputs via `kani::any()`** — Generate all possible values, not samples
2. **Constrain with `kani::assume()`** — Narrow state space for tractable verification
3. **Bound loops explicitly** — Use `#[kani::unwind(N)]` for termination
4. **Stub complex dependencies** — Replace I/O, networking, etc. with abstract models
5. **Verify properties, not implementations** — Focus on what should hold, not how

---

## When to Use Kani vs Other Verification Tools

| Tool | Best For | Limitations |
|------|----------|-------------|
| **Unit Tests** | Happy path, basic correctness | Sample-based, can miss edge cases |
| **Proptest/Quickcheck** | Property-based random testing | Statistical, not exhaustive |
| **Fuzzing (cargo-fuzz)** | Finding crashes in complex code | Coverage-guided, not complete |
| **MIRI** | Undefined behavior detection | Runtime checking, samples only |
| **Kani** | Mathematical proof of properties | Slower, requires bounds |
| **TLA+** | Distributed protocol design | High-level models, not code |

### Kani Shines For

- **Boundary conditions** — Zero, max, overflow at limits
- **Untrusted input validation** — Verify all possible inputs are handled
- **Arithmetic correctness** — Catch overflow, underflow, division by zero
- **State machine invariants** — Verify valid transitions
- **Unsafe code justification** — Prove unsafe operations are actually safe
- **Code that's hard to test** — Clock-dependent, large state spaces

### Kani Is NOT Suitable For

- Concurrent code (limited support)
- Inline assembly (`asm!`)
- FFI and external C code
- Unbounded loops without explicit limits
- Performance-critical hot loops (verification overhead)

---

## Installation and Setup

### Prerequisites

- Rust 1.58+ installed via `rustup`
- Linux or macOS (x86_64 or aarch64)

### Install Kani

```bash
# Install the Kani verifier
cargo install --locked kani-verifier

# Download prebuilt dependencies (CBMC, etc.)
cargo kani setup

# Verify installation
kani --version
```

### Install Specific Version

```bash
cargo install --locked kani-verifier --version 0.66.0
cargo kani setup
```

---

## Writing Proof Harnesses

### Basic Harness Structure

```rust
// Use cfg(kani) to include proofs only during verification
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn verify_function_does_not_panic() {
        // Generate symbolic inputs
        let x: u32 = kani::any();
        let y: u32 = kani::any();

        // Call the function under verification
        let _ = my_function(x, y);

        // Kani automatically checks for:
        // - Panics (unwrap, expect, assert failures)
        // - Arithmetic overflow/underflow
        // - Out-of-bounds access
        // - Division by zero
    }
}
```

### The `kani::any()` API

Generate symbolic values representing ALL possible values of a type:

```rust
// Primitives
let x: u32 = kani::any();       // All 2^32 values
let b: bool = kani::any();      // Both true and false
let c: char = kani::any();      // All valid Unicode code points

// Arrays
let arr: [u8; 4] = kani::any(); // All possible 4-byte arrays

// Options and Results
let opt: Option<u32> = kani::any();  // None + all Some(x)

// Custom types with derive
#[derive(kani::Arbitrary)]
struct Point { x: i32, y: i32 }

let point: Point = kani::any();
```

### Constraining Inputs with `kani::assume()`

Narrow the state space for tractable verification:

```rust
#[kani::proof]
fn verify_division() {
    let numerator: u32 = kani::any();
    let denominator: u32 = kani::any();

    // ❌ Without constraint: Kani will find division by zero
    // let result = numerator / denominator;

    // ✅ Constrain to valid inputs
    kani::assume(denominator != 0);
    let result = numerator / denominator;

    // Now verify properties
    assert!(result <= numerator);
}

// Alternative: kani::any_where()
#[kani::proof]
fn verify_with_constrained_input() {
    let denominator: u32 = kani::any_where(|&d| d != 0);
    // denominator is guaranteed non-zero
}
```

### Loop Unwinding

Kani needs explicit bounds for loops:

```rust
#[kani::proof]
#[kani::unwind(11)]  // Unwind loops up to 11 iterations
fn verify_loop() {
    let iterations: usize = kani::any();
    kani::assume(iterations <= 10);

    let mut sum = 0u32;
    for i in 0..iterations {
        sum = sum.saturating_add(i as u32);
    }

    // Verify property
    assert!(sum <= 45);  // Max sum of 0..10
}
```

### Stubbing Functions

Replace complex functions with abstract models:

```rust
// Original function uses external I/O
fn get_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// Stub for verification
#[cfg(kani)]
fn get_timestamp_stub() -> u64 {
    kani::any()  // Any possible timestamp
}

#[kani::proof]
#[kani::stub(get_timestamp, get_timestamp_stub)]
fn verify_with_any_timestamp() {
    let ts = get_timestamp();
    // ts is now symbolic, Kani explores all values
}
```

---

## Deriving Arbitrary for Custom Types

### Auto-Derive

```rust
#[derive(kani::Arbitrary)]
pub struct PlayerInput {
    pub frame: u32,
    pub buttons: u16,
    pub player_id: u8,
}

#[derive(kani::Arbitrary)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected { latency_ms: u32 },
    Error { code: i32 },
}

#[kani::proof]
fn verify_handles_all_inputs() {
    let input: PlayerInput = kani::any();
    let state: ConnectionState = kani::any();

    // Process all possible combinations
    process(input, state);
}
```

### Manual Implementation for Complex Invariants

```rust
// Type with invariants that can't be expressed via derive
pub struct Frame(u32);

impl kani::Arbitrary for Frame {
    fn any() -> Self {
        let value: u32 = kani::any();
        // Encode invariant: frame must be less than max
        kani::assume(value <= Frame::MAX_VALUE);
        Frame(value)
    }
}
```

### Bounded Arbitrary for Unbounded Types

```rust
use kani::BoundedArbitrary;

// For Vec, String, and other unbounded types
impl BoundedArbitrary for MyBuffer {
    const MAX_LEN: usize = 16;  // Verification bound

    fn any_bounded() -> Self {
        let len: usize = kani::any_where(|&l| l <= Self::MAX_LEN);
        let data: [u8; 16] = kani::any();
        MyBuffer { data: data[..len].to_vec() }
    }
}
```

---

## Verification Patterns for Common Scenarios

### Pattern 1: Verify No Panics (Zero-Panic Policy)

```rust
#[kani::proof]
fn verify_no_panics() {
    // Generate arbitrary valid inputs
    let input: Input = kani::any();

    // The function should never panic
    // Kani will find any path that panics
    let result = process_input(input);

    // Optionally verify result properties
    assert!(result.is_ok() || matches!(result, Err(FortressError::_)));
}
```

### Pattern 2: Verify Arithmetic Safety

```rust
#[kani::proof]
fn verify_frame_arithmetic() {
    let frame: u32 = kani::any();
    let delta: u32 = kani::any();

    // checked_add should never panic
    if let Some(result) = frame.checked_add(delta) {
        assert!(result >= frame);
        assert!(result >= delta);
    }
    // If None, verify overflow was correctly detected
}
```

### Pattern 3: Verify State Machine Transitions

```rust
#[derive(kani::Arbitrary, Clone, Copy)]
enum State { Init, Running, Paused, Stopped }

#[kani::proof]
fn verify_valid_transitions() {
    let current: State = kani::any();
    let event: Event = kani::any();

    let next = transition(current, event);

    // Verify invariant: can't go from Stopped to Running
    kani::assume(matches!(current, State::Stopped));
    assert!(!matches!(next, State::Running));
}
```

### Pattern 4: Verify Unsafe Code Safety

```rust
// Function using unsafe for performance
fn sum_unchecked(a: u32, b: u32) -> u32 {
    // Precondition: a + b must not overflow
    unsafe { a.unchecked_add(b) }
}

#[kani::proof]
fn verify_unchecked_add_precondition() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();

    // Only call if overflow is impossible
    kani::assume(a.checked_add(b).is_some());

    let result = sum_unchecked(a, b);

    // Verify result correctness
    assert_eq!(result, a.wrapping_add(b));
}
```

### Pattern 5: Verify Input Validation

```rust
#[kani::proof]
fn verify_input_validation_is_complete() {
    let raw_input: RawInput = kani::any();

    match validate_input(raw_input) {
        Ok(validated) => {
            // All validated inputs must satisfy invariants
            assert!(validated.frame <= MAX_FRAME);
            assert!(validated.player_id < MAX_PLAYERS);
        }
        Err(_) => {
            // Rejected inputs should violate some constraint
            // (This is a sanity check that we're not too restrictive)
        }
    }
}
```

---

## Performance Optimization

### Start Small, Scale Up

```rust
// ❌ Too expensive: huge state space
#[kani::proof]
#[kani::unwind(1000)]
fn verify_expensive() {
    let buffer: [u8; 1024] = kani::any();  // 2^8192 possibilities!
    process(&buffer);
}

// ✅ Start with minimal bounds
#[kani::proof]
#[kani::unwind(5)]
fn verify_minimal() {
    let buffer: [u8; 4] = kani::any();  // 2^32 possibilities
    process(&buffer);
}
```

### Constrain Early

```rust
// ❌ Constraint too late
#[kani::proof]
fn verify_late_constraint() {
    let x: u32 = kani::any();
    let y: u32 = kani::any();
    let z = expensive_computation(x, y);
    kani::assume(x < 100);  // Wasted work on x >= 100
}

// ✅ Constraint early
#[kani::proof]
fn verify_early_constraint() {
    let x: u32 = kani::any_where(|&v| v < 100);
    let y: u32 = kani::any();
    let z = expensive_computation(x, y);
}
```

### Use Targeted Harnesses

```rust
// ❌ One huge harness verifying everything
#[kani::proof]
fn verify_everything() {
    // ... 500 lines of setup and verification
}

// ✅ Multiple focused harnesses
#[kani::proof]
fn verify_initialization() { /* ... */ }

#[kani::proof]
fn verify_state_transition() { /* ... */ }

#[kani::proof]
fn verify_cleanup() { /* ... */ }
```

### Solver Selection

```rust
// Try different SAT solvers for performance
#[kani::proof]
#[kani::solver(cadical)]  // Default, good general performance
fn verify_with_cadical() { /* ... */ }

#[kani::proof]
#[kani::solver(kissat)]   // Sometimes faster
fn verify_with_kissat() { /* ... */ }

#[kani::proof]
#[kani::solver(minisat)]  // Lightweight
fn verify_with_minisat() { /* ... */ }
```

---

## CI Integration

### CRITICAL: Adding New Kani Proofs to CI

**When adding new `#[kani::proof]` functions, you MUST also add them to the tier lists in `scripts/verify-kani.sh`.**

This is enforced by CI — the `kani-coverage-check` job will fail if any proofs are missing.

#### Step-by-Step Process

1. **Write your Kani proof** in the appropriate source file:

   ```rust
   #[cfg(kani)]
   mod kani_proofs {
       #[kani::proof]
       fn proof_my_invariant() {
           // Your proof here
       }
   }
   ```

2. **Add the proof to the appropriate tier** in `scripts/verify-kani.sh`:
   - **Tier 1** (`TIER1_PROOFS`): Fast proofs (<30s each) — simple property checks
   - **Tier 2** (`TIER2_PROOFS`): Medium proofs (30s-2min each) — moderate complexity
   - **Tier 3** (`TIER3_PROOFS`): Slow proofs (>2min each) — complex state verification

3. **Validate coverage** before committing:

   ```bash
   ./scripts/check-kani-coverage.sh
   ```

   This must show "SUCCESS: All N Kani proofs are covered in tier lists."

#### Example: Adding a New Proof

```bash
# 1. Add proof to source code
# 2. Add to tier list in verify-kani.sh:
TIER1_PROOFS=(
    # ... existing proofs ...
    "proof_my_invariant"  # Add here
)

# 3. Validate coverage
./scripts/check-kani-coverage.sh
# Output: SUCCESS: All N Kani proofs are covered in tier lists.
```

#### CI Enforcement

The CI workflow includes a `kani-coverage-check` job that:

1. Scans source code for all `#[kani::proof]` functions
2. Scans `verify-kani.sh` for all tier list entries
3. **Fails the build** if any proofs are missing or stale

This ensures no proof is ever silently skipped during verification.

### GitHub Actions Workflow

```yaml
name: Kani Verification

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  kani:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Kani
        run: |
          cargo install --locked kani-verifier
          cargo kani setup

      - name: Run Kani Proofs
        run: cargo kani --tests
```

### Tiered Verification Strategy

```yaml
jobs:
  kani-fast:
    # Run on every PR - under 5 minutes
    runs-on: ubuntu-latest
    steps:
      - run: cargo kani --harness "verify_quick_*"

  kani-full:
    # Run on main branch only - may take longer
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - run: cargo kani
```

### Organize Harnesses by Verification Time

```rust
// Fast proofs - run on every PR
#[kani::proof]
fn verify_quick_arithmetic() { /* ... */ }

#[kani::proof]
fn verify_quick_bounds_check() { /* ... */ }

// Slow proofs - run nightly or on main only
#[kani::proof]
#[kani::unwind(100)]
fn verify_full_state_machine() { /* ... */ }
```

---

## Integration with AI/LLM Workflows

### The Kani Feedback Loop

When using LLMs to generate or modify code:

```
┌─────────────────────────────────────────────────────────┐
│  1. LLM generates/modifies Rust code                    │
│  2. Add #[kani::proof] harness with kani::any() inputs  │
│  3. Run `cargo kani`                                    │
│  4. If FAILED → Feed error + counterexample to LLM      │
│  5. LLM generates fix → Repeat from step 3              │
│  6. VERIFICATION: SUCCESSFUL → Code is formally proven  │
└─────────────────────────────────────────────────────────┘
```

### Prompting for Kani Harnesses

When asking an LLM to write Kani proofs:

```
Write a Kani proof harness for this function that:
1. Uses kani::any() for all input parameters
2. Adds appropriate kani::assume() constraints for valid inputs
3. Verifies the function doesn't panic on any valid input
4. Checks that return values satisfy specified postconditions
5. Uses #[kani::unwind(N)] if there are loops (start with N=10)
```

### Interpreting Kani Output for LLMs

When verification fails, include in prompt:

```
Kani verification FAILED with:
- Failed Check: "attempt to add with overflow"
- Location: src/lib.rs line 42
- Counterexample: x = 4294967295, y = 1

Fix the code to handle this edge case without panicking.
Use checked_add() or saturating_add() as appropriate.
```

---

## Common Pitfalls and Solutions

### Pitfall 1: Unbounded Loops

```rust
// ❌ Will never terminate
#[kani::proof]
fn verify_unbounded() {
    let n: usize = kani::any();
    for i in 0..n {  // n could be huge!
        // ...
    }
}

// ✅ Add explicit bound
#[kani::proof]
#[kani::unwind(101)]
fn verify_bounded() {
    let n: usize = kani::any();
    kani::assume(n <= 100);
    for i in 0..n {
        // ...
    }
}
```

### Pitfall 2: State Explosion

```rust
// ❌ State space too large
#[kani::proof]
fn verify_huge_state() {
    let data: [u64; 100] = kani::any();  // 2^6400 states!
}

// ✅ Use bounded verification
#[kani::proof]
fn verify_reasonable_state() {
    let data: [u64; 2] = kani::any();  // 2^128 states
    // Or use constrained elements
}
```

### Pitfall 3: Ignoring Counterexamples

```rust
// ❌ Ignoring the bug
#[kani::proof]
fn verify_ignore_bug() {
    let x: u32 = kani::any();
    kani::assume(x < u32::MAX);  // Just to make it pass!
    let result = x + 1;          // Still overflows at MAX-1!
}

// ✅ Fix the actual code
fn safe_increment(x: u32) -> Option<u32> {
    x.checked_add(1)
}

#[kani::proof]
fn verify_fixed() {
    let x: u32 = kani::any();
    let _ = safe_increment(x);  // Now handles all inputs
}
```

### Pitfall 4: Not Testing What You Think

```rust
// ❌ Only tests one code path
#[kani::proof]
fn verify_misleading() {
    let x: u32 = 42;  // Concrete value, not symbolic!
    let result = process(x);
    assert!(result > 0);  // Only proves for x=42
}

// ✅ Test all inputs
#[kani::proof]
fn verify_all_inputs() {
    let x: u32 = kani::any();  // Symbolic
    let result = process(x);
    assert!(result > 0);  // Proves for ALL u32
}
```

---

## Real-World Success Stories

### Firecracker (AWS microVM)

- **Verified:** I/O rate limiter, VirtIO transport layer
- **Bugs found:** 5 in rate limiter + 1 in VirtIO
- **Key finding:** Rounding error allowing bandwidth exceeded by 0.01%

### Hifitime (Scientific Time Library)

- **Context:** 74+ integration tests already existed
- **Bugs found:** 8+ categories in single PR
- **Locations:** All near boundaries (zero, max, min durations)
- **Lesson:** Even well-tested code has boundary bugs

### s2n-quic (AWS QUIC Implementation)

- **Verified:** Protocol formatting, transmission, encryption
- **Benefit:** Confidence for optimization work on critical paths

---

## Quick Reference

### Essential Attributes

| Attribute | Purpose |
|-----------|---------|
| `#[kani::proof]` | Mark function as proof harness |
| `#[kani::unwind(N)]` | Set loop unwinding bound |
| `#[kani::should_panic]` | Expect harness to panic |
| `#[kani::stub(orig, repl)]` | Replace function |
| `#[kani::solver(name)]` | Choose SAT solver |

### Essential Functions

| Function | Purpose |
|----------|---------|
| `kani::any::<T>()` | Generate symbolic value of type T |
| `kani::any_where(\|v\| pred)` | Constrained symbolic value |
| `kani::assume(cond)` | Add constraint (narrows state space) |
| `kani::assert(cond)` | Verify property (failure = bug found) |

### Commands

```bash
# Run all proofs in a cargo project
cargo kani

# Run specific harness
cargo kani --harness verify_specific

# Run on single file
kani src/lib.rs

# With verbose output
cargo kani -v

# Generate HTML report
cargo kani --visualize
```

---

## Checklist for Adding Kani to a Codebase

- [ ] Install Kani: `cargo install --locked kani-verifier && cargo kani setup`
- [ ] Add `#[cfg(kani)]` module for proofs
- [ ] Derive `kani::Arbitrary` for domain types
- [ ] Write focused harnesses for critical functions
- [ ] Set appropriate `#[kani::unwind(N)]` bounds
- [ ] Stub I/O, networking, and external dependencies
- [ ] Add Kani to CI pipeline
- [ ] Document verification assumptions
- [ ] Run `cargo kani` locally before pushing

---

## Resources

| Resource | Link |
|----------|------|
| Official Documentation | <https://model-checking.github.io/kani/> |
| Tutorial | <https://model-checking.github.io/kani/kani-tutorial.html> |
| GitHub Repository | <https://github.com/model-checking/kani> |
| Kani Blog | <https://model-checking.github.io/kani-verifier-blog/> |
| Bolero Integration | <https://model-checking.github.io/kani-verifier-blog/2022/10/27/using-kani-with-the-bolero-property-testing-framework.html> |
| Firecracker Case Study | <https://model-checking.github.io/kani-verifier-blog/2023/08/31/using-kani-to-validate-security-boundaries-in-aws-firecracker.html> |

---

*This policy applies to all formal verification code in Fortress Rollback.*
