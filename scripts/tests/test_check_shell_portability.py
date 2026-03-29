#!/usr/bin/env python3
"""Tests for scripts/ci/check-shell-portability.sh."""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER_SOURCE = REPO_ROOT / "scripts" / "ci" / "check-shell-portability.sh"


def _setup_repo(tmp_path: Path) -> tuple[Path, Path]:
    """Create a temporary repo containing the portability checker."""
    repo = tmp_path / "repo"
    checker = repo / "scripts" / "ci" / "check-shell-portability.sh"
    checker.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(CHECKER_SOURCE, checker)
    return repo, checker


def _write_script(repo: Path, rel_path: str, content: str) -> Path:
    """Write a shell script fixture into the temporary repo."""
    path = repo / rel_path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    path.chmod(0o755)
    return path


def _run_checker(checker: Path) -> subprocess.CompletedProcess[str]:
    """Run the checker script and capture stdout/stderr."""
    return subprocess.run(
        ["bash", str(checker)],
        cwd=checker.parent.parent.parent,
        capture_output=True,
        text=True,
        check=False,
    )


def test_detects_nonportable_word_boundary_escape(tmp_path: Path) -> None:
    """grep -E with \\b is reported as non-portable."""
    repo, checker = _setup_repo(tmp_path)
    _write_script(
        repo,
        "scripts/sample.sh",
        "#!/bin/bash\n"
        "grep -qE '\\\\bEq\\\\b' sample.txt\n",
    )

    result = _run_checker(checker)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "Non-portable regex escape" in combined
    assert "\\b" in combined


def test_detects_nonportable_whitespace_and_word_escapes(tmp_path: Path) -> None:
    """grep -E with \\s or \\w is reported as non-portable."""
    repo, checker = _setup_repo(tmp_path)
    _write_script(
        repo,
        "scripts/sample.sh",
        "#!/bin/bash\n"
        "grep -qE '^\\s*pub\\s+\\w+$' sample.txt\n",
    )

    result = _run_checker(checker)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "Non-portable regex escape" in combined


def test_detects_nonportable_digit_escape(tmp_path: Path) -> None:
    """grep -E with \\d is reported as non-portable."""
    repo, checker = _setup_repo(tmp_path)
    _write_script(
        repo,
        "scripts/sample.sh",
        "#!/bin/bash\n"
        "grep -qE '^\\d+$' sample.txt\n",
    )

    result = _run_checker(checker)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "Non-portable regex escape" in combined


def test_detects_nonportable_pattern_assignment(tmp_path: Path) -> None:
    """Variable-assigned regex escapes are also reported."""
    repo, checker = _setup_repo(tmp_path)
    _write_script(
        repo,
        "scripts/sample.sh",
        "#!/bin/bash\n"
        "pattern='\\\\bEq\\\\b'\n"
        "grep -qE \"$pattern\" sample.txt\n",
    )

    result = _run_checker(checker)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "pattern assignment" in combined


def test_detects_nonportable_pattern_append_assignment(tmp_path: Path) -> None:
    """Append assignments with regex escapes are also reported."""
    repo, checker = _setup_repo(tmp_path)
    _write_script(
        repo,
        "scripts/sample.sh",
        "#!/bin/bash\n"
        "patterns+=('\\\\w+')\n"
        "grep -qE \"${patterns[0]}\" sample.txt\n",
    )

    result = _run_checker(checker)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "pattern assignment" in combined


def test_detects_nonportable_pattern_array_assignment(tmp_path: Path) -> None:
    """Array index assignments with regex escapes are reported."""
    repo, checker = _setup_repo(tmp_path)
    _write_script(
        repo,
        "scripts/sample.sh",
        "#!/bin/bash\n"
        "patterns[0]='\\\\d+'\n"
        "grep -qE \"${patterns[0]}\" sample.txt\n",
    )

    result = _run_checker(checker)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "pattern assignment" in combined


def test_ignores_invalid_spaced_equals_pseudo_assignment(tmp_path: Path) -> None:
    """Spacing around '=' is not a valid shell assignment and is ignored."""
    repo, checker = _setup_repo(tmp_path)
    _write_script(
        repo,
        "scripts/sample.sh",
        "#!/bin/bash\n"
        "pattern = '\\\\bEq\\\\b'\n",
    )

    result = _run_checker(checker)

    assert result.returncode == 0


def test_allows_posix_classes_for_boundaries(tmp_path: Path) -> None:
    """POSIX-safe ERE patterns are accepted."""
    repo, checker = _setup_repo(tmp_path)
    _write_script(
        repo,
        "scripts/sample.sh",
        "#!/bin/bash\n"
        "grep -qE '(^|[^[:alnum:]_])Eq([^[:alnum:]_]|$)' sample.txt\n"
        "grep -qE '^[[:space:]]*pub[[:space:]]+[[:alnum:]_]+$' sample.txt\n",
    )

    result = _run_checker(checker)

    assert result.returncode == 0


def test_ignores_echoed_examples(tmp_path: Path) -> None:
    """Descriptive echo lines containing regex text are ignored."""
    repo, checker = _setup_repo(tmp_path)
    _write_script(
        repo,
        "scripts/sample.sh",
        "#!/bin/bash\n"
        "echo \"Use grep -E '\\\\bword\\\\b' patterns with care\"\n",
    )

    result = _run_checker(checker)

    assert result.returncode == 0
