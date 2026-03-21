# Claude Instructions for Fortress Rollback

**Read and follow [`.llm/context.md`](.llm/context.md)** — the canonical source of truth for all project context, development policies, testing guidelines, and coding standards. You must read it before making any changes.

When clarifying questions are needed, follow [`.llm/templates/ask-user-question.md`](.llm/templates/ask-user-question.md) to keep questions concise and actionable.

## Critical Reminders

- **Zero-panic:** No `unwrap()`, `expect()`, `panic!()`, `todo!()` in production code
- **Pre-commit:** `cargo fmt && cargo clippy --all-targets --features tokio,json && cargo nextest run --no-capture` (or `cargo c && cargo t`)
- **Test output:** NEVER pipe test output through `tail`/`head` (e.g., `cargo nextest run 2>&1 | tail -40`). Instead, redirect to a temp file and read it: `cargo nextest run --no-capture > /tmp/test-results.txt 2>&1`. For repeated runs, use a for loop.
- **Kani:** Always add `#[kani::unwind(N)]` to proofs; CI uses `--default-unwind 8` via `--quick` mode
- **Changelog:** Ask "Does this affect `pub` items or user-observable behavior?" — if yes, update `CHANGELOG.md`
