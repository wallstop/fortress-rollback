# Rust Idioms, Patterns, and Anti-Patterns

> A comprehensive guide to idiomatic Rust programming extracted from the Rust Design Patterns Book and Rust API Guidelines.

## Quick Reference

| Category | Count | Key Focus |
|----------|-------|-----------|
| Idioms | 10 | Community conventions and best practices |
| Behavioral Patterns | 6 | Communication between objects |
| Creational Patterns | 2 | Object construction |
| Structural Patterns | 4 | Relationships between entities |
| Anti-Patterns | 3 | What NOT to do |
| API Guidelines | 8 | Public API design |

---

## Part 1: Rust Idioms

### 1. Use Borrowed Types for Arguments

**Problem:** Functions accepting `&String` or `&Vec<T>` are unnecessarily restrictive.

**Solution:** Accept `&str` or `&[T]` instead to allow more input types via deref coercion.

```rust
// ❌ Avoid: Restricts callers to owned String
fn process(data: &String) -> bool { /* ... */ }

// ✅ Prefer: Accepts &String, &str, and string literals
fn process(data: &str) -> bool { /* ... */ }

// Works with all of these:
process("literal");           // &str
process(&String::from("owned")); // &String coerces to &str
process(&some_string);        // &String coerces to &str
```

**When to use:** Any function that only needs to read data, not own it.

**When to avoid:** When you actually need ownership of the data.

---

### 2. Constructors with `new()`

**Problem:** Rust has no language-level constructor syntax.

**Solution:** Use an associated function named `new` as the primary constructor.

```rust
pub struct Config {
    timeout: Duration,
    retries: u32,
}

impl Config {
    /// Creates a new Config with default settings.
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            retries: 3,
        }
    }

    /// Creates a Config with custom timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout, retries: 3 }
    }
}

// Also implement Default for generic contexts
impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
```

**When to use:** Every type that can be constructed.

**When to avoid:** I/O resources may use domain-specific names like `open()`, `connect()`.

---

### 3. The Default Trait

**Problem:** Need a common way to create "default" instances across generic code.

**Solution:** Implement or derive `Default` for types with sensible defaults.

```rust
#[derive(Default, Debug)]
struct GameConfig {
    max_players: u32,      // defaults to 0
    enable_rollback: bool, // defaults to false
    input_delay: Option<u32>, // defaults to None
}

// Use with struct update syntax
let config = GameConfig {
    max_players: 4,
    ..Default::default()
};

// Use in generic contexts
fn create_with_default<T: Default>() -> T {
    T::default()
}
```

**When to use:** Types with obvious default values; enables `unwrap_or_default()`.

**When to avoid:** Types where "default" has no clear meaning.

---

### 4. Concatenating Strings with `format!()`

**Problem:** String concatenation with `+` or multiple `push_str` calls is verbose.

**Solution:** Use `format!()` for readable string building.

```rust
// ❌ Verbose and error-prone
let mut result = "Player ".to_owned();
result.push_str(&player_name);
result.push_str(" scored ");
result.push_str(&score.to_string());

// ✅ Clear and concise
let result = format!("Player {player_name} scored {score}");
```

**When to use:** Building strings with mixed literals and variables.

**When to avoid:** Hot loops where pre-allocated `String::with_capacity()` + `push_str` is faster.

---

### 5. `mem::take()` and `mem::replace()` for Ownership Transfers

**Problem:** Need to move a value out of a mutable reference without cloning.

**Solution:** Use `mem::take()` (replaces with Default) or `mem::replace()` (replaces with specific value).

```rust
use std::mem;

enum State {
    Active { data: String },
    Inactive,
}

fn deactivate(state: &mut State) {
    if let State::Active { data } = state {
        // Take ownership of data, replace with empty String
        let owned_data = mem::take(data);
        process(owned_data);
        *state = State::Inactive;
    }
}

// Or swap with a specific value
fn swap_buffer(buffer: &mut Vec<u8>) -> Vec<u8> {
    mem::replace(buffer, Vec::with_capacity(1024))
}
```

**When to use:** Avoiding clones when transforming owned data in enums/structs.

**When to avoid:** When `Option::take()` suffices for `Option` types.

---

### 6. Finalisation in Destructors (RAII)

**Problem:** Need to ensure cleanup code runs regardless of how a function exits.

**Solution:** Put cleanup in a type's `Drop` implementation.

```rust
struct CleanupGuard {
    resource_id: u32,
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        // This runs when guard goes out of scope
        release_resource(self.resource_id);
    }
}

fn do_work() -> Result<(), Error> {
    let _guard = CleanupGuard { resource_id: 42 };

    // Even if this returns early with ?, the guard is dropped
    risky_operation()?;
    another_operation()?;

    Ok(())
} // _guard dropped here, cleanup runs
```

**When to use:** Resource cleanup, lock releasing, file closing.

**When to avoid:** Destructors shouldn't panic; don't rely on them for critical finalization.

---

### 7. On-Stack Dynamic Dispatch

**Problem:** Need dynamic dispatch without heap allocation.

**Solution:** Use `&dyn Trait` with temporary values extended by the compiler.

```rust
use std::io::{self, Read};

fn read_data(use_stdin: bool) -> io::Result<String> {
    // Since Rust 1.79, temporaries in &mut are automatically extended
    let reader: &mut dyn Read = if use_stdin {
        &mut io::stdin()
    } else {
        &mut std::fs::File::open("data.txt")?
    };

    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;
    Ok(buffer)
}
```

**When to use:** Conditional dispatch without `Box<dyn Trait>` allocation.

**When to avoid:** When you need the trait object to outlive the current scope.

---

### 8. `#[non_exhaustive]` for Extensibility

**Problem:** Adding variants to enums or fields to structs is a breaking change.

**Solution:** Use `#[non_exhaustive]` to allow future additions.

```rust
#[non_exhaustive]
pub enum Error {
    Network(io::Error),
    Parse(String),
    // Can add new variants in future minor releases
}

#[non_exhaustive]
pub struct Config {
    pub timeout: Duration,
    // Can add new fields in future minor releases
}

// Users must handle unknown variants
match error {
    Error::Network(e) => { /* ... */ }
    Error::Parse(s) => { /* ... */ }
    _ => { /* handle future variants */ }
}

// Users must use `..` when matching structs
let Config { timeout, .. } = config;
```

**When to use:** Public types that may gain variants/fields in semver-compatible releases.

**When to avoid:** Internal types; makes matching more verbose for users.

---

### 9. Pass Variables to Closures Explicitly

**Problem:** Move closures capture entire environment; borrow closures have unclear ownership.

**Solution:** Use a block to explicitly prepare closure captures.

```rust
use std::rc::Rc;

let data = Rc::new(vec![1, 2, 3]);
let shared = Rc::new(42);

let closure = {
    let data = Rc::clone(&data);  // Clone what we need
    let shared = Rc::clone(&shared);
    move || {
        println!("data: {:?}, shared: {}", data, shared);
    }
};
// Original `data` and `shared` still usable here
```

**When to use:** Complex closures where capture behavior should be explicit.

**When to avoid:** Simple closures where default capture is clear.

---

### 10. Return Consumed Argument on Error

**Problem:** Functions that consume arguments make retry difficult on failure.

**Solution:** Return the consumed argument in the error type.

```rust
pub struct SendError<T>(pub T);

pub fn send<T>(value: T) -> Result<(), SendError<T>> {
    if network_available() {
        do_send(&value);
        Ok(())
    } else {
        Err(SendError(value)) // Return value so caller can retry
    }
}

// Caller can recover and retry
let mut msg = create_message();
loop {
    match send(msg) {
        Ok(()) => break,
        Err(SendError(returned_msg)) => {
            msg = returned_msg;
            wait_for_network();
        }
    }
}
```

**When to use:** Fallible operations on owned data where retry is common.

**When to avoid:** When the data can't meaningfully be retried.

---

## Part 2: Behavioral Patterns

### 11. Newtype Pattern

**Problem:** Need type safety for wrapper types, or to implement traits on foreign types.

**Solution:** Wrap the type in a single-field tuple struct.

```rust
// Type safety: can't mix up Miles and Kilometers
struct Miles(f64);
struct Kilometers(f64);

impl Miles {
    fn to_kilometers(self) -> Kilometers {
        Kilometers(self.0 * 1.60934)
    }
}

// Implement foreign trait on foreign type
struct MyVec(Vec<i32>);

impl std::fmt::Display for MyVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.0.iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", "))
    }
}

// Encapsulate implementation details
pub struct UserId(u64);  // Internal representation hidden
```

**When to use:** Unit type safety, implementing traits on foreign types, hiding internals.

**When to avoid:** When the boilerplate of forwarding methods isn't worth it.

---

### 12. RAII Guards

**Problem:** Resources need cleanup, and access should be mediated safely.

**Solution:** Return a guard type that provides access and cleans up on drop.

```rust
use std::ops::Deref;

struct Connection { /* ... */ }
struct ConnectionGuard<'a> {
    connection: &'a Connection,
    // Internal state for cleanup
}

impl<'a> Deref for ConnectionGuard<'a> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.connection
    }
}

impl Drop for ConnectionGuard<'_> {
    fn drop(&mut self) {
        // Release the connection back to pool
    }
}

impl ConnectionPool {
    fn acquire(&self) -> ConnectionGuard<'_> {
        // Lock, get connection, return guard
        ConnectionGuard { connection: /* ... */ }
    }
}

// Usage: connection released when guard drops
{
    let conn = pool.acquire();
    conn.execute("SELECT 1");
} // Guard dropped, connection returned to pool
```

**When to use:** `Mutex`, connection pools, file locks, scoped resources.

**When to avoid:** When simple ownership suffices without the guard pattern.

---

### 13. Strategy Pattern

**Problem:** Need interchangeable algorithms without modifying context code.

**Solution:** Use traits or closures to inject behavior.

```rust
// Trait-based strategy
trait Formatter {
    fn format(&self, data: &Data) -> String;
}

struct JsonFormatter;
impl Formatter for JsonFormatter {
    fn format(&self, data: &Data) -> String {
        serde_json::to_string(data).unwrap()
    }
}

struct Report;
impl Report {
    fn generate<F: Formatter>(formatter: F, data: &Data) -> String {
        formatter.format(data)
    }
}

// Closure-based strategy (simpler for single methods)
fn process_with<F>(items: &[i32], strategy: F) -> i32
where
    F: Fn(i32, i32) -> i32
{
    items.iter().copied().reduce(strategy).unwrap_or(0)
}

let sum = process_with(&[1, 2, 3], |a, b| a + b);
let max = process_with(&[1, 2, 3], std::cmp::max);
```

**When to use:** Interchangeable algorithms, dependency injection, testing.

**When to avoid:** When behavior never varies; unnecessary abstraction.

---

### 14. Command Pattern

**Problem:** Need to encapsulate operations as objects for queuing, undo, or logging.

**Solution:** Define operations as types implementing a common trait.

```rust
trait Command {
    fn execute(&self) -> Result<(), Error>;
    fn undo(&self) -> Result<(), Error>;
}

struct CreateUser { username: String }
impl Command for CreateUser {
    fn execute(&self) -> Result<(), Error> {
        db::insert_user(&self.username)
    }
    fn undo(&self) -> Result<(), Error> {
        db::delete_user(&self.username)
    }
}

struct CommandQueue {
    history: Vec<Box<dyn Command>>,
}

impl CommandQueue {
    fn execute(&mut self, cmd: Box<dyn Command>) -> Result<(), Error> {
        cmd.execute()?;
        self.history.push(cmd);
        Ok(())
    }

    fn undo_last(&mut self) -> Result<(), Error> {
        if let Some(cmd) = self.history.pop() {
            cmd.undo()?;
        }
        Ok(())
    }
}
```

**When to use:** Undo/redo, command queues, transaction logs, macro recording.

**When to avoid:** Simple direct function calls without undo requirements.

---

### 15. Visitor Pattern

**Problem:** Need to perform different operations on heterogeneous data without modifying it.

**Solution:** Define a visitor trait that visits each type in the data structure.

```rust
// Data structure (doesn't change when adding operations)
enum Expr {
    Num(i64),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

// Visitor trait
trait Visitor {
    fn visit_num(&mut self, n: i64);
    fn visit_add(&mut self, left: &Expr, right: &Expr);
    fn visit_mul(&mut self, left: &Expr, right: &Expr);
}

impl Expr {
    fn accept(&self, visitor: &mut dyn Visitor) {
        match self {
            Expr::Num(n) => visitor.visit_num(*n),
            Expr::Add(l, r) => visitor.visit_add(l, r),
            Expr::Mul(l, r) => visitor.visit_mul(l, r),
        }
    }
}

// Add new operations without modifying Expr
struct Evaluator { result: i64 }
impl Visitor for Evaluator { /* ... */ }

struct PrettyPrinter { output: String }
impl Visitor for PrettyPrinter { /* ... */ }
```

**When to use:** ASTs, document processing, operations on complex hierarchies.

**When to avoid:** Homogeneous data where iterators suffice.

---

### 16. Type State Pattern

**Problem:** Invalid state transitions should be compile-time errors.

**Solution:** Use generics to encode state in the type system.

```rust
struct Unvalidated;
struct Validated;

struct Request<State> {
    data: String,
    _state: std::marker::PhantomData<State>,
}

impl Request<Unvalidated> {
    fn new(data: String) -> Self {
        Request { data, _state: std::marker::PhantomData }
    }

    fn validate(self) -> Result<Request<Validated>, Error> {
        if self.data.is_empty() {
            return Err(Error::EmptyData);
        }
        Ok(Request { data: self.data, _state: std::marker::PhantomData })
    }
}

impl Request<Validated> {
    // This method only exists for validated requests
    fn process(&self) -> Response {
        // Can assume data is valid
        process_data(&self.data)
    }
}

// Compile error: process() doesn't exist on unvalidated request
// let r = Request::new("data".into());
// r.process(); // ERROR!
```

**When to use:** State machines, protocol implementations, ensuring operation ordering.

**When to avoid:** Simple validations where runtime checks suffice.

---

## Part 3: Creational Patterns

### 17. Builder Pattern

**Problem:** Complex object construction with many optional parameters.

**Solution:** Use a separate builder type with chainable methods.

```rust
#[derive(Debug)]
pub struct Session {
    players: u32,
    input_delay: u32,
    max_prediction: u32,
}

#[derive(Default)]
pub struct SessionBuilder {
    players: u32,
    input_delay: u32,
    max_prediction: u32,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn players(mut self, n: u32) -> Self {
        self.players = n;
        self
    }

    pub fn input_delay(mut self, frames: u32) -> Self {
        self.input_delay = frames;
        self
    }

    pub fn max_prediction(mut self, frames: u32) -> Self {
        self.max_prediction = frames;
        self
    }

    pub fn build(self) -> Result<Session, BuildError> {
        if self.players == 0 {
            return Err(BuildError::NoPlayers);
        }
        Ok(Session {
            players: self.players,
            input_delay: self.input_delay,
            max_prediction: self.max_prediction,
        })
    }
}

// Usage
let session = SessionBuilder::new()
    .players(2)
    .input_delay(2)
    .build()?;
```

**When to use:** Many optional parameters, validation at build time, progressive configuration.

**When to avoid:** Simple structs with few fields; just use struct literal syntax.

---

### 18. Fold Pattern

**Problem:** Need to transform a data structure into a new one while traversing.

**Solution:** Define a folder trait with methods for each node type.

```rust
// Original AST
enum Expr {
    Var(String),
    Add(Box<Expr>, Box<Expr>),
}

// Folder trait - transforms Expr to Expr
trait Folder {
    fn fold_var(&mut self, name: String) -> Expr {
        Expr::Var(name) // default: identity
    }

    fn fold_add(&mut self, left: Box<Expr>, right: Box<Expr>) -> Expr {
        Expr::Add(
            Box::new(self.fold_expr(*left)),
            Box::new(self.fold_expr(*right)),
        )
    }

    fn fold_expr(&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::Var(n) => self.fold_var(n),
            Expr::Add(l, r) => self.fold_add(l, r),
        }
    }
}

// Rename all variables
struct Renamer;
impl Folder for Renamer {
    fn fold_var(&mut self, _name: String) -> Expr {
        Expr::Var("x".to_string())
    }
}
```

**When to use:** AST transformations, deep cloning with modifications.

**When to avoid:** Simple transformations where `map` suffices.

---

## Part 4: Structural Patterns

### 19. Struct Decomposition for Independent Borrowing

**Problem:** Borrow checker prevents using multiple fields simultaneously.

**Solution:** Split large structs into smaller, independently borrowable pieces.

```rust
// ❌ Problem: Can't borrow parts independently
struct Game {
    renderer: Renderer,
    physics: Physics,
    state: State,
}

impl Game {
    fn update(&mut self) {
        // Can't do this: both need &mut self
        // self.physics.update(&mut self.state);
        // self.renderer.draw(&self.state);
    }
}

// ✅ Solution: Decompose into borrowable components
struct GameRenderer { /* ... */ }
struct GamePhysics { /* ... */ }
struct GameState { /* ... */ }

struct Game {
    renderer: GameRenderer,
    physics: GamePhysics,
    state: GameState,
}

impl Game {
    fn update(&mut self) {
        // Now we can borrow fields independently
        self.physics.update(&mut self.state);
        self.renderer.draw(&self.state);
    }
}
```

**When to use:** Large structs with fields that need independent mutable access.

**When to avoid:** When single-responsibility design naturally avoids the problem.

---

### 20. Prefer Small Crates

**Problem:** Large monolithic libraries are hard to maintain and slow to compile.

**Solution:** Split functionality into small, focused crates.

```rust
// Instead of one mega-crate, have focused crates:
//
// networking/     - Just network transport
// serialization/  - Just serialization
// protocol/       - Just protocol definitions
// sync/           - Just synchronization logic

// Benefits:
// - Parallel compilation
// - Easier to understand
// - Reusable across projects
// - Faster incremental builds
```

**When to use:** Libraries with separable concerns, reusable components.

**When to avoid:** Tightly coupled code where separation adds complexity.

---

### 21. Custom Traits for Complex Bounds

**Problem:** Complex trait bounds make code hard to read.

**Solution:** Define a custom trait that encapsulates the bounds.

```rust
// ❌ Complex and repetitive
fn process<F, T>(f: F) -> T
where
    F: FnMut() -> Result<T, Error>,
    T: Display + Debug + Clone,
{ /* ... */ }

// ✅ Encapsulate in custom trait
trait Processor {
    type Output: Display + Debug + Clone;
    fn process(&mut self) -> Result<Self::Output, Error>;
}

// Blanket impl for closures
impl<F, T> Processor for F
where
    F: FnMut() -> Result<T, Error>,
    T: Display + Debug + Clone,
{
    type Output = T;
    fn process(&mut self) -> Result<T, Error> {
        self()
    }
}

// Clean function signature
fn run<P: Processor>(mut p: P) -> P::Output {
    p.process().unwrap()
}
```

**When to use:** Repeated complex bounds, especially with `Fn` traits.

**When to avoid:** Simple one-off bounds.

---

### 22. Collections as Smart Pointers

**Problem:** Collections own data but users need borrowed views.

**Solution:** Implement `Deref` to provide borrowed view of owned data.

```rust
use std::ops::Deref;

struct MyVec<T> {
    data: Vec<T>,
}

impl<T> Deref for MyVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.data
    }
}

// Now MyVec can use all slice methods
let v = MyVec { data: vec![1, 2, 3] };
let first = v.first(); // Slice method works!
let len = v.len();     // Slice method works!
```

**When to use:** Owned collections that should expose borrowed views.

**When to avoid:** Non-collection types (see Deref Polymorphism anti-pattern).

---

## Part 5: Anti-Patterns to Avoid

### 23. ❌ Clone to Satisfy Borrow Checker

**Problem:** Using `.clone()` to make borrow checker errors go away.

**Why it's bad:** Unnecessary allocations, hides design problems, defeats ownership benefits.

```rust
// ❌ Anti-pattern: Cloning to avoid borrow issues
fn process(data: &mut Data) {
    let copy = data.items.clone(); // Unnecessary clone!
    for item in copy {
        data.process_item(item);
    }
}

// ✅ Better: Restructure to avoid the need
fn process(data: &mut Data) {
    let indices: Vec<_> = (0..data.items.len()).collect();
    for i in indices {
        let item = data.items[i].clone(); // Clone only if truly needed
        data.process_item(item);
    }
}

// ✅ Best: Use mem::take or split_at_mut
fn process(data: &mut Data) {
    let items = std::mem::take(&mut data.items);
    for item in items {
        data.process_item(item);
    }
}
```

**Fix:** Understand ownership, use `mem::take`, restructure code, or accept the clone consciously.

---

### 24. ❌ Deref Polymorphism

**Problem:** Using `Deref` to emulate inheritance.

**Why it's bad:** Confusing semantics, doesn't work with trait bounds, surprising behavior.

```rust
// ❌ Anti-pattern: Using Deref for "inheritance"
struct Base { /* ... */ }
impl Base {
    fn base_method(&self) { /* ... */ }
}

struct Derived {
    base: Base,
}

impl Deref for Derived {
    type Target = Base;
    fn deref(&self) -> &Base { &self.base }
}

// Now derived.base_method() works, but this is misleading!

// ✅ Better: Explicit delegation or traits
impl Derived {
    fn base_method(&self) {
        self.base.base_method()
    }
}

// Or use delegation crates like `delegate` or `ambassador`
```

**Fix:** Use composition with explicit delegation, or traits for shared behavior.

---

### 25. ❌ `#![deny(warnings)]` in Published Crates

**Problem:** Denying all warnings makes builds fragile across Rust versions.

**Why it's bad:** New compiler versions may add warnings, breaking builds unexpectedly.

```rust
// ❌ Anti-pattern in lib.rs
#![deny(warnings)]

// ✅ Better: Deny specific lints
#![deny(
    unsafe_code,
    missing_docs,
    unused_must_use,
)]

// ✅ Best: Use RUSTFLAGS in CI instead
// RUSTFLAGS="-D warnings" cargo build
```

**Fix:** Deny specific lints, or use `RUSTFLAGS` in CI only.

---

## Part 6: API Guidelines

### 26. Naming Conventions (C-CASE, C-CONV)

Follow RFC 430 naming:

| Item | Convention | Example |
|------|------------|---------|
| Types, Traits | `UpperCamelCase` | `HashMap`, `Iterator` |
| Functions, Methods | `snake_case` | `get_value`, `into_inner` |
| Constants, Statics | `SCREAMING_SNAKE_CASE` | `MAX_SIZE` |
| Type Parameters | Single uppercase | `T`, `E`, `K`, `V` |
| Lifetimes | Short lowercase | `'a`, `'de`, `'src` |

**Conversion method prefixes:**

| Prefix | Cost | Ownership Change |
|--------|------|------------------|
| `as_` | Free | `&T` → `&U` |
| `to_` | Expensive | `&T` → `U` |
| `into_` | Variable | `T` → `U` (consumes) |

```rust
impl MyType {
    fn as_bytes(&self) -> &[u8] { /* ... */ }     // Free, borrowed
    fn to_string(&self) -> String { /* ... */ }   // Allocates
    fn into_inner(self) -> Inner { /* ... */ }    // Consumes self
}
```

---

### 27. Implement Common Traits (C-COMMON-TRAITS)

Types should eagerly implement applicable standard traits:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct PlayerId(u32);

// Consider also:
// - Copy (if small and doesn't manage resources)
// - Ord, PartialOrd (if ordering makes sense)
// - Display (for user-facing output)
// - Serialize, Deserialize (behind feature flag)
```

**Why:** The orphan rule prevents downstream users from adding these impls.

---

### 28. Meaningful Error Types (C-GOOD-ERR)

Never use `()` as an error type:

```rust
// ❌ Anti-pattern
fn parse(s: &str) -> Result<Value, ()>

// ✅ Proper error type
#[derive(Debug)]
pub struct ParseError {
    line: usize,
    message: String,
}

impl std::error::Error for ParseError {}
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at line {}: {}", self.line, self.message)
    }
}

fn parse(s: &str) -> Result<Value, ParseError>
```

Error messages should be: lowercase, no trailing punctuation, concise.

---

### 29. Sealed Traits for Future-Proofing (C-SEALED)

Prevent external implementations when you need to add methods later:

```rust
mod private {
    pub trait Sealed {}
}

/// This trait is sealed and cannot be implemented outside this crate.
pub trait Protocol: private::Sealed {
    fn encode(&self) -> Vec<u8>;
    fn decode(bytes: &[u8]) -> Result<Self, Error> where Self: Sized;
}

// Only implement Sealed (and thus Protocol) for your types
impl private::Sealed for MessageV1 {}
impl Protocol for MessageV1 { /* ... */ }

// External crates can't implement Protocol because they can't
// implement private::Sealed
```

**When to use:** Traits you may need to extend in non-breaking releases.

---

### 30. Functions Should Use Generics (C-GENERIC)

Accept the most general type that provides needed functionality:

```rust
// ❌ Overly specific
fn read_config(path: &PathBuf) -> Config { /* ... */ }

// ✅ Generic over AsRef<Path>
fn read_config(path: impl AsRef<Path>) -> Config {
    let path = path.as_ref();
    // ...
}

// Now accepts: &str, String, PathBuf, &Path, OsString, etc.
read_config("config.toml");
read_config(PathBuf::from("config.toml"));
```

---

## Quick Decision Guide

| Situation | Pattern to Use |
|-----------|----------------|
| Many optional constructor params | Builder |
| Wrap primitive for type safety | Newtype |
| Ensure cleanup on scope exit | RAII Guard |
| Interchangeable algorithms | Strategy |
| Queueable/undoable operations | Command |
| Operations on heterogeneous tree | Visitor |
| Compile-time state machine | Type State |
| Need `&str` not `&String` | Borrowed Types |
| Fields need independent borrowing | Struct Decomposition |

---

## References

- [Rust Design Patterns Book](https://rust-unofficial.github.io/patterns/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [The Rust Book - Patterns](https://doc.rust-lang.org/book/ch18-00-patterns.html)
