#!/usr/bin/env bash
# Adapt the canonical project key name to the Z.ai Vision MCP package.

set -euo pipefail

zai_api_key="${ZAI_API_KEY:-${Z_AI_API_KEY:-}}"
export -n zai_api_key 2>/dev/null || true
if [ -z "${zai_api_key}" ] && [ -n "${CLAUDE_PROJECT_DIR:-}" ]; then
    runtime_env="${CLAUDE_PROJECT_DIR}/.devcontainer/.mcp.env"
    if [ ! -f "${runtime_env}" ] || [ -L "${runtime_env}" ]; then
        printf 'zai-mcp-vision: Filtered MCP credentials are missing; rebuild the devcontainer.\n' >&2
        exit 1
    fi
    if [ "$(stat -c '%a' "${runtime_env}")" != "600" ]; then
        printf 'zai-mcp-vision: Filtered MCP credentials must have mode 600; rebuild the devcontainer.\n' >&2
        exit 1
    fi
    definition_count="$({ grep -Ec '^ZAI_API_KEY=' "${runtime_env}" || true; })"
    if [ "${definition_count}" != "1" ]; then
        printf 'zai-mcp-vision: Filtered MCP credentials contain an invalid ZAI_API_KEY.\n' >&2
        exit 1
    fi
    zai_api_key="$(sed -n 's/^ZAI_API_KEY=//p' "${runtime_env}")"
    zai_api_key="${zai_api_key%$'\r'}"
    case "${zai_api_key}" in
        *[[:space:]\"\']*)
            printf 'zai-mcp-vision: Filtered MCP credentials contain an invalid ZAI_API_KEY.\n' >&2
            exit 1
            ;;
    esac
fi
if [ -z "${zai_api_key}" ]; then
    printf 'zai-mcp-vision: ZAI_API_KEY is missing; update .env.local and rebuild the devcontainer.\n' >&2
    exit 1
fi
if ! command -v zai-mcp-server >/dev/null 2>&1; then
    printf 'zai-mcp-vision: zai-mcp-server is missing; rebuild the devcontainer.\n' >&2
    exit 1
fi

unset ZAI_API_KEY \
    Z_AI_API_KEY \
    ZAI_MCP_API_KEY \
    ANTHROPIC_AUTH_TOKEN \
    ANTHROPIC_API_KEY \
    CLAUDE_CODE_OAUTH_TOKEN \
    GITHUB_PERSONAL_ACCESS_TOKEN \
    GITHUB_PAT \
    GITHUB_TOKEN \
    GH_TOKEN \
    OPENAI_API_KEY \
    CODEX_API_KEY
export Z_AI_API_KEY="${zai_api_key}"
export Z_AI_MODE=ZAI
unset zai_api_key
exec zai-mcp-server "$@"
