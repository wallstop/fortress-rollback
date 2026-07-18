#!/usr/bin/env python3
"""Check or apply the declarative GitHub default-branch ruleset."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CONFIG = REPO_ROOT / ".github" / "rulesets" / "main-protection.json"
API_ROOT = "https://api.github.com"
API_VERSION = "2022-11-28"
MAX_RESPONSE_BYTES = 1_048_576
REPOSITORY = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
MANAGED_FIELDS = (
    "name",
    "target",
    "enforcement",
    "bypass_actors",
    "conditions",
    "rules",
)


class RulesetError(RuntimeError):
    """The configured and live repository rulesets cannot be reconciled safely."""


def _managed(document: dict[str, Any]) -> dict[str, Any]:
    """Return only fields controlled by the declarative ruleset."""
    return {field: document.get(field) for field in MANAGED_FIELDS}


def load_config(path: Path = DEFAULT_CONFIG) -> dict[str, Any]:
    """Load and validate the mandatory release-protection invariants."""
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise RulesetError(f"cannot read ruleset config {path}: {error}") from error
    if not isinstance(document, dict):
        raise RulesetError("ruleset config must be a JSON object")
    if document.get("name") != "Main Protection":
        raise RulesetError("ruleset config must be named 'Main Protection'")
    if document.get("target") != "branch" or document.get("enforcement") != "active":
        raise RulesetError("main protection must be an active branch ruleset")
    if document.get("bypass_actors") != []:
        raise RulesetError("main protection must not grant bypass actors")
    conditions = document.get("conditions")
    if conditions != {
        "ref_name": {"exclude": [], "include": ["~DEFAULT_BRANCH"]}
    }:
        raise RulesetError("main protection must target only the default branch")
    rules = document.get("rules")
    if not isinstance(rules, list):
        raise RulesetError("main protection rules must be an array")
    if not all(isinstance(rule, dict) for rule in rules):
        raise RulesetError("every main protection rule must be an object")
    required_rule_types = [
        "deletion",
        "non_fast_forward",
        "required_linear_history",
        "required_status_checks",
    ]
    rule_types = [rule.get("type") for rule in rules]
    if rule_types != required_rule_types:
        raise RulesetError(
            "main protection must contain exactly the deletion, non-fast-forward, "
            "linear-history, and release-state status-check rules"
        )
    status_rule = rules[-1]
    required_parameters = {
        "required_status_checks": [
            {
                "context": "Verify prepared release state",
                "integration_id": 15368,
            }
        ],
        "strict_required_status_checks_policy": True,
        "do_not_enforce_on_create": False,
    }
    if status_rule.get("parameters") != required_parameters:
        raise RulesetError(
            "release-state status check must be exact, required, and strict"
        )
    return _managed(document)


def _request_json(
    method: str,
    path: str,
    *,
    token: str,
    payload: dict[str, Any] | None = None,
) -> Any:
    """Call the GitHub API without ever rendering the credential."""
    if not token or "\n" in token or "\r" in token:
        raise RulesetError("a non-empty single-line GitHub token is required")
    data = None
    if payload is not None:
        data = json.dumps(payload, separators=(",", ":")).encode("utf-8")
    request = urllib.request.Request(
        f"{API_ROOT}{path}",
        data=data,
        method=method,
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
            "User-Agent": "fortress-rollback-ruleset-sync",
            "X-GitHub-Api-Version": API_VERSION,
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            raw = response.read(MAX_RESPONSE_BYTES + 1)
    except urllib.error.HTTPError as error:
        snippet = error.read(201).decode(errors="replace")[:200]
        raise RulesetError(
            f"GitHub returned HTTP {error.code} for {method} {path}: {snippet}"
        ) from error
    except (urllib.error.URLError, TimeoutError, OSError) as error:
        raise RulesetError(
            f"GitHub request failed for {method} {path}: {error}"
        ) from error
    if len(raw) > MAX_RESPONSE_BYTES:
        raise RulesetError(f"GitHub response for {method} {path} is too large")
    try:
        return json.loads(raw.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise RulesetError(
            f"GitHub returned malformed JSON for {method} {path}"
        ) from error


def _find_ruleset(repository: str, token: str, name: str) -> int | None:
    document = _request_json(
        "GET",
        f"/repos/{repository}/rulesets?per_page=100&includes_parents=false",
        token=token,
    )
    if not isinstance(document, list):
        raise RulesetError("GitHub ruleset listing must be an array")
    if not all(isinstance(item, dict) for item in document):
        raise RulesetError("every GitHub ruleset listing entry must be an object")
    matches = [item for item in document if item.get("name") == name]
    if len(matches) > 1:
        raise RulesetError(f"repository has multiple rulesets named {name!r}")
    if not matches:
        return None
    ruleset_id = matches[0].get("id")
    if not isinstance(ruleset_id, int) or ruleset_id < 1:
        raise RulesetError("GitHub returned an invalid ruleset ID")
    return ruleset_id


def synchronize(
    repository: str,
    token: str,
    *,
    config_path: Path = DEFAULT_CONFIG,
    apply: bool = False,
) -> str:
    """Check live state, or create/update it when explicitly requested."""
    if REPOSITORY.fullmatch(repository) is None:
        raise RulesetError("repository must use the owner/name form")
    expected = load_config(config_path)
    ruleset_id = _find_ruleset(repository, token, expected["name"])
    collection_path = f"/repos/{repository}/rulesets"
    if ruleset_id is None:
        if not apply:
            raise RulesetError("live Main Protection ruleset is missing")
        created = _request_json(
            "POST", collection_path, token=token, payload=expected
        )
        if not isinstance(created, dict) or _managed(created) != expected:
            raise RulesetError("GitHub did not create the requested ruleset")
        return "created"
    item_path = f"{collection_path}/{ruleset_id}"
    live = _request_json("GET", item_path, token=token)
    if not isinstance(live, dict):
        raise RulesetError("GitHub ruleset response must be an object")
    if _managed(live) == expected:
        return "current"
    if not apply:
        raise RulesetError("live Main Protection ruleset differs from config")
    updated = _request_json(
        "PUT", item_path, token=token, payload=expected
    )
    if not isinstance(updated, dict) or _managed(updated) != expected:
        raise RulesetError("GitHub did not apply the requested ruleset")
    return "updated"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("check", "apply"))
    parser.add_argument("--repo", required=True, dest="repository")
    parser.add_argument("--config", type=Path, default=DEFAULT_CONFIG)
    parser.add_argument("--token-env", default="GH_TOKEN")
    args = parser.parse_args()
    token = os.environ.get(args.token_env, "")
    try:
        outcome = synchronize(
            args.repository,
            token,
            config_path=args.config,
            apply=args.mode == "apply",
        )
    except RulesetError as error:
        print(f"main-ruleset: error: {error}", file=sys.stderr)
        return 1
    print(f"main_ruleset={outcome}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
