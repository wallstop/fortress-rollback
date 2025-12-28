# GitHub Actions Best Practices

> **A guide to writing correct, maintainable GitHub Actions workflows with proper linting and shell scripting.**

## Overview

GitHub Actions workflows combine YAML configuration with embedded shell scripts. Both layers require validation to catch errors before they reach CI. This guide covers mandatory linting, common pitfalls, and best practices.

---

## Mandatory Linting

### actionlint

**Always run `actionlint` after modifying any workflow file:**

```bash
# Lint all workflow files
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
| SC2129 | Multiple individual redirects | Use `{ cmd1; cmd2; } >> file` |
| SC2086 | Unquoted variables | Quote: `"$VAR"` |
| SC2016 | Variables in single quotes | Use double quotes |
| SC2028 | `echo` with escapes | Use `printf` |
| SC2035 | Unsafe glob patterns | Prefix with `./` |
| SC2155 | `export x=$(cmd)` | Separate: `x=$(cmd); export x` |
| Invalid expression | `${{ }}` syntax error | Check contexts, use `toJSON()` for debugging |
| Unknown runner | Invalid `runs-on` | Use `ubuntu-latest`, `macos-latest`, `windows-latest` |
| Missing permissions | GITHUB_TOKEN scope | Add `permissions:` block |

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

## Related Documentation

- [Cross-Platform CI/CD](cross-platform-ci-cd.md) — Multi-platform build strategies
- [Defensive Programming](defensive-programming.md) — Error handling principles
- [Testing Guide](rust-testing-guide.md) — CI test organization

---

*License: MIT OR Apache-2.0*
