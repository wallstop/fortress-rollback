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

[ -f "${env_file}" ] \
    || fail "Missing .env.local. Copy .env.local.example, fill both credentials, then rebuild the devcontainer."
[ ! -L "${env_file}" ] \
    || fail "Refusing to read a symlinked .env.local."
chmod 600 "${env_file}" \
    || fail "Could not restrict .env.local permissions to the current host user."

unset github_token zai_token value
github_token=""
zai_token=""
for key in GITHUB_PERSONAL_ACCESS_TOKEN ZAI_API_KEY; do
    definition_count="$({ grep -Ec "^${key}=" "${env_file}" || true; })"
    [ "${definition_count}" = "1" ] \
        || fail "${key} must appear exactly once with a non-empty, unquoted value in .env.local."
    value="$(sed -n "s/^${key}=//p" "${env_file}")"
    value="${value%$'\r'}"
    [ -n "${value}" ] \
        || fail "${key} must appear exactly once with a non-empty, unquoted value in .env.local."
    case "${value}" in
        *[[:space:]\"\']*)
            fail "${key} must use Docker env-file syntax without quotes or whitespace."
            ;;
    esac
    case "${key}" in
        GITHUB_PERSONAL_ACCESS_TOKEN) github_token="${value}" ;;
        ZAI_API_KEY) zai_token="${value}" ;;
    esac
done

umask 077
temporary_file="$(mktemp "${generated_file}.XXXXXX")" \
    || fail "Could not create the filtered MCP environment file."
{
    printf 'GITHUB_PERSONAL_ACCESS_TOKEN=%s\n' "${github_token}"
    printf 'ZAI_API_KEY=%s\n' "${zai_token}"
} >"${temporary_file}"
chmod 600 "${temporary_file}"
mv -f "${temporary_file}" "${generated_file}"
temporary_file=""
[ -f "${generated_file}" ] \
    || fail "Atomic replacement did not create the filtered MCP environment file."
generation_complete=1
trap - EXIT
printf '[devcontainer:mcp] Filtered MCP credentials generated; values were not displayed.\n'
