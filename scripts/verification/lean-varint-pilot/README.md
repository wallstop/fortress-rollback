# Varint Lean pilot (#365)

Opt-in, reproducible harness for the source-linked Lean experiment on the
production varint seam (`src/rle/varint.rs`). It is **not** wired into CI: the
experiment is recorded as rejected for now (see the decision below), and the
generated package plus the ~7.8 GB dependency cache stay out of the repository.

## Pins

- `cargo-hax 0.4.2` (`cargo install --locked cargo-hax --version 0.4.2`)
- Aeneas `build-183e4f0` and hax-lean `v0.3.30` (pinned by the generated
  `lakefile.toml`)
- Lean `leanprover/lean4:v4.31.0` via elan

The smoke crate is a standalone package (empty `[workspace]`) and includes the
**unchanged** production file via `#[path]`.

## Usage

```bash
./scripts/verification/lean-varint-pilot/run.sh
```

The script:

1. removes `smoke/proofs/` and `target/lean-varint-pilot/`, then re-extracts
   with `cargo hax into lean` (the generated package is moved under `target/`);
2. normalizes absolute paths and hashes the generated Lean files, failing when
   they drift from `MANIFEST.sha256` (stale output or a pin update);
3. copies the checked-in `Contracts.lean` / `Audit.lean` into the package;
4. builds the package and prints the measured verification time;
5. runs `#print axioms VarintSmoke.encoded_len_zero`, failing on `sorryAx` or
   on any proof hole / `axiom` in the generated extraction.

Updating a pin is deliberate: regenerate, inspect the diff, and replace the
digest in `MANIFEST.sha256` with the value the script prints.

## Measured cost

Local 8-core devcontainer, package dependencies prebuilt:

| Measurement | Result |
| --- | --- |
| Warm extraction (`cargo hax into lean`, toolchains installed) | ~1.1 s |
| Warm verification (`lake build`, no-op) | 1.2 s / 1.2 s / 1.2 s |
| Cold project verification (fresh package build, packages cached) | ~2 min (118-136 s measured) |
| Dependency cache, uncompressed | 7.8 GB |
| Dependency cache, gzip `-1` | 2.8 GB |

Hosted-run numbers were not taken: the lane is not proposed for CI, so there is
no hosted cache or runner to measure. Warm verification is far below the
five-minute budget and the compressed cache is near the 3 GB budget; cost is not
the blocker.

## Decision: not retained (rejected for now)

The pilot fails the adoption criterion that Lean must prove a **valuable
property beyond the calibrated Kani baseline**:

- The only Lean theorem is `encoded_len_zero`, and Kani already proves the full
  `1 <= encoded_len(v) <= 10` contract for every `u64`
  (`proof_varint_encoded_len_correct`), the full-`u64` roundtrip, and the decode
  bounds/termination properties.
- The loop-heavy contracts (short-buffer `encode`, full roundtrip, termination,
  consumed bounds, malformed/truncated) are not yet proven in Lean: Aeneas emits
  `loop` as a `partial_fixpoint`, so each needs loop induction over the
  `rust_primitives` slice models.
- No maintenance ownership for the pinned Hax/Aeneas/Lean stack has been
  established.

The toolchain is viable and the mutation control works: changing the production
`encoded_len(0)` return from `1` to `2` still compiles and extracts, and the
Lean build then fails `encoded_len_zero` with an unsolved `False` goal. Revisit
when (a) a property beyond Kani is nominated, (b) the loop-heavy proofs are
demonstrated, and (c) an owner accepts the pin maintenance.
