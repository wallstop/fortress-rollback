#!/usr/bin/env python3
"""Check that lint hook scripts follow output format conventions.

Validates:
- No leading whitespace on issue output lines (breaks editor hyperlinking)
- Error messages include line numbers in {path}:{line}: {message} format
- No Warning: prints that bypass the {path}:{line}: format convention
- No print() followed by return-in-list (causes duplicate output)

Cross-platform: Works on Linux, macOS, and Windows.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path


def check_file(filepath: Path) -> list[str]:
    """Check a hook script for output format violations.

    Returns a list of issue descriptions (empty if no issues).
    """
    issues: list[str] = []

    try:
        content = filepath.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        return [f"{filepath}:0: cannot read file: {exc}"]

    lines = content.splitlines()

    for line_num, line in enumerate(lines, start=1):
        stripped = line.strip()

        # Skip comments and blank lines
        if stripped.startswith("#") or not stripped:
            continue

        # Check 1: Indented issue printing in f-strings
        # Detect print(f"  {variable}") which adds leading whitespace to
        # lint output, breaking editor hyperlinking on the path:line: prefix.
        if re.search(r"""print\(r?fr?["']\s+\{""", stripped):
            issues.append(
                f"{filepath}:{line_num}: print() with leading whitespace "
                f"in f-string breaks editor hyperlinking -- remove the "
                f"leading spaces"
            )

        # Check 2: Error messages missing line numbers
        # Detect f"{path_var}: cannot read" that should be f"{path_var}:0: cannot read"
        match = re.search(r'''r?fr?["']\{(\w+)\}:\s+cannot\s+read''', stripped)
        if match:
            var_name = match.group(1)
            # Not already in {path}:{num}: or {path}:{var}: format
            has_line_num = re.search(
                r"\{" + re.escape(var_name) + r"\}:\d+:", stripped
            )
            has_line_var = re.search(
                r"\{" + re.escape(var_name) + r"\}:\{", stripped
            )
            if not has_line_num and not has_line_var:
                issues.append(
                    f"{filepath}:{line_num}: error message missing line "
                    f"number -- use {{path}}:0: for file-level errors"
                )

        # Check 3: Warning: prints that bypass lint format
        # Detect print(f"Warning: ...") which produces output that doesn't
        # follow {path}:{line}: {message} format and is often duplicative
        # when check_file() also returns a formatted error.
        if re.search(r'''print\(r?fr?["']Warning:\s''', stripped):
            issues.append(
                f"{filepath}:{line_num}: print() with Warning: prefix "
                f"bypasses {{path}}:{{line}}: format -- return a "
                f"formatted error instead"
            )

        # Check 4: Dual-output anti-pattern (print + return in list)
        # Detect print(..., file=sys.stderr) followed by return [...] within
        # 3 lines. This causes duplicate output: check_file() prints AND
        # returns the message, then main() prints it again from the list.
        if re.search(r"print\(.*file=sys\.stderr\)", stripped):
            # Look ahead up to 3 lines for a return-list
            for ahead in range(1, 4):
                if line_num - 1 + ahead < len(lines):
                    next_line = lines[line_num - 1 + ahead].strip()
                    if re.search(r"return\s+\[(?!\])", next_line):
                        issues.append(
                            f"{filepath}:{line_num}: print() followed by "
                            f"return-in-list causes duplicate output -- "
                            f"remove the print() and let the caller print"
                        )
                        break

    return issues


def main() -> int:
    """Check lint hook scripts for output format violations."""
    files = sys.argv[1:] if len(sys.argv) > 1 else []

    if not files:
        # Scan all check-*.py files in scripts/hooks/
        hooks_dir = Path(__file__).parent
        for path in sorted(hooks_dir.glob("check-*.py")):
            # Don't check ourselves
            if path.name == "check-hook-output-format.py":
                continue
            files.append(str(path))

    if not files:
        return 0

    all_issues: list[str] = []

    for arg in files:
        filepath = Path(arg)
        if not filepath.name.endswith(".py"):
            continue
        issues = check_file(filepath)
        all_issues.extend(issues)

    if all_issues:
        print("Hook output format violations detected:", file=sys.stderr)
        for issue in all_issues:
            print(issue, file=sys.stderr)
        print(f"\n{len(all_issues)} violation(s) found.", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
