#!/bin/bash
#
# check-agent-skills.sh - Validate .agents/skills/ markdown files.
#
# The scan is implemented in Python so it can be single-pass and sub-second;
# this wrapper preserves the established shell invocation contract.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$SCRIPT_DIR/check_agent_skills.py" "$@"
