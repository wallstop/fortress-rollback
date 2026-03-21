<!-- CATEGORY: Formal Verification -->
<!-- WHEN: Running Miri, debugging undefined behavior, adapting code for Miri -->

# Miri Verification

Miri interprets Rust's MIR to detect undefined behavior at runtime, tracking pointer provenance and aliasing rules.

## What Miri Detects

| Category | Examples |
|----------|----------|
| Memory access | Out-of-bounds, use-after-free |
| Uninitialized data | Reading uninitialized memory |
| Alignment | Misaligned pointer dereferences |
| Type invariants | Invalid `bool`, invalid enum discriminant, null references |
| Intrinsic violations | `unreachable_unchecked` reached, overlapping `copy_nonoverlapping` |
| Data races | Unsynchronized shared access |
| Memory leaks | Unreachable allocations at exit |

### What Miri Does NOT Detect

| Limitation | Mitigation |
|------------|------------|
| Non-deterministic bugs | Use `-Zmiri-many-seeds` |
| All thread interleavings | Use Loom |
| FFI/foreign functions | Mock FFI, use sanitizers |
| Future UB / incomplete weak memory | Stay updated, use Loom for atomics |

## Quick Decision Tree

```
Miri error
  |-- "tag does not exist in borrow stack" --> Aliasing violation
  |-- "dereferencing pointer failed: dangling" --> Use-after-free
  |-- "alignment X, but Y required" --> Alignment issue
  |-- "memory access to uninitialized" --> Uninitialized memory
  |-- "can't call foreign function" --> FFI not supported
  |-- "data race detected" --> Synchronization issue
```

## Commands

```bash
cargo miri test                                          # Run all tests
cargo miri test test_name                                # Specific test
MIRIFLAGS="-Zmiri-disable-isolation" cargo miri test     # Access host resources
MIRIFLAGS="-Zmiri-tree-borrows" cargo miri test          # Tree Borrows model
MIRIFLAGS="-Zmiri-many-seeds=0..16" cargo miri test      # Multiple executions
MIRIFLAGS="-Zmiri-strict-provenance" cargo miri test     # Strict provenance
cargo miri test --target s390x-unknown-linux-gnu          # Big-endian
cargo miri test --target i686-unknown-linux-gnu           # 32-bit
cargo miri clean                                          # Clean artifacts
```

## Essential Flags

| Flag | Purpose |
|------|---------|
| `-Zmiri-disable-isolation` | Access host filesystem/env |
| `-Zmiri-tree-borrows` | Use Tree Borrows (more permissive) |
| `-Zmiri-many-seeds=0..N` | Test N random executions |
| `-Zmiri-seed=X` | Reproduce specific failure |
| `-Zmiri-strict-provenance` | Strict pointer provenance |
| `-Zmiri-symbolic-alignment-check` | Stricter alignment |
| `-Zmiri-track-alloc-id=X` | Debug allocation lifecycle |
| `-Zmiri-track-pointer-tag=X` | Debug pointer aliasing |
| `-Zmiri-backtrace=full` | Full backtraces |

## Stacked Borrows vs Tree Borrows

**Stacked Borrows** (default): Strict stack-based model. Creating `&mut` immediately invalidates other pointers.

**Tree Borrows** (`-Zmiri-tree-borrows`): More permissive tree model. `&mut` starts "Reserved", becomes "Active" on first write.

Use Tree Borrows when code seems correct but fails Stacked Borrows. Document the deviation.

## Fix Patterns

### Aliasing Violations (Most Common)

```rust
// FAILS: Box access invalidates raw pointer
struct List<T> {
    head: Option<Box<Node<T>>>,
    tail: *mut Node<T>,
}

// WORKS: All raw pointers, no Box mixing
struct List<T> {
    head: *mut Node<T>,
    tail: *mut Node<T>,
}

impl<T> List<T> {
    fn push(&mut self, elem: T) {
        let new = Box::into_raw(Box::new(Node { elem, next: ptr::null_mut() }));
        unsafe {
            if !self.tail.is_null() { (*self.tail).next = new; }
            else { self.head = new; }
            self.tail = new;
        }
    }
}
// Remember Box::from_raw in Drop!
```

**Golden rule:** Once you start using raw pointers, stay with raw pointers.

### Alignment Issues

```rust
// FAILS: u8 slice may not be aligned for u32
fn read_u32(bytes: &[u8]) -> u32 {
    unsafe { *(bytes.as_ptr() as *const u32) }
}

// WORKS: Use byte operations
fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}
```

### Uninitialized Memory

```rust
// FAILS: Only writes first element
let mut arr: [u32; 4];
unsafe { arr.as_mut_ptr().write(1); }
arr  // Other elements uninitialized

// WORKS: Initialize all elements
let arr = [0u32; 4];
```

### Data Races

```rust
// FAILS: No synchronization
static mut COUNTER: u32 = 0;

// WORKS: Use atomics
static COUNTER: AtomicU32 = AtomicU32::new(0);
```

## Adapting Tests

### Reduce Iteration Counts (Miri is 1000-7000x slower)

```rust
fn iteration_count() -> usize {
    if cfg!(miri) { 10 } else { 10_000 }
}
```

### Skip Unsupported Tests

```rust
#[test]
#[cfg_attr(miri, ignore)]
fn uses_networking() { /* Miri can't run FFI/network */ }
```

### Miri-Specific Fallbacks

```rust
fn get_seed() -> u64 {
    if cfg!(miri) { 12345 } else { real_random() }
}
```

## Debugging Workflow

```bash
# 1. Full backtrace
RUST_BACKTRACE=1 MIRIFLAGS="-Zmiri-backtrace=full" cargo miri test

# 2. Track specific allocation/pointer
MIRIFLAGS="-Zmiri-track-alloc-id=456" cargo miri test
MIRIFLAGS="-Zmiri-track-pointer-tag=1234" cargo miri test

# 3. Try permissive options
MIRIFLAGS="-Zmiri-tree-borrows" cargo miri test

# 4. Multiple seeds for non-deterministic issues
MIRIFLAGS="-Zmiri-many-seeds=0..64" cargo miri test
```

## CI Integration

```yaml
miri:
  name: Miri UB Check (${{ matrix.os }})
  runs-on: ${{ matrix.os }}
  timeout-minutes: 30
  strategy:
    fail-fast: false
    matrix:
      os: [ubuntu-latest, windows-latest, macos-latest]
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@master
      with: { toolchain: nightly, components: miri }
    - uses: actions/cache@v5
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
        # Avoid caching target/ for Miri (stale artifacts across nightlies)
        key: miri-${{ matrix.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    - run: cargo miri setup
    - run: cargo miri test --lib
      env:
        MIRIFLAGS: "-Zmiri-disable-isolation"
```

Cross-platform because UB detection catches platform-specific layout/alignment issues.

## Checklist

- [ ] `cargo miri test` passes for changed code
- [ ] Heavy tests marked `#[cfg_attr(miri, ignore)]`
- [ ] Iteration counts reduced under `cfg!(miri)`
- [ ] FFI code has Miri fallbacks or is skipped
- [ ] No `Box` mixed with raw pointers to its contents
- [ ] All memory initialized before use
- [ ] Alignment respected for pointer casts
