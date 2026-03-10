#!/usr/bin/env python3
"""
Cross-platform Kani proof coverage checker for pre-commit hooks.

Validates that all #[kani::proof] functions in the source code are included
in the tier lists in scripts/verify-kani.sh.

Works on Windows, macOS, and Linux.

Note: Kani itself doesn't support Windows, but this validation script
can still run to catch issues before pushing to CI.
"""

import re
import sys
from pathlib import Path


def get_script_dir() -> Path:
    """Get the directory containing this script."""
    return Path(__file__).parent.resolve()


def get_project_root() -> Path:
    """Get the project root directory."""
    return get_script_dir().parent


def find_source_proofs(project_root: Path) -> set[str]:
    """Find all proof function names in source code."""
    proofs = set()
    src_dir = project_root / "src"

    if not src_dir.exists():
        return proofs

    # Pattern to match #[kani::proof] followed by fn name
    # Look for the function name after the attribute
    kani_attr_pattern = re.compile(r"#\[kani::proof\]")
    fn_pattern = re.compile(r"fn\s+(\w+)")

    for rs_file in src_dir.rglob("*.rs"):
        try:
            content = rs_file.read_text(encoding="utf-8")
            lines = content.splitlines()

            for i, line in enumerate(lines):
                if kani_attr_pattern.search(line):
                    # Look for fn declaration on this or next few lines
                    for j in range(i, min(i + 5, len(lines))):
                        fn_match = fn_pattern.search(lines[j])
                        if fn_match:
                            proofs.add(fn_match.group(1))
                            break

        except (OSError, UnicodeDecodeError) as e:
            print(f"Warning: Could not read {rs_file}: {e}", file=sys.stderr)

    return proofs


def find_script_proofs(project_root: Path) -> set[str]:
    """Find all proof names referenced in verify-kani.sh."""
    proofs = set()
    verify_script = project_root / "scripts" / "verify-kani.sh"

    if not verify_script.exists():
        print(f"Warning: {verify_script} not found", file=sys.stderr)
        return proofs

    try:
        content = verify_script.read_text(encoding="utf-8")

        # Look for proof names in tier arrays (proof_* or verify_*)
        # These are typically in arrays like: TIER1_PROOFS=("proof_foo" "proof_bar")
        proof_pattern = re.compile(r'"((?:proof|verify)_\w+)"')

        for match in proof_pattern.finditer(content):
            proofs.add(match.group(1))

    except (OSError, UnicodeDecodeError) as e:
        print(f"Warning: Could not read {verify_script}: {e}", file=sys.stderr)

    return proofs


def check_unwind_attributes(project_root: Path, verbose: bool = False) -> None:
    """Check that #[kani::proof] functions have #[kani::unwind(N)] attributes.

    This is advisory only — many simple proofs legitimately work without
    explicit unwind bounds. The warning helps catch potential timeout issues
    in CI where --default-unwind 8 is used via --quick mode.
    """
    src_dir = project_root / "src"

    if not src_dir.exists():
        return

    kani_attr_pattern = re.compile(r"#\[kani::proof\]")
    fn_pattern = re.compile(r"fn\s+(\w+)")
    unwind_pattern = re.compile(r"#\[kani::unwind\(\d+\)\]")
    allowlist_pattern = re.compile(r"//\s*kani::no-unwind-needed")

    warnings = []

    for rs_file in src_dir.rglob("*.rs"):
        try:
            content = rs_file.read_text(encoding="utf-8")
            lines = content.splitlines()

            for i, line in enumerate(lines):
                if kani_attr_pattern.search(line):
                    # Find the fn name on this or subsequent lines
                    fn_name = None
                    for j in range(i, min(i + 10, len(lines))):
                        fn_match = fn_pattern.search(lines[j])
                        if fn_match:
                            fn_name = fn_match.group(1)
                            break

                    if fn_name is None:
                        continue

                    # Check for #[kani::unwind(N)] in the attribute block:
                    # - Up to 10 lines before #[kani::proof]
                    # - Between #[kani::proof] and the fn declaration
                    has_unwind = False
                    start = max(0, i - 10)
                    end = min(i + 10, len(lines))
                    for k in range(start, end):
                        if unwind_pattern.search(lines[k]):
                            has_unwind = True
                            break

                    if has_unwind:
                        continue

                    # Check for // kani::no-unwind-needed allowlist marker
                    # in the preceding 3 lines
                    has_allowlist = False
                    allow_start = max(0, i - 3)
                    for k in range(allow_start, i + 1):
                        if allowlist_pattern.search(lines[k]):
                            has_allowlist = True
                            break

                    if has_allowlist:
                        continue

                    rel_path = rs_file.relative_to(project_root)
                    warnings.append((fn_name, str(rel_path)))

        except (OSError, UnicodeDecodeError) as e:
            print(f"Warning: Could not read {rs_file}: {e}", file=sys.stderr)

    if warnings:
        if verbose:
            print(f"\n[Advisory] {len(warnings)} proof(s) without explicit #[kani::unwind(N)]:")
            for fn_name, file_path in sorted(warnings):
                print(
                    f"  WARNING: proof '{fn_name}' in file '{file_path}' has no explicit "
                    f"#[kani::unwind(N)]. CI uses --default-unwind 8; larger data "
                    f"structures may cause timeouts."
                )
        else:
            print(
                f"\n[Advisory] {len(warnings)} proof(s) without explicit "
                f"#[kani::unwind(N)]. Run with --verbose for details."
            )
    else:
        print("\n[OK] All proofs have explicit #[kani::unwind(N)] or allowlist markers.")


def main() -> int:
    """Check that all Kani proofs are covered in verify-kani.sh."""
    verbose = "--verbose" in sys.argv

    project_root = get_project_root()

    source_proofs = find_source_proofs(project_root)
    script_proofs = find_script_proofs(project_root)

    # Find proofs in source but not in script
    missing_proofs = source_proofs - script_proofs

    # Find proofs in script but not in source (stale references)
    extra_proofs = script_proofs - source_proofs

    has_errors = False

    if missing_proofs:
        has_errors = True
        print("ERROR: The following Kani proofs are NOT in verify-kani.sh:")
        for proof in sorted(missing_proofs):
            print(f"  - {proof}")
        print("\nAdd them to one of the TIER*_PROOFS arrays in scripts/verify-kani.sh")

    if extra_proofs:
        # This is a warning, not an error (could be commented out proofs)
        print("\nWARNING: The following proofs are in verify-kani.sh but not in source:")
        for proof in sorted(extra_proofs):
            print(f"  - {proof}")

    if not has_errors:
        print(f"[OK] All {len(source_proofs)} Kani proofs are covered in verify-kani.sh")

    # Advisory check for unwind attributes (runs regardless of coverage result)
    check_unwind_attributes(project_root, verbose=verbose)

    return 1 if has_errors else 0


if __name__ == "__main__":
    sys.exit(main())
