#!/usr/bin/env python3
"""
Unit tests for check-dockerfile.py hook.

Verifies that the Dockerfile anti-pattern checker correctly detects
pip install without --no-cache-dir and command -v with >&2 redirect.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

# Import the hook module (hyphenated filename requires importlib)
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_dockerfile", scripts_dir / "hooks" / "check-dockerfile.py"
)
check_dockerfile = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_dockerfile)

check_file = check_dockerfile.check_file


def _write(directory: Path, name: str, content: str) -> Path:
    """Helper to create a file with given content."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_text(content, encoding="utf-8")
    return filepath


class TestPipNoCacheDir:
    """Tests for pip install --no-cache-dir detection."""

    def test_pip_install_without_flag_detected(self, tmp_path: Path) -> None:
        """pip install without --no-cache-dir is flagged."""
        f = _write(tmp_path, "Dockerfile", "RUN pip install requests\n")
        issues = check_file(f)
        assert len(issues) == 1
        assert "--no-cache-dir" in issues[0]

    def test_pip3_install_without_flag_detected(self, tmp_path: Path) -> None:
        """pip3 install without --no-cache-dir is flagged."""
        f = _write(tmp_path, "Dockerfile", "RUN pip3 install z3-solver\n")
        issues = check_file(f)
        assert len(issues) == 1
        assert "--no-cache-dir" in issues[0]

    def test_pip_install_with_flag_passes(self, tmp_path: Path) -> None:
        """pip install with --no-cache-dir passes."""
        f = _write(tmp_path, "Dockerfile", "RUN pip install --no-cache-dir requests\n")
        issues = check_file(f)
        assert issues == []

    def test_pip3_install_with_flag_passes(self, tmp_path: Path) -> None:
        """pip3 install with --no-cache-dir passes."""
        f = _write(
            tmp_path,
            "Dockerfile",
            "RUN pip3 install --no-cache-dir --break-system-packages z3\n",
        )
        issues = check_file(f)
        assert issues == []

    def test_comment_line_skipped(self, tmp_path: Path) -> None:
        """Comment lines with pip install are not flagged in Dockerfiles."""
        f = _write(tmp_path, "Dockerfile", "# RUN pip install requests\n")
        issues = check_file(f)
        assert issues == []

    def test_pip_in_json_detected(self, tmp_path: Path) -> None:
        """pip install in devcontainer.json is detected."""
        f = _write(
            tmp_path,
            "devcontainer.json",
            '{"postCreateCommand": "pip install foo"}\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "--no-cache-dir" in issues[0]


class TestCommandVRedirect:
    """Tests for command -v stderr redirect detection."""

    def test_stderr_redirect_detected(self, tmp_path: Path) -> None:
        """command -v with >&2 is flagged."""
        f = _write(
            tmp_path,
            "Dockerfile",
            'RUN if command -v sccache >&2; then export RUSTC_WRAPPER=sccache; fi\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert ">/dev/null 2>&1" in issues[0]

    def test_devnull_redirect_passes(self, tmp_path: Path) -> None:
        """command -v with >/dev/null 2>&1 passes."""
        f = _write(
            tmp_path,
            "Dockerfile",
            "RUN command -v sccache >/dev/null 2>&1\n",
        )
        issues = check_file(f)
        assert issues == []

    def test_stderr_redirect_in_json(self, tmp_path: Path) -> None:
        """command -v with >&2 in devcontainer.json is flagged."""
        f = _write(
            tmp_path,
            "devcontainer.json",
            '{"cmd": "if command -v vale >&2; then vale sync; fi"}\n',
        )
        issues = check_file(f)
        assert len(issues) == 1

    def test_comment_line_skipped(self, tmp_path: Path) -> None:
        """Commented-out command -v is not flagged in Dockerfiles."""
        f = _write(tmp_path, "Dockerfile", "# command -v sccache >&2\n")
        issues = check_file(f)
        assert issues == []


class TestFileHandling:
    """Tests for file type detection and error handling."""

    def test_clean_dockerfile_returns_empty(self, tmp_path: Path) -> None:
        """A Dockerfile with no issues returns empty list."""
        f = _write(
            tmp_path,
            "Dockerfile",
            "FROM ubuntu:22.04\nRUN apt-get update\n",
        )
        issues = check_file(f)
        assert issues == []

    def test_nonexistent_file_returns_empty(self, tmp_path: Path) -> None:
        """Nonexistent file returns empty list with stderr warning."""
        issues = check_file(tmp_path / "nonexistent")
        assert issues == []

    def test_multiple_issues_in_one_file(self, tmp_path: Path) -> None:
        """Multiple anti-patterns in one file are all detected."""
        content = (
            "FROM ubuntu:22.04\n"
            "RUN pip install requests\n"
            "RUN pip3 install flask\n"
            'RUN if command -v tool >&2; then echo "found"; fi\n'
        )
        f = _write(tmp_path, "Dockerfile", content)
        issues = check_file(f)
        assert len(issues) == 3

    def test_dockerfile_with_suffix(self, tmp_path: Path) -> None:
        """Dockerfile.dev is also checked."""
        f = _write(tmp_path, "Dockerfile.dev", "RUN pip install foo\n")
        issues = check_file(f)
        assert len(issues) == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
