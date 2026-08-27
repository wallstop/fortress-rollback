#!/usr/bin/env python3
"""Generate the deterministic Fortress Rollback public API census.

The normal rustdoc index omits ``#[doc(hidden)]`` paths even though those items
remain callable and semver-relevant. This tool builds rustdoc JSON with private
and hidden items available, then follows only public modules and re-exports
from the crate root. Public associated items, enum variants, and public data
shape are included. Explicit and module-path aliases are retained as separate
rows so removing a duplicate path changes the checked snapshot.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

REPO_ROOT = Path(__file__).resolve().parents[2]
PIN_PATH = REPO_ROOT / ".github" / "actions" / "install-pinned-nightly" / "toolchain"
SNAPSHOT_PATH = REPO_ROOT / "docs" / "api" / "public-api-snapshot.tsv"
REMOVALS_PATH = REPO_ROOT / "docs" / "api" / "public-api-removals.tsv"
CRATE_NAME = "fortress_rollback"
SCHEMA_VERSION = 1
API_NEUTRAL_FEATURES = {"z3-verification", "z3-verification-bundled"}
AMBIENT_RUST_BUILD_FLAGS = {
    "CARGO_BUILD_TARGET",
    "CARGO_ENCODED_RUSTFLAGS",
    "RUSTDOCFLAGS",
    "RUSTFLAGS",
}


def _item_id(value: object) -> int | None:
    if value is None:
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, str) and value.isdigit():
        return int(value)
    raise ValueError(f"unsupported rustdoc item id: {value!r}")


def _item_kind(item: dict[str, Any]) -> str:
    inner = item.get("inner")
    if not isinstance(inner, dict) or len(inner) != 1:
        raise ValueError(f"rustdoc item {item.get('id')} has invalid inner payload")
    return str(next(iter(inner)))


def _is_doc_hidden(item: dict[str, Any]) -> bool:
    return any(
        "doc(hidden)" in str(attr).replace(" ", "") for attr in item.get("attrs", [])
    )


def _field_ids(kind: object) -> list[int]:
    if not isinstance(kind, dict):
        return []
    candidates: Iterable[object]
    if "tuple" in kind:
        candidates = kind["tuple"] or []
    elif "plain" in kind and isinstance(kind["plain"], dict):
        candidates = kind["plain"].get("fields", [])
    elif "struct" in kind and isinstance(kind["struct"], dict):
        candidates = kind["struct"].get("fields", [])
    else:
        candidates = []
    return [item_id for raw in candidates if (item_id := _item_id(raw)) is not None]


@dataclass(frozen=True)
class ExportPath:
    target: int
    path: tuple[str, ...]
    hidden: bool
    explicit_alias: bool
    source_item: int | None


@dataclass(frozen=True)
class SurfaceRow:
    path: str
    kind: str
    profile: str
    documented: bool
    alias_of: str
    source: str


class RustdocSurface:
    """Extract externally reachable paths from one rustdoc JSON document."""

    def __init__(self, document: dict[str, Any], profile: str) -> None:
        self.document = document
        self.profile = profile
        self.index = {
            item_id: item
            for raw_id, item in document.get("index", {}).items()
            if (item_id := _item_id(raw_id)) is not None
        }
        self.paths = {
            item_id: path
            for raw_id, path in document.get("paths", {}).items()
            if (item_id := _item_id(raw_id)) is not None
        }
        self.exports: dict[int, dict[tuple[str, ...], ExportPath]] = {}
        self._walked_modules: set[tuple[int, tuple[str, ...]]] = set()
        self._rows: dict[tuple[str, str, str], SurfaceRow] = {}
        self._alias_prefixes: dict[tuple[str, ...], tuple[str, ...]] = {}

    def extract(self) -> list[SurfaceRow]:
        root = _item_id(self.document.get("root"))
        if root is None or root not in self.index:
            raise ValueError("rustdoc JSON is missing the local crate root")
        root_name = self.index[root].get("name") or CRATE_NAME
        self._add_export(root, (root_name,), False, False, None)
        self._walk_module(root, (root_name,), False)

        for target, paths in sorted(self.exports.items()):
            ordered = sorted(
                paths.values(),
                key=lambda export: (len(export.path), export.path, export.hidden),
            )
            if not ordered:
                continue
            primary = ordered[0]
            target_item = self.index.get(target)
            if target_item is None or target_item.get("crate_id") != 0:
                canonical = self._external_path(target)
                for export in ordered:
                    self._emit(
                        export.path,
                        self._external_kind(target),
                        not export.hidden,
                        canonical,
                        self._source_for(export.source_item),
                    )
                continue

            self._add_declaration(target, primary.path, primary.hidden, set())
            primary_text = "::".join(primary.path)
            for export in ordered[1:]:
                self._emit(
                    export.path,
                    "alias",
                    not export.hidden and not _is_doc_hidden(target_item),
                    primary_text,
                    self._source_for(export.source_item),
                )
                self._alias_prefixes[export.path] = primary.path
                self._add_declaration(
                    target,
                    export.path,
                    export.hidden,
                    set(),
                    emit_self=False,
                )

        return sorted(
            self._rows.values(), key=lambda row: (row.path, row.kind, row.alias_of)
        )

    def _add_export(
        self,
        target: int,
        path: tuple[str, ...],
        hidden: bool,
        explicit_alias: bool,
        source_item: int | None,
    ) -> None:
        record = ExportPath(target, path, hidden, explicit_alias, source_item)
        existing = self.exports.setdefault(target, {}).get(path)
        if existing is None or (existing.hidden and not hidden):
            self.exports[target][path] = record

    def _walk_module(self, module_id: int, path: tuple[str, ...], hidden: bool) -> None:
        visit = (module_id, path)
        if visit in self._walked_modules:
            return
        self._walked_modules.add(visit)
        module = self.index.get(module_id)
        if module is None or _item_kind(module) != "module":
            return
        module_hidden = hidden or _is_doc_hidden(module)
        for raw_child in module["inner"]["module"].get("items", []):
            child_id = _item_id(raw_child)
            if child_id is None:
                continue
            child = self.index.get(child_id)
            if child is None or child.get("visibility") != "public":
                continue
            child_kind = _item_kind(child)
            child_hidden = module_hidden or _is_doc_hidden(child)
            if child_kind == "use":
                use = child["inner"]["use"]
                target = _item_id(use.get("id"))
                if target is None:
                    continue
                if use.get("is_glob"):
                    self._expand_glob(target, path, child_hidden, child_id)
                    continue
                name = use.get("name")
                if not isinstance(name, str) or not name:
                    raise ValueError(f"public use item {child_id} has no stable name")
                self._add_export(target, (*path, name), child_hidden, True, child_id)
                continue

            name = child.get("name")
            if not isinstance(name, str) or not name:
                continue
            child_path = (*path, name)
            self._add_export(child_id, child_path, child_hidden, False, child_id)
            if child_kind == "module":
                self._walk_module(child_id, child_path, child_hidden)

    def _expand_glob(
        self,
        target: int,
        destination: tuple[str, ...],
        hidden: bool,
        source_item: int,
    ) -> None:
        module = self.index.get(target)
        if module is None or _item_kind(module) != "module":
            raise ValueError(f"public glob use {source_item} does not target a module")
        for raw_child in module["inner"]["module"].get("items", []):
            child_id = _item_id(raw_child)
            if child_id is None:
                continue
            child = self.index.get(child_id)
            if child is None or child.get("visibility") != "public":
                continue
            child_kind = _item_kind(child)
            child_hidden = hidden or _is_doc_hidden(child)
            if child_kind == "use":
                use = child["inner"]["use"]
                nested_target = _item_id(use.get("id"))
                name = use.get("name")
                if nested_target is not None and isinstance(name, str) and name:
                    self._add_export(
                        nested_target,
                        (*destination, name),
                        child_hidden,
                        True,
                        source_item,
                    )
                continue
            name = child.get("name")
            if isinstance(name, str) and name:
                self._add_export(
                    child_id,
                    (*destination, name),
                    child_hidden,
                    True,
                    source_item,
                )

    def _add_declaration(
        self,
        item_id: int,
        path: tuple[str, ...],
        hidden: bool,
        active: set[int],
        *,
        emit_self: bool = True,
    ) -> None:
        item = self.index.get(item_id)
        if item is None or item.get("crate_id") != 0 or item_id in active:
            return
        active = {*active, item_id}
        kind = _item_kind(item)
        item_hidden = hidden or _is_doc_hidden(item)
        if emit_self:
            self._emit(path, kind, not item_hidden, "", self._source_for(item_id))
        inner = item["inner"][kind]

        if kind == "struct":
            for field_id in _field_ids(inner.get("kind")):
                field = self.index.get(field_id)
                if field is not None and field.get("visibility") == "public":
                    self._add_named_child(field_id, path, item_hidden, active)
            self._add_inherent_items(inner.get("impls", []), path, item_hidden, active)
        elif kind == "union":
            for raw_field in inner.get("fields", []):
                union_field_id = _item_id(raw_field)
                if union_field_id is None:
                    continue
                field = self.index.get(union_field_id)
                if field is not None and field.get("visibility") == "public":
                    self._add_named_child(union_field_id, path, item_hidden, active)
            self._add_inherent_items(inner.get("impls", []), path, item_hidden, active)
        elif kind == "enum":
            for raw_variant in inner.get("variants", []):
                variant_id = _item_id(raw_variant)
                if variant_id is None or variant_id not in self.index:
                    continue
                variant = self.index[variant_id]
                variant_name = variant.get("name")
                if not isinstance(variant_name, str) or not variant_name:
                    continue
                variant_path = (*path, variant_name)
                self._add_declaration(variant_id, variant_path, item_hidden, active)
                variant_kind = variant["inner"]["variant"].get("kind")
                for field_id in _field_ids(variant_kind):
                    self._add_named_child(field_id, variant_path, item_hidden, active)
            self._add_inherent_items(inner.get("impls", []), path, item_hidden, active)
        elif kind == "trait":
            for raw_child in inner.get("items", []):
                child_id = _item_id(raw_child)
                if child_id is not None:
                    self._add_named_child(child_id, path, item_hidden, active)
        elif kind == "primitive":
            self._add_inherent_items(inner.get("impls", []), path, item_hidden, active)

    def _add_named_child(
        self,
        child_id: int,
        parent_path: tuple[str, ...],
        hidden: bool,
        active: set[int],
    ) -> None:
        child = self.index.get(child_id)
        if child is None:
            return
        name = child.get("name")
        if isinstance(name, str) and name:
            self._add_declaration(child_id, (*parent_path, name), hidden, active)

    def _add_inherent_items(
        self,
        raw_impls: Iterable[object],
        parent_path: tuple[str, ...],
        hidden: bool,
        active: set[int],
    ) -> None:
        for raw_impl in raw_impls:
            impl_id = _item_id(raw_impl)
            impl_item = self.index.get(impl_id) if impl_id is not None else None
            if impl_item is None or _item_kind(impl_item) != "impl":
                continue
            impl = impl_item["inner"]["impl"]
            if impl.get("trait") is not None:
                continue
            for raw_child in impl.get("items", []):
                child_id = _item_id(raw_child)
                if child_id is None:
                    continue
                child = self.index.get(child_id)
                if child is not None and child.get("visibility") == "public":
                    self._add_named_child(child_id, parent_path, hidden, active)

    def _emit(
        self,
        path: tuple[str, ...],
        kind: str,
        documented: bool,
        alias_of: str,
        source: str,
    ) -> None:
        text_path = "::".join(path)
        if not alias_of:
            matching_prefixes = [
                (alias, canonical)
                for alias, canonical in self._alias_prefixes.items()
                if len(path) > len(alias) and path[: len(alias)] == alias
            ]
            if matching_prefixes:
                alias, canonical = max(matching_prefixes, key=lambda pair: len(pair[0]))
                alias_of = "::".join((*canonical, *path[len(alias) :]))
        row = SurfaceRow(text_path, kind, self.profile, documented, alias_of, source)
        key = (text_path, kind, alias_of)
        existing = self._rows.get(key)
        if existing is None or (not existing.documented and documented):
            self._rows[key] = row

    def _source_for(self, item_id: int | None) -> str:
        item = self.index.get(item_id) if item_id is not None else None
        span = item.get("span") if item is not None else None
        if not isinstance(span, dict):
            return "external"
        return str(span.get("filename", "unknown"))

    def _external_path(self, item_id: int) -> str:
        path = self.paths.get(item_id, {}).get("path")
        if isinstance(path, list) and path:
            return "::".join(str(part) for part in path)
        item = self.index.get(item_id, {})
        return f"external::{item.get('name') or item_id}"

    def _external_kind(self, item_id: int) -> str:
        path_kind = self.paths.get(item_id, {}).get("kind")
        if isinstance(path_kind, str):
            return f"external-{path_kind}"
        item = self.index.get(item_id)
        return f"external-{_item_kind(item)}" if item is not None else "external-item"


def _owner(path: str, source: str) -> str:
    if "::__internal" in path:
        return "verification"
    if "/network/" in source or "::network::" in path or "::tokio_socket" in path:
        return "transport"
    if "/sessions/" in source or "::sessions::" in path:
        return "sessions"
    if "telemetry" in source or "metrics" in source:
        return "observability"
    if "replay" in source:
        return "replay"
    if source == "external":
        return "dependency-compatibility"
    return "core"


def _usage(row: SurfaceRow) -> str:
    if row.alias_of:
        if "::__internal::" in row.path:
            return "workspace-verification-export"
        if "::prelude::" in row.path:
            return "prelude-export"
        return "compatibility-reexport"
    if row.kind in {"variant", "struct_field"}:
        return "public-data-shape"
    if row.kind in {"trait", "assoc_type"}:
        return "implementor-contract"
    if "::__internal" in row.path:
        return "workspace-verification-api"
    return "rustdoc-contract"


def _risk(row: SurfaceRow) -> str:
    if row.kind.startswith("external-"):
        return "dependency-coupled-export"
    if row.alias_of:
        return "duplicate-callable-path"
    if not row.documented:
        return "hidden-semver-surface"
    if row.kind in {"variant", "struct_field"}:
        return "exhaustive-data-shape"
    if row.kind in {"trait", "assoc_type"}:
        return "downstream-implementation-contract"
    return "supported-public-contract"


def _disposition(row: SurfaceRow) -> str:
    if "::__internal" in row.path:
        return "keep-verification-compatibility"
    if row.alias_of or not row.documented:
        return "keep-compatibility"
    return "keep-supported"


def merge_rows(profile_rows: Iterable[Iterable[SurfaceRow]], toolchain: str) -> str:
    merged: dict[tuple[str, str, str], list[SurfaceRow]] = {}
    for rows in profile_rows:
        for row in rows:
            merged.setdefault((row.path, row.kind, row.alias_of), []).append(row)

    textual_paths = {path for path, _, _ in merged}
    lines = [
        f"# schema={SCHEMA_VERSION}",
        f"# rustdoc-toolchain={toolchain}",
        "# all-features-excludes=z3-verification,z3-verification-bundled (test/build-only)",
        "# generated-by=scripts/api/public_api_census.py",
        f"# symbols={len(merged)}",
        f"# textual-paths={len(textual_paths)}",
        "path\tkind\tavailability\tdocumented\towner\tusage-evidence\trisk\tdisposition\talias-of\tsource",
    ]
    for key in sorted(merged):
        rows = merged[key]
        profiles = {row.profile for row in rows}
        availability = (
            "no-default+all-features"
            if profiles == {"no-default", "all-features"}
            else "+".join(sorted(profiles))
        )
        exemplar = sorted(rows, key=lambda row: (not row.documented, row.source))[0]
        combined = SurfaceRow(
            exemplar.path,
            exemplar.kind,
            availability,
            any(row.documented for row in rows),
            exemplar.alias_of,
            exemplar.source,
        )
        values = (
            combined.path,
            combined.kind,
            availability,
            "yes" if combined.documented else "no",
            _owner(combined.path, combined.source),
            _usage(combined),
            _risk(combined),
            _disposition(combined),
            combined.alias_of or "-",
            combined.source,
        )
        if any("\t" in value or "\n" in value for value in values):
            raise ValueError(f"snapshot value contains a TSV delimiter: {values!r}")
        lines.append("\t".join(values))
    return "\n".join(lines) + "\n"


def _symbol_count(snapshot: str) -> int:
    return sum(
        1
        for line in snapshot.splitlines()
        if line and not line.startswith("#") and not line.startswith("path\t")
    )


def _read_document(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        document = json.load(handle)
    if not isinstance(document, dict):
        raise ValueError(f"rustdoc JSON root must be an object: {path}")
    return document


def _public_feature_set() -> list[str]:
    metadata = subprocess.run(  # noqa: S603, S607 -- fixed Cargo command
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],  # noqa: S607
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    packages = json.loads(metadata.stdout).get("packages", [])
    package = next(
        (
            candidate
            for candidate in packages
            if candidate.get("name") == "fortress-rollback"
        ),
        None,
    )
    if package is None:
        raise RuntimeError("cargo metadata did not report fortress-rollback")
    features = set(package.get("features", {}))
    missing = API_NEUTRAL_FEATURES - features
    if missing:
        raise RuntimeError(
            f"API-neutral feature exemptions disappeared: {sorted(missing)}"
        )
    for source in (REPO_ROOT / "src").rglob("*.rs"):
        text = source.read_text(encoding="utf-8")
        for feature in API_NEUTRAL_FEATURES:
            if f'feature = "{feature}"' in text:
                raise RuntimeError(
                    f"{feature} now controls production API in {source}; "
                    "remove it from API_NEUTRAL_FEATURES"
                )
    return sorted(features - API_NEUTRAL_FEATURES)


def _generate_rustdoc_json(toolchain: str, all_features: bool, target: Path) -> Path:
    command = [
        "cargo",
        f"+{toolchain}",
        "rustdoc",
        "--locked",
        "--package",
        "fortress-rollback",
        "--lib",
        "--no-default-features",
    ]
    if all_features:
        command.extend(("--features", ",".join(_public_feature_set())))
    command.extend(
        [
            "--",
            "-Z",
            "unstable-options",
            "--output-format",
            "json",
            "--document-private-items",
            "--document-hidden-items",
            "--cap-lints",
            "allow",
        ]
    )
    environment = os.environ.copy()
    for variable in AMBIENT_RUST_BUILD_FLAGS:
        environment.pop(variable, None)
    environment["CARGO_TARGET_DIR"] = str(target)
    subprocess.run(  # noqa: S603 -- fixed Cargo command plus repository feature names
        command,
        cwd=REPO_ROOT,
        env=environment,
        check=True,
    )
    output = target / "doc" / f"{CRATE_NAME}.json"
    if not output.is_file():
        raise RuntimeError(f"rustdoc did not produce expected JSON: {output}")
    return output


def _toolchain_pin() -> str:
    pin = PIN_PATH.read_text(encoding="utf-8").strip()
    if not pin.startswith("nightly-") or len(pin) != len("nightly-YYYY-MM-DD"):
        raise ValueError(f"invalid repository nightly pin: {pin!r}")
    return pin


def _snapshot(default_json: Path, all_json: Path, toolchain: str) -> str:
    default_rows = RustdocSurface(_read_document(default_json), "no-default").extract()
    all_rows = RustdocSurface(_read_document(all_json), "all-features").extract()
    return merge_rows((default_rows, all_rows), toolchain)


def validate_removals(snapshot: str, ledger_path: Path = REMOVALS_PATH) -> None:
    """Ensure reviewed removals stay distinct from the current API contract."""
    snapshot_paths = {
        line.split("\t", 1)[0]
        for line in snapshot.splitlines()
        if line and not line.startswith("#") and not line.startswith("path\t")
    }
    lines = [
        line
        for line in ledger_path.read_text(encoding="utf-8").splitlines()
        if line and not line.startswith("#")
    ]
    expected_header = (
        "path\towner\tavailability\tusage-evidence\trisk\tdisposition\t"
        "replacement\trationale"
    )
    if not lines or lines[0] != expected_header:
        raise ValueError(f"invalid public API removal ledger header: {ledger_path}")

    seen: set[str] = set()
    for line in lines[1:]:
        fields = line.split("\t")
        if len(fields) != 8:
            raise ValueError(f"invalid public API removal ledger row: {line!r}")
        path, _, _, _, _, disposition, replacement, rationale = fields
        if path in seen:
            raise ValueError(f"duplicate reviewed public API removal: {path}")
        seen.add(path)
        if path in snapshot_paths:
            raise ValueError(f"reviewed removal remains in public API snapshot: {path}")
        if disposition != "remove-reviewed":
            raise ValueError(f"reviewed removal has invalid disposition: {path}")
        if replacement not in snapshot_paths:
            raise ValueError(
                f"reviewed removal replacement is not public: {replacement}"
            )
        if not rationale:
            raise ValueError(f"reviewed removal has no rationale: {path}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument(
        "--check", action="store_true", help="compare generated census to snapshot"
    )
    mode.add_argument(
        "--write", action="store_true", help="replace the checked snapshot"
    )
    parser.add_argument(
        "--default-json", type=Path, help="use an existing no-default rustdoc JSON"
    )
    parser.add_argument(
        "--all-features-json",
        type=Path,
        help="use an existing all-feature rustdoc JSON",
    )
    parser.add_argument("--output", type=Path, default=SNAPSHOT_PATH)
    args = parser.parse_args()

    toolchain = _toolchain_pin()
    supplied = (args.default_json, args.all_features_json)
    if any(supplied) and not all(supplied):
        parser.error("--default-json and --all-features-json must be supplied together")

    if all(supplied):
        content = _snapshot(args.default_json, args.all_features_json, toolchain)
    else:
        target_root = Path(
            os.environ.get(
                "FORTRESS_API_CENSUS_TARGET_DIR",
                REPO_ROOT / "target" / "public-api-census",
            )
        )
        default_json = _generate_rustdoc_json(
            toolchain, False, target_root / "no-default"
        )
        all_json = _generate_rustdoc_json(toolchain, True, target_root / "all-features")
        content = _snapshot(default_json, all_json, toolchain)

    validate_removals(content)

    output = args.output.resolve()
    if args.write:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(content, encoding="utf-8")
        print(f"wrote {_symbol_count(content)} public API symbols to {output}")
        return 0

    if not output.is_file():
        print(f"public API snapshot is missing: {output}")
        return 1
    expected = output.read_text(encoding="utf-8")
    if expected != content:
        print("public API snapshot differs; review the semver change and run:")
        print("  python3 scripts/api/public_api_census.py --write")
        return 1
    print(f"public API snapshot matches ({_symbol_count(content)} symbols)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
