<!-- SYNC: This wiki page is generated from docs/contributing.md. Edit docs source. -->

<p align="center">
  <img src="assets/logo.svg" alt="Fortress Rollback" width="128">
</p>

# Contribution Guidelines

First and foremost: Thank you for showing interest in contributing to Fortress Rollback (a fork of GGRS)! Make sure to read the [Code of Conduct](Code-of-Conduct).
If you have a cool example or showcase of Fortress Rollback in use, let me know so your project can be highlighted!

## Create an issue

Visit [GitHub Issues](https://github.com/wallstop/fortress-rollback/issues) to:

- Report a bug
- Request a feature

## Contribute to Fortress Rollback

Please send a [GitHub Pull Request](https://github.com/wallstop/fortress-rollback/pull/new/main) with a clear list of what you've done
(read more about [pull requests](http://help.github.com/pull-requests/)). When you send a pull request,
it would be great if you wrote unit- or integration tests for your changes. Please format your code via `cargo fmt` and
make sure all of your commits are atomic (one feature per commit).

Always write a clear log message for your commits. One-line messages are fine for small changes, but bigger changes should look like this:

```shell
git commit -m "prefix: brief summary of the commit

A paragraph describing what changed and its impact."
```

With the following prefixes commonly used:

- `feat`: for new features
- `fix`: for fixing a bug
- `doc`: for adding/changing documentation
- `test`: for adding/changing tests
- `chore`: for any minor code cleanups

More about the [GitHub flow](https://guides.github.com/introduction/flow/).
More about the [Conventional Commits Specification](https://www.conventionalcommits.org/en/v1.0.0/)

## Local Hooks

This project uses [pre-commit](https://pre-commit.com/) for fast local feedback
before commits and pushes. CI runs exhaustive Rust, documentation, and
feature-matrix checks; developers can run those checks manually when needed.

### Setup

```bash
# Install pre-commit (requires Python)
pip install pre-commit

# Install the git hooks managed by .pre-commit-config.yaml
pre-commit install --hook-type pre-commit --hook-type pre-push
```

### What's Checked

The pre-commit hook is intentionally fast (<10 seconds) and file-scoped. It
validates:

- **Code formatting**: `rustfmt` for changed Rust files
- **Markdown formatting**: `markdownlint` for consistent documentation
- **Python types**: strict mypy for every production module under `scripts/`
- **General hygiene**: Trailing whitespace, YAML/TOML syntax, merge conflicts

Slow full-repository checks such as `cargo clippy`, `cargo doc`, link
validation, and `cargo hack` are manual/CI checks rather than blocking every
commit or push.

### Running Manually

```bash
# Run fast pre-commit hooks on current changes
pre-commit run

# Run manual full-repository hooks
pre-commit run --hook-stage manual cargo-clippy --all-files
pre-commit run --hook-stage manual rustdoc-links --all-files
pre-commit run --hook-stage manual check-links --all-files
pre-commit run --hook-stage manual cargo-hack-check --all-files
pre-commit run --hook-stage manual sync-wiki --all-files
pre-commit run --hook-stage manual check-agent-skills --all-files
pre-commit run --hook-stage manual check-shell-portability --all-files
pre-commit run --hook-stage manual sync-version-check --all-files
pre-commit run --hook-stage manual check-doc-claims --all-files
pre-commit run --hook-stage manual check-derive-bounds --all-files

# Run a specific hook
pre-commit run markdownlint --all-files

# Type-check all production Python at the supported Python 3.10 floor
mypy --config-file mypy.ini

# Run the link checker script directly
python3 scripts/docs/check-links.py --verbose

# Verify markdown code samples compile
./scripts/docs/verify-markdown-code.sh

# With verbose output for debugging
./scripts/docs/verify-markdown-code.sh --verbose

# Check a specific file
./scripts/docs/verify-markdown-code.sh docs/user-guide.md

# Check for invalid rustdoc-style code fence attributes in markdown
./scripts/docs/check-code-fence-syntax.sh

# Check a specific directory
./scripts/docs/check-code-fence-syntax.sh docs/
```

### Python Type Checker

Strict mypy 2.3.1 checks production Python under `scripts/`. Ruff and pytest continue to cover
`scripts/tests/` without requiring test annotations.
The configuration targets Python 3.10, so code accepted by the gate remains compatible with both
Python 3.10 and 3.11; keep the repository's `tomllib`/`tomli` fallback when reading TOML.

We selected the checker from a strict-mode comparison over the same 58 production files. Mypy
reported 34 diagnostics across 15 files after import-path normalization and reached zero with
narrow boundary annotations. Pyright 1.1.411 reported 495 strict errors; its warm scan took about
4.2 seconds here versus 0.2–0.4 seconds for mypy. Pyright's larger migration and second runtime did
not provide distinct enough signal to justify maintaining both tools.

### Bypassing Hooks (Emergencies Only)

```bash
git commit --no-verify -m "emergency fix"
```

Note: CI will still run these checks on pull requests

## Release automation

Releases use two manually dispatched GitHub Actions workflows:

1. Run **Release - Prepare PR** from the default branch and choose `patch`,
   `minor`, or `major`. The workflow rotates the Unreleased changelog notes,
   synchronizes version references, validates the package, and opens a
   `release/vX.Y.Z` pull request.
2. Review and merge that pull request only after its normal CI and reviews are
   green.
3. Run **Release - Publish Crate** from the default branch with the exact
   `X.Y.Z` version. It packages and checksums the crate, publishes it to
   crates.io, creates the matching tag and GitHub release, and attaches the
   `.crate` plus its SHA-256 file.

The preparation workflow requires a repository GitHub App whose installation
can write repository contents and pull requests. Store its App ID as
`RELEASE_APP_ID` and its private key as `RELEASE_APP_PRIVATE_KEY`. Use a GitHub
App token because pull requests created with the default
`GITHUB_TOKEN` do not trigger the normal pull-request CI workflows. The publish
workflow continues to require `CRATES_IO_TOKEN`.

Both workflows fail if dispatched from a non-default branch. Publishing can
safely rerun after a partial failure: the workflow skips an existing crates.io
version only when its registry checksum exactly matches the newly packaged
crate, and accepts an existing tag only when it points to the selected source
commit.
