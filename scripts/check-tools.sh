#!/bin/bash
# Verification script to check all development tools are installed
#
# Run this after setting up the devcontainer to verify all tools are working

set -e

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
            ((ERRORS++))
            return 1
        else
            echo -e "${YELLOW}○ missing (optional)${NC}"
            return 0
        fi
    fi
}

echo "Rust Toolchain:"
check_tool "rustc" "rustc --version"
check_tool "cargo" "cargo --version"
check_tool "rustup" "rustup --version"
check_tool "rust-nightly" "rustup run nightly rustc --version"
check_tool "rust-analyzer" "which rust-analyzer" "false"
echo ""

echo "Formal Verification:"
check_tool "TLA+ (tla2tools.jar)" "test -f /opt/tla/tla2tools.jar || test -f .tla-tools/tla2tools.jar || test -f ~/.tla-tools/tla2tools.jar"
check_tool "Java (for TLA+)" "java -version"
check_tool "Kani" "cargo kani --version" "false"
check_tool "Miri" "rustup +nightly which miri" "false"
echo ""

echo "Testing & Coverage:"
check_tool "cargo-tarpaulin" "cargo tarpaulin --version" "false"
check_tool "cargo-llvm-cov" "cargo llvm-cov --version" "false"
check_tool "cargo-nextest" "cargo nextest --version" "false"
check_tool "cargo-mutants" "cargo mutants --version" "false"
check_tool "cargo-fuzz" "cargo fuzz --version" "false"
echo ""

echo "Security & Quality:"
check_tool "cargo-audit" "cargo audit --version" "false"
check_tool "cargo-deny" "cargo deny --version" "false"
check_tool "clippy" "cargo clippy --version"
check_tool "rustfmt" "cargo fmt --version"
echo ""

echo "Documentation & Linting:"
check_tool "pre-commit" "pre-commit --version" "false"
check_tool "markdownlint" "markdownlint --version" "false"
check_tool "markdown-link-check" "markdown-link-check --version" "false"
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
