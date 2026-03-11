<!-- CATEGORY: Testing -->
<!-- WHEN: Fuzz testing, cargo-fuzz, writing fuzz harnesses, crash analysis -->

# Fuzz Testing

## Quick Start

```bash
rustup install nightly
cargo +nightly install cargo-fuzz
cargo +nightly fuzz init
cargo +nightly fuzz add my_target
cargo +nightly fuzz run my_target
cargo +nightly fuzz run my_target -- -max_total_time=300  # CI-friendly
cargo +nightly fuzz run my_target -- -runs=0              # Corpus regression
cargo +nightly fuzz cmin my_target                         # Minimize corpus
```

## Fuzzer Comparison

| Tool | Best For | Setup | Speed |
|------|----------|-------|-------|
| **cargo-fuzz** | Library fuzzing, OSS-Fuzz | Low | Fast |
| **AFL.rs** | Detailed TUI, stdin targets | Medium | Medium |
| **honggfuzz-rs** | Hardware feedback (Intel PT) | Medium | Fast |
| **LibAFL** | Custom fuzzers, distributed | High | Very Fast |

Recommendation: Start with **cargo-fuzz**.

## Project Structure

```
fuzz/
  Cargo.toml           # cargo-fuzz = true, libfuzzer-sys, arbitrary
  fuzz_targets/
    target1.rs          # Fuzz targets
  corpus/target1/       # Seed inputs
  artifacts/target1/    # Crash files
```

## Writing Fuzz Targets

### Basic Template

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = my_crate::parse(data);
});
```

### Rules
1. Must tolerate ANY input (empty, huge, malformed)
2. Must NOT panic on valid paths
3. Must be deterministic
4. Must be fast (avoid O(n^3), excessive allocations)
5. Should not accumulate global state

### Structured Fuzzing with Arbitrary

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
impl<'a> Arbitrary<'a> for BoundedInput {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let size = u.int_in_range(1..=1000)?;
        let len = u.int_in_range(0..=64)?;
        let items = (0..len).map(|_| Item::arbitrary(u)).collect::<arbitrary::Result<Vec<_>>>()?;
        Ok(BoundedInput { size, items })
    }
}
```

## Effective Patterns

### Round-Trip Testing (most effective)
```rust
fuzz_target!(|input: MyType| {
    let encoded = encode(&input);
    // Fuzz targets: panic signals a bug to the fuzzer
    let decoded = decode(&encoded).expect("round-trip decode failed");
    assert_eq!(input, decoded);
});
```

### Differential Testing
```rust
fuzz_target!(|data: &[u8]| {
    let a = reference_impl::parse(data);
    let b = optimized_impl::parse(data);
    assert_eq!(a.is_ok(), b.is_ok());
    if let (Ok(a), Ok(b)) = (a, b) { assert_eq!(a, b); }
});
```

### State Machine Fuzzing
```rust
fuzz_target!(|ops: Vec<SessionOp>| {
    let mut session = Session::new();
    for op in ops.iter().take(1000) {  // Bound iterations!
        match op { /* ... */ }
    }
});
```

## Avoiding Common Pitfalls

```rust
fuzz_target!(|input: FuzzInput| {
    if input.data.len() > 10_000 { return; }           // Prevent OOM
    let size = (input.size as usize).min(MAX_SIZE);     // Clamp allocations
    let bounded: Vec<_> = input.items.into_iter().take(1000).collect(); // Bound collections
});
```

Use `#[cfg(fuzzing)]` to disable expensive operations (crypto verification, etc.) during fuzzing.

## LibFuzzer Options

| Option | Default | Description |
|--------|---------|-------------|
| `-max_len=N` | auto | Maximum input length |
| `-max_total_time=N` | 0 (infinite) | Total fuzzing time (seconds) |
| `-runs=N` | -1 (infinite) | Number of test runs |
| `-timeout=N` | 1200 | Per-input timeout (seconds) |
| `-rss_limit_mb=N` | 2048 | Memory limit (MB) |
| `-dict=FILE` | none | Dictionary file |

## Crash Handling

```bash
# Reproduce
cargo +nightly fuzz run target fuzz/artifacts/target/crash-xxx

# Minimize
cargo +nightly fuzz tmin target fuzz/artifacts/target/crash-xxx

# Format output
cargo +nightly fuzz fmt target fuzz/artifacts/target/crash-xxx
```

### Convert to Regression Test

```rust
#[test]
fn regression_crash_abc123() {
    let data = include_bytes!("../fuzz/artifacts/target/crash-abc123");
    let _ = my_function(data);  // Should not panic
}
```

## Corpus Management

- Seed with valid sample inputs: `cp test_data/*.bin fuzz/corpus/my_target/`
- Minimize periodically: `cargo +nightly fuzz cmin my_target`
- Commit crash reproducers to `fuzz/corpus/`
- For large corpus (>50MB), use artifact storage

## CI Integration

```yaml
jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: rustup toolchain install nightly && rustup default nightly
      - run: cargo install cargo-fuzz
      - uses: actions/cache@v4
        with:
          path: fuzz/corpus
          key: fuzz-corpus-${{ github.ref }}-${{ hashFiles('fuzz/**/*.rs') }}
      - name: Corpus regression
        run: |
          for target in $(cargo fuzz list); do
            cargo fuzz run $target -- -runs=0
          done
      - name: Extended fuzzing (scheduled only)
        if: github.event_name == 'schedule'
        run: |
          for target in $(cargo fuzz list); do
            cargo fuzz run $target -- -max_total_time=300
          done
      - uses: actions/upload-artifact@v4
        if: failure()
        with: { name: fuzz-crashes, path: fuzz/artifacts/ }
```

## High-Value Targets

| Target Type | Common Bugs |
|-------------|-------------|
| Parsers/Deserializers | panic, out-of-range, arithmetic overflow |
| Codec/Compression | OOM, infinite loop, out-of-range |
| Network Protocols | arithmetic, panic, out-of-range |
| State Machines | logic errors, unwrap, infinite loop |
