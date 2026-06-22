#!/usr/bin/env python3
"""Behavioural tests for ``scripts/docs/verify-markdown-code.sh``.

The script:

* accepts a single markdown FILE,
* (after Chunk A.3) accepts a directory and recursively processes its
  ``*.md`` files,
* reports ``File or directory not found`` and exits ``1`` when given a
  path that resolves to neither a file nor a directory,
* honours ``--fail-fast`` to stop on the first compile failure.

These tests parametrise small markdown trees built under ``tmp_path`` and
drive the script as a subprocess via ``bash`` (rather than ``./``) so the
tests do not depend on the script's executable bit -- that invariant is
covered separately by ``test_workflow_script_executability.py``.

The script invokes ``cargo check`` against the project crate to compile
each Rust block. ``cargo`` is required; tests are skipped when it is
unavailable so this file can still be collected on minimal CI shards.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import textwrap
from dataclasses import dataclass, field
from pathlib import Path

import pytest

PROJECT_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = PROJECT_ROOT / "scripts" / "docs" / "verify-markdown-code.sh"

# A minimal Rust block that compiles cleanly inside the script's auto-wrapper.
GOOD_RUST_BLOCK = textwrap.dedent(
    """
    ```rust
    let _x: u32 = 1 + 2;
    ```
    """
).strip()

# A Rust block that the wrapper will attempt to compile and fail on.
# The reference to an undeclared identifier ``totally_undefined_symbol_xyz``
# is not in the script's auto-skip heuristic patterns, so it reaches cargo.
BAD_RUST_BLOCK = textwrap.dedent(
    """
    ```rust
    let _ = totally_undefined_symbol_xyz_42();
    ```
    """
).strip()

# A block that the script auto-skips via the ``no_run`` attribute -- used to
# confirm that markers suppress compilation rather than fail it.
NO_RUN_RUST_BLOCK = textwrap.dedent(
    """
    ```rust,no_run
    let _ = totally_undefined_symbol_xyz_42();
    ```
    """
).strip()

TYPE_ALIAS_EXCERPT_BLOCK = textwrap.dedent(
    """
    ```rust
    pub type ExampleResult<T, E = ExampleError> = std::result::Result<T, E>;
    ```
    """
).strip()


def _write_md(path: Path, body: str) -> Path:
    path.write_text(body + "\n", encoding="utf-8")
    return path


@dataclass(frozen=True)
class Case:
    """One parametrised invocation case.

    ``args_builder`` receives the per-test ``tmp_path`` (already populated
    by the fixture) and returns the CLI args to pass to the script. This
    indirection lets us reference paths the test fixture creates while
    keeping the parametrise list declarative.
    """

    name: str
    args_builder: object  # Callable[[Path], list[str]]
    expected_exit: int
    expected_substrings: tuple[str, ...] = field(default_factory=tuple)


def _build_tree(tmp_path: Path) -> dict[str, Path]:
    """Materialise a small markdown tree and return a name -> path map."""
    tree_root = tmp_path / "md_tree"
    tree_root.mkdir()

    good = _write_md(
        tree_root / "good.md",
        f"# Good\n\nA passing snippet:\n\n{GOOD_RUST_BLOCK}\n",
    )
    bad = _write_md(
        tree_root / "bad.md",
        f"# Bad\n\nAn intentionally-broken snippet:\n\n{BAD_RUST_BLOCK}\n",
    )
    no_run = _write_md(
        tree_root / "no_run.md",
        f"# No run\n\nMarked no_run so the bad code is auto-skipped:\n\n{NO_RUN_RUST_BLOCK}\n",
    )
    type_alias_excerpt = _write_md(
        tree_root / "type_alias_excerpt.md",
        f"# Type alias excerpt\n\n{TYPE_ALIAS_EXCERPT_BLOCK}\n",
    )

    only_good_dir = tmp_path / "only_good"
    only_good_dir.mkdir()
    only_good = _write_md(
        only_good_dir / "ok.md",
        f"# Only good\n\n{GOOD_RUST_BLOCK}\n",
    )

    only_bad_dir = tmp_path / "only_bad"
    only_bad_dir.mkdir()
    only_bad = _write_md(
        only_bad_dir / "broken.md",
        f"# Only bad\n\n{BAD_RUST_BLOCK}\n",
    )

    return {
        "good": good,
        "bad": bad,
        "no_run": no_run,
        "type_alias_excerpt": type_alias_excerpt,
        "tree_root": tree_root,
        "only_good": only_good,
        "only_good_dir": only_good_dir,
        "only_bad": only_bad,
        "only_bad_dir": only_bad_dir,
    }


@pytest.fixture(scope="session")
def cargo_target_dir(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """Share cargo build artifacts across subprocess cases."""
    return tmp_path_factory.mktemp("verify-markdown-code-target")


CASES: tuple[Case, ...] = (
    Case(
        name="single_good_file_passes",
        args_builder=lambda paths: [str(paths["only_good"])],
        expected_exit=0,
    ),
    Case(
        name="directory_only_good_passes",
        args_builder=lambda paths: [str(paths["only_good_dir"])],
        expected_exit=0,
    ),
    Case(
        name="non_existent_path_reports_not_found",
        args_builder=lambda paths: ["this/path/definitely/does/not/exist.md"],
        expected_exit=1,
        expected_substrings=("File or directory not found",),
    ),
    Case(
        name="fail_fast_on_bad_directory",
        args_builder=lambda paths: ["--fail-fast", str(paths["only_bad_dir"])],
        expected_exit=1,
    ),
    Case(
        name="no_run_block_is_auto_skipped",
        args_builder=lambda paths: [str(paths["no_run"])],
        expected_exit=0,
    ),
    Case(
        name="type_alias_excerpt_is_auto_skipped",
        args_builder=lambda paths: ["--verbose", str(paths["type_alias_excerpt"])],
        expected_exit=0,
        expected_substrings=("contains API declaration excerpt",),
    ),
)


@pytest.mark.skipif(
    shutil.which("cargo") is None,
    reason="cargo is required to compile rust code samples; skipping verify-markdown-code.sh tests.",
)
@pytest.mark.parametrize(
    "case",
    CASES,
    ids=[c.name for c in CASES],
)
def test_verify_markdown_code_behaviour(
    case: Case,
    tmp_path: Path,
    cargo_target_dir: Path,
) -> None:
    """Drive verify-markdown-code.sh and assert exit code + key substrings."""
    paths = _build_tree(tmp_path)
    args = case.args_builder(paths)  # type: ignore[operator]
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(cargo_target_dir)
    result = subprocess.run(
        ["bash", str(SCRIPT_PATH), *args],
        cwd=PROJECT_ROOT,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    combined = result.stdout + result.stderr
    assert result.returncode == case.expected_exit, (
        f"{case.name}: expected exit {case.expected_exit}, got "
        f"{result.returncode}.\n--- stdout ---\n{result.stdout}\n"
        f"--- stderr ---\n{result.stderr}"
    )
    missing = [s for s in case.expected_substrings if s not in combined]
    assert not missing, (
        f"{case.name}: missing expected substrings {missing!r}.\n"
        f"--- stdout ---\n{result.stdout}\n--- stderr ---\n{result.stderr}"
    )
