#!/usr/bin/env python3
"""Tests for scripts/hooks/check-rust-semantic-claims.py."""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CHECK_SOURCE = REPO_ROOT / "scripts" / "hooks" / "check-rust-semantic-claims.py"


def _setup_repo(tmp_path: Path) -> Path:
    """Create a temporary repo with the semantic-claim checker copied in."""
    repo = tmp_path / "repo"
    script_path = repo / "scripts" / "hooks" / CHECK_SOURCE.name
    script_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(CHECK_SOURCE, script_path)
    return repo


def _write_rust(repo: Path, rel_path: str, content: str) -> Path:
    """Write a Rust source fixture into the temporary repo."""
    path = repo / rel_path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    return path


def _run_checker(repo: Path) -> subprocess.CompletedProcess[str]:
    """Run the copied semantic-claim checker in its temporary repo."""
    return subprocess.run(
        ["python3", "scripts/hooks/check-rust-semantic-claims.py"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )


def test_flags_diverges_test_name_with_empty_divergence_assertion(
    tmp_path: Path,
) -> None:
    """A positive-divergence test name must not assert no divergence."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "tests/peer_drop.rs",
        "#[test]\n"
        "fn stale_echo_diverges_across_survivors() {\n"
        "    let divergences: Vec<u8> = Vec::new();\n"
        "    assert!(divergences.is_empty());\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "says divergence is expected" in combined
    assert "stale_echo_diverges_across_survivors" in combined


def test_accepts_convergence_test_name_with_empty_divergence_assertion(
    tmp_path: Path,
) -> None:
    """A convergence/no-divergence test name may assert no divergence."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "tests/peer_drop.rs",
        "#[test]\n"
        "fn stale_echo_converges_across_survivors() {\n"
        "    let divergences: Vec<u8> = Vec::new();\n"
        "    assert!(divergences.is_empty());\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 0, combined


def test_accepts_diverges_test_name_with_positive_divergence_assertion(
    tmp_path: Path,
) -> None:
    """A positive-divergence test name may assert a non-empty divergence set."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "tests/peer_drop.rs",
        "#[test]\n"
        "fn double_failure_diverges_across_survivors() {\n"
        "    let divergences = vec![1_u8];\n"
        "    assert!(!divergences.is_empty());\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 0, combined


def test_accepts_mixed_oracle_with_no_divergence_and_positive_cases(
    tmp_path: Path,
) -> None:
    """A non-vacuity oracle may assert clean and divergent subcases together."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "tests/network/in_process_chaos.rs",
        "#[test]\n"
        "fn npeer_oracle_detects_divergence() {\n"
        "    let divergence = None;\n"
        "    assert_eq!(divergence, None);\n"
        "    let divergence = Some((2, 2));\n"
        "    assert_eq!(divergence, Some((2, 2)));\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 0, combined


def test_accepts_negative_divergence_name_with_no_divergence_assertion(
    tmp_path: Path,
) -> None:
    """A name that explicitly says without/no divergence is not a positive claim."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "src/sessions/p2p_spectator_session.rs",
        "#[test]\n"
        "fn pending_failover_without_divergence() {\n"
        "    assert!(session.spectator_divergence.is_none());\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 0, combined


def test_flags_divergence_noun_test_name_with_absent_divergence_assertion(
    tmp_path: Path,
) -> None:
    """A noun-form divergence test name must not assert no divergence."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "src/sessions/p2p_spectator_session.rs",
        "#[test]\n"
        "fn spectator_nonoverlapping_region_divergence() {\n"
        "    assert!(session.spectator_divergence.is_none());\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "spectator_nonoverlapping_region_divergence" in combined


def test_ignores_positive_divergence_assertions_inside_non_code(
    tmp_path: Path,
) -> None:
    """Comments and strings must not satisfy positive-divergence claims."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "tests/peer_drop.rs",
        "#[test]\n"
        "fn stale_echo_diverges_across_survivors() {\n"
        "    let divergences: Vec<u8> = Vec::new();\n"
        "    let _ = \"assert!(!divergences.is_empty());\";\n"
        "    // assert!(!divergences.is_empty());\n"
        "    assert!(divergences.is_empty());\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "says divergence is expected" in combined
    assert "stale_echo_diverges_across_survivors" in combined


def test_flags_inclusive_range_assertion_missing_upper_bound_in_docs(
    tmp_path: Path,
) -> None:
    """Rustdoc contracts must document both bounds from inclusive assertions."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "tests/common/channel_socket.rs",
        "/// Creates a mesh.\n"
        "///\n"
        "/// # Arguments\n"
        "///\n"
        "/// * `n` - The number of peers in the mesh (must be `>= 2`).\n"
        "///\n"
        "/// # Panics\n"
        "///\n"
        "/// Panics if `n < 2`.\n"
        "pub fn create_channel_mesh(n: usize) {\n"
        "    assert!((2..=1000).contains(&n));\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "asserts `n` is in `2..=1000`" in combined
    assert "# Panics section omits bound(s): 1000" in combined


def test_ignores_range_assertions_inside_non_code(tmp_path: Path) -> None:
    """Comments and strings must not trigger range-contract checks."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "tests/common/channel_socket.rs",
        "/// Creates a mesh.\n"
        "pub fn create_channel_mesh(n: usize) {\n"
        "    let _ = \"assert!((2..=1000).contains(&n));\";\n"
        "    // assert!((2..=1000).contains(&n));\n"
        "    let _ = n;\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 0, combined


def test_flags_delegated_channel_mesh_panic_docs_missing_upper_bound(
    tmp_path: Path,
) -> None:
    """Delegating helpers must document inherited mesh-size panic bounds."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "tests/common/channel_socket.rs",
        "/// Creates a mesh.\n"
        "///\n"
        "/// # Panics\n"
        "///\n"
        "/// Panics if fewer than 2 configs are provided.\n"
        "pub fn create_chaos_channel_mesh(configs: &[u8]) {\n"
        "    let _ = create_channel_mesh(configs.len());\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "delegates to `create_channel_mesh`" in combined
    assert "omits bound(s): 1000" in combined


def test_flags_delegated_channel_mesh_panic_docs_missing_panics_section(
    tmp_path: Path,
) -> None:
    """Delegating helpers must have a # Panics section for inherited bounds."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "tests/common/channel_socket.rs",
        "/// Creates a mesh.\n"
        "pub fn create_chaos_channel_mesh(configs: &[u8]) {\n"
        "    let _ = create_channel_mesh(configs.len());\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "delegates to `create_channel_mesh`" in combined
    assert "has no # Panics section" in combined


def test_ignores_delegated_channel_mesh_calls_inside_non_code(
    tmp_path: Path,
) -> None:
    """Comments and strings must not trigger delegated mesh-contract checks."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "tests/common/channel_socket.rs",
        "/// Creates a mesh.\n"
        "pub fn create_chaos_channel_mesh(configs: &[u8]) {\n"
        "    let _ = \"create_channel_mesh(configs.len())\";\n"
        "    // create_channel_mesh(configs.len());\n"
        "    let _ = configs;\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 0, combined


def test_accepts_inclusive_range_assertion_with_complete_docs(
    tmp_path: Path,
) -> None:
    """Rustdoc contracts pass when argument and panic docs name both bounds."""
    repo = _setup_repo(tmp_path)
    _write_rust(
        repo,
        "tests/common/channel_socket.rs",
        "/// Creates a mesh.\n"
        "///\n"
        "/// # Arguments\n"
        "///\n"
        "/// * `n` - The number of peers in the mesh (must be in `2..=1000`).\n"
        "///\n"
        "/// # Panics\n"
        "///\n"
        "/// Panics if `n` is outside `2..=1000`.\n"
        "pub fn create_channel_mesh(n: usize) {\n"
        "    assert!((2..=1000).contains(&n));\n"
        "}\n",
    )

    result = _run_checker(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 0, combined
