#!/usr/bin/env python3
"""Validate hand-authored NDJSON traces against SyncHandshakeV1 actions."""

from __future__ import annotations

import argparse
import copy
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
TLA_DIR = ROOT / "specs" / "tla"
DEFAULT_TRACE_DIR = TLA_DIR / "traces" / "sync-handshake-v1"
DEFAULT_JAR = ROOT / ".tla-tools" / "tla2tools.jar"

PEERS = ("p1", "p2")
PLAYER_COUNTS = {2, 3}
INPUT_WIDTHS = {4, 32}
NUM_SYNC_PACKETS = 2
REQUEST_ID_COUNT = 3
TIMEOUT_TICKS = 1
MAX_NETWORK = 3

DEFAULT_MANIFEST = {
    "matching.ndjson": ("matching", "accept"),
    "mismatch.ndjson": ("mismatch", "accept"),
    "timeout.ndjson": ("timeout", "accept"),
    "duplicate-reply-decrement.ndjson": ("duplicate-reply-decrement", "reject"),
}
DEFAULT_REJECT_BASELINE = "matching.ndjson"
DEFAULT_REJECT_MUTATION = {
    "step": 9,
    "variable": "syncRemaining",
    "peer": "p1",
    "from": 1,
    "to": 0,
}
RUNTIME_MANIFEST = {
    "runtime-matching.ndjson": ("runtime-matching", "accept"),
    "runtime-mismatch.ndjson": ("runtime-mismatch", "accept"),
    "runtime-timeout.ndjson": ("runtime-timeout", "accept"),
    "runtime-duplicate-reply-decrement.ndjson": (
        "runtime-duplicate-reply-decrement",
        "reject",
    ),
}
RUNTIME_REJECT_BASELINE = "runtime-matching.ndjson"

INITIAL_VARIABLES = {
    "phase",
    "localConfig",
    "learnedConfig",
    "learnedFrom",
    "syncRemaining",
    "acceptedTokens",
    "nextToken",
    "timeoutTicks",
    "timeoutEventCount",
    "incompatibleEventCount",
    "reasonField",
    "reasonOurs",
    "reasonTheirs",
    "network",
}
PEER_MAPPED_VARIABLES = INITIAL_VARIABLES - {"network"}

ACTION_UPDATES = {
    "SendSyncRequest": {"nextToken", "network"},
    "HandleSyncRequest": {
        "phase",
        "learnedConfig",
        "learnedFrom",
        "incompatibleEventCount",
        "reasonField",
        "reasonOurs",
        "reasonTheirs",
        "network",
    },
    "HandleSyncReply": {
        "phase",
        "learnedConfig",
        "learnedFrom",
        "syncRemaining",
        "acceptedTokens",
        "incompatibleEventCount",
        "reasonField",
        "reasonOurs",
        "reasonTheirs",
        "network",
    },
    "DuplicateMessage": {"network"},
    "TickTimeout": {"timeoutTicks"},
    "ReportSyncTimeout": {"timeoutEventCount"},
}

HEADER_KEYS = {
    "schema",
    "trace",
    "expect",
    "description",
    "derived_from",
    "mutation",
}
ROW_KEYS = {"step", "action", "peer", "state", "updates"}
TRACE_NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
MODULE_NAME_RE = re.compile(r"[^A-Za-z0-9_]")


class TraceError(ValueError):
    """A fail-closed trace schema or execution error."""


@dataclass(frozen=True)
class TraceCase:
    """One parsed trace and its expected TLC disposition."""

    path: Path
    header: dict[str, Any]
    rows: tuple[dict[str, Any], ...]

    @property
    def name(self) -> str:
        return str(self.header["trace"])

    @property
    def expect(self) -> str:
        return str(self.header["expect"])


def _fail(path: Path, line: int, message: str) -> TraceError:
    return TraceError(f"{path}:{line}: {message}")


def _require_exact_keys(
    value: dict[str, Any], expected: set[str], path: Path, line: int, label: str
) -> None:
    actual = set(value)
    if actual != expected:
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        raise _fail(path, line, f"{label} keys mismatch; missing={missing}, extra={extra}")


def _validate_config(value: Any, path: Path, line: int, label: str) -> None:
    if not isinstance(value, dict):
        raise _fail(path, line, f"{label} must be an object")
    _require_exact_keys(value, {"numPlayers", "inputWidth"}, path, line, label)
    if type(value["numPlayers"]) is not int or value["numPlayers"] not in PLAYER_COUNTS:
        raise _fail(path, line, f"{label}.numPlayers must be one of {sorted(PLAYER_COUNTS)}")
    if type(value["inputWidth"]) is not int or value["inputWidth"] not in INPUT_WIDTHS:
        raise _fail(path, line, f"{label}.inputWidth must be one of {sorted(INPUT_WIDTHS)}")


def _validate_peer_map(
    value: Any, path: Path, line: int, label: str, validator: Any
) -> None:
    if not isinstance(value, dict):
        raise _fail(path, line, f"{label} must be an object")
    _require_exact_keys(value, set(PEERS), path, line, label)
    for peer in PEERS:
        validator(value[peer], path, line, f"{label}.{peer}")


def _enum_validator(allowed: set[Any]) -> Any:
    def validate(value: Any, path: Path, line: int, label: str) -> None:
        if value not in allowed or isinstance(value, (dict, list)):
            raise _fail(path, line, f"{label} must be one of {sorted(allowed, key=str)}")

    return validate


def _int_range_validator(low: int, high: int) -> Any:
    def validate(value: Any, path: Path, line: int, label: str) -> None:
        if type(value) is not int or not low <= value <= high:
            raise _fail(path, line, f"{label} must be an integer in {low}..{high}")

    return validate


def _config_or_null(value: Any, path: Path, line: int, label: str) -> None:
    if value is not None:
        _validate_config(value, path, line, label)


def _tokens(value: Any, path: Path, line: int, label: str) -> None:
    if not isinstance(value, list) or any(type(item) is not int for item in value):
        raise _fail(path, line, f"{label} must be an integer array")
    if value != sorted(set(value)) or any(not 1 <= item <= REQUEST_ID_COUNT for item in value):
        raise _fail(
            path,
            line,
            f"{label} must be a sorted unique subset of 1..{REQUEST_ID_COUNT}",
        )


def _reason_value(value: Any, path: Path, line: int, label: str) -> None:
    allowed = PLAYER_COUNTS | INPUT_WIDTHS
    if value is not None and (type(value) is not int or value not in allowed):
        raise _fail(path, line, f"{label} must be null or one of {sorted(allowed)}")


def _validate_network(value: Any, path: Path, line: int, label: str) -> None:
    if not isinstance(value, list) or len(value) > MAX_NETWORK:
        raise _fail(path, line, f"{label} must be an array of at most {MAX_NETWORK} messages")
    for index, message in enumerate(value):
        item_label = f"{label}[{index}]"
        if not isinstance(message, dict):
            raise _fail(path, line, f"{item_label} must be an object")
        _require_exact_keys(
            message, {"kind", "from", "to", "token", "config"}, path, line, item_label
        )
        if message["kind"] not in {"SyncRequest", "SyncReply"}:
            raise _fail(path, line, f"{item_label}.kind is invalid")
        if message["from"] not in PEERS or message["to"] not in PEERS:
            raise _fail(path, line, f"{item_label} endpoints must be p1 or p2")
        if message["from"] == message["to"]:
            raise _fail(path, line, f"{item_label} endpoints must differ")
        if type(message["token"]) is not int or not 1 <= message["token"] <= REQUEST_ID_COUNT:
            raise _fail(path, line, f"{item_label}.token must be in 1..{REQUEST_ID_COUNT}")
        _validate_config(message["config"], path, line, f"{item_label}.config")


def _validate_variable(name: str, value: Any, path: Path, line: int) -> None:
    validators = {
        "phase": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _enum_validator({"Syncing", "Synced", "Failed"})
        ),
        "localConfig": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _validate_config
        ),
        "learnedConfig": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _config_or_null
        ),
        "learnedFrom": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _enum_validator({None, *PEERS})
        ),
        "syncRemaining": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _int_range_validator(0, NUM_SYNC_PACKETS)
        ),
        "acceptedTokens": lambda v, p, n, label: _validate_peer_map(v, p, n, label, _tokens),
        "nextToken": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _int_range_validator(1, REQUEST_ID_COUNT + 1)
        ),
        "timeoutTicks": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _int_range_validator(0, TIMEOUT_TICKS)
        ),
        "timeoutEventCount": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _int_range_validator(0, 1)
        ),
        "incompatibleEventCount": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _int_range_validator(0, 1)
        ),
        "reasonField": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _enum_validator({"None", "NumPlayers", "InputWidth"})
        ),
        "reasonOurs": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _reason_value
        ),
        "reasonTheirs": lambda v, p, n, label: _validate_peer_map(
            v, p, n, label, _reason_value
        ),
        "network": _validate_network,
    }
    validators[name](value, path, line, name)


def load_trace(path: Path) -> TraceCase:
    """Parse and strictly validate one NDJSON trace."""

    records: list[dict[str, Any]] = []
    try:
        raw_lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise TraceError(f"{path}: unable to read trace: {error}") from error

    for line_number, raw in enumerate(raw_lines, start=1):
        if not raw.strip():
            raise _fail(path, line_number, "blank lines are not allowed in NDJSON traces")
        try:
            value = json.loads(raw)
        except json.JSONDecodeError as error:
            raise _fail(path, line_number, f"invalid JSON: {error.msg}") from error
        if not isinstance(value, dict):
            raise _fail(path, line_number, "each NDJSON record must be an object")
        records.append(value)

    if not records:
        raise TraceError(f"{path}: trace is empty")

    header = records[0]
    extra_header = set(header) - HEADER_KEYS
    if extra_header:
        raise _fail(path, 1, f"unknown header keys: {sorted(extra_header)}")
    for required in ("schema", "trace", "expect", "description"):
        if required not in header:
            raise _fail(path, 1, f"missing header key {required!r}")
    if type(header["schema"]) is not int or header["schema"] != 1:
        raise _fail(path, 1, "schema must equal 1")
    if not isinstance(header["trace"], str) or not TRACE_NAME_RE.fullmatch(header["trace"]):
        raise _fail(path, 1, "trace must be a lowercase hyphenated name")
    if header["expect"] not in {"accept", "reject"}:
        raise _fail(path, 1, "expect must be 'accept' or 'reject'")
    if not isinstance(header["description"], str) or not header["description"].strip():
        raise _fail(path, 1, "description must be a non-empty string")

    if header["expect"] == "reject":
        for required in ("derived_from", "mutation"):
            if required not in header:
                raise _fail(path, 1, f"reject trace requires {required!r}")
        mutation = header["mutation"]
        if not isinstance(mutation, dict):
            raise _fail(path, 1, "mutation must be an object")
        _require_exact_keys(mutation, {"step", "variable", "peer", "from", "to"}, path, 1, "mutation")
        if type(mutation["step"]) is not int or mutation["step"] <= 0:
            raise _fail(path, 1, "mutation.step must be a positive integer")
        if mutation["variable"] not in PEER_MAPPED_VARIABLES:
            raise _fail(path, 1, "mutation.variable must name a peer-mapped variable")
        if mutation["peer"] not in PEERS:
            raise _fail(path, 1, "mutation.peer must be p1 or p2")
        if mutation["from"] == mutation["to"]:
            raise _fail(path, 1, "mutation.from and mutation.to must differ")
    elif "derived_from" in header or "mutation" in header:
        raise _fail(path, 1, "accept traces cannot declare mutation metadata")

    rows = records[1:]
    if header["expect"] == "accept" and not rows:
        raise TraceError(f"{path}: accept trace requires at least one state row")
    if header["expect"] == "reject" and rows:
        raise TraceError(f"{path}: reject trace must derive all rows from its accepted baseline")
    for index, row in enumerate(rows):
        line_number = index + 2
        extra = set(row) - ROW_KEYS
        if extra:
            raise _fail(path, line_number, f"unknown row keys: {sorted(extra)}")
        if row.get("step") != index:
            raise _fail(path, line_number, f"step must equal {index}")
        if index == 0:
            _require_exact_keys(row, {"step", "action", "state"}, path, line_number, "initial row")
            if row["action"] != "Init" or not isinstance(row["state"], dict):
                raise _fail(path, line_number, "step 0 must be an Init row with state")
            _require_exact_keys(row["state"], INITIAL_VARIABLES, path, line_number, "initial state")
            variables = row["state"]
        else:
            _require_exact_keys(
                row, {"step", "action", "peer", "updates"}, path, line_number, "action row"
            )
            action = row["action"]
            if action not in ACTION_UPDATES:
                raise _fail(path, line_number, f"unknown action {action!r}")
            if row["peer"] not in PEERS:
                raise _fail(path, line_number, "peer must be p1 or p2")
            if not isinstance(row["updates"], dict):
                raise _fail(path, line_number, "updates must be an object")
            _require_exact_keys(
                row["updates"], ACTION_UPDATES[action], path, line_number, f"{action} updates"
            )
            variables = row["updates"]
        for name, value in variables.items():
            _validate_variable(name, value, path, line_number)

    return TraceCase(path=path, header=header, rows=tuple(rows))


def materialize_mutation(
    case: TraceCase, cases_by_filename: dict[str, TraceCase]
) -> TraceCase:
    """Build a reject trace by applying exactly its declared baseline mutation."""

    if case.expect != "reject":
        return case
    derived_name = case.header["derived_from"]
    if not isinstance(derived_name, str) or derived_name not in cases_by_filename:
        raise TraceError(f"{case.path}: derived_from must name a loaded trace file")
    baseline = cases_by_filename[derived_name]
    if baseline.expect != "accept":
        raise TraceError(f"{case.path}: derived trace {derived_name} is not expected to pass")

    mutation = case.header["mutation"]
    step = mutation["step"]
    if step >= len(baseline.rows):
        raise TraceError(f"{case.path}: mutation step does not exist in {derived_name}")

    mutated_rows = copy.deepcopy(list(baseline.rows))
    row = mutated_rows[step]
    variable = mutation["variable"]
    peer = mutation["peer"]
    try:
        current = row["updates"][variable][peer]
    except (KeyError, TypeError) as error:
        raise TraceError(f"{case.path}: declared mutation path does not exist") from error
    if current != mutation["from"]:
        raise TraceError(f"{case.path}: mutation.from does not match the accepted trace")
    row["updates"][variable][peer] = mutation["to"]
    _validate_variable(variable, row["updates"][variable], case.path, 1)
    return TraceCase(path=case.path, header=case.header, rows=tuple(mutated_rows))


def expand_trace_states(case: TraceCase) -> tuple[dict[str, Any], ...]:
    """Expand strict action deltas into complete state snapshots."""

    state: dict[str, Any] = {}
    states = []
    for row in case.rows:
        if row["step"] == 0:
            state = copy.deepcopy(row["state"])
        else:
            state.update(copy.deepcopy(row["updates"]))
        states.append(copy.deepcopy(state))
    return tuple(states)


def _validate_matching_semantics(case: TraceCase) -> None:
    states = expand_trace_states(case)
    actions = [(row["action"], row.get("peer")) for row in case.rows]
    expected_prefix = [
        ("Init", None),
        ("SendSyncRequest", "p1"),
        ("HandleSyncRequest", "p2"),
        ("SendSyncRequest", "p1"),
        ("HandleSyncRequest", "p2"),
        ("DuplicateMessage", "p1"),
        ("HandleSyncReply", "p1"),
        ("SendSyncRequest", "p1"),
        ("HandleSyncRequest", "p2"),
        ("HandleSyncReply", "p1"),
        ("HandleSyncReply", "p1"),
        ("HandleSyncReply", "p1"),
    ]
    if len(states) <= 11 or actions[:12] != expected_prefix:
        raise TraceError("default matching trace must exercise a duplicated reply")
    duplicated = states[5]["network"]
    before_duplicate_reply = states[8]
    after_duplicate_reply = states[9]
    sent_request_ids = [
        message["token"]
        for state in states[:9]
        for message in state["network"]
        if message["kind"] == "SyncRequest" and message["from"] == "p1"
    ]
    if (
        sorted(set(sent_request_ids)) != [1, 2, 3]
        or states[7]["nextToken"]["p1"] != 4
        or [message["token"] for message in duplicated] != [1, 2, 1]
        or duplicated[0] != duplicated[2]
        or before_duplicate_reply["acceptedTokens"]["p1"] != [1]
        or before_duplicate_reply["syncRemaining"]["p1"] != 1
        or [message["token"] for message in before_duplicate_reply["network"]] != [2, 1, 3]
        or after_duplicate_reply["acceptedTokens"]["p1"] != [1]
        or after_duplicate_reply["syncRemaining"]["p1"] != 1
        or [message["token"] for message in after_duplicate_reply["network"]] != [2, 3]
        or states[10]["acceptedTokens"]["p1"] != [1, 3]
        or states[10]["syncRemaining"]["p1"] != 0
        or [message["token"] for message in states[10]["network"]] != [2]
        or states[11]["acceptedTokens"]["p1"] != [1, 3]
        or states[-1]["phase"] != {"p1": "Synced", "p2": "Synced"}
    ):
        raise TraceError(
            "default matching trace must preserve fresh request IDs, keep the duplicated "
            "reply idempotent, and sync both peers"
        )


def _validate_mismatch_semantics(case: TraceCase) -> None:
    actions = [(row["action"], row.get("peer")) for row in case.rows]
    states = expand_trace_states(case)
    expected_actions = [
        ("Init", None),
        ("SendSyncRequest", "p1"),
        ("HandleSyncRequest", "p2"),
        ("HandleSyncReply", "p1"),
    ]
    expected_counts = [
        {"p1": 0, "p2": 0},
        {"p1": 0, "p2": 0},
        {"p1": 0, "p2": 1},
        {"p1": 1, "p2": 1},
    ]
    final = states[-1] if states else {}
    if (
        actions != expected_actions
        or [state["incompatibleEventCount"] for state in states] != expected_counts
        or final.get("phase") != {"p1": "Failed", "p2": "Failed"}
        or final.get("reasonField") != {"p1": "NumPlayers", "p2": "NumPlayers"}
        or final.get("reasonOurs") != {"p1": 2, "p2": 3}
        or final.get("reasonTheirs") != {"p1": 3, "p2": 2}
    ):
        raise TraceError(
            "default mismatch trace must fail both peers once with oriented NumPlayers reasons"
        )


def _validate_timeout_semantics(case: TraceCase) -> None:
    actions = [(row["action"], row.get("peer")) for row in case.rows]
    states = expand_trace_states(case)
    expected_actions = [
        ("Init", None),
        ("TickTimeout", "p1"),
        ("ReportSyncTimeout", "p1"),
        ("SendSyncRequest", "p1"),
    ]
    if (
        actions != expected_actions
        or [state["phase"] for state in states]
        != [{"p1": "Syncing", "p2": "Syncing"}] * 4
        or [state["timeoutTicks"]["p1"] for state in states] != [0, 1, 1, 1]
        or [state["timeoutEventCount"]["p1"] for state in states] != [0, 0, 1, 1]
        or len(states[-1]["network"]) != 1
        or states[-1]["network"][0]["kind"] != "SyncRequest"
        or states[-1]["network"][0]["from"] != "p1"
        or states[-1]["network"][0]["to"] != "p2"
    ):
        raise TraceError(
            "default timeout trace must tick, report once, stay Syncing, and retry"
        )


def validate_default_manifest(cases: list[TraceCase]) -> None:
    """Require the complete canonical positive/negative trace gate."""

    by_filename = {case.path.name: case for case in cases}
    actual = set(by_filename)
    expected = set(DEFAULT_MANIFEST)
    if actual != expected:
        raise TraceError(
            "default trace manifest mismatch; "
            f"missing={sorted(expected - actual)}, extra={sorted(actual - expected)}"
        )
    for filename, (name, disposition) in DEFAULT_MANIFEST.items():
        case = by_filename[filename]
        if (case.name, case.expect) != (name, disposition):
            raise TraceError(
                f"default trace manifest entry {filename} must be "
                f"trace={name!r}, expect={disposition!r}"
            )
    reject = by_filename["duplicate-reply-decrement.ndjson"]
    if (
        reject.header.get("derived_from") != DEFAULT_REJECT_BASELINE
        or reject.header.get("mutation") != DEFAULT_REJECT_MUTATION
    ):
        raise TraceError(
            "default duplicate-reply-decrement mutation metadata does not match the contract"
        )
    _validate_matching_semantics(by_filename["matching.ndjson"])
    _validate_mismatch_semantics(by_filename["mismatch.ndjson"])
    _validate_timeout_semantics(by_filename["timeout.ndjson"])


def generate_runtime_traces(output_dir: Path) -> None:
    """Run the feature-gated SimNet producer and require its complete output."""

    environment = os.environ.copy()
    environment["FORTRESS_RUNTIME_TRACE_DIR"] = str(output_dir)
    environment["CARGO_TERM_COLOR"] = "never"
    command = [
        "cargo",
        "test",
        "--test",
        "simulation",
        "--features",
        "trace-validation",
        "simulation::trace_validation::export_runtime_handshake_traces_for_tlc",
        "--",
        "--exact",
        "--nocapture",
    ]
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            env=environment,
            capture_output=True,
            text=True,
            timeout=180,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise TraceError(f"runtime trace producer failed to execute: {error}") from error
    if result.returncode != 0:
        output = result.stdout.rstrip("\n") + "\n" + result.stderr.rstrip("\n")
        raise TraceError(
            f"runtime trace producer failed (exit {result.returncode})\n{output}"
        )

    actual = {path.name for path in output_dir.glob("*.ndjson")}
    expected = set(RUNTIME_MANIFEST)
    if actual != expected:
        raise TraceError(
            "runtime trace producer manifest mismatch; "
            f"missing={sorted(expected - actual)}, extra={sorted(actual - expected)}"
        )


def validate_runtime_manifest(cases: list[TraceCase]) -> None:
    """Require all runtime scenarios and the observed duplicate mutation."""

    by_filename = {case.path.name: case for case in cases}
    actual = set(by_filename)
    expected = set(RUNTIME_MANIFEST)
    if actual != expected:
        raise TraceError(
            "runtime trace manifest mismatch; "
            f"missing={sorted(expected - actual)}, extra={sorted(actual - expected)}"
        )
    for filename, (name, disposition) in RUNTIME_MANIFEST.items():
        case = by_filename[filename]
        if (case.name, case.expect) != (name, disposition):
            raise TraceError(
                f"runtime trace manifest entry {filename} must be "
                f"trace={name!r}, expect={disposition!r}"
            )

    matching = by_filename["runtime-matching.ndjson"]
    states = expand_trace_states(matching)
    duplicate_steps = [
        index
        for index, row in enumerate(matching.rows)
        if row["action"] == "HandleSyncReply"
        and row["peer"] == "p1"
        and index > 0
        and states[index]["acceptedTokens"]["p1"]
        == states[index - 1]["acceptedTokens"]["p1"]
        and states[index]["syncRemaining"]["p1"]
        == states[index - 1]["syncRemaining"]["p1"]
    ]
    if len(duplicate_steps) != 1:
        raise TraceError("runtime matching trace must contain exactly one ignored p1 reply")
    duplicate_step = duplicate_steps[0]
    if not any(row["action"] == "DuplicateMessage" for row in matching.rows):
        raise TraceError("runtime matching trace must reconstruct one network duplication")
    if states[-1]["phase"] != {"p1": "Synced", "p2": "Synced"}:
        raise TraceError("runtime matching trace must end with both peers synchronized")
    if states[-1]["acceptedTokens"]["p1"] != [1, 3]:
        raise TraceError("runtime matching trace must accept fresh C while B stays outstanding")
    if not any(
        message["kind"] == "SyncReply"
        and message["to"] == "p1"
        and message["token"] == 2
        for message in states[-1]["network"]
    ):
        raise TraceError("runtime matching trace must retain delayed reply B")

    reject = by_filename["runtime-duplicate-reply-decrement.ndjson"]
    mutation = reject.header.get("mutation")
    if reject.header.get("derived_from") != RUNTIME_REJECT_BASELINE or mutation != {
        "step": duplicate_step,
        "variable": "syncRemaining",
        "peer": "p1",
        "from": 1,
        "to": 0,
    }:
        raise TraceError("runtime reject mutation must derive from the observed duplicate row")

    mismatch_states = expand_trace_states(by_filename["runtime-mismatch.ndjson"])
    if mismatch_states[-1]["phase"] != {"p1": "Failed", "p2": "Failed"}:
        raise TraceError("runtime mismatch trace must fail both peers")
    timeout_actions = [
        row["action"] for row in by_filename["runtime-timeout.ndjson"].rows
    ]
    if "ReportSyncTimeout" not in timeout_actions or timeout_actions[-1] != "SendSyncRequest":
        raise TraceError("runtime timeout trace must report once and then retry")


def _tla_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def _peer_expr(peer: str) -> str:
    return "TRACE_P1" if peer == "p1" else "TRACE_P2"


def _config_expr(config: dict[str, int]) -> str:
    return (
        "[numPlayers |-> "
        f"{config['numPlayers']}, inputWidth |-> {config['inputWidth']}]"
    )


def _peer_map_expr(value: dict[str, Any], item: Any) -> str:
    return f"[p \\in PEERS |-> IF p = TRACE_P1 THEN {item(value['p1'])} ELSE {item(value['p2'])}]"


def _network_expr(value: list[dict[str, Any]]) -> str:
    messages = []
    for message in value:
        messages.append(
            "[kind |-> "
            f"{_tla_string(message['kind'])}, from |-> {_peer_expr(message['from'])}, "
            f"to |-> {_peer_expr(message['to'])}, token |-> {message['token']}, "
            f"config |-> {_config_expr(message['config'])}]"
        )
    return "<<" + ", ".join(messages) + ">>"


def _variable_expr(name: str, value: Any) -> str:
    if name in {"phase", "reasonField"}:
        return _peer_map_expr(value, _tla_string)
    if name == "localConfig":
        return _peer_map_expr(value, _config_expr)
    if name == "learnedConfig":
        return _peer_map_expr(value, lambda item: "NO_CONFIG" if item is None else _config_expr(item))
    if name == "learnedFrom":
        return _peer_map_expr(value, lambda item: "NO_PEER" if item is None else _peer_expr(item))
    if name in {
        "syncRemaining",
        "nextToken",
        "timeoutTicks",
        "timeoutEventCount",
        "incompatibleEventCount",
    }:
        return _peer_map_expr(value, str)
    if name == "acceptedTokens":
        return _peer_map_expr(value, lambda items: "{" + ", ".join(map(str, items)) + "}")
    if name in {"reasonOurs", "reasonTheirs"}:
        return _peer_map_expr(value, lambda item: "NO_VALUE" if item is None else str(item))
    if name == "network":
        return _network_expr(value)
    raise AssertionError(f"unhandled variable {name}")


def render_module(case: TraceCase, module_name: str) -> str:
    """Render a generated module that supplies the validated TraceData operator."""

    records = []
    for row, state in zip(case.rows, expand_trace_states(case), strict=True):
        fields = [f"action |-> {_tla_string(row['action'])}"]
        if row["step"] > 0:
            fields.append(f"peer |-> {_peer_expr(row['peer'])}")
        fields.extend(
            f"{name} |-> {_variable_expr(name, value)}" for name, value in state.items()
        )
        records.append("    [" + ",\n     ".join(fields) + "]")

    return (
        f"---------------- MODULE {module_name} ----------------\n"
        "EXTENDS SyncHandshakeV1Trace\n\n"
        "CONSTANTS TRACE_P1, TRACE_P2\n\n"
        "ASSUME /\\ TRACE_P1 \\in PEERS\n"
        "       /\\ TRACE_P2 \\in PEERS\n"
        "       /\\ TRACE_P1 # TRACE_P2\n\n"
        "TraceData == <<\n"
        + ",\n".join(records)
        + "\n>>\n\n"
        "=============================================================================\n"
    )


def render_config() -> str:
    """Render the fixed small-bounds config used by every hand-authored trace."""

    return """SPECIFICATION TraceSpec

CONSTANT PEERS = {p1, p2}
CONSTANT PLAYER_COUNTS = {2, 3}
CONSTANT INPUT_WIDTHS = {4, 32}
CONSTANT NUM_SYNC_PACKETS = 2
CONSTANT REQUEST_ID_COUNT = 3
CONSTANT TIMEOUT_TICKS = 1
CONSTANT MAX_NETWORK = 3
CONSTANT DELIVERY_MODE = "TraceDelivery"
CONSTANT CONFIG_MODE = "All"
CONSTANT NO_CONFIG = NoConfig
CONSTANT NO_PEER = NoPeer
CONSTANT NO_VALUE = NoValue
CONSTANT TRACE_P1 = p1
CONSTANT TRACE_P2 = p2
CONSTANT TRACE <- TraceData

INVARIANT TypeInvariant
INVARIANT TraceTypeInvariant

PROPERTY EventuallyTraceConsumed
"""


def run_tlc(case: TraceCase, jar: Path, timeout_seconds: int, verbose: bool) -> float:
    """Run TLC and enforce the case's exact expected disposition."""

    safe_name = MODULE_NAME_RE.sub("_", case.name.title().replace("-", "_"))
    file_descriptor, module_path_raw = tempfile.mkstemp(
        prefix=f"GeneratedSyncTrace_{safe_name}_", suffix=".tla", dir=TLA_DIR
    )
    os.close(file_descriptor)
    module_path = Path(module_path_raw)
    module_name = module_path.stem
    config_path = module_path.with_suffix(".cfg")
    try:
        module_path.write_text(render_module(case, module_name), encoding="utf-8")
        config_path.write_text(render_config(), encoding="utf-8")

        with tempfile.TemporaryDirectory(prefix="sync_trace_states_") as metadir:
            command = [
                "java",
                "-Xmx1g",
                "-XX:+UseParallelGC",
                "-jar",
                str(jar),
                "-workers",
                "1",
                "-deadlock",
                "-lncheck",
                "final",
                "-metadir",
                metadir,
                "-config",
                str(config_path),
                str(module_path),
            ]
            started = time.monotonic()
            try:
                result = subprocess.run(
                    command,
                    cwd=TLA_DIR,
                    capture_output=True,
                    text=True,
                    timeout=timeout_seconds,
                    check=False,
                )
            except subprocess.TimeoutExpired as error:
                raise TraceError(f"{case.path}: TLC timed out after {timeout_seconds}s") from error
            duration = time.monotonic() - started
            output = result.stdout.rstrip("\n") + "\n" + result.stderr.rstrip("\n")
    finally:
        module_path.unlink(missing_ok=True)
        config_path.unlink(missing_ok=True)
        for trace_explorer_path in TLA_DIR.glob(f"{module_name}_TTrace_*.tla"):
            trace_explorer_path.unlink(missing_ok=True)
        for trace_explorer_bin in TLA_DIR.glob(f"{module_name}_TTrace_*.bin"):
            trace_explorer_bin.unlink(missing_ok=True)

    success_line = "Model checking completed. No error has been found."
    expected_failure = "Error: Temporal property EventuallyTraceConsumed was violated."
    expected_counterexample = "Error: The following behavior constitutes a counter-example:"
    error_lines = re.findall(r"^[ \t]*Error:.*$", output, re.MULTILINE)
    success_lines = re.findall(
        rf"^{re.escape(success_line)}$", output, re.MULTILINE
    )
    complete_summaries = re.findall(
        r"^[1-9][0-9,]* states generated, [0-9][0-9,]* distinct states found, "
        r"0 states left on queue\.$",
        output,
        re.MULTILINE,
    )
    incomplete_summaries = re.findall(
        r"^[1-9][0-9,]* states generated, [0-9][0-9,]* distinct states found, "
        r"[1-9][0-9,]* states left on queue\.$",
        output,
        re.MULTILINE,
    )
    tool_error_markers = (
        "Parsing or semantic analysis failed",
        "Unable to access jarfile",
        "Exception in thread",
        "TLC threw",
    )
    if verbose:
        print(output, end="" if output.endswith("\n") else "\n")

    if case.expect == "accept":
        if (
            result.returncode != 0
            or len(success_lines) != 1
            or len(complete_summaries) != 1
            or incomplete_summaries
            or result.stderr.strip()
            or error_lines
            or any(marker in output for marker in tool_error_markers)
        ):
            raise TraceError(
                f"{case.path}: expected TLC acceptance (exit {result.returncode})\n"
                + "\n".join(output.splitlines()[-25:])
            )
    else:
        if (
            result.returncode != 13
            or error_lines != [expected_failure, expected_counterexample]
            or success_lines
            or len(complete_summaries) != 1
            or incomplete_summaries
            or result.stderr.strip()
            or any(marker in output for marker in tool_error_markers)
        ):
            raise TraceError(
                f"{case.path}: expected only the trace-consumption counterexample "
                f"(exit {result.returncode})\n" + "\n".join(output.splitlines()[-25:])
            )
    return duration


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("traces", nargs="*", type=Path, help="NDJSON traces (default: all)")
    parser.add_argument("--jar", type=Path, default=DEFAULT_JAR, help="path to tla2tools.jar")
    parser.add_argument(
        "--timeout", type=int, default=int(os.environ.get("TLA_TRACE_TIMEOUT", "60"))
    )
    parser.add_argument("--verbose", action="store_true", help="print complete TLC output")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    if args.timeout <= 0:
        print("error: --timeout must be positive", file=sys.stderr)
        return 2
    jar = args.jar.resolve()
    if not jar.is_file() or jar.stat().st_size == 0:
        print(
            f"error: TLA+ tools jar not found at {jar}; run verify-tla.sh once to download it",
            file=sys.stderr,
        )
        return 2

    using_default_manifest = not args.traces
    paths = [path.resolve() for path in args.traces]
    if using_default_manifest:
        paths = sorted(DEFAULT_TRACE_DIR.glob("*.ndjson"))
    if not paths:
        print("error: no trace files selected", file=sys.stderr)
        return 2

    runtime_temp: tempfile.TemporaryDirectory[str] | None = None
    try:
        cases = [load_trace(path) for path in paths]
        if using_default_manifest:
            validate_default_manifest(cases)
            runtime_temp = tempfile.TemporaryDirectory(prefix="fortress_runtime_sync_traces_")
            runtime_dir = Path(runtime_temp.name)
            generate_runtime_traces(runtime_dir)
            runtime_cases = [
                load_trace(runtime_dir / filename) for filename in sorted(RUNTIME_MANIFEST)
            ]
            validate_runtime_manifest(runtime_cases)
            cases.extend(runtime_cases)
        names = [case.name for case in cases]
        if len(names) != len(set(names)):
            raise TraceError("trace names must be unique")
        filenames = [case.path.name for case in cases]
        if len(filenames) != len(set(filenames)):
            raise TraceError("trace filenames must be unique")
        cases_by_filename = {case.path.name: case for case in cases}
        cases = [materialize_mutation(case, cases_by_filename) for case in cases]
        # Accepted baselines run first so every reject verdict is paired with a
        # green behavior that differs only by its declared mutation.
        cases.sort(key=lambda case: case.expect == "reject")
        for case in cases:
            duration = run_tlc(case, jar, args.timeout, args.verbose)
            disposition = "accepted" if case.expect == "accept" else "rejected as required"
            print(
                f"PASS {case.name}: {disposition} "
                f"({len(case.rows)} trace rows, {duration:.3f}s)"
            )
    except TraceError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    finally:
        if runtime_temp is not None:
            runtime_temp.cleanup()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
