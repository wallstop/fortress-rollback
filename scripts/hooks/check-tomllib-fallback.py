#!/usr/bin/env python3
"""Pre-commit hook: require a `tomli` fallback wherever `tomllib` is imported.

`tomllib` is in the standard library only on Python 3.11+. The repo tooling
targets a wider Python range -- `scripts/hooks/check-toml.py` degrades to the
`tomli` backport -- so any Python file that imports `tomllib` must also import
`tomli` (the fallback). Otherwise a bare `import tomllib` raises
ModuleNotFoundError at import time on Python < 3.11, taking down the whole module
(or, for a test module, the entire test collection) before a single check runs.

The canonical pattern (see scripts/hooks/check-toml.py)::

    try:
        import tomllib
    except ImportError:  # Python < 3.11
        import tomli as tomllib

Detection uses the `ast` module, so it sees only REAL import statements -- a
`tomllib` mentioned in a comment, string, or docstring code example is never
flagged, and an `import tomli as tomllib` (whose module name is `tomli`) is
correctly recognized as the fallback. A file is flagged iff it really imports
`tomllib` (via `import` or `from`) but never imports `tomli`.
"""

from __future__ import annotations

import ast
import sys
from pathlib import Path


def _display_path(filepath: str | Path) -> str:
    """Convert a file path to a relative display path.

    Pre-commit sets CWD to the repo root, so paths relative to CWD
    are also relative to the project root.
    """
    try:
        return str(Path(filepath).resolve().relative_to(Path.cwd().resolve()))
    except ValueError:
        return str(filepath)


def _imports_module(node: ast.AST, name: str) -> bool:
    """Return True if *node* is a real import of top-level module *name*.

    Matches both ``import name`` / ``import name as alias`` (the module's real
    name is ``alias.name``, unaffected by ``as``) and ``from name import ...``.
    """
    if isinstance(node, ast.Import):
        return any(alias.name == name for alias in node.names)
    if isinstance(node, ast.ImportFrom):
        return node.module == name
    return False


def check_file(path: Path) -> list[str]:
    """Return error messages if *path* imports tomllib without a tomli fallback."""
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        return [f"{_display_path(path)}:0: cannot read file: {exc}"]

    try:
        tree = ast.parse(text)
    except SyntaxError:
        # Not parseable Python -- it cannot import anything at runtime, and the
        # syntax error itself is caught by the compiler / other tooling.
        return []

    tomllib_node: ast.AST | None = None
    has_tomli = False
    for node in ast.walk(tree):
        if tomllib_node is None and _imports_module(node, "tomllib"):
            tomllib_node = node
        if _imports_module(node, "tomli"):
            has_tomli = True

    if tomllib_node is None or has_tomli:
        return []

    line = getattr(tomllib_node, "lineno", 0)
    return [
        f"{_display_path(path)}:{line}: imports `tomllib` with no `tomli` fallback "
        f"(tomllib is stdlib only on Python 3.11+)"
    ]


def main() -> int:
    """Check all .py files passed as arguments."""
    errors: list[str] = []
    for arg in sys.argv[1:]:
        path = Path(arg)
        if path.suffix == ".py":
            errors.extend(check_file(path))

    if errors:
        print(
            "ERROR: importing `tomllib` requires a `tomli` fallback for Python < 3.11:",
            file=sys.stderr,
        )
        for err in errors:
            print(err, file=sys.stderr)
        print(
            "\nWrap the import so older interpreters use the backport "
            "(mirrors scripts/hooks/check-toml.py):\n"
            "    try:\n"
            "        import tomllib\n"
            "    except ImportError:  # Python < 3.11\n"
            "        import tomli as tomllib",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
