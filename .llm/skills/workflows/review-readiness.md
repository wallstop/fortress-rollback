<!-- CATEGORY: Workflows -->
<!-- WHEN: Before opening PRs, self-review, merge readiness checks -->

# Review Readiness Checklist

Concrete gate between implementation and external review. Use this after `dev-pipeline.md` Phase 4 and before opening a PR.

---

## Quick Gate (Must Pass)

- [ ] `cargo c && cargo t` passes
- [ ] Zero-panic scan has no new production hits
- [ ] Determinism scan has no risky additions in simulation paths
- [ ] Error handling is explicit (`Result`, no silent discard)
- [ ] Tests cover happy + error paths for changed behavior
- [ ] Design decision log reviewed for major architecture choices
- [ ] CHANGELOG decision applied for user-observable/public changes

If two or more checks fail, return to design and reduce scope before requesting review.

---

## Commands

```bash
# Build + tests
cargo c && cargo t

# Zero-panic scan (production code)
rg '\.unwrap\(\)|\.expect\(|panic!\(|todo!\(|unimplemented!\(' --type rust src/

# Determinism scan
rg 'HashMap|HashSet|Instant::now|SystemTime|thread_rng|random\(\)' --type rust src/
```

---

## Review Readiness Output

Use this in PR descriptions or self-review comments:

```text
Review Readiness
- Build/tests: PASS|FAIL
- Zero-panic: PASS|FAIL
- Determinism: PASS|FAIL
- Error handling: PASS|FAIL
- Tests breadth: PASS|FAIL
- Design log reviewed: YES|NO|N/A
- CHANGELOG reviewed: YES|NO|N/A
```

---

## Anti-Patterns

- Do not replace this checklist with prose summaries only.
- Do not mark PASS without running the checks.
- Do not treat this as a style checklist; this is correctness-first.
- Do not open PRs with known blocking failures from this gate.
