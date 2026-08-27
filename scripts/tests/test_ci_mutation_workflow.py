#!/usr/bin/env python3
"""Structural contracts for bounded, fail-closed mutation CI."""

from __future__ import annotations

import os
import re
import subprocess
import textwrap
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python <3.11 compatibility
    import tomli as tomllib

REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-mutation.yml"
MUTANTS_CONFIG = REPO_ROOT / ".cargo" / "mutants.toml"


def workflow_text() -> str:
    return WORKFLOW.read_text(encoding="utf-8")


def named_step(text: str, name: str) -> str:
    marker = f"      - name: {name}\n"
    start = text.index(marker)
    next_step = text.find("\n      - name: ", start + len(marker))
    next_job_match = re.search(
        r"(?m)^  [a-zA-Z0-9_-]+:\s*$", text[start + len(marker) :]
    )
    next_job = (
        start + len(marker) + next_job_match.start()
        if next_job_match is not None
        else -1
    )
    ends = [end for end in (next_step, next_job) if end != -1]
    return text[start : min(ends) if ends else len(text)]


def mutation_list_fragment(step: str, count_variable: str) -> str:
    """Extract the producer and normalization fragment from a workflow step."""
    lines = step.splitlines()
    start = next(
        index
        for index, line in enumerate(lines)
        if "cargo mutants --list --json" in line
    )
    end = next(
        index
        for index, line in enumerate(lines[start:], start=start)
        if line.strip() == "' mutation-list.json)\""
    )
    fragment = textwrap.dedent("\n".join(lines[start : end + 1]))
    return (
        "set -euo pipefail\n"
        "filter_args=()\n"
        'shard="0/1"\n'
        f"{fragment}\n"
        f'printf "count=%s\\n" "${{{count_variable}}}"\n'
    )


def test_workflow_pins_tool_and_runs_linux_only() -> None:
    text = workflow_text()

    assert text.count("version: 27.1.0") == 2
    assert "runs-on: ubuntu-latest" in text
    assert "macos-latest" not in text
    assert "windows-latest" not in text
    assert "continue-on-error" not in text


def test_workflow_uses_exact_head_diff_and_bounded_dynamic_shards() -> None:
    text = workflow_text()

    assert text.count("fetch-depth: 0") == 2
    assert text.count("github.event.pull_request.head.sha || github.sha") == 3
    assert "git diff --binary --unified=0" in text
    assert "--in-diff mutation-scope.diff" in text
    assert "(expected_mutants + 49) / 50" in text
    assert 'shard_count" -gt 96' in text
    assert "[range(0; $count)]" in text
    assert "max-parallel: 16" in text


def test_runtime_flags_are_reproducible_and_list_only_json() -> None:
    text = workflow_text()
    step = named_step(text, "Run bounded mutation shard")
    runtime = step[step.index("          set +e") :]

    for flag in (
        "--no-shuffle",
        "--sharding round-robin",
        "--shard",
        "--baseline=skip",
        "--timeout",
        "--in-place",
    ):
        assert flag in runtime
    assert "--timeout-multiplier" not in step
    assert "--json" not in runtime
    assert "mutants-out/mutants.out/outcomes.json" in step
    assert "mutants-out/mutants.json" not in step
    assert 'pipeline_status=("${PIPESTATUS[@]}")' in step
    assert 'command_status="${pipeline_status[0]}"' in step
    assert 'tee_status="${pipeline_status[1]}"' in step


def test_mutant_list_normalization_behavior(tmp_path: Path) -> None:
    fake_cargo = tmp_path / "cargo"
    fake_cargo.write_text(
        """#!/usr/bin/env bash
set -euo pipefail
case "${FAKE_MUTANT_LIST_MODE}" in
  zero) exit 0 ;;
  valid) printf '%s\\n' '[{}, {}]' ;;
  malformed) printf '%s\\n' '{' ;;
  non_array) printf '%s\\n' '{}' ;;
  producer_failure) exit 42 ;;
  *) exit 64 ;;
esac
""",
        encoding="utf-8",
    )
    fake_cargo.chmod(0o755)
    env = os.environ.copy()
    env["PATH"] = f"{tmp_path}{os.pathsep}{env['PATH']}"

    cases = (
        ("zero", 0, "count=0\n"),
        ("valid", 0, "count=2\n"),
        ("malformed", None, ""),
        ("non_array", None, ""),
        ("producer_failure", 42, ""),
    )
    steps = (
        ("Plan exact mutation corpus", "expected_mutants"),
        ("Run bounded mutation shard", "selected_mutants"),
    )
    for step_name, count_variable in steps:
        fragment = mutation_list_fragment(named_step(workflow_text(), step_name), count_variable)
        for mode, expected_status, expected_output in cases:
            env["FAKE_MUTANT_LIST_MODE"] = mode
            result = subprocess.run(
                ["bash", "-c", fragment],
                cwd=tmp_path,
                env=env,
                capture_output=True,
                text=True,
                timeout=10,
                check=False,
            )

            context = f"{step_name}/{mode}: {result.stderr}"
            if expected_status is None:
                assert result.returncode != 0, context
            else:
                assert result.returncode == expected_status, context
            assert result.stdout == expected_output, context


def test_baseline_and_summary_are_fail_closed() -> None:
    text = workflow_text()
    baseline = named_step(text, "Run authoritative baseline")
    summary = named_step(text, "Aggregate mutation policy")
    terminal = named_step(text, "Require successful baseline and shard jobs")

    assert "--no-capture" in baseline
    assert "aggregate_mutation_results.py" in summary
    assert "--expected-shards" in summary
    assert "--expected-mutants" in summary
    assert 'BASELINE_RESULT" != "success"' in terminal
    assert 'SHARD_RESULT" != "success"' in terminal


def test_baseline_features_match_mutation_profile() -> None:
    config = tomllib.loads(MUTANTS_CONFIG.read_text(encoding="utf-8"))
    baseline = named_step(workflow_text(), "Run authoritative baseline")
    feature_match = re.search(r"--features\s+([^\s]+)", baseline)

    assert feature_match is not None
    assert set(feature_match.group(1).split(",")) == set(config["features"])


def test_untrusted_dispatch_values_are_passed_via_env_and_arrays() -> None:
    text = workflow_text()

    run_blocks = "\n".join(re.findall(r"(?ms)^        run: \|\n(.*?)(?=^      - |^  [a-z])", text))
    assert "${{ github.event.inputs" not in run_blocks
    assert 'filter_args+=(--file "$FILE_FILTER")' in text
    assert '"${filter_args[@]}"' in text


def test_kani_only_corpus_is_owned_by_formal_verification() -> None:
    config = tomllib.loads(MUTANTS_CONFIG.read_text(encoding="utf-8"))
    assert "src/proof_vec.rs" in config["exclude_globs"]
    assert "kani.*proof" in config["exclude_re"]

    missing_markers: list[str] = []
    declaration = re.compile(
        r"#\[cfg\(kani\)\]\s*"
        r"(?:(?://[^\n]*\n)\s*)*"
        r"(?P<marker>#\[cfg_attr\(any\(\), mutants::skip\)\]\s*)?"
        r"mod kani(?:_[a-z_]+)?_proofs\s*\{"
    )
    for source in (REPO_ROOT / "src").rglob("*.rs"):
        for match in declaration.finditer(source.read_text(encoding="utf-8")):
            if match.group("marker") is None:
                missing_markers.append(str(source.relative_to(REPO_ROOT)))
    assert not missing_markers, f"Kani proof modules lack cargo-mutants markers: {missing_markers}"
