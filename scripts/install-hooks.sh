#!/bin/bash
# Install Git Hooks for Fortress Rollback
#
# This script installs the pre-commit hook and other git hooks
# for the Fortress Rollback project.
#
# Usage:
#   ./scripts/install-hooks.sh
#   ./scripts/install-hooks.sh --uninstall

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Get the project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

# Parse arguments
UNINSTALL=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --uninstall)
            UNINSTALL=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --uninstall  Remove installed hooks"
            echo "  --help       Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Check if we're in a git repository
if [[ ! -d "$PROJECT_ROOT/.git" ]]; then
    echo -e "${RED}Error: Not a git repository${NC}"
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

if [[ "$UNINSTALL" == "true" ]]; then
    echo -e "${BLUE}Uninstalling git hooks...${NC}"
    
    if [[ -f "$HOOKS_DIR/pre-commit" ]]; then
        rm "$HOOKS_DIR/pre-commit"
        echo -e "${GREEN}✓${NC} Removed pre-commit hook"
    else
        echo -e "${YELLOW}•${NC} pre-commit hook not installed"
    fi
    
    echo -e "${GREEN}Done!${NC}"
    exit 0
fi

echo -e "${BLUE}Installing git hooks for Fortress Rollback...${NC}"
echo ""

# Install pre-commit hook
HOOK_SOURCE="$SCRIPT_DIR/pre-commit"
HOOK_DEST="$HOOKS_DIR/pre-commit"

if [[ -f "$HOOK_SOURCE" ]]; then
    # Backup existing hook if it exists and is different
    if [[ -f "$HOOK_DEST" ]]; then
        if ! diff -q "$HOOK_SOURCE" "$HOOK_DEST" > /dev/null 2>&1; then
            BACKUP="$HOOK_DEST.backup.$(date +%Y%m%d%H%M%S)"
            cp "$HOOK_DEST" "$BACKUP"
            echo -e "${YELLOW}•${NC} Backed up existing pre-commit hook to: ${BACKUP#$PROJECT_ROOT/}"
        fi
    fi
    
    cp "$HOOK_SOURCE" "$HOOK_DEST"
    chmod +x "$HOOK_DEST"
    echo -e "${GREEN}✓${NC} Installed pre-commit hook"
else
    echo -e "${RED}✗${NC} pre-commit hook source not found: $HOOK_SOURCE"
    exit 1
fi

# Make sync-version.sh executable
SYNC_SCRIPT="$SCRIPT_DIR/sync-version.sh"
if [[ -f "$SYNC_SCRIPT" ]]; then
    chmod +x "$SYNC_SCRIPT"
    echo -e "${GREEN}✓${NC} Made sync-version.sh executable"
fi

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║              Git hooks installed successfully!             ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}The following hooks are now active:${NC}"
echo -e "  ${GREEN}•${NC} pre-commit - Version sync, formatting, and lint checks"
echo ""
echo -e "${BLUE}To uninstall:${NC} $0 --uninstall"
