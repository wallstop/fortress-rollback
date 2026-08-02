"""Tests for the time-bounded cargo-deny advisory disposition policy."""

from __future__ import annotations

import datetime as dt
import importlib.util
import re
import subprocess
import sys
from pathlib import Path

import pytest

SCRIPTS = Path(__file__).parent.parent
REPO_ROOT = SCRIPTS.parent
SPEC = importlib.util.spec_from_file_location(
    "check_advisory_dispositions",
    SCRIPTS / "ci" / "check-advisory-dispositions.py",
)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


def _write_repository(root: Path) -> None:
    (root / "supply-chain").mkdir()
    (root / "src").mkdir()
    (root / "src" / "lib.rs").write_text("", encoding="utf-8")
    (root / "deny.toml").write_text(
        '[advisories]\nignore = ["RUSTSEC-2025-0141"]\n', encoding="utf-8"
    )
    (root / "Cargo.toml").write_text(
        '[package]\nname = "policy-fixture"\nversion = "0.1.0"\nedition = "2021"\n'
        '[dependencies]\nbincode = { version = "=2.0.1" }\n',
        encoding="utf-8",
    )
    (root / "Cargo.lock").write_text(
        '[[package]]\nname = "bincode"\nversion = "2.0.1"\n'
        'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
        'checksum = "36eaf5d7b090263e8150820482d5d93cd964a81e4019913c972f4edcc6edb740"\n',
        encoding="utf-8",
    )
    (root / "supply-chain" / "advisory-dispositions.toml").write_text(
        """format = 1
[[dispositions]]
advisory = "RUSTSEC-2025-0141"
package = "bincode"
version = "2.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "36eaf5d7b090263e8150820482d5d93cd964a81e4019913c972f4edcc6edb740"
owner = "maintainers"
issue = "https://github.com/example/project/issues/1"
review_by = 2026-11-02
reason = "Format migration is tracked."
exit_criteria = ["Complete the migration."]
""",
        encoding="utf-8",
    )
    subprocess.run(["git", "-C", str(root), "init", "-q"], check=True)
    subprocess.run(
        ["git", "-C", str(root), "config", "user.email", "policy@example.invalid"],
        check=True,
    )
    subprocess.run(
        ["git", "-C", str(root), "config", "user.name", "Policy Test"],
        check=True,
    )
    subprocess.run(["git", "-C", str(root), "add", "."], check=True)
    subprocess.run(
        ["git", "-C", str(root), "commit", "-qm", "policy fixture"], check=True
    )


@pytest.fixture()
def repository(tmp_path: Path) -> Path:
    _write_repository(tmp_path)
    return tmp_path


def test_complete_current_disposition_passes(repository: Path) -> None:
    assert CHECKER.check_repository(repository, today=dt.date(2026, 11, 2)) == []


def test_expired_disposition_fails(repository: Path) -> None:
    assert CHECKER.check_repository(repository, today=dt.date(2026, 11, 3)) == [
        "supply-chain/advisory-dispositions.toml:0: RUSTSEC-2025-0141 review expired on 2026-11-02"
    ]


@pytest.mark.parametrize(
    ("path", "old", "new", "expected"),
    [
        (
            "supply-chain/advisory-dispositions.toml",
            '[[dispositions]]\nadvisory = "RUSTSEC-2025-0141"',
            '[[dispositions]]\nadvisory = "RUSTSEC-2025-9999"',
            "RUSTSEC-2025-0141 requires exactly one disposition, found 0",
        ),
        (
            "Cargo.toml",
            'version = "=2.0.1"',
            'version = "2.0"',
            "bincode must be pinned to =2.0.1",
        ),
        (
            "Cargo.lock",
            'version = "2.0.1"',
            'version = "2.0.0"',
            "bincode must resolve version 2.0.1",
        ),
    ],
)
def test_disposition_drift_fails(
    repository: Path, path: str, old: str, new: str, expected: str
) -> None:
    target = repository / path
    target.write_text(target.read_text(encoding="utf-8").replace(old, new), encoding="utf-8")

    errors = CHECKER.check_repository(repository, today=dt.date(2026, 8, 2))

    assert any(expected in error for error in errors)


@pytest.mark.parametrize(
    ("path", "old", "new", "expected"),
    [
        (
            "Cargo.toml",
            'version = "=2.0.1"',
            'version = "=2.0.1", git = "https://example.invalid/bincode"',
            "must not select an alternate source (git)",
        ),
        (
            "Cargo.lock",
            'source = "registry+https://github.com/rust-lang/crates.io-index"',
            'source = "git+https://example.invalid/bincode"',
            "source does not match",
        ),
        (
            "Cargo.lock",
            'checksum = "36eaf5d7b090263e8150820482d5d93cd964a81e4019913c972f4edcc6edb740"',
            'checksum = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"',
            "checksum does not match",
        ),
        (
            "supply-chain/advisory-dispositions.toml",
            "format = 1",
            "format = true",
            "format must be the integer 1",
        ),
    ],
)
def test_source_and_schema_mutations_fail(
    repository: Path, path: str, old: str, new: str, expected: str
) -> None:
    target = repository / path
    target.write_text(target.read_text(encoding="utf-8").replace(old, new), encoding="utf-8")

    errors = CHECKER.check_repository(repository, today=dt.date(2026, 8, 2))

    assert any(expected in error for error in errors)


def test_duplicate_locked_package_fails(repository: Path) -> None:
    lock = repository / "Cargo.lock"
    lock.write_text(
        lock.read_text(encoding="utf-8")
        + '[[package]]\nname = "bincode"\nversion = "1.3.3"\n'
        'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
        'checksum = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"\n',
        encoding="utf-8",
    )

    errors = CHECKER.check_repository(repository, today=dt.date(2026, 8, 2))

    assert any("must resolve exactly once" in error for error in errors)


def test_standalone_workspace_bincode_drift_fails(repository: Path) -> None:
    standalone = repository / "standalone"
    standalone.mkdir()
    (standalone / "src").mkdir()
    (standalone / "src" / "lib.rs").write_text("", encoding="utf-8")
    (standalone / "Cargo.toml").write_text(
        '[package]\nname = "standalone"\nversion = "0.1.0"\nedition = "2021"\n'
        '[dependencies]\nbincode = "1.3.3"\n',
        encoding="utf-8",
    )
    (standalone / "Cargo.lock").write_text(
        '[[package]]\nname = "bincode"\nversion = "1.3.3"\n'
        'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
        'checksum = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"\n',
        encoding="utf-8",
    )
    subprocess.run(
        [
            "git",
            "-C",
            str(repository),
            "add",
            "standalone/Cargo.toml",
            "standalone/Cargo.lock",
            "standalone/src/lib.rs",
        ],
        check=True,
    )

    errors = CHECKER.check_repository(repository, today=dt.date(2026, 8, 2))

    assert any(error.startswith("standalone/Cargo.toml:0:") for error in errors)
    assert any(error.startswith("standalone/Cargo.lock:0:") for error in errors)


def test_relevant_standalone_workspace_cannot_drop_bincode(repository: Path) -> None:
    standalone = repository / "standalone"
    standalone.mkdir()
    (standalone / "src").mkdir()
    (standalone / "src" / "lib.rs").write_text("", encoding="utf-8")
    (standalone / "Cargo.toml").write_text(
        '[package]\nname = "standalone"\nversion = "0.1.0"\nedition = "2021"\n'
        '[dependencies]\npolicy-fixture = { path = ".." }\n',
        encoding="utf-8",
    )
    (standalone / "Cargo.lock").write_text(
        '[[package]]\nname = "policy-fixture"\nversion = "0.1.0"\n'
        '[[package]]\nname = "standalone"\nversion = "0.1.0"\n',
        encoding="utf-8",
    )
    subprocess.run(
        ["git", "-C", str(repository), "add", "standalone"], check=True
    )

    errors = CHECKER.check_repository(repository, today=dt.date(2026, 8, 2))

    assert any(
        error.startswith("standalone/Cargo.lock:0:")
        and "found 0 packages" in error
        for error in errors
    )


@pytest.mark.parametrize(
    "malformed",
    [
        'package = "not-an-array"\n',
        "version = 4\n",
        'package = ["not-a-table", { name = "bincode", version = "2.0.1" }]\n',
    ],
)
def test_malformed_lock_package_schema_fails(
    repository: Path, malformed: str
) -> None:
    (repository / "Cargo.lock").write_text(malformed, encoding="utf-8")

    errors = CHECKER.check_repository(repository, today=dt.date(2026, 8, 2))

    assert errors == ["Cargo.lock:0: package must be an array of tables"]


def test_all_policy_errors_use_editor_diagnostic_format(repository: Path) -> None:
    disposition = repository / "supply-chain" / "advisory-dispositions.toml"
    disposition.write_text(
        disposition.read_text(encoding="utf-8").replace(
            "review_by = 2026-11-02", "review_by = 2026-01-01"
        ),
        encoding="utf-8",
    )

    errors = CHECKER.check_repository(repository, today=dt.date(2026, 8, 2))

    assert errors
    assert all(re.fullmatch(r"[^:\n]+:\d+: .+", error) for error in errors)


def test_invalid_toml_error_uses_editor_diagnostic_format(repository: Path) -> None:
    (repository / "deny.toml").write_text(
        "[advisories]\nignore = @", encoding="utf-8"
    )

    with pytest.raises(RuntimeError, match=r"^deny\.toml:2: invalid TOML:"):
        CHECKER.check_repository(repository, today=dt.date(2026, 8, 2))


def test_security_workflow_runs_disposition_deadline_check_daily() -> None:
    workflow = (REPO_ROOT / ".github" / "workflows" / "ci-security.yml").read_text(
        encoding="utf-8"
    )

    assert "- cron: '0 0 * * *'" in workflow
    assert "run: python3 scripts/ci/check-advisory-dispositions.py" in workflow


def test_precommit_disposition_hook_matches_nested_cargo_files() -> None:
    precommit = (REPO_ROOT / ".pre-commit-config.yaml").read_text(encoding="utf-8")
    hook = re.search(
        r"- id: advisory-dispositions\n(?P<body>.*?)(?=\n\s+- id:)",
        precommit,
        flags=re.DOTALL,
    )
    assert hook is not None
    pattern_match = re.search(r"files: '([^']+)'", hook.group("body"))
    assert pattern_match is not None
    pattern = re.compile(pattern_match.group(1))

    assert pattern.search("Cargo.toml")
    assert pattern.search("fuzz/Cargo.lock")
    assert pattern.search("tests/godot-emscripten/Cargo.toml")
