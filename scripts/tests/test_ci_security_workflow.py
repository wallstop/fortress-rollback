"""Contracts for standalone Cargo workspace security coverage."""

from __future__ import annotations

from pathlib import Path

import yaml

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 fallback
    import tomli as tomllib  # type: ignore[no-redef]

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_PATH = ROOT / ".github" / "workflows" / "ci-security.yml"

EXPECTED_WORKSPACES = {
    "root": ("Cargo.toml", "Cargo.lock"),
    "fuzz": ("fuzz/Cargo.toml", "fuzz/Cargo.lock"),
    "loom": ("loom-tests/Cargo.toml", "loom-tests/Cargo.lock"),
    "godot-emscripten": (
        "tests/godot-emscripten/Cargo.toml",
        "tests/godot-emscripten/Cargo.lock",
    ),
}
EXPECTED_LICENSE_CONFIGS = {
    "root": "deny.toml",
    "fuzz": "supply-chain/deny-fuzz-licenses.toml",
    "loom": "deny.toml",
    "godot-emscripten": "supply-chain/deny-godot-licenses.toml",
}
EXPECTED_LICENSE_EXCEPTIONS = {
    "libfuzzer-sys@0.4.13": ("NCSA", "fuzz/Cargo.lock"),
    "gdextension-api@0.5.1": ("MPL-2.0", "tests/godot-emscripten/Cargo.lock"),
    "godot@0.5.5": ("MPL-2.0", "tests/godot-emscripten/Cargo.lock"),
    "godot-bindings@0.5.5": ("MPL-2.0", "tests/godot-emscripten/Cargo.lock"),
    "godot-cell@0.5.5": ("MPL-2.0", "tests/godot-emscripten/Cargo.lock"),
    "godot-codegen@0.5.5": ("MPL-2.0", "tests/godot-emscripten/Cargo.lock"),
    "godot-core@0.5.5": ("MPL-2.0", "tests/godot-emscripten/Cargo.lock"),
    "godot-ffi@0.5.5": ("MPL-2.0", "tests/godot-emscripten/Cargo.lock"),
    "godot-macros@0.5.5": ("MPL-2.0", "tests/godot-emscripten/Cargo.lock"),
}


def _workflow() -> dict:
    return yaml.safe_load(WORKFLOW_PATH.read_text(encoding="utf-8"))


def _workspace_matrix(job: dict) -> dict[str, tuple[str, str]]:
    entries = job["strategy"]["matrix"]["include"]
    return {
        entry["workspace"]: (entry["manifest"], entry["lockfile"])
        for entry in entries
    }


def _locked_packages(path: str) -> set[str]:
    with (ROOT / path).open("rb") as lock_file:
        packages = tomllib.load(lock_file)["package"]
    return {f'{package["name"]}@{package["version"]}' for package in packages}


def test_security_and_freshness_matrices_cover_every_authoritative_lock() -> None:
    jobs = _workflow()["jobs"]

    for job_name in ("supply-chain-security", "outdated"):
        job = jobs[job_name]
        assert _workspace_matrix(job) == EXPECTED_WORKSPACES
        assert job["strategy"]["fail-fast"] is False

    security_entries = jobs["supply-chain-security"]["strategy"]["matrix"]["include"]
    assert {
        entry["workspace"]: entry["license-config"] for entry in security_entries
    } == EXPECTED_LICENSE_CONFIGS


def test_security_commands_are_explicit_and_fail_closed() -> None:
    job = _workflow()["jobs"]["supply-chain-security"]
    steps = job["steps"]
    run_blocks = "\n".join(str(step.get("run", "")) for step in steps)

    assert 'cargo audit --file "$LOCKFILE"' in run_blocks
    assert 'cargo deny --manifest-path "$MANIFEST" --locked check advisories bans sources' in run_blocks
    assert (
        'cargo deny --manifest-path "$MANIFEST" --locked '
        '--config "$LICENSE_CONFIG" check licenses'
    ) in run_blocks
    assert all(not step.get("continue-on-error", False) for step in steps)


def test_freshness_reports_are_required_but_version_lag_is_advisory() -> None:
    job = _workflow()["jobs"]["outdated"]
    steps = job["steps"]
    run_blocks = "\n".join(str(step.get("run", "")) for step in steps)

    assert 'cargo outdated --manifest-path "$MANIFEST" --root-deps-only' in run_blocks
    assert 'test -s "$REPORT"' in run_blocks
    assert all(not step.get("continue-on-error", False) for step in steps)


def test_license_exceptions_are_exact_and_lockfile_scoped() -> None:
    with (ROOT / "deny.toml").open("rb") as deny_file:
        root_licenses = tomllib.load(deny_file)["licenses"]

    assert "exceptions" not in root_licenses

    scoped_configs = (
        "supply-chain/deny-fuzz-licenses.toml",
        "supply-chain/deny-godot-licenses.toml",
    )
    exceptions = []
    for config in scoped_configs:
        with (ROOT / config).open("rb") as config_file:
            licenses = tomllib.load(config_file)["licenses"]
        assert licenses["allow"] == root_licenses["allow"]
        exceptions.extend(licenses["exceptions"])

    actual = {
        exception["crate"]: exception["allow"]
        for exception in exceptions
    }
    assert actual == {
        package: [license_id]
        for package, (license_id, _) in EXPECTED_LICENSE_EXCEPTIONS.items()
    }

    packages_by_lock = {
        lockfile: _locked_packages(lockfile)
        for _, lockfile in EXPECTED_WORKSPACES.values()
    }
    for package, (_, expected_lock) in EXPECTED_LICENSE_EXCEPTIONS.items():
        containing_locks = {
            lockfile
            for lockfile, packages in packages_by_lock.items()
            if package in packages
        }
        assert containing_locks == {expected_lock}
