#!/usr/bin/env bash
# Install and launch isolated Z.ai backends without changing native Codex or Claude defaults.

set -euo pipefail

# Prevent an imported lowercase variable from retaining its export attribute
# when a function-local variable temporarily holds the real credential.
export -n zai_key 2>/dev/null || true

readonly PROFILE_NAME="devcontainer-zai"
readonly ZAI_RESPONSES_URL="https://api.z.ai/api/v1"
readonly ZAI_ANTHROPIC_URL="https://api.z.ai/api/anthropic"
AI_BACKENDS_SOURCE_DIR="$(dirname "$(realpath "${BASH_SOURCE[0]}")")"
readonly AI_BACKENDS_SOURCE_DIR
readonly CLAUDE_ZAI_MCP_CONFIG="${AI_BACKENDS_SOURCE_DIR}/claude-zai.mcp.json"

die() {
    printf '[ai-backends] ERROR: %s\n' "$*" >&2
    exit 1
}

resolve_action() {
    local invoked_as
    invoked_as="$(basename "$0")"
    case "${invoked_as}" in
        codex-zai|claude-zai)
            printf '%s\n' "${invoked_as}"
            ;;
        *)
            printf '%s\n' "${1:-help}"
            ;;
    esac
}

resolve_zai_key() {
    if [ -n "${ZAI_API_KEY:-}" ]; then
        printf '%s' "${ZAI_API_KEY}"
        return
    fi
    if [ -n "${Z_AI_API_KEY:-}" ]; then
        printf '%s' "${Z_AI_API_KEY}"
        return
    fi
    die "Set ZAI_API_KEY (or Z_AI_API_KEY) before launching a Z.ai backend."
}

install_codex_profile() {
    local codex_home catalog_file catalog_path_toml catalog_tmp profile_file profile_tmp
    codex_home="${CODEX_HOME:-${HOME}/.codex}"
    catalog_file="${codex_home}/${PROFILE_NAME}-models.json"
    profile_file="${codex_home}/${PROFILE_NAME}.config.toml"
    catalog_path_toml="${catalog_file//\\/\\\\}"
    catalog_path_toml="${catalog_path_toml//\"/\\\"}"

    mkdir -p "${codex_home}"
    chmod 700 "${codex_home}"

    catalog_tmp="$(mktemp "${codex_home}/.${PROFILE_NAME}-models.XXXXXX")"
    profile_tmp="$(mktemp "${codex_home}/.${PROFILE_NAME}-profile.XXXXXX")"
    trap 'rm -f "${catalog_tmp:-}" "${profile_tmp:-}"' RETURN

    # Model metadata follows Z.ai's current Codex Responses integration contract.
    cat >"${catalog_tmp}" <<'JSON'
{
  "models": [
    {
      "slug": "glm-5.3",
      "display_name": "glm-5.3",
      "description": "Z.ai flagship coding model",
      "default_reasoning_level": "max",
      "supported_reasoning_levels": [
        {
          "effort": "low",
          "description": "Light reasoning"
        },
        {
          "effort": "high",
          "description": "Enhanced reasoning"
        },
        {
          "effort": "max",
          "description": "Deep reasoning"
        }
      ],
      "shell_type": "shell_command",
      "visibility": "list",
      "supported_in_api": true,
      "priority": 0,
      "base_instructions": "",
      "supports_reasoning_summaries": true,
      "default_reasoning_summary": "none",
      "support_verbosity": false,
      "apply_patch_tool_type": "freeform",
      "truncation_policy": {
        "mode": "bytes",
        "limit": 10000
      },
      "context_window": 1048576,
      "max_context_window": 1048576,
      "effective_context_window_percent": 95,
      "supports_parallel_tool_calls": true,
      "experimental_supported_tools": [],
      "input_modalities": [
        "text"
      ]
    }
  ]
}
JSON

    cat >"${profile_tmp}" <<TOML
model_provider = "ZAI"
model = "glm-5.3"
model_reasoning_effort = "max"
model_catalog_json = "${catalog_path_toml}"

[model_providers.ZAI]
name = "ZAI"
base_url = "${ZAI_RESPONSES_URL}"
env_key = "ZAI_API_KEY"
wire_api = "responses"
request_max_retries = 4
stream_max_retries = 5
stream_idle_timeout_ms = 300000

[shell_environment_policy]
filters = { ZAI_API_KEY = "exclude", Z_AI_API_KEY = "exclude" }
TOML

    chmod 600 "${catalog_tmp}" "${profile_tmp}"
    mv "${catalog_tmp}" "${catalog_file}"
    mv "${profile_tmp}" "${profile_file}"
    trap - RETURN
}

install_launchers() {
    local bin_dir codex_command launcher script_path target
    if [ -n "${AI_BACKENDS_BIN_DIR:-}" ]; then
        bin_dir="${AI_BACKENDS_BIN_DIR}"
    elif case ":${PATH}:" in *":${HOME}/.local/bin:"*) true ;; *) false ;; esac; then
        bin_dir="${HOME}/.local/bin"
    else
        codex_command="$(command -v codex || true)"
        if [ -n "${codex_command}" ] && [ -w "$(dirname "${codex_command}")" ]; then
            bin_dir="$(dirname "${codex_command}")"
        else
            bin_dir="${HOME}/.local/bin"
        fi
    fi
    script_path="$(realpath "${BASH_SOURCE[0]}")"
    mkdir -p "${bin_dir}"

    for launcher in codex-zai claude-zai; do
        target="${bin_dir}/${launcher}"
        if [ -e "${target}" ] && [ ! -L "${target}" ]; then
            die "Refusing to replace non-symlink launcher: ${target}"
        fi
        ln -sfn "${script_path}" "${target}"
    done
}

install_backend_support() {
    install_codex_profile
    install_launchers
    printf '[ai-backends] Installed codex-zai and claude-zai launchers.\n'
}

launch_codex_zai() {
    local catalog_file codex_home model reasoning_effort zai_key
    command -v codex >/dev/null 2>&1 || die "codex is not installed."
    zai_key="$(resolve_zai_key)"
    export -n zai_key 2>/dev/null || true
    export ZAI_API_KEY="${zai_key}"
    zai_key=""

    codex_home="${CODEX_HOME:-${HOME}/.codex}"
    catalog_file="${codex_home}/${PROFILE_NAME}-models.json"
    if [ ! -f "${codex_home}/${PROFILE_NAME}.config.toml" ] || [ ! -f "${catalog_file}" ]; then
        install_codex_profile
    fi

    model="${CODEX_ZAI_MODEL:-glm-5.3}"
    reasoning_effort="${CODEX_ZAI_REASONING_EFFORT:-max}"
    case "${reasoning_effort}" in
        low|high|max) ;;
        *) die "CODEX_ZAI_REASONING_EFFORT must be low, high, or max." ;;
    esac
    exec codex \
        --profile "${PROFILE_NAME}" \
        --model "${model}" \
        --config 'model_provider="ZAI"' \
        --config "model_reasoning_effort=\"${reasoning_effort}\"" \
        --config "model_catalog_json=\"${catalog_file}\"" \
        "$@"
}

launch_claude_zai() {
    local claude_settings config_dir container_mode subprocess_scrub timeout_ms zai_key
    local -a claude_args=()
    command -v claude >/dev/null 2>&1 || die "claude is not installed."
    zai_key="$(resolve_zai_key)"
    export -n zai_key 2>/dev/null || true
    config_dir="${CLAUDE_ZAI_CONFIG_DIR:-${HOME}/.claude-zai}"
    timeout_ms="${ZAI_API_TIMEOUT_MS:-300000}"
    case "${timeout_ms}" in
        ''|*[!0-9]*) die "ZAI_API_TIMEOUT_MS must be a positive integer." ;;
        0) die "ZAI_API_TIMEOUT_MS must be greater than zero." ;;
    esac
    mkdir -p "${config_dir}"
    chmod 700 "${config_dir}"
    [ -f "${CLAUDE_ZAI_MCP_CONFIG}" ] \
        || die "Claude Z.ai MCP config is missing: ${CLAUDE_ZAI_MCP_CONFIG}"
    claude_args+=(--mcp-config "${CLAUDE_ZAI_MCP_CONFIG}" --strict-mcp-config)

    container_mode="${AI_BACKENDS_CONTAINER_MODE:-auto}"
    case "${container_mode}" in
        auto)
            if [ -f /.dockerenv ] || [ -f /run/.containerenv ]; then
                container_mode=yes
            else
                container_mode=no
            fi
            ;;
        yes|no) ;;
        *) die "AI_BACKENDS_CONTAINER_MODE must be auto, yes, or no." ;;
    esac

    subprocess_scrub="${CLAUDE_ZAI_SUBPROCESS_ENV_SCRUB:-auto}"
    case "${subprocess_scrub}" in
        auto)
            if [ "${container_mode}" = "yes" ]; then
                # The devcontainer is already the isolation boundary. Claude's
                # additional Linux scrub sandbox still requires CLONE_NEWUSER
                # and mount operations that Docker's default profiles deny.
                subprocess_scrub=0
            else
                subprocess_scrub=1
            fi
            ;;
        0|1) ;;
        *) die "CLAUDE_ZAI_SUBPROCESS_ENV_SCRUB must be auto, 0, or 1." ;;
    esac

    if [ "$(uname -s)" = "Linux" ] && [ "${subprocess_scrub}" = "1" ]; then
        command -v bwrap >/dev/null 2>&1 \
            || die "bubblewrap is required for Claude subprocess isolation; install bubblewrap and socat."
        command -v socat >/dev/null 2>&1 \
            || die "socat is required for Claude sandbox networking; install bubblewrap and socat."
        bwrap --unshare-user --ro-bind / / --bind /proc /proc --dev /dev true \
            || die "Claude subprocess isolation cannot create its nested sandbox. Set CLAUDE_ZAI_SUBPROCESS_ENV_SCRUB=0 to rely on the outer devcontainer boundary."
    fi

    claude_settings='{"enabledMcpjsonServers":["github-mcp-server","zai-vision","zai-web-search","zai-web-reader","zai-zread"]}'
    if [ "${container_mode}" = "yes" ]; then
        # The outer devcontainer is the primary isolation boundary. Anthropic's
        # weaker mode still needs blocked namespaces, so keep the inner sandbox
        # off unless the caller explicitly opts into the scrubber and preflight.
        claude_settings='{"enabledMcpjsonServers":["github-mcp-server","zai-vision","zai-web-search","zai-web-reader","zai-zread"],"sandbox":{"enabled":false,"enableWeakerNestedSandbox":true}}'
    fi
    claude_args+=(--settings "${claude_settings}")

    unset ANTHROPIC_API_KEY \
        CLAUDE_CODE_OAUTH_TOKEN \
        ANTHROPIC_MODEL \
        ANTHROPIC_DEFAULT_MODEL \
        ANTHROPIC_DEFAULT_HAIKU_MODEL \
        ANTHROPIC_DEFAULT_SONNET_MODEL \
        ANTHROPIC_DEFAULT_OPUS_MODEL \
        ANTHROPIC_SMALL_FAST_MODEL \
        CLAUDE_CODE_PROVIDER_MANAGED_BY_HOST \
        CLAUDE_CODE_USE_ANTHROPIC_AWS \
        CLAUDE_CODE_USE_BEDROCK \
        CLAUDE_CODE_USE_VERTEX \
        CLAUDE_CODE_USE_FOUNDRY \
        CLAUDE_CODE_USE_MANTLE \
        CLAUDE_CODE_USE_GATEWAY
    export ANTHROPIC_AUTH_TOKEN="${zai_key}"
    zai_key=""
    unset ZAI_API_KEY Z_AI_API_KEY ZAI_MCP_API_KEY
    export ANTHROPIC_BASE_URL="${ZAI_ANTHROPIC_URL}"
    export ANTHROPIC_DEFAULT_HAIKU_MODEL="${CLAUDE_ZAI_HAIKU_MODEL:-glm-5.3-flash[1m]}"
    export ANTHROPIC_DEFAULT_SONNET_MODEL="${CLAUDE_ZAI_SONNET_MODEL:-glm-5.3[1m]}"
    export ANTHROPIC_DEFAULT_OPUS_MODEL="${CLAUDE_ZAI_OPUS_MODEL:-glm-5.3[1m]}"
    export API_TIMEOUT_MS="${timeout_ms}"
    export CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000
    export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1
    export CLAUDE_CODE_PROVIDER_MANAGED_BY_HOST=1
    export CLAUDE_CODE_SUBPROCESS_ENV_SCRUB="${subprocess_scrub}"
    export CLAUDE_CONFIG_DIR="${config_dir}"
    exec claude "${claude_args[@]}" "$@"
}

print_help() {
    cat <<'HELP'
Usage:
  bash .devcontainer/ai-backends.sh install
  codex-zai [codex arguments...]
  claude-zai [claude arguments...]

The ordinary `codex` and `claude` commands retain their native backends.
Set ZAI_API_KEY (or Z_AI_API_KEY) only in your private shell/secret store.
Optional overrides: CODEX_ZAI_MODEL, CODEX_ZAI_REASONING_EFFORT,
ZAI_API_TIMEOUT_MS, CLAUDE_ZAI_CONFIG_DIR, CLAUDE_ZAI_HAIKU_MODEL,
CLAUDE_ZAI_SONNET_MODEL, CLAUDE_ZAI_OPUS_MODEL, AI_BACKENDS_CONTAINER_MODE,
CLAUDE_ZAI_SUBPROCESS_ENV_SCRUB (auto, 0, or 1).
HELP
}

action="$(resolve_action "${1:-}")"
if [ "$(basename "$0")" != "codex-zai" ] && [ "$(basename "$0")" != "claude-zai" ] && [ "$#" -gt 0 ]; then
    shift
fi

case "${action}" in
    install) install_backend_support ;;
    codex-zai) launch_codex_zai "$@" ;;
    claude-zai) launch_claude_zai "$@" ;;
    help|-h|--help) print_help ;;
    *) die "Unknown action: ${action}" ;;
esac
