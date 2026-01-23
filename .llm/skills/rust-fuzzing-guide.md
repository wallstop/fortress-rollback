# Rust Fuzzing Guide — Comprehensive Best Practices

> **This guide covers fuzz testing for Rust projects using cargo-fuzz, LibAFL, and other tools.**
> Use this guide when adding, improving, or troubleshooting fuzz targets.

## TL;DR — Quick Start

```bash
# Install (requires nightly Rust)
rustup install nightly
cargo +nightly install cargo-fuzz

# Initialize fuzzing in project
cargo +nightly fuzz init

# Add a fuzz target
cargo +nightly fuzz add my_target

# Run fuzzing
cargo +nightly fuzz run my_target

# Run with time limit (CI-friendly)
cargo +nightly fuzz run my_target -- -max_total_time=300

# Run corpus regression test
cargo +nightly fuzz run my_target -- -runs=0

# Minimize corpus
cargo +nightly fuzz cmin my_target
```

---

## Why Fuzz Rust Code?

**Rust's safety doesn't eliminate all bugs.** Fuzzing finds:

- **Logic errors** — incorrect algorithm behavior
- **Panics** — `unwrap()`, `expect()`, array bounds, arithmetic overflow
- **Denial of service** — infinite loops, OOM, stack overflow
- **Unsafe code bugs** — memory corruption in `unsafe` blocks

**The [rust-fuzz trophy case](https://github.com/rust-fuzz/trophy-case) documents 445+ bugs found by fuzzing**, including critical security vulnerabilities in popular crates like `brotli-rs`, `png`, `hyper`, and `regex`.

---

## Choosing a Fuzzer

| Tool | Best For | Setup Complexity | Speed |
|------|----------|------------------|-------|
| **cargo-fuzz** | Quick library fuzzing, OSS-Fuzz | Low | Fast |
| **AFL.rs** | Detailed TUI, stdin-based targets | Medium | Medium |
| **honggfuzz-rs** | Hardware feedback (Intel PT) | Medium | Fast |
| **LibAFL** | Custom fuzzers, binary-only, distributed | High | Very Fast |

**Recommendation:** Start with **cargo-fuzz** — it's the Rust ecosystem standard with excellent OSS-Fuzz integration.

---

## Project Structure

```
project/
├── Cargo.toml           # Main crate
├── src/
└── fuzz/
    ├── Cargo.toml       # Fuzz crate config
    ├── fuzz_targets/
    │   ├── target1.rs   # Fuzz targets
    │   └── target2.rs
    ├── corpus/
    │   ├── target1/     # Seed inputs
    │   └── target2/
    └── artifacts/
        ├── target1/     # Crash files
        └── target2/
```

### fuzz/Cargo.toml Structure

```toml
[package]
name = "my-crate-fuzz"
version = "0.0.0"
publish = false
edition = "2021"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"
arbitrary = { version = "1", features = ["derive"] }

[dependencies.my-crate]
path = ".."

# Each fuzz target needs a [[bin]] entry
[[bin]]
name = "fuzz_parser"
path = "fuzz_targets/fuzz_parser.rs"
test = false
doc = false
bench = false
```

---

## What to Fuzz — High-Value Targets

Based on trophy case analysis, prioritize these target types:

| Target Type | Bug Categories Found | Examples |
|-------------|---------------------|----------|
| **Parsers/Deserializers** | `panic`, `oor`, `arith`, `utf-8` | JSON, TOML, protocol buffers |
| **Codec/Compression** | `oom`, `loop`, `oor` | Image decoders, compression libs |
| **Network Protocols** | `arith`, `panic`, `oor` | HTTP, WebSocket, custom protocols |
| **State Machines** | `logic`, `unwrap`, `loop` | Workflow engines, protocol handlers |
| **Cryptographic Formats** | `arith`, `oor` | Key parsing, certificate handling |

### Common Bug Categories

- **`arith`** — Arithmetic overflow (most common!)
- **`oor`** — Out of range/bounds access
- **`panic`/`unwrap`** — Unhandled errors
- **`oom`** — Memory exhaustion
- **`loop`** — Infinite loops
- **`utf-8`** — Invalid string handling
- **`so`** — Stack overflow from recursion

---

## Writing Fuzz Targets

### Basic Structure

```rust
#![no_main]  // Required: libfuzzer provides main()

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Your fuzzing code here
    // Must handle ANY input without panicking (except to find bugs)
    let _ = my_crate::parse(data);
});
```

### Critical Rules for Fuzz Targets

1. **Must tolerate ANY input** — empty, huge, malformed
2. **Must NOT panic on valid paths** — only panic to signal bugs
3. **Must be deterministic** — no randomness outside input bytes
4. **Must be fast** — avoid O(n³) complexity, excessive allocations
5. **Should not accumulate global state** — reset between iterations
6. **Narrower is better** — one target per format/protocol

### Raw Bytes vs Structured Inputs

**Use raw `&[u8]` when:**

- Testing parsers/deserializers directly
- Need exact byte-level control
- Input format is inherently bytes

**Use structured inputs when:**

- Testing business logic after parsing
- Multiple parameters needed
- State machine fuzzing
- Need semantic validity

---

## Structured Fuzzing with Arbitrary

The `arbitrary` crate generates well-typed inputs from fuzzer bytes:

### Derive Macro Usage

```rust
use arbitrary::Arbitrary;

#[derive(Debug, Arbitrary)]
enum Operation {
    Add { key: String, value: Vec<u8> },
    Remove { key: String },
    Lookup { key: String },
}

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    initial_size: u8,
    operations: Vec<Operation>,
}

fuzz_target!(|input: FuzzInput| {
    let mut map = MyMap::with_capacity(input.initial_size as usize);
    for op in &input.operations {
        match op {
            Operation::Add { key, value } => { map.insert(key, value); }
            Operation::Remove { key } => { map.remove(key); }
            Operation::Lookup { key } => { let _ = map.get(key); }
        }
    }
});
```

### Custom Arbitrary for Constrained Values

```rust
use arbitrary::{Arbitrary, Result, Unstructured};

struct BoundedInput {
    size: usize,           // Constrained to 1..=1000
    items: Vec<Item>,      // Max 64 items
}

impl<'a> Arbitrary<'a> for BoundedInput {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        // Constrain size to prevent OOM/timeouts
        let size = u.int_in_range(1..=1000)?;

        // Bound collection length
        let len = u.int_in_range(0..=64)?;
        let items = (0..len)
            .map(|_| Item::arbitrary(u))
            .collect::<Result<Vec<_>>>()?;

        Ok(BoundedInput { size, items })
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        // Help fuzzer estimate byte consumption
        let (item_min, item_max) = Item::size_hint(depth);
        (2, item_max.map(|m| 2 + 64 * m))
    }
}
```

### Key Unstructured Methods

| Method | Use Case |
|--------|----------|
| `u.arbitrary::<T>()` | Generate any Arbitrary type |
| `u.int_in_range(0..=100)` | Constrained integer |
| `u.arbitrary_len::<T>()` | Smart collection length |
| `u.choose(&[a, b, c])` | Pick from options |
| `u.ratio(1, 10)` | Boolean with probability |
| `u.bytes(n)` | Raw byte slice |

---

## Effective Fuzzing Patterns

### Pattern 1: Round-Trip Testing

**Most effective pattern** — found bugs in 30+ crates:

```rust
fuzz_target!(|input: MyType| {
    let encoded = encode(&input);
    let decoded = decode(&encoded).expect("round-trip decode failed");
    assert_eq!(input, decoded, "round-trip mismatch");
});
```

### Pattern 2: Differential Testing

Compare two implementations:

```rust
fuzz_target!(|data: &[u8]| {
    let result_a = reference_impl::parse(data);
    let result_b = optimized_impl::parse(data);
    assert_eq!(result_a.is_ok(), result_b.is_ok());
    if let (Ok(a), Ok(b)) = (result_a, result_b) {
        assert_eq!(a, b);
    }
});
```

### Pattern 3: State Machine Fuzzing

```rust
#[derive(Debug, Arbitrary)]
enum SessionOp {
    Connect,
    SendMessage { data: Vec<u8> },
    ReceiveMessage,
    Disconnect,
}

fuzz_target!(|ops: Vec<SessionOp>| {
    let mut session = Session::new();
    for op in ops.iter().take(1000) {  // Bound iterations!
        match op {
            SessionOp::Connect => { let _ = session.connect(); }
            SessionOp::SendMessage { data } => { let _ = session.send(data); }
            SessionOp::ReceiveMessage => { let _ = session.receive(); }
            SessionOp::Disconnect => { session.disconnect(); }
        }
    }
});
```

### Pattern 4: Bounded Decompression

Prevent OOM in decompression targets:

```rust
fuzz_target!(|data: &[u8]| {
    // Limit output size
    let mut output = vec![0u8; 1_000_000]; // Max 1MB
    let _ = decompress(data, &mut output);
});
```

---

## Avoiding Common Pitfalls

### Preventing OOM

```rust
fuzz_target!(|input: FuzzInput| {
    // Reject huge inputs early
    if input.data.len() > 10_000 {
        return;
    }

    // Clamp allocation sizes
    let size = (input.requested_size as usize).min(MAX_REASONABLE_SIZE);

    // Use bounded containers
    let bounded: Vec<_> = input.items.into_iter().take(1000).collect();
});
```

### Preventing Timeouts

```rust
const MAX_ITERATIONS: usize = 10_000;

fuzz_target!(|input: FuzzInput| {
    for (i, op) in input.operations.iter().enumerate() {
        if i >= MAX_ITERATIONS {
            break;  // Prevent timeout
        }
        process(op);
    }
});
```

### Preventing Stack Overflow

```rust
// Set recursion limits in parsers
fn process_recursive(data: &Data, depth: usize) {
    if depth > 100 {
        return;  // Prevent stack overflow
    }
    for child in &data.children {
        process_recursive(child, depth + 1);
    }
}
```

### Ensuring Determinism

```rust
// DON'T: Non-deterministic
let random_val = rand::random::<u32>();

// DO: Derive from input
let seed = u64::from_le_bytes(data[..8].try_into().unwrap_or([0; 8]));
let mut rng = StdRng::seed_from_u64(seed);
```

### Using `#[cfg(fuzzing)]`

Disable expensive operations during fuzzing:

```rust
#[cfg(not(fuzzing))]
fn verify_signature(data: &[u8], sig: &[u8]) -> bool {
    // Expensive crypto verification
}

#[cfg(fuzzing)]
fn verify_signature(_data: &[u8], _sig: &[u8]) -> bool {
    true  // Skip during fuzzing
}
```

---

## LibFuzzer Options Reference

| Option | Default | Description |
|--------|---------|-------------|
| `-max_len=N` | auto | Maximum input length |
| `-max_total_time=N` | 0 (infinite) | Total fuzzing time (seconds) |
| `-runs=N` | -1 (infinite) | Number of test runs |
| `-timeout=N` | 1200 | Per-input timeout (seconds) |
| `-rss_limit_mb=N` | 2048 | Memory limit (MB) |
| `-jobs=N` | 0 | Parallel fuzzing jobs |
| `-dict=FILE` | none | Dictionary file |
| `-only_ascii=1` | 0 | Generate ASCII inputs only |
| `-use_value_profile=1` | 0 | Enhanced coverage tracking |
| `-fork=N` | 0 | Fork mode (crash resistant) |

### Common Command Patterns

```bash
# CI regression test (fast)
cargo +nightly fuzz run target -- -runs=0

# CI short fuzz (5 minutes)
cargo +nightly fuzz run target -- -max_total_time=300

# Development fuzzing (verbose)
cargo +nightly fuzz run target -- -V

# Minimize a crash
cargo +nightly fuzz tmin target artifacts/target/crash-xxx

# Generate coverage report
cargo +nightly fuzz coverage target
```

---

## Corpus Management

### Seeding the Corpus

Provide valid sample inputs:

```bash
mkdir -p fuzz/corpus/my_target
cp test_data/*.bin fuzz/corpus/my_target/
```

### Corpus Minimization

Remove redundant inputs while preserving coverage:

```bash
cargo +nightly fuzz cmin my_target
```

### Merging Corpora

```bash
# Add interesting inputs from NEW_INPUTS to existing corpus
cargo +nightly fuzz run target -- -merge=1 fuzz/corpus/target NEW_INPUTS/
```

### Version Control Strategy

| Scenario | Recommendation |
|----------|----------------|
| Small corpus (<50MB) | Commit to `fuzz/corpus/` |
| Large corpus (>50MB) | Use artifact storage or separate repo |
| Crash reproducers | **Always commit** to `fuzz/corpus/` |
| Minimized crashes | Commit as regression tests |

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: Fuzz Testing
on:
  pull_request:
  push:
    branches: [main]
  schedule:
    - cron: '0 0 * * *'  # Daily batch fuzzing

jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install nightly Rust
        run: |
          rustup toolchain install nightly
          rustup default nightly

      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz

      - name: Restore corpus cache
        uses: actions/cache@v4
        with:
          path: fuzz/corpus
          key: fuzz-corpus-${{ github.ref }}-${{ hashFiles('fuzz/**/*.rs') }}
          restore-keys: |
            fuzz-corpus-${{ github.ref }}-
            fuzz-corpus-main-

      - name: Build fuzz targets
        run: cargo fuzz build

      - name: Run corpus regression
        run: |
          for target in $(cargo fuzz list); do
            echo "Testing $target"
            cargo fuzz run $target -- -runs=0
          done

      - name: Extended fuzzing (scheduled only)
        if: github.event_name == 'schedule'
        run: |
          for target in $(cargo fuzz list); do
            echo "Fuzzing $target"
            cargo fuzz run $target -- -max_total_time=300
          done

      - name: Upload crash artifacts
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: fuzz-crashes
          path: fuzz/artifacts/
          retention-days: 30
```

### Two-Tier Strategy

| Tier | Trigger | Duration | Purpose |
|------|---------|----------|---------|
| **PR Fuzzing** | Pull requests | 5-10 min | Regression testing |
| **Batch Fuzzing** | Cron schedule | 30-60 min | Deep exploration |

---

## Crash Handling

### Reproduction

```bash
# Reproduce locally
cargo +nightly fuzz run target fuzz/artifacts/target/crash-xxx

# Get formatted output
cargo +nightly fuzz fmt target fuzz/artifacts/target/crash-xxx
```

### Minimization

```bash
cargo +nightly fuzz tmin target fuzz/artifacts/target/crash-xxx
```

### Converting to Regression Test

```rust
#[test]
fn regression_crash_abc123() {
    let data = include_bytes!("../fuzz/artifacts/target/crash-abc123");
    // Should not panic
    let _ = my_function(data);
}
```

---

## Advanced Topics

### OSS-Fuzz Integration

For open-source projects with wide usage:

**project.yaml:**

```yaml
language: rust
sanitizers:
  - address
fuzzing_engines:
  - libfuzzer
```

**Dockerfile:**

```dockerfile
FROM gcr.io/oss-fuzz-base/base-builder-rust
RUN git clone --depth 1 https://github.com/your/project
```

### LibAFL for Custom Fuzzers

When you need:

- Custom mutators for domain-specific inputs
- Distributed fuzzing across machines
- Binary-only fuzzing (via Frida/QEMU)
- Novel feedback mechanisms

```rust
// LibAFL provides building blocks for custom fuzzers
use libafl::prelude::*;
// See https://github.com/AFLplusplus/LibAFL/tree/main/fuzzers
```

### Honggfuzz-rs for Hardware Feedback

```bash
cargo install honggfuzz
cargo hfuzz run target_name
```

Unique features:

- Intel PT hardware tracing
- Different mutation strategies than libFuzzer
- May find bugs missed by other fuzzers

---

## Measuring Effectiveness

### Coverage Metrics

```bash
# Generate coverage
cargo +nightly fuzz coverage my_target

# View report
cargo cov -- show fuzz/target/*/release/my_target \
    --format=html \
    -instr-profile=fuzz/coverage/my_target/coverage.profdata \
    > coverage.html
```

### LibFuzzer Output Indicators

- **`cov:`** — Code blocks covered (should grow initially)
- **`ft:`** — Features discovered (edges + counters)
- **`corp:`** — Corpus size and total bytes
- **`exec/s`** — Executions per second (aim for 100+)
- **`NEW`** — New coverage found (good sign!)

### When Fuzzing is "Done"

Diminishing returns indicators:

- No `NEW` coverage events for extended period
- Corpus size stable after minimization
- Coverage report shows all reachable branches hit

**Realistic expectations:**

- First bugs: minutes to hours
- Deep bugs: days of continuous fuzzing
- Some paths may be mutation-unreachable

---

## Troubleshooting

### "error: the option `Z` is only accepted on the nightly compiler"

```bash
rustup default nightly
# Or use: cargo +nightly fuzz run
```

### Timeouts During Fuzzing

- Add iteration limits in fuzz target
- Use `-timeout=10` for tighter per-input limits
- Check for infinite loops in code

### Out of Memory

- Bound input sizes: `if data.len() > LIMIT { return; }`
- Use `-rss_limit_mb=1024` for stricter limits
- Check for unbounded allocations

### Crashes in std/allocator

Usually indicates memory corruption in `unsafe` code. Use:

```bash
RUSTFLAGS="-Zsanitizer=address" cargo +nightly fuzz run target
```

### Low Executions Per Second

- Fuzz target may be too slow (O(n³), I/O-bound)
- Consider fuzzing smaller units
- Use `lazy_static` for expensive initialization

---

## Summary Checklist

### When Adding Fuzzing to a Project

- [ ] Install nightly Rust and cargo-fuzz
- [ ] Run `cargo fuzz init`
- [ ] Add targets for parsers, codecs, and protocol handlers first
- [ ] Use `#[derive(Arbitrary)]` for structured inputs
- [ ] Bound all inputs to prevent OOM/timeouts
- [ ] Seed corpus with valid sample inputs
- [ ] Set up CI regression testing
- [ ] Commit corpus to repository

### When Writing a Fuzz Target

- [ ] Use `#![no_main]` attribute
- [ ] Handle ANY input without panicking (except to find bugs)
- [ ] Bound iterations and allocations
- [ ] Ensure deterministic execution
- [ ] Reset state between iterations
- [ ] Use structured inputs for state machine testing

### When Maintaining Fuzz Targets

- [ ] Run corpus regression on every PR
- [ ] Schedule extended fuzzing runs
- [ ] Periodically minimize corpus
- [ ] Convert crashes to regression tests
- [ ] Update seed corpus with new valid inputs

---

## Resources

| Resource | URL |
|----------|-----|
| Rust Fuzz Book | <https://rust-fuzz.github.io/book/> |
| Trophy Case | <https://github.com/rust-fuzz/trophy-case> |
| cargo-fuzz | <https://github.com/rust-fuzz/cargo-fuzz> |
| arbitrary crate | <https://docs.rs/arbitrary> |
| LibAFL | <https://github.com/AFLplusplus/LibAFL> |
| OSS-Fuzz | <https://google.github.io/oss-fuzz/> |
| libFuzzer docs | <https://llvm.org/docs/LibFuzzer.html> |

---

*This guide is part of the Fortress Rollback LLM skill set for agentic code assistance.*
