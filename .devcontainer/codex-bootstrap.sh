#!/bin/bash
# Codex CLI readiness helper for devcontainer lifecycle hooks.
# This script is intentionally non-blocking so startup keeps working even
# when authentication is not configured yet.

set -euo pipefail
trap 'echo "[codex-bootstrap] Unexpected error at line ${LINENO}; continuing startup." >&2; exit 0' ERR

HOOK_PHASE="${1:-post-start}"
CODEX_HOME_DIR="${CODEX_HOME:-$HOME/.codex}"

print_login_guidance() {
    echo "[codex-bootstrap] Login options:"
    echo "[codex-bootstrap]   codex login"
    echo "[codex-bootstrap]   codex login --device-auth"
    echo "[codex-bootstrap]   printenv OPENAI_API_KEY | codex login --with-api-key"
    echo "[codex-bootstrap] For non-interactive one-shot runs, set CODEX_API_KEY and run codex exec ..."
}

ensure_codex_home_writable() {
    if [ -w "${CODEX_HOME_DIR}" ]; then
        return 0
    fi

    CURRENT_OWNER="$(stat -c '%U:%G' "${CODEX_HOME_DIR}" 2>/dev/null || echo 'unknown')"
    echo "[codex-bootstrap] ${CODEX_HOME_DIR} is not writable (owner ${CURRENT_OWNER}); attempting permission repair."

    if command -v sudo >/dev/null 2>&1; then
        if sudo -n chown -R "$(id -un):$(id -gn)" "${CODEX_HOME_DIR}" >/dev/null 2>&1; then
            chmod 700 "${CODEX_HOME_DIR}" >/dev/null 2>&1 || true
        else
            echo "[codex-bootstrap] Non-interactive sudo permission repair was unavailable." >&2
        fi
    fi

    if [ -w "${CODEX_HOME_DIR}" ]; then
        echo "[codex-bootstrap] Repaired Codex home permissions."
        return 0
    fi

    echo "[codex-bootstrap] Could not repair ${CODEX_HOME_DIR}; run: sudo chown -R $(id -un):$(id -gn) ${CODEX_HOME_DIR}" >&2
    return 0
}

echo "[codex-bootstrap] phase=${HOOK_PHASE}"

if ! command -v codex >/dev/null 2>&1; then
    echo "[codex-bootstrap] Codex CLI is not on PATH. Rebuild the devcontainer image if this persists."
    exit 0
fi

mkdir -p "${CODEX_HOME_DIR}"
ensure_codex_home_writable

if CODEX_VERSION="$(codex --version 2>/dev/null)"; then
    echo "[codex-bootstrap] ${CODEX_VERSION}"
else
    echo "[codex-bootstrap] Codex CLI found but version check failed."
fi

LOGIN_OUTPUT=""
if LOGIN_OUTPUT="$(codex login status 2>&1)"; then
    echo "[codex-bootstrap] Authentication is already configured."
    exit 0
fi

if ! printf '%s' "${LOGIN_OUTPUT}" | grep -qi "not logged in"; then
    FIRST_LOGIN_LINE="${LOGIN_OUTPUT%%$'\n'*}"
    echo "[codex-bootstrap] Authentication check returned an unexpected result: ${FIRST_LOGIN_LINE}" >&2
fi

if [ -n "${OPENAI_API_KEY:-}" ] || [ -n "${CODEX_API_KEY:-}" ]; then
    if codex login --with-api-key <<<"${OPENAI_API_KEY:-${CODEX_API_KEY:-}}" >/dev/null 2>&1; then
        echo "[codex-bootstrap] Logged in using API key from environment."
        exit 0
    fi
    echo "[codex-bootstrap] API-key login attempt failed."
fi

echo "[codex-bootstrap] Authentication is not configured yet."
print_login_guidance
exit 0
