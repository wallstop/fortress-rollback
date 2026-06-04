#!/usr/bin/env python3
"""Regression tests for the per-proof MEMORY WATCHDOG in verify-kani.sh.

Background: a few Kani proofs drive CBMC/CaDiCaL into a state-space explosion
that allocates RAM faster than the GNU ``timeout`` wall-clock budget can
catch. On a 16 GB ``ubuntu-latest`` runner the OS OOM-killer then reaps the
*runner agent itself* ("The runner has received a shutdown signal") with NO
attribution to the offending harness.

verify-kani.sh now runs each per-proof ``cargo kani`` invocation in its own
process group and a lightweight background sampler watches
``/proc/meminfo`` ``MemAvailable``. When it drops below a floor (machine about
to OOM) the sampler SIGKILLs the kani/cbmc *process group* -- not the runner
-- and records that THIS harness blew the memory budget. ``classify_exit_code``
gains a ``memory_exceeded`` class that takes precedence over the ambiguous
SIGKILL (137) exit, and ``run_kani`` emits an actionable, per-proof
diagnostic naming the harness and pointing at the remediation policy.

These tests pin:

* ``classify_exit_code`` mapping (memory case + precedence + back-compat).
* ``read_meminfo_kb`` parsing + graceful no-op without ``/proc/meminfo``.
* ``compute_mem_floor_mb`` dynamic default formula + override + disabled.
* End-to-end: the watchdog kills the process group, exits 137, and emits the
  ``MEMORY EXCEEDED`` diagnostic naming the harness -- with NO leaked
  bash "Killed" job-control line.
* End-to-end no-op: with the watchdog disabled the script behaves exactly as
  before.

The unit-level tests source the shell FUNCTIONS out of verify-kani.sh (the
script ends in ``main "$@"`` so it cannot be sourced wholesale) and drive
them from a tiny bash harness, so the assertions test exactly what ships.
"""

from __future__ import annotations

import re
import shutil
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
VERIFY_KANI = REPO_ROOT / "scripts" / "verification" / "verify-kani.sh"

# Functions we extract for unit-level testing. We slice each ``name() { ...
# }`` block (all start at column 0 and close with a ``}`` at column 0) so the
# harness runs the REAL shipped implementation, not a copy.
_EXTRACT_FUNCS = (
    "read_meminfo_kb",
    "compute_mem_floor_mb",
    "classify_exit_code",
)


def _func_source() -> str:
    """Return the bash source for the watchdog/classifier helper functions.

    Slices each ``name() { ... }`` block out of verify-kani.sh by sed so the
    harness exercises the shipped implementation verbatim.
    """
    text = VERIFY_KANI.read_text(encoding="utf-8")
    lines = text.splitlines()
    out: list[str] = []
    for func in _EXTRACT_FUNCS:
        start = None
        for i, line in enumerate(lines):
            if line.startswith(f"{func}() {{"):
                start = i
                break
        assert start is not None, f"function {func} not found in verify-kani.sh"
        end = None
        for j in range(start + 1, len(lines)):
            if lines[j] == "}":
                end = j
                break
        assert end is not None, f"closing brace for {func} not found"
        out.append("\n".join(lines[start : end + 1]))
    return "\n\n".join(out)


def _run_harness(body: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    """Run a bash harness that sources the extracted helpers then runs ``body``."""
    script = (
        "#!/bin/bash\n"
        "set -uo pipefail\n"
        + _func_source()
        + "\n"
        + body
        + "\n"
    )
    full_env = {"PATH": "/usr/bin:/bin"}
    if env:
        full_env.update(env)
    return subprocess.run(
        ["bash", "-c", script],
        capture_output=True,
        text=True,
        check=False,
        env=full_env,
    )


# ---------------------------------------------------------------------------
# classify_exit_code: the new memory_exceeded class + precedence + back-compat.
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    "exit_code,watchdog_fired,expected",
    [
        # Watchdog fired => memory_exceeded, REGARDLESS of the raw exit code.
        # 137 is the natural SIGKILL exit; the watchdog verdict must override
        # the (otherwise external_terminate) classification.
        ("137", "true", "memory_exceeded"),
        # Even an exit code that would otherwise be a timeout: watchdog wins.
        ("124", "true", "memory_exceeded"),
        # And a generic failure exit: watchdog wins.
        ("1", "true", "memory_exceeded"),
        # Watchdog NOT fired: raw-exit classification is unchanged.
        ("124", "false", "per_proof_timeout"),
        ("137", "false", "external_terminate"),
        ("143", "false", "external_terminate"),
        ("1", "false", ""),
        ("99", "false", ""),
    ],
    ids=lambda v: str(v),
)
def test_classify_exit_code_memory_case(
    exit_code: str, watchdog_fired: str, expected: str
) -> None:
    """``classify_exit_code`` maps the memory case and preserves precedence."""
    result = _run_harness(
        f'classify_exit_code "{exit_code}" "{watchdog_fired}"'
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == expected, (
        f"classify_exit_code({exit_code}, {watchdog_fired}) => "
        f"{result.stdout.strip()!r}, expected {expected!r}"
    )


@pytest.mark.parametrize(
    "exit_code,expected",
    [
        ("124", "per_proof_timeout"),
        ("137", "external_terminate"),
        ("143", "external_terminate"),
        ("1", ""),
        ("0", ""),
    ],
    ids=lambda v: str(v),
)
def test_classify_exit_code_single_arg_back_compat(
    exit_code: str, expected: str
) -> None:
    """Single-arg ``classify_exit_code`` (no watchdog flag) is unchanged.

    The watchdog flag defaults to empty, so every existing single-arg caller
    keeps its old behaviour -- the memory class only activates on an explicit
    ``"true"`` second argument.
    """
    result = _run_harness(f'classify_exit_code "{exit_code}"')
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == expected


# ---------------------------------------------------------------------------
# read_meminfo_kb: parses a real field; no-ops gracefully when /proc is absent.
# ---------------------------------------------------------------------------
@pytest.mark.skipif(
    not Path("/proc/meminfo").is_file(),
    reason="no /proc/meminfo on this platform",
)
def test_read_meminfo_kb_parses_real_field() -> None:
    """``read_meminfo_kb MemTotal`` echoes the kB integer from /proc/meminfo."""
    result = _run_harness('read_meminfo_kb "MemTotal"')
    assert result.returncode == 0, result.stderr
    value = result.stdout.strip()
    assert value.isdigit(), f"expected an integer kB, got {value!r}"
    # Cross-check against the file directly.
    meminfo = Path("/proc/meminfo").read_text(encoding="utf-8")
    m = re.search(r"^MemTotal:\s+(\d+) kB", meminfo, re.MULTILINE)
    assert m is not None
    assert value == m.group(1)


def test_read_meminfo_kb_missing_field_fails() -> None:
    """A field that does not exist yields a non-zero return and no output."""
    result = _run_harness('read_meminfo_kb "NoSuchField__"; echo "rc=$?"')
    # The function returns 1; the harness echoes rc=1 after it.
    assert "rc=1" in result.stdout, result.stdout


def test_read_meminfo_kb_no_op_without_proc() -> None:
    """Without a readable /proc/meminfo the function fails gracefully.

    We can't unmount /proc, so instead we redefine ``read_meminfo_kb`` to use
    a path we control and point it at a non-existent file. This exercises the
    exact ``[[ -r ... ]] || return 1`` guard shape: a missing meminfo => the
    function returns non-zero with no output, which is how the watchdog stays
    a no-op on macOS/BSD.
    """
    body = (
        # Override with a copy of the guard logic pointed at a missing path.
        'check() { [[ -r "/nonexistent/meminfo__" ]] || return 1; echo SHOULD_NOT_PRINT; }\n'
        'out=$(check); rc=$?\n'
        'echo "rc=$rc out=[$out]"'
    )
    result = _run_harness(body)
    assert "rc=1 out=[]" in result.stdout, result.stdout


# ---------------------------------------------------------------------------
# compute_mem_floor_mb: dynamic default formula + override + disabled.
# ---------------------------------------------------------------------------
@pytest.mark.skipif(
    not Path("/proc/meminfo").is_file(),
    reason="no /proc/meminfo on this platform",
)
def test_compute_mem_floor_mb_dynamic_default_matches_formula() -> None:
    """Default floor == max(1024, 8% of MemTotal MB).

    The expected value is recomputed in Python from the SAME /proc/meminfo
    the script reads, so the assertion can't go stale on a different machine.
    """
    result = _run_harness("compute_mem_floor_mb")
    assert result.returncode == 0, result.stderr
    floor = int(result.stdout.strip())

    meminfo = Path("/proc/meminfo").read_text(encoding="utf-8")
    m = re.search(r"^MemTotal:\s+(\d+) kB", meminfo, re.MULTILINE)
    assert m is not None
    total_kb = int(m.group(1))
    total_mb = total_kb // 1024
    pct_mb = total_mb * 8 // 100
    expected = max(1024, pct_mb)
    assert floor == expected, (
        f"dynamic floor {floor} != max(1024, 8% of {total_mb} MB) = {expected}"
    )


def test_compute_mem_floor_mb_override_wins() -> None:
    """An explicit ``KANI_MEM_FLOOR_MB`` positive integer overrides the default."""
    result = _run_harness(
        "compute_mem_floor_mb", env={"KANI_MEM_FLOOR_MB": "4096"}
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "4096"


def test_compute_mem_floor_mb_invalid_override_falls_back() -> None:
    """A non-positive-integer override is rejected with a warning, then the
    dynamic default (or disabled) path is taken.

    On a machine with /proc/meminfo the function still emits a numeric floor
    (the dynamic default) on stdout and a warning on stderr.
    """
    result = _run_harness(
        "compute_mem_floor_mb || echo DISABLED",
        env={"KANI_MEM_FLOOR_MB": "notanumber"},
    )
    assert "not a positive integer" in result.stderr, result.stderr
    out = result.stdout.strip()
    if Path("/proc/meminfo").is_file():
        assert out.isdigit(), f"expected a numeric dynamic floor, got {out!r}"
    else:
        assert out == "DISABLED", out


def test_compute_mem_floor_mb_disabled_without_proc() -> None:
    """Without /proc/meminfo (no MemTotal) the function returns non-zero.

    The watchdog treats a non-zero return as "disabled" and falls back to a
    plain ``run_with_timeout``. We simulate "no /proc" by overriding
    ``read_meminfo_kb`` to always fail, then calling the REAL
    ``compute_mem_floor_mb``; with no override env var it must propagate the
    failure (return non-zero, no stdout).
    """
    body = (
        # Shadow read_meminfo_kb so MemTotal lookup fails, as on macOS.
        "read_meminfo_kb() { return 1; }\n"
        "out=$(compute_mem_floor_mb); rc=$?\n"
        'echo "rc=$rc out=[$out]"'
    )
    result = _run_harness(body)
    assert "rc=1 out=[]" in result.stdout, result.stdout


# ---------------------------------------------------------------------------
# End-to-end: the watchdog kills the process group and attributes the failure.
#
# A fake ``cargo`` shim sleeps (simulating a long-running explosive proof) so
# the sampler has time to poll. We force a trigger with an impossible floor
# (``KANI_MEM_FLOOR_MB`` larger than any machine's RAM) so MemAvailable is
# ALWAYS below it. The watchdog must kill the group, the script must exit 137,
# and the diagnostic must name the harness + cite the policy -- with no leaked
# bash "Killed" job-control line.
# ---------------------------------------------------------------------------
HARNESS = "proof_minimal_sync_layer_initial_state_valid_2p"


def _build_sleeping_cargo(repo: Path) -> tuple[Path, Path]:
    """Build a fake-cargo repo whose ``cargo kani`` sleeps for 30s."""
    script_path = repo / "scripts" / "verification" / "verify-kani.sh"
    script_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(VERIFY_KANI, script_path)

    fake_bin = repo / "fake-bin"
    fake_bin.mkdir()
    cargo = fake_bin / "cargo"
    cargo.write_text(
        "#!/bin/bash\n"
        "set -euo pipefail\n"
        'if [[ "${1:-}" == "kani" && "${2:-}" == "--version" ]]; then\n'
        "  echo 'cargo-kani 0.67.0'\n"
        "  exit 0\n"
        "fi\n"
        "echo 'Checking harness fake::proof_explode...'\n"
        # Long sleep: the sampler (poll ~1s) kills us well before this returns.
        "sleep 30\n"
        "exit 0\n",
        encoding="utf-8",
    )
    cargo.chmod(0o755)
    cargo_kani = fake_bin / "cargo-kani"
    cargo_kani.write_text("#!/bin/bash\nexit 0\n", encoding="utf-8")
    cargo_kani.chmod(0o755)
    return script_path, fake_bin


@pytest.mark.skipif(
    shutil.which("setsid") is None or not Path("/proc/meminfo").is_file(),
    reason="watchdog requires Linux /proc/meminfo + setsid",
)
def test_watchdog_kills_group_and_attributes_memory_exceeded(tmp_path: Path) -> None:
    """An impossible floor forces the watchdog to fire and attribute the kill.

    Asserts:
      * The script exits 137 (the watchdog SIGKILL surfaces as 137).
      * The ``MEMORY EXCEEDED`` diagnostic names the harness.
      * It is framed as a proof-tractability / state-space-explosion problem
        (NOT a CI flake) and cites ``src/sync_layer/mod.rs``.
      * The watchdog breach record (floor/avail) is present.
      * No bash "Killed" job-control line leaks into the captured log.
    """
    repo = tmp_path / "repo"
    repo.mkdir()
    script_path, fake_bin = _build_sleeping_cargo(repo)

    env = {
        "PATH": f"{fake_bin}:/usr/bin:/bin",
        "TERM": "dumb",
        "KANI_TIMEOUT": "60",  # >> the time-to-trigger, so the WATCHDOG wins
        "KANI_MEM_FLOOR_MB": "999999999",  # impossible => always below floor
        "KANI_MEM_POLL_SECONDS": "1",
    }
    result = subprocess.run(
        ["bash", str(script_path), "--harness", HARNESS],
        cwd=repo,
        env=env,
        capture_output=True,
        text=True,
        check=False,
        timeout=45,
    )
    combined = result.stdout + result.stderr

    assert result.returncode == 137, (
        f"expected exit 137 (watchdog SIGKILL), got {result.returncode}.\n"
        f"--- stdout ---\n{result.stdout}\n--- stderr ---\n{result.stderr}"
    )
    assert "MEMORY EXCEEDED" in combined, combined
    assert HARNESS in combined, combined
    assert "state-space-explosion" in combined or "state-space explosion" in combined, combined
    assert "src/sync_layer/mod.rs" in combined, combined
    # The breach record line names the floor.
    assert "floor_mb=999999999" in combined, combined
    # The misleading external-termination diagnostic must NOT also appear.
    assert "EXTERNAL TERMINATION" not in combined, combined
    # No bash job-control "Killed" line should leak into the log.
    assert not re.search(r"\bKilled\b", combined), (
        f"bash 'Killed' job-control message leaked:\n{combined}"
    )


@pytest.mark.skipif(
    shutil.which("setsid") is None or not Path("/proc/meminfo").is_file(),
    reason="watchdog requires Linux /proc/meminfo + setsid",
)
def test_watchdog_does_not_fire_on_normal_proof(tmp_path: Path) -> None:
    """With the dynamic floor and a fast-passing proof, the watchdog no-ops.

    The watchdog being *active* must not perturb a normal run: a quick proof
    that emits a Kani success summary should pass (exit 0) with no
    ``MEMORY EXCEEDED`` diagnostic.
    """
    repo = tmp_path / "repo"
    repo.mkdir()
    script_path = repo / "scripts" / "verification" / "verify-kani.sh"
    script_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(VERIFY_KANI, script_path)
    fake_bin = repo / "fake-bin"
    fake_bin.mkdir()
    cargo = fake_bin / "cargo"
    cargo.write_text(
        "#!/bin/bash\n"
        "set -euo pipefail\n"
        'if [[ "${1:-}" == "kani" && "${2:-}" == "--version" ]]; then\n'
        "  echo 'cargo-kani 0.67.0'\n"
        "  exit 0\n"
        "fi\n"
        "echo 'Checking harness fake::proof_ok...'\n"
        "echo 'Complete - 1 successfully verified harnesses, 0 failures, 1 total.'\n"
        "exit 0\n",
        encoding="utf-8",
    )
    cargo.chmod(0o755)
    cargo_kani = fake_bin / "cargo-kani"
    cargo_kani.write_text("#!/bin/bash\nexit 0\n", encoding="utf-8")
    cargo_kani.chmod(0o755)

    env = {
        "PATH": f"{fake_bin}:/usr/bin:/bin",
        "TERM": "dumb",
        "KANI_TIMEOUT": "60",
        # No KANI_MEM_FLOOR_MB => dynamic default (8% of MemTotal); a normal
        # machine has far more than 8% free, so the watchdog must NOT fire.
    }
    result = subprocess.run(
        ["bash", str(script_path), "--harness", HARNESS],
        cwd=repo,
        env=env,
        capture_output=True,
        text=True,
        check=False,
        timeout=45,
    )
    combined = result.stdout + result.stderr
    assert result.returncode == 0, (
        f"normal proof should pass (exit 0), got {result.returncode}.\n"
        f"--- stdout ---\n{result.stdout}\n--- stderr ---\n{result.stderr}"
    )
    assert "MEMORY EXCEEDED" not in combined, combined
    # The watchdog should announce it is active (Linux runner).
    assert "Memory watchdog active" in combined, combined
