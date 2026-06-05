#!/usr/bin/env python3
"""
Cross-platform rustdoc warning checker for pre-commit hooks.

Runs the same strict rustdoc passes used by CI, treating rustdoc warnings as
errors. Fails if any cargo doc pass returns a non-zero exit code.

Works on Windows, macOS, and Linux.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "build"))

from cargo_linker import get_cargo_env


# RUSTDOCFLAGS that match CI configuration - treats warnings as errors
RUSTDOCFLAGS = (
    "-D warnings "
    "-D rustdoc::broken_intra_doc_links "
    "-D rustdoc::private_intra_doc_links "
    "-D rustdoc::invalid_codeblock_attributes "
    "-D rustdoc::invalid_html_tags "
    "-D rustdoc::bare_urls"
)

DOC_PASSES = (
    ("default public docs", ["cargo", "doc", "--no-deps"]),
    (
        "private feature-gated docs",
        [
            "cargo",
            "doc",
            "--no-deps",
            "--features",
            "hot-join,tokio,json,sync-send",
            "--document-private-items",
        ],
    ),
)


def main() -> int:
    """Run cargo doc with CI-matching RUSTDOCFLAGS and check exit code."""
    try:
        # Set up environment with linker overrides and RUSTDOCFLAGS matching CI
        env = os.environ.copy()
        env.update(get_cargo_env())
        existing_rustdocflags = env.get("RUSTDOCFLAGS", "").strip()

        # In CI, enforce exact CI RUSTDOCFLAGS; locally, append to any existing flags
        if env.get("CI"):
            env["RUSTDOCFLAGS"] = RUSTDOCFLAGS
        elif existing_rustdocflags:
            env["RUSTDOCFLAGS"] = f"{existing_rustdocflags} {RUSTDOCFLAGS}"
        else:
            env["RUSTDOCFLAGS"] = RUSTDOCFLAGS
        for pass_name, command in DOC_PASSES:
            result = subprocess.run(
                command,
                capture_output=True,
                text=True,
                env=env,
            )

            if result.returncode != 0:
                print(
                    f"ERROR: cargo doc failed during {pass_name} with rustdoc warnings/errors:",
                    file=sys.stderr,
                )
                if result.stderr:
                    print(result.stderr, file=sys.stderr)
                if result.stdout:
                    print(result.stdout, file=sys.stderr)
                return 1

        return 0

    except FileNotFoundError:
        print("ERROR: cargo not found. Is Rust installed?", file=sys.stderr)
        return 1
    except Exception as e:
        print(f"ERROR: Failed to run cargo doc: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
