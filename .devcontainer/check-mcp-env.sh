#!/usr/bin/env bash
# Filter and validate host-side credentials before Docker consumes .mcp.env.

set -euo pipefail

workspace_dir="${1:-$(pwd -P)}"
env_file="${workspace_dir}/.env.local"
generated_file="${workspace_dir}/.devcontainer/.mcp.env"
temporary_file=""
generation_complete=0

fail() {
    printf '[devcontainer:mcp] ERROR: %s\n' "$*" >&2
    exit 1
}

warn() {
    printf '[devcontainer:mcp] WARNING: %s\n' "$*" >&2
}

cleanup_temporary_file() {
    if [ -n "${temporary_file}" ] && [ -f "${temporary_file}" ]; then
        rm -f -- "${temporary_file}"
    fi
    if [ "${generation_complete}" != "1" ] && [ -f "${generated_file}" ] \
        && [ ! -L "${generated_file}" ]; then
        rm -f -- "${generated_file}"
    fi
}

[ -d "${workspace_dir}" ] || fail "Workspace directory does not exist."
[ -d "${workspace_dir}/.devcontainer" ] && [ ! -L "${workspace_dir}/.devcontainer" ] \
    || fail ".devcontainer must be a real directory inside the workspace."
trap cleanup_temporary_file EXIT
[ ! -L "${generated_file}" ] \
    || fail "Refusing to replace a symlinked .devcontainer/.mcp.env."
# Revoke the previously generated copy before validating the source. A missing or
# malformed .env.local must fail closed instead of leaving stale credentials.
rm -f -- "${generated_file}" \
    || fail "Could not remove the previous filtered MCP environment file."

unset github_token zai_token value
github_token=""
zai_token=""
credentials_configured=0
if [ -e "${env_file}" ] || [ -L "${env_file}" ]; then
    [ ! -L "${env_file}" ] \
        || fail "Refusing to read a symlinked .env.local."
    [ -f "${env_file}" ] \
        || fail ".env.local must be a regular file."
    chmod 600 "${env_file}" \
        || fail "Could not restrict .env.local permissions to the current host user."

    # Each key is independent: a contributor with only a GitHub token can still
    # enable the GitHub server, and vice versa. Blank values behave like an
    # omitted key so copying .env.local.example verbatim never blocks startup.
    for key in GITHUB_PERSONAL_ACCESS_TOKEN ZAI_API_KEY; do
        definition_count="$({ grep -Ec "^${key}=" "${env_file}" || true; })"
        if [ "${definition_count}" = "0" ]; then
            if grep -Eq "^[[:space:]]*(export[[:space:]]+)?${key}[[:space:]]*=" \
                "${env_file}"; then
                fail "${key} must be written as ${key}=value on its own line without 'export' or spaces."
            fi
            warn "${key} is not set; its MCP servers will stay unavailable."
            continue
        fi
        [ "${definition_count}" = "1" ] \
            || fail "${key} must appear at most once in .env.local."
        value="$(sed -n "s/^${key}=//p" "${env_file}")"
        value="${value%$'\r'}"
        if [ -z "${value}" ]; then
            warn "${key} is blank; its MCP servers will stay unavailable."
            continue
        fi
        case "${value}" in
            *[[:space:]\"\']*)
                fail "${key} must use Docker env-file syntax without quotes or whitespace."
                ;;
        esac
        case "${key}" in
            GITHUB_PERSONAL_ACCESS_TOKEN) github_token="${value}" ;;
            ZAI_API_KEY) zai_token="${value}" ;;
        esac
        credentials_configured=1
    done
fi

umask 077
temporary_file="$(mktemp "${generated_file}.XXXXXX")" \
    || fail "Could not create the filtered MCP environment file."
if [ -n "${github_token}" ]; then
    printf 'GITHUB_PERSONAL_ACCESS_TOKEN=%s\n' "${github_token}" \
        >"${temporary_file}"
fi
if [ -n "${zai_token}" ]; then
    printf 'ZAI_API_KEY=%s\n' "${zai_token}" >>"${temporary_file}"
fi
chmod 600 "${temporary_file}"
mv -f "${temporary_file}" "${generated_file}"
temporary_file=""
[ -f "${generated_file}" ] \
    || fail "Atomic replacement did not create the filtered MCP environment file."
generation_complete=1
trap - EXIT
if [ "${credentials_configured}" = "1" ]; then
    printf '[devcontainer:mcp] Filtered MCP credentials generated; values were not displayed.\n'
    if [ -z "${github_token}" ]; then
        warn "github-mcp-server stays unavailable without GITHUB_PERSONAL_ACCESS_TOKEN."
    fi
    if [ -z "${zai_token}" ]; then
        warn "The Vision, Web Search, Web Reader, and Zread servers stay unavailable without ZAI_API_KEY."
    fi
else
    # Project MCP servers are optional developer tooling. Docker still requires
    # the env file named by runArgs, so write an empty one instead of blocking
    # container startup for contributors who do not configure those servers.
    warn "No MCP credentials are configured; project MCP servers will stay unavailable. Copy .env.local.example to .env.local, fill the keys you have, and rebuild the devcontainer (a restart is not enough)."
    printf '[devcontainer:mcp] Empty MCP environment generated; continuing without project MCP servers.\n'
fi
