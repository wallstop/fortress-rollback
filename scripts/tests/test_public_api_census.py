"""Regression tests for the rustdoc-JSON public API census."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "public_api_census",
    SCRIPTS_DIR / "api" / "public_api_census.py",
)
assert SPEC is not None and SPEC.loader is not None
public_api_census = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = public_api_census
SPEC.loader.exec_module(public_api_census)

RustdocSurface = public_api_census.RustdocSurface
merge_rows = public_api_census.merge_rows
validate_removals = public_api_census.validate_removals


def _item(
    item_id: int,
    name: str | None,
    visibility: str,
    inner: dict,
    *,
    hidden: bool = False,
    crate_id: int = 0,
) -> dict:
    attrs = [{"other": "#[doc(hidden)]"}] if hidden else []
    return {
        "id": item_id,
        "crate_id": crate_id,
        "name": name,
        "span": {
            "filename": "src/fixture.rs",
            "begin": [item_id, 1],
            "end": [item_id, 2],
        },
        "visibility": visibility,
        "docs": "fixture docs",
        "links": {},
        "attrs": attrs,
        "deprecation": None,
        "inner": inner,
    }


def _fixture() -> dict:
    items = {
        1: _item(
            1,
            "fortress_rollback",
            "public",
            {"module": {"is_crate": True, "items": [2, 3, 9, 13]}},
        ),
        2: _item(
            2,
            "hidden",
            "public",
            {"module": {"is_crate": False, "items": [4]}},
            hidden=True,
        ),
        3: _item(
            3,
            None,
            "public",
            {
                "use": {
                    "source": "hidden::Thing",
                    "name": "Thing",
                    "id": 4,
                    "is_glob": False,
                }
            },
        ),
        4: _item(
            4,
            "Thing",
            "public",
            {"struct": {"kind": {"plain": {"fields": [5]}}, "impls": [6]}},
        ),
        5: _item(5, "value", "public", {"struct_field": {"type": {"primitive": "u8"}}}),
        6: _item(
            6,
            None,
            "default",
            {
                "impl": {
                    "is_unsafe": False,
                    "generics": {"params": [], "where_predicates": []},
                    "provided_trait_methods": [],
                    "trait": None,
                    "for": {"resolved_path": {"name": "Thing", "id": 4, "args": {}}},
                    "items": [7, 8],
                    "is_negative": False,
                    "is_synthetic": False,
                    "blanket_impl": None,
                }
            },
        ),
        7: _item(7, "visible_method", "public", {"function": {}}),
        8: _item(8, "private_method", "crate", {"function": {}}),
        9: _item(
            9,
            None,
            "public",
            {
                "use": {
                    "source": "exports::*",
                    "name": "exports",
                    "id": 10,
                    "is_glob": True,
                }
            },
        ),
        10: _item(
            10,
            "exports",
            "public",
            {"module": {"is_crate": False, "items": [11, 12]}},
        ),
        11: _item(11, "Exported", "public", {"struct": {"kind": "unit", "impls": []}}),
        12: _item(
            12, "NotExported", "crate", {"struct": {"kind": "unit", "impls": []}}
        ),
        13: _item(
            13,
            None,
            "public",
            {
                "use": {
                    "source": "external_crate::External",
                    "name": "External",
                    "id": 14,
                    "is_glob": False,
                }
            },
        ),
        14: _item(
            14,
            "External",
            "public",
            {"struct": {"kind": "unit", "impls": []}},
            crate_id=1,
        ),
    }
    return {
        "root": 1,
        "crate_version": "0.0.0",
        "includes_private": True,
        "index": {str(item_id): item for item_id, item in items.items()},
        "paths": {
            "14": {
                "crate_id": 1,
                "path": ["external_crate", "External"],
                "kind": "struct",
            }
        },
        "external_crates": {"1": {"name": "external_crate", "html_root_url": None}},
        "format_version": 60,
    }


def test_hidden_module_alias_and_associated_items_are_preserved() -> None:
    rows = RustdocSurface(_fixture(), "no-default").extract()
    by_path = {row.path: row for row in rows}

    assert by_path["fortress_rollback::Thing"].kind == "struct"
    assert by_path["fortress_rollback::Thing"].documented
    assert by_path["fortress_rollback::Thing::value"].kind == "struct_field"
    assert by_path["fortress_rollback::Thing::visible_method"].kind == "function"
    assert "fortress_rollback::Thing::private_method" not in by_path

    hidden_alias = by_path["fortress_rollback::hidden::Thing"]
    assert hidden_alias.kind == "alias"
    assert hidden_alias.alias_of == "fortress_rollback::Thing"
    assert not hidden_alias.documented
    hidden_field = by_path["fortress_rollback::hidden::Thing::value"]
    assert hidden_field.kind == "struct_field"
    assert hidden_field.alias_of == "fortress_rollback::Thing::value"
    assert not hidden_field.documented
    hidden_method = by_path["fortress_rollback::hidden::Thing::visible_method"]
    assert hidden_method.kind == "function"
    assert hidden_method.alias_of == "fortress_rollback::Thing::visible_method"
    assert not hidden_method.documented


def test_glob_and_external_aliases_are_censused_without_private_children() -> None:
    rows = RustdocSurface(_fixture(), "all-features").extract()
    by_path = {row.path: row for row in rows}

    assert by_path["fortress_rollback::Exported"].kind == "struct"
    assert "fortress_rollback::NotExported" not in by_path
    external = by_path["fortress_rollback::External"]
    assert external.kind == "external-struct"
    assert external.alias_of == "external_crate::External"


def test_profile_merge_is_sorted_and_records_availability() -> None:
    fixture = _fixture()
    default_rows = RustdocSurface(fixture, "no-default").extract()
    all_rows = RustdocSurface(fixture, "all-features").extract()
    snapshot = merge_rows((default_rows, all_rows), "nightly-2099-01-01")

    assert "# symbols=" in snapshot
    assert "# textual-paths=" in snapshot
    assert (
        "fortress_rollback::Thing\tstruct\tno-default+all-features\tyes\t" in snapshot
    )
    data = [line for line in snapshot.splitlines() if not line.startswith("#")][1:]
    assert data == sorted(data)


def test_removal_ledger_requires_absent_paths_and_public_replacements(
    tmp_path: Path,
) -> None:
    snapshot = merge_rows(
        (RustdocSurface(_fixture(), "no-default").extract(),),
        "nightly-2099-01-01",
    )
    ledger = tmp_path / "removals.tsv"
    ledger.write_text(
        "# schema=1\n"
        "path\towner\tavailability\tusage-evidence\trisk\tdisposition\t"
        "replacement\trationale\n"
        "fortress_rollback::OldThing\tcore\tno-default\tno-callers\t"
        "duplicate-callable-path\tremove-reviewed\tfortress_rollback::Thing\t"
        "Use the canonical path.\n",
        encoding="utf-8",
    )

    validate_removals(snapshot, ledger)

    ledger.write_text(
        ledger.read_text(encoding="utf-8").replace(
            "fortress_rollback::OldThing", "fortress_rollback::Thing"
        ),
        encoding="utf-8",
    )
    try:
        validate_removals(snapshot, ledger)
    except ValueError as error:
        assert "remains in public API snapshot" in str(error)
    else:
        raise AssertionError("a removal may not remain in the public API snapshot")
