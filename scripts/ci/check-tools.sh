#!/bin/bash
# Verification script to check all development tools are installed
#
# Run this after setting up the devcontainer to verify all tools are working

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Fortress Rollback - Tool Verification                        ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════════╝${NC}"
echo ""

ERRORS=0
PIN_DIRECTORY="$REPO_ROOT/.github/actions/install-pinned-nightly"
PIN_VALUE=""

read_dated_nightly_pin() {
    local pin_file="$1"
    local label="$2"
    local pin_value=""
    local extra_line=""

    PIN_VALUE=""
    if [ ! -f "$pin_file" ]; then
        echo -e "${RED}ERROR: $label pin is missing: $pin_file${NC}" >&2
        ((ERRORS += 1))
        return 1
    fi

    {
        # Avoid Bash-4-only bulk line readers because macOS ships Bash 3.2.
        if ! IFS= read -r pin_value && [ -z "$pin_value" ]; then
            echo -e "${RED}ERROR: $label pin must contain exactly one line: $pin_file${NC}" >&2
            ((ERRORS += 1))
            return 1
        fi
        if IFS= read -r extra_line || [ -n "$extra_line" ]; then
            echo -e "${RED}ERROR: $label pin must contain exactly one line: $pin_file${NC}" >&2
            ((ERRORS += 1))
            return 1
        fi
    } < "$pin_file"

    PIN_VALUE="${pin_value%$'\r'}"
    if [[ ! "$PIN_VALUE" =~ ^nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
        echo -e "${RED}ERROR: $label pin must be exactly nightly-YYYY-MM-DD: $pin_file${NC}" >&2
        ((ERRORS += 1))
        PIN_VALUE=""
        return 1
    fi
}

PINNED_NIGHTLY_TOOLCHAIN=""
if read_dated_nightly_pin "$PIN_DIRECTORY/toolchain" "generic nightly"; then
    PINNED_NIGHTLY_TOOLCHAIN="$PIN_VALUE"
fi

PINNED_MIRI_TOOLCHAIN=""
if read_dated_nightly_pin "$PIN_DIRECTORY/miri-toolchain" "Miri"; then
    PINNED_MIRI_TOOLCHAIN="$PIN_VALUE"
fi

check_tool() {
    local name="$1"
    local cmd="$2"
    local required="${3:-true}"

    printf "  %-25s" "$name"
    if eval "$cmd" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ installed${NC}"
        return 0
    else
        if [ "$required" = "true" ]; then
            echo -e "${RED}✗ missing (required)${NC}"
            ((ERRORS += 1))
        else
            echo -e "${YELLOW}○ missing (optional)${NC}"
            return 0
        fi
    fi
}

# Execute dynamic arguments directly. In particular, toolchain pins must never
# be interpolated into a command string evaluated as shell code.
check_tool_args() {
    local name="$1"
    local required="$2"
    shift 2

    printf "  %-25s" "$name"
    if "$@" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ installed${NC}"
    elif [ "$required" = "true" ]; then
        echo -e "${RED}✗ missing (required)${NC}"
        ((ERRORS += 1))
    else
        echo -e "${YELLOW}○ missing (optional)${NC}"
    fi
}

echo "Rust Toolchain:"
check_tool "rustc" "rustc --version"
check_tool "cargo" "cargo --version"
check_tool "rustup" "rustup --version"
if [ -n "$PINNED_NIGHTLY_TOOLCHAIN" ]; then
    check_tool_args "rust-nightly ($PINNED_NIGHTLY_TOOLCHAIN)" true \
        rustup run "$PINNED_NIGHTLY_TOOLCHAIN" rustc --version
fi
check_tool "rust-analyzer" "which rust-analyzer" "false"
echo ""

echo "Formal Verification:"
check_tool "TLA+ (tla2tools.jar)" "test -f /opt/tla/tla2tools.jar || test -f .tla-tools/tla2tools.jar || test -f ~/.tla-tools/tla2tools.jar"
check_tool "Java (for TLA+)" "java -version"
check_tool "Kani" "cargo kani --version" "false"
if [ -n "$PINNED_MIRI_TOOLCHAIN" ]; then
    check_tool_args "Miri ($PINNED_MIRI_TOOLCHAIN)" false \
        rustup run "$PINNED_MIRI_TOOLCHAIN" cargo miri --version
fi
echo ""

echo "Testing & Coverage:"
check_tool "cargo-tarpaulin" "cargo tarpaulin --version" "false"
check_tool "cargo-llvm-cov" "cargo llvm-cov --version" "false"
check_tool "cargo-nextest" "cargo nextest --version" "false"
check_tool "cargo-mutants" "cargo mutants --version" "false"
check_tool "cargo-fuzz" "cargo fuzz --version" "false"
check_tool "cargo-hack" "cargo hack --version" "false"
echo ""

echo "Security & Quality:"
check_tool "cargo-audit" "cargo audit --version" "false"
check_tool "cargo-deny" "cargo deny --version" "false"
check_tool "cargo-geiger" "cargo geiger --version" "false"
check_tool "cargo-semver-checks" "cargo semver-checks --version" "false"
check_tool "cargo-udeps" "cargo udeps --version" "false"
check_tool "clippy" "cargo clippy --version"
check_tool "rustfmt" "cargo fmt --version"
echo ""

echo "Documentation & Linting:"
check_tool "pre-commit" "pre-commit --version" "false"
check_tool "markdownlint" "markdownlint --version" "false"
check_tool "markdown-link-check" "markdown-link-check --version" "false"
check_tool "vale" "vale --version" "false"
echo ""

echo "CI/CD Linting:"
check_tool "actionlint" "actionlint --version" "false"
check_tool "yamllint" "yamllint --version" "false"
echo ""

echo "AI Tooling:"
check_tool "codex" "codex --version" "false"
echo ""

echo "Cargo Quality Tools:"
check_tool "cargo-shear" "cargo shear --version" "false"
check_tool "cargo-spellcheck" "cargo spellcheck --version" "false"
check_tool "cargo-careful" "which cargo-careful" "false"
echo ""

echo "Profiling:"
check_tool "flamegraph" "cargo flamegraph --version" "false"
check_tool "valgrind" "valgrind --version" "false"
echo ""

echo "SMT Solving:"
check_tool "Z3 (Python)" "python3 -c 'import z3'" "false"
check_tool "libclang-dev" "test -f /usr/lib/llvm-*/lib/libclang.so* || pkg-config --exists libclang" "false"
echo ""

echo "Network Testing:"
check_tool "iproute2 (tc)" "tc -V" "false"
check_tool "netcat" "nc -h 2>&1 | head -1" "false"
check_tool "tcpdump" "tcpdump --version" "false"
echo ""

echo "Other Tools:"
check_tool "graphviz (dot)" "dot -V" "false"
check_tool "cmake" "cmake --version" "false"
check_tool "clang" "clang --version" "false"
echo ""

echo "───────────────────────────────────────────────────────────────────"
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}All required tools are installed!${NC}"
else
    echo -e "${RED}$ERRORS required tool(s) missing${NC}"
fi
echo ""

# Version details for key tools
echo "Tool Versions:"
echo "  Rust: $(rustc --version 2>/dev/null || echo 'N/A')"
echo "  Cargo: $(cargo --version 2>/dev/null || echo 'N/A')"
echo "  Java: $(java -version 2>&1 | head -1 || echo 'N/A')"
if command -v cargo-kani &> /dev/null; then
    echo "  Kani: $(cargo kani --version 2>/dev/null || echo 'N/A')"
fi
if python3 -c 'import z3' 2>/dev/null; then
    echo "  Z3: $(python3 -c 'import z3; print(z3.get_version_string())' 2>/dev/null || echo 'N/A')"
fi
echo ""

exit $ERRORS
