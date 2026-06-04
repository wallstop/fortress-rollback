# `setup-rust-cache` composite action

Unified Rust build caching for CI jobs: best-effort [sccache] compiler caching
plus an `actions/cache` of the cargo registry and `target/` directory. It
replaces the ~25-line sccache configure/verify/clear block that was previously
copy-pasted into every heavy cargo job.

## What it does

1. Configures sccache (`mozilla-actions/sccache-action`), tolerating failure — a
   GitHub Actions Cache service outage must never fail the calling job.
2. Verifies sccache can actually compile (`scripts/ci/verify-sccache.sh`),
   cross-platform via `bash`.
3. Exports `RUSTC_WRAPPER`/`SCCACHE_*` to `$GITHUB_ENV` when sccache is verified
   working, so **every subsequent cargo step in the job** is accelerated with no
   per-step `env:` blocks. If sccache is unavailable the wrapper is cleared and
   cargo compiles directly. This is the approach recommended by
   `mozilla-actions/sccache-action`.
4. Caches the cargo registry/git and (by default) `target/`.

sccache is purely an accelerator — **build correctness never depends on it.**

## Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `cache-key` | yes | — | Prefix for the `actions/cache` key, e.g. `build`, `wasm`, `feature`, `semver`, `z3`. The full key is `<cache-key>-<runner.os>-cargo-<hash(Cargo.lock)>`. |
| `cache-paths` | no | `~/.cargo/registry`, `~/.cargo/git`, `target` | Newline-separated paths to cache. Override to add job-specific paths (e.g. `semver-checks/target` for the semver job). |

The action exposes no formal outputs; it communicates with the rest of the job
through the `$GITHUB_ENV` exports described above.

## Usage

```yaml
- name: Setup Rust cache
  uses: ./.github/actions/setup-rust-cache
  with:
    cache-key: wasm

# subsequent cargo steps are automatically sccache-accelerated:
- run: cargo check --target wasm32-unknown-unknown
```

With extra cache paths:

```yaml
- name: Setup Rust cache
  uses: ./.github/actions/setup-rust-cache
  with:
    cache-key: semver
    cache-paths: |
      ~/.cargo/registry
      ~/.cargo/git
      target
      semver-checks/target
```

## Notes

- Each `cache-key` prefix gets its own `actions/cache` namespace (jobs do not
  share the on-disk `target/` cache). Cross-job reuse of *compiled artifacts*
  happens at the compiler level through the shared sccache GHA backend, so each
  job's dependency compilations are reused without clobbering each other's
  `target/`.
- Cross-OS: the verify step runs under `bash`, which is available on the Linux,
  macOS, and Windows GitHub runners. The action is currently used only on
  `ubuntu-latest` jobs.

[sccache]: https://github.com/mozilla/sccache
