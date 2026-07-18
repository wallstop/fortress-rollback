#!/usr/bin/env python3
"""Tests for declarative GitHub main-branch protection."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path
from typing import Callable
from unittest.mock import MagicMock

import pytest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "release" / "main_ruleset.py"
SPEC = importlib.util.spec_from_file_location("main_ruleset", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
main_ruleset = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = main_ruleset
SPEC.loader.exec_module(main_ruleset)


def _response(document: object) -> MagicMock:
    response = MagicMock()
    response.__enter__.return_value = response
    response.__exit__.return_value = False
    response.read.return_value = json.dumps(document).encode("utf-8")
    return response


def _live_config(**extra: object) -> dict[str, object]:
    return {**main_ruleset.load_config(), "id": 16185604, **extra}


def test_checked_in_config_requires_exact_strict_release_check() -> None:
    config = main_ruleset.load_config()

    status_rule = next(
        rule for rule in config["rules"] if rule["type"] == "required_status_checks"
    )
    assert config["conditions"] == {
        "ref_name": {"exclude": [], "include": ["~DEFAULT_BRANCH"]}
    }
    assert status_rule["parameters"] == {
        "required_status_checks": [
            {"context": "Verify prepared release state"}
        ],
        "strict_required_status_checks_policy": True,
        "do_not_enforce_on_create": False,
    }


@pytest.mark.parametrize(
    ("mutation", "message"),
    [
        (
            lambda config: config["rules"][-1]["parameters"].update(
                strict_required_status_checks_policy=False
            ),
            "exact, required, and strict",
        ),
        (
            lambda config: config.update(bypass_actors=[{"actor_type": "OrganizationAdmin"}]),
            "bypass actors",
        ),
    ],
)
def test_config_validation_rejects_weakened_policy(
    tmp_path: Path,
    mutation: Callable[[dict[str, object]], object],
    message: str,
) -> None:
    config = main_ruleset.load_config()
    mutation(config)
    path = tmp_path / "ruleset.json"
    path.write_text(json.dumps(config), encoding="utf-8")

    with pytest.raises(main_ruleset.RulesetError, match=message):
        main_ruleset.load_config(path)


def test_check_accepts_matching_live_ruleset(monkeypatch: pytest.MonkeyPatch) -> None:
    calls: list[tuple[str, str]] = []

    def urlopen(request: object, timeout: int) -> MagicMock:
        calls.append((request.get_method(), request.full_url))
        if request.full_url.endswith("rulesets?per_page=100"):
            return _response([{"id": 16185604, "name": "Main Protection"}])
        return _response(_live_config())

    monkeypatch.setattr(main_ruleset.urllib.request, "urlopen", urlopen)

    outcome = main_ruleset.synchronize("wallstop/fortress-rollback", "token")

    assert outcome == "current"
    assert [method for method, _url in calls] == ["GET", "GET"]


def test_check_fails_closed_on_live_drift(monkeypatch: pytest.MonkeyPatch) -> None:
    live = _live_config(enforcement="evaluate")
    responses = iter(
        [
            _response([{"id": 16185604, "name": "Main Protection"}]),
            _response(live),
        ]
    )
    monkeypatch.setattr(
        main_ruleset.urllib.request,
        "urlopen",
        lambda _request, timeout: next(responses),
    )

    with pytest.raises(main_ruleset.RulesetError, match="differs from config"):
        main_ruleset.synchronize("wallstop/fortress-rollback", "token")


def test_apply_repairs_drift_with_exact_config(monkeypatch: pytest.MonkeyPatch) -> None:
    requests: list[object] = []

    def urlopen(request: object, timeout: int) -> MagicMock:
        requests.append(request)
        if request.full_url.endswith("rulesets?per_page=100"):
            return _response([{"id": 16185604, "name": "Main Protection"}])
        if request.get_method() == "GET":
            return _response(_live_config(enforcement="evaluate"))
        return _response(_live_config())

    monkeypatch.setattr(main_ruleset.urllib.request, "urlopen", urlopen)

    outcome = main_ruleset.synchronize(
        "wallstop/fortress-rollback", "token", apply=True
    )

    assert outcome == "updated"
    put = requests[-1]
    assert put.get_method() == "PUT"
    assert json.loads(put.data.decode("utf-8")) == main_ruleset.load_config()
    assert put.headers["Authorization"] == "Bearer token"


def test_apply_creates_missing_ruleset(monkeypatch: pytest.MonkeyPatch) -> None:
    requests: list[object] = []

    def urlopen(request: object, timeout: int) -> MagicMock:
        requests.append(request)
        if request.get_method() == "GET":
            return _response([])
        return _response(_live_config())

    monkeypatch.setattr(main_ruleset.urllib.request, "urlopen", urlopen)

    outcome = main_ruleset.synchronize(
        "wallstop/fortress-rollback", "token", apply=True
    )

    assert outcome == "created"
    assert [request.get_method() for request in requests] == ["GET", "POST"]
