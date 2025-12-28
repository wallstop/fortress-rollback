# Zero-Copy Patterns and Memory Efficiency in Rust

> A comprehensive guide to minimizing memory allocations, avoiding unnecessary copies, and maximizing performance in Rust applications.

## Table of Contents

1. [Core Concepts](#core-concepts)
2. [20 Zero-Copy and Memory Efficiency Patterns](#20-zero-copy-and-memory-efficiency-patterns)
3. [Serialization and Deserialization](#serialization-and-deserialization)
4. [Library Recommendations](#library-recommendations)
5. [Common Mistakes to Avoid](#common-mistakes-to-avoid)
6. [Trade-offs and Decision Guide](#trade-offs-and-decision-guide)

---

## Core Concepts

### What is Zero-Copy?

Zero-copy refers to techniques where data is accessed directly from its original location without copying to intermediate buffers. Benefits include:

- **Reduced memory usage**: No duplicate data in memory
- **Lower CPU usage**: No time spent copying bytes
- **Better cache efficiency**: Access original data directly
- **Lower latency**: Avoid allocation overhead

### Borrowed vs. Owned Data

| Aspect | Borrowed (`&T`, `&[T]`) | Owned (`T`, `Vec<T>`) |
|--------|------------------------|----------------------|
| Memory | References existing data | Owns its own allocation |
| Lifetime | Limited by source | Independent |
| Mutation | Immutable (or `&mut`) | Full control |
| Copying | No copy (just pointer) | Requires allocation |
| Use case | Temporary access | Long-lived storage |

---

## 20 Zero-Copy and Memory Efficiency Patterns

### Pattern 1: Accept Borrowed Types in Function Arguments

**Problem**: Functions accepting `&String` or `&Vec<T>` are unnecessarily restrictive.

**Solution**: Accept `&str` or `&[T]` to enable zero-copy from multiple source types.

```rust
// ❌ Restrictive: Only accepts &String
fn process_name(name: &String) -> bool {
    name.len() > 0
}

// ✅ Flexible: Accepts &str, &String, String slices, etc.
fn process_name(name: &str) -> bool {
    !name.is_empty()
}

// Usage - all work without copying:
process_name("literal");           // &str
process_name(&owned_string);       // &String deref-coerces to &str
process_name(&owned_string[1..5]); // Substring slice
```

**When to use**: Any function that only reads data without needing ownership.

---

### Pattern 2: Use `Cow<'a, T>` for Clone-on-Write Semantics

**Problem**: Functions that *sometimes* need to modify data, but often just read it.

**Solution**: Use `Cow` (Clone-on-Write) to avoid allocations when mutation isn't needed.

```rust
use std::borrow::Cow;

/// Normalizes a string, only allocating if changes are needed.
fn normalize_whitespace(input: &str) -> Cow<'_, str> {
    if input.contains("  ") {
        // Only allocate when modification is needed
        Cow::Owned(input.split_whitespace().collect::<Vec<_>>().join(" "))
    } else {
        // Zero-copy: return reference to original
        Cow::Borrowed(input)
    }
}

// Usage:
let clean = "hello world";
let normalized = normalize_whitespace(clean);
assert!(matches!(normalized, Cow::Borrowed(_))); // No allocation!

let messy = "hello   world";
let normalized = normalize_whitespace(messy);
assert!(matches!(normalized, Cow::Owned(_))); // Had to allocate
```

**When to use**: 
- Parsing where most inputs are already valid
- Transformation functions where modification is rare
- APIs that accept both owned and borrowed data

---

### Pattern 3: Store `Cow` in Structs for Flexible Ownership

**Problem**: Structs that sometimes own data, sometimes borrow it.

```rust
use std::borrow::Cow;

/// A message that may own or borrow its content.
#[derive(Debug, Clone)]
struct Message<'a> {
    id: u64,
    content: Cow<'a, str>,
    tags: Cow<'a, [String]>,
}

impl<'a> Message<'a> {
    /// Creates a message from borrowed content (zero-copy).
    fn from_borrowed(id: u64, content: &'a str, tags: &'a [String]) -> Self {
        Self {
            id,
            content: Cow::Borrowed(content),
            tags: Cow::Borrowed(tags),
        }
    }
    
    /// Creates a message with owned content.
    fn from_owned(id: u64, content: String, tags: Vec<String>) -> Self {
        Self {
            id,
            content: Cow::Owned(content),
            tags: Cow::Owned(tags),
        }
    }
    
    /// Converts to owned data for long-term storage.
    fn into_owned(self) -> Message<'static> {
        Message {
            id: self.id,
            content: Cow::Owned(self.content.into_owned()),
            tags: Cow::Owned(self.tags.into_owned()),
        }
    }
}
```

**When to use**: Message types, parsed data structures, caches with mixed ownership.

---

### Pattern 4: Use `&[u8]` Slices for Binary Data Views

**Problem**: Need to access parts of binary data without copying.

```rust
/// Parse a network packet header without copying.
struct PacketHeader<'a> {
    version: u8,
    payload_type: u8,
    payload: &'a [u8],
}

impl<'a> PacketHeader<'a> {
    fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        
        let version = data[0];
        let payload_type = data[1];
        let length = u16::from_be_bytes([data[2], data[3]]) as usize;
        
        if data.len() < 4 + length {
            return None;
        }
        
        // Zero-copy: payload is a view into original data
        Some(Self {
            version,
            payload_type,
            payload: &data[4..4 + length],
        })
    }
}
```

---

### Pattern 5: Use `bytes::Bytes` for Shared Byte Buffers

**Problem**: Multiple parts of your application need access to the same byte buffer.

**Solution**: Use `bytes::Bytes` for reference-counted, cheaply cloneable byte slices.

```rust
use bytes::{Bytes, BytesMut, BufMut};

// Create a mutable buffer
let mut buf = BytesMut::with_capacity(1024);
buf.put_slice(b"header:");
buf.put_u32(42);
buf.put_slice(b":payload");

// Freeze into immutable, shareable Bytes
let data: Bytes = buf.freeze();

// Cheap cloning - just increments refcount, no data copy
let data2 = data.clone();

// Cheap slicing - creates view into same underlying memory
let header = data.slice(0..7);
let payload = data.slice(11..);

// All three share the same underlying allocation
assert_eq!(&header[..], b"header:");
```

**When to use**: Network protocols, multi-threaded data sharing, streaming parsers.

```toml
# Cargo.toml
[dependencies]
bytes = "1"
```

---

### Pattern 6: Use `zerocopy` for Safe Transmutation

**Problem**: Need to interpret raw bytes as typed structs without copying.

**Solution**: Use the `zerocopy` crate for compile-time verified zero-copy conversions.

```rust
use zerocopy::{FromBytes, IntoBytes, Immutable, KnownLayout};

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout, Debug)]
#[repr(C)]
struct NetworkPacket {
    magic: [u8; 4],
    version: u8,
    flags: u8,
    length: u16,
    sequence: u32,
}

fn parse_packet(data: &[u8]) -> Option<&NetworkPacket> {
    // Zero-copy: reinterprets bytes as struct reference
    zerocopy::Ref::<_, NetworkPacket>::from_bytes(data)
        .ok()
        .map(|r| r.into_ref())
}

fn serialize_packet(packet: &NetworkPacket) -> &[u8] {
    // Zero-copy: views struct as bytes
    packet.as_bytes()
}
```

**When to use**: Network protocols, file formats, shared memory, FFI.

```toml
# Cargo.toml
[dependencies]
zerocopy = { version = "0.8", features = ["derive"] }
```

---

### Pattern 7: Use `mem::take()` and `mem::replace()` to Avoid Cloning

**Problem**: Need to move a value out of a mutable reference.

```rust
use std::mem;

struct Connection {
    buffer: Vec<u8>,
    pending: Option<Vec<u8>>,
}

impl Connection {
    /// Takes the buffer, replacing it with an empty one (no allocation).
    fn take_buffer(&mut self) -> Vec<u8> {
        mem::take(&mut self.buffer) // Replaces with Vec::new()
    }
    
    /// Takes pending data if present.
    fn take_pending(&mut self) -> Option<Vec<u8>> {
        mem::take(&mut self.pending) // Replaces with None
    }
    
    /// Replaces buffer with a new one, returning the old.
    fn swap_buffer(&mut self, new_buffer: Vec<u8>) -> Vec<u8> {
        mem::replace(&mut self.buffer, new_buffer)
    }
}
```

**When to use**: State machines, builder patterns, ownership transfer from `&mut`.

---

### Pattern 8: Pre-allocate with `with_capacity()`

**Problem**: Growing collections causes multiple allocations and copies.

```rust
// ❌ Multiple reallocations as vec grows
let mut items = Vec::new();
for i in 0..1000 {
    items.push(i); // May reallocate multiple times
}

// ✅ Single allocation upfront
let mut items = Vec::with_capacity(1000);
for i in 0..1000 {
    items.push(i); // No reallocation needed
}

// Also works for strings:
let mut s = String::with_capacity(100);
for word in words {
    s.push_str(word);
    s.push(' ');
}

// And HashMaps:
use std::collections::HashMap;
let mut map = HashMap::with_capacity(1000);
```

**When to use**: Any time you know (or can estimate) the final size.

---

### Pattern 9: Use `String::from_utf8_unchecked` for Trusted Data

**Problem**: `String::from_utf8()` validates UTF-8, which has a cost.

```rust
// Safe version - validates UTF-8
fn parse_safe(data: Vec<u8>) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(data)
}

// ✅ When you KNOW the data is valid UTF-8 (e.g., you generated it)
fn parse_trusted(data: Vec<u8>) -> String {
    // SAFETY: We generated this data and know it's valid UTF-8
    unsafe { String::from_utf8_unchecked(data) }
}

// Even better: Use from_utf8_lossy for potentially invalid data
fn parse_lossy(data: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(data) // Returns Cow - no allocation if valid
}
```

**Warning**: Only use `_unchecked` variants when you can truly guarantee validity.

---

### Pattern 10: Use Iterators Instead of Collecting

**Problem**: Intermediate collections waste memory.

```rust
// ❌ Allocates intermediate Vec
let doubled: Vec<i32> = numbers
    .iter()
    .map(|n| n * 2)
    .collect();
let sum: i32 = doubled.iter().sum();

// ✅ No intermediate allocation
let sum: i32 = numbers
    .iter()
    .map(|n| n * 2)
    .sum();

// ❌ Multiple intermediate allocations
let result: Vec<String> = items
    .iter()
    .filter(|x| x.is_active())
    .map(|x| x.name.clone())
    .collect::<Vec<_>>() // Allocation 1
    .into_iter()
    .filter(|name| !name.is_empty())
    .collect(); // Allocation 2

// ✅ Single pass, single allocation
let result: Vec<String> = items
    .iter()
    .filter(|x| x.is_active() && !x.name.is_empty())
    .map(|x| x.name.clone())
    .collect(); // Single allocation at the end
```

---

### Pattern 11: Use `ArrayVec` or `SmallVec` for Small Collections

**Problem**: Heap allocation for small, bounded collections is wasteful.

```rust
use arrayvec::ArrayVec;
use smallvec::SmallVec;

// ArrayVec: Fixed capacity, stack allocated, no heap
fn process_small_batch() {
    let mut batch: ArrayVec<u32, 16> = ArrayVec::new();
    for i in 0..10 {
        batch.push(i); // All on stack, no heap allocation
    }
}

// SmallVec: Stack until it exceeds capacity, then heap
fn process_variable_batch(items: &[u32]) {
    let mut batch: SmallVec<[u32; 8]> = SmallVec::new();
    for &item in items {
        batch.push(item); // Stack for ≤8 items, heap otherwise
    }
}
```

```toml
# Cargo.toml
[dependencies]
arrayvec = "0.7"
smallvec = "1"
```

---

### Pattern 12: Use Arena Allocators for Short-Lived Objects

**Problem**: Many small allocations with similar lifetimes cause fragmentation.

```rust
use bumpalo::Bump;

fn process_request(data: &[u8]) {
    // Create arena for this request
    let arena = Bump::new();
    
    // All allocations are fast bump-pointer allocations
    let parsed: &mut ParsedData = arena.alloc(ParsedData::new());
    let temp_buffer: &mut [u8] = arena.alloc_slice_copy(&[0u8; 1024]);
    let strings: Vec<&str, _> = bumpalo::vec![in &arena; "hello", "world"];
    
    // Process...
    
    // When arena drops, ALL memory is freed at once (single operation)
}

struct ParsedData {
    // ...fields
}

impl ParsedData {
    fn new() -> Self {
        Self { /* ... */ }
    }
}
```

```toml
# Cargo.toml
[dependencies]
bumpalo = "3"
```

**When to use**: Request handlers, parsers, compilers, game frames.

---

### Pattern 13: Use Slices for Substring Operations

**Problem**: Creating substrings allocates new Strings.

```rust
// ❌ Allocates new String for each operation
fn extract_parts_allocating(input: &str) -> (String, String, String) {
    let prefix = input[..5].to_string();    // Allocation
    let middle = input[5..10].to_string();  // Allocation
    let suffix = input[10..].to_string();   // Allocation
    (prefix, middle, suffix)
}

// ✅ Zero-copy: returns references into original string
fn extract_parts_borrowed(input: &str) -> (&str, &str, &str) {
    let prefix = &input[..5];   // No allocation
    let middle = &input[5..10]; // No allocation
    let suffix = &input[10..];  // No allocation
    (prefix, middle, suffix)
}

// For more complex cases, use indices
struct StringParts {
    prefix_end: usize,
    middle_end: usize,
}

impl StringParts {
    fn prefix<'a>(&self, source: &'a str) -> &'a str {
        &source[..self.prefix_end]
    }
    
    fn middle<'a>(&self, source: &'a str) -> &'a str {
        &source[self.prefix_end..self.middle_end]
    }
    
    fn suffix<'a>(&self, source: &'a str) -> &'a str {
        &source[self.middle_end..]
    }
}
```

---

### Pattern 14: Use `Entry` API to Avoid Double Lookups

**Problem**: Check-then-insert patterns do two map lookups.

```rust
use std::collections::HashMap;

let mut map: HashMap<String, Vec<i32>> = HashMap::new();

// ❌ Two lookups: one for contains_key, one for insert/get_mut
fn add_value_inefficient(map: &mut HashMap<String, Vec<i32>>, key: String, value: i32) {
    if map.contains_key(&key) {
        map.get_mut(&key).unwrap().push(value);
    } else {
        map.insert(key, vec![value]);
    }
}

// ✅ Single lookup with Entry API
fn add_value_efficient(map: &mut HashMap<String, Vec<i32>>, key: String, value: i32) {
    map.entry(key)
        .or_insert_with(Vec::new)
        .push(value);
}

// Also great for counting
fn count_words(words: &[&str]) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for word in words {
        *counts.entry(*word).or_insert(0) += 1;
    }
    counts
}
```

---

### Pattern 15: Use `Box<[T]>` Instead of `Vec<T>` for Fixed-Size Data

**Problem**: `Vec<T>` has capacity overhead (stores length + capacity + pointer).

```rust
// Vec<T> stores: ptr, len, capacity (24 bytes on 64-bit)
let vec: Vec<i32> = vec![1, 2, 3, 4, 5];

// Box<[T]> stores: ptr, len (16 bytes on 64-bit)
let boxed: Box<[i32]> = vec![1, 2, 3, 4, 5].into_boxed_slice();

// Useful for fixed-size arrays of unknown size at compile time
struct Lookup {
    // Instead of Vec<Option<Entry>>
    table: Box<[Option<Entry>]>,
}

impl Lookup {
    fn new(size: usize) -> Self {
        Self {
            table: vec![None; size].into_boxed_slice(),
        }
    }
}

struct Entry {
    key: u64,
    value: String,
}
```

**When to use**: Hash tables, lookup tables, any fixed-size heap data.

---

### Pattern 16: Use `Rc<str>` or `Arc<str>` Instead of `Rc<String>`

**Problem**: `Rc<String>` has two levels of indirection.

```rust
use std::rc::Rc;
use std::sync::Arc;

// ❌ Two indirections: Rc -> String -> str data
let s1: Rc<String> = Rc::new("hello".to_string());

// ✅ Single indirection: Rc -> str data (inline)
let s2: Rc<str> = Rc::from("hello");
let s3: Rc<str> = "hello".into();

// Same for Arc
let s4: Arc<str> = Arc::from("hello");

// Works for slices too
let arr: Arc<[i32]> = Arc::from([1, 2, 3, 4, 5].as_slice());

// Convert from owned
let owned = String::from("hello world");
let shared: Arc<str> = owned.into();
```

---

### Pattern 17: Use `std::io::copy` for Streaming I/O

**Problem**: Reading entire files into memory wastes memory for large files.

```rust
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};

// ❌ Loads entire file into memory
fn copy_file_inefficient(src: &str, dst: &str) -> io::Result<()> {
    let contents = std::fs::read(src)?;  // Entire file in memory
    std::fs::write(dst, contents)?;
    Ok(())
}

// ✅ Streams data in chunks, constant memory usage
fn copy_file_efficient(src: &str, dst: &str) -> io::Result<u64> {
    let mut reader = BufReader::new(File::open(src)?);
    let mut writer = BufWriter::new(File::create(dst)?);
    io::copy(&mut reader, &mut writer)
}

// For custom processing, use read_exact or fill_buf
fn process_in_chunks(path: &str) -> io::Result<()> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut chunk = [0u8; 8192];
    
    loop {
        match reader.read(&mut chunk)? {
            0 => break,  // EOF
            n => process_chunk(&chunk[..n]),
        }
    }
    Ok(())
}

fn process_chunk(_data: &[u8]) {
    // Process the chunk...
}
```

---

### Pattern 18: Memory-Mapped Files for Large Read-Only Data

**Problem**: Need to access large files without loading entirely into memory.

```rust
use memmap2::Mmap;
use std::fs::File;

fn search_large_file(path: &str, pattern: &[u8]) -> std::io::Result<Option<usize>> {
    let file = File::open(path)?;
    
    // SAFETY: File must not be modified while mapped
    let mmap = unsafe { Mmap::map(&file)? };
    
    // Access file contents as a slice (loaded on-demand by OS)
    Ok(mmap.windows(pattern.len())
        .position(|window| window == pattern))
}

// For structured data
fn read_header(path: &str) -> std::io::Result<Header> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    
    // Zero-copy parse of header
    let header = Header::from_bytes(&mmap[..Header::SIZE])?;
    Ok(header)
}

struct Header;

impl Header {
    const SIZE: usize = 64;
    
    fn from_bytes(_data: &[u8]) -> std::io::Result<Self> {
        // Parse header...
        Ok(Self)
    }
}
```

```toml
# Cargo.toml
[dependencies]
memmap2 = "0.9"
```

**When to use**: Large read-only files, databases, search indices.

**Warning**: Ensure files aren't modified while mapped; this can cause undefined behavior.

---

### Pattern 19: Use `str::split` Iterators Instead of `split().collect()`

**Problem**: Splitting strings into vectors allocates unnecessarily.

```rust
let line = "field1,field2,field3,field4,field5";

// ❌ Collects into Vec (allocates)
let fields: Vec<&str> = line.split(',').collect();
let third = fields[2];

// ✅ Use iterator directly (no allocation)
let third = line.split(',').nth(2);

// ✅ For multiple access, use splitn for early termination
let mut parts = line.splitn(4, ',');
let first = parts.next();
let second = parts.next();
let third = parts.next();
// Remaining contains "field4,field5" (not split further)

// ✅ Pattern matching on fixed formats
fn parse_kv(input: &str) -> Option<(&str, &str)> {
    let (key, value) = input.split_once('=')?;
    Some((key.trim(), value.trim()))
}
```

---

### Pattern 20: Use `#[repr(C)]` and Careful Layout for Cache Efficiency

**Problem**: Poor struct layout causes cache misses.

```rust
// ❌ Poor layout: padding wastes memory, poor cache utilization
struct PoorLayout {
    flag: bool,      // 1 byte
    // 7 bytes padding
    big_value: u64,  // 8 bytes
    small: u8,       // 1 byte
    // 7 bytes padding
    another: u64,    // 8 bytes
}
// Total: 32 bytes (vs 18 bytes of actual data)

// ✅ Optimized layout: pack fields by alignment
struct OptimizedLayout {
    big_value: u64,  // 8 bytes (align 8)
    another: u64,    // 8 bytes
    small: u8,       // 1 byte
    flag: bool,      // 1 byte
    // 6 bytes padding at end (unavoidable for alignment)
}
// Total: 24 bytes

// For arrays of structs, consider struct-of-arrays
// ❌ Array of structs: cache unfriendly for single-field access
struct Particle {
    position: [f32; 3],
    velocity: [f32; 3],
    mass: f32,
}
let particles: Vec<Particle> = Vec::new();

// ✅ Struct of arrays: cache friendly for bulk operations
struct ParticleSystem {
    positions: Vec<[f32; 3]>,
    velocities: Vec<[f32; 3]>,
    masses: Vec<f32>,
}
```

---

## Serialization and Deserialization

### Zero-Copy Deserialization with Serde

```rust
use serde::Deserialize;

// ❌ Owned deserialization - allocates strings
#[derive(Deserialize)]
struct MessageOwned {
    id: u64,
    sender: String,
    content: String,
}

// ✅ Zero-copy deserialization - borrows from input
#[derive(Deserialize)]
struct MessageBorrowed<'a> {
    id: u64,
    #[serde(borrow)]
    sender: &'a str,
    #[serde(borrow)]
    content: &'a str,
}

fn parse_json(json: &str) -> Result<MessageBorrowed<'_>, serde_json::Error> {
    serde_json::from_str(json)
}

// With Cow for flexibility
#[derive(Deserialize)]
struct MessageCow<'a> {
    id: u64,
    #[serde(borrow)]
    sender: Cow<'a, str>,
    #[serde(borrow)]
    content: Cow<'a, str>,
}
```

### Using `rkyv` for Zero-Copy Archives

```rust
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug)]
struct GameState {
    score: u64,
    player_name: String,
    positions: Vec<(f32, f32)>,
}

// Serialize (writes to bytes)
let state = GameState {
    score: 1000,
    player_name: "Player1".to_string(),
    positions: vec![(1.0, 2.0), (3.0, 4.0)],
};
let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&state).unwrap();

// Zero-copy access (no deserialization needed!)
let archived = rkyv::access::<ArchivedGameState, rkyv::rancor::Error>(&bytes).unwrap();
println!("Score: {}", archived.score);  // Direct access
```

```toml
# Cargo.toml
[dependencies]
rkyv = { version = "0.8", features = ["validation"] }
```

---

## Library Recommendations

| Library | Use Case | Key Feature |
|---------|----------|-------------|
| `bytes` | Network buffers | Reference-counted byte slices |
| `zerocopy` | Binary protocols | Safe transmutation with compile-time checks |
| `rkyv` | Serialization | Zero-copy deserialization |
| `bumpalo` | Arena allocation | Fast bump-pointer allocation |
| `smallvec` | Small vectors | Stack allocation for small sizes |
| `arrayvec` | Fixed vectors | No-heap bounded vectors |
| `memmap2` | Large files | OS-level memory mapping |
| `beef` | Cow alternative | Faster, smaller `Cow` replacement |

---

## Common Mistakes to Avoid

### Mistake 1: Unnecessary `.clone()`

```rust
// ❌ Cloning when borrowing would work
fn process(items: Vec<String>) {
    for item in items.clone() {  // Why clone?
        println!("{}", item);
    }
}

// ✅ Borrow instead
fn process(items: &[String]) {
    for item in items {
        println!("{}", item);
    }
}
```

### Mistake 2: Collecting Then Iterating

```rust
// ❌ Collecting into intermediate Vec
let sum: i32 = data
    .iter()
    .collect::<Vec<_>>()  // Unnecessary allocation!
    .iter()
    .sum();

// ✅ Direct iteration
let sum: i32 = data.iter().copied().sum();
```

### Mistake 3: String Formatting in Hot Paths

```rust
// ❌ Allocates on every call
fn log_value(value: i32) {
    println!("{}", format!("Value: {}", value));
}

// ✅ Let println! handle formatting
fn log_value(value: i32) {
    println!("Value: {}", value);
}
```

### Mistake 4: Returning `String` When `&str` Would Work

```rust
// ❌ Allocates new String
fn get_name(&self) -> String {
    self.name.clone()
}

// ✅ Returns reference (zero-copy)
fn get_name(&self) -> &str {
    &self.name
}
```

### Mistake 5: Using `to_string()` for Static Strings

```rust
// ❌ Allocates at runtime
const ERROR_MSG: &str = "An error occurred";
fn get_error() -> String {
    ERROR_MSG.to_string()  // Heap allocation!
}

// ✅ Return reference to static string
fn get_error() -> &'static str {
    ERROR_MSG
}
```

---

## Trade-offs and Decision Guide

### When to Use Each Pattern

| Situation | Pattern | Trade-off |
|-----------|---------|-----------|
| Temporary data access | Borrowing (`&T`) | Lifetime constraints |
| May or may not modify | `Cow<T>` | Complexity |
| Shared byte buffers | `bytes::Bytes` | Dependency |
| Binary protocols | `zerocopy` | Repr constraints |
| Many short-lived allocations | Arena (`bumpalo`) | Memory not freed until arena drops |
| Small collections | `SmallVec` | Slightly larger type |
| Fixed-size collections | `ArrayVec` | Fixed capacity |
| Large files | `memmap2` | Platform-specific, unsafe |

### Safety vs Performance Spectrum

```
Most Safe                                    Most Performant
    |                                              |
    v                                              v
    
  Clone    Cow     Rc/Arc    Borrowing    Zerocopy    Unsafe
  everything      slices      slices      transmute   pointer ops
```

### Quick Decision Guide

1. **Can you borrow?** → Use references (`&T`, `&[T]`, `&str`)
2. **Sometimes need to own?** → Use `Cow<T>`
3. **Need shared ownership?** → Use `Rc<T>` or `Arc<T>`
4. **Binary data?** → Use `bytes::Bytes` or `zerocopy`
5. **Many small allocations?** → Use arena allocators
6. **Small, bounded collections?** → Use `SmallVec` or `ArrayVec`
7. **Large files?** → Use memory mapping

---

## Summary

Zero-copy patterns and memory efficiency are about:

1. **Avoiding unnecessary allocations** — Use borrowing, slices, and pre-allocation
2. **Sharing data without copying** — Use `Cow`, `Bytes`, reference counting
3. **Efficient data structures** — Choose the right container for your use case
4. **Understanding lifetimes** — Embrace Rust's borrow checker to enable zero-copy
5. **Measuring before optimizing** — Profile to find actual bottlenecks

Remember: **Correctness first, then measure, then optimize.** Zero-copy patterns add complexity; use them where they matter.
