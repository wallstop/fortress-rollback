# Rulesets

`main-protection.json` is the live, enforced main-branch policy. It requires
the `Verify prepared release state` and `CI Gate` checks and is synchronized by
`scripts/release/main_ruleset.py`.

`merge-queue.json` is the live merge-queue policy, applied to
`Ambiguous-Interactive/fortress-rollback` as ruleset `27289856`. It uses squash
merges, `ALLGREEN` grouping, five concurrent builds, and a 60-minute required
check timeout.

To recreate it on another organization-owned repository:

```bash
gh api -X POST repos/OWNER/fortress-rollback/rulesets \
  --input .github/rulesets/merge-queue.json
```

The required workflows already run on `merge_group`, so no workflow changes are
needed after the rule is applied.
