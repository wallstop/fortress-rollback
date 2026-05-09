#!/usr/bin/env python3
"""Shared pytest fixtures for verify-kani.sh tests.

These fixtures build the temp repo + fake-cargo shim used by both
``test_verify_kani_script.py`` (exit-code classification) and
``test_verify_kani_partition.py`` (partition-layout introspection /
end-to-end run path).

Two fixtures are exposed:

* :func:`fake_cargo_repo_factory` -- a factory function ``(tmp_path,
  *, success_summary=None) -> (repo, script_path, fake_bin)`` so tests
  can customise the fake cargo's "Complete - ..." summary line for the
  end-to-end run-path tests while sharing one builder.
* :func:`verify_kani_runner` -- a callable that invokes verify-kani.sh
  inside the fake repo with ``KANI_TIMEOUT`` and ``FAKE_KANI_EXIT``
  wired up.

The factory pattern (rather than a single ``fake_cargo_repo`` fixture)
keeps test setup self-documenting at the call site -- each test states
its own success-summary string.
"""

from __future__ import annotations

import os
import shlex
import shutil
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
VERIFY_KANI_SOURCE = REPO_ROOT / "scripts" / "verification" / "verify-kani.sh"


def _build_fake_cargo_script(success_summary: str | None) -> str:
    """Return the bash source for the fake ``cargo`` shim.

    The shim handles two concerns:

    1. ``cargo kani --version`` must succeed so verify-kani.sh's
       ``check_kani`` passes.
    2. Any other invocation echoes a fake "Checking harness ..." line,
       optionally a ``success_summary`` line that verify-kani.sh's
       parser scrapes for "Complete - N successfully verified ...", and
       a trailing diagnostic, then exits with ``$FAKE_KANI_EXIT``.

    All interpolated values are passed through :func:`shlex.quote` so
    apostrophes or other shell metacharacters in ``success_summary``
    don't break the shim source. Without quoting, a string like
    ``"can't parse"`` would close the surrounding single-quote and
    inject the rest as bash code.
    """
    summary_echo = ""
    if success_summary is not None:
        summary_echo = f"echo {shlex.quote(success_summary)}\n"
    return (
        "#!/bin/bash\n"
        "set -euo pipefail\n"
        'if [[ "${1:-}" == "kani" && "${2:-}" == "--version" ]]; then\n'
        "  echo 'cargo-kani 0.67.0'\n"
        "  exit 0\n"
        "fi\n"
        "echo 'Checking harness fake::proof_timeout_like_exit...'\n"
        + summary_echo
        + "echo 'last diagnostic line from fake Kani'\n"
        'exit "${FAKE_KANI_EXIT:-1}"\n'
    )


@pytest.fixture()
def fake_cargo_repo_factory(tmp_path: Path):
    """Factory that builds a temp repo with verify-kani.sh and fake cargo.

    Tests call ``factory(success_summary=...)`` to get
    ``(repo, script_path, fake_bin)``. Pass a Kani-style ``"Complete -
    N successfully verified harnesses, M failures, T total."`` string
    when the test needs verify-kani.sh's success-path parser to see a
    summary line; pass ``None`` (the default) when the test cares only
    about the failure path.
    """

    def _factory(*, success_summary: str | None = None) -> tuple[Path, Path, Path]:
        repo = tmp_path / "repo"
        script_path = repo / "scripts" / "verification" / "verify-kani.sh"
        script_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(VERIFY_KANI_SOURCE, script_path)

        fake_bin = repo / "fake-bin"
        fake_bin.mkdir()
        cargo = fake_bin / "cargo"
        cargo.write_text(
            _build_fake_cargo_script(success_summary), encoding="utf-8"
        )
        cargo.chmod(0o755)

        # check_kani also looks for the ``cargo-kani`` subcommand binary.
        cargo_kani = fake_bin / "cargo-kani"
        cargo_kani.write_text("#!/bin/bash\nexit 0\n", encoding="utf-8")
        cargo_kani.chmod(0o755)

        return repo, script_path, fake_bin

    return _factory


#: Environment variables we explicitly pass through from the parent shell
#: into the verify-kani.sh subprocess. Anything outside this list is dropped
#: so a developer's ambient ``KANI_UNWIND``/``NO_COLOR``/``KANI_TIMEOUT``/etc.
#: cannot perturb the test run. We keep:
#:   * ``PATH``: required to find bash + the fake-cargo shim.
#:   * ``HOME``/``USER``: some tools (e.g. cargo) refuse to start without them.
#:   * ``LANG`` / all ``LC_*`` vars: locale must be inherited so subprocess
#:     output encoding (UTF-8 vs C locale) matches the parent's expectations.
#:     ``LC_ALL`` is intentionally NOT listed here -- the ``LC_*`` family
#:     loop in :func:`_build_minimal_env` already catches it (and every
#:     other ``LC_*`` variable like ``LC_CTYPE``, ``LC_COLLATE``, ...).
#:     Listing it twice would be dead duplication.
#:   * ``TZ``: keeps date-arithmetic deterministic if the script ever uses it.
#:   * ``TERM``: set to ``dumb`` regardless of inherited value (verify-kani.sh
#:     already forces ``TERM=dumb`` before invoking cargo kani; we set it
#:     here as well to keep the harness side ANSI-free).
_PASSTHROUGH_ENV_KEYS: frozenset[str] = frozenset(
    {"PATH", "HOME", "USER", "LANG", "TZ"}
)


def _build_minimal_env(fake_bin: Path) -> dict[str, str]:
    """Build the minimal env dict for the verify-kani.sh subprocess.

    We DON'T inherit ``os.environ`` wholesale: a developer's shell could
    have ``KANI_UNWIND``, ``KANI_TIMEOUT``, ``NO_COLOR``, ``TERM`` etc.
    exported, and any of those would silently change verify-kani.sh's
    behaviour and break the tests' assumptions. The env dict here is
    constructed explicitly: the fixture's caller adds ``KANI_TIMEOUT``,
    ``FAKE_KANI_EXIT``, and any test-specific overrides on top.
    """
    env: dict[str, str] = {
        # PATH is rewritten by the caller to put the fake-bin shim first.
        "PATH": f"{fake_bin}{os.pathsep}{os.environ.get('PATH', '/usr/bin:/bin')}",
        # Harness-side terminal must be dumb (no ANSI) to match the
        # inner verify-kani.sh's own NO_COLOR/TERM=dumb policy.
        "TERM": "dumb",
    }
    for key in _PASSTHROUGH_ENV_KEYS:
        if key == "PATH":
            # Already handled above.
            continue
        value = os.environ.get(key)
        if value is not None:
            env[key] = value
    # LC_* is a family rather than a single key. Pass through every LC_*
    # variable so e.g. LC_CTYPE/LC_COLLATE inherits without listing each.
    for key, value in os.environ.items():
        if key.startswith("LC_") and key not in env:
            env[key] = value
    return env


@pytest.fixture()
def verify_kani_runner():
    """Callable that runs verify-kani.sh inside a fake-cargo repo.

    Signature::

        runner(repo, script_path, fake_bin, *, args, fake_exit,
               kani_timeout="42", extra_env=None) -> CompletedProcess

    Centralising the env-var wiring (``PATH``, ``FAKE_KANI_EXIT``,
    ``KANI_TIMEOUT``) keeps tests focused on the assertions that matter.
    The env dict is built MINIMALLY (see :func:`_build_minimal_env`)
    rather than inheriting ``os.environ`` wholesale, so ambient
    ``KANI_UNWIND``/``NO_COLOR``/``TERM`` from the developer's shell
    cannot leak in and perturb the test.
    """

    def _run(
        repo: Path,
        script_path: Path,
        fake_bin: Path,
        *,
        args: list[str],
        fake_exit: int,
        kani_timeout: str = "42",
        extra_env: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        env = _build_minimal_env(fake_bin)
        env["FAKE_KANI_EXIT"] = str(fake_exit)
        env["KANI_TIMEOUT"] = kani_timeout
        if extra_env:
            env.update(extra_env)
        return subprocess.run(
            ["bash", str(script_path), *args],
            cwd=repo,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

    return _run
