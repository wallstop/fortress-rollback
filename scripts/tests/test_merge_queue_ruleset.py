#!/usr/bin/env python3
"""Contract test for the prepared merge-queue ruleset."""

from __future__ import annotations

import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CONFIG = REPO_ROOT / ".github" / "rulesets" / "merge-queue.json"


def test_merge_queue_config_uses_canonical_squash_policy() -> None:
    document = json.loads(CONFIG.read_text(encoding="utf-8"))
    queue = next(rule for rule in document["rules"] if rule["type"] == "merge_queue")

    assert document["target"] == "branch"
    assert document["conditions"]["ref_name"]["include"] == ["~DEFAULT_BRANCH"]
    assert queue["parameters"] == {
        "merge_method": "SQUASH",
        "grouping_strategy": "ALLGREEN",
        "max_entries_to_build": 5,
        "min_entries_to_merge": 1,
        "max_entries_to_merge": 5,
        "min_entries_to_merge_wait_minutes": 0,
        "check_response_timeout_minutes": 60,
    }
