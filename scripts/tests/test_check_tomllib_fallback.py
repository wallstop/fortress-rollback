#!/usr/bin/env python3
"""Unit tests for the check-tomllib-fallback.py hook.

The hook uses `ast`, so it flags only REAL `tomllib` imports (not mentions in
comments, strings, or docstring code examples) that lack a `tomli` fallback.
These tests cover both directions (flag / don't-flag), the `ast`-specific
robustness cases, and a dynamic meta-test asserting no real repo Python file is
flagged.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

# Import the hook module (hyphenated filename requires importlib).
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_tomllib_fallback",
    scripts_dir / "hooks" / "check-tomllib-fallback.py",
)
assert spec is not None and spec.loader is not None
check_tomllib_fallback = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_tomllib_fallback)

check_file = check_tomllib_fallback.check_file
main = check_tomllib_fallback.main

REPO_ROOT = Path(__file__).resolve().parents[2]

# The canonical compliant pattern, mirrored from scripts/hooks/check-toml.py.
_FALLBACK = (
    "try:\n"
    "    import tomllib\n"
    "except ImportError:\n"
    "    import tomli as tomllib\n"
)


def _write(directory: Path, name: str, content: str) -> Path:
    """Create a file with given content and return its path."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_text(content, encoding="utf-8")
    return filepath


class TestViolations:
    """Files that should be flagged."""

    def test_bare_tomllib_import_is_flagged(self, tmp_path: Path) -> None:
        f = _write(tmp_path, "mod.py", "import tomllib\n\nx = 1\n")
        errors = check_file(f)
        assert len(errors) == 1
        assert "no `tomli` fallback" in errors[0]
        assert f"{f}:1:" in errors[0]

    def test_indented_tomllib_without_fallback_is_flagged(
        self, tmp_path: Path
    ) -> None:
        # `import tomllib` indented in a try block, but the except imports the
        # WRONG module (not tomli) -- still missing the real fallback.
        content = "try:\n    import tomllib\nexcept ImportError:\n    pass\n"
        f = _write(tmp_path, "mod.py", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert f"{f}:2:" in errors[0]

    def test_from_tomllib_import_is_flagged(self, tmp_path: Path) -> None:
        # `from tomllib import loads` also fails on Python < 3.11.
        f = _write(tmp_path, "mod.py", "from tomllib import loads\n")
        errors = check_file(f)
        assert len(errors) == 1
        assert f"{f}:1:" in errors[0]

    def test_aliased_tomllib_import_is_flagged(self, tmp_path: Path) -> None:
        # `import tomllib as t` is still the stdlib module under another name.
        f = _write(tmp_path, "mod.py", "import tomllib as t\n")
        assert len(check_file(f)) == 1

    def test_line_number_points_at_the_import(self, tmp_path: Path) -> None:
        content = "import os\nimport sys\nimport tomllib\n"
        f = _write(tmp_path, "mod.py", content)
        errors = check_file(f)
        assert errors and f"{f}:3:" in errors[0]


class TestCompliant:
    """Files that should NOT be flagged."""

    def test_try_except_fallback_is_accepted(self, tmp_path: Path) -> None:
        f = _write(tmp_path, "mod.py", _FALLBACK)
        assert check_file(f) == []

    def test_no_tomllib_import_is_ignored(self, tmp_path: Path) -> None:
        f = _write(tmp_path, "mod.py", "import json\nimport re\n")
        assert check_file(f) == []

    def test_direct_tomli_backport_is_ignored(self, tmp_path: Path) -> None:
        # Using only the backport (no stdlib tomllib import) is fine.
        f = _write(tmp_path, "mod.py", "import tomli as tomllib\n")
        assert check_file(f) == []

    def test_tomllib_in_comment_is_ignored(self, tmp_path: Path) -> None:
        # A comment mentioning the import must not match (no real statement).
        f = _write(tmp_path, "mod.py", "# import tomllib here someday\nx = 1\n")
        assert check_file(f) == []

    def test_tomllib_in_docstring_prose_is_ignored(self, tmp_path: Path) -> None:
        f = _write(
            tmp_path,
            "mod.py",
            '"""We could import tomllib but choose tomli."""\nx = 1\n',
        )
        assert check_file(f) == []

    def test_tomllib_in_docstring_code_block_is_ignored(
        self, tmp_path: Path
    ) -> None:
        # An indented `import tomllib` inside a docstring code example is NOT a
        # real import -- the ast-based check must not flag it (this is exactly
        # the false-positive class a line-regex would hit). No `tomli` anywhere.
        content = '"""Example:\n\n    import tomllib\n"""\nx = 1\n'
        f = _write(tmp_path, "mod.py", content)
        assert check_file(f) == []

    def test_tomllib_substring_module_is_ignored(self, tmp_path: Path) -> None:
        # `import tomllib_helper` is a different module, not `tomllib`.
        f = _write(tmp_path, "mod.py", "import tomllib_helper\n")
        assert check_file(f) == []

    def test_syntax_error_file_is_ignored(self, tmp_path: Path) -> None:
        # Python that fails to parse cannot import anything; skip gracefully.
        f = _write(tmp_path, "mod.py", "import tomllib\ndef (:\n")
        assert check_file(f) == []


class TestMain:
    """End-to-end behavior of main() over argv."""

    def test_main_returns_one_on_violation(
        self, tmp_path: Path, monkeypatch
    ) -> None:
        f = _write(tmp_path, "bad.py", "import tomllib\n")
        monkeypatch.setattr("sys.argv", ["check-tomllib-fallback.py", str(f)])
        assert main() == 1

    def test_main_returns_zero_when_compliant(
        self, tmp_path: Path, monkeypatch
    ) -> None:
        f = _write(tmp_path, "good.py", _FALLBACK)
        monkeypatch.setattr("sys.argv", ["check-tomllib-fallback.py", str(f)])
        assert main() == 0

    def test_main_ignores_non_python_files(
        self, tmp_path: Path, monkeypatch
    ) -> None:
        # A non-.py file that literally contains `import tomllib` is skipped.
        f = _write(tmp_path, "notes.txt", "import tomllib\n")
        monkeypatch.setattr("sys.argv", ["check-tomllib-fallback.py", str(f)])
        assert main() == 0


class TestRepoFilesComply:
    """No real repo Python file imports tomllib without a fallback.

    A dynamic scan (rather than a fixed list) so any future tomllib-importing
    script is covered automatically -- including this hook itself and
    agent-preflight.py, both of which contain the literal `import tomllib` in
    docstrings/help strings that the ast-based check correctly ignores.
    """

    def test_no_scripts_python_file_is_flagged(self) -> None:
        scripts_root = REPO_ROOT / "scripts"
        flagged = {
            str(path.relative_to(REPO_ROOT)): check_file(path)
            for path in sorted(scripts_root.rglob("*.py"))
            if check_file(path)
        }
        assert flagged == {}, f"tomllib-fallback violations: {flagged}"
