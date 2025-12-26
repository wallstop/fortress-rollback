# Claude Instructions for Fortress Rollback

**Read [`.llm/context.md`](.llm/context.md)** — the canonical source of truth for all project context, development policies, testing guidelines, and coding standards.

## Mandatory Pre-Commit Checks

**ALWAYS run before committing ANY changes:**

```bash
cargo fmt && cargo clippy --all-targets && cargo nextest run

# Or use the convenient aliases defined in .cargo/config.toml
cargo c && cargo t
```

**This applies to ALL code changes, including those made by sub-agents.** CI will fail if:
- Code is not formatted (`cargo fmt --check`)
- Clippy warnings exist (`cargo clippy` with warnings as errors)
