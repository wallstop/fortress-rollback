#!/usr/bin/env python3
"""Tests for scripts/hooks/check-network-timing-invariants.py."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_network_timing_invariants",
    scripts_dir / "hooks" / "check-network-timing-invariants.py",
)
check_network_timing_invariants = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = check_network_timing_invariants
spec.loader.exec_module(check_network_timing_invariants)

check_repo = check_network_timing_invariants.check_repo


VALID_NEXTEST = """
[profile.default]
[[profile.default.overrides]]
filter = 'test(network::multi_process::)'
slow-timeout = { period = "60s", terminate-after = 3 }

[profile.ci]
[[profile.ci.overrides]]
filter = 'test(network::multi_process::)'
slow-timeout = { period = "60s", terminate-after = 3 }

[profile.ci-network-nightly]
[[profile.ci-network-nightly.overrides]]
filter = 'test(network::multi_process::)'
slow-timeout = { period = "60s", terminate-after = 16 }
"""

VALID_MULTI_PROCESS = """
const PROCESS_TIMEOUT_OVERHEAD_SECS: u64 = 30;
const PER_PR_MAX_PEER_TIMEOUT_SECS: u64 = 90;
const NIGHTLY_MAX_PEER_TIMEOUT_SECS: u64 = 600;
const MACOS_TIMEOUT_SCALE_NUMERATOR: u64 = 3;
const MACOS_TIMEOUT_SCALE_DENOMINATOR: u64 = 2;

fn wait_for_peer_with_timeout() {}
fn wait_for_peer_results() {
    wait_for_peer_with_timeout();
    let example = "wait_for_peer(child, name)";
    let raw = r#"wait_for_peer(child, name)"#;
    // wait_for_peer(child, name);
}
"""

VALID_PROTOCOL = """
fn deterministic_protocol_test() {
    advance_test_clock(&clock, Duration::from_millis(1));
    let example = "thread::sleep(Duration::from_millis(1))";
    let raw = r#"thread::sleep(Duration::from_millis(1))"#;
    let quote = '\\'';
    // thread::sleep(Duration::from_millis(1));
}
"""


def write_fixture(
    root: Path,
    *,
    nextest: str = VALID_NEXTEST,
    multi_process: str = VALID_MULTI_PROCESS,
    protocol: str = VALID_PROTOCOL,
) -> None:
    (root / ".config").mkdir()
    (root / ".config" / "nextest.toml").write_text(nextest, encoding="utf-8")
    (root / "tests" / "network").mkdir(parents=True)
    (root / "tests" / "network" / "multi_process.rs").write_text(
        multi_process,
        encoding="utf-8",
    )
    (root / "src" / "network" / "protocol").mkdir(parents=True)
    (root / "src" / "network" / "protocol" / "mod.rs").write_text(
        protocol,
        encoding="utf-8",
    )


def test_valid_fixture_passes(tmp_path: Path) -> None:
    write_fixture(tmp_path)

    assert check_repo(tmp_path) == []


def test_flags_per_pr_budget_below_harness_ceiling(tmp_path: Path) -> None:
    nextest = VALID_NEXTEST.replace('terminate-after = 3', 'terminate-after = 2', 1)
    write_fixture(tmp_path, nextest=nextest)

    issues = check_repo(tmp_path)

    assert len(issues) == 1
    assert "profile.default" in issues[0]
    assert "165s" in issues[0]


def test_flags_nightly_budget_below_macos_harness_ceiling(tmp_path: Path) -> None:
    nextest = VALID_NEXTEST.replace('terminate-after = 16', 'terminate-after = 12')
    write_fixture(tmp_path, nextest=nextest)

    issues = check_repo(tmp_path)

    assert len(issues) == 1
    assert "ci-network-nightly" in issues[0]
    assert "930s" in issues[0]


def test_flags_direct_wait_for_peer_bypass(tmp_path: Path) -> None:
    multi_process = VALID_MULTI_PROCESS + """
fn staggered_test() {
    let result = wait_for_peer(child, "Peer 1");
}
"""
    write_fixture(tmp_path, multi_process=multi_process)

    issues = check_repo(tmp_path)

    assert len(issues) == 1
    assert "direct wait_for_peer()" in issues[0]


@pytest.mark.parametrize(
    ("function_name", "sleep_call"),
    [
        ("flaky_protocol_test", "thread::sleep(Duration::from_millis(1));"),
        (
            "fully_qualified_flaky_protocol_test",
            "std::thread::sleep(std::time::Duration::from_millis(1));",
        ),
        (
            "millis_since_epoch_advances_over_time",
            "std::thread::sleep(Duration::from_millis(1));",
        ),
    ],
)
def test_flags_any_protocol_thread_sleep(
    tmp_path: Path,
    function_name: str,
    sleep_call: str,
) -> None:
    protocol = VALID_PROTOCOL + f"""
fn {function_name}() {{
    {sleep_call}
}}
"""
    write_fixture(tmp_path, protocol=protocol)

    issues = check_repo(tmp_path)

    assert len(issues) == 1
    assert "ProtocolConfig.clock" in issues[0]
