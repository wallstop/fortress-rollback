#!/bin/bash
# Promote one replay-confirmed simulation failure artifact into the corpus.

set -euo pipefail

usage() {
    echo "Usage: $0 <failure-artifact.json> <lowercase-slug>" >&2
}

if [ "$#" -ne 2 ]; then
    usage
    exit 2
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../.." && pwd)
case "$1" in
    /*) artifact=$1 ;;
    *) artifact=$(pwd)/$1 ;;
esac
slug=$2
cargo_bin=${FORTRESS_SIM_CARGO:-cargo}
corpus_root=${FORTRESS_SIM_CORPUS_ROOT:-$repo_root/tests/simulation/corpus}

case "$slug" in
    ''|*[!a-z0-9-]*|-*|*-|*--*)
        echo "slug must use lowercase letters, digits, and single interior hyphens: $slug" >&2
        exit 2
        ;;
esac
if [ ! -f "$artifact" ]; then
    echo "failure artifact does not exist: $artifact" >&2
    exit 2
fi

cd "$repo_root"
mkdir -p "$corpus_root"
temp_root=${RUNNER_TEMP:-$corpus_root}
mkdir -p "$temp_root"
route_output=$(mktemp "$temp_root/fortress-sim-route.XXXXXX")
rm -f "$route_output"
candidate=
lock=
cleanup() {
    rm -f "$route_output"
    if [ -n "$candidate" ]; then
        rm -f "$candidate"
    fi
    if [ -n "$lock" ]; then
        rmdir "$lock" 2>/dev/null || true
    fi
}
trap cleanup EXIT INT TERM

FORTRESS_SIM_CORPUS_ARTIFACT="$artifact" \
FORTRESS_SIM_CORPUS_MODE=classify \
FORTRESS_SIM_CORPUS_ROUTE_OUTPUT="$route_output" \
"$cargo_bin" nextest run \
    --test simulation \
    --features hot-join \
    --run-ignored ignored-only \
    -E 'test(simulation::corpus_replay::validate_and_extract_candidate_artifact)' \
    --no-capture

route=$(tr -d '\r\n' < "$route_output")
case "$route" in
    default)
        corpus_dir=$corpus_root
        feature_args=()
        ;;
    hot-join)
        corpus_dir=$corpus_root/hot-join
        feature_args=(--features hot-join)
        ;;
    *)
        echo "promotion helper returned unknown corpus route: $route" >&2
        exit 2
        ;;
esac
mkdir -p "$corpus_dir"
candidate=$(mktemp "$corpus_dir/.candidate.XXXXXX")
rm -f "$candidate"

FORTRESS_SIM_CORPUS_ARTIFACT="$artifact" \
FORTRESS_SIM_CORPUS_MODE=extract \
FORTRESS_SIM_CORPUS_OUTPUT="$candidate" \
"$cargo_bin" nextest run \
    --test simulation \
    "${feature_args[@]}" \
    --run-ignored ignored-only \
    -E 'test(simulation::corpus_replay::validate_and_extract_candidate_artifact)' \
    --no-capture

lock=$corpus_dir/.promotion.lock
if ! mkdir "$lock" 2>/dev/null; then
    echo "another corpus promotion is already in progress: $lock" >&2
    exit 2
fi

next=1
for path in "$corpus_dir"/[0-9][0-9][0-9]-*.json; do
    [ -e "$path" ] || continue
    name=${path##*/}
    number=${name%%-*}
    value=$((10#$number))
    if [ "$value" -ge "$next" ]; then
        next=$((value + 1))
    fi
done
if [ "$next" -gt 999 ]; then
    echo "corpus id space is full in $corpus_dir (maximum 999)" >&2
    exit 2
fi
destination=$(printf '%s/%03d-%s.json' "$corpus_dir" "$next" "$slug")
if [ -e "$destination" ]; then
    echo "refusing to overwrite existing corpus schedule: $destination" >&2
    exit 2
fi
# The candidate and destination share a filesystem. A hard link publishes the
# already-validated inode atomically and fails if even a non-cooperating writer
# won the destination race; removing the hidden name completes the rename-like
# handoff without a copy window.
if ! ln "$candidate" "$destination"; then
    echo "refusing to overwrite concurrently-created corpus schedule: $destination" >&2
    exit 2
fi
rm "$candidate"
candidate=
rmdir "$lock"
lock=
trap - EXIT INT TERM
echo "Promoted replay-confirmed reproducer to $destination"
