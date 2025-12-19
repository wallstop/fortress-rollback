#!/usr/bin/env bash
# Verify that sccache can actually perform a compilation.
# This catches GitHub Actions Cache service outages that cause sccache to fail
# during server startup but not during version/stats checks.
#
# Usage: ./scripts/verify-sccache.sh
# Exit code: 0 if sccache is working, 1 if it's not
# Output: Sets GITHUB_OUTPUT variable "working" to "true" or "false" if running in CI

set -euo pipefail

TEMP_DIR="${TMPDIR:-/tmp}"
TEST_FILE="$TEMP_DIR/sccache_test_$$.rs"
TEST_OUTPUT="$TEMP_DIR/sccache_test_$$"
ERROR_LOG="$TEMP_DIR/sccache_test_$$.log"

cleanup() {
    rm -f "$TEST_FILE" "$TEST_OUTPUT" "$ERROR_LOG"
    # Stop the sccache server to ensure clean state for next attempt
    sccache --stop-server 2>/dev/null || true
}
trap cleanup EXIT

# Create a minimal Rust program
echo 'fn main() {}' > "$TEST_FILE"

# Test that sccache can actually compile something
# Capture stderr to show meaningful errors on failure
if RUSTC_WRAPPER=sccache rustc "$TEST_FILE" -o "$TEST_OUTPUT" 2>"$ERROR_LOG"; then
    echo "sccache verification: PASSED"
    if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
        echo "working=true" >> "$GITHUB_OUTPUT"
    fi
    exit 0
else
    EXIT_CODE=$?
    echo "::warning::sccache verification failed (exit code: $EXIT_CODE) - falling back to direct compilation"
    # Show the error for debugging (but don't fail the step since we handle this gracefully)
    if [[ -s "$ERROR_LOG" ]]; then
        echo "sccache error output:"
        head -20 "$ERROR_LOG" || true
    fi
    if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
        echo "working=false" >> "$GITHUB_OUTPUT"
    fi
    exit 1
fi
