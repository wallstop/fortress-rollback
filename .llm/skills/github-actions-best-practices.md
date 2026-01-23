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

**CRITICAL: Run `actionlint` after EVERY modification to ANY workflow file — no exceptions.**

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
```

---

## Shellcheck Best Practices for GitHub Actions

GitHub Actions `run:` blocks are shell scripts and must follow shellcheck rules. When `actionlint` runs, it invokes shellcheck on your `run:` blocks, which can produce confusing errors because shellcheck doesn't understand GitHub's `${{ }}` template syntax.

### SC2193: Template Expressions in Conditionals

**Problem:** When you use `${{ }}` expressions directly in bash conditionals with glob/pattern matching, shellcheck sees them as literal strings and warns that the comparison can never match.

```yaml
# ❌ WRONG: SC2193 - Arguments can never be equal (glob pattern matching)
- name: Check test filter
  run: |
    if [[ "${{ matrix.test_filter }}" == MULTI:* ]]; then
      echo "Multi-test mode"
    fi
```

**Why this fails:** Shellcheck analyzes the bash code before GitHub Actions substitutes the `${{ }}` expressions. It sees `"${{ matrix.test_filter }}"` as a literal string (not a variable), and since a literal string can only equal itself, glob patterns like `== MULTI:*` appear to never match.

**When SC2193 triggers:**

| Comparison Type | Triggers SC2193? | Example |
|-----------------|------------------|---------|
| Glob/pattern matching | **Yes** | `== MULTI:*`, `== prefix*`, `!= *suffix` |
| Simple equality | Sometimes | `== "exact-string"` may or may not trigger |
| Numeric comparison | Rarely | `-eq`, `-gt`, etc. |

> **Note:** The warning behavior can vary depending on the shellcheck version and the exact expression. Simple equality comparisons like `== "ubuntu-latest"` may not always trigger SC2193, but the variable pattern is still recommended as a defensive practice.

**Solution:** Assign `${{ }}` expressions to bash variables first, then use those variables in conditionals:

```yaml
# ✅ CORRECT: Assign to variable, then use in conditional
- name: Check test filter
  run: |
    test_filter="${{ matrix.test_filter }}"
    if [[ "$test_filter" == MULTI:* ]]; then
      echo "Multi-test mode"
    fi
```

**When this pattern is required vs recommended:**

| Scenario | Required or Recommended? |
|----------|-------------------------|
| Glob/pattern matching (`== pattern*`, `!= *suffix`) | **Required** — SC2193 will trigger |
| Equality with `==` or `!=` against exact strings | **Recommended** — defensive practice, may not always trigger SC2193 |
| Multiple comparisons in the same block | **Recommended** — cleaner code, easier to maintain |

**Example with exact string comparisons:**

```yaml
# ⚠️ May work, but variable pattern is preferred for consistency
- run: |
    if [[ "${{ matrix.os }}" == "ubuntu-latest" ]]; then
      apt-get update
    elif [[ "${{ matrix.os }}" == "macos-latest" ]]; then
      brew update
    fi

# ✅ RECOMMENDED: Assign first, then compare (consistent and defensive)
- run: |
    os="${{ matrix.os }}"
    if [[ "$os" == "ubuntu-latest" ]]; then
      apt-get update
    elif [[ "$os" == "macos-latest" ]]; then
      brew update
    fi
```

**Common contexts where this pattern applies:**

| Context | Incorrect | Correct |
|---------|-----------|---------|
| Matrix values | `if [[ "${{ matrix.x }}" == ... ]]` | `x="${{ matrix.x }}"; if [[ "$x" == ... ]]` |
| Inputs | `if [[ "${{ inputs.mode }}" == ... ]]` | `mode="${{ inputs.mode }}"; if [[ "$mode" == ... ]]` |
| Environment | `if [[ "${{ env.VAR }}" == ... ]]` | `var="${{ env.VAR }}"; if [[ "$var" == ... ]]` |
| GitHub context | `if [[ "${{ github.event_name }}" == ... ]]` | `event="${{ github.event_name }}"; if [[ "$event" == ... ]]` |
| Step outputs | `if [[ "${{ steps.x.outputs.y }}" == ... ]]` | `val="${{ steps.x.outputs.y }}"; if [[ "$val" == ... ]]` |
| Job results | `if [[ "${{ needs.x.result }}" == ... ]]` | `result="${{ needs.x.result }}"; if [[ "$result" == ... ]]` |
| Job outputs | `if [[ "${{ needs.x.outputs.y }}" == ... ]]` | `val="${{ needs.x.outputs.y }}"; if [[ "$val" == ... ]]` |

**Why the fix works:** Once assigned to a bash variable, shellcheck recognizes it as a proper variable and can analyze the conditional correctly. The value substitution still happens at GitHub Actions runtime, but shellcheck can now validate the bash syntax.

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

## Handling GitHub API Transient Errors

GitHub's API can experience transient failures that cause workflow steps to fail intermittently. Understanding and handling these errors properly improves workflow reliability.

### Why Transient Errors Occur

GitHub's API may return temporary errors under various conditions:

| Error Code | Meaning | Common Causes |
|------------|---------|---------------|
| **504 Gateway Timeout** | Request took too long | High API load, complex queries |
| **503 Service Unavailable** | Service temporarily down | Maintenance, capacity issues |
| **429 Too Many Requests** | Rate limit exceeded | Burst of API calls, shared limits |

These errors are **not** indicative of bugs in your workflow — they're normal operational conditions that workflows should handle gracefully.

### Using actions/github-script with Retries

The `actions/github-script` action provides built-in retry support:

```yaml
# ✅ RECOMMENDED: Use built-in retry options
- name: Add comment with retry
  uses: actions/github-script@v7
  with:
    retries: 3
    retry-exempt-status-codes: 400 401 403 404 422
    script: |
      await github.rest.issues.createComment({
        owner: context.repo.owner,
        repo: context.repo.repo,
        issue_number: context.issue.number,
        body: 'Automated comment'
      });
```

**Key options:**

- **`retries`**: Number of retry attempts (default: 0)
- **`retry-exempt-status-codes`**: Status codes that should NOT be retried (client errors)

By setting `retry-exempt-status-codes: 400 401 403 404 422`, only server errors (5xx) and rate limits (429) trigger retries.

> **⚠️ Important:** Use EITHER the built-in `retries` option OR a custom `withRetry()` function — never both. Combining them creates redundant retry logic where each API call could be retried up to N×M times (e.g., 3×3 = 9 times), causing excessive delays on persistent failures.

### Custom Retry Logic with Exponential Backoff

For more control (e.g., custom delay patterns or selective retries), implement a retry helper function. **Only use this if you need behavior the built-in option doesn't provide:**

```yaml
# ⚠️ ADVANCED: Custom retry (do NOT combine with retries: option above)
- name: Fetch data with custom retry
  uses: actions/github-script@v7
  with:
    # NOTE: No 'retries' option here - using custom function instead
    script: |
      // Retry helper with exponential backoff
      async function withRetry(fn, maxRetries = 3, baseDelayMs = 1000) {
        let lastError;
        for (let attempt = 1; attempt <= maxRetries; attempt++) {
          try {
            return await fn();
          } catch (error) {
            lastError = error;
            const isRetryable = error.status >= 500 || error.status === 429 || error.message?.includes('timeout');
            if (!isRetryable || attempt === maxRetries) {
              throw error;
            }
            const delayMs = baseDelayMs * Math.pow(2, attempt - 1);
            console.log(`Attempt ${attempt} failed (${error.message}), retrying in ${delayMs}ms...`);
            await new Promise(resolve => setTimeout(resolve, delayMs));
          }
        }
        throw lastError;
      }

      // Use the retry helper
      const result = await withRetry(async () => {
        return await github.rest.repos.get({
          owner: context.repo.owner,
          repo: context.repo.repo
        });
      });

      console.log(`Repository: ${result.data.full_name}`);
```

**How exponential backoff works:**

- Attempt 1: Immediate
- Attempt 2: Wait 1 second (1000ms)
- Attempt 3: Wait 2 seconds (2000ms)
- Attempt 4: Wait 4 seconds (4000ms)

This prevents overwhelming an already struggling service.

### When to Use continue-on-error

For **non-critical** steps where failure shouldn't block the workflow:

```yaml
# ✅ CORRECT: Non-critical step that shouldn't fail the build
- name: Post status comment
  uses: actions/github-script@v7
  continue-on-error: true
  with:
    retries: 2
    script: |
      await github.rest.issues.createComment({
        owner: context.repo.owner,
        repo: context.repo.repo,
        issue_number: context.issue.number,
        body: 'Build completed successfully!'
      });
```

**Use `continue-on-error: true` when:**

- The step is informational (comments, labels, status updates)
- Failure doesn't affect the core workflow outcome
- You have fallback handling or can live without the result

**Do NOT use `continue-on-error: true` when:**

- The step's output is required by subsequent steps
- Failure indicates a real problem that needs attention
- Security-critical operations (secrets, permissions)

### Choosing a Retry Strategy

For most use cases, the built-in `retries` option is sufficient and preferred:

```yaml
# ✅ RECOMMENDED: Simple and effective for most cases
- name: API operation with retry
  uses: actions/github-script@v7
  with:
    retries: 3
    retry-exempt-status-codes: 400 401 403 404 422
    script: |
      const data = await github.rest.repos.getContent({
        owner: context.repo.owner,
        repo: context.repo.repo,
        path: 'important-file.json'
      });
      const content = JSON.parse(Buffer.from(data.data.content, 'base64').toString());
      core.setOutput('data', JSON.stringify(content));
```

Use a custom `withRetry()` function only when you need specific behavior:

- Custom delay patterns (e.g., longer delays for specific error types)
- Selective retry based on response content (not just status code)
- Different retry counts for different API calls within the same step

> **⚠️ Never combine both approaches.** Using `retries: 3` with a custom `withRetry()` function creates redundant retry logic (up to 9 retries total), causing excessive delays on persistent failures.

### Transient Error Handling Checklist

Before deploying workflows with GitHub API calls:

- [ ] Add `retries` option to `actions/github-script` steps
- [ ] Configure `retry-exempt-status-codes` to skip client errors
- [ ] Use `continue-on-error: true` for non-critical notifications
- [ ] Log retry attempts for debugging
- [ ] Consider rate limit implications when setting retry counts
- [ ] Avoid combining built-in retries with custom retry wrappers

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
| SC2193 | `${{ }}` in conditionals | Assign to variable first: `var="${{ x }}"; if [[ "$var" == ... ]]` |
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

## CI Job Timeout Guidelines

Appropriate timeout values prevent runaway jobs while avoiding false failures. Set timeouts based on job type and expected duration.

### Recommended Timeout Values

| Job Type | Timeout | Rationale |
|----------|---------|-----------|
| **Build & Test** | 30 min | Standard compile + test; includes sccache startup |
| **Miri (parallelized)** | 20 min | Miri is ~1000-7000x slower; parallelization helps |
| **Miri (single job)** | 60 min | Non-parallelized Miri needs more headroom |
| **Kani verification** | 60-120 min | SMT solving can be slow; depends on proof complexity |
| **Network tests** | 10-20 min | Real network operations have inherent delays |
| **Docker builds** | 15-30 min | Image building can be slow without cache |
| **Quick checks** (fmt, clippy) | 5-10 min | Should be fast; timeout catches hangs |
| **Documentation** | 10-15 min | Doc generation is I/O bound |
| **WASM builds** | 15 min | Cross-compilation adds overhead |
| **ARM64 cross-compile** | 30 min | Cross-rs containers need startup time |

### Setting Timeouts

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 30  # Job-level timeout

    steps:
      - name: Run tests
        run: cargo test
        timeout-minutes: 15  # Step-level timeout (optional)
```

### Timeout Best Practices

1. **Always set job-level timeouts** — Prevents runaway jobs from consuming minutes
2. **Use step-level timeouts sparingly** — For steps that might hang (network, external services)
3. **Add buffer for cold caches** — First runs without cache take longer
4. **Monitor actual durations** — Adjust timeouts based on real-world data
5. **Consider platform differences** — Windows and macOS runners may be slower

---

## Miri Testing in CI

Miri undefined behavior detection requires special considerations for CI due to its performance characteristics and symbolic execution nature.

> **See also:** [Miri Verification](miri-verification.md) for the complete Miri guide, including installation, configuration, and advanced usage patterns.

### Miri Performance Characteristics

| Factor | Impact | Mitigation |
|--------|--------|------------|
| **~1000-7000x slower** | Long test times | Parallelize across jobs |
| **Symbolic execution** | Memory intensive | Reduce iteration counts |
| **Multiple seeds** | Multiplies runtime | Balance seeds vs. parallelism |
| **Big-endian targets** | Additional overhead | Run selectively |

### Parallelizing Miri Tests

Split tests across multiple jobs for reasonable CI times:

```yaml
miri:
  name: Miri (${{ matrix.name }})
  runs-on: ${{ matrix.os }}
  timeout-minutes: 20
  strategy:
    fail-fast: false
    matrix:
      include:
        # Split by module for balanced load
        - name: Linux - protocol
          os: ubuntu-latest
          seeds: "0..4"
          test_filter: "network::protocol"

        - name: Linux - sessions
          os: ubuntu-latest
          seeds: "0..4"
          test_filter: "sessions::"

        # Combine smaller modules
        - name: Linux - core + misc
          os: ubuntu-latest
          seeds: "0..4"
          test_filter: "MULTI:hash::|rng::|checksum::"
```

### Miri Seed Configuration

Using `-Zmiri-many-seeds` tests multiple random interleavings:

```yaml
env:
  MIRIFLAGS: "-Zmiri-disable-isolation -Zmiri-many-seeds=0..4"
```

| Seed Count | CI Time | Coverage |
|------------|---------|----------|
| 0..4 | Fast | **Recommended for CI** |
| 0..8 | Moderate | Basic race detection |
| 0..16 | Slow | Thorough, use for critical code |
| 0..32 | Very slow | Use only for targeted investigation |

### Miri-Incompatible Test Patterns

Some test patterns don't work with Miri's symbolic execution:

#### Skip Tests Using Real Time

```rust
#[test]
#[cfg_attr(miri, ignore)]
fn test_with_timing() {
    let start = Instant::now();
    do_something();
    assert!(start.elapsed() < Duration::from_secs(1));
}
```

#### Skip Tests Using Sleep

```rust
#[test]
#[cfg_attr(miri, ignore)]
fn test_with_sleep() {
    std::thread::sleep(Duration::from_millis(100));
    // Miri doesn't simulate real time passage
}
```

#### Skip FFI Tests

```rust
#[test]
#[cfg_attr(miri, ignore)]
fn test_with_ffi() {
    // Miri cannot interpret C code
    unsafe { external_c_function(); }
}
```

#### Skip Heavy Property Tests

```rust
proptest! {
    // Reduce iterations under Miri
    #![proptest_config(ProptestConfig::with_cases(
        if cfg!(miri) { 5 } else { 256 }
    ))]

    #[test]
    fn prop_roundtrip(data in any::<Vec<u8>>()) {
        // Property test body
    }
}
```

### Running Multiple Test Groups

For complex test filtering, use a custom parsing approach:

```yaml
- name: Run Miri tests
  shell: bash
  run: |
    test_filter="${{ matrix.test_filter }}"

    # Handle MULTI: prefix for multiple test groups
    if [[ "$test_filter" == MULTI:* ]]; then
      filters="${test_filter#MULTI:}"
      IFS='|' read -ra FILTER_ARRAY <<< "$filters"
      for filter in "${FILTER_ARRAY[@]}"; do
        echo "Running tests matching: $filter"
        cargo miri test --lib -- "$filter"
      done
    else
      cargo miri test --lib -- "$test_filter"
    fi
```

### Cross-Platform Miri

Run Miri on all platforms to catch platform-specific UB:

```yaml
matrix:
  include:
    - os: ubuntu-latest
    - os: windows-latest
    - os: macos-latest
```

### Big-Endian Testing

Test endianness issues with s390x target:

```yaml
- name: Run Miri (big-endian)
  if: matrix.big_endian
  run: cargo miri test --lib --target s390x-unknown-linux-gnu -- --skip property_tests
  env:
    MIRIFLAGS: "-Zmiri-disable-isolation"
```

---

## Property-Based Tests in CI (Proptest)

Property-based tests generate random inputs, requiring special CI considerations.

### Timing Considerations

| Setting | Development | CI |
|---------|-------------|-----|
| Cases | 256 (default) | 256 or fewer |
| Timeout | None | Set step timeout |
| Seeds | Random | Consider deterministic |

### Reducing Test Cases for CI

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        cases: if std::env::var("CI").is_ok() { 100 } else { 256 },
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_expensive_test(data in complex_strategy()) {
        // Property test body
    }
}
```

### Deterministic Seeds for Reproducibility

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        source_file: Some("tests/property_tests.rs"),
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_reproducible(x in any::<i32>()) {
        // Same seed from file path = same test sequence
    }
}
```

### Proptest with Miri

Property tests under Miri need dramatically reduced iterations:

```rust
fn miri_case_count() -> u32 {
    if cfg!(miri) { 5 } else { 256 }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(miri_case_count()))]

    #[test]
    fn prop_roundtrip(data in proptest::collection::vec(any::<u8>(), 0..100)) {
        // Reduced input sizes for Miri
    }
}
```

### Regression File Management

```yaml
# Ensure proptest regressions are committed
- name: Check for uncommitted regression files
  run: |
    if [ -n "$(git status --porcelain proptest-regressions/)" ]; then
      echo "ERROR: Uncommitted proptest regression files found"
      git status proptest-regressions/
      exit 1
    fi
```

---

## Cross-Platform Shell Script Compatibility

Shell scripts in workflows must work across Linux, macOS, and Windows runners.

### Temporary Directories

| Platform | Temp Path | Environment Variable |
|----------|-----------|---------------------|
| Linux | `/tmp` | `$TMPDIR` or `/tmp` |
| macOS | `/var/folders/...` | `$TMPDIR` |
| Windows | `C:\Users\...\Temp` | `$TEMP` or `$TMP` |

**Always use `$RUNNER_TEMP`** for cross-platform compatibility:

```yaml
# ✅ CORRECT: Cross-platform
- run: |
    WORK_DIR="$RUNNER_TEMP/my-work"
    mkdir -p "$WORK_DIR"

# ❌ WRONG: Linux-only
- run: |
    WORK_DIR="/tmp/my-work"
    mkdir -p "$WORK_DIR"
```

### Timeout Commands

The `timeout` command differs across platforms:

```yaml
# ✅ CORRECT: Use step-level timeout
- name: Run potentially slow command
  run: cargo test
  timeout-minutes: 10

# ❌ WRONG: timeout command doesn't exist on Windows/macOS
- run: timeout 600 cargo test
```

For inline timeouts, use a cross-platform approach:

```yaml
- name: Run with timeout (cross-platform)
  shell: bash
  run: |
    if command -v timeout &> /dev/null; then
      timeout 600 cargo test
    elif command -v gtimeout &> /dev/null; then
      gtimeout 600 cargo test  # macOS with coreutils
    else
      cargo test  # Rely on step timeout
    fi
  timeout-minutes: 12
```

### Path Handling

```yaml
# ✅ CORRECT: Forward slashes work everywhere in bash
- shell: bash
  run: |
    CONFIG_PATH="${GITHUB_WORKSPACE}/config/settings.toml"
    cat "$CONFIG_PATH"

# ❌ PROBLEMATIC: Backslashes can cause issues
- shell: bash
  run: |
    CONFIG_PATH="${GITHUB_WORKSPACE}\\config\\settings.toml"
```

### Line Endings

```yaml
# Ensure consistent line endings for scripts
- name: Configure git for consistent line endings
  run: git config --global core.autocrlf false
```

### Shell Selection

```yaml
# Explicitly set bash for consistent behavior
- name: Cross-platform script
  shell: bash
  run: |
    # This runs in bash on all platforms
    echo "Running on $OSTYPE"
```

---

## CI Job Monitoring and Debugging

### Identifying Empty Log Files

Empty logs often indicate the job was cancelled before producing output.

**Common causes:**

1. **Job timeout reached** — Increase `timeout-minutes`
2. **Workflow cancelled** — Check for concurrent workflow runs
3. **Runner crashed** — Retry the job
4. **OOM killed** — Reduce memory usage or use larger runner

### Debugging Silent Failures

```yaml
- name: Run with verbose output
  run: |
    set -x  # Echo commands
    cargo test --verbose 2>&1 | tee test-output.log
  continue-on-error: true

- name: Upload logs on failure
  if: failure()
  uses: actions/upload-artifact@v4
  with:
    name: debug-logs
    path: test-output.log
```

### Capturing Output for Analysis

```yaml
- name: Run tests with output capture
  id: tests
  run: |
    set +e  # Don't exit on error
    OUTPUT=$(cargo test 2>&1)
    EXIT_CODE=$?
    echo "$OUTPUT"
    echo "output<<EOF" >> "$GITHUB_OUTPUT"
    echo "$OUTPUT" >> "$GITHUB_OUTPUT"
    echo "EOF" >> "$GITHUB_OUTPUT"
    exit $EXIT_CODE
```

### Job Failure Notifications

```yaml
- name: Notify on failure
  if: failure()
  uses: actions/github-script@v7
  with:
    retries: 3
    script: |
      await github.rest.issues.createComment({
        owner: context.repo.owner,
        repo: context.repo.repo,
        issue_number: context.issue.number,
        body: `CI failed: ${context.workflow} / ${context.job}`
      });
```

### Debugging Specific Job Types

#### Miri Debugging

```yaml
env:
  MIRIFLAGS: "-Zmiri-disable-isolation -Zmiri-backtrace=full"
  RUST_BACKTRACE: 1
```

#### Kani Debugging

```yaml
- name: Run Kani with verbose output
  run: cargo kani --harness proof_name --verbose
```

#### Network Test Debugging

```yaml
- name: Debug network tests
  run: |
    # Show network configuration
    ip addr show || ifconfig
    netstat -tuln || ss -tuln

    # Run tests with debug output
    RUST_LOG=debug cargo test network
```

---

## Preventing Common CI Failures

### Pre-Flight Checks

Run these locally before pushing:

```bash
# Format + lint + test
cargo fmt && cargo clippy --all-targets && cargo nextest run --no-capture

# Workflow validation
actionlint

# Markdown linting
npx markdownlint '**/*.md' --config .markdownlint.json --fix
```

### CI-Specific Environment Variables

```yaml
env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1
  # Prevent cargo from downloading same crate multiple times
  CARGO_NET_RETRY: 10
  CARGO_NET_TIMEOUT: 120
```

### Handling Flaky Tests

```yaml
# Retry flaky tests
- name: Run tests with retry
  uses: nick-fields/retry@v3
  with:
    timeout_minutes: 15
    max_attempts: 3
    command: cargo test
```

### Cache Warming for Cold Starts

```yaml
# Pre-warm cache in a dedicated job
warm-cache:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: Swatinem/rust-cache@v2
    - run: cargo fetch
    - run: cargo build --all-targets
```

---

## Related Documentation

- [Cross-Platform CI/CD](cross-platform-ci-cd.md) — Multi-platform build strategies
- [CI/CD Debugging](ci-cd-debugging.md) — Reproducing and debugging CI failures locally
- [Miri Verification](miri-verification.md) — Miri UB detection guide
- [Miri Adaptation Guide](miri-adaptation-guide.md) — Adapting code for Miri
- [Property Testing](property-testing.md) — Proptest patterns and best practices
- [Defensive Programming](defensive-programming.md) — Error handling principles
- [Testing Guide](rust-testing-guide.md) — CI test organization

---

*License: MIT OR Apache-2.0*
