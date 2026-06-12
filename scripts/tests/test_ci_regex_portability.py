#!/usr/bin/env python3
"""Regression tests for CI scripts affected by ERE portability issues."""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DOC_CLAIMS_SOURCE = REPO_ROOT / "scripts" / "ci" / "check-doc-claims.sh"
DERIVE_BOUNDS_SOURCE = REPO_ROOT / "scripts" / "ci" / "check-derive-bounds.sh"


def _setup_repo_with_script(tmp_path: Path, script_source: Path) -> tuple[Path, Path]:
    """Create a temporary repo with one CI script copied into scripts/ci/."""
    repo = tmp_path / "repo"
    script_path = repo / "scripts" / "ci" / script_source.name
    script_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(script_source, script_path)
    return repo, script_path


def _write_rust(repo: Path, rel_path: str, content: str) -> Path:
    """Write a Rust source fixture into the temporary repo."""
    path = repo / rel_path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    return path


def _run_script(script_path: Path) -> subprocess.CompletedProcess[str]:
    """Run a copied CI shell script in its temporary repo."""
    return subprocess.run(
        ["bash", str(script_path)],
        cwd=script_path.parent.parent.parent,
        capture_output=True,
        text=True,
        check=False,
    )


def test_doc_claims_accepts_downcast_method_syntax(tmp_path: Path) -> None:
    """Doc claims with .downcast::<T>() are recognized as backed by infra."""
    repo, script_path = _setup_repo_with_script(tmp_path, DOC_CLAIMS_SOURCE)
    _write_rust(
        repo,
        "src/downcast_ok.rs",
        "/// This helper supports downcast-based extraction.\n"
        "pub fn extract(value: Box<dyn std::any::Any>) {\n"
        "    let _ = value.downcast::<u32>();\n"
        "}\n",
    )

    result = _run_script(script_path)

    combined = result.stdout + result.stderr
    assert result.returncode == 0, combined


def test_doc_claims_flags_unbacked_downcast_docs(tmp_path: Path) -> None:
    """Doc claims mentioning downcast without infrastructure are rejected."""
    repo, script_path = _setup_repo_with_script(tmp_path, DOC_CLAIMS_SOURCE)
    _write_rust(
        repo,
        "src/downcast_missing.rs",
        "/// This type allows callers to downcast to concrete types.\n"
        "pub struct NoDowncastInfra;\n",
    )

    result = _run_script(script_path)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert 'Doc comments mention "downcast"' in combined


def test_doc_claims_flags_bounded_decode_docs_pointing_to_unbounded_decode(
    tmp_path: Path,
) -> None:
    """Bounded decode docs must not recommend unbounded decode for consumed length."""
    repo, script_path = _setup_repo_with_script(tmp_path, DOC_CLAIMS_SOURCE)
    _write_rust(
        repo,
        "src/network/codec.rs",
        "/// Decodes a value with a fixed allocation limit.\n"
        "///\n"
        "/// Returns an error when bincode fails to decode (trailing bytes are not\n"
        "/// rejected here -- use [`decode`] if you need the consumed length).\n"
        "pub(crate) fn decode_bounded<T>(bytes: &[u8]) -> Result<T, ()> {\n"
        "    unimplemented!()\n"
        "}\n",
    )

    result = _run_script(script_path)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "decode_bounded docs" in combined
    assert "decode_bounded_with_consumed" in combined


def test_doc_claims_flags_bounded_decode_docs_pointing_to_qualified_unbounded_decode(
    tmp_path: Path,
) -> None:
    """Qualified generic decode links are still unsafe bounded-decode guidance."""
    repo, script_path = _setup_repo_with_script(tmp_path, DOC_CLAIMS_SOURCE)
    _write_rust(
        repo,
        "src/network/codec.rs",
        "/// Decodes a value with a fixed allocation limit.\n"
        "///\n"
        "/// Returns an error when bincode fails to decode (trailing bytes are not\n"
        "/// rejected here -- use [`codec::decode`] if you need the consumed length).\n"
        "pub(crate) fn decode_bounded<T>(bytes: &[u8]) -> Result<T, ()> {\n"
        "    unimplemented!()\n"
        "}\n",
    )

    result = _run_script(script_path)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "decode_bounded docs" in combined
    assert "decode_bounded_with_consumed" in combined


def test_doc_claims_flags_bounded_decode_docs_pointing_to_crate_qualified_unbounded_decode(
    tmp_path: Path,
) -> None:
    """Crate-qualified generic decode links are still unsafe bounded-decode guidance."""
    repo, script_path = _setup_repo_with_script(tmp_path, DOC_CLAIMS_SOURCE)
    _write_rust(
        repo,
        "src/network/codec.rs",
        "/// Decodes a value with a fixed allocation limit.\n"
        "///\n"
        "/// Returns an error when bincode fails to decode (trailing bytes are not\n"
        "/// rejected here -- use [`crate::network::codec::decode`] if you need the\n"
        "/// consumed length).\n"
        "pub(crate) fn decode_bounded<T>(bytes: &[u8]) -> Result<T, ()> {\n"
        "    unimplemented!()\n"
        "}\n",
    )

    result = _run_script(script_path)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "decode_bounded docs" in combined
    assert "decode_bounded_with_consumed" in combined


def test_doc_claims_accepts_bounded_decode_docs_with_bounded_consumed_helper(
    tmp_path: Path,
) -> None:
    """Bounded decode docs may reference the bounded consumed-length helper."""
    repo, script_path = _setup_repo_with_script(tmp_path, DOC_CLAIMS_SOURCE)
    _write_rust(
        repo,
        "src/network/codec.rs",
        "/// Decodes a value with a fixed allocation limit.\n"
        "///\n"
        "/// Returns an error when bincode fails to decode (trailing bytes are not\n"
        "/// rejected here -- use [`decode_bounded_with_consumed`] if you need the\n"
        "/// consumed length).\n"
        "pub(crate) fn decode_bounded<T>(bytes: &[u8]) -> Result<T, ()> {\n"
        "    unimplemented!()\n"
        "}\n"
        "pub(crate) fn decode_bounded_with_consumed<T>(\n"
        "    bytes: &[u8],\n"
        ") -> Result<(T, usize), ()> {\n"
        "    unimplemented!()\n"
        "}\n",
    )

    result = _run_script(script_path)

    combined = result.stdout + result.stderr
    assert result.returncode == 0, combined


def test_doc_claims_flags_stale_codec_decode_reference_in_rustdoc(
    tmp_path: Path,
) -> None:
    """Rustdoc references to codec::decode_* helpers must name real helpers."""
    repo, script_path = _setup_repo_with_script(tmp_path, DOC_CLAIMS_SOURCE)
    _write_rust(
        repo,
        "src/network/codec.rs",
        "pub(crate) fn decode_bounded<T>(bytes: &[u8]) -> Result<T, ()> {\n"
        "    unimplemented!()\n"
        "}\n",
    )
    _write_rust(
        repo,
        "src/session.rs",
        "/// Delegates to codec::decode_bounded_with_consumption for bounded decode.\n"
        "pub fn decode_state() {}\n",
    )

    result = _run_script(script_path)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "unknown codec decode helper" in combined
    assert "decode_bounded_with_consumption" in combined


def test_doc_claims_flags_stale_codec_decode_reference_in_alloc_bound_comment(
    tmp_path: Path,
) -> None:
    """alloc-bound comments must not cite nonexistent codec::decode_* helpers."""
    repo, script_path = _setup_repo_with_script(tmp_path, DOC_CLAIMS_SOURCE)
    _write_rust(
        repo,
        "src/network/codec.rs",
        "pub(crate) fn decode_bounded<T>(bytes: &[u8]) -> Result<T, ()> {\n"
        "    unimplemented!()\n"
        "}\n",
    )
    _write_rust(
        repo,
        "src/session.rs",
        "pub fn decode_state() {\n"
        "    // alloc-bound: delegated to codec::decode_bounded_with_consumption.\n"
        "    let _bytes = Vec::<u8>::new();\n"
        "}\n",
    )

    result = _run_script(script_path)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "unknown codec decode helper" in combined
    assert "alloc-bound" in combined


def test_doc_claims_avoids_bash_four_lowercase_expansion() -> None:
    """Doc-claims script must stay compatible with macOS /bin/bash 3.2."""
    content = DOC_CLAIMS_SOURCE.read_text(encoding="utf-8")

    assert ",," not in content
    assert "tr '[:upper:]' '[:lower:]'" in content


def test_derive_bounds_flags_eq_without_eq_bound(tmp_path: Path) -> None:
    """Public generic derives Eq without Eq bound should fail."""
    repo, script_path = _setup_repo_with_script(tmp_path, DERIVE_BOUNDS_SOURCE)
    _write_rust(
        repo,
        "src/derive_bad.rs",
        "#[derive(Clone, Debug, PartialEq, Eq)]\n"
        "pub struct Wrapper<T> {\n"
        "    value: T,\n"
        "}\n",
    )

    result = _run_script(script_path)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "derives Eq" in combined


def test_derive_bounds_accepts_explicit_eq_bound(tmp_path: Path) -> None:
    """Public generic derives Eq with explicit Eq bound should pass."""
    repo, script_path = _setup_repo_with_script(tmp_path, DERIVE_BOUNDS_SOURCE)
    _write_rust(
        repo,
        "src/derive_ok.rs",
        "#[derive(Clone, Debug, PartialEq, Eq)]\n"
        "pub struct Wrapper<T: Eq> {\n"
        "    value: T,\n"
        "}\n",
    )

    result = _run_script(script_path)

    combined = result.stdout + result.stderr
    assert result.returncode == 0, combined
