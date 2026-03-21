<!-- CATEGORY: Rust Language -->
<!-- WHEN: Choosing concurrency primitives, Mutex vs RwLock decisions, channel patterns -->
# Concurrency Patterns

## Decision Tree

```
Is data accessed from multiple threads?
  No -> No synchronization needed
  Yes -> Read-only after init?
    Yes -> Arc<T>
    No -> What access pattern?
      Mostly reads, rare writes -> RwLock<T>
      Frequent writes, short sections -> Mutex<T>
      Single atomic value -> Atomic* types
      Producer-consumer -> Channels
```

## Primitive Comparison

| Primitive | Use When | Avoid When |
|-----------|----------|------------|
| `Mutex<T>` | Short critical sections, frequent writes | Long-held locks, read-heavy |
| `RwLock<T>` | Read-heavy, occasional writes | Write-heavy (reader starvation) |
| `Atomic*` | Single values, lock-free algorithms | Complex multi-field updates |
| `Arc<T>` | Shared ownership, immutable data | Mutable data (use `Arc<Mutex<T>>`) |
| `mpsc::channel` | Producer-consumer, message passing | Low-latency requirements |
| `crossbeam::channel` | High-performance channels | Simple use cases |

## Key Patterns

### Mutex -- Minimize Hold Time
```rust
// Use parking_lot::Mutex -- no poisoning, so .lock() returns the guard directly
let value = {
    let guard = counter.lock();
    *guard // copy value
}; // lock released
expensive_operation(); // other threads can proceed
```

### RwLock -- Drop Read Before Write
```rust
// Use parking_lot::RwLock -- no poisoning, no .unwrap() needed
let needs_write = {
    let guard = self.data.read();
    needs_update(&guard)
}; // read lock MUST drop here
if needs_write {
    let mut guard = self.data.write();
    if needs_update(&guard) { update(&mut guard); } // double-check
}
```

### Atomic State Machine
```rust
fn try_connect(&self) -> bool {
    self.state.compare_exchange(
        Disconnected as u8, Connecting as u8,
        Ordering::AcqRel, Ordering::Acquire,
    ).is_ok()
}
```

### Message Passing
```rust
enum Command { Process(Data), Shutdown }
fn worker(rx: mpsc::Receiver<Command>) {
    loop {
        match rx.recv() {
            Ok(Command::Process(d)) => process(d),
            Ok(Command::Shutdown) | Err(_) => break,
        }
    }
}
```

## Memory Ordering

| Ordering | Use Case |
|----------|----------|
| `Relaxed` | Counters where order doesn't matter |
| `Acquire` | Load that must see prior Release stores |
| `Release` | Store that must be visible to Acquire loads |
| `AcqRel` | Read-modify-write (fetch_add, compare_exchange) |
| `SeqCst` | Total ordering needed, or when in doubt |

## Deadlock Prevention

1. **Consistent lock ordering** -- always acquire in same order (by address or ID)
2. **Don't hold locks across await** -- clone data, release, then await
3. **Use `try_lock`** for timeout-based prevention

## Testing

- **Loom:** Deterministic exploration of all interleavings for critical code
- **Stress testing:** Run many iterations to catch races
- **ThreadSanitizer:** `RUSTFLAGS="-Z sanitizer=thread" cargo +nightly test`

## Performance Tips

- **Sharded locks** reduce contention (e.g., 16 Mutex shards)
- **`parking_lot`** is faster than `std::sync` (smaller Mutex, no poisoning)
- **Lock-free structures** via `crossbeam` (SegQueue, etc.)

## Common Pitfalls

| Pitfall | Fix |
|---------|-----|
| No `Arc` for shared ownership | `Arc::clone()` before `thread::spawn` |
| Poisoned mutexes | Use `parking_lot::Mutex` (no poisoning) |
| `Rc` across threads | `Rc` is not `Send`; use `Arc` |
| Busy-wait without yield | `std::hint::spin_loop()` or condvar |
| `std::sync::Mutex` held across `.await` | Use `tokio::sync::Mutex` |

## Checklist

- [ ] Minimize shared mutable state
- [ ] Keep critical sections short -- no I/O under lock
- [ ] Drop read locks before acquiring write locks
- [ ] Consistent lock ordering
- [ ] Appropriate memory ordering (SeqCst if unsure)
- [ ] Test with loom for critical concurrent code
- [ ] Consider `parking_lot` over `std::sync`
- [ ] Document synchronization intent
