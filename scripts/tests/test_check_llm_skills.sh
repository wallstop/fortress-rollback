#!/usr/bin/env bash
#
# test_check_llm_skills.sh - Tests for check-llm-skills.sh
#
# Creates temporary markdown files with known violations and verifies
# that the script correctly detects them (or passes clean files).
#
# Usage:
#   ./scripts/tests/test_check_llm_skills.sh
#
# Exit codes:
#   0 - All tests passed
#   1 - One or more tests failed

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK_SCRIPT="$SCRIPT_DIR/../docs/check-llm-skills.sh"
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

# ── Colors ────────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BOLD='\033[1m'
NC='\033[0m'

# ── Test helpers ──────────────────────────────────────────────────────────────

setup_tmpdir() {
    TEST_TMPDIR="$(mktemp -d)"
    mkdir -p "$TEST_TMPDIR/.llm/skills"
}

teardown_tmpdir() {
    if [[ -n "${TEST_TMPDIR:-}" ]] && [[ -d "$TEST_TMPDIR" ]]; then
        rm -rf "$TEST_TMPDIR"
    fi
}

# Run the check script and capture exit code and output
run_check() {
    local dir="${1:-$TEST_TMPDIR/.llm}"
    local exit_code=0
    OUTPUT="$(bash "$CHECK_SCRIPT" --dir "$dir" 2>&1)" || exit_code=$?
    echo "$OUTPUT"
    return "$exit_code"
}

run_check_fix() {
    local dir="${1:-$TEST_TMPDIR/.llm}"
    local exit_code=0
    OUTPUT="$(bash "$CHECK_SCRIPT" --fix --dir "$dir" 2>&1)" || exit_code=$?
    echo "$OUTPUT"
    return "$exit_code"
}

assert_pass() {
    local test_name="$1"
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    local exit_code=0
    OUTPUT="$(run_check 2>&1)" || exit_code=$?
    if [[ "$exit_code" -eq 0 ]]; then
        echo -e "  ${GREEN}PASS${NC}: $test_name"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "  ${RED}FAIL${NC}: $test_name (expected pass, got exit code $exit_code)"
        echo "    Output: $OUTPUT"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

assert_fail() {
    local test_name="$1"
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    local exit_code=0
    OUTPUT="$(run_check 2>&1)" || exit_code=$?
    if [[ "$exit_code" -ne 0 ]]; then
        echo -e "  ${GREEN}PASS${NC}: $test_name"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "  ${RED}FAIL${NC}: $test_name (expected failure, got exit code 0)"
        echo "    Output: $OUTPUT"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

assert_output_contains() {
    local test_name="$1"
    local expected="$2"
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    if echo "$OUTPUT" | grep -qF "$expected"; then
        echo -e "  ${GREEN}PASS${NC}: $test_name"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "  ${RED}FAIL${NC}: $test_name (expected output to contain '$expected')"
        echo "    Output: $OUTPUT"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

assert_output_not_contains() {
    local test_name="$1"
    local unexpected="$2"
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    if ! echo "$OUTPUT" | grep -qF "$unexpected"; then
        echo -e "  ${GREEN}PASS${NC}: $test_name"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "  ${RED}FAIL${NC}: $test_name (output should NOT contain '$unexpected')"
        echo "    Output: $OUTPUT"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

# ── Tests ─────────────────────────────────────────────────────────────────────

test_clean_file_passes() {
    echo -e "${BOLD}Test: Clean file passes all checks${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/clean.md" << 'MDEOF'
# Clean Skill

Some documentation text.

```rust
let value = some_fn().ok_or(Error::Missing)?;
let items: Vec<_> = data.iter().collect();
```

```bash
set -euo pipefail
echo "hello"
```
MDEOF

    assert_pass "clean file passes"
    teardown_tmpdir
}

test_unwrap_without_comment_fails() {
    echo -e "${BOLD}Test: .unwrap() without comment is flagged${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/bad-unwrap.md" << 'MDEOF'
# Bad Unwrap

```rust
let val = something.unwrap();
```
MDEOF

    assert_fail ".unwrap() without comment fails"
    assert_output_contains "violation mentions unwrap" "unwrap"
    teardown_tmpdir
}

test_expect_without_comment_fails() {
    echo -e "${BOLD}Test: .expect() without comment is flagged${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/bad-expect.md" << 'MDEOF'
# Bad Expect

```rust
let val = something.expect("should work");
```
MDEOF

    assert_fail ".expect() without comment fails"
    assert_output_contains "violation mentions unwrap check" "unwrap"
    teardown_tmpdir
}

test_unwrap_with_justifying_comment_passes() {
    echo -e "${BOLD}Test: .unwrap() with justifying comment passes${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/ok-unwrap.md" << 'MDEOF'
# Justified Unwrap

```rust
let val = something.unwrap(); // test: this is in a test context
let val2 = other.unwrap(); // build.rs: guaranteed to exist
let val3 = another.unwrap(); // Loom test: loom model
let val4 = thing.expect("msg"); // allowed: static data
let val5 = item.unwrap(); // proptest: strategy value
let val6 = data.unwrap(); // Fuzz target: fuzzer input
let val7 = result.unwrap(); // SAFETY: infallible
```
MDEOF

    assert_pass "justified .unwrap() passes"
    teardown_tmpdir
}

test_unwrap_outside_rust_block_ignored() {
    echo -e "${BOLD}Test: .unwrap() outside rust code block is ignored${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/prose-unwrap.md" << 'MDEOF'
# Discussion

Never use .unwrap() in production code.

```python
value = dict.get("key")  # not rust, so .unwrap() reference is fine
```
MDEOF

    assert_pass ".unwrap() in prose/non-rust block ignored"
    teardown_tmpdir
}

test_ambiguous_result_collect_fails() {
    echo -e "${BOLD}Test: Ambiguous Result in collect is flagged${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/bad-collect.md" << 'MDEOF'
# Ambiguous Collect

```rust
let results: Vec<_> = items.iter().map(parse).collect::<Result<Vec<i32>>>();
```
MDEOF

    assert_fail "ambiguous Result in collect fails"
    assert_output_contains "violation mentions ambiguous-result" "ambiguous-result"
    teardown_tmpdir
}

test_explicit_result_collect_passes() {
    echo -e "${BOLD}Test: Explicit Result<T, E> in collect passes${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/ok-collect.md" << 'MDEOF'
# Explicit Collect

```rust
let results = items.iter().map(parse).collect::<Result<Vec<i32>, MyError>>();
```
MDEOF

    assert_pass "explicit Result<T, E> in collect passes"
    teardown_tmpdir
}

test_catch_unwind_without_assert_fails() {
    echo -e "${BOLD}Test: catch_unwind without AssertUnwindSafe is flagged${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/bad-unwind.md" << 'MDEOF'
# Bad Unwind

```rust
let result = std::panic::catch_unwind(|| {
    dangerous_code();
});
```
MDEOF

    assert_fail "catch_unwind without AssertUnwindSafe fails"
    assert_output_contains "violation mentions catch-unwind" "catch-unwind"
    teardown_tmpdir
}

test_catch_unwind_with_assert_passes() {
    echo -e "${BOLD}Test: catch_unwind with AssertUnwindSafe passes${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/ok-unwind.md" << 'MDEOF'
# OK Unwind

```rust
let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
    dangerous_code();
}));
```
MDEOF

    assert_pass "catch_unwind with AssertUnwindSafe passes"
    teardown_tmpdir
}

test_line_count_over_limit_fails() {
    echo -e "${BOLD}Test: File over 300 lines is flagged${NC}"
    setup_tmpdir

    # Generate a file with 310 lines
    {
        echo "# Long File"
        for i in $(seq 1 310); do
            echo "Line $i of content"
        done
    } > "$TEST_TMPDIR/.llm/skills/long.md"

    assert_fail "file over 300 lines fails"
    assert_output_contains "violation mentions line-limit" "line-limit"
    teardown_tmpdir
}

test_line_count_at_limit_passes() {
    echo -e "${BOLD}Test: File at exactly 300 lines passes${NC}"
    setup_tmpdir

    # Generate a file with exactly 300 lines
    {
        for i in $(seq 1 300); do
            echo "Line $i"
        done
    } > "$TEST_TMPDIR/.llm/skills/exact.md"

    assert_pass "file at exactly 300 lines passes"
    teardown_tmpdir
}

test_dev_null_in_bash_block_fails() {
    echo -e "${BOLD}Test: 2>/dev/null in bash block is flagged${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/bad-devnull.md" << 'MDEOF'
# Bad Dev Null

```bash
command -v tool 2>/dev/null || echo "not found"
```
MDEOF

    assert_fail "2>/dev/null in bash block fails"
    assert_output_contains "violation mentions dev-null" "dev-null"
    teardown_tmpdir
}

test_dev_null_in_sh_block_fails() {
    echo -e "${BOLD}Test: 2>/dev/null in sh block is flagged${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/bad-devnull-sh.md" << 'MDEOF'
# Bad Dev Null Shell

```sh
ls missing_file 2>/dev/null
```
MDEOF

    assert_fail "2>/dev/null in sh block fails"
    teardown_tmpdir
}

test_dev_null_outside_shell_block_ignored() {
    echo -e "${BOLD}Test: 2>/dev/null outside shell block is ignored${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/prose-devnull.md" << 'MDEOF'
# Discussion

Never use 2>/dev/null in scripts.

```rust
// This is rust, 2>/dev/null doesn't apply
let x = 5;
```
MDEOF

    assert_pass "2>/dev/null in prose/rust block ignored"
    teardown_tmpdir
}

test_fix_mode_shows_suggestions() {
    echo -e "${BOLD}Test: --fix mode shows fix suggestions${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/fixable.md" << 'MDEOF'
# Fixable

```rust
let val = something.unwrap();
```
MDEOF

    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    local exit_code=0
    OUTPUT="$(run_check_fix 2>&1)" || exit_code=$?
    if echo "$OUTPUT" | grep -qF "FIX:"; then
        echo -e "  ${GREEN}PASS${NC}: --fix mode shows suggestions"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "  ${RED}FAIL${NC}: --fix mode should show FIX: suggestions"
        echo "    Output: $OUTPUT"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    teardown_tmpdir
}

test_multiple_violations_counted() {
    echo -e "${BOLD}Test: Multiple violations are counted correctly${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/multi.md" << 'MDEOF'
# Multiple Issues

```rust
let a = x.unwrap();
let b = y.expect("bad");
```

```bash
cmd 2>/dev/null
```
MDEOF

    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    local exit_code=0
    OUTPUT="$(run_check 2>&1)" || exit_code=$?
    if echo "$OUTPUT" | grep -qE "Found [3-9]|Found [0-9][0-9]+ violation"; then
        echo -e "  ${GREEN}PASS${NC}: multiple violations counted"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "  ${RED}FAIL${NC}: expected 3+ violations"
        echo "    Output: $OUTPUT"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    teardown_tmpdir
}

test_nonexistent_directory_exits_zero() {
    echo -e "${BOLD}Test: Nonexistent directory exits with 0${NC}"
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    local exit_code=0
    OUTPUT="$(bash "$CHECK_SCRIPT" --dir "/tmp/nonexistent_dir_$$" 2>&1)" || exit_code=$?
    if [[ "$exit_code" -eq 0 ]]; then
        echo -e "  ${GREEN}PASS${NC}: nonexistent directory exits 0"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "  ${RED}FAIL${NC}: expected exit 0 for nonexistent dir, got $exit_code"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

test_empty_directory_passes() {
    echo -e "${BOLD}Test: Empty directory passes${NC}"
    setup_tmpdir

    # .llm exists but no .md files
    assert_pass "empty directory passes"
    teardown_tmpdir
}

# ── New tests: preceding-line justification, comment skipping, allow attrs ────

test_unwrap_with_justification_on_preceding_line_passes() {
    echo -e "${BOLD}Test: .unwrap() justified by comment on preceding line passes${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/prev-line-just.md" << 'MDEOF'
# Preceding-Line Justification

```rust
// build.rs: Cargo guarantees these env vars exist
let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
```
MDEOF

    assert_pass "justification on preceding line passes"
    teardown_tmpdir
}

test_unwrap_with_justification_several_lines_above_passes() {
    echo -e "${BOLD}Test: .unwrap() justified by comment several lines above passes${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/multi-line-just.md" << 'MDEOF'
# Multi-Line Chain Justification

```rust
// build.rs: panic is appropriate for build failures
cbindgen::Builder::new()
    .with_crate(&crate_dir)
    .with_language(cbindgen::Language::C)
    .generate().unwrap()
    .write_to_file("include/my_lib.h");
```
MDEOF

    assert_pass "justification several lines above passes"
    teardown_tmpdir
}

test_unwrap_in_rust_comment_line_ignored() {
    echo -e "${BOLD}Test: .unwrap() in a Rust comment line is ignored${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/comment-unwrap.md" << 'MDEOF'
# Comment Mentions Unwrap

```rust
// Use parking_lot::RwLock -- no poisoning, no .unwrap() needed
let lock = RwLock::new(42);
// FORBIDDEN in production:  value.unwrap(), .expect()
let x = 5;
// In tests: .unwrap() is idiomatic -- panics = test failure
let y = 10;
```
MDEOF

    assert_pass ".unwrap() in Rust comment line ignored"
    teardown_tmpdir
}

test_unwrap_with_allow_attribute_passes() {
    echo -e "${BOLD}Test: .unwrap() with #[allow(clippy::unwrap_used)] passes${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/allow-attr.md" << 'MDEOF'
# Allow Attribute

```rust
#[allow(clippy::unwrap_used, reason = "Vec non-empty after push")]
let first = vec.first().unwrap();
```
MDEOF

    assert_pass "#[allow(clippy::unwrap_used)] justifies .unwrap()"
    teardown_tmpdir
}

test_unwrap_with_fixture_attribute_passes() {
    echo -e "${BOLD}Test: .unwrap() in #[fixture] fn passes (test context)${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/fixture-attr.md" << 'MDEOF'
# Fixture Attribute

```rust
#[fixture]
fn session() -> Session { Session::new(4).unwrap() }
```
MDEOF

    assert_pass "#[fixture] attribute justifies .unwrap()"
    teardown_tmpdir
}

test_unwrap_in_forbidden_block_passes() {
    echo -e "${BOLD}Test: .unwrap() in forbidden-patterns demonstration passes${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/forbidden-demo.md" << 'MDEOF'
# Zero-Panic Policy

### Forbidden Patterns

```rust
panic!(); value.unwrap(); value.expect("..."); array[index]; todo!();
```
MDEOF

    assert_pass ".unwrap() in forbidden-patterns demonstration ignored"
    teardown_tmpdir
}

test_unwrap_still_flagged_without_justification() {
    echo -e "${BOLD}Test: .unwrap() without any justification is still flagged${NC}"
    setup_tmpdir

    cat > "$TEST_TMPDIR/.llm/skills/no-just.md" << 'MDEOF'
# No Justification

Some paragraph.

```rust
let val = something.unwrap();
```
MDEOF

    assert_fail ".unwrap() without any justification is still flagged"
    teardown_tmpdir
}

# ── Run all tests ─────────────────────────────────────────────────────────────
main() {
    echo -e "${BOLD}Running tests for check-llm-skills.sh${NC}"
    echo "================================================"
    echo ""

    test_clean_file_passes
    test_unwrap_without_comment_fails
    test_expect_without_comment_fails
    test_unwrap_with_justifying_comment_passes
    test_unwrap_outside_rust_block_ignored
    test_ambiguous_result_collect_fails
    test_explicit_result_collect_passes
    test_catch_unwind_without_assert_fails
    test_catch_unwind_with_assert_passes
    test_line_count_over_limit_fails
    test_line_count_at_limit_passes
    test_dev_null_in_bash_block_fails
    test_dev_null_in_sh_block_fails
    test_dev_null_outside_shell_block_ignored
    test_fix_mode_shows_suggestions
    test_multiple_violations_counted
    test_nonexistent_directory_exits_zero
    test_empty_directory_passes
    test_unwrap_with_justification_on_preceding_line_passes
    test_unwrap_with_justification_several_lines_above_passes
    test_unwrap_in_rust_comment_line_ignored
    test_unwrap_with_allow_attribute_passes
    test_unwrap_with_fixture_attribute_passes
    test_unwrap_in_forbidden_block_passes
    test_unwrap_still_flagged_without_justification

    echo ""
    echo "================================================"
    echo -e "${BOLD}Results: $TESTS_PASSED/$TESTS_TOTAL passed, $TESTS_FAILED failed${NC}"

    if [[ "$TESTS_FAILED" -gt 0 ]]; then
        echo -e "${RED}Some tests failed${NC}"
        exit 1
    else
        echo -e "${GREEN}All tests passed${NC}"
        exit 0
    fi
}

main "$@"
