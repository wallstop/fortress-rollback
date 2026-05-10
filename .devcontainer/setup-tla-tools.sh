#!/bin/bash
# Sync the workspace-local TLA+ tools jar with the repository-pinned version.

set -euo pipefail

PROJECT_ROOT="${1:-$(pwd)}"
TOOLS_DIR="${PROJECT_ROOT}/.tla-tools"
TLA_TOOLS_JAR="${TLA_TOOLS_JAR:-${TOOLS_DIR}/tla2tools.jar}"
VERSION_FILE="${PROJECT_ROOT}/.tla-tools-version"

if [ -z "${TLA_TOOLS_VERSION:-}" ]; then
    if [ ! -f "${VERSION_FILE}" ]; then
        echo "Missing ${VERSION_FILE}; cannot determine TLA+ tools version." >&2
        exit 1
    fi
    TLA_TOOLS_VERSION="$(tr -d '[:space:]' < "${VERSION_FILE}")"
fi

if [ -z "${TLA_TOOLS_VERSION}" ]; then
    echo "TLA+ tools version is empty." >&2
    exit 1
fi

mkdir -p "${TOOLS_DIR}"

workspace_version_file="${TOOLS_DIR}/.version"
if [ -s "${TLA_TOOLS_JAR}" ] \
    && [ -f "${workspace_version_file}" ] \
    && [ "$(cat "${workspace_version_file}")" = "${TLA_TOOLS_VERSION}" ]; then
    echo "TLA+ tools v${TLA_TOOLS_VERSION} already present."
    exit 0
fi

if [ -s /opt/tla/tla2tools.jar ] \
    && [ -f /opt/tla/.version ] \
    && [ "$(cat /opt/tla/.version)" = "${TLA_TOOLS_VERSION}" ]; then
    cp /opt/tla/tla2tools.jar "${TLA_TOOLS_JAR}"
    printf '%s\n' "${TLA_TOOLS_VERSION}" > "${workspace_version_file}"
    echo "Copied TLA+ tools v${TLA_TOOLS_VERSION} from devcontainer image."
    exit 0
fi

echo "Downloading TLA+ tools v${TLA_TOOLS_VERSION}..."
curl --proto "=https" --tlsv1.2 -fsSL --retry 5 --retry-delay 2 --retry-all-errors --max-time 120 \
    "https://github.com/tlaplus/tlaplus/releases/download/v${TLA_TOOLS_VERSION}/tla2tools.jar" \
    -o "${TLA_TOOLS_JAR}"

if [ ! -s "${TLA_TOOLS_JAR}" ]; then
    echo "Downloaded ${TLA_TOOLS_JAR} is empty." >&2
    rm -f "${TLA_TOOLS_JAR}"
    exit 1
fi

printf '%s\n' "${TLA_TOOLS_VERSION}" > "${workspace_version_file}"
echo "Downloaded TLA+ tools v${TLA_TOOLS_VERSION}."
