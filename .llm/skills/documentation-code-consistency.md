# Documentation-Code Consistency — Keeping Docs and Code in Sync

> **Documentation that lies is worse than no documentation.** Always verify that documentation accurately reflects the code.

## TL;DR — Key Verification Steps

**Before committing documentation or CHANGELOG entries:**

1. **Verify derives exist:** `rg '#\[derive.*Hash' --type rust` before claiming Hash in CHANGELOG
2. **Verify error variants:** Check actual `return Err(...)` statements match `# Errors` docs
3. **Run doc build:** `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
4. **Test doc examples:** `cargo test --doc`

---

## Problem Categories

### 1. CHANGELOG-Code Mismatch

**The Issue:** CHANGELOG claims a feature exists, but code doesn't have it.

```markdown
# ❌ CHANGELOG claims this:
- Added `Hash` derive to `PlayerHandle` for use in HashMaps
```

```rust
// ❌ But code only has:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]  // No Hash!
pub struct PlayerHandle(usize);
```

**Why It Happens:**

- CHANGELOG written before implementation
- Feature removed during review but CHANGELOG not updated
- Copy-paste from planning notes, not from actual code

### 2. Rustdoc Error Variant Mismatch

**The Issue:** Doc comments reference one error variant, but code returns a different one.

```rust
// ❌ Doc says InvalidRequest:
/// # Errors
///
/// Returns [`FortressError::InvalidRequest`] if the session is not running.

// ❌ But code returns InvalidRequestStructured:
return Err(FortressError::InvalidRequestStructured {
    operation: "advance_frame",
    reason: InvalidRequestReason::SessionNotRunning,
});
```

**Why It Happens:**

- Error types evolved (e.g., added structured variants)
- `From` implementations convert errors automatically
- Docs written before error handling finalized

### 3. Inconsistent Documentation Patterns

**The Issue:** Different files use different documentation styles.

```rust
// File A: Uses inline links
/// Returns [`FortressError::InvalidRequest`] if invalid.

// File B: Uses reference-style links
/// Returns [`InvalidRequest`] if invalid.
///
/// [`InvalidRequest`]: FortressError::InvalidRequest

// File C: Uses generic error type
/// Returns a [`FortressError`] if invalid.
```

---

## Prevention: Verification Commands

### Before CHANGELOG Entries

Always verify claimed features exist:

```bash
# Verify derive traits exist
rg '#\[derive.*Hash' src/lib.rs
rg '#\[derive.*Serialize' src/sessions/

# Verify method exists
rg 'pub fn method_name' --type rust

# Verify constant/type exists
rg 'pub const NAME|pub type Name' --type rust

# Verify feature flag works
cargo build --features claimed-feature
```

### Before Documentation Changes

Verify error variants are accurate:

```bash
# Find what error variants a function actually returns
rg 'return Err\(' src/sessions/builder.rs -A 2

# Check From implementations for error conversions
rg 'impl From<.*> for FortressError' src/error.rs

# Find all uses of a specific error variant
rg 'FortressError::InvalidRequest' --type rust
```

### Build and Test Documentation

**Always run before committing:**

```bash
# Strict doc build - catches broken links and warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Test all doc examples compile and run
cargo test --doc

# Check for broken intra-doc links specifically
cargo doc --no-deps 2>&1 | rg 'unresolved link'
```

---

## Best Practices: Error Documentation

### Option 1: Generic Error Type (Recommended for Stability)

Use when the specific error variant might change:

```rust
/// # Errors
///
/// Returns a [`FortressError`] if the delay exceeds the maximum value.
pub fn with_input_delay(self, delay: usize) -> Result<Self, FortressError> {
    // Implementation may use different variants over time
}
```

**Advantages:**

- Docs remain accurate even if error variants change
- Simpler to maintain
- Users know to expect `FortressError`, can match as needed

### Option 2: Specific Variant (When Stable and Important)

Use when the error type is stable and users need to match on it:

```rust
/// # Errors
///
/// Returns [`FortressError::InvalidFrameStructured`] with reason
/// [`InvalidFrameReason::NegativeValue`] if the frame is negative.
///
/// [`FortressError::InvalidFrameStructured`]: crate::FortressError::InvalidFrameStructured
/// [`InvalidFrameReason::NegativeValue`]: crate::InvalidFrameReason::NegativeValue
pub fn from_i32(value: i32) -> Result<Self, FortressError> {
    if value < 0 {
        return Err(FortressError::InvalidFrameStructured {
            frame_value: i64::from(value),
            reason: InvalidFrameReason::NegativeValue,
        });
    }
    Ok(Self(value))
}
```

**When to use:**

- Error variant is part of the stable API
- Users need to match on specific error conditions
- The variant has important structured data users need

### Verifying Error Documentation Accuracy

When documenting `# Errors`, verify the actual error returned:

```rust
// Step 1: Check what the function body actually returns
// Look for: return Err(...) or .map_err(...) or ?

// Step 2: Check From implementations for automatic conversions
// The actual variant might differ from what's written

// Step 3: Use the exact variant name from the code
```

---

## Best Practices: CHANGELOG Accuracy

### Verification Before Entry

**ALWAYS verify claims before writing CHANGELOG:**

```bash
# ❌ DON'T: Write CHANGELOG first, implement later
# (Easy to forget to update if implementation changes)

# ✅ DO: Verify feature exists, then document
rg '#\[derive.*Hash.*\]' src/lib.rs  # Verify Hash exists
# Then add to CHANGELOG:
# - Added `Hash` derive to `Frame` type
```

### Template for Verified Entries

```markdown
### Added

- `Frame` now implements `Hash` for use in hash-based collections
  <!-- Verified: rg '#\[derive.*Hash' src/lib.rs shows Frame has Hash -->
```

*Note: The verification comment is for your reference during writing. Remove it before committing.*

### Cross-Reference with Code

When writing CHANGELOG entries for derives or trait implementations:

```bash
# List all derives on a type
rg 'struct Frame' -A 1 src/lib.rs

# Find trait implementations
rg 'impl.*for Frame' --type rust

# Verify specific trait
rg 'impl Hash for Frame' --type rust
```

---

## Best Practices: Link Consistency

### Preferred Pattern: Generic Error Links

For most error documentation, use the generic pattern:

```rust
/// # Errors
///
/// Returns a [`FortressError`] if the configuration is invalid.
```

This is:

- Always accurate (the error type won't change)
- Simpler to maintain
- Consistent across the codebase

### When Using Specific Variant Links

If documenting specific variants, use reference-style links at the bottom:

```rust
/// # Errors
///
/// Returns [`FortressError::InvalidFrameStructured`] with:
/// - [`InvalidFrameReason::NegativeValue`] if the frame is negative
/// - [`InvalidFrameReason::Overflow`] if arithmetic overflows
///
/// [`FortressError::InvalidFrameStructured`]: crate::FortressError::InvalidFrameStructured
/// [`InvalidFrameReason::NegativeValue`]: crate::InvalidFrameReason::NegativeValue
/// [`InvalidFrameReason::Overflow`]: crate::InvalidFrameReason::Overflow
```

### Avoid Inline Long Paths

```rust
// ❌ Avoid: Long inline paths
/// Returns [`crate::error::FortressError::InvalidFrameStructured`]

// ✅ Prefer: Reference at bottom
/// Returns [`FortressError::InvalidFrameStructured`]
///
/// [`FortressError::InvalidFrameStructured`]: crate::FortressError::InvalidFrameStructured
```

---

## Checklist: Before Committing Documentation

### For CHANGELOG Entries

- [ ] Feature/derive claimed actually exists in code (verified with `rg`)
- [ ] Method signatures match what's documented
- [ ] Error types mentioned are the actual types returned
- [ ] Examples in CHANGELOG would compile

### For Rustdoc Comments

- [ ] `# Errors` section matches actual `return Err(...)` variants
- [ ] Link references resolve (tested with `cargo doc --no-deps`)
- [ ] Doc examples compile (`cargo test --doc`)
- [ ] Consistent pattern used across related functions

### For Both

- [ ] Run `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- [ ] Search for inconsistencies: `rg 'FortressError::InvalidRequest[^S]' --type rust`

---

## Detecting Inconsistencies

### Find Undocumented Error Returns

```bash
# Find functions returning FortressError without # Errors section
rg 'pub fn.*-> Result<.*FortressError>' -l | \
  xargs -I{} sh -c 'rg "# Errors" {} || echo "Missing # Errors in: {}"'
```

### Find Mismatched Error Documentation

```bash
# List all documented error variants
rg '\[`?FortressError::\w+' --type rust -o | sort | uniq

# List all actually used error variants
rg 'FortressError::\w+' src/ --type rust -o | sort | uniq

# Compare the lists for mismatches
```

### Find Inconsistent Link Styles

```bash
# Find inline error links (may want to convert to reference style)
rg '\[\`FortressError::\w+\`\]\(crate::' --type rust

# Find orphaned link references (defined but not used)
rg '^\s*/// \[`\w+`\]:' --type rust
```

---

## Recovery: Fixing Inconsistencies

When you find documentation-code mismatches:

### 1. Determine Ground Truth

The **code is always the source of truth**. Documentation must match code, not vice versa.

```rust
// If code returns this:
return Err(FortressError::InvalidRequestStructured { ... });

// Then docs must say InvalidRequestStructured, not InvalidRequest
```

### 2. Update Documentation to Match Code

```rust
// ❌ Before: Doc mentions wrong variant
/// Returns [`FortressError::InvalidRequest`] if...

// ✅ After: Doc matches actual code
/// Returns [`FortressError::InvalidRequestStructured`] if...
```

### 3. Consider Generic Documentation

If the variant might change again, switch to generic form:

```rust
// ✅ Stable documentation that won't need updates
/// Returns a [`FortressError`] if the request is invalid.
```

---

## Integration with CI

### Pre-commit Hook Additions

The pre-commit hook should already run:

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

This catches:

- Broken intra-doc links
- References to non-existent items
- Invalid Rust syntax in doc examples

### Additional CI Checks (Optional)

```yaml
# In .github/workflows/ci-docs.yml
- name: Check doc examples
  run: cargo test --doc

- name: Strict doc build
  run: cargo doc --no-deps
  env:
    RUSTDOCFLAGS: -D warnings
```

---

## Best Practices: Deprecation Version Accuracy

### The Problem

Deprecation attributes and documentation must reference ACTUAL published versions, not:

- Future versions that don't exist yet
- Inherited versions from forked projects
- Placeholder versions

```rust
// ❌ WRONG: Version 0.12.0 doesn't exist (crate is at 0.3.0)
#[deprecated(since = "0.12.0", note = "Use new_method() instead")]
pub fn old_method() { }

// ❌ WRONG: Doc claims deprecation in unreleased version
/// **Deprecated since 0.4.0**: Use `new_method()` instead.
```

### Why This Matters

- Users see deprecation warnings with version numbers
- Invalid versions confuse users ("Is my crate out of date?")
- Creates false changelog expectations

### Verification Before Using `#[deprecated]`

```bash
# Check current crate version
rg '^version = ' Cargo.toml

# Find all deprecation versions in codebase
rg 'since = "\d+\.\d+\.\d+"' --type rust

# Verify versions are valid (exist in CHANGELOG or are current)
rg '## \[\d+\.\d+\.\d+\]' CHANGELOG.md
```

### Choosing the Correct Version

| Situation | Version to Use |
|-----------|----------------|
| Deprecating in unreleased code | Use `[Unreleased]` or omit `since` |
| Deprecating in a published version | Use that version (e.g., `0.3.0`) |
| Inherited from fork | Update to Fortress version when deprecated |

```rust
// ✅ CORRECT: Use version when deprecation was added
#[deprecated(
    since = "0.2.0",  // When Fortress introduced the replacement
    note = "Use `with_save_mode(SaveMode::Sparse)` instead"
)]
pub fn with_sparse_saving_mode(self, sparse_saving: bool) -> Self { }
```

### Doc Comment Deprecation (When `#[deprecated]` Can't Be Used)

For trait impls where `#[deprecated]` isn't supported, use "Soft-deprecated"
instead of "Deprecated since X.Y.Z" to avoid implying a compiler warning exists:

```rust
/// Converts a `usize` to a `Frame`.
///
/// # ⚠️ Discouraged
///
/// **Soft-deprecated**: This conversion silently truncates values.
/// Use [`Frame::from_usize()`] or [`Frame::try_from_usize()`] instead.
///
/// This impl cannot use `#[deprecated]` because Rust doesn't support that
/// attribute on trait impl blocks — no compiler warning will be emitted.
impl From<usize> for Frame {
    fn from(value: usize) -> Self {
        Self(value as i32)
    }
}
```

---

## Best Practices: Code Fence Language in Documentation

### The Problem

Using ` ```rust ` for illustrative pseudo-code that won't compile causes confusion:

```markdown
# ❌ WRONG: Claims to be Rust but won't compile
` ` `rust
fn handle_inputs(inputs: Vec<(MyInput, InputStatus)>) { ... }
` ` `
```

Users might try to copy-paste this and get compiler errors.

### When to Use Each Fence Type

| Content Type | Fence Language | Example |
|--------------|----------------|---------|
| Compilable Rust code | ` ```rust ` | Full examples with all imports |
| Illustrative pseudo-code | ` ```text ` | Migration patterns with `...`, `MyType` |
| Shell commands | ` ```bash ` | Terminal commands |
| Configuration | ` ```toml ` | Cargo.toml snippets |
| Conceptual structure | ` ```text ` | Type outlines, patterns |

### Detecting Non-Compilable Rust Fences

```bash
# Find rust fences with pseudo-code markers
rg -A 5 '```rust' CHANGELOG.md | rg '\.\.\.|MyType|MyInput|/\* \.\.\. \*/'

# Find rust fences in markdown files
rg '```rust' --type md -l

# Then manually verify each is actually compilable
```

### Migration Guide Examples

For migration documentation showing before/after patterns:

```markdown
# ✅ CORRECT: Use text fence for illustrative code
### Address Trait Bounds

` ` `text
// Before
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct MyAddress { /* ... */ }

// After
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct MyAddress { /* ... */ }
` ` `
```

### README and Doc Examples

For README or rustdoc examples that SHOULD compile, use proper Rust:

```rust
// ✅ CORRECT: Full compilable example
use fortress_rollback::{SessionBuilder, P2PSession, FortressError};

fn main() -> Result<(), FortressError> {
    let session = SessionBuilder::new()
        .with_num_players(2)?
        .with_input_delay(2)?
        .build()?;
    Ok(())
}
```

---

## Best Practices: Doc Comment Grammar

### The Problem

Doc comments are English sentences that describe code. Common grammar mistakes make documentation unclear or unprofessional:

```rust
// ❌ WRONG: Missing verb — "information" is a noun, not a verb
/// Returns a [`NetworkStats`] struct that information about...

// ❌ WRONG: Missing auxiliary verb
/// Returns a [`NetworkStats`] struct that give information...

// ❌ WRONG: Wrong verb form (plural with singular subject)
/// Returns a [`NetworkStats`] struct that provide details...
```

### The Pattern: Third-Person Singular

Rust doc comments use **third-person singular** because the implied subject is "this function/method":

```rust
// ✅ CORRECT: Third-person singular throughout
/// Returns a [`NetworkStats`] struct that gives information about...
/// Registers local input for a player for the current frame.
/// Creates a new session with the specified configuration.
/// Validates the input and returns an error if invalid.
```

**Rule:** The main verb and any verbs in subordinate clauses must agree with their subjects.

### Common Grammar Mistakes

| Mistake | Incorrect | Correct |
|---------|-----------|---------|
| Missing verb | "struct that information" | "struct that **gives** information" |
| Missing auxiliary | "struct that give" | "struct that **gives**" |
| Plural with singular | "struct that provide" | "struct that **provides**" |
| Wrong tense | "Returns a struct that gave" | "Returns a struct that **gives**" |
| Dangling modifier | "Returning a struct, the caller..." | "Returns a struct that the caller..." |

### Correct Patterns for Common Doc Comments

```rust
// Function descriptions (implied subject: "this function")
/// Creates a new session builder.
/// Validates the configuration parameters.
/// Computes the checksum of the game state.

// Returns clauses
/// Returns a [`NetworkStats`] struct that contains connection quality metrics.
/// Returns `true` if the session is synchronized.
/// Returns an error if the player handle is invalid.

// Complex descriptions with relative clauses
/// Returns a [`SavedStates`] buffer that stores game states for rollback.
/// Creates a [`SessionBuilder`] that allows configuring all session parameters.
/// Registers input that will be sent to remote players.
```

### Detection: Reading Aloud

Read your doc comments aloud. If they sound wrong, they probably are:

```rust
// Read aloud: "Returns a struct that information about..."
// Sounds wrong → Missing verb

// Read aloud: "Returns a struct that gives information about..."
// Sounds correct → ✅
```

### Verification Commands

```bash
# Find potential grammar issues in doc comments (heuristic)
# Look for "that" followed by a noun without a verb
rg '/// .* that (information|data|details|metrics|stats)' --type rust

# Find doc comments to review manually
rg '/// Returns a \[' --type rust | head -20
```

> **Note:** This heuristic catches common patterns but won't find all grammar issues
> (e.g., "not referring" instead of "does not refer"). Manual review is still valuable.

---

## Best Practices: Parallel Documentation Synchronization

### The Problem

Projects often maintain documentation in multiple locations that can drift out of sync:

```text
docs/user-guide.md      ← MkDocs site documentation
wiki/User-Guide.md      ← GitHub Wiki documentation
README.md               ← Repository quick start
```

When content exists in multiple places, changes to one location are easily forgotten in others, leading to:

- Contradictory information ("docs/ says X, wiki/ says Y")
- Stale examples in one location
- Users getting different answers depending on where they look

### Parallel Files Checklist

Identify and track files that contain overlapping content:

| Primary Source | Parallel Location(s) | Content Type |
|----------------|---------------------|--------------|
| `docs/user-guide.md` | `wiki/User-Guide.md` | Usage examples, API patterns |
| `docs/migration.md` | `wiki/Migration.md` | Migration guidance |
| `docs/architecture.md` | `wiki/Architecture.md` | System design |
| `README.md` | `docs/index.md` | Quick start, overview |
| `CHANGELOG.md` | `docs/changelog.md` | Release history |

### Verification Commands

**Check for documentation drift between parallel files:**

```bash
# Compare docs/ and wiki/ versions (shows differences)
diff -u docs/user-guide.md wiki/User-Guide.md | head -50

# Find content that might be out of sync
# Look for version numbers, method names, or key phrases
rg 'SessionBuilder|with_input_delay|PlayerHandle' docs/ wiki/ --type md

# Check if parallel files have different modification times
ls -la docs/user-guide.md wiki/User-Guide.md

# Find potential inconsistencies in trait bound claims
rg 'trait bounds|no.*(trait|bound).*required|requires.*Clone' docs/ wiki/ --type md -i
```

### Warning Signs of Documentation Drift

Watch for these indicators:

1. **Different timestamps** — Parallel files modified on different dates
2. **Version mismatches** — One file references newer API than the other
3. **Example differences** — Code examples use different patterns
4. **Contradictory claims** — "No trait bounds required" vs code requiring `Clone`

### Prevention Strategies

**Option 1: Single Source of Truth**

Eliminate parallel files by having one authoritative location:

```yaml
# mkdocs.yml - Include wiki as a symlink or git submodule
nav:
  - User Guide: user-guide.md  # Single source
```

**Option 2: Automated Sync Check**

Add a CI check that compares parallel files:

```bash
#!/bin/bash
# scripts/check-doc-sync.sh
if ! diff -q docs/user-guide.md wiki/User-Guide.md > /dev/null 2>&1; then
    echo "WARNING: docs/user-guide.md and wiki/User-Guide.md have drifted"
    echo "Run: diff docs/user-guide.md wiki/User-Guide.md"
    exit 1
fi
```

**Option 3: Include Markers**

Add sync markers to help track parallel content:

```markdown
<!-- SYNC: This content is duplicated in wiki/User-Guide.md -->
<!-- Last synced: 2025-01-15 -->
```

### When Editing Parallel Documentation

**Always ask:** Does this content exist elsewhere?

```bash
# Before editing docs/user-guide.md, check for parallel content
rg -l 'user.guide|User.Guide' --type md

# Search for the specific content being changed
rg 'PlayerHandle.*Display|format.*Player' docs/ wiki/ README.md --type md
```

### Handling Intentional Differences

Sometimes parallel docs SHOULD differ (e.g., wiki has more detail). Document this:

```markdown
<!-- NOTE: This wiki version includes extended examples not in docs/user-guide.md -->
<!-- The docs/ version is the authoritative quick reference -->
```

### Checklist for Parallel Documentation

Before committing documentation changes:

- [ ] Identified all parallel files containing similar content
- [ ] Updated ALL parallel locations with the same changes
- [ ] Verified no contradictions between locations
- [ ] Checked that examples compile in all locations
- [ ] Added sync markers if maintaining intentional differences

---

## Summary: The Seven Rules

1. **Verify before documenting** — Always check code exists before claiming it does
2. **Build docs with warnings as errors** — `RUSTDOCFLAGS="-D warnings" cargo doc`
3. **Prefer stable patterns** — Use `[`FortressError`]` over specific variants when stability matters
4. **Use valid deprecation versions** — Only reference published versions, not future or inherited ones
5. **Use correct code fence language** — `text` for pseudo-code, `rust` only for compilable examples
6. **Use correct grammar** — Match verb forms to subjects; read aloud to verify
7. **Sync parallel documentation** — Update all locations when content exists in multiple places (docs/, wiki/, README)

---

## References

- [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
- [Rustdoc Book - Linking to Items](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html)
- [changelog-practices.md](changelog-practices.md) — When and what to document in CHANGELOG
- [public-api-design.md](public-api-design.md) — API design including documentation patterns
