<!-- CATEGORY: Formal Verification -->
<!-- WHEN: Writing TLA+ specs, Z3 SMT proofs, formal modeling -->

# Formal Verification: TLA+ and Z3

## TLA+ Quick Start

TLA+ specs are abstract models of behavior, NOT translations of code. Model what could go wrong subtly (concurrency, coordination, protocol logic). Skip implementation details (serialization, checksums, performance).

### Minimal Specification Template

```tla
---- MODULE ModuleName ----
EXTENDS Integers, Sequences, FiniteSets

CONSTANTS MAX_VALUE, NUM_PROCESSES

VARIABLES state, pc
vars == <<state, pc>>

TypeInvariant ==
    /\ state \in [1..NUM_PROCESSES -> 0..MAX_VALUE]
    /\ pc \in [1..NUM_PROCESSES -> {"init", "running", "done"}]

Init ==
    /\ state = [p \in 1..NUM_PROCESSES |-> 0]
    /\ pc = [p \in 1..NUM_PROCESSES |-> "init"]

DoWork(p) ==
    /\ pc[p] = "running"
    /\ state[p] < MAX_VALUE
    /\ state' = [state EXCEPT ![p] = state[p] + 1]
    /\ UNCHANGED pc

Next == \E p \in 1..NUM_PROCESSES: \/ Start(p) \/ DoWork(p) \/ Finish(p)

Spec == Init /\ [][Next]_vars /\ Fairness
SafetyInvariant == \A p \in 1..NUM_PROCESSES: state[p] <= MAX_VALUE
====
```

### What to Model vs Skip

| Model | Skip |
|-------|------|
| State machine transitions | Byte-level serialization |
| Concurrency/interleavings | Checksum algorithms |
| Coordination logic | Performance optimizations |
| Message ordering | Memory management |
| Failure scenarios | Logging/telemetry |

### Common Patterns

**State Machines:**
```tla
States == {"Initializing", "Synchronizing", "Running", "Disconnected"}
StartSync == /\ state = "Initializing" /\ state' = "Synchronizing"
```

**Nondeterministic Failures:**
```tla
SendMessage(msg) ==
    \/ network' = network \union {msg}     \* Success
    \/ UNCHANGED network                    \* Loss
```

**Rollback:**
```tla
StartRollback(targetFrame) ==
    /\ targetFrame < currentFrame
    /\ targetFrame >= currentFrame - MAX_PREDICTION
    /\ savedStates[targetFrame] /= <<>>
    /\ inRollback' = TRUE
    /\ rollbackTarget' = targetFrame
    /\ UNCHANGED <<currentFrame, savedStates>>
```

### Safety vs Liveness

```tla
\* Safety: "nothing bad happens" -- checked at every state
RollbackBounded == inRollback => (currentFrame - rollbackTarget <= MAX_PREDICTION)

\* Liveness: "something good eventually happens" -- requires fairness
EventuallySynced == <>(state = "Running")
RollbackCompletes == inRollback ~> ~inRollback
```

Liveness checking is much slower. Run safety first.

### Rust to TLA+ Translation

| Rust | TLA+ |
|------|------|
| `enum { A, B }` | `{"A", "B"}` |
| `struct { field: T }` | `[field: T]` |
| `Vec<T>` | `Seq(T)` |
| `HashMap<K, V>` | `[K -> V]` |
| `Option<T>` | `T \union {NULL}` |
| `match` | `CASE` or `IF/THEN/ELSE` |
| `thread::spawn` | Process / parallel composition |
| `Mutex<T>` | Lock variable + mutual exclusion invariant |

### Common Pitfalls

- **Forgetting UNCHANGED**: Always specify what doesn't change
- **Modeling too much detail**: Abstract bytes to messages
- **Assuming atomicity**: Model read-modify-write as separate steps
- **Wrong quantifier**: Use `=>` with `\A`, use `/\` with `\E`
- **Unbounded state**: Always add `CONSTANTS` bounds

### Model Checking

```bash
java -jar tla2tools.jar -workers auto -config Spec.cfg Spec.tla
java -Xmx8G -jar tla2tools.jar -workers 8 Spec.tla  # Large models
```

Start with tiny constants. Use `SYMMETRY` for interchangeable values.

### Project Structure

```
specs/tla/
  Rollback.tla + Rollback.cfg
  InputQueue.tla + InputQueue.cfg
scripts/verify-tla.sh
```

---

## Z3 Quick Start

Z3 proves properties about abstract mathematical models, not code. Method: assert preconditions AND negation of property; UNSAT = property holds.

### Setup

```toml
[features]
z3-verification = ["dep:z3"]
z3-verification-bundled = ["z3-verification", "z3?/bundled"]

[dependencies]
z3 = { version = "0.19", optional = true }
```

```bash
# System install (fast): apt-get install libz3-dev / brew install z3
cargo test --features z3-verification
```

### Proof by Contradiction (Most Common Pattern)

```rust
#[test]
fn z3_proof_circular_index_valid() {
    let cfg = Config::new();
    z3::with_z3_config(&cfg, || {
        let solver = Solver::new();
        let frame = Int::fresh_const("frame");

        // Precondition
        solver.assert(frame.ge(0));

        // Computation
        let index = &frame % QUEUE_LENGTH;

        // Property (negated): try to find counterexample
        let out_of_bounds = index.lt(0) | index.ge(QUEUE_LENGTH);
        solver.assert(&out_of_bounds);

        // UNSAT = no counterexample = property holds
        assert_eq!(solver.check(), SatResult::Unsat);
    });
}
```

### Other Proof Patterns

**Existence Proof** (SAT = valid state exists):
```rust
solver.assert(target.lt(&current));
assert_eq!(solver.check(), SatResult::Sat);
```

**Mutual Exclusion** (UNSAT = can't both be true):
```rust
solver.assert(&should_skip);    // frame >= current
solver.assert(&should_execute); // frame < current
assert_eq!(solver.check(), SatResult::Unsat);
```

### Z3 API Essentials

```rust
use z3::ast::Int;

let x = Int::fresh_const("x");           // Symbolic variable
let five = Int::from_i64(5);              // Constant
let sum = &x + &y;                        // Arithmetic
let ge = x.ge(0);                         // Comparison -> Bool
let and = cond1 & cond2;                  // Boolean ops
let result = cond.ite(&then_val, &else_val); // if-then-else

solver.push();                            // Save state
solver.assert(&constraint);
solver.check();
solver.pop(1);                            // Restore state
```

### Common Pitfalls

- **Forgetting domain constraints**: Z3 integers include negatives -- always bound
- **Modulo with negatives**: Ensure inputs are non-negative before `%`
- **No overflow modeling**: Z3 integers are unbounded -- explicitly bound to model Rust overflow
- **Wrong SAT/UNSAT interpretation**: For "P always holds", assert NOT(P) and expect UNSAT

### Proof Documentation Template

```rust
/// Z3 Proof: [Property]
///
/// # Model
/// - `frame`: Non-negative frame number
///
/// # Preconditions
/// - frame >= 0
///
/// # Property
/// frame % QUEUE_LENGTH is always in [0, QUEUE_LENGTH)
///
/// # Relationship to Production Code
/// Models `get_index()` in `src/module.rs`
///
/// # Limitations
/// - Does not model overflow (Z3 integers are unbounded)
```

### Performance Tips

- Use bitvectors (`BV`) for fixed-width arithmetic
- Use incremental solving (`push/pop`) instead of creating new solvers
- Flatten nested implications to CNF when possible

## Verification Tool Comparison

| Tool | Verifies | Best For |
|------|----------|----------|
| Unit Tests | Specific cases | Happy path |
| Property Tests | Random samples | Edge cases |
| Kani | Actual Rust code (bounded) | Arithmetic, panics |
| TLA+ | Abstract protocol models | Distributed systems |
| Z3 | Mathematical models | Algorithm properties |
| Loom | Thread interleavings | Lock-free structures |
| Miri | Undefined behavior | Unsafe code |
