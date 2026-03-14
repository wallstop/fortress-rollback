#!/usr/bin/env python3
"""
Unit tests for regenerate-skills-index.py hook.

Verifies that the skills index generator correctly handles metadata
extraction, error propagation, and fail-closed behavior on unreadable files.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

# Import the hook module (hyphenated filename requires importlib)
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "regenerate_skills_index",
    scripts_dir / "hooks" / "regenerate-skills-index.py",
)
regenerate_skills_index = importlib.util.module_from_spec(spec)
spec.loader.exec_module(regenerate_skills_index)

extract_metadata = regenerate_skills_index.extract_metadata
build_index = regenerate_skills_index.build_index
main = regenerate_skills_index.main


def _write(directory: Path, name: str, content: str) -> Path:
    """Helper to create a file with given content."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_text(content, encoding="utf-8")
    return filepath


class TestExtractMetadata:
    """Tests for extract_metadata() functionality."""

    def test_extracts_category_and_when(self, tmp_path: Path) -> None:
        """Extracts CATEGORY and WHEN from metadata comments."""
        f = _write(
            tmp_path,
            "test-skill.md",
            "<!-- CATEGORY: Testing -->\n<!-- WHEN: Writing tests -->\n# Test Skill\n",
        )
        result = extract_metadata(f)
        assert result is not None
        category, when = result
        assert category == "Testing"
        assert when == "Writing tests"

    def test_fallback_to_heading_when_no_when_comment(self, tmp_path: Path) -> None:
        """Falls back to first heading when WHEN comment is absent."""
        f = _write(
            tmp_path,
            "test-skill.md",
            "<!-- CATEGORY: Tools -->\n# My Skill Guide\n",
        )
        result = extract_metadata(f)
        assert result is not None
        category, when = result
        assert category == "Tools"
        assert when == "My Skill Guide"

    def test_fallback_to_filename_when_no_heading(self, tmp_path: Path) -> None:
        """Falls back to filename stem when no heading exists."""
        f = _write(
            tmp_path,
            "my-tool.md",
            "<!-- CATEGORY: Tools -->\nSome content without heading.\n",
        )
        result = extract_metadata(f)
        assert result is not None
        category, when = result
        assert category == "Tools"
        assert when == "my tool"

    def test_fallback_to_defaults_when_no_metadata(self, tmp_path: Path) -> None:
        """Falls back to Uncategorized and heading when no comments exist."""
        f = _write(
            tmp_path,
            "basic.md",
            "# Basic Guide\nSome content.\n",
        )
        result = extract_metadata(f)
        assert result is not None
        category, when = result
        assert category == "Uncategorized"
        assert when == "Basic Guide"

    def test_nonexistent_file_returns_none(self, tmp_path: Path) -> None:
        """Nonexistent file returns None (error), not fallback metadata."""
        result = extract_metadata(tmp_path / "nonexistent.md")
        assert result is None

    def test_nonexistent_file_prints_to_stderr(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Nonexistent file prints error to stderr with :0: format."""
        extract_metadata(tmp_path / "nonexistent.md")
        captured = capsys.readouterr()
        assert ":0:" in captured.err
        assert "cannot read file" in captured.err

    def test_unreadable_file_returns_none(self, tmp_path: Path) -> None:
        """Unreadable file returns None (error), not fallback metadata."""
        f = _write(tmp_path, "noperm.md", "# Skill\n")
        f.chmod(0o000)
        try:
            result = extract_metadata(f)
            assert result is None
        finally:
            f.chmod(0o644)

    def test_binary_file_returns_none(self, tmp_path: Path) -> None:
        """Binary (non-UTF-8) file returns None (error), not fallback metadata."""
        f = tmp_path / "binary.md"
        f.write_bytes(b"\xff\xfe\x00\x01")
        result = extract_metadata(f)
        assert result is None


class TestBuildIndex:
    """Tests for build_index() error tracking."""

    def test_valid_skills_no_error(self, tmp_path: Path) -> None:
        """Valid skill files produce index content with no error."""
        _write(
            tmp_path,
            "skill-a.md",
            "<!-- CATEGORY: Cat -->\n<!-- WHEN: Do A -->\n# A\n",
        )
        content, had_error = build_index(tmp_path)
        assert not had_error
        assert "skill-a.md" in content

    def test_unreadable_skill_sets_had_error(self, tmp_path: Path) -> None:
        """Unreadable skill file sets had_error flag."""
        good = _write(
            tmp_path,
            "good.md",
            "<!-- CATEGORY: Cat -->\n<!-- WHEN: Do good -->\n# Good\n",
        )
        bad = _write(tmp_path, "bad.md", "# Bad\n")
        bad.chmod(0o000)
        try:
            content, had_error = build_index(tmp_path)
            assert had_error
            # Good file should still be in index
            assert "good.md" in content
            # Bad file should NOT be in index (skipped due to error)
            assert "bad.md" not in content
        finally:
            bad.chmod(0o644)

    def test_binary_skill_sets_had_error(self, tmp_path: Path) -> None:
        """Binary (non-UTF-8) skill file sets had_error flag."""
        _write(
            tmp_path,
            "valid.md",
            "<!-- CATEGORY: Cat -->\n<!-- WHEN: Valid -->\n# Valid\n",
        )
        binary = tmp_path / "binary.md"
        binary.write_bytes(b"\xff\xfe\x00\x01")
        _content, had_error = build_index(tmp_path)
        assert had_error

    def test_ignores_index_file(self, tmp_path: Path) -> None:
        """_index.md is not treated as a skill file."""
        _write(tmp_path, "_index.md", "# Old Index\n")
        _write(
            tmp_path,
            "real.md",
            "<!-- CATEGORY: Cat -->\n<!-- WHEN: Real -->\n# Real\n",
        )
        content, had_error = build_index(tmp_path)
        assert not had_error
        assert "real.md" in content
        assert "_index.md" not in content


class TestMain:
    """Tests for main() entry point fail-closed behavior."""

    def test_up_to_date_index_returns_zero(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Returns 0 when index is already up-to-date."""
        skills_dir = tmp_path / ".llm" / "skills"
        skills_dir.mkdir(parents=True)
        _write(
            skills_dir,
            "test.md",
            "<!-- CATEGORY: Cat -->\n<!-- WHEN: Test -->\n# Test\n",
        )
        # Generate index, write it, then run main() which should find up-to-date
        content, _had_error = build_index(skills_dir)
        (skills_dir / "_index.md").write_text(content, encoding="utf-8")
        # Patch __file__ so main() resolves to our tmp structure
        fake_script = tmp_path / "scripts" / "hooks" / "regenerate-skills-index.py"
        fake_script.parent.mkdir(parents=True, exist_ok=True)
        fake_script.write_text("", encoding="utf-8")
        monkeypatch.setattr(
            regenerate_skills_index, "__file__", str(fake_script)
        )
        monkeypatch.setattr(sys, "argv", ["regenerate-skills-index.py"])
        assert main() == 0

    def test_unreadable_skill_causes_nonzero_exit(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() returns 1 when a skill file cannot be read (fail-closed)."""
        skills_dir = tmp_path / ".llm" / "skills"
        skills_dir.mkdir(parents=True)
        bad = _write(skills_dir, "bad.md", "# Bad\n")
        bad.chmod(0o000)
        try:
            fake_script = tmp_path / "scripts" / "hooks" / "regenerate-skills-index.py"
            fake_script.parent.mkdir(parents=True, exist_ok=True)
            fake_script.write_text("", encoding="utf-8")
            monkeypatch.setattr(
                regenerate_skills_index, "__file__", str(fake_script)
            )
            monkeypatch.setattr(sys, "argv", ["regenerate-skills-index.py"])
            assert main() == 1
        finally:
            bad.chmod(0o644)

    def test_binary_skill_causes_nonzero_exit(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() returns 1 when a skill file is binary (fail-closed)."""
        skills_dir = tmp_path / ".llm" / "skills"
        skills_dir.mkdir(parents=True)
        binary = skills_dir / "binary.md"
        binary.write_bytes(b"\xff\xfe\x00\x01")
        fake_script = tmp_path / "scripts" / "hooks" / "regenerate-skills-index.py"
        fake_script.parent.mkdir(parents=True, exist_ok=True)
        fake_script.write_text("", encoding="utf-8")
        monkeypatch.setattr(
            regenerate_skills_index, "__file__", str(fake_script)
        )
        monkeypatch.setattr(sys, "argv", ["regenerate-skills-index.py"])
        assert main() == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
