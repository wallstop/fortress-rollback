<!-- SYNC: This source doc syncs to wiki/Contributing.md. -->

<p align="center">
  <img src="assets/logo.svg" alt="Fortress Rollback" width="128">
</p>

# Contribution Guidelines

First and foremost: Thank you for showing interest in contributing to Fortress Rollback (a fork of GGRS)! Make sure to read the [Code of Conduct](./code-of-conduct.md).
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
pre-commit run --hook-stage manual check-llm-skills --all-files
pre-commit run --hook-stage manual check-shell-portability --all-files
pre-commit run --hook-stage manual sync-version-check --all-files
pre-commit run --hook-stage manual check-doc-claims --all-files
pre-commit run --hook-stage manual check-derive-bounds --all-files

# Run a specific hook
pre-commit run markdownlint --all-files

# Run the link checker script directly
./scripts/docs/check-links.sh --verbose

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

### Bypassing Hooks (Emergencies Only)

```bash
git commit --no-verify -m "emergency fix"
```

Note: CI will still run these checks on pull requests
