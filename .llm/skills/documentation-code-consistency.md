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

## Summary: The Three Rules

1. **Verify before documenting** — Always check code exists before claiming it does
2. **Build docs with warnings as errors** — `RUSTDOCFLAGS="-D warnings" cargo doc`
3. **Prefer stable patterns** — Use `[`FortressError`]` over specific variants when stability matters

---

## References

- [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
- [Rustdoc Book - Linking to Items](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html)
- [changelog-practices.md](changelog-practices.md) — When and what to document in CHANGELOG
- [public-api-design.md](public-api-design.md) — API design including documentation patterns
