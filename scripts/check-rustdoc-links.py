#!/usr/bin/env python3
"""
Cross-platform rustdoc warning checker for pre-commit hooks.

Runs `cargo doc --no-deps` with RUSTDOCFLAGS that match CI configuration,
treating all rustdoc warnings as errors. Fails if cargo doc returns a
non-zero exit code.

Works on Windows, macOS, and Linux.
"""

import os
import subprocess
import sys


# RUSTDOCFLAGS that match CI configuration - treats warnings as errors
RUSTDOCFLAGS = (
    "-D warnings "
    "-D rustdoc::broken_intra_doc_links "
    "-D rustdoc::private_intra_doc_links "
    "-D rustdoc::invalid_codeblock_attributes "
    "-D rustdoc::invalid_html_tags "
    "-D rustdoc::bare_urls"
)


def main() -> int:
    """Run cargo doc with CI-matching RUSTDOCFLAGS and check exit code."""
    try:
        # Set up environment with RUSTDOCFLAGS matching CI
        env = os.environ.copy()
        env["RUSTDOCFLAGS"] = RUSTDOCFLAGS

        # Run cargo doc and capture output
        result = subprocess.run(
            ["cargo", "doc", "--no-deps"],
            capture_output=True,
            text=True,
            env=env,
        )

        # If cargo doc failed (non-zero exit code), print stderr and fail
        if result.returncode != 0:
            print("ERROR: cargo doc failed with rustdoc warnings/errors:")
            if result.stderr:
                print(result.stderr)
            if result.stdout:
                print(result.stdout)
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
