# OpenAI Agents Instructions for Fortress Rollback

**Read and follow [`.agents/skills/fortress-development/SKILL.md`](.agents/skills/fortress-development/SKILL.md)** — the canonical source of truth for all project context, development policies, testing guidelines, and coding standards. You must read it before making any changes.

When clarifying questions are needed, follow [`.agents/skills/fortress-development/assets/ask-user-question.md`](.agents/skills/fortress-development/assets/ask-user-question.md) to keep questions concise and actionable.

## Git and GitHub Tool Preference

Use callable VS Code integrations first: the built-in VS Code Git extension for source-control
operations and the GitHub Pull Requests and Issues connector for pull requests, issues, reviews,
checks, and related GitHub work. If they cannot perform the operation, use local `git` for
repository work and the connected GitHub connector/app for platform work. `gh` is an absolute
fallback, allowed only when the VS Code connector, local Git, and connected GitHub connector/app
are unavailable or incapable. `gh auth status` is not a prerequisite when another supported path
exists. Never treat an unavailable VS Code UI as a blocker when an equivalent callable or Git path
is available, and never print or persist credentials.

## Codex CLI Devcontainer Contract

When editing devcontainer/tooling files, preserve these guarantees:

- Install Codex via npm package `@openai/codex@latest` in `.devcontainer/Dockerfile`
- Keep `.devcontainer/codex-bootstrap.sh` wired into devcontainer create/start hooks
- Do not commit credentials; use `codex login`, `codex login --device-auth`, or API key env vars
