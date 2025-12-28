# Deterministic Simulation Testing (DST) in Rust

> **A comprehensive guide to deterministic simulation testing for distributed and networked systems.** DST enables rapid, reproducible bug discovery by controlling all sources of non-determinism and running systems in a single-threaded simulator.

## Why DST Matters

**Deterministic Simulation Testing (DST)** is a testing methodology that allows you to:

1. **Find rare concurrency bugs** — Explore thousands of execution orderings per second
2. **100% reproducibility** — Every bug can be reproduced with the same seed
3. **Fast iteration** — Run in microseconds what would take minutes in real systems
4. **Controlled chaos** — Inject failures, delays, and partitions deterministically

### DST vs Other Testing Approaches

| Approach | Speed | Reproducibility | Coverage | Complexity |
|----------|-------|-----------------|----------|------------|
| **DST** | ⚡ Thousands/sec | ✅ 100% | High | Medium |
| **Jepsen** | 🐢 5 min/test | ❌ Probabilistic | Very High | High |
| **Chaos Engineering** | 🐢 Real-time | ❌ Probabilistic | Medium | Medium |
| **Unit Tests** | ⚡ Instant | ✅ 100% | Low | Low |
| **Fuzzing** | ⚡ Fast | ⚠️ Seed-dependent | Medium | Low |

### Who Uses DST Successfully?

- **FoundationDB** — "Jepsen-proof" distributed database (original DST pioneers)
- **RisingWave** — Cloud-native streaming database using madsim
- **TigerBeetle** — Financial database built around DST from day one
- **Riak** — Distributed key-value store
- **Dropbox Sync** — File synchronization engine

---

## The Four Ingredients of DST

DST requires controlling four sources of non-determinism:

### 1. Concurrency Control

**Problem:** Thread scheduling is non-deterministic.

**Solution:** Run everything on a single thread with cooperative scheduling.

```rust
// ❌ Non-deterministic: OS schedules threads unpredictably
std::thread::spawn(|| { /* ... */ });

// ✅ Deterministic: Cooperative tasks on single thread
tokio::spawn(async { /* ... */ });  // Under DST runtime control
```

**Key patterns:**
- Use async/await with a controlled executor
- Tasks yield at known points (`.await`)
- Scheduler picks next task deterministically (often FIFO or seeded random)

### 2. Time Control

**Problem:** `Instant::now()` and `SystemTime::now()` return wall-clock time.

**Solution:** Mock time that the simulator controls.

```rust
// ❌ Non-deterministic: Real wall-clock time
let start = std::time::Instant::now();
tokio::time::sleep(Duration::from_secs(1)).await;
let elapsed = start.elapsed();  // Actually waited 1 second

// ✅ Deterministic: Simulated time
// Under madsim/turmoil, time is controlled by the simulator
tokio::time::sleep(Duration::from_secs(1)).await;
// Simulator instantly advances virtual time — no actual waiting
```

**Implementation approaches:**
1. **Override libc functions** (`gettimeofday`, `clock_gettime`) — madsim approach
2. **Conditional compilation** — Replace `std::time` with simulation version
3. **Dependency injection** — Pass time source as parameter

### 3. Randomness Control

**Problem:** `rand::random()` uses system entropy.

**Solution:** Seeded deterministic RNG.

```rust
// ❌ Non-deterministic: System entropy
let x: u32 = rand::random();

// ✅ Deterministic: Seeded RNG
use rand_pcg::Pcg64;
use rand::SeedableRng;

let seed = 12345u64;
let mut rng = Pcg64::seed_from_u64(seed);
let x: u32 = rng.gen();  // Same seed = same sequence
```

**Best practices:**
- Single global RNG for the simulation
- All random decisions flow from this RNG
- Include RNG state in saved states for rollback

### 4. Failure Injection

**Problem:** Real failures are rare and unpredictable.

**Solution:** Controlled, deterministic failure injection.

```rust
// Simulator decides when failures occur based on seeded RNG
pub fn buggify() -> bool {
    // Returns true ~25% of time when buggify is enabled
    // Deterministic given the same seed
}

if buggify() {
    return Err(io::Error::new(io::ErrorKind::TimedOut, "injected"));
}
```

**Types of failures to inject:**
- Network partitions and message loss
- Message delays and reordering
- Disk I/O failures
- Node crashes and restarts
- Resource exhaustion (memory, file descriptors)

---

## DST Frameworks in Rust

### 1. madsim — Drop-in Tokio Replacement

**Repository:** https://github.com/madsim-rs/madsim

**Philosophy:** Replace tokio and external dependencies at compile time.

```toml
# Cargo.toml
[dependencies]
madsim = "0.2"

# Replace standard dependencies with simulators
tokio = { version = "0.2", package = "madsim-tokio" }
tonic = { version = "0.5", package = "madsim-tonic" }
etcd-client = { version = "0.4", package = "madsim-etcd-client" }
aws-sdk-s3 = { version = "0.5", package = "madsim-aws-sdk-s3" }

# Patch dependencies that access real randomness/time
[patch.crates-io]
getrandom = { git = "https://github.com/madsim-rs/getrandom.git", rev = "8daf97e" }
quanta = { git = "https://github.com/madsim-rs/quanta.git", rev = "948bdc3" }
```

**Running simulation tests:**

```bash
# Enable madsim mode via cfg flag
RUSTFLAGS="--cfg madsim" cargo test

# Run with specific seed for reproduction
MADSIM_TEST_SEED=12345 RUSTFLAGS="--cfg madsim" cargo test

# Check for non-determinism (runs twice with same seed)
MADSIM_TEST_CHECK_DETERMINISM=1 RUSTFLAGS="--cfg madsim" cargo test
```

**Basic test structure:**

```rust
use madsim::runtime::Runtime;
use std::time::Duration;

#[madsim::test]
async fn my_deterministic_test() {
    // Create virtual nodes
    let handle = madsim::runtime::Handle::current();
    
    let server = handle.create_node().build();
    let client = handle.create_node().build();
    
    // Spawn tasks on nodes
    server.spawn(async {
        let listener = TcpListener::bind("0.0.0.0:8080").await?;
        // ...
    });
    
    client.spawn(async {
        let stream = TcpStream::connect("server:8080").await?;
        // ...
    });
    
    // Inject failures
    handle.kill(server.id());  // Kill a node
    madsim::time::sleep(Duration::from_secs(1)).await;  // Simulated time
    handle.restart(server.id());  // Restart the node
}
```

**Failure injection APIs:**

```rust
use madsim::runtime::Handle;
use madsim::net::NetSim;

let handle = Handle::current();

// Node control
handle.kill(node_id);      // Immediately kill all tasks on node
handle.restart(node_id);   // Restart node's init function
handle.pause(node_id);     // Pause execution
handle.resume(node_id);    // Resume execution

// Network control (via NetSim plugin)
let net = handle.net_sim();
net.disconnect(node_a, node_b);    // Partition nodes
net.connect(node_a, node_b);       // Repair connection
net.set_link_latency(a, b, delay); // Inject latency

// Buggify for controlled randomness
use madsim::buggify::{buggify, buggify_with_prob};

if buggify() {  // ~25% chance when enabled
    return Err("injected failure");
}

if buggify_with_prob(0.1) {  // 10% chance
    tokio::time::sleep(Duration::from_secs(5)).await;  // Inject delay
}
```

**Architecture:**

```
┌─────────────────────────────────────────────────────────────┐
│                    Madsim Runtime                           │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Global RNG │  │    Timer    │  │   Task Scheduler    │  │
│  │   (seeded)  │  │ (priority Q)│  │  (FIRO: First In,   │  │
│  │             │  │             │  │   Random Out)       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│              Environment Simulators                         │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌──────────┐ │
│  │  Network  │  │   Disk    │  │   Etcd    │  │   S3     │ │
│  │ Simulator │  │ Simulator │  │ Simulator │  │ Simulator│ │
│  └───────────┘  └───────────┘  └───────────┘  └──────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 2. turmoil — Tokio-Native DST

**Repository:** https://github.com/tokio-rs/turmoil

**Philosophy:** Minimal API surface, integrate with existing tokio code.

```toml
# Cargo.toml
[dev-dependencies]
turmoil = "0.7"
```

**Basic structure:**

```rust
use turmoil::{Builder, Result};
use std::time::Duration;

#[test]
fn my_deterministic_test() -> Result {
    let mut sim = Builder::new()
        .simulation_duration(Duration::from_secs(60))
        .tick_duration(Duration::from_millis(1))
        .min_message_latency(Duration::from_millis(1))
        .max_message_latency(Duration::from_millis(10))
        .fail_rate(0.05)  // 5% message failure rate
        .build();

    // Register a server host (can restart)
    sim.host("server", || async {
        let listener = turmoil::net::TcpListener::bind("0.0.0.0:8080").await?;
        loop {
            let (stream, _) = listener.accept().await?;
            // Handle connection...
        }
    });

    // Register a client (one-shot)
    sim.client("client", async {
        let stream = turmoil::net::TcpStream::connect("server:8080").await?;
        // Test logic...
        Ok(())
    });

    sim.run()
}
```

**Failure injection:**

```rust
use turmoil::{partition, repair, hold, release};

// In test code:
sim.partition("server", "client");  // Network partition
sim.repair("server", "client");     // Heal partition

sim.crash("server");   // Crash a host
sim.bounce("server");  // Restart a host

// From within host/client code:
turmoil::partition("server", "client");
turmoil::hold("server", "client");    // Hold messages without dropping
turmoil::release("server", "client"); // Release held messages
```

**Deterministic seed:**

```rust
let mut sim = Builder::new()
    .rng_seed(12345)  // Reproducible execution
    .build();
```

**Filesystem simulation (unstable feature):**

```toml
[dev-dependencies]
turmoil = { version = "0.7", features = ["unstable-fs"] }
```

```rust
use turmoil::fs::shim::std::fs::OpenOptions;

// Simulated filesystem with crash consistency testing
let file = OpenOptions::new()
    .write(true)
    .create(true)
    .open("/data/db")?;

file.write_all(b"data")?;
file.sync_all()?;  // Data survives sim.crash()

// Unsynced data is lost on crash
```

### 3. Manual DST (sled-style)

**Repository:** https://sled.rs/simulation.html

**Philosophy:** Design your system as a state machine from the start.

**Core interface:**

```rust
/// State machine that receives and sends messages
trait Actor {
    type Message;
    type Response;
    
    /// Process incoming message, return outgoing messages
    fn receive(
        &mut self,
        msg: Self::Message,
        at: Instant,
    ) -> Vec<(Self::Message, NodeId)>;
    
    /// Periodic tick for timeouts, elections, etc.
    fn tick(&mut self, at: Instant) -> Vec<(Self::Message, NodeId)>;
}
```

**Simulator structure:**

```rust
use std::collections::BinaryHeap;

struct Event<M> {
    delivery_time: Instant,
    message: M,
    destination: NodeId,
}

struct Simulator<A: Actor> {
    nodes: HashMap<NodeId, A>,
    events: BinaryHeap<Event<A::Message>>,
    current_time: Instant,
    rng: Pcg64,
}

impl<A: Actor> Simulator<A> {
    fn step(&mut self) -> Option<(NodeId, A::Response)> {
        let event = self.events.pop()?;
        self.current_time = event.delivery_time;
        
        let node = self.nodes.get_mut(&event.destination)?;
        let responses = node.receive(event.message, self.current_time);
        
        for (msg, dest) in responses {
            // Deterministically assign delivery time
            let delay = self.random_delay();
            let should_deliver = self.rng.gen_bool(1.0 - self.drop_rate);
            
            if should_deliver {
                self.events.push(Event {
                    delivery_time: self.current_time + delay,
                    message: msg,
                    destination: dest,
                });
            }
        }
        
        None
    }
    
    fn random_delay(&mut self) -> Duration {
        let micros = self.rng.gen_range(100..10_000);
        Duration::from_micros(micros)
    }
}
```

**Benefits of manual approach:**
- Total control over simulation behavior
- Minimal dependencies
- Easy to extend with custom failure modes
- Natural separation of pure logic from I/O

---

## Design Patterns for DST

### 1. Message Bus Architecture

Centralize all inter-component communication through a single abstraction:

```rust
/// All messages go through the bus
trait MessageBus {
    fn send(&mut self, to: NodeId, msg: Message);
    fn receive(&mut self) -> Option<(NodeId, Message)>;
    fn current_time(&self) -> Instant;
}

/// Production implementation
struct TcpMessageBus { /* real TCP */ }

/// Simulation implementation
struct SimulatedMessageBus {
    queue: BinaryHeap<Event>,
    rng: Pcg64,
    time: SimulatedTime,
}
```

### 2. Time Abstraction

Never call `Instant::now()` directly in business logic:

```rust
/// Time provider trait
trait Clock {
    fn now(&self) -> Instant;
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()>;
}

/// Production implementation
struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
    
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await
    }
}

/// Simulation implementation
struct SimulatedClock {
    time: Arc<AtomicU64>,  // Controlled by simulator
}
```

### 3. Deterministic State Machine

Structure core logic as pure state transitions:

```rust
/// Pure state machine - no I/O, no randomness
struct RaftNode {
    state: NodeState,
    log: Vec<LogEntry>,
    // ...
}

impl RaftNode {
    /// Pure function: old state + input → new state + outputs
    fn handle_message(
        &mut self,
        msg: RaftMessage,
        ctx: &mut MessageContext,
    ) -> Vec<RaftMessage> {
        match (self.state, msg) {
            (NodeState::Follower, RaftMessage::RequestVote { .. }) => {
                // Pure logic, no I/O
            }
            // ...
        }
    }
}

/// Context provides controlled randomness
struct MessageContext<'a> {
    rng: &'a mut Pcg64,
    time: Instant,
}
```

### 4. Feature-Flagged Implementations

```rust
#[cfg(not(madsim))]
mod real_impl {
    pub fn current_time() -> Instant {
        Instant::now()
    }
    
    pub async fn sleep(duration: Duration) {
        tokio::time::sleep(duration).await
    }
}

#[cfg(madsim)]
mod sim_impl {
    pub fn current_time() -> Instant {
        madsim::time::Instant::now()
    }
    
    pub async fn sleep(duration: Duration) {
        madsim::time::sleep(duration).await
    }
}

// Re-export the appropriate implementation
#[cfg(not(madsim))]
pub use real_impl::*;
#[cfg(madsim)]
pub use sim_impl::*;
```

---

## Property-Based Testing with DST

Combine DST with property testing for maximum bug discovery:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn linearizability(
        seed: u64,
        operations in prop::collection::vec(operation_strategy(), 1..100),
        partitions in prop::collection::vec(partition_strategy(), 0..10),
    ) {
        let mut sim = Builder::new()
            .rng_seed(seed)
            .build();
        
        // Apply operations and partitions
        for op in operations {
            apply_operation(&mut sim, op);
        }
        
        // Check linearizability
        let history = sim.collect_history();
        assert!(is_linearizable(&history));
    }
}
```

---

## Common Pitfalls

### 1. Hidden Sources of Non-Determinism

```rust
// ❌ HashMap iteration is non-deterministic
for (k, v) in &hash_map {
    process(k, v);  // Order affects outcomes
}

// ✅ Use BTreeMap or sort keys
for (k, v) in &btree_map {
    process(k, v);  // Consistent ordering
}
```

### 2. System Thread Spawning

```rust
// ❌ Spawns OS thread, escapes simulation control
std::thread::spawn(|| { /* ... */ });

// ✅ Use async tasks under simulator control
tokio::spawn(async { /* ... */ });

// madsim explicitly blocks system threads:
// MADSIM_ALLOW_SYSTEM_THREAD=1 to override (use carefully)
```

### 3. Forgetting to Patch Dependencies

```rust
// If any dependency calls getrandom or accesses real time,
// your simulation is not deterministic!

// Check with:
MADSIM_TEST_CHECK_DETERMINISM=1 RUSTFLAGS="--cfg madsim" cargo test
```

### 4. Unbounded Loops Without Yields

```rust
// ❌ Simulator cannot make progress
loop {
    if condition { break; }
    // No await point - never yields to scheduler
}

// ✅ Yield to allow other tasks to run
loop {
    if condition { break; }
    tokio::task::yield_now().await;
}
```

### 5. Memory Address Dependent Logic

```rust
// ❌ Pointer addresses are non-deterministic
let ptr = &value as *const _ as usize;
let hash = ptr % 1024;

// ✅ Use deterministic identifiers
let id = self.next_id.fetch_add(1, Ordering::SeqCst);
```

---

## Testing Strategy

### Phased Approach

1. **Unit Tests** — Fast, focused, deterministic
2. **DST Integration Tests** — Explore concurrent behaviors
3. **Long-Running Fuzzing** — Nightly random exploration
4. **Determinism Verification** — CI checks for reproducibility

### CI Configuration

```yaml
# .github/workflows/dst.yml
jobs:
  dst-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Run DST tests
        run: RUSTFLAGS="--cfg madsim" cargo test --release
        
      - name: Check determinism
        run: |
          MADSIM_TEST_CHECK_DETERMINISM=1 \
          RUSTFLAGS="--cfg madsim" \
          cargo test --release

  dst-fuzzing:
    runs-on: ubuntu-latest
    steps:
      - name: Long-running random exploration
        run: |
          for i in $(seq 1 100); do
            MADSIM_TEST_SEED=$RANDOM \
            RUSTFLAGS="--cfg madsim" \
            cargo test --release
          done
```

---

## Resources

### Primary References

- [FoundationDB: Testing Distributed Systems w/ Deterministic Simulation](https://www.youtube.com/watch?v=4fFDFbi3toc) — Original DST talk
- [sled Simulation Guide](https://sled.rs/simulation.html) — Actor-based DST patterns
- [madsim Documentation](https://docs.rs/madsim) — API reference
- [turmoil Documentation](https://docs.rs/turmoil) — API reference

### Blog Posts

- [Deterministic Simulation: A New Era (Part 1)](https://www.risingwave.com/blog/deterministic-simulation-a-new-era-of-distributed-system-testing/) — Madsim architecture
- [Applying DST: The RisingWave Story (Part 2)](https://www.risingwave.com/blog/applying-deterministic-simulation-the-risingwave-story-part-2-of-2/) — Production usage patterns

### Related Tools

- **loom** — Exhaustive concurrency testing (see [loom-testing.md](loom-testing.md))
- **shuttle** — Randomized concurrency testing
- **tokio-test** — Lightweight time mocking for tokio

---

## Quick Reference

### When to Use Each Tool

| Scenario | Tool |
|----------|------|
| Drop-in tokio replacement for large systems | **madsim** |
| Simple network simulation | **turmoil** |
| Maximum control over simulation | **Manual (sled-style)** |
| Lock-free data structures | **loom** |
| Quick time mocking in unit tests | **tokio-test** |

### Madsim Quick Commands

```bash
# Run tests in simulation mode
RUSTFLAGS="--cfg madsim" cargo test

# Reproduce a failure
MADSIM_TEST_SEED=12345 RUSTFLAGS="--cfg madsim" cargo test test_name

# Verify determinism
MADSIM_TEST_CHECK_DETERMINISM=1 RUSTFLAGS="--cfg madsim" cargo test

# Limit simulation time
MADSIM_TEST_TIME_LIMIT=30 RUSTFLAGS="--cfg madsim" cargo test
```

### Turmoil Quick Pattern

```rust
#[test]
fn my_test() -> turmoil::Result {
    let mut sim = turmoil::Builder::new()
        .rng_seed(42)
        .build();
    
    sim.host("server", || async { /* ... */ Ok(()) });
    sim.client("client", async { /* ... */ Ok(()) });
    
    sim.run()
}
```
