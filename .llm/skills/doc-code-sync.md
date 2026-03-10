<!-- CATEGORY: CI/CD & Tooling -->
<!-- WHEN: CHANGELOG verification, error variant matching, struct field checks, doc-code consistency -->
# Documentation-Code Consistency

## Verification Commands (Run Before Committing)

```bash
rg '#\[derive.*Hash' --type rust        # Verify derives before claiming in CHANGELOG
rg 'return Err\(' src/file.rs -A 2      # Check actual error variants
rg 'pub struct TypeName' --type rust -A 10  # Verify struct fields before writing examples
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps  # Strict doc build
cargo test --doc                         # Test doc examples compile
```

## Problem Categories

### 1. CHANGELOG-Code Mismatch

CHANGELOG claims feature exists but code does not. Always verify with `rg` before writing entries.

### 2. Error Variant Mismatch

Doc comments reference wrong error variant (e.g., `InvalidRequest` vs `InvalidRequestStructured`).

**Preferred pattern:** Use generic error type for stability:

```rust
/// # Errors
///
/// Returns a [`FortressError`] if the delay exceeds the maximum value.
```

Use specific variants only when the variant is stable API and users need to match on it.

### 3. Struct Field Name Mismatch

Examples use field names that don't exist on actual struct. Always check:

```bash
rg 'pub struct PlayerInput' --type rust -A 10
```

### 4. Deprecation Version Errors

Only reference published versions, never future or inherited versions:

```bash
rg '^version = ' Cargo.toml             # Check current version
rg 'since = "\d+\.\d+\.\d+"' --type rust  # Find all deprecation versions
```

For trait impls where `#[deprecated]` is unsupported, use "Soft-deprecated" language.

### 5. Zero-Panic Language

Never claim the library "will panic" in doc comments. Describe actual consequences:

```rust
// WRONG
/// If the input queue is full, this will panic.

// CORRECT
/// If the input queue is full, returns [`FortressError::InvalidRequestStructured`].
```

Production code should never have a `# Panics` rustdoc section.

## Error Documentation Patterns

### Generic (Recommended)

```rust
/// # Errors
///
/// Returns a [`FortressError`] if the configuration is invalid.
```

### Specific (When API Is Stable)

```rust
/// # Errors
///
/// Returns [`FortressError::InvalidFrameStructured`] with
/// [`InvalidFrameReason::NegativeValue`] if the frame is negative.
///
/// [`FortressError::InvalidFrameStructured`]: crate::FortressError::InvalidFrameStructured
```

## Code Fence Language

| Content | Fence | When |
|---------|-------|------|
| Compilable Rust | ` ```rust ` | Full examples with imports |
| Pseudo-code/patterns | ` ```text ` | Migration patterns, type outlines |
| Shell commands | ` ```bash ` | Terminal commands |
| Config | ` ```toml ` | Cargo.toml snippets |

## Self-Contained Examples

All identifiers must be imported, defined, or explained:

```rust
// INCOMPLETE: where do socket, remote_addr, MyConfig come from?
let session = SessionBuilder::<MyConfig>::new()
    .start_p2p_session(socket)?;

// COMPLETE: comments explain or stubs define everything
# struct MyConfig;
# impl fortress_rollback::Config for MyConfig { /* ... */ }
# let socket = /* ... */;
let session = SessionBuilder::<MyConfig>::new()
    .start_p2p_session(socket)?;
```

## API Renaming Checklist

When changing `fn old_name() -> OldType` to `fn new_name() -> NewType`:

```bash
# BEFORE: find all references
rg 'old_name|OldType' --type rust --type md -l

# AFTER: update everywhere
rg -l 'old_name|OldType' docs/ wiki/ examples/  # Update each
cargo test --doc && cargo build --examples       # Verify

# Verify no stale references remain (except CHANGELOG migration notes)
rg 'old_name|OldType' --type rust --type md
```

Update: `docs/*.md`, `wiki/*.md`, `README.md`, `examples/*.rs`, `CHANGELOG.md`, stub implementations in doc examples.

## Parallel Documentation Sync

| Primary Source | Parallel Location | Content Type |
|----------------|-------------------|--------------|
| `docs/user-guide.md` | `wiki/User-Guide.md` | Usage examples |
| `README.md` | `docs/index.md` | Quick start |
| `CHANGELOG.md` | `docs/changelog.md` | Release history |

Check for drift:

```bash
rg 'SessionBuilder|with_input_delay' docs/ wiki/ --type md
```

## Doc Comment Grammar

Use third-person singular (implied subject: "this function"):

```rust
/// Returns a [`NetworkStats`] struct that gives information about...
/// Registers local input for a player for the current frame.
```

Common mistakes: missing verb ("struct that information"), wrong form ("struct that provide").

## Version Numbers in Documentation

After bumping `Cargo.toml` version, update ALL doc snippets:

```bash
rg 'fortress-rollback.*=.*"\d+\.\d+' docs/ wiki/ README.md --type md
```

## Detecting Inconsistencies

```bash
# Undocumented error returns
rg 'pub fn.*-> Result<.*FortressError>' -l | \
  xargs -I{} sh -c 'rg "# Errors" {} || echo "Missing # Errors in: {}"'

# Mismatched error documentation
rg '\[`?FortressError::\w+' --type rust -o | sort | uniq   # Documented
rg 'FortressError::\w+' src/ --type rust -o | sort | uniq  # Used

# Panic language in doc comments
rg 'will.*panic|cause.*panic' src/ --type rust
rg '# Panics' src/ --type rust
```
