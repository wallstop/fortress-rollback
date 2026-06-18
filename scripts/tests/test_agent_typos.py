#!/usr/bin/env python3
"""Unit tests for the agent-typos.py preflight wrapper.

The wrapper shells out to the real `typos` binary, so these tests mock
`shutil.which` and `subprocess.run` to exercise every branch deterministically
WITHOUT requiring typos to be installed:

- missing binary      -> soft-skip, exit 0
- typos found (code 2)-> propagate non-zero (blocking)
- clean (code 0)      -> exit 0
- OSError on invoke   -> soft-skip, exit 0
- config wiring       -> `--config .typos.toml` is passed when the file exists
"""

from __future__ import annotations

import importlib.util
import subprocess
from pathlib import Path

# Import the hook module (hyphenated filename requires importlib).
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "agent_typos",
    scripts_dir / "hooks" / "agent-typos.py",
)
assert spec is not None and spec.loader is not None
agent_typos = importlib.util.module_from_spec(spec)
spec.loader.exec_module(agent_typos)

main = agent_typos.main


class _FakeCompleted:
    def __init__(self, returncode: int) -> None:
        self.returncode = returncode


def test_missing_typos_soft_skips(monkeypatch, capsys) -> None:
    monkeypatch.setattr(agent_typos.shutil, "which", lambda _name: None)
    assert main() == 0
    assert "not installed" in capsys.readouterr().err


def test_typos_found_propagates_nonzero(monkeypatch) -> None:
    monkeypatch.setattr(agent_typos.shutil, "which", lambda _name: "/usr/bin/typos")
    monkeypatch.setattr(
        agent_typos.subprocess, "run", lambda *a, **k: _FakeCompleted(2)
    )
    assert main() == 2


def test_clean_run_returns_zero(monkeypatch) -> None:
    monkeypatch.setattr(agent_typos.shutil, "which", lambda _name: "/usr/bin/typos")
    monkeypatch.setattr(
        agent_typos.subprocess, "run", lambda *a, **k: _FakeCompleted(0)
    )
    assert main() == 0


def test_invoke_oserror_soft_skips(monkeypatch, capsys) -> None:
    monkeypatch.setattr(agent_typos.shutil, "which", lambda _name: "/usr/bin/typos")

    def _boom(*_a, **_k):
        raise OSError("vanished")

    monkeypatch.setattr(agent_typos.subprocess, "run", _boom)
    assert main() == 0
    assert "could not invoke typos" in capsys.readouterr().err


def test_config_flag_is_passed_when_present(monkeypatch, tmp_path) -> None:
    # Run from a temp dir that contains a .typos.toml so the wrapper adds the
    # `--config` flag. Capture the command handed to subprocess.run.
    (tmp_path / ".typos.toml").write_text("[default]\n", encoding="utf-8")
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(agent_typos.shutil, "which", lambda _name: "/usr/bin/typos")

    captured: dict[str, list[str]] = {}

    def _capture(cmd, *_a, **_k):
        captured["cmd"] = cmd
        return _FakeCompleted(0)

    monkeypatch.setattr(agent_typos.subprocess, "run", _capture)
    assert main() == 0
    assert "--config" in captured["cmd"]
    assert ".typos.toml" in captured["cmd"]


def test_no_config_flag_when_absent(monkeypatch, tmp_path) -> None:
    # No .typos.toml in CWD -> the wrapper invokes typos with no --config.
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(agent_typos.shutil, "which", lambda _name: "/usr/bin/typos")

    captured: dict[str, list[str]] = {}

    def _capture(cmd, *_a, **_k):
        captured["cmd"] = cmd
        return _FakeCompleted(0)

    monkeypatch.setattr(agent_typos.subprocess, "run", _capture)
    assert main() == 0
    assert "--config" not in captured["cmd"]


def test_module_imports_subprocess() -> None:
    # Guard: the wrapper must keep the subprocess import the tests patch.
    assert agent_typos.subprocess is subprocess
