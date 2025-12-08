#!/bin/bash
# Kani Formal Verification Script for Fortress Rollback
#
# Usage: ./scripts/verify-kani.sh [options]
#   ./scripts/verify-kani.sh              # Run all Kani proofs
#   ./scripts/verify-kani.sh --list       # List all proof harnesses
#   ./scripts/verify-kani.sh --harness X  # Run specific harness
#   ./scripts/verify-kani.sh --quick      # Run with reduced unwind bounds
#
# Prerequisites:
#   - Kani installed (script provides install instructions if missing)
#   - Rust nightly toolchain
#
# Environment Variables:
#   KANI_TIMEOUT     - Timeout per proof in seconds (default: 300)
#   KANI_UNWIND      - Default unwind bound (default: use Kani defaults)

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
KANI_TIMEOUT="${KANI_TIMEOUT:-300}"
KANI_UNWIND="${KANI_UNWIND:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Track results
PASSED=0
FAILED=0
TOTAL=0

print_usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --list          List all Kani proof harnesses"
    echo "  --harness NAME  Run specific harness (can be repeated)"
    echo "  --quick         Run with reduced bounds for faster verification"
    echo "  --verbose       Show detailed Kani output"
    echo "  --help          Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  KANI_TIMEOUT    Timeout per proof in seconds (default: 300)"
    echo ""
    echo "Examples:"
    echo "  $0                              # Run all proofs"
    echo "  $0 --harness kani_proof_frame_new  # Run single proof"
    echo "  $0 --list                       # List available proofs"
}

check_kani() {
    if ! command -v cargo-kani &> /dev/null; then
        echo -e "${RED}Error: Kani is not installed.${NC}"
        echo ""
        echo "To install Kani, run:"
        echo "  cargo install --locked kani-verifier"
        echo "  cargo kani setup"
        echo ""
        echo "For more information, see: https://model-checking.github.io/kani/install-guide.html"
        exit 1
    fi
    
    local kani_version
    kani_version=$(cargo kani --version 2>&1 | head -n 1 || echo "unknown")
    echo -e "${BLUE}Using Kani: $kani_version${NC}"
}

list_harnesses() {
    echo "Kani Proof Harnesses in Fortress Rollback:"
    echo ""
    
    cd "$PROJECT_ROOT"
    
    # Find all kani_proof functions
    local harnesses
    harnesses=$(grep -rh '#\[kani::proof\]' --include='*.rs' -A 1 src/ 2>/dev/null | \
                grep -oP 'fn \K[a-zA-Z_][a-zA-Z0-9_]*' | sort -u || echo "")
    
    if [[ -z "$harnesses" ]]; then
        echo "  No Kani proof harnesses found."
        echo ""
        echo "  Harnesses are defined with #[kani::proof] attribute."
        return 1
    fi
    
    local count=0
    while IFS= read -r harness; do
        echo "  - $harness"
        ((count++))
    done <<< "$harnesses"
    
    echo ""
    echo "Total: $count harnesses"
}

run_kani() {
    local harness="${1:-}"
    local quick="${2:-false}"
    local verbose="${3:-false}"
    
    cd "$PROJECT_ROOT"
    
    echo -e "${BLUE}Running Kani verification...${NC}"
    echo ""
    
    # Build Kani command
    local kani_cmd=(cargo kani)
    
    # Add harness filter if specified
    if [[ -n "$harness" ]]; then
        kani_cmd+=(--harness "$harness")
    fi
    
    # Add unwind if set
    if [[ -n "$KANI_UNWIND" ]]; then
        kani_cmd+=(--default-unwind "$KANI_UNWIND")
    fi
    
    # Quick mode uses smaller bounds
    if [[ "$quick" == "true" ]]; then
        kani_cmd+=(--default-unwind 12)
    fi
    
    local start_time
    start_time=$(date +%s)
    
    local output_file
    output_file=$(mktemp)
    
    local exit_code=0
    if [[ "$verbose" == "true" ]]; then
        "${kani_cmd[@]}" 2>&1 | tee "$output_file" || exit_code=$?
    else
        "${kani_cmd[@]}" > "$output_file" 2>&1 || exit_code=$?
    fi
    
    local end_time
    end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    # Parse results - look for summary line which has the format:
    # Complete - N successfully verified harnesses, M failures, T total.
    local summary_line
    summary_line=$(grep "Complete - " "$output_file" 2>/dev/null | tail -1 || echo "")
    
    if [[ -n "$summary_line" ]]; then
        PASSED=$(echo "$summary_line" | grep -oP '\d+ successfully' | grep -oP '\d+' || echo "0")
        FAILED=$(echo "$summary_line" | grep -oP '\d+ failures' | grep -oP '\d+' || echo "0")
        TOTAL=$(echo "$summary_line" | grep -oP '\d+ total' | grep -oP '\d+' || echo "0")
    else
        # Fallback to counting individual results
        PASSED=$(grep -c "VERIFICATION:- SUCCESSFUL" "$output_file" 2>/dev/null || echo "0")
        FAILED=$(grep -c "VERIFICATION:- FAILED" "$output_file" 2>/dev/null || echo "0")
        TOTAL=$((PASSED + FAILED))
    fi
    
    # Show summary
    echo ""
    echo "=========================================="
    echo "Kani Verification Summary"
    echo "=========================================="
    echo "  Duration: ${duration}s"
    echo -e "  ${GREEN}Passed: $PASSED${NC}"
    echo -e "  ${RED}Failed: $FAILED${NC}"
    echo "  Total:  $TOTAL"
    echo "=========================================="
    
    if [[ $FAILED -gt 0 ]]; then
        echo ""
        echo -e "${RED}Failed proofs:${NC}"
        grep -B 5 "VERIFICATION:- FAILED" "$output_file" | grep "Checking harness" || true
        
        if [[ "$verbose" != "true" ]]; then
            echo ""
            echo "Run with --verbose for detailed output"
        fi
    fi
    
    rm -f "$output_file"
    
    if [[ $exit_code -ne 0 ]] || [[ $FAILED -gt 0 ]]; then
        return 1
    fi
    
    return 0
}

main() {
    local quick=false
    local verbose=false
    local list_only=false
    local harnesses=()
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --quick)
                quick=true
                shift
                ;;
            --verbose)
                verbose=true
                shift
                ;;
            --list)
                list_only=true
                shift
                ;;
            --harness)
                if [[ $# -lt 2 ]]; then
                    echo "Error: --harness requires an argument"
                    exit 1
                fi
                harnesses+=("$2")
                shift 2
                ;;
            --help|-h)
                print_usage
                exit 0
                ;;
            -*)
                echo "Unknown option: $1"
                print_usage
                exit 1
                ;;
            *)
                # Treat as harness name
                harnesses+=("$1")
                shift
                ;;
        esac
    done
    
    echo "=========================================="
    echo "Fortress Rollback Kani Verification"
    echo "=========================================="
    
    check_kani
    echo ""
    
    if [[ "$list_only" == "true" ]]; then
        list_harnesses
        exit 0
    fi
    
    # Run verification
    local any_failed=false
    
    if [[ ${#harnesses[@]} -gt 0 ]]; then
        # Run specific harnesses
        for harness in "${harnesses[@]}"; do
            echo -e "${BLUE}Verifying harness: $harness${NC}"
            if ! run_kani "$harness" "$quick" "$verbose"; then
                any_failed=true
            fi
        done
    else
        # Run all harnesses
        if ! run_kani "" "$quick" "$verbose"; then
            any_failed=true
        fi
    fi
    
    if [[ "$any_failed" == "true" ]]; then
        exit 1
    fi
    
    echo ""
    echo -e "${GREEN}All Kani proofs passed!${NC}"
}

main "$@"
