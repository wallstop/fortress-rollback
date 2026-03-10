<!-- CATEGORY: Rust Language -->
<!-- WHEN: Choosing design patterns, implementing builders, state machines, or strategy pattern -->
# Rust Design Patterns

## Builder Pattern

Use for complex object construction with many optional parameters:

```rust
#[derive(Default)]
pub struct SessionBuilder {
    players: u32,
    input_delay: u32,
    max_prediction: u32,
}

impl SessionBuilder {
    pub fn new() -> Self { Self::default() }
    pub fn players(mut self, n: u32) -> Self { self.players = n; self }
    pub fn input_delay(mut self, frames: u32) -> Self { self.input_delay = frames; self }
    pub fn build(self) -> Result<Session, BuildError> {
        if self.players == 0 { return Err(BuildError::NoPlayers); }
        Ok(Session { players: self.players, input_delay: self.input_delay, max_prediction: self.max_prediction })
    }
}
```

**Use when:** Many optional params, validation at build time. **Avoid:** Simple structs with few fields.

## Type State Pattern

Encode state transitions in the type system -- invalid transitions become compile errors:

```rust
struct Unvalidated;
struct Validated;

struct Request<State> {
    data: String,
    _state: PhantomData<State>,
}

impl Request<Unvalidated> {
    fn validate(self) -> Result<Request<Validated>, Error> {
        if self.data.is_empty() { return Err(Error::EmptyData); }
        Ok(Request { data: self.data, _state: PhantomData })
    }
}

impl Request<Validated> {
    fn process(&self) -> Response { /* only available for validated requests */ }
}
```

**Use when:** State machines, protocol implementations, ensuring operation ordering.

## Strategy Pattern

Interchangeable algorithms via traits or closures:

```rust
// Trait-based
trait Formatter { fn format(&self, data: &Data) -> String; }

// Closure-based (simpler for single methods)
fn process_with<F: Fn(i32, i32) -> i32>(items: &[i32], strategy: F) -> i32 {
    items.iter().copied().reduce(strategy).unwrap_or(0)
}
```

## Command Pattern

Encapsulate operations as objects for queuing/undo:

```rust
trait Command {
    fn execute(&self) -> Result<(), Error>;
    fn undo(&self) -> Result<(), Error>;
}

struct CommandQueue { history: Vec<Box<dyn Command>> }
```

**Use when:** Undo/redo, command queues, transaction logs.

## Visitor Pattern

Operations on heterogeneous data without modifying it:

```rust
trait Visitor {
    fn visit_num(&mut self, n: i64);
    fn visit_add(&mut self, left: &Expr, right: &Expr);
}

impl Expr {
    fn accept(&self, visitor: &mut dyn Visitor) {
        match self {
            Expr::Num(n) => visitor.visit_num(*n),
            Expr::Add(l, r) => visitor.visit_add(l, r),
        }
    }
}
```

**Use when:** ASTs, document processing. **Avoid:** Homogeneous data where iterators suffice.

## Newtype Pattern

Type safety for wrappers, or implementing traits on foreign types:

```rust
struct Miles(f64);
struct Kilometers(f64);
impl Miles {
    fn to_kilometers(self) -> Kilometers { Kilometers(self.0 * 1.60934) }
}
```

## RAII Guards

Return a guard type that provides access and cleans up on drop:

```rust
struct ConnectionGuard<'a> { connection: &'a Connection }
impl<'a> Deref for ConnectionGuard<'a> {
    type Target = Connection;
    fn deref(&self) -> &Connection { self.connection }
}
impl Drop for ConnectionGuard<'_> {
    fn drop(&mut self) { /* release connection back to pool */ }
}
```

## Struct Decomposition for Independent Borrowing

```rust
struct Game { renderer: GameRenderer, physics: GamePhysics, state: GameState }
impl Game {
    fn update(&mut self) {
        self.physics.update(&mut self.state); // independent borrows
        self.renderer.draw(&self.state);
    }
}
```

## Custom Traits for Complex Bounds

```rust
trait Processor {
    type Output: Display + Debug + Clone;
    fn process(&mut self) -> Result<Self::Output, Error>;
}
// Blanket impl for closures
impl<F, T> Processor for F where F: FnMut() -> Result<T, Error>, T: Display + Debug + Clone {
    type Output = T;
    fn process(&mut self) -> Result<T, Error> { self() }
}
```

## Fold Pattern

Transform data structures while traversing:

```rust
trait Folder {
    fn fold_var(&mut self, name: String) -> Expr { Expr::Var(name) }
    fn fold_expr(&mut self, expr: Expr) -> Expr { /* dispatch */ }
}
```

## Anti-Patterns to Avoid

### Clone to Satisfy Borrow Checker

Use `mem::take`, restructure code, or accept the clone consciously -- do not blindly `.clone()`.

### Deref Polymorphism

Do not use `Deref` to emulate inheritance. Use composition with explicit delegation or traits.

### `#![deny(warnings)]` in Published Crates

Deny specific lints instead, or use `RUSTFLAGS="-D warnings"` in CI only.
