#!/bin/bash
# Doc Comment Accuracy Check for Fortress Rollback
#
# Checks that doc comments mentioning "downcast" are backed by actual
# downcasting infrastructure in the same file. This prevents misleading
# documentation that references capabilities the code doesn't support.
#
# Usage: ./scripts/ci/check-doc-claims.sh [options]
#   ./scripts/ci/check-doc-claims.sh            # Check all Rust files
#   ./scripts/ci/check-doc-claims.sh --verbose  # Show all files checked
#   ./scripts/ci/check-doc-claims.sh --help     # Show help
#
# Exit codes:
#   0 - No issues found
#   1 - Misleading doc comments detected

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Options
VERBOSE=false

print_usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --verbose  Show all files checked"
    echo "  --help     Show this help message"
    echo ""
    echo "Checks doc comments for claims about downcasting that aren't"
    echo "backed by actual downcasting infrastructure in the same file."
}

main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --verbose)
                VERBOSE=true
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
    echo " Doc Comment Accuracy Check"
    echo "=========================================="
    echo ""

    # Patterns that indicate actual downcasting infrastructure
    # If a file mentions downcasting in docs, it should contain at least one of these
    local downcast_infra_patterns='(as_any|downcast_ref|downcast_mut|dyn Any|: Any|impl Any|Any \+|Any\+|\.downcast\b)'

    local issues=0
    local files_with_claims=0

    # Find all Rust source files (excluding target directories)
    while IFS= read -r file; do
        [[ -z "$file" ]] && continue

        local rel_path="${file#"$PROJECT_ROOT/"}"

        # Find doc comment lines mentioning "downcast" (case-insensitive)
        local doc_matches
        doc_matches=$(grep -niE '^\s*///.*downcast|^\s*//!.*downcast' "$file" 2>/dev/null || true)

        if [[ -z "$doc_matches" ]]; then
            if [[ "$VERBOSE" == "true" ]]; then
                echo -e "  ${GREEN}OK${NC}: $rel_path (no downcast doc claims)"
            fi
            continue
        fi

        files_with_claims=$((files_with_claims + 1))

        if [[ "$VERBOSE" == "true" ]]; then
            echo -e "  ${YELLOW}Checking${NC}: $rel_path (has downcast doc claims)"
        fi

        # Check if the file has actual downcasting infrastructure
        local has_infra
        has_infra=0
        has_infra=$(grep -cE "$downcast_infra_patterns" "$file" 2>/dev/null) || true

        if [[ "$has_infra" -eq 0 ]]; then
            issues=$((issues + 1))
            echo ""
            echo -e "${RED}ERROR${NC}: $rel_path"
            echo -e "  Doc comments mention \"downcast\" but no downcasting infrastructure found."
            echo -e "  ${YELLOW}Doc comment(s):${NC}"
            while IFS= read -r match_line; do
                echo -e "    $match_line"
            done <<< "$doc_matches"
            echo -e "  ${BLUE}Expected one of:${NC} as_any, downcast_ref, downcast_mut, dyn Any, : Any"
            echo -e "  ${BLUE}Fix:${NC} Either add downcasting support or update the doc comment"
            echo -e "       to accurately describe the actual pattern used."
        else
            if [[ "$VERBOSE" == "true" ]]; then
                echo -e "    ${GREEN}OK${NC}: downcasting infrastructure found ($has_infra occurrence(s))"
            fi
        fi

    done < <(find "$PROJECT_ROOT/src" "$PROJECT_ROOT/tests" "$PROJECT_ROOT/examples" "$PROJECT_ROOT/benches" \
        -name '*.rs' -print 2>/dev/null \
        | sort)

    echo ""

    if [[ "$issues" -eq 0 ]]; then
        echo -e "${GREEN}SUCCESS: No misleading downcast doc claims found.${NC}"
        if [[ "$files_with_claims" -gt 0 ]]; then
            echo -e "  ($files_with_claims file(s) with downcast references verified)"
        fi
        exit 0
    fi

    echo -e "${RED}FAILED: $issues file(s) have misleading downcast doc claims.${NC}"
    echo ""
    echo "Doc comments should accurately describe the code's capabilities."
    echo "If downcasting isn't supported, describe the actual pattern instead."
    exit 1
}

main "$@"
