#!/bin/bash
# Shell Script Portability Lint for Fortress Rollback
#
# Checks all .sh files in the repository for common cross-platform portability
# issues that break on macOS, BSD, or other non-GNU systems.
#
# Usage: ./scripts/ci/check-shell-portability.sh [options]
#   ./scripts/ci/check-shell-portability.sh            # Check all scripts
#   ./scripts/ci/check-shell-portability.sh --verbose  # Show all files checked
#   ./scripts/ci/check-shell-portability.sh --help     # Show help
#
# Exit codes:
#   0 - No portability issues found
#   1 - Portability issues detected

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
    echo "Checks .sh files for cross-platform portability issues."
    echo "Excludes target/, node_modules/, .git/, and .venv/ directories."
}

# Collect all .sh files, excluding build artifacts and vendored directories
collect_shell_files() {
    find "$PROJECT_ROOT" \
        -path "$PROJECT_ROOT/target" -prune -o \
        -path "$PROJECT_ROOT/node_modules" -prune -o \
        -path "$PROJECT_ROOT/.git" -prune -o \
        -path "$PROJECT_ROOT/.venv" -prune -o \
        -name '*.sh' -print \
        | sort
}

# Check a single file for portability issues.
# Appends findings to the ISSUES array (global).
# Arguments: $1 = file path
check_file() {
    local file="$1"
    local rel_path="${file#"$PROJECT_ROOT/"}"
    local line_num=0

    # Regex patterns stored in variables to avoid quoting issues in [[ =~ ]]
    local timeout_start='^[[:space:]]*timeout[[:space:]]'
    local timeout_after_sep='[;&|][[:space:]]*timeout[[:space:]]'
    local timeout_subshell='[$][(]timeout[[:space:]]'

    while IFS= read -r line; do
        line_num=$((line_num + 1))

        # Skip comment-only lines (leading whitespace + #)
        if [[ "$line" =~ ^[[:space:]]*# ]]; then
            continue
        fi

        # Skip lines that are purely string literals (echo, printf, array assignments
        # used for reporting). These contain pattern keywords as descriptive text, not
        # as actual command invocations.
        if [[ "$line" =~ ^[[:space:]]*(echo|printf)[[:space:]] ]]; then
            continue
        fi
        if [[ "$line" =~ ^[[:space:]]*(ISSUES|DETAILS|SUGGESTIONS)\+= ]]; then
            continue
        fi

        # (a) grep --include or grep --exclude (GNU-specific)
        if [[ "$line" =~ grep[[:space:]].*--include ]] || [[ "$line" =~ grep[[:space:]].*--exclude ]]; then
            ISSUES+=("${rel_path}:${line_num}: GNU-specific grep flag")
            DETAILS+=("  ${YELLOW}Line:${NC} $line")
            SUGGESTIONS+=("  ${BLUE}Fix:${NC} Use 'find ... -exec grep' instead of grep --include/--exclude")
        fi

        # (b) grep -P or grep -oP (Perl regex, not available on macOS/BSD)
        # Match grep with flags containing P (e.g., -P, -oP, -Po, -cP, etc.)
        if [[ "$line" =~ grep[[:space:]]+-[a-zA-Z]*P ]]; then
            ISSUES+=("${rel_path}:${line_num}: Perl regex grep flag (-P)")
            DETAILS+=("  ${YELLOW}Line:${NC} $line")
            SUGGESTIONS+=("  ${BLUE}Fix:${NC} Use 'grep -E' with 'sed' for post-processing instead of grep -P")
        fi

        # (c) Hardcoded /bin/sed or /usr/bin/sed paths
        if [[ "$line" =~ /bin/sed[[:space:]] ]] || [[ "$line" =~ /usr/bin/sed[[:space:]] ]]; then
            ISSUES+=("${rel_path}:${line_num}: Hardcoded sed path")
            DETAILS+=("  ${YELLOW}Line:${NC} $line")
            SUGGESTIONS+=("  ${BLUE}Fix:${NC} Use bare 'sed' and rely on PATH resolution")
        fi

        # (d) Direct 'timeout' calls not inside a run_with_timeout function definition
        # Only match 'timeout' when used as a command (at line start or after ; && || |)
        # not when it appears as a string argument to another command.
        if [[ "$line" =~ $timeout_start ]] || \
           [[ "$line" =~ $timeout_after_sep ]] || \
           [[ "$line" =~ $timeout_subshell ]]; then
            # Allow if we're inside a run_with_timeout function body.
            # Heuristic: check if this file defines run_with_timeout and the line
            # is between the function header and its closing brace.
            local in_helper=false
            if grep -q 'run_with_timeout()' "$file" 2>/dev/null; then
                # Get the line range of the run_with_timeout function
                local func_start
                func_start=$(grep -n 'run_with_timeout()' "$file" | head -1 | cut -d: -f1)
                if [[ -n "$func_start" ]] && [[ "$line_num" -ge "$func_start" ]]; then
                    # Check if we're within ~30 lines of the function start (reasonable body size)
                    local func_end=$((func_start + 50))
                    if [[ "$line_num" -le "$func_end" ]]; then
                        in_helper=true
                    fi
                fi
            fi
            if [[ "$in_helper" == "false" ]]; then
                ISSUES+=("${rel_path}:${line_num}: Direct 'timeout' call (not portable)")
                DETAILS+=("  ${YELLOW}Line:${NC} $line")
                SUGGESTIONS+=("  ${BLUE}Fix:${NC} Wrap in a portability function like 'run_with_timeout' (see verify-sccache.sh)")
            fi
        fi

        # (e) readlink -f (not available on macOS)
        if [[ "$line" =~ readlink[[:space:]]+-f[[:space:]] ]] || [[ "$line" =~ readlink[[:space:]]+-f$ ]]; then
            ISSUES+=("${rel_path}:${line_num}: readlink -f (not available on macOS)")
            DETAILS+=("  ${YELLOW}Line:${NC} $line")
            SUGGESTIONS+=("  ${BLUE}Fix:${NC} Use 'realpath' or a manual workaround: \$(cd \"\$(dirname \"\$path\")\" && pwd)/\$(basename \"\$path\")")
        fi

        # (f) sed -i without .bak suffix (GNU/BSD incompatibility)
        # GNU sed: sed -i 's/...' works. BSD sed: sed -i '' 's/...' required.
        # Using sed -i .bak 's/...' works on both.
        if [[ "$line" =~ sed[[:space:]]+-i[[:space:]] ]]; then
            # Check it's not sed -i .bak or sed -i'.bak' or sed -i '' (BSD form)
            if ! [[ "$line" =~ sed[[:space:]]+-i[[:space:]]*\. ]] && \
               ! [[ "$line" =~ sed[[:space:]]+-i[[:space:]]*\'\'[[:space:]] ]] && \
               ! [[ "$line" =~ sed[[:space:]]+-i[[:space:]]*\"\"[[:space:]] ]]; then
                ISSUES+=("${rel_path}:${line_num}: sed -i without backup suffix (GNU/BSD difference)")
                DETAILS+=("  ${YELLOW}Line:${NC} $line")
                SUGGESTIONS+=("  ${BLUE}Fix:${NC} Use 'sed -i .bak' for portability, or use a helper function for GNU/BSD detection")
            fi
        fi

        # (g) date -d (GNU-specific, not available on macOS)
        if [[ "$line" =~ date[[:space:]]+-d[[:space:]] ]] || [[ "$line" =~ date[[:space:]]+-d\" ]] || [[ "$line" =~ date[[:space:]]+-d\' ]]; then
            ISSUES+=("${rel_path}:${line_num}: date -d (GNU-specific, not on macOS)")
            DETAILS+=("  ${YELLOW}Line:${NC} $line")
            SUGGESTIONS+=("  ${BLUE}Fix:${NC} Use 'date -j -f' on macOS or implement a cross-platform wrapper")
        fi

    done < "$file"
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
    echo " Shell Script Portability Check"
    echo "=========================================="
    echo ""

    # Collect files
    local files
    files=$(collect_shell_files)

    if [[ -z "$files" ]]; then
        echo -e "${YELLOW}No .sh files found to check.${NC}"
        exit 0
    fi

    local file_count
    file_count=$(echo "$files" | wc -l | tr -d ' ')
    echo -e "${BLUE}Scanning $file_count shell script(s) for portability issues...${NC}"
    echo ""

    # Global arrays for collecting issues
    ISSUES=()
    DETAILS=()
    SUGGESTIONS=()

    while IFS= read -r file; do
        [[ -z "$file" ]] && continue

        if [[ "$VERBOSE" == "true" ]]; then
            local rel_path="${file#"$PROJECT_ROOT/"}"
            echo -e "  Checking: $rel_path"
        fi

        check_file "$file"
    done <<< "$files"

    # Report results
    local issue_count=${#ISSUES[@]}

    if [[ "$issue_count" -eq 0 ]]; then
        echo ""
        echo -e "${GREEN}SUCCESS: No portability issues found in $file_count script(s).${NC}"
        exit 0
    fi

    echo ""
    echo -e "${RED}Found $issue_count portability issue(s):${NC}"
    echo ""

    local i
    for ((i = 0; i < issue_count; i++)); do
        echo -e "${RED}  ${ISSUES[$i]}${NC}"
        echo -e "${DETAILS[$i]}"
        echo -e "${SUGGESTIONS[$i]}"
        echo ""
    done

    echo -e "${RED}FAILED: $issue_count portability issue(s) detected in $file_count script(s).${NC}"
    echo ""
    echo "These patterns may break on macOS, BSD, or other non-GNU systems."
    echo "See each suggestion above for a portable alternative."
    exit 1
}

main "$@"
