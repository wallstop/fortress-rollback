#!/usr/bin/env python3
"""Validate repository skills against the Agent Skills open format."""

from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover - exercised only in dependency-poor environments
    yaml = None


MAX_SKILL_LINES = 500
MAX_NAME_LENGTH = 64
MAX_DESCRIPTION_LENGTH = 1024
MAX_COMPATIBILITY_LENGTH = 500
NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
ALLOWED_FIELDS = {
    "allowed-tools",
    "compatibility",
    "description",
    "license",
    "metadata",
    "name",
}


if yaml is not None:

    class _UniqueKeyLoader(yaml.SafeLoader):
        """Safe YAML loader that rejects duplicate mapping keys."""


    def _construct_unique_mapping(
        loader: _UniqueKeyLoader,
        node: yaml.MappingNode,
        deep: bool = False,
    ) -> dict[object, object]:
        """Construct one mapping while failing closed on duplicate keys."""
        mapping: dict[object, object] = {}
        for key_node, value_node in node.value:
            key = loader.construct_object(key_node, deep=deep)
            try:
                duplicate = key in mapping
            except TypeError as exc:
                raise yaml.constructor.ConstructorError(
                    "while constructing a mapping",
                    node.start_mark,
                    "found an unhashable mapping key",
                    key_node.start_mark,
                ) from exc
            if duplicate:
                raise yaml.constructor.ConstructorError(
                    "while constructing a mapping",
                    node.start_mark,
                    f"found duplicate key {key!r}",
                    key_node.start_mark,
                )
            mapping[key] = loader.construct_object(value_node, deep=deep)
        return mapping


    _UniqueKeyLoader.add_constructor(
        yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG,
        _construct_unique_mapping,
    )


def _display(path: Path, repo_root: Path) -> str:
    """Return a stable repository-relative path for diagnostics."""
    try:
        return path.relative_to(repo_root).as_posix()
    except ValueError:
        return path.as_posix()


def _parse_frontmatter(
    skill_file: Path, repo_root: Path
) -> tuple[dict[str, object], list[str]]:
    """Parse YAML frontmatter without accepting duplicate mapping keys."""
    display_path = _display(skill_file, repo_root)
    try:
        content = skill_file.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        return {}, [f"{display_path}:0: cannot read file: {exc}"]

    lines = content.splitlines()
    if not lines or lines[0] != "---":
        return {}, [f"{display_path}:1: SKILL.md must start with YAML frontmatter"]

    try:
        closing_index = lines.index("---", 1)
    except ValueError:
        return {}, [
            f"{display_path}:1: YAML frontmatter is missing its closing delimiter"
        ]

    errors: list[str] = []
    fields: dict[str, object] = {}
    if yaml is None:
        errors.append(
            f"{display_path}:0: PyYAML is required to validate SKILL.md frontmatter"
        )
    else:
        frontmatter = "\n".join(lines[1:closing_index])
        try:
            parsed = yaml.load(frontmatter, Loader=_UniqueKeyLoader)
        except yaml.YAMLError as exc:
            mark = getattr(exc, "problem_mark", None)
            line_number = (mark.line + 2) if mark is not None else 1
            errors.append(
                f"{display_path}:{line_number}: invalid YAML frontmatter: {exc}"
            )
        else:
            if not isinstance(parsed, dict):
                errors.append(
                    f"{display_path}:1: YAML frontmatter must be a mapping"
                )
            elif not all(isinstance(key, str) for key in parsed):
                errors.append(
                    f"{display_path}:1: YAML frontmatter keys must be strings"
                )
            else:
                fields = parsed

    if not any(line.strip() for line in lines[closing_index + 1 :]):
        errors.append(
            f"{display_path}:0: SKILL.md must contain Markdown instructions"
        )

    if len(lines) > MAX_SKILL_LINES:
        errors.append(
            f"{display_path}:0: has {len(lines)} lines and exceeds "
            f"{MAX_SKILL_LINES} lines"
        )

    return fields, errors


def _validate_skill(skill_file: Path, repo_root: Path) -> list[str]:
    """Validate one SKILL.md file and return every detected error."""
    display_path = _display(skill_file, repo_root)
    fields, errors = _parse_frontmatter(skill_file, repo_root)

    for required in ("name", "description"):
        if required not in fields:
            errors.append(
                f"{display_path}:0: missing required '{required}' frontmatter field"
            )

    unexpected = sorted(set(fields) - ALLOWED_FIELDS)
    if unexpected:
        errors.append(
            f"{display_path}:0: unexpected frontmatter fields: "
            f"{', '.join(unexpected)}"
        )

    if "name" in fields:
        name = fields["name"]
        if not isinstance(name, str):
            errors.append(f"{display_path}:0: name must be a string")
        elif not name.strip():
            errors.append(f"{display_path}:0: name must be a non-empty string")
        else:
            if len(name) > MAX_NAME_LENGTH or NAME_RE.fullmatch(name) is None:
                errors.append(f"{display_path}:0: invalid skill name '{name}'")
            if name != skill_file.parent.name:
                errors.append(
                    f"{display_path}:0: name '{name}' must match its parent directory "
                    f"'{skill_file.parent.name}'"
                )

    if "description" in fields:
        description = fields["description"]
        if not isinstance(description, str):
            errors.append(f"{display_path}:0: description must be a string")
        elif not description.strip():
            errors.append(
                f"{display_path}:0: description must be a non-empty string"
            )
        else:
            if len(description) > MAX_DESCRIPTION_LENGTH:
                errors.append(
                    f"{display_path}:0: description exceeds 1024 characters"
                )
            if re.search(r"\buse when\b", description, re.IGNORECASE) is None:
                errors.append(
                    f"{display_path}:0: description must explain when to use the skill"
                )

    for scalar_field in ("license", "allowed-tools"):
        value = fields.get(scalar_field)
        if value is not None and (not isinstance(value, str) or not value.strip()):
            errors.append(
                f"{display_path}:0: {scalar_field} must be a non-empty string"
            )

    compatibility = fields.get("compatibility")
    if compatibility is not None:
        if not isinstance(compatibility, str) or not compatibility.strip():
            errors.append(
                f"{display_path}:0: compatibility must be a non-empty string"
            )
        elif len(compatibility) > MAX_COMPATIBILITY_LENGTH:
            errors.append(
                f"{display_path}:0: compatibility exceeds 500 characters"
            )

    metadata = fields.get("metadata")
    if metadata is not None:
        if not isinstance(metadata, dict):
            errors.append(
                f"{display_path}:0: metadata must be a string-to-string mapping"
            )
        elif not all(
            isinstance(key, str) and isinstance(value, str)
            for key, value in metadata.items()
        ):
            errors.append(
                f"{display_path}:0: metadata must be a string-to-string mapping"
            )

    return errors


def validate_repository(repo_root: Path) -> list[str]:
    """Validate the complete project-level skill collection."""
    errors: list[str] = []
    if (repo_root / ".llm").exists():
        errors.append(
            ".llm:0: legacy .llm directory remains after the Agent Skills migration"
        )

    skills_dir = repo_root / ".agents" / "skills"
    if not skills_dir.is_dir():
        errors.append(".agents/skills:0: directory is missing")
        return errors

    skill_directories = sorted(path for path in skills_dir.iterdir() if path.is_dir())
    if not skill_directories:
        errors.append(".agents/skills:0: contains no skills")

    for directory in skill_directories:
        skill_file = directory / "SKILL.md"
        if not skill_file.is_file():
            errors.append(f"{_display(directory, repo_root)}:0: missing SKILL.md")
            continue
        errors.extend(_validate_skill(skill_file, repo_root))

    return errors


def main() -> int:
    """Run repository validation and print actionable diagnostics."""
    repo_root = Path(__file__).resolve().parents[2]
    errors = validate_repository(repo_root)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    skill_count = sum(1 for _ in (repo_root / ".agents" / "skills").glob("*/SKILL.md"))
    print(f"Validated {skill_count} Agent Skills.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
