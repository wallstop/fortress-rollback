<!-- CATEGORY: Workflows -->
<!-- WHEN: Reviewing PRs, auditing code changes, verifying zero-panic compliance, checking determinism -->

# Code Review Guide

Structured two-pass review for fortress-rollback. Pass 1 finds correctness issues. Pass 2 finds style and improvement opportunities. Fix mechanical issues directly; flag judgment calls for discussion.

---

## Pass 1: Correctness (Blocking)

### 1A. Zero-Panic Compliance

Search the diff for forbidden patterns:

```bash
rg '\.unwrap\(\)|\.expect\(|panic!\(|todo!\(|unimplemented!\(' --type rust -- src/
```

Direct indexing (`array[i]` instead of `array.get(i)`) is caught by `clippy::indexing_slicing`, which is denied in CI safety checks. Run the strict clippy pass or review manually.

Every hit in production code (`src/`, excluding `#[cfg(test)]` blocks) is a blocking issue. Verify the fix uses structured error handling.

### Forbidden vs Required Pattern

```rust
// FORBIDDEN
let val = map.get(&key).unwrap();

// REQUIRED
let val = map.get(&key).ok_or(FortressError::InternalErrorStructured {
    kind: InternalErrorKind::Custom("key not found in map"),
})?;
```

### 1B. Determinism Violations

Check for non-deterministic patterns in any code that runs during simulation:

| Pattern | Grep | Fix |
|---------|------|-----|
| HashMap iteration | `rg 'HashMap' --type rust src/` | `BTreeMap` or sort before use |
| System time | `rg 'Instant::now\|SystemTime' --type rust src/` | Frame counters |
| Thread-local RNG | `rg 'thread_rng\|random\(\)' --type rust src/` | Seeded RNG (`rand_pcg` or `rand_chacha`) |
| Pointer-based ordering | `rg 'as \*const\|addr\(\)' --type rust src/` | Stable IDs |
| Unordered iteration | `rg '\.par_iter\(\)' --type rust src/` | Sequential or collect+sort |

### 1C. Error Handling Completeness

For every new `match` on `FortressError` or its sub-enums:

- Verify all variants are handled (no silent `_ =>` on `#[non_exhaustive]` enums that drops new cases)
- Verify structured variants are preferred over legacy string variants
- Verify `Result` values are propagated with `?`, not discarded with `let _ =`

### 1D. Enum Exhaustiveness

When a PR adds a new variant to any enum, search for all `match` sites:

```bash
rg 'match.*error|match.*kind|match.*reason|match.*state' --type rust src/
```

Every `_ =>` arm on a non-`#[non_exhaustive]` enum is suspicious. Adding a variant should cause compile errors at all match sites.

### 1E. State Consistency

For changes to `SyncLayer`, `P2PSession`, or `InputQueue`:

- Does the operation succeed completely or leave state unchanged?
- Could a partial failure corrupt the circular buffer in `SavedStates`?
- Is `Frame` arithmetic using `try_add`/`try_sub` (not raw `+`/`-`)?

### 1F. Adversarial Escalation Decision

Escalate using [adversarial-handoff.md](adversarial-handoff.md) when changes touch high-risk areas:

- Untrusted network input handling
- Frame arithmetic/rollback bounds
- Save/load state integrity
- Determinism-critical logic

If triggered, complete adversarial handoff before final merge approval.

---

## Pass 2: Quality (Non-Blocking)

### 2A. API Design

- Are new public types/functions documented with `///` and examples?
- Do new structs use `#[must_use]` where appropriate?
- Are new error variants structured (enum-based, not string-based)?
- Does the change follow the Builder pattern for configuration?

### 2B. Performance on Hot Path

Code in `advance_frame`, `SyncLayer::advance_frame`, input queues, and protocol handling is hot path:

- No allocations (no `String`, `Vec::new()`, `format!()`)
- No `clone()` where a reference suffices
- Prefer `SmallVec` over `Vec` for bounded collections

### 2C. Test Coverage

- Does the PR include tests for new functionality?
- Are error paths tested, not just happy paths?
- For bug fixes: is there a regression test that would have caught it?
- Naming convention: `what_condition_expected_behavior`

### 2D. Documentation Staleness

When public API changes, verify these are updated:

```bash
rg 'old_function_name|OldTypeName' --type rust --type md
```

Check: rustdoc, CHANGELOG.md, docs/user-guide.md, examples/.

### 2E. Design Decision Log Coverage

If the PR introduces a major architectural or behavior choice, verify a corresponding entry exists per [design-decisions.md](design-decisions.md).

---

## Fortress-Specific Checks

### Frame Arithmetic

All frame math must use checked operations:

```rust
// FORBIDDEN: can overflow silently
let next = current_frame + offset;

// REQUIRED: returns FortressError::FrameArithmeticOverflow on overflow
let next = current_frame.try_add(offset)?;
```

### Serialization Compatibility

Changes to `Message`, network protocol types (`InputBytes`, `codec`), or serialized types risk wire-protocol breakage. Verify:

- Serde attributes are preserved
- New fields have defaults for backward compatibility
- Wire format tests exist

### Config Validation

New `SessionBuilder` parameters must validate at build time:

```rust
// REQUIRED: validate in builder, not at runtime
if fps == 0 {
    return Err(InvalidRequestKind::ZeroFps.into());
}
```

---

## Review Output Format

Structure findings as:

```
## Critical (must fix before merge)
- [file:line] Description citing specific code

## Suggestions (non-blocking)
- [file:line] Description with proposed alternative

## Verified
- Zero-panic compliance: PASS/FAIL (N hits found)
- Determinism: PASS/FAIL
- Error handling: PASS/FAIL
- Tests included: YES/NO
- Design decision log: YES/NO/N/A
- Adversarial escalation required: YES/NO
```

---

## Checklist

- [ ] No `unwrap`/`expect`/`panic!`/`todo!` in production code
- [ ] No `HashMap` iteration in simulation paths
- [ ] No `Instant::now()` in game logic
- [ ] All `match` arms exhaustive (no silent `_ =>`)
- [ ] Frame arithmetic uses checked operations
- [ ] New public items have rustdoc with examples
- [ ] Error variants are structured, not string-based
- [ ] Tests cover both happy path and error cases
- [ ] `cargo doc --no-deps` passes (no broken doc links)
- [ ] CHANGELOG updated if pub/user-observable change
- [ ] `cargo c && cargo t` passes
