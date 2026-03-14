#!/usr/bin/env python3
"""Check that lint hook scripts follow output format conventions.

Validates:
- No leading whitespace on issue output lines (breaks editor hyperlinking)
- Error messages include line numbers in {path}:{line}: {message} format
- No Warning: prints that bypass the {path}:{line}: format convention
- No print() followed by return-in-list (causes duplicate output)
- No except-pass/except-return-fallback that swallows I/O errors (fail-open)
- No raw path variables in error output when file uses glob/rglob/iterdir
  (absolute paths break {path}:{line}: parsing on Windows due to drive letter
  colons -- use relative_to() or a display_path variable instead)
- No ERROR:/WARNING: diagnostic prints going to stdout (must use file=sys.stderr)

Cross-platform: Works on Linux, macOS, and Windows.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path


def check_file(filepath: Path, repo_root: Path | None = None) -> list[str]:
    """Check a hook script for output format violations.

    Returns a list of issue descriptions (empty if no issues).
    When repo_root is provided, paths in output are relative to it.
    """
    issues: list[str] = []

    if repo_root is not None:
        try:
            display_path = filepath.relative_to(repo_root)
        except ValueError:
            display_path = filepath
    else:
        display_path = filepath

    try:
        content = filepath.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        return [f"{display_path}:0: cannot read file: {exc}"]

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
                f"{display_path}:{line_num}: print() with leading whitespace "
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
                    f"{display_path}:{line_num}: error message missing line "
                    f"number -- use {{path}}:0: for file-level errors"
                )

        # Check 3: Warning: prints that bypass lint format
        # Detect print(f"Warning: ...") which produces output that doesn't
        # follow {path}:{line}: {message} format and is often duplicative
        # when check_file() also returns a formatted error.
        if re.search(r'''print\(r?fr?["']Warning:\s''', stripped):
            issues.append(
                f"{display_path}:{line_num}: print() with Warning: prefix "
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
                            f"{display_path}:{line_num}: print() followed by "
                            f"return-in-list causes duplicate output -- "
                            f"remove the print() and let the caller print"
                        )
                        break

        # Check 5: Fail-open anti-pattern (except + pass/return fallback)
        # Detect except blocks for I/O errors that silently swallow the
        # error via `pass` or by returning a fallback/default value.
        # This allows hooks to exit 0 on unreadable files.
        if re.search(
            r"except\s+\(?\s*(?:OSError|UnicodeDecodeError|IOError)"
            r"[\w\s,]*\)?\s*:",
            stripped,
        ):
            # Look ahead up to 2 lines for `pass` or `return <default>`
            for ahead in range(1, 3):
                idx = line_num - 1 + ahead
                if idx < len(lines):
                    next_line = lines[idx].strip()
                    # Skip blank lines
                    if not next_line:
                        continue
                    # Bare `pass` swallows the error entirely
                    if next_line == "pass" or next_line.startswith(("pass ", "pass\t")):
                        issues.append(
                            f"{display_path}:{line_num}: except block swallows "
                            f"I/O error with pass -- fail closed by "
                            f"returning an error or re-raising"
                        )
                        break
                    # `return True` treats an unreadable file as valid
                    if re.search(r"return\s+True\b", next_line):
                        issues.append(
                            f"{display_path}:{line_num}: except block returns "
                            f"True on I/O error -- fail closed by "
                            f"returning False or raising"
                        )
                        break
                    # `return []` or `return ""` or `return ''` treats
                    # an unreadable file as having no issues/content
                    if re.search(r"return\s+(\[\]|\"\"|\'\')(\s|$)", next_line):
                        issues.append(
                            f"{display_path}:{line_num}: except block returns "
                            f"empty value on I/O error -- fail closed by "
                            f"returning an error indicator"
                        )
                        break
                    # Any other statement means the except block has real
                    # logic -- stop looking
                    break

        # Check 7: ERROR/WARNING prints going to stdout instead of stderr
        # Detect print() calls containing ERROR: or WARNING: diagnostic
        # prefixes that do not include file=sys.stderr.  These messages
        # must go to stderr per project conventions.
        #
        # Handles both single-line and multi-line cases:
        #   Single-line:  print("ERROR: something")
        #   Multi-line:   print(
        #                     f"  WARNING: proof ..."
        #                 )
        # For multi-line, we check if we're inside an open print() call
        # (tracked by in_print_call) and look for ERROR:/WARNING: on
        # continuation lines.
        if re.search(
            r"""print\(.*(?:ERROR|WARNING)\s*:""", stripped
        ) and "file=sys.stderr" not in stripped:
            issues.append(
                f"{display_path}:{line_num}: print() with ERROR:/WARNING: diagnostic goes to stdout -- add file=sys.stderr"
            )

    # Check 7b: Multi-line print() calls with ERROR:/WARNING: on
    # continuation lines.  Walk the file a second time, tracking open
    # print( calls and scanning their bodies for diagnostic prefixes.
    in_print = False
    print_start_line = 0
    print_lines: list[str] = []
    paren_depth = 0
    for line_num, line in enumerate(lines, start=1):
        stripped = line.strip()
        if stripped.startswith("#") or not stripped:
            if in_print:
                # accumulate even blank/comment lines inside a print call
                print_lines.append(stripped)
            continue

        if not in_print:
            # Detect start of a print( call
            if re.match(r"print\(", stripped):
                in_print = True
                print_start_line = line_num
                print_lines = [stripped]
                paren_depth = stripped.count("(") - stripped.count(")")
                if paren_depth <= 0:
                    # Single-line — already handled by Check 7 above
                    in_print = False
        else:
            print_lines.append(stripped)
            paren_depth += stripped.count("(") - stripped.count(")")
            if paren_depth <= 0:
                # End of multi-line print call — analyse the joined body
                joined = " ".join(print_lines)
                has_diag = re.search(r"(?:ERROR|WARNING)\s*:", joined)
                has_stderr = "file=sys.stderr" in joined
                if has_diag and not has_stderr:
                    issues.append(
                        f"{display_path}:{print_start_line}: multi-line print() with ERROR:/WARNING: diagnostic goes to stdout -- add file=sys.stderr"
                    )
                in_print = False

    # Check 6: Raw path variables in error output when file uses glob/rglob
    # When a script discovers files via glob(), rglob(), or iterdir(), the
    # resulting Path objects are absolute. Using them directly in error
    # output (e.g., f"{filepath}:0:") produces absolute paths that break
    # the {path}:{line}: convention on Windows (drive letter colon, e.g.,
    # C:\...:0:). Scripts must convert to relative paths first.
    uses_glob = any(
        re.search(r"\.(rglob|glob|iterdir)\(", ln)
        for ln in lines
        if not ln.strip().startswith("#")
    )
    if uses_glob:
        # Allowed path variable names that indicate a relative/display path
        safe_vars = {"rel", "display_path", "rel_path", "relative", "rel_index"}
        for line_num, line in enumerate(lines, start=1):
            stripped = line.strip()
            if stripped.startswith("#") or not stripped:
                continue
            # Look for f-string path:line: patterns in error-producing lines
            path_in_fstring = re.search(
                r'''r?fr?["']\{(\w+)\}:\{?\w*\}?:''', stripped
            )
            if path_in_fstring:
                var_name = path_in_fstring.group(1)
                if var_name not in safe_vars:
                    issues.append(
                        f"{display_path}:{line_num}: f-string uses "
                        f"{{{var_name}}} which may be absolute (file uses "
                        f"glob/rglob/iterdir) -- use relative_to() or a "
                        f"display_path variable"
                    )

    return issues


def main() -> int:
    """Check lint hook scripts for output format violations."""
    files = sys.argv[1:] if len(sys.argv) > 1 else []
    repo_root = Path(__file__).resolve().parent.parent.parent

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
        issues = check_file(filepath, repo_root)
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
