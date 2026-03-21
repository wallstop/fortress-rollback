<!-- CATEGORY: Workflows -->
<!-- WHEN: Architectural decisions, design alternatives, superseding prior choices -->

# Design Decision Log Pattern

Use a lightweight, append-only decision log for major design choices. Keep it short, searchable, and linked to code and PR context.

---

## Location

Store logs under `.llm/design-history/` as plain text files by domain, for example:

- `.llm/design-history/network.txt`
- `.llm/design-history/sync-layer.txt`

---

## Entry Format

```text
YYYY-MM-DD | Decision | Supersedes | Rationale | Evidence | PR/Issue
```

Example:

```text
2026-03-20 | Use checked frame arithmetic in rollback paths | None | Prevent overflow-triggered desync | src/lib.rs Frame::try_add/try_sub | PR #123
```

---

## When to Log

- Public API behavior choices
- Determinism-sensitive algorithm choices
- Safety/reliability guardrail choices
- Performance trade-offs on hot paths
- Replacements of prior architectural patterns

## When Not to Log

- Trivial refactors
- Test-only maintenance changes
- Formatting/lint-only changes
- Non-behavioral comment updates

---

## Supersedes Chains

If replacing prior guidance or behavior, set `Supersedes` to the prior entry date/title.

```text
2026-03-20 | Switch queue iteration to stable order | 2025-11-02 HashMap iteration in queue processing | Determinism | src/input_queue/mod.rs | PR #456
```

---

## Review Integration

During review:

- Confirm major design-impacting PRs include a decision log entry.
- Confirm supersedes links are correct when old patterns are replaced.
- Reject vague rationale with no code evidence.

---

## Anti-Patterns

- Do not write essays. One line per decision.
- Do not create logs for every small change.
- Do not omit evidence links (file/symbol/PR).