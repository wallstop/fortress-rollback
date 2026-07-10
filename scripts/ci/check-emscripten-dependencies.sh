#!/usr/bin/env bash
# Reject browser-only JS bridge crates from Fortress's Emscripten dependency graph.

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
readonly MANIFEST_PATH="${1:-${REPO_ROOT}/Cargo.toml}"
readonly TARGET="wasm32-unknown-emscripten"
readonly FEATURES="sync-send,paranoid,json,hot-join"

if [[ ! -f "${MANIFEST_PATH}" ]]; then
    printf 'error: Cargo manifest not found: %s\n' "${MANIFEST_PATH}" >&2
    exit 2
fi

tree_output="$("${CARGO:-cargo}" tree \
    --manifest-path "${MANIFEST_PATH}" \
    --package fortress-rollback \
    --target "${TARGET}" \
    --edges normal \
    --prefix none \
    --format '{p}' \
    --locked \
    --no-default-features \
    --features "${FEATURES}")"

offenders="$(printf '%s\n' "${tree_output}" | awk '
    $1 == "js-sys" || $1 == "web-sys" || $1 ~ /^wasm-bindgen($|-)/ { print }
' | sort -u)"

if [[ -n "${offenders}" ]]; then
    {
        printf 'error: browser-only JS bridge crates selected for %s:\n' "${TARGET}"
        printf '%s\n' "${offenders}"
        printf '\n'
        printf 'The Emscripten normal dependency graph must not contain js-sys, web-sys,\n'
        printf 'or wasm-bindgen packages. These imports target wasm32-unknown-unknown and\n'
        printf 'panic when called from a Godot Rust GDExtension.\n'
        printf 'Checked features: %s\n' "${FEATURES}"
    } >&2
    exit 1
fi

printf 'Emscripten dependency graph is free of browser-only JS bridge crates.\n'
