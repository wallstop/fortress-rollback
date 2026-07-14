#!/usr/bin/env python3
"""Tests for the fail-closed SyncHandshakeV1 NDJSON trace wrapper."""

from __future__ import annotations

import copy
import importlib.util
import json
import sys
from types import SimpleNamespace
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "verification" / "verify-sync-handshake-traces.py"
SPEC = ROOT / "specs" / "tla" / "SyncHandshakeV1Trace.tla"

module_spec = importlib.util.spec_from_file_location("verify_sync_handshake_traces", SCRIPT)
assert module_spec is not None and module_spec.loader is not None
trace_verifier = importlib.util.module_from_spec(module_spec)
sys.modules[module_spec.name] = trace_verifier
module_spec.loader.exec_module(trace_verifier)


def _load_default_cases() -> list[trace_verifier.TraceCase]:
    return [
        trace_verifier.load_trace(path)
        for path in sorted(trace_verifier.DEFAULT_TRACE_DIR.glob("*.ndjson"))
    ]


def test_default_traces_are_strict_and_mutation_is_single_field() -> None:
    cases = _load_default_cases()
    assert {case.name for case in cases} == {
        "matching",
        "mismatch",
        "timeout",
        "duplicate-reply-decrement",
    }

    by_filename = {case.path.name: case for case in cases}
    baseline = by_filename["matching.ndjson"]
    mutation = trace_verifier.materialize_mutation(
        by_filename["duplicate-reply-decrement.ndjson"], by_filename
    )

    expected = copy.deepcopy(list(baseline.rows))
    expected[8]["updates"]["syncRemaining"]["p1"] = 0
    assert list(mutation.rows) == expected
    assert baseline.rows[8]["updates"]["syncRemaining"]["p1"] == 1


def test_matching_trace_exercises_duplicate_before_both_sync() -> None:
    matching = trace_verifier.load_trace(
        trace_verifier.DEFAULT_TRACE_DIR / "matching.ndjson"
    )
    duplicate = matching.rows[8]
    assert duplicate["action"] == "HandleSyncReply"
    assert duplicate["updates"]["acceptedTokens"]["p1"] == [1]
    assert duplicate["updates"]["syncRemaining"]["p1"] == 1
    assert matching.rows[-1]["updates"]["phase"] == {
        "p1": "Synced",
        "p2": "Synced",
    }


def test_unknown_header_key_fails_closed(tmp_path: Path) -> None:
    path = tmp_path / "bad.ndjson"
    path.write_text(
        json.dumps(
            {
                "schema": 1,
                "trace": "bad",
                "expect": "accept",
                "description": "bad",
                "ignored": True,
            }
        )
        + "\n",
        encoding="utf-8",
    )
    with pytest.raises(trace_verifier.TraceError, match="unknown header keys"):
        trace_verifier.load_trace(path)


def test_blank_ndjson_line_fails_closed(tmp_path: Path) -> None:
    path = tmp_path / "blank.ndjson"
    path.write_text(
        '{"schema":1,"trace":"blank","expect":"accept","description":"bad"}\n\n',
        encoding="utf-8",
    )
    with pytest.raises(trace_verifier.TraceError, match="blank lines are not allowed"):
        trace_verifier.load_trace(path)


def test_reject_trace_cannot_supply_hidden_rows(tmp_path: Path) -> None:
    header = {
        "schema": 1,
        "trace": "bad-mutation",
        "expect": "reject",
        "description": "bad",
        "derived_from": "matching.ndjson",
        "mutation": {
            "step": 8,
            "variable": "syncRemaining",
            "peer": "p1",
            "from": 1,
            "to": 0,
        },
    }
    path = tmp_path / "bad-mutation.ndjson"
    path.write_text(
        json.dumps(header) + "\n" + json.dumps({"step": 0}) + "\n",
        encoding="utf-8",
    )
    with pytest.raises(trace_verifier.TraceError, match="must derive all rows"):
        trace_verifier.load_trace(path)


def test_wrong_mutation_source_value_fails_closed(tmp_path: Path) -> None:
    header = {
        "schema": 1,
        "trace": "wrong-source",
        "expect": "reject",
        "description": "bad",
        "derived_from": "matching.ndjson",
        "mutation": {
            "step": 8,
            "variable": "syncRemaining",
            "peer": "p1",
            "from": 2,
            "to": 0,
        },
    }
    path = tmp_path / "wrong-source.ndjson"
    path.write_text(json.dumps(header) + "\n", encoding="utf-8")
    mutation = trace_verifier.load_trace(path)
    matching = trace_verifier.load_trace(
        trace_verifier.DEFAULT_TRACE_DIR / "matching.ndjson"
    )
    with pytest.raises(trace_verifier.TraceError, match="mutation.from"):
        trace_verifier.materialize_mutation(
            mutation, {"matching.ndjson": matching, path.name: mutation}
        )


def test_mutation_must_target_a_peer_mapped_variable(tmp_path: Path) -> None:
    header = {
        "schema": 1,
        "trace": "network-mutation",
        "expect": "reject",
        "description": "bad",
        "derived_from": "matching.ndjson",
        "mutation": {
            "step": 8,
            "variable": "network",
            "peer": "p1",
            "from": [],
            "to": [],
        },
    }
    path = tmp_path / "network-mutation.ndjson"
    path.write_text(json.dumps(header) + "\n", encoding="utf-8")
    with pytest.raises(trace_verifier.TraceError, match="peer-mapped variable"):
        trace_verifier.load_trace(path)


def test_mutation_must_change_its_target(tmp_path: Path) -> None:
    header = {
        "schema": 1,
        "trace": "identity-mutation",
        "expect": "reject",
        "description": "bad",
        "derived_from": "matching.ndjson",
        "mutation": {
            "step": 8,
            "variable": "syncRemaining",
            "peer": "p1",
            "from": 1,
            "to": 1,
        },
    }
    path = tmp_path / "identity-mutation.ndjson"
    path.write_text(json.dumps(header) + "\n", encoding="utf-8")
    with pytest.raises(trace_verifier.TraceError, match="must differ"):
        trace_verifier.load_trace(path)


def test_trace_spec_invokes_base_actions_and_requires_consumption() -> None:
    source = SPEC.read_text(encoding="utf-8")
    for action in trace_verifier.ACTION_UPDATES:
        assert f'{action}(row.peer)' in source
    for variable in trace_verifier.INITIAL_VARIABLES:
        assert f"{variable}' = row.{variable}" in source
    assert "WF_traceVars(TraceStep)" in source
    assert "EventuallyTraceConsumed == <> (traceIndex = Len(TRACE))" in source


def test_missing_tla_jar_is_a_hard_error(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    result = trace_verifier.main(["--jar", str(tmp_path / "missing.jar")])
    assert result == 2
    assert "TLA+ tools jar not found" in capsys.readouterr().err


def test_default_gate_requires_complete_canonical_manifest(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    trace_dir = tmp_path / "traces"
    trace_dir.mkdir()
    matching_source = trace_verifier.DEFAULT_TRACE_DIR / "matching.ndjson"
    (trace_dir / "matching.ndjson").write_text(
        matching_source.read_text(encoding="utf-8"), encoding="utf-8"
    )
    jar = tmp_path / "tla2tools.jar"
    jar.write_bytes(b"stub")
    monkeypatch.setattr(trace_verifier, "DEFAULT_TRACE_DIR", trace_dir)
    monkeypatch.setattr(
        trace_verifier,
        "run_tlc",
        lambda *args, **kwargs: pytest.fail("TLC must not run with an incomplete manifest"),
    )

    assert trace_verifier.main(["--jar", str(jar)]) == 1
    assert "default trace manifest mismatch" in capsys.readouterr().err


@pytest.mark.parametrize("target", ["mismatch.ndjson", "timeout.ndjson"])
def test_default_manifest_rejects_renamed_matching_scenarios(target: str) -> None:
    cases = _load_default_cases()
    by_filename = {case.path.name: case for case in cases}
    matching = by_filename["matching.ndjson"]
    original = by_filename[target]
    replacement = trace_verifier.TraceCase(
        path=original.path,
        header=original.header,
        rows=matching.rows,
    )
    replaced_cases = [replacement if case.path.name == target else case for case in cases]

    with pytest.raises(trace_verifier.TraceError, match="default (mismatch|timeout) trace"):
        trace_verifier.validate_default_manifest(replaced_cases)


def test_partial_tlc_completion_fails_closed(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    matching = trace_verifier.load_trace(
        trace_verifier.DEFAULT_TRACE_DIR / "matching.ndjson"
    )
    jar = tmp_path / "tla2tools.jar"
    jar.write_bytes(b"stub")

    monkeypatch.setattr(
        trace_verifier.subprocess,
        "run",
        lambda *args, **kwargs: SimpleNamespace(
            returncode=0,
            stdout=(
                "Model checking completed. No error has been found.\n"
                "5 states generated, 4 distinct states found, 1 states left on queue.\n"
            ),
            stderr="",
        ),
    )

    with pytest.raises(trace_verifier.TraceError, match="expected TLC acceptance"):
        trace_verifier.run_tlc(matching, jar, timeout_seconds=1, verbose=False)


def test_accept_rejects_unrelated_tlc_error(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    matching = trace_verifier.load_trace(
        trace_verifier.DEFAULT_TRACE_DIR / "matching.ndjson"
    )
    jar = tmp_path / "tla2tools.jar"
    jar.write_bytes(b"stub")
    monkeypatch.setattr(
        trace_verifier.subprocess,
        "run",
        lambda *args, **kwargs: SimpleNamespace(
            returncode=0,
            stdout=(
                "Error: unrelated TLC diagnostic\n"
                "Model checking completed. No error has been found.\n"
                "5 states generated, 4 distinct states found, 0 states left on queue.\n"
            ),
            stderr="",
        ),
    )

    with pytest.raises(trace_verifier.TraceError, match="expected TLC acceptance"):
        trace_verifier.run_tlc(matching, jar, timeout_seconds=1, verbose=False)


def test_reject_allows_only_exact_counterexample_errors(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    cases = _load_default_cases()
    by_filename = {case.path.name: case for case in cases}
    mutation = trace_verifier.materialize_mutation(
        by_filename["duplicate-reply-decrement.ndjson"], by_filename
    )
    jar = tmp_path / "tla2tools.jar"
    jar.write_bytes(b"stub")
    monkeypatch.setattr(
        trace_verifier.subprocess,
        "run",
        lambda *args, **kwargs: SimpleNamespace(
            returncode=13,
            stdout=(
                "Error: Temporal property EventuallyTraceConsumed was violated.\n"
                "Error: The following behavior constitutes a counter-example:\n"
                "Error: unrelated TLC diagnostic\n"
                "8 states generated, 8 distinct states found, 0 states left on queue.\n"
            ),
            stderr="",
        ),
    )

    with pytest.raises(trace_verifier.TraceError, match="expected only"):
        trace_verifier.run_tlc(mutation, jar, timeout_seconds=1, verbose=False)


@pytest.mark.parametrize(
    ("stdout", "stderr"),
    [
        (
            "Model checking completed. No error has been found.\n"
            "0 states generated, 0 distinct states found, 0 states left on queue.\n",
            "",
        ),
        (
            "Model checking completed. No error has been found.\n"
            "5 states generated, 4 distinct states found, 0 states left on queue.",
            "Error: hidden stderr diagnostic\n",
        ),
        (
            "Not a verdict: Model checking completed. No error has been found.\n"
            "5 states generated, 4 distinct states found, 0 states left on queue.\n",
            "",
        ),
        (
            "Model checking completed. No error has been found.\n"
            "Model checking completed. No error has been found.\n"
            "5 states generated, 4 distinct states found, 0 states left on queue.\n",
            "",
        ),
        (
            "Model checking completed. No error has been found.\n"
            "5 states generated, 4 distinct states found, 0 states left on queue.\n"
            "5 states generated, 4 distinct states found, 0 states left on queue.\n",
            "",
        ),
    ],
)
def test_accept_requires_one_exact_positive_completion(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    stdout: str,
    stderr: str,
) -> None:
    matching = trace_verifier.load_trace(
        trace_verifier.DEFAULT_TRACE_DIR / "matching.ndjson"
    )
    jar = tmp_path / "tla2tools.jar"
    jar.write_bytes(b"stub")
    monkeypatch.setattr(
        trace_verifier.subprocess,
        "run",
        lambda *args, **kwargs: SimpleNamespace(
            returncode=0,
            stdout=stdout,
            stderr=stderr,
        ),
    )

    with pytest.raises(trace_verifier.TraceError, match="expected TLC acceptance"):
        trace_verifier.run_tlc(matching, jar, timeout_seconds=1, verbose=False)
