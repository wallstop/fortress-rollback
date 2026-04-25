#!/usr/bin/env python3
"""Regression tests for Dependabot auto-merge strategy selection."""

from __future__ import annotations

import os
import stat
import subprocess
from pathlib import Path

import pytest
import yaml


SCRIPT_PATH = (
    Path(__file__).resolve().parent.parent / "ci" / "enable-dependabot-automerge.sh"
)
WORKFLOW_PATH = (
    Path(__file__).resolve().parents[2]
    / ".github"
    / "workflows"
    / "dependabot-auto-merge.yml"
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
    if [[ "$subcmd" == "--paginate" ]]; then
        subcmd="${1:-}"
        shift || true
    fi

    if [[ "$subcmd" == *"/commits/"*"/check-runs"* ]]; then
        printf '%s\\n' "api $subcmd $*" >> "${GH_LOG_PATH:?GH_LOG_PATH is required}"
        if [[ -n "${GH_CHECK_RUNS_JSON:-}" ]]; then
            printf '%s\\n' "$GH_CHECK_RUNS_JSON"
        else
            printf '%s\\n' '{"check_runs":[{"status":"completed","conclusion":"success","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"}]}'
        fi
        exit 0
    fi
    if [[ "$subcmd" == *"/commits/"*"/status"* ]]; then
        printf '%s\\n' "api $subcmd $*" >> "${GH_LOG_PATH:?GH_LOG_PATH is required}"
        if [[ -n "${GH_COMMIT_STATUS_JSON:-}" ]]; then
            printf '%s\\n' "$GH_COMMIT_STATUS_JSON"
        else
            printf '%s\\n' '{"statuses":[]}'
        fi
        exit 0
    fi

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
    if [[ "$*" == *"--required --json name --jq length"* ]]; then
        if [[ "${GH_REQUIRED_CHECKS_ERROR:-false}" == "true" ]]; then
            printf '%s\\n' "failed to query required checks" >&2
            exit 1
        fi
        if [[ "${GH_REQUIRED_CHECKS_UNAVAILABLE:-false}" == "true" ]]; then
            printf '%s\\n' "no required checks reported on the 'mock-branch' branch" >&2
            exit 1
        fi
        next_sequence_value "${GH_REQUIRED_CHECKS_COUNT_SEQUENCE:-}" "${GH_REQUIRED_CHECKS_COUNT:-1}" "required_checks_count"
        exit 0
    fi
    if [[ "$*" == *"--required --json name,state,link"* ]]; then
        if [[ -n "${GH_REQUIRED_CHECKS_DETAILS_JSON:-}" ]]; then
            printf '%s\\n' "$GH_REQUIRED_CHECKS_DETAILS_JSON"
        elif [[ -n "${GH_REQUIRED_CHECKS_STATE_JSON:-}" ]]; then
            printf '%s\\n' "$GH_REQUIRED_CHECKS_STATE_JSON"
        else
            printf '%s\\n' '[{"name":"ci","state":"pass","link":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"}]'
        fi
        exit 0
    fi
    if [[ "$*" == *"--json name --jq length"* ]]; then
        next_sequence_value "${GH_ALL_CHECKS_COUNT_SEQUENCE:-}" "${GH_ALL_CHECKS_COUNT:-1}" "all_checks_count"
        exit 0
    fi
    if [[ "$*" == *"--watch"* ]]; then
        if [[ "$*" == *"--required"* ]]; then
            exit "${GH_CHECKS_REQUIRED_WATCH_EXIT_CODE:-${GH_CHECKS_WATCH_EXIT_CODE:-0}}"
        fi
        exit "${GH_CHECKS_ALL_WATCH_EXIT_CODE:-${GH_CHECKS_WATCH_EXIT_CODE:-0}}"
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
            "GITHUB_RUN_ID": "12345",
            "GH_TOKEN": "fake-token",
            "GH_LOG_PATH": str(log_path),
            "GH_STATE_DIR": str(tmp_path),
            "REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS": "0",
            "REQUIRED_CHECKS_POLL_INTERVAL_SECONDS": "1",
            "REQUIRED_CHECKS_WATCH_INTERVAL_SECONDS": "1",
            "FALLBACK_CHECKS_TIMEOUT_SECONDS": "1",
            "FALLBACK_CHECKS_POLL_INTERVAL_SECONDS": "1",
            "FALLBACK_STABLE_POLLS_REQUIRED": "1",
            "REQUIRED_CHECKS_SETTLE_TIMEOUT_SECONDS": "1",
            "REQUIRED_CHECKS_SETTLE_POLL_INTERVAL_SECONDS": "1",
            "REQUIRED_STABLE_POLLS_REQUIRED": "1",
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
    assert "--required --json name,state,link" in log_lines[1]
    assert "--squash" in log_lines[2]
    assert "--rebase" not in log_lines[2]
    assert "--merge" not in log_lines[2]


def test_one_shot_env_var_has_no_effect(tmp_path: Path) -> None:
    # The script no longer reads DEPENDABOT_AUTOMERGE_ONE_SHOT. Setting it to "true"
    # must not bypass the check-gating polling loop -- proof the legacy escape hatch
    # is fully gone.
    result = _run_script(
        tmp_path,
        {
            "DEPENDABOT_AUTOMERGE_ONE_SHOT": "true",
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0

    log_text = (tmp_path / "gh.log").read_text(encoding="utf-8")
    # The polling path must run regardless of the legacy env var.
    assert "--required --json name --jq length" in log_text
    assert "--required --json name,state,link" in log_text
    assert "--squash" in log_text


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


def test_falls_back_to_all_checks_when_required_checks_count_is_zero(
    tmp_path: Path,
) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "0",
            "GH_ALL_CHECKS_COUNT": "2",
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0
    assert (
        "Required checks did not appear before timeout; waiting for fallback to non-self checks/statuses."
        in result.stdout
    )

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 5
    assert "--required --json name --jq length" in log_lines[0]
    assert "--json name --jq length" in log_lines[1]
    assert "/check-runs" in log_lines[2]
    assert "/status" in log_lines[3]
    assert "--squash" in log_lines[4]


def test_falls_back_when_required_checks_unavailable(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_UNAVAILABLE": "true",
            "GH_ALL_CHECKS_COUNT": "1",
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 5
    assert "--required --json name --jq length" in log_lines[0]
    assert "--json name --jq length" in log_lines[1]
    assert "/check-runs" in log_lines[2]
    assert "/status" in log_lines[3]
    assert "--squash" in log_lines[4]


def test_fails_when_required_checks_query_errors(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_ERROR": "true",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 1
    assert "Failed to query required checks for PR" in result.stderr

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 1
    assert "--required --json name --jq length" in log_lines[0]
    assert "pr merge" not in "\n".join(log_lines)


def test_fails_when_no_checks_are_detected(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "0",
            "GH_ALL_CHECKS_COUNT": "0",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 1
    assert "No checks detected for PR within timeout" in result.stderr

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 2
    assert "--required --json name --jq length" in log_lines[0]
    assert "--json name --jq length" in log_lines[1]
    assert "pr merge" not in "\n".join(log_lines)


@pytest.mark.parametrize(
    ("check_runs_json", "commit_status_json", "expected_diagnostic"),
    [
        (
            '{"check_runs":[{"name":"strict-build","status":"completed","conclusion":"failure","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"}]}',
            '{"statuses":[]}',
            "check_run: strict-build [failure] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1",
        ),
        (
            '{"check_runs":[]}',
            '{"statuses":[{"context":"ci/external","state":"error","target_url":"https://example.invalid/check"}]}',
            "status: ci/external [error] https://example.invalid/check",
        ),
        (
            '{"check_runs":[{"name":"strict-build","status":"completed","conclusion":"timed_out","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/2"}]}',
            '{"statuses":[]}',
            "check_run: strict-build [timed_out] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/2",
        ),
        (
            '{"check_runs":[{"name":"strict-build","status":"completed","conclusion":"cancelled","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/3"}]}',
            '{"statuses":[]}',
            "check_run: strict-build [cancelled] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/3",
        ),
        (
            '{"check_runs":[{"name":"strict-build","status":"completed","conclusion":"action_required","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/4"}]}',
            '{"statuses":[]}',
            "check_run: strict-build [action_required] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/4",
        ),
        (
            '{"check_runs":[{"name":"strict-build","status":"completed","conclusion":"startup_failure","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/5"}]}',
            '{"statuses":[]}',
            "check_run: strict-build [startup_failure] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/5",
        ),
        (
            '{"check_runs":[{"name":"strict-build","status":"completed","conclusion":"stale","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/6"}]}',
            '{"statuses":[]}',
            "check_run: strict-build [stale] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/6",
        ),
    ],
)
def test_fails_when_fallback_checks_report_failures_with_diagnostics(
    tmp_path: Path,
    check_runs_json: str,
    commit_status_json: str,
    expected_diagnostic: str,
) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "0",
            "GH_ALL_CHECKS_COUNT": "1",
            "GH_CHECK_RUNS_JSON": check_runs_json,
            "GH_COMMIT_STATUS_JSON": commit_status_json,
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 1
    assert "Fallback checks/statuses failing/cancelled" in result.stderr
    assert expected_diagnostic in result.stderr
    assert "Checks did not pass" in result.stderr

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 4
    assert "--required --json name --jq length" in log_lines[0]
    assert "--json name --jq length" in log_lines[1]
    assert "/check-runs" in log_lines[2]
    assert "/status" in log_lines[3]
    assert "pr merge" not in "\n".join(log_lines)


def test_fails_when_fallback_only_sees_self_pending_check(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "0",
            "GH_ALL_CHECKS_COUNT": "1",
            "GH_CHECK_RUNS_JSON": '{"check_runs":[{"status":"in_progress","conclusion":null,"details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/12345/job/1"}]}',
            "FALLBACK_CHECKS_TIMEOUT_SECONDS": "1",
            "FALLBACK_CHECKS_POLL_INTERVAL_SECONDS": "1",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 1
    assert "Checks did not settle in fallback mode within timeout" in result.stderr

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) >= 4
    assert "--required --json name --jq length" in log_lines[0]
    assert "--json name --jq length" in log_lines[1]
    assert "/check-runs" in log_lines[2]
    assert "/status" in log_lines[3]
    assert "pr merge" not in "\n".join(log_lines)


def test_fallback_uses_latest_check_run_result_per_context(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "0",
            "GH_ALL_CHECKS_COUNT": "1",
            "GH_CHECK_RUNS_JSON": (
                '{"check_runs":['
                '{"name":"build","app":{"slug":"github-actions"},"status":"completed","conclusion":"failure","completed_at":"2026-01-01T00:00:00Z","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"},'
                '{"name":"build","app":{"slug":"github-actions"},"status":"completed","conclusion":"success","completed_at":"2026-01-01T00:01:00Z","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/2"}'
                ']}'
            ),
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0
    assert "Auto-merge enabled with squash strategy." in result.stdout

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 5
    assert "--required --json name --jq length" in log_lines[0]
    assert "--json name --jq length" in log_lines[1]
    assert "/check-runs" in log_lines[2]
    assert "/status" in log_lines[3]
    assert "--squash" in log_lines[4]


def test_fallback_ignores_older_pending_when_latest_check_is_success(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "0",
            "GH_ALL_CHECKS_COUNT": "1",
            "GH_CHECK_RUNS_JSON": (
                '{"check_runs":['
                '{"name":"build","app":{"slug":"github-actions"},"id":100,"status":"in_progress","conclusion":null,"started_at":"2026-01-01T00:00:00Z","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"},'
                '{"name":"build","app":{"slug":"github-actions"},"id":101,"status":"completed","conclusion":"success","completed_at":"2026-01-01T00:01:00Z","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/2"}'
                ']}'
            ),
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0
    assert "Auto-merge enabled with squash strategy." in result.stdout

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 5
    assert "--required --json name --jq length" in log_lines[0]
    assert "--json name --jq length" in log_lines[1]
    assert "/check-runs" in log_lines[2]
    assert "/status" in log_lines[3]
    assert "--squash" in log_lines[4]


@pytest.mark.parametrize(
    ("required_state", "expected_diagnostic"),
    [
        (
            "fail",
            "  - ci [fail] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1",
        ),
        (
            "cancel",
            "  - ci [cancel] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1",
        ),
        (
            "failure",
            "  - ci [failure] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1",
        ),
        (
            "error",
            "  - ci [error] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1",
        ),
        (
            "timed_out",
            "  - ci [timed_out] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1",
        ),
        (
            "cancelled",
            "  - ci [cancelled] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1",
        ),
        (
            "action_required",
            "  - ci [action_required] https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1",
        ),
    ],
)
def test_fails_when_required_checks_fail_with_diagnostics(
    tmp_path: Path,
    required_state: str,
    expected_diagnostic: str,
) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_STATE_JSON": f'[{{"name":"ci","state":"{required_state}","link":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"}}]',
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 1
    assert "Required checks failing/cancelled" in result.stderr
    assert expected_diagnostic in result.stderr
    assert "Required checks did not pass" in result.stderr

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 2
    assert "--json name --jq length" in log_lines[0]
    assert "--required --json name,state,link" in log_lines[1]
    assert "pr merge" not in "\n".join(log_lines)


@pytest.mark.parametrize(
    "required_state",
    ["success", "skipping", "neutral"],
)
def test_required_checks_non_blocking_states_proceed_to_merge(
    tmp_path: Path,
    required_state: str,
) -> None:
    """States in the allow-list (success, skipping, neutral) must proceed to merge.

    `gh pr checks --required` lowercases via `ascii_downcase`; the script's allow-list
    must accept these tokens so legitimate non-failure outcomes don't block merging.
    """
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_STATE_JSON": f'[{{"name":"ci","state":"{required_state}","link":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"}}]',
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0, (
        f"State {required_state!r} must proceed to merge, got stderr={result.stderr!r}"
    )
    log_text = (tmp_path / "gh.log").read_text(encoding="utf-8")
    assert "--squash" in log_text


def test_waits_for_required_checks_to_appear_then_merges(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT_SEQUENCE": "0,0,1",
            "GH_ALL_CHECKS_COUNT": "0",
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
    assert "--required --json name --jq length" in log_lines[0]
    assert "--required --json name --jq length" in log_lines[1]
    assert "--required --json name --jq length" in log_lines[2]
    assert "--required --json name,state,link" in log_lines[3]
    assert "--squash" in log_lines[4]


def test_skips_when_head_becomes_stale_while_waiting(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT_SEQUENCE": "0,0,1",
            "GH_ALL_CHECKS_COUNT": "0",
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
    assert "after required checks completed" not in result.stdout

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 1
    assert "--required --json name --jq length" in log_lines[0]
    assert "/check-runs" not in "\n".join(log_lines)
    assert "pr merge" not in "\n".join(log_lines)


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
    assert "after required checks completed" not in result.stdout

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) == 1
    assert "--json name --jq length" in log_lines[0]
    assert "/check-runs" not in log_lines[0]
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
    assert (
        "PR head moved while waiting for required checks" in result.stdout
        or "PR head moved after required checks completed" in result.stdout
    )

    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert len(log_lines) >= 1
    assert "--json name --jq length" in log_lines[0]
    assert "pr merge" not in "\n".join(log_lines)


def test_caps_poll_sleep_to_remaining_timeout(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT_SEQUENCE": "0,1",
            "GH_ALL_CHECKS_COUNT": "0",
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
    assert "--required --json name --jq length" in log_lines[0]
    assert "--required --json name --jq length" in log_lines[1]
    assert "--required --json name,state,link" in log_lines[2]
    assert "--squash" in log_lines[3]


def test_skipped_conclusion_does_not_block_automerge(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "0",
            "GH_ALL_CHECKS_COUNT": "1",
            "GH_CHECK_RUNS_JSON": (
                '{"check_runs":[{"name":"matrix-skipped","status":"completed",'
                '"conclusion":"skipped",'
                '"details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"}]}'
            ),
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0
    assert "Auto-merge enabled with squash strategy." in result.stdout

    log_text = (tmp_path / "gh.log").read_text(encoding="utf-8")
    assert "--squash" in log_text


def test_neutral_conclusion_does_not_block_automerge(tmp_path: Path) -> None:
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "0",
            "GH_ALL_CHECKS_COUNT": "1",
            "GH_CHECK_RUNS_JSON": (
                '{"check_runs":[{"name":"informational","status":"completed",'
                '"conclusion":"neutral",'
                '"details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"}]}'
            ),
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0
    assert "Auto-merge enabled with squash strategy." in result.stdout

    log_text = (tmp_path / "gh.log").read_text(encoding="utf-8")
    assert "--squash" in log_text


def _iter_workflow_steps(workflow: dict) -> list[dict]:
    """Return a flat list of every step across every job in the workflow."""
    steps: list[dict] = []
    for job in (workflow.get("jobs") or {}).values():
        if not isinstance(job, dict):
            continue
        for step in job.get("steps") or []:
            if isinstance(step, dict):
                steps.append(step)
    return steps


def test_workflow_does_not_set_one_shot_env() -> None:
    workflow = yaml.safe_load(WORKFLOW_PATH.read_text(encoding="utf-8"))

    enable_step = next(
        (
            step
            for step in _iter_workflow_steps(workflow)
            if step.get("name") == "Enable auto-merge"
        ),
        None,
    )
    assert enable_step is not None, "Enable auto-merge step not found in workflow"
    step_env = enable_step.get("env") or {}
    assert "DEPENDABOT_AUTOMERGE_ONE_SHOT" not in step_env

    # Defensive sweep: even if some other job/step ever re-introduced the variable,
    # this test fails. The legacy escape hatch must stay gone everywhere.
    for step in _iter_workflow_steps(workflow):
        env = step.get("env") or {}
        assert "DEPENDABOT_AUTOMERGE_ONE_SHOT" not in env, (
            f"DEPENDABOT_AUTOMERGE_ONE_SHOT must not appear in any step env "
            f"(found in step {step.get('name', '<unnamed>')})"
        )

    for job in (workflow.get("jobs") or {}).values():
        if not isinstance(job, dict):
            continue
        job_env = job.get("env") or {}
        assert "DEPENDABOT_AUTOMERGE_ONE_SHOT" not in job_env

    workflow_env = workflow.get("env") or {}
    assert "DEPENDABOT_AUTOMERGE_ONE_SHOT" not in workflow_env


def test_workflow_timeout_is_sufficient_for_polling() -> None:
    # script settle timeout is 1800s (30 min); workflow needs >=30 min + 2 min
    # buffer = 32 minutes minimum so polling can finish before the job is killed.
    workflow = yaml.safe_load(WORKFLOW_PATH.read_text(encoding="utf-8"))
    job = workflow["jobs"]["dependabot"]
    timeout_minutes = job.get("timeout-minutes")
    assert isinstance(timeout_minutes, int), (
        f"jobs.dependabot.timeout-minutes must be an integer, got {timeout_minutes!r}"
    )
    assert timeout_minutes >= 32, (
        f"jobs.dependabot.timeout-minutes must be >=32 to outlive polling; got {timeout_minutes}"
    )


def _find_step(workflow: dict, name: str) -> dict:
    """Find a step by name across all jobs in the workflow."""
    enable_step = next(
        (
            step
            for step in _iter_workflow_steps(workflow)
            if step.get("name") == name
        ),
        None,
    )
    assert enable_step is not None, f"Step {name!r} not found in workflow"
    return enable_step


def test_workflow_run_command_is_pure_script_invocation() -> None:
    """The Enable auto-merge step must invoke the script directly with no wrapper.

    Wrapping the script in `timeout`, `timeout-cmd`, `xargs`, or any prefix is
    forbidden -- the workflow's `timeout-minutes` is the sole ceiling. This test
    locks in that contract by asserting the run command equals the bare script
    invocation verbatim, so any wrapper at all (regardless of duration) fails.
    """
    workflow = yaml.safe_load(WORKFLOW_PATH.read_text(encoding="utf-8"))
    enable_step = _find_step(workflow, "Enable auto-merge")
    run_value = enable_step["run"].strip()
    assert run_value == "bash ./scripts/ci/enable-dependabot-automerge.sh", (
        f"Enable auto-merge step must run the script directly, got: {run_value!r}"
    )


def test_script_has_no_one_shot_references() -> None:
    """The script must not reference DEPENDABOT_AUTOMERGE_ONE_SHOT in any form.

    The bypass code path was deleted to lock in the gating policy. If anyone
    re-introduces the variable in the script -- to read, validate, or branch on it --
    this test fails. Test files and docs are exempt: they reference the name to
    prevent its return.
    """
    script_text = SCRIPT_PATH.read_text(encoding="utf-8")
    assert "DEPENDABOT_AUTOMERGE_ONE_SHOT" not in script_text, (
        "Script references DEPENDABOT_AUTOMERGE_ONE_SHOT; bypass mode is forbidden."
    )


def test_falls_back_when_only_self_required_check_present(tmp_path: Path) -> None:
    """If the only required check is the auto-merge job itself, fall back to non-self gating.

    The required-checks loop counts non-self entries. If self-exclusion drops the
    count to zero, the loop must fall through to wait_for_all_checks_without_required_metadata
    rather than waiting forever for "non-zero required_total".
    """
    self_link = (
        "https://github.com/wallstop/fortress-rollback/actions/runs/12345/job/1"
    )
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "1",
            "GH_REQUIRED_CHECKS_DETAILS_JSON": (
                f'[{{"name":"dependabot-automerge","state":"pending","link":"{self_link}"}}]'
            ),
            "GH_ALL_CHECKS_COUNT": "1",
            "GH_CHECK_RUNS_JSON": (
                '{"check_runs":[{"name":"non-self-build","status":"completed",'
                '"conclusion":"success",'
                '"details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"}]}'
            ),
            "GH_MERGE_SUCCESS_FLAG": "--squash",
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 0, (
        f"Self-only required check must fall back; got stderr={result.stderr!r}"
    )
    assert "All required checks are self" in result.stdout
    log_text = (tmp_path / "gh.log").read_text(encoding="utf-8")
    assert "--squash" in log_text


def test_null_conclusion_is_treated_as_failure(tmp_path: Path) -> None:
    """A check-run with status=completed and conclusion=null must NOT merge.

    The script uses an allow-list of safe conclusions; anything else (including
    null/missing) blocks. This guards against future GitHub Checks API additions
    that we haven't taught the script about.
    """
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "0",
            "GH_ALL_CHECKS_COUNT": "1",
            "GH_CHECK_RUNS_JSON": (
                '{"check_runs":[{"name":"weird","status":"completed","conclusion":null,'
                '"details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"}]}'
            ),
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 1
    assert "Checks did not pass" in result.stderr
    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert "pr merge" not in "\n".join(log_lines)


def test_workflow_preserves_fork_guard() -> None:
    """The pull_request_target trigger requires a fork-PR guard to be safe.

    `pull_request_target` runs in the base-ref context with secrets exposure;
    the head.repo.full_name guard is what prevents fork PRs from triggering.
    Locking it in here so a future YAML edit cannot weaken it without failing CI.
    """
    workflow = yaml.safe_load(WORKFLOW_PATH.read_text(encoding="utf-8"))
    if_clause = workflow["jobs"]["dependabot"]["if"]
    assert "github.event.pull_request.head.repo.full_name == github.repository" in if_clause, (
        f"Fork-PR guard missing or weakened in if: clause: {if_clause!r}"
    )
    assert "github.event.pull_request.user.login == 'dependabot[bot]'" in if_clause, (
        f"Dependabot-only guard missing or weakened in if: clause: {if_clause!r}"
    )


def test_fallback_blocks_when_same_name_check_from_different_app_fails(tmp_path: Path) -> None:
    """Two check-runs with the same name from different apps must each be evaluated.

    The grouping key is (name, app.slug). Two runs with the same name but distinct
    app slugs are kept separately; if either fails, the merge is blocked. Regression
    guard against group-key changes.
    """
    result = _run_script(
        tmp_path,
        {
            "GH_REQUIRED_CHECKS_COUNT": "0",
            "GH_ALL_CHECKS_COUNT": "1",
            "GH_CHECK_RUNS_JSON": (
                '{"check_runs":['
                '{"name":"build","app":{"slug":"github-actions"},"status":"completed","conclusion":"success",'
                '"completed_at":"2026-01-01T00:00:00Z","details_url":"https://github.com/wallstop/fortress-rollback/actions/runs/999/job/1"},'
                '{"name":"build","app":{"slug":"third-party"},"status":"completed","conclusion":"failure",'
                '"completed_at":"2026-01-01T00:01:00Z","details_url":"https://example.invalid/check"}'
                ']}'
            ),
            "GH_ALLOW_SQUASH": "true",
            "GH_ALLOW_REBASE": "false",
            "GH_ALLOW_MERGE": "false",
        },
    )
    assert result.returncode == 1
    assert "Fallback checks/statuses failing/cancelled" in result.stderr
    assert "build" in result.stderr
    log_lines = (tmp_path / "gh.log").read_text(encoding="utf-8").splitlines()
    assert "pr merge" not in "\n".join(log_lines)
