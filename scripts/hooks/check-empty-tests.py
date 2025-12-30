#!/usr/bin/env python3
"""Pre-commit hook to detect empty test methods.

Empty test methods pass trivially without testing anything. This hook catches
test methods that have only a docstring (or nothing) but no actual assertions.

Usage:
    python scripts/hooks/check-empty-tests.py [files...]
"""

from __future__ import annotations

import ast
import sys
from pathlib import Path


def is_empty_function(node: ast.FunctionDef | ast.AsyncFunctionDef) -> bool:
    """Check if a function is empty (has only pass, docstring, or ellipsis).

    Returns True if the function body contains no meaningful code.
    """
    meaningful_statements = []

    for stmt in node.body:
        # Skip docstrings (first Expr with Constant str)
        if (
            isinstance(stmt, ast.Expr)
            and isinstance(stmt.value, ast.Constant)
            and isinstance(stmt.value.value, str)
        ):
            continue

        # Skip pass statements
        if isinstance(stmt, ast.Pass):
            continue

        # Skip ellipsis (...)
        if (
            isinstance(stmt, ast.Expr)
            and isinstance(stmt.value, ast.Constant)
            and stmt.value.value is ...
        ):
            continue

        meaningful_statements.append(stmt)

    return len(meaningful_statements) == 0


def check_file(filepath: Path) -> list[str]:
    """Check a Python file for empty test methods.

    Returns a list of error messages (empty if no issues).
    """
    errors = []

    try:
        content = filepath.read_text(encoding="utf-8")
        tree = ast.parse(content, filename=str(filepath))
    except (SyntaxError, OSError, UnicodeDecodeError):
        # SyntaxError: Let other tools catch syntax errors
        # OSError: File not readable (permissions, deleted, etc.)
        # UnicodeDecodeError: Non-UTF-8 files (rare for Python tests)
        return []

    # Collect all functions that are inside classes (methods)
    methods_in_classes: set[int] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.ClassDef):
            for item in node.body:
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    methods_in_classes.add(id(item))
                    if item.name.startswith("test_") and is_empty_function(item):
                        errors.append(
                            f"{filepath}:{item.lineno}: Empty test method "
                            f"'{node.name}.{item.name}'"
                        )

    # Check top-level test functions (not methods in classes)
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if id(node) in methods_in_classes:
                continue  # Already checked as a class method
            if node.name.startswith("test_") and is_empty_function(node):
                errors.append(
                    f"{filepath}:{node.lineno}: Empty test function '{node.name}'"
                )

    return errors


def main() -> int:
    """Main entry point for the hook."""
    if len(sys.argv) < 2:
        return 0

    all_errors = []

    for arg in sys.argv[1:]:
        filepath = Path(arg)

        # Only check Python test files
        if not filepath.suffix == ".py":
            continue

        # Only check test files (test_*.py or *_test.py)
        if not (filepath.stem.startswith("test_") or filepath.stem.endswith("_test")):
            continue

        errors = check_file(filepath)
        all_errors.extend(errors)

    if all_errors:
        print("Empty test methods detected:")
        for error in all_errors:
            print(f"  {error}")
        print("\nTest methods must have at least one assertion or meaningful code.")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
