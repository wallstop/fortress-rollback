#!/bin/bash
# Version Synchronization Script for Fortress Rollback
#
# This script ensures all version references in the codebase are consistent
# with the version declared in Cargo.toml.
#
# Usage:
#   ./scripts/sync-version.sh           # Check and update versions
#   ./scripts/sync-version.sh --check   # Check only (exit 1 if inconsistent)
#   ./scripts/sync-version.sh --dry-run # Show what would be changed
#   ./scripts/sync-version.sh --verbose # Show detailed output
#
# ═══════════════════════════════════════════════════════════════════════════════
# FILES SCANNED (comprehensive coverage)
# ═══════════════════════════════════════════════════════════════════════════════
#
# Source Code:
#   - Rust files (*.rs) - including doc comments (/// and //!)
#   - Shell scripts (*.sh)
#
# Documentation:
#   - Markdown files (*.md)
#   - Text files (*.txt)
#
# Configuration:
#   - TOML files (*.toml) - excluding Cargo.toml and Cargo.lock
#   - YAML/YML files (*.yml, *.yaml) - CI/CD workflows, etc.
#   - JSON files (*.json)
#
# Progress/Notes:
#   - Progress directory markdown files
#
# ═══════════════════════════════════════════════════════════════════════════════
# PATTERNS MATCHED
# ═══════════════════════════════════════════════════════════════════════════════
#
# Pattern 1 - Simple dependency:
#   fortress-rollback = "0.2"
#   fortress-rollback = "0.2.0"
#
# Pattern 2 - Dependency with features/options:
#   fortress-rollback = { version = "0.2", features = ["tokio"] }
#   fortress-rollback = { version = "0.2.0", features = ["sync-send"] }
#
# Pattern 3 - CHANGELOG.md link footers:
#   [Unreleased]: https://github.com/.../compare/v0.2.0...HEAD
#   [0.2.0]: https://github.com/.../compare/v0.1.0...v0.2.0
#
# These patterns are matched in ALL contexts including:
#   - Rust doc comments (/// and //!)
#   - Regular Rust comments (//)
#   - Markdown code blocks
#   - TOML dependency sections
#   - YAML configuration
#   - Any other text file
#
# ═══════════════════════════════════════════════════════════════════════════════

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
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Flags
CHECK_ONLY=false
DRY_RUN=false
VERBOSE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --check)
            CHECK_ONLY=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [options]"
            echo ""
            echo "Synchronize all fortress-rollback version references with Cargo.toml"
            echo ""
            echo "Options:"
            echo "  --check     Check only, exit 1 if versions are inconsistent"
            echo "  --dry-run   Show what would be changed without modifying files"
            echo "  --verbose   Show detailed output including all files scanned"
            echo "  --help      Show this help message"
            echo ""
            echo "File types scanned:"
            echo "  *.rs        Rust source files (including doc comments)"
            echo "  *.md        Markdown documentation"
            echo "  *.toml      TOML config files (except Cargo.toml/Cargo.lock)"
            echo "  *.yml,yaml  CI/CD workflows and YAML configs"
            echo "  *.sh        Shell scripts"
            echo "  *.txt       Text files"
            echo "  *.json      JSON files"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Extract version from Cargo.toml
get_cargo_version() {
    local version
    version=$(grep -E '^version = "[0-9]+\.[0-9]+(\.[0-9]+)?"' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed -E 's/version = "([^"]+)"/\1/')
    if [[ -z "$version" ]]; then
        echo -e "${RED}Error: Could not extract version from Cargo.toml${NC}" >&2
        exit 1
    fi
    echo "$version"
}

# Get major.minor version (for patterns like "0.2")
get_major_minor_version() {
    local full_version="$1"
    echo "$full_version" | sed -E 's/^([0-9]+\.[0-9]+).*/\1/'
}

# Log function (only prints in verbose mode)
log() {
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "$1"
    fi
}

# Log always (prints regardless of verbose mode)
log_always() {
    echo -e "$1"
}

# Check if a file should be excluded
should_exclude_file() {
    local file="$1"
    local basename
    basename=$(basename "$file")

    # Exclude specific files
    case "$basename" in
        Cargo.toml|Cargo.lock|sync-version.sh)
            return 0  # true = exclude
            ;;
    esac

    # Exclude paths containing these directories
    case "$file" in
        */target/*|*/.git/*|*/node_modules/*|*/.tla-tools/*|*/site/*|*/proptest-regressions/*|*/mutants.out*/*|*/.venv/*|*/fuzz/target/*)
            return 0  # true = exclude
            ;;
    esac

    return 1  # false = don't exclude
}

# De-duplicate when adding files to the INCONSISTENT_FILES array
# (called from main(); accesses main()'s local INCONSISTENT_FILES via shared scope)
add_inconsistent_file() {
    local relative_path="$1"
    local already_listed=false
    for f in "${INCONSISTENT_FILES[@]+"${INCONSISTENT_FILES[@]}"}"; do
        if [[ "$f" == "$relative_path" ]]; then
            already_listed=true
            break
        fi
    done
    if [[ "$already_listed" == "false" ]]; then
        INCONSISTENT_FILES+=("$relative_path")
    fi
}

# Main logic
main() {
    cd "$PROJECT_ROOT"

    echo -e "${CYAN}╔════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║         Fortress Rollback Version Synchronization                      ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Get current version
    local VERSION
    VERSION=$(get_cargo_version)
    local MAJOR_MINOR
    MAJOR_MINOR=$(get_major_minor_version "$VERSION")

    echo -e "${BLUE}Current Cargo.toml version:${NC} ${GREEN}$VERSION${NC}"
    echo -e "${BLUE}Major.Minor version:${NC} ${GREEN}$MAJOR_MINOR${NC}"
    echo ""

    local FILES_CHANGED=0
    local TOTAL_REPLACEMENTS=0
    local INCONSISTENT_FILES=()
    local SCANNED_COUNT=0

    # ═══════════════════════════════════════════════════════════════════════════
    # File Discovery - Comprehensive file type coverage
    # ═══════════════════════════════════════════════════════════════════════════

    log "${MAGENTA}Discovering files to scan...${NC}"

    # Find all relevant files
    # Extensions: .rs .md .toml .yml .yaml .sh .txt .json
    local FILES_TO_SCAN
    FILES_TO_SCAN=$(find "$PROJECT_ROOT" \
        -type f \( \
            -name "*.rs" \
            -o -name "*.md" \
            -o -name "*.toml" \
            -o -name "*.yml" \
            -o -name "*.yaml" \
            -o -name "*.sh" \
            -o -name "*.txt" \
            -o -name "*.json" \
        \) \
        ! -path "*/target/*" \
        ! -path "*/.git/*" \
        ! -path "*/node_modules/*" \
        ! -path "*/.tla-tools/*" \
        ! -path "*/site/*" \
        ! -path "*/proptest-regressions/*" \
        ! -path "*/mutants.out*/*" \
        ! -path "*/.venv/*" \
        ! -path "*/fuzz/target/*" \
        ! -name "Cargo.toml" \
        ! -name "Cargo.lock" \
        2>/dev/null | sort || true)

    # ═══════════════════════════════════════════════════════════════════════════
    # Pattern Definitions
    # ═══════════════════════════════════════════════════════════════════════════

    # Pattern 1: Simple dependency declaration
    # Examples:
    #   fortress-rollback = "0.2"
    #   fortress-rollback = "0.2.0"
    local PATTERN1='fortress-rollback = "[0-9]+\.[0-9]+(\.[0-9]+)?"'

    # Pattern 2: Dependency with features/options (inline table)
    # Examples:
    #   fortress-rollback = { version = "0.2", features = ["tokio"] }
    #   /// fortress-rollback = { version = "0.2", features = ["tokio"] }
    #   //! fortress-rollback = { version = "0.2", features = ["tokio"] }
    local PATTERN2='fortress-rollback = \{ version = "[0-9]+\.[0-9]+(\.[0-9]+)?"'

    echo -e "${BLUE}Scanning files for version references...${NC}"
    echo ""

    # ═══════════════════════════════════════════════════════════════════════════
    # File Processing
    # ═══════════════════════════════════════════════════════════════════════════

    while IFS= read -r file; do
        [[ -z "$file" ]] && continue
        [[ ! -f "$file" ]] && continue

        # Skip excluded files
        if should_exclude_file "$file"; then
            continue
        fi

        ((SCANNED_COUNT++)) || true

        local file_changed=false
        local relative_path="${file#$PROJECT_ROOT/}"
        local file_ext="${file##*.}"

        log "  Scanning: $relative_path"

        # ─────────────────────────────────────────────────────────────────────
        # Check for Pattern 1: fortress-rollback = "X.Y.Z"
        # ─────────────────────────────────────────────────────────────────────
        if grep -qE "$PATTERN1" "$file" 2>/dev/null; then
            local matches
            matches=$(grep -cE "$PATTERN1" "$file" 2>/dev/null | tr -d '[:space:]' || true)
            matches=${matches:-0}

            # Check if any don't match the current version
            local current_matches
            current_matches=$(grep -E "$PATTERN1" "$file" 2>/dev/null | grep -cF "\"$MAJOR_MINOR\"" | tr -d '[:space:]' || true)
            current_matches=${current_matches:-0}

            if [[ "$matches" != "$current_matches" ]]; then
                log "${YELLOW}  → Found outdated version (pattern 1): $relative_path${NC}"

                add_inconsistent_file "$relative_path"

                if [[ "$CHECK_ONLY" == "false" ]]; then
                    if [[ "$DRY_RUN" == "true" ]]; then
                        echo -e "${YELLOW}Would update:${NC} $relative_path ${MAGENTA}($file_ext)${NC}"
                        grep -nE "$PATTERN1" "$file" 2>/dev/null | while read -r line; do
                            echo -e "  ${CYAN}$line${NC}"
                        done
                    else
                        # Replace the version, keeping the simple format
                        # Use sd (modern sed replacement) for portable cross-platform compatibility
                        sd 'fortress-rollback = "[0-9]+\.[0-9]+(\.[0-9]+)?"' "fortress-rollback = \"$MAJOR_MINOR\"" "$file"
                        file_changed=true
                        local diff_count=$((matches - current_matches))
                        TOTAL_REPLACEMENTS=$((TOTAL_REPLACEMENTS + diff_count))
                    fi
                fi
            fi
        fi

        # ─────────────────────────────────────────────────────────────────────
        # Check for Pattern 2: fortress-rollback = { version = "X.Y.Z"
        # This covers both TOML tables and doc comments
        # ─────────────────────────────────────────────────────────────────────
        if grep -qE "$PATTERN2" "$file" 2>/dev/null; then
            local matches
            matches=$(grep -cE "$PATTERN2" "$file" 2>/dev/null | tr -d '[:space:]' || true)
            matches=${matches:-0}

            # Check if any don't match the current version
            local current_matches
            current_matches=$(grep -E "$PATTERN2" "$file" 2>/dev/null | grep -cF "version = \"$MAJOR_MINOR\"" | tr -d '[:space:]' || true)
            current_matches=${current_matches:-0}

            if [[ "$matches" != "$current_matches" ]]; then
                log "${YELLOW}  → Found outdated version (pattern 2): $relative_path${NC}"

                add_inconsistent_file "$relative_path"

                if [[ "$CHECK_ONLY" == "false" ]]; then
                    if [[ "$DRY_RUN" == "true" ]]; then
                        echo -e "${YELLOW}Would update:${NC} $relative_path ${MAGENTA}($file_ext)${NC}"
                        grep -nE "$PATTERN2" "$file" 2>/dev/null | while read -r line; do
                            echo -e "  ${CYAN}$line${NC}"
                        done
                    else
                        # Replace the version in the complex format
                        # Use sd (modern sed replacement) for portable cross-platform compatibility
                        sd '(fortress-rollback = \{ version = ")[0-9]+\.[0-9]+(\.[0-9]+)?(")' "\${1}$MAJOR_MINOR\${3}" "$file"
                        file_changed=true
                        local diff_count=$((matches - current_matches))
                        TOTAL_REPLACEMENTS=$((TOTAL_REPLACEMENTS + diff_count))
                    fi
                fi
            fi
        fi

        if [[ "$file_changed" == "true" ]]; then
            ((FILES_CHANGED++)) || true
            echo -e "${GREEN}✓ Updated:${NC} $relative_path ${MAGENTA}($file_ext)${NC}"
        fi

    done <<< "$FILES_TO_SCAN"

    # ═══════════════════════════════════════════════════════════════════════════
    # Pattern 3: CHANGELOG.md Link Footers
    # ═══════════════════════════════════════════════════════════════════════════

    local CHANGELOG="$PROJECT_ROOT/CHANGELOG.md"
    if [[ -f "$CHANGELOG" ]]; then
        log "${MAGENTA}Checking CHANGELOG.md link footers...${NC}"

        # Check that [Unreleased] link footer exists
        if ! grep -qE '^\[Unreleased\]: https://' "$CHANGELOG" 2>/dev/null; then
            log_always "${YELLOW}  ⚠ CHANGELOG.md is missing [Unreleased] link footer${NC}"
            add_inconsistent_file "CHANGELOG.md (link footers)"
        fi

        # Check [Unreleased] link points to current version
        local UNRELEASED_PATTERN='\[Unreleased\]: https://github\.com/.*/compare/v[0-9]+\.[0-9]+\.[0-9]+\.\.\.HEAD'
        if grep -qE "$UNRELEASED_PATTERN" "$CHANGELOG" 2>/dev/null; then
            local current_unreleased
            current_unreleased=$(grep -oE 'compare/v[0-9]+\.[0-9]+\.[0-9]+\.\.\.HEAD' "$CHANGELOG" | head -1 | sed -E 's|compare/v([0-9]+\.[0-9]+\.[0-9]+)\.\.\.HEAD|\1|')
            if [[ "$current_unreleased" != "$VERSION" ]]; then
                log "${YELLOW}  → [Unreleased] link points to v$current_unreleased, should be v$VERSION${NC}"
                add_inconsistent_file "CHANGELOG.md (link footers)"

                if [[ "$CHECK_ONLY" == "false" ]]; then
                    if [[ "$DRY_RUN" == "true" ]]; then
                        echo -e "${YELLOW}Would update:${NC} CHANGELOG.md [Unreleased] link footer"
                    else
                        sd "(\\[Unreleased\\]: https://github\\.com/.*/compare/v)[0-9]+\\.[0-9]+\\.[0-9]+(\\.\\.\\.HEAD)" "\${1}$VERSION\${2}" "$CHANGELOG"
                        echo -e "${GREEN}✓ Updated:${NC} CHANGELOG.md [Unreleased] link → v$VERSION"
                        ((FILES_CHANGED++)) || true
                        ((TOTAL_REPLACEMENTS++)) || true
                    fi
                fi
            fi
        fi

        # Check that a [$VERSION] link entry exists
        local ESCAPED_VERSION="${VERSION//./\\.}"
        if ! grep -qE "^\[$ESCAPED_VERSION\]: https://github\.com/" "$CHANGELOG" 2>/dev/null; then
            log "${YELLOW}  → Missing [$VERSION] link entry in CHANGELOG.md${NC}"
            # Flag as inconsistent in all modes — adding entries
            # requires knowledge of the previous version tag
            add_inconsistent_file "CHANGELOG.md (link footers)"
            if [[ "$DRY_RUN" == "true" ]]; then
                echo -e "${YELLOW}⚠ CHANGELOG.md is missing a [$VERSION] link footer entry (dry-run)${NC}"
            else
                echo -e "${YELLOW}⚠ CHANGELOG.md is missing a [$VERSION] link footer entry${NC}"
            fi
        else
            log "${GREEN}  ✓ [$VERSION] link entry exists in CHANGELOG.md${NC}"
        fi

        # Check that every ## [x.y.z] header has a matching [x.y.z]: link footer
        local version_headers
        version_headers=$(grep -oE '^## \[[0-9]+\.[0-9]+\.[0-9]+\]' "$CHANGELOG" | sed -E 's/^## \[([0-9]+\.[0-9]+\.[0-9]+)\]/\1/' || true)
        if [[ -n "$version_headers" ]]; then
            while IFS= read -r header_version; do
                [[ -z "$header_version" ]] && continue
                local ESCAPED_HEADER_VERSION="${header_version//./\\.}"
                if ! grep -qE "^\[$ESCAPED_HEADER_VERSION\]: https://" "$CHANGELOG" 2>/dev/null; then
                    log "${YELLOW}  → ## [$header_version] header has no matching [$header_version]: link footer${NC}"
                    add_inconsistent_file "CHANGELOG.md (link footers)"
                    echo -e "${YELLOW}⚠ CHANGELOG.md ## [$header_version] header is missing a matching link footer${NC}"
                else
                    log "${GREEN}  ✓ [$header_version] link footer matches header${NC}"
                fi
            done <<< "$version_headers"
        fi
    fi

    # ═══════════════════════════════════════════════════════════════════════════
    # Summary Report
    # ═══════════════════════════════════════════════════════════════════════════

    echo ""
    echo -e "${CYAN}════════════════════════════════════════════════════════════════════════${NC}"
    log "${BLUE}Files scanned:${NC} $SCANNED_COUNT"

    if [[ "$CHECK_ONLY" == "true" ]]; then
        if [[ ${#INCONSISTENT_FILES[@]} -gt 0 ]]; then
            echo -e "${RED}✗ Version inconsistencies found in ${#INCONSISTENT_FILES[@]} file(s):${NC}"
            echo ""
            for f in "${INCONSISTENT_FILES[@]}"; do
                local ext="${f##*.}"
                ext="${ext%% (*}"
                echo -e "  ${YELLOW}•${NC} $f ${MAGENTA}($ext)${NC}"
            done
            echo ""
            echo -e "${YELLOW}Run './scripts/sync-version.sh' to fix these inconsistencies.${NC}"
            exit 1
        else
            echo -e "${GREEN}✓ All version references are consistent with Cargo.toml ($VERSION)${NC}"
            exit 0
        fi
    elif [[ "$DRY_RUN" == "true" ]]; then
        echo ""
        if [[ ${#INCONSISTENT_FILES[@]} -gt 0 ]]; then
            echo -e "${YELLOW}Would update ${#INCONSISTENT_FILES[@]} file(s)${NC}"
        else
            echo -e "${GREEN}✓ All version references are already consistent${NC}"
        fi
    else
        if [[ $FILES_CHANGED -gt 0 ]]; then
            echo -e "${GREEN}✓ Updated $FILES_CHANGED file(s) with $TOTAL_REPLACEMENTS replacement(s)${NC}"
            echo -e "${BLUE}All version references now match:${NC} ${GREEN}$MAJOR_MINOR${NC}"
        fi
        if [[ ${#INCONSISTENT_FILES[@]} -gt 0 ]]; then
            echo -e "${YELLOW}⚠ ${#INCONSISTENT_FILES[@]} issue(s) require manual intervention:${NC}"
            for f in "${INCONSISTENT_FILES[@]}"; do
                echo -e "  ${YELLOW}•${NC} $f"
            done
        elif [[ $FILES_CHANGED -eq 0 ]]; then
            echo -e "${GREEN}✓ All version references are already consistent with Cargo.toml ($VERSION)${NC}"
        fi
    fi

    # ═══════════════════════════════════════════════════════════════════════════
    # Coverage Summary (verbose mode)
    # ═══════════════════════════════════════════════════════════════════════════

    if [[ "$VERBOSE" == "true" ]]; then
        echo ""
        echo -e "${BLUE}File types coverage:${NC}"
        echo -e "  ${GREEN}•${NC} Rust source files (*.rs) - including /// and //! doc comments"
        echo -e "  ${GREEN}•${NC} Markdown documentation (*.md)"
        echo -e "  ${GREEN}•${NC} TOML configuration (*.toml) - except Cargo.toml/Cargo.lock"
        echo -e "  ${GREEN}•${NC} CI/CD workflows (*.yml, *.yaml)"
        echo -e "  ${GREEN}•${NC} Shell scripts (*.sh)"
        echo -e "  ${GREEN}•${NC} Text files (*.txt)"
        echo -e "  ${GREEN}•${NC} JSON files (*.json)"
    fi
}

main "$@"
