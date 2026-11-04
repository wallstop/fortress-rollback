#!/bin/bash
# Keep user-installed npm CLIs current and bound disposable Cargo artifacts.
# Lifecycle maintenance is intentionally non-blocking so an offline registry or
# a failed cleanup can never prevent the development container from opening.

set -u -o pipefail

HOOK_PHASE="${1:-post-start}"
WORKSPACE_DIR="${DEVCONTAINER_WORKSPACE_DIR:-$(pwd -P)}"

NPM_USER_PREFIX="${NPM_CONFIG_PREFIX:-${HOME}/.local}"
NPM_USER_CACHE="${NPM_CONFIG_CACHE:-${HOME}/.cache/npm}"
TARGET_DIR="${FORTRESS_TARGET_DIR:-${CARGO_TARGET_DIR:-${WORKSPACE_DIR}/target}}"
TARGET_MAX_BYTES="${FORTRESS_TARGET_MAX_BYTES:-8589934592}"
TARGET_CLEAN_MIN_AGE_DAYS="${FORTRESS_TARGET_CLEAN_MIN_AGE_DAYS:-7}"
NPM_REFRESH_TIMEOUT_SECONDS="${FORTRESS_NPM_REFRESH_TIMEOUT_SECONDS:-120}"
MAINTENANCE_CACHE_DIR="${XDG_CACHE_HOME:-${HOME}/.cache}/fortress-rollback"
TARGET_CLEAN_STAMP="${MAINTENANCE_CACHE_DIR}/last-target-clean"
MAX_SAFE_INTEGER_DIGITS=18

log() {
    printf '[devcontainer-bootstrap] %s\n' "$*"
}

warn() {
    printf '[devcontainer-bootstrap] WARNING: %s\n' "$*" >&2
}

is_nonnegative_integer() {
    case "$1" in
        ''|*[!0-9]*) return 1 ;;
        *) return 0 ;;
    esac
}

is_bounded_nonnegative_integer() {
    is_nonnegative_integer "$1" && [ "${#1}" -le "${MAX_SAFE_INTEGER_DIGITS}" ]
}

run_with_timeout() {
    local timeout_seconds="$1"
    shift

    if command -v timeout >/dev/null 2>&1; then
        timeout "${timeout_seconds}" "$@"
    elif command -v gtimeout >/dev/null 2>&1; then
        gtimeout "${timeout_seconds}" "$@"
    else
        "$@"
    fi
}

restore_opencode_v2_executable() {
    local relative_target="../lib/node_modules/@opencode/cli/bin/opencode.exe"
    local v2_executable="${NPM_USER_PREFIX}/lib/node_modules/@opencode/cli/bin/opencode.exe"

    [ -x "${v2_executable}" ] || return 1
    ln -sfn "${relative_target}" "${NPM_USER_PREFIX}/bin/opencode" || return 1
}

# OpenCode 1 (npm package ``opencode-ai``) and OpenCode 2 (``@opencode/cli``)
# both publish an ``opencode`` executable. Refresh V2 before removing V1, then
# restore the shared bin from the installed package without relying on npm access.
remove_legacy_opencode_v1() {
    local legacy_package_dir="${NPM_USER_PREFIX}/lib/node_modules/opencode-ai"
    local v2_executable="${NPM_USER_PREFIX}/lib/node_modules/@opencode/cli/bin/opencode.exe"

    if [ ! -d "${legacy_package_dir}" ]; then
        return 0
    fi
    if [ ! -x "${v2_executable}" ]; then
        warn "Keeping legacy opencode-ai because no usable OpenCode 2 executable is installed."
        return 0
    fi

    log "Removing legacy OpenCode 1 package; restoring the installed OpenCode 2 executable."
    if run_with_timeout "${NPM_REFRESH_TIMEOUT_SECONDS}" \
        npm uninstall --global --no-audit --no-fund opencode-ai; then
        if restore_opencode_v2_executable; then
            log "Removed legacy opencode-ai and restored the OpenCode 2 executable."
        else
            warn "Removed legacy opencode-ai but could not restore the OpenCode 2 executable."
        fi
    else
        warn "Could not remove legacy opencode-ai; a stale OpenCode 1 binary may shadow OpenCode 2."
    fi
}

verify_opencode_v2() {
    if run_with_timeout "${NPM_REFRESH_TIMEOUT_SECONDS}" opencode --version 2>/dev/null \
        | grep -Eq '(^|[[:space:]])v?2[.]'; then
        log "OpenCode 2 executable verified."
    else
        warn "OpenCode 2 is not available after refresh; check for a stale V1 executable and rebuild."
    fi
}

refresh_ai_clis() {
    local package package_name package_json installed_version latest_version
    local lookup_status install_status

    if [ "${DEVCONTAINER_SKIP_TOOL_REFRESH:-0}" = "1" ]; then
        log "Skipping npm CLI refresh by request."
        return 0
    fi

    if ! command -v npm >/dev/null 2>&1; then
        warn "npm is unavailable; keeping the image-bundled CLI versions."
        return 0
    fi

    if ! is_nonnegative_integer "${NPM_REFRESH_TIMEOUT_SECONDS}" \
        || [ "${NPM_REFRESH_TIMEOUT_SECONDS}" -eq 0 ]; then
        warn "FORTRESS_NPM_REFRESH_TIMEOUT_SECONDS must be a positive integer; using 120."
        NPM_REFRESH_TIMEOUT_SECONDS=120
    fi

    if ! mkdir -p \
        "${NPM_USER_PREFIX}/bin" \
        "${NPM_USER_PREFIX}/lib/node_modules" \
        "${NPM_USER_CACHE}"; then
        warn "Could not create the user-owned npm prefix/cache; skipping refresh."
        return 0
    fi

    if [ ! -w "${NPM_USER_PREFIX}" ] || [ ! -w "${NPM_USER_CACHE}" ]; then
        warn "npm prefix/cache is not writable by $(id -un); skipping refresh without sudo."
        return 0
    fi

    export NPM_CONFIG_PREFIX="${NPM_USER_PREFIX}"
    export NPM_CONFIG_CACHE="${NPM_USER_CACHE}"
    export PATH="${NPM_USER_PREFIX}/bin:${PATH}"

    log "Refreshing AI CLIs from npm latest tags in ${NPM_USER_PREFIX}."
    for package in \
        '@nanocollective/nanocoder@latest' \
        '@opencode/cli@latest' \
        '@openai/codex@latest' \
        '@anthropic-ai/claude-code@latest' \
        '@github/copilot@latest' \
        'mcp-remote@latest' \
        '@z_ai/mcp-server@latest'; do
        package_name="${package%@latest}"
        package_json="${NPM_USER_PREFIX}/lib/node_modules/${package_name}/package.json"
        installed_version=""
        if [ -f "${package_json}" ]; then
            installed_version="$(
                node -e 'process.stdout.write(require(process.argv[1]).version)' \
                    "${package_json}" 2>/dev/null
            )"
        fi

        latest_version="$(
            run_with_timeout "${NPM_REFRESH_TIMEOUT_SECONDS}" \
                npm view "${package}" version --prefer-online
        )"
        lookup_status=$?

        if [ "${lookup_status}" -eq 0 ] \
            && [ -n "${latest_version}" ] \
            && [ "${installed_version}" = "${latest_version}" ]; then
            log "${package_name} ${installed_version} is current."
            continue
        fi

        if [ "${lookup_status}" -ne 0 ] && [ -n "${installed_version}" ]; then
            warn "Could not resolve ${package}; keeping installed ${installed_version}."
            continue
        fi

        run_with_timeout "${NPM_REFRESH_TIMEOUT_SECONDS}" \
            npm install --global --no-audit --no-fund --prefer-online \
                --allow-scripts=@opencode/cli,@anthropic-ai/claude-code "${package}"
        install_status=$?

        if [ "${install_status}" -eq 0 ]; then
            log "${package} is current."
        else
            warn "${package} refresh failed; continuing with any installed version."
        fi
    done

    remove_legacy_opencode_v1
    verify_opencode_v2
}

cleanup_target_if_needed() {
    if [ "${DEVCONTAINER_SKIP_TARGET_CLEANUP:-0}" = "1" ]; then
        log "Skipping Cargo artifact cleanup by request."
        return 0
    fi

    if [ ! -d "${TARGET_DIR}" ]; then
        return 0
    fi

    if ! is_bounded_nonnegative_integer "${TARGET_MAX_BYTES}"; then
        warn "FORTRESS_TARGET_MAX_BYTES must fit the supported non-negative integer range; skipping cleanup."
        return 0
    fi
    if ! is_bounded_nonnegative_integer "${TARGET_CLEAN_MIN_AGE_DAYS}"; then
        warn "FORTRESS_TARGET_CLEAN_MIN_AGE_DAYS must fit the supported non-negative integer range; skipping cleanup."
        return 0
    fi

    local resolved_target resolved_workspace managed_target
    resolved_workspace="$(realpath -m "${WORKSPACE_DIR}")"
    managed_target="${resolved_workspace}/target"

    if [ -L "${managed_target}" ]; then
        warn "Refusing symlink target directory: ${managed_target}"
        return 0
    fi

    resolved_target="$(realpath -m "${TARGET_DIR}")"

    if [ "${resolved_target}" != "${managed_target}" ]; then
        warn "Refusing unmanaged target directory: ${resolved_target}"
        return 0
    fi

    local cargo_cache_marker
    cargo_cache_marker="${resolved_target}/CACHEDIR.TAG"
    if [ ! -f "${cargo_cache_marker}" ] \
        || ! grep -Fqx 'Signature: 8a477f597d28d172789f06886806bc55' \
            "${cargo_cache_marker}"; then
        warn "Refusing target directory missing Cargo cache marker: ${resolved_target}"
        return 0
    fi

    local target_kib target_bytes
    if ! target_kib="$(du -sk "${resolved_target}" 2>/dev/null | awk '{print $1}')"; then
        warn "Could not measure ${resolved_target}; skipping cleanup."
        return 0
    fi
    if ! is_nonnegative_integer "${target_kib}"; then
        warn "Unexpected size while measuring ${resolved_target}; skipping cleanup."
        return 0
    fi
    target_bytes=$((target_kib * 1024))

    if [ "${target_bytes}" -le "${TARGET_MAX_BYTES}" ]; then
        log "Cargo target is within limit (${target_bytes} <= ${TARGET_MAX_BYTES} bytes)."
        return 0
    fi

    if [ "${TARGET_CLEAN_MIN_AGE_DAYS}" -gt 0 ] && [ -f "${TARGET_CLEAN_STAMP}" ]; then
        local recent_clean_stamp
        if ! recent_clean_stamp="$(
            find "${TARGET_CLEAN_STAMP}" \
                -mtime "-${TARGET_CLEAN_MIN_AGE_DAYS}" -print -quit
        )"; then
            warn "Could not validate the target cleanup interval; skipping cleanup."
            return 0
        fi
        if [ -n "${recent_clean_stamp}" ]; then
            log "Cargo target exceeds the limit, but cleanup ran within ${TARGET_CLEAN_MIN_AGE_DAYS} days."
            return 0
        fi
    fi

    if ! command -v cargo >/dev/null 2>&1; then
        warn "Cargo target exceeds the limit, but cargo is unavailable for safe cleanup."
        return 0
    fi

    log "Cargo target exceeds ${TARGET_MAX_BYTES} bytes (${target_bytes}); running cargo clean."
    if cargo clean --target-dir "${resolved_target}"; then
        if mkdir -p "${MAINTENANCE_CACHE_DIR}" && touch "${TARGET_CLEAN_STAMP}"; then
            log "Cargo artifacts cleaned; future builds will repopulate the cache."
        else
            warn "Cargo artifacts cleaned, but the maintenance timestamp could not be recorded."
        fi
    else
        warn "Cargo cleanup failed; continuing startup."
    fi
}

# Report the credentials actually present in this process. The generated file is
# only a cross-check: stale or file-only values must never look successfully injected.
report_mcp_configuration() {
    if [ "${DEVCONTAINER_SKIP_MCP_NOTICE:-0}" = "1" ]; then
        return 0
    fi

    local runtime_env="${WORKSPACE_DIR}/.devcontainer/.mcp.env"
    local github_token="${GITHUB_PERSONAL_ACCESS_TOKEN:-}"
    local zai_token="${ZAI_API_KEY:-}"
    local key file_value process_value definition_count
    local missing_groups=""

    if [ ! -f "${runtime_env}" ] || [ -L "${runtime_env}" ]; then
        if [ -n "${github_token}" ] || [ -n "${zai_token}" ]; then
            warn "Project MCP process credentials do not match the missing runtime file; rebuild required (a restart is not enough)."
        else
            warn "Project MCP servers are disabled: no MCP credentials were injected. Copy .env.local.example to .env.local, fill the keys you have, and rebuild the container (a restart is not enough)."
        fi
        return 0
    fi

    for key in GITHUB_PERSONAL_ACCESS_TOKEN ZAI_API_KEY; do
        definition_count="$({ grep -Ec "^${key}=" "${runtime_env}" || true; })"
        file_value=""
        if [ "${definition_count}" = "1" ]; then
            file_value="$(sed -n "s/^${key}=//p" "${runtime_env}")"
            file_value="${file_value%$'\r'}"
        elif [ "${definition_count}" != "0" ]; then
            warn "Project MCP runtime file has duplicate ${key} entries; rebuild required after fixing .env.local."
            return 0
        fi
        process_value="${!key:-}"
        if [ "${file_value}" != "${process_value}" ]; then
            warn "Project MCP runtime file does not match the actual process environment; rebuild required (a restart is not enough)."
            return 0
        fi
    done
    if [ -z "${github_token}" ] && [ -z "${zai_token}" ]; then
        warn "Project MCP servers are disabled: no MCP credentials were injected. Copy .env.local.example to .env.local, fill the keys you have, and rebuild the container (a restart is not enough)."
        return 0
    fi
    if [ -z "${github_token}" ]; then
        missing_groups="${missing_groups} github-mcp-server"
    fi
    if [ -z "${zai_token}" ]; then
        missing_groups="${missing_groups} zai-*"
    fi
    if [ -n "${missing_groups}" ]; then
        warn "Project MCP servers are partially configured; still disabled:${missing_groups}. Fill the missing keys in .env.local and rebuild the container."
    fi
}

log "phase=${HOOK_PHASE}"
if [ "${HOOK_PHASE}" = "post-create" ]; then
    refresh_ai_clis
else
    log "Skipping npm CLI refresh outside post-create."
fi
cleanup_target_if_needed
if [ -f .devcontainer/ai-backends.sh ]; then
    if ! bash .devcontainer/ai-backends.sh install; then
        warn "AI backend launcher setup failed; native CLIs remain available."
    fi
fi
report_mcp_configuration
exit 0
