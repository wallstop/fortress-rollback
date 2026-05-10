# OpenAI Agents Instructions for Fortress Rollback

**Read and follow [`.llm/context.md`](.llm/context.md)** — the canonical source of truth for all project context, development policies, testing guidelines, and coding standards. You must read it before making any changes.

When clarifying questions are needed, follow [`.llm/templates/ask-user-question.md`](.llm/templates/ask-user-question.md) to keep questions concise and actionable.

## Codex CLI Devcontainer Contract

When editing devcontainer/tooling files, preserve these guarantees:

- Install Codex via npm package `@openai/codex@latest` in `.devcontainer/Dockerfile`
- Keep `.devcontainer/codex-bootstrap.sh` wired into devcontainer create/start hooks
- Do not commit credentials; use `codex login`, `codex login --device-auth`, or API key env vars
