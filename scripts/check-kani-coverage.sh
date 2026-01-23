#!/bin/bash
# Kani Proof Coverage Validation Script for Fortress Rollback
#
# This script ensures that all #[kani::proof] functions defined in the source code
# are included in the tier lists in verify-kani.sh. This prevents new proofs from
# being silently skipped during CI verification.
#
# Usage: ./scripts/check-kani-coverage.sh [options]
#   ./scripts/check-kani-coverage.sh           # Validate coverage
#   ./scripts/check-kani-coverage.sh --fix     # Show what needs to be added
#   ./scripts/check-kani-coverage.sh --verbose # Show detailed comparison
#
# Exit codes:
#   0 - All proofs are covered
#   1 - Missing or extra proofs found

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VERIFY_KANI_SCRIPT="$SCRIPT_DIR/verify-kani.sh"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Options
VERBOSE=false
FIX_MODE=false

print_usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --verbose  Show detailed comparison of proofs"
    echo "  --fix      Show what proofs need to be added/removed"
    echo "  --help     Show this help message"
    echo ""
    echo "This script validates that all #[kani::proof] functions in the source code"
    echo "are included in the tier lists in scripts/verify-kani.sh."
}

# Extract proof names from source code
get_source_proofs() {
    # Find all functions marked with #[kani::proof] in src/
    # Pattern: look for #[kani::proof] followed by fn proof_*
    # Use ripgrep if available, otherwise fall back to grep -r
    if command -v rg &> /dev/null; then
        rg -A 3 '#\[kani::proof\]' "$PROJECT_ROOT/src/" 2>/dev/null \
            | grep 'fn proof_' \
            | /bin/sed 's/.*fn \(proof_[a-zA-Z0-9_]*\).*/\1/' \
            | sort \
            | uniq
    else
        grep -r -A 3 '#\[kani::proof\]' "$PROJECT_ROOT/src/" 2>/dev/null \
            | grep 'fn proof_' \
            | /bin/sed 's/.*fn \(proof_[a-zA-Z0-9_]*\).*/\1/' \
            | sort \
            | uniq
    fi
}

# Extract proof names from verify-kani.sh tier arrays
get_tiered_proofs() {
    # Extract all proof names from TIER1_PROOFS, TIER2_PROOFS, and TIER3_PROOFS arrays
    grep -E '^\s*"proof_[^"]+"\s*$' "$VERIFY_KANI_SCRIPT" \
        | /bin/sed 's/.*"\(proof_[^"]*\)".*/\1/' \
        | sort \
        | uniq
}

# Main validation logic
main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --verbose)
                VERBOSE=true
                shift
                ;;
            --fix)
                FIX_MODE=true
                shift
                ;;
            --help)
                print_usage
                exit 0
                ;;
            *)
                echo "Unknown argument: $1"
                print_usage
                exit 1
                ;;
        esac
    done

    echo "=========================================="
    echo " Kani Proof Coverage Validation"
    echo "=========================================="
    echo ""

    # Check prerequisites - ripgrep is optional, grep -r is fallback
    if command -v rg &> /dev/null; then
        echo -e "${BLUE}Using ripgrep for source scanning${NC}"
    else
        echo -e "${YELLOW}Note: ripgrep not found, using grep -r fallback${NC}"
    fi

    if [[ ! -f "$VERIFY_KANI_SCRIPT" ]]; then
        echo -e "${RED}Error: verify-kani.sh not found at $VERIFY_KANI_SCRIPT${NC}"
        exit 1
    fi

    # Get proof lists
    echo -e "${BLUE}Scanning source code for #[kani::proof] functions...${NC}"
    SOURCE_PROOFS=$(get_source_proofs)
    # Use || true to handle grep exit code 1 (no matches), then default to 0 if empty
    SOURCE_COUNT=$(echo "$SOURCE_PROOFS" | grep -c '^proof_' || true)
    SOURCE_COUNT=${SOURCE_COUNT:-0}

    echo -e "${BLUE}Scanning verify-kani.sh for tiered proofs...${NC}"
    TIERED_PROOFS=$(get_tiered_proofs)
    # Use || true to handle grep exit code 1 (no matches), then default to 0 if empty
    TIERED_COUNT=$(echo "$TIERED_PROOFS" | grep -c '^proof_' || true)
    TIERED_COUNT=${TIERED_COUNT:-0}

    echo ""
    echo "Found $SOURCE_COUNT proofs in source code"
    echo "Found $TIERED_COUNT proofs in tier lists"
    echo ""

    # Find proofs in source but not in tiers (missing from CI)
    MISSING_PROOFS=$(comm -23 <(echo "$SOURCE_PROOFS") <(echo "$TIERED_PROOFS") || true)
    if [[ -n "$MISSING_PROOFS" ]]; then
        # Use || true to handle grep exit code 1 (no matches), then default to 0 if empty
        MISSING_COUNT=$(echo "$MISSING_PROOFS" | grep -c '^proof_' 2>/dev/null || true)
        MISSING_COUNT=${MISSING_COUNT:-0}
    else
        MISSING_COUNT=0
    fi

    # Find proofs in tiers but not in source (stale entries)
    EXTRA_PROOFS=$(comm -13 <(echo "$SOURCE_PROOFS") <(echo "$TIERED_PROOFS") || true)
    if [[ -n "$EXTRA_PROOFS" ]]; then
        # Use || true to handle grep exit code 1 (no matches), then default to 0 if empty
        EXTRA_COUNT=$(echo "$EXTRA_PROOFS" | grep -c '^proof_' 2>/dev/null || true)
        EXTRA_COUNT=${EXTRA_COUNT:-0}
    else
        EXTRA_COUNT=0
    fi

    # Report findings
    local has_errors=false

    if [[ "$MISSING_COUNT" -gt 0 ]]; then
        has_errors=true
        echo -e "${RED}ERROR: $MISSING_COUNT proof(s) found in source but NOT in verify-kani.sh tier lists:${NC}"
        echo ""
        echo "$MISSING_PROOFS" | while read -r proof; do
            if [[ -n "$proof" ]]; then
                # Find which file contains this proof
                local file
                file=$(rg -l "fn $proof" "$PROJECT_ROOT/src/" 2>/dev/null | head -1 || echo "unknown")
                file=${file#"$PROJECT_ROOT/"}
                echo -e "  ${YELLOW}$proof${NC} (in $file)"
            fi
        done
        echo ""
        if [[ "$FIX_MODE" == "true" ]]; then
            echo -e "${BLUE}To fix: Add these proofs to the appropriate tier in scripts/verify-kani.sh${NC}"
            echo ""
            echo "Suggested additions (copy to appropriate TIER array):"
            echo "$MISSING_PROOFS" | while read -r proof; do
                if [[ -n "$proof" ]]; then
                    echo "    \"$proof\""
                fi
            done
            echo ""
        fi
    fi

    if [[ "$EXTRA_COUNT" -gt 0 ]]; then
        has_errors=true
        echo -e "${RED}ERROR: $EXTRA_COUNT proof(s) found in verify-kani.sh but NOT in source code:${NC}"
        echo ""
        echo "$EXTRA_PROOFS" | while read -r proof; do
            if [[ -n "$proof" ]]; then
                echo -e "  ${YELLOW}$proof${NC}"
            fi
        done
        echo ""
        if [[ "$FIX_MODE" == "true" ]]; then
            echo -e "${BLUE}To fix: Remove these stale entries from scripts/verify-kani.sh${NC}"
            echo ""
        fi
    fi

    # Verbose output
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "${BLUE}=== Detailed Comparison ===${NC}"
        echo ""
        echo "Proofs in source code:"
        echo "$SOURCE_PROOFS" | while read -r proof; do
            if [[ -n "$proof" ]]; then
                if echo "$TIERED_PROOFS" | grep -q "^$proof$"; then
                    echo -e "  ${GREEN}✓${NC} $proof"
                else
                    echo -e "  ${RED}✗${NC} $proof (NOT in tier lists)"
                fi
            fi
        done
        echo ""
        echo "Proofs in tier lists:"
        echo "$TIERED_PROOFS" | while read -r proof; do
            if [[ -n "$proof" ]]; then
                if echo "$SOURCE_PROOFS" | grep -q "^$proof$"; then
                    echo -e "  ${GREEN}✓${NC} $proof"
                else
                    echo -e "  ${RED}✗${NC} $proof (NOT in source code)"
                fi
            fi
        done
        echo ""
    fi

    # Final result
    if [[ "$has_errors" == "true" ]]; then
        echo -e "${RED}FAILED: Kani proof coverage is incomplete.${NC}"
        echo ""
        echo "All #[kani::proof] functions must be included in a tier list in"
        echo "scripts/verify-kani.sh to ensure they are run during CI verification."
        echo ""
        echo "Run with --fix for suggestions on how to fix this."
        exit 1
    else
        echo -e "${GREEN}SUCCESS: All $SOURCE_COUNT Kani proofs are covered in tier lists.${NC}"
        exit 0
    fi
}

main "$@"
