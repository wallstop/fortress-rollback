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


def _display_path(path: Path, project_root: Path) -> Path:
    """Convert an absolute path to a project-relative display path.

    Unlike the CWD-based ``_display_path`` in argv-receiving hook scripts,
    this variant takes an explicit *project_root* because the script discovers
    files via ``rglob()`` and always knows its own root.
    """
    try:
        return path.relative_to(project_root)
    except ValueError:
        return path


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
            print(f"{_display_path(rs_file, project_root)}:0: cannot read file: {e}", file=sys.stderr)

    return proofs


def find_script_proofs(project_root: Path) -> set[str]:
    """Find all proof names referenced in verify-kani.sh."""
    proofs = set()
    verify_script = project_root / "scripts" / "verify-kani.sh"

    if not verify_script.exists():
        print(f"{_display_path(verify_script, project_root)}:0: file not found", file=sys.stderr)
        return proofs

    try:
        content = verify_script.read_text(encoding="utf-8")

        # Look for proof names in tier arrays (proof_* or verify_*)
        # These are typically in arrays like: TIER1_PROOFS=("proof_foo" "proof_bar")
        proof_pattern = re.compile(r'"((?:proof|verify)_\w+)"')

        for match in proof_pattern.finditer(content):
            proofs.add(match.group(1))

    except (OSError, UnicodeDecodeError) as e:
        print(f"{_display_path(verify_script, project_root)}:0: cannot read file: {e}", file=sys.stderr)

    return proofs


def find_script_proof_tiers(project_root: Path) -> dict[str, int]:
    """Find all proof names and their tier assignments from verify-kani.sh.

    Returns a dict mapping proof name -> tier number (1, 2, or 3).
    """
    proof_tiers: dict[str, int] = {}
    verify_script = project_root / "scripts" / "verify-kani.sh"

    if not verify_script.exists():
        return proof_tiers

    try:
        content = verify_script.read_text(encoding="utf-8")

        # Parse each TIER*_PROOFS array
        for tier in (1, 2, 3):
            # Match TIER{N}_PROOFS=( ... ) spanning multiple lines
            pattern = re.compile(
                rf"TIER{tier}_PROOFS=\((.*?)\)",
                re.DOTALL,
            )
            match = pattern.search(content)
            if match:
                block = match.group(1)
                proof_pattern = re.compile(r'"((?:proof|verify)_\w+)"')
                for proof_match in proof_pattern.finditer(block):
                    proof_tiers[proof_match.group(1)] = tier

    except (OSError, UnicodeDecodeError) as e:
        print(f"{_display_path(verify_script, project_root)}:0: cannot read file: {e}", file=sys.stderr)

    return proof_tiers


def check_unwind_attributes(
    project_root: Path, verbose: bool = False
) -> bool:
    """Check that #[kani::proof] functions have #[kani::unwind(N)] attributes.

    For Tier 2 and Tier 3 proofs, missing unwind is an ERROR (returns True
    to indicate failure). These proofs are complex enough that the default
    --default-unwind 8 used in CI --quick mode is often insufficient,
    causing timeouts.

    For Tier 1 proofs, missing unwind remains advisory — they are fast and
    simple enough that the default unwind bound is usually sufficient.

    Returns True if any Tier 2/3 proofs are missing unwind (error condition).
    """
    src_dir = project_root / "src"

    if not src_dir.exists():
        return False

    proof_tiers = find_script_proof_tiers(project_root)

    kani_attr_pattern = re.compile(r"#\[kani::proof\]")
    fn_pattern = re.compile(r"fn\s+(\w+)")
    unwind_pattern = re.compile(r"#\[kani::unwind\(\d+\)\]")
    allowlist_pattern = re.compile(r"//\s*kani::no-unwind-needed")

    advisories: list[tuple[str, str]] = []
    errors: list[tuple[str, str, int]] = []

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
                    tier = proof_tiers.get(fn_name, 0)

                    if tier >= 2:
                        errors.append((fn_name, str(rel_path), tier))
                    else:
                        advisories.append((fn_name, str(rel_path)))

        except (OSError, UnicodeDecodeError) as e:
            print(f"{_display_path(rs_file, project_root)}:0: cannot read file: {e}", file=sys.stderr)

    has_errors = False

    if errors:
        has_errors = True
        print(
            f"\nERROR: {len(errors)} Tier 2/3 proof(s) missing required "
            f"#[kani::unwind(N)]:",
            file=sys.stderr,
        )
        for fn_name, file_path, tier in sorted(errors):
            print(
                f"  ERROR: Tier {tier} proof '{fn_name}' in file '{file_path}' "
                f"has no #[kani::unwind(N)]. Tier 2/3 proofs MUST have explicit "
                f"unwind bounds to prevent CI timeouts.",
                file=sys.stderr,
            )

    if advisories:
        if verbose:
            print(
                f"\n[Advisory] {len(advisories)} Tier 1 proof(s) without "
                f"explicit #[kani::unwind(N)]:"
            )
            for fn_name, file_path in sorted(advisories):
                print(
                    f"  WARNING: proof '{fn_name}' in file '{file_path}' has no explicit "
                    f"#[kani::unwind(N)]. CI uses --default-unwind 8; larger data "
                    f"structures may cause timeouts.",
                    file=sys.stderr,
                )
        else:
            print(
                f"\n[Advisory] {len(advisories)} Tier 1 proof(s) without explicit "
                f"#[kani::unwind(N)]. Run with --verbose for details."
            )
    elif not errors:
        print("\n[OK] All proofs have explicit #[kani::unwind(N)] or allowlist markers.")

    return has_errors


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
        print("ERROR: The following Kani proofs are NOT in verify-kani.sh:", file=sys.stderr)
        for proof in sorted(missing_proofs):
            print(f"  - {proof}", file=sys.stderr)
        print("\nAdd them to one of the TIER*_PROOFS arrays in scripts/verify-kani.sh", file=sys.stderr)

    if extra_proofs:
        # This is a warning, not an error (could be commented out proofs)
        print("\nWARNING: The following proofs are in verify-kani.sh but not in source:", file=sys.stderr)
        for proof in sorted(extra_proofs):
            print(f"  - {proof}", file=sys.stderr)

    if not has_errors:
        print(f"[OK] All {len(source_proofs)} Kani proofs are covered in verify-kani.sh")

    # Check for unwind attributes (enforced for Tier 2/3, advisory for Tier 1)
    unwind_errors = check_unwind_attributes(project_root, verbose=verbose)
    if unwind_errors:
        has_errors = True

    return 1 if has_errors else 0


if __name__ == "__main__":
    sys.exit(main())
