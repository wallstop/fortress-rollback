<!-- CATEGORY: Rust Language -->
<!-- WHEN: Writing async code, debugging futures, Send/Sync issues -->
# Async Rust Best Practices

## Golden Rule

> Async code should never spend a long time without reaching an `.await`.

Task switching only happens at `.await` points. Long-running code between awaits blocks the runtime thread.

## Blocking Operations

| Scenario | Solution |
|----------|----------|
| Short blocking (<100us) | Run inline |
| File system ops | `spawn_blocking` or `tokio::fs` |
| CPU-bound computation | `spawn_blocking` or `rayon` |
| Forever-running blocking | Dedicated `std::thread::spawn` |

```rust
let result = tokio::task::spawn_blocking(|| expensive_sync_operation()).await?;
```

## Concurrency Patterns

| Pattern | Primitive | Use Case |
|---------|-----------|----------|
| Sequential | `a.await; b.await` | Dependent operations |
| Concurrent, wait all | `join!` / `try_join!` | Independent operations |
| First wins | `select!` | Racing, timeouts |
| Spawn independent | `tokio::spawn` | Fire-and-forget |
| Limit concurrency | `Semaphore` | Rate limiting |
| Process stream | `StreamExt::for_each_concurrent` | Bounded parallel processing |
| Dynamic futures | `FuturesUnordered` / `JoinSet` | Variable number of tasks |

## Timeouts

```rust
match timeout(Duration::from_secs(5), fetch_data()).await {
    Ok(result) => handle(result?),
    Err(_) => return Err(Error::Timeout),
}
```

## Channel Selection

| Channel | Use Case |
|---------|----------|
| `mpsc` | Multiple producers, single consumer |
| `oneshot` | Send exactly one value |
| `broadcast` | All consumers receive all messages |
| `watch` | Single changing value |

## Synchronization

- **Across `.await` points:** Use `tokio::sync::Mutex`
- **Quick synchronous access (no `.await` while held):** `std::sync::Mutex` is fine

## Graceful Shutdown

Use `CancellationToken` + `TaskTracker`:

```rust
let token = CancellationToken::new();
tokio::spawn(async move {
    tokio::select! {
        _ = token.cancelled() => { break; }
        result = do_work() => { handle(result); }
    }
});
token.cancel(); // trigger shutdown
```

## Spawned Tasks Require `'static`

Prefer owned data or `Arc` for shared read-only data.

## Backpressure

Use bounded channels (`mpsc::channel(100)`) to prevent memory exhaustion. Never `unbounded_channel` in production without good reason.

## Common Pitfalls

| Pitfall | Fix |
|---------|-----|
| `std::thread::sleep` in async | `tokio::time::sleep().await` |
| Forgetting to `.await` futures | Futures are lazy -- nothing happens without await |
| `std::sync::Mutex` held across `.await` | Use `tokio::sync::Mutex` or release before await |
| Spawning millions of tasks | Use `for_each_concurrent` with limit |
| Ignoring task panics | Check `JoinHandle` result |
| Dropping future cancels it | Use guards for cleanup |

## Blocking Ops Cheat Sheet

| std | Async Alternative |
|-----|------------------|
| `std::thread::sleep` | `tokio::time::sleep` |
| `std::fs::*` | `tokio::fs::*` |
| `std::net::*` | `tokio::net::*` |
| `std::sync::Mutex` (across await) | `tokio::sync::Mutex` |
| Blocking FFI | `spawn_blocking` |

## Testing

```rust
#[tokio::test]
async fn test_fn() { /* ... */ }

#[tokio::test(start_paused = true)]
async fn test_timeout() {
    // Time is paused -- sleeps complete instantly
    tokio::time::sleep(Duration::from_secs(3600)).await;
}
```

## When to Use Async vs Sync

**Async:** High concurrency, I/O-bound, event-driven, library requires it.
**Sync/Threads:** CPU-bound (use rayon), simple I/O, simpler debugging, blocking FFI.
