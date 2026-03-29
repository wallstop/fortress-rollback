#!/bin/bash
# Derive Bounds Check for Fortress Rollback
#
# Detects overly-strict derive bounds on generic types. Specifically, flags
# cases where `Eq` is derived on a public struct/enum with generic type
# parameters but the generic bounds don't actually require `Eq`. This means
# the derive is silently adding `Eq` bounds beyond what the type needs,
# preventing it from being used with types that implement `PartialEq` but
# not `Eq`.
#
# Usage: ./scripts/ci/check-derive-bounds.sh [options]
#   ./scripts/ci/check-derive-bounds.sh            # Check all Rust files
#   ./scripts/ci/check-derive-bounds.sh --verbose  # Show all types checked
#   ./scripts/ci/check-derive-bounds.sh --help     # Show help
#
# Exit codes:
#   0 - No issues found
#   1 - Overly-strict derive bounds detected

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
    echo "  --verbose  Show all types checked"
    echo "  --help     Show this help message"
    echo ""
    echo "Detects overly-strict derive bounds on generic types."
    echo "Flags #[derive(Eq)] on public generic types whose bounds"
    echo "don't require Eq."
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
    echo " Derive Bounds Check"
    echo "=========================================="
    echo ""

    local issues=0
    local types_checked=0

    # Process all Rust source files under src/
    while IFS= read -r file; do
        [[ -z "$file" ]] && continue

        local rel_path="${file#"$PROJECT_ROOT/"}"
        local total_lines
        total_lines=$(wc -l < "$file")

        # Find lines with #[derive(] and collect the full derive text,
        # handling both single-line and multi-line derives.
        # Output format: "start_lineno:end_lineno:full derive text (joined on one line)"
        local derive_lines=""
        local derive_start_lines
        derive_start_lines=$(grep -n '#\[derive(' "$file" 2>/dev/null || true)
        [[ -z "$derive_start_lines" ]] && continue

        while IFS= read -r start_match; do
            [[ -z "$start_match" ]] && continue
            local start_lineno
            start_lineno=$(echo "$start_match" | cut -d: -f1)
            local start_text
            start_text=$(echo "$start_match" | cut -d: -f2-)

            local full_derive_text="$start_text"
            local derive_end_lineno="$start_lineno"
            # If the closing )] is not on the same line, collect subsequent lines
            if ! echo "$start_text" | grep -qF ')'; then
                local scan_line=$((start_lineno + 1))
                while [[ "$scan_line" -le "$total_lines" ]]; do
                    local next_line
                    next_line=$(sed -n "${scan_line}p" "$file")
                    full_derive_text="$full_derive_text $next_line"
                    if echo "$next_line" | grep -qF ')'; then
                        derive_end_lineno="$scan_line"
                        break
                    fi
                    scan_line=$((scan_line + 1))
                done
            fi

            # Check if the full derive text contains standalone Eq (not just PartialEq)
            local without_partial_check
            without_partial_check=$(echo "$full_derive_text" | sed 's/PartialEq//g')
            if echo "$without_partial_check" | grep -qE '\bEq\b'; then
                # Format: start_lineno:end_lineno:full derive text
                derive_lines+="${start_lineno}:${derive_end_lineno}:${full_derive_text}"$'\n'
            fi
        done <<< "$derive_start_lines"

        # Trim trailing newline and skip if empty
        derive_lines=$(echo "$derive_lines" | sed '/^$/d')
        [[ -z "$derive_lines" ]] && continue

        while IFS= read -r derive_match; do
            [[ -z "$derive_match" ]] && continue

            local derive_lineno
            derive_lineno=$(echo "$derive_match" | cut -d: -f1)
            local derive_end
            derive_end=$(echo "$derive_match" | cut -d: -f2)
            local derive_text
            derive_text=$(echo "$derive_match" | cut -d: -f3-)

            # Skip if this derive line doesn't actually contain standalone Eq
            # (not just PartialEq). We need Eq as a separate token.
            # Remove PartialEq first, then check for Eq
            local without_partial
            without_partial=$(echo "$derive_text" | sed 's/PartialEq//g')
            if ! echo "$without_partial" | grep -qE '\bEq\b'; then
                continue
            fi

            # Skip if the derive line has a "derive-bounds:ok" suppression comment.
            # Use this for types where Eq is intentional despite no explicit bounds
            # (e.g., types always used with Config::Input which requires Eq).
            if echo "$derive_text" | grep -qF 'derive-bounds:ok'; then
                if [[ "$VERBOSE" == "true" ]]; then
                    echo -e "  ${GREEN}SKIP${NC}: $rel_path:$derive_lineno (suppressed via derive-bounds:ok)"
                fi
                continue
            fi

            # Look at the next few lines after the derive for a pub struct/enum
            # with generics. Use derive_end to skip past multi-line derives.
            local search_end=$((derive_end + 5))
            if [[ "$search_end" -gt "$total_lines" ]]; then
                search_end="$total_lines"
            fi

            local following
            following=$(sed -n "$((derive_end + 1)),${search_end}p" "$file")

            # Check for pub struct/enum with generic parameters <...>
            local type_line
            type_line=$(echo "$following" | grep -E '^\s*pub\s+(struct|enum)\s+\w+\s*<' | head -1 || true)

            if [[ -z "$type_line" ]]; then
                if [[ "$VERBOSE" == "true" ]]; then
                    echo -e "  ${GREEN}SKIP${NC}: $rel_path:$derive_lineno (not a public generic type)"
                fi
                continue
            fi

            types_checked=$((types_checked + 1))

            # Extract the type name
            local type_name
            type_name=$(echo "$type_line" | sed -E 's/^\s*pub\s+(struct|enum)\s+(\w+).*/\2/')

            # Now check if there's a where clause or inline bounds requiring Eq
            # We look at the type definition and any where clause up to the opening brace
            local def_start=$((derive_end + 1))
            local def_end=$((derive_end + 15))
            if [[ "$def_end" -gt "$total_lines" ]]; then
                def_end="$total_lines"
            fi

            local type_block
            type_block=$(sed -n "${def_start},${def_end}p" "$file")

            # Flatten the block into a single line for multi-line where clause matching
            local flat_block
            flat_block=$(echo "$type_block" | tr '\n' ' ')

            # Check if Eq appears in bounds (where clause or inline bounds)
            # Look for patterns like: T: Eq, T: ... + Eq, where ... Eq
            local has_eq_bound=false

            # Check inline bounds on the generic parameter, e.g. <I: Eq> or <I: Foo + Eq>
            if echo "$flat_block" | grep -qE '<[^>]*\bEq\b[^>]*>'; then
                has_eq_bound=true
            fi

            # Check where clause for Eq bound (flattened, so multi-line where clauses work)
            if echo "$flat_block" | grep -qE '\bwhere\b.*\bEq\b'; then
                has_eq_bound=true
            fi

            # Check if generic parameter is bounded by a trait whose associated types
            # require Eq (e.g., T: Config where Config::Address: Eq). We recognize
            # the Config trait specifically since it's this crate's main trait.
            if echo "$flat_block" | grep -qE '\b\w+\s*:\s*Config\b'; then
                has_eq_bound=true
            fi

            if [[ "$has_eq_bound" == "true" ]]; then
                if [[ "$VERBOSE" == "true" ]]; then
                    echo -e "  ${GREEN}OK${NC}: $rel_path:$derive_lineno $type_name (Eq bound present)"
                fi
            else
                issues=$((issues + 1))
                echo ""
                echo -e "${RED}ERROR${NC}: $rel_path:$derive_lineno"
                echo -e "  Type ${YELLOW}${type_name}${NC} derives Eq but its generic bounds don't require Eq."
                echo -e "  ${BLUE}Derive line:${NC} $derive_text"
                echo -e "  ${BLUE}Type line:${NC}  $(echo "$type_line" | sed 's/^\s*//')"
                echo -e "  ${BLUE}Fix:${NC} Remove Eq from the derive, or add Eq to the generic bounds."
                echo -e "       Deriving Eq on a generic type adds an implicit Eq bound on all"
                echo -e "       type parameters, which may be stricter than necessary."
            fi

        done <<< "$derive_lines"

    done < <(find "$PROJECT_ROOT/src" -name '*.rs' -print 2>/dev/null | sort)

    echo ""

    if [[ "$issues" -eq 0 ]]; then
        echo -e "${GREEN}SUCCESS: No overly-strict derive bounds found.${NC}"
        if [[ "$types_checked" -gt 0 ]]; then
            echo -e "  ($types_checked public generic type(s) with Eq checked)"
        fi
        exit 0
    fi

    echo -e "${RED}FAILED: $issues type(s) have overly-strict derive bounds.${NC}"
    echo ""
    echo "When a generic type derives Eq, it adds an implicit I: Eq bound."
    echo "If the type's explicit bounds only require PartialEq, the Eq derive"
    echo "is overly strict and prevents use with PartialEq-only types."
    exit 1
}

main "$@"
