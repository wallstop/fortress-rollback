#!/bin/bash
set -euo pipefail

: "${PR_URL:?PR_URL is required}"
: "${PR_HEAD_SHA:?PR_HEAD_SHA is required}"
: "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"

get_pr_field() {
    local jq_expr="$1"
    gh pr view "$PR_URL" --json state,isDraft,autoMergeRequest,headRefOid --jq "$jq_expr"
}

is_auto_merge_enabled() {
    [[ "$(get_pr_field '.autoMergeRequest != null')" == "true" ]]
}

is_stale_event() {
    [[ "$(get_pr_field '.headRefOid')" != "$PR_HEAD_SHA" ]]
}

attempt_automerge() {
    local strategy="${1:-}"
    local args=(pr merge --auto --match-head-commit "$PR_HEAD_SHA")
    if [[ -n "$strategy" ]]; then
        args+=("$strategy")
    fi
    args+=("$PR_URL")
    gh "${args[@]}"
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

if attempt_automerge; then
    echo "Auto-merge enabled without forcing merge strategy."
    exit 0
fi

if is_auto_merge_enabled; then
    echo "Auto-merge enabled by another run after initial failure."
    exit 0
fi

if is_stale_event; then
    echo "PR head moved after initial failure; skipping stale retry."
    exit 0
fi

allow_squash_merge="$(gh api "repos/$GITHUB_REPOSITORY" --jq '.allow_squash_merge')"
allow_rebase_merge="$(gh api "repos/$GITHUB_REPOSITORY" --jq '.allow_rebase_merge')"
allow_merge_commit="$(gh api "repos/$GITHUB_REPOSITORY" --jq '.allow_merge_commit')"

for strategy in --squash --rebase --merge; do
    if [[ "$strategy" == "--squash" && "$allow_squash_merge" != "true" ]]; then
        continue
    fi
    if [[ "$strategy" == "--rebase" && "$allow_rebase_merge" != "true" ]]; then
        continue
    fi
    if [[ "$strategy" == "--merge" && "$allow_merge_commit" != "true" ]]; then
        continue
    fi

    if attempt_automerge "$strategy"; then
        echo "Auto-merge enabled with fallback strategy $strategy."
        exit 0
    fi

    if is_auto_merge_enabled; then
        echo "Auto-merge enabled by another run during fallback."
        exit 0
    fi

    if is_stale_event; then
        echo "PR head moved during fallback; skipping stale retry."
        exit 0
    fi
done

echo "Failed to enable Dependabot auto-merge with all supported strategies."
exit 1
