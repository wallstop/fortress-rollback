<!-- CATEGORY: Rust Language -->
<!-- WHEN: Designing public APIs, checking semver compliance, reviewing breaking changes -->
# Public API Design

## Key Principles

1. **Minimize surface area** -- expose only what users truly need
2. **Hide implementation** -- newtypes, `pub(crate)`, private fields
3. **Design for stability** -- `#[non_exhaustive]` judiciously
4. **Document everything** -- every public item needs rustdoc
5. **Re-export dependencies** -- users should not hunt for your deps

## Visibility Rules

| Level | When |
|-------|------|
| `fn` (private) | Default -- accessible only in module |
| `pub(crate)` | Crate-internal helpers |
| `pub(super)` | Parent module helpers |
| `pub` | Intentional public API -- forever commitment |

**Never expose struct fields directly** unless fundamental to meaning. Use accessor methods.

## `#[non_exhaustive]`

| Use on | When |
|--------|------|
| Error enums | May gain variants in minor releases |
| Config structs | May gain fields |
| **Avoid on** | Fixed-set enums where exhaustive matching catches bugs |

## Newtype Pattern

- Wrap external dependency types -- changing deps should not break users
- Create distinct types for values that should not be mixed (`Frame`, `PlayerHandle`, `Port`)
- Derive useful traits: `Clone, Copy, Debug, PartialEq, Eq, Hash`

## Trait Design

- **Minimal bounds** -- document why each bound is needed
- **Re-export** traits users need to use your API
- **Associated types** for flexibility over fixed types
- **Combined traits** with blanket impls for common bound sets

## Error Handling

### Scoped Error Types
```rust
pub mod session {
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum CreateError { /* ... */ }

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum AdvanceError { /* ... */ }
}
```

### Error Documentation
```rust
/// # Errors
///
/// Returns [`CreateError::InvalidConfig`] if config.max_players is zero.
/// Returns [`CreateError::NetworkBind`] if UDP socket cannot bind.
pub fn create(config: SessionConfig) -> Result<Session, CreateError> { /* ... */ }
```

## Documentation

```rust
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
```

- Crate-level docs with quick-start example
- Every public item documented with examples
- Use intra-doc links: `[`Session::rollback_count`]`
- Document feature flags in crate-level docs

## Re-exports and Prelude

```rust
pub use bytes::{Bytes, BytesMut}; // re-export dep types in public API
pub mod prelude; // common imports for convenience
pub type Result<T, E = Error> = std::result::Result<T, E>; // result alias
```

**Result alias hazard:** Use distinctive names (`FortressResult`) to avoid shadowing `std::result::Result`.

## Feature Flags

- **Must be additive** -- enabling multiple features must compile
- **Document all features** in crate-level docs table
- **Document feature-dependent bounds** (e.g., `+ Send + Sync` only with `sync` feature)

```toml
[features]
default = []
serde = ["dep:serde"]
async = ["dep:tokio"]
full = ["serde", "async"]
```

## Semver Rules

### Breaking Changes (MAJOR bump, or MINOR pre-1.0)

- [ ] Removing public items
- [ ] Changing function signatures
- [ ] Adding required struct fields
- [ ] Adding required trait methods (without defaults)
- [ ] Changing enum variants (unless `#[non_exhaustive]`)
- [ ] Tightening trait bounds
- [ ] Removing trait implementations
- [ ] Changing MSRV

```bash
cargo install cargo-semver-checks
cargo semver-checks check-release
```

## API Review Checklist

### Visibility
- [ ] All items private by default?
- [ ] `pub(crate)` for internal items?
- [ ] Struct fields private with accessors?

### Types
- [ ] Newtypes wrap external dep types?
- [ ] Similar primitives distinguished by type?
- [ ] Trait bounds minimal and documented?

### Enums
- [ ] `#[non_exhaustive]` only where catch-all is acceptable?
- [ ] Error enums scoped to specific operations?

### Errors
- [ ] Distinct operations have distinct error types?
- [ ] Errors include context for debugging?
- [ ] `# Errors` section in docs?

### Documentation
- [ ] `#![warn(missing_docs)]` enabled?
- [ ] Examples tested via doctests?
- [ ] Intra-doc links to related items?

### Versioning
- [ ] Semantic versioning followed?
- [ ] `cargo semver-checks` run?
- [ ] Breaking changes documented?

## Anti-Patterns

| Anti-Pattern | Solution |
|--------------|----------|
| Public fields | Private fields + accessors |
| Exposing dep types | Newtype wrappers |
| One mega error type | Scoped error types |
| Complex trait bounds | Minimal bounds |
| Missing `#[non_exhaustive]` on errors | Add annotation |
| `#[non_exhaustive]` on fixed enums | Keep exhaustive |
| Missing re-exports | Re-export public deps |
| Breaking changes in minor | cargo-semver-checks |
