#!/usr/bin/env bash
# Verify that sccache can actually perform a compilation.
# This catches GitHub Actions Cache service outages that cause sccache to fail
# during server startup but not during version/stats checks.
#
# The script performs multiple verification stages:
# 1. Test rustc version query (catches "rustc -vV" timeout issues)
# 2. Test actual compilation
# 3. Retry both tests to catch intermittent failures
#
# Usage: ./scripts/verify-sccache.sh
# Exit code: 0 if sccache is working, 1 if it's not
# Output: Sets GITHUB_OUTPUT variable "working" to "true" or "false" if running in CI

set -euo pipefail

# Configuration
# Use platform-appropriate temp directory
if [[ "${OSTYPE:-}" == msys* ]] || [[ "${OSTYPE:-}" == cygwin* ]] || [[ -n "${WINDIR:-}" ]]; then
    # Windows: use TEMP or USERPROFILE
    TEMP_DIR="${TEMP:-${USERPROFILE:-/tmp}/AppData/Local/Temp}"
else
    # Unix: use TMPDIR or /tmp
    TEMP_DIR="${TMPDIR:-/tmp}"
fi
TEST_FILE="$TEMP_DIR/sccache_test_$$.rs"
TEST_OUTPUT="$TEMP_DIR/sccache_test_$$"
ERROR_LOG="$TEMP_DIR/sccache_test_$$.log"
VERSION_LOG="$TEMP_DIR/sccache_version_$$.log"
MAX_RETRIES=3
RETRY_DELAY_SECONDS=2
# Timeout for each sccache operation (seconds)
SCCACHE_TIMEOUT=30

# Cross-platform timeout function
# On Unix, uses the 'timeout' command if available
# On Windows (Git Bash), runs without timeout since Windows timeout.exe is interactive
run_with_timeout() {
    local timeout_seconds="$1"
    shift

    # Check if we're on Windows (Git Bash/MSYS)
    if [[ "${OSTYPE:-}" == msys* ]] || [[ "${OSTYPE:-}" == cygwin* ]] || [[ -n "${WINDIR:-}" ]]; then
        # On Windows, just run without timeout (Windows timeout.exe is interactive)
        "$@"
    elif command -v timeout &>/dev/null; then
        # Unix with GNU timeout
        timeout "$timeout_seconds" "$@"
    else
        # Fallback: run without timeout
        "$@"
    fi
}

cleanup() {
    rm -f "$TEST_FILE" "$TEST_OUTPUT" "$ERROR_LOG" "$VERSION_LOG"
    # Stop the sccache server to ensure clean state for next attempt
    sccache --stop-server 2>/dev/null || true
}
trap cleanup EXIT

output_failure() {
    local reason="$1"
    echo "::warning::sccache verification failed ($reason) - falling back to direct compilation"
    if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
        echo "working=false" >> "$GITHUB_OUTPUT"
    fi
    exit 1
}

output_success() {
    echo "sccache verification: PASSED"
    if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
        echo "working=true" >> "$GITHUB_OUTPUT"
    fi
    exit 0
}

# Test 1: Verify sccache can perform version queries
# This catches the exact error: "sccache rustc -vV" timing out
verify_version_query() {
    local attempt=$1
    echo "Attempt $attempt: Testing sccache version query (rustc -vV)..."

    if run_with_timeout "$SCCACHE_TIMEOUT" env RUSTC_WRAPPER=sccache rustc -vV 2>"$VERSION_LOG"; then
        echo "  Version query: OK"
        return 0
    else
        local exit_code=$?
        echo "  Version query: FAILED (exit code: $exit_code)"
        if [[ -s "$VERSION_LOG" ]]; then
            echo "  Error output:"
            head -5 "$VERSION_LOG" | sed 's/^/    /'
        fi
        return 1
    fi
}

# Test 2: Verify sccache can compile a simple program
verify_compilation() {
    local attempt=$1
    echo "Attempt $attempt: Testing sccache compilation..."

    # Create a minimal Rust program
    echo 'fn main() {}' > "$TEST_FILE"

    if run_with_timeout "$SCCACHE_TIMEOUT" env RUSTC_WRAPPER=sccache rustc "$TEST_FILE" -o "$TEST_OUTPUT" 2>"$ERROR_LOG"; then
        echo "  Compilation: OK"
        return 0
    else
        local exit_code=$?
        echo "  Compilation: FAILED (exit code: $exit_code)"
        if [[ -s "$ERROR_LOG" ]]; then
            echo "  Error output:"
            head -10 "$ERROR_LOG" | sed 's/^/    /'
        fi
        return 1
    fi
}

# Main verification loop with retries
main() {
    echo "=== sccache verification ==="
    echo "Max retries: $MAX_RETRIES"
    echo "Timeout per operation: ${SCCACHE_TIMEOUT}s"
    echo ""

    local version_ok=false
    local compile_ok=false

    # Use bash arithmetic instead of seq for better portability
    for ((attempt = 1; attempt <= MAX_RETRIES; attempt++)); do
        if [[ $attempt -gt 1 ]]; then
            echo ""
            echo "Retrying after ${RETRY_DELAY_SECONDS}s delay..."
            sleep "$RETRY_DELAY_SECONDS"
            # Restart sccache server between retries
            sccache --stop-server 2>/dev/null || true
        fi

        # Test version query
        if verify_version_query "$attempt"; then
            version_ok=true
        else
            version_ok=false
            continue  # Try again from the beginning
        fi

        # Test compilation
        if verify_compilation "$attempt"; then
            compile_ok=true
        else
            compile_ok=false
            continue  # Try again from the beginning
        fi

        # Both tests passed
        if $version_ok && $compile_ok; then
            echo ""
            output_success
        fi
    done

    # All retries exhausted
    echo ""
    echo "All $MAX_RETRIES attempts failed."

    if ! $version_ok; then
        output_failure "version query failed after $MAX_RETRIES attempts"
    elif ! $compile_ok; then
        output_failure "compilation failed after $MAX_RETRIES attempts"
    else
        output_failure "unknown failure after $MAX_RETRIES attempts"
    fi
}

main
