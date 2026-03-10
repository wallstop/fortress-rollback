<!-- CATEGORY: Publishing & Organization -->
<!-- WHEN: Writing CHANGELOG entries, deciding what to document -->
# Changelog Practices

## Decision Table

| Change Type | Changelog? | Section | Prefix |
|-------------|-----------|---------|--------|
| New public function/type/trait impl | Yes | Added | |
| Changed function signature | Yes | Changed | **Breaking:** |
| Removed public item | Yes | Removed | **Breaking:** |
| New enum variant (exhaustive enum) | Yes | Changed | **Breaking:** |
| New enum variant (`#[non_exhaustive]`) | Yes | Added | |
| Changed `Display`/`Debug` output | Yes | Changed | **Breaking:** |
| Bug fix (user-visible) | Yes | Fixed | |
| Default value change | Yes | Changed | |
| Performance improvement (noticeable) | Yes | Changed | |
| New trait impl (`Display`, `Hash`, etc.) | Yes | Added | |
| `pub(crate)` or private changes | No | -- | -- |
| Test-only changes | No | -- | -- |
| CI/tooling changes | No | -- | -- |
| Internal refactoring | No | -- | -- |
| Doc typo fixes | No | -- | -- |

**Golden rule:** If a user's code could behave differently or they need to change their code, document it.

## Visibility Reference

| Visibility | User-Facing? | Changelog? |
|------------|--------------|------------|
| `pub` | Yes | Yes |
| `pub(crate)` | No | No |
| `pub(super)` | No | No |
| private | No | No |

## Format (Keep a Changelog)

```markdown
## [Unreleased]

### Added
- `ProtocolConfig::deterministic(seed)` preset for reproducible sessions

### Changed
- **Breaking:** `SessionBuilder::with_input_delay()` now returns `Result<Self, FortressError>` instead of panicking on invalid values

### Fixed
- Fixed crash when misprediction detected at frame 0
```

Sections: Added, Changed, Deprecated, Removed, Fixed, Security.

## Writing Guidelines

### Be User-Focused

```markdown
# WRONG: too internal
- Replaced HashMap with BTreeMap in sync_manager.rs

# CORRECT
- Improved iteration order determinism in session state
```

### Be Concise

```markdown
# WRONG: verbose
- Added a new method called `with_event_queue_size` to the `SessionBuilder` struct...

# CORRECT
- `SessionBuilder::with_event_queue_size()` for configurable event queue capacity
```

### Include Migration for Breaking Changes

```markdown
# WRONG: no guidance
- **Breaking:** Changed `with_input_delay()` return type

# CORRECT
- **Breaking:** `SessionBuilder::with_input_delay()` now returns `Result<Self, FortressError>` instead of silently clamping invalid values
```

## Enum Variants Are Breaking (Unless `#[non_exhaustive]`)

```rust
// Adding variants to this is BREAKING
pub enum ConnectionState { Disconnected, Connecting, Connected }

// Adding variants to this is NOT breaking
#[non_exhaustive]
pub enum ConnectionState { Disconnected, Connecting, Connected }
```

CHANGELOG entry for new variant on exhaustive enum:

```markdown
### Changed
- **Breaking:** Added `ConnectionState::Syncing` variant. Update exhaustive matches.
```

## Trait Implementations

Document under Added:

```markdown
### Added
- `Display` implementation for `Frame` and `PlayerHandle`
- `Hash` implementation for `SessionConfig`
```

## Output Format Changes

Changing `Display`/`Debug` output is **Breaking** if users might depend on format:

```markdown
### Changed
- **Breaking:** `PlayerHandle` now displays as `PlayerHandle(N)` instead of `Player N`
```

## Verification Before Committing

```bash
# Verify derives exist before claiming them
rg '#\[derive.*Hash' src/lib.rs

# Verify method/type exists
rg 'pub fn method_name|pub struct TypeName' --type rust

# Build docs to catch broken links
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

## Example Code Maintenance

When documenting API changes, also update examples:

```bash
cargo build --examples     # Verify examples compile
cargo test --doc           # Verify doc examples
rg 'changed_function|ChangedStruct' --type rust --type md  # Find stale refs
```

Locations: `examples/*.rs`, `README.md`, `docs/user-guide.md`, rustdoc `# Examples`.

## Internal-Only Release

If a release has no user-facing changes, add a single summary line:

```markdown
- Internal: Improved test coverage and code organization
```
