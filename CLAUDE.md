# Claude Instructions for Fortress Rollback

**Read [`.llm/context.md`](.llm/context.md)** — the canonical source of truth for all project context, development policies, testing guidelines, and coding standards. You must read it before making any changes.

## Critical Reminders

- **Zero-panic:** No `unwrap()`, `expect()`, `panic!()`, `todo!()` in production code
- **Pre-commit:** `cargo fmt && cargo clippy --all-targets --features tokio,json && cargo nextest run --no-capture` (or `cargo c && cargo t`)
- **Kani:** Always add `#[kani::unwind(N)]` to proofs; CI uses `--default-unwind 8` via `--quick` mode
- **Changelog:** Ask "Does this affect `pub` items or user-observable behavior?" — if yes, update `CHANGELOG.md`

All agents (including sub-agents) must read and follow `.llm/context.md`.
