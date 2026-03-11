<!-- CATEGORY: Formal Verification -->
<!-- WHEN: Writing Kani proofs, debugging verification failures, unwind configuration -->

# Kani Verification

Kani is a bit-precise model checker that exhaustively verifies properties about Rust code -- unlike testing, it explores ALL possible inputs within bounds.

## CRITICAL: Loop Unwinding (The #1 CI Failure Cause)

**ALWAYS add `#[kani::unwind(N)]` for ANY loop with symbolic bounds**, where N = max_iterations + 1.

```rust
// WILL FAIL CI -- missing unwind attribute
#[kani::proof]
fn verify_loop_broken() {
    let n: usize = kani::any();
    kani::assume(n <= 10);
    for _ in 0..n { /* ... */ }  // Hangs or times out
}

// CORRECT -- always specify unwind bound
#[kani::proof]
#[kani::unwind(11)]  // 10 max iterations + 1
fn verify_loop_correct() {
    let n: usize = kani::any();
    kani::assume(n <= 10);
    for _ in 0..n { /* ... */ }
}
```

### Common Loop Sources (Check All)

| Loop Type | Requires Unwind? |
|-----------|------------------|
| `for i in 0..n` where n is symbolic | **YES** |
| `while` with symbolic condition | **YES** |
| `.iter()` on symbolic-length collection | **YES** |
| `loop` with symbolic break | **YES** |
| Fixed iteration count (`for i in 0..10`) | No |
| Iterator over fixed array (`[1,2,3].iter()`) | No |

### CI Quick Mode (`--default-unwind 8`)

CI uses `--quick` which sets `--default-unwind 8`. Proofs without explicit `#[kani::unwind(N)]` that iterate over arrays >8 elements will **fail**.

| Data Structure | Max Iterations | Unwind Bound (N) |
|----------------|----------------|-------------------|
| `[u8; 4]` | 4 | 5 |
| `[u32; 8]` | 8 | 9 |
| `[u64; 16]` | 16 | 17 |
| Nested loops (3 x 4) | 4 (outer) | 5 |
| `kani::assume(n <= 10)` | 10 | 11 |

For nested loops, set unwind to `max(all_loop_bounds) + 1`.

### Simple Proofs Without Loops

```rust
// kani::no-unwind-needed
#[kani::proof]
fn proof_simple_property() {
    let x: u32 = kani::any();
    assert!(x.wrapping_add(0) == x);
}
```

## Proof Harness Template

```rust
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    #[kani::unwind(N)]  // REQUIRED for loops with symbolic bounds
    fn verify_function_does_not_panic() {
        let x: u32 = kani::any();
        let y: u32 = kani::any();
        kani::assume(y != 0);  // Constrain inputs

        let result = my_function(x, y);

        // Kani auto-checks: panics, overflow, out-of-bounds, div-by-zero
        assert!(result <= x);  // Additional property
    }
}
```

## Essential API

| Function/Attribute | Purpose |
|-------------------|---------|
| `kani::any::<T>()` | Generate symbolic value (ALL possible values) |
| `kani::any_where(\|v\| pred)` | Constrained symbolic value |
| `kani::assume(cond)` | Narrow state space |
| `#[kani::proof]` | Mark as proof harness |
| `#[kani::unwind(N)]` | Set loop unwinding bound |
| `#[kani::stub(orig, repl)]` | Replace function with stub |
| `#[kani::solver(cadical)]` | Choose SAT solver |
| `#[kani::should_panic]` | Expect harness to panic |

## Deriving Arbitrary

```rust
#[derive(kani::Arbitrary)]
pub struct PlayerInput { pub frame: u32, pub buttons: u16, pub player_id: u8 }

#[derive(kani::Arbitrary)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected { latency_ms: u32 },
}

// Manual implementation for types with invariants
impl kani::Arbitrary for Frame {
    fn any() -> Self {
        let value: u32 = kani::any();
        kani::assume(value <= Frame::MAX_VALUE);
        Frame(value)
    }
}
```

## Verification Patterns

### No Panics (Zero-Panic Policy)
```rust
#[kani::proof]
fn verify_no_panics() {
    let input: Input = kani::any();
    let result = process_input(input);
    assert!(result.is_ok() || matches!(result, Err(FortressError::_)));
}
```

### Arithmetic Safety
```rust
#[kani::proof]
fn verify_frame_arithmetic() {
    let frame: u32 = kani::any();
    let delta: u32 = kani::any();
    if let Some(result) = frame.checked_add(delta) {
        assert!(result >= frame);
    }
}
```

### State Machine Transitions
```rust
#[kani::proof]
fn verify_valid_transitions() {
    let current: State = kani::any();
    let event: Event = kani::any();
    let next = transition(current, event);
    kani::assume(matches!(current, State::Stopped));
    assert!(!matches!(next, State::Running));
}
```

### Input Validation
```rust
#[kani::proof]
fn verify_input_validation_complete() {
    let raw: RawInput = kani::any();
    match validate_input(raw) {
        Ok(v) => { assert!(v.frame <= MAX_FRAME); assert!(v.player_id < MAX_PLAYERS); }
        Err(_) => { /* Rejected -- ok */ }
    }
}
```

## Stubbing Functions

```rust
#[cfg(kani)]
fn get_timestamp_stub() -> u64 { kani::any() }

#[kani::proof]
#[kani::stub(get_timestamp, get_timestamp_stub)]
fn verify_with_any_timestamp() {
    let ts = get_timestamp();
    // ts is now symbolic
}
```

## Performance Tips

- **Start small**: Use `[u8; 4]` not `[u8; 1024]`
- **Constrain early**: Use `kani::any_where()` instead of late `kani::assume()`
- **Multiple focused harnesses** instead of one huge harness
- **Try different solvers**: `cadical` (default), `kissat`, `minisat`

## CI Integration (CRITICAL)

**When adding new `#[kani::proof]` functions, you MUST add them to tier lists in `scripts/verify-kani.sh`.**

### Tier System

| Tier | Speed | Use Case |
|------|-------|----------|
| Tier 1 | <30s | Simple property checks |
| Tier 2 | 30s-2min | Moderate complexity |
| Tier 3 | >2min | Complex state verification |

### Step-by-Step Process

1. Write proof with `#[kani::unwind(N)]`
2. Run locally: `cargo kani --harness proof_name`
3. Add to tier list in `scripts/verify-kani.sh`:
   ```bash
   TIER1_PROOFS=( ... "proof_my_invariant" )
   ```
4. Validate: `./scripts/check-kani-coverage.sh`
   Must show: "SUCCESS: All N Kani proofs are covered in tier lists."

CI `kani-coverage-check` job scans source for all `#[kani::proof]` and **fails** if any are missing from tiers.

### Local Verification

```bash
cargo kani --harness verify_my_function    # Run specific proof
cargo kani                                  # Run all proofs
cargo kani --harness proof_name --default-unwind 8  # Simulate CI
```

## Common Timeout Causes

### `format!()` in macros creates explosive CBMC state space

The `format!()` macro generates complex string-formatting code that CBMC must
model symbolically. When a macro like `report_violation!` calls `format!()`,
and that macro is used inside hot paths (e.g., `safe_frame_add!`,
`safe_frame_sub!`), each call site multiplies the state space. This is the
most common cause of Kani proof timeouts after missing unwind bounds.

**Standard fix:** gate expensive logging/telemetry code with
`#[cfg(not(kani))]` so it becomes a no-op during verification.

```rust
// The report_violation! macro is ALREADY gated:
//   #[cfg(not(kani))] { /* actual reporting */ }
// So callers do NOT need to add their own #[cfg(not(kani))] guards.
```

### Macros already gated in this project

| Macro | Status | Notes |
|-------|--------|-------|
| `report_violation!` | No-op under `cfg(kani)` | All arms gated internally |
| `safe_frame_add!` | Safe to use in proofs | Calls `report_violation!` (no-op) |
| `safe_frame_sub!` | Safe to use in proofs | Calls `report_violation!` (no-op) |

### General principle

Kani proofs verify **correctness properties** (no panics, arithmetic safety,
state machine invariants), not logging or telemetry behavior. If a proof
times out, check whether the code under verification calls macros that expand
to `format!()`, `tracing::*`, or other string-heavy infrastructure, and gate
those with `#[cfg(not(kani))]`.

## Common Pitfalls

### Proof assertions don't match implementation (CRITICAL)

```rust
// WRONG: Asserts something the code doesn't do
#[kani::proof]
fn proof_default() {
    let s = ConnectionStatus::default();
    assert!(matches!(s, ConnectionStatus::Connected));  // Code returns Disconnected!
}

// CORRECT: Matches actual implementation
#[kani::proof]
fn proof_default() {
    let s = ConnectionStatus::default();
    assert!(matches!(s, ConnectionStatus::Disconnected));
}
```

Prevention: Read the implementation first. Document the property being verified.

### State explosion
Use `[u64; 2]` not `[u64; 100]`. Constrain inputs aggressively.

### Ignoring counterexamples
Don't add `kani::assume()` just to make proofs pass. Fix the actual code.

### Concrete values in proofs
Use `kani::any()` (symbolic), not literal values like `let x: u32 = 42`.

## Checklist for New Kani Proofs

- [ ] Identify ALL loops (including nested/called functions) and add `#[kani::unwind(N)]` (N = max + 1)
- [ ] Run locally (`cargo kani --harness proof_name`) and verify it completes in under 5 minutes
- [ ] Add to tier list in `scripts/verify-kani.sh` and run `./scripts/check-kani-coverage.sh`
- [ ] Proof assertions match actual implementation behavior
