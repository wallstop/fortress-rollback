#!/usr/bin/env python3
"""Report which checked-in ``report_violation!`` sites are exercised.

The inventory in :mod:`violation_sites` gives every production assertion a
line-independent ID. This report joins two exercise sources onto that baseline:

- ``--coverage-xml``: a Cobertura report from the test suite. A site counts as
  exercised when its invocation line was executed.
- ``--fleet-dir``: bounded ``violation-sites-*.json`` shards emitted by the
  simulation fleet. Each shard lists the ``file:line`` locations that actually
  reported a violation at runtime.

Line numbers are only a join key. Identity always comes from the checked-in
baseline, so moving a site between lines or files does not reset coverage.

Coverage is a conservative lower bound: LLVM can attribute a macro expansion to
the surrounding lines, so a site absent from coverage is not proof that its
branch never runs. Fleet shard observations are exact runtime hits and should be
preferred whenever a site appears there.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path

import violation_sites

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BASELINE = violation_sites.DEFAULT_BASELINE
DEFAULT_SOURCE_ROOT = violation_sites.DEFAULT_SOURCE_ROOT
CENSUS_SCHEMA_VERSION = 1
FLEET_SCHEMA_VERSION = 1
MAX_FLEET_SHARDS = 8
MAX_FLEET_LOCATIONS = 4096
MAX_FLEET_ARTIFACT_BYTES = 65_536
MAX_COVERAGE_BYTES = 64 * 1024 * 1024
_LOCATION = re.compile(r"(?P<path>[^:\n]+):(?P<line>[0-9]+)")
_SOURCE_MARKER = "src/"


@dataclass(frozen=True)
class SiteReport:
    """One baseline site's exercise status."""

    site_id: str
    kind: str
    message: str
    exercised: bool


def _read_bounded(path: Path, maximum: int) -> bytes:
    """Read at most ``maximum`` bytes, rejecting larger artifacts."""
    size = path.stat().st_size
    if size > maximum:
        raise ValueError(f"{path} has {size} bytes (max {maximum})")
    return path.read_bytes()


def _normalize_source_path(filename: str) -> str | None:
    """Return a repository-relative ``src/...`` path, or ``None``."""
    normalized = filename.replace("\\", "/")
    marker = normalized.find(_SOURCE_MARKER)
    if marker < 0:
        return None
    relative = normalized[marker:]
    if relative == _SOURCE_MARKER.rstrip("/"):
        return None
    return relative


def parse_coverage(path: Path) -> dict[str, set[int]]:
    """Return executed line numbers per repository-relative source path."""
    try:
        # Trusted CI artifact: tarpaulin writes this file in the same pipeline,
        # and the read is size-bounded before parsing. Entity expansion has no
        # attacker-controlled input here.
        document = ET.fromstring(  # noqa: S314
            _read_bounded(path, MAX_COVERAGE_BYTES)
        )
    except ET.ParseError as error:
        raise ValueError(f"{path} is not valid coverage XML: {error}") from error
    hits: dict[str, set[int]] = {}
    for class_element in document.iter("class"):
        filename = class_element.get("filename")
        if not filename:
            continue
        relative = _normalize_source_path(filename)
        if relative is None:
            continue
        for line_element in class_element.iter("line"):
            try:
                line = int(line_element.get("number", ""))
                count = int(line_element.get("hits", "0"))
            except ValueError as error:
                raise ValueError(
                    f"{path} has a malformed coverage line: {error}"
                ) from error
            if count > 0:
                hits.setdefault(relative, set()).add(line)
    if not hits:
        raise ValueError(f"{path} contains no covered src/ lines")
    return hits


def _parse_location(location: str) -> tuple[str, int]:
    match = _LOCATION.fullmatch(location)
    if match is None:
        raise ValueError(f"malformed violation location {location!r}")
    return match.group("path"), int(match.group("line"))


def parse_fleet_shards(directory: Path) -> set[tuple[str, int]]:
    """Return observed ``(path, line)`` locations from fleet shard artifacts."""
    if not directory.is_dir():
        raise ValueError(f"{directory} is not a directory")
    paths = sorted(directory.glob("*.json"))
    if len(paths) > MAX_FLEET_SHARDS:
        raise ValueError(
            f"{directory} has {len(paths)} shards (max {MAX_FLEET_SHARDS})"
        )
    observed: set[tuple[str, int]] = set()
    seen_shards: set[int] = set()
    run_id: int | None = None
    for path in paths:
        try:
            artifact = json.loads(_read_bounded(path, MAX_FLEET_ARTIFACT_BYTES))
        except json.JSONDecodeError as error:
            raise ValueError(f"{path} is not valid JSON: {error}") from error
        if not isinstance(artifact, dict):
            raise ValueError(f"{path} must contain a JSON object")
        if artifact.get("schema_version") != FLEET_SCHEMA_VERSION:
            raise ValueError(f"{path} has an unsupported schema version")
        artifact_run_id = artifact.get("run_id")
        if not isinstance(artifact_run_id, int):
            raise ValueError(f"{path} has an invalid run id")
        if run_id is None:
            run_id = artifact_run_id
        elif artifact_run_id != run_id:
            raise ValueError(f"{path} mixes fleet run ids {run_id} and {artifact_run_id}")
        shard_index = artifact.get("shard_index")
        if not isinstance(shard_index, int) or not 0 <= shard_index < MAX_FLEET_SHARDS:
            raise ValueError(f"{path} has an invalid shard index")
        if shard_index in seen_shards:
            raise ValueError(f"{path} duplicates shard {shard_index}")
        seen_shards.add(shard_index)
        locations = artifact.get("locations")
        if not isinstance(locations, list) or len(locations) > MAX_FLEET_LOCATIONS:
            raise ValueError(f"{path} has invalid locations")
        for location in locations:
            if not isinstance(location, str):
                raise ValueError(f"{path} has a non-string location")
        if locations != sorted(set(locations)):
            raise ValueError(f"{path} locations are not sorted and unique")
        for location in locations:
            observed.add(_parse_location(location))
    return observed


def build_report(
    sites: list[violation_sites.Site],
    locations: list[violation_sites.SiteLocation],
    covered: dict[str, set[int]] | None,
    fleet: set[tuple[str, int]] | None,
) -> dict[str, object]:
    """Join exercise sources onto the baseline and render a deterministic report."""
    by_location: dict[tuple[str, int], set[str]] = {}
    for location in locations:
        by_location.setdefault((location.path, location.line), set()).add(
            location.site_id
        )

    exercised: set[str] = set()
    coverage_locations = 0
    if covered is not None:
        for site_path, lines in covered.items():
            for line in lines:
                ids = by_location.get((site_path, line))
                if ids:
                    coverage_locations += 1
                    exercised.update(ids)

    fleet_locations = 0
    if fleet is not None:
        for observed in fleet:
            ids = by_location.get(observed)
            if ids is None:
                raise ValueError(
                    f"fleet reported an unknown source location {observed!r}"
                )
            fleet_locations += 1
            exercised.update(ids)

    baseline_ids = {site.site_id for site in sites}
    unknown = exercised - baseline_ids
    if unknown:
        raise ValueError(
            f"exercise sources reference sites outside the baseline: {sorted(unknown)}"
        )

    reports = [
        SiteReport(site.site_id, site.kind, site.message, site.site_id in exercised)
        for site in sites
    ]
    unexercised = [report for report in reports if not report.exercised]
    return {
        "schema_version": CENSUS_SCHEMA_VERSION,
        "baseline_sites": len(reports),
        "exercised_sites": len(reports) - len(unexercised),
        "unexercised_sites": len(unexercised),
        "sources": {
            "coverage": (
                None
                if covered is None
                else {
                    "covered_src_files": len(covered),
                    "matched_locations": coverage_locations,
                }
            ),
            "fleet": (
                None
                if fleet is None
                else {
                    "observed_locations": len(fleet),
                    "matched_locations": fleet_locations,
                }
            ),
        },
        "unexercised": [
            {
                "site_id": report.site_id,
                "kind": report.kind,
                "message": report.message,
            }
            for report in unexercised
        ],
    }


def write_report(path: Path, report: dict[str, object]) -> None:
    """Write the report as deterministic JSON for the CI artifact."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def main(argv: list[str] | None = None) -> int:
    """Build the exercise report over the checked-in baseline."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--coverage-xml", type=Path)
    parser.add_argument("--fleet-dir", type=Path)
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    parser.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    arguments = parser.parse_args(argv)
    try:
        baseline = violation_sites.parse_baseline(
            arguments.baseline.read_text(encoding="utf-8")
        )
        sites, locations, issues = violation_sites.inventory_locations(
            arguments.source_root, arguments.repo_root
        )
        if issues:
            for issue in issues:
                violation_sites.print_issue(issue)
            raise ValueError(
                f"{len(issues)} production invocation(s) cannot be inventoried"
            )
        if [site.site_id for site in sites] != [site.site_id for site in baseline]:
            raise ValueError(
                "violation-site baseline is stale; run "
                "`python3 scripts/simulation/violation_sites.py --write`"
            )
        covered = (
            None
            if arguments.coverage_xml is None
            else parse_coverage(arguments.coverage_xml)
        )
        fleet = (
            None
            if arguments.fleet_dir is None
            else parse_fleet_shards(arguments.fleet_dir)
        )
        report = build_report(sites, locations, covered, fleet)
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    write_report(arguments.output, report)
    print(
        "violation sites: "
        f"{report['baseline_sites']} baseline, "
        f"{report['exercised_sites']} exercised, "
        f"{report['unexercised_sites']} unexercised"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
