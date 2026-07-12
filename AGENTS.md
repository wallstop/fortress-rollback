# OpenAI Agents Instructions for Fortress Rollback

**Read and follow [`.llm/context.md`](.llm/context.md)** — the canonical source of truth for all project context, development policies, testing guidelines, and coding standards. You must read it before making any changes.

When clarifying questions are needed, follow [`.llm/templates/ask-user-question.md`](.llm/templates/ask-user-question.md) to keep questions concise and actionable.

## Git and GitHub Tool Preference

When the environment exposes callable VS Code integrations, prefer the built-in VS Code Git
extension for source-control operations and the GitHub Pull Requests and Issues extension for
pull requests, issues, reviews, checks, and related GitHub work. Fall back to local `git`, the
connected GitHub app, and then authenticated `gh` only when the preferred integration is
unavailable or cannot perform the required operation. Never treat an unavailable VS Code UI as a
blocker when an equivalent non-UI path is available, and never print or persist credentials.

## Codex CLI Devcontainer Contract

When editing devcontainer/tooling files, preserve these guarantees:

- Install Codex via npm package `@openai/codex@latest` in `.devcontainer/Dockerfile`
- Keep `.devcontainer/codex-bootstrap.sh` wired into devcontainer create/start hooks
- Do not commit credentials; use `codex login`, `codex login --device-auth`, or API key env vars
