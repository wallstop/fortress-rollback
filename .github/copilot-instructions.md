# GitHub Copilot Instructions for Fortress Rollback

**Read [`.llm/context.md`](../.llm/context.md)** — the canonical source of truth for all project context, development policies, testing guidelines, and coding standards.

## Critical: Zero-Panic Policy

**All production code must follow defensive programming practices.** See [`.llm/skills/defensive-programming.md`](../.llm/skills/defensive-programming.md) for the complete guide.

Key requirements:

- **Never panic** — No `unwrap()`, `expect()`, `panic!()`, `todo!()`
- **Return `Result`** — All fallible operations must return errors, not panic
- **Never swallow errors** — Propagate or explicitly handle, never ignore
- **Validate everything** — Don't assume inputs or internal state are valid

## Quick Commands

```bash
cargo fmt && cargo clippy --all-targets && cargo nextest run --no-capture

# Or use the convenient aliases defined in .cargo/config.toml
cargo c && cargo t

# Markdown linting
npx markdownlint '**/*.md' --config .markdownlint.json --fix
```

**Always use `--no-capture`** (nextest) or `-- --nocapture` (cargo test) so that test output is visible immediately when failures occur. The aliases include this by default.

## Changelog Reminder

After code changes, ask: **Does this affect `pub` items or user-observable behavior?**

- If YES → Add entry to `CHANGELOG.md` under `## [Unreleased]` (use `**Breaking:**` prefix for API changes)
- If NO (`pub(crate)`, private, tests, CI) → No changelog needed
- See [changelog-practices.md](../.llm/skills/changelog-practices.md) for detailed guidance
