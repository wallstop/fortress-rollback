#!/bin/bash
# Kani Formal Verification Script for Fortress Rollback
#
# Usage: ./scripts/verify-kani.sh [options]
#   ./scripts/verify-kani.sh              # Run all Kani proofs
#   ./scripts/verify-kani.sh --list       # List all proof harnesses
#   ./scripts/verify-kani.sh --harness X  # Run specific harness
#   ./scripts/verify-kani.sh --quick      # Run with reduced unwind bounds
#   ./scripts/verify-kani.sh --tier N     # Run only tier N proofs (1=fast, 2=medium, 3=slow)
#
# Prerequisites:
#   - Kani installed (script provides install instructions if missing)
#   - Rust nightly toolchain
#
# Environment Variables:
#   KANI_TIMEOUT     - Timeout per proof in seconds (default: 300)
#   KANI_UNWIND      - Default unwind bound (default: use Kani defaults)
#   KANI_JOBS        - Number of parallel jobs (default: 1)

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
KANI_TIMEOUT="${KANI_TIMEOUT:-300}"
KANI_UNWIND="${KANI_UNWIND:-}"
KANI_JOBS="${KANI_JOBS:-1}"

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

# Tier definitions - proofs grouped by approximate runtime
# Tier 1: Fast proofs (<30s each) - simple property checks
TIER1_PROOFS=(
    "proof_frame_new_valid"
    "proof_frame_null_consistency"
    "proof_frame_to_option"
    "proof_frame_from_option"
    "proof_varint_encoded_len_correct"
    "proof_varint_encode_single_byte"
    "proof_varint_encoded_len_no_overflow"
    "proof_varint_decode_empty_safe"
    "proof_window_index_in_bounds"
    "proof_sum_no_overflow"
    "proof_division_safe"
    "proof_window_size_minimum"
    "proof_default_valid"
    "proof_validate_accepts_valid_queue_lengths"
    "proof_validate_boundary_at_two"
    "proof_max_frame_delay_derivation"
    "proof_all_presets_valid"
    "proof_preset_values"
    "proof_preset_configs_valid"
    "proof_zero_window_size_corrected"
    "proof_negative_frame_safe"
)

# Tier 2: Medium proofs (30s-2min each) - moderate complexity
TIER2_PROOFS=(
    "proof_frame_add_small_safe"
    "proof_frame_sub_frames_correct"
    "proof_frame_ordering_consistent"
    "proof_frame_modulo_for_queue"
    "proof_frame_add_assign_consistent"
    "proof_frame_sub_assign_consistent"
    "proof_player_handle_validity"
    "proof_varint_decode_terminates"
    "proof_varint_decode_offset_safe"
    "proof_varint_roundtrip_small"
    "proof_varint_continuation_handling"
    "proof_advance_frame_safe"
    "proof_validate_frame_delay_constraint"
    "proof_max_frame_delay_is_valid_delay"
    "proof_new_queue_valid"
    "proof_head_wraparound"
    "proof_queue_index_calculation"
    "proof_length_calculation_consistent"
    "proof_new_sync_layer_valid"
    "proof_advance_frame_monotonic"
    "proof_saved_states_count"
    "proof_get_cell_validates_frame"
    "proof_saved_states_circular_index"
    "proof_index_wrapping_consistent"
)

# Tier 3: Slow proofs (>2min each) - complex state verification
TIER3_PROOFS=(
    "proof_add_single_input_maintains_invariants"
    "proof_sequential_inputs_maintain_invariants"
    "proof_discard_maintains_invariants"
    "proof_frame_delay_maintains_invariants"
    "proof_non_sequential_rejected"
    "proof_reset_maintains_structure"
    "proof_confirmed_input_valid_index"
    "proof_multiple_advances_monotonic"
    "proof_save_maintains_inv8"
    "proof_load_frame_validates_bounds"
    "proof_load_frame_success_maintains_invariants"
    "proof_set_frame_delay_validates_handle"
    "proof_reset_prediction_preserves_frames"
    "proof_confirmed_frame_bounded"
    "proof_sparse_saving_respects_saved_frame"
)

print_usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --list          List all Kani proof harnesses"
    echo "  --harness NAME  Run specific harness (can be repeated)"
    echo "  --quick         Run with reduced bounds for faster verification"
    echo "  --tier N        Run only tier N proofs (1=fast, 2=medium, 3=slow)"
    echo "  --verbose       Show detailed Kani output"
    echo "  --jobs N        Run N harnesses in parallel (default: 1)"
    echo "  --fail-fast     Stop immediately when any proof fails (useful for CI)"
    echo "  --help          Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  KANI_TIMEOUT    Timeout per proof in seconds (default: 300)"
    echo "  KANI_JOBS       Number of parallel jobs (default: 1)"
    echo ""
    echo "Tiers:"
    echo "  Tier 1: ${#TIER1_PROOFS[@]} fast proofs (<30s each)"
    echo "  Tier 2: ${#TIER2_PROOFS[@]} medium proofs (30s-2min each)"
    echo "  Tier 3: ${#TIER3_PROOFS[@]} slow proofs (>2min each)"
    echo ""
    echo "Examples:"
    echo "  $0                              # Run all proofs"
    echo "  $0 --tier 1                     # Run only fast proofs"
    echo "  $0 --tier 1 --tier 2            # Run fast and medium proofs"
    echo "  $0 --harness proof_frame_new_valid  # Run single proof"
    echo "  $0 --quick --jobs 4             # Fast mode with parallel execution"
    echo "  $0 --quick --fail-fast          # CI mode: fast bounds, stop on first failure"
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

get_tier_proofs() {
    local tier=$1
    case "$tier" in
        1) printf '%s\n' "${TIER1_PROOFS[@]}" ;;
        2) printf '%s\n' "${TIER2_PROOFS[@]}" ;;
        3) printf '%s\n' "${TIER3_PROOFS[@]}" ;;
        *) echo "Invalid tier: $tier" >&2; return 1 ;;
    esac
}

list_harnesses() {
    echo "Kani Proof Harnesses in Fortress Rollback:"
    echo ""
    
    echo -e "${BLUE}Tier 1 - Fast proofs (${#TIER1_PROOFS[@]} proofs, <30s each):${NC}"
    for harness in "${TIER1_PROOFS[@]}"; do
        echo "  - $harness"
    done
    
    echo ""
    echo -e "${YELLOW}Tier 2 - Medium proofs (${#TIER2_PROOFS[@]} proofs, 30s-2min each):${NC}"
    for harness in "${TIER2_PROOFS[@]}"; do
        echo "  - $harness"
    done
    
    echo ""
    echo -e "${RED}Tier 3 - Slow proofs (${#TIER3_PROOFS[@]} proofs, >2min each):${NC}"
    for harness in "${TIER3_PROOFS[@]}"; do
        echo "  - $harness"
    done
    
    local total=$((${#TIER1_PROOFS[@]} + ${#TIER2_PROOFS[@]} + ${#TIER3_PROOFS[@]}))
    echo ""
    echo "Total: $total harnesses"
    
    # Also show any proofs not in tiers (discovered from source)
    cd "$PROJECT_ROOT"
    local all_proofs
    all_proofs=$(grep -rh '#\[kani::proof\]' --include='*.rs' -A 1 src/ 2>/dev/null | \
                grep -oP 'fn \K[a-zA-Z_][a-zA-Z0-9_]*' | sort -u || echo "")
    
    local uncategorized=()
    while IFS= read -r proof; do
        local found=false
        for t1 in "${TIER1_PROOFS[@]}"; do [[ "$proof" == "$t1" ]] && found=true && break; done
        if [[ "$found" == "false" ]]; then
            for t2 in "${TIER2_PROOFS[@]}"; do [[ "$proof" == "$t2" ]] && found=true && break; done
        fi
        if [[ "$found" == "false" ]]; then
            for t3 in "${TIER3_PROOFS[@]}"; do [[ "$proof" == "$t3" ]] && found=true && break; done
        fi
        if [[ "$found" == "false" ]]; then
            uncategorized+=("$proof")
        fi
    done <<< "$all_proofs"
    
    if [[ ${#uncategorized[@]} -gt 0 ]]; then
        echo ""
        echo -e "${YELLOW}Uncategorized proofs (will run with default tier):${NC}"
        for proof in "${uncategorized[@]}"; do
            echo "  - $proof"
        done
    fi
}

run_kani() {
    local harness="${1:-}"
    local quick="${2:-false}"
    local verbose="${3:-false}"
    local jobs="${4:-1}"
    
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
    
    # Quick mode uses smaller bounds and enables optimizations
    if [[ "$quick" == "true" ]]; then
        kani_cmd+=(--default-unwind 8)
    fi
    
    # Add jobs for parallel execution
    if [[ "$jobs" -gt 1 ]]; then
        kani_cmd+=(--jobs "$jobs")
    fi
    
    local start_time
    start_time=$(date +%s)
    
    local output_file
    output_file=$(mktemp)
    
    local exit_code=0
    # Disable colors in Kani output to ensure reliable parsing
    export NO_COLOR=1
    export TERM=dumb
    if [[ "$verbose" == "true" ]]; then
        "${kani_cmd[@]}" 2>&1 | tee "$output_file" || exit_code=$?
    else
        "${kani_cmd[@]}" > "$output_file" 2>&1 || exit_code=$?
    fi

    local end_time
    end_time=$(date +%s)
    local duration=$((end_time - start_time))

    # Strip ANSI color codes from output before parsing
    # This is necessary because Kani may output colored text
    local clean_output
    clean_output=$(mktemp)
    sed 's/\x1b\[[0-9;]*m//g' "$output_file" > "$clean_output" 2>/dev/null || cp "$output_file" "$clean_output"

    # Parse results - look for summary line which has the format:
    # Complete - N successfully verified harnesses, M failures, T total.
    local summary_line
    summary_line=$(grep "Complete - " "$clean_output" 2>/dev/null | tail -1 || echo "")

    if [[ -n "$summary_line" ]]; then
        # Extract numbers using sed for better portability (grep -P not available everywhere)
        # Use tr -d to strip any whitespace/newlines from extracted values
        PASSED=$(echo "$summary_line" | sed -n 's/.*\([0-9][0-9]*\) successfully.*/\1/p' | head -1 | tr -d '[:space:]')
        FAILED=$(echo "$summary_line" | sed -n 's/.*\([0-9][0-9]*\) failure.*/\1/p' | head -1 | tr -d '[:space:]')
        TOTAL=$(echo "$summary_line" | sed -n 's/.*\([0-9][0-9]*\) total.*/\1/p' | head -1 | tr -d '[:space:]')
        # Default to 0 if extraction failed
        PASSED=${PASSED:-0}
        FAILED=${FAILED:-0}
        TOTAL=${TOTAL:-0}
    else
        # Fallback to counting individual results
        # Note: grep -c returns exit code 1 when no matches (but still outputs "0")
        # Using || VARNAME=0 pattern to handle this correctly
        # Use tr -d to strip any whitespace/newlines from grep output
        # Try multiple patterns to handle different Kani output formats
        PASSED=$(grep -c -E "VERIFICATION:- SUCCESSFUL|Verification succeeded" "$clean_output" 2>/dev/null | tr -d '[:space:]') || PASSED=0
        FAILED=$(grep -c -E "VERIFICATION:- FAILED|Verification failed|VERIFICATION RESULT.*FAILURE" "$clean_output" 2>/dev/null | tr -d '[:space:]') || FAILED=0
        # Ensure values are clean integers before arithmetic
        PASSED=${PASSED%%[^0-9]*}
        FAILED=${FAILED%%[^0-9]*}
        PASSED=${PASSED:-0}
        FAILED=${FAILED:-0}
        TOTAL=$((PASSED + FAILED))
    fi

    # If we got no results, something went wrong - diagnose the issue
    if [[ $TOTAL -eq 0 ]]; then
        if [[ $exit_code -ne 0 ]]; then
            # Kani failed with an error - show what went wrong
            echo -e "${RED}Kani exited with error code $exit_code${NC}"
            echo ""
            # Check for common errors
            if grep -q -i "error\[E" "$clean_output" 2>/dev/null; then
                echo "Compilation errors detected:"
                grep -E "error\[E[0-9]+\]" "$clean_output" | head -5
            elif grep -q "no harnesses\|No proof harness\|no proof harness" "$clean_output" 2>/dev/null; then
                echo -e "${RED}Error: Harness not found by Kani${NC}"
            elif grep -q "unsupported\|not supported" "$clean_output" 2>/dev/null; then
                echo "Unsupported feature detected:"
                grep -i "unsupported\|not supported" "$clean_output" | head -3
            fi
            if [[ "$verbose" != "true" ]]; then
                echo ""
                echo "Last 30 lines of Kani output:"
                tail -30 "$clean_output"
                echo ""
                echo "Run with --verbose for full output"
            fi
        elif grep -q "Checking harness" "$clean_output" 2>/dev/null; then
            # Harness was found but no verification result detected - parsing issue
            echo -e "${YELLOW}Warning: Could not parse Kani output. Check output format.${NC}"
            if [[ "$verbose" != "true" ]]; then
                echo "Last 20 lines of Kani output:"
                tail -20 "$clean_output"
            fi
        else
            # No "Checking harness" found and exit code is 0 - very strange
            echo -e "${YELLOW}Warning: Kani completed but no harness was processed${NC}"
            if [[ "$verbose" != "true" ]]; then
                echo "Last 20 lines of Kani output:"
                tail -20 "$clean_output"
            fi
        fi
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
        # Try both old and new Kani output formats
        grep -E -B 5 "VERIFICATION:- FAILED|Verification failed" "$clean_output" | grep -E "Checking harness|harness" || true

        if [[ "$verbose" != "true" ]]; then
            echo ""
            echo "Run with --verbose for detailed output"
        fi
    fi

    rm -f "$output_file" "$clean_output"
    
    if [[ $exit_code -ne 0 ]] || [[ $FAILED -gt 0 ]]; then
        return 1
    fi
    
    return 0
}

run_tier_proofs() {
    local tier=$1
    local quick=$2
    local verbose=$3
    local jobs=$4
    local fail_fast=$5
    
    local proofs
    case "$tier" in
        1) proofs=("${TIER1_PROOFS[@]}") ;;
        2) proofs=("${TIER2_PROOFS[@]}") ;;
        3) proofs=("${TIER3_PROOFS[@]}") ;;
        *) echo "Invalid tier: $tier" >&2; return 1 ;;
    esac
    
    echo -e "${BLUE}Running Tier $tier proofs (${#proofs[@]} harnesses)...${NC}"
    
    local any_failed=false
    local tier_passed=0
    local tier_failed=0
    
    for harness in "${proofs[@]}"; do
        echo -e "${BLUE}  Verifying: $harness${NC}"
        if run_kani "$harness" "$quick" "$verbose" "$jobs"; then
            ((tier_passed++))
        else
            any_failed=true
            ((tier_failed++))
            if [[ "$fail_fast" == "true" ]]; then
                echo -e "${RED}Stopping early due to --fail-fast${NC}"
                echo ""
                echo "Tier $tier Results: $tier_passed passed, $tier_failed failed (stopped early)"
                return 1
            fi
        fi
    done
    
    echo ""
    echo "Tier $tier Results: $tier_passed passed, $tier_failed failed"
    
    if [[ "$any_failed" == "true" ]]; then
        return 1
    fi
    return 0
}

main() {
    local quick=false
    local verbose=false
    local list_only=false
    local fail_fast=false
    local harnesses=()
    local tiers=()
    local jobs=1
    
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
            --fail-fast)
                fail_fast=true
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
            --tier)
                if [[ $# -lt 2 ]]; then
                    echo "Error: --tier requires an argument (1, 2, or 3)"
                    exit 1
                fi
                if [[ "$2" =~ ^[1-3]$ ]]; then
                    tiers+=("$2")
                else
                    echo "Error: --tier must be 1, 2, or 3"
                    exit 1
                fi
                shift 2
                ;;
            --jobs)
                if [[ $# -lt 2 ]]; then
                    echo "Error: --jobs requires a number"
                    exit 1
                fi
                jobs="$2"
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
            if ! run_kani "$harness" "$quick" "$verbose" "$jobs"; then
                any_failed=true
                if [[ "$fail_fast" == "true" ]]; then
                    echo -e "${RED}Stopping early due to --fail-fast${NC}"
                    break
                fi
            fi
        done
    elif [[ ${#tiers[@]} -gt 0 ]]; then
        # Run specific tiers
        for tier in "${tiers[@]}"; do
            if ! run_tier_proofs "$tier" "$quick" "$verbose" "$jobs" "$fail_fast"; then
                any_failed=true
                if [[ "$fail_fast" == "true" ]]; then
                    break
                fi
            fi
        done
    else
        # Run all harnesses (default)
        if ! run_kani "" "$quick" "$verbose" "$jobs"; then
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
