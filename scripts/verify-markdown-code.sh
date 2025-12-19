#!/bin/bash
# Markdown Code Sample Verification Script for Fortress Rollback
#
# This script extracts Rust code samples from markdown files and verifies
# they compile (and optionally run) without errors.
#
# Usage:
#   ./scripts/verify-markdown-code.sh              # Verify all markdown files
#   ./scripts/verify-markdown-code.sh --verbose    # Show detailed output
#   ./scripts/verify-markdown-code.sh --fix        # Show suggestions for fixes
#   ./scripts/verify-markdown-code.sh --strict     # Don't auto-skip incomplete snippets
#   ./scripts/verify-markdown-code.sh file.md      # Verify specific file
#
# The script handles:
# - Rust code blocks (```rust)
# - Code blocks marked as no_run, ignore, or compile_fail
# - Auto-detects diff-style snippets, placeholder code, and intentionally incomplete examples
# - Building a test crate with proper dependencies
# - Reporting exact line numbers of failures

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEMP_DIR=""
VERBOSE=false
FIX_MODE=false
STRICT_MODE=false
FAIL_FAST=false
SPECIFIC_FILE=""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Counters
TOTAL_BLOCKS=0
COMPILED_BLOCKS=0
SKIPPED_BLOCKS=0
AUTO_SKIPPED_BLOCKS=0
FAILED_BLOCKS=0
WARN_BLOCKS=0

# Arrays to track failures
declare -a FAILURES=()
declare -a WARNINGS=()
declare -a AUTO_SKIPPED=()

print_usage() {
    echo "Usage: $0 [options] [file.md]"
    echo ""
    echo "Options:"
    echo "  --verbose, -v   Show detailed output including extracted code"
    echo "  --fix           Show suggestions for fixing failing samples"
    echo "  --strict        Don't auto-skip incomplete snippets (diff-style, placeholders)"
    echo "  --fail-fast     Stop immediately when any code block fails"
    echo "  --help, -h      Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                          # Verify all markdown files"
    echo "  $0 --verbose                # Verify with detailed output"
    echo "  $0 docs/user-guide.md       # Verify specific file"
    echo "  $0 --fix docs/user-guide.md # Show fix suggestions"
    echo "  $0 --strict                 # Don't auto-skip incomplete code"
    echo "  $0 --fail-fast              # CI mode: stop on first failure"
}

print_header() {
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║        Markdown Code Sample Verification                       ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

log_verbose() {
    if $VERBOSE; then
        echo -e "$1"
    fi
}

log_info() {
    echo -e "${BLUE}INFO:${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}WARNING:${NC} $1"
    WARNINGS+=("$1")
    ((WARN_BLOCKS++)) || true
}

log_auto_skip() {
    local msg="$1"
    log_verbose "  ${YELLOW}AUTO-SKIP:${NC} $msg"
    AUTO_SKIPPED+=("$msg")
    ((AUTO_SKIPPED_BLOCKS++)) || true
}

log_error() {
    echo -e "${RED}ERROR:${NC} $1"
    FAILURES+=("$1")
    ((FAILED_BLOCKS++)) || true
}

cleanup() {
    if [[ -n "$TEMP_DIR" && -d "$TEMP_DIR" ]]; then
        rm -rf "$TEMP_DIR"
    fi
}

trap cleanup EXIT

setup_temp_crate() {
    TEMP_DIR=$(mktemp -d)
    log_verbose "Created temp directory: $TEMP_DIR"
    
    # Create Cargo.toml for the test crate
    cat > "$TEMP_DIR/Cargo.toml" << EOF
[package]
name = "markdown-code-test"
version = "0.1.0"
edition = "2021"

[dependencies]
fortress-rollback = { path = "$PROJECT_ROOT" }
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"
web-time = "1.1"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }

# Allow warnings in test code
[lints.rust]
dead_code = "allow"
unused_variables = "allow"
unused_imports = "allow"
EOF
    
    mkdir -p "$TEMP_DIR/src"
    echo "fn main() {}" > "$TEMP_DIR/src/main.rs"
}

# Extract code blocks from a markdown file using Python
# This is more reliable than bash for parsing markdown
extract_code_blocks() {
    local file="$1"
    
    python3 - "$file" << 'PYTHON_SCRIPT'
import re
import sys

file_path = sys.argv[1]

with open(file_path, 'r') as f:
    content = f.read()

# Find all code blocks with their positions
pattern = r'```(\w*(?:,\s*\w+)*)\n(.*?)```'
matches = list(re.finditer(pattern, content, re.DOTALL))

block_num = 0
for match in matches:
    block_num += 1
    attrs = match.group(1).strip()
    code = match.group(2)
    
    # Calculate line number
    line_num = content[:match.start()].count('\n') + 1
    
    # Parse language from attributes
    lang = attrs.split(',')[0].split()[0] if attrs else ''
    
    print("---BLOCK_START---")
    print(f"FILE:{file_path}")
    print(f"LINE:{line_num}")
    print(f"NUM:{block_num}")
    print(f"LANG:{lang}")
    print(f"ATTRS:{attrs}")
    print("---CONTENT_START---")
    print(code, end='')
    print("---BLOCK_END---")

PYTHON_SCRIPT
}

# Check if a code block should be skipped based on attributes
# Arguments: $1 = language, $2 = attributes
# Returns: 0 if should skip, 1 if should compile
should_skip_block() {
    local lang="$1"
    local attrs="$2"
    
    # Only process Rust code
    if [[ "$lang" != "rust" ]]; then
        return 0
    fi
    
    # Check for skip markers
    if echo "$attrs" | grep -qE '(ignore|no_run|compile_fail)'; then
        return 0
    fi
    
    return 1
}

# Get skip reason for a code block
get_skip_reason() {
    local lang="$1"
    local attrs="$2"
    
    if [[ "$lang" != "rust" ]]; then
        echo "not Rust (language: $lang)"
    elif echo "$attrs" | grep -q 'ignore'; then
        echo "marked as ignore"
    elif echo "$attrs" | grep -q 'no_run'; then
        echo "marked as no_run"
    elif echo "$attrs" | grep -q 'compile_fail'; then
        echo "marked as compile_fail"
    else
        echo "unknown"
    fi
}

# Check if code appears to be an incomplete snippet that shouldn't be compiled
# Returns: 0 if incomplete (should auto-skip), 1 if complete (should compile)
is_incomplete_snippet() {
    local code="$1"
    
    # Diff-style markers (+ or - at start of line for additions/removals)
    if echo "$code" | grep -qE '^[+-] '; then
        echo "diff-style snippet"
        return 0
    fi
    
    # Contains ... placeholder (common in documentation)
    if echo "$code" | grep -qE '\.\.\.[^.]|{ \.\.\. }|\{ \.\.\. \}|\.\.\.}'; then
        echo "contains ... placeholder"
        return 0
    fi
    
    # Contains // ... comment placeholder
    if echo "$code" | grep -qE '//\s*\.\.\.|//\s*\.\.\.'; then
        echo "contains // ... comment"
        return 0
    fi
    
    # References to old crate names (migration examples)
    if echo "$code" | grep -qE 'use ggrs::'; then
        echo "references old crate name (migration example)"
        return 0
    fi
    
    # Contains obvious placeholder patterns
    if echo "$code" | grep -qE '<.*>.*//.*placeholder|PLACEHOLDER|TODO:|your_'; then
        echo "contains placeholder text"
        return 0
    fi
    
    # Shell command prefixed with $ (sometimes in markdown)
    if echo "$code" | grep -qE '^\$\s'; then
        echo "appears to be shell command"
        return 0
    fi
    
    # References undefined generic types like MyConfig, GameConfig, etc.
    # These are typically documentation examples
    if echo "$code" | grep -qE '::<(My|Game|Your|Example|Test|Demo)[A-Z][a-z]*>'; then
        echo "uses generic placeholder type (documentation example)"
        return 0
    fi
    
    # References undefined builder variables
    if echo "$code" | grep -qE '^[[:space:]]*builder\.' && ! echo "$code" | grep -qE 'let.*builder'; then
        echo "references undefined builder variable"
        return 0
    fi
    
    # Before/After style documentation
    if echo "$code" | grep -qE '//\s*(Before|After)'; then
        echo "before/after documentation example"
        return 0
    fi
    
    # References undefined session variable (common in documentation)
    # Only skip if session is used but not defined with "let session" or "let mut session"
    if echo "$code" | grep -qE '\bsession\b'; then
        if ! echo "$code" | grep -qE 'let\s+(mut\s+)?session\s*[=:]'; then
            echo "references undefined session variable (documentation example)"
            return 0
        fi
    fi
    
    # References undefined game_state variable
    if echo "$code" | grep -qE '\bgame_state\b' && ! echo "$code" | grep -qE 'let.*game_state'; then
        echo "references undefined game_state variable (documentation example)"
        return 0
    fi
    
    # References functions that are meant to be user-defined
    if echo "$code" | grep -qE '\b(handle_event|handle_requests|get_local_input|apply_input|compute_checksum|render)\s*\('; then
        echo "references user-defined functions (documentation example)"
        return 0
    fi
    
    # Incomplete match arms with { ... } or similar
    if echo "$code" | grep -qE '\{[[:space:]]*\}|=>[[:space:]]*\{'; then
        # Only skip if it looks like placeholder
        if echo "$code" | grep -qE '\{[[:space:]]*\}'; then
            echo "contains empty block placeholder"
            return 0
        fi
    fi
    
    # Contains undefined Config type references (GameConfig without definition)
    if echo "$code" | grep -qE 'GameConfig|GameState|GameInput' && ! echo "$code" | grep -qE '(struct|enum|type)[[:space:]]+(GameConfig|GameState|GameInput)'; then
        echo "references undefined Game* types (documentation example)"
        return 0
    fi
    
    # Short snippets (< 5 non-comment lines) that appear to be inline examples
    # These often reference undefined variables
    local non_comment_lines
    non_comment_lines=$(echo "$code" | grep -cvE '^[[:space:]]*(//|$)' 2>/dev/null || echo "0")
    non_comment_lines="${non_comment_lines%%$'\n'*}"  # Strip trailing newlines
    non_comment_lines="${non_comment_lines:-0}"  # Default to 0 if empty
    if [[ "$non_comment_lines" =~ ^[0-9]+$ ]] && [[ "$non_comment_lines" -lt 5 ]]; then
        # Check if it uses undefined variables (variables without let binding)
        if echo "$code" | grep -qE '_frames|_ms|_rtt|estimated_|network_|local_|remote_'; then
            echo "short inline example with undefined variables"
            return 0
        fi
    fi
    
    # Short snippets referencing config variables that need definition
    if echo "$code" | grep -qE '\b(sparse_saving|first_incorrect|last_saved|check_distance)\b'; then
        if ! echo "$code" | grep -qE 'let.*(sparse_saving|first_incorrect|last_saved|check_distance)'; then
            echo "references undefined config variable (documentation example)"
            return 0
        fi
    fi
    
    # References to types that need to be defined (spectator, player, etc.)
    if echo "$code" | grep -qE '\bspectator\b|\bplayer\b' && echo "$code" | grep -qE '\.(address|handle|socket)'; then
        if ! echo "$code" | grep -qE 'let.*(spectator|player)'; then
            echo "references undefined spectator/player variables"
            return 0
        fi
    fi
    
    # Comment-only blocks
    if echo "$code" | grep -qvE '^[[:space:]]*(//|$)'; then
        : # Has non-comment content
    else
        echo "comment-only block"
        return 0
    fi
    
    return 1
}

# Check if code is a complete program or a snippet
is_complete_program() {
    local code="$1"
    
    # Check for fn main
    if echo "$code" | grep -q 'fn main()'; then
        return 0
    fi
    
    return 1
}

# Wrap code snippet to make it compilable
wrap_code_snippet() {
    local code="$1"
    
    # Check if it already has a main function
    if is_complete_program "$code"; then
        echo "$code"
        return
    fi
    
    # Use Python for more reliable code wrapping
    python3 - << PYTHON_SCRIPT
import sys

code = '''$code'''

# Separate use statements from the rest
lines = code.split('\n')
uses = []
rest = []

for line in lines:
    stripped = line.strip()
    if stripped.startswith('use ') or stripped.startswith('pub use '):
        uses.append(line)
    else:
        rest.append(line)

# Build wrapped code
print('''// Auto-generated wrapper for code sample verification
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unreachable_code)]
#![allow(clippy::all)]

''')

for u in uses:
    print(u)

print('''
fn main() -> Result<(), Box<dyn std::error::Error>> {
''')

for r in rest:
    print('    ' + r)

print('''
    #[allow(unreachable_code)]
    Ok(())
}''')

PYTHON_SCRIPT
}

# Compile a code sample
# Arguments: $1 = file, $2 = line, $3 = block_num, $4 = code
# Returns: 0 on success, 1 on failure
compile_code_sample() {
    local file="$1"
    local line="$2"
    local block_num="$3"
    local code="$4"
    
    local test_file="$TEMP_DIR/src/main.rs"
    local wrapped_code
    
    wrapped_code=$(wrap_code_snippet "$code")
    
    echo "$wrapped_code" > "$test_file"
    
    log_verbose "  Compiling block $block_num from $file:$line"
    if $VERBOSE; then
        echo "  Code:"
        echo "$wrapped_code" | head -30 | sed 's/^/    /'
        if [[ $(echo "$wrapped_code" | wc -l) -gt 30 ]]; then
            echo "    ... (truncated)"
        fi
    fi
    
    # Try to compile
    local compile_output
    local compile_result=0
    
    cd "$TEMP_DIR"
    if compile_output=$(cargo check --message-format=short 2>&1); then
        compile_result=0
    else
        compile_result=1
    fi
    cd "$PROJECT_ROOT"
    
    if [[ $compile_result -eq 0 ]]; then
        log_success "Block $block_num ($file:$line) compiled successfully"
        ((COMPILED_BLOCKS++)) || true
        return 0
    else
        log_error "Block $block_num ($file:$line) failed to compile"
        if $VERBOSE || $FIX_MODE; then
            echo "$compile_output" | grep -E '(error|warning)\[' | head -10 | sed 's/^/    /'
        fi
        if $FIX_MODE; then
            echo -e "  ${YELLOW}Suggestion:${NC} Add \`\`\`rust,ignore or \`\`\`rust,no_run if this is intentionally incomplete"
        fi
        return 1
    fi
}

# Process a single markdown file
process_markdown_file() {
    local file="$1"
    local relative_file="${file#$PROJECT_ROOT/}"
    
    log_info "Processing $relative_file"
    
    local block_file=""
    local block_line=""
    local block_num=""
    local block_lang=""
    local block_attrs=""
    local block_content=""
    local in_content=false
    
    while IFS= read -r line; do
        case "$line" in
            "---BLOCK_START---")
                block_content=""
                in_content=false
                ;;
            FILE:*)
                block_file="${line#FILE:}"
                ;;
            LINE:*)
                block_line="${line#LINE:}"
                ;;
            NUM:*)
                block_num="${line#NUM:}"
                ;;
            LANG:*)
                block_lang="${line#LANG:}"
                ;;
            ATTRS:*)
                block_attrs="${line#ATTRS:}"
                ;;
            "---CONTENT_START---")
                in_content=true
                ;;
            "---BLOCK_END---")
                ((TOTAL_BLOCKS++)) || true
                
                if should_skip_block "$block_lang" "$block_attrs"; then
                    local reason
                    reason=$(get_skip_reason "$block_lang" "$block_attrs")
                    log_verbose "  Skipping block $block_num at line $block_line ($reason)"
                    ((SKIPPED_BLOCKS++)) || true
                else
                    # Check for incomplete snippets unless in strict mode
                    local incomplete_reason
                    if ! $STRICT_MODE && incomplete_reason=$(is_incomplete_snippet "$block_content"); then
                        log_auto_skip "Block $block_num at line $block_line ($incomplete_reason)"
                    else
                        if ! compile_code_sample "$relative_file" "$block_line" "$block_num" "$block_content"; then
                            if $FAIL_FAST; then
                                echo -e "${RED}Stopping early due to --fail-fast${NC}"
                                return 1
                            fi
                        fi
                    fi
                fi
                
                in_content=false
                ;;
            *)
                if $in_content; then
                    if [[ -z "$block_content" ]]; then
                        block_content="$line"
                    else
                        block_content="$block_content"$'\n'"$line"
                    fi
                fi
                ;;
        esac
    done < <(extract_code_blocks "$file")
}

# Find all markdown files in the project
find_markdown_files() {
    find "$PROJECT_ROOT" \
        -name "*.md" \
        -not -path "*/target/*" \
        -not -path "*/.git/*" \
        -not -path "*/node_modules/*" \
        -type f \
        | sort
}

# Print summary
print_summary() {
    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}                           Summary                                  ${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo "  Total code blocks found:    $TOTAL_BLOCKS"
    echo "  Rust blocks compiled:       $COMPILED_BLOCKS"
    echo "  Blocks skipped (attrs):     $SKIPPED_BLOCKS"
    echo "  Blocks auto-skipped:        $AUTO_SKIPPED_BLOCKS"
    echo -e "  ${RED}Blocks failed:              $FAILED_BLOCKS${NC}"
    echo ""
    
    if [[ ${#FAILURES[@]} -gt 0 ]]; then
        echo -e "${RED}Failed blocks:${NC}"
        for failure in "${FAILURES[@]}"; do
            echo "  - $failure"
        done
        echo ""
    fi
    
    if $VERBOSE && [[ ${#AUTO_SKIPPED[@]} -gt 0 ]]; then
        echo -e "${YELLOW}Auto-skipped blocks:${NC}"
        for skipped in "${AUTO_SKIPPED[@]}"; do
            echo "  - $skipped"
        done
        echo ""
    fi
    
    if [[ ${#WARNINGS[@]} -gt 0 ]]; then
        echo -e "${YELLOW}Warnings:${NC}"
        for warning in "${WARNINGS[@]}"; do
            echo "  - $warning"
        done
        echo ""
    fi
    
    if [[ $FAILED_BLOCKS -eq 0 ]]; then
        echo -e "${GREEN}All code samples compiled successfully!${NC}"
        if [[ $AUTO_SKIPPED_BLOCKS -gt 0 ]]; then
            echo -e "${YELLOW}Note:${NC} $AUTO_SKIPPED_BLOCKS incomplete snippets were auto-skipped."
            echo "Run with --strict to attempt compiling all snippets."
        fi
        return 0
    else
        echo -e "${RED}Some code samples failed to compile.${NC}"
        echo "Run with --fix for suggestions on how to address failures."
        echo "Run with --verbose to see detailed error messages."
        return 1
    fi
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --fix)
            FIX_MODE=true
            VERBOSE=true
            shift
            ;;
        --strict)
            STRICT_MODE=true
            shift
            ;;
        --fail-fast)
            FAIL_FAST=true
            shift
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
            if [[ -f "$1" ]]; then
                SPECIFIC_FILE="$1"
            elif [[ -f "$PROJECT_ROOT/$1" ]]; then
                SPECIFIC_FILE="$PROJECT_ROOT/$1"
            else
                echo "File not found: $1"
                exit 1
            fi
            shift
            ;;
    esac
done

# Main execution
main() {
    print_header
    setup_temp_crate
    
    local process_result=0
    if [[ -n "$SPECIFIC_FILE" ]]; then
        process_markdown_file "$SPECIFIC_FILE" || process_result=$?
    else
        while IFS= read -r file; do
            if ! process_markdown_file "$file"; then
                process_result=1
                if $FAIL_FAST; then
                    break
                fi
            fi
        done < <(find_markdown_files)
    fi
    
    print_summary
}

main
