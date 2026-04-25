<!-- CATEGORY: Workflows -->
<!-- WHEN: Planning features, structuring development work, end-to-end development process -->

# Development Pipeline

Structured workflow from planning through shipping for fortress-rollback. Each phase has defined inputs, outputs, and quality gates. Skip phases only when explicitly noted.

---

## Phase 1: Scope

**Input:** Feature request, bug report, or improvement idea.
**Output:** Written scope document (can be a PR description or issue comment).

### Scope Template

```
## What
[One sentence: what changes and why]

## Why
[What problem does this solve? Who benefits?]

## Affected Components
- [ ] src/sync_layer/ (SyncLayer, SavedStates, GameStateCell)
- [ ] src/input_queue/ (InputQueue, prediction)
- [ ] src/sessions/ (P2PSession, SpectatorSession, SyncTestSession, SessionBuilder)
- [ ] src/network/ (protocol, codec, compression, messages)
- [ ] src/error.rs (FortressError, structured error types)
- [ ] Public API (new pub items, changed signatures)
- [ ] Wire protocol (message format, serialization)

## Determinism Impact
[Does this touch simulation code? Could it affect determinism?]

## Breaking Changes
[Any pub API changes? Wire protocol changes? Default value changes?]

## Size Estimate
- [ ] Small (1-2 files, <100 lines)
- [ ] Medium (3-5 files, 100-500 lines)
- [ ] Large (>5 files or >500 lines) -- consider splitting
```

### Scope Decision Rules

| Change Type           | Needs Scope Doc? | Needs CHANGELOG? |
| --------------------- | ---------------- | ---------------- |
| Bug fix (pub-visible) | Yes              | Yes              |
| Bug fix (internal)    | Yes              | No               |
| New public API        | Yes              | Yes              |
| Refactoring           | Brief            | No               |
| Dependency update     | No               | If user-visible  |
| CI/tooling            | No               | No               |
| Kani proof            | Brief            | No               |

---

## Prerequisites

Before running verification/debugging commands in this workflow, confirm:

- Rust toolchain and cargo available
- `rg` available for scan commands
- `cargo-nextest` available for `cargo nextest` commands
- Kani setup verified if running proof levels (see [kani.md](../formal-verification/kani.md))

---

## Phase 2: Design

**Input:** Scope document.
**Output:** Design decisions documented in code comments or PR description.
**Skip if:** Change is <100 lines and touches only one module.

### Design Checklist

- [ ] Read existing code in affected modules first
- [ ] Check for similar patterns already in codebase
- [ ] Consider impact on all three session types (P2P, Spectator, SyncTest)
- [ ] Consider WASM compatibility (no `std::time`, no threads)
- [ ] Consider `no_std` compatibility if applicable
- [ ] Plan the error types (structured, not string-based)
- [ ] Plan the test approach before writing code

### Design Patterns to Follow

| When You Need             | Use This Pattern   | Example in Codebase                       |
| ------------------------- | ------------------ | ----------------------------------------- |
| Configurable construction | Builder            | `SessionBuilder`                          |
| Protocol state machine    | Type-state or enum | `SessionState`, protocol states           |
| Input prediction          | Strategy           | `PredictionStrategy` trait                |
| Bounded collections       | Circular buffer    | `SavedStates`                             |
| Request/response          | Request enum       | `FortressRequest`                         |
| Error context             | Structured enum    | `InternalErrorKind`, `InvalidRequestKind` |

---

## Design Review Entrance Gate

Before moving to implementation, verify these project-specific checks:

- [ ] Determinism impact is explicit (state ordering, RNG seeding, frame-based behavior)
- [ ] Zero-panic error paths are planned (`Result`, structured errors, no panic shortcuts)
- [ ] Session impact is reviewed across P2P, Spectator, and SyncTest
- [ ] Test strategy includes more than happy path (negative/error/extreme cases)
- [ ] Existing patterns were checked to avoid duplicating abstractions

If any check fails, refine scope/design before writing code.

### Design Decision Log

If the change introduces a meaningful architecture or behavior choice, add a one-line record using [design-decisions.md](design-decisions.md) in `.llm/design-history/`.

- Log major choices (determinism, API behavior, safety/performance trade-offs)
- Skip trivial refactors and style-only changes
- Use supersedes links when replacing prior patterns

---

## Phase 3: Implement

**Input:** Design decisions.
**Output:** Working code with tests.

### Implementation Order

1. **Write the error types first** -- define what can go wrong
2. **Write the tests second** -- define expected behavior
3. **Write the implementation third** -- make tests pass
4. **Write the docs last** -- document what you built

### During Implementation

```bash
# Check frequently (after every logical change)
cargo check
cargo clippy --all-targets --features tokio,json

# Run affected tests
cargo nextest run module_name --no-capture
```

### Commit Granularity

| Good Commit                                                  | Bad Commit        |
| ------------------------------------------------------------ | ----------------- |
| "Add `InputDelayTooLarge` variant to `InvalidRequestKind`"   | "Various changes" |
| "Validate input delay in `SessionBuilder::with_input_delay`" | "Fix stuff"       |
| "Add regression test for issue #42"                          | "WIP"             |

Each commit should pass `cargo c && cargo t` independently.

---

## Phase 4: Self-Review

**Input:** Complete implementation.
**Output:** Reviewed diff ready for external review.
**Do this BEFORE opening a PR.**

Run the readiness gate from [review-readiness.md](review-readiness.md), then perform targeted checks from [code-review.md](code-review.md):

```bash
# Zero-panic scan
rg '\.unwrap\(\)|\.expect\(|panic!\(|todo!\(' --type rust src/

# Determinism scan
rg 'HashMap|HashSet|Instant::now|thread_rng' --type rust src/

# Agent preflight (catches version sync/.llm/workflow issues early)
python3 scripts/ci/agent-preflight.py --auto-fix

# Full quality gate (see context.md "Mandatory Linting" for details)
cargo c && cargo t
cargo doc --no-deps
typos
```

All readiness checks should pass before opening a PR.

### Self-Review Questions

1. If I came to this code in 6 months, would I understand why?
2. Is there a simpler way to achieve the same result?
3. Did I test the error paths, not just the happy path?
4. Could this change cause a desync in a running game?
5. Would a user need to change their code because of this?

---

## Phase 5: Verification

**Input:** Self-reviewed code.
**Output:** All verification passes.

### Verification Levels (run in order, stop at first failure)

```bash
# Level 1: Format + lint + unit tests (always)
cargo c && cargo t

# Level 2: Doc tests (if docs changed)
cargo test --doc -- --nocapture

# Level 3: Kani proofs (if affected module has proofs; see [kani.md](../formal-verification/kani.md) for setup)
./scripts/verification/verify-kani.sh --tier 1 --quick

# Level 4: Z3 proofs (if math/bounds changed)
cargo test --features z3-verification -- --nocapture

# Level 5: Miri (if unsafe-adjacent patterns changed)
# Runs in CI automatically

# Level 6: Full CI (let CI run it)
# Cross-compilation, mutation testing, chaos testing
```

### Kani Proof Obligations

If your change affects code covered by a Kani proof:

1. Run the affected proof: `cargo kani --harness proof_name`
2. If the proof fails, fix the code OR update the proof (with justification)
3. If you add new invariants, add new proofs
4. Register new proofs in `scripts/verification/verify-kani.sh`

---

## Phase 6: Ship

**Input:** Passing CI, approved review.
**Output:** Merged PR.

### Pre-Merge Checklist

- [ ] All CI checks pass
- [ ] CHANGELOG updated (if pub/user-observable)
- [ ] No TODO/FIXME left in new code without tracking issue
- [ ] Commit messages are descriptive
- [ ] PR description explains the "why"

### Post-Merge

- Monitor CI on main branch
- If the change is user-facing, consider updating wiki/docs
- If the change is a breaking change, plan the next release

---

## Emergency Fixes

For production-blocking bugs, compress the pipeline:

1. **Scope:** One-line description in the PR title
2. **Skip design phase** (but still read existing code)
3. **Implement:** Fix + regression test in one commit
4. **Self-review:** Run `cargo c && cargo t` only
5. **Ship:** Get one reviewer, merge quickly
6. **Follow up:** Add docs, additional tests, and related fixes as separate PRs

---

## Anti-Patterns

| Anti-Pattern             | Why It Hurts                       | Do Instead                 |
| ------------------------ | ---------------------------------- | -------------------------- |
| Code first, think later  | Rework, wrong abstraction          | Scope and design first     |
| Skip tests               | Bugs ship, regression debt         | Write tests before code    |
| Mega-PR (>500 lines)     | Hard to review, risky merge        | Split into stacked PRs     |
| Fix + refactor in one PR | Hard to review, bisect-breaking    | Separate commits/PRs       |
| Skip self-review         | Obvious issues waste reviewer time | Review your own diff first |
