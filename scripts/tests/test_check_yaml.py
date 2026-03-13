#!/usr/bin/env python3
"""
Unit tests for check-yaml.py hook.

Verifies that the YAML validator correctly detects invalid YAML files
and that output follows the {path}:{line}: {message} format.
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
    "check_yaml", scripts_dir / "hooks" / "check-yaml.py"
)
check_yaml = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_yaml)

check_file = check_yaml.check_file
main = check_yaml.main
HAS_YAML = check_yaml.HAS_YAML


def _write(directory: Path, name: str, content: str) -> Path:
    """Helper to create a file with given content."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_text(content, encoding="utf-8")
    return filepath


@pytest.mark.skipif(not HAS_YAML, reason="PyYAML not installed")
class TestYamlValidation:
    """Tests for YAML validation."""

    def test_valid_yaml_passes(self, tmp_path: Path) -> None:
        """A valid YAML file passes."""
        f = _write(tmp_path, "config.yaml", "key: value\nlist:\n  - item1\n  - item2\n")
        assert check_file(str(f)) is True

    def test_empty_yaml_passes(self, tmp_path: Path) -> None:
        """An empty YAML file passes."""
        f = _write(tmp_path, "empty.yaml", "")
        assert check_file(str(f)) is True

    def test_invalid_yaml_fails(self, tmp_path: Path) -> None:
        """An invalid YAML file fails."""
        f = _write(tmp_path, "bad.yaml", "key: value\n  bad_indent: oops\n")
        assert check_file(str(f)) is False

    def test_invalid_yaml_with_tabs_fails(self, tmp_path: Path) -> None:
        """YAML with tab indentation fails."""
        f = _write(tmp_path, "tabs.yaml", "key:\n\t- value\n")
        assert check_file(str(f)) is False

    def test_valid_yaml_with_anchors_passes(self, tmp_path: Path) -> None:
        """YAML with anchors/aliases passes."""
        content = "defaults: &defaults\n  key: value\nconfig:\n  <<: *defaults\n"
        f = _write(tmp_path, "anchors.yaml", content)
        assert check_file(str(f)) is True


@pytest.mark.skipif(not HAS_YAML, reason="PyYAML not installed")
class TestOutputFormat:
    """Tests that output follows {path}:{line_number}: {message} format."""

    def test_output_starts_with_path_colon_line(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Error output first line must start with path:line: (no leading whitespace)."""
        f = _write(tmp_path, "bad.yaml", "key: value\n  bad_indent: oops\n")
        check_file(str(f))
        captured = capsys.readouterr()
        # Only check the first line -- YAML exceptions produce multiline str(e)
        first_line = captured.err.splitlines()[0]
        assert re.match(r'^.+:\d+: ', first_line), f"Bad format: {first_line}"
        assert not first_line.startswith(" "), f"Leading whitespace: {first_line}"

    def test_output_contains_yaml_error(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Error output contains 'YAML error'."""
        f = _write(tmp_path, "bad.yaml", "key: value\n  bad_indent: oops\n")
        check_file(str(f))
        captured = capsys.readouterr()
        assert "YAML error" in captured.err

    def test_read_error_uses_zero_line_number(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Read error message must include :0: synthetic line number."""
        path = tmp_path / "nonexistent.yaml"
        check_file(str(path))
        captured = capsys.readouterr()
        for line in captured.err.splitlines():
            if line:
                assert ":0:" in line, f"Missing :0: in read error: {line}"
                assert re.match(r'^.+:\d+: ', line), f"Bad format: {line}"


@pytest.mark.skipif(not HAS_YAML, reason="PyYAML not installed")
class TestMain:
    """Tests for the main() entry point."""

    def test_main_no_args_returns_zero(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        monkeypatch.setattr(sys, "argv", ["check-yaml.py"])
        assert main() == 0

    def test_main_valid_file_returns_zero(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "good.yaml", "key: value\n")
        monkeypatch.setattr(sys, "argv", ["check-yaml.py", str(f)])
        assert main() == 0

    def test_main_invalid_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "bad.yaml", "key: value\n  bad: indent\n")
        monkeypatch.setattr(sys, "argv", ["check-yaml.py", str(f)])
        assert main() == 1

    def test_main_multiple_files(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Returns 1 if any file is invalid."""
        good = _write(tmp_path, "good.yaml", "key: value\n")
        bad = _write(tmp_path, "bad.yaml", "key: value\n  bad: indent\n")
        monkeypatch.setattr(
            sys, "argv", ["check-yaml.py", str(good), str(bad)]
        )
        assert main() == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
