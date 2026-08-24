"""Contracts for the crates.io source archive allowlist."""

from __future__ import annotations

import re
import shutil
import subprocess
from pathlib import Path

import pytest

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 fallback
    import tomli as tomllib  # type: ignore[no-redef]

ROOT = Path(__file__).resolve().parents[2]

EXPECTED_INCLUDE = [
    "/src/**/*.rs",
    "/tests/network/wire_golden_legacy_0_9.rs",
    "/README.md",
    "/LICENSE-APACHE",
    "/LICENSE-MIT",
]
GENERATED_ROOT_FILES = {
    ".cargo_vcs_info.json",
    "Cargo.lock",
    "Cargo.toml",
    "Cargo.toml.orig",
}
PUBLISHED_ROOT_FILES = {"README.md", "LICENSE-APACHE", "LICENSE-MIT"}
SELF_TEST_FILES = {"tests/network/wire_golden_legacy_0_9.rs"}


def test_manifest_uses_one_anchored_package_allowlist() -> None:
    with (ROOT / "Cargo.toml").open("rb") as manifest_file:
        package = tomllib.load(manifest_file)["package"]

    assert package["include"] == EXPECTED_INCLUDE
    assert "exclude" not in package
    assert all(pattern.startswith("/") for pattern in package["include"])


def test_packaged_readme_does_not_link_to_excluded_paths() -> None:
    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    relative_targets = {
        match.group(1).split("#", 1)[0]
        for match in re.finditer(r"\]\((?!https?://|#|mailto:)([^) ]+)", readme)
    }
    assert relative_targets == {"LICENSE-MIT", "LICENSE-APACHE"}
    assert all(
        target.startswith(("https://", "http://"))
        for target in re.findall(r'(?:src|href)="([^"]+)"', readme)
    )


@pytest.mark.skipif(shutil.which("cargo") is None, reason="cargo is required")
def test_cargo_package_list_contains_only_runtime_sources_and_metadata() -> None:
    result = subprocess.run(
        ["cargo", "package", "--list", "--allow-dirty"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr

    paths = {line for line in result.stdout.splitlines() if line}
    root_files = {path for path in paths if "/" not in path}
    source_files = {path for path in paths if path.startswith("src/")}

    assert root_files == GENERATED_ROOT_FILES | PUBLISHED_ROOT_FILES
    assert source_files
    assert paths == root_files | source_files | SELF_TEST_FILES
