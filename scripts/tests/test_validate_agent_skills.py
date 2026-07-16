"""Contract tests for Agent Skills open-format validation."""

from __future__ import annotations

import importlib.util
import re
from pathlib import Path

import pytest


SCRIPTS_DIR = Path(__file__).parent.parent
SPEC = importlib.util.spec_from_file_location(
    "validate_agent_skills",
    SCRIPTS_DIR / "hooks" / "validate-agent-skills.py",
)
assert SPEC is not None
assert SPEC.loader is not None
validate_agent_skills = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validate_agent_skills)

EXPECTED_MIGRATED_SKILLS = {
    "adversarial-handoff",
    "adversarial-review",
    "api-design",
    "async-rust",
    "binary-size",
    "changelog",
    "ci-debugging",
    "clippy",
    "code-review",
    "concurrency",
    "cross-platform",
    "defensive-programming",
    "dependency-management",
    "design-decisions",
    "determinism",
    "dev-pipeline",
    "doc-code-sync",
    "dst",
    "ffi",
    "fortress-development",
    "fuzzing",
    "github-actions",
    "investigation",
    "kani",
    "loom",
    "markdown",
    "miri",
    "mutation-testing",
    "network-chaos-testing",
    "no-std",
    "performance",
    "property-testing",
    "publishing",
    "refactoring",
    "review-readiness",
    "rollback",
    "rust-design-patterns",
    "rust-idioms",
    "rust-pitfalls",
    "scripting",
    "testing",
    "text-parsing",
    "type-driven-design",
    "user-facing-docs",
    "verification",
    "wasm",
    "wiki-sync",
    "workspace",
    "zero-copy",
}


def _write_skill(
    root: Path,
    name: str = "test-skill",
    description: str = "Runs a test workflow. Use when validating test inputs.",
    body: str = "# Test skill\n\nFollow the workflow.\n",
) -> Path:
    skill = root / ".agents" / "skills" / name / "SKILL.md"
    skill.parent.mkdir(parents=True)
    skill.write_text(
        f"---\nname: {name}\ndescription: {description}\n---\n\n{body}",
        encoding="utf-8",
    )
    return skill


def test_valid_skill_passes(tmp_path: Path) -> None:
    _write_skill(tmp_path)

    assert validate_agent_skills.validate_repository(tmp_path) == []


@pytest.mark.parametrize(
    ("name", "expected"),
    [
        ("Uppercase", "invalid skill name"),
        ("double--hyphen", "invalid skill name"),
        ("-leading", "invalid skill name"),
    ],
)
def test_invalid_skill_names_fail(
    tmp_path: Path, name: str, expected: str
) -> None:
    _write_skill(tmp_path, name=name)

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any(expected in error for error in errors)


def test_skill_diagnostics_use_editor_parseable_locations(tmp_path: Path) -> None:
    _write_skill(tmp_path, name="Uppercase")

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert errors
    assert all(
        re.match(r"^\.agents/skills/[^:]+(?:/SKILL\.md)?:\d+:", error)
        for error in errors
    )


def test_frontmatter_name_must_match_directory(tmp_path: Path) -> None:
    skill = _write_skill(tmp_path)
    skill.write_text(
        skill.read_text(encoding="utf-8").replace(
            "name: test-skill", "name: another-skill"
        ),
        encoding="utf-8",
    )

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any("must match its parent directory" in error for error in errors)


@pytest.mark.parametrize("missing_field", ["name", "description"])
def test_required_frontmatter_fields_fail(
    tmp_path: Path, missing_field: str
) -> None:
    skill = _write_skill(tmp_path)
    lines = [
        line
        for line in skill.read_text(encoding="utf-8").splitlines()
        if not line.startswith(f"{missing_field}:")
    ]
    skill.write_text("\n".join(lines) + "\n", encoding="utf-8")

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any(f"missing required '{missing_field}'" in error for error in errors)


def test_description_must_explain_when_to_trigger(tmp_path: Path) -> None:
    _write_skill(tmp_path, description="Runs a test workflow.")

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any("must explain when to use the skill" in error for error in errors)


def test_description_and_body_limits_fail(tmp_path: Path) -> None:
    _write_skill(
        tmp_path,
        description=f"{'x' * 1025} Use when testing.",
        body="\n".join("line" for _ in range(501)),
    )

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any("description exceeds 1024 characters" in error for error in errors)
    assert any("exceeds 500 lines" in error for error in errors)


def test_invalid_yaml_frontmatter_fails(tmp_path: Path) -> None:
    _write_skill(
        tmp_path,
        description="Use when: validating malformed YAML.",
    )

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any("invalid YAML frontmatter" in error for error in errors)


def test_quoted_description_with_colon_passes(tmp_path: Path) -> None:
    _write_skill(
        tmp_path,
        description='"Use when: validating quoted YAML."',
    )

    assert validate_agent_skills.validate_repository(tmp_path) == []


@pytest.mark.parametrize(
    ("field", "value", "expected"),
    [
        ("name", "null", "name must be a string"),
        ("name", "''", "name must be a non-empty string"),
        ("description", "false", "description must be a string"),
        ("description", "''", "description must be a non-empty string"),
    ],
)
def test_required_fields_must_be_non_empty_strings(
    tmp_path: Path,
    field: str,
    value: str,
    expected: str,
) -> None:
    skill = _write_skill(tmp_path)
    lines = skill.read_text(encoding="utf-8").splitlines()
    for index, line in enumerate(lines):
        if line.startswith(f"{field}:"):
            lines[index] = f"{field}: {value}"
            break
    skill.write_text("\n".join(lines) + "\n", encoding="utf-8")

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any(expected in error for error in errors)


def test_duplicate_frontmatter_key_fails(tmp_path: Path) -> None:
    skill = _write_skill(tmp_path)
    content = skill.read_text(encoding="utf-8").replace(
        "description: Runs a test workflow. Use when validating test inputs.",
        "description: Runs a test workflow. Use when validating test inputs.\n"
        "description: Duplicate. Use when testing.",
    )
    skill.write_text(content, encoding="utf-8")

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any("duplicate key" in error for error in errors)


def test_unexpected_frontmatter_field_fails(tmp_path: Path) -> None:
    skill = _write_skill(tmp_path)
    content = skill.read_text(encoding="utf-8").replace(
        "---\n\n# Test skill",
        "owner: fortress\n---\n\n# Test skill",
    )
    skill.write_text(content, encoding="utf-8")

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any("unexpected frontmatter fields: owner" in error for error in errors)


def test_supported_optional_frontmatter_fields_pass(tmp_path: Path) -> None:
    skill = _write_skill(tmp_path)
    content = skill.read_text(encoding="utf-8").replace(
        "---\n\n# Test skill",
        "license: MIT\n"
        "compatibility: Requires Rust and Cargo\n"
        "allowed-tools: Bash(cargo:*) Read\n"
        "metadata:\n"
        "  author: fortress\n"
        "  version: '1.0'\n"
        "---\n\n# Test skill",
    )
    skill.write_text(content, encoding="utf-8")

    assert validate_agent_skills.validate_repository(tmp_path) == []


def test_optional_frontmatter_constraints_fail(tmp_path: Path) -> None:
    skill = _write_skill(tmp_path)
    content = skill.read_text(encoding="utf-8").replace(
        "---\n\n# Test skill",
        f"compatibility: {'x' * 501}\n"
        "metadata:\n"
        "  version: 1\n"
        "---\n\n# Test skill",
    )
    skill.write_text(content, encoding="utf-8")

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any("compatibility exceeds 500 characters" in error for error in errors)
    assert any("metadata must be a string-to-string mapping" in error for error in errors)


def test_skill_directory_requires_skill_md(tmp_path: Path) -> None:
    directory = tmp_path / ".agents" / "skills" / "empty-skill"
    directory.mkdir(parents=True)
    (directory / "references").mkdir()

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any("missing SKILL.md" in error for error in errors)


def test_legacy_llm_tree_fails_after_migration(tmp_path: Path) -> None:
    _write_skill(tmp_path)
    legacy = tmp_path / ".llm"
    legacy.mkdir()
    (legacy / "context.md").write_text("legacy\n", encoding="utf-8")

    errors = validate_agent_skills.validate_repository(tmp_path)

    assert any("legacy .llm directory" in error for error in errors)


def test_repository_contains_complete_migrated_corpus() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    skills_root = repo_root / ".agents" / "skills"
    actual = {path.parent.name for path in skills_root.glob("*/SKILL.md")}

    assert actual == EXPECTED_MIGRATED_SKILLS
    assert not (repo_root / ".llm").exists()
    assert (
        skills_root / "fortress-development" / "assets" / "ask-user-question.md"
    ).is_file()
    assert (
        skills_root / "design-decisions" / "references" / "network.txt"
    ).is_file()
