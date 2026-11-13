#!/usr/bin/env python3
"""Report which checked-in ``report_violation!`` sites are exercised.

The inventory in :mod:`violation_sites` gives every production assertion a
line-independent ID. This report joins three exercise sources onto that baseline:

- ``--coverage-xml``: a Cobertura report from the test suite. A site counts as
  exercised when its invocation line was executed.
- ``--fleet-dir``: bounded ``violation-sites-*.json`` shards emitted by the
  simulation fleet. Each shard lists the ``file:line`` locations that actually
  reported a violation at runtime.
- ``--observations-dir``: bounded per-process ``observations-*.json`` artifacts
  emitted by the ``violation-census`` feature hook while the standard test suite
  runs. These are exact runtime observations from unit and integration tests.

Line numbers are only a join key. Identity always comes from the checked-in
baseline, so moving a site between lines or files does not reset coverage.

Coverage is a conservative lower bound: LLVM can attribute a macro expansion to
the surrounding lines, so a site absent from coverage is not proof that its
branch never runs. Fleet and observation artifacts are exact runtime hits and
should be preferred whenever a site appears there. Observation artifacts can
also contain test-only assertions that are not production sites; those are
counted as unmatched and do not fail the report.
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
CENSUS_SCHEMA_VERSION = 2
FLEET_SCHEMA_VERSION = 1
OBSERVATION_SCHEMA_VERSION = 1
MAX_FLEET_SHARDS = 8
MAX_FLEET_LOCATIONS = 512
MAX_FLEET_ARTIFACT_BYTES = 65_536
MAX_OBSERVATION_ARTIFACTS = 4_096
MAX_OBSERVATION_LOCATIONS = 256
MAX_OBSERVATION_TOTAL_LOCATIONS = 4_096
MAX_OBSERVATION_ARTIFACT_BYTES = 65_536
MAX_COVERAGE_BYTES = 64 * 1024 * 1024
_LOCATION = re.compile(r"(?P<path>[^:\n]+):(?P<line>[0-9]+)")
_SOURCE_MARKER = "src/"


@dataclass(frozen=True)
class ObservationSource:
    """Validated exact test-suite observations from one artifact directory."""

    locations: set[tuple[str, int]]
    artifact_count: int
    truncated_artifacts: int
    ignored_locations: int
    run_id: str | None


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
    match = _LOCATION.fullmatch(location.replace("\\", "/"))
    if match is None:
        raise ValueError(f"malformed violation location {location!r}")
    return match.group("path"), int(match.group("line"))


def parse_fleet_shards(directory: Path) -> set[tuple[str, int]]:
    """Return observed ``(path, line)`` locations from fleet shard artifacts."""
    if not directory.is_dir():
        raise ValueError(f"{directory} is not a directory")
    paths = sorted(directory.glob("*.json"))
    if not paths:
        raise ValueError(f"{directory} contains no fleet shards")
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
            raise ValueError(
                f"{path} mixes fleet run ids {run_id} and {artifact_run_id}"
            )
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


def parse_observation_artifacts(
    directory: Path, expected_run_id: str | None = None
) -> ObservationSource:
    """Return validated exact test-suite observations from ``directory``."""
    if not directory.is_dir():
        raise ValueError(f"{directory} is not a directory")
    paths = sorted(directory.glob("*.json"))
    if not paths:
        raise ValueError(f"{directory} contains no observation artifacts")
    if len(paths) > MAX_OBSERVATION_ARTIFACTS:
        raise ValueError(
            f"{directory} has {len(paths)} observation artifacts "
            f"(max {MAX_OBSERVATION_ARTIFACTS})"
        )
    observed: set[tuple[str, int]] = set()
    truncated_artifacts = 0
    ignored_locations = 0
    processed_locations = 0
    run_ids: set[str | None] = set()
    for path in paths:
        try:
            artifact = json.loads(_read_bounded(path, MAX_OBSERVATION_ARTIFACT_BYTES))
        except json.JSONDecodeError as error:
            raise ValueError(f"{path} is not valid JSON: {error}") from error
        except RecursionError as error:
            raise ValueError(f"{path} is nested too deeply: {error}") from error
        if not isinstance(artifact, dict):
            raise ValueError(f"{path} must contain a JSON object")
        if type(artifact.get("schema_version")) is not int:
            raise ValueError(f"{path} has an invalid schema version")
        if artifact["schema_version"] != OBSERVATION_SCHEMA_VERSION:
            raise ValueError(f"{path} has an unsupported schema version")
        run_id = artifact.get("run_id")
        if run_id is not None and (not isinstance(run_id, str) or not run_id):
            raise ValueError(f"{path} has an invalid run id")
        if expected_run_id is not None and run_id != expected_run_id:
            raise ValueError(
                f"{path} belongs to run {run_id!r}, not {expected_run_id!r}"
            )
        run_ids.add(run_id)
        process_id = artifact.get("process_id")
        if type(process_id) is not int or process_id <= 0:
            raise ValueError(f"{path} has an invalid process id")
        if path.name != f"observations-{process_id}.json":
            raise ValueError(f"{path} does not match its process id")
        truncated = artifact.get("truncated", False)
        if type(truncated) is not bool:
            raise ValueError(f"{path} has an invalid truncated flag")
        if truncated:
            truncated_artifacts += 1
        locations = artifact.get("locations")
        if (
            not isinstance(locations, list)
            or len(locations) > MAX_OBSERVATION_LOCATIONS
        ):
            raise ValueError(f"{path} has invalid locations")
        if not all(isinstance(location, str) for location in locations):
            raise ValueError(f"{path} has a non-string location")
        if locations != sorted(set(locations)):
            raise ValueError(f"{path} locations are not sorted and unique")
        processed_locations += len(locations)
        if processed_locations > MAX_OBSERVATION_TOTAL_LOCATIONS:
            raise ValueError(
                f"{directory} has more than {MAX_OBSERVATION_TOTAL_LOCATIONS} "
                "observation locations"
            )
        for location in locations:
            normalized = location.replace("\\", "/")
            source = _normalize_source_path(normalized)
            try:
                observed.add(_parse_location(source or normalized))
            except ValueError as error:
                # Unit tests may construct SpecViolation directly with
                # test-local labels. A malformed production path still fails
                # loudly because production locations always look like
                # `src/...:line`.
                if source is not None:
                    raise ValueError(
                        f"{path} has a malformed location: {error}"
                    ) from error
                ignored_locations += 1
    if len(run_ids) > 1:
        raise ValueError(f"{directory} mixes observation run ids")
    return ObservationSource(
        locations=observed,
        artifact_count=len(paths),
        truncated_artifacts=truncated_artifacts,
        ignored_locations=ignored_locations,
        run_id=next(iter(run_ids)),
    )


def build_report(
    sites: list[violation_sites.Site],
    locations: list[violation_sites.SiteLocation],
    covered: dict[str, set[int]] | None,
    fleet: set[tuple[str, int]] | None,
    observations: ObservationSource | None = None,
    partial: bool = False,
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

    observation_locations = 0
    unmatched_observations = 0
    if observations is not None:
        for observed in observations.locations:
            ids = by_location.get(observed)
            if ids is None:
                # Unit tests can report violations from test-only assertions or
                # construct SpecViolation directly; only inventory locations
                # contribute to the exercise set.
                unmatched_observations += 1
                continue
            observation_locations += 1
            exercised.update(ids)

    baseline_ids = {site.site_id for site in sites}
    unknown = exercised - baseline_ids
    if unknown:
        raise ValueError(
            f"exercise sources reference sites outside the baseline: {sorted(unknown)}"
        )

    unexercised = [site for site in sites if site.site_id not in exercised]
    unmatched_source_locations = (
        []
        if observations is None
        else sorted(
            f"{path}:{line}"
            for path, line in observations.locations
            if (path, line) not in by_location and path.startswith(_SOURCE_MARKER)
        )
    )
    return {
        "schema_version": CENSUS_SCHEMA_VERSION,
        "partial": partial,
        "baseline_sites": len(sites),
        "exercised_sites": len(sites) - len(unexercised),
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
            "observations": (
                None
                if observations is None
                else {
                    "artifact_count": observations.artifact_count,
                    "truncated_artifacts": observations.truncated_artifacts,
                    "ignored_locations": observations.ignored_locations,
                    "run_id": observations.run_id,
                    "observed_locations": len(observations.locations),
                    "matched_locations": observation_locations,
                    "unmatched_locations": unmatched_observations,
                    "unmatched_source_locations": unmatched_source_locations,
                }
            ),
        },
        "unexercised": [
            {
                "site_id": site.site_id,
                "kind": site.kind,
                "message": site.message,
            }
            for site in unexercised
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
    parser.add_argument("--observations-dir", type=Path)
    parser.add_argument("--expected-run-id")
    parser.add_argument("--partial", action="store_true")
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
        observations = (
            None
            if arguments.observations_dir is None
            else parse_observation_artifacts(
                arguments.observations_dir, arguments.expected_run_id
            )
        )
        report = build_report(
            sites,
            locations,
            covered,
            fleet,
            observations,
            arguments.partial,
        )
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
