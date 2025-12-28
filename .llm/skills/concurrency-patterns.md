# Concurrent Rust Patterns — Thread Safety Without Tears

> **This document provides guidance for writing correct concurrent Rust code.**
> These patterns complement loom testing and help prevent common concurrency bugs.

## Core Philosophy

**Concurrent code should be correct by construction, not by luck.** Achieve this through:

1. **Minimize shared mutable state** — Less sharing = fewer bugs
2. **Use appropriate synchronization** — Choose the right primitive for the job
3. **Prefer message passing** — Channels over shared memory when practical
4. **Test deterministically** — Use loom for critical concurrent code
5. **Document synchronization intent** — Make the contract explicit

---

## Choosing Synchronization Primitives

### Decision Tree

```
Is the data accessed from multiple threads?
├── No → Use regular types, no synchronization needed
└── Yes → Is it read-only after initialization?
    ├── Yes → Use Arc<T> (no interior mutability needed)
    └── No → What's the access pattern?
        ├── Mostly reads, rare writes → RwLock<T>
        ├── Frequent writes, short critical sections → Mutex<T>
        ├── Single atomic value → Atomic* types
        └── Producer-consumer pattern → Channels (mpsc, crossbeam)
```

### Primitive Comparison

| Primitive | Use When | Avoid When |
|-----------|----------|------------|
| `Mutex<T>` | Short critical sections, frequent writes | Long-held locks, read-heavy workloads |
| `RwLock<T>` | Read-heavy, occasional writes | Write-heavy (readers starve writers) |
| `Atomic*` | Single values, lock-free algorithms | Complex multi-field updates |
| `Arc<T>` | Shared ownership, immutable data | Mutable data (use `Arc<Mutex<T>>`) |
| `mpsc::channel` | Producer-consumer, message passing | Low-latency requirements |
| `crossbeam::channel` | High-performance channels | Simple use cases (more dependencies) |

---

## Common Patterns

### Pattern 1: Shared State with Mutex

```rust
use std::sync::{Arc, Mutex};
use std::thread;

// ✅ CORRECT: Minimal time holding lock
fn update_counter(counter: Arc<Mutex<u64>>, increment: u64) {
    let mut guard = counter.lock().unwrap();
    *guard += increment;
    // Lock released here when guard goes out of scope
}

// ❌ DANGEROUS: Holding lock during I/O or long operations
fn bad_update(counter: Arc<Mutex<u64>>) {
    let guard = counter.lock().unwrap();
    expensive_network_call();  // Other threads blocked!
    drop(guard);
}

// ✅ BETTER: Clone data, release lock, then do expensive work
fn good_update(counter: Arc<Mutex<u64>>) -> u64 {
    let value = {
        let guard = counter.lock().unwrap();
        *guard  // Copy the value
    };  // Lock released here
    
    expensive_network_call();  // Other threads can proceed
    value
}
```

### Pattern 2: RwLock for Read-Heavy Workloads

```rust
use std::sync::{Arc, RwLock};

struct Cache {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl Cache {
    // Multiple threads can read simultaneously
    fn get(&self, key: &str) -> Option<String> {
        let guard = self.data.read().unwrap();
        guard.get(key).cloned()
    }
    
    // ⚠️ CRITICAL: Drop read lock before acquiring write lock!
    fn get_or_insert(&self, key: String, default: String) -> String {
        // First, try to read
        {
            let guard = self.data.read().unwrap();
            if let Some(value) = guard.get(&key) {
                return value.clone();
            }
        }  // Read lock MUST be dropped here
        
        // Now acquire write lock
        let mut guard = self.data.write().unwrap();
        // Double-check (another thread might have inserted)
        guard.entry(key).or_insert(default).clone()
    }
}
```

### Pattern 3: Atomic State Machines

```rust
use std::sync::atomic::{AtomicU8, Ordering};

#[repr(u8)]
enum ConnectionState {
    Disconnected = 0,
    Connecting = 1,
    Connected = 2,
    Disconnecting = 3,
}

struct Connection {
    state: AtomicU8,
}

impl Connection {
    // ✅ Atomic state transitions
    fn try_connect(&self) -> bool {
        // Only transition Disconnected -> Connecting
        self.state.compare_exchange(
            ConnectionState::Disconnected as u8,
            ConnectionState::Connecting as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        ).is_ok()
    }
    
    fn complete_connection(&self) {
        // Connecting -> Connected
        let _ = self.state.compare_exchange(
            ConnectionState::Connecting as u8,
            ConnectionState::Connected as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
}
```

### Pattern 4: Message Passing with Channels

```rust
use std::sync::mpsc;
use std::thread;

enum Command {
    Process(Data),
    Shutdown,
}

fn worker_thread(rx: mpsc::Receiver<Command>) {
    loop {
        match rx.recv() {
            Ok(Command::Process(data)) => {
                process(data);
            }
            Ok(Command::Shutdown) | Err(_) => {
                break;
            }
        }
    }
}

// ✅ Clean producer-consumer separation
fn run_workers(data: Vec<Data>) {
    let (tx, rx) = mpsc::channel();
    
    let handle = thread::spawn(move || worker_thread(rx));
    
    for item in data {
        tx.send(Command::Process(item)).unwrap();
    }
    tx.send(Command::Shutdown).unwrap();
    
    handle.join().unwrap();
}
```

---

## Memory Ordering Guide

### Quick Reference

| Ordering | Use Case | Guarantees |
|----------|----------|------------|
| `Relaxed` | Counters where order doesn't matter | No synchronization, just atomicity |
| `Acquire` | Load that must see prior Release stores | Prevents reordering of later reads/writes before this |
| `Release` | Store that must be visible to Acquire loads | Prevents reordering of earlier reads/writes after this |
| `AcqRel` | Read-modify-write (fetch_add, compare_exchange) | Both Acquire and Release |
| `SeqCst` | When in doubt, or need total ordering | Strongest guarantees, highest cost |

### Common Patterns

```rust
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// Pattern: Flag to signal completion
static DONE: AtomicBool = AtomicBool::new(false);
static DATA: AtomicUsize = AtomicUsize::new(0);

fn producer() {
    DATA.store(42, Ordering::Relaxed);      // Data write
    DONE.store(true, Ordering::Release);    // Signal: all prior writes visible
}

fn consumer() {
    while !DONE.load(Ordering::Acquire) {}  // Wait for signal
    let data = DATA.load(Ordering::Relaxed); // Safe: Acquire synced with Release
    assert_eq!(data, 42);
}
```

### When to Use SeqCst

```rust
// ✅ Use SeqCst when multiple atomics must be globally ordered
// Example: Implementing a spinlock with fairness

// ❌ DON'T use SeqCst just because you're unsure
// It's the slowest ordering and often unnecessary

// ✅ DO use SeqCst for:
// - Lock-free algorithms requiring total order
// - When debugging and correctness > performance
// - Fences that need to order SeqCst operations
```

---

## Deadlock Prevention

### Rule 1: Consistent Lock Ordering

```rust
// ❌ DEADLOCK RISK: Different threads acquire in different order
fn thread_a(lock1: &Mutex<()>, lock2: &Mutex<()>) {
    let _g1 = lock1.lock();
    let _g2 = lock2.lock();  // Waits for thread_b
}

fn thread_b(lock1: &Mutex<()>, lock2: &Mutex<()>) {
    let _g2 = lock2.lock();
    let _g1 = lock1.lock();  // Waits for thread_a → DEADLOCK
}

// ✅ SAFE: Always acquire in same order (by address, ID, etc.)
fn safe_acquire(lock1: &Mutex<()>, lock2: &Mutex<()>) {
    let (first, second) = if (lock1 as *const _) < (lock2 as *const _) {
        (lock1, lock2)
    } else {
        (lock2, lock1)
    };
    let _g1 = first.lock();
    let _g2 = second.lock();
}
```

### Rule 2: Don't Hold Locks Across Await Points

```rust
// ❌ DANGEROUS in async code: Lock held across await
async fn bad_async(data: Arc<Mutex<String>>) {
    let guard = data.lock().unwrap();
    some_async_operation().await;  // Lock held during await!
    println!("{}", *guard);
}

// ✅ SAFE: Clone data, drop lock, then await
async fn good_async(data: Arc<Mutex<String>>) {
    let value = {
        let guard = data.lock().unwrap();
        guard.clone()
    };
    some_async_operation().await;
    println!("{}", value);
}
```

### Rule 3: Use try_lock for Timeout-Based Deadlock Prevention

```rust
use std::time::Duration;
use parking_lot::Mutex;

fn try_acquire_with_timeout(lock: &Mutex<()>) -> Option<parking_lot::MutexGuard<'_, ()>> {
    lock.try_lock_for(Duration::from_millis(100))
}
```

---

## Testing Concurrent Code

### Strategy 1: Unit Test with Loom

```rust
#[test]
#[cfg(loom)]
fn test_concurrent_counter() {
    loom::model(|| {
        use loom::sync::Arc;
        use loom::sync::atomic::{AtomicUsize, Ordering};
        use loom::thread;
        
        let counter = Arc::new(AtomicUsize::new(0));
        let c1 = counter.clone();
        let c2 = counter.clone();
        
        let t1 = thread::spawn(move || c1.fetch_add(1, Ordering::SeqCst));
        let t2 = thread::spawn(move || c2.fetch_add(1, Ordering::SeqCst));
        
        t1.join().unwrap();
        t2.join().unwrap();
        
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    });
}
```

### Strategy 2: Stress Testing (When Loom Doesn't Scale)

```rust
#[test]
fn stress_test_concurrent_queue() {
    use std::sync::Arc;
    use std::thread;
    
    for _ in 0..100 {  // Run many times to catch races
        let queue = Arc::new(ConcurrentQueue::new());
        let mut handles = vec![];
        
        for i in 0..8 {
            let q = queue.clone();
            handles.push(thread::spawn(move || {
                for j in 0..1000 {
                    q.push(i * 1000 + j);
                }
            }));
        }
        
        for h in handles {
            h.join().unwrap();
        }
        
        assert_eq!(queue.len(), 8 * 1000);
    }
}
```

### Strategy 3: ThreadSanitizer

```bash
# Compile with ThreadSanitizer (requires nightly)
RUSTFLAGS="-Z sanitizer=thread" cargo +nightly test

# Or use Miri for memory safety
cargo +nightly miri test
```

---

## Common Pitfalls

### Pitfall 1: Forgetting Arc for Shared Ownership

```rust
// ❌ COMPILE ERROR: Cannot move into multiple closures
fn broken(data: Mutex<Vec<i32>>) {
    thread::spawn(move || { data.lock(); });
    thread::spawn(move || { data.lock(); });  // Error: data already moved
}

// ✅ CORRECT: Use Arc for shared ownership
fn working(data: Arc<Mutex<Vec<i32>>>) {
    let d1 = data.clone();
    let d2 = data.clone();
    thread::spawn(move || { d1.lock(); });
    thread::spawn(move || { d2.lock(); });
}
```

### Pitfall 2: Poisoned Mutexes

```rust
use std::sync::Mutex;

// std::sync::Mutex becomes "poisoned" if a thread panics while holding it
fn handle_poisoned_mutex(mutex: &Mutex<i32>) -> i32 {
    match mutex.lock() {
        Ok(guard) => *guard,
        Err(poisoned) => {
            // Recover: get the data anyway (may be inconsistent!)
            *poisoned.into_inner()
        }
    }
}

// ✅ BETTER: Use parking_lot::Mutex which doesn't poison
use parking_lot::Mutex as ParkingLotMutex;

fn no_poison_worry(mutex: &ParkingLotMutex<i32>) -> i32 {
    *mutex.lock()  // Never returns Err
}
```

### Pitfall 3: Send + Sync Confusion

```rust
// Send: Safe to transfer ownership to another thread
// Sync: Safe to share references between threads

// Rc<T> is neither Send nor Sync
use std::rc::Rc;
// let rc = Rc::new(42);
// thread::spawn(move || { rc; });  // ERROR: Rc is not Send

// Arc<T> is Send + Sync (if T is Send + Sync)
use std::sync::Arc;
let arc = Arc::new(42);
thread::spawn(move || { arc; });  // OK

// RefCell<T> is Send but NOT Sync
use std::cell::RefCell;
// Cannot share &RefCell between threads

// Mutex<T> makes T Sync (if T is Send)
use std::sync::Mutex;
let mutex = Arc::new(Mutex::new(RefCell::new(42)));
// Now can share between threads via Arc<Mutex<RefCell<T>>>
```

### Pitfall 4: Busy-Waiting Without Yield

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::hint;

// ❌ BAD: Burns CPU, unfair to other threads
fn bad_spin_wait(flag: &AtomicBool) {
    while !flag.load(Ordering::Acquire) {}
}

// ✅ BETTER: Hint to CPU, slightly less wasteful
fn better_spin_wait(flag: &AtomicBool) {
    while !flag.load(Ordering::Acquire) {
        hint::spin_loop();
    }
}

// ✅ BEST: Use parking/condition variable for real waiting
use parking_lot::{Mutex, Condvar};

struct WaitableFlag {
    flag: Mutex<bool>,
    condvar: Condvar,
}

impl WaitableFlag {
    fn wait(&self) {
        let mut guard = self.flag.lock();
        while !*guard {
            self.condvar.wait(&mut guard);
        }
    }
    
    fn signal(&self) {
        *self.flag.lock() = true;
        self.condvar.notify_all();
    }
}
```

---

## Performance Tips

### Tip 1: Reduce Lock Contention

```rust
// ❌ Single lock for entire data structure
struct Slow {
    data: Mutex<HashMap<String, Vec<i32>>>,
}

// ✅ Sharded locks reduce contention
struct Fast {
    shards: [Mutex<HashMap<String, Vec<i32>>>; 16],
}

impl Fast {
    fn get_shard(&self, key: &str) -> &Mutex<HashMap<String, Vec<i32>>> {
        let hash = calculate_hash(key);
        &self.shards[hash % 16]
    }
}
```

### Tip 2: Use parking_lot Instead of std

```toml
[dependencies]
parking_lot = "0.12"
```

`parking_lot` is faster than `std::sync` for most use cases:

- Smaller `Mutex` (1 byte vs 40 bytes on Linux)
- No poisoning overhead
- Better algorithms for fairness/throughput tradeoff

### Tip 3: Consider Lock-Free Data Structures

```toml
[dependencies]
crossbeam = "0.8"
```

```rust
use crossbeam::queue::SegQueue;

// Lock-free MPMC queue
let queue = SegQueue::new();
queue.push(42);
let item = queue.pop();
```

---

## Summary Checklist

When writing concurrent code:

- [ ] Minimize shared mutable state
- [ ] Use `Arc<Mutex<T>>` or `Arc<RwLock<T>>` for shared state
- [ ] Keep critical sections short — don't hold locks during I/O
- [ ] Drop read locks before acquiring write locks (`RwLock`)
- [ ] Use consistent lock ordering to prevent deadlocks
- [ ] Choose appropriate memory ordering (when in doubt, use `SeqCst`)
- [ ] Test with loom for critical concurrent code
- [ ] Consider `parking_lot` over `std::sync` for performance
- [ ] Document synchronization intent and invariants
- [ ] Use channels for producer-consumer patterns

---

*See also: [loom-testing.md](loom-testing.md) for deterministic concurrency testing, [rust-pitfalls.md](rust-pitfalls.md) for common bugs.*
