<!-- CATEGORY: CI/CD & Tooling -->
<!-- WHEN: Writing GitHub Actions workflows, CI debugging, actionlint, caching -->
# GitHub Actions Best Practices

## Mandatory: Run actionlint After Every Change

```bash
actionlint                                    # Lint all workflows
actionlint .github/workflows/ci-security.yml  # Lint specific file
```

actionlint validates: YAML syntax, `${{ }}` expressions, step/job references, contexts, and shell scripts via shellcheck.

## Shellcheck Rules for run: Blocks

### SC2193: Template Expressions in Conditionals

Assign `${{ }}` to variables before using in conditionals (required for glob patterns, recommended for all):

```yaml
# WRONG: shellcheck sees literal string, warns it can never match
- run: |
    if [[ "${{ matrix.test_filter }}" == MULTI:* ]]; then echo "match"; fi

# CORRECT: assign to variable first
- run: |
    test_filter="${{ matrix.test_filter }}"
    if [[ "$test_filter" == MULTI:* ]]; then echo "match"; fi
```

Applies to all contexts: `matrix.*`, `inputs.*`, `env.*`, `github.*`, `steps.*.outputs.*`, `needs.*.result`.

### Other Common Shellcheck Fixes

| Code | Issue | Fix |
|------|-------|-----|
| SC2086 | Unquoted variable | Quote: `"$VAR"` |
| SC2129 | Multiple individual redirects | `{ echo "a"; echo "b"; } >> file` |
| SC2155 | `export x=$(cmd)` masks exit | `x=$(cmd); export x` |
| SC2028 | `echo` with escapes | Use `printf` |
| SC2035 | Unsafe glob patterns | Prefix with `./` |
| SC2016 | Variables in single quotes | Use double quotes, or suppress if intentional |

### Intentional Suppressions

```bash
# shellcheck disable=SC2016  # Intentional: function body expands at call time
git config --global credential.helper '!f() { echo "password=${GITHUB_TOKEN}"; }; f'
```

Always add a comment explaining WHY. Be specific -- disable only the specific code.

## GITHUB_OUTPUT and GITHUB_ENV

```yaml
- name: Set outputs
  id: step_id
  run: |
    {
      echo "name=value"
      echo "multi_line<<EOF"
      echo "line 1"
      echo "EOF"
    } >> "$GITHUB_OUTPUT"

- name: Set env vars for subsequent steps
  run: echo "MY_VAR=value" >> "$GITHUB_ENV"
```

## GitHub API Retry Patterns

```yaml
# RECOMMENDED: Built-in retries
- uses: actions/github-script@v7
  with:
    retries: 3
    retry-exempt-status-codes: 400 401 403 404 422
    script: |
      await github.rest.issues.createComment({ ... });
```

Never combine built-in `retries` with custom `withRetry()` functions (creates N*M retries).

Use `continue-on-error: true` only for non-critical steps (comments, labels, status updates).

## Security

```yaml
# Use environment variables, not inline expressions for user input
- run: echo "$PR_TITLE"
  env:
    PR_TITLE: ${{ github.event.pull_request.title }}

# Use git credential helper instead of embedding tokens in URLs
- run: |
    git config --global credential.helper '!f() { echo "password=${GITHUB_TOKEN}"; }; f'
    git config --global credential.https://github.com.username "x-access-token"
    git clone "https://github.com/owner/repo.wiki.git" _wiki_clone
```

## Directory Conflicts in Clone Operations

Clone directories must not conflict with existing repo directories. Use `_` prefix:

| Avoid | Use Instead | Why |
|-------|-------------|-----|
| `wiki` | `_wiki_clone` | `wiki/` common in repos |
| `docs` | `_docs_clone` | `docs/` exists everywhere |
| `build` | `_build_temp` | `build/` used by many tools |

Or use `$RUNNER_TEMP` for complete isolation.

## Workflow Structure

```yaml
permissions:
  contents: read          # Explicit minimal permissions

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always
  CARGO_NET_RETRY: 10
```

## Timeout Guidelines

| Job Type | Timeout | Rationale |
|----------|---------|-----------|
| Quick checks (fmt, clippy) | 5-10 min | Should be fast |
| Build & Test | 30 min | Standard compile + test |
| Miri (parallelized) | 20 min | ~1000-7000x slower |
| Kani verification | 60-120 min | SMT solving is slow |
| WASM builds | 15 min | Cross-compilation overhead |
| Docker builds | 15-30 min | Slow without cache |

```yaml
jobs:
  build:
    timeout-minutes: 30
```

## Caching

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: "ci"
```

Or manual:

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: ${{ runner.os }}-cargo-
```

## Miri in CI

Split tests across jobs for reasonable times. Use `-Zmiri-many-seeds=0..4` for CI:

```yaml
miri:
  timeout-minutes: 20
  strategy:
    fail-fast: false
    matrix:
      include:
        - { name: "protocol", test_filter: "network::protocol" }
        - { name: "sessions", test_filter: "sessions::" }
        - { name: "core", test_filter: "MULTI:hash::|rng::|checksum::" }
```

Handle MULTI: prefix for multiple test groups:

```yaml
- run: |
    test_filter="${{ matrix.test_filter }}"
    if [[ "$test_filter" == MULTI:* ]]; then
      filters="${test_filter#MULTI:}"
      IFS='|' read -ra FILTER_ARRAY <<< "$filters"
      for filter in "${FILTER_ARRAY[@]}"; do
        cargo miri test --lib -- "$filter"
      done
    else
      cargo miri test --lib -- "$test_filter"
    fi
```

Skip Miri-incompatible tests with `#[cfg_attr(miri, ignore)]` for tests using real time, sleep, or FFI.

## Cross-Platform Shell Compatibility

- Use `$RUNNER_TEMP` (not `/tmp`) for temp directories
- Use step-level `timeout-minutes:` (not `timeout` command)
- Use `shell: bash` explicitly for consistent behavior
- Use forward slashes in paths (work everywhere in bash)

## Debugging

```yaml
# Enable verbose output
env:
  ACTIONS_STEP_DEBUG: true
  RUST_BACKTRACE: 1

# Debug expressions
- run: echo '${{ toJSON(github) }}'

# Upload logs on failure
- if: failure()
  uses: actions/upload-artifact@v4
  with:
    name: debug-logs
    path: test-output.log
```

## Pre-Commit Checklist

- [ ] Run `actionlint` on modified workflow files
- [ ] Check shellcheck warnings in `run:` blocks
- [ ] Verify `permissions:` block is minimal
- [ ] Verify `runs-on` uses valid runner labels
- [ ] Validate matrix combinations
