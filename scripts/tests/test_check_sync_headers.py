#!/usr/bin/env python3
"""Unit tests for check-sync-headers.py hook."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_sync_headers",
    scripts_dir / "hooks" / "check-sync-headers.py",
)
check_sync_headers = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_sync_headers)

_check_file = check_sync_headers._check_file
_check_required_pair = check_sync_headers._check_required_pair
_extract_sync_target = check_sync_headers._extract_sync_target
main = check_sync_headers.main


def _write(path: Path, content: str) -> None:
    """Write UTF-8 text after creating parent directories."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def _assert_no_absolute_paths(issues: list[str]) -> None:
    """Verify no diagnostic messages contain absolute filesystem paths.

    Diagnostics follow the format ``path:line: message``.
    The path portion must never start with ``/`` or a Windows drive letter.
    """
    for issue in issues:
        assert not issue.startswith("/"), (
            f"diagnostic contains absolute path: {issue}"
        )
        # Windows drive letter check (e.g., "C:\...")
        if len(issue) >= 3 and issue[1] == ":" and issue[0].isalpha() and issue[2] in ("\\/"):
            raise AssertionError(f"diagnostic contains absolute path: {issue}")


def _write_sync_script(repo_root: Path, mapping: dict[str, str]) -> None:
    """Create a minimal sync-wiki.py with an AST-parsable WIKI_STRUCTURE."""
    entries = "\n".join(
        f'    "{docs_rel}": "{wiki_name}",' for docs_rel, wiki_name in mapping.items()
    )
    _write(
        repo_root / "scripts" / "docs" / "sync-wiki.py",
        f"WIKI_STRUCTURE = {{\n{entries}\n}}\n",
    )


class TestRequiredPairDiagnostics:
    """Tests for required docs/wiki pair diagnostics."""

    @pytest.mark.parametrize(
        ("wiki_filename", "expected_fragment"),
        [
            (
                "Replay.md",
                "remediation: run python scripts/docs/sync-wiki.py",
            ),
            (
                "replay.md",
                "case mismatch; found wiki/replay.md",
            ),
        ],
        ids=["missing-wiki", "case-mismatch"],
    )
    def test_missing_wiki_mirror_reports_actionable_message(
        self,
        tmp_path: Path,
        wiki_filename: str,
        expected_fragment: str,
    ) -> None:
        """Missing required wiki files include remediation and case mismatch hints."""
        _write(
            tmp_path / "docs" / "replay.md",
            "<!-- SYNC: This source doc syncs to wiki/Replay.md. -->\n",
        )

        if wiki_filename != "Replay.md":
            _write(
                tmp_path / "wiki" / wiki_filename,
                "<!-- SYNC: This source doc syncs to docs/replay.md. -->\n",
            )

        issues = _check_required_pair(tmp_path, "replay.md", "Replay")

        assert len(issues) == 1
        assert expected_fragment in issues[0]
        _assert_no_absolute_paths(issues)


class TestCheckFileDiagnostics:
    """Tests for free-form SYNC header diagnostics."""

    def test_missing_target_reports_case_mismatch(self, tmp_path: Path) -> None:
        """A missing target points to likely casing errors when possible."""
        _write(
            tmp_path / "docs" / "replay.md",
            "<!-- SYNC: This source doc syncs to wiki/Replay.md. -->\n",
        )
        _write(
            tmp_path / "wiki" / "replay.md",
            "<!-- SYNC: This source doc syncs to docs/replay.md. -->\n",
        )

        issues = _check_file(tmp_path, Path("docs/replay.md"))

        assert len(issues) == 1
        assert "case mismatch; found wiki/replay.md" in issues[0]
        _assert_no_absolute_paths(issues)


class TestPathDiagnostics:
    """Ensure all diagnostic paths are repo-relative, never absolute."""

    def test_check_file_case_mismatch_uses_relative_paths(self, tmp_path: Path) -> None:
        """_check_file diagnostics use relative paths even without chdir."""
        _write(tmp_path / "docs" / "replay.md",
               "<!-- SYNC: This source doc syncs to wiki/Replay.md. -->\n")
        _write(tmp_path / "wiki" / "replay.md",  # wrong case
               "<!-- SYNC: This source doc syncs to docs/replay.md. -->\n")

        issues = _check_file(tmp_path, Path("docs/replay.md"))
        assert len(issues) == 1
        _assert_no_absolute_paths(issues)

    def test_check_required_pair_case_mismatch_uses_relative_paths(self, tmp_path: Path) -> None:
        """_check_required_pair diagnostics use relative paths even without chdir."""
        _write(tmp_path / "docs" / "replay.md",
               "<!-- SYNC: This source doc syncs to wiki/Replay.md. -->\n")
        _write(tmp_path / "wiki" / "replay.md",  # wrong case
               "<!-- SYNC: This source doc syncs to docs/replay.md. -->\n")

        issues = _check_required_pair(tmp_path, "replay.md", "Replay")
        assert len(issues) == 1
        _assert_no_absolute_paths(issues)

    def test_check_file_missing_target_uses_relative_paths(self, tmp_path: Path) -> None:
        """Missing target diagnostics use relative paths."""
        _write(tmp_path / "docs" / "replay.md",
               "<!-- SYNC: This source doc syncs to wiki/Replay.md. -->\n")
        # Don't create wiki/Replay.md at all

        issues = _check_file(tmp_path, Path("docs/replay.md"))
        assert len(issues) == 1
        _assert_no_absolute_paths(issues)

    def test_check_file_reciprocal_mismatch_uses_relative_paths(self, tmp_path: Path) -> None:
        """Reciprocal mismatch diagnostics use relative paths."""
        _write(tmp_path / "docs" / "replay.md",
               "<!-- SYNC: This source doc syncs to wiki/Replay.md. -->\n")
        _write(tmp_path / "wiki" / "Replay.md",
               "<!-- SYNC: This source doc syncs to docs/other.md. -->\n")

        issues = _check_file(tmp_path, Path("docs/replay.md"))
        assert len(issues) == 1
        _assert_no_absolute_paths(issues)

    def test_extract_sync_target_error_uses_relative_path(self, tmp_path: Path) -> None:
        """_extract_sync_target OSError diagnostics use relative paths."""
        missing = tmp_path / "docs" / "nonexistent.md"
        with pytest.raises(OSError) as exc_info:
            _extract_sync_target(missing, repo_root=tmp_path)
        msg = str(exc_info.value)
        assert not msg.startswith("/"), f"error contains absolute path: {msg}"

    def test_check_file_unreadable_returns_issue_not_exception(self, tmp_path: Path) -> None:
        """_check_file gracefully handles unreadable files as diagnostics."""
        _write(tmp_path / "docs" / "replay.md",
               "<!-- SYNC: This source doc syncs to wiki/Replay.md. -->\n")
        # Create the target as a directory so reading it as a file raises OSError.
        (tmp_path / "wiki" / "Replay.md").mkdir(parents=True)

        issues = _check_file(tmp_path, Path("docs/replay.md"))
        # The target exists (as a directory) but _extract_sync_target raises OSError.
        assert len(issues) >= 1
        _assert_no_absolute_paths(issues)


class TestMain:
    """Integration tests for the hook entrypoint."""

    def test_main_emits_remediation_hint(self, tmp_path: Path, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch) -> None:
        """When validation fails, the hook prints a global remediation hint."""
        _write_sync_script(tmp_path, {"replay.md": "Replay"})
        _write(
            tmp_path / "docs" / "replay.md",
            "<!-- SYNC: This source doc syncs to wiki/Replay.md. -->\n",
        )

        monkeypatch.chdir(tmp_path)
        exit_code = main()

        captured = capsys.readouterr()
        assert exit_code == 1
        assert "hint: regenerate wiki mirrors with `python scripts/docs/sync-wiki.py`" in captured.err

    def test_main_passes_for_valid_reciprocal_pair(
        self,
        tmp_path: Path,
        monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        """The hook succeeds when required docs/wiki pairs are reciprocal."""
        _write_sync_script(tmp_path, {"replay.md": "Replay"})
        _write(
            tmp_path / "docs" / "replay.md",
            "<!-- SYNC: This source doc syncs to wiki/Replay.md. -->\n",
        )
        _write(
            tmp_path / "wiki" / "Replay.md",
            "<!-- SYNC: This source doc syncs to docs/replay.md. -->\n",
        )

        monkeypatch.chdir(tmp_path)
        assert main() == 0
