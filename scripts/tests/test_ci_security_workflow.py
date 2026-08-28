"""Contracts for standalone Cargo workspace security coverage."""

from __future__ import annotations

import re
import subprocess
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


def _run_unsafe_audit_producer(
    tmp_path: Path, output: str, status: int, *, tee_status: int = 0
) -> subprocess.CompletedProcess[str]:
    cargo_stub = tmp_path / "cargo"
    cargo_stub.write_text(
        "#!/bin/sh\n"
        "printf '%s\\n' \"${GEIGER_STUB_OUTPUT:?}\"\n"
        "exit \"${GEIGER_STUB_STATUS:?}\"\n",
        encoding="utf-8",
    )
    cargo_stub.chmod(0o755)
    if tee_status != 0:
        tee_stub = tmp_path / "tee"
        tee_stub.write_text(
            "#!/bin/sh\n"
            "/usr/bin/tee \"$@\"\n"
            "exit \"${TEE_STUB_STATUS:?}\"\n",
            encoding="utf-8",
        )
        tee_stub.chmod(0o755)
    audit_step = next(
        step
        for step in _workflow()["jobs"]["unsafe-audit"]["steps"]
        if step.get("name") == "Audit unsafe code in dependencies"
    )
    return subprocess.run(
        ["bash", "-c", audit_step["run"]],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        check=False,
        env={
            "PATH": f"{tmp_path}:/usr/bin:/bin",
            "GEIGER_STUB_OUTPUT": output,
            "GEIGER_STUB_STATUS": str(status),
            "TEE_STUB_STATUS": str(tee_status),
        },
    )


def _run_unsafe_audit_verifier(
    tmp_path: Path, report: str
) -> subprocess.CompletedProcess[str]:
    (tmp_path / "geiger-report.txt").write_text(report, encoding="utf-8")
    (tmp_path / "geiger-report.complete").touch()
    verify_step = next(
        step
        for step in _workflow()["jobs"]["unsafe-audit"]["steps"]
        if step.get("name") == "Verify no unsafe in library code"
    )
    return subprocess.run(
        ["bash", "-c", verify_step["run"]],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        check=False,
    )


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
        'check --config "$LICENSE_CONFIG" licenses'
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


def test_unsafe_audit_excludes_only_bundled_z3_from_feature_census() -> None:
    workflow = _workflow()
    job = workflow["jobs"]["unsafe-audit"]
    run_blocks = "\n".join(str(step.get("run", "")) for step in job["steps"])
    feature_match = re.search(r'--features "([^"]+)"', run_blocks)
    assert feature_match is not None

    with (ROOT / "Cargo.toml").open("rb") as manifest_file:
        manifest_features = set(tomllib.load(manifest_file)["features"])

    assert set(feature_match.group(1).split()) == manifest_features - {
        "z3-verification-bundled"
    }
    assert "--all-features" not in run_blocks
    assert "cargo geiger --locked" in run_blocks
    assert job["timeout-minutes"] == 20


def test_unsafe_audit_runs_for_pull_requests() -> None:
    job = _workflow()["jobs"]["unsafe-audit"]

    assert "github.event_name == 'pull_request'" in str(job["if"])


def test_unsafe_audit_report_is_recomputed_for_every_run() -> None:
    job = _workflow()["jobs"]["unsafe-audit"]
    audit_step = next(
        step
        for step in job["steps"]
        if step.get("name") == "Audit unsafe code in dependencies"
    )

    assert "if" not in audit_step
    assert all(step.get("id") != "geiger-cache" for step in job["steps"])
    assert all(
        step.get("name") != "Display cached geiger report" for step in job["steps"]
    )


def test_unsafe_audit_report_verification_is_ansi_free_and_non_vacuous() -> None:
    job = _workflow()["jobs"]["unsafe-audit"]
    audit_step = next(
        step for step in job["steps"] if step.get("name") == "Audit unsafe code in dependencies"
    )
    verify_step = next(
        step for step in job["steps"] if step.get("name") == "Verify no unsafe in library code"
    )
    verify_run = verify_step["run"]
    audit_run = audit_step["run"]

    assert audit_step["env"]["CARGO_TERM_COLOR"] == "never"
    assert 'pipeline_status=("${PIPESTATUS[@]}")' in audit_run
    assert 'tee_status="${pipeline_status[1]}"' in audit_run
    assert "|| true" not in audit_run
    assert "Found [1-9][0-9]* warnings" in audit_run
    assert "touch geiger-report.complete" in audit_run
    assert "test -f geiger-report.complete" in verify_run
    assert 'test -n "$root_row"' in verify_run
    assert 'unsafe_total="$(' in verify_run
    assert 'if [ "$unsafe_total" -ne 0 ]' in verify_run
    assert '$7 == "fortress-rollback"' in verify_run
    assert '$6 == ":)"' in verify_run
    assert "grep -v \"0/0\"" not in verify_run


def test_unsafe_audit_accepts_geiger_warning_exit_after_complete_report(
    tmp_path: Path,
) -> None:
    result = _run_unsafe_audit_producer(
        tmp_path,
        "0/0 0/0 0/0 0/0 0/0 :) fortress-rollback 0.13.0\n"
        "error: Found 2 warnings",
        1,
    )

    assert result.returncode == 0, result.stderr
    assert (tmp_path / "geiger-report.complete").is_file()


def test_unsafe_audit_rejects_failed_partial_geiger_report(tmp_path: Path) -> None:
    result = _run_unsafe_audit_producer(
        tmp_path,
        "0/0 0/0 0/0 0/0 0/0 fortress-rollback\nerror: could not compile dependency",
        1,
    )

    assert result.returncode != 0
    assert "producer failed" in result.stdout
    assert not (tmp_path / "geiger-report.complete").exists()


def test_unsafe_audit_rejects_warning_summary_followed_by_fatal_error(
    tmp_path: Path,
) -> None:
    result = _run_unsafe_audit_producer(
        tmp_path,
        "0/0 0/0 0/0 0/0 0/0 :) fortress-rollback 0.13.0\n"
        "error: Found 2 warnings\n"
        "error: could not compile dependency",
        1,
    )

    assert result.returncode != 0
    assert "incomplete or fatal output" in result.stdout
    assert not (tmp_path / "geiger-report.complete").exists()


def test_unsafe_audit_rejects_report_writer_failure(tmp_path: Path) -> None:
    result = _run_unsafe_audit_producer(
        tmp_path,
        "0/0 0/0 0/0 0/0 0/0 :) fortress-rollback 0.13.0",
        0,
        tee_status=74,
    )

    assert result.returncode == 74
    assert "report writer failed with status 74" in result.stdout
    assert not (tmp_path / "geiger-report.complete").exists()


def test_unsafe_audit_verifier_accepts_real_geiger_safety_column(
    tmp_path: Path,
) -> None:
    result = _run_unsafe_audit_verifier(
        tmp_path,
        "0/0 0/0 0/1 0/0 0/0 :) fortress-rollback 0.13.0\n",
    )

    assert result.returncode == 0, result.stderr
    assert "No unsafe code in fortress-rollback library" in result.stdout
