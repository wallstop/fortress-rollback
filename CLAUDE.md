# Claude Instructions for Fortress Rollback

**Read [`.llm/context.md`](.llm/context.md)** — the canonical source of truth for all project context, development policies, testing guidelines, and coding standards.

## Critical: Zero-Panic Policy

**All production code must follow defensive programming practices.** See [`.llm/skills/defensive-programming.md`](.llm/skills/defensive-programming.md) for the complete guide.

Key requirements:

- **Never panic** — No `unwrap()`, `expect()`, `panic!()`, `todo!()`
- **Return `Result`** — All fallible operations must return errors, not panic
- **Never swallow errors** — Propagate or explicitly handle, never ignore
- **Validate everything** — Don't assume inputs or internal state are valid

## Mandatory Pre-Commit Checks

**ALWAYS run before committing ANY changes:**

```bash
# Rust code (always capture output so failures are visible)
cargo fmt && cargo clippy --all-targets && cargo nextest run --no-capture

# Or use the convenient aliases defined in .cargo/config.toml (includes --no-capture)
cargo c && cargo t

# Documentation changes (rustdoc comments)
cargo doc --no-deps

# Markdown files (if modified)
npx markdownlint '**/*.md' --config .markdownlint.json --fix

# GitHub Actions workflows (if modified)
actionlint
```

**Run linters after EVERY change, not just before committing.** This catches errors immediately while context is fresh.

**Always use `--no-capture`** (nextest) or `-- --nocapture` (cargo test) so that test output is visible immediately when failures occur. This avoids having to re-run tests to see what went wrong.

## Changelog Reminder

After code changes, ask: **Does this affect `pub` items or user-observable behavior?**

- If YES → Add entry to `CHANGELOG.md` under `## [Unreleased]` (use `**Breaking:**` prefix for API changes)
- If NO (`pub(crate)`, private, tests, CI) → No changelog needed
- See [changelog-practices.md](.llm/skills/changelog-practices.md) for detailed guidance

## Kani Formal Verification

**When modifying Kani proofs or code verified by Kani proofs, run the affected proof locally:**

```bash
# Run a specific proof harness
cargo kani --harness proof_function_name

# Run Tier 1 (fast) proofs — useful for quick validation
./scripts/verify-kani.sh --tier 1 --quick

# Check that new proofs are registered in tier lists
./scripts/check-kani-coverage.sh
```

**Why this matters:**

- Pre-commit only checks that proofs are **registered**, not that they **pass**
- Kani verification failures won't be caught until CI runs
- Running the specific affected proof takes seconds to minutes, not hours

**CRITICAL — Loop Unwinding:** Always add `#[kani::unwind(N)]` for loops with symbolic bounds, where N = max_iterations + 1. Missing unwind attributes are the #1 cause of Kani CI failures.

**When to run Kani locally:**

- Adding or modifying a `#[kani::proof]` function
- Changing code that is verified by existing Kani proofs
- Adding new `kani::assume()` or `kani::assert()` calls

**Kani installation (Linux/macOS only):**

```bash
cargo install --locked kani-verifier
cargo kani setup
```

See [`.llm/skills/kani-verification.md`](.llm/skills/kani-verification.md) for the complete Kani guide.

**This applies to ALL code changes, including those made by sub-agents.** CI will fail if:

- Code is not formatted (`cargo fmt --check`)
- Clippy warnings exist (`cargo clippy` with warnings as errors)
- Rustdoc has broken links or warnings (pre-commit uses strict `RUSTDOCFLAGS=-D warnings`)
- Markdown has lint errors (lists need blank lines around them)
- GitHub Actions workflows have syntax errors
