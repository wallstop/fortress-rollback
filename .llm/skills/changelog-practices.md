# Changelog Practices — Documenting User-Observable Changes

> **When in doubt, add a CHANGELOG entry.** Users benefit from knowing what changed.

## TL;DR — When to Update CHANGELOG

**ALWAYS document:**

- Public API changes (new functions, changed signatures, removed items)
- Behavior changes (different output for same input)
- Default value changes
- Error type/message changes users might match on
- Performance improvements users would notice
- Bug fixes affecting user-visible behavior

**NEVER document:**

- Internal refactoring with no external effect
- `pub(crate)` or private API changes
- Test-only changes
- Documentation typo fixes
- CI/tooling changes (unless affecting published crate)

---

## The Golden Rule

**If a user's code could behave differently, or they need to change their code, document it.**

```rust
// ❌ Internal change — NO changelog needed
pub(crate) fn internal_helper() -> Result<(), Error> { ... }

// ✅ Public API change — MUST document
pub fn public_function() -> Result<(), Error> { ... }
```

---

## What Requires Documentation

### Breaking Changes (MUST document)

Always document under `### Changed` with **Breaking:** prefix:

```markdown
### Changed

- **Breaking:** `SessionBuilder::with_input_delay()` now returns `Result<Self, FortressError>` instead of panicking on invalid values
```

Examples:

- Changing function signatures
- Removing public items
- Adding required trait bounds
- Changing return types
- Changing default behavior

### New Features (SHOULD document)

Document under `### Added`:

```markdown
### Added

- `ProtocolConfig::deterministic(seed)` preset for reproducible sessions
- `SessionBuilder::with_event_queue_size()` for configurable queue capacity
```

### Bug Fixes (SHOULD document)

Document under `### Fixed`:

```markdown
### Fixed

- Fixed crash when misprediction detected at frame 0
- Fixed sync timeout event flooding under certain conditions
```

### Behavioral Changes (MUST document)

Even if not breaking, document observable changes:

```markdown
### Changed

- Desync detection now enabled by default (`DesyncDetection::On { interval: 60 }`)
- Reduced memory allocation in network hot paths
```

---

## What Does NOT Require Documentation

### Internal Implementation Changes

```rust
// Before: assert that panics
pub(crate) fn synchronize(&mut self) {
    assert_eq!(self.state, ProtocolState::Initializing);
    // ...
}

// After: returns Result
pub(crate) fn synchronize(&mut self) -> Result<(), FortressError> {
    if self.state != ProtocolState::Initializing {
        return Err(FortressError::InvalidRequest { ... });
    }
    // ...
}
```

This is `pub(crate)` — users never see it. No changelog entry needed.

### Test Changes

```rust
#[cfg(test)]
mod tests {
    // Any changes here — no changelog needed
}
```

### CI/Tooling Changes

Unless they affect the published crate:

- Workflow updates
- Linter configuration
- Development dependencies

---

## CHANGELOG Format

Follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format:

```markdown
## [Unreleased]

### Added
- New features

### Changed
- Changes to existing functionality
- **Breaking:** prefix for breaking changes

### Deprecated
- Soon-to-be removed features

### Removed
- Removed features

### Fixed
- Bug fixes

### Security
- Vulnerability fixes
```

---

## Entry Writing Guidelines

### Be User-Focused

```markdown
# ❌ Too technical/internal
- Replaced HashMap with BTreeMap in sync_manager.rs

# ✅ User-focused
- Improved iteration order determinism in session state
```

### Be Concise

```markdown
# ❌ Too verbose
- Added a new method called `with_event_queue_size` to the `SessionBuilder` struct
  that allows users to configure the size of the event queue capacity, which
  determines how many events can be buffered before...

# ✅ Concise
- `SessionBuilder::with_event_queue_size()` for configurable event queue capacity
```

### Include Context for Breaking Changes

```markdown
# ❌ Missing migration guidance
- **Breaking:** Changed `with_input_delay()` return type

# ✅ Includes guidance
- **Breaking:** `SessionBuilder::with_input_delay()` now returns `Result<Self, FortressError>` instead of silently clamping invalid values
```

---

## Visibility Reference

| Visibility | User-Facing? | Changelog Needed? |
|------------|--------------|-------------------|
| `pub` | Yes | Yes, for any change |
| `pub(crate)` | No | No |
| `pub(super)` | No | No |
| `pub(in path)` | No | No |
| private | No | No |

---

## Checklist Before Committing

When making changes, ask:

1. **Is this `pub`?** If yes, consider changelog entry
2. **Does behavior change?** If yes, document it
3. **Could user code break?** If yes, mark as **Breaking:**
4. **Is this a bug fix users would care about?** If yes, document it
5. **Is this purely internal?** If yes, skip changelog

---

## Example: Determining Changelog Need

```rust
// Change: assert! to Result

// Case 1: Public method
pub fn connect(&mut self) -> Result<(), Error> { ... }
// → YES, document: "**Breaking:** `connect()` now returns Result"

// Case 2: pub(crate) method
pub(crate) fn synchronize(&mut self) -> Result<(), Error> { ... }
// → NO, internal only

// Case 3: Public method, behavior change only
pub fn validate(&self) -> bool {
    // Was: only checked field A
    // Now: checks fields A and B
}
// → YES, document: "Changed: `validate()` now checks additional fields"
```

---

## Example Code Maintenance

When documenting API changes, also update example code:

### Locations to Check

| Location | Purpose |
|----------|--------|
| `examples/*.rs` | Runnable standalone examples |
| `README.md` | Quick start code snippets |
| `docs/user-guide.md` | Detailed usage examples |
| Rustdoc `# Examples` | Inline documentation examples |

### Verification Commands

```bash
# Ensure all examples compile
cargo build --examples

# Ensure rustdoc examples compile
cargo test --doc

# Find references to changed APIs
rg 'changed_function|ChangedStruct' --type rust --type md
```

### Why This Matters

- Outdated examples confuse users and erode trust
- Broken examples in README are often the first impression
- CI may not catch example drift if examples aren't compiled

**Rule:** If you change a `pub` API, search the codebase for all usages before committing.

---

## Verification Before Committing

**Always verify CHANGELOG claims match actual code:**

```bash
# Verify derives exist before claiming them
rg '#\[derive.*Hash' src/lib.rs

# Verify method/type exists
rg 'pub fn method_name|pub struct TypeName' --type rust

# Build docs to catch broken links
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

> **See also:** [documentation-code-consistency.md](documentation-code-consistency.md) for comprehensive verification commands and common pitfalls.

---

## References

- [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
- [Semantic Versioning](https://semver.org/)
- [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
- [documentation-code-consistency.md](documentation-code-consistency.md) — Keeping docs and code in sync
