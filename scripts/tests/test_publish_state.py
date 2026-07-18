#!/usr/bin/env python3
"""Tests for checksum-based, retry-safe crates.io publication."""

from __future__ import annotations

import importlib.util
import io
import json
import subprocess
import sys
import urllib.error
from pathlib import Path
from unittest.mock import MagicMock

import pytest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "release" / "publish_state.py"
SPEC = importlib.util.spec_from_file_location("publish_state", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
publish_state = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = publish_state
SPEC.loader.exec_module(publish_state)

CHECKSUM = "a" * 64


def _response(document: object) -> MagicMock:
    response = MagicMock()
    response.__enter__.return_value = response
    response.__exit__.return_value = False
    response.read.return_value = json.dumps(document).encode("utf-8")
    return response


def test_probe_registry_classifies_absent_matching_and_conflict(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    not_found = urllib.error.HTTPError("url", 404, "not found", {}, io.BytesIO())
    monkeypatch.setattr(publish_state.urllib.request, "urlopen", MagicMock(side_effect=not_found))
    assert publish_state.probe_registry("crate", "1.2.3", CHECKSUM).state is publish_state.RegistryState.ABSENT

    monkeypatch.setattr(
        publish_state.urllib.request,
        "urlopen",
        MagicMock(return_value=_response({"version": {"checksum": CHECKSUM}})),
    )
    assert publish_state.probe_registry("crate", "1.2.3", CHECKSUM).state is publish_state.RegistryState.MATCHING

    monkeypatch.setattr(
        publish_state.urllib.request,
        "urlopen",
        MagicMock(return_value=_response({"version": {"checksum": "b" * 64}})),
    )
    result = publish_state.probe_registry("crate", "1.2.3", CHECKSUM)
    assert result.state is publish_state.RegistryState.CONFLICT
    assert result.published_checksum == "b" * 64


@pytest.mark.parametrize(
    "document",
    [{}, {"version": {}}, {"version": {"checksum": None}}],
)
def test_probe_registry_fails_closed_on_malformed_metadata(
    monkeypatch: pytest.MonkeyPatch, document: object
) -> None:
    monkeypatch.setattr(
        publish_state.urllib.request, "urlopen", MagicMock(return_value=_response(document))
    )

    with pytest.raises(publish_state.PublishError, match="malformed|empty checksum"):
        publish_state.probe_registry("crate", "1.2.3", CHECKSUM)


def test_existing_matching_version_skips_token_and_cargo(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.delenv("CARGO_REGISTRY_TOKEN", raising=False)
    monkeypatch.setattr(
        publish_state,
        "probe_registry",
        MagicMock(return_value=publish_state.RegistryResult(publish_state.RegistryState.MATCHING, CHECKSUM)),
    )
    cargo = MagicMock()
    monkeypatch.setattr(publish_state.subprocess, "run", cargo)

    outcome = publish_state.reconcile_publish("crate", "1.2.3", CHECKSUM)

    assert outcome == "already-published"
    cargo.assert_not_called()


def test_absent_version_requires_repository_token(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.delenv("CARGO_REGISTRY_TOKEN", raising=False)
    monkeypatch.setattr(
        publish_state,
        "probe_registry",
        MagicMock(return_value=publish_state.RegistryResult(publish_state.RegistryState.ABSENT)),
    )

    with pytest.raises(publish_state.PublishError, match="CRATES_IO_TOKEN"):
        publish_state.reconcile_publish("crate", "1.2.3", CHECKSUM)


def test_cargo_timeout_after_accepted_upload_is_success(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "test-token")
    probes = iter(
        [
            publish_state.RegistryResult(publish_state.RegistryState.ABSENT),
            publish_state.RegistryResult(publish_state.RegistryState.MATCHING, CHECKSUM),
        ]
    )
    monkeypatch.setattr(publish_state, "probe_registry", MagicMock(side_effect=lambda *_args, **_kwargs: next(probes)))
    monkeypatch.setattr(
        publish_state.subprocess,
        "run",
        MagicMock(return_value=subprocess.CompletedProcess(["cargo"], 101)),
    )
    monkeypatch.setattr(publish_state.time, "sleep", MagicMock())

    outcome = publish_state.reconcile_publish("crate", "1.2.3", CHECKSUM)

    assert outcome == "published"


def test_cargo_publish_is_locked_and_bound_to_crates_io(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "test-token")
    probes = iter(
        [
            publish_state.RegistryResult(publish_state.RegistryState.ABSENT),
            publish_state.RegistryResult(publish_state.RegistryState.MATCHING, CHECKSUM),
        ]
    )
    monkeypatch.setattr(
        publish_state,
        "probe_registry",
        MagicMock(side_effect=lambda *_args, **_kwargs: next(probes)),
    )
    cargo = MagicMock(return_value=subprocess.CompletedProcess(["cargo"], 0))
    monkeypatch.setattr(publish_state.subprocess, "run", cargo)

    assert publish_state.reconcile_publish("crate", "1.2.3", CHECKSUM) == "published"
    cargo.assert_called_once_with(
        ["cargo", "publish", "--locked", "--registry", "crates-io"],
        check=False,
        timeout=publish_state.DEFAULT_CARGO_TIMEOUT,
    )


def test_subprocess_timeout_after_accepted_upload_is_success(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "test-token")
    probes = iter(
        [
            publish_state.RegistryResult(publish_state.RegistryState.ABSENT),
            publish_state.RegistryResult(publish_state.RegistryState.MATCHING, CHECKSUM),
        ]
    )
    monkeypatch.setattr(
        publish_state,
        "probe_registry",
        MagicMock(side_effect=lambda *_args, **_kwargs: next(probes)),
    )
    monkeypatch.setattr(
        publish_state.subprocess,
        "run",
        MagicMock(side_effect=subprocess.TimeoutExpired(["cargo"], 600)),
    )
    monkeypatch.setattr(publish_state.time, "sleep", MagicMock())

    assert publish_state.reconcile_publish("crate", "1.2.3", CHECKSUM) == "published"


def test_delayed_registry_visibility_retries_transient_failures(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "test-token")
    probes = iter(
        [
            publish_state.RegistryResult(publish_state.RegistryState.ABSENT),
            publish_state.RegistryResult(publish_state.RegistryState.ABSENT),
            publish_state.PublishError("HTTP 503"),
            publish_state.RegistryResult(publish_state.RegistryState.MATCHING, CHECKSUM),
        ]
    )

    def probe(*_args: object, **_kwargs: object) -> object:
        result = next(probes)
        if isinstance(result, Exception):
            raise result
        return result

    monkeypatch.setattr(publish_state, "probe_registry", probe)
    monkeypatch.setattr(
        publish_state.subprocess,
        "run",
        MagicMock(return_value=subprocess.CompletedProcess(["cargo"], 0)),
    )
    sleep = MagicMock()
    monkeypatch.setattr(publish_state.time, "sleep", sleep)

    assert publish_state.reconcile_publish("crate", "1.2.3", CHECKSUM) == "published"
    assert sleep.call_count == 2


def test_initial_registry_failure_is_retried_before_publishing(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "test-token")
    probes = iter(
        [
            publish_state.PublishError("HTTP 503"),
            publish_state.RegistryResult(publish_state.RegistryState.ABSENT),
            publish_state.RegistryResult(publish_state.RegistryState.MATCHING, CHECKSUM),
        ]
    )

    def probe(*_args: object, **_kwargs: object) -> object:
        result = next(probes)
        if isinstance(result, Exception):
            raise result
        return result

    monkeypatch.setattr(publish_state, "probe_registry", probe)
    monkeypatch.setattr(
        publish_state.subprocess,
        "run",
        MagicMock(return_value=subprocess.CompletedProcess(["cargo"], 0)),
    )
    sleep = MagicMock()
    monkeypatch.setattr(publish_state.time, "sleep", sleep)

    assert publish_state.reconcile_publish("crate", "1.2.3", CHECKSUM) == "published"
    sleep.assert_called_once_with(1.0)


def test_conflicting_checksum_fails_before_or_after_publish(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    conflict = publish_state.RegistryResult(publish_state.RegistryState.CONFLICT, "b" * 64)
    monkeypatch.setattr(publish_state, "probe_registry", MagicMock(return_value=conflict))
    with pytest.raises(publish_state.PublishError, match="exists with checksum"):
        publish_state.reconcile_publish("crate", "1.2.3", CHECKSUM)

    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "test-token")
    probes = iter([publish_state.RegistryResult(publish_state.RegistryState.ABSENT), conflict])
    monkeypatch.setattr(publish_state, "probe_registry", MagicMock(side_effect=lambda *_args, **_kwargs: next(probes)))
    monkeypatch.setattr(
        publish_state.subprocess,
        "run",
        MagicMock(return_value=subprocess.CompletedProcess(["cargo"], 0)),
    )
    with pytest.raises(publish_state.PublishError, match="crates.io exposed"):
        publish_state.reconcile_publish("crate", "1.2.3", CHECKSUM)


def test_registry_never_visible_reports_cargo_result(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "test-token")
    monkeypatch.setattr(
        publish_state,
        "probe_registry",
        MagicMock(return_value=publish_state.RegistryResult(publish_state.RegistryState.ABSENT)),
    )
    monkeypatch.setattr(
        publish_state.subprocess,
        "run",
        MagicMock(return_value=subprocess.CompletedProcess(["cargo"], 7)),
    )
    monkeypatch.setattr(publish_state.time, "sleep", MagicMock())

    with pytest.raises(publish_state.PublishError, match="result was 7"):
        publish_state.reconcile_publish(
            "crate", "1.2.3", CHECKSUM, attempts=2, initial_delay=0, max_delay=0
        )
