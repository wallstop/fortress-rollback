#!/usr/bin/env bash

set -euo pipefail

readonly FIXTURE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly MANIFEST_PATH="${FIXTURE_ROOT}/Cargo.toml"
readonly GODOT_PROJECT="${FIXTURE_ROOT}/godot"
readonly GODOT_BIN_DIR="${GODOT_PROJECT}/bin"
readonly DIST_DIR="${FIXTURE_ROOT}/dist"
readonly NIGHTLY="nightly-2026-07-08"
readonly TARGET="wasm32-unknown-emscripten"
readonly LIBRARY_NAME="fortress_godot_probe"
readonly GODOT_BIN="${GODOT4_BIN:-godot4}"
readonly GODOT_VERSION="4.6.3.stable.official.7d41c59c4"
readonly COMMON_RUSTFLAGS="-C panic=abort -C link-args=-sSIDE_MODULE=2 -C llvm-args=-enable-emscripten-cxx-exceptions=0 -Z default-visibility=hidden -Z link-native-libraries=no"

if ! command -v emcc >/dev/null 2>&1; then
    if [[ -n "${EMSDK:-}" && -f "${EMSDK}/emsdk_env.sh" ]]; then
        # shellcheck disable=SC1090
        source "${EMSDK}/emsdk_env.sh" >/dev/null
    else
        printf 'error: emcc is not on PATH and EMSDK is not configured\n' >&2
        exit 1
    fi
fi
if [[ ! -x "${GODOT_BIN}" ]] && ! command -v "${GODOT_BIN}" >/dev/null 2>&1; then
    printf 'error: Godot executable not found: %s\n' "${GODOT_BIN}" >&2
    exit 1
fi
if [[ "$("${GODOT_BIN}" --version)" != "${GODOT_VERSION}" ]]; then
    printf 'error: Godot %s is required\n' "${GODOT_VERSION}" >&2
    "${GODOT_BIN}" --version >&2
    exit 1
fi
if ! emcc --version | head -n 1 | grep -F '4.0.11' >/dev/null; then
    printf 'error: Emscripten 4.0.11 is required\n' >&2
    emcc --version >&2
    exit 1
fi

rm -rf "${GODOT_BIN_DIR}" "${DIST_DIR}"
mkdir -p "${GODOT_BIN_DIR}" "${DIST_DIR}/threaded" "${DIST_DIR}/nothreads"

(
    unset CARGO_ENCODED_RUSTFLAGS
    export CARGO_TARGET_DIR="${FIXTURE_ROOT}/target-threads"
    export RUSTFLAGS="${COMMON_RUSTFLAGS} -C target-feature=+atomics -C link-arg=-pthread -C link-arg=-Wno-experimental -A unstable-features"
    cargo +"${NIGHTLY}" build \
        --manifest-path "${MANIFEST_PATH}" \
        --locked \
        -Z build-std=std,panic_abort \
        --target "${TARGET}"
)
cp "${FIXTURE_ROOT}/target-threads/${TARGET}/debug/${LIBRARY_NAME}.wasm" \
    "${GODOT_BIN_DIR}/${LIBRARY_NAME}.threads.wasm"

(
    unset CARGO_ENCODED_RUSTFLAGS
    export CARGO_TARGET_DIR="${FIXTURE_ROOT}/target-nothreads"
    export RUSTFLAGS="${COMMON_RUSTFLAGS}"
    cargo +"${NIGHTLY}" build \
        --manifest-path "${MANIFEST_PATH}" \
        --locked \
        -Z build-std=std,panic_abort \
        --target "${TARGET}" \
        --features nothreads
)
cp "${FIXTURE_ROOT}/target-nothreads/${TARGET}/debug/${LIBRARY_NAME}.wasm" \
    "${GODOT_BIN_DIR}/${LIBRARY_NAME}.wasm"

"${GODOT_BIN}" --headless --path "${GODOT_PROJECT}" \
    --export-debug "Web Threaded" "${DIST_DIR}/threaded/index.html"
"${GODOT_BIN}" --headless --path "${GODOT_PROJECT}" \
    --export-debug "Web Non-Threaded" "${DIST_DIR}/nothreads/index.html"

cd "${FIXTURE_ROOT}"
npm ci
if [[ "${INSTALL_PLAYWRIGHT:-0}" == "1" ]]; then
    npx --no-install playwright install --with-deps chromium
fi
if ! command -v xvfb-run >/dev/null 2>&1; then
    printf 'error: xvfb-run is required for Godot WebGL2 browser tests\n' >&2
    exit 1
fi
xvfb-run -a npm test
