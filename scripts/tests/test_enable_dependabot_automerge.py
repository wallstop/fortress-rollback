#!/usr/bin/env python3
"""Regression tests for Dependabot auto-merge strategy selection."""

from __future__ import annotations

import os
import stat
import subprocess
from pathlib import Path


SCRIPT_PATH = (
    Path(__file__).resolve().parent.parent / "ci" / "enable-dependabot-automerge.sh"
)


def _write_stub_gh(path: Path) -> None:
    path.write_text(
        """#!/bin/bash
set -euo pipefail

cmd="$1"
subcmd="${2:-}"
shift
shift || true

if [[ "$cmd" == "pr" && "$subcmd" == "view" ]]; then
    jq_expr=""
    while [[ $# -gt 0 ]]; do
        if [[ "$1" == "--jq" ]]; then
            jq_expr="$2"
            break
        fi
        shift
    done
    case "$jq_expr" in
        ".state") printf '%s\\n' "${GH_PR_STATE:-OPEN}" ;;
        ".isDraft") printf '%s\\n' "${GH_PR_DRAFT:-false}" ;;
        ".autoMergeRequest != null") printf '%s\\n' "${GH_PR_AUTO_MERGE:-false}" ;;
        ".headRefOid") printf '%s\\n' "${GH_PR_HEAD_OID:-head-sha}" ;;
        *) exit 1 ;;
    esac
    exit 0
fi

if [[ "$cmd" == "api" ]]; then
    jq_expr=""
    while [[ $# -gt 0 ]]; do
        if [[ "$1" == "--jq" ]]; then
            jq_expr="$2"
            break
        fi
        shift
    done
    case "$jq_expr" in
        ".allow_squash_merge") printf '%s\\n' "${GH_ALLOW_SQUASH:-true}" ;;
        ".allow_rebase_merge") printf '%s\\n' "${GH_ALLOW_REBASE:-true}" ;;
        ".allow_merge_commit") printf '%s\\n' "${GH_ALLOW_MERGE:-false}" ;;
        *) exit 1 ;;
    esac
    exit 0
fi

if [[ "$cmd" == "pr" && "$subcmd" == "merge" ]]; then
    printf '%s\\n' "pr merge $*" >> "${GH_LOG_PATH:?GH_LOG_PATH is required}"
    success_flag="${GH_MERGE_SUCCESS_FLAG:-__none__}"
    if [[ "$success_flag" == "__none__" ]]; then
        if [[ "$*" == *"--squash"* || "$*" == *"--rebase"* || "$*" == *"--merge"* ]]; then
            exit 1
        fi
        exit 0
    fi
    if [[ "$*" == *"$success_flag"* ]]; then
        exit 0
    fi
    exit 1
fi

exit 1
""",
        encoding="utf-8",
    )
    path.chmod(path.stat().st_mode | stat.S_IEXEC)


def _run_script(tmp_path: Path, extra_env: dict[str, str]) -> subprocess.CompletedProcess[str]:
    gh_stub = tmp_path / "gh"
    _write_stub_gh(gh_stub)
    log_path = tmp_path / "gh.log"
    env = os.environ.copy()
    env.update(
        {
            "PATH": f"{tmp_path}:{env['PATH']}",
            "PR_URL": "https://github.com/wallstop/fortress-rollback/pull/144",
            "PR_HEAD_SHA": "head-sha",
            "GITHUB_REPOSITORY": "wallstop/fortress-rollback",
            "GH_TOKEN": "fake-token",
            "GH_LOG_PATH": str(log_path),
        }
    )
    env.update(extra_env)
    return subprocess.run(
        [str(SCRIPT_PATH)],
        check=False,
        capture_output=True,
        text=True,
        env=env,
    )


def test_skips_when_auto_merge_already_enabled(tmp_path: Path) -> None:
    result = _run_script(tmp_path, {"GH_PR_AUTO_MERGE": "true"})
    assert result.returncode == 0
    assert "Auto-merge already enabled." in result.stdout
    log_path = tmp_path / "gh.log"
    assert not log_path.exists()


def test_falls_back_to_squash_when_no_strategy_fails(tmp_path: Path) -> None:
    result = _run_script(tmp_path, {"GH_MERGE_SUCCESS_FLAG": "--squash"})
    assert result.returncode == 0

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) >= 2
    assert "--squash" not in log_lines[0]
    assert "--rebase" not in log_lines[0]
    assert "--merge" not in log_lines[0]
    assert "--squash" in log_lines[1]


def test_skips_stale_event_without_merging(tmp_path: Path) -> None:
    result = _run_script(tmp_path, {"GH_PR_HEAD_OID": "new-head-sha"})
    assert result.returncode == 0
    assert "PR head moved since event" in result.stdout
    log_path = tmp_path / "gh.log"
    assert not log_path.exists()
