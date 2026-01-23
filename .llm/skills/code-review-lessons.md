# Code Review Lessons Learned

> **This document captures patterns and anti-patterns discovered through code review feedback.**
> Use these guidelines to prevent similar issues in future development.

---

## Eager vs Lazy Error Construction (`ok_or` vs `ok_or_else`)

### The Pattern

When using `Option::ok_or()`, the error value is constructed eagerly (every time), even when
the Option contains a value. This can be wasteful in hot paths.

```rust
// Eager construction — error built even when index is valid
value.ok_or(ExpensiveError { context: compute_context() })?

// Lazy construction — error only built on error path
value.ok_or_else(|| ExpensiveError { context: compute_context() })?
```

### When to Use `ok_or_else`

Use `ok_or_else(|| ...)` when:

- The error type allocates (contains `String`, `Vec`, `Box`, etc.)
- Computing error field values is expensive
- The code is in a hot path (inner loops, frequently called functions)
- The error construction has side effects

### When `ok_or` Is Fine

Use simple `ok_or(...)` when:

- The error type is `Copy` (no allocation, trivial construction)
- All field values are already computed or are `Copy`
- Clippy's `unnecessary_lazy_evaluations` lint would trigger

**Important:** Clippy will warn if you use `ok_or_else` with a `Copy` type. Trust the lint —
for `Copy` types, eager construction is actually more efficient than closure overhead.

```rust
// ❌ Clippy warning: unnecessary_lazy_evaluations
.ok_or_else(|| CopyError::IndexOutOfBounds { index, len })

// ✅ Correct for Copy types
.ok_or(CopyError::IndexOutOfBounds { index, len })
```

---

## Result Type Alias Semver Hazard

### The Problem

Exporting a `Result` type alias at the crate root can shadow `std::result::Result` for
downstream users who use glob imports:

```rust
// In your library
pub type Result<T, E = MyError> = std::result::Result<T, E>;

// Downstream user
use my_library::*;  // Now `Result` shadows std::result::Result!
```

### The Solution

Use a distinctive name that cannot shadow standard library types:

```rust
// ✅ Safe — cannot shadow std::result::Result
pub type FortressResult<T, E = FortressError> = std::result::Result<T, E>;
```

### Best Practices

1. **Use distinctive names** — `FortressResult`, `MyLibResult`, etc.
2. **Export from prelude only** — Don't export at crate root, only in a `prelude` module
3. **Document the pattern** — Show users how to alias locally if they prefer short names:

```rust
// Users can create a local alias
use fortress_rollback::FortressResult as Result;
```

---

## Test-Production Code Alignment

### The Problem

Tests that simulate internal implementation details can drift from production code:

```rust
// Production code (evolved)
fn decode(data: &[u8]) -> Result<Vec<u8>, MyError> {
    inner_decode(data).map_err(|e| match e {
        InnerError::Foo => MyError::Foo,
        InnerError::Bar => MyError::Bar,
    })?
}

// Tests (stuck on old implementation)
fn test_error_mapping() {
    let error: Box<dyn Error> = Box::new(SomeError);
    let result = error.downcast_ref::<InnerError>();  // Production doesn't do this!
    // ...
}
```

### The Solution

**Extract testable helpers.** When production code has error mapping logic, extract it:

```rust
// Extracted helper — testable in isolation
fn map_inner_error(e: InnerError) -> MyError {
    match e {
        InnerError::Foo => MyError::Foo,
        InnerError::Bar => MyError::Bar,
    }
}

// Production code uses the helper
fn decode(data: &[u8]) -> Result<Vec<u8>, MyError> {
    inner_decode(data).map_err(map_inner_error)?
}

// Tests test the helper directly
#[test]
fn test_map_inner_error_foo() {
    assert_eq!(map_inner_error(InnerError::Foo), MyError::Foo);
}
```

### Best Practices

1. **Test the actual code path** — Don't simulate patterns not in production
2. **Extract helpers for complex mappings** — Makes them unit-testable
3. **Add integration tests** — Verify end-to-end behavior with real inputs
4. **Review tests when refactoring** — Ensure tests still test the right thing

---

## Kani Proof Naming and Verification

### The Problem

Proof names and documentation can claim properties that the proof doesn't actually verify:

```rust
/// Proof: Clone creates independent copy.
/// Verifies that modifying one doesn't affect other.  // <-- claim
#[kani::proof]
fn proof_clone_is_independent() {
    let a = MyStruct::new();
    let b = a.clone();

    // Only checks equality, never modifies!
    kani::assert(a.field == b.field, "fields match");
}
```

### The Solution

**Proofs must verify what they claim.** If the name says "independent", actually test
modification independence:

```rust
#[kani::proof]
fn proof_clone_is_independent() {
    let a = MyStruct::new();
    let mut b = a.clone();

    let original_value = a.field;

    // Actually modify the clone
    b.field = different_value();

    // Verify original is unchanged (independence)
    kani::assert(a.field == original_value, "Original unchanged after modifying clone");

    // Verify clone has modification
    kani::assert(b.field != original_value, "Clone has new value");
}
```

### Best Practices

1. **Name proofs accurately** — `proof_clone_preserves_fields` vs `proof_clone_is_independent`
2. **Verify all claimed properties** — Read doc comments, ensure assertions match
3. **Consider renaming over extending** — If a proof tests X but claims Y, maybe rename to X
4. **Document proof scope clearly** — What exactly does this proof verify?

---

## Doc Comments and Implementation Details

### The Problem

Doc comments that describe *how* code works (implementation details) become stale when the
implementation changes:

```rust
/// Creates a violation with a unique ID.
/// Uses static string slices for zero-allocation performance.  // <-- LIE!
fn make_violation(id: u32) -> Violation {
    Violation::new(
        format!("test violation {}", id),  // Actually allocates!
    )
}
```

The doc comment claims "static string slices" but the code uses `format!()` which allocates.
This mismatch misleads readers and erodes trust in documentation.

### The Root Cause

Doc comments describing implementation details are inherently fragile because:

1. **Code changes, comments don't** — Refactoring updates code but forgets comments
2. **Performance claims age poorly** — Optimizations may be added or removed
3. **Allocation patterns shift** — Moving from `&'static str` to `String` is common
4. **Reviewers focus on code** — Comments are often skimmed, not verified against code

### The Solution

**Doc comments should describe WHAT, not HOW** — unless the HOW is part of the API contract.

```rust
// ❌ Describes implementation (fragile)
/// Creates a violation with a unique ID.
/// Uses static string slices for zero-allocation performance.

// ✅ Describes behavior (stable)
/// Creates a violation with a unique ID.

// ✅ OK if allocation IS the contract
/// Creates a violation with a unique ID.
///
/// # Performance
///
/// This function is allocation-free and suitable for hot paths.
/// (Note: This creates an API contract — changing it is breaking!)
```

### When Implementation Details ARE Appropriate

Include HOW only when it's part of the API contract:

- **Performance guarantees** — "O(1) lookup", "allocation-free"
- **Thread safety** — "Lock-free", "Uses interior mutability"
- **Determinism** — "Uses seeded RNG for reproducibility"
- **Resource management** — "Caches results", "Lazily initialized"

But remember: documenting these creates an implicit contract. Changing them becomes a
breaking change, even if the function signature doesn't change.

### Best Practices

1. **Focus on WHAT, not HOW** — Describe behavior and purpose
2. **Omit performance claims** — Unless they're API guarantees
3. **Review comments when refactoring** — Update or remove stale implementation details
4. **Use `# Performance` sections** — Makes performance contracts explicit and findable
5. **Avoid redundant phrases** — "for testing purposes" in test code is obvious

---

## GitHub Actions Permissions

### The Problem

Writing to system directories like `/usr/local/bin` requires elevated permissions on
GitHub-hosted runners:

```yaml
# ❌ Can fail with "permission denied"
run: |
  curl -sfL "$URL" | tar xz -C /usr/local/bin my_tool
```

### Solutions

**Option 1: Use sudo**

```yaml
# ✅ Works on GitHub-hosted runners
run: |
  curl -sfL "$URL" | sudo tar xz -C /usr/local/bin my_tool
```

**Option 2: Install to user directory**

```yaml
# ✅ No sudo needed
run: |
  mkdir -p "$HOME/.local/bin"
  curl -sfL "$URL" | tar xz -C "$HOME/.local/bin" my_tool
  echo "$HOME/.local/bin" >> "$GITHUB_PATH"
```

### Best Practices

1. **Be consistent** — Use the same pattern across all workflows
2. **Prefer sudo for /usr/local/bin** — It's simpler and widely understood
3. **Use GITHUB_PATH for custom directories** — Ensures tools are available to later steps
4. **Test on fresh runners** — Local dev containers may have different permissions

---

## Pattern Matching in Error Mappers

### The Problem

Using `if let` with a fallthrough path causes use-after-move bugs:

```rust
// ❌ BUG: `e` moved in if-let, unusable in fallback
fn map_error(e: MyError) -> OtherError {
    if let MyError::Specific { data } = e {
        return OtherError::Mapped { data };
    }
    log::warn!("unexpected: {:?}", e);  // ERROR: use of moved value!
    OtherError::Unknown
}
```

### The Solution

**Always use `match` for error mapping functions** that need to:

1. Handle a specific variant
2. Log/warn about unexpected variants

```rust
// ✅ CORRECT: Single match expression
fn map_error(e: MyError) -> OtherError {
    match e {
        MyError::Specific { data } => OtherError::Mapped { data },
        other => {
            log::warn!("unexpected: {:?}", other);
            OtherError::Unknown
        }
    }
}
```

### Why This Pattern Exists

Error mapping functions commonly need to:

- Extract fields from a specific error variant for the happy path
- Handle unexpected variants gracefully with logging/metrics
- Include the original error in the fallback for debugging

The `if let` + fallthrough pattern seems natural but moves ownership before the fallback.

### Best Practices

1. **Prefer `match` over `if let` + fallthrough** when you need the value in both paths
2. **Use `other` binding in catch-all arm** — gives access to the unmatched value
3. **Consider borrowing** — `if let Pattern = &value` if you don't need ownership
4. **Add `Unknown` variants to reason enums** — provides a fallback for error mapping

See also: [rust-pitfalls.md](rust-pitfalls.md#use-after-move-in-if-let-fallthrough)

---

## Shell Script Regex Flag Consistency

### The Problem

Using different regex modes in the same script leads to inconsistent behavior — one command
finds matches while another silently fails:

```bash
# ❌ BUG: Mixed regex modes
# Count uses extended regex (works with +, |, ?)
count=$(grep -Ec 'pattern1|pattern2+' file.txt)
echo "Found: $count matches"

# Display uses basic regex (+ and | are literal characters!)
grep -n 'pattern1|pattern2+' file.txt
# Output: nothing (even though count was > 0)
```

In basic regex (BRE), characters like `+`, `?`, `|`, and `()` are treated as literals.
Extended regex (ERE) treats them as metacharacters. Mixing modes causes correct counts
but empty output — a confusing failure mode.

### The Solution

**Use consistent regex flags throughout a script:**

```bash
# ✅ CORRECT: Consistent extended regex everywhere
count=$(grep -Ec 'pattern1|pattern2+' file.txt)
grep -En 'pattern1|pattern2+' file.txt

# ✅ ALTERNATIVE: Escape metacharacters in basic regex
count=$(grep -c 'pattern1\|pattern2\+' file.txt)
grep -n 'pattern1\|pattern2\+' file.txt
```

### Extended vs Basic Regex Quick Reference

| Character | Basic Regex (BRE) | Extended Regex (ERE) |
|-----------|-------------------|----------------------|
| `+`       | Literal `+`       | One or more          |
| `?`       | Literal `?`       | Zero or one          |
| `\|`      | Alternation       | Literal `\|`         |
| `()`      | Literal `()`      | Grouping             |
| `\(\)`    | Grouping          | Literal `()`         |

### Best Practices

1. **Use `-E` (extended regex) consistently** — It's more intuitive for complex patterns
2. **Define regex mode at script top** — Use a variable: `GREP_FLAGS="-E"`
3. **Test both count and display commands** — Verify they produce matching results
4. **Use shellcheck** — It catches some regex inconsistencies
5. **Prefer `grep -E` over `egrep`** — `egrep` is deprecated

---

## Shell Script Grep Count Error Handling

### The Problem

When using `grep -c` (count matches) with `|| true` or `|| echo "0"` under `set -euo pipefail`,
the variable can become empty or contain unexpected values, causing numeric comparisons to fail.

**Understanding grep exit codes:**

| Exit Code | Meaning | `grep -c` Output |
|-----------|---------|------------------|
| 0         | Matches found | Count (e.g., "5") |
| 1         | No matches found | "0" |
| 2         | Error (file not found, permission denied) | Nothing |

**The broken pattern:**

```bash
#!/bin/bash
set -euo pipefail

# ❌ BROKEN: Produces "0\n0" when no matches found
COUNT=$(grep -Ec "pattern" "$file" 2>/dev/null || echo "0")
if [[ "$COUNT" -gt 0 ]]; then  # Fails: "0\n0" is not a valid integer
    echo "Found $COUNT matches"
fi
```

When `grep -c` finds **no matches** in an existing file:

1. It outputs "0" to stdout
2. It exits with code 1 (failure)
3. The `|| echo "0"` fires because of exit code 1
4. Result: `COUNT` contains `"0\n0"` — **two zeros separated by a newline**

This causes `[[ "$COUNT" -gt 0 ]]` to fail with:

```
bash: [[: 0
0: syntax error in expression (error token is "0")
```

**Another broken pattern:**

```bash
# ❌ BROKEN: COUNT may be empty if grep exits with code 2 (file error)
COUNT=$(grep -Ec "pattern" "$file" 2>/dev/null || true)
if [[ "$COUNT" -gt 0 ]]; then  # Fails if COUNT is empty
    echo "Found matches"
fi
```

### The Solution

**Use `|| true` to suppress the exit code, then default to 0 if empty:**

```bash
# ✅ CORRECT: Handles all cases safely
COUNT=$(grep -Ec "pattern" "$file" 2>/dev/null || true)
COUNT=${COUNT:-0}  # Default to 0 if empty

if [[ "$COUNT" -gt 0 ]]; then
    echo "Found $COUNT matches"
fi
```

**Why this works:**

| Scenario | `grep -c` Output | Exit Code | After `\|\| true` | After `${COUNT:-0}` |
|----------|------------------|-----------|-------------------|---------------------|
| Matches found | "5" | 0 | "5" | "5" ✓ |
| No matches | "0" | 1 | "0" | "0" ✓ |
| File error | "" (empty) | 2 | "" | "0" ✓ |

### Best Practices

1. **Always use `|| true` + `${VAR:-0}`** — Never use `|| echo "0"` for grep counts
2. **Add a comment explaining the pattern** — Future maintainers may not know the pitfall
3. **Validate before arithmetic** — Use `[[ "$VAR" =~ ^[0-9]+$ ]]` if paranoid
4. **Prefer `wc -l` for line counting** — `wc -l` exits 0 even for zero lines

```bash
# Alternative: wc -l always exits 0
COUNT=$(grep -E "pattern" "$file" 2>/dev/null | wc -l)
# No fallback needed — wc -l outputs "0" and exits 0 for empty input
```

### Related Scripts in This Repository

The correct pattern is used in:

- `scripts/check-code-fence-syntax.sh`
- `scripts/check-kani-coverage.sh`
- `scripts/sync-version.sh`
- `scripts/verify-markdown-code.sh`
- `.github/workflows/ci-quality.yml`

---

## Cross-Platform Documentation Accuracy

### The Problem

Documentation claims cross-platform compatibility that isn't actually true:

```markdown
<!-- Documentation -->
# Pre-commit Hooks

Works on all platforms — no Git Bash or WSL required!

<!-- Reality: the hook is a bash script -->
#!/bin/bash
# This requires bash, which isn't default on Windows!
```

When new scripts or hooks are added, existing cross-platform claims may become false.

### The Solution

**Validate cross-platform claims whenever adding platform-dependent components:**

1. **Audit existing claims** — Search docs for "cross-platform", "all platforms", "Windows"
2. **Update or qualify claims** — Be specific about what works where
3. **Document requirements** — List what's needed on each platform

```markdown
<!-- ✅ ACCURATE: Specific requirements -->
# Pre-commit Hooks

Requires bash. On Windows, use Git Bash, WSL, or install bash via MSYS2.

<!-- ✅ ALTERNATIVE: Different hooks per platform -->
The pre-commit hook uses:
- **Unix/macOS**: Native bash script
- **Windows**: PowerShell version (see `hooks/pre-commit.ps1`)
```

### Cross-Platform Audit Checklist

When adding new scripts or hooks, verify:

- [ ] Shell scripts: Do they require bash, sh, or work with cmd/PowerShell?
- [ ] Path separators: `/` vs `\` — use `$()` or portable path handling
- [ ] Line endings: Will CRLF break the script?
- [ ] Available tools: Are `grep`, `sed`, `awk` available on Windows?
- [ ] File permissions: Does the script need `chmod +x`?

### Best Practices

1. **Be specific, not absolute** — "Works on Linux/macOS" is better than "cross-platform"
2. **List platform requirements** — "Requires bash 4.0+" or "Tested on: Ubuntu, macOS, Git Bash"
3. **Audit docs when adding scripts** — Search for platform claims and validate them
4. **Consider CI validation** — Run scripts on multiple OS runners to verify claims
5. **Provide alternatives** — PowerShell equivalents for Windows users

---

## Documentation Code Example Accuracy

### The Problem

Code examples in documentation don't match the actual API:

```markdown
<!-- Documentation shows -->
return Err(FortressError::InvalidRequest { info: "bad handle" });

<!-- Actual API requires -->
return Err(FortressError::InvalidRequest {
    kind: InvalidRequestKind::InvalidSpectatorHandle,
});
// Plus: InvalidRequestKind has a From impl for conversion
```

Readers try the documented pattern, get compiler errors, and lose trust in the docs.

### The Solution

**Option 1: Use exact API patterns (preferred)**

```rust
// ✅ Exact API usage — compiles and works
return Err(FortressError::InvalidRequest {
    kind: InvalidRequestKind::InvalidSpectatorHandle,
});
```

**Option 2: Mark as pseudo-code explicitly**

````markdown
```text
// Pseudo-code — see API docs for exact syntax
return Err(FortressError::InvalidRequest { info: "..." });
```
````

Or use inline annotation:

```rust
// Simplified for illustration — actual API differs
return Err(Error::SomeVariant { ... });
```

**Option 3: Use doc tests to validate examples**

```rust
/// # Example
///
/// ```
/// # use my_crate::*;
/// let result = fallible_operation();
/// assert!(result.is_err());
/// ```
```

Doc tests won't compile if the API changes, catching drift automatically.

### Best Practices

1. **Prefer compilable examples** — Use real API patterns that work
2. **Run doc tests in CI** — Catches documentation drift automatically
3. **Mark pseudo-code clearly** — Use `text` fence or explicit comments
4. **Keep examples minimal** — Less code = less to keep in sync
5. **Link to real examples** — "See `examples/error_handling.rs` for complete usage"
6. **Review examples when changing APIs** — Search docs for affected patterns

---

## Metrics Consistency Across Documentation

### The Problem

Project metrics appear in multiple places with different values:

```markdown
<!-- README.md -->
✅ 1100+ tests with comprehensive coverage

<!-- docs/index.md -->
The test suite includes ~1500 tests...

<!-- CONTRIBUTING.md -->
We maintain over 1000 tests...
```

Which is correct? Readers notice inconsistencies and question accuracy.

### The Solution

**Option 1: Centralize metrics (recommended for frequently-updated values)**

Create a single source of truth:

```markdown
<!-- docs/includes/metrics.md -->
- **Test count**: ~1500 (as of 2025-01)
- **Coverage**: >90%
- **Supported platforms**: 5
```

Reference via includes or variables:

```markdown
<!-- In README.md -->
{% include "includes/metrics.md" %}
```

**Option 2: Use relative/stable descriptions**

Avoid specific numbers that will become stale:

```markdown
<!-- ✅ Stable — won't become stale -->
Comprehensive test suite with >90% coverage

<!-- ❌ Specific — will become stale -->
1,523 tests covering all modules
```

**Option 3: Automate metric extraction**

Generate metrics from CI:

```yaml
- name: Update metrics
  run: |
    TEST_COUNT=$(cargo test --no-run 2>&1 | grep -oP '\d+ tests')
    # Use sed -i '' on macOS, sed -i on GNU/Linux
    # This pattern works on both by creating a backup then removing it
    sed -i.bak "s/TEST_COUNT_PLACEHOLDER/$TEST_COUNT/" docs/metrics.md
    rm -f docs/metrics.md.bak
```

### Metrics That Should Be Centralized

| Metric | Why | Recommended Approach |
|--------|-----|---------------------|
| Test count | Changes frequently | Use "comprehensive" or automate |
| Coverage % | Changes per commit | Centralize or use badge |
| Supported platforms | Changes rarely | OK to duplicate, audit quarterly |
| Performance benchmarks | Must be reproducible | Single source + methodology |

### Best Practices

1. **Audit metrics quarterly** — Search for numbers and percentages across docs
2. **Use relative terms for volatile metrics** — "comprehensive", "high coverage"
3. **Centralize stable metrics** — Platform count, version requirements
4. **Automate where possible** — Generate from CI, use badges
5. **Date your metrics** — "~1500 tests (as of 2025-01)" helps readers calibrate
6. **Search before adding** — Check if the metric exists elsewhere before duplicating

---

## Summary Checklist

Before submitting code:

- [ ] `ok_or` vs `ok_or_else` — Used correctly based on error type (Copy vs allocating)
- [ ] Type aliases — Use distinctive names that can't shadow stdlib types
- [ ] Tests match production — No simulating patterns not in real code
- [ ] Kani proofs — Actually verify what their names/docs claim
- [ ] Doc comments — Describe WHAT, not HOW (unless HOW is API contract)
- [ ] CI permissions — Use sudo for system directories
- [ ] Pattern matching — Use `match` not `if let` when fallback needs the value
- [ ] Shell scripts — Use consistent regex flags (`-E` everywhere or escape metacharacters)
- [ ] Cross-platform claims — Validate after adding scripts/hooks
- [ ] Code examples — Use real API patterns or mark as pseudo-code
- [ ] Metrics — Check for consistency across all documentation

---

*This document should be updated as new patterns are discovered through code review.*
