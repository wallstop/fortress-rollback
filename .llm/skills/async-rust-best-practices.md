# Async Rust Best Practices Guide

A comprehensive guide to writing efficient, correct, and maintainable async Rust code.

## Table of Contents

1. [Fundamental Principles](#fundamental-principles)
2. [Best Practices](#best-practices)
3. [Common Pitfalls to Avoid](#common-pitfalls-to-avoid)
4. [Performance Considerations](#performance-considerations)
5. [Testing Async Code](#testing-async-code)
6. [When to Use Async vs Sync](#when-to-use-async-vs-sync)

---

## Fundamental Principles

### The Golden Rule

> **Async code should never spend a long time without reaching an `.await`.**

Task switching in async Rust only happens at `.await` points. Code that runs for extended periods between `.await`s blocks the entire runtime thread, preventing other tasks from executing.

### Understanding Futures

- **Futures are lazy**: They do nothing until polled/awaited
- **Futures are state machines**: The compiler transforms async blocks into efficient state machines
- **Futures must be pinned**: Once polled, a future cannot be moved in memory

```rust
// ❌ This does nothing - the future is never awaited
async fn fetch_data() -> Data { /* ... */ }
let _ = fetch_data(); // Warning: unused future

// ✅ Correct: await the future
let data = fetch_data().await;
```

---

## Best Practices

### 1. Use Async-Native APIs, Not Blocking Equivalents

**Bad**: Using `std::thread::sleep` in async code blocks the entire thread:

```rust
// ❌ Blocks the runtime thread
async fn bad_delay() {
    std::thread::sleep(Duration::from_secs(1)); // Blocks!
    println!("Done");
}
```

**Good**: Use Tokio's async sleep:

```rust
// ✅ Yields control back to the runtime
async fn good_delay() {
    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("Done");
}
```

### 2. Handle Blocking Operations Correctly

For unavoidable blocking operations, use `spawn_blocking`:

```rust
// ✅ Run blocking code on dedicated thread pool
let result = tokio::task::spawn_blocking(|| {
    // Expensive computation or blocking I/O
    expensive_sync_operation()
}).await?;
```

**When to use each approach**:

| Scenario | Solution |
|----------|----------|
| Short blocking (<10-100μs) | Can run inline |
| File system operations | `spawn_blocking` or `tokio::fs` |
| CPU-bound computation | `spawn_blocking` or `rayon` |
| Forever-running blocking | Dedicated `std::thread::spawn` |

### 3. Choose the Right Concurrency Primitive

**Sequential Execution** (one after another):
```rust
let a = fetch_a().await?;
let b = fetch_b().await?;
```

**Concurrent Execution** (all at once, wait for all):
```rust
// Using join! - runs concurrently, returns tuple
let (a, b, c) = tokio::join!(fetch_a(), fetch_b(), fetch_c());

// Using try_join! - fails fast on first error
let (a, b) = tokio::try_join!(fetch_a(), fetch_b())?;
```

**Concurrent Execution** (first to complete wins):
```rust
tokio::select! {
    result = fetch_a() => { /* a completed first */ }
    result = fetch_b() => { /* b completed first */ }
    _ = tokio::time::sleep(timeout) => { /* timeout */ }
}
```

**Dynamic Number of Futures**:
```rust
use futures::stream::FuturesUnordered;
use futures::StreamExt;

let mut futures = FuturesUnordered::new();
for url in urls {
    futures.push(fetch(url));
}

while let Some(result) = futures.next().await {
    handle(result);
}
```

### 4. Use Timeouts for Reliability

Always add timeouts to operations that could hang:

```rust
use tokio::time::{timeout, Duration};

// ✅ Operation with timeout
match timeout(Duration::from_secs(5), fetch_data()).await {
    Ok(result) => handle(result?),
    Err(_) => return Err(Error::Timeout),
}
```

### 5. Choose the Right Channel Type

| Channel | Use Case |
|---------|----------|
| `mpsc` | Multiple producers, single consumer |
| `oneshot` | Send exactly one value (result of computation) |
| `broadcast` | Multiple consumers, all receive all messages |
| `watch` | Single value that changes over time |

```rust
// oneshot: Perfect for returning results from spawned tasks
let (tx, rx) = tokio::sync::oneshot::channel();
tokio::spawn(async move {
    let result = compute().await;
    let _ = tx.send(result);
});
let result = rx.await?;

// mpsc: Job queue pattern
let (tx, mut rx) = tokio::sync::mpsc::channel(100);
tokio::spawn(async move {
    while let Some(job) = rx.recv().await {
        process(job).await;
    }
});
```

### 6. Implement Graceful Shutdown

Use cancellation tokens for coordinated shutdown:

```rust
use tokio_util::sync::CancellationToken;

let token = CancellationToken::new();
let cloned_token = token.clone();

// Worker task
tokio::spawn(async move {
    loop {
        tokio::select! {
            _ = cloned_token.cancelled() => {
                // Cleanup and exit
                break;
            }
            result = do_work() => {
                handle(result);
            }
        }
    }
});

// Shutdown trigger
tokio::signal::ctrl_c().await?;
token.cancel();
```

Use `TaskTracker` to wait for all tasks to complete:

```rust
use tokio_util::task::TaskTracker;

let tracker = TaskTracker::new();

for i in 0..10 {
    tracker.spawn(async move { process(i).await });
}

tracker.close();
tracker.wait().await;  // Wait for all tasks
```

### 7. Prefer Owned Data in Spawned Tasks

Spawned tasks require `'static` bounds. Prefer owned data:

```rust
// ❌ Won't compile - borrows data
let data = vec![1, 2, 3];
tokio::spawn(async {
    println!("{:?}", data);  // Error: data doesn't live long enough
});

// ✅ Clone or move owned data
let data = vec![1, 2, 3];
tokio::spawn(async move {
    println!("{:?}", data);  // data is moved into the task
});

// ✅ Use Arc for shared read-only data
let data = Arc::new(vec![1, 2, 3]);
let data_clone = Arc::clone(&data);
tokio::spawn(async move {
    println!("{:?}", data_clone);
});
```

### 8. Use Async-Aware Synchronization Primitives

**For shared state across `.await` points**, use `tokio::sync::Mutex`:

```rust
use tokio::sync::Mutex;

let shared = Arc::new(Mutex::new(HashMap::new()));

// ✅ Safe to hold across .await
let mut guard = shared.lock().await;
let value = fetch_value().await;
guard.insert(key, value);
```

**For quick, synchronous access**, `std::sync::Mutex` is fine:

```rust
use std::sync::Mutex;

let shared = Arc::new(Mutex::new(counter));

// ✅ OK if lock is brief and no .await while held
{
    let mut guard = shared.lock().unwrap();
    *guard += 1;
}  // Lock released before any await
```

### 9. Structure Error Handling for Async Code

Use `Result` types consistently and handle errors at appropriate levels:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum ServiceError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    #[error("cancelled")]
    Cancelled,
}

async fn fetch_with_retry(url: &str) -> Result<Response, ServiceError> {
    for attempt in 0..3 {
        match timeout(Duration::from_secs(5), client.get(url).send()).await {
            Ok(Ok(response)) => return Ok(response),
            Ok(Err(e)) if attempt < 2 => {
                tokio::time::sleep(backoff(attempt)).await;
                continue;
            }
            Ok(Err(e)) => return Err(ServiceError::Network(e)),
            Err(_) => return Err(ServiceError::Timeout(Duration::from_secs(5))),
        }
    }
    unreachable!()
}
```

### 10. Use JoinSet for Managing Multiple Tasks

For spawning and managing multiple tasks:

```rust
use tokio::task::JoinSet;

let mut set = JoinSet::new();

for url in urls {
    set.spawn(async move {
        fetch(url).await
    });
}

// Process results as they complete
while let Some(result) = set.join_next().await {
    match result {
        Ok(Ok(data)) => process(data),
        Ok(Err(e)) => log::error!("Task failed: {e}"),
        Err(e) => log::error!("Task panicked: {e}"),
    }
}
```

### 11. Configure Runtime Appropriately

**For applications** - use full features:
```toml
tokio = { version = "1", features = ["full"] }
```

**For libraries** - use minimal features:
```toml
tokio = { version = "1", features = ["rt", "sync"] }
```

**For single-threaded scenarios**:
```rust
#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Simpler, no Send + 'static requirements for local tasks
}
```

**For custom configuration**:
```rust
let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(4)
    .enable_all()
    .build()?;

runtime.block_on(async { /* ... */ });
```

### 12. Handle Cancellation Safely

Remember: dropping a future cancels it. Ensure cleanup happens:

```rust
async fn safe_operation() {
    let temp_file = create_temp_file().await?;
    
    // Use a guard to ensure cleanup
    let _guard = scopeguard::guard((), |_| {
        // This runs even if the future is cancelled
        let _ = std::fs::remove_file(&temp_file);
    });
    
    process_file(&temp_file).await?;
    // Guard drops here, cleaning up
}
```

### 13. Implement Backpressure

Prevent memory exhaustion with bounded channels:

```rust
// ❌ Unbounded can grow forever
let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

// ✅ Bounded provides backpressure
let (tx, rx) = tokio::sync::mpsc::channel(100);

// Sender will wait when buffer is full
tx.send(item).await?;
```

### 14. Use Semaphores for Concurrency Limiting

Control maximum concurrent operations:

```rust
use tokio::sync::Semaphore;

let semaphore = Arc::new(Semaphore::new(10));  // Max 10 concurrent

for url in urls {
    let permit = semaphore.clone().acquire_owned().await?;
    tokio::spawn(async move {
        let _permit = permit;  // Held for duration
        fetch(url).await
    });
}
```

### 15. Use Streams for Async Iteration

Process items as they arrive:

```rust
use tokio_stream::StreamExt;

let mut stream = tokio_stream::iter(items)
    .map(|item| async move { process(item).await })
    .buffer_unordered(10);  // Process 10 concurrently

while let Some(result) = stream.next().await {
    handle(result)?;
}
```

---

## Common Pitfalls to Avoid

### 1. Blocking the Runtime Thread

```rust
// ❌ These block the runtime:
std::thread::sleep(duration);
std::fs::read_to_string(path);
mutex.lock().unwrap();  // If held across .await

// ✅ Use async alternatives:
tokio::time::sleep(duration).await;
tokio::fs::read_to_string(path).await;
async_mutex.lock().await;
```

### 2. Forgetting to Await Futures

```rust
// ❌ Does nothing!
async fn process() {
    fetch_data();  // Warning: unused future
}

// ✅ Await the result
async fn process() {
    fetch_data().await;
}
```

### 3. Holding Locks Across Await Points

```rust
// ❌ Can cause deadlocks with std::sync::Mutex
let guard = mutex.lock().unwrap();
async_operation().await;  // Other tasks can't get lock
drop(guard);

// ✅ Release before await
{
    let mut guard = mutex.lock().unwrap();
    *guard = new_value;
}  // Lock released
async_operation().await;

// ✅ Or use tokio::sync::Mutex
let guard = async_mutex.lock().await;
async_operation().await;  // Safe with async mutex
```

### 4. Creating Too Many Tasks

```rust
// ❌ Spawns millions of tasks
for i in 0..1_000_000 {
    tokio::spawn(async move { process(i).await });
}

// ✅ Use streams with concurrency limit
use futures::stream::{self, StreamExt};
stream::iter(0..1_000_000)
    .for_each_concurrent(100, |i| async move {
        process(i).await;
    })
    .await;
```

### 5. Ignoring Backpressure

```rust
// ❌ Can exhaust memory
loop {
    let data = fetch().await;
    tx.send(data).await;  // If receiver is slow, queue grows
}

// ✅ Handle channel full condition
loop {
    let data = fetch().await;
    if tx.send(data).await.is_err() {
        break;  // Receiver dropped
    }
}
```

### 6. Not Handling Task Panics

```rust
// ❌ Panic goes unnoticed
tokio::spawn(async { panic!("oops") });

// ✅ Handle join errors
let handle = tokio::spawn(async { risky_operation().await });
match handle.await {
    Ok(result) => process(result),
    Err(e) if e.is_panic() => log::error!("Task panicked: {e}"),
    Err(e) => log::error!("Task cancelled: {e}"),
}
```

---

## Performance Considerations

### Runtime Selection

| Runtime | Best For |
|---------|----------|
| Multi-thread (`rt-multi-thread`) | I/O-heavy servers, parallel work |
| Current-thread (`current_thread`) | Simpler apps, embedded, avoiding `Send` bounds |
| `rayon` | CPU-bound parallelism |

### Memory Efficiency

- **Avoid unnecessary `Arc` wrapping** - pass by reference when possible
- **Use `Box<[T]>` over `Vec<T>`** for fixed-size allocations
- **Reuse buffers** in hot loops instead of allocating

### Task Granularity

- **Too coarse**: Poor utilization, one slow operation blocks others
- **Too fine**: Overhead from scheduling dominates actual work

Rule of thumb: Tasks should represent logical units of work that can progress independently.

### Avoiding Allocation in Hot Paths

```rust
// ❌ Allocates on every iteration
loop {
    let buffer = vec![0u8; 1024];
    socket.read(&mut buffer).await?;
}

// ✅ Reuse buffer
let mut buffer = vec![0u8; 1024];
loop {
    let n = socket.read(&mut buffer).await?;
    process(&buffer[..n]);
}
```

---

## Testing Async Code

### Use `#[tokio::test]`

```rust
#[tokio::test]
async fn test_async_function() {
    let result = my_async_function().await;
    assert_eq!(result, expected);
}
```

### Time Manipulation

Fast-forward time in tests:

```rust
#[tokio::test(start_paused = true)]
async fn test_timeout_behavior() {
    // Time is paused - sleeps complete instantly
    let start = std::time::Instant::now();
    tokio::time::sleep(Duration::from_secs(3600)).await;
    assert!(start.elapsed() < Duration::from_millis(10));
}
```

### Mocking I/O

Use `tokio_test::io::Builder` for mocking:

```rust
#[tokio::test]
async fn test_protocol() {
    let reader = tokio_test::io::Builder::new()
        .read(b"hello\r\n")
        .build();
    let writer = tokio_test::io::Builder::new()
        .write(b"world\r\n")
        .build();
    
    handle_connection(reader, writer).await.unwrap();
}
```

### Testing Cancellation

```rust
#[tokio::test]
async fn test_cancellation_cleanup() {
    let (tx, rx) = oneshot::channel();
    
    let handle = tokio::spawn(async move {
        // Setup
        let _guard = scopeguard::guard(tx, |tx| {
            let _ = tx.send(());  // Signal cleanup happened
        });
        
        tokio::time::sleep(Duration::from_secs(100)).await;
    });
    
    // Cancel the task
    handle.abort();
    
    // Verify cleanup occurred
    rx.await.expect("cleanup should have run");
}
```

---

## When to Use Async vs Sync

### Use Async When:

- **High concurrency** - Thousands of concurrent connections
- **I/O-bound workloads** - Network servers, database clients
- **External library requires it** - Many modern Rust libraries are async-first
- **Natural fit** - Event-driven architectures, websockets, streaming

### Use Sync (Threads) When:

- **CPU-bound computation** - Use `rayon` for parallelism
- **Simple I/O patterns** - Reading a few files, one-shot HTTP requests
- **Simpler code matters** - Easier error handling, debugging
- **Blocking is unavoidable** - Legacy APIs, FFI

### Hybrid Approach

```rust
// CPU-bound work on rayon, communicate via channel
async fn process_with_rayon(data: Vec<Item>) -> Vec<Result> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    
    rayon::spawn(move || {
        let results: Vec<_> = data
            .par_iter()
            .map(|item| heavy_computation(item))
            .collect();
        let _ = tx.send(results);
    });
    
    rx.await.expect("rayon task completed")
}
```

---

## Quick Reference

### Blocking Operations Cheat Sheet

| Operation | Async Alternative |
|-----------|------------------|
| `std::thread::sleep` | `tokio::time::sleep` |
| `std::fs::*` | `tokio::fs::*` |
| `std::net::*` | `tokio::net::*` |
| `std::sync::Mutex` | `tokio::sync::Mutex` (if held across `.await`) |
| DNS lookup | `tokio::net::lookup_host` |
| Blocking FFI | `spawn_blocking` |

### Concurrency Patterns Quick Reference

| Pattern | Primitive | Use Case |
|---------|-----------|----------|
| Run sequentially | `a.await; b.await` | Dependent operations |
| Run concurrently, wait all | `join!` / `try_join!` | Independent operations |
| Run concurrently, first wins | `select!` | Racing, timeouts |
| Spawn independent work | `tokio::spawn` | Fire-and-forget, parallelism |
| Limit concurrency | `Semaphore` | Rate limiting |
| Process stream | `StreamExt::for_each_concurrent` | Bounded parallel processing |

---

## Summary

1. **Never block the runtime** - Use `spawn_blocking` for unavoidable blocking
2. **Choose concurrency primitives wisely** - `join!`, `select!`, channels, semaphores
3. **Handle errors and cancellation** - Use `Result`, timeouts, cancellation tokens
4. **Test with time control** - Use `start_paused = true` for deterministic tests
5. **Consider if you need async** - Threads are simpler when concurrency is low
6. **Implement graceful shutdown** - Use `CancellationToken` and `TaskTracker`
7. **Apply backpressure** - Use bounded channels to prevent memory exhaustion
