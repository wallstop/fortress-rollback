#!/usr/bin/env python3
"""Regression tests for scripts/verification/verify-kani.sh diagnostics.

The verify-kani.sh script classifies non-success exit codes from cargo kani
into two timeout-class buckets:

* ``per_proof_timeout`` -- GNU ``timeout(1)`` fired its own ``KANI_TIMEOUT``
  (exit ``124``).
* ``external_terminate`` -- the runner sent ``SIGTERM``/``SIGKILL`` (exit
  ``143``/``137``); GNU timeout did NOT fire.

The script also propagates these signal-class exits verbatim so the GitHub
workflow's ``nick-fields/retry@v4`` ``retry_on_exit_code`` can match runner
preemption (e.g. ``143``). Real proof failures collapse to exit ``1``.

Each case below pins one ``(fake-cargo exit code) -> (expected verify-kani
exit code, expected diagnostic substrings)`` mapping. A single parametrized
test exercises all of them.
"""

from __future__ import annotations

from dataclasses import dataclass, field

import pytest

# The harness name is used in the fake-cargo invocation and is asserted in
# the diagnostic command echo. Centralising it keeps the test cases concise.
HARNESS = "proof_minimal_sync_layer_initial_state_valid"
KANI_TIMEOUT = "42"

# Verify-kani.sh appends `--default-unwind 8` when invoked with `--quick`.
COMMAND_ECHO = (
    f"Command: cargo kani --harness {HARNESS} --default-unwind 8"
)


@dataclass(frozen=True)
class KaniExitCase:
    """One fake-cargo-exit -> verify-kani diagnostic mapping.

    Attributes:
        exit_code: Exit code returned by the fake ``cargo`` shim.
        expected_returncode: Exit code verify-kani.sh must propagate. Signal-
            class exits (124/137/143) are passed through verbatim so the
            workflow retry logic can match them; everything else collapses
            to ``1``.
        expected_diag_substrings: Substrings that MUST all appear in the
            combined stdout+stderr.
        forbidden_substrings: Substrings that MUST NOT appear (used to
            assert that mutually-exclusive diagnostics never coexist).
    """

    exit_code: int
    expected_returncode: int
    expected_diag_substrings: tuple[str, ...]
    forbidden_substrings: tuple[str, ...] = field(default_factory=tuple)


CASES: tuple[KaniExitCase, ...] = (
    # GNU timeout(1) fired -> per-proof timeout diagnostic.
    #
    # We assert on a small, semantically-meaningful set of tokens rather
    # than a single long sentence: the bucket label, the harness name,
    # the configured KANI_TIMEOUT echo, and the command-echo prefix. If
    # someone re-words the prose (e.g. "exceeded" -> "ran past"), the
    # test still catches a missing harness name or a wrong bucket label
    # without firing on every copy-edit.
    KaniExitCase(
        exit_code=124,
        expected_returncode=124,
        expected_diag_substrings=(
            "PER-PROOF TIMEOUT",
            f"'{HARNESS}'",
            f"KANI_TIMEOUT={KANI_TIMEOUT}s",
            COMMAND_ECHO,
            "last diagnostic line from fake Kani",
        ),
        forbidden_substrings=("EXTERNAL TERMINATION",),
    ),
    # SIGKILL (137) -- runner OOM/preemption, NOT a per-proof timeout.
    KaniExitCase(
        exit_code=137,
        expected_returncode=137,
        expected_diag_substrings=(
            "EXTERNAL TERMINATION",
            f"'{HARNESS}'",
            "exit 137",
            "GNU timeout did NOT fire",
            COMMAND_ECHO,
            "last diagnostic line from fake Kani",
        ),
        forbidden_substrings=("PER-PROOF TIMEOUT",),
    ),
    # SIGTERM (143) -- workflow cancellation / job timeout / spot reclaim.
    KaniExitCase(
        exit_code=143,
        expected_returncode=143,
        expected_diag_substrings=(
            "EXTERNAL TERMINATION",
            f"'{HARNESS}'",
            "exit 143",
            "GNU timeout did NOT fire",
            COMMAND_ECHO,
            "last diagnostic line from fake Kani",
        ),
        forbidden_substrings=("PER-PROOF TIMEOUT",),
    ),
    # Generic non-signal failure -- collapses to exit 1, no timeout-class
    # diagnostic, but the "Kani exited with error code N" line is preserved.
    KaniExitCase(
        exit_code=2,
        expected_returncode=1,
        expected_diag_substrings=(
            "Kani exited with error code 2",
            "last diagnostic line from fake Kani",
        ),
        forbidden_substrings=(
            "PER-PROOF TIMEOUT",
            "EXTERNAL TERMINATION",
        ),
    ),
    # Unknown non-signal exit code -- only 124/137/143 are propagated
    # verbatim; everything else (here 99, an arbitrary non-special value)
    # MUST normalise to exit 1. This pins the "signal-class allowlist" so
    # adding a new bucket without thinking about the workflow's
    # retry_on_exit_code list is caught immediately.
    KaniExitCase(
        exit_code=99,
        expected_returncode=1,
        expected_diag_substrings=(
            "Kani exited with error code 99",
            "last diagnostic line from fake Kani",
        ),
        forbidden_substrings=(
            "PER-PROOF TIMEOUT",
            "EXTERNAL TERMINATION",
        ),
    ),
)


@pytest.mark.parametrize(
    "case",
    CASES,
    ids=lambda c: f"exit{c.exit_code}",
)
def test_kani_exit_classification(
    case: KaniExitCase,
    fake_cargo_repo_factory,
    verify_kani_runner,
) -> None:
    """verify-kani.sh classifies cargo kani exit codes into the right bucket.

    For every fake-cargo exit code, the script must:

    1. Propagate the documented exit code (signal-class exits 124/137/143
       verbatim, all other failures collapsed to ``1``).
    2. Emit every substring in ``expected_diag_substrings``.
    3. Emit none of the substrings in ``forbidden_substrings`` (mutually-
       exclusive diagnostics).
    4. Preserve the tail-output diagnostic line from cargo kani.

    Substrings are deliberately short and semantic (e.g. ``"PER-PROOF
    TIMEOUT"``, the harness name, ``KANI_TIMEOUT={n}s``) so prose
    copy-edits don't break the test, while a missing bucket label or
    wrong harness name still fails immediately.
    """
    repo, script_path, fake_bin = fake_cargo_repo_factory()

    result = verify_kani_runner(
        repo,
        script_path,
        fake_bin,
        args=["--quick", "--harness", HARNESS],
        fake_exit=case.exit_code,
        kani_timeout=KANI_TIMEOUT,
    )
    combined = result.stdout + result.stderr

    assert result.returncode == case.expected_returncode, (
        f"verify-kani.sh returned {result.returncode}, expected "
        f"{case.expected_returncode} for fake exit {case.exit_code}.\n"
        f"--- stdout ---\n{result.stdout}\n--- stderr ---\n{result.stderr}"
    )

    missing = [s for s in case.expected_diag_substrings if s not in combined]
    assert not missing, (
        f"Missing expected diagnostic substrings for exit "
        f"{case.exit_code}: {missing!r}\n"
        f"--- stdout ---\n{result.stdout}\n--- stderr ---\n{result.stderr}"
    )

    leaked = [s for s in case.forbidden_substrings if s in combined]
    assert not leaked, (
        f"Forbidden diagnostic substrings leaked for exit "
        f"{case.exit_code}: {leaked!r}\n"
        f"--- stdout ---\n{result.stdout}\n--- stderr ---\n{result.stderr}"
    )
