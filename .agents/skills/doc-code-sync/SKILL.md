---
name: doc-code-sync
description: "Documentation-Code Consistency guidance for Fortress Rollback. Use when CHANGELOG verification, error variant matching, struct field checks, doc-code consistency."
---

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

## Intra-Doc Link Text vs Target

A backticked link whose text names an item must point at THAT item, not its
enclosing module. `cargo doc` does NOT catch this -- the link resolves but lands
on the wrong page. `check-links.py` flags it: it compares the final `::` segment
of the link text against the final segment of any `crate::`/`super::`/`self::`
target.

```rust
// WRONG: text names the constant, target is the module -> wrong page
/// [`MAX_BOUNDED_DECODE_LEN`](crate::network::codec)

// RIGHT (pick based on item visibility):
/// [`MAX_BOUNDED_DECODE_LEN`](crate::network::codec::MAX_BOUNDED_DECODE_LEN)  // if reachable
/// `MAX_BOUNDED_DECODE_LEN`                                                   // de-link if pub(crate) in a pub module
/// the [`codec`](crate::network::codec) module                               // or name the module
```

The fix is NOT always "append the item". A `pub(crate)` item inside a `pub`
module cannot be linked directly -- that trips `rustdoc::private_intra_doc_links`
(CI denies it). De-link to a plain code span or link the module instead. A direct
item link is fine when the item is `pub`, or when both item and module are
crate-internal. External-crate targets (e.g. `bincode::...`) and `Self::`/bare
relative paths are exempt.

## Intra-Doc Links to `#[cfg(kani)]`-Only Items

Never write a rustdoc intra-doc link to an item defined only under `#[cfg(kani)]`
(e.g. `InlineVec`, `KANI_INLINE_CAP` in `src/proof_vec.rs`):

```rust
// WRONG: resolves only under --cfg kani; a broken intra-doc link everywhere else.
/// The Kani-only [`InlineVec`](crate::proof_vec::InlineVec) backs the queue.

// RIGHT: a plain code span -- no link, nothing to break.
/// The Kani-only `InlineVec` (defined under `#[cfg(kani)]`) backs the queue.
```

`cargo doc` never sets `--cfg kani`, so the item is not compiled and the target
does not exist; `-D rustdoc::broken_intra_doc_links` would fail. Such a link only
*appears* safe when it happens to sit on a `#[cfg(kani)]` item that rustdoc skips
entirely -- a fragile coincidence, not a guarantee. Use a code span and name the
defining module in prose.

`check-links.py` enforces this (`find_cfg_kani_only_items` +
`cfg_kani_only_link_error`), with no Kani toolchain needed: it harvests names
gated by an exact `#[cfg(kani)]` and forbids intra-doc links to any that never
appear unconditionally or under `#[cfg(not(kani))]`. Items defined under *both*
cfgs (e.g. `ProofVec`, `INPUT_QUEUE_LENGTH`) exist in every build and stay
linkable. Tests: `scripts/tests/test_check_links_cfg_kani.py`.

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

## Enumerated-Set Counts in Prose (derive, don't hand-count)

Prose that states a *count* of an enumerable set ("seven `FIX_MODE` configs",
"the four error variants") rots the moment the set grows. Two defenses, in order
of preference:

1. **State the fact once; link, don't restate.** When the same invariant lives in
   N doc comments, fixing one leaves N-1 stale. Make one site canonical and have
   the others defer to it. Example: the double-failure-relay pessimistic floor is
   defined once on `P2PSession::pessimistic_floors`; the wire field
   (`FloorReply::floors`) defers to that definition directly, and the relay-reply
   cache (`UdpProtocol::round_floor`) and the consumer (`remote_slot_confirmed_bound`)
   defer via the reply — none re-describe the `Frame::NULL`-skip fold.
2. **Derive machine-checkable counts from the source of truth.** For an
   enumerable set with a single definition site, add a checker that reads the
   definition and compares. Each TLA `FIX_MODE` set lives in one
   `ASSUME FIX_MODE \in {...}` clause in its own spec (e.g.
   `DoubleFailureRelay.tla`, `SpectatorReactivationEpoch.tla`);
   `scripts/docs/check-tla-config-consistency.py` *discovers* every such spec (a
   new FIX_MODE spec needs no edit), pairs each `.cfg` to its spec by filename,
   and enforces that every `.cfg` names a mode *its own* spec defines, every
   defined mode has a `.cfg` and a README mention, and every prose mode-count
   matches some spec's size. Run it via agent preflight (`tla-config-consistency`)
   or the `ci-docs` workflow. Tests: `scripts/tests/test_check_tla_config_consistency.py`.

The checker only reads a count claim written in the form `<N> FIX_MODE modes`
(FIX_MODE inside the counted phrase) — so author mode counts that way. There is
no proximity guessing: a bare "<N> configs" near a `FIX_MODE=` table cell, and
sub-counts like "the four original configs", are deliberately left alone.

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

A *removed* `pub(crate)`/private identifier still named in docs is the same drift in
reverse (a doc claiming code that no longer exists). `scripts/ci/check-doc-claims.sh`
(`check_removed_floor_identifiers`) enforces that any tracked doc/comment naming a
known-removed identifier carries a same-line historical qualifier
(`legacy`/`formerly`/`removed`/`pre-S55`); when you delete a referenced item, add a
tombstone entry there (and re-point live references at the replacement). Tests:
`scripts/tests/test_check_doc_claims_floor_ids.py`. Runs in agent preflight (`doc-claims`).
