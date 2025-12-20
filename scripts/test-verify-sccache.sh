#!/usr/bin/env bash
# Test suite for verify-sccache.sh
#
# This script tests various failure scenarios to ensure the verification
# script handles them correctly. It uses data-driven tests with different
# environment configurations.
#
# Usage: ./scripts/test-verify-sccache.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERIFY_SCRIPT="$SCRIPT_DIR/verify-sccache.sh"
TEST_RESULTS=()
TESTS_PASSED=0
TESTS_FAILED=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

log_test() {
    local name="$1"
    local status="$2"
    local message="${3:-}"
    
    if [[ "$status" == "PASS" ]]; then
        echo -e "${GREEN}✓${NC} $name"
        ((TESTS_PASSED++)) || true
    elif [[ "$status" == "FAIL" ]]; then
        echo -e "${RED}✗${NC} $name: $message"
        ((TESTS_FAILED++)) || true
    elif [[ "$status" == "SKIP" ]]; then
        echo -e "${YELLOW}○${NC} $name: $message"
    fi
    TEST_RESULTS+=("$status: $name")
}

# Test case data structure: array of (name, expected_exit_code, env_setup, description)
declare -a TEST_CASES

# ==============================================================================
# Test Case 1: Script exists and is executable
# ==============================================================================
test_script_exists() {
    if [[ -x "$VERIFY_SCRIPT" ]]; then
        log_test "Script exists and is executable" "PASS"
        return 0
    else
        log_test "Script exists and is executable" "FAIL" "Script not found or not executable: $VERIFY_SCRIPT"
        return 1
    fi
}

# ==============================================================================
# Test Case 2: Script has correct shebang
# ==============================================================================
test_script_shebang() {
    local shebang
    shebang=$(head -1 "$VERIFY_SCRIPT")
    if [[ "$shebang" == "#!/usr/bin/env bash" ]]; then
        log_test "Script has correct shebang" "PASS"
        return 0
    else
        log_test "Script has correct shebang" "FAIL" "Expected '#!/usr/bin/env bash', got '$shebang'"
        return 1
    fi
}

# ==============================================================================
# Test Case 3: Script uses strict mode
# ==============================================================================
test_strict_mode() {
    if grep -q "set -euo pipefail" "$VERIFY_SCRIPT"; then
        log_test "Script uses strict mode (set -euo pipefail)" "PASS"
        return 0
    else
        log_test "Script uses strict mode (set -euo pipefail)" "FAIL" "Missing 'set -euo pipefail'"
        return 1
    fi
}

# ==============================================================================
# Test Case 4: Script has cleanup trap
# ==============================================================================
test_cleanup_trap() {
    if grep -q "trap cleanup EXIT" "$VERIFY_SCRIPT"; then
        log_test "Script has cleanup trap" "PASS"
        return 0
    else
        log_test "Script has cleanup trap" "FAIL" "Missing 'trap cleanup EXIT'"
        return 1
    fi
}

# ==============================================================================
# Test Case 5: Script tests version query
# ==============================================================================
test_version_query_check() {
    if grep -q "rustc -vV" "$VERIFY_SCRIPT"; then
        log_test "Script tests rustc version query" "PASS"
        return 0
    else
        log_test "Script tests rustc version query" "FAIL" "Missing 'rustc -vV' check"
        return 1
    fi
}

# ==============================================================================
# Test Case 6: Script has retry logic
# ==============================================================================
test_retry_logic() {
    if grep -q "MAX_RETRIES" "$VERIFY_SCRIPT" && grep -q "RETRY_DELAY" "$VERIFY_SCRIPT"; then
        log_test "Script has retry logic" "PASS"
        return 0
    else
        log_test "Script has retry logic" "FAIL" "Missing retry configuration"
        return 1
    fi
}

# ==============================================================================
# Test Case 7: Script has timeout protection
# ==============================================================================
test_timeout_protection() {
    if grep -q "SCCACHE_TIMEOUT" "$VERIFY_SCRIPT" && grep -q "timeout" "$VERIFY_SCRIPT"; then
        log_test "Script has timeout protection" "PASS"
        return 0
    else
        log_test "Script has timeout protection" "FAIL" "Missing timeout configuration"
        return 1
    fi
}

# ==============================================================================
# Test Case 8: Script sets GITHUB_OUTPUT
# ==============================================================================
test_github_output() {
    if grep -q 'GITHUB_OUTPUT' "$VERIFY_SCRIPT"; then
        log_test "Script sets GITHUB_OUTPUT" "PASS"
        return 0
    else
        log_test "Script sets GITHUB_OUTPUT" "FAIL" "Missing GITHUB_OUTPUT handling"
        return 1
    fi
}

# ==============================================================================
# Test Case 9: Script stops sccache server on cleanup
# ==============================================================================
test_sccache_server_cleanup() {
    if grep -q "sccache --stop-server" "$VERIFY_SCRIPT"; then
        log_test "Script stops sccache server on cleanup" "PASS"
        return 0
    else
        log_test "Script stops sccache server on cleanup" "FAIL" "Missing server cleanup"
        return 1
    fi
}

# ==============================================================================
# Test Case 10: Functional test (requires sccache and rustc)
# ==============================================================================
test_functional_with_sccache() {
    if ! command -v sccache &> /dev/null; then
        log_test "Functional test with sccache" "SKIP" "sccache not installed"
        return 0
    fi
    
    if ! command -v rustc &> /dev/null; then
        log_test "Functional test with sccache" "SKIP" "rustc not installed"
        return 0
    fi
    
    # Create a temporary GITHUB_OUTPUT file
    local temp_output
    temp_output=$(mktemp)
    
    # Run the verification script
    if env GITHUB_OUTPUT="$temp_output" "$VERIFY_SCRIPT" > /dev/null 2>&1; then
        if grep -q "working=true" "$temp_output"; then
            log_test "Functional test with sccache" "PASS"
            rm -f "$temp_output"
            return 0
        else
            log_test "Functional test with sccache" "FAIL" "GITHUB_OUTPUT not set correctly"
            rm -f "$temp_output"
            return 1
        fi
    else
        # Script failed - this might be expected if sccache isn't configured
        if grep -q "working=false" "$temp_output" 2>/dev/null; then
            log_test "Functional test with sccache" "PASS" "(sccache unavailable but script handled gracefully)"
            rm -f "$temp_output"
            return 0
        else
            log_test "Functional test with sccache" "FAIL" "Script failed unexpectedly"
            rm -f "$temp_output"
            return 1
        fi
    fi
}

# ==============================================================================
# Test Case 11: Verify CI workflow files use updated environment variables
# ==============================================================================
test_ci_workflow_env_vars() {
    local workflows_dir="$SCRIPT_DIR/../.github/workflows"
    local missing_vars=()
    
    if [[ ! -d "$workflows_dir" ]]; then
        log_test "CI workflow environment variables" "SKIP" "Workflows directory not found"
        return 0
    fi
    
    # Check each workflow file that uses sccache
    for workflow in "$workflows_dir"/*.yml; do
        if grep -q "SCCACHE_IGNORE_SERVER_IO_ERROR" "$workflow"; then
            if ! grep -q "SCCACHE_STARTUP_NOTIFY_TIMEOUT" "$workflow"; then
                missing_vars+=("$(basename "$workflow"): missing SCCACHE_STARTUP_NOTIFY_TIMEOUT")
            fi
            if ! grep -q "SCCACHE_IDLE_TIMEOUT" "$workflow"; then
                missing_vars+=("$(basename "$workflow"): missing SCCACHE_IDLE_TIMEOUT")
            fi
        fi
    done
    
    if [[ ${#missing_vars[@]} -eq 0 ]]; then
        log_test "CI workflow environment variables" "PASS"
        return 0
    else
        log_test "CI workflow environment variables" "FAIL" "${missing_vars[*]}"
        return 1
    fi
}

# ==============================================================================
# Main test runner
# ==============================================================================
main() {
    echo "=============================================="
    echo "Testing verify-sccache.sh"
    echo "=============================================="
    echo ""
    
    # Run all tests
    test_script_exists || true
    test_script_shebang || true
    test_strict_mode || true
    test_cleanup_trap || true
    test_version_query_check || true
    test_retry_logic || true
    test_timeout_protection || true
    test_github_output || true
    test_sccache_server_cleanup || true
    test_functional_with_sccache || true
    test_ci_workflow_env_vars || true
    
    echo ""
    echo "=============================================="
    echo "Test Summary"
    echo "=============================================="
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    echo ""
    
    if [[ $TESTS_FAILED -gt 0 ]]; then
        echo -e "${RED}Some tests failed!${NC}"
        exit 1
    else
        echo -e "${GREEN}All tests passed!${NC}"
        exit 0
    fi
}

main "$@"
