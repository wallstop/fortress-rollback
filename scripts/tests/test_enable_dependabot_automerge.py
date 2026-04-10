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

next_sequence_value() {
    local sequence="$1"
    local default_value="$2"
    local counter_key="$3"
    local state_dir="${GH_STATE_DIR:?GH_STATE_DIR is required}"
    local index_file="$state_dir/$counter_key.idx"
    local index=0

    if [[ -z "$sequence" ]]; then
        printf '%s\\n' "$default_value"
        return
    fi

    if [[ -f "$index_file" ]]; then
        index="$(cat "$index_file")"
    fi

    IFS=',' read -r -a values <<< "$sequence"
    if ((index < ${#values[@]})); then
        printf '%s\\n' "${values[$index]}"
    else
        printf '%s\\n' "${values[$((${#values[@]} - 1))]}"
    fi

    printf '%s\\n' $((index + 1)) > "$index_file"
}

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
        ".headRefOid") next_sequence_value "${GH_PR_HEAD_OID_SEQUENCE:-}" "${GH_PR_HEAD_OID:-head-sha}" "head_ref_oid" ;;
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

if [[ "$cmd" == "pr" && "$subcmd" == "checks" ]]; then
    printf '%s\\n' "pr checks $*" >> "${GH_LOG_PATH:?GH_LOG_PATH is required}"
    if [[ "$*" == *"--json name --jq length"* ]]; then
        next_sequence_value "${GH_REQUIRED_CHECKS_COUNT_SEQUENCE:-}" "${GH_REQUIRED_CHECKS_COUNT:-1}" "required_checks_count"
        exit 0
    fi
    if [[ "$*" == *"--watch"* ]]; then
        exit "${GH_CHECKS_WATCH_EXIT_CODE:-0}"
    fi
    exit 0
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
            "GH_STATE_DIR": str(tmp_path),
            "REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS": "0",
            "REQUIRED_CHECKS_POLL_INTERVAL_SECONDS": "1",
            "REQUIRED_CHECKS_WATCH_INTERVAL_SECONDS": "1",
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


def test_uses_squash_strategy_only(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 3
    assert "--json name --jq length" in log_lines[0]
    assert "--watch" in log_lines[1]
    assert "--squash" in log_lines[2]
    assert "--rebase" not in log_lines[2]
    assert "--merge" not in log_lines[2]


def test_skips_stale_event_without_merging(tmp_path: Path) -> None:
    result = _run_script(tmp_path, {"GH_PR_HEAD_OID": "new-head-sha"})
    assert result.returncode == 0
    assert "PR head moved since event" in result.stdout
    log_path = tmp_path / "gh.log"
    assert not log_path.exists()


def test_fails_on_merge_policy_drift(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "true",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 1
    assert "squash-only settings" in result.stderr
    log_path = tmp_path / "gh.log"
    assert not log_path.exists()


def test_fails_when_required_checks_are_missing(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "0",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 1
    assert "No required checks detected for PR within timeout" in result.stderr

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 1
    assert "--json name --jq length" in log_lines[0]
    assert "--watch" not in log_lines[0]
    assert "pr merge" not in log_lines[0]


def test_fails_when_required_checks_fail(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_CHECKS_WATCH_EXIT_CODE": "1",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 1
    assert "Required checks did not pass" in result.stderr

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 2
    assert "--json name --jq length" in log_lines[0]
    assert "--watch" in log_lines[1]
    assert "pr merge" not in "\n".join(log_lines)


def test_waits_for_required_checks_to_appear_then_merges(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT_SEQUENCE": "0,0,1",
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
            "REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS": "3",
            "REQUIRED_CHECKS_POLL_INTERVAL_SECONDS": "1",
        },
    )
    assert result.returncode == 0

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 5
    assert "--json name --jq length" in log_lines[0]
    assert "--json name --jq length" in log_lines[1]
    assert "--json name --jq length" in log_lines[2]
    assert "--watch" in log_lines[3]
    assert "--squash" in log_lines[4]


def test_skips_when_head_becomes_stale_while_waiting(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT_SEQUENCE": "0,0,1",
            "GH_PR_HEAD_OID_SEQUENCE": "head-sha,head-sha,new-head-sha",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
            "REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS": "3",
            "REQUIRED_CHECKS_POLL_INTERVAL_SECONDS": "1",
        },
    )
    assert result.returncode == 0
    assert "PR head moved while waiting for required checks" in result.stdout

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 1
    assert "--json name --jq length" in log_lines[0]
    assert "--watch" not in log_lines[0]
    assert "pr merge" not in log_lines[0]


def test_skips_when_head_becomes_stale_after_checks_appear(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT_SEQUENCE": "1",
            "GH_PR_HEAD_OID_SEQUENCE": "head-sha,head-sha,new-head-sha",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0
    assert "PR head moved after required checks appeared" in result.stdout

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 1
    assert "--json name --jq length" in log_lines[0]
    assert "--watch" not in log_lines[0]
    assert "pr merge" not in log_lines[0]


def test_skips_when_head_becomes_stale_after_checks_complete(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT_SEQUENCE": "1",
            "GH_PR_HEAD_OID_SEQUENCE": "head-sha,head-sha,head-sha,new-head-sha",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0
    assert "PR head moved after required checks completed" in result.stdout

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 2
    assert "--json name --jq length" in log_lines[0]
    assert "--watch" in log_lines[1]
    assert "pr merge" not in "\n".join(log_lines)


def test_caps_poll_sleep_to_remaining_timeout(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT_SEQUENCE": "0,1",
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
            "REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS": "1",
            "REQUIRED_CHECKS_POLL_INTERVAL_SECONDS": "10",
        },
    )
    assert result.returncode == 0

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 4
    assert "--json name --jq length" in log_lines[0]
    assert "--json name --jq length" in log_lines[1]
    assert "--watch" in log_lines[2]
    assert "--squash" in log_lines[3]
