# Documentation-Code Consistency — Keeping Docs and Code in Sync

> **Documentation that lies is worse than no documentation.** Always verify that documentation accurately reflects the code.

## TL;DR — Key Verification Steps

**Before committing documentation or CHANGELOG entries:**

1. **Verify derives exist:** `rg '#\[derive.*Hash' --type rust` before claiming Hash in CHANGELOG
2. **Verify error variants:** Check actual `return Err(...)` statements match `# Errors` docs
3. **Verify struct field names:** `rg 'pub struct TypeName' --type rust -A 10` before writing examples
4. **Check parallel files:** `diff docs/file.md wiki/File.md` to detect drift
5. **Run doc build:** `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
6. **Test doc examples:** `cargo test --doc`

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

### 4. Struct Field Name Mismatch in Examples

**The Issue:** Doc examples use field names that don't exist on the actual struct.

```rust
// ❌ Doc example uses wrong field name:
/// ```
/// for (input, status) in inputs {
///     match status {
///         InputStatus::Confirmed => game.apply(input),
///         InputStatus::Predicted => game.apply_predicted(input),
///     }
/// }
/// ```

// ❌ But the actual struct has different fields:
pub struct PlayerInput<I> {
    pub input: I,           // Field is 'input', not tuple index
    pub status: InputStatus, // Field is 'status', not tuple index
}
```

**Why It Happens:**

- Struct definition evolved but examples weren't updated
- Examples written from memory without checking actual definitions
- Different iteration patterns assumed (tuple vs named fields)

### 5. Inconsistent Iteration Patterns

**The Issue:** Different doc examples use different patterns for the same data structure.

```rust
// Example A: Tuple destructuring (wrong if struct has named fields)
for (input, status) in inputs { ... }

// Example B: Field access (correct for named fields)
for player_input in inputs {
    let input = player_input.input;
    let status = player_input.status;
}

// Example C: Pattern matching (correct for named fields)
for PlayerInput { input, status } in inputs { ... }
```

**Why This Matters:**

- Users copy-paste examples; wrong patterns won't compile
- Inconsistent patterns confuse users about the correct approach
- Makes documentation look unmaintained

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

### Before Writing Struct Examples

Verify struct field names match actual definitions:

```bash
# Check actual struct definition before writing examples
rg 'pub struct PlayerInput' --type rust -A 10

# Find all field names for a struct
rg 'pub struct FrameInputs' --type rust -A 20 | rg 'pub \w+:'

# Verify iteration patterns in existing examples match struct
rg 'for.*in.*inputs' docs/ wiki/ --type md -B 2 -A 5
```

**Example verification workflow:**

```bash
# Before writing: for (input, status) in inputs
# Check if inputs is Vec<(I, InputStatus)> or Vec<PlayerInput<I>>

rg 'type.*Inputs|struct.*Inputs|fn.*inputs.*->' --type rust | head -10

# If struct has named fields, use:
#   for player_input in inputs { ... player_input.input ... }
# or:
#   for PlayerInput { input, status } in inputs { ... }
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
| `docs/specs/spec-divergences.md` | `wiki/Spec-Divergences.md` | Spec-production alignment (versioned) |
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

### Versioned Documents (spec-divergences, etc.)

Some parallel documents include version headers that must stay synchronized:

```markdown
**Version:** 1.4
**Date:** February 5, 2026
**Status:** Documented
```

**Common version drift patterns:**

```text
docs/specs/spec-divergences.md: Version 1.3, January 15, 2026
wiki/Spec-Divergences.md:       Version 1.4, February 5, 2026
```

**Verification for versioned documents:**

```bash
# Check version headers in parallel files
rg '\*\*Version:\*\*|\*\*Date:\*\*' docs/ wiki/ --type md

# Compare revision histories
rg -A 5 'Revision History' docs/specs/ wiki/ --type md

# Find version mismatches
diff <(rg 'Version:' docs/specs/spec-divergences.md) \
     <(rg 'Version:' wiki/Spec-Divergences.md)
```

**When syncing versioned documents:**

1. **Identify the more recent version** — Check revision history
2. **Copy content from newer to older** — Preserve any location-specific paths (e.g., image refs)
3. **Bump version number** — Increment to indicate sync occurred
4. **Update date** — Use the current date since you're modifying the document
5. **Add revision entry** — Document the sync in revision history

**Example revision history entry for sync:**

```markdown
| 1.4 | 2026-02-05 | Synchronized docs/ and wiki/ versions; no content changes |
```

---

## Best Practices: API Renaming and Signature Changes

### The Problem

When methods are renamed or return types change, documentation drift occurs across multiple files:

- **Method reference tables** show old return types (e.g., `Vec<PlayerHandle>` → `HandleVec`)
- **Code examples** call old method names (e.g., `is_spectator()` → `is_spectator_handle()`)
- **Stub implementations** in doc examples use old signatures
- **Wiki files** have stale information separate from main docs

This creates confusion when users find contradictory information depending on where they look.

### API Renaming Checklist

**BEFORE renaming a method or changing a return type:**

```bash
# 1. Find ALL references to the method/type being changed
rg 'old_method_name|OldReturnType' --type rust --type md

# 2. Specifically check documentation files
rg 'old_method_name' docs/ wiki/ README.md examples/

# 3. Find method reference tables (usually in | format)
rg '\| `?old_method_name' --type md

# 4. Find stub implementations in doc examples
rg '#\s*fn old_method_name|#\s*fn.*OldReturnType' --type md
```

**AFTER making the code change:**

- [ ] Update all `docs/*.md` files
- [ ] Update all `wiki/*.md` files (if they exist)
- [ ] Update `README.md` examples
- [ ] Update `examples/*.rs` files
- [ ] Update `CHANGELOG.md` with migration guidance
- [ ] Run `cargo test --doc` to verify doc examples compile
- [ ] Run `cargo build --examples` to verify examples compile

### Method Reference Tables

Documentation often contains reference tables like:

```markdown
| Method | Return Type | Description |
|--------|-------------|-------------|
| `local_player_handles()` | `HandleVec` | All local players |
| `is_spectator_handle(h)` | `bool` | Check if spectator |
```

**Finding reference tables that need updates:**

```bash
# Find tables containing the old method/type name
rg '\|.*old_method_name.*\|' --type md

# Find tables with old return types
rg '\|.*`Vec<PlayerHandle>`.*\|' --type md
rg '\|.*`Vec<.*>`.*\|' --type md  # Broader search for Vec return types

# List all reference tables in docs (to manually verify)
rg '^\|.*\|.*\|$' docs/ wiki/ --type md | grep -v '^\|---'
```

**Verification after updating tables:**

```bash
# Verify table return types match actual code
rg 'pub fn local_player_handles' --type rust -A 1  # Check actual signature
rg '\| `local_player_handles' --type md            # Check documented signature
```

### Stub Implementations in Doc Examples

Doc examples often contain hidden stub code to make examples compile:

```rust
/// # Example
/// ```
/// # use fortress_rollback::{PlayerHandle, HandleVec};
/// # struct FakeSession;
/// # impl FakeSession {
/// #     fn local_player_handles(&self) -> HandleVec { HandleVec::new() }
/// # }
/// let session = FakeSession;
/// for handle in session.local_player_handles() {
///     // ...
/// }
/// ```
```

**Finding stubs that need updates:**

```bash
# Find stub method definitions (lines starting with # fn)
rg '#\s*fn\s+method_name' --type rust --type md

# Find stub return types
rg '#.*-> Vec<PlayerHandle>' --type rust --type md
rg '#.*-> HandleVec' --type rust --type md

# Find all stub implementations in a specific file
rg '^#\s*(fn|impl|struct|use)' docs/user-guide.md
```

**Critical:** Stub signatures must match the real API exactly, or doc tests will pass but teach users incorrect patterns.

### Complete API Change Workflow

When changing `fn old_name() -> OldType` to `fn new_name() -> NewType`:

```bash
# Step 1: Audit all references BEFORE changing code
echo "=== Finding all references ==="
rg 'old_name|OldType' --type rust --type md -l

# Step 2: Make the code change
# ... edit source files ...

# Step 3: Update documentation systematically
echo "=== Updating docs/ ==="
rg -l 'old_name|OldType' docs/ | xargs -I{} echo "Update: {}"

echo "=== Updating wiki/ ==="
rg -l 'old_name|OldType' wiki/ | xargs -I{} echo "Update: {}"

echo "=== Updating examples/ ==="
rg -l 'old_name|OldType' examples/ | xargs -I{} echo "Update: {}"

# Step 4: Verify no stale references remain
echo "=== Verification ==="
rg 'old_name|OldType' --type rust --type md
# Should return no results (or only CHANGELOG migration notes)

# Step 5: Verify everything compiles
cargo test --doc
cargo build --examples
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

### Example: Renaming `is_spectator` to `is_spectator_handle`

```bash
# Find all affected files
rg 'is_spectator\(' --type md -l
# Results:
#   docs/user-guide.md
#   wiki/User-Guide.md
#   wiki/API-Contracts.md

# Check reference tables
rg '\| `is_spectator\(' --type md
# Update any matches to is_spectator_handle

# Check code examples
rg 'session\.is_spectator\(' --type md
# Update to session.is_spectator_handle()

# Check stub implementations
rg '#.*fn is_spectator\(' --type md
# Update stub signatures

# Verify no stale references (except CHANGELOG migration notes)
rg 'is_spectator\(' --type md | grep -v CHANGELOG | grep -v 'renamed to'
```

### Common Pitfalls

1. **Forgetting wiki/** — Wiki files are often maintained separately and easy to miss
2. **Partial table updates** — Updating the method name but not the return type in a reference table
3. **Stale stubs** — Updating visible example code but not the hidden `# impl` stubs
4. **Missing CHANGELOG** — Not documenting the breaking change with migration guidance
5. **README drift** — README quick-start examples using old API patterns

### Prevention: Documentation Change Hooks

Consider adding a pre-commit check for common documentation drift patterns:

```bash
#!/bin/bash
# scripts/check-doc-api-sync.sh

# Check for common return type mismatches
if rg 'Vec<PlayerHandle>' docs/ wiki/ --type md -q; then
    echo "WARNING: Found Vec<PlayerHandle> in docs - should this be HandleVec?"
    rg 'Vec<PlayerHandle>' docs/ wiki/ --type md
fi

# Check for old method names (add patterns as needed)
OLD_PATTERNS='is_spectator\(|\.players\(\)'
if rg "$OLD_PATTERNS" docs/ wiki/ --type md -q; then
    echo "WARNING: Found potentially outdated method references"
    rg "$OLD_PATTERNS" docs/ wiki/ --type md
fi
```

---

## Best Practices: Version Numbers in Documentation Examples

### The Problem

Dependency version snippets in documentation (e.g., `Cargo.toml` examples) drift out of sync
with the actual crate version. Users copy-paste stale version numbers and get unexpected behavior
or resolution failures.

```toml
# ❌ WRONG: docs/ says 0.2.0 but Cargo.toml is at 0.3.0
[dependencies]
fortress-rollback = "0.2.0"
```

### Why This Matters

- Users follow quick-start guides verbatim — stale versions cause immediate friction
- Different docs locations (docs/, wiki/, README) may show different versions
- Version mismatches erode trust in documentation accuracy

### Verification Commands

```bash
# Check the actual crate version
rg '^version = ' Cargo.toml

# Find all version references in documentation
rg 'fortress-rollback.*version' docs/ wiki/ --type md

# Find Cargo.toml dependency snippets in markdown
rg 'fortress-rollback.*=.*"\d+\.\d+' docs/ wiki/ README.md --type md
```

### When to Update

**Every time the crate version bumps**, update ALL documentation snippets:

```bash
# After bumping version in Cargo.toml, find all stale references
OLD_VERSION='0\.2\.0'
rg "$OLD_VERSION" docs/ wiki/ README.md examples/ --type md --type toml -l
```

### Common Anti-Patterns

| Anti-Pattern | Why It Happens | Prevention |
|--------------|----------------|------------|
| Updating docs/ but forgetting wiki/ | Wiki is maintained separately | Search both: `rg 'fortress-rollback' docs/ wiki/` |
| Updating README but forgetting user-guide | Multiple quick-start locations | Search all md files at once |
| Hardcoded version in CI examples | Copy-pasted from old docs | Use `rg` after every version bump |

### Automated Check

Add to your version-bump workflow:

```bash
#!/bin/bash
# scripts/check-doc-versions.sh
CRATE_VERSION=$(rg '^version = "(.+)"' Cargo.toml -o -r '$1')
echo "Crate version: $CRATE_VERSION"

# Find any version references that DON'T match current
rg 'fortress-rollback.*=.*"\d+\.\d+' docs/ wiki/ README.md --type md \
  | grep -v "$CRATE_VERSION" \
  && echo "ERROR: Stale version references found" && exit 1 \
  || echo "OK: All version references match $CRATE_VERSION"
```

---

## Best Practices: Zero-Panic Language in Doc Comments

### The Problem

Doc comments that claim the library "will panic" or "will cause panics" directly contradict
the project's zero-panic policy. This confuses users and undermines trust in the safety guarantees.

```rust
// ❌ WRONG: Claims the library panics — violates zero-panic policy
/// If the input queue is full, this will panic.

// ❌ WRONG: Suggests panics are an expected outcome
/// Using an invalid player handle may cause a panic.
```

### Why This Matters

- Fortress Rollback's core promise is **zero panics in production code**
- Doc comments saying "will panic" tell users the library is unsafe
- Users may add unnecessary defensive code (or worse, avoid the API)
- Contradicts `# Panics` being absent from documentation (by design)

### Correct Alternatives

Describe the **actual** consequences — errors, incorrect state, or desync:

```rust
// ✅ CORRECT: Describes actual error behavior
/// If the input queue is full, returns
/// [`FortressError::InvalidRequestStructured`].

// ✅ CORRECT: Describes consequence without claiming panic
/// Using an invalid player handle returns an error. If the error is
/// not handled, the session may enter an inconsistent state.

// ✅ CORRECT: Describes desync risk
/// Failing to process all [`FortressRequest`] items may lead to
/// state desync between peers.
```

### Verification Commands

```bash
# Find panic language in doc comments
rg 'will.*panic|cause.*panic|may.*panic|could.*panic' src/ --type rust

# Also check for "# Panics" sections (should not exist in production code)
rg '# Panics' src/ --type rust

# Check documentation files too
rg 'will.*panic|cause.*panic' docs/ wiki/ --type md
```

### Exceptions

Panic language is acceptable in these limited contexts:

| Context | Why It's OK |
|---------|-------------|
| `#[cfg(test)]` code and test comments | Tests may use `unwrap()` and assert macros |
| `debug_assert!` documentation | Debug-only checks that are stripped in release |
| Migration guides describing old behavior | "Previously this would panic; now it returns `Result`" |
| Documenting what happens if `debug_assertions` are enabled | Explicitly scoped to debug builds |

### Anti-Pattern: `# Panics` Section

Production code should **never** have a `# Panics` rustdoc section. If you find yourself
writing one, the function needs to be refactored to return `Result` instead:

```rust
// ❌ WRONG: Function that documents its own panics
/// # Panics
///
/// Panics if `index` is out of bounds.
pub fn get_player(&self, index: usize) -> &Player {
    &self.players[index]  // Forbidden: direct indexing
}

// ✅ CORRECT: Function returns Result, no panic possible
/// # Errors
///
/// Returns [`FortressError::InvalidPlayerIndex`] if `index`
/// is out of bounds.
pub fn get_player(&self, index: usize) -> Result<&Player, FortressError> {
    self.players.get(index).ok_or(FortressError::InvalidPlayerIndex {
        index,
        count: self.players.len(),
    })
}
```

---

## Best Practices: Self-Contained Code Examples

### The Problem

Code examples in documentation use identifiers (`socket`, `remote_addr`, `MyConfig`) that
are neither imported nor defined within the snippet. Users cannot run these examples without
guessing what the undefined references should be.

````rust
// ❌ WRONG: Where do `socket`, `remote_addr`, and `MyConfig` come from?
let session = SessionBuilder::<MyConfig>::new()
    .with_num_players(2)?
    .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
    .start_p2p_session(socket)?;
````

### Why This Matters

- Users copy-paste examples and get immediate compile errors
- Undefined identifiers force users to search for types/constructors
- Incomplete examples suggest the library is poorly documented
- New users have no way to know what `MyConfig` should implement

### Two Levels of Completeness

#### Level 1: Conceptual Snippets (Illustrative)

For snippets showing patterns or API shapes, add comments explaining undefined references:

````rust
// ✅ OK: Comments explain what users must provide
// `MyConfig` must implement `fortress_rollback::Config`
// `socket` is any type implementing `NonBlockingSocket<SocketAddr>`
// `remote_addr` is the `SocketAddr` of the remote peer
let session = SessionBuilder::<MyConfig>::new()
    .with_num_players(2)?
    .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
    .start_p2p_session(socket)?;
````

#### Level 2: Complete Examples (Runnable)

For examples that should compile, include ALL imports and variable definitions:

````rust
// ✅ CORRECT: Fully self-contained
use fortress_rollback::{
    SessionBuilder, PlayerType, PlayerHandle, FortressError,
    NonBlockingSocket,
};
use std::net::SocketAddr;

# struct MyConfig;
# impl fortress_rollback::Config for MyConfig {
#     type Input = Vec<u8>;
#     type State = Vec<u8>;
#     type Address = SocketAddr;
# }
# let remote_addr: SocketAddr = "127.0.0.1:7000".parse().unwrap();
# let socket = /* ... */;

let session = SessionBuilder::<MyConfig>::new()
    .with_num_players(2)?
    .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
    .start_p2p_session(socket)?;
# Ok::<(), FortressError>(())
````

### Common Missing Identifiers

| Identifier | What Users Need |
|------------|-----------------|
| `MyConfig` | A struct implementing `Config` trait |
| `socket` | A type implementing `NonBlockingSocket` |
| `remote_addr` | A `SocketAddr` (or the session's address type) |
| `game_state` | The user's game state struct |
| `FortressError` | Must be imported for `.ok_or()` and `?` usage |
| `input` | The user's input type matching `Config::Input` |

### Verification Commands

```bash
# Find examples that reference common undefined identifiers
rg 'socket|remote_addr|MyConfig|game_state' docs/ wiki/ --type md -C 3

# Check that FortressError is imported when used in .ok_or()
rg '\.ok_or\(FortressError' docs/ wiki/ --type md -B 10 | rg 'use.*FortressError'

# Find code fences and check for imports
rg -A 20 '```rust' docs/ wiki/ --type md | rg 'use fortress_rollback'
```

### Rule of Thumb

**If a user cannot determine the type or value of every identifier in the snippet
by reading only the snippet itself, the example is incomplete.** Either:

1. Add comments explaining undefined references (conceptual snippets)
2. Add imports and definitions to make it self-contained (complete examples)
3. Use `text` fence instead of `rust` if it's purely illustrative

---

## Summary: The Thirteen Rules

1. **Verify before documenting** — Always check code exists before claiming it does
2. **Build docs with warnings as errors** — `RUSTDOCFLAGS="-D warnings" cargo doc`
3. **Prefer stable patterns** — Use `[`FortressError`]` over specific variants when stability matters
4. **Use valid deprecation versions** — Only reference published versions, not future or inherited ones
5. **Use correct code fence language** — `text` for pseudo-code, `rust` only for compilable examples
6. **Use correct grammar** — Match verb forms to subjects; read aloud to verify
7. **Sync parallel documentation** — Update all locations when content exists in multiple places (docs/, wiki/, README)
8. **Follow API change workflow** — When renaming methods or changing types, audit ALL references before and after
9. **Verify struct field names** — Check actual struct definitions before writing examples with field access
10. **Use consistent iteration patterns** — Match iteration style to actual data structure (tuple vs named fields)
11. **Keep version numbers in sync** — Dependency snippets in docs/wiki MUST match `Cargo.toml` version; check ALL locations when bumping
12. **No panic language in doc comments** — Never claim the library "will panic"; describe actual consequences (errors, desync, incorrect state)
13. **Make examples self-contained** — All identifiers in code examples must be imported, defined, or explained with comments

---

## References

- [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
- [Rustdoc Book - Linking to Items](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html)
- [changelog-practices.md](changelog-practices.md) — When and what to document in CHANGELOG
- [public-api-design.md](public-api-design.md) — API design including documentation patterns
