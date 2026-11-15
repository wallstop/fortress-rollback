#!/bin/bash
# Remove local generated trees that slow down scanners such as doc-claims.
#
# Default is a dry run. Pass --yes to delete. Tracked files are never touched.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CONFIRM=0

if [[ "${1:-}" == "--yes" ]]; then
    CONFIRM=1
elif [[ -n "${1:-}" ]]; then
    echo "Usage: $0 [--yes]" >&2
    exit 2
fi

ARTIFACTS=(
    "target"
    "tests/godot-emscripten/target"
    "tests/godot-emscripten/target-threads"
    "tests/godot-emscripten/target-nothreads"
    "tests/godot-emscripten/dist"
    "fuzz/target"
    "specs/tla/states"
    ".tla-tools"
    "site"
)

for relative in "${ARTIFACTS[@]}"; do
    path="$REPO_ROOT/$relative"
    if [[ ! -e "$path" ]]; then
        continue
    fi
    size="$(du -sh "$path" 2>/dev/null | cut -f1 || echo "?")"
    if [[ "$CONFIRM" -eq 1 ]]; then
        echo "Removing $relative ($size)"
        rm -rf -- "$path"
    else
        echo "Would remove $relative ($size)"
    fi
done

if [[ "$CONFIRM" -eq 0 ]]; then
    echo ""
    echo "Dry run only. Re-run with --yes to delete."
fi
