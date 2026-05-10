#!/usr/bin/env bash
#
# Advisory check: flag report_violation! callsites with >= 2 format args
# inside files that contain Kani proofs. The cfg(kani) branch of
# report_violation! is a true no-op (see telemetry.rs), but excessive
# format args still pay non-trivial CBMC modeling cost during proof
# verification. This script does NOT block; it only surfaces findings.
#
# Always exits 0.

set -u

cd "$(dirname "$0")/../.."

echo "[kani-violation-cost] advisory scan: report_violation! callsites with"
echo "[kani-violation-cost] >= 2 format args in files containing #[kani::proof]."
echo "[kani-violation-cost] This check NEVER blocks; findings are advisory only."
echo

if ! command -v rg >/dev/null 2>&1; then
    echo "[kani-violation-cost] ripgrep (rg) not found; skipping advisory check."
    exit 0
fi

# Files where report_violation! is allowed to be expensive: the macro
# itself lives in src/telemetry.rs and must be skipped to avoid matching
# the macro_rules! definition.
SKIP_FILE='src/telemetry.rs'

# Find all .rs files under src/ that contain at least one #[kani::proof]
# attribute. These are "Kani-reachable" files for our heuristic.
kani_files_raw="$(rg -l --type rust 'kani::proof' src/ 2>/dev/null | sort -u)"

if [ -z "$kani_files_raw" ]; then
    echo "[kani-violation-cost] no Kani proof files found; nothing to check."
    exit 0
fi

# Read into a portable array (works on bash 3.2 / macOS).
kani_files=()
while IFS= read -r line; do
    [ -n "$line" ] && kani_files+=("$line")
done <<EOF
$kani_files_raw
EOF

findings=0

for file in "${kani_files[@]}"; do
    if [ "$file" = "$SKIP_FILE" ]; then
        continue
    fi

    # Enumerate callsite line numbers: every "report_violation!(" in the file.
    callsite_lines="$(rg -n 'report_violation!\(' "$file" 2>/dev/null | awk -F: '{print $1}')"
    [ -z "$callsite_lines" ] && continue

    while IFS= read -r line; do
        [ -z "$line" ] && continue

        # Read up to 30 lines starting at $line; find the matching ')'.
        # awk tracks paren depth ignoring parens inside double-quoted strings.
        invocation="$(awk -v start="$line" -v maxlines=30 '
            BEGIN { depth = 0; found = 0; in_str = 0; esc = 0; started = 0 }
            NR < start { next }
            NR >= start + maxlines { exit }
            {
                buf = buf $0 "\n"
                for (i = 1; i <= length($0); i++) {
                    ch = substr($0, i, 1)
                    if (esc) { esc = 0; continue }
                    if (in_str) {
                        if (ch == "\\") { esc = 1; continue }
                        if (ch == "\"") in_str = 0
                        continue
                    }
                    if (ch == "\"") { in_str = 1; continue }
                    if (ch == "(") { depth++; started = 1 }
                    else if (ch == ")") {
                        depth--
                        if (started && depth == 0) { found = 1 }
                    }
                }
                if (found) exit
            }
            END { if (found) print buf }
        ' "$file")"

        if [ -z "$invocation" ]; then
            continue
        fi

        # Count format args: total commas at outer-paren depth (depth == 1),
        # minus 2 (severity, kind). The fmt literal counts as the first format
        # arg-bearing position; if there are exactly 2 commas at depth 1, we
        # have 3 args (severity, kind, fmt) and 0 format args. So format-arg
        # count = commas_at_depth_1 - 2.
        nargs="$(printf '%s' "$invocation" | awk '
            BEGIN { depth = 0; commas = 0; in_str = 0; esc = 0 }
            {
                for (i = 1; i <= length($0); i++) {
                    ch = substr($0, i, 1)
                    if (esc) { esc = 0; continue }
                    if (in_str) {
                        if (ch == "\\") { esc = 1; continue }
                        if (ch == "\"") in_str = 0
                        continue
                    }
                    if (ch == "\"") { in_str = 1; continue }
                    if (ch == "(" || ch == "[" || ch == "{") depth++
                    else if (ch == ")" || ch == "]" || ch == "}") depth--
                    else if (ch == "," && depth == 1) commas++
                }
            }
            END { print commas - 2 }
        ')"

        # Guard against malformed parses.
        case "$nargs" in
            ''|*[!0-9-]*) continue ;;
        esac

        if [ "$nargs" -ge 2 ]; then
            echo "${file}:${line}: report_violation! has ${nargs} format arg(s) in Kani-reachable file (advisory)"
            findings=$((findings + 1))
        fi
    done <<EOF
$callsite_lines
EOF
done

echo
if [ "$findings" -eq 0 ]; then
    echo "[kani-violation-cost] no findings."
else
    echo "[kani-violation-cost] ${findings} advisory finding(s)."
    echo "Context: report_violation! is a no-op under cfg(kani) but format args still"
    echo "appear in CBMC analysis. See src/telemetry.rs (above macro_rules! report_violation)."
fi

exit 0
