# Adapting Code for Miri — Agentic Workflow Guide

> **This document provides step-by-step guidance for agents adapting Rust code to pass Miri checks.**
> Use this when encountering Miri errors or preparing code for Miri verification.

## Quick Decision Tree

```
Miri reports an error
    │
    ├─► "tag does not exist in borrow stack"
    │       → Aliasing violation (see: Raw Pointer Patterns)
    │
    ├─► "dereferencing pointer failed: dangling"
    │       → Use-after-free (see: Memory Lifetime Issues)
    │
    ├─► "alignment X, but alignment Y required"
    │       → Alignment issue (see: Alignment Fixes)
    │
    ├─► "memory access to uninitialized"
    │       → Uninitialized memory (see: Initialization Patterns)
    │
    ├─► "can't call foreign function"
    │       → FFI not supported (see: Skipping Unsupported Code)
    │
    └─► "data race detected"
            → Synchronization issue (see: Concurrency Fixes)
```

---

## Common Miri Errors and Fixes

### 1. Stacked Borrows / Aliasing Violations

**Error pattern:**

```
error: Undefined Behavior: attempting a read/write access using <tag>
       at alloc123, but that tag does not exist in the borrow stack
```

**Root cause:** A pointer's permission was revoked before it was used.

#### Fix Strategy: Minimize Reference↔Pointer Crossings

**Problem code:**

```rust
// ❌ FAILS: Box access invalidates raw pointer
struct List<T> {
    head: Option<Box<Node<T>>>,
    tail: *mut Node<T>,
}

impl<T> List<T> {
    fn push(&mut self, elem: T) {
        let node = Box::new(Node { elem, next: ptr::null_mut() });
        let new_tail = &mut *node as *mut Node<T>;

        // This Box access invalidates new_tail!
        self.head = Some(node);  // ❌ Moves Box, invalidating new_tail
        self.tail = new_tail;    // ❌ Using invalidated pointer
    }
}
```

**Fixed code:**

```rust
// ✅ WORKS: Use raw pointers throughout
struct List<T> {
    head: *mut Node<T>,
    tail: *mut Node<T>,
}

impl<T> List<T> {
    fn push(&mut self, elem: T) {
        // Convert Box to raw immediately, never touch Box again
        let new = Box::into_raw(Box::new(Node { elem, next: ptr::null_mut() }));

        unsafe {
            if !self.tail.is_null() {
                (*self.tail).next = new;
            } else {
                self.head = new;
            }
            self.tail = new;
        }
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        // Clean up by converting back to Box
        let mut current = self.head;
        while !current.is_null() {
            let next = unsafe { (*current).next };
            unsafe { drop(Box::from_raw(current)); }
            current = next;
        }
    }
}
```

#### Alternative: Try Tree Borrows

If code seems correct but fails Stacked Borrows:

```bash
MIRIFLAGS="-Zmiri-tree-borrows" cargo miri test
```

Tree Borrows is more permissive. If code passes Tree Borrows but fails Stacked Borrows, document this:

```rust
// NOTE: This code requires Tree Borrows model.
// Run with: MIRIFLAGS="-Zmiri-tree-borrows" cargo miri test
// Stacked Borrows is overly conservative for this pattern because...
```

---

### 2. Use-After-Free / Dangling Pointers

**Error pattern:**

```
error: Undefined Behavior: dereferencing pointer failed:
       pointer is dangling (pointing to freed memory)
```

**Root cause:** Memory was freed while pointers to it still exist.

#### Fix Strategy: Track Lifetimes Carefully

**Problem code:**

```rust
// ❌ FAILS: Pointer outlives allocation
fn get_ptr() -> *const i32 {
    let x = 42;
    &x as *const i32  // Returns pointer to stack-local!
}

fn main() {
    let ptr = get_ptr();
    unsafe { *ptr }  // ❌ x is gone
}
```

**Fixed code:**

```rust
// ✅ WORKS: Use proper ownership
fn get_value() -> Box<i32> {
    Box::new(42)
}

// Or if you need raw pointers, ensure proper lifetime management
struct Owner {
    data: Box<i32>,
}

impl Owner {
    fn get_ptr(&self) -> *const i32 {
        &*self.data as *const i32
    }
}
```

#### Debug with Allocation Tracking

```bash
# Find where allocation was freed
MIRIFLAGS="-Zmiri-track-alloc-id=123" cargo miri test
```

---

### 3. Alignment Issues

**Error pattern:**

```
error: Undefined Behavior: accessing memory with alignment X,
       but alignment Y is required
```

**Root cause:** Pointer cast violates alignment requirements.

#### Fix Strategy: Ensure Proper Alignment

**Problem code:**

```rust
// ❌ FAILS: u8 slice may not be aligned for u32
fn read_u32(bytes: &[u8]) -> u32 {
    unsafe {
        *(bytes.as_ptr() as *const u32)  // Alignment violation!
    }
}
```

**Fixed code:**

```rust
// ✅ WORKS: Use byte operations
fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

// Or use proper aligned read
fn read_u32_aligned(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 4 {
        return None;
    }
    let ptr = bytes.as_ptr();
    if ptr.align_offset(std::mem::align_of::<u32>()) != 0 {
        // Not aligned, use byte-by-byte read
        Some(u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    } else {
        // Aligned, safe to cast
        Some(unsafe { *(ptr as *const u32) })
    }
}
```

---

### 4. Uninitialized Memory

**Error pattern:**

```
error: Undefined Behavior: using uninitialized data
```

**Root cause:** Reading memory before it's been written.

#### Fix Strategy: Always Initialize

**Problem code:**

```rust
// ❌ FAILS: Reading uninitialized memory
fn get_array() -> [u32; 4] {
    let mut arr: [u32; 4];
    unsafe { arr.as_mut_ptr().write(1); }  // Only writes first element
    arr  // ❌ Other elements uninitialized
}
```

**Fixed code:**

```rust
// ✅ WORKS: Initialize all elements
fn get_array() -> [u32; 4] {
    [0u32; 4]  // Default initialize
}

// Or use MaybeUninit properly
fn get_array_uninit() -> [u32; 4] {
    use std::mem::MaybeUninit;

    let mut arr: [MaybeUninit<u32>; 4] = unsafe { MaybeUninit::uninit().assume_init() };
    for (i, elem) in arr.iter_mut().enumerate() {
        elem.write(i as u32);
    }
    unsafe { std::mem::transmute(arr) }
}
```

---

### 5. FFI / Unsupported Operations

**Error pattern:**

```
error: unsupported operation: can't call foreign function: function_name
```

**Root cause:** Miri cannot interpret C code.

#### Fix Strategy: Skip Under Miri

```rust
#[test]
#[cfg_attr(miri, ignore)]
fn test_with_ffi() {
    // This test uses FFI that Miri can't run
    external_c_function();
}

// For conditional compilation in production code
fn do_operation() {
    if cfg!(miri) {
        // Miri-compatible fallback
        software_implementation()
    } else {
        // Native FFI
        ffi_implementation()
    }
}
```

---

### 6. Data Races

**Error pattern:**

```
error: Undefined Behavior: Data race detected between (1) Read on thread `main`
       and (2) Write on thread `<unnamed>`
```

**Root cause:** Unsynchronized access to shared mutable data.

#### Fix Strategy: Add Proper Synchronization

**Problem code:**

```rust
// ❌ FAILS: No synchronization
use std::thread;

static mut COUNTER: u32 = 0;

fn main() {
    let t = thread::spawn(|| {
        unsafe { COUNTER += 1; }  // ❌ Data race
    });
    unsafe { COUNTER += 1; }      // ❌ Data race
    t.join().unwrap();
}
```

**Fixed code:**

```rust
// ✅ WORKS: Use atomics or locks
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn main() {
    let t = thread::spawn(|| {
        COUNTER.fetch_add(1, Ordering::SeqCst);
    });
    COUNTER.fetch_add(1, Ordering::SeqCst);
    t.join().unwrap();
}
```

---

## Adapting Tests for Miri

### Reduce Iteration Counts

```rust
fn test_iterations() -> usize {
    if cfg!(miri) { 10 } else { 10_000 }
}

#[test]
fn stress_test() {
    for _ in 0..test_iterations() {
        // Test logic
    }
}
```

### Skip Heavy Tests

```rust
#[test]
#[cfg_attr(miri, ignore)]
fn expensive_integration_test() {
    // Too slow for Miri
}
```

### Provide Miri-Specific Implementations

```rust
fn get_random_seed() -> u64 {
    if cfg!(miri) {
        // Deterministic seed for Miri
        12345
    } else {
        // Real random for native
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}
```

---

## Debugging Workflow

### Step 1: Get Full Backtrace

```bash
RUST_BACKTRACE=1 MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-backtrace=full" cargo miri test
```

### Step 2: Track Specific Issues

```bash
# Track an allocation
MIRIFLAGS="-Zmiri-track-alloc-id=<id>" cargo miri test

# Track a pointer tag
MIRIFLAGS="-Zmiri-track-pointer-tag=<tag>" cargo miri test
```

### Step 3: Try Permissive Options

```bash
# Try Tree Borrows if Stacked Borrows is too strict
MIRIFLAGS="-Zmiri-tree-borrows" cargo miri test

# Silence provenance warnings (while investigating)
MIRIFLAGS="-Zmiri-permissive-provenance" cargo miri test
```

### Step 4: Test Multiple Executions

```bash
# Find non-deterministic issues
MIRIFLAGS="-Zmiri-many-seeds=0..64" cargo miri test

# Or loop through seeds
for SEED in $(seq 0 63); do
    MIRIFLAGS="-Zmiri-seed=$SEED" cargo miri test || break
done
```

---

## Checklist for Miri Compatibility

### Before Submitting Code

- [ ] `cargo miri test` passes (at least for the changed code)
- [ ] Heavy tests marked with `#[cfg_attr(miri, ignore)]`
- [ ] Iteration counts reduced under `cfg!(miri)`
- [ ] FFI code has Miri-compatible fallbacks or is skipped
- [ ] No `Box` mixed with raw pointers to its contents
- [ ] All memory properly initialized before use
- [ ] Alignment requirements respected for pointer casts

### CI Configuration

```yaml
miri:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - run: |
        rustup toolchain install nightly --component miri
        rustup override set nightly
        cargo miri setup
    - run: cargo miri test --lib
      env:
        MIRIFLAGS: "-Zmiri-disable-isolation"
```

---

## Quick Reference: Miri Flags

| Flag | Use When |
|------|----------|
| `-Zmiri-disable-isolation` | Tests need env vars, filesystem, etc. |
| `-Zmiri-tree-borrows` | Stacked Borrows seems overly strict |
| `-Zmiri-many-seeds=0..N` | Testing concurrent code |
| `-Zmiri-seed=X` | Reproducing a specific failure |
| `-Zmiri-track-alloc-id=X` | Debugging allocation issues |
| `-Zmiri-track-pointer-tag=X` | Debugging aliasing issues |
| `-Zmiri-strict-provenance` | Catching int↔pointer issues |
| `-Zmiri-symbolic-alignment-check` | Finding alignment bugs |

---

*This guide is designed for agentic workflows. Follow the decision tree, apply the appropriate fix pattern, and verify with `cargo miri test`.*
