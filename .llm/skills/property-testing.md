# Property-Based Testing — Finding Edge Cases Automatically in Rust

> **This document provides comprehensive guidance for property-based testing in Rust.**
> Property testing generates thousands of random inputs to find edge cases that hand-written tests miss.

## TL;DR — Quick Start

```bash
# Add proptest to Cargo.toml
[dev-dependencies]
proptest = "1.9"

# Run property tests
cargo test

# Run specific property test
cargo test prop_roundtrip

# View regression file after failure
cat proptest-regressions/<module_name>.txt
```

**Key insight**: Instead of testing `f(5) == expected_output`, property testing verifies that `f` satisfies invariants like `decode(encode(x)) == x` for *all* possible inputs.

---

## What is Property-Based Testing?

Property-based testing verifies that **general properties hold for all inputs**, not just specific examples.

### Unit Tests vs Property Tests

```rust
// ❌ Unit test: Tests ONE specific case
#[test]
fn test_sort_specific() {
    let input = vec![3, 1, 2];
    let output = sort(&input);
    assert_eq!(output, vec![1, 2, 3]);
}

// ✅ Property test: Verifies ALL inputs satisfy the property
proptest! {
    #[test]
    fn prop_sort_is_sorted(input in any::<Vec<i32>>()) {
        let output = sort(&input);
        // Property: output is always sorted
        prop_assert!(output.windows(2).all(|w| w[0] <= w[1]));
        // Property: output has same elements as input
        prop_assert_eq!(output.len(), input.len());
    }
}
```

### Key Benefits

| Benefit | Description |
|---------|-------------|
| **Edge Case Discovery** | Finds inputs you didn't think to test (empty, max values, unicode) |
| **Property Verification** | Proves invariants hold for the entire input space |
| **Automatic Shrinking** | Minimizes failing inputs to smallest reproducible case |
| **Regression Tracking** | Failing cases are saved and replayed automatically |

### When to Use Property Testing

✅ **Excellent for:**

- Serialization/deserialization (roundtrip properties)
- Encoders/decoders, compression algorithms
- Data structures (sorted, invariants maintained)
- Parsers (valid input produces valid output)
- Mathematical functions (commutativity, associativity)
- State machines (transitions preserve invariants)
- Protocol implementations

❌ **NOT ideal for:**

- Simple validation with specific expected outputs
- Tests requiring specific I/O behavior
- UI/integration tests with external dependencies
- Tests where *any* output is valid (no properties to check)

---

## Library Ecosystem in Rust

### Comparison Table

| Library | Best For | Complexity | Shrinking | Notes |
|---------|----------|------------|-----------|-------|
| **proptest** | General property testing | Medium | Excellent | Most popular, powerful strategies |
| **quickcheck** | Simple property tests | Low | Good | Simpler API, less flexible |
| **arbitrary** | Fuzzing interop | Low | None | Generates arbitrary values, pairs with arbtest |
| **test-strategy** | Better ergonomics | Low | (uses proptest) | Attribute macros over proptest |
| **proptest-stateful** | State machine testing | High | Excellent | Sequences of operations |

### Recommended: **proptest**

For this project, we use **proptest** because:

- Rich strategy combinators for custom types
- Excellent shrinking finds minimal failing cases
- Regression file support for CI reproducibility
- Active maintenance and community support

```toml
[dev-dependencies]
proptest = "1.9"
proptest-derive = "0.5"  # Optional: #[derive(Arbitrary)]
```

---

## Getting Started with Proptest

### Basic Usage

```rust
use proptest::prelude::*;

proptest! {
    /// Property: Adding zero doesn't change the value
    #[test]
    fn prop_add_zero_identity(x in any::<i32>()) {
        prop_assert_eq!(x + 0, x);
    }

    /// Property: Addition is commutative
    #[test]
    fn prop_addition_commutative(a in any::<i32>(), b in any::<i32>()) {
        // Use wrapping_add to avoid overflow panic
        prop_assert_eq!(a.wrapping_add(b), b.wrapping_add(a));
    }
}
```

### The `prop_assert!` Family

```rust
proptest! {
    #[test]
    fn example_assertions(x in 0..100i32, s in ".*") {
        // Boolean assertion
        prop_assert!(x >= 0);

        // Equality assertion
        prop_assert_eq!(x * 2, x + x);

        // Inequality assertion
        prop_assert_ne!(x, -1);

        // With custom message
        prop_assert!(
            s.len() < 10000,
            "String too long: {} chars",
            s.len()
        );
    }
}
```

### Regression Files

When a test fails, proptest saves the failing input to `proptest-regressions/`:

```text
# Shrunk: false
# Seed: 12345
cc 0x0 0x1 0x2
```

**Always commit regression files to version control!** They ensure:

- CI catches the same failures
- Fixed bugs stay fixed
- Team members reproduce issues

```bash
# Add to .gitignore exceptions
!proptest-regressions/
```

---

## Strategies In-Depth

Strategies define *how* to generate test inputs.

### Built-in Strategies

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn example_strategies(
        // Primitives
        b in any::<bool>(),
        byte in any::<u8>(),
        i in any::<i64>(),
        f in any::<f32>(),
        c in any::<char>(),

        // Ranges
        small in 0..100i32,
        range in -50..=50i32,

        // Strings
        s in ".*",                           // Any string
        ascii in "[a-zA-Z0-9]+",            // Regex pattern
        sized in ".{1,100}",                // Bounded length

        // Collections
        vec in prop::collection::vec(any::<u8>(), 0..100),
        hash in prop::collection::hash_set(any::<i32>(), 0..50),

        // Options and Results
        opt in any::<Option<u32>>(),
        res in any::<Result<i32, bool>>(),
    ) {
        // Test using generated values
        prop_assert!(vec.len() <= 100);
    }
}
```

### Strategy Combinators

```rust
use proptest::prelude::*;

// prop_map: Transform generated values
fn even_numbers() -> impl Strategy<Value = i32> {
    any::<i32>().prop_map(|n| n * 2)
}

// prop_filter: Reject invalid values (use sparingly!)
fn positive_numbers() -> impl Strategy<Value = i32> {
    any::<i32>().prop_filter("must be positive", |&n| n > 0)
}

// prop_flat_map: Generate dependent values
fn vec_with_valid_index() -> impl Strategy<Value = (Vec<u8>, usize)> {
    prop::collection::vec(any::<u8>(), 1..100)
        .prop_flat_map(|vec| {
            let len = vec.len();
            (Just(vec), 0..len)
        })
}

// prop_oneof!: Choose between strategies
fn mixed_values() -> impl Strategy<Value = Value> {
    prop_oneof![
        any::<i32>().prop_map(Value::Int),
        ".*".prop_map(Value::String),
        any::<bool>().prop_map(Value::Bool),
    ]
}

// Just: Constant value
fn always_zero() -> impl Strategy<Value = i32> {
    Just(0)
}
```

### Weighted Choices

```rust
use proptest::prelude::*;

// 90% zeros, 10% random bytes (common in game state compression)
fn sparse_bytes() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(
        prop_oneof![
            9 => Just(0u8),    // 90% weight
            1 => any::<u8>(),  // 10% weight
        ],
        0..1024,
    )
}
```

### Regex-Based Strings

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_email_parsing(
        // Generate valid email-like strings
        email in "[a-z]{1,20}@[a-z]{1,10}\\.[a-z]{2,4}"
    ) {
        prop_assert!(email.contains('@'));
        prop_assert!(email.contains('.'));
    }
}
```

---

## Implementing Arbitrary for Custom Types

### Using `#[derive(Arbitrary)]`

```rust
use proptest::prelude::*;
use proptest_derive::Arbitrary;

#[derive(Debug, Clone, Arbitrary)]
struct GameInput {
    buttons: u8,
    analog_x: i8,
    analog_y: i8,
}

proptest! {
    #[test]
    fn test_game_input(input in any::<GameInput>()) {
        // Use the generated GameInput
        prop_assert!(input.analog_x >= i8::MIN);
    }
}
```

### Manual Arbitrary Implementation

```rust
use proptest::prelude::*;

#[derive(Debug, Clone)]
struct Frame(i32);

impl Arbitrary for Frame {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        // Frames are always non-negative
        (0..=i32::MAX)
            .prop_map(Frame)
            .boxed()
    }
}
```

### Custom Strategy with Attribute

```rust
use proptest::prelude::*;
use proptest_derive::Arbitrary;

#[derive(Debug, Clone, Arbitrary)]
struct Player {
    // Use custom strategy for this field
    #[proptest(strategy = "1..=4u8")]
    player_id: u8,

    #[proptest(strategy = "valid_name_strategy()")]
    name: String,

    // Skip this field, use default
    #[proptest(value = "100")]
    health: i32,
}

fn valid_name_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z]{3,20}"
}
```

### Handling Types Without Arbitrary (e.g., Uuid)

```rust
use proptest::prelude::*;

// Create a newtype wrapper
#[derive(Debug, Clone)]
struct TestUuid(uuid::Uuid);

impl Arbitrary for TestUuid {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        // Generate random bytes and construct UUID
        any::<[u8; 16]>()
            .prop_map(|bytes| TestUuid(uuid::Uuid::from_bytes(bytes)))
            .boxed()
    }
}
```

---

## Common Property Testing Patterns

### 1. Round-Trip / Symmetry

**The most important pattern**: `decode(encode(x)) == x`

```rust
proptest! {
    /// Property: RLE roundtrip - decode(encode(data)) == data
    #[test]
    fn prop_rle_roundtrip(data in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let encoded = encode(&data);
        let decoded = decode(&encoded)
            .expect("decode should not fail on valid encoded data");
        prop_assert_eq!(
            data, decoded,
            "Roundtrip failed: encode/decode produced different data"
        );
    }
}
```

**Applies to:**

- Serialization/deserialization (serde, bincode, protobuf)
- Compression/decompression
- Encryption/decryption (with key)
- Base64/hex encoding
- URL encoding/decoding

### 2. Commutativity

`f(a, b) == f(b, a)`

```rust
proptest! {
    #[test]
    fn prop_min_commutative(a in any::<i32>(), b in any::<i32>()) {
        prop_assert_eq!(std::cmp::min(a, b), std::cmp::min(b, a));
    }

    #[test]
    fn prop_set_union_commutative(
        a in proptest::collection::hash_set(any::<i32>(), 0..50),
        b in proptest::collection::hash_set(any::<i32>(), 0..50),
    ) {
        let union_ab: HashSet<_> = a.union(&b).copied().collect();
        let union_ba: HashSet<_> = b.union(&a).copied().collect();
        prop_assert_eq!(union_ab, union_ba);
    }
}
```

### 3. Invariant Preservation

Properties that must always hold after operations:

```rust
proptest! {
    /// Property: Sorted output is always sorted
    #[test]
    fn prop_sort_preserves_sorted_invariant(input in proptest::collection::vec(any::<i32>(), 0..1000)) {
        let sorted = {
            let mut v = input.clone();
            v.sort();
            v
        };

        // Invariant: output is sorted
        prop_assert!(sorted.windows(2).all(|w| w[0] <= w[1]));

        // Invariant: same length
        prop_assert_eq!(sorted.len(), input.len());
    }

    /// Property: Queue length is always >= 0
    #[test]
    fn prop_queue_length_invariant(
        ops in proptest::collection::vec(
            prop_oneof![Just(Op::Push), Just(Op::Pop)],
            0..100
        )
    ) {
        let mut queue = Queue::new();
        for op in ops {
            match op {
                Op::Push => queue.push(1),
                Op::Pop => { let _ = queue.pop(); }
            }
            // Invariant must hold after every operation
            prop_assert!(queue.len() >= 0);
        }
    }
}
```

### 4. Idempotency

`f(f(x)) == f(x)`

```rust
proptest! {
    #[test]
    fn prop_normalize_idempotent(path in "(/[a-z]+)+") {
        let once = normalize_path(&path);
        let twice = normalize_path(&once);
        prop_assert_eq!(once, twice, "normalize should be idempotent");
    }

    #[test]
    fn prop_sort_idempotent(input in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut once = input.clone();
        once.sort();
        let mut twice = once.clone();
        twice.sort();
        prop_assert_eq!(once, twice);
    }
}
```

### 5. Reference Implementation Comparison

Test optimized code against known-correct (but slow) implementation:

```rust
fn naive_compress(data: &[u8]) -> Vec<u8> {
    // Simple but obviously correct implementation
    data.to_vec() // No compression - just copy
}

fn optimized_compress(data: &[u8]) -> Vec<u8> {
    // Complex RLE implementation
    rle::encode(data)
}

proptest! {
    #[test]
    fn prop_optimized_matches_reference(data in proptest::collection::vec(any::<u8>(), 0..1000)) {
        // Both should decompress to same result
        let naive_result = naive_decompress(&naive_compress(&data));
        let optimized_result = rle::decode(&optimized_compress(&data)).unwrap();
        prop_assert_eq!(naive_result, optimized_result);
    }
}
```

### 6. Oracle Testing

Compare implementation against a known oracle:

```rust
proptest! {
    #[test]
    fn prop_our_sqrt_matches_stdlib(x in 0.0f64..1e10) {
        let our_result = our_sqrt(x);
        let stdlib_result = x.sqrt();
        
        // Allow small floating-point differences
        let diff = (our_result - stdlib_result).abs();
        prop_assert!(
            diff < 1e-10,
            "Results differ: {} vs {}",
            our_result, stdlib_result
        );
    }
}
```

### 7. State Machine Testing

Sequences of operations maintaining invariants:

```rust
#[derive(Debug, Clone)]
enum QueueOp {
    Push(i32),
    Pop,
    Peek,
}

fn queue_operations() -> impl Strategy<Value = Vec<QueueOp>> {
    proptest::collection::vec(
        prop_oneof![
            any::<i32>().prop_map(QueueOp::Push),
            Just(QueueOp::Pop),
            Just(QueueOp::Peek),
        ],
        0..50
    )
}

proptest! {
    #[test]
    fn prop_queue_state_machine(ops in queue_operations()) {
        let mut queue = Queue::new();
        let mut model = VecDeque::new();  // Reference model

        for op in ops {
            match op {
                QueueOp::Push(x) => {
                    queue.push(x);
                    model.push_back(x);
                }
                QueueOp::Pop => {
                    let actual = queue.pop();
                    let expected = model.pop_front();
                    prop_assert_eq!(actual, expected);
                }
                QueueOp::Peek => {
                    let actual = queue.peek();
                    let expected = model.front().copied();
                    prop_assert_eq!(actual, expected);
                }
            }
            // Invariant: lengths always match
            prop_assert_eq!(queue.len(), model.len());
        }
    }
}
```

---

## Stateful Property Testing

For complex state machines, use **proptest-stateful** or **proptest-state-machine**.

### The Model-Based Testing Approach

1. Define a **model** (simple, obviously correct representation)
2. Define **operations** as an enum
3. Generate **sequences** of operations
4. Execute on both model and system under test
5. Verify they stay in sync

```rust
use proptest::prelude::*;
use proptest_stateful::prelude::*;

// The system under test
struct InputQueue {
    buffer: Vec<GameInput>,
    head: usize,
}

// Simple model for verification
struct InputQueueModel {
    inputs: VecDeque<GameInput>,
}

// Operations that can be performed
#[derive(Debug, Clone)]
enum QueueAction {
    AddInput { frame: i32, input: GameInput },
    GetInput { frame: i32 },
    DiscardBefore { frame: i32 },
}

impl StatefulTest for InputQueue {
    type Model = InputQueueModel;
    type Action = QueueAction;

    fn init() -> (Self, Self::Model) {
        (InputQueue::new(), InputQueueModel::new())
    }

    fn precondition(model: &Self::Model, action: &Self::Action) -> bool {
        match action {
            QueueAction::GetInput { frame } => {
                // Precondition: frame must exist in model
                model.has_frame(*frame)
            }
            _ => true,
        }
    }

    fn apply(mut self, model: &mut Self::Model, action: Self::Action)
        -> Result<Self, TestCaseError>
    {
        match action {
            QueueAction::AddInput { frame, input } => {
                self.add_input(frame, input.clone())?;
                model.add_input(frame, input);
            }
            QueueAction::GetInput { frame } => {
                let actual = self.get_input(frame)?;
                let expected = model.get_input(frame);
                prop_assert_eq!(actual, expected);
            }
            QueueAction::DiscardBefore { frame } => {
                self.discard_before(frame)?;
                model.discard_before(frame);
            }
        }
        Ok(self)
    }
}
```

### Action Generation with Preconditions

```rust
fn action_strategy(model: &InputQueueModel) -> impl Strategy<Value = QueueAction> {
    let frames: Vec<i32> = model.frames().collect();

    if frames.is_empty() {
        // Can only add when empty
        (any::<i32>(), any::<GameInput>())
            .prop_map(|(frame, input)| QueueAction::AddInput { frame, input })
            .boxed()
    } else {
        prop_oneof![
            // Add new input
            (any::<i32>(), any::<GameInput>())
                .prop_map(|(frame, input)| QueueAction::AddInput { frame, input }),
            // Get existing input
            proptest::sample::select(frames.clone())
                .prop_map(|frame| QueueAction::GetInput { frame }),
            // Discard old inputs
            proptest::sample::select(frames)
                .prop_map(|frame| QueueAction::DiscardBefore { frame }),
        ].boxed()
    }
}
```

---

## Best Practices and Pitfalls

### ✅ DO: Keep Strategies Focused

```rust
// ✅ GOOD: Constrained to realistic sizes
fn game_inputs() -> impl Strategy<Value = Vec<GameInput>> {
    proptest::collection::vec(any::<GameInput>(), 0..100)
}

// ❌ BAD: Unbounded can be slow and find unrealistic bugs
fn game_inputs_bad() -> impl Strategy<Value = Vec<GameInput>> {
    proptest::collection::vec(any::<GameInput>(), 0..usize::MAX)
}
```

### ✅ DO: Configure Test Count Appropriately

```rust
// In the test module
proptest! {
    // Increase iterations for critical properties
    #![proptest_config(ProptestConfig::with_cases(10000))]

    #[test]
    fn prop_critical_roundtrip(data in any::<Vec<u8>>()) {
        // ...
    }
}

// Or per-test
proptest! {
    #[test]
    #[proptest_config(ProptestConfig::with_cases(100))]
    fn prop_expensive_test(data in any::<LargeStruct>()) {
        // ...
    }
}
```

### ✅ DO: Use `prop_assert!` Not `assert!`

```rust
proptest! {
    #[test]
    fn example(x in any::<i32>()) {
        // ✅ GOOD: Integrates with shrinking and error reporting
        prop_assert!(x.checked_add(0).is_some());

        // ❌ BAD: Panics immediately, no shrinking
        assert!(x.checked_add(0).is_some());
    }
}
```

### ✅ DO: Commit Regression Files

```bash
# .gitignore
# ... other entries ...
!proptest-regressions/
```

### ✅ DO: Use Deterministic Seeds for CI

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        // Use deterministic source for reproducible CI
        source_file: Some("tests/property_tests.rs"),
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_example(x in any::<i32>()) {
        // ...
    }
}
```

### ❌ AVOID: Filter-Heavy Strategies

```rust
// ❌ BAD: Most generated values rejected (slow, may give up)
fn prime_numbers_bad() -> impl Strategy<Value = u32> {
    any::<u32>().prop_filter("must be prime", |&n| is_prime(n))
}

// ✅ GOOD: Generate valid values directly
fn prime_numbers_good() -> impl Strategy<Value = u32> {
    proptest::sample::select(vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31])
}
```

### ❌ AVOID: Overly Large Inputs

```rust
// ❌ BAD: 1MB strings are slow to shrink
fn any_string_bad() -> impl Strategy<Value = String> {
    ".{0,1000000}"
}

// ✅ GOOD: Reasonable bounds catch most bugs
fn any_string_good() -> impl Strategy<Value = String> {
    ".{0,1000}"
}
```

### ❌ AVOID: Non-Deterministic Properties

```rust
// ❌ BAD: Depends on external time
proptest! {
    #[test]
    fn prop_flaky(x in any::<i32>()) {
        let start = Instant::now();
        compute(x);
        prop_assert!(start.elapsed() < Duration::from_secs(1));
    }
}
```

---

## Converting Existing Tests to Property Tests

### Step-by-Step Process

1. **Identify the property** being implicitly tested
2. **Abstract the input** from specific values to types
3. **Write a strategy** for that type
4. **Replace hard-coded values** with generated values
5. **Convert assertions** to property assertions
6. **Add edge case strategies** if needed

### Example Conversion

**Before (Unit Test):**

```rust
#[test]
fn test_rle_encode_decode() {
    let data = vec![0, 0, 0, 1, 2, 3, 255, 255];
    let encoded = encode(&data);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(data, decoded);
}
```

**After (Property Test):**

```rust
proptest! {
    /// Property: RLE roundtrip preserves data for ALL inputs
    #[test]
    fn prop_rle_roundtrip(data in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let encoded = encode(&data);
        let decoded = decode(&encoded)
            .expect("decode should succeed on valid encoded data");
        prop_assert_eq!(
            data, decoded,
            "Roundtrip failed: encode/decode produced different data"
        );
    }
}

// Keep the original test too! It documents a specific example.
#[test]
fn test_rle_encode_decode_example() {
    let data = vec![0, 0, 0, 1, 2, 3, 255, 255];
    let encoded = encode(&data);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(data, decoded);
}
```

---

## Integration with Fortress Rollback

### Testing Frame Arithmetic

```rust
use proptest::prelude::*;
use crate::Frame;

proptest! {
    /// Property: Frame arithmetic is consistent
    #[test]
    fn prop_frame_add_sub_inverse(
        frame in any::<i32>().prop_map(Frame),
        delta in -1000..1000i32,
    ) {
        // Skip if would overflow
        prop_assume!(frame.0.checked_add(delta).is_some());
        prop_assume!(frame.0.checked_sub(delta).is_some());

        let added = Frame(frame.0 + delta);
        let back = Frame(added.0 - delta);
        prop_assert_eq!(frame, back);
    }
}
```

### Testing InputQueue Operations

```rust
proptest! {
    /// Property: InputQueue maintains sorted order by frame
    #[test]
    fn prop_input_queue_sorted(
        inputs in proptest::collection::vec(
            (0..1000i32, any::<GameInput>()),
            0..100
        )
    ) {
        let mut queue = InputQueue::new();
        for (frame, input) in inputs {
            let _ = queue.add_input(Frame(frame), input);
        }

        // All frames should be in sorted order
        let frames: Vec<_> = queue.iter().map(|(f, _)| f.0).collect();
        prop_assert!(frames.windows(2).all(|w| w[0] <= w[1]));
    }
}
```

### Testing Message Serialization

```rust
use proptest::prelude::*;
use proptest_derive::Arbitrary;

#[derive(Debug, Clone, Arbitrary)]
enum TestMessage {
    SyncRequest { random_request: u32 },
    Input { frame: i32, input: Vec<u8> },
    QualityReport { ping: u32 },
}

proptest! {
    /// Property: Message serialization roundtrip
    #[test]
    fn prop_message_roundtrip(msg in any::<TestMessage>()) {
        let bytes = bincode::encode_to_vec(&msg, bincode::config::standard())
            .expect("encode should succeed");
        let decoded: TestMessage = bincode::decode_from_slice(&bytes, bincode::config::standard())
            .expect("decode should succeed")
            .0;

        // Compare variant and key fields (derive PartialEq or check manually)
        match (&msg, &decoded) {
            (TestMessage::SyncRequest { random_request: a },
             TestMessage::SyncRequest { random_request: b }) => {
                prop_assert_eq!(a, b);
            }
            // ... other variants
            _ => prop_assert!(false, "Variant mismatch"),
        }
    }
}
```

### Connection to Formal Verification

Property testing and formal verification are complementary:

| Approach | Coverage | Confidence | Speed |
|----------|----------|------------|-------|
| **Unit Tests** | Specific cases | Low | Fast |
| **Property Tests** | Random sampling | Medium | Medium |
| **Kani Proofs** | Exhaustive (bounded) | High | Slow |
| **TLA+ Models** | Protocol design | Very High | N/A |

**Use property tests to:**

- Find bugs quickly during development
- Build confidence before writing formal proofs
- Test properties that are hard to prove formally

**Use Kani/TLA+ to:**

- Prove critical safety properties
- Verify protocol correctness
- Document verified invariants

---

## Quick Reference

### Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `cases` | Number of test cases | 256 |
| `max_shrink_iters` | Max shrinking attempts | 10000 |
| `timeout` | Per-test timeout | 0 (none) |
| `source_file` | For deterministic seeds | None |
| `result_cache` | Cache passing results | Enabled |

```rust
ProptestConfig {
    cases: 1000,
    max_shrink_iters: 100000,
    timeout: 60000, // ms
    ..Default::default()
}
```

### Strategy Cheat Sheet

| Pattern | Strategy |
|---------|----------|
| Any value of type T | `any::<T>()` |
| Range | `0..100i32` or `-50..=50i32` |
| Constant | `Just(42)` |
| Regex string | `"[a-z]+"` |
| Sized string | `".{0,100}"` |
| Vec with bounds | `prop::collection::vec(any::<T>(), 0..100)` |
| HashMap | `prop::collection::hash_map(any::<K>(), any::<V>(), 0..50)` |
| Choose from list | `prop_oneof![a, b, c]` |
| Weighted choice | `prop_oneof![9 => a, 1 => b]` |
| Transform | `strategy.prop_map(\|x\| transform(x))` |
| Dependent | `strategy.prop_flat_map(\|x\| dependent_strategy(x))` |
| Filter (use sparingly) | `strategy.prop_filter("reason", \|x\| predicate(x))` |
| Option | `proptest::option::of(strategy)` |

### Troubleshooting

| Problem | Solution |
|---------|----------|
| Tests timeout | Reduce input sizes, add bounds |
| "Too many rejections" | Use `prop_flat_map` instead of `prop_filter` |
| Shrinking takes forever | Reduce `max_shrink_iters`, simplify strategy |
| Non-deterministic failures | Set `source_file`, check for external state |
| Can't derive Arbitrary | Implement manually or use `#[proptest(strategy)]` |

---

## Resources

- [Proptest Book](https://proptest-rs.github.io/proptest/intro.html) — Official documentation
- [test-strategy](https://docs.rs/test-strategy/latest/test_strategy/) — Attribute macros for proptest
- [proptest-state-machine](https://docs.rs/proptest-state-machine/latest/proptest_state_machine/) — Stateful testing
- [Hypothesis (Python)](https://hypothesis.readthedocs.io/) — Inspiration for proptest
- [QuickCheck paper](https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quick.pdf) — Original academic paper
