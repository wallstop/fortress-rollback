# Kani Obligations: `Frame::buffer_index` and varint

Reference for the #365 calibration step: the exact contracts, domains,
target-width assumptions, existing coverage, and exclusions for the two functions
nominated for the source-linked Lean pilot. Measure any new harness before
adding it to a tier; a 64-bit symbolic modulus does **not** discharge in Kani's
SAT backend (measured >15 min, see below).

## `Frame::buffer_index`

`src/lib.rs` — `pub const fn buffer_index(self, buffer_size: usize) -> Option<usize>`

| Item | Value |
| --- | --- |
| Contract | Negative frame or zero buffer size returns `None`. Otherwise returns exactly `frame % buffer_size` and the index is strictly less than `buffer_size`. |
| Domain | `i32` frame (all values) × `usize` buffer size. |
| Target-width assumption | `usize` is 32-bit or 64-bit; the production ring is `MAX_FRAME_DELAY + 1 == 128`, so both widths are far below the arithmetic ceiling. |
| Inputs that matter | `frame ∈ [0, i32::MAX]`, `buffer_size ∈ [1, 128]` for the production domain; negative frames and zero sizes for the invalid half. |
| Existing coverage | `proof_frame_buffer_index_invalid_none` (full symbolic `i32` × `usize`, ~2.3 s); `proof_frame_buffer_index_valid_modulo` (full non-negative `i32` with the concrete production ring size 128, ~2.4 s); `proof_frame_modulo_for_queue` (bounded frame, concrete 128). |
| Exclusions | A **symbolic divisor** is not proved in Kani: `size <= 128` still exceeded a 7-minute attempt and the unbounded form exceeded 15 minutes. Exactness for arbitrary ring sizes is recorded as a Lean candidate, not claimed. |
| Why it matters | The ring index selects saved-state and input-queue cells; an out-of-range index is an in-memory-corruption class defect, so the bounds half is the high-value property. |

## `rle::varint`

`src/rle.rs` — private `varint::{encoded_len, encode, decode}` used by the RLE
header and run lengths.

| Item | Value |
| --- | --- |
| `encoded_len` contract | `1 <= encoded_len(v) <= 10` for every `u64`; `encoded_len(0) == 1`; the length is minimal (a value needing `N` bytes satisfies `v >= 1 << (7 * (N - 1))` for `N > 1`). |
| `encode` contract | Writes without panicking; returns `encoded_len(v)` for a buffer of at least 10 bytes; a shorter buffer returns the number of bytes fully written (partial writes are the documented short-buffer behaviour, not an error). |
| `decode` contract | Returns `(value, consumed)` with `consumed <= buf.len() - offset`; every truncated prefix returns `Err`; continuation bytes are limited to ten (`shift == 63 && byte > 1` is malformed); the loop always terminates. |
| Domain | All `u64` values; arbitrary byte slices and `offset <= buf.len()`. |
| Target-width assumptions | None beyond `usize` for lengths; shifts are on `u64`. |
| Existing coverage | `proof_varint_encoded_len_correct`, `proof_varint_encode_single_byte`, `proof_varint_encoded_len_no_overflow`, `proof_varint_decode_empty_safe`, `proof_varint_decode_terminates` (3-byte buffer), `proof_varint_decode_offset_safe` (2-byte buffer), `proof_varint_roundtrip_small` (`value < 16384`), `proof_varint_continuation_handling`, and `proof_varint_full_u64_roundtrip_fixed_buffer` (every `u64`, fixed ten-byte buffer, ~6.3 s). |
| Exclusions | Decode over an arbitrary-length slice with a symbolic `offset` is covered only up to the harness buffer sizes; arbitrary malformed-slice rejection is exercised by unit/fuzz tests. Encoder short-buffer partial-write semantics are covered by unit tests, not by a proof. |
| Why it matters | The full-`u64` roundtrip removes the prior `< 16384` bound and proves `encode` writes exactly `encoded_len` bytes for every value. This is the cheap Kani baseline the Lean pilot must beat in assurance or cost to be retained. |

## Reproduce

```bash
export KANI_HOME="$HOME/.kani/kani-0.68.0"   # only if the bundle is not auto-detected
cargo kani --harness proof_frame_buffer_index_invalid_none --default-unwind 2
cargo kani --harness proof_frame_buffer_index_valid_modulo --default-unwind 2
cargo kani --harness proof_varint_full_u64_roundtrip_fixed_buffer --default-unwind 12
```

All three are registered in `scripts/verification/verify-kani.sh` Tier 1 and are
checked by `bash scripts/verification/check-kani-coverage.sh`.

## Lean pilot boundary

The pilot is retained only if it proves a property beyond this calibrated
baseline (for example unbounded arbitrary-size `buffer_index` exactness) with
compiling-source mutation controls, fail-closed regeneration and axiom audits,
warm verification around five minutes or less, and an explicit owner. Duplicated
scalar coverage of these tables is not a Lean win. The stop conditions in #365
(handwritten replacement implementation, sustained upstream fork, extensive
production rewriting, unapproved assumptions) end the experiment early.
