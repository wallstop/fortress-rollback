#!/bin/bash
# Decide whether a PR / merge-queue candidate can affect the cfg(kani) tree.
#
# The Kani compile gate is only meaningful when Rust sources or build inputs
# changed. This script compares the merge candidate against the base SHA and
# writes `kani=true|false` to $GITHUB_OUTPUT. Every detection failure fails
# closed: a missing/unfetchable base runs the gate rather than skipping it.
#
# Environment:
#   BASE_SHA     Base commit to diff against. Empty => run the gate.
#   GITHUB_OUTPUT  File receiving the `kani=` output (required).
#
# The file list here is the single authority for the gate's path filter; the
# workflow only invokes this script.

set -euo pipefail
IFS=$' \t\n'

base_sha="${BASE_SHA:-}"
output="${GITHUB_OUTPUT:?GITHUB_OUTPUT must be set}"

emit() {
    printf 'kani=%s\n' "$1" >> "$output"
}

if [[ -z "$base_sha" ]]; then
    emit true
    exit 0
fi

if ! git fetch --no-tags --depth=1 origin "$base_sha"; then
    echo "::warning::Could not fetch base $base_sha; running the Kani compile gate defensively."
    emit true
    exit 0
fi

# Nested Cargo.toml files cover workspace members; cargo kani builds the root
# package but member manifests can change feature unification and build scripts.
if git diff --quiet FETCH_HEAD HEAD -- \
    'src/**' \
    'Cargo.toml' \
    '**/Cargo.toml' \
    'Cargo.lock' \
    'build.rs' \
    'rust-toolchain.toml' \
    '.cargo/config.toml' \
    '.kani-version' \
    '.github/actions/setup-kani/**' \
    '.github/actions/install-pinned-nightly/**' \
    '.github/workflows/ci-pr.yml'; then
    emit false
else
    emit true
fi
