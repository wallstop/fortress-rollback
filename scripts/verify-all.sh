#!/bin/bash
# Unified Formal Verification Script for Fortress Rollback
#
# Usage: ./scripts/verify-all.sh [options]
#   ./scripts/verify-all.sh              # Run all verifiers
#   ./scripts/verify-all.sh --tla        # TLA+ only
#   ./scripts/verify-all.sh --kani       # Kani only
#   ./scripts/verify-all.sh --quick      # Fast subset for dev iteration
#   ./scripts/verify-all.sh --parallel   # Run TLA+ and Kani in parallel
#
# This script orchestrates all formal verification tools:
#   - TLA+ model checking (specs/tla/*.tla)
#   - Kani bounded model checking (Rust proofs)
#   - Z3 SMT solving (future)
#
# Prerequisites:
#   - Java 11+ (for TLA+)
#   - Kani (cargo install kani-verifier)
#   - Z3 (future, for SMT proofs)

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Tool flags
RUN_TLA=true
RUN_KANI=true
RUN_Z3=false  # Not yet implemented
QUICK_MODE=false
PARALLEL=false
VERBOSE=false

# Results tracking
TLA_RESULT=""
KANI_RESULT=""
Z3_RESULT=""

print_usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --tla       Run TLA+ verification only"
    echo "  --kani      Run Kani verification only"
    echo "  --z3        Run Z3 verification only (not yet implemented)"
    echo "  --quick     Use reduced bounds for faster verification"
    echo "  --parallel  Run verifiers in parallel (faster but more output)"
    echo "  --verbose   Show detailed output from all tools"
    echo "  --help      Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                  # Run all verifiers sequentially"
    echo "  $0 --quick          # Quick verification for development"
    echo "  $0 --tla --verbose  # TLA+ only with detailed output"
    echo "  $0 --parallel       # Run all verifiers in parallel"
}

print_header() {
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║           Fortress Rollback Formal Verification                ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

run_tla() {
    echo -e "${BLUE}┌────────────────────────────────────────────────────────────────┐${NC}"
    echo -e "${BLUE}│  TLA+ Model Checking                                           │${NC}"
    echo -e "${BLUE}└────────────────────────────────────────────────────────────────┘${NC}"
    
    local tla_args=()
    [[ "$QUICK_MODE" == "true" ]] && tla_args+=(--quick)
    [[ "$VERBOSE" == "true" ]] && tla_args+=(--verbose)
    
    local start_time
    start_time=$(date +%s)
    
    if "$SCRIPT_DIR/verify-tla.sh" "${tla_args[@]}"; then
        TLA_RESULT="PASS"
    else
        TLA_RESULT="FAIL"
    fi
    
    local end_time
    end_time=$(date +%s)
    echo "TLA+ verification completed in $((end_time - start_time))s"
    echo ""
}

run_kani() {
    echo -e "${BLUE}┌────────────────────────────────────────────────────────────────┐${NC}"
    echo -e "${BLUE}│  Kani Bounded Model Checking                                   │${NC}"
    echo -e "${BLUE}└────────────────────────────────────────────────────────────────┘${NC}"
    
    local kani_args=()
    [[ "$QUICK_MODE" == "true" ]] && kani_args+=(--quick)
    [[ "$VERBOSE" == "true" ]] && kani_args+=(--verbose)
    
    local start_time
    start_time=$(date +%s)
    
    if "$SCRIPT_DIR/verify-kani.sh" "${kani_args[@]}"; then
        KANI_RESULT="PASS"
    else
        KANI_RESULT="FAIL"
    fi
    
    local end_time
    end_time=$(date +%s)
    echo "Kani verification completed in $((end_time - start_time))s"
    echo ""
}

run_z3() {
    echo -e "${BLUE}┌────────────────────────────────────────────────────────────────┐${NC}"
    echo -e "${BLUE}│  Z3 SMT Verification                                           │${NC}"
    echo -e "${BLUE}└────────────────────────────────────────────────────────────────┘${NC}"
    
    echo -e "${YELLOW}Z3 verification not yet implemented.${NC}"
    echo "See Phase 2.5 in PLAN.md for planned Z3 verification targets."
    Z3_RESULT="SKIP"
    echo ""
}

print_summary() {
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║                   Verification Summary                         ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    local all_passed=true
    
    # TLA+ result
    if [[ -n "$TLA_RESULT" ]]; then
        local tla_color="$NC"
        local tla_symbol="?"
        case "$TLA_RESULT" in
            PASS) tla_color="$GREEN"; tla_symbol="✓" ;;
            FAIL) tla_color="$RED"; tla_symbol="✗"; all_passed=false ;;
            SKIP) tla_color="$YELLOW"; tla_symbol="-" ;;
        esac
        echo -e "  $tla_symbol TLA+ Model Checking:    ${tla_color}$TLA_RESULT${NC}"
    fi
    
    # Kani result
    if [[ -n "$KANI_RESULT" ]]; then
        local kani_color="$NC"
        local kani_symbol="?"
        case "$KANI_RESULT" in
            PASS) kani_color="$GREEN"; kani_symbol="✓" ;;
            FAIL) kani_color="$RED"; kani_symbol="✗"; all_passed=false ;;
            SKIP) kani_color="$YELLOW"; kani_symbol="-" ;;
        esac
        echo -e "  $kani_symbol Kani Bounded Checking: ${kani_color}$KANI_RESULT${NC}"
    fi
    
    # Z3 result
    if [[ -n "$Z3_RESULT" ]]; then
        local z3_color="$NC"
        local z3_symbol="?"
        case "$Z3_RESULT" in
            PASS) z3_color="$GREEN"; z3_symbol="✓" ;;
            FAIL) z3_color="$RED"; z3_symbol="✗"; all_passed=false ;;
            SKIP) z3_color="$YELLOW"; z3_symbol="-" ;;
        esac
        echo -e "  $z3_symbol Z3 SMT Verification:    ${z3_color}$Z3_RESULT${NC}"
    fi
    
    echo ""
    
    if [[ "$all_passed" == "true" ]]; then
        echo -e "${GREEN}╔════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${GREEN}║           ✓ All Formal Verification Passed!                    ║${NC}"
        echo -e "${GREEN}╚════════════════════════════════════════════════════════════════╝${NC}"
        return 0
    else
        echo -e "${RED}╔════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${RED}║           ✗ Some Verification Failed                           ║${NC}"
        echo -e "${RED}╚════════════════════════════════════════════════════════════════╝${NC}"
        return 1
    fi
}

main() {
    # Parse arguments
    local explicit_tool=false
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --tla)
                RUN_TLA=true
                RUN_KANI=false
                RUN_Z3=false
                explicit_tool=true
                shift
                ;;
            --kani)
                RUN_TLA=false
                RUN_KANI=true
                RUN_Z3=false
                explicit_tool=true
                shift
                ;;
            --z3)
                RUN_TLA=false
                RUN_KANI=false
                RUN_Z3=true
                explicit_tool=true
                shift
                ;;
            --quick)
                QUICK_MODE=true
                shift
                ;;
            --parallel)
                PARALLEL=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
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
                echo "Unknown argument: $1"
                print_usage
                exit 1
                ;;
        esac
    done
    
    # If multiple explicit tools specified, enable all mentioned
    if [[ "$explicit_tool" == "false" ]]; then
        # Default: run TLA+ and Kani (not Z3 until implemented)
        RUN_TLA=true
        RUN_KANI=true
        RUN_Z3=false
    fi
    
    print_header
    
    if [[ "$QUICK_MODE" == "true" ]]; then
        echo -e "${YELLOW}Quick mode enabled - using reduced bounds${NC}"
        echo ""
    fi
    
    local start_time
    start_time=$(date +%s)
    
    if [[ "$PARALLEL" == "true" ]] && [[ "$RUN_TLA" == "true" ]] && [[ "$RUN_KANI" == "true" ]]; then
        # Run in parallel
        echo -e "${BLUE}Running verifiers in parallel...${NC}"
        echo ""
        
        local tla_log kani_log
        tla_log=$(mktemp)
        kani_log=$(mktemp)
        
        # Start both processes
        (run_tla > "$tla_log" 2>&1; echo "TLA_EXIT=$?" >> "$tla_log") &
        local tla_pid=$!
        
        (run_kani > "$kani_log" 2>&1; echo "KANI_EXIT=$?" >> "$kani_log") &
        local kani_pid=$!
        
        # Wait for both
        wait $tla_pid || true
        wait $kani_pid || true
        
        # Show output
        echo "=== TLA+ Output ==="
        cat "$tla_log"
        echo ""
        echo "=== Kani Output ==="
        cat "$kani_log"
        
        rm -f "$tla_log" "$kani_log"
    else
        # Run sequentially
        [[ "$RUN_TLA" == "true" ]] && run_tla
        [[ "$RUN_KANI" == "true" ]] && run_kani
        [[ "$RUN_Z3" == "true" ]] && run_z3
    fi
    
    local end_time
    end_time=$(date +%s)
    
    echo ""
    echo "Total verification time: $((end_time - start_time))s"
    echo ""
    
    print_summary
}

main "$@"
