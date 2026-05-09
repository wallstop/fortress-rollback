#!/usr/bin/env python3
"""Regression tests for the harness sharding logic in verify-kani.sh.

History: a ceiling-division partition (``chunk_size = ceil(total / parts)``)
silently produced an empty trailing shard for layouts like 16 proofs across
5 parts (``4,4,4,4,0``). CI burned a runner doing nothing while reporting
success. The fix replaces the algorithm with a balanced partition where
sizes differ by at most 1::

    quotient  = total / parts
    remainder = total % parts
    # First `remainder` shards each get (quotient + 1) items.
    # Remaining shards each get `quotient` items.

This test exercises the partition logic via ``--print-partition``, a
no-cargo-needed introspection mode that prints the planned layout and
exits.
"""

from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
VERIFY_KANI = REPO_ROOT / "scripts" / "verification" / "verify-kani.sh"

# Tier sizes pinned to the production registry. These are intentionally
# duplicated rather than scraped so the test breaks loudly when somebody
# adds/removes a proof without thinking about sharding.
TIER_SIZES: dict[int, int] = {
    1: 63,
    2: 37,
    3: 16,
}


def _balanced_layout(total: int, parts: int) -> list[int]:
    """Return the expected per-shard sizes for a balanced partition.

    Sizes differ by at most 1; first ``total % parts`` shards get the
    larger size.
    """
    quotient, remainder = divmod(total, parts)
    return [quotient + 1 if p < remainder else quotient for p in range(parts)]


def _print_partition(
    tier: int, part: int | None = None, parts: int | None = None
) -> subprocess.CompletedProcess[str]:
    """Invoke verify-kani.sh in --print-partition mode."""
    cmd: list[str] = ["bash", str(VERIFY_KANI), "--print-partition", "--tier", str(tier)]
    if part is not None:
        cmd.extend(["--part", str(part)])
    if parts is not None:
        cmd.extend(["--parts", str(parts)])
    return subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


def _parse_partition_output(stdout: str) -> tuple[dict[str, str], list[str]]:
    """Parse ``--print-partition`` output.

    Returns ``(headers, harnesses)`` where ``headers`` maps key -> raw
    string value across the ``PARTITION ...`` and ``PART ...`` lines.

    The first line of ``--print-partition`` is the human-readable
    "Tier T (N proofs) -> ..." diagnostic that mirrors the run-path
    format (NIT #11). It's not a header and it's not a harness, so we
    skip any line that starts with ``Tier `` -- that prefix can never
    appear in a harness name (Kani proofs all start with ``proof_``).
    """
    headers: dict[str, str] = {}
    harnesses: list[str] = []
    for line in stdout.splitlines():
        if line.startswith("PARTITION ") or line.startswith("PART "):
            for token in line.split()[1:]:
                if "=" in token:
                    key, _, value = token.partition("=")
                    headers[key] = value
        elif line.startswith("Tier "):
            # Human-readable layout diagnostic; not a harness.
            continue
        elif line.strip():
            harnesses.append(line.strip())
    return headers, harnesses


# Validate the test's own ground truth: TIER_SIZES must match the script.
def test_tier_sizes_match_script() -> None:
    """Hard-pin TIER_SIZES so adding proofs without updating tests fails."""
    for tier, expected in TIER_SIZES.items():
        result = _print_partition(tier=tier)
        assert result.returncode == 0, (
            f"--print-partition --tier {tier} failed: {result.stderr}"
        )
        headers, harnesses = _parse_partition_output(result.stdout)
        assert int(headers["total"]) == expected, (
            f"Tier {tier} total mismatch: script says {headers['total']}, "
            f"test pins {expected}. Update TIER_SIZES (and verify the "
            f"matrix shard count is still appropriate)."
        )
        assert len(harnesses) == expected, (
            f"Tier {tier}: header total={headers['total']} but emitted "
            f"{len(harnesses)} harness lines."
        )


@dataclass(frozen=True)
class PartitionCase:
    """One ``(tier, parts)`` shard layout to validate.

    The expected sizes are derived from the balanced-partition formula at
    test time so the test stays in sync with TIER_SIZES.
    """

    tier: int
    parts: int


# Production matrices currently use:
#   tier 1: parts=1
#   tier 2: parts=6  (37/6 -> 7,6,6,6,6,6 balanced)
#   tier 3: parts=5  (16/5 -> 4,3,3,3,3 balanced; was 4,4,4,4,0 buggy)
#
# Plus a couple of additional shapes to cover edge ratios.
PARTITION_CASES: tuple[PartitionCase, ...] = (
    PartitionCase(tier=1, parts=1),
    PartitionCase(tier=2, parts=6),
    PartitionCase(tier=3, parts=5),
    PartitionCase(tier=3, parts=4),
    # Tier 3 / 8 parts: 16/8 -> 2,2,2,2,2,2,2,2 (exact).
    PartitionCase(tier=3, parts=8),
    # Tier 3 / 7 parts: 16/7 -> 3,3,2,2,2,2,2 (uneven, no empty shards).
    PartitionCase(tier=3, parts=7),
)


@pytest.mark.parametrize(
    "case",
    PARTITION_CASES,
    ids=lambda c: f"tier{c.tier}_parts{c.parts}",
)
def test_balanced_partition_no_empty_shards(case: PartitionCase) -> None:
    """Every shard from 1..parts gets a non-empty, balanced subset.

    Asserts (in addition to the per-case PARTITION_CASES checks):
        * Each shard's harness count matches the balanced layout
          (sizes differ by at most 1).
        * No shard is empty (the original 16/5 -> 4,4,4,4,0 bug).
        * Sum of all shards equals tier total.
        * Shards are contiguous and non-overlapping (start/end indices).
        * GLOBAL invariant (independent of layout case):
          ``min(parsed_sizes) > 0`` AND
          ``max(parsed_sizes) - min(parsed_sizes) <= 1`` AND
          ``sum(parsed_sizes) == total``.
          This is a stricter, more general check than asserting the absence
          of one historical buggy literal -- any future partition algorithm
          that violates "balanced + non-empty + total-preserving" will fail
          here regardless of the specific bad layout it produces.
    """
    total = TIER_SIZES[case.tier]
    expected_sizes = _balanced_layout(total, case.parts)
    assert min(expected_sizes) >= 1, (
        "Test invariant: total >= parts so no shard should be empty."
    )
    # Sanity-check the balanced-layout helper itself.
    assert sum(expected_sizes) == total
    assert max(expected_sizes) - min(expected_sizes) <= 1

    seen_indices: set[int] = set()
    summed = 0
    expected_start = 0
    parsed_sizes: list[int] = []

    for part in range(1, case.parts + 1):
        result = _print_partition(case.tier, part=part, parts=case.parts)
        assert result.returncode == 0, (
            f"--print-partition tier={case.tier} part={part} parts={case.parts} "
            f"failed (exit {result.returncode}):\n{result.stderr}"
        )

        headers, harnesses = _parse_partition_output(result.stdout)

        assert int(headers["tier"]) == case.tier
        assert int(headers["total"]) == total
        assert int(headers["parts"]) == case.parts
        assert int(headers["part"]) == part

        size = int(headers["size"])
        start = int(headers["start"])
        end = int(headers["end"])

        # Header consistency.
        assert size == end - start, (
            f"size/start/end mismatch for part {part}: "
            f"size={size}, start={start}, end={end}"
        )
        assert size == len(harnesses), (
            f"part {part} declared size={size} but emitted "
            f"{len(harnesses)} harness lines."
        )
        assert start == expected_start, (
            f"part {part} starts at {start} but the previous shard ended "
            f"at {expected_start}: shards must be contiguous."
        )

        # Per-shard size matches the balanced layout.
        assert size == expected_sizes[part - 1], (
            f"part {part}: balanced layout expected "
            f"{expected_sizes[part - 1]} harnesses, got {size}. "
            f"Full sizes header: {headers.get('sizes')}"
        )

        # No overlap with prior shards.
        for idx in range(start, end):
            assert idx not in seen_indices, (
                f"index {idx} appears in part {part} and an earlier shard."
            )
            seen_indices.add(idx)

        parsed_sizes.append(size)
        summed += size
        expected_start = end

    # Coverage: every harness in the tier belongs to exactly one shard.
    assert summed == total
    assert seen_indices == set(range(total))

    # Stricter, layout-agnostic invariant: balanced + non-empty + total-
    # preserving. Replaces the historical narrow check against the literal
    # "4,4,4,4,0" string -- any future bug that produces an empty shard,
    # an unbalanced shard, or a total-mismatch fails here.
    assert min(parsed_sizes) > 0, (
        f"Empty shard in tier {case.tier} parts {case.parts}: "
        f"sizes={parsed_sizes}"
    )
    assert max(parsed_sizes) - min(parsed_sizes) <= 1, (
        f"Unbalanced shards in tier {case.tier} parts {case.parts}: "
        f"sizes={parsed_sizes} (max - min must be <= 1)"
    )
    assert sum(parsed_sizes) == total, (
        f"Sum of shard sizes ({sum(parsed_sizes)}) != tier total ({total}) "
        f"for tier {case.tier} parts {case.parts}: sizes={parsed_sizes}"
    )


@pytest.mark.parametrize("tier,parts", [(3, 17), (3, 20), (2, 38)])
def test_partition_rejects_more_parts_than_proofs(tier: int, parts: int) -> None:
    """Misconfigured matrices (parts > total) MUST fail loudly.

    The previous implementation silently produced empty shards. We now
    refuse to plan such a layout so CI fails at partition time rather
    than wasting a runner that does nothing.
    """
    total = TIER_SIZES[tier]
    assert parts > total, "Test setup error: parts must exceed total."

    result = _print_partition(tier, part=1, parts=parts)
    assert result.returncode != 0, (
        f"Expected non-zero exit for parts={parts} > total={total}, "
        f"got {result.returncode}."
    )
    combined = result.stdout + result.stderr
    assert f"Tier {tier} has {total} proofs" in combined
    assert f"--parts={parts}" in combined
    assert "misconfigured CI matrix" in combined


def test_partition_layout_diagnostic_matches_balanced_formula() -> None:
    """The PARTITION header's sizes CSV matches the balanced-layout formula.

    This is the single source of truth for the planned shard sizes and is
    printed once before partition runs so future sharding changes are
    verifiable from CI logs.

    Note: the historical buggy layout (``4,4,4,4,0`` for tier 3 / 5 parts)
    is deliberately not asserted-against by literal here; the
    balanced-layout invariants in
    :func:`test_balanced_partition_no_empty_shards`
    (``min > 0``, ``max - min <= 1``, ``sum == total``) catch any empty/
    unbalanced shard regardless of the specific bad layout.
    """
    tier = 3
    parts = 5
    total = TIER_SIZES[tier]
    expected_sizes = _balanced_layout(total, parts)

    result = _print_partition(tier, part=1, parts=parts)
    assert result.returncode == 0
    headers, _ = _parse_partition_output(result.stdout)

    assert headers["sizes"] == ",".join(str(s) for s in expected_sizes), (
        f"PARTITION sizes={headers.get('sizes')} does not match the "
        f"balanced-layout formula {expected_sizes}."
    )


# ---------------------------------------------------------------------------
# Argument-validation surface tests.
#
# Bundled into one parametrized test (per the review note) to keep the
# rejection cases co-located and easy to extend. Each row is
#   (cli args after `--print-partition --tier 3`, expected error substring).
# We match on diagnostic substrings rather than full prose so future
# copy-edits don't churn the test.
# ---------------------------------------------------------------------------
INVALID_PART_ARGS: tuple[tuple[list[str], str], ...] = (
    # `--part 0` is positional/integer-valid but not positive: must be
    # rejected with a "must be a positive integer" diagnostic, NOT the
    # misleading "--parts requires --part" message that the previous
    # validation block produced (because it conflated 0 with "absent").
    (["--part", "0", "--parts", "5"], "--part must be a positive integer"),
    # Negative `--part` was the CRITICAL silent-bypass: the previous
    # `[[ "$part" -gt 0 ]]` gate fell through to the unsharded branch and
    # ran the FULL tier with exit 0. Must now be rejected.
    (["--part", "-1", "--parts", "5"], "--part must be a positive integer"),
    # part > parts: would index off the end of the proofs array.
    (["--part", "6", "--parts", "5"], "--part (6) cannot be greater than --parts (5)"),
    # Mixed presence: --parts without --part.
    (["--parts", "5"], "--parts requires --part"),
    # Mixed presence: --part without --parts.
    (["--part", "1"], "--part requires --parts"),
    # Non-integer --part. The arg-parser regex is the first gate; this
    # case pins that "abc" never reaches the partition math.
    (["--part", "abc", "--parts", "5"], "--part must be a positive integer"),
    # Symmetric non-integer --parts. The previous test suite covered the
    # `--part abc` side but not `--parts abc`; pin the symmetric path so
    # an arg-parser regex regression on either flag is caught.
    (["--parts", "abc", "--part", "1"], "--parts must be a positive integer"),
    # `--parts 0`: zero shards is meaningless; must fail at validation
    # rather than reaching the divide-by-zero in compute_partition_layout.
    (["--parts", "0", "--part", "1"], "--parts must be a positive integer"),
)


@pytest.mark.parametrize(
    "extra_args,expected_substring",
    INVALID_PART_ARGS,
    ids=lambda v: v if isinstance(v, str) else "_".join(v),
)
def test_partition_rejects_invalid_args(
    extra_args: list[str], expected_substring: str
) -> None:
    """Invalid --part/--parts combinations exit non-zero with a clear msg.

    This is the regression suite for the CRITICAL "negative --part
    silently bypassed validation and ran the full tier with exit 0" bug
    plus its near-neighbours (zero, mixed presence, non-integer, parts <
    1, part > parts). Each row pins one rejection.
    """
    cmd = [
        "bash",
        str(VERIFY_KANI),
        "--print-partition",
        "--tier",
        "3",
        *extra_args,
    ]
    result = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    combined = result.stdout + result.stderr

    assert result.returncode != 0, (
        f"Expected non-zero exit for invalid args {extra_args!r}, got "
        f"{result.returncode}.\nstdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )
    assert expected_substring in combined, (
        f"Expected diagnostic substring {expected_substring!r} for args "
        f"{extra_args!r}; got:\nstdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )
    # No PARTITION/PART block must leak when validation fails -- the
    # script must abort BEFORE printing a partial layout.
    assert "PARTITION tier=" not in combined, (
        f"Validation failure leaked a PARTITION block for args "
        f"{extra_args!r}:\n{combined}"
    )


# ---------------------------------------------------------------------------
# Standalone bad-value tests -- positivity must be checked BEFORE presence
# pairing so the error message matches the actual user mistake.
#
# An earlier revision of main()'s validation block ran presence pairing
# first, so `--part 0` alone reported "--part requires --parts" -- which is
# misleading: the real bug is that 0 is not positive (and the script would
# have rejected it even if --parts were also given). These tests pin that
# the diagnostic now matches the actual mistake regardless of which side
# of the pair was passed.
# ---------------------------------------------------------------------------
STANDALONE_BAD_VALUE_CASES: tuple[tuple[list[str], str], ...] = (
    # `--part 0` alone -> positivity error, NOT "requires --parts".
    (["--part", "0"], "--part must be a positive integer"),
    # `--part -1` alone -> positivity error.
    (["--part", "-1"], "--part must be a positive integer"),
    # `--parts 0` alone -> positivity error, NOT "requires --part".
    (["--parts", "0"], "--parts must be a positive integer"),
    # `--parts -1` alone -> positivity error.
    (["--parts", "-1"], "--parts must be a positive integer"),
)


@pytest.mark.parametrize(
    "extra_args,expected_substring",
    STANDALONE_BAD_VALUE_CASES,
    ids=lambda v: v if isinstance(v, str) else "_".join(v),
)
def test_standalone_bad_values_report_positivity_not_presence(
    extra_args: list[str], expected_substring: str
) -> None:
    """`--part 0` alone reports positivity, not presence-pairing.

    The validation order in ``main()`` puts per-flag positivity BEFORE
    presence pairing, so the diagnostic always matches the actual user
    error. The opposite order ("require both" first) produced misleading
    messages like ``"--part requires --parts"`` for ``--part 0``, where
    the real bug is that 0 is not positive.

    These cases are invoked WITHOUT ``--print-partition``/``--tier`` --
    they exercise the bare run-path validation pipe, not introspection
    mode, and must reject before reaching cargo kani.
    """
    cmd = ["bash", str(VERIFY_KANI), *extra_args]
    result = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    combined = result.stdout + result.stderr

    assert result.returncode != 0, (
        f"Expected non-zero exit for standalone bad value {extra_args!r}, "
        f"got {result.returncode}.\nstdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )
    assert expected_substring in combined, (
        f"Expected positivity diagnostic {expected_substring!r} for "
        f"standalone {extra_args!r}; got:\nstdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )
    # Must NOT report the presence-pairing error -- that would mean the
    # validation order regressed.
    presence_messages = ("--part requires --parts", "--parts requires --part")
    leaked = [m for m in presence_messages if m in combined]
    assert not leaked, (
        f"Standalone {extra_args!r} reported presence-pairing error "
        f"{leaked!r} instead of positivity. Validation order regressed.\n"
        f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )


# ---------------------------------------------------------------------------
# Mutually-exclusive mode tests.
#
# `--list`, `--print-partition`, `--harness`, and `--tier`+sharding are
# distinct execution modes that don't naturally compose. An earlier
# revision silently shadowed `--list` when `--print-partition` was also
# given, masking what the user actually asked for.
# ---------------------------------------------------------------------------
MUTUALLY_EXCLUSIVE_CASES: tuple[tuple[list[str], str], ...] = (
    # --list + --print-partition: both are no-run modes; pick one.
    (
        ["--list", "--print-partition", "--tier", "3"],
        "--list and --print-partition are mutually exclusive",
    ),
    # --list + --harness: --list is "list everything"; pairing with
    # --harness either silently ignored --harness or filtered, neither
    # being obvious. Reject.
    (
        ["--list", "--harness", "proof_frame_new_valid"],
        "--list cannot be combined with --harness",
    ),
    # --list + --tier: same reasoning as --list + --harness.
    (
        ["--list", "--tier", "1"],
        "--list cannot be combined with --tier",
    ),
    # --list + --part/--parts: sharding only makes sense within a tier.
    (
        ["--list", "--part", "1", "--parts", "2"],
        "--list cannot be combined with --part or --parts",
    ),
    # --print-partition + --harness: introspection mode is per-tier; a
    # single harness is a different (run-path) operation.
    (
        ["--print-partition", "--tier", "3", "--harness", "proof_frame_new_valid"],
        "--print-partition and --harness are mutually exclusive",
    ),
)


@pytest.mark.parametrize(
    "args,expected_substring",
    MUTUALLY_EXCLUSIVE_CASES,
    ids=lambda v: v if isinstance(v, str) else "_".join(v),
)
def test_rejects_mutually_exclusive_modes(
    args: list[str], expected_substring: str
) -> None:
    """Mutually-exclusive run/no-run modes reject with a clear message.

    The previous behaviour silently shadowed one mode with another (e.g.
    ``--list --print-partition --tier 3`` ignored ``--list`` entirely).
    Each row pins one rejection so future regressions surface here
    rather than as confusing log output.
    """
    cmd = ["bash", str(VERIFY_KANI), *args]
    result = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    combined = result.stdout + result.stderr

    assert result.returncode != 0, (
        f"Expected non-zero exit for mutually-exclusive args {args!r}, "
        f"got {result.returncode}.\nstdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )
    assert expected_substring in combined, (
        f"Expected mutual-exclusion diagnostic {expected_substring!r} "
        f"for args {args!r}; got:\nstdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )
    # No PARTITION block must leak: the script must abort BEFORE running
    # any of the two competing modes.
    assert "PARTITION tier=" not in combined, (
        f"Mutually-exclusive rejection leaked a PARTITION block for "
        f"{args!r}:\n{combined}"
    )


# ---------------------------------------------------------------------------
# --jobs argument validation.
#
# `--jobs` previously had no integer check, so `--jobs abc` produced a
# noisy bash arithmetic error or a downstream "unbound variable". Now it
# rejects at parse time with the same diagnostic shape as `--part`/`--parts`.
# ---------------------------------------------------------------------------
JOBS_INVALID_CASES: tuple[tuple[list[str], str], ...] = (
    (["--jobs", "abc"], "--jobs must be a positive integer"),
    (["--jobs", "0"], "--jobs must be a positive integer"),
    (["--jobs", "-1"], "--jobs must be a positive integer"),
    (["--jobs", ""], "--jobs must be a positive integer"),
)


@pytest.mark.parametrize(
    "args,expected_substring",
    JOBS_INVALID_CASES,
    ids=lambda v: v if isinstance(v, str) else "_".join(v),
)
def test_jobs_rejects_non_positive_integer(
    args: list[str], expected_substring: str
) -> None:
    """`--jobs` rejects non-positive-integer values at parse time."""
    cmd = ["bash", str(VERIFY_KANI), *args]
    result = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    combined = result.stdout + result.stderr

    assert result.returncode != 0, (
        f"Expected non-zero exit for --jobs args {args!r}, got "
        f"{result.returncode}.\nstdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )
    assert expected_substring in combined, (
        f"Expected jobs diagnostic {expected_substring!r} for args "
        f"{args!r}; got:\nstdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )


# ---------------------------------------------------------------------------
# Introspection-mode notice.
#
# When the user passes flags that only matter for the run path (--quick,
# --verbose, --jobs, --fail-fast) alongside --print-partition, surface a
# one-line note so they don't infer those flags took effect.
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    "extra_flag",
    [
        ["--quick"],
        ["--verbose"],
        ["--jobs", "2"],
        ["--fail-fast"],
    ],
    ids=lambda v: v if isinstance(v, str) else "_".join(v),
)
def test_print_partition_notes_ignored_run_flags(extra_flag: list[str]) -> None:
    """`--print-partition` with run-only flags emits an [note] line."""
    cmd = [
        "bash",
        str(VERIFY_KANI),
        "--print-partition",
        "--tier",
        "3",
        *extra_flag,
    ]
    result = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, (
        f"--print-partition with {extra_flag!r} should still succeed; "
        f"got exit {result.returncode}.\nstdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )
    combined = result.stdout + result.stderr
    assert (
        "[note] introspection mode; --quick/--verbose/--jobs/--fail-fast ignored"
        in combined
    ), (
        f"Expected introspection-mode note when {extra_flag!r} was passed.\n"
        f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )


def test_print_partition_omits_note_when_no_run_flags() -> None:
    """Without run-only flags, `--print-partition` does NOT emit the note."""
    result = _print_partition(tier=3, part=1, parts=5)
    combined = result.stdout + result.stderr
    assert result.returncode == 0
    assert "[note] introspection mode" not in combined, (
        f"Introspection-mode note leaked without trigger flags.\n"
        f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )


# ---------------------------------------------------------------------------
# `--print-partition` now emits a human-readable layout line as the FIRST
# line of output (matches the run-path format), followed by the
# machine-parseable PARTITION/PART headers. The previous output started
# with PARTITION which made the introspection mode and run-path log
# format gratuitously inconsistent.
# ---------------------------------------------------------------------------
def test_print_partition_starts_with_human_readable_diagnostic() -> None:
    """First line of `--print-partition` is the human-readable layout line."""
    result = _print_partition(tier=3, part=1, parts=5)
    assert result.returncode == 0
    lines = result.stdout.splitlines()
    assert lines, f"Empty stdout from --print-partition: {result!r}"
    # Should match the same shape as run_tier_proofs's diagnostic.
    expected_prefix = (
        "Tier 3 (16 proofs) -> 5 parts: [4,3,3,3,3]; this is part 1 -> "
        "4 proofs, indices 0..3"
    )
    assert lines[0] == expected_prefix, (
        f"First line of --print-partition does not match the run-path "
        f"diagnostic format.\nExpected: {expected_prefix!r}\n"
        f"Got: {lines[0]!r}\nFull stdout:\n{result.stdout}"
    )
    # The PARTITION header must still follow.
    assert any(line.startswith("PARTITION ") for line in lines), (
        "PARTITION header missing after the human-readable line."
    )


def test_print_partition_unsharded_starts_with_human_readable_diagnostic() -> None:
    """Unsharded mode also emits the human-readable layout line first.

    Note: the diagnostic uses ``"1 part"`` (singular) -- not ``"1 parts"`` --
    so the unsharded message reads grammatically. Sharded layouts (parts>1)
    keep ``"N parts"``.
    """
    result = _print_partition(tier=3)
    assert result.returncode == 0
    lines = result.stdout.splitlines()
    assert lines
    assert lines[0].startswith("Tier 3 (16 proofs) -> 1 part: ["), (
        f"Unsharded --print-partition first line should be the "
        f"human-readable diagnostic with singular 'part'; got: {lines[0]!r}"
    )


def test_print_partition_rejects_multiple_tiers() -> None:
    """``--print-partition`` with multiple ``--tier`` values is rejected.

    Two unlabelled PARTITION/PART blocks back-to-back are ambiguous for
    downstream consumers (CI parsers, the tests in this module). The
    script must reject the combination with a clear message; the user
    can invoke once per tier when they need both.
    """
    result = subprocess.run(
        [
            "bash",
            str(VERIFY_KANI),
            "--print-partition",
            "--tier",
            "2",
            "--tier",
            "3",
            "--part",
            "3",
            "--parts",
            "5",
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    combined = result.stdout + result.stderr

    assert result.returncode != 0, (
        f"--print-partition with multiple --tier should reject; got "
        f"exit {result.returncode}.\n{combined}"
    )
    assert "--print-partition does not support multiple --tier" in combined
    # Must not emit any PARTITION block on the rejection path.
    assert "PARTITION tier=" not in combined, (
        f"Multi-tier rejection leaked a PARTITION block:\n{combined}"
    )


# ---------------------------------------------------------------------------
# End-to-end run-path test.
#
# `--print-partition` is introspection-only and skips run_tier_proofs(); we
# also need to assert that the LAYOUT DIAGNOSTIC line emitted on the real
# run path (the human-facing "Tier T (N proofs) -> M parts: [...]" line in
# CI logs) is correct. This test invokes the script with a fake `cargo`
# shim that emits a Kani-style success summary so verify-kani.sh's parser
# reports success and we can see the diagnostic in stdout.
# ---------------------------------------------------------------------------
KANI_SUCCESS_SUMMARY = (
    "Complete - 1 successfully verified harnesses, 0 failures, 1 total."
)


def test_run_path_emits_layout_diagnostic_and_runs_balanced_partition(
    fake_cargo_repo_factory,
    verify_kani_runner,
) -> None:
    """The real run path prints the layout diagnostic and runs the right shards.

    `--print-partition` only exercises the introspection path. This test
    invokes the script through ``run_tier_proofs`` with a fake cargo
    shim that emits a Kani-style "Complete - ..." summary so the script
    reports success, and asserts:

    * The layout diagnostic line ``"Tier 3 (16 proofs) -> 5 parts:
      [4,3,3,3,3]; this is part 1 -> 4 proofs, indices 0..3"`` appears
      verbatim in stdout (substring-matched against ANSI-stripped text).
    * Every harness in part 1's expected balanced subset is verified
      (the script echoes ``Verifying: <harness>`` per harness).
    * No harness from outside part 1's range is verified.

    Together these lock down the run-path partitioning end-to-end, not
    just the introspection layout.
    """
    repo, script_path, fake_bin = fake_cargo_repo_factory(
        success_summary=KANI_SUCCESS_SUMMARY,
    )

    # Reference harnesses for tier 3: pinned here so any change to the
    # tier-3 ordering is caught instead of silently rebalancing.
    tier = 3
    parts = 5
    part = 1
    total = TIER_SIZES[tier]
    expected_sizes = _balanced_layout(total, parts)
    expected_part_size = expected_sizes[part - 1]
    # First 4 harnesses of tier 3 in script order.
    expected_part_harnesses = [
        "proof_index_wrapping_consistent",
        "proof_add_single_input_maintains_invariants",
        "proof_sequential_inputs_maintain_invariants",
        "proof_discard_maintains_invariants",
    ]
    assert len(expected_part_harnesses) == expected_part_size, (
        "Test setup error: expected_part_harnesses must equal balanced "
        "layout's part-1 size."
    )

    result = verify_kani_runner(
        repo,
        script_path,
        fake_bin,
        args=[
            "--quick",
            "--tier",
            str(tier),
            "--part",
            str(part),
            "--parts",
            str(parts),
        ],
        fake_exit=0,
    )

    assert result.returncode == 0, (
        f"verify-kani.sh returned {result.returncode}, expected 0.\n"
        f"--- stdout ---\n{result.stdout}\n--- stderr ---\n{result.stderr}"
    )

    # Strip ANSI colour codes once so substring matching is reliable.
    import re

    plain = re.sub(r"\x1b\[[0-9;]*m", "", result.stdout)

    expected_diag = (
        f"Tier {tier} ({total} proofs) -> {parts} parts: "
        f"[{','.join(str(s) for s in expected_sizes)}]; this is part "
        f"{part} -> {expected_part_size} proofs, indices 0..3"
    )
    assert expected_diag in plain, (
        f"Run-path layout diagnostic missing.\n"
        f"Expected substring: {expected_diag!r}\n"
        f"--- stdout (ANSI-stripped) ---\n{plain}"
    )

    # Every harness in part 1's expected subset must be verified.
    for harness in expected_part_harnesses:
        assert f"Verifying: {harness}" in plain, (
            f"Expected harness {harness!r} to be verified in part "
            f"{part}; not found in stdout.\n--- stdout ---\n{plain}"
        )

    # No harness from later parts must run in part 1. Pick a known
    # tier-3 harness that should land in part 5 (last shard, index 13+).
    out_of_part_harness = "proof_sparse_saving_respects_saved_frame"
    assert f"Verifying: {out_of_part_harness}" not in plain, (
        f"Harness {out_of_part_harness!r} should not run in part "
        f"{part}; it appeared in stdout.\n--- stdout ---\n{plain}"
    )


# ---------------------------------------------------------------------------
# Pluralisation: "1 part" (singular) vs "N parts" (plural).
#
# The introspection-mode unsharded diagnostic historically read "1 parts"
# which is ungrammatical. The grammatical fix uses singular for parts==1
# and singular ``proof`` for size==1. These tests pin both forms.
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    "tier,parts,expected_phrase",
    [
        # parts > 1 -> plural everywhere.
        (3, 5, "5 parts:"),
        (2, 6, "6 parts:"),
        # parts == 1 (sharded form) -> "1 part" singular.
        # 16 / 1 = 16 proofs in the only shard, so "16 proofs" plural.
        (3, 1, "1 part:"),
    ],
    ids=lambda v: v if isinstance(v, str) else str(v),
)
def test_print_partition_pluralises_parts_word(
    tier: int, parts: int, expected_phrase: str
) -> None:
    """The diagnostic uses singular ``part`` for parts==1 and plural otherwise."""
    result = _print_partition(tier=tier, part=1, parts=parts)
    assert result.returncode == 0, (
        f"--print-partition failed: {result.stderr}"
    )
    first_line = result.stdout.splitlines()[0]
    assert expected_phrase in first_line, (
        f"Expected pluralisation token {expected_phrase!r} in first line; "
        f"got: {first_line!r}"
    )
    # Inverse: the wrong form must never coexist with the right one.
    if expected_phrase == "1 part:":
        assert " 1 parts:" not in first_line, (
            f"Singular case leaked plural form: {first_line!r}"
        )
    else:
        # Plural cases must not accidentally print "<N> part:" (without 's').
        bad = expected_phrase.replace(" parts:", " part:")
        assert bad not in first_line, (
            f"Plural case leaked singular form: {first_line!r}"
        )


def test_print_partition_pluralises_proof_word_for_single_proof_shard() -> None:
    """When a shard contains exactly one proof, the diagnostic says ``1 proof``.

    Tier 3 has 16 proofs; sharding into 16 parts gives 1 proof per shard.
    The historical bug emitted ``"1 proofs"`` which is ungrammatical.
    """
    result = _print_partition(tier=3, part=1, parts=16)
    assert result.returncode == 0, result.stderr
    first_line = result.stdout.splitlines()[0]
    # "this is part 1 -> 1 proof, indices 0..0" (singular).
    assert "-> 1 proof," in first_line, (
        f"Expected '-> 1 proof,' (singular) in: {first_line!r}"
    )
    assert "-> 1 proofs," not in first_line, (
        f"Plural 'proofs' leaked when shard size is 1: {first_line!r}"
    )


# ---------------------------------------------------------------------------
# Issue 5: --jobs N --print-partition must NOT emit the "--jobs ignored
# per-harness" clamp warning. Cargo kani is never invoked in introspection
# mode, so that warning is misleading. The introspection-mode note (which
# already mentions --jobs) is the single source of truth.
# ---------------------------------------------------------------------------
def test_print_partition_with_jobs_emits_only_introspection_note() -> None:
    """`--print-partition --jobs N` does not emit the per-harness clamp warning."""
    cmd = [
        "bash",
        str(VERIFY_KANI),
        "--print-partition",
        "--tier",
        "3",
        "--jobs",
        "4",
    ]
    result = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, (
        f"--print-partition --jobs N should succeed; got "
        f"{result.returncode}.\nstdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )
    combined = result.stdout + result.stderr
    # Introspection-mode note must appear (it covers --jobs).
    assert (
        "[note] introspection mode; --quick/--verbose/--jobs/--fail-fast ignored"
        in combined
    ), f"Missing introspection-mode note:\n{combined}"
    # The per-harness clamp warning must NOT appear -- under --print-partition
    # cargo kani is never invoked, so "ignored per-harness" is misleading.
    assert "is ignored: this script invokes cargo kani per-harness" not in combined, (
        f"Misleading --jobs clamp warning leaked under --print-partition:\n"
        f"{combined}"
    )


# ---------------------------------------------------------------------------
# Issue 7: --harness rejects --tier/--part/--parts.
# Issue 8: run-path multi-tier with --part/--parts is rejected.
# Plus: --part/--parts without --tier is rejected (sharding is per-tier).
# ---------------------------------------------------------------------------
HARNESS_AND_RUN_PATH_REJECTION_CASES: tuple[tuple[list[str], str], ...] = (
    # Issue 7: --harness + --tier.
    (
        ["--harness", "proof_frame_new_valid", "--tier", "1"],
        "--harness cannot be combined with --tier",
    ),
    # Issue 7: --harness + --part (with --parts to pass pairing validation).
    (
        ["--harness", "proof_frame_new_valid", "--part", "1", "--parts", "2"],
        "--harness cannot be combined with --part or --parts",
    ),
    # Issue 7: --harness + --parts alone -- still must reject. The
    # presence-pairing validation runs before this check, so this case
    # actually trips the pairing error first; we pin that exact message
    # rather than the harness-combination one to document the order.
    (
        ["--harness", "proof_frame_new_valid", "--parts", "2"],
        "--parts requires --part",
    ),
    # Issue 8: multi-tier on the run path with --part/--parts.
    (
        ["--tier", "2", "--tier", "3", "--part", "1", "--parts", "2"],
        "--part/--parts cannot be combined with multiple --tier values",
    ),
    # --part/--parts without --tier on the run path.
    (
        ["--part", "1", "--parts", "2"],
        "--part/--parts require --tier",
    ),
)


@pytest.mark.parametrize(
    "args,expected_substring",
    HARNESS_AND_RUN_PATH_REJECTION_CASES,
    ids=lambda v: v if isinstance(v, str) else "_".join(v),
)
def test_run_path_rejects_ambiguous_combinations(
    args: list[str], expected_substring: str
) -> None:
    """Run-path argument combinations that are ambiguous reject loudly.

    Covers issue 7 (``--harness`` cannot combine with ``--tier``/``--part``/
    ``--parts``) and issue 8 (multi-``--tier`` on the run path rejects
    ``--part``/``--parts``). Each row pins one rejection.
    """
    cmd = ["bash", str(VERIFY_KANI), *args]
    result = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    combined = result.stdout + result.stderr

    assert result.returncode != 0, (
        f"Expected non-zero exit for {args!r}, got {result.returncode}.\n"
        f"--- stdout ---\n{result.stdout}\n--- stderr ---\n{result.stderr}"
    )
    assert expected_substring in combined, (
        f"Expected diagnostic {expected_substring!r} for args {args!r}; "
        f"got:\n--- stdout ---\n{result.stdout}\n"
        f"--- stderr ---\n{result.stderr}"
    )
    # The script must abort BEFORE invoking cargo kani; no
    # "Verifying:" or "Running Tier" lines should leak.
    assert "Verifying: " not in combined, (
        f"Run-path rejection leaked a 'Verifying:' line for {args!r}:\n"
        f"{combined}"
    )
    assert "Running Tier" not in combined, (
        f"Run-path rejection leaked a 'Running Tier' line for {args!r}:\n"
        f"{combined}"
    )


# ---------------------------------------------------------------------------
# Help-text content tests.
#
# The help text is the user-facing surface; missing / dead env vars and
# missing constraint documentation have caused real confusion. Pin both
# the inclusions and exclusions here so future edits don't regress the
# polish layer.
# ---------------------------------------------------------------------------
def test_help_lists_kani_unwind_env_var() -> None:
    """`--help` documents ``KANI_UNWIND`` (the script reads it)."""
    result = subprocess.run(
        ["bash", str(VERIFY_KANI), "--help"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert "KANI_UNWIND" in result.stdout, (
        f"--help omits KANI_UNWIND:\n{result.stdout}"
    )


def test_help_omits_dead_kani_jobs_env_var() -> None:
    """`--help` does NOT advertise the dead ``KANI_JOBS`` env var.

    The script previously read ``KANI_JOBS`` but never used it -- only
    the ``--jobs`` flag controls parallelism. The dead env var was removed
    from the script and from the help text; this test pins the removal.
    """
    result = subprocess.run(
        ["bash", str(VERIFY_KANI), "--help"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert "KANI_JOBS" not in result.stdout, (
        f"--help still advertises dead KANI_JOBS env var:\n{result.stdout}"
    )


def test_help_documents_mutually_exclusive_constraints() -> None:
    """`--help` has a Constraints section listing mutually-exclusive flags."""
    result = subprocess.run(
        ["bash", str(VERIFY_KANI), "--help"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    text = result.stdout
    assert "Constraints:" in text, f"Missing 'Constraints:' section:\n{text}"
    # Each row of the constraint list must mention the flag(s) it covers.
    expected_tokens = (
        "--list",
        "--print-partition",
        "--harness",
        "--part",
        "--parts",
        "--tier",
    )
    missing = [t for t in expected_tokens if t not in text]
    assert not missing, (
        f"Constraints section is missing tokens {missing!r}:\n{text}"
    )


# ---------------------------------------------------------------------------
# Stderr/stdout discipline for argument-validation diagnostics.
#
# Every "Error:" / "Unknown option:" message emitted by the arg-parser must
# go to stderr -- mixing them on stdout breaks consumers that capture stdout
# for parsing (CI workflows, pipelines like `verify-kani.sh --list | grep
# proof_X`). A previous revision had four legacy paths that wrote to stdout;
# this parametrized test pins each one to stderr-only so a regression flips
# loudly.
#
# Each row is `(cli args, error substring)`. The assertion is:
#   * Error substring appears in stderr.
#   * Error substring does NOT appear in stdout.
# ---------------------------------------------------------------------------
ERROR_GOES_TO_STDERR_CASES: tuple[tuple[list[str], str], ...] = (
    # `--harness` with no value -> "Error: --harness requires an argument".
    (["--harness"], "Error: --harness requires an argument"),
    # `--tier` with no value -> "Error: --tier requires an argument".
    (["--tier"], "Error: --tier requires an argument"),
    # `--tier 4` (out of 1..3) -> "Error: --tier must be 1, 2, or 3".
    (["--tier", "4"], "Error: --tier must be 1, 2, or 3"),
    # Unknown option -> "Unknown option: ...".
    (["--no-such-flag"], "Unknown option: --no-such-flag"),
    # And a sample from each of the validation paths that already used
    # stderr, to lock in the invariant for the whole arg-parser.
    (["--part", "abc", "--parts", "5"], "Error: --part must be a positive integer"),
    (["--parts", "abc", "--part", "1"], "Error: --parts must be a positive integer"),
    (["--jobs", "abc"], "Error: --jobs must be a positive integer"),
)


@pytest.mark.parametrize(
    "args,expected_substring",
    ERROR_GOES_TO_STDERR_CASES,
    ids=lambda v: v if isinstance(v, str) else "_".join(v),
)
def test_error_diagnostics_go_to_stderr_not_stdout(
    args: list[str], expected_substring: str
) -> None:
    """`Error:` / `Unknown option:` messages must go to stderr only.

    Captures stdout and stderr separately and asserts the diagnostic
    substring appears in stderr but not stdout. This is the regression
    suite for the four legacy paths (--harness/--tier missing-arg,
    --tier out-of-range, unknown option) that previously wrote to stdout.
    """
    result = subprocess.run(
        ["bash", str(VERIFY_KANI), *args],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode != 0, (
        f"Expected non-zero exit for {args!r}; got {result.returncode}.\n"
        f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )
    assert expected_substring in result.stderr, (
        f"Expected diagnostic {expected_substring!r} on stderr for "
        f"{args!r}.\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )
    assert expected_substring not in result.stdout, (
        f"Diagnostic {expected_substring!r} leaked to stdout for {args!r} "
        f"-- it must be stderr-only so consumers parsing stdout (CI "
        f"workflows, pipelines) don't trip on diagnostic noise.\n"
        f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )


# ---------------------------------------------------------------------------
# `--list` introspection-mode notice (mirrors --print-partition).
#
# Like --print-partition, --list ignores the run-only flags (--quick,
# --verbose, --jobs, --fail-fast). For consistency, surface the same
# `[note]` line so callers don't infer those flags took effect.
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    "extra_flag",
    [
        ["--quick"],
        ["--verbose"],
        ["--jobs", "2"],
        ["--fail-fast"],
    ],
    ids=lambda v: v if isinstance(v, str) else "_".join(v),
)
def test_list_notes_ignored_run_flags(extra_flag: list[str]) -> None:
    """`--list` with run-only flags emits the introspection-mode note."""
    cmd = ["bash", str(VERIFY_KANI), "--list", *extra_flag]
    result = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    # --list runs check_kani; if cargo/kani aren't installed the script
    # exits before reaching list_harnesses. Skip rather than fail in that
    # environment -- the note we're asserting on still gets emitted on
    # the success path which is what CI exercises.
    if result.returncode != 0:
        pytest.skip(
            f"--list returned {result.returncode}; cargo/kani may be "
            f"missing in this environment.\nstderr:\n{result.stderr}"
        )
    combined = result.stdout + result.stderr
    assert (
        "[note] introspection mode; --quick/--verbose/--jobs/--fail-fast ignored"
        in combined
    ), (
        f"Expected introspection-mode note when {extra_flag!r} was passed "
        f"with --list.\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )


def test_list_omits_note_when_no_run_flags() -> None:
    """Without run-only flags, `--list` does NOT emit the introspection note."""
    result = subprocess.run(
        ["bash", str(VERIFY_KANI), "--list"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        pytest.skip(
            f"--list returned {result.returncode}; cargo/kani may be "
            f"missing in this environment.\nstderr:\n{result.stderr}"
        )
    combined = result.stdout + result.stderr
    assert "[note] introspection mode" not in combined, (
        f"Introspection-mode note leaked from --list without trigger flags.\n"
        f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )
