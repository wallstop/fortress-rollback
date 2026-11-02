#!/usr/bin/env bash
# Bridge Z.ai's SSE-formatted Streamable HTTP responses to stdio for Claude and Codex.

set -euo pipefail

fail() {
    printf '[zai-mcp-remote] ERROR: %s\n' "$*" >&2
    exit 1
}

server_alias="${1:-}"
case "${server_alias}" in
    web-search) endpoint="https://api.z.ai/api/mcp/web_search_prime/mcp" ;;
    web-reader) endpoint="https://api.z.ai/api/mcp/web_reader/mcp" ;;
    zread) endpoint="https://api.z.ai/api/mcp/zread/mcp" ;;
    *) fail "Expected one of: web-search, web-reader, zread." ;;
esac

[ "$#" = "1" ] || fail "Unexpected arguments for ${server_alias}."
command -v mcp-remote >/dev/null 2>&1 \
    || fail "mcp-remote is not installed; rebuild the devcontainer."

unset token zai_mcp_token
if [ -n "${CLAUDE_PROJECT_DIR:-}" ]; then
    runtime_env="${CLAUDE_PROJECT_DIR}/.devcontainer/.mcp.env"
    [ -f "${runtime_env}" ] && [ ! -L "${runtime_env}" ] \
        || fail "Filtered MCP credentials are missing; rebuild the devcontainer."
    [ "$(stat -c '%a' "${runtime_env}")" = "600" ] \
        || fail "Filtered MCP credentials must have mode 600; rebuild the devcontainer."
    definition_count="$({ grep -Ec '^ZAI_API_KEY=' "${runtime_env}" || true; })"
    [ "${definition_count}" = "1" ] \
        || fail "Filtered MCP credentials contain an invalid ZAI_API_KEY."
    zai_mcp_token="$(sed -n 's/^ZAI_API_KEY=//p' "${runtime_env}")"
elif [ -n "${ZAI_API_KEY:-}" ]; then
    # Codex explicitly forwards only this key through the server's env_vars.
    zai_mcp_token="${ZAI_API_KEY}"
else
    fail "A filtered Claude runtime credential or Codex-forwarded ZAI_API_KEY is required."
fi
zai_mcp_token="${zai_mcp_token%$'\r'}"
[ -n "${zai_mcp_token}" ] || fail "Filtered MCP credentials contain an invalid ZAI_API_KEY."
case "${zai_mcp_token}" in
    *[[:space:]\"\']*) fail "Filtered MCP credentials contain an invalid ZAI_API_KEY." ;;
esac

# The bridge receives the credential through an anonymous file descriptor. The
# handoff creates no additional persistent/header file and keeps the value out
# of argv and the bridge process environment.
unset ZAI_MCP_API_KEY \
    ANTHROPIC_AUTH_TOKEN \
    ANTHROPIC_API_KEY \
    CLAUDE_CODE_OAUTH_TOKEN \
    ZAI_API_KEY \
    Z_AI_API_KEY \
    GITHUB_PERSONAL_ACCESS_TOKEN \
    GITHUB_PAT \
    GITHUB_TOKEN \
    GH_TOKEN \
    OPENAI_API_KEY \
    CODEX_API_KEY
exec mcp-remote "${endpoint}" --header-file \
    <(printf 'Authorization: Bearer %s\n' "${zai_mcp_token}")
