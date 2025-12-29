#!/usr/bin/env python3
"""
Cross-platform pre-commit hook for Fortress Rollback.

This hook runs before each commit to ensure code quality and consistency.
It performs the following checks:
  1. Version synchronization (if Cargo.toml version changed)
  2. Code formatting (cargo fmt --check)

⚠️ NOTE: This is a STANDALONE script that uses --check mode (fails on unformatted code).
If you use the pre-commit framework (recommended), it uses run-cargo-fmt.py instead,
which auto-fixes formatting and stages modified files.

Installation (use pre-commit framework instead):
  pip install pre-commit
  pre-commit install

Or for legacy hook installation:
  - Linux/macOS: cp scripts/pre-commit .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit
  - Windows: copy scripts\\pre-commit.py .git\\hooks\\pre-commit

Works on Windows, macOS, and Linux.
"""

import os
import subprocess
import sys
from pathlib import Path


# ANSI color codes (work on most modern terminals including Windows 10+)
class Colors:
    RED = "\033[0;31m"
    GREEN = "\033[0;32m"
    YELLOW = "\033[1;33m"
    BLUE = "\033[0;34m"
    NC = "\033[0m"  # No Color


def supports_color() -> bool:
    """Check if the terminal supports color output."""
    # Check for NO_COLOR environment variable
    if os.environ.get("NO_COLOR"):
        return False
    # Check if stdout is a TTY
    if not hasattr(sys.stdout, "isatty") or not sys.stdout.isatty():
        return False
    # On Windows, enable ANSI escape sequences
    if sys.platform == "win32":
        try:
            import ctypes

            kernel32 = ctypes.windll.kernel32
            # Enable ANSI escape sequences on Windows 10+
            kernel32.SetConsoleMode(kernel32.GetStdHandle(-11), 7)
            return True
        except Exception:
            return False
    return True


def colored(text: str, color: str) -> str:
    """Return colored text if terminal supports it."""
    if supports_color():
        return f"{color}{text}{Colors.NC}"
    return text


def get_project_root() -> Path:
    """Get the git project root directory."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            check=True,
        )
        return Path(result.stdout.strip())
    except subprocess.CalledProcessError:
        # Fallback to script directory's parent
        return Path(__file__).parent.parent.resolve()


def get_staged_files(pattern: str = "") -> list[str]:
    """Get list of staged files, optionally filtered by pattern."""
    try:
        result = subprocess.run(
            ["git", "diff", "--cached", "--name-only", "--diff-filter=ACM"],
            capture_output=True,
            text=True,
            check=True,
        )
        files = result.stdout.strip().split("\n")
        files = [f for f in files if f]  # Remove empty strings

        if pattern:
            import re

            regex = re.compile(pattern)
            files = [f for f in files if regex.search(f)]

        return files
    except subprocess.CalledProcessError:
        return []


def check_cargo_fmt() -> bool:
    """Check if Rust code is properly formatted."""
    rust_files = get_staged_files(r"\.rs$")

    if not rust_files:
        print(colored("[OK]", Colors.GREEN) + " No Rust files to check")
        return True

    print(colored("Checking code formatting...", Colors.BLUE))

    try:
        result = subprocess.run(
            ["cargo", "fmt", "--check"],
            capture_output=True,
            text=True,
        )

        if result.returncode != 0:
            print(colored("[FAIL] Code formatting issues detected", Colors.RED))
            print(colored("Run 'cargo fmt' to fix formatting.", Colors.YELLOW))
            return False
        else:
            print(colored("[OK]", Colors.GREEN) + " Code formatting OK")
            return True

    except FileNotFoundError:
        print(
            colored("[WARN] cargo not found, skipping format check", Colors.YELLOW)
        )
        return True


def check_version_sync(project_root: Path) -> bool:
    """Check if version references are consistent."""
    # Check if Cargo.toml is staged
    staged_files = get_staged_files(r"^Cargo\.toml$")

    sync_script = project_root / "scripts" / "sync-version.sh"

    # On Windows, we can't run bash scripts directly
    # Skip this check on Windows or when bash isn't available
    if sys.platform == "win32":
        print(
            colored("[SKIP]", Colors.YELLOW)
            + " Version sync check (requires bash, skipped on Windows)"
        )
        return True

    if not sync_script.exists():
        return True

    if staged_files:
        # Check if version line changed
        try:
            result = subprocess.run(
                ["git", "diff", "--cached", "Cargo.toml"],
                capture_output=True,
                text=True,
                check=True,
            )
            if '+version = "' in result.stdout:
                print(
                    colored(
                        "Cargo.toml version changed, checking synchronization...",
                        Colors.YELLOW,
                    )
                )
                # Run sync script
                sync_result = subprocess.run(
                    [str(sync_script)],
                    capture_output=True,
                    text=True,
                )
                if sync_result.returncode != 0:
                    print(
                        colored(
                            "Version references updated. Stage changes and retry.",
                            Colors.YELLOW,
                        )
                    )
                    return False
        except (subprocess.CalledProcessError, FileNotFoundError):
            pass  # Script not found or failed; skip auto-sync and rely on consistency check below

    # Always check consistency
    try:
        result = subprocess.run(
            [str(sync_script), "--check"],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(
                colored(
                    "[FAIL] Version references inconsistent with Cargo.toml",
                    Colors.RED,
                )
            )
            print(
                colored(
                    "Run './scripts/sync-version.sh' to fix this.",
                    Colors.YELLOW,
                )
            )
            return False
        else:
            print(colored("[OK]", Colors.GREEN) + " Version references consistent")
            return True
    except FileNotFoundError:
        # Bash not available
        return True


def main() -> int:
    """Run pre-commit checks."""
    print(colored("Running pre-commit checks...", Colors.BLUE))
    print()

    project_root = get_project_root()
    failed = False

    # Check 1: Version synchronization
    if not check_version_sync(project_root):
        failed = True

    print()

    # Check 2: Rust formatting
    if not check_cargo_fmt():
        failed = True

    print()

    # Final result
    if failed:
        print(colored("=" * 60, Colors.RED))
        print(colored("          Pre-commit checks FAILED", Colors.RED))
        print(colored("=" * 60, Colors.RED))
        print()
        print(colored("Please fix the issues above and try again.", Colors.YELLOW))
        print(
            colored(
                "To bypass (not recommended): git commit --no-verify",
                Colors.YELLOW,
            )
        )
        return 1
    else:
        print(colored("=" * 60, Colors.GREEN))
        print(colored("          All pre-commit checks PASSED", Colors.GREEN))
        print(colored("=" * 60, Colors.GREEN))
        return 0


if __name__ == "__main__":
    sys.exit(main())
