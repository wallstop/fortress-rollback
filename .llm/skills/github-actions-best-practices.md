# GitHub Actions Best Practices

> **A guide to writing correct, maintainable GitHub Actions workflows with proper linting and shell scripting.**

## Overview

GitHub Actions workflows combine YAML configuration with embedded shell scripts. Both layers require validation to catch errors before they reach CI. This guide covers mandatory linting, common pitfalls, and best practices.

---

## Mandatory Linting

### Pre-commit Integration

Pre-commit hooks now include `actionlint` validation. This provides an additional safety net before commits:

```bash
# Install pre-commit hooks (one-time setup)
pre-commit install

# Manual run on all workflow files
pre-commit run actionlint --all-files
```

The pre-commit hook catches workflow issues before they reach CI, saving time and preventing broken builds.

### actionlint (MUST Run After EVERY Change)

**⚠️ CRITICAL: Run `actionlint` after EVERY modification to ANY workflow file — no exceptions.**

Workflow linting catches errors that are impossible to detect without running CI. A single missing quote or syntax error can break the entire workflow. **Always lint before committing.**

```bash
# Lint all workflow files (run this!)
actionlint

# Lint a specific workflow
actionlint .github/workflows/ci-security.yml

# Show all available checks
actionlint -list
```

`actionlint` validates:

- YAML syntax and structure
- GitHub Actions expressions (`${{ }}`)
- Step references and job dependencies
- Available contexts (github, env, secrets, etc.)
- Shell script syntax via shellcheck integration

### Local Installation

The dev container includes `actionlint`. If installing manually:

```bash
# Via Go
go install github.com/rhysd/actionlint/cmd/actionlint@latest

# Via Homebrew (macOS)
brew install actionlint

# Via npm (alternative)
npm install -g actionlint
```

---

## Shellcheck Best Practices for GitHub Actions

GitHub Actions `run:` blocks are shell scripts and must follow shellcheck rules. Common issues:

### SC2129: Use Grouped Redirects

**Problem:** Multiple commands appending to the same file with individual redirects is inefficient and harder to read.

| ❌ Incorrect | ✅ Correct |
|-------------|-----------|
| `echo "A" >> file`<br>`echo "B" >> file`<br>`echo "C" >> file` | `{ echo "A"; echo "B"; echo "C"; } >> file` |

```yaml
# ❌ WRONG: Individual redirects (SC2129)
- name: Write outputs
  run: |
    echo "foo=bar" >> "$GITHUB_OUTPUT"
    echo "baz=qux" >> "$GITHUB_OUTPUT"
    echo "num=42" >> "$GITHUB_OUTPUT"

# ✅ CORRECT: Grouped redirects
- name: Write outputs
  run: |
    {
      echo "foo=bar"
      echo "baz=qux"
      echo "num=42"
    } >> "$GITHUB_OUTPUT"
```

### SC2086: Quote Variables to Prevent Globbing

**Problem:** Unquoted variables undergo word splitting and glob expansion.

```yaml
# ❌ WRONG: Unquoted variable
- run: echo $GITHUB_OUTPUT

# ✅ CORRECT: Quoted variable
- run: echo "$GITHUB_OUTPUT"
```

### SC2016: Expressions Don't Expand in Single Quotes

**Problem:** Single quotes prevent variable expansion.

```yaml
# ❌ WRONG: Variable won't expand
- run: echo '$HOME is here'

# ✅ CORRECT: Use double quotes for expansion
- run: echo "$HOME is here"
```

**When SC2016 is intentional — use suppression with comment:**

Sometimes you WANT single quotes to prevent expansion at definition time (e.g., shell functions that should expand at call time):

```yaml
# ✅ CORRECT: Intentional suppression with explanation
- name: Configure git credential helper
  run: |
    # shellcheck disable=SC2016  # Intentional: function body expands at call time, not definition
    git config --global credential.helper '!f() { echo "password=${GITHUB_TOKEN}"; }; f'
```

**Why this works:** The git credential helper is a shell function invoked by git. The `${GITHUB_TOKEN}` must expand when git calls the function, not when we define it. Single quotes preserve the literal `${GITHUB_TOKEN}` text.

**Always include the comment explaining WHY** — future maintainers need to understand this is intentional, not a bug.

### SC2028: echo Escape Sequences

**Problem:** `echo` doesn't interpret escape sequences portably.

```yaml
# ❌ WRONG: Escape may not work
- run: echo "Line1\nLine2"

# ✅ CORRECT: Use printf or $'...'
- run: printf "Line1\nLine2\n"

# ✅ ALSO CORRECT: ANSI-C quoting
- run: echo $'Line1\nLine2'
```

### SC2035: Use ./ for Glob Patterns

**Problem:** Glob patterns starting with `-` can be interpreted as options.

```yaml
# ❌ WRONG: Could match files starting with -
- run: rm *.tmp

# ✅ CORRECT: Explicit path prefix
- run: rm ./*.tmp
```

### SC2155: Declare and Assign Separately

**Problem:** `export var=$(cmd)` masks the exit status of `cmd`.

```yaml
# ❌ WRONG: Exit status masked
- run: export VERSION=$(cargo pkgid | cut -d# -f2)

# ✅ CORRECT: Separate declaration and assignment
- run: |
    VERSION=$(cargo pkgid | cut -d# -f2)
    export VERSION
```

---

## Common Workflow Issues Reference

| Issue | Symptom | Fix |
|-------|---------|-----|
| SC2016 | Variables in single quotes | Use double quotes, or suppress if intentional (see below) |
| SC2028 | `echo` with escapes | Use `printf` |
| SC2035 | Unsafe glob patterns | Prefix with `./` |
| SC2086 | Unquoted variables | Quote: `"$VAR"` |
| SC2129 | Multiple individual redirects | Use `{ cmd1; cmd2; } >> file` |
| SC2155 | `export x=$(cmd)` | Separate: `x=$(cmd); export x` |
| Invalid expression | `${{ }}` syntax error | Check contexts, use `toJSON()` for debugging |
| Unknown runner | Invalid `runs-on` | Use `ubuntu-latest`, `macos-latest`, `windows-latest` |
| Missing permissions | GITHUB_TOKEN scope | Add `permissions:` block |

### Intentional Shellcheck Suppressions

When a shellcheck warning is triggered by intentional code, suppress it with a comment explaining WHY:

```bash
# shellcheck disable=SC2016  # Intentional: function body expands at call time, not definition
git config --global credential.helper '!f() { echo "password=${GITHUB_TOKEN}"; }; f'
```

**Rules for suppressions:**

1. **Always add a comment** explaining why the suppression is needed
2. **Be specific** — disable only the specific code, not entire files
3. **Document the intent** — future maintainers must understand it's not a bug

---

## GITHUB_OUTPUT and GITHUB_ENV

Modern GitHub Actions use file-based outputs instead of deprecated set-output commands.

### Setting Outputs

```yaml
# ✅ CORRECT: Modern file-based output
- name: Set output
  id: step_id
  run: |
    {
      echo "name=value"
      echo "multi_line<<EOF"
      echo "line 1"
      echo "line 2"
      echo "EOF"
    } >> "$GITHUB_OUTPUT"

# Reference later
- run: echo "${{ steps.step_id.outputs.name }}"
```

### Setting Environment Variables

```yaml
# ✅ CORRECT: Set for subsequent steps
- name: Set env
  run: |
    {
      echo "MY_VAR=value"
      echo "ANOTHER=other"
    } >> "$GITHUB_ENV"

- name: Use env
  run: echo "$MY_VAR"
```

---

## Workflow File Change Checklist

Before committing workflow changes:

- [ ] Run `actionlint` on modified workflow files
- [ ] Check for shellcheck warnings in `run:` blocks
- [ ] Verify all `${{ }}` expressions are valid
- [ ] Ensure `permissions:` block is minimal (least privilege)
- [ ] Test complex expressions with `toJSON()` first
- [ ] Verify `runs-on` uses valid runner labels
- [ ] Check that secrets are referenced correctly
- [ ] Validate matrix combinations make sense

### Quick Validation Command

```bash
# Run after any workflow change
actionlint .github/workflows/*.yml
```

---

## Workflow Structure Best Practices

### Use Explicit Permissions

```yaml
# ✅ CORRECT: Explicit minimal permissions
permissions:
  contents: read
  pull-requests: write

jobs:
  lint:
    runs-on: ubuntu-latest
    steps: ...
```

### Prefer Reusable Workflows

```yaml
# .github/workflows/reusable-test.yml
on:
  workflow_call:
    inputs:
      rust-version:
        required: true
        type: string

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ inputs.rust-version }}
```

### Use Concurrency to Cancel Outdated Runs

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

### Cache Dependencies

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: "ci"
```

---

## Debugging Workflows

### Enable Debug Logging

Set these secrets/variables for verbose output:

- `ACTIONS_STEP_DEBUG=true`
- `ACTIONS_RUNNER_DEBUG=true`

### Debug Expressions

```yaml
- name: Debug context
  run: |
    echo "github context:"
    echo '${{ toJSON(github) }}'
    echo "env context:"
    echo '${{ toJSON(env) }}'
```

### Local Testing with act

```bash
# Install act
brew install act

# Run workflow locally
act push -j lint

# With secrets
act push -s GITHUB_TOKEN="$(gh auth token)"
```

---

## Security Considerations

### Avoid Embedding Credentials in URLs

**Problem:** Embedding tokens directly in URLs risks exposure in logs, error messages, or shell traces.

```yaml
# ❌ DANGEROUS: Token embedded in URL string
- name: Clone wiki
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    # Token could be exposed in error messages or if set -x is enabled
    WIKI_URL="https://x-access-token:${GITHUB_TOKEN}@github.com/owner/repo.wiki.git"
    git clone "$WIKI_URL" wiki

# ✅ SECURE: Use git credential helper
- name: Clone wiki
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    # Configure credential helper to provide token from environment
    # The token is never embedded in the URL string
    git config --global credential.helper '!f() { echo "password=${GITHUB_TOKEN}"; }; f'
    git config --global credential.https://github.com.username "x-access-token"

    # Clone using plain URL (credentials provided by helper)
    git clone "https://github.com/owner/repo.wiki.git" wiki
```

**Why this matters:**

- Error messages may include the URL with embedded credentials
- If `set -x` (shell tracing) is accidentally enabled, the URL is printed
- Subprocess output may expose the URL
- GitHub masks secrets in logs, but other tools might not

### Avoid Command Injection

```yaml
# ❌ DANGEROUS: User input in command
- run: echo "${{ github.event.pull_request.title }}"

# ✅ SAFER: Use environment variable
- run: echo "$PR_TITLE"
  env:
    PR_TITLE: ${{ github.event.pull_request.title }}
```

### Pin Actions to SHA

```yaml
# ❌ RISKY: Tag can be moved
- uses: actions/checkout@v4

# ✅ SECURE: Pinned to specific commit (with comment for version)
- uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11 # v4.1.1
```

### Minimal Token Permissions

```yaml
permissions:
  contents: read  # Only what's needed
```

---

## Directory Conflicts in Workflows

### The Problem

A common and frustrating workflow failure occurs when a `git clone` operation targets a directory that already exists in the repository structure. This causes `git clone` to fail with:

```
fatal: destination path 'wiki' already exists and is not an empty directory.
```

**This class of error is particularly insidious because:**

- It passes all local linting checks (`actionlint`, shellcheck)
- It only fails at runtime in CI
- The error message doesn't clearly indicate it's a naming conflict
- It can be caused by adding a new directory to the repo that matches a clone target

### Common Mistake Examples

```yaml
# ❌ WRONG: 'wiki' directory might exist in repo
- run: git clone https://github.com/owner/repo.wiki.git wiki

# ❌ WRONG: 'docs' exists in almost every project
- run: git clone https://github.com/other/docs-repo.git docs

# ❌ WRONG: Common directory names
- run: |
    git clone https://github.com/owner/repo.git src
    git clone https://github.com/owner/tools.git build
    git clone https://github.com/owner/config.git config
```

### Correct Approaches

#### 1. Use Unique Prefixed Directory Names

Prefix clone directories with `_` to make them clearly temporary and avoid conflicts:

```yaml
# ✅ CORRECT: Prefixed temporary directories
- name: Clone wiki repository
  run: git clone https://github.com/owner/repo.wiki.git _wiki_clone

- name: Clone external dependency
  run: git clone https://github.com/other/repo.git _external_repo

- name: Clone multiple repos
  run: |
    git clone https://github.com/owner/repo.git _main_clone
    git clone https://github.com/owner/tools.git _tools_clone
```

#### 2. Use $RUNNER_TEMP for Complete Isolation

The `$RUNNER_TEMP` directory is guaranteed to be empty and isolated from the workspace:

```yaml
# ✅ CORRECT: Complete isolation from workspace
- name: Clone to temp directory
  run: |
    git clone https://github.com/owner/repo.wiki.git "$RUNNER_TEMP/wiki"

    # Work with the cloned repo
    cd "$RUNNER_TEMP/wiki"
    # ... do work ...

    # Copy results back if needed
    cp -r "$RUNNER_TEMP/wiki/output" ./results/
```

#### 3. Clean Before Clone (When Necessary)

If you must use a specific directory name, ensure it doesn't exist first:

```yaml
# ✅ CORRECT: Defensive cleanup before clone
- name: Clone wiki (with cleanup)
  run: |
    # Remove if exists (from previous run or repo structure)
    rm -rf _wiki_clone || true

    # Now safe to clone
    git clone https://github.com/owner/repo.wiki.git _wiki_clone
```

#### 4. Use Subdirectory of GITHUB_WORKSPACE

Create a dedicated subdirectory for cloned repos:

```yaml
# ✅ CORRECT: Dedicated clones directory
- name: Clone external repos
  run: |
    mkdir -p _clones
    git clone https://github.com/owner/wiki.git _clones/wiki
    git clone https://github.com/owner/tools.git _clones/tools
```

### Directory Naming Conventions

| ❌ Avoid | ✅ Use Instead | Why |
|----------|----------------|-----|
| `wiki` | `_wiki_clone` | `wiki/` folder common in repos |
| `docs` | `_docs_clone` | `docs/` exists everywhere |
| `src` | `_src_checkout` | `src/` is standard |
| `build` | `_build_temp` | `build/` used by many tools |
| `dist` | `_dist_temp` | `dist/` for distributions |
| `config` | `_config_clone` | `config/` very common |
| `scripts` | `_scripts_temp` | `scripts/` exists in most repos |
| `assets` | `_assets_clone` | `assets/` for resources |

### Defensive Workflow Pattern

Here's a complete defensive pattern for cloning external repositories:

```yaml
- name: Clone and sync wiki
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    # Always use unique directory name with underscore prefix
    CLONE_DIR="_wiki_sync"

    # Defensive cleanup
    rm -rf "$CLONE_DIR" || true

    # Configure git credentials securely
    git config --global credential.helper '!f() { echo "password=${GITHUB_TOKEN}"; }; f'
    git config --global credential.https://github.com.username "x-access-token"

    # Clone to safe directory
    git clone "https://github.com/${{ github.repository }}.wiki.git" "$CLONE_DIR"

    # Do work...
    cd "$CLONE_DIR"
    # ...

    # Cleanup when done
    cd ..
    rm -rf "$CLONE_DIR"
```

### Pre-Workflow Checklist for Clone Operations

Before adding `git clone` to any workflow:

- [ ] Check if the target directory exists in the repo (`ls -la`, `fd -t d "name"`)
- [ ] Use `_` prefix for all clone directories
- [ ] Consider using `$RUNNER_TEMP` for complete isolation
- [ ] Add defensive `rm -rf` cleanup before clone
- [ ] Verify the directory name doesn't match any gitignored directories
- [ ] Check `.gitignore` for directories that might be created at runtime

---

## Related Documentation

- [Cross-Platform CI/CD](cross-platform-ci-cd.md) — Multi-platform build strategies
- [Defensive Programming](defensive-programming.md) — Error handling principles
- [Testing Guide](rust-testing-guide.md) — CI test organization

---

*License: MIT OR Apache-2.0*
