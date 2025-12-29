# OpenAI Agents Instructions for Fortress Rollback

**Read [`.llm/context.md`](.llm/context.md)** — the canonical source of truth for all project context, development policies, testing guidelines, and coding standards.

## Critical: Zero-Panic Policy

**All production code must follow defensive programming practices.** See [`.llm/skills/defensive-programming.md`](.llm/skills/defensive-programming.md) for the complete guide.

Key requirements:

- **Never panic** — No `unwrap()`, `expect()`, `panic!()`, `todo!()`
- **Return `Result`** — All fallible operations must return errors, not panic
- **Never swallow errors** — Propagate or explicitly handle, never ignore
- **Validate everything** — Don't assume inputs or internal state are valid

## Quick Commands

```bash
# Rust code
cargo fmt && cargo clippy --all-targets && cargo nextest run

# Or use the convenient aliases defined in .cargo/config.toml
cargo c && cargo t

# Markdown files (if modified)
npx markdownlint '**/*.md' --config .markdownlint.json --fix
```
