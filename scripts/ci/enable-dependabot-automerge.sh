#!/bin/bash
set -euo pipefail

: "${PR_URL:?PR_URL is required}"
: "${PR_HEAD_SHA:?PR_HEAD_SHA is required}"
: "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"

REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS="${REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS:-120}"
REQUIRED_CHECKS_POLL_INTERVAL_SECONDS="${REQUIRED_CHECKS_POLL_INTERVAL_SECONDS:-10}"
REQUIRED_CHECKS_WATCH_INTERVAL_SECONDS="${REQUIRED_CHECKS_WATCH_INTERVAL_SECONDS:-10}"

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
    gh pr checks "$PR_URL" --required --json name --jq 'length'
}

wait_for_required_checks() {
    local elapsed=0
    local remaining
    local sleep_for
    local required_count

    while ((elapsed <= REQUIRED_CHECKS_APPEAR_TIMEOUT_SECONDS)); do
        if is_stale_event; then
            echo "PR head moved while waiting for required checks; skipping stale auto-merge attempt."
            return 2
        fi

        required_count="$(required_checks_count)"
        if [[ "$required_count" =~ ^[0-9]+$ ]] && ((required_count > 0)); then
            if is_stale_event; then
                echo "PR head moved after required checks appeared; skipping stale auto-merge attempt."
                return 2
            fi
            echo "Waiting for $required_count required checks to pass before enabling auto-merge."
            if ! gh pr checks "$PR_URL" --required --watch --fail-fast --interval "$REQUIRED_CHECKS_WATCH_INTERVAL_SECONDS"; then
                echo "Required checks did not pass; refusing to enable auto-merge." >&2
                return 1
            fi
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

    echo "No required checks detected for PR within timeout; refusing to enable auto-merge." >&2
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

if is_stale_event; then
    echo "PR head moved after required checks completed; skipping stale auto-merge attempt."
    exit 0
fi

if attempt_automerge; then
    echo "Auto-merge enabled with squash strategy."
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
