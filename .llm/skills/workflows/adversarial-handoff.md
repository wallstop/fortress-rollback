<!-- CATEGORY: Workflows -->
<!-- WHEN: High-risk reviews, post-incident hardening, escalation from code review -->

# Adversarial Handoff Workflow

Use this workflow to escalate high-risk changes from standard review into focused adversarial analysis, then feed fixes back into normal review and investigation.

---

## Trigger Modes

## Mode 1: Pre-Merge High-Risk PR

Trigger when changes touch:

- Untrusted network input handling
- Frame arithmetic / rollback bounds
- Save/load state integrity
- Determinism-critical logic

Action:

1. Reviewer flags adversarial handoff in PR.
2. Run [adversarial-review.md](adversarial-review.md) on scoped files.
3. Capture findings with severity + code pointers.
4. Author fixes and adds regression tests.
5. Reviewer verifies closure before merge.

## Mode 2: Post-Incident Hardening

Trigger after root-cause fix is identified in investigation.

Action:

1. Fix immediate bug.
2. Use adversarial lens to search for same bug class elsewhere.
3. Add proactive fixes and regression coverage.
4. Record hardened scope in investigation output.

## Mode 3: Planned Deep Audit

Periodic targeted review of a hotspot module. Non-blocking for in-flight PRs unless Critical/High risk is found.

---

## Handoff Template

```text
Adversarial Handoff
Mode: Pre-Merge | Post-Incident | Planned
Scope:
- path/to/file.rs: concern
- path/to/file.rs: concern
Questions:
1. Can untrusted input break invariants here?
2. Can this path desync peers under extreme values?
3. What fails closed vs fails open?
Exit Criteria:
- Critical/High findings resolved
- Regression tests added
- Reviewer verification complete
```

---

## Integration Rules

- `code-review.md` decides whether escalation is required.
- [adversarial-review.md](adversarial-review.md) performs red-team-style challenge.
- `investigation.md` consumes findings for post-incident pattern scans.

---

## Anti-Patterns

- Do not escalate every PR.
- Do not run adversarial review without scoped questions.
- Do not close handoff without regression coverage.
- Do not merge High/Critical findings without explicit acceptance.