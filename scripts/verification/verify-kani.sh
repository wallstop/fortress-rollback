#!/bin/bash
# Kani Formal Verification Script for Fortress Rollback
#
# Usage: ./scripts/verification/verify-kani.sh [options]
#   See --help for options.
#
# Prerequisites:
#   - Kani installed (script provides install instructions if missing)
#   - Rust nightly toolchain
#
# Environment Variables:
#   KANI_TIMEOUT       - Timeout per proof in seconds (default: 300)
#   KANI_UNWIND        - Default unwind bound (default: use Kani defaults)
#   KANI_MEM_FLOOR_MB  - Memory watchdog floor in MB (default: dynamic, see
#                        compute_mem_floor_mb). When MemAvailable drops below
#                        this, the running cargo kani/cbmc process group is
#                        killed and the harness is flagged as memory_exceeded.
#
# IMPORTANT: When adding new #[kani::proof] functions, you must also add them
# to the appropriate TIER*_PROOFS array below. Use ./scripts/verification/check-kani-coverage.sh
# to validate that all proofs are covered.

set -euo pipefail

# Reset IFS to the POSIX default so word-splitting on space-separated layout
# strings (used by print_partition / run_tier_proofs via `read -r`) is robust
# against parent shells that exported a non-default IFS. We additionally use
# `read -r ... <<<` rather than `set -- $layout` for the layout split, but
# resetting IFS keeps every other word-split site in this script predictable.
IFS=$' \t\n'

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
KANI_TIMEOUT="${KANI_TIMEOUT:-300}"
KANI_UNWIND="${KANI_UNWIND:-}"
# Memory watchdog floor in MB. Empty => compute_mem_floor_mb derives a
# dynamic default from /proc/meminfo MemTotal. See run_kani_with_mem_watchdog.
KANI_MEM_FLOOR_MB="${KANI_MEM_FLOOR_MB:-}"
# Poll interval (seconds) for the memory sampler. ~2s is frequent enough to
# catch a fast CBMC/CaDiCaL allocation spike before the OS OOM-killer fires,
# while being light enough that the sampler itself is negligible overhead.
KANI_MEM_POLL_SECONDS="${KANI_MEM_POLL_SECONDS:-2}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Track results
PASSED=0
FAILED=0
TOTAL=0

# PIDs of the in-flight memory-watchdog sampler and the cargo-kani child it
# guards (0 = none). The EXIT/signal trap below reaps BOTH so neither can
# outlive the script, even if the runner sends SIGTERM/SIGINT mid-proof (the
# requirement: clean up the background sampler on every exit path -- normal,
# timeout, signal). The normal path reaps them inline in run_kani_invocation;
# these traps are the belt-and-suspenders for the signal path.
MEM_WATCHDOG_SAMPLER_PID=0
MEM_WATCHDOG_CHILD_PID=0

cleanup_mem_watchdog_sampler() {
    if [[ "${MEM_WATCHDOG_SAMPLER_PID:-0}" -gt 0 ]]; then
        kill "$MEM_WATCHDOG_SAMPLER_PID" 2>/dev/null || true
        MEM_WATCHDOG_SAMPLER_PID=0
    fi
    # Kill the cargo-kani child's whole process group (it ran under setsid, so
    # PGID == child PID) so cbmc/cadical don't survive a script-level signal.
    # Fall back to the bare PID if the group form fails.
    if [[ "${MEM_WATCHDOG_CHILD_PID:-0}" -gt 0 ]]; then
        kill -- "-${MEM_WATCHDOG_CHILD_PID}" 2>/dev/null \
            || kill "$MEM_WATCHDOG_CHILD_PID" 2>/dev/null || true
        MEM_WATCHDOG_CHILD_PID=0
    fi
}
# EXIT covers normal/error returns; the signal traps re-raise after cleanup so
# the script's exit status still reflects the signal (preserving the existing
# 130/143-style propagation the workflow relies on).
trap 'cleanup_mem_watchdog_sampler' EXIT
trap 'cleanup_mem_watchdog_sampler; trap - INT; kill -INT $$' INT
trap 'cleanup_mem_watchdog_sampler; trap - TERM; kill -TERM $$' TERM

# Cross-platform timeout wrapper
# Uses GNU timeout if available, gtimeout on macOS, or runs without timeout as fallback.
#
# When the internal env var _KANI_TIMEOUT_FOREGROUND=1 is set (only the memory
# watchdog path sets it), pass GNU timeout's `--foreground` flag. Rationale:
# WITHOUT --foreground, GNU timeout puts the monitored command in its OWN new
# process group so it can signal that group on timeout. That defeats our
# watchdog, which runs cargo kani under setsid and kills the SETSID group --
# timeout's private sub-group (containing cbmc/cadical) would survive. With
# --foreground, timeout keeps the command in the existing (setsid) group, so a
# single `kill -- -PGID` reaps the whole tree. --foreground is GNU/gtimeout-
# only; the watchdog path already requires GNU coreutils (Linux + setsid), and
# default (non-watchdog) callers never set the flag, so behaviour is unchanged
# for them.
run_with_timeout() {
    local timeout_seconds="$1"
    shift

    local fg_flag=()
    [[ "${_KANI_TIMEOUT_FOREGROUND:-}" == "1" ]] && fg_flag=(--foreground)

    if [[ "${OSTYPE:-}" == msys* ]] || [[ "${OSTYPE:-}" == cygwin* ]] || [[ -n "${WINDIR:-}" ]]; then
        "$@"
    elif command -v timeout &>/dev/null; then
        timeout "${fg_flag[@]}" "$timeout_seconds" "$@"
    elif command -v gtimeout &>/dev/null; then
        gtimeout "${fg_flag[@]}" "$timeout_seconds" "$@"
    else
        "$@"
    fi
}

# Classify "non-success but no proof verdict" exit codes so we can give a
# specific diagnostic instead of a generic "Kani failed".
#
# Echoes one of:
#   memory_exceeded    - Our per-proof MEMORY WATCHDOG killed the cargo
#                        kani/cbmc process group because MemAvailable dropped
#                        below the floor (machine about to OOM). This takes
#                        PRECEDENCE over the raw exit code: a watchdog SIGKILL
#                        surfaces to the shell as exit 137, which is otherwise
#                        indistinguishable from an external SIGKILL. Callers
#                        pass watchdog_fired="true" (arg 2) when the sentinel
#                        was set so we attribute the kill to THIS harness's
#                        state-space explosion rather than to runner
#                        preemption.
#   per_proof_timeout  - GNU timeout(1) fired its own KANI_TIMEOUT (exit 124).
#                        The harness genuinely ran too long for our budget.
#   external_terminate - The runner sent SIGTERM/SIGKILL to the child
#                        (exit 143/137). This is *not* a per-proof timeout:
#                        it almost always means the enclosing job timeout
#                        fired, the workflow was cancelled, or (very common
#                        on GitHub-hosted runners) the runner was preempted.
#   ""                 - Not a timeout-class exit; treat as a normal failure.
#
# Args:
#   $1 - exit_code        (required)
#   $2 - watchdog_fired   (optional; "true" when the memory watchdog killed
#                          the process group for this harness). Defaults to
#                          empty so existing single-arg callers are unaffected.
classify_exit_code() {
    local exit_code="$1"
    local watchdog_fired="${2:-}"
    # The watchdog verdict overrides the raw exit code: the kill it issues
    # looks like a generic SIGKILL (137) to the shell, but its root cause is
    # a proof-tractability / state-space-explosion problem, NOT runner
    # preemption. Decide on the sentinel, not the ambiguous exit code.
    if [[ "$watchdog_fired" == "true" ]]; then
        echo "memory_exceeded"
        return 0
    fi
    case "$exit_code" in
        124) echo "per_proof_timeout" ;;
        137|143) echo "external_terminate" ;;
        *) echo "" ;;
    esac
}

# ----------------------------------------------------------------------------
# Per-proof MEMORY WATCHDOG
# ----------------------------------------------------------------------------
#
# Why this exists: a few Kani proofs drive CBMC/CaDiCaL into a state-space
# explosion that allocates RAM faster than GNU timeout's wall-clock budget can
# catch. On a 16 GB GitHub `ubuntu-latest` runner the OS OOM-killer then
# reaps the *runner agent itself*, producing the opaque "The runner has
# received a shutdown signal" with no clue which harness was at fault.
#
# The watchdog samples /proc/meminfo MemAvailable while cargo kani runs. If it
# drops below a floor (machine about to thrash/OOM), we SIGKILL the kani/cbmc
# *process group* -- not the runner -- and record that THIS harness blew the
# memory budget. That turns an unattributable runner death into a clean,
# per-proof "memory_exceeded" diagnostic naming the harness.
#
# Portability: Linux /proc only. On macOS/BSD (no /proc/meminfo) the watchdog
# no-ops and the script behaves exactly as before (plain cargo kani under
# run_with_timeout).

# Read a kB-valued field from /proc/meminfo (e.g. MemTotal, MemAvailable) and
# echo the integer kB. Echoes nothing and returns 1 if /proc/meminfo or the
# field is unavailable. POSIX-class regex only (no \d/\s PCRE escapes) so the
# shell-portability checker stays green.
read_meminfo_kb() {
    local field="$1"
    [[ -r /proc/meminfo ]] || return 1
    # Lines look like: "MemAvailable:   14436764 kB". Anchor on the field name
    # followed by ':' and grab the first run of digits. grep -E / sed -E with
    # POSIX classes only.
    local line
    line=$(grep -E "^${field}:" /proc/meminfo 2>/dev/null | head -1) || return 1
    [[ -n "$line" ]] || return 1
    local kb
    kb=$(echo "$line" | sed -E 's/^[^0-9]*([0-9][0-9]*).*/\1/')
    [[ "$kb" =~ ^[0-9]+$ ]] || return 1
    echo "$kb"
}

# Compute the memory floor (in MB) below which MemAvailable triggers a kill.
#
# Default formula: max(1024 MB, ~8% of MemTotal). Reasoning:
#   - Anchoring near runner capacity (8% headroom) means the watchdog fires
#     ONLY on a true explosion that is consuming essentially all of RAM, never
#     on a legitimately heavy proof that still leaves comfortable headroom.
#     On a 16 GB runner, 8% ~= 1.3 GB; on a 7 GB runner the 1024 MB floor
#     dominates so we still leave a safety margin on small machines.
#   - The 1024 MB absolute floor guards tiny machines where 8% would be too
#     small to react before the OOM-killer beats us.
# Operators can override via KANI_MEM_FLOOR_MB (e.g. lower it on a constrained
# runner, or raise it to be more aggressive). Echoes nothing + returns 1 when
# MemTotal is unavailable (the caller treats that as "watchdog disabled").
compute_mem_floor_mb() {
    # Explicit override always wins (validated as a positive integer).
    if [[ -n "${KANI_MEM_FLOOR_MB:-}" ]]; then
        if [[ "$KANI_MEM_FLOOR_MB" =~ ^[1-9][0-9]*$ ]]; then
            echo "$KANI_MEM_FLOOR_MB"
            return 0
        fi
        echo "Warning: KANI_MEM_FLOOR_MB='${KANI_MEM_FLOOR_MB}' is not a positive integer; using dynamic default." >&2
    fi

    local total_kb
    total_kb=$(read_meminfo_kb "MemTotal") || return 1
    local total_mb=$(( total_kb / 1024 ))
    # ~8% of MemTotal (integer math), clamped to a 1024 MB absolute minimum.
    local pct_mb=$(( total_mb * 8 / 100 ))
    if [[ "$pct_mb" -lt 1024 ]]; then
        echo 1024
    else
        echo "$pct_mb"
    fi
}

# Background sampler. Polls MemAvailable every KANI_MEM_POLL_SECONDS; when it
# drops below floor_mb, SIGKILLs the target process GROUP and records the
# breach into sentinel_file, then exits.
#
# Args:
#   $1 - target_pid   (the cargo-kani child PID. setsid made it a process
#                      group leader, so its PGID == its PID; we kill the GROUP
#                      via `kill -- -PID` but check LIVENESS via the bare PID.)
#   $2 - floor_mb     (trigger threshold, MB)
#   $3 - sentinel_file (written with a one-line breach record on trigger)
#
# Liveness is checked with `kill -0 PID` (the PID exists the instant `&`
# returns) rather than `kill -0 -- -PID` (the process GROUP only exists once
# setsid has actually called setsid(2) and become a group leader -- a brief
# window during which a group-existence check spuriously reports "finished"
# and the sampler would exit before ever sampling). The KILL still targets the
# whole group so cbmc/cadilcal children die too.
#
# Runs as a detached background job; the parent reaps it on every exit path
# (see run_kani_invocation's cleanup). Self-terminates once it has triggered
# or once the child exits. The sampler NEVER touches the runner: it only
# signals the target process group.
mem_watchdog_sampler() {
    local target_pid="$1"
    local floor_mb="$2"
    local sentinel_file="$3"

    local poll="${KANI_MEM_POLL_SECONDS:-2}"
    [[ "$poll" =~ ^[1-9][0-9]*$ ]] || poll=2

    local min_avail_kb=0
    while :; do
        # Stop once the cargo-kani child is gone (proof finished normally).
        # Check the bare PID, not the group: the group may not exist yet in
        # the first poll, but the PID always does.
        kill -0 "$target_pid" 2>/dev/null || return 0

        local avail_kb
        if avail_kb=$(read_meminfo_kb "MemAvailable"); then
            # Track the lowest MemAvailable seen, for the diagnostic.
            if [[ "$min_avail_kb" -eq 0 ]] || [[ "$avail_kb" -lt "$min_avail_kb" ]]; then
                min_avail_kb="$avail_kb"
            fi
            local avail_mb=$(( avail_kb / 1024 ))
            if [[ "$avail_mb" -lt "$floor_mb" ]]; then
                # Machine is about to OOM. Kill the whole kani/cbmc group so
                # the runner survives. SIGKILL: CBMC ignores gentler signals
                # under memory pressure and we cannot afford a grace window.
                {
                    echo "memory_exceeded floor_mb=${floor_mb} avail_mb_at_kill=${avail_mb} min_avail_mb=$(( min_avail_kb / 1024 ))"
                } > "$sentinel_file" 2>/dev/null || true
                # Kill the GROUP (negative PID) so cbmc/cadical children die
                # too. Fall back to killing the bare PID if the group form
                # fails (e.g. setsid not yet a group leader -- unlikely once
                # we're consuming this much RAM, but belt-and-suspenders).
                kill -KILL -- "-${target_pid}" 2>/dev/null \
                    || kill -KILL "$target_pid" 2>/dev/null || true
                return 0
            fi
        fi
        # Sleep between samples. If sleep is interrupted we just loop again.
        sleep "$poll" 2>/dev/null || true
    done
}

# Print the canonical "misconfigured matrix" error to stderr.
#
# Used by both print_partition() and run_tier_proofs() so the wording stays
# consistent and there's a single place to update when the diagnostic
# evolves. Callers pass tier/total/parts; we emit three lines and return.
emit_misconfigured_matrix_error() {
    local tier="$1"
    local total="$2"
    local parts="$3"
    local color_on="${4:-}"
    local color_off="${5:-}"

    echo -e "${color_on}Error: Tier $tier has $total proofs but --parts=$parts was requested.${color_off}" >&2
    echo -e "${color_on}This is a misconfigured CI matrix: shards beyond shard $total would be empty.${color_off}" >&2
    echo -e "${color_on}Reduce --parts to at most $total, or rebalance the matrix.${color_off}" >&2
}

# Compute a balanced 1-indexed shard layout.
#
# Given `total` items and `parts` shards, this prints a single line of the
# form:
#   "<size_p> <start_p> <end_p> <sizes_csv>"
# where:
#   - size_p:    number of items in shard P (0-indexed-exclusive end - start)
#   - start_p:   inclusive start index for shard P (0-indexed)
#   - end_p:     exclusive end index for shard P (0-indexed)
#   - sizes_csv: comma-separated list of per-shard sizes for all P in 1..parts
#
# Layout rules (balanced partition; sizes differ by at most 1):
#   quotient  = total / parts
#   remainder = total % parts
#   The first `remainder` shards each get (quotient + 1) items.
#   The remaining (parts - remainder) shards each get `quotient` items.
#
# Preconditions (enforced at runtime; failures abort with a clear message):
#   - total >= 0, parts >= 1, 1 <= part <= parts
#   - total >= parts (i.e., no shard is empty); callers MUST validate this
#     beforehand and emit a diagnostic. We still defensively check it here
#     so that even if main()'s validation regresses we fail loudly rather
#     than computing negative or empty indices.
compute_partition_layout() {
    local total="$1"
    local parts="$2"
    local part="$3"

    # Defensive validation. main() validates --part/--parts before reaching
    # here, but partition math with non-positive inputs silently produces
    # negative indices and out-of-bounds reads downstream; we'd rather fail
    # the function loudly than emit a corrupt layout.
    if ! [[ "$total" =~ ^[0-9]+$ ]]; then
        echo "compute_partition_layout: total must be a non-negative integer (got '$total')" >&2
        return 2
    fi
    if ! [[ "$parts" =~ ^[1-9][0-9]*$ ]]; then
        echo "compute_partition_layout: parts must be a positive integer (got '$parts')" >&2
        return 2
    fi
    if ! [[ "$part" =~ ^[1-9][0-9]*$ ]]; then
        echo "compute_partition_layout: part must be a positive integer (got '$part')" >&2
        return 2
    fi
    if [[ "$part" -gt "$parts" ]]; then
        echo "compute_partition_layout: part ($part) must be <= parts ($parts)" >&2
        return 2
    fi
    if [[ "$parts" -gt "$total" ]]; then
        echo "compute_partition_layout: parts ($parts) exceeds total ($total); trailing shards would be empty" >&2
        return 2
    fi

    local quotient=$(( total / parts ))
    local remainder=$(( total % parts ))

    local size_p
    local start_p
    if [[ "$part" -le "$remainder" ]]; then
        size_p=$(( quotient + 1 ))
        start_p=$(( (part - 1) * (quotient + 1) ))
    else
        size_p=$quotient
        start_p=$(( remainder * (quotient + 1) + (part - 1 - remainder) * quotient ))
    fi
    local end_p=$(( start_p + size_p ))

    # Build per-shard sizes for the diagnostic.
    local sizes_csv=""
    local p
    for ((p=1; p<=parts; p++)); do
        local s
        if [[ "$p" -le "$remainder" ]]; then
            s=$(( quotient + 1 ))
        else
            s=$quotient
        fi
        if [[ -z "$sizes_csv" ]]; then
            sizes_csv="$s"
        else
            sizes_csv="${sizes_csv},${s}"
        fi
    done

    echo "$size_p $start_p $end_p $sizes_csv"
}

# Tier definitions - proofs grouped by approximate runtime
# Tier 1: Fast proofs (<30s each) - simple property checks
TIER1_PROOFS=(
    # Core frame/handle proofs (src/lib.rs)
    "proof_frame_new_valid"
    "proof_frame_null_consistency"
    "proof_frame_to_option"
    "proof_frame_from_option"
    "proof_frame_ordering"
    # Varint proofs (src/rle.rs)
    "proof_varint_encoded_len_correct"
    "proof_varint_encode_single_byte"
    "proof_varint_encoded_len_no_overflow"
    "proof_varint_decode_empty_safe"
    # TimeSync proofs (src/time_sync.rs)
    "proof_window_index_in_bounds"
    "proof_sum_no_overflow"
    "proof_division_safe"
    "proof_window_size_minimum"
    "proof_default_valid"
    # Config proofs (src/sessions/config.rs)
    "proof_validate_accepts_valid_queue_lengths"
    "proof_validate_boundary_at_two"
    "proof_max_frame_delay_derivation"
    "proof_all_presets_valid"
    "proof_preset_values"
    "proof_preset_configs_valid"
    "proof_zero_window_size_corrected"
    "proof_negative_frame_safe"
    # Protocol state proofs (src/network/protocol/state.rs)
    "proof_state_index_domain"
    "proof_state_index_bijection"
    "proof_clone_correctness"
    "proof_protocol_state_partial_eq_symmetric"
    "proof_variants_distinct"
    "proof_exhaustive_match"
    "proof_shutdown_variant_index"
    "proof_initializing_variant_index"
    # Protocol event proofs (src/network/protocol/event.rs)
    "proof_event_partial_eq_symmetric"
    "proof_synchronizing_preserves_fields"
    "proof_synchronizing_boundary_values"
    "proof_network_interrupted_timeout_preserved"
    "proof_sync_timeout_elapsed_preserved"
    "proof_unit_variants_distinct"
    "proof_clone_synchronizing"
    "proof_clone_network_interrupted"
    "proof_clone_sync_timeout"
    "proof_clone_unit_variants"
    "proof_input_event_preserves_data"
    "proof_input_event_clone"
    "proof_synchronizing_inequality"
    "proof_network_interrupted_inequality"
    "proof_all_variants_distinct"
    # Protocol input_bytes proofs (src/network/protocol/input_bytes.rs)
    "proof_input_bytes_frame_preserved"
    "proof_input_bytes_stores_bytes"
    "proof_clone_preserves_frame"
    "proof_clone_preserves_bytes"
    "proof_clone_is_independent"
    "proof_null_frame_detection"
    "proof_divisibility_check"
    "proof_try_to_player_inputs_rejects_zero_players"
    "proof_try_to_player_inputs_rejects_non_divisible_lengths"
    "proof_empty_input_bytes_valid"
    "proof_extreme_frame_values"
    # SyncLayer construction preconditions (src/sync_layer/mod.rs)
    "proof_sync_layer_default_queue_length_valid"
    # Protocol mod.rs proofs (src/network/protocol/mod.rs)
    "proof_connection_status_default"
    "proof_connection_status_frame_preservation"
    "proof_connection_status_disconnected_flag"
    "proof_frame_null_detection"
    "proof_player_handle_preservation"
    "proof_player_handle_equality"
)

# Tier 2: Medium proofs (30s-2min each) - moderate complexity
TIER2_PROOFS=(
    # Core frame proofs (src/lib.rs)
    "proof_frame_add_small_safe"
    "proof_frame_sub_frames_correct"
    "proof_frame_ordering_consistent"
    "proof_frame_modulo_for_queue"
    "proof_frame_add_assign_consistent"
    "proof_frame_sub_assign_consistent"
    "proof_player_handle_validity"
    # Varint proofs (src/rle.rs)
    "proof_varint_decode_terminates"
    "proof_varint_decode_offset_safe"
    "proof_varint_roundtrip_small"
    "proof_varint_continuation_handling"
    # TimeSync proofs (src/time_sync.rs)
    "proof_advance_frame_safe"
    # Config proofs (src/sessions/config.rs)
    "proof_validate_frame_delay_constraint"
    "proof_max_frame_delay_is_valid_delay"
    # InputQueue proofs (src/input_queue/mod.rs)
    "proof_new_queue_valid"
    "proof_head_wraparound"
    "proof_queue_index_calculation"
    "proof_length_calculation_consistent"
    "proof_delay_decrease_after_input_rejected_no_mutation"
    "proof_freeze_add_input_noop_preserves_state"
    "proof_prediction_entry_at_first_missing_frame"
    # SyncLayer proofs (src/sync_layer/mod.rs)
    "proof_minimal_sync_layer_initial_state_valid_1p"
    "proof_minimal_sync_layer_initial_state_valid_2p"
    "proof_advance_frame_monotonic"
    "proof_saved_states_count"
    "proof_get_cell_validates_frame"
    "proof_saved_states_circular_index"
    "proof_freeze_player_rejects_invalid_handle"
    "proof_freeze_player_preserves_frame_state"
    # Protocol input_bytes proofs (src/network/protocol/input_bytes.rs)
    "proof_player_slice_bounds_valid"
    "proof_first_non_null_frame_selection"
    # Protocol mod.rs proofs (src/network/protocol/mod.rs)
    "proof_frame_addition_safe"
    "proof_frame_gap_safe"
    "proof_sync_counter_decrement_safe"
    "proof_sync_remaining_bounds"
    "proof_local_frame_advantage_bounds"
    "proof_remote_frame_advantage_from_i8"
    "proof_frame_advantage_null_guard"
)

# Tier 3: Slow proofs (>2min each) - complex state verification
TIER3_PROOFS=(
    "proof_index_wrapping_consistent"
    "proof_add_single_input_maintains_invariants"
    "proof_sequential_inputs_maintain_invariants"
    "proof_discard_maintains_invariants"
    "proof_frame_delay_maintains_invariants"
    "proof_frame_delay_increase_gap_fills_confirmed_inputs"
    "proof_frame_delay_decrease_rejected_no_mutation"
    "proof_freeze_add_input_no_mutation"
    "proof_confirmed_input_matches_delayed_add"
    "proof_mid_session_delay_increase_gap_fills_sequentially"
    "proof_non_sequential_rejected"
    "proof_reset_maintains_structure"
    "proof_confirmed_input_valid_index"
    "proof_multiple_advances_monotonic"
    "proof_save_maintains_inv8"
    "proof_load_frame_validates_bounds"
    "proof_load_frame_success_maintains_invariants"
    "proof_set_frame_delay_validates_handle"
    "proof_reset_prediction_preserves_frames"
    "proof_confirmed_frame_bounded"
    "proof_sparse_saving_respects_saved_frame"
    "proof_frozen_disconnected_inputs_match_confirmed_stream"
)

print_usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --list             List all Kani proof harnesses"
    echo "  --harness NAME     Run specific harness (can be repeated)"
    echo "  --quick            Run with reduced bounds for faster verification"
    echo "  --tier N           Run only tier N proofs (1=fast, 2=medium, 3=slow)"
    echo "  --part P           Run only part P of the tier (use with --parts)"
    echo "  --parts N          Split tier into N parts (use with --part)"
    echo "  --print-partition  Print the planned harness partition for the"
    echo "                     given --tier/--part/--parts (or --tier alone)"
    echo "                     and exit without invoking cargo kani"
    echo "  --verbose          Show detailed Kani output"
    echo "  --jobs N           Run N harnesses in parallel (default: 1)"
    echo "  --fail-fast        Stop immediately when any proof fails (useful for CI)"
    echo "  --help             Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  KANI_TIMEOUT       Timeout per proof in seconds (default: 300)"
    echo "  KANI_UNWIND        Default unwind bound (default: use Kani defaults)"
    echo "  KANI_MEM_FLOOR_MB  Memory watchdog floor in MB. When MemAvailable"
    echo "                     drops below this while a proof runs, the cargo"
    echo "                     kani/cbmc process group is killed and the proof"
    echo "                     is flagged 'memory_exceeded' (Linux /proc only;"
    echo "                     default: max(1024, ~8% of MemTotal))"
    echo ""
    echo "Constraints:"
    echo "  --list             cannot combine with --harness, --tier, --part,"
    echo "                     --parts, or --print-partition"
    echo "  --print-partition  requires exactly one --tier (multiple --tier"
    echo "                     values are rejected) and cannot combine with"
    echo "                     --harness"
    echo "  --harness          cannot combine with --tier, --part, or --parts"
    echo "  --part / --parts   must both be supplied or both omitted, and"
    echo "                     require --tier (a single --tier on the run"
    echo "                     path; sharding is per-tier)"
    echo ""
    echo "Tiers:"
    echo "  Tier 1: ${#TIER1_PROOFS[@]} fast proofs (<30s each)"
    echo "  Tier 2: ${#TIER2_PROOFS[@]} medium proofs (30s-2min each)"
    echo "  Tier 3: ${#TIER3_PROOFS[@]} slow proofs (>2min each)"
    echo ""
    echo "Examples:"
    echo "  $0                              # Run all proofs"
    echo "  $0 --tier 1                     # Run only fast proofs"
    echo "  $0 --tier 1 --tier 2            # Run fast and medium proofs"
    echo "  $0 --tier 2 --part 1 --parts 3  # Run first third of tier 2"
    echo "  $0 --harness proof_frame_new_valid  # Run single proof"
    echo "  $0 --quick --jobs 4             # Fast mode with parallel execution"
    echo "  $0 --quick --fail-fast          # CI mode: fast bounds, stop on first failure"
    echo "  $0 --list                       # List available proofs"
}

print_diagnostics() {
    echo -e "${BLUE}=== Environment Diagnostics ===${NC}"
    echo "  Working directory: $(pwd)"
    echo "  PATH entries (cargo-related):"
    echo "$PATH" | tr ':' '\n' | grep -E '(cargo|rust)' | while read -r p; do
        echo "    - $p"
    done
    echo "  Cargo location: $(command -v cargo 2>/dev/null || echo 'NOT FOUND')"
    echo "  Rustc version: $(rustc --version 2>/dev/null || echo 'NOT FOUND')"
    echo ""
}

check_kani() {
    # First check if cargo is available
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: cargo is not in PATH.${NC}"
        echo ""
        print_diagnostics
        echo "This usually means the Rust toolchain was not properly installed or"
        echo "the shell environment was not configured correctly."
        echo ""
        echo "If running in CI, ensure the 'Install Rust' step uses an action that"
        echo "properly exports environment variables (e.g., dtolnay/rust-toolchain)."
        exit 1
    fi

    if ! command -v cargo-kani &> /dev/null; then
        echo -e "${RED}Error: Kani is not installed.${NC}"
        echo ""
        print_diagnostics
        echo "To install Kani, run:"
        echo "  cargo install --locked kani-verifier"
        echo "  cargo kani setup"
        echo ""
        echo "For more information, see: https://model-checking.github.io/kani/install-guide.html"
        exit 1
    fi

    local kani_version
    kani_version=$(cargo kani --version 2>&1 | head -n 1 || echo "unknown")
    echo -e "${BLUE}Using Kani: $kani_version${NC}"
}

get_tier_proofs() {
    local tier=$1
    case "$tier" in
        1) printf '%s\n' "${TIER1_PROOFS[@]}" ;;
        2) printf '%s\n' "${TIER2_PROOFS[@]}" ;;
        3) printf '%s\n' "${TIER3_PROOFS[@]}" ;;
        *) echo "Invalid tier: $tier" >&2; return 1 ;;
    esac
}

list_harnesses() {
    echo "Kani Proof Harnesses in Fortress Rollback:"
    echo ""

    echo -e "${BLUE}Tier 1 - Fast proofs (${#TIER1_PROOFS[@]} proofs, <30s each):${NC}"
    for harness in "${TIER1_PROOFS[@]}"; do
        echo "  - $harness"
    done

    echo ""
    echo -e "${YELLOW}Tier 2 - Medium proofs (${#TIER2_PROOFS[@]} proofs, 30s-2min each):${NC}"
    for harness in "${TIER2_PROOFS[@]}"; do
        echo "  - $harness"
    done

    echo ""
    echo -e "${RED}Tier 3 - Slow proofs (${#TIER3_PROOFS[@]} proofs, >2min each):${NC}"
    for harness in "${TIER3_PROOFS[@]}"; do
        echo "  - $harness"
    done

    local total=$((${#TIER1_PROOFS[@]} + ${#TIER2_PROOFS[@]} + ${#TIER3_PROOFS[@]}))
    echo ""
    echo "Total: $total harnesses"

    # Also show any proofs not in tiers (discovered from source)
    cd "$PROJECT_ROOT"
    local all_proofs
    all_proofs=$(find src/ -name '*.rs' -exec grep -h '#\[kani::proof\]' -A 1 {} + 2>/dev/null | \
                grep -o 'fn [a-zA-Z_][a-zA-Z0-9_]*' | sed 's/^fn //' | sort -u || echo "")

    local uncategorized=()
    while IFS= read -r proof; do
        local found=false
        for t1 in "${TIER1_PROOFS[@]}"; do [[ "$proof" == "$t1" ]] && found=true && break; done
        if [[ "$found" == "false" ]]; then
            for t2 in "${TIER2_PROOFS[@]}"; do [[ "$proof" == "$t2" ]] && found=true && break; done
        fi
        if [[ "$found" == "false" ]]; then
            for t3 in "${TIER3_PROOFS[@]}"; do [[ "$proof" == "$t3" ]] && found=true && break; done
        fi
        if [[ "$found" == "false" ]]; then
            uncategorized+=("$proof")
        fi
    done <<< "$all_proofs"

    if [[ ${#uncategorized[@]} -gt 0 ]]; then
        echo ""
        echo -e "${YELLOW}Uncategorized proofs (will run with default tier):${NC}"
        for proof in "${uncategorized[@]}"; do
            echo "  - $proof"
        done
    fi
}

# Print the planned harness partition for the given tier (and optional
# part/parts) without invoking cargo kani. Used by CI introspection and
# by the partition tests.
#
# Output format on success:
#   PARTITION tier=<T> total=<N> parts=<M> sizes=<csv>
#   PART part=<P> size=<S> start=<START> end=<END_EXCL>
#   <harness 1>
#   <harness 2>
#   ...
#
# When --part/--parts are omitted, prints all harnesses in the tier.
print_partition() {
    local tier="$1"
    local part="$2"
    local parts="$3"

    local proofs
    case "$tier" in
        1) proofs=("${TIER1_PROOFS[@]}") ;;
        2) proofs=("${TIER2_PROOFS[@]}") ;;
        3) proofs=("${TIER3_PROOFS[@]}") ;;
        *) echo "Invalid tier: $tier" >&2; return 1 ;;
    esac

    local total_proofs=${#proofs[@]}

    if [[ "$part" -gt 0 ]] && [[ "$parts" -gt 0 ]]; then
        if [[ "$parts" -gt "$total_proofs" ]]; then
            emit_misconfigured_matrix_error "$tier" "$total_proofs" "$parts"
            return 1
        fi

        local layout
        # compute_partition_layout exits 2 on programmer-error inputs (after
        # printing the underlying diagnostic to stderr). We collapse that to 1
        # here: the user has nothing actionable to do with the distinction.
        if ! layout=$(compute_partition_layout "$total_proofs" "$parts" "$part"); then
            return 1
        fi
        local size_p start_idx end_idx sizes_csv
        # `read -r ... <<<` is IFS-local to the heredoc word and avoids the
        # `set -- $layout` form, which would re-split on whatever IFS the
        # parent shell exported. We also reset IFS at the top of the script.
        read -r size_p start_idx end_idx sizes_csv <<< "$layout"

        local last_idx=$(( end_idx - 1 ))
        # Pluralise "part(s)" so the diagnostic reads naturally for both
        # parts=1 (rare here -- most callers omit the flags entirely -- but
        # still possible) and parts>1.
        local parts_word="parts"
        [[ "$parts" -eq 1 ]] && parts_word="part"
        local proofs_word="proofs"
        [[ "$size_p" -eq 1 ]] && proofs_word="proof"
        # Human-first diagnostic line (matches the run-path format) so logs
        # read consistently between introspection and run modes.
        echo "Tier $tier ($total_proofs proofs) -> $parts $parts_word: [${sizes_csv}]; this is part $part -> $size_p $proofs_word, indices ${start_idx}..${last_idx}"
        echo "PARTITION tier=$tier total=$total_proofs parts=$parts sizes=$sizes_csv"
        echo "PART part=$part size=$size_p start=$start_idx end=$end_idx"

        local i
        for ((i=start_idx; i<end_idx; i++)); do
            echo "${proofs[$i]}"
        done
    else
        # Human-first diagnostic for the unsharded case too. Pluralise so
        # "1 part" reads grammatically (was "1 parts" historically).
        local proofs_word="proofs"
        [[ "$total_proofs" -eq 1 ]] && proofs_word="proof"
        echo "Tier $tier ($total_proofs proofs) -> 1 part: [${total_proofs}]; this is part 1 -> $total_proofs $proofs_word, indices 0..$(( total_proofs - 1 ))"
        echo "PARTITION tier=$tier total=$total_proofs parts=1 sizes=$total_proofs"
        echo "PART part=1 size=$total_proofs start=0 end=$total_proofs"
        local h
        for h in "${proofs[@]}"; do
            echo "$h"
        done
    fi
}

# Run a single cargo kani invocation under both the GNU timeout AND the
# per-proof memory watchdog, capturing output and cleaning up the sampler on
# every exit path.
#
# Args:
#   $1  - verbose       ("true" => also tee output to the terminal)
#   $2  - sentinel_file (the sampler writes a breach record here on trigger)
#   $3  - output_file   (combined stdout+stderr captured here)
#   $4+ - the cargo kani command + args
#
# Returns the exit code of the cargo kani invocation (124 on GNU timeout,
# 137 on a watchdog SIGKILL, etc.). Composability: the watchdog and GNU
# timeout coexist -- whichever fires first wins; the other becomes a no-op.
#
# Watchdog ENABLED requires Linux /proc/meminfo (so we can sample MemAvailable)
# AND setsid (so cargo kani + cbmc run in their own killable process group).
# When either is missing we fall back to plain run_with_timeout: identical
# behaviour to before this watchdog existed.
run_kani_invocation() {
    local verbose="$1"
    local sentinel_file="$2"
    local output_file="$3"
    shift 3
    # Remaining args ("$@") are the cargo kani command.

    local floor_mb=""
    floor_mb=$(compute_mem_floor_mb) || floor_mb=""

    # Watchdog requires both a memory floor (=> /proc/meminfo readable) and
    # setsid (=> we can put the child in its own process group). Missing
    # either => disabled, behave exactly as before.
    if [[ -z "$floor_mb" ]] || ! command -v setsid &>/dev/null; then
        local rc=0
        if [[ "$verbose" == "true" ]]; then
            run_with_timeout "$KANI_TIMEOUT" "$@" 2>&1 | tee "$output_file" || rc=$?
        else
            run_with_timeout "$KANI_TIMEOUT" "$@" > "$output_file" 2>&1 || rc=$?
        fi
        return "$rc"
    fi

    echo -e "${BLUE}Memory watchdog active: will kill this proof if MemAvailable drops below ${floor_mb} MB.${NC}"

    # Tell run_with_timeout (inside the setsid'd subshell) to pass GNU
    # timeout's --foreground so the monitored cargo/cbmc tree stays in the
    # SETSID process group instead of timeout spawning its own sub-group.
    # Without this, `kill -- -PGID` would miss timeout's private group and
    # leave cbmc/cadical running. Exported so the subshell inherits it.
    export _KANI_TIMEOUT_FOREGROUND=1

    # Launch cargo kani (under the GNU timeout) in a NEW process group via
    # setsid, so the sampler can kill the whole tree (cargo -> kani -> cbmc ->
    # cadical) with a single `kill -- -PGID`. setsid makes the child a process
    # group leader whose PGID equals its PID.
    local child_pid
    if [[ "$verbose" == "true" ]]; then
        # Tee to terminal AND capture. The pipeline runs inside the setsid'd
        # subshell so the whole group is killable. `set -o pipefail` inside
        # the subshell ensures the pipeline's exit status is cargo kani's, not
        # tee's (the subshell does NOT inherit the parent's `set` options).
        setsid bash -c 'set -o pipefail; run_kani_cmd_under_timeout "$@" 2>&1 | tee "$0"' \
            "$output_file" "$KANI_TIMEOUT" "$@" &
        child_pid=$!
    else
        setsid bash -c 'run_kani_cmd_under_timeout "$@" > "$0" 2>&1' \
            "$output_file" "$KANI_TIMEOUT" "$@" &
        child_pid=$!
    fi

    # Start the sampler against the child PID (setsid made it a process group
    # leader, so PGID == child_pid). The sampler checks liveness via the bare
    # PID and kills the whole group; it self-exits when the child disappears
    # or after it triggers.
    # Publish the child PID to the script-global BEFORE starting the sampler so
    # the EXIT/signal trap can reap the cargo-kani process group if the script
    # is killed mid-proof (e.g. runner SIGTERM). Cleared below after the inline
    # reap on the normal path.
    MEM_WATCHDOG_CHILD_PID=$child_pid

    mem_watchdog_sampler "$child_pid" "$floor_mb" "$sentinel_file" &
    local sampler_pid=$!
    # Likewise publish the sampler PID so the trap reaps it on a signal.
    MEM_WATCHDOG_SAMPLER_PID=$sampler_pid

    # Always reap the sampler, on every exit path (normal, timeout, watchdog
    # kill, or signal). Killing a sampler that already self-exited is a
    # harmless no-op.
    #
    # The `wait` is wrapped in a `{ ...; } 2>/dev/null` group so bash's
    # job-control "Killed" notification (printed when it reaps a child that
    # died from SIGKILL -- exactly the watchdog path) does not leak into the
    # captured log. The exit status (137 on a SIGKILL) is preserved.
    local rc=0
    { wait "$child_pid"; } 2>/dev/null || rc=$?
    kill "$sampler_pid" 2>/dev/null || true
    { wait "$sampler_pid"; } 2>/dev/null || true
    # Reaped inline; clear the globals so the EXIT trap has nothing to do. The
    # child group is already gone here (it either finished, timed out, or was
    # killed by the watchdog), so clearing CHILD_PID avoids a spurious kill of
    # a recycled PID by a later trap.
    MEM_WATCHDOG_SAMPLER_PID=0
    MEM_WATCHDOG_CHILD_PID=0
    # Scope the foreground flag to this invocation so a later fallback-path
    # call doesn't inherit it.
    unset _KANI_TIMEOUT_FOREGROUND

    return "$rc"
}

# Helper executed inside the setsid'd subshell: runs the cargo kani command
# under the GNU timeout. Exported (with run_with_timeout) so the `bash -c`
# subshell spawned by setsid can call it.
# Args: $1 = timeout seconds, $2+ = command.
run_kani_cmd_under_timeout() {
    local t="$1"
    shift
    run_with_timeout "$t" "$@"
}
# These two functions execute inside the `setsid bash -c ...` subshell, which
# does NOT inherit shell functions unless they are exported. Export both so
# the watchdog path works; harmless on the fallback path.
export -f run_with_timeout
export -f run_kani_cmd_under_timeout

run_kani() {
    local harness="${1:-}"
    local quick="${2:-false}"
    local verbose="${3:-false}"
    local jobs="${4:-1}"

    cd "$PROJECT_ROOT"

    echo -e "${BLUE}Running Kani verification...${NC}"
    echo ""

    # Build Kani command
    local kani_cmd=(cargo kani)

    # Add harness filter if specified
    if [[ -n "$harness" ]]; then
        kani_cmd+=(--harness "$harness")
    fi

    # Add unwind if set
    if [[ -n "$KANI_UNWIND" ]]; then
        kani_cmd+=(--default-unwind "$KANI_UNWIND")
    fi

    # Quick mode uses smaller bounds and enables optimizations
    if [[ "$quick" == "true" ]]; then
        kani_cmd+=(--default-unwind 8)
    fi

    # cargo kani's --jobs N parallelizes across harnesses passed in one
    # invocation; with --harness it's a no-op. main() pre-clamps jobs=1 in
    # the per-harness/per-tier paths, so reaching here with jobs>1 means the
    # user invoked the bare "verify all" path where it actually helps.
    # --jobs requires --output-format=terse in Kani 0.66.0+.
    if [[ "$jobs" -gt 1 ]]; then
        kani_cmd+=(--jobs "$jobs" --output-format terse)
    fi

    local start_time
    start_time=$(date +%s)

    local output_file
    output_file=$(mktemp)

    local exit_code=0
    # Disable colors in Kani output to ensure reliable parsing
    export NO_COLOR=1
    export TERM=dumb

    # Per-proof memory watchdog. On Linux (/proc/meminfo present + setsid
    # available) we run cargo kani in its own process group and a background
    # sampler kills that group if MemAvailable falls below the floor -- so a
    # state-space explosion fails THIS harness cleanly instead of OOM-killing
    # the runner. The sentinel file is written by the sampler on trigger.
    # On platforms without /proc/meminfo, run_kani_invocation transparently
    # falls back to plain run_with_timeout (watchdog disabled, no behaviour
    # change). Cleanup of the sampler happens inside run_kani_invocation on
    # every exit path.
    local mem_sentinel
    mem_sentinel=$(mktemp)
    : > "$mem_sentinel"

    if [[ "$verbose" == "true" ]]; then
        run_kani_invocation "true" "$mem_sentinel" "$output_file" "${kani_cmd[@]}" || exit_code=$?
    else
        run_kani_invocation "false" "$mem_sentinel" "$output_file" "${kani_cmd[@]}" || exit_code=$?
    fi

    # Did the memory watchdog fire for THIS harness? The sentinel is non-empty
    # only when the sampler killed the process group. This decision overrides
    # the raw exit code (a watchdog SIGKILL surfaces as 137, which is otherwise
    # indistinguishable from an external SIGKILL).
    local watchdog_fired="false"
    local watchdog_record=""
    if [[ -s "$mem_sentinel" ]]; then
        watchdog_fired="true"
        watchdog_record=$(head -1 "$mem_sentinel" 2>/dev/null || echo "")
    fi
    rm -f "$mem_sentinel"

    # Diagnose timeout-class exits. These look identical to "Kani failed"
    # without context, but they have very different root causes and remedies.
    # See classify_exit_code() above for the taxonomy.
    local exit_class
    exit_class=$(classify_exit_code "$exit_code" "$watchdog_fired")
    if [[ -n "$exit_class" ]]; then
        local timed_out_harness="${harness:-all}"
        case "$exit_class" in
            memory_exceeded)
                echo -e "${RED}MEMORY EXCEEDED: '$timed_out_harness' was killed by the per-proof memory watchdog (machine about to OOM).${NC}"
                echo "Command: ${kani_cmd[*]}"
                [[ -n "$watchdog_record" ]] && echo "Watchdog: ${watchdog_record}"
                echo "Hint: this is a *proof-tractability / state-space-explosion* problem -- NOT a CI flake."
                echo "  - A symbolic kani::any() value is almost certainly driving an unbounded"
                echo "    loop or data structure, blowing up CBMC/CaDiCaL memory."
                echo "  - Concretize the parameter that flows into the symbolic loop, or"
                echo "    lower #[kani::unwind(N)] / shrink symbolic input ranges."
                echo "  - See the remediation policy at src/sync_layer/mod.rs:2249-2270."
                echo "Without this watchdog the OS OOM-killer would have reaped the runner"
                echo "('The runner has received a shutdown signal') with no attribution."
                ;;
            per_proof_timeout)
                echo -e "${RED}PER-PROOF TIMEOUT: '$timed_out_harness' exceeded KANI_TIMEOUT=${KANI_TIMEOUT}s (exit 124, GNU timeout fired).${NC}"
                echo "Command: ${kani_cmd[*]}"
                echo "Hint: this is a *proof-tractability* problem -- not a CI flake."
                echo "  - Lower #[kani::unwind(N)] if the bound is too generous."
                echo "  - Split the harness or shrink symbolic input ranges."
                echo "  - Raise KANI_TIMEOUT only as a last resort."
                ;;
            external_terminate)
                echo -e "${RED}EXTERNAL TERMINATION: '$timed_out_harness' was killed by signal (exit ${exit_code}).${NC}"
                echo "Command: ${kani_cmd[*]}"
                echo "Hint: GNU timeout did NOT fire (that would be exit 124). Likely causes:"
                echo "  - GitHub-hosted runner preemption / spot reclaim."
                echo "  - Enclosing job-level 'timeout-minutes' fired in the workflow."
                echo "  - Workflow cancelled (push superseded, manual cancel, concurrency.cancel-in-progress)."
                echo "  - OOM kill (less common for Kani; would usually log earlier)."
                echo "This is a *CI infrastructure* failure mode, distinct from a proof timeout."
                echo "Re-running the job is the typical mitigation; if it recurs, investigate the runner."
                ;;
        esac
    fi

    local end_time
    end_time=$(date +%s)
    local duration=$((end_time - start_time))

    # Strip ANSI color codes from output before parsing
    # This is necessary because Kani may output colored text
    local clean_output
    clean_output=$(mktemp)
    sed 's/\x1b\[[0-9;]*m//g' "$output_file" > "$clean_output" 2>/dev/null || cp "$output_file" "$clean_output"

    # Parse results - look for summary line which has the format:
    # Complete - N successfully verified harnesses, M failures, T total.
    local summary_line
    summary_line=$(grep "Complete - " "$clean_output" 2>/dev/null | tail -1 || echo "")

    if [[ -n "$summary_line" ]]; then
        # Extract numbers using sed for better portability (grep -P not available everywhere)
        # Use tr -d to strip any whitespace/newlines from extracted values
        PASSED=$(echo "$summary_line" | sed -n 's/.*\([0-9][0-9]*\) successfully.*/\1/p' | head -1 | tr -d '[:space:]')
        FAILED=$(echo "$summary_line" | sed -n 's/.*\([0-9][0-9]*\) failure.*/\1/p' | head -1 | tr -d '[:space:]')
        TOTAL=$(echo "$summary_line" | sed -n 's/.*\([0-9][0-9]*\) total.*/\1/p' | head -1 | tr -d '[:space:]')
        # Default to 0 if extraction failed
        PASSED=${PASSED:-0}
        FAILED=${FAILED:-0}
        TOTAL=${TOTAL:-0}
    else
        # Fallback to counting individual results
        # Note: grep -c returns exit code 1 when no matches (but still outputs "0")
        # Using || VARNAME=0 pattern to handle this correctly
        # Use tr -d to strip any whitespace/newlines from grep output
        # Try multiple patterns to handle different Kani output formats
        PASSED=$(grep -c -E "VERIFICATION:- SUCCESSFUL|Verification succeeded" "$clean_output" 2>/dev/null | tr -d '[:space:]') || PASSED=0
        FAILED=$(grep -c -E "VERIFICATION:- FAILED|Verification failed|VERIFICATION RESULT.*FAILURE" "$clean_output" 2>/dev/null | tr -d '[:space:]') || FAILED=0
        # Ensure values are clean integers before arithmetic
        PASSED=${PASSED%%[^0-9]*}
        FAILED=${FAILED%%[^0-9]*}
        PASSED=${PASSED:-0}
        FAILED=${FAILED:-0}
        TOTAL=$((PASSED + FAILED))
    fi

    # If we got no results, something went wrong - diagnose the issue
    if [[ $TOTAL -eq 0 ]]; then
        if [[ $exit_code -ne 0 ]]; then
            # Kani failed with an error - show what went wrong
            echo -e "${RED}Kani exited with error code $exit_code${NC}"
            echo ""
            # Check for common errors
            if grep -q -i "error\[E" "$clean_output" 2>/dev/null; then
                echo "Compilation errors detected:"
                { grep -E "error\[E[0-9]+\]" "$clean_output" | head -5; } || true
                echo ""
                echo "Relevant compiler context:"
                { grep -E "error\[E[0-9]+\]|^[[:space:]]+-->|value moved here|value used here|move occurs|help:|consider cloning" "$clean_output" | head -40; } || true
                echo ""
                echo "Diagnostic hint: Kani compiles with cfg(kani). If normal cargo builds pass,"
                echo "check #[cfg(kani)] code paths and macros that evaluate arguments differently"
                echo "from production builds."
            elif grep -q "no harnesses\|No proof harness\|no proof harness" "$clean_output" 2>/dev/null; then
                echo -e "${RED}Error: Harness not found by Kani${NC}"
            elif grep -q "unsupported\|not supported" "$clean_output" 2>/dev/null; then
                echo "Unsupported feature detected:"
                { grep -i "unsupported\|not supported" "$clean_output" | head -3; } || true
            fi
            if [[ "$verbose" != "true" ]]; then
                echo ""
                echo "Last 30 lines of Kani output:"
                tail -30 "$clean_output"
                echo ""
                echo "Run with --verbose for full output"
            fi
        elif grep -q "Checking harness" "$clean_output" 2>/dev/null; then
            # Harness was found but no verification result detected - parsing issue
            echo -e "${YELLOW}Warning: Could not parse Kani output. Check output format.${NC}"
            if [[ "$verbose" != "true" ]]; then
                echo "Last 20 lines of Kani output:"
                tail -20 "$clean_output"
            fi
        else
            # No "Checking harness" found and exit code is 0 - very strange
            echo -e "${YELLOW}Warning: Kani completed but no harness was processed${NC}"
            if [[ "$verbose" != "true" ]]; then
                echo "Last 20 lines of Kani output:"
                tail -20 "$clean_output"
            fi
        fi
    fi

    # Show summary
    echo ""
    echo "=========================================="
    echo "Kani Verification Summary"
    echo "=========================================="
    echo "  Duration: ${duration}s"
    echo -e "  ${GREEN}Passed: $PASSED${NC}"
    echo -e "  ${RED}Failed: $FAILED${NC}"
    echo "  Total:  $TOTAL"
    echo "=========================================="

    if [[ $FAILED -gt 0 ]]; then
        echo ""
        echo -e "${RED}Failed proofs:${NC}"
        # Try both old and new Kani output formats
        grep -E -B 5 "VERIFICATION:- FAILED|Verification failed" "$clean_output" | grep -E "Checking harness|harness" || true

        if [[ "$verbose" != "true" ]]; then
            echo ""
            echo "Run with --verbose for detailed output"
        fi
    fi

    rm -f "$output_file" "$clean_output"

    # Propagate timeout/signal exits verbatim so the workflow's
    # nick-fields/retry@v4 retry_on_exit_code can match (e.g. 143).
    # Real proof failures collapse to 1.
    if [[ $exit_code -ne 0 ]] || [[ $FAILED -gt 0 ]]; then
        case "$exit_code" in
            124|137|143) return "$exit_code" ;;
            *)           return 1 ;;
        esac
    fi

    return 0
}

run_tier_proofs() {
    local tier=$1
    local quick=$2
    local verbose=$3
    local jobs=$4
    local fail_fast=$5
    local part=${6:-0}
    local parts=${7:-0}

    local proofs
    case "$tier" in
        1) proofs=("${TIER1_PROOFS[@]}") ;;
        2) proofs=("${TIER2_PROOFS[@]}") ;;
        3) proofs=("${TIER3_PROOFS[@]}") ;;
        *) echo "Invalid tier: $tier" >&2; return 1 ;;
    esac

    # If part/parts specified, select subset of proofs using a balanced
    # partition (sizes differ by at most 1). The previous ceiling-division
    # algorithm could leave the trailing shard empty (e.g., 16 proofs across
    # 5 parts: 4,4,4,4,0) or unevenly distributed (37 across 6 parts:
    # 7,7,7,7,7,2). Balanced layout: 16/5 -> 4,3,3,3,3 and 37/6 -> 7,6,6,6,6,6.
    if [[ "$part" -gt 0 ]] && [[ "$parts" -gt 0 ]]; then
        local total_proofs=${#proofs[@]}

        # Refuse misconfigured matrices (more shards than items). The trailing
        # shards would be empty under any partitioning scheme; better to fail
        # loudly than to silently waste a CI runner.
        if [[ "$parts" -gt "$total_proofs" ]]; then
            emit_misconfigured_matrix_error "$tier" "$total_proofs" "$parts" "$RED" "$NC"
            return 1
        fi

        local layout
        # See print_partition() for why we collapse compute_partition_layout's
        # exit 2 (programmer error) into 1 (generic failure): the user has
        # nothing actionable to do with the distinction.
        if ! layout=$(compute_partition_layout "$total_proofs" "$parts" "$part"); then
            return 1
        fi
        local size_p start_idx end_idx sizes_csv
        # `read -r ... <<<` avoids `set -- $layout`, which would re-split on
        # whatever IFS the parent shell exported. Belt-and-suspenders: we
        # also reset IFS at the top of the script.
        read -r size_p start_idx end_idx sizes_csv <<< "$layout"

        # One-time diagnostic: print the planned layout so future sharding
        # changes are verifiable from CI logs. Pluralise "part(s)" / "proof(s)"
        # so single-shard / single-proof cases read naturally.
        local last_idx=$(( end_idx - 1 ))
        local parts_word="parts"
        [[ "$parts" -eq 1 ]] && parts_word="part"
        local proofs_word="proofs"
        [[ "$size_p" -eq 1 ]] && proofs_word="proof"
        echo -e "${BLUE}Tier $tier ($total_proofs proofs) -> $parts $parts_word: [${sizes_csv}]; this is part $part -> $size_p $proofs_word, indices ${start_idx}..${last_idx}${NC}"

        local selected_proofs=()
        local i
        for ((i=start_idx; i<end_idx; i++)); do
            selected_proofs+=("${proofs[$i]}")
        done
        proofs=("${selected_proofs[@]}")

        echo -e "${BLUE}Running Tier $tier Part $part/$parts proofs (${#proofs[@]} harnesses)...${NC}"
    else
        echo -e "${BLUE}Running Tier $tier proofs (${#proofs[@]} harnesses)...${NC}"
    fi

    local any_failed=false
    local tier_passed=0
    local tier_failed=0
    # Highest signal-class exit (124/137/143) seen in this tier; propagated
    # so the workflow's retry_on_exit_code can fire on runner preemption.
    local last_signal_exit=0

    for harness in "${proofs[@]}"; do
        echo -e "${BLUE}  Verifying: $harness${NC}"
        local rk_exit=0
        if run_kani "$harness" "$quick" "$verbose" "$jobs"; then
            ((tier_passed++))
        else
            rk_exit=$?
            any_failed=true
            ((tier_failed++))
            case "$rk_exit" in
                124|137|143) last_signal_exit=$rk_exit ;;
            esac
            if [[ "$quick" == "true" ]]; then
                echo -e "${YELLOW}Note: Running in --quick mode (--default-unwind 8). Proofs iterating over structures with >8 elements need explicit #[kani::unwind(N)].${NC}"
            fi
            if [[ "$fail_fast" == "true" ]]; then
                echo -e "${RED}Stopping early due to --fail-fast${NC}"
                echo ""
                echo "Tier $tier Results: $tier_passed passed, $tier_failed failed (stopped early)"
                [[ $last_signal_exit -ne 0 ]] && return "$last_signal_exit"
                return 1
            fi
        fi
    done

    echo ""
    echo "Tier $tier Results: $tier_passed passed, $tier_failed failed"

    if [[ "$any_failed" == "true" ]]; then
        [[ $last_signal_exit -ne 0 ]] && return "$last_signal_exit"
        return 1
    fi
    return 0
}

main() {
    local quick=false
    local verbose=false
    local list_only=false
    local print_partition_only=false
    local fail_fast=false
    local harnesses=()
    local tiers=()
    local jobs=1
    local part=0
    local parts=0
    # Track whether --part / --parts were *passed* (vs. defaulting to 0).
    # This separates "is integer" from "is positive" from "both/neither
    # supplied" so the user-facing diagnostic always matches what they did
    # wrong (e.g. `--part 0` is rejected as "must be positive", not the
    # misleading "--parts requires --part").
    local part_given=false
    local parts_given=false
    # Track which "ignored under --print-partition" flags were actually
    # passed, so we only emit the introspection-mode note when there's a
    # real mismatch between user intent and what the mode does.
    local quick_given=false
    local verbose_given=false
    local jobs_given=false
    local fail_fast_given=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --quick)
                quick=true
                quick_given=true
                shift
                ;;
            --verbose)
                verbose=true
                verbose_given=true
                shift
                ;;
            --list)
                list_only=true
                shift
                ;;
            --print-partition)
                print_partition_only=true
                shift
                ;;
            --fail-fast)
                fail_fast=true
                fail_fast_given=true
                shift
                ;;
            --harness)
                if [[ $# -lt 2 ]]; then
                    echo >&2 "Error: --harness requires an argument"
                    exit 1
                fi
                harnesses+=("$2")
                shift 2
                ;;
            --tier)
                if [[ $# -lt 2 ]]; then
                    echo >&2 "Error: --tier requires an argument (1, 2, or 3)"
                    exit 1
                fi
                if [[ "$2" =~ ^[1-3]$ ]]; then
                    tiers+=("$2")
                else
                    echo >&2 "Error: --tier must be 1, 2, or 3"
                    exit 1
                fi
                shift 2
                ;;
            --part)
                if [[ $# -lt 2 ]]; then
                    echo "Error: --part requires a number" >&2
                    exit 1
                fi
                if ! [[ "$2" =~ ^-?[0-9]+$ ]]; then
                    echo "Error: --part must be a positive integer (got '$2')" >&2
                    exit 1
                fi
                part="$2"
                part_given=true
                shift 2
                ;;
            --parts)
                if [[ $# -lt 2 ]]; then
                    echo "Error: --parts requires a number" >&2
                    exit 1
                fi
                if ! [[ "$2" =~ ^-?[0-9]+$ ]]; then
                    echo "Error: --parts must be a positive integer (got '$2')" >&2
                    exit 1
                fi
                parts="$2"
                parts_given=true
                shift 2
                ;;
            --jobs)
                if [[ $# -lt 2 ]]; then
                    echo "Error: --jobs requires a number" >&2
                    exit 1
                fi
                # Same positive-integer gate as --part/--parts. The previous
                # behaviour ("--jobs abc" -> arithmetic on a non-numeric ->
                # "unbound variable" or noisy bash error) is hostile; reject
                # at parse time with a matching diagnostic.
                if ! [[ "$2" =~ ^[1-9][0-9]*$ ]]; then
                    echo "Error: --jobs must be a positive integer (got '$2')" >&2
                    exit 1
                fi
                jobs="$2"
                jobs_given=true
                shift 2
                ;;
            --help|-h)
                print_usage
                exit 0
                ;;
            -*)
                echo >&2 "Unknown option: $1"
                print_usage
                exit 1
                ;;
            *)
                # Treat as harness name
                harnesses+=("$1")
                shift
                ;;
        esac
    done

    # Validate --part / --parts.
    #
    # The validation is ordered so each diagnostic matches the exact thing
    # the user did wrong. Earlier versions of this block ran presence
    # pairing before positivity, which produced misleading messages
    # ("--part requires --parts") for standalone bad values like `--part 0`
    # whose REAL bug is "0 is not positive". The corrected order is:
    #
    #   1. If neither flag was given, sharding is off -- nothing to check.
    #   2. Per-flag positivity FIRST (only checked when the flag was
    #      passed, so a missing flag doesn't mask a present-but-bad one).
    #      This is what makes `--part 0` (alone) report "must be a positive
    #      integer" instead of "requires --parts".
    #   3. Presence pairing (both or neither must be given).
    #   4. Range: 1 <= part <= parts.
    if [[ "$part_given" == "true" ]] || [[ "$parts_given" == "true" ]]; then
        # Step 2a: positivity for --part, only when --part was passed.
        if [[ "$part_given" == "true" ]] && ! [[ "$part" =~ ^[1-9][0-9]*$ ]]; then
            echo "Error: --part must be a positive integer (got '$part')" >&2
            exit 1
        fi
        # Step 2b: positivity for --parts, only when --parts was passed.
        if [[ "$parts_given" == "true" ]] && ! [[ "$parts" =~ ^[1-9][0-9]*$ ]]; then
            echo "Error: --parts must be a positive integer (got '$parts')" >&2
            exit 1
        fi
        # Step 3: presence pairing.
        if [[ "$part_given" != "true" ]]; then
            echo "Error: --parts requires --part to be specified" >&2
            exit 1
        fi
        if [[ "$parts_given" != "true" ]]; then
            echo "Error: --part requires --parts to be specified" >&2
            exit 1
        fi
        # Step 4: range.
        if [[ "$part" -gt "$parts" ]]; then
            echo "Error: --part ($part) cannot be greater than --parts ($parts)" >&2
            exit 1
        fi
    fi

    # Mutually-exclusive run/no-run modes.
    #
    # `--list`, `--print-partition`, `--harness`, and `--tier`+sharding
    # don't naturally compose:
    #   * `--list` is a "list everything in every tier" mode; pairing it
    #     with --harness/--tier/--part/--parts/--print-partition silently
    #     ignores those flags and emits the unfiltered listing, which is
    #     surprising (caller probably expected a filtered list).
    #   * `--print-partition` is a per-tier introspection mode; pairing it
    #     with --list or --harness silently shadowed --list/--harness in
    #     earlier revisions. Reject explicitly.
    if [[ "$list_only" == "true" ]]; then
        if [[ "$print_partition_only" == "true" ]]; then
            echo "Error: --list and --print-partition are mutually exclusive" >&2
            exit 1
        fi
        if [[ ${#harnesses[@]} -gt 0 ]]; then
            echo "Error: --list cannot be combined with --harness" >&2
            exit 1
        fi
        if [[ ${#tiers[@]} -gt 0 ]]; then
            echo "Error: --list cannot be combined with --tier" >&2
            exit 1
        fi
        if [[ "$part_given" == "true" ]] || [[ "$parts_given" == "true" ]]; then
            echo "Error: --list cannot be combined with --part or --parts" >&2
            exit 1
        fi
    fi
    if [[ "$print_partition_only" == "true" ]] && [[ ${#harnesses[@]} -gt 0 ]]; then
        echo "Error: --print-partition and --harness are mutually exclusive" >&2
        exit 1
    fi

    # `--harness` selects specific proofs by name; `--tier` selects them by
    # tier; `--part`/`--parts` shard within a tier. Combining `--harness`
    # with any of these is ambiguous -- the previous behaviour silently
    # ignored --tier/--part/--parts when --harness was passed. Reject
    # explicitly so the user sees the error immediately. (The script's
    # default, with neither --harness nor --tier, runs every harness; if
    # the user wants a tier, drop --harness; if they want one harness, drop
    # --tier/--part/--parts.)
    if [[ ${#harnesses[@]} -gt 0 ]]; then
        if [[ ${#tiers[@]} -gt 0 ]]; then
            echo "Error: --harness cannot be combined with --tier" >&2
            exit 1
        fi
        if [[ "$part_given" == "true" ]] || [[ "$parts_given" == "true" ]]; then
            echo "Error: --harness cannot be combined with --part or --parts" >&2
            exit 1
        fi
    fi

    # Run-path multi-tier with --part/--parts is ambiguous. The introspection
    # mode (--print-partition) already rejects multiple --tier values lower
    # down with its own dedicated message; this check only fires on the run
    # path. The previous behaviour silently applied the same --part/--parts
    # to BOTH tiers, producing e.g. "part 3 of 5 of tier 2 AND part 3 of 5
    # of tier 3" -- a layout the user almost certainly didn't ask for.
    # Reject with a message that points at the obvious workaround (one
    # invocation per tier).
    if [[ "$print_partition_only" != "true" ]] \
        && [[ ${#tiers[@]} -gt 1 ]] \
        && { [[ "$part_given" == "true" ]] || [[ "$parts_given" == "true" ]]; }; then
        echo "Error: --part/--parts cannot be combined with multiple --tier values; invoke once per tier" >&2
        exit 1
    fi

    # Sharding is per-tier; --part/--parts without --tier has no tier to
    # shard within. Reject explicitly rather than silently doing nothing
    # useful (the bare "verify all" path ignored --part/--parts). Skip
    # under --print-partition: that mode requires --tier explicitly via
    # its own check below, so this check would only fire when
    # --print-partition is also missing --tier -- in which case the
    # introspection-mode error is the more relevant message.
    if [[ "$print_partition_only" != "true" ]] \
        && { [[ "$part_given" == "true" ]] || [[ "$parts_given" == "true" ]]; } \
        && [[ ${#tiers[@]} -eq 0 ]] \
        && [[ ${#harnesses[@]} -eq 0 ]]; then
        echo "Error: --part/--parts require --tier" >&2
        exit 1
    fi

    # cargo kani's --jobs parallelizes across multiple harnesses in one
    # invocation. --harness/--tier paths fan out per-harness, so --jobs N
    # is a no-op there. Warn once and clamp; the bare "verify all" path
    # below (no harnesses, no tiers) keeps --jobs intact.
    #
    # Skip the clamp warning entirely under --print-partition: introspection
    # mode never invokes cargo kani, so the "ignored per-harness" note is
    # misleading. The introspection-mode note emitted below already covers
    # --jobs (alongside --quick/--verbose/--fail-fast).
    if [[ "$print_partition_only" != "true" ]] \
        && [[ "$jobs" -gt 1 ]] \
        && { [[ ${#harnesses[@]} -gt 0 ]] || [[ ${#tiers[@]} -gt 0 ]]; }; then
        echo -e "${YELLOW}[note] --jobs $jobs is ignored: this script invokes cargo kani per-harness; cargo kani's --jobs parallelizes across harnesses passed in a single invocation${NC}" >&2
        jobs=1
    fi

    # --print-partition is a pure introspection mode: it prints the planned
    # harness layout for the given --tier (with optional --part/--parts) and
    # exits. It must NOT depend on Kani being installed -- tests and CI
    # introspection both rely on this.
    #
    # Multi-tier --print-partition is rejected: emitting two unlabelled
    # PARTITION/PART blocks back-to-back is ambiguous for downstream
    # consumers (CI parsers, the partition tests). The user can always
    # invoke the script once per tier when they want both.
    if [[ "$print_partition_only" == "true" ]]; then
        if [[ ${#tiers[@]} -eq 0 ]]; then
            echo "Error: --print-partition requires --tier" >&2
            exit 1
        fi
        if [[ ${#tiers[@]} -gt 1 ]]; then
            echo "Error: --print-partition does not support multiple --tier values; invoke once per tier" >&2
            exit 1
        fi
        # Introspection mode: --print-partition prints the planned layout
        # and exits without invoking cargo kani, so flags that only matter
        # for the run path (--quick, --verbose, --jobs, --fail-fast) have
        # no effect. Surface that explicitly when any of them was passed
        # so the caller doesn't infer they took effect.
        if [[ "$quick_given" == "true" ]] || [[ "$verbose_given" == "true" ]] \
            || [[ "$jobs_given" == "true" ]] || [[ "$fail_fast_given" == "true" ]]; then
            echo "[note] introspection mode; --quick/--verbose/--jobs/--fail-fast ignored" >&2
        fi
        print_partition "${tiers[0]}" "$part" "$parts" || exit $?
        exit 0
    fi

    echo "=========================================="
    echo "Fortress Rollback Kani Verification"
    echo "=========================================="

    check_kani
    echo ""

    if [[ "$list_only" == "true" ]]; then
        # Introspection mode: --list prints the harness registry and exits
        # without invoking cargo kani, so flags that only matter for the
        # run path (--quick, --verbose, --jobs, --fail-fast) have no
        # effect. Mirror the --print-partition note so callers don't
        # infer those flags took effect.
        if [[ "$quick_given" == "true" ]] || [[ "$verbose_given" == "true" ]] \
            || [[ "$jobs_given" == "true" ]] || [[ "$fail_fast_given" == "true" ]]; then
            echo "[note] introspection mode; --quick/--verbose/--jobs/--fail-fast ignored" >&2
        fi
        list_harnesses
        exit 0
    fi

    # Run verification
    local any_failed=false
    # Propagate signal-class exits (124/137/143) verbatim so the workflow's
    # nick-fields/retry@v4 retry_on_exit_code matches.
    local signal_exit=0

    if [[ ${#harnesses[@]} -gt 0 ]]; then
        # Run specific harnesses
        for harness in "${harnesses[@]}"; do
            echo -e "${BLUE}Verifying harness: $harness${NC}"
            local rk_exit=0
            run_kani "$harness" "$quick" "$verbose" "$jobs" || rk_exit=$?
            if [[ $rk_exit -ne 0 ]]; then
                any_failed=true
                case "$rk_exit" in
                    124|137|143) signal_exit=$rk_exit ;;
                esac
                if [[ "$quick" == "true" ]]; then
                    echo -e "${YELLOW}Note: Running in --quick mode (--default-unwind 8). Proofs iterating over structures with >8 elements need explicit #[kani::unwind(N)].${NC}"
                fi
                if [[ "$fail_fast" == "true" ]]; then
                    echo -e "${RED}Stopping early due to --fail-fast${NC}"
                    break
                fi
            fi
        done
    elif [[ ${#tiers[@]} -gt 0 ]]; then
        # Run specific tiers
        for tier in "${tiers[@]}"; do
            local rt_exit=0
            run_tier_proofs "$tier" "$quick" "$verbose" "$jobs" "$fail_fast" "$part" "$parts" || rt_exit=$?
            if [[ $rt_exit -ne 0 ]]; then
                any_failed=true
                case "$rt_exit" in
                    124|137|143) signal_exit=$rt_exit ;;
                esac
                if [[ "$fail_fast" == "true" ]]; then
                    break
                fi
            fi
        done
    else
        # Run all harnesses (default)
        local rk_exit=0
        run_kani "" "$quick" "$verbose" "$jobs" || rk_exit=$?
        if [[ $rk_exit -ne 0 ]]; then
            any_failed=true
            case "$rk_exit" in
                124|137|143) signal_exit=$rk_exit ;;
            esac
        fi
    fi

    if [[ "$any_failed" == "true" ]]; then
        [[ $signal_exit -ne 0 ]] && exit "$signal_exit"
        exit 1
    fi

    echo ""
    echo -e "${GREEN}All Kani proofs passed!${NC}"
}

main "$@"
