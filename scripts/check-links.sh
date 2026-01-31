#!/bin/bash
# Link validation script for Fortress Rollback
#
# Validates:
# - Local file references in markdown files
# - Relative paths in code comments and documentation
# - Anchor links within markdown files
#
# Usage: ./scripts/check-links.sh [--verbose]

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

VERBOSE=false
if [[ "$1" == "--verbose" ]] || [[ "$1" == "-v" ]]; then
    VERBOSE=true
fi

ERRORS=0
WARNINGS=0
CHECKED=0

# Get repository root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Safe increment function that doesn't trigger set -e
incr_errors() { ERRORS=$((ERRORS + 1)); }
incr_warnings() { WARNINGS=$((WARNINGS + 1)); }
incr_checked() { CHECKED=$((CHECKED + 1)); }

log_verbose() {
    if $VERBOSE; then
        echo -e "$1"
    fi
}

log_error() {
    echo -e "${RED}ERROR:${NC} $1"
    incr_errors
}

log_warning() {
    echo -e "${YELLOW}WARNING:${NC} $1"
    incr_warnings
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

# Check if a local file exists
check_local_file() {
    local source_file="$1"
    local link="$2"

    # Remove anchor from link
    local path="${link%%#*}"

    # Skip empty paths (pure anchor links)
    if [[ -z "$path" ]]; then
        return 0
    fi

    # Skip external URLs
    if [[ "$link" =~ ^https?:// ]] || [[ "$link" =~ ^mailto: ]]; then
        return 0
    fi

    # Resolve relative path
    local source_dir
    source_dir="$(dirname "$source_file")"
    local resolved_path

    if [[ "$path" == /* ]]; then
        # Absolute path from repo root
        resolved_path="$REPO_ROOT$path"
    else
        # Relative path
        resolved_path="$source_dir/$path"
    fi

    # Normalize path
    resolved_path="$(cd "$(dirname "$resolved_path")" 2>/dev/null && pwd)/$(basename "$resolved_path")" 2>/dev/null || resolved_path=""

    incr_checked

    if [[ -z "$resolved_path" ]] || [[ ! -e "$resolved_path" ]]; then
        # For wiki files, try adding .md extension (GitHub Wiki uses extensionless links)
        if [[ "$source_file" == *"/wiki/"* ]] && [[ ! "$path" =~ \. ]]; then
            local wiki_resolved="${resolved_path}.md"
            if [[ -e "$wiki_resolved" ]]; then
                log_verbose "  ${GREEN}✓${NC} $link (wiki format)"
                return 0
            fi
        fi
        log_error "Broken link in $source_file: '$link' (resolved to: ${resolved_path:-<invalid path>})"
        return 1
    else
        log_verbose "  ${GREEN}✓${NC} $link"
        return 0
    fi
}

# Check anchor links within a markdown file
check_anchor() {
    local file="$1"
    local anchor="$2"

    # GitHub creates anchors from headings by:
    # 1. Converting to lowercase
    # 2. Replacing spaces with hyphens
    # 3. Removing punctuation (except hyphens)
    # 4. Keeping consecutive hyphens
    local normalized_anchor
    normalized_anchor=$(echo "$anchor" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | sed 's/[^a-z0-9-]//g')

    # Generate all heading anchors in the file using GitHub's algorithm
    # Note: Use sed -E for extended regex (needed for #+)
    local headings
    headings=$(grep -E "^#+[[:space:]]" "$file" 2>/dev/null | \
        sed -E 's/^#+[[:space:]]*//' | \
        tr '[:upper:]' '[:lower:]' | \
        tr ' ' '-' | \
        sed 's/[^a-z0-9-]//g')

    # Check if our normalized anchor matches any heading anchor
    if echo "$headings" | grep -qxF "$normalized_anchor" 2>/dev/null; then
        log_verbose "  ${GREEN}✓${NC} #$anchor"
        return 0
    fi

    # Also check for explicit anchor definitions: <a name="..."> or id="..."
    if grep -qE "<a[^>]*name=[\"']?${anchor}[\"']?" "$file" 2>/dev/null; then
        log_verbose "  ${GREEN}✓${NC} #$anchor (explicit)"
        return 0
    fi

    if grep -qE "id=[\"']?${anchor}[\"']?" "$file" 2>/dev/null; then
        log_verbose "  ${GREEN}✓${NC} #$anchor (id)"
        return 0
    fi

    # Check for MkDocs-style custom anchors: { #anchor-name }
    if grep -qE "\{[[:space:]]*#${anchor}[[:space:]]*\}" "$file" 2>/dev/null; then
        log_verbose "  ${GREEN}✓${NC} #$anchor (mkdocs)"
        return 0
    fi

    log_warning "Possibly broken anchor in $file: '#$anchor'"
    return 0  # Don't fail on anchor warnings, they can be complex
}

# Extract and check links from markdown files
check_markdown_links() {
    local file="$1"

    log_verbose "${BLUE}Checking:${NC} $file"

    # Remove fenced code blocks before extracting links
    # Uses awk to skip lines between ``` or ~~~ markers (with optional leading whitespace)
    local content_without_code
    content_without_code=$(awk '
        /^[[:space:]]*```|^[[:space:]]*~~~/ {
            if (in_fence) {
                in_fence = 0
            } else {
                in_fence = 1
            }
            next
        }
        !in_fence { print }
    ' "$file" 2>/dev/null)

    # Also remove inline code spans (backticks) to avoid false positives
    # Handles both single `code` and double ``code`` backtick syntax
    # Uses perl for proper non-greedy matching across the content
    content_without_code=$(echo "$content_without_code" | perl -pe 's/``[^`]+``//g; s/`[^`]+`//g' 2>/dev/null || echo "$content_without_code")

    # Extract markdown links: [text](link) from content without code blocks
    local links
    links=$(echo "$content_without_code" | grep -oE '\[([^]]*)\]\(([^)]+)\)' 2>/dev/null | sed 's/\[.*\](\(.*\))/\1/' | sed 's/)$//')

    for link in $links; do
        # Check if it's an anchor-only link
        if [[ "$link" =~ ^# ]]; then
            check_anchor "$file" "${link#\#}"
        # Check if it contains an anchor
        elif [[ "$link" =~ "#" ]]; then
            local path="${link%%#*}"
            local anchor="${link#*#}"
            check_local_file "$file" "$path"
            # TODO: Could also check anchor in target file
        else
            check_local_file "$file" "$link"
        fi
    done

    # Extract reference-style links: [text]: url (also filtered)
    local ref_links
    ref_links=$(echo "$content_without_code" | grep -oE '^\[[^]]+\]:[[:space:]]+.+' 2>/dev/null | sed 's/^\[[^]]*\]:[[:space:]]*//')

    for link in $ref_links; do
        check_local_file "$file" "$link"
    done

    # Extract raw HTML links: href="..." and src="..." (also filtered)
    local html_links
    html_links=$(echo "$content_without_code" | grep -oE '(href|src)="[^"]*"' 2>/dev/null | sed 's/.*="\([^"]*\)"/\1/')

    for link in $html_links; do
        check_local_file "$file" "$link"
    done
}

# Extract and check links from Rust files (doc comments)
check_rust_doc_links() {
    local file="$1"

    log_verbose "${BLUE}Checking:${NC} $file (doc comments)"

    # Extract links from doc comments: /// [text](link) or //! [text](link)
    local doc_links
    doc_links=$(grep -E '^\s*(///|//!)' "$file" 2>/dev/null | grep -oE '\[([^]]*)\]\(([^)]+)\)' | sed 's/\[.*\](\(.*\))/\1/' | sed 's/)$//')

    for link in $doc_links; do
        # Skip intra-doc links:
        # - Start with crate::, super::, self::, Self::
        # - Contain :: (Rust path separator, like Deref::deref, std::ops::Deref)
        # - Start with backtick (code references)
        if [[ "$link" =~ ^(crate|super|self|Self):: ]] || [[ "$link" =~ :: ]] || [[ "$link" =~ ^\` ]]; then
            continue
        fi
        # Skip external URLs
        if [[ "$link" =~ ^https?:// ]]; then
            continue
        fi
        check_local_file "$file" "$link"
    done
}

# Check links embedded in source code comments referencing local files
check_code_file_references() {
    local file="$1"

    log_verbose "${BLUE}Checking:${NC} $file (file references)"

    # Look for file path patterns in comments
    # Common patterns: See `path/to/file.rs`, file: `path/to/file`, etc.
    local file_refs
    file_refs=$(grep -oE '(See|see|File|file|Source|source)[[:space:]]*[:\`]?[[:space:]]*[`"]?[a-zA-Z0-9_./-]+\.(rs|md|toml|json|yaml|yml|sh)[`"]?' "$file" 2>/dev/null | grep -oE '[a-zA-Z0-9_./-]+\.(rs|md|toml|json|yaml|yml|sh)')

    for ref in $file_refs; do
        # Only check if it looks like a relative path
        if [[ "$ref" != *"/"* ]] && [[ ! -f "$REPO_ROOT/$ref" ]]; then
            continue
        fi
        check_local_file "$file" "$ref"
    done
}

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Fortress Rollback - Link Validation                          ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Find and check all markdown files
echo -e "${BLUE}Checking markdown files...${NC}"
while IFS= read -r -d '' file; do
    # Skip target directories and progress (session notes)
    if [[ "$file" == *"/target/"* ]] || [[ "$file" == *"/progress/"* ]]; then
        continue
    fi
    check_markdown_links "$file"
done < <(find "$REPO_ROOT" -name "*.md" -type f -print0)

echo ""
echo -e "${BLUE}Checking Rust doc comments...${NC}"
while IFS= read -r -d '' file; do
    # Skip target directories
    if [[ "$file" == *"/target/"* ]]; then
        continue
    fi
    check_rust_doc_links "$file"
done < <(find "$REPO_ROOT/src" -name "*.rs" -type f -print0)

# Also check example and test files
while IFS= read -r -d '' file; do
    if [[ "$file" == *"/target/"* ]]; then
        continue
    fi
    check_rust_doc_links "$file"
done < <(find "$REPO_ROOT/examples" "$REPO_ROOT/tests" -name "*.rs" -type f -print0 2>/dev/null || true)

echo ""
echo -e "${BLUE}══════════════════════════════════════════════════════════════════${NC}"
echo ""
echo "Links checked: $CHECKED"
echo -e "Errors: ${RED}$ERRORS${NC}"
echo -e "Warnings: ${YELLOW}$WARNINGS${NC}"
echo ""

if [[ $ERRORS -gt 0 ]]; then
    echo -e "${RED}Link validation failed with $ERRORS error(s).${NC}"
    exit 1
else
    echo -e "${GREEN}Link validation passed.${NC}"
    exit 0
fi
