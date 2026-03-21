#!/usr/bin/env bash
#
# check-llm-skills.sh - Validate .llm/skills/ markdown files for code quality issues
#
# Checks:
#   1. Rust code blocks with unguarded .unwrap()/.expect() (no justifying comment)
#   2. Ambiguous Result in collect (missing error type parameter)
#   3. catch_unwind without AssertUnwindSafe
#   4. Line count enforcement (300 lines max for all .llm/*.md files)
#   5. 2>/dev/null in bash/shell code blocks (banned pattern)
#
# Usage:
#   ./scripts/docs/check-llm-skills.sh              # Check .llm/ directory
#   ./scripts/docs/check-llm-skills.sh --fix        # Print fix suggestions
#   ./scripts/docs/check-llm-skills.sh --dir /path  # Check a custom directory
#
# Exit codes:
#   0 - No issues found
#   1 - Violations detected

set -euo pipefail

# ── Colors ────────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# ── Globals ───────────────────────────────────────────────────────────────────
VIOLATIONS=0
FIX_MODE=0
SEARCH_DIR=""
MAX_LINES=300

# ── Argument parsing ─────────────────────────────────────────────────────────
parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --fix)
                FIX_MODE=1
                shift
                ;;
            --dir)
                if [[ $# -lt 2 ]]; then
                    echo -e "${RED}ERROR: --dir requires an argument${NC}" >&2
                    exit 1
                fi
                SEARCH_DIR="$2"
                shift 2
                ;;
            -h|--help)
                echo "Usage: $0 [--fix] [--dir <directory>]"
                echo ""
                echo "Options:"
                echo "  --fix     Print fix suggestions for each violation"
                echo "  --dir     Directory to scan (default: .llm/)"
                echo "  -h        Show this help"
                exit 0
                ;;
            *)
                echo -e "${RED}ERROR: Unknown argument: $1${NC}" >&2
                exit 1
                ;;
        esac
    done

    # Default: derive .llm/ relative to script location
    if [[ -z "$SEARCH_DIR" ]]; then
        SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
        SEARCH_DIR="$REPO_ROOT/.llm"
    fi
}

# ── Helpers ───────────────────────────────────────────────────────────────────
report_violation() {
    local file="$1"
    local line_num="$2"
    local check_name="$3"
    local message="$4"
    VIOLATIONS=$((VIOLATIONS + 1))
    echo -e "  ${RED}[$check_name]${NC} ${file}:${line_num}: ${message}"
}

suggest_fix() {
    local suggestion="$1"
    if [[ "$FIX_MODE" -eq 1 ]]; then
        echo -e "    ${CYAN}FIX:${NC} $suggestion"
    fi
}

# ── Check 1: Unguarded .unwrap()/.expect() in Rust code blocks ───────────────
# Allowed justification comments (on the same line or within a few preceding
# lines in the same code block):
#   // build.rs:    // test:    // Loom test:    // Fuzz target(s):
#   // proptest:    // allowed:    // SAFETY:     // In tests:
#   #[allow(       #[test]    #[fixture]    #[cfg(test)]
#
# Lines that are pure Rust comments (// ...) are skipped entirely -- mentioning
# .unwrap() in a comment is not executable code.
#
# The markdown line immediately before the code fence is also checked for
# "forbidden" context (e.g., "### Forbidden Patterns") to skip negative examples.
check_unguarded_unwrap() {
    local file="$1"
    local in_rust_block=0
    local line_num=0
    local rust_block_start=0
    # Justification tracking: line number of the last justifying comment/attribute
    local last_justification_at=0
    # Maximum distance (in lines) a justification comment can cover
    local justification_reach=8
    # The markdown line immediately before the code fence
    local pre_fence_line=""
    # Whether the current block is a "forbidden patterns" demonstration
    local forbidden_block=0

    while IFS= read -r line; do
        line_num=$((line_num + 1))

        # Detect start of a rust code block
        if [[ "$line" =~ ^\`\`\`rust ]]; then
            in_rust_block=1
            rust_block_start=$line_num
            last_justification_at=0
            forbidden_block=0
            # Check if the markdown context before the fence indicates forbidden/negative examples
            if echo "$pre_fence_line" | grep -qiE '(forbidden|wrong|bad example|what not to do|never use|do not use)'; then
                forbidden_block=1
            fi
            continue
        fi

        # Track the last non-blank line before a code fence (markdown context)
        if [[ "$in_rust_block" -eq 0 ]]; then
            local trimmed_check
            trimmed_check="$(echo "$line" | sed 's/^[[:space:]]*//')"
            if [[ -n "$trimmed_check" ]] && [[ ! "$trimmed_check" =~ ^\`\`\` ]]; then
                pre_fence_line="$line"
            fi
        fi

        # Detect end of code block
        if [[ "$in_rust_block" -eq 1 ]] && [[ "$line" =~ ^\`\`\` ]]; then
            in_rust_block=0
            continue
        fi

        if [[ "$in_rust_block" -eq 1 ]]; then
            # Skip entire block if it's a forbidden-patterns demonstration
            if [[ "$forbidden_block" -eq 1 ]]; then
                continue
            fi

            # Check if the current line is a pure Rust comment
            if echo "$line" | grep -qE '^\s*//'; then
                # Track justification comments for nearby code lines
                if echo "$line" | grep -qE '//\s*(build\.rs:|test:|Loom test:|Fuzz target|proptest:|allowed:|SAFETY:|In tests:)'; then
                    last_justification_at=$line_num
                fi
                # Skip -- mentioning .unwrap() in a comment is not executable code
                continue
            fi

            # Check if the current line is an accepted attribute
            if echo "$line" | grep -qE '^\s*#\[(allow\(|test|fixture|cfg\(test\))'; then
                last_justification_at=$line_num
            fi

            # Check for .unwrap() or .expect(
            if echo "$line" | grep -qE '\.(unwrap|expect)\('; then
                local justified=0

                # Check if there's a justifying comment on the same line
                if echo "$line" | grep -qE '//\s*(build\.rs:|test:|Loom test:|Fuzz target|proptest:|allowed:|SAFETY:|In tests:)'; then
                    justified=1
                fi

                # Check if the same line has an allow attribute inline
                if [[ "$justified" -eq 0 ]]; then
                    if echo "$line" | grep -qE '#\[allow\('; then
                        justified=1
                    fi
                fi

                # Check if a recent justification comment/attribute covers this line
                if [[ "$justified" -eq 0 ]] && [[ "$last_justification_at" -gt 0 ]]; then
                    local distance=$((line_num - last_justification_at))
                    if [[ "$distance" -le "$justification_reach" ]]; then
                        justified=1
                    fi
                fi

                if [[ "$justified" -eq 0 ]]; then
                    local trimmed
                    trimmed="$(echo "$line" | sed 's/^[[:space:]]*//' | head -c 100)"
                    report_violation "$file" "$line_num" "unwrap" \
                        ".unwrap()/.expect() without justifying comment: $trimmed"
                    suggest_fix "Add a comment like: // test: or // allowed: <reason>"
                fi
            fi
        fi
    done < "$file"
}

# ── Check 2: Ambiguous Result in collect ──────────────────────────────────────
check_ambiguous_collect() {
    local file="$1"
    local in_rust_block=0
    local line_num=0

    while IFS= read -r line; do
        line_num=$((line_num + 1))

        if [[ "$line" =~ ^\`\`\`rust ]]; then
            in_rust_block=1
            continue
        fi
        if [[ "$in_rust_block" -eq 1 ]] && [[ "$line" =~ ^\`\`\` ]]; then
            in_rust_block=0
            continue
        fi

        if [[ "$in_rust_block" -eq 1 ]]; then
            # Match collect::<Result< that lacks a second type parameter
            # Pattern: collect::<Result<SomeType>> without a comma separating two type params
            # We look for collect::<Result<...>> where ... has no comma (meaning no error type)
            if echo "$line" | grep -qE 'collect::<Result<[^,>]+>>' ; then
                local trimmed
                trimmed="$(echo "$line" | sed 's/^[[:space:]]*//' | head -c 100)"
                report_violation "$file" "$line_num" "ambiguous-result" \
                    "collect::<Result<...>> missing explicit error type: $trimmed"
                suggest_fix "Specify the error type: collect::<Result<T, E>>()"
            fi
        fi
    done < "$file"
}

# ── Check 3: catch_unwind without AssertUnwindSafe ────────────────────────────
check_catch_unwind() {
    local file="$1"
    local in_rust_block=0
    local line_num=0
    local rust_block=""
    local rust_block_start=0

    while IFS= read -r line; do
        line_num=$((line_num + 1))

        if [[ "$line" =~ ^\`\`\`rust ]]; then
            in_rust_block=1
            rust_block=""
            rust_block_start=$line_num
            continue
        fi
        if [[ "$in_rust_block" -eq 1 ]] && [[ "$line" =~ ^\`\`\` ]]; then
            # Check the entire block for catch_unwind without AssertUnwindSafe
            if echo "$rust_block" | grep -q 'catch_unwind'; then
                if ! echo "$rust_block" | grep -q 'AssertUnwindSafe'; then
                    # Find the specific line with catch_unwind
                    local block_line_num=$rust_block_start
                    while IFS= read -r block_line; do
                        block_line_num=$((block_line_num + 1))
                        if echo "$block_line" | grep -q 'catch_unwind'; then
                            local trimmed
                            trimmed="$(echo "$block_line" | sed 's/^[[:space:]]*//' | head -c 100)"
                            report_violation "$file" "$block_line_num" "catch-unwind" \
                                "catch_unwind without AssertUnwindSafe: $trimmed"
                            suggest_fix "Wrap the closure: catch_unwind(AssertUnwindSafe(|| { ... }))"
                        fi
                    done <<< "$rust_block"
                fi
            fi
            in_rust_block=0
            continue
        fi

        if [[ "$in_rust_block" -eq 1 ]]; then
            rust_block="$rust_block
$line"
        fi
    done < "$file"
}

# ── Check 4: Line count enforcement ──────────────────────────────────────────
check_line_count() {
    local file="$1"
    local count
    count=$(wc -l < "$file")
    # Remove leading whitespace that wc may produce
    count=$(echo "$count" | tr -d '[:space:]')

    if [[ "$count" -gt "$MAX_LINES" ]]; then
        local over=$((count - MAX_LINES))
        report_violation "$file" "$count" "line-limit" \
            "File has $count lines ($over over the $MAX_LINES-line limit)"
        suggest_fix "Split into smaller focused documents or trim content"
    fi
}

# ── Check 5: 2>/dev/null in bash/shell code blocks ───────────────────────────
check_dev_null() {
    local file="$1"
    local in_shell_block=0
    local line_num=0

    while IFS= read -r line; do
        line_num=$((line_num + 1))

        if [[ "$line" =~ ^\`\`\`(bash|shell|sh)$ ]] || [[ "$line" =~ ^\`\`\`(bash|shell|sh)[[:space:]] ]]; then
            in_shell_block=1
            continue
        fi
        if [[ "$in_shell_block" -eq 1 ]] && [[ "$line" =~ ^\`\`\` ]]; then
            in_shell_block=0
            continue
        fi

        if [[ "$in_shell_block" -eq 1 ]]; then
            if echo "$line" | grep -qF '2>/dev/null'; then
                local trimmed
                trimmed="$(echo "$line" | sed 's/^[[:space:]]*//' | head -c 100)"
                report_violation "$file" "$line_num" "dev-null" \
                    "2>/dev/null in shell code block (banned): $trimmed"
                suggest_fix "Remove 2>/dev/null; handle errors explicitly or use '|| true'"
            fi
        fi
    done < "$file"
}

# ── Main ──────────────────────────────────────────────────────────────────────
main() {
    parse_args "$@"

    if [[ ! -d "$SEARCH_DIR" ]]; then
        echo -e "${YELLOW}Warning: Directory '$SEARCH_DIR' does not exist, skipping${NC}"
        exit 0
    fi

    echo -e "${BOLD}Checking .llm/ markdown files for code quality issues...${NC}"
    echo ""

    # Collect all .md files under the search directory
    local md_files=()
    while IFS= read -r -d '' f; do
        md_files+=("$f")
    done < <(find "$SEARCH_DIR" -name "*.md" -type f -print0 | sort -z)

    if [[ ${#md_files[@]} -eq 0 ]]; then
        echo -e "${YELLOW}No .md files found under $SEARCH_DIR${NC}"
        exit 0
    fi

    echo "Scanning ${#md_files[@]} markdown file(s)..."
    echo ""

    for file in "${md_files[@]}"; do
        check_unguarded_unwrap "$file"
        check_ambiguous_collect "$file"
        check_catch_unwind "$file"
        check_line_count "$file"
        check_dev_null "$file"
    done

    echo ""

    if [[ "$VIOLATIONS" -gt 0 ]]; then
        echo -e "${RED}${BOLD}Found $VIOLATIONS violation(s)${NC}"
        if [[ "$FIX_MODE" -eq 0 ]]; then
            echo -e "Run with ${CYAN}--fix${NC} to see fix suggestions"
        fi
        exit 1
    else
        echo -e "${GREEN}All checks passed${NC}"
        exit 0
    fi
}

main "$@"
