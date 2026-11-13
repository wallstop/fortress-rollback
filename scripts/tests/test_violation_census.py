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


def _write_fleet(path: Path, shard: int, locations: list[str], run_id: int = 7) -> None:
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


def _write_observations(
    path: Path,
    process_id: int,
    locations: list[str],
    *,
    truncated: bool = False,
    run_id: str | None = None,
) -> None:
    path.mkdir(parents=True, exist_ok=True)
    (path / f"observations-{process_id}.json").write_text(
        json.dumps(
            {
                "schema_version": 1,
                "run_id": run_id,
                "process_id": process_id,
                "locations": locations,
                "truncated": truncated,
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


def test_report_counts_exact_observations_and_ignores_test_only_locations(
    tmp_path: Path,
) -> None:
    source_root, _, locations = _write_fixture(tmp_path)
    first, _, third = locations
    sites, locations_now, issues = violation_sites.inventory_locations(
        source_root, tmp_path
    )
    assert issues == []
    observations_dir = tmp_path / "observations"
    _write_observations(
        observations_dir,
        41,
        sorted(
            [
                f"/build/repo/src/sample.rs:{first.line}",
                f"src\\sample.rs:{third.line}",
            ]
        ),
    )
    _write_observations(
        observations_dir,
        42,
        [
            "not-a-location",
            "src/test-only/assertion.rs:1",
            "tests/fixtures/src/sample.rs:1",
        ],
        truncated=True,
    )

    observations = violation_census.parse_observation_artifacts(observations_dir)
    assert ("tests/fixtures/src/sample.rs", 1) in observations.locations
    assert ("src/sample.rs", 1) not in observations.locations
    report = violation_census.build_report(
        sites,
        locations_now,
        None,
        None,
        observations,
    )

    assert report["exercised_sites"] == 2
    assert report["partial"] is False
    source = report["sources"]["observations"]
    assert source == {
        "artifact_count": 2,
        "truncated_artifacts": 1,
        "ignored_locations": 1,
        "run_id": None,
        "observed_locations": 4,
        "matched_locations": 2,
        "unmatched_locations": 2,
        "unmatched_source_locations": ["src/test-only/assertion.rs:1"],
    }


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
    assert report["sources"] == {
        "coverage": None,
        "fleet": None,
        "observations": None,
    }


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


def test_observation_artifacts_validate_schema_and_order(tmp_path: Path) -> None:
    observations = tmp_path / "observations"
    _write_observations(observations, 7, ["src/sample.rs:1"])
    artifact = observations / "observations-7.json"
    payload = json.loads(artifact.read_text(encoding="utf-8"))
    payload["schema_version"] = 99
    artifact.write_text(json.dumps(payload), encoding="utf-8")
    with pytest.raises(ValueError, match="unsupported schema"):
        violation_census.parse_observation_artifacts(observations)

    _write_observations(observations, 7, ["src/sample.rs:2", "src/sample.rs:1"])
    with pytest.raises(ValueError, match="sorted and unique"):
        violation_census.parse_observation_artifacts(observations)


@pytest.mark.parametrize(
    ("field", "value", "match"),
    [
        ("schema_version", True, "invalid schema version"),
        ("schema_version", 1.0, "invalid schema version"),
        ("process_id", True, "invalid process id"),
        ("process_id", 0, "invalid process id"),
        ("locations", "src/sample.rs:1", "invalid locations"),
        ("locations", [1], "non-string location"),
        ("locations", ["src/sample.rs:not-a-line"], "malformed location"),
        ("truncated", "yes", "invalid truncated flag"),
    ],
)
def test_observation_artifacts_reject_invalid_fields(
    tmp_path: Path, field: str, value: object, match: str
) -> None:
    observations = tmp_path / "observations"
    _write_observations(observations, 7, ["src/sample.rs:1"])
    artifact = observations / "observations-7.json"
    payload = json.loads(artifact.read_text(encoding="utf-8"))
    payload[field] = value
    artifact.write_text(json.dumps(payload), encoding="utf-8")
    with pytest.raises(ValueError, match=match):
        violation_census.parse_observation_artifacts(observations)


def test_observation_artifacts_require_artifacts_and_matching_process_ids(
    tmp_path: Path,
) -> None:
    observations = tmp_path / "observations"
    observations.mkdir()
    with pytest.raises(ValueError, match="contains no observation"):
        violation_census.parse_observation_artifacts(observations)

    _write_observations(observations, 7, ["src/sample.rs:1"])
    (observations / "observations-7.json").rename(observations / "observations-8.json")
    with pytest.raises(ValueError, match="does not match its process id"):
        violation_census.parse_observation_artifacts(observations)

    _write_observations(observations, 7, ["src/sample.rs:1"], run_id="run-a")
    _write_observations(observations, 8, ["src/sample.rs:2"], run_id="run-b")
    with pytest.raises(ValueError, match="mixes observation run ids"):
        violation_census.parse_observation_artifacts(observations)

    with pytest.raises(ValueError, match="belongs to run"):
        violation_census.parse_observation_artifacts(
            observations, expected_run_id="run-c"
        )


def test_empty_fleet_directory_is_rejected(tmp_path: Path) -> None:
    fleet = tmp_path / "fleet"
    fleet.mkdir()
    with pytest.raises(ValueError, match="contains no fleet shards"):
        violation_census.parse_fleet_shards(fleet)


def test_observation_artifacts_enforce_bounds(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    observations = tmp_path / "observations"
    _write_observations(observations, 7, ["src/sample.rs:1"])

    monkeypatch.setattr(violation_census, "MAX_OBSERVATION_ARTIFACT_BYTES", 10)
    with pytest.raises(ValueError, match="bytes"):
        violation_census.parse_observation_artifacts(observations)
    monkeypatch.setattr(violation_census, "MAX_OBSERVATION_ARTIFACT_BYTES", 65_536)

    _write_observations(observations, 8, ["src/sample.rs:2"])
    monkeypatch.setattr(violation_census, "MAX_OBSERVATION_ARTIFACTS", 1)
    with pytest.raises(ValueError, match="observation artifacts"):
        violation_census.parse_observation_artifacts(observations)
    monkeypatch.setattr(violation_census, "MAX_OBSERVATION_ARTIFACTS", 4_096)

    _write_observations(observations, 9, ["src/sample.rs:2", "src/sample.rs:3"])
    monkeypatch.setattr(violation_census, "MAX_OBSERVATION_LOCATIONS", 1)
    with pytest.raises(ValueError, match="invalid locations"):
        violation_census.parse_observation_artifacts(observations)
    monkeypatch.setattr(violation_census, "MAX_OBSERVATION_LOCATIONS", 256)

    monkeypatch.setattr(violation_census, "MAX_OBSERVATION_TOTAL_LOCATIONS", 1)
    with pytest.raises(ValueError, match="more than 1"):
        violation_census.parse_observation_artifacts(observations)


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
