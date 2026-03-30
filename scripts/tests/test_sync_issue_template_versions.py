#!/usr/bin/env python3
"""Unit tests for sync-issue-template-versions.py transformations."""

from __future__ import annotations

import importlib.util
import json as _json
import sys
import urllib.error
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "sync_issue_template_versions",
    scripts_dir / "ci" / "sync-issue-template-versions.py",
)
sync_mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync_mod)

build_version_block = sync_mod.build_version_block
update_template = sync_mod.update_template
BEGIN_SENTINEL = sync_mod.BEGIN_SENTINEL
END_SENTINEL = sync_mod.END_SENTINEL
TEMPLATE_PATH = sync_mod.TEMPLATE_PATH


def _make_template(
    versions: list[str],
    indent: str = "        ",
    post_sentinel: str = "",
) -> str:
    """Build a minimal template string with versions between sentinels."""
    block = "\n".join(
        [f"{indent}{BEGIN_SENTINEL}"]
        + [f"{indent}- {v}" for v in versions]
        + [f"{indent}{END_SENTINEL}"]
    )
    body = f"options:\n{block}\n"
    if post_sentinel:
        body += post_sentinel
    return body


def _mock_response(data: list[dict]) -> MagicMock:
    """Create a mock urlopen response that works as a context manager."""
    mock = MagicMock()
    mock.__enter__ = MagicMock(return_value=mock)
    mock.__exit__ = MagicMock(return_value=False)
    mock.read.return_value = _json.dumps(data).encode()
    mock.headers = {}
    return mock


class TestBuildVersionBlock:
    def test_basic(self) -> None:
        result = build_version_block(["v1.0.0", "v0.9.0"], "  ")
        assert "  # BEGIN_FORTRESS_VERSIONS" in result
        assert "  - v1.0.0" in result
        assert "  - v0.9.0" in result
        assert "  # END_FORTRESS_VERSIONS" in result
        assert result.index("v1.0.0") < result.index("v0.9.0")

    def test_empty_versions(self) -> None:
        result = build_version_block([], "")
        assert BEGIN_SENTINEL in result
        assert END_SENTINEL in result
        lines = result.splitlines()
        assert lines[0] == BEGIN_SENTINEL
        assert lines[-1] == END_SENTINEL
        assert len(lines) == 2

    def test_indent_preserved(self) -> None:
        result = build_version_block(["v1.0.0"], "        ")
        assert result.startswith("        " + BEGIN_SENTINEL)


class TestUpdateTemplate:
    def test_happy_path(self) -> None:
        template = _make_template(["v0.1.0"])
        new_content, changed = update_template(template, ["v1.0.0", "v0.9.0"])
        assert changed
        assert "- v1.0.0" in new_content
        assert "- v0.9.0" in new_content
        assert "- v0.1.0" not in new_content

    def test_already_up_to_date(self) -> None:
        versions = ["v1.0.0", "v0.9.0"]
        template = _make_template(versions)
        new_content, changed = update_template(template, versions)
        assert not changed
        assert new_content == template

    def test_missing_begin_sentinel_raises(self) -> None:
        template = f"options:\n        {END_SENTINEL}\n"
        with pytest.raises(RuntimeError, match="missing.*BEGIN_FORTRESS_VERSIONS"):
            update_template(template, ["v1.0.0"])

    def test_missing_end_sentinel_raises(self) -> None:
        template = f"options:\n        {BEGIN_SENTINEL}\n"
        with pytest.raises(RuntimeError, match="missing.*END_FORTRESS_VERSIONS"):
            update_template(template, ["v1.0.0"])

    def test_sentinels_wrong_order_raises(self) -> None:
        template = (
            f"options:\n"
            f"        {END_SENTINEL}\n"
            f"        {BEGIN_SENTINEL}\n"
        )
        with pytest.raises(RuntimeError, match="must appear before"):
            update_template(template, ["v1.0.0"])

    def test_indentation_preserved(self) -> None:
        indent = "        "
        template = _make_template(["v0.1.0"], indent=indent)
        new_content, changed = update_template(template, ["v2.0.0"])
        assert changed
        lines = new_content.splitlines()
        version_lines = [ln for ln in lines if "- v2.0.0" in ln]
        assert version_lines, "Expected at least one version line"
        assert version_lines[0].startswith(indent)

    def test_sentinel_lines_use_correct_indent(self) -> None:
        indent = "    "
        template = _make_template(["v0.1.0"], indent=indent)
        new_content, _ = update_template(template, ["v1.0.0"])
        assert f"{indent}{BEGIN_SENTINEL}" in new_content
        assert f"{indent}{END_SENTINEL}" in new_content

    def test_post_sentinel_content_preserved(self) -> None:
        post = "- Other / not listed\n"
        template = _make_template(["v0.1.0"], post_sentinel=post)
        new_content, changed = update_template(template, ["v1.0.0"])
        assert changed
        assert "Other / not listed" in new_content
        assert new_content.endswith(post)


class TestFetchVersions:
    def test_single_page(self) -> None:
        releases = [
            {"tag_name": "v1.0.0", "prerelease": False, "draft": False},
            {"tag_name": "v0.9.0", "prerelease": False, "draft": False},
        ]
        with patch("urllib.request.urlopen", return_value=_mock_response(releases)):
            versions = sync_mod.fetch_versions()
        assert versions == ["v1.0.0", "v0.9.0"]

    def test_multi_page_pagination(self) -> None:
        page1 = [
            {"tag_name": f"v1.{i}.0", "prerelease": False, "draft": False}
            for i in range(100)
        ]
        page2 = [
            {"tag_name": "v0.1.0", "prerelease": False, "draft": False},
        ]
        responses = iter([_mock_response(page1), _mock_response(page2)])
        with patch("urllib.request.urlopen", side_effect=lambda _req: next(responses)):
            versions = sync_mod.fetch_versions()
        assert len(versions) == 101
        assert "v0.1.0" in versions

    def test_prerelease_filtered(self) -> None:
        releases = [
            {"tag_name": "v1.0.0-beta", "prerelease": True, "draft": False},
            {"tag_name": "v1.0.0", "prerelease": False, "draft": False},
        ]
        with patch("urllib.request.urlopen", return_value=_mock_response(releases)):
            versions = sync_mod.fetch_versions()
        assert "v1.0.0-beta" not in versions
        assert "v1.0.0" in versions

    def test_draft_filtered(self) -> None:
        releases = [
            {"tag_name": "v2.0.0-draft", "prerelease": False, "draft": True},
            {"tag_name": "v1.0.0", "prerelease": False, "draft": False},
        ]
        with patch("urllib.request.urlopen", return_value=_mock_response(releases)):
            versions = sync_mod.fetch_versions()
        assert "v2.0.0-draft" not in versions
        assert "v1.0.0" in versions

    def test_http_error_raises_runtime_error(self) -> None:
        exc = urllib.error.HTTPError(
            url="http://example.com", code=403, msg="Forbidden", hdrs={}, fp=None
        )
        exc.read = lambda: b"not authorized"
        with patch("urllib.request.urlopen", side_effect=exc):
            with pytest.raises(RuntimeError, match="HTTP 403"):
                sync_mod.fetch_versions()

    def test_url_error_raises_runtime_error(self) -> None:
        exc = urllib.error.URLError(reason="Name or service not known")
        with patch("urllib.request.urlopen", side_effect=exc):
            with pytest.raises(RuntimeError, match="network error"):
                sync_mod.fetch_versions()

    def test_empty_response(self) -> None:
        with patch("urllib.request.urlopen", return_value=_mock_response([])):
            versions = sync_mod.fetch_versions()
        assert versions == []


class TestMain:
    def test_already_up_to_date(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        versions = ["v1.0.0", "v0.9.0"]
        template_file = tmp_path / "bug_report.yml"
        template_file.write_text(_make_template(versions))
        monkeypatch.setattr(sync_mod, "TEMPLATE_PATH", str(template_file))
        monkeypatch.setattr(sys, "argv", ["prog"])
        with patch.object(sync_mod, "fetch_versions", return_value=versions):
            result = sync_mod.main()
        assert result == 0
        assert template_file.read_text() == _make_template(versions)

    def test_template_updated(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        template_file = tmp_path / "bug_report.yml"
        template_file.write_text(_make_template(["v0.1.0"]))
        monkeypatch.setattr(sync_mod, "TEMPLATE_PATH", str(template_file))
        monkeypatch.setattr(sys, "argv", ["prog"])
        with patch.object(sync_mod, "fetch_versions", return_value=["v1.0.0", "v0.1.0"]):
            result = sync_mod.main()
        assert result == 0
        new_text = template_file.read_text()
        assert "- v1.0.0" in new_text
        assert "- v0.1.0" in new_text

    def test_dry_run(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture) -> None:
        template_file = tmp_path / "bug_report.yml"
        original = _make_template(["v0.1.0"])
        template_file.write_text(original)
        monkeypatch.setattr(sync_mod, "TEMPLATE_PATH", str(template_file))
        monkeypatch.setattr(sys, "argv", ["prog", "--dry-run"])
        with patch.object(sync_mod, "fetch_versions", return_value=["v1.0.0", "v0.1.0"]):
            result = sync_mod.main()
        assert result == 0
        assert template_file.read_text() == original
        out = capsys.readouterr().out
        assert "Would update version list:" in out
        assert "- v1.0.0" in out
        assert "- v0.1.0" in out
        # Verify no leading whitespace before '-' in version lines
        for line in out.splitlines():
            if line.startswith("-"):
                assert not line.startswith("  -"), f"Unexpected leading whitespace in: {line!r}"

    def test_check_mode_out_of_date(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        template_file = tmp_path / "bug_report.yml"
        template_file.write_text(_make_template(["v0.1.0"]))
        monkeypatch.setattr(sync_mod, "TEMPLATE_PATH", str(template_file))
        monkeypatch.setattr(sys, "argv", ["prog", "--check"])
        with patch.object(sync_mod, "fetch_versions", return_value=["v1.0.0", "v0.1.0"]):
            result = sync_mod.main()
        assert result == 1

    def test_check_mode_up_to_date(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        versions = ["v1.0.0", "v0.1.0"]
        template_file = tmp_path / "bug_report.yml"
        template_file.write_text(_make_template(versions))
        monkeypatch.setattr(sync_mod, "TEMPLATE_PATH", str(template_file))
        monkeypatch.setattr(sys, "argv", ["prog", "--check"])
        with patch.object(sync_mod, "fetch_versions", return_value=versions):
            result = sync_mod.main()
        assert result == 0

    def test_file_read_error(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.setattr(sync_mod, "TEMPLATE_PATH", str(tmp_path / "nonexistent.yml"))
        monkeypatch.setattr(sys, "argv", ["prog"])
        result = sync_mod.main()
        assert result == 1

    def test_fetch_error_propagates(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        template_file = tmp_path / "bug_report.yml"
        template_file.write_text(_make_template(["v0.1.0"]))
        monkeypatch.setattr(sync_mod, "TEMPLATE_PATH", str(template_file))
        monkeypatch.setattr(sys, "argv", ["prog"])
        with patch.object(sync_mod, "fetch_versions", side_effect=RuntimeError("network error fetching releases from URL: reason")):
            result = sync_mod.main()
        assert result == 1

    def test_empty_versions_returns_error(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        template_file = tmp_path / "bug_report.yml"
        template_file.write_text(_make_template(["v0.1.0"]))
        monkeypatch.setattr(sync_mod, "TEMPLATE_PATH", str(template_file))
        monkeypatch.setattr(sys, "argv", ["prog"])
        with patch.object(sync_mod, "fetch_versions", return_value=[]):
            result = sync_mod.main()
        assert result == 1
