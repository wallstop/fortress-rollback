<!-- CATEGORY: Determinism & Rollback -->
<!-- WHEN: Deterministic simulation testing, madsim, turmoil, failure injection -->

# Deterministic Simulation Testing (DST)

DST enables rapid, reproducible bug discovery by controlling all sources of non-determinism and running systems in a single-threaded simulator.

---

## The Four Ingredients

### 1. Concurrency Control

Run everything on a single thread with cooperative scheduling. Tasks yield at `.await` points; scheduler picks next task deterministically.

### 2. Time Control

Mock time controlled by the simulator. `sleep()` advances virtual time instantly.

### 3. Randomness Control

Single seeded RNG (e.g., `Pcg64::seed_from_u64(seed)`) drives all random decisions.

### 4. Failure Injection

Controlled, deterministic failures: network partitions, message loss/delay, disk I/O failures, node crashes.

```rust
// FoundationDB-style buggify
if buggify() {  // ~25% chance when enabled, deterministic from seed
    return Err(io::Error::new(io::ErrorKind::TimedOut, "injected"));
}
```

---

## Framework Comparison

| Feature | madsim | turmoil | Manual (sled-style) |
|---------|--------|---------|---------------------|
| Approach | Drop-in tokio replacement | Tokio-native DST | State machine actors |
| Setup | `[patch]` + cfg flag | Dev-dependency | Custom implementation |
| Network sim | Yes (TCP/UDP) | Yes (TCP) | Custom |
| Disk sim | Yes | Unstable feature | Custom |
| Time sim | Yes (overrides libc) | Yes | Custom |
| Node control | kill/restart/pause | crash/bounce | Custom |
| Complexity | Medium | Low | High (but max control) |
| Best for | Large tokio systems | Simple network tests | Maximum control |

---

## madsim

```toml
[dependencies]
tokio = { version = "0.2", package = "madsim-tokio" }

[patch.crates-io]
getrandom = { git = "https://github.com/madsim-rs/getrandom.git", rev = "8daf97e" }
```

```bash
# Run tests in simulation mode
RUSTFLAGS="--cfg madsim" cargo test

# Reproduce a failure
MADSIM_TEST_SEED=12345 RUSTFLAGS="--cfg madsim" cargo test test_name

# Verify determinism (runs twice with same seed)
MADSIM_TEST_CHECK_DETERMINISM=1 RUSTFLAGS="--cfg madsim" cargo test
```

```rust
#[madsim::test]
async fn test_partition_recovery() {
    let handle = madsim::runtime::Handle::current();
    let server = handle.create_node().build();
    let client = handle.create_node().build();

    server.spawn(async { /* server logic */ });
    client.spawn(async { /* client logic */ });

    // Failure injection
    let net = handle.net_sim();
    net.disconnect(server.id(), client.id());
    madsim::time::sleep(Duration::from_secs(5)).await;
    net.connect(server.id(), client.id());
}
```

Failure injection APIs: `handle.kill()`, `handle.restart()`, `handle.pause()`, `handle.resume()`, `net.set_link_latency()`, `buggify()`, `buggify_with_prob(0.1)`.

---

## turmoil

```toml
[dev-dependencies]
turmoil = "0.7"
```

```rust
#[test]
fn test_partition() -> turmoil::Result {
    let mut sim = turmoil::Builder::new()
        .rng_seed(42)
        .simulation_duration(Duration::from_secs(60))
        .fail_rate(0.05)
        .build();

    sim.host("server", || async { /* ... */ Ok(()) });
    sim.client("client", async { /* ... */ Ok(()) });

    sim.partition("server", "client");
    sim.run()?;

    sim.repair("server", "client");
    sim.run()
}
```

APIs: `sim.partition()`, `sim.repair()`, `sim.crash()`, `sim.bounce()`, `turmoil::hold()`, `turmoil::release()`.

---

## Manual DST (sled-style)

Design system as state machines with a message-based simulator:

```rust
trait Actor {
    type Message;
    fn receive(&mut self, msg: Self::Message, at: Instant) -> Vec<(Self::Message, NodeId)>;
}

struct Simulator<A: Actor> {
    nodes: HashMap<NodeId, A>,
    events: BinaryHeap<Event<A::Message>>,
    rng: Pcg64,
}
```

Benefits: total control, minimal dependencies, natural separation of pure logic from I/O.

---

## Design Patterns

### Message Bus Abstraction

```rust
trait MessageBus {
    fn send(&mut self, to: NodeId, msg: Message);
    fn receive(&mut self) -> Option<(NodeId, Message)>;
    fn current_time(&self) -> Instant;
}
// Production: TcpMessageBus; Simulation: SimulatedMessageBus
```

### Feature-Flagged Implementations

```rust
#[cfg(not(madsim))]
pub fn current_time() -> Instant { Instant::now() }

#[cfg(madsim)]
pub fn current_time() -> Instant { madsim::time::Instant::now() }
```

### Pure State Machine Core

Structure core logic as pure state transitions (old state + input = new state + outputs). Keep I/O at the edges.

---

## Common Pitfalls

| Pitfall | Solution |
|---------|----------|
| HashMap iteration order | Use `BTreeMap` or sort keys |
| `std::thread::spawn` | Use `tokio::spawn` under simulator |
| Unpatched deps calling `getrandom` | Check with `MADSIM_TEST_CHECK_DETERMINISM=1` |
| Unbounded loops without yields | Add `tokio::task::yield_now().await` |
| Memory address-dependent logic | Use deterministic IDs |

---

## Testing Strategy

1. **Unit tests** -- fast, focused, deterministic
2. **DST integration tests** -- explore concurrent behaviors
3. **Long-running fuzzing** -- nightly random seed exploration
4. **Determinism verification** -- CI checks (run twice, compare)

```yaml
# CI: Run DST + verify determinism
- run: RUSTFLAGS="--cfg madsim" cargo test --release
- run: MADSIM_TEST_CHECK_DETERMINISM=1 RUSTFLAGS="--cfg madsim" cargo test --release
```

---

## Tool Selection

| Scenario | Tool |
|----------|------|
| Drop-in tokio replacement | madsim |
| Simple network simulation | turmoil |
| Maximum control | Manual (sled-style) |
| Lock-free data structures | loom |
| Quick time mocking | tokio-test |
