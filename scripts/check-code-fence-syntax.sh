#!/usr/bin/env bash
#
# check-code-fence-syntax.sh - Detect rustdoc-style code fence attributes in markdown
#
# MkDocs and most markdown renderers don't understand rustdoc attributes like:
#   ```rust,ignore
#   ```rust,no_run
#   ```rust,compile_fail
#   ```rust,should_panic
#
# These should be plain ```rust for documentation that will be rendered by MkDocs.
# Rustdoc attributes are only valid in Rust source file doc comments.
#
# Usage:
#   ./scripts/check-code-fence-syntax.sh [directory]
#
# Exit codes:
#   0 - No issues found
#   1 - Rustdoc-style attributes detected
#
# Examples:
#   ./scripts/check-code-fence-syntax.sh           # Check docs/ directory
#   ./scripts/check-code-fence-syntax.sh docs/     # Explicit directory
#   ./scripts/check-code-fence-syntax.sh .         # Check all markdown files

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Default to docs/ directory if not specified
SEARCH_DIR="${1:-docs}"

# Check if directory exists
if [[ ! -d "$SEARCH_DIR" ]]; then
    echo -e "${YELLOW}Warning: Directory '$SEARCH_DIR' does not exist, skipping check${NC}"
    exit 0
fi

# Pattern matches rustdoc-style code fence attributes:
#   ```rust,ignore
#   ```rust,no_run
#   ```rust,compile_fail
#   ```rust,should_panic
#   ```rust,edition2018
#   ```rust,edition2021
#   And any other comma-separated attributes after rust
#
# This regex matches: backticks + "rust" + comma + one or more word characters
PATTERN='```rust,[a-zA-Z0-9_]+'

echo "Checking for rustdoc-style code fence attributes in ${SEARCH_DIR}..."
echo ""

# Find all markdown files and check for the pattern
FOUND_ISSUES=0
TOTAL_OCCURRENCES=0

while IFS= read -r -d '' file; do
    # Count occurrences in this file
    COUNT=$(grep -Ec "$PATTERN" "$file" 2>/dev/null || true)

    if [[ "$COUNT" -gt 0 ]]; then
        if [[ "$FOUND_ISSUES" -eq 0 ]]; then
            echo -e "${RED}ERROR: Found rustdoc-style code fence attributes in markdown files${NC}"
            echo ""
            echo "These attributes (e.g., \`\`\`rust,ignore) are only valid in Rust doc comments."
            echo "For MkDocs markdown files, use plain \`\`\`rust instead."
            echo ""
            echo "Files with issues:"
        fi

        FOUND_ISSUES=1
        TOTAL_OCCURRENCES=$((TOTAL_OCCURRENCES + COUNT))

        echo -e "  ${YELLOW}${file}${NC} (${COUNT} occurrence(s))"

        # Show the specific lines with context
        grep -n "$PATTERN" "$file" | while IFS= read -r line; do
            echo "    $line"
        done
    fi
done < <(find "$SEARCH_DIR" -name "*.md" -type f -print0 2>/dev/null)

echo ""

if [[ "$FOUND_ISSUES" -eq 1 ]]; then
    echo -e "${RED}Found ${TOTAL_OCCURRENCES} rustdoc-style code fence attribute(s)${NC}"
    echo ""
    echo "To fix, replace patterns like:"
    echo "  \`\`\`rust,ignore  →  \`\`\`rust"
    echo "  \`\`\`rust,no_run  →  \`\`\`rust"
    echo ""
    echo "Quick fix command:"
    echo "  sed -i 's/\`\`\`rust,[a-zA-Z0-9_]*/\`\`\`rust/g' <file>"
    exit 1
else
    echo -e "${GREEN}✓ No rustdoc-style code fence attributes found${NC}"
    exit 0
fi
