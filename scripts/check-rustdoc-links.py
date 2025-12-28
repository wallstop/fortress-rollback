#!/usr/bin/env python3
"""
Cross-platform rustdoc link checker for pre-commit hooks.

Runs `cargo doc --no-deps` and checks for unresolved link warnings.
Works on Windows, macOS, and Linux.
"""

import subprocess
import sys
import re


def main() -> int:
    """Run cargo doc and check for unresolved link warnings."""
    try:
        # Run cargo doc and capture stderr (where warnings go)
        result = subprocess.run(
            ["cargo", "doc", "--no-deps"],
            capture_output=True,
            text=True,
        )

        # Check for unresolved link warnings in stderr
        output = result.stderr + result.stdout
        unresolved_pattern = re.compile(r"warning:.*unresolved link", re.IGNORECASE)

        if unresolved_pattern.search(output):
            print("ERROR: Found unresolved rustdoc links:")
            for line in output.splitlines():
                if unresolved_pattern.search(line):
                    print(f"  {line}")
            return 1

        return 0

    except FileNotFoundError:
        print("ERROR: cargo not found. Is Rust installed?")
        return 1
    except Exception as e:
        print(f"ERROR: Failed to run cargo doc: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
