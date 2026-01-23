#!/bin/bash
# TLA+ Specification Verification Script for Fortress Rollback
#
# Usage: ./scripts/verify-tla.sh [options] [spec-name]
#   ./scripts/verify-tla.sh              # Verify all specs
#   ./scripts/verify-tla.sh NetworkProtocol  # Verify single spec
#   ./scripts/verify-tla.sh --quick      # Quick verification with smaller bounds
#   ./scripts/verify-tla.sh --list       # List available specs
#
# Prerequisites:
#   - Java 11+ installed (script will check)
#   - tla2tools.jar downloaded (script will fetch if missing)
#
# Environment Variables:
#   TLA_TOOLS_JAR    - Path to tla2tools.jar (default: .tla-tools/tla2tools.jar)
#   TLA_WORKERS      - Number of worker threads (default: auto)
#   TLA_MEMORY       - JVM heap size (default: 4g)
#   TLA_DEPTH        - Maximum state graph depth (default: unlimited)

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TLA_DIR="$PROJECT_ROOT/specs/tla"
TOOLS_DIR="$PROJECT_ROOT/.tla-tools"
TLA_TOOLS_JAR="${TLA_TOOLS_JAR:-$TOOLS_DIR/tla2tools.jar}"
TLA_TOOLS_URL="https://github.com/tlaplus/tlaplus/releases/download/v1.8.0/tla2tools.jar"
TLA_WORKERS="${TLA_WORKERS:-auto}"
TLA_MEMORY="${TLA_MEMORY:-4g}"
TLA_DEPTH="${TLA_DEPTH:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Specs to verify (only specs with .cfg files will be checked)
SPECS=(
    "NetworkProtocol"
    "InputQueue"
    "Concurrency"
    "Rollback"
    "TimeSync"
    "SpectatorSession"
)

# Track results
declare -A RESULTS
PASSED=0
FAILED=0
SKIPPED=0

print_usage() {
    echo "Usage: $0 [options] [spec-name]"
    echo ""
    echo "Options:"
    echo "  --quick     Use smaller bounds for faster verification"
    echo "  --list      List available specifications"
    echo "  --fail-fast Stop immediately when any spec fails"
    echo "  --help      Show this help message"
    echo "  --verbose   Show detailed TLC output"
    echo ""
    echo "Examples:"
    echo "  $0                      # Verify all specs"
    echo "  $0 NetworkProtocol      # Verify single spec"
    echo "  $0 --quick              # Quick verification"
    echo "  $0 --quick --fail-fast  # CI mode: quick, stop on first failure"
}

print_specs() {
    echo "Available TLA+ Specifications:"
    echo ""
    for spec in "${SPECS[@]}"; do
        local tla_file="$TLA_DIR/$spec.tla"
        local cfg_file="$TLA_DIR/$spec.cfg"
        local status="✓"
        [[ ! -f "$tla_file" ]] && status="✗ (missing .tla)"
        [[ ! -f "$cfg_file" ]] && status="✗ (missing .cfg)"
        echo "  $spec  $status"
    done
}

check_java() {
    if ! command -v java &> /dev/null; then
        echo -e "${RED}Error: Java is required but not installed.${NC}"
        echo "Please install Java 11+ and try again."
        exit 1
    fi

    local java_version
    java_version=$(java -version 2>&1 | head -n 1 | cut -d'"' -f2 | cut -d'.' -f1)
    if [[ "$java_version" -lt 11 ]] 2>/dev/null; then
        echo -e "${YELLOW}Warning: Java 11+ recommended, found version $java_version${NC}"
    fi
}

download_tla_tools() {
    if [[ -f "$TLA_TOOLS_JAR" ]]; then
        return 0
    fi

    echo -e "${BLUE}Downloading TLA+ tools...${NC}"
    mkdir -p "$TOOLS_DIR"

    if command -v curl &> /dev/null; then
        curl -L -o "$TLA_TOOLS_JAR" "$TLA_TOOLS_URL"
    elif command -v wget &> /dev/null; then
        wget -O "$TLA_TOOLS_JAR" "$TLA_TOOLS_URL"
    else
        echo -e "${RED}Error: curl or wget required to download TLA+ tools${NC}"
        exit 1
    fi

    echo -e "${GREEN}Downloaded TLA+ tools to $TLA_TOOLS_JAR${NC}"
}

verify_spec() {
    local spec_name="$1"
    local quick="${2:-false}"
    local verbose="${3:-false}"

    local tla_file="$TLA_DIR/$spec_name.tla"
    local cfg_file="$TLA_DIR/$spec_name.cfg"

    # Check files exist
    if [[ ! -f "$tla_file" ]]; then
        echo -e "${RED}Error: $tla_file not found${NC}"
        RESULTS[$spec_name]="SKIP"
        ((SKIPPED++))
        return 1
    fi

    if [[ ! -f "$cfg_file" ]]; then
        echo -e "${RED}Error: $cfg_file not found${NC}"
        RESULTS[$spec_name]="SKIP"
        ((SKIPPED++))
        return 1
    fi

    echo -e "${BLUE}Verifying $spec_name...${NC}"

    # Build TLC command
    local tlc_cmd=(
        java
        "-Xmx$TLA_MEMORY"
        -XX:+UseParallelGC
        -jar "$TLA_TOOLS_JAR"
        -deadlock
    )

    # Add worker count
    if [[ "$TLA_WORKERS" != "auto" ]]; then
        tlc_cmd+=(-workers "$TLA_WORKERS")
    fi

    # Add depth limit if set
    if [[ -n "$TLA_DEPTH" ]]; then
        tlc_cmd+=(-depth "$TLA_DEPTH")
    fi

    # Add config and spec files
    tlc_cmd+=(-config "$cfg_file" "$tla_file")

    # Run TLC
    local start_time
    start_time=$(date +%s)

    local output_file
    output_file=$(mktemp)

    local exit_code=0
    if [[ "$verbose" == "true" ]]; then
        "${tlc_cmd[@]}" 2>&1 | tee "$output_file" || exit_code=$?
    else
        "${tlc_cmd[@]}" > "$output_file" 2>&1 || exit_code=$?
    fi

    local end_time
    end_time=$(date +%s)
    local duration=$((end_time - start_time))

    # Check results
    if [[ $exit_code -eq 0 ]] && grep -q "Model checking completed. No error has been found." "$output_file"; then
        echo -e "${GREEN}✓ $spec_name passed (${duration}s)${NC}"

        # Extract stats
        local states
        states=$(grep -oP '\d+ states generated' "$output_file" | head -1 || echo "")
        if [[ -n "$states" ]]; then
            echo "  $states"
        fi

        RESULTS[$spec_name]="PASS"
        ((PASSED++))
        rm -f "$output_file"
        return 0
    else
        echo -e "${RED}✗ $spec_name failed (${duration}s)${NC}"

        # Show error details
        if grep -q "Error:" "$output_file"; then
            echo -e "${RED}Error details:${NC}"
            grep -A 5 "Error:" "$output_file" | head -20
        elif grep -q "Invariant.*violated" "$output_file"; then
            echo -e "${RED}Invariant violation:${NC}"
            grep -A 10 "Invariant.*violated" "$output_file" | head -20
        else
            echo "TLC output (last 20 lines):"
            tail -20 "$output_file"
        fi

        RESULTS[$spec_name]="FAIL"
        ((FAILED++))
        rm -f "$output_file"
        return 1
    fi
}

print_summary() {
    echo ""
    echo "=========================================="
    echo "TLA+ Verification Summary"
    echo "=========================================="

    for spec in "${SPECS[@]}"; do
        local result="${RESULTS[$spec]:-SKIP}"
        local color="$NC"
        local symbol="?"

        case "$result" in
            PASS) color="$GREEN"; symbol="✓" ;;
            FAIL) color="$RED"; symbol="✗" ;;
            SKIP) color="$YELLOW"; symbol="-" ;;
        esac

        echo -e "  $symbol $spec: ${color}$result${NC}"
    done

    echo "=========================================="
    echo -e "  ${GREEN}Passed: $PASSED${NC}  ${RED}Failed: $FAILED${NC}  ${YELLOW}Skipped: $SKIPPED${NC}"
    echo "=========================================="
}

main() {
    local quick=false
    local verbose=false
    local fail_fast=false
    local target_spec=""

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
            --fail-fast)
                fail_fast=true
                shift
                ;;
            --list)
                print_specs
                exit 0
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
                target_spec="$1"
                shift
                ;;
        esac
    done

    echo "=========================================="
    echo "Fortress Rollback TLA+ Verification"
    echo "=========================================="

    # Check prerequisites
    check_java
    download_tla_tools

    # Determine which specs to verify
    local specs_to_verify=("${SPECS[@]}")
    if [[ -n "$target_spec" ]]; then
        # Verify target spec exists
        local found=false
        for spec in "${SPECS[@]}"; do
            if [[ "$spec" == "$target_spec" ]]; then
                found=true
                break
            fi
        done

        if [[ "$found" == "false" ]]; then
            echo -e "${RED}Error: Unknown spec '$target_spec'${NC}"
            print_specs
            exit 1
        fi

        specs_to_verify=("$target_spec")
    fi

    # Apply quick mode settings
    if [[ "$quick" == "true" ]]; then
        echo -e "${YELLOW}Quick mode: using reduced bounds${NC}"
        export TLA_DEPTH="${TLA_DEPTH:-100}"
    fi

    echo ""

    # Verify each spec
    local any_failed=false
    for spec in "${specs_to_verify[@]}"; do
        if ! verify_spec "$spec" "$quick" "$verbose"; then
            any_failed=true
            if [[ "$fail_fast" == "true" ]]; then
                echo -e "${RED}Stopping early due to --fail-fast${NC}"
                break
            fi
        fi
        echo ""
    done

    # Print summary
    print_summary

    # Exit with appropriate code
    if [[ "$any_failed" == "true" ]]; then
        exit 1
    fi
}

main "$@"
