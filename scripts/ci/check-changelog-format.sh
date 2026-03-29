#!/bin/bash
# Changelog Format Check for Fortress Rollback
#
# Validates that CHANGELOG.md follows the Keep a Changelog convention:
# all ### level headings within version sections must be one of the six
# standard types: Added, Changed, Deprecated, Removed, Fixed, Security.
#
# Non-standard headings (e.g., "### Breaking") are flagged as errors with
# a suggestion to use the appropriate standard heading instead.
#
# Usage: ./scripts/ci/check-changelog-format.sh [options]
#   ./scripts/ci/check-changelog-format.sh            # Check CHANGELOG.md
#   ./scripts/ci/check-changelog-format.sh --verbose  # Show all headings checked
#   ./scripts/ci/check-changelog-format.sh --help     # Show help
#
# Exit codes:
#   0 - No issues found
#   1 - Non-standard headings detected

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHANGELOG="$PROJECT_ROOT/CHANGELOG.md"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Standard Keep a Changelog heading types
VALID_HEADINGS="Added|Changed|Deprecated|Removed|Fixed|Security"

# Options
VERBOSE=false

print_usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --verbose  Show all headings checked"
    echo "  --help     Show this help message"
    echo ""
    echo "Validates that CHANGELOG.md uses only standard Keep a Changelog"
    echo "section headings: Added, Changed, Deprecated, Removed, Fixed, Security."
    echo ""
    echo "Non-standard headings like '### Breaking' should use '### Changed'"
    echo "with a '**Breaking:**' prefix on each entry instead."
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
    echo " Changelog Format Check"
    echo "=========================================="
    echo ""

    if [[ ! -f "$CHANGELOG" ]]; then
        echo -e "${RED}ERROR: CHANGELOG.md not found at $CHANGELOG${NC}"
        exit 1
    fi

    local issues=0
    local headings_checked=0
    local line_num=0
    local in_version_section=false
    local past_separator=false

    while IFS= read -r line; do
        line_num=$((line_num + 1))

        # Detect the --- separator that ends the version sections.
        # After this point, headings belong to non-version content (e.g.,
        # the "Breaking Changes from GGRS" migration guide) and should
        # not be validated against Keep a Changelog conventions.
        if [[ "$line" =~ ^---[[:space:]]*$ ]]; then
            past_separator=true
            continue
        fi

        # Stop checking once we're past the separator
        if [[ "$past_separator" == "true" ]]; then
            continue
        fi

        # Detect ## [version] headers (version sections)
        if [[ "$line" =~ ^##[[:space:]]+\[ ]]; then
            in_version_section=true
            continue
        fi

        # Detect other ## headers (non-version, e.g., top-level "# Changelog")
        # A bare ## without [ after it means we left a version section
        if [[ "$line" =~ ^##[[:space:]] ]] && ! [[ "$line" =~ ^##[[:space:]]+\[ ]]; then
            in_version_section=false
            continue
        fi

        # Only check ### headings inside version sections
        if [[ "$in_version_section" == "true" ]] && [[ "$line" =~ ^###[[:space:]] ]]; then
            # Extract the heading text (everything after "### ")
            local heading="${line#\#\#\# }"

            headings_checked=$((headings_checked + 1))

            if [[ "$heading" =~ ^($VALID_HEADINGS)$ ]]; then
                if [[ "$VERBOSE" == "true" ]]; then
                    echo -e "  ${GREEN}OK${NC}: Line $line_num: ### $heading"
                fi
            else
                issues=$((issues + 1))
                echo -e "${RED}ERROR${NC}: Line $line_num: non-standard heading '### $heading'"

                # Provide specific guidance for common mistakes
                if [[ "$heading" == "Breaking" ]]; then
                    echo -e "  ${BLUE}Fix:${NC} Change '### Breaking' to '### Changed'"
                    echo -e "       and prefix each entry with '**Breaking:**' (entries may already have this prefix)."
                else
                    echo -e "  ${BLUE}Fix:${NC} Use one of the standard Keep a Changelog headings:"
                    echo -e "       Added, Changed, Deprecated, Removed, Fixed, Security"
                fi
                echo ""
            fi
        fi

    done < "$CHANGELOG"

    echo ""

    if [[ "$issues" -eq 0 ]]; then
        echo -e "${GREEN}SUCCESS: All $headings_checked section heading(s) use standard Keep a Changelog types.${NC}"
        echo -e "  Valid types: Added, Changed, Deprecated, Removed, Fixed, Security"
        exit 0
    fi

    echo -e "${RED}FAILED: $issues non-standard heading(s) found in CHANGELOG.md.${NC}"
    echo ""
    echo "All ### section headings within version entries must be one of:"
    echo "  Added, Changed, Deprecated, Removed, Fixed, Security"
    echo ""
    echo "For breaking changes, use '### Changed' with '**Breaking:**' prefix on each entry."
    echo "See https://keepachangelog.com/en/1.1.0/ for the full specification."
    exit 1
}

main "$@"
