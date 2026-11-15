# Rulesets

`main-protection.json` is the live, enforced main-branch policy. It requires
the `Verify prepared release state` and `CI Gate` checks and is synchronized by
`scripts/release/main_ruleset.py`.

`merge-queue.json` is the prepared merge-queue policy. GitHub only accepts the
`merge_queue` rule for repositories owned by an organization, so it cannot be
applied to the current personal-account repository. Apply it after transferring
the repository to an organization:

```bash
gh api -X POST repos/OWNER/fortress-rollback/rulesets \
  --input .github/rulesets/merge-queue.json
```

The required workflows already run on `merge_group`, so no workflow changes are
needed after the rule is applied.
