<!-- SYNC: This source doc syncs to wiki/Public-API-Census.md. -->

# Public API Census

Fortress Rollback checks its complete callable Rust surface, including items that normal rustdoc
hides. The checked snapshot is [`public-api-snapshot.tsv`](public-api-snapshot.tsv); reviewed
removals remain in [`public-api-removals.tsv`](public-api-removals.tsv).

## Why rendered rustdoc is not the census

Rust treats `#[doc(hidden)]` as a presentation choice. A public hidden module, a public item below
that module, and a root re-export from a private module remain callable and semver-relevant. An
inherent method can likewise remain callable even when rustdoc omits its canonical path.

The census generates rustdoc JSON with private and hidden records available, then follows only
externally reachable edges from the crate root:

- public modules and their public children;
- explicit and glob public re-exports;
- public inherent methods and associated constants;
- trait requirements and associated types;
- enum variants and public data-shape fields; and
- public dependency re-exports.

Private records are present only to resolve re-export targets. They do not enter the snapshot unless
a public root-to-item path exists.

Local re-export aliases expand their reachable associated items, fields, and variants. External
dependency exports are recorded as dependency-coupled boundary rows because rustdoc JSON does not
embed the upstream crate's item graph; `Cargo.lock`, dependency policy, and cargo-semver-checks
govern that upstream surface.

## Profiles and pinning

The generator uses the dated nightly in
`.github/actions/install-pinned-nightly/toolchain`. It records both `--no-default-features` and the
complete production-API feature set. `z3-verification` and `z3-verification-bundled` are omitted
because they affect verification-test dependency construction only; the generator fails if either
feature starts controlling code under `src/`.

The generator uses the locked workspace graph and removes ambient Rust/Cargo target and compiler
flags before invoking rustdoc, so a caller's shell configuration cannot silently change the
checked API profile.

The current ledger contains 2,709 public symbol rows across 2,703 textual paths. Rust permits a
field and method to share a path; the `kind` column keeps those semver-distinct items separate.
Within those rows are:

- 2,475 symbols in both profiles;
- 234 symbols available only with the complete API feature set;
- 125 retained root alias paths, whose associated items contribute to 1,425 rows with an
  `alias-of` relationship; and
- 1,082 callable symbols hidden from normal rendered documentation.

## Ledger columns

| Column | Meaning |
| --- | --- |
| `path` | Stable public path, including associated items and aliases |
| `kind` | Rust declaration kind and the disambiguator for a shared field/method path; dependency re-exports use an `external-` prefix |
| `availability` | No-default, complete-feature, or shared availability |
| `documented` | Whether normal rustdoc presents this path |
| `owner` | Responsible subsystem derived from the defining source |
| `usage-evidence` | Contract role: supported rustdoc, public data shape, implementor contract, compatibility export, or workspace verification |
| `risk` | Semver hypothesis for changes to the path |
| `disposition` | Reviewed keep category; removed paths live in the removals ledger |
| `alias-of` | Primary callable path for a duplicate or dependency export |
| `source` | Stable defining source file, without line numbers |

The generated classifications are deliberately conservative. `#[doc(hidden)]` and the
`__internal` name do not waive semver: retained hidden paths use compatibility dispositions.

## Updating the snapshot

Run the check before changing public visibility, features, re-exports, enum variants, public fields,
traits, or associated items:

```bash
python3 scripts/api/public_api_census.py --check
```

For an intentional change:

1. Run `cargo semver-checks check-release` against the release baseline.
2. Choose a compatible migration or the repository-approved version boundary.
3. Update user documentation and `CHANGELOG.md`.
4. Record removed paths and replacements in `public-api-removals.tsv`.
5. Regenerate and inspect the path diff:

   ```bash
   python3 scripts/api/public_api_census.py --write
   git diff -- docs/api/public-api-snapshot.tsv docs/api/public-api-removals.tsv
   ```

6. Run the census parser tests and the normal repository validation.

Do not hand-edit the generated snapshot. The CI census job rebuilds both profiles from the pinned
toolchain and rejects any difference.
