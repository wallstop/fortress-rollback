#!/bin/bash
set -euo pipefail

: "${PR_URL:?PR_URL is required}"
: "${PR_HEAD_SHA:?PR_HEAD_SHA is required}"
: "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"

REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS="${REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS:-120}"
REQUIRED_CHECKS_POLL_INTERVAL_SECONDS="${REQUIRED_CHECKS_POLL_INTERVAL_SECONDS:-10}"
REQUIRED_CHECKS_WATCH_INTERVAL_SECONDS="${REQUIRED_CHECKS_WATCH_INTERVAL_SECONDS:-10}"
FALLBACK_CHECKS_TIMEOUT_SECONDS="${FALLBACK_CHECKS_TIMEOUT_SECONDS:-1800}"
FALLBACK_CHECKS_POLL_INTERVAL_SECONDS="${FALLBACK_CHECKS_POLL_INTERVAL_SECONDS:-10}"
FALLBACK_STABLE_POLLS_REQUIRED="${FALLBACK_STABLE_POLLS_REQUIRED:-2}"
REQUIRED_CHECKS_SETTLE_TIMEOUT_SECONDS="${REQUIRED_CHECKS_SETTLE_TIMEOUT_SECONDS:-1800}"
REQUIRED_CHECKS_SETTLE_POLL_INTERVAL_SECONDS="${REQUIRED_CHECKS_SETTLE_POLL_INTERVAL_SECONDS:-10}"
REQUIRED_STABLE_POLLS_REQUIRED="${REQUIRED_STABLE_POLLS_REQUIRED:-2}"
NO_REQUIRED_CHECKS_REPORTED_MSG="no required checks reported"
NO_REQUIRED_CHECKS_SENTINEL="-1"
DEPENDABOT_AUTOMERGE_ONE_SHOT="${DEPENDABOT_AUTOMERGE_ONE_SHOT:-false}"

if ! [[ "$REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS" =~ ^[0-9]+$ ]]; then
    echo "REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS must be a non-negative integer." >&2
    exit 1
fi

if ! [[ "$REQUIRED_CHECKS_POLL_INTERVAL_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
    echo "REQUIRED_CHECKS_POLL_INTERVAL_SECONDS must be a positive integer." >&2
    exit 1
fi

if ! [[ "$REQUIRED_CHECKS_WATCH_INTERVAL_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
    echo "REQUIRED_CHECKS_WATCH_INTERVAL_SECONDS must be a positive integer." >&2
    exit 1
fi

if ! [[ "$FALLBACK_CHECKS_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
    echo "FALLBACK_CHECKS_TIMEOUT_SECONDS must be a positive integer." >&2
    exit 1
fi

if ! [[ "$FALLBACK_CHECKS_POLL_INTERVAL_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
    echo "FALLBACK_CHECKS_POLL_INTERVAL_SECONDS must be a positive integer." >&2
    exit 1
fi

if ! [[ "$FALLBACK_STABLE_POLLS_REQUIRED" =~ ^[1-9][0-9]*$ ]]; then
    echo "FALLBACK_STABLE_POLLS_REQUIRED must be a positive integer." >&2
    exit 1
fi

if ! [[ "$REQUIRED_CHECKS_SETTLE_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
    echo "REQUIRED_CHECKS_SETTLE_TIMEOUT_SECONDS must be a positive integer." >&2
    exit 1
fi

if ! [[ "$REQUIRED_CHECKS_SETTLE_POLL_INTERVAL_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
    echo "REQUIRED_CHECKS_SETTLE_POLL_INTERVAL_SECONDS must be a positive integer." >&2
    exit 1
fi

if ! [[ "$REQUIRED_STABLE_POLLS_REQUIRED" =~ ^[1-9][0-9]*$ ]]; then
    echo "REQUIRED_STABLE_POLLS_REQUIRED must be a positive integer." >&2
    exit 1
fi

if [[ "$DEPENDABOT_AUTOMERGE_ONE_SHOT" != "true" && "$DEPENDABOT_AUTOMERGE_ONE_SHOT" != "false" ]]; then
    echo "DEPENDABOT_AUTOMERGE_ONE_SHOT must be either 'true' or 'false'." >&2
    exit 1
fi

if [[ "$DEPENDABOT_AUTOMERGE_ONE_SHOT" != "true" ]] && ! command -v jq >/dev/null 2>&1; then
    echo "jq is required to evaluate check state in fallback mode." >&2
    exit 1
fi

get_pr_field() {
    local jq_expr="$1"
    local output
    if ! output="$(gh pr view "$PR_URL" --json state,isDraft,autoMergeRequest,headRefOid --jq "$jq_expr" 2>&1)"; then
        echo "Failed to read PR field '$jq_expr' from $PR_URL: $output" >&2
        exit 1
    fi
    printf '%s\n' "$output"
}

is_auto_merge_enabled() {
    [[ "$(get_pr_field '.autoMergeRequest != null')" == "true" ]]
}

is_stale_event() {
    [[ "$(get_pr_field '.headRefOid')" != "$PR_HEAD_SHA" ]]
}

attempt_automerge() {
    # Protect against races: only enable auto-merge if the PR head still matches the triggering SHA.
    local args=(pr merge --auto --squash --match-head-commit "$PR_HEAD_SHA")
    local output
    args+=("$PR_URL")
    if output="$(gh "${args[@]}" 2>&1)"; then
        return 0
    fi
    echo "Auto-merge attempt failed for squash strategy: $output" >&2
    return 1
}

required_checks_count() {
    local output
    if output="$(gh pr checks "$PR_URL" --required --json name --jq 'length' 2>&1)"; then
        printf '%s\n' "$output"
        return 0
    fi

    # GitHub CLI returns this message when required-check metadata is unavailable for the PR branch.
    if [[ "$output" == *"$NO_REQUIRED_CHECKS_REPORTED_MSG"* ]]; then
        printf '%s\n' "$NO_REQUIRED_CHECKS_SENTINEL"
        return 0
    fi

    echo "Failed to query required checks for PR: $output" >&2
    return 1
}

all_checks_count() {
    local output
    if output="$(gh pr checks "$PR_URL" --json name --jq 'length' 2>&1)"; then
        printf '%s\n' "$output"
        return 0
    fi
    echo "Failed to query checks for PR: $output" >&2
    return 1
}

count_non_self_checks() {
    local check_runs_json="$1"
    # Use ASCII Unit Separator (\u001f) as a low-collision key delimiter for jq group_by.
    jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
        [
            .check_runs[]?
            | select(
                ($run_id | length) == 0
                or ((.details_url // "") | contains("/actions/runs/\($run_id)/") | not)
            )
        ]
        | sort_by(.name, (.app.slug // ""), (.completed_at // .started_at // ""), (.id // 0))
        | group_by((.name // "") + "\u001f" + (.app.slug // ""))
        | map(last)
        | length
    ' <<<"$check_runs_json"
}

count_non_self_pending_checks() {
    local check_runs_json="$1"
    jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
        [
            .check_runs[]?
            | select(
                ($run_id | length) == 0
                or ((.details_url // "") | contains("/actions/runs/\($run_id)/") | not)
            )
        ]
        | sort_by(.name, (.app.slug // ""), (.completed_at // .started_at // ""), (.id // 0))
        | group_by((.name // "") + "\u001f" + (.app.slug // ""))
        | map(last)
        | map(select(.status != "completed"))
        | length
    ' <<<"$check_runs_json"
}

count_non_self_failed_checks() {
    local check_runs_json="$1"
    jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
        [
            .check_runs[]?
            | select(
                ($run_id | length) == 0
                or ((.details_url // "") | contains("/actions/runs/\($run_id)/") | not)
            )
        ]
        | sort_by(.name, (.app.slug // ""), (.completed_at // .started_at // ""), (.id // 0))
        | group_by((.name // "") + "\u001f" + (.app.slug // ""))
        | map(last)
        | map(select(
            .status == "completed" and (
                (.conclusion // "") == "failure"
                or (.conclusion // "") == "timed_out"
                or (.conclusion // "") == "cancelled"
                or (.conclusion // "") == "action_required"
                or (.conclusion // "") == "startup_failure"
                or (.conclusion // "") == "stale"
            )
        ))
        | length
    ' <<<"$check_runs_json"
}

count_non_self_commit_statuses() {
    local status_json="$1"
    jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
        [
            .statuses[]?
            | select(
                ($run_id | length) == 0
                or ((.target_url // "") | contains("/actions/runs/\($run_id)/") | not)
            )
        ]
        | sort_by(.context, (.updated_at // .created_at // ""))
        | group_by(.context)
        | map(last)
        | length
    ' <<<"$status_json"
}

count_non_self_pending_statuses() {
    local status_json="$1"
    jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
        [
            .statuses[]?
            | select(
                ($run_id | length) == 0
                or ((.target_url // "") | contains("/actions/runs/\($run_id)/") | not)
            )
        ]
        | sort_by(.context, (.updated_at // .created_at // ""))
        | group_by(.context)
        | map(last)
        | map(select(.state == "pending"))
        | length
    ' <<<"$status_json"
}

count_non_self_failed_statuses() {
    local status_json="$1"
    jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
        [
            .statuses[]?
            | select(
                ($run_id | length) == 0
                or ((.target_url // "") | contains("/actions/runs/\($run_id)/") | not)
            )
        ]
        | sort_by(.context, (.updated_at // .created_at // ""))
        | group_by(.context)
        | map(last)
        | map(select(.state == "failure" or .state == "error"))
        | length
    ' <<<"$status_json"
}

emit_required_checks_diagnostics() {
    # Emit actionable diagnostics for required checks when auto-merge cannot proceed.
    # Parameters:
    #   $1: JSON array from `gh pr checks --required --json name,state,link`
    # Behavior:
    #   - Excludes entries associated with the current workflow run to avoid self-noise.
    #   - Prints failed/cancelled checks and pending checks with names, states, and links.
    local checks_json="$1"
    local failed_checks
    local pending_checks

    failed_checks="$(jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
        [
            .[]?
            | select(
                (
                    ($run_id | length) == 0
                    or ((.link // "") | contains("/actions/runs/\($run_id)/") | not)
                )
                and (
                    (.state // "" | ascii_downcase) == "fail"
                    or (.state // "" | ascii_downcase) == "failure"
                    or (.state // "" | ascii_downcase) == "error"
                    or (.state // "" | ascii_downcase) == "timed_out"
                    or (.state // "" | ascii_downcase) == "cancel"
                    or (.state // "" | ascii_downcase) == "cancelled"
                    or (.state // "" | ascii_downcase) == "action_required"
                )
            )
        ]
        | sort_by((.name // ""), (.link // ""), (.state // ""))
        | map("  - \(.name // "<unknown>") [\(.state // "unknown")] \(.link // "no-link")")
        | .[]?
    ' <<<"$checks_json")"

    pending_checks="$(jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
        [
            .[]?
            | select(
                (
                    ($run_id | length) == 0
                    or ((.link // "") | contains("/actions/runs/\($run_id)/") | not)
                )
                and ((.state // "" | ascii_downcase) == "pending")
            )
        ]
        | sort_by((.name // ""), (.link // ""))
        | map("  - \(.name // "<unknown>") [pending] \(.link // "no-link")")
        | .[]?
    ' <<<"$checks_json")"

    if [[ -n "$failed_checks" ]]; then
        echo "Required checks failing/cancelled:" >&2
        printf '%s\n' "$failed_checks" >&2
    fi

    if [[ -n "$pending_checks" ]]; then
        echo "Required checks still pending:" >&2
        printf '%s\n' "$pending_checks" >&2
    fi
}

emit_fallback_checks_diagnostics() {
    # Emit actionable diagnostics for fallback check-run/status evaluation failures.
    # Parameters:
    #   $1: JSON object with `.check_runs[]` from commit check-runs API.
    #   $2: JSON object with `.statuses[]` from commit status API.
    # Behavior:
    #   - Excludes entries associated with the current workflow run to avoid self-noise.
    #   - Prints failed/cancelled and pending entries with context and links.
    local check_runs_json="$1"
    local status_json="$2"
    local failed_checks
    local pending_checks

    failed_checks="$(
        {
            jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
                [
                    .check_runs[]?
                    | select(
                        ($run_id | length) == 0
                        or ((.details_url // "") | contains("/actions/runs/\($run_id)/") | not)
                    )
                ]
                | sort_by(.name, (.app.slug // ""), (.completed_at // .started_at // ""), (.id // 0))
                | group_by((.name // "") + "\u001f" + (.app.slug // ""))
                | map(last)
                | map(select(
                    .status == "completed" and (
                        (.conclusion // "") == "failure"
                        or (.conclusion // "") == "timed_out"
                        or (.conclusion // "") == "cancelled"
                        or (.conclusion // "") == "action_required"
                        or (.conclusion // "") == "startup_failure"
                        or (.conclusion // "") == "stale"
                    )
                ))
                | map("  - check_run: \(.name // "<unknown>") [\(.conclusion // "unknown")] \(.details_url // "no-link")")
                | .[]?
            ' <<<"$check_runs_json"

            jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
                [
                    .statuses[]?
                    | select(
                        ($run_id | length) == 0
                        or ((.target_url // "") | contains("/actions/runs/\($run_id)/") | not)
                    )
                ]
                | sort_by(.context, (.updated_at // .created_at // ""))
                | group_by(.context)
                | map(last)
                | map(select(.state == "failure" or .state == "error"))
                | map("  - status: \(.context // "<unknown>") [\(.state // "unknown")] \(.target_url // "no-link")")
                | .[]?
            ' <<<"$status_json"
        }
    )"

    pending_checks="$(
        {
            jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
                [
                    .check_runs[]?
                    | select(
                        ($run_id | length) == 0
                        or ((.details_url // "") | contains("/actions/runs/\($run_id)/") | not)
                    )
                ]
                | sort_by(.name, (.app.slug // ""), (.completed_at // .started_at // ""), (.id // 0))
                | group_by((.name // "") + "\u001f" + (.app.slug // ""))
                | map(last)
                | map(select(.status != "completed"))
                | map("  - check_run: \(.name // "<unknown>") [\(.status // "unknown")] \(.details_url // "no-link")")
                | .[]?
            ' <<<"$check_runs_json"

            jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
                [
                    .statuses[]?
                    | select(
                        ($run_id | length) == 0
                        or ((.target_url // "") | contains("/actions/runs/\($run_id)/") | not)
                    )
                ]
                | sort_by(.context, (.updated_at // .created_at // ""))
                | group_by(.context)
                | map(last)
                | map(select(.state == "pending"))
                | map("  - status: \(.context // "<unknown>") [pending] \(.target_url // "no-link")")
                | .[]?
            ' <<<"$status_json"
        }
    )"

    if [[ -n "$failed_checks" ]]; then
        echo "Fallback checks/statuses failing/cancelled:" >&2
        printf '%s\n' "$failed_checks" >&2
    fi

    if [[ -n "$pending_checks" ]]; then
        echo "Fallback checks/statuses still pending:" >&2
        printf '%s\n' "$pending_checks" >&2
    fi
}

wait_for_all_checks_without_required_metadata() {
    local elapsed=0
    local remaining
    local sleep_for
    local stable_polls=0

    local check_runs_json
    local status_json
    local check_total
    local check_pending
    local check_failed
    local status_total
    local status_pending
    local status_failed

    while ((elapsed <= FALLBACK_CHECKS_TIMEOUT_SECONDS)); do
        if is_stale_event; then
            echo "PR head moved while waiting for fallback checks; skipping stale auto-merge attempt."
            return 2
        fi

        if ! check_runs_json="$(gh api --paginate "repos/$GITHUB_REPOSITORY/commits/$PR_HEAD_SHA/check-runs?per_page=100" 2>&1)"; then
            echo "Failed to query check runs for PR head: $check_runs_json" >&2
            return 1
        fi
        if ! check_runs_json="$(jq -cs '{check_runs: [.[].check_runs[]?]}' <<<"$check_runs_json" 2>&1)"; then
            echo "Failed to parse check runs for PR head: $check_runs_json" >&2
            return 1
        fi
        if ! status_json="$(gh api --paginate "repos/$GITHUB_REPOSITORY/commits/$PR_HEAD_SHA/status?per_page=100" 2>&1)"; then
            echo "Failed to query commit statuses for PR head: $status_json" >&2
            return 1
        fi
        if ! status_json="$(jq -cs '{statuses: [.[].statuses[]?]}' <<<"$status_json" 2>&1)"; then
            echo "Failed to parse commit statuses for PR head: $status_json" >&2
            return 1
        fi

        check_total="$(count_non_self_checks "$check_runs_json")"
        check_pending="$(count_non_self_pending_checks "$check_runs_json")"
        check_failed="$(count_non_self_failed_checks "$check_runs_json")"
        status_total="$(count_non_self_commit_statuses "$status_json")"
        status_pending="$(count_non_self_pending_statuses "$status_json")"
        status_failed="$(count_non_self_failed_statuses "$status_json")"

        if ((check_failed > 0 || status_failed > 0)); then
            emit_fallback_checks_diagnostics "$check_runs_json" "$status_json"
            echo "Checks did not pass; refusing to enable auto-merge." >&2
            return 1
        fi

        if ((check_total + status_total > 0)) && ((check_pending == 0 && status_pending == 0)); then
            stable_polls=$((stable_polls + 1))
            if ((stable_polls >= FALLBACK_STABLE_POLLS_REQUIRED)); then
                return 0
            fi
        else
            stable_polls=0
        fi

        remaining=$((FALLBACK_CHECKS_TIMEOUT_SECONDS - elapsed))
        if ((remaining <= 0)); then
            break
        fi

        sleep_for="$FALLBACK_CHECKS_POLL_INTERVAL_SECONDS"
        if ((sleep_for > remaining)); then
            sleep_for="$remaining"
        fi

        sleep "$sleep_for"
        elapsed=$((elapsed + sleep_for))
    done

    emit_fallback_checks_diagnostics "$check_runs_json" "$status_json"
    echo "Checks did not settle in fallback mode within timeout; refusing to enable auto-merge." >&2
    return 1
}

wait_for_required_checks_without_self() {
    local elapsed=0
    local remaining
    local sleep_for
    local stable_polls=0
    local checks_json
    local required_total
    local required_pending
    local required_failed

    while ((elapsed <= REQUIRED_CHECKS_SETTLE_TIMEOUT_SECONDS)); do
        if is_stale_event; then
            echo "PR head moved while waiting for required checks; skipping stale auto-merge attempt."
            return 2
        fi

        if ! checks_json="$(gh pr checks "$PR_URL" --required --json name,state,link 2>&1)"; then
            echo "Failed to query required checks state for PR: $checks_json" >&2
            return 1
        fi

        required_total="$(jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
            [
                .[]?
                | select(
                    ($run_id | length) == 0
                    or ((.link // "") | contains("/actions/runs/\($run_id)/") | not)
                )
            ] | length
        ' <<<"$checks_json")"
        required_pending="$(jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
            [
                .[]?
                | select(
                    (
                        ($run_id | length) == 0
                        or ((.link // "") | contains("/actions/runs/\($run_id)/") | not)
                    )
                    and ((.state // "" | ascii_downcase) == "pending")
                )
            ] | length
        ' <<<"$checks_json")"
        required_failed="$(jq -r --arg run_id "${GITHUB_RUN_ID:-}" '
            [
                .[]?
                | select(
                    (
                        ($run_id | length) == 0
                        or ((.link // "") | contains("/actions/runs/\($run_id)/") | not)
                    )
                    and (
                        (.state // "" | ascii_downcase) == "fail"
                        or (.state // "" | ascii_downcase) == "failure"
                        or (.state // "" | ascii_downcase) == "error"
                        or (.state // "" | ascii_downcase) == "timed_out"
                        or (.state // "" | ascii_downcase) == "cancel"
                        or (.state // "" | ascii_downcase) == "cancelled"
                        or (.state // "" | ascii_downcase) == "action_required"
                    )
                )
            ] | length
        ' <<<"$checks_json")"

        if ((required_failed > 0)); then
            emit_required_checks_diagnostics "$checks_json"
            echo "Required checks did not pass; refusing to enable auto-merge." >&2
            return 1
        fi

        if ((required_total > 0)) && ((required_pending == 0)); then
            stable_polls=$((stable_polls + 1))
            if ((stable_polls >= REQUIRED_STABLE_POLLS_REQUIRED)); then
                return 0
            fi
        else
            stable_polls=0
        fi

        remaining=$((REQUIRED_CHECKS_SETTLE_TIMEOUT_SECONDS - elapsed))
        if ((remaining <= 0)); then
            break
        fi

        sleep_for="$REQUIRED_CHECKS_SETTLE_POLL_INTERVAL_SECONDS"
        if ((sleep_for > remaining)); then
            sleep_for="$remaining"
        fi

        sleep "$sleep_for"
        elapsed=$((elapsed + sleep_for))
    done

    emit_required_checks_diagnostics "$checks_json"
    echo "Required checks did not settle within timeout; refusing to enable auto-merge." >&2
    return 1
}

wait_for_required_checks() {
    local elapsed=0
    local remaining
    local sleep_for
    local required_count
    local all_count

    while ((elapsed <= REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS)); do
        if is_stale_event; then
            echo "PR head moved while waiting for required checks; skipping stale auto-merge attempt."
            return 2
        fi

        required_count="$(required_checks_count)" || return 1
        if [[ "$required_count" == "$NO_REQUIRED_CHECKS_SENTINEL" ]]; then
            if is_stale_event; then
                echo "PR head moved after checks appeared; skipping stale auto-merge attempt."
                return 2
            fi
            all_count="$(all_checks_count)" || return 1
            if [[ "$all_count" =~ ^[0-9]+$ ]] && ((all_count > 0)); then
                echo "No required checks reported; waiting for non-self checks/statuses to settle before enabling auto-merge."
                wait_for_all_checks_without_required_metadata || return $?
                return 0
            fi
        elif [[ "$required_count" =~ ^[0-9]+$ ]] && ((required_count > 0)); then
            if is_stale_event; then
                echo "PR head moved after required checks appeared; skipping stale auto-merge attempt."
                return 2
            fi
            echo "Waiting for $required_count required checks to settle before enabling auto-merge."
            wait_for_required_checks_without_self || return $?
            return 0
        fi

        remaining=$((REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS - elapsed))
        if ((remaining <= 0)); then
            break
        fi

        sleep_for="$REQUIRED_CHECKS_POLL_INTERVAL_SECONDS"
        if ((sleep_for > remaining)); then
            sleep_for="$remaining"
        fi

        sleep "$sleep_for"
        elapsed=$((elapsed + sleep_for))
    done

    all_count="$(all_checks_count)" || return 1
    if [[ "$all_count" =~ ^[0-9]+$ ]] && ((all_count > 0)); then
        if is_stale_event; then
            echo "PR head moved after checks appeared; skipping stale auto-merge attempt."
            return 2
        fi
        echo "Required checks did not appear before timeout; waiting for fallback to non-self checks/statuses."
        wait_for_all_checks_without_required_metadata || return $?
        return 0
    fi

    echo "No checks detected for PR within timeout; refusing to enable auto-merge." >&2
    return 1
}

if [[ "$(get_pr_field '.state')" != "OPEN" ]]; then
    echo "PR is not open; skipping auto-merge enable."
    exit 0
fi

if [[ "$(get_pr_field '.isDraft')" == "true" ]]; then
    echo "PR is draft; skipping auto-merge enable."
    exit 0
fi

if is_auto_merge_enabled; then
    echo "Auto-merge already enabled."
    exit 0
fi

if is_stale_event; then
    echo "PR head moved since event; skipping stale auto-merge attempt."
    exit 0
fi

allow_squash_merge="$(gh api "repos/$GITHUB_REPOSITORY" --jq '.allow_squash_merge')"
allow_rebase_merge="$(gh api "repos/$GITHUB_REPOSITORY" --jq '.allow_rebase_merge')"
allow_merge_commit="$(gh api "repos/$GITHUB_REPOSITORY" --jq '.allow_merge_commit')"

if [[ "$allow_squash_merge" != "true" ]]; then
    echo "Repository policy does not allow squash merges; cannot enable Dependabot auto-merge safely." >&2
    exit 1
fi

if [[ "$allow_rebase_merge" == "true" || "$allow_merge_commit" == "true" ]]; then
    echo "Repository merge policy drift detected: Dependabot auto-merge expects squash-only settings." >&2
    exit 1
fi

if [[ "$DEPENDABOT_AUTOMERGE_ONE_SHOT" != "true" ]]; then
    if wait_for_required_checks; then
        wait_status=0
    else
        wait_status=$?
    fi
    if [[ "$wait_status" -eq 2 ]]; then
        exit 0
    fi
    if [[ "$wait_status" -ne 0 ]]; then
        exit 1
    fi
fi

if is_stale_event; then
    if [[ "$DEPENDABOT_AUTOMERGE_ONE_SHOT" == "true" ]]; then
        echo "PR head moved before one-shot auto-merge attempt; skipping stale auto-merge attempt."
    else
        echo "PR head moved after required checks completed; skipping stale auto-merge attempt."
    fi
    exit 0
fi

if attempt_automerge; then
    if [[ "$DEPENDABOT_AUTOMERGE_ONE_SHOT" == "true" ]]; then
        echo "Auto-merge enabled with squash strategy (one-shot)."
    else
        echo "Auto-merge enabled with squash strategy."
    fi
    exit 0
fi

if is_auto_merge_enabled; then
    echo "Auto-merge enabled by another run after squash attempt failure."
    exit 0
fi

if is_stale_event; then
    echo "PR head moved after squash attempt failure; skipping stale retry."
    exit 0
fi

echo "Failed to enable Dependabot auto-merge with squash strategy."
exit 1
