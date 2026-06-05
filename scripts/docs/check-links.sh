#!/bin/bash
# Compatibility wrapper for the Python local link checker.
#
# Usage: ./scripts/docs/check-links.sh [--verbose]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exec python3 "$REPO_ROOT/scripts/docs/check-links.py" "$@"
