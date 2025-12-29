# Z3 Verification — SMT-Based Proofs for Algorithm Correctness

> **This document provides guidance for writing Z3 proofs to verify mathematical properties of algorithms.**
> Z3 is an SMT (Satisfiability Modulo Theories) solver that can prove properties about abstract mathematical models of code — unlike Kani which verifies actual Rust code.

## Core Philosophy: Mathematical Modeling

The key insight: **Z3 proves properties about *abstract models* of algorithms, not the code itself.**

```rust
// Z3 proves: "For ALL valid inputs, the mathematical model satisfies the property"
// This is different from testing (samples) and Kani (bounded model checking of code)

#[test]
fn z3_proof_rollback_valid() {
    let solver = Solver::new();

    // Model the algorithm mathematically
    let current_frame = Int::fresh_const("current_frame");
    let target_frame = Int::fresh_const("target_frame");

    // Assert preconditions (the "given")
    solver.assert(current_frame.ge(0));
    solver.assert(target_frame.lt(&current_frame));

    // Try to find a counterexample to our property
    solver.assert(target_frame.ge(&current_frame));  // Negation of property

    // If UNSAT, no counterexample exists → property holds!
    assert_eq!(solver.check(), SatResult::Unsat);
}
```

### Key Principles

1. **Model, don't translate** — Create mathematical abstractions, not line-by-line code translations
2. **Prove by contradiction** — Assert the negation of your property; UNSAT = property holds
3. **Preconditions matter** — Explicitly model all assumptions about inputs
4. **Size-independent proofs** — Verify properties that scale to any configuration
5. **Document the gap** — Note what the model covers vs. what code actually does

---

## When to Use Z3 vs Other Verification Tools

| Tool | Best For | What It Proves |
|------|----------|----------------|
| **Unit Tests** | Expected behavior | Specific concrete cases work |
| **Property Tests** | Invariant fuzzing | Property holds for random samples |
| **Kani** | Rust code correctness | Bounded exhaustive over actual code |
| **TLA+** | Distributed protocols | State machine model is correct |
| **Z3** | Algorithm properties | Mathematical model is correct |

### Z3 Shines For

- **Arithmetic reasoning** — Overflow, bounds, modular arithmetic
- **Constraint validation** — Proving bounds checks are sufficient
- **Algorithm correctness** — Circular buffers, frame calculations
- **Invariant verification** — "This property always holds given these preconditions"
- **Design validation** — Before implementing, prove the algorithm is sound

### Z3 Is NOT Suitable For

- Verifying actual Rust code (use Kani)
- Concurrent algorithm correctness (use TLA+ or loom)
- Testing specific implementations (use unit tests)
- Unbounded data structures (use bounded approximations)

---

## Installation and Setup

### System Z3 (Recommended — Fast)

```bash
# Ubuntu/Debian
sudo apt-get install libz3-dev

# macOS
brew install z3

# Then enable the feature
cargo test --features z3-verification
```

### Bundled Build (Slow — Fallback)

```bash
# Compiles Z3 from source (~30+ minutes)
cargo test --features z3-verification-bundled
```

### Cargo.toml Configuration

```toml
[features]
z3-verification = ["dep:z3"]
z3-verification-bundled = ["z3-verification", "z3?/bundled"]

[dependencies]
z3 = { version = "0.19", optional = true }
```

---

## Z3 API Quick Reference

### Core Types

| Type | Purpose | Example |
|------|---------|---------|
| `Config` | Solver configuration | `Config::new()` |
| `Context` | Manager for Z3 objects | Created via `with_z3_config` |
| `Solver` | Satisfiability checker | `Solver::new()` |
| `Int` | Integer values/variables | `Int::fresh_const("x")` |
| `Bool` | Boolean values/variables | `Bool::fresh_const("b")` |
| `Real` | Real number values | `Real::new_const("r")` |
| `Model` | Solution extraction | `solver.get_model()` |
| `Optimize` | Min/max optimization | `Optimize::new()` |

### Creating Variables and Constants

```rust
use z3::ast::Int;

// Named symbolic variable (unknown to solve for)
let x = Int::fresh_const("x");

// Literal constant
let five = Int::from_i64(5);

// Variable with specific name (for debugging)
let frame = Int::new_const("frame");
```

### Building Constraints

```rust
// Arithmetic
let sum = &x + &y;                    // Addition
let diff = &x - &y;                   // Subtraction
let prod = &x * 3;                    // Multiplication by constant
let remainder = &x % QUEUE_LENGTH;    // Modulo

// Comparisons (return Bool)
let ge = x.ge(0);                     // x >= 0
let lt = x.lt(&y);                    // x < y
let eq = x.eq(5);                     // x == 5
let ne = x.ne(&y);                    // x != y

// Boolean operations
let and = cond1 & cond2;              // AND
let or = cond1 | cond2;               // OR
let not = cond1.not();                // NOT
let xor = cond1.xor(&cond2);          // XOR
let implies = cond1.implies(&cond2);  // cond1 → cond2

// Conditional
let result = cond.ite(&then_val, &else_val);  // if-then-else
```

### Using the Solver

```rust
let cfg = Config::new();
z3::with_z3_config(&cfg, || {
    let solver = Solver::new();

    // Add constraints
    solver.assert(x.ge(0));
    solver.assert(x.lt(100));

    // Check satisfiability
    match solver.check() {
        SatResult::Sat => {
            // Solution exists
            let model = solver.get_model().unwrap();
            let value = model.eval(&x, true).unwrap();
        }
        SatResult::Unsat => {
            // No solution — constraints are contradictory
            // For proofs: this means the property HOLDS
        }
        SatResult::Unknown => {
            // Solver couldn't determine (timeout, etc.)
        }
    }
});
```

---

## Proof Patterns

### Pattern 1: Proof by Contradiction (Most Common)

To prove: "Property P always holds given preconditions"
Method: Assert preconditions AND negation of P; UNSAT means P always holds.

```rust
#[test]
fn z3_proof_index_always_valid() {
    let cfg = Config::new();
    z3::with_z3_config(&cfg, || {
        let solver = Solver::new();

        let frame = Int::fresh_const("frame");

        // PRECONDITIONS: What we know to be true
        solver.assert(frame.ge(0));  // Frame is non-negative

        // COMPUTATION: Model the algorithm
        let index = &frame % QUEUE_LENGTH;

        // PROPERTY (negated): Try to find a counterexample
        // Property: 0 <= index < QUEUE_LENGTH
        // Negation: index < 0 OR index >= QUEUE_LENGTH
        let invalid = index.lt(0) | index.ge(QUEUE_LENGTH);
        solver.assert(&invalid);

        // If UNSAT, no counterexample exists → property HOLDS
        assert_eq!(
            solver.check(),
            SatResult::Unsat,
            "Z3 should prove index is always valid"
        );
    });
}
```

### Pattern 2: Existence Proof

To prove: "There exists a valid state satisfying these conditions"
Method: Assert conditions; SAT means such a state exists.

```rust
#[test]
fn z3_proof_valid_rollback_exists() {
    let cfg = Config::new();
    z3::with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current = Int::fresh_const("current");
        let target = Int::fresh_const("target");

        // Constraints defining a valid rollback
        solver.assert(current.ge(1));           // At least frame 1
        solver.assert(target.ge(0));            // Target is valid
        solver.assert(target.lt(&current));     // Target is in past

        // SAT means valid rollback states exist
        assert_eq!(
            solver.check(),
            SatResult::Sat,
            "Z3 should find valid rollback scenarios"
        );
    });
}
```

### Pattern 3: Mutual Exclusion

To prove: "Conditions A and B cannot both be true"
Method: Assert both; UNSAT means they're mutually exclusive.

```rust
#[test]
fn z3_proof_skip_and_execute_exclusive() {
    let cfg = Config::new();
    z3::with_z3_config(&cfg, || {
        let solver = Solver::new();

        let frame_to_load = Int::fresh_const("frame_to_load");
        let current_frame = Int::fresh_const("current_frame");

        // Valid frames
        solver.assert(current_frame.ge(0));
        solver.assert(frame_to_load.ge(0));

        // Skip condition: frame_to_load >= current_frame
        let should_skip = frame_to_load.ge(&current_frame);

        // Execute condition: frame_to_load < current_frame
        let should_execute = frame_to_load.lt(&current_frame);

        // Try to find state where both are true
        solver.assert(&should_skip);
        solver.assert(&should_execute);

        assert_eq!(
            solver.check(),
            SatResult::Unsat,
            "Skip and execute are mutually exclusive"
        );
    });
}
```

### Pattern 4: Inductive Step (Sequence Preservation)

To prove: "Sequential operation preserves sequential property"

```rust
#[test]
fn z3_proof_sequential_preserved() {
    let cfg = Config::new();
    z3::with_z3_config(&cfg, || {
        let solver = Solver::new();

        let frame_n = Int::fresh_const("frame_n");
        let delay = Int::fresh_const("delay");

        solver.assert(frame_n.ge(0));
        solver.assert(delay.ge(0));
        solver.assert(delay.le(MAX_DELAY));

        // Sequential: frame_n+1 = frame_n + 1
        let frame_next = &frame_n + 1;

        // Positions with delay
        let pos_n = &frame_n + &delay;
        let pos_next = &frame_next + &delay;

        // Property: positions should be sequential (differ by 1)
        let sequential = pos_next.eq(&(&pos_n + 1));

        solver.assert(sequential.not());  // Negate property

        assert_eq!(solver.check(), SatResult::Unsat);
    });
}
```

### Pattern 5: State Machine Transitions

To prove: "Only valid state transitions are possible"

```rust
#[test]
fn z3_proof_valid_state_transitions() {
    let cfg = Config::new();
    z3::with_z3_config(&cfg, || {
        let solver = Solver::new();

        // States: 0=Pending, 1=InSync, 2=Desync
        let current = Int::fresh_const("current_state");
        let next = Int::fresh_const("next_state");
        let checksums_match = Int::fresh_const("match");  // 1=yes, 0=no

        // Valid states
        solver.assert(current.ge(0));
        solver.assert(current.le(2));
        solver.assert(checksums_match.ge(0));
        solver.assert(checksums_match.le(1));

        // Transition function
        // From Pending: InSync if match, Desync if not
        let from_pending = current.eq(0).implies(
            &checksums_match.eq(1).ite(&next.eq(1), &next.eq(2))
        );

        // From InSync: Stay InSync if match, Desync if not
        let from_insync = current.eq(1).implies(
            &checksums_match.eq(1).ite(&next.eq(1), &next.eq(2))
        );

        // From Desync: Always stay Desync (permanent)
        let from_desync = current.eq(2).implies(&next.eq(2));

        solver.assert(&from_pending);
        solver.assert(&from_insync);
        solver.assert(&from_desync);

        // Property: Can't go from Desync to InSync
        solver.assert(current.eq(2));
        solver.assert(next.eq(1));

        assert_eq!(solver.check(), SatResult::Unsat);
    });
}
```

---

## Common Idioms and Best Practices

### Absolute Value

```rust
// Z3 integers can be negative, so compute |a - b| carefully
let diff_ab = &a - &b;
let diff_ba = &b - &a;
let abs_diff = diff_ab.gt(0).ite(&diff_ab, &diff_ba);
```

### Distinctness (All Different)

```rust
use z3::ast::Ast;

// Efficient built-in for "all values different"
solver.assert(&Ast::distinct(&[&x, &y, &z, &w]));

// Manual pairwise (for custom logic)
for i in 0..n {
    for j in (i + 1)..n {
        solver.assert(&items[i].ne(&items[j]));
    }
}
```

### Bounded Domains

```rust
// Always constrain integers to their valid domain!
// Z3 integers span ALL integers including negative
solver.assert(frame.ge(0));
solver.assert(frame.lt(MAX_FRAME));

// For indices
solver.assert(index.ge(0));
solver.assert(index.lt(ARRAY_LENGTH));
```

### Iterating Solutions

```rust
// Get multiple solutions (use .take() to limit!)
for solution in solver.solutions([x, y], false).take(100) {
    let values: Vec<i64> = solution
        .iter()
        .filter_map(Int::as_i64)
        .collect();
    println!("Solution: {:?}", values);
}
```

### Push/Pop for Incremental Solving

```rust
// Test different scenarios without recreating solver
solver.push();  // Save state
solver.assert(&additional_constraint);
let result1 = solver.check();
solver.pop(1);  // Restore state

solver.push();
solver.assert(&different_constraint);
let result2 = solver.check();
solver.pop(1);
```

---

## Performance Tips

### 1. Use Bitvectors for Fixed-Width Arithmetic

```rust
use z3::ast::BV;

// When you know the bit width, BV can be faster
let frame = BV::new_const("frame", 32);  // 32-bit bitvector
let index = frame.bvurem(&BV::from_u64(128, 32));  // Unsigned modulo
```

### 2. Minimize Constraint Complexity

```rust
// ❌ Slow: Complex nested conditions
solver.assert(&a.implies(&b.implies(&c.implies(&d))));

// ✅ Faster: Flatten when possible
solver.assert(&(!a | !b | !c | d));  // Equivalent CNF
```

### 3. Use Incremental Solving

```rust
// ❌ Slow: Create new solver for each check
for scenario in scenarios {
    let solver = Solver::new();
    // rebuild all constraints...
}

// ✅ Faster: Reuse solver with push/pop
let solver = Solver::new();
setup_base_constraints(&solver);

for scenario in scenarios {
    solver.push();
    add_scenario_constraints(&solver, scenario);
    solver.check();
    solver.pop(1);
}
```

### 4. Binary Search for Optimization

```rust
// ❌ Slow: Direct optimization
let opt = Optimize::new();
opt.maximize(&value);

// ✅ Faster: Binary search with constraints
fn find_max(solver: &Solver, value: &Int, lo: i64, hi: i64) -> i64 {
    if lo >= hi { return lo; }
    let mid = (lo + hi + 1) / 2;

    solver.push();
    solver.assert(&value.ge(mid));
    let result = if solver.check() == SatResult::Sat {
        find_max(solver, value, mid, hi)
    } else {
        find_max(solver, value, lo, mid - 1)
    };
    solver.pop(1);
    result
}
```

---

## Documenting Z3 Proofs

### Required Documentation

```rust
/// Z3 Proof: [Property being proved]
///
/// Proves that [detailed description of what holds].
///
/// # Model
/// - `variable1`: Represents [what it models]
/// - `variable2`: Represents [what it models]
///
/// # Preconditions (Assumptions)
/// - [Precondition 1]
/// - [Precondition 2]
///
/// # Property
/// [What property is being verified]
///
/// # Relationship to Production Code
/// This models the logic in `module::function()`:
/// ```ignore
/// if condition {
///     // This is what we're proving is safe
/// }
/// ```
///
/// # Limitations
/// - [What the proof doesn't cover]
/// - [Gap between model and implementation]
#[test]
fn z3_proof_example() {
    // ...
}
```

### Tracking Model-Code Alignment

Use comments to link proofs to production code:

```rust
// In production code (src/module.rs):
/// Frame arithmetic for circular buffer.
///
/// # Formal Verification
/// - **Z3**: `z3_proof_circular_index_valid` in `tests/verification/z3.rs`
/// - **Kani**: `verify_circular_index` in `src/module.rs`
pub fn get_index(frame: Frame) -> usize {
    (frame.0 as usize) % QUEUE_LENGTH
}
```

---

## Common Pitfalls and Solutions

### Pitfall 1: Forgetting Domain Constraints

```rust
// ❌ Bug: Z3 integers include negatives!
let frame = Int::fresh_const("frame");
let index = &frame % 128;
// index could be negative for negative frames

// ✅ Fix: Always bound domains
solver.assert(frame.ge(0));
```

### Pitfall 2: Modulo with Negative Numbers

```rust
// ❌ Z3 modulo with negatives may not match Rust
// Z3: -1 % 128 could be -1 or 127 depending on theory

// ✅ Ensure inputs are non-negative before modulo
solver.assert(value.ge(0));
let index = &value % 128;
```

### Pitfall 3: Integer Overflow Not Modeled

```rust
// ❌ Z3 integers are unbounded — no overflow!
let x = Int::fresh_const("x");
let y = &x + 1;  // Never overflows in Z3

// ✅ Explicitly bound to model Rust overflow
solver.assert(x.lt(i32::MAX as i64));
```

### Pitfall 4: Confusing SAT/UNSAT for Properties

```rust
// For proving "P always holds":
// - Assert NOT(P)
// - UNSAT means P holds (no counterexample)
// - SAT means P doesn't hold (found counterexample)

// ❌ Wrong: Asserting P directly
solver.assert(&property);
assert_eq!(solver.check(), SatResult::Sat);  // Just says P is possible!

// ✅ Correct: Assert negation
solver.assert(&property.not());
assert_eq!(solver.check(), SatResult::Unsat);  // No counterexample → P holds
```

### Pitfall 5: Not Modeling Guards/Branches

```rust
// Production code has guards:
// if frame < 0 { return Err(...); }
// process(frame);

// ❌ Wrong: Not modeling the guard
let frame = Int::fresh_const("frame");
// Missing: this is only called when frame >= 0

// ✅ Correct: Include guard as precondition
solver.assert(frame.ge(0));  // Guard ensures this
```

---

## Z3 Proof Checklist

Before committing Z3 proofs:

- [ ] **Clear purpose**: Comment explains what property is being proved
- [ ] **Preconditions documented**: All assumptions about inputs are explicit
- [ ] **Correct proof direction**: Using UNSAT for universal proofs
- [ ] **Domain constraints**: All variables bounded appropriately
- [ ] **Model-code link**: Reference to production code being modeled
- [ ] **Limitation noted**: Document what the model doesn't cover
- [ ] **Test passes**: `cargo test --features z3-verification`

---

## Resources

| Resource | Link |
|----------|------|
| Z3 Rust Crate Docs | <https://docs.rs/z3/latest/z3/> |
| Z3 GitHub | <https://github.com/Z3Prover/z3> |
| Z3 Guide (interactive) | <https://microsoft.github.io/z3guide/> |
| SMT-LIB Standard | <http://smtlib.cs.uiowa.edu/> |
| Z3 Rust Guide PDF | <https://z3prover.github.io/papers/> |
| Z3 Playground | <https://jfmc.github.io/z3-play/> |

---

## Example: Complete Proof Module Structure

```rust
//! Z3 SMT Solver Verification Tests
//!
//! This module uses Z3 to formally verify safety properties of
//! core algorithms.

#![cfg(feature = "z3-verification")]
// Allow test-specific patterns
#![allow(clippy::unwrap_used, clippy::expect_used)]

use z3::ast::Int;
use z3::{with_z3_config, Config, SatResult, Solver};

/// Constants matching production configuration
const QUEUE_LENGTH: i64 = 128;
const MAX_PREDICTION: i64 = 8;

// =============================================================================
// Category: Frame Arithmetic Proofs
// =============================================================================

/// Z3 Proof: Circular buffer index is always valid
///
/// Proves: For any non-negative frame, frame % QUEUE_LENGTH ∈ [0, QUEUE_LENGTH)
#[test]
fn z3_proof_circular_index_valid() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let frame = Int::fresh_const("frame");

        // Precondition: frame is non-negative (enforced by Frame type)
        solver.assert(frame.ge(0));

        // Computation
        let index = &frame % QUEUE_LENGTH;

        // Property (negated): index is out of bounds
        let out_of_bounds = index.lt(0) | index.ge(QUEUE_LENGTH);
        solver.assert(&out_of_bounds);

        assert_eq!(
            solver.check(),
            SatResult::Unsat,
            "Z3 should prove circular index is always valid"
        );
    });
}
```

---

*This skill guide complements [kani-verification.md](kani-verification.md) for Rust code verification and [tla-plus-modeling.md](tla-plus-modeling.md) for protocol verification.*
