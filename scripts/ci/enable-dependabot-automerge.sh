#!/bin/bash
set -euo pipefail

: "${PR_URL:?PR_URL is required}"
: "${PR_HEAD_SHA:?PR_HEAD_SHA is required}"
: "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"

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
