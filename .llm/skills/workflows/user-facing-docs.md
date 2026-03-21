<!-- CATEGORY: Workflows -->
<!-- WHEN: New public API, breaking changes, new features, configuration changes, behavioral changes requiring user documentation -->

# User-Facing Documentation Workflow

Structured process for creating and updating user-facing documentation alongside library changes that impact clients. Ensures docs stay in sync with code and meet quality standards.

**Related workflows:** [dev-pipeline.md](dev-pipeline.md) (full development lifecycle), [review-readiness.md](review-readiness.md) (pre-PR checks).

**Prerequisites:** `npx`/`markdownlint` for local lint checks. Skip if unavailable; CI will catch lint issues.

---

## Trigger: When to Invoke This Workflow

| Change Type | Triggered? |
|-------------|-----------|
| New `pub` API (function, struct, enum, trait) | Yes |
| Breaking change to existing `pub` API | Yes |
| New user-facing feature | Yes |
| New or changed configuration option | Yes |
| Behavioral change (same API, different result) | Yes |
| Internal refactoring only | No |
| CI/tooling changes | No |
| Test-only changes | No |

---

## Documentation Inventory

Which files to update for each change type:

| Change Type | user-guide.md | CHANGELOG.md | migration.md | README.md | examples/*.rs |
|-------------|:------------:|:------------:|:------------:|:---------:|:-------------:|
| New feature/API | Yes | Yes | -- | If significant | If affected |
| Breaking change | Yes | Yes | Yes | If significant | Yes |
| New configuration | Yes (usage + ref) | Yes | -- | -- | If affected |
| Behavioral change | Yes | Yes | -- | -- | If affected |

When API changes affect public signatures, check `examples/*.rs` for outdated usage.

### File Locations and Ownership

| File | Path | Edit Directly? |
|------|------|---------------|
| User guide | `docs/user-guide.md` | Yes |
| Changelog | `CHANGELOG.md` | Yes |
| Migration guide | `docs/migration.md` | Yes |
| README | `README.md` | Yes |
| MkDocs homepage | `docs/index.md` | Yes |
| Examples | `examples/*.rs` | Yes |
| Wiki files | `wiki/*.md` | **Never** -- auto-generated |

Wiki files are generated from `docs/` by `scripts/docs/sync-wiki.py`. Never edit `wiki/` directly.

---

## Step 1: Update the Documentation Files

### User Guide (`docs/user-guide.md`)

- Add new features to the appropriate section
- For new configuration options, update **both** the usage section and the **Complete Configuration Reference** section
- Update Table of Contents if adding new sections

### Rustdoc (`///` comments)

When changing `pub` items, update inline `///` doc comments on affected structs, functions, and traits. Rustdoc is the primary API reference; keep it accurate.

### Migration Guide (`docs/migration.md`) -- Breaking Changes Only

- Add a section under the target version heading
- Show before/after code for the migration path
- Explain why the change was made

### CHANGELOG (`CHANGELOG.md`)

- Follow the existing format (see [changelog.md](../publishing-organization/changelog.md))
- Categorize under Added / Changed / Fixed / Removed

### README (`README.md`) -- Significant Changes Only

- Update quick start if the primary API surface changed
- Update feature list if a major capability was added

---

## Step 2: Write Quality Documentation

### Code Example Standards

All code examples must follow these rules:

1. **Self-contained with imports** -- include all `use` statements
2. **Compilable** -- test with `cargo test --doc` or use hidden `# ` lines for boilerplate
3. **Zero-panic** -- no `.unwrap()` in production examples; use `?` or proper error handling
4. **Show simple then advanced** -- start with the common case, then show the full-control case

### Example Structure

Use the real `SessionBuilder` API -- builder methods return `Result` where validation occurs, and sessions are started via `start_p2p_session(socket)`, `start_synctest_session()`, or `start_spectator_session(host_addr, socket)`:

```rust
# use fortress_rollback::prelude::*;
# use std::net::SocketAddr;
# fn example(socket: impl NonBlockingSocket<SocketAddr> + 'static) -> Result<(), FortressError> {
// Simple P2P session
let session = SessionBuilder::new()
    .with_num_players(2)?
    .with_fps(60)?
    .start_p2p_session(socket)?;

// SyncTest session (no socket needed)
let session = SessionBuilder::new()
    .with_num_players(2)?
    .with_fps(60)?
    .with_check_distance(7)
    .start_synctest_session()?;
# Ok(())
# }
```

### Writing Guidelines

| Guideline | Example |
|-----------|---------|
| Explain the "why" not just "what" | "Input delay trades latency for smoother gameplay by..." |
| Production vs Testing note | "In production use X; in tests you can simplify with Y" |
| Link to related docs | "See the Configuration Reference section for all options" |
| Warn about common mistakes | "Note: setting fps to 0 returns `InvalidRequestKind::ZeroFps`" |

---

## Step 3: Wiki Sync

After editing any file in `docs/`, run the wiki sync pipeline:

```bash
# Generate wiki files from docs/
python3 scripts/docs/sync-wiki.py

# Validate consistency
python3 scripts/docs/check-wiki-consistency.py
```

Both commands must succeed. If consistency check fails, fix the source in `docs/` and re-run.

---

## Step 4: Verification

Run all checks before considering documentation complete:

```bash
# 1. Doc examples compile
cargo test --doc -- --nocapture

# 2. Rustdoc builds without warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# 3. Markdown lint (all changed docs)
npx markdownlint '**/*.md' --config .markdownlint.json --ignore 'target/**'

# 4. Link validation
./scripts/docs/check-links.sh

# 5. Wiki sync and validation
python3 scripts/docs/sync-wiki.py
python3 scripts/docs/check-wiki-consistency.py
```

---

## Checklist

Complete before marking documentation work done:

- [ ] Table of Contents updated in modified files
- [ ] Code examples are self-contained with all imports
- [ ] No `.unwrap()` or `.expect()` in production examples
- [ ] Both simple and advanced usage shown where appropriate
- [ ] "Why" is explained, not just "what"
- [ ] `///` rustdoc comments updated on changed `pub` items
- [ ] Breaking changes have migration guide entries
- [ ] `examples/*.rs` updated if API signatures changed
- [ ] `cargo test --doc` passes
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` passes
- [ ] Wiki sync run and validated

---

## Anti-Patterns

| Anti-Pattern | Do Instead |
|-------------|------------|
| Edit `wiki/` files directly | Edit `docs/` and run sync script |
| Code examples without imports | Include all `use` statements |
| `.unwrap()` in examples | Use `?` with proper error handling |
| Document only the happy path | Show error cases and edge cases too |
| Skip wiki sync after docs edit | Always run sync + consistency check |
| Update docs without testing examples | Run `cargo test --doc` |
| Use fictional API (e.g. `.start()`) | Check real signatures in `src/sessions/builder.rs` |
