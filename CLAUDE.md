# Claude Instructions for Fortress Rollback

**Read and follow [`.llm/context.md`](.llm/context.md)** — the canonical source of truth for all project context, development policies, testing guidelines, and coding standards. You must read it before making any changes.

When clarifying questions are needed, follow [`.llm/templates/ask-user-question.md`](.llm/templates/ask-user-question.md) to keep questions concise and actionable.

## Git and GitHub Tool Preference

Use callable VS Code integrations first: the built-in VS Code Git extension for source-control
operations and the GitHub Pull Requests and Issues connector for pull requests, issues, reviews,
checks, and related GitHub work. If they cannot perform the operation, use local `git` for
repository work and the connected GitHub connector/app for platform work. `gh` is an absolute
fallback, allowed only when the VS Code connector, local Git, and connected GitHub connector/app
are unavailable or incapable. `gh auth status` is not a prerequisite when another supported path
exists. Never print or persist credentials.

## Codex CLI Devcontainer Contract

When editing devcontainer/tooling files, preserve these guarantees:

- Install Codex via npm package `@openai/codex@latest` in `.devcontainer/Dockerfile`
- Keep `.devcontainer/codex-bootstrap.sh` wired into devcontainer create/start hooks
- Do not commit credentials; use `codex login`, `codex login --device-auth`, or API key env vars

## Critical Reminders

- **Zero-panic:** No `unwrap()`, `expect()`, `panic!()`, `todo!()` in production code
- **Pre-commit:** `cargo fmt && cargo clippy --workspace --all-targets --features tokio,json && cargo nextest run --no-capture` (or `cargo c && cargo t`)
- **Agent preflight:** Before finalizing changes, run `python3 scripts/ci/agent-preflight.py --auto-fix`. If output includes `Falling back to --all checks.`, resolve the git-state issue and rerun preflight.
- **Test output:** NEVER pipe test output through `tail`/`head` (e.g., `cargo nextest run 2>&1 | tail -40`). Instead, redirect to a temp file and read it: `cargo nextest run --no-capture > /tmp/test-results.txt 2>&1`. For repeated runs, use a for loop.
- **Kani:** Always add `#[kani::unwind(N)]` to proofs; CI uses `--default-unwind 8` via `--quick` mode
- **Changelog:** Ask "Does this affect `pub` items or user-observable behavior?" — if yes, update `CHANGELOG.md`
