<!-- CATEGORY: Workflows -->
<!-- WHEN: Debugging failures, diagnosing desyncs, root-cause analysis, systematic troubleshooting -->

# Investigation and Debugging Workflow

Structured 4-phase investigation for fortress-rollback. Never guess fixes. Never apply a fix without evidence of root cause. If 3 hypotheses fail, stop and escalate.

---

## Phase 1: Observe (Gather Facts)

Collect evidence BEFORE forming any hypothesis.

### For Test Failures

**Rule:** Never pipe test output through `tail`/`head`. Redirect to temp files instead.

```bash
# Reproduce with full output (redirect to temp file, then read it)
RUST_BACKTRACE=1 cargo nextest run failing_test_name --no-capture > /tmp/test-fail.txt 2>&1

# Run in isolation (eliminate ordering/parallelism effects)
cargo test failing_test_name -- --test-threads=1 --nocapture > /tmp/test-isolated.txt 2>&1

# Check if flaky (run 10 times, use a for loop)
for i in $(seq 1 10); do cargo nextest run failing_test_name --no-capture >> /tmp/flaky-check.txt 2>&1 || echo "FAIL on run $i"; done
```

### For Desync Issues

Desync = peers diverge in game state. Always the hardest bugs.

```bash
# Enable checksum logging (requires tracing-subscriber initialization; see dev-pipeline prerequisites)
RUST_LOG=fortress_rollback=debug cargo test desync_test --no-capture

# Run SyncTestSession (forces rollback every frame)
# If SyncTest passes but P2P desyncs, the bug is in network/timing, not simulation
```

Record these facts before proceeding:

| Fact | Value |
|------|-------|
| Exact error message | |
| Stack trace (if panic) | |
| Frame number at failure | |
| Which players affected | |
| Reproducible? (always / sometimes / rare) | |
| First frame where checksums diverge | |

### For CI Failures

```bash
# Reproduce the exact CI command
cargo fmt --check
cargo clippy --all-targets --features tokio,json -- -D warnings
cargo nextest run --no-capture
```

Check environment differences: Rust version, feature flags, OS.

---

## Phase 2: Hypothesize (Max 3 Attempts)

Form a specific, testable hypothesis. Write it down before investigating.

**Template:** "The failure occurs because [specific mechanism] in [specific location] causes [specific wrong behavior] when [specific condition]."

### Hypothesis Quality Check

- BAD: "Something is wrong with the input queue"
- GOOD: "InputQueue::add_input at src/input_queue/mod.rs:142 drops input for frame N when the queue wraps around because the circular index calculation uses `%` on a negative frame difference"

### Common Root Causes in Fortress

| Symptom | Likely Root Cause | Where to Look |
|---------|-------------------|---------------|
| `MismatchedChecksum` | Non-determinism in simulation | State save/load, float ops, HashMap |
| `InvalidFrameStructured` | Frame arithmetic overflow | `try_add`/`try_sub` call sites |
| `PredictionThreshold` | Remote too far behind | Network latency, `max_prediction` config |
| `InternalErrorStructured` | Index out of bounds | `SavedStates` circular buffer, `InputQueue` |
| Flaky test | Race condition or timing | Socket tests, async code, thread ordering |
| Desync only on rollback | State not fully saved/restored | `SaveGameState`/`LoadGameState` handlers |
| Desync after disconnect | Disconnect frame handling | `P2PSession::disconnect_player_at_frame` |

### Three-Strike Rule

After each failed hypothesis:

1. Document what you tested and what the evidence showed
2. Update your understanding of the system
3. Form the NEXT hypothesis using the new evidence

If 3 hypotheses fail, STOP. Write up what you know and escalate:

```
## Investigation Summary
### Observed behavior: [what happens]
### Expected behavior: [what should happen]
### Hypotheses tested:
1. [hypothesis] -- REJECTED because [evidence]
2. [hypothesis] -- REJECTED because [evidence]
3. [hypothesis] -- REJECTED because [evidence]
### Remaining unknowns: [what you still don't understand]
### Suggested next steps: [what to try next, with whom]
```

---

## Phase 3: Verify (Prove the Root Cause)

Before writing any fix, prove the root cause with a failing test.

```rust
#[test]
fn regression_issue_NNN_description() {
    // Arrange: set up the exact conditions from the bug
    // test: .unwrap() is idiomatic in tests
    let mut session = SessionBuilder::<TestConfig>::new()
        .with_num_players(2)
        .unwrap()
        .start_synctest_session()
        .unwrap();

    // Act: trigger the specific failure
    // ...

    // Assert: verify the bug manifests
    assert!(matches!(result, Err(FortressError::InvalidFrameStructured { .. })));
}
```

### Verification Checklist

- [ ] Failing test reproduces the exact bug
- [ ] Test fails WITHOUT the fix
- [ ] Test passes WITH the fix
- [ ] Test is specific enough to catch regressions
- [ ] Root cause is documented in the test comment

---

## Phase 4: Fix (Minimal, Correct Change)

### Fix Principles

1. **Fix at the right level.** If the bug is a missing bounds check, add the check where the invariant should hold, not where the symptom appeared.
2. **Fix one thing.** Do not refactor while fixing a bug. Separate commits.
3. **Check for similar bugs.** If you found an off-by-one in one circular buffer index, check ALL circular buffer indices.

### Post-Incident Hardening Handoff

For security/correctness-sensitive bugs, run a scoped post-incident adversarial handoff using [adversarial-handoff.md](adversarial-handoff.md) Mode 2:

1. Fix the immediate bug.
2. Scan for the same bug class in related modules.
3. Add proactive fixes and regression coverage.
4. Document hardened scope in the investigation summary.

```bash
# After finding a bug pattern, search for similar issues
rg 'pattern_that_caused_bug' --type rust src/
```

### Desync-Specific Debugging

Desyncs require bisecting the frame where state diverges:

1. Add checksums at the START of each frame (before inputs)
2. Add checksums at the END of each frame (after advance)
3. Compare checksums between peers
4. When you find the first divergent frame, add per-subsystem checksums
5. Narrow to the specific field that diverges

Common desync causes:

| Cause | Detection | Fix |
|-------|-----------|-----|
| HashMap iteration order | `rg 'HashMap' src/` | Replace with `BTreeMap` |
| Float precision | Compare `.to_bits()` | Use `libm` or fixed-point |
| Missing state in save/load | Diff saved vs loaded fields | Add missing field to snapshot |
| Input prediction mismatch | Log predicted vs actual | Fix prediction strategy |
| Uninitialized struct padding | Hash individual fields | Derive `Hash` on all fields |

---

## Anti-Patterns

| Anti-Pattern | Why It Fails | Do Instead |
|-------------|--------------|------------|
| "Try random changes" | Hides root cause, introduces new bugs | Form hypothesis first |
| "Add more logging and ship" | Logging is not a fix | Find and fix root cause |
| "It works on my machine" | Environment difference IS the bug | Reproduce in CI environment |
| "Revert and move on" | Bug still exists, will return | Understand why, add regression test |
| "The test is wrong" | Sometimes true, but prove it | Fix test OR fix code, with evidence |

---

## Checklist

- [ ] Facts gathered before hypothesizing
- [ ] Hypothesis is specific and testable
- [ ] Three-strike rule followed (escalate if stuck)
- [ ] Root cause proven with failing test
- [ ] Fix is minimal and at the right level
- [ ] Similar patterns checked across codebase
- [ ] Regression test added
- [ ] `cargo c && cargo t` passes after fix
