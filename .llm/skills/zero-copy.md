<!-- CATEGORY: Rust Language -->
<!-- WHEN: Optimizing memory usage, zero-copy patterns, Cow vs clone decisions -->
# Zero-Copy and Memory Efficiency Patterns

## Decision Guide

1. **Can you borrow?** -> Use references (`&T`, `&[T]`, `&str`)
2. **Sometimes need to own?** -> Use `Cow<T>`
3. **Need shared ownership?** -> Use `Rc<T>` or `Arc<T>`
4. **Binary data?** -> Use `bytes::Bytes` or `zerocopy`
5. **Many small allocations?** -> Arena allocators (`bumpalo`)
6. **Small, bounded collections?** -> `SmallVec` or `ArrayVec`
7. **Large files?** -> Memory mapping (`memmap2`)

## Patterns Table

| Pattern | When | Trade-off |
|---------|------|-----------|
| Borrowing (`&T`) | Temporary access | Lifetime constraints |
| `Cow<T>` | May or may not modify | Complexity |
| `bytes::Bytes` | Shared byte buffers | Dependency |
| `zerocopy` | Binary protocols | Repr constraints |
| Arena (`bumpalo`) | Many short-lived allocs | Memory freed only at arena drop |
| `SmallVec` | Usually-small collections | Slightly larger type |
| `ArrayVec` | Fixed-capacity collections | Fixed capacity |
| `memmap2` | Large files | Platform-specific, unsafe |
| `Box<[T]>` | Fixed-size heap data | Cannot resize |
| `Rc<str>`/`Arc<str>` | Shared strings | Single indirection vs `Rc<String>` |

## Accept Borrowed Types

```rust
fn process(name: &str) -> bool { !name.is_empty() }
// Accepts: "literal", &String, &owned[1..5]
```

## Cow (Clone-on-Write)

```rust
fn normalize(input: &str) -> Cow<'_, str> {
    if input.contains("  ") {
        Cow::Owned(input.split_whitespace().collect::<Vec<_>>().join(" "))
    } else {
        Cow::Borrowed(input) // zero-copy
    }
}
```

Store `Cow` in structs for flexible ownership. Use `.into_owned()` for `'static` lifetime.

## Byte Slices for Binary Data

```rust
struct PacketHeader<'a> { version: u8, payload: &'a [u8] }
impl<'a> PacketHeader<'a> {
    fn parse(data: &'a [u8]) -> Option<Self> {
        // Zero-copy: payload is a view into original data
        Some(Self { version: data.get(0).copied()?, payload: data.get(4..)? })
    }
}
```

## `bytes::Bytes` for Shared Buffers

```rust
let data: Bytes = buf.freeze();
let data2 = data.clone();    // cheap: increments refcount
let header = data.slice(0..7); // cheap: view into same memory
```

## `zerocopy` for Safe Transmutation

```rust
#[derive(FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
struct NetworkPacket { magic: [u8; 4], version: u8, length: u16 }
```

## `mem::take()` and `mem::replace()`

```rust
fn take_buffer(&mut self) -> Vec<u8> { mem::take(&mut self.buffer) }
fn swap_buffer(&mut self, new: Vec<u8>) -> Vec<u8> { mem::replace(&mut self.buffer, new) }
```

## Pre-allocate with `with_capacity()`

```rust
let mut items = Vec::with_capacity(1000); // single allocation
let mut s = String::with_capacity(100);
let mut map = HashMap::with_capacity(1000);
```

## Iterators Over Collecting

```rust
// Avoid intermediate allocations
let sum: i32 = numbers.iter().map(|n| n * 2).sum(); // no intermediate Vec
```

## SmallVec / ArrayVec

```rust
let mut batch: ArrayVec<u32, 16> = ArrayVec::new(); // stack only
let mut items: SmallVec<[u32; 8]> = SmallVec::new(); // stack until >8
```

## Arena Allocators

```rust
let arena = Bump::new();
let parsed: &mut Data = arena.alloc(Data::new());
// All memory freed at once when arena drops
```

## Entry API (Avoid Double Lookup)

```rust
map.entry(key).or_insert_with(Vec::new).push(value); // single lookup
```

## `Box<[T]>` for Fixed-Size Data

```rust
let table: Box<[Option<Entry>]> = vec![None; size].into_boxed_slice();
// 16 bytes (ptr+len) vs Vec's 24 bytes (ptr+len+capacity)
```

## `Rc<str>` / `Arc<str>` (Single Indirection)

```rust
let s: Arc<str> = Arc::from("hello"); // one indirection, not two
let arr: Arc<[i32]> = Arc::from([1, 2, 3].as_slice());
```

## Zero-Copy Serde

```rust
#[derive(Deserialize)]
struct Message<'a> {
    id: u64,
    #[serde(borrow)]
    content: &'a str, // borrows from input, no allocation
}
```

## Struct Layout for Cache Efficiency

Order fields by alignment (largest first) to minimize padding. Consider struct-of-arrays for bulk single-field access.

## Common Mistakes

- **Unnecessary `.clone()`** -- borrow instead
- **`collect()` then iterate** -- chain iterators
- **`format!()` in hot paths** -- let `println!` handle formatting
- **Returning `String` when `&str` works** -- return reference
- **`to_string()` for static strings** -- return `&'static str`

## Library Reference

| Library | Use Case |
|---------|----------|
| `bytes` | Reference-counted byte slices |
| `zerocopy` | Safe transmutation |
| `rkyv` | Zero-copy deserialization |
| `bumpalo` | Arena allocation |
| `smallvec` | Stack-first small vectors |
| `arrayvec` | No-heap bounded vectors |
| `memmap2` | OS-level memory mapping |
