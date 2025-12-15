# Release Process (fortress-rollback)

1. Ensure `Cargo.toml` version is the release version.
2. Verify tests/CI are green (rust.yml) and `package-dry-run` passes.
3. Trigger publish:
   - Tag push: `git tag vX.Y.Z && git push origin vX.Y.Z` (preferred), **or**
   - Manual dispatch: GitHub Actions → "Publish Crate" → set `release_version` to match `Cargo.toml`.
4. The workflow will:
   - Validate tag vs `Cargo.toml` (for tag runs)
   - `cargo publish --dry-run`
   - `cargo publish` using `CRATES_IO_TOKEN`
5. After publish, add a corresponding GitHub Release (optional but recommended).
6. If a bad release slips through, yank it: `cargo yank --vers X.Y.Z` (keeps name reserved).

Notes:

- The crates.io token must be stored as `CRATES_IO_TOKEN` in repo secrets.
- Keep `README` badges pointing to the current workflows.
- For pre-release/testing, you can publish with a pre-release version (e.g., `0.11.1-alpha.1`).
