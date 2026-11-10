#!/usr/bin/env bash
# Deterministic regeneration, stale-output rejection, axiom audit, and optional
# cost measurement for the #365 varint Lean pilot.
#
# Prerequisites: cargo-hax 0.4.2, elan/Lean 4.31.0, a C compiler, rustup.
# The generated package lives under proofs/ (gitignored); only
# MANIFEST.sha256 is checked in.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SMOKE_DIR="$SCRIPT_DIR/smoke"
OUT_DIR="$PROJECT_ROOT/target/lean-varint-pilot"
LEAN_DIR="$OUT_DIR/proofs/lean"
PROOF_LIB="VarintLeanPilot"

command -v cargo-hax >/dev/null || { echo "error: cargo-hax 0.4.2 is required" >&2; exit 1; }
command -v lake >/dev/null || { echo "error: elan/lake is required" >&2; exit 1; }

normalize() {
    sed -e "s|$PROJECT_ROOT|<repo>|g" -e "s|$SCRIPT_DIR|<pilot>|g" -e "s|$SMOKE_DIR|<smoke>|g"
}

generated_digest() {
    {
        find "$LEAN_DIR" -type f \( -name '*.lean' -o -name 'lakefile.toml' -o -name 'lean-toolchain' \) \
            ! -path "$LEAN_DIR/VarintSmoke/Verification/*" \
            -print0 | sort -z | xargs -0 cat
        normalize < "$LEAN_DIR/lakefile.toml"
        normalize < "$LEAN_DIR/lean-toolchain"
    } | normalize | sha256sum | cut -d' ' -f1
}

measure() {
    local label="$1"
    shift
    local start end
    start=$(date +%s%N)
    (cd "$LEAN_DIR" && "$@") > /tmp/lean-varint-pilot-"$label".log 2>&1
    end=$(date +%s%N)
    echo "[$label] $(( (end - start) / 1000000 )) ms"
}

if [[ $# -ne 0 ]]; then
    echo "usage: $0" >&2
    exit 2
fi

# Always extract into a fresh tree so the manifest covers exactly the files the
# pinned toolchain produces (no stale leftovers from a previous pin). The
# generated package is kept under target/ so repository scans never see the
# dependency tree's own scripts.
rm -rf "$SMOKE_DIR/proofs" "$OUT_DIR"

echo "[extract] cargo hax into lean (hax 0.4.2, Aeneas build-183e4f0, hax-lean v0.3.30)"
start=$(date +%s%N)
(cd "$SMOKE_DIR" && cargo hax into lean) > /tmp/lean-varint-pilot-extract.log 2>&1
mkdir -p "$OUT_DIR"
mv "$SMOKE_DIR/proofs" "$OUT_DIR"
end=$(date +%s%N)
echo "[extract] $(( (end - start) / 1000000 )) ms"

cp "$SCRIPT_DIR/Contracts.lean" "$LEAN_DIR/$PROOF_LIB/Verification/Contracts.lean"
cp "$SCRIPT_DIR/Audit.lean" "$LEAN_DIR/$PROOF_LIB/Verification/Audit.lean"

digest=$(generated_digest)
expected=$(cut -d' ' -f1 "$SCRIPT_DIR/MANIFEST.sha256")
if [[ "$digest" != "$expected" ]]; then
    echo "error: generated Lean output drifted from MANIFEST.sha256 (stale output or pin update)" >&2
    echo "  expected: $expected" >&2
    echo "  actual:   $digest" >&2
    echo "Regenerate deliberately and update MANIFEST.sha256 with: $digest" >&2
    exit 1
fi
echo "[manifest] generated output matches MANIFEST.sha256 ($digest)"

measure verify lake build "$PROOF_LIB.Verification.Contracts"

axioms=$(cd "$LEAN_DIR" && lake env lean "$PROOF_LIB/Verification/Audit.lean" 2>&1)
echo "$axioms"
if grep -q "sorryAx" <<<"$axioms"; then
    echo "error: axiomatized proof hole detected (sorryAx)" >&2
    exit 1
fi
if ! grep -q "depends on axioms:" <<<"$axioms"; then
    echo "error: missing axiom audit output" >&2
    exit 1
fi
if grep -rqE "^(axiom|theorem .*:= by[[:space:]]*$)|sorry|admit" "$LEAN_DIR/$PROOF_LIB/Extraction"; then
    echo "error: proof hole or axiom found in the generated extraction" >&2
    exit 1
fi
echo "[axioms] no proof holes; standard Lean axioms only"

echo "ok: deterministic regeneration, manifest, build, and axiom audit passed"
