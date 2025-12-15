<p align="center">
  <img src="../assets/logo.svg" alt="Fortress Rollback" width="128">
</p>

# Contribution Guidelines

First and foremost: Thank you for showing interest in contributing to Fortress Rollback (a fork of GGRS)! Make sure to read the [Code of Conduct](./code-of-conduct.md).
If you have a cool example or showcase of Fortress Rollback in use, let me know so your project can be highlighted!

## Create an issue

- [Bug report](https://github.com/wallstop/fortress-rollback/issues/new?assignees=&labels=bug&template=bug_report.md&title=)
- [Feature request](https://github.com/wallstop/fortress-rollback/issues/new?assignees=&labels=enhancement&template=feature_request.md&title=)

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

## Pre-commit Hooks

This project uses [pre-commit](https://pre-commit.com/) to ensure code quality before commits.

### Setup

```bash
# Install pre-commit (requires Python)
pip install pre-commit

# Install the git hooks
pre-commit install
```

### What's Checked

The pre-commit hooks validate:

- **Code formatting**: `cargo fmt` for Rust files
- **Linting**: `cargo clippy` for Rust code quality
- **Markdown formatting**: `markdownlint` for consistent documentation
- **Link validation**: Local file references and markdown links
- **Rustdoc links**: Intra-doc link resolution
- **General hygiene**: Trailing whitespace, YAML/TOML syntax, merge conflicts

### Running Manually

```bash
# Run all hooks on all files
pre-commit run --all-files

# Run a specific hook
pre-commit run markdownlint --all-files
pre-commit run check-links --all-files

# Run the link checker script directly
./scripts/check-links.sh --verbose

# Verify markdown code samples compile
./scripts/verify-markdown-code.sh

# With verbose output for debugging
./scripts/verify-markdown-code.sh --verbose

# Check a specific file
./scripts/verify-markdown-code.sh docs/user-guide.md
```

### Bypassing Hooks (Emergencies Only)

```bash
git commit --no-verify -m "emergency fix"
```

Note: CI will still run these checks on pull requests
