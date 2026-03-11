<!-- CATEGORY: Testing -->
<!-- WHEN: Writing property-based tests, proptest strategies, custom generators -->

# Property-Based Testing

## Quick Start

```toml
[dev-dependencies]
proptest = "1.9"
proptest-derive = "0.5"  # Optional: #[derive(Arbitrary)]
```

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_roundtrip(data in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let encoded = encode(&data);
        // proptest: use TestCaseError for proper shrinking (or .expect() as assertion)
        let decoded = decode(&encoded).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(data, decoded);
    }
}
```

Always use `prop_assert!` / `prop_assert_eq!` (not `assert!`) for proper shrinking.
Always commit `proptest-regressions/` files to version control.

## Strategy Cheat Sheet

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
| Transform | `strategy.prop_map(\|x\| f(x))` |
| Dependent | `strategy.prop_flat_map(\|x\| g(x))` |
| Filter (sparingly) | `strategy.prop_filter("reason", \|x\| pred(x))` |

## Strategy Combinators

```rust
// Transform
fn even_numbers() -> impl Strategy<Value = i32> {
    any::<i32>().prop_map(|n| n * 2)
}

// Dependent values
fn vec_with_valid_index() -> impl Strategy<Value = (Vec<u8>, usize)> {
    prop::collection::vec(any::<u8>(), 1..100)
        .prop_flat_map(|vec| { let len = vec.len(); (Just(vec), 0..len) })
}

// Weighted choice (90% zeros, 10% random)
fn sparse_bytes() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(
        prop_oneof![9 => Just(0u8), 1 => any::<u8>()],
        0..1024,
    )
}
```

## Custom Strategies with `prop_compose!`

```rust
prop_compose! {
    fn valid_player_id()(id in 0usize..100) -> PlayerId { PlayerId(id) }
}

prop_compose! {
    fn valid_input()(
        frame in 0u32..1000,
        player in valid_player_id(),
        data in prop::collection::vec(any::<u8>(), 0..32),
    ) -> GameInput {
        GameInput { frame, player, data }
    }
}
```

## Implementing Arbitrary

```rust
// Derive
#[derive(Debug, Clone, Arbitrary)]
struct GameInput { buttons: u8, analog_x: i8, analog_y: i8 }

// Manual
impl Arbitrary for Frame {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        (0..=i32::MAX).prop_map(Frame).boxed()
    }
}

// Per-field strategy
#[derive(Debug, Clone, Arbitrary)]
struct Player {
    #[proptest(strategy = "1..=4u8")]
    player_id: u8,
    #[proptest(value = "100")]
    health: i32,
}
```

## Property Patterns

### Round-Trip (most important)
`decode(encode(x)) == x` -- serialization, compression, encoding

### Commutativity
`f(a, b) == f(b, a)` -- min, max, set union

### Idempotency
`f(f(x)) == f(x)` -- normalize, sort, deduplicate

### Invariant Preservation
Operations maintain structural properties (sorted order, length, bounds)

### Reference Implementation
Compare optimized code against known-correct naive implementation

### State Machine Testing

```rust
#[derive(Debug, Clone)]
enum QueueOp { Push(i32), Pop, Peek }

proptest! {
    #[test]
    fn prop_queue_state_machine(ops in proptest::collection::vec(
        prop_oneof![
            any::<i32>().prop_map(QueueOp::Push),
            Just(QueueOp::Pop),
            Just(QueueOp::Peek),
        ], 0..50
    )) {
        let mut queue = Queue::new();
        let mut model = VecDeque::new();
        for op in ops {
            match op {
                QueueOp::Push(x) => { queue.push(x); model.push_back(x); }
                QueueOp::Pop => { prop_assert_eq!(queue.pop(), model.pop_front()); }
                QueueOp::Peek => { prop_assert_eq!(queue.peek(), model.front().copied()); }
            }
            prop_assert_eq!(queue.len(), model.len());
        }
    }
}
```

## Fortress-Specific Examples

### Frame Arithmetic

```rust
proptest! {
    #[test]
    fn prop_frame_add_sub_inverse(
        frame in any::<i32>().prop_map(Frame),
        delta in -1000..1000i32,
    ) {
        prop_assume!(frame.0.checked_add(delta).is_some());
        prop_assume!(frame.0.checked_sub(delta).is_some());
        let added = Frame(frame.0 + delta);
        let back = Frame(added.0 - delta);
        prop_assert_eq!(frame, back);
    }
}
```

### InputQueue Sorted Order

```rust
proptest! {
    #[test]
    fn prop_input_queue_sorted(
        inputs in proptest::collection::vec((0..1000i32, any::<GameInput>()), 0..100)
    ) {
        let mut queue = InputQueue::new();
        for (frame, input) in inputs {
            let _ = queue.add_input(Frame(frame), input);
        }
        let frames: Vec<_> = queue.iter().map(|(f, _)| f.0).collect();
        prop_assert!(frames.windows(2).all(|w| w[0] <= w[1]));
    }
}
```

## Configuration

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn prop_critical_roundtrip(data in any::<Vec<u8>>()) { /* ... */ }
}
```

| Option | Description | Default |
|--------|-------------|---------|
| `cases` | Number of test cases | 256 |
| `max_shrink_iters` | Max shrinking attempts | 10000 |
| `timeout` | Per-test timeout (ms) | 0 (none) |
| `source_file` | For deterministic seeds | None |

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "Too many rejections" | Use `prop_flat_map` instead of `prop_filter` |
| Shrinking takes forever | Reduce `max_shrink_iters`, simplify strategy |
| Tests timeout | Reduce input sizes, add bounds |
| Non-deterministic failures | Set `source_file`, check external state |

## Anti-Patterns

- Do NOT test hash collision resistance -- test determinism and known vectors instead
- Do NOT use `assert!` inside `proptest!` -- use `prop_assert!`
- Do NOT use unbounded strategies (`.{0,1000000}`) -- keep bounds reasonable
- Do NOT use filter-heavy strategies -- generate valid values directly
- Do NOT re-implement production logic in property assertions
