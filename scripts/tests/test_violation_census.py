#!/usr/bin/env python3
"""Unit tests for the violation-site exercise census."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest

scripts_dir = Path(__file__).parent.parent


def _load(name: str):
    spec = importlib.util.spec_from_file_location(
        name, scripts_dir / "simulation" / f"{name}.py"
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


violation_sites = _load("violation_sites")
violation_census = _load("violation_census")

SAMPLE_SOURCE = """\
fn sample() {
    report_violation!(
        ViolationSeverity::Warning,
        ViolationKind::FrameSync,
        "first: {}",
        1
    );
    report_violation!(
        ViolationSeverity::Warning,
        ViolationKind::InternalError,
        "second: {}",
        2
    );
    report_violation!(
        ViolationSeverity::Warning,
        ViolationKind::NetworkProtocol,
        "third: {}",
        3
    );
}
"""


def _write_fixture(
    tmp_path: Path,
) -> tuple[Path, Path, list[violation_sites.SiteLocation]]:
    source_root = tmp_path / "src"
    source_root.mkdir()
    (source_root / "sample.rs").write_text(SAMPLE_SOURCE, encoding="utf-8")
    baseline = tmp_path / "baseline.tsv"
    assert violation_sites.write(source_root, baseline) == 0
    sites, locations, issues = violation_sites.inventory_locations(
        source_root, tmp_path
    )
    assert issues == []
    assert len(sites) == 3
    return source_root, baseline, locations


def _write_coverage(path: Path, filename: str, lines: dict[int, int]) -> None:
    rendered = "".join(
        f'<line number="{line}" hits="{hits}"/>' for line, hits in sorted(lines.items())
    )
    path.write_text(
        "<coverage><packages><package><classes>"
        f'<class filename="{filename}"><lines>{rendered}</lines></class>'
        "</classes></package></packages></coverage>",
        encoding="utf-8",
    )


def _write_fleet(
    path: Path, shard: int, locations: list[str], run_id: int = 7
) -> None:
    path.mkdir(parents=True, exist_ok=True)
    (path / f"shard-{shard}.json").write_text(
        json.dumps(
            {
                "schema_version": 1,
                "run_id": run_id,
                "shard_index": shard,
                "locations": locations,
            }
        ),
        encoding="utf-8",
    )


def test_report_joins_coverage_and_fleet_locations(tmp_path: Path) -> None:
    source_root, baseline, locations = _write_fixture(tmp_path)
    assert [location.path for location in locations] == ["src/sample.rs"] * 3
    first, second, third = locations
    coverage = tmp_path / "coverage.xml"
    _write_coverage(coverage, "src/sample.rs", {first.line: 4, second.line: 0})
    fleet = tmp_path / "fleet"
    _write_fleet(fleet, 0, [f"src/sample.rs:{second.line}"])

    sites = violation_sites.parse_baseline(baseline.read_text(encoding="utf-8"))
    sites_now, locations_now, issues = violation_sites.inventory_locations(
        source_root, tmp_path
    )
    assert issues == []
    assert sites_now == sites
    report = violation_census.build_report(
        sites_now,
        locations_now,
        violation_census.parse_coverage(coverage),
        violation_census.parse_fleet_shards(fleet),
    )

    assert report["baseline_sites"] == 3
    assert report["exercised_sites"] == 2
    assert [entry["site_id"] for entry in report["unexercised"]] == [third.site_id]
    assert report["sources"]["coverage"]["matched_locations"] == 1
    assert report["sources"]["fleet"]["matched_locations"] == 1


def test_report_without_exercise_sources_marks_every_site_unexercised(
    tmp_path: Path,
) -> None:
    source_root, _, _ = _write_fixture(tmp_path)
    sites, locations, issues = violation_sites.inventory_locations(
        source_root, tmp_path
    )
    assert issues == []
    report = violation_census.build_report(sites, locations, None, None)
    assert report["exercised_sites"] == 0
    assert len(report["unexercised"]) == 3
    assert report["sources"] == {"coverage": None, "fleet": None}


def test_unknown_fleet_location_is_rejected(tmp_path: Path) -> None:
    source_root, _, _ = _write_fixture(tmp_path)
    sites, locations, issues = violation_sites.inventory_locations(
        source_root, tmp_path
    )
    assert issues == []
    with pytest.raises(ValueError, match="unknown source location"):
        violation_census.build_report(sites, locations, None, {("src/sample.rs", 9999)})


def test_fleet_shard_schema_is_validated(tmp_path: Path) -> None:
    fleet = tmp_path / "fleet"
    _write_fleet(fleet, 0, ["src/sample.rs:1"])
    artifact = fleet / "shard-0.json"
    payload = json.loads(artifact.read_text(encoding="utf-8"))
    payload["schema_version"] = 99
    artifact.write_text(json.dumps(payload), encoding="utf-8")
    with pytest.raises(ValueError, match="unsupported schema"):
        violation_census.parse_fleet_shards(fleet)


def test_fleet_shards_must_share_a_run_id(tmp_path: Path) -> None:
    fleet = tmp_path / "fleet"
    _write_fleet(fleet, 0, ["src/sample.rs:1"], run_id=7)
    _write_fleet(fleet, 1, ["src/sample.rs:2"], run_id=8)
    with pytest.raises(ValueError, match="mixes fleet run ids"):
        violation_census.parse_fleet_shards(fleet)


def test_coverage_without_covered_source_is_rejected(tmp_path: Path) -> None:
    coverage = tmp_path / "coverage.xml"
    _write_coverage(coverage, "tests/other.rs", {1: 1})
    with pytest.raises(ValueError, match="no covered src/ lines"):
        violation_census.parse_coverage(coverage)


def test_cli_rejects_a_stale_baseline(tmp_path: Path) -> None:
    source_root, baseline, _ = _write_fixture(tmp_path)
    baseline.write_text(
        "site_id\tkind\tmessage_json\n"
        '0000000000000000\tFrameSync\t"\\"retired: {}\\""\n',
        encoding="utf-8",
    )
    output = tmp_path / "report.json"
    assert (
        violation_census.main(
            [
                "--output",
                str(output),
                "--source-root",
                str(source_root),
                "--repo-root",
                str(tmp_path),
                "--baseline",
                str(baseline),
            ]
        )
        == 2
    )
    assert not output.exists()
