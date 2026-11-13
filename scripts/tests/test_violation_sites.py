#!/usr/bin/env python3
"""Unit tests for the production violation-site inventory."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "violation_sites",
    scripts_dir / "simulation" / "violation_sites.py",
)
assert spec is not None and spec.loader is not None
violation_sites = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = violation_sites
spec.loader.exec_module(violation_sites)

DEFAULT_BASELINE = violation_sites.DEFAULT_BASELINE
DEFAULT_SOURCE_ROOT = violation_sites.DEFAULT_SOURCE_ROOT
Site = violation_sites.Site
check = violation_sites.check
inventory = violation_sites.inventory
parse_baseline = violation_sites.parse_baseline
render_baseline = violation_sites.render_baseline
scan_text = violation_sites.scan_text
write = violation_sites.write


def _messages(sites: list[Site]) -> list[str]:
    return [site.message for site in sites]


def _site(kind: str, message: str) -> str:
    return f"""
fn sample() {{
    report_violation!(
        ViolationSeverity::Warning,
        ViolationKind::{kind},
        {message}
    );
}}
"""


def test_stable_id_ignores_line_position() -> None:
    first = _site("FrameSync", '"frame mismatch: expected={}, actual={}"')
    second = "\n\n\n" + first
    first_sites, first_issues = scan_text(first, Path("first.rs"))
    second_sites, second_issues = scan_text(second, Path("second.rs"))
    assert first_issues == []
    assert second_issues == []
    assert [site.site_id for site in first_sites] == [
        site.site_id for site in second_sites
    ]


def test_scanner_skips_test_kani_and_loom_only_blocks() -> None:
    text = (
        _site("FrameSync", '"production"')
        + """
#[cfg(test)]
mod tests {
    fn test_only() {
        report_violation!(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "test only"
        );
    }
}

#[cfg(kani)]
fn kani_only() {
    report_violation!(
        ViolationSeverity::Warning,
        ViolationKind::FrameSync,
        "kani only"
    );
}

#[cfg(loom)]
fn loom_only() {
    report_violation!(
        ViolationSeverity::Warning,
        ViolationKind::FrameSync,
        "loom only"
    );
}

#[cfg(not(kani))]
fn production_again() {
    report_violation!(
        ViolationSeverity::Warning,
        ViolationKind::FrameSync,
        "not kani"
    );
}
"""
    )
    sites, issues = scan_text(text, Path("cfg.rs"))
    assert issues == []
    assert _messages(sites) == ['"production"', '"not kani"']


def test_cfg_range_skips_braces_inside_string_literals() -> None:
    text = (
        """
#[cfg(test)]
const OPEN_BRACE: &str = "{";
"""
        + _site("FrameSync", '"production after gated const"')
    )
    sites, issues = scan_text(text, Path("cfg-brace.rs"))
    assert issues == []
    assert _messages(sites) == ['"production after gated const"']


def test_identical_assertions_share_one_site_id() -> None:
    text = _site("FrameSync", '"same assertion"') * 2
    sites, issues = scan_text(text, Path("duplicate.rs"))
    assert issues == []
    assert len(sites) == 2
    assert sites[0] == sites[1]


def test_scanner_ignores_comments_and_string_literals() -> None:
    text = (
        """
// report_violation!(ViolationSeverity::Warning, ViolationKind::FrameSync, "comment");
const NOTE: &str = "report_violation!(ViolationSeverity::Warning, ViolationKind::FrameSync, \\"literal\\")";
"""
        + _site("FrameSync", '"real"')
    )
    sites, issues = scan_text(text, Path("comments.rs"))
    assert issues == []
    assert _messages(sites) == ['"real"']


def test_scanner_reports_non_literal_kind_and_format() -> None:
    text = """
fn bad_kind(kind: ViolationKind) {
    report_violation!(ViolationSeverity::Warning, kind, "dynamic kind");
}
fn bad_format(message: &str) {
    report_violation!(ViolationSeverity::Warning, ViolationKind::FrameSync, message);
}
"""
    sites, issues = scan_text(text, Path("dynamic.rs"))
    assert sites == []
    assert len(issues) == 2
    assert "non-literal kind" in issues[0].detail
    assert "non-literal format" in issues[1].detail


def test_forwarding_only_formats_are_not_assertion_sites() -> None:
    text = _site("Invariant", '"{}"') + _site("Invariant", '"{} [context: {}]"')
    sites, issues = scan_text(text, Path("forwarding.rs"))
    assert sites == []
    assert issues == []


def test_baseline_round_trips_multiline_messages() -> None:
    message = '"line one \\\n            line two"'
    site = Site(violation_sites.stable_site_id("FrameSync", message), "FrameSync", message)
    rendered = render_baseline([site])
    assert rendered.count("\n") == 2
    assert parse_baseline(rendered) == [site]


def test_write_and_check_detect_added_and_removed_sites(tmp_path: Path) -> None:
    source_root = tmp_path / "src"
    source_root.mkdir()
    baseline = tmp_path / "baseline.tsv"
    (source_root / "lib.rs").write_text(_site("FrameSync", '"first"'))
    assert write(source_root, baseline) == 0
    assert check(source_root, baseline) == 0

    (source_root / "lib.rs").write_text(_site("FrameSync", '"second"'))
    assert check(source_root, baseline) == 1

    assert write(source_root, baseline) == 0
    assert check(source_root, baseline) == 0


def test_invalid_baseline_is_rejected(tmp_path: Path) -> None:
    baseline = tmp_path / "baseline.tsv"
    baseline.write_text("site_id\tkind\tmessage_json\nnot-a-site\tFrameSync\t\"x\"\n")
    with pytest.raises(ValueError, match="invalid site ID"):
        parse_baseline(baseline.read_text())


def test_checked_in_baseline_matches_production_source() -> None:
    sites, issues = inventory(DEFAULT_SOURCE_ROOT)
    assert issues == []
    baseline = parse_baseline(DEFAULT_BASELINE.read_text(encoding="utf-8"))
    assert sites == sorted(baseline)
    assert len({site.site_id for site in baseline}) == len(baseline)
