#!/usr/bin/env python3
"""
Unit tests for check-dockerfile.py hook.

Verifies that the Dockerfile anti-pattern checker correctly detects
pip install without --no-cache-dir, command -v with >&2 redirect,
and eval "$(..." without command -v guard.
"""

from __future__ import annotations

import importlib.util
import re
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


class TestUnguardedEval:
    """Tests for unguarded eval "$(..." detection."""

    def test_unguarded_eval_detected(self, tmp_path: Path) -> None:
        """eval "$(tool init bash)" without command -v guard is flagged."""
        f = _write(
            tmp_path,
            "Dockerfile",
            'RUN echo \'eval "$(zoxide init bash)"\' >> ~/.bashrc\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert 'eval "$(...)"' in issues[0]
        assert "command -v" in issues[0]

    def test_guarded_eval_passes(self, tmp_path: Path) -> None:
        """eval "$(..." with command -v guard on the same line passes."""
        f = _write(
            tmp_path,
            "Dockerfile",
            'RUN echo \'command -v zoxide >/dev/null 2>&1 && eval "$(zoxide init bash)"\' >> ~/.bashrc\n',
        )
        issues = check_file(f)
        assert issues == []

    def test_unguarded_eval_single_quotes(self, tmp_path: Path) -> None:
        """eval '$(tool init bash)' without guard is flagged."""
        f = _write(
            tmp_path,
            "Dockerfile",
            "RUN echo \"eval '$(starship init bash)'\" >> ~/.bashrc\n",
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert 'eval "$(...)"' in issues[0]

    def test_unguarded_eval_no_quotes(self, tmp_path: Path) -> None:
        """eval $(tool init bash) without guard is flagged."""
        f = _write(
            tmp_path,
            "Dockerfile",
            "RUN echo 'eval $(atuin init bash)' >> ~/.bashrc\n",
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert 'eval "$(...)"' in issues[0]

    def test_unguarded_eval_in_devcontainer_json(self, tmp_path: Path) -> None:
        """eval "$(..." in devcontainer.json without guard is flagged."""
        f = _write(
            tmp_path,
            "devcontainer.json",
            '{"postCreateCommand": "eval \\"$(zoxide init bash)\\"" }\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert 'eval "$(...)"' in issues[0]

    def test_guarded_eval_in_devcontainer_json_passes(self, tmp_path: Path) -> None:
        """eval "$(..." with command -v in devcontainer.json passes."""
        f = _write(
            tmp_path,
            "devcontainer.json",
            '{"postCreateCommand": "command -v zoxide >/dev/null 2>&1 && eval \\"$(zoxide init bash)\\"" }\n',
        )
        issues = check_file(f)
        assert issues == []

    def test_comment_line_skipped(self, tmp_path: Path) -> None:
        """Commented-out eval is not flagged in Dockerfiles."""
        f = _write(
            tmp_path,
            "Dockerfile",
            '# eval "$(zoxide init bash)"\n',
        )
        issues = check_file(f)
        assert issues == []

    def test_multiple_unguarded_evals(self, tmp_path: Path) -> None:
        """Multiple unguarded eval lines each produce a separate issue."""
        content = (
            "FROM ubuntu:22.04\n"
            'RUN echo \'eval "$(zoxide init bash)"\' >> ~/.bashrc\n'
            'RUN echo \'eval "$(starship init bash)"\' >> ~/.bashrc\n'
        )
        f = _write(tmp_path, "Dockerfile", content)
        issues = check_file(f)
        assert len(issues) == 2

    def test_mixed_guarded_and_unguarded(self, tmp_path: Path) -> None:
        """Only unguarded evals are flagged when mixed with guarded ones."""
        content = (
            "FROM ubuntu:22.04\n"
            'RUN echo \'eval "$(zoxide init bash)"\' >> ~/.bashrc\n'
            'RUN echo \'command -v starship >/dev/null 2>&1 && eval "$(starship init bash)"\' >> ~/.bashrc\n'
        )
        f = _write(tmp_path, "Dockerfile", content)
        issues = check_file(f)
        assert len(issues) == 1
        assert ":2:" in issues[0]

    def test_eval_on_continuation_line(self, tmp_path: Path) -> None:
        """eval on a continuation line is flagged."""
        content = (
            "FROM ubuntu:22.04\n"
            "RUN echo 'first line \\\n"
            '  eval "$(zoxide init bash)"\' >> ~/.bashrc\n'
        )
        f = _write(tmp_path, "Dockerfile", content)
        issues = check_file(f)
        assert len(issues) == 1
        assert ":3:" in issues[0]


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


class TestOutputFormat:
    """Tests that output follows {path}:{line_number}: {message} format."""

    def test_issues_start_with_path_colon_line(self, tmp_path: Path) -> None:
        """Each issue must start with path:line: (no leading whitespace)."""
        f = _write(tmp_path, "Dockerfile", "RUN pip install requests\n")
        issues = check_file(f)
        assert len(issues) == 1
        # Must match path:line_number: pattern
        assert re.match(r'^.+:\d+: ', issues[0]), f"Bad format: {issues[0]}"
        # Must not start with whitespace
        assert not issues[0].startswith(" "), f"Leading whitespace: {issues[0]}"

    def test_main_output_no_leading_whitespace(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
        capsys: pytest.CaptureFixture[str],
    ) -> None:
        """main() prints issue lines without leading whitespace."""
        f = _write(tmp_path, "Dockerfile", "RUN pip install requests\n")
        monkeypatch.setattr(sys, "argv", ["check-dockerfile.py", str(f)])
        # Import main from the module
        check_dockerfile.main()
        captured = capsys.readouterr()
        # Each non-header, non-summary line should not start with spaces
        for line in captured.err.splitlines():
            if line and not line.startswith(("Dockerfile anti-patterns", "\n")) and "issue(s) found" not in line:
                assert not line.startswith("  "), f"Leading indent: {line!r}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
