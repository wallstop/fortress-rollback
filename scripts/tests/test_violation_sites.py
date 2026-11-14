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
    text = _site("Invariant", '"{}"') + _site("Invariant", '"{}: {}"')
    sites, issues = scan_text(text, Path("forwarding.rs"))
    assert sites == []
    assert issues == []


def test_placeholder_first_messages_with_static_text_are_sites() -> None:
    text = _site("NetworkProtocol", '"{} receive attempts reached the cap"')
    sites, issues = scan_text(text, Path("placeholder.rs"))
    assert issues == []
    assert _messages(sites) == ['"{} receive attempts reached the cap"']


def test_macro_definition_assertions_are_not_runtime_sites() -> None:
    text = """
macro_rules! forward {
    ($($arg:tt)*) => {
        report_violation!(
            ViolationSeverity::Warning,
            ViolationKind::Invariant,
            "{} [context: {}]",
            $($arg)*
        );
    };
}
"""
    sites, issues = scan_text(text, Path("macro-def.rs"))
    assert issues == []
    assert sites == []


def test_observer_macro_sites_are_inventoried() -> None:
    text = """
fn sample() {
    report_violation_to!(
        &observer,
        ViolationSeverity::Warning,
        ViolationKind::FrameSync,
        "observer frame mismatch: expected={}, actual={}",
        1,
        2
    );
}
"""
    sites, issues = scan_text(text, Path("observer.rs"))
    assert issues == []
    assert len(sites) == 1
    assert sites[0].kind == "FrameSync"
    assert sites[0].message == '"observer frame mismatch: expected={}, actual={}"'


@pytest.mark.parametrize(
    ("macro", "kind", "message"),
    [
        (
            "safe_frame_add!",
            "ArithmeticOverflow",
            '"Frame overflow in {}: {} + {} would overflow"',
        ),
        (
            "safe_frame_sub!",
            "ArithmeticOverflow",
            '"Frame underflow in {}: {} - {} would underflow"',
        ),
    ],
)
def test_wrapper_call_sites_share_the_wrapper_site(
    macro: str, kind: str, message: str
) -> None:
    text = f"""
macro_rules! {macro.rstrip("!")} {{
    ($frame:expr, $delta:expr, $context:expr) => {{
        report_violation!(
            ViolationSeverity::Warning,
            ViolationKind::ArithmeticOverflow,
            {message},
            $frame,
            $delta
        );
    }};
}}

fn first() {{
    let frame = {macro}(frame, 1, "first");
}}

fn second() {{
    let frame = {macro}(frame, 2, "second");
}}
"""
    sites, issues = scan_text(text, Path("wrapper.rs"))
    assert issues == []
    assert [site.site_id for site in sites] == [
        violation_sites.stable_site_id(kind, message)
    ] * 2


def test_wrapper_messages_match_definitions() -> None:
    telemetry = (DEFAULT_SOURCE_ROOT / "telemetry.rs").read_text(encoding="utf-8")
    for macro, (_, message) in violation_sites._WRAPPER_MACROS.items():
        start = telemetry.find(f"macro_rules! {macro.rstrip('!')} ")
        assert start >= 0, f"missing definition for {macro}"
        body_start = telemetry.find("{", start)
        parsed = violation_sites._parse_balanced(telemetry, body_start, "{", "}")
        assert parsed is not None
        assert message in telemetry[start : parsed[1]]


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
