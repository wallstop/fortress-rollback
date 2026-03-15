## Description

<!-- Provide a clear and concise description of your changes. -->
<!-- What problem does this solve? Why is this change needed? -->

## Type of Change

<!-- Check all that apply -->

- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)
- [ ] ✨ New feature (non-breaking change that adds functionality)
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to change)
- [ ] 📚 Documentation (changes to documentation only)
- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a feature)
- [ ] 🧪 Test (adding or updating tests)
- [ ] 🔧 CI/Build (changes to CI configuration or build process)

## Checklist

<!-- Please review and check all applicable items -->

### Required

- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)
- [ ] I have followed the **zero-panic policy**:
  - No `unwrap()` in production code
  - No `expect()` in production code
  - No `panic!()` or `todo!()`
  - All fallible operations return `Result`
- [ ] I have added tests that prove my fix is effective or my feature works
- [ ] I have run `cargo fmt && cargo clippy --all-targets --features tokio,json` with no warnings
- [ ] I have run `cargo nextest run` and all tests pass

### If Applicable

- [ ] I have updated the documentation accordingly
- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes
- [ ] I have updated relevant examples in the `examples/` directory
- [ ] My changes generate no new compiler warnings

## Testing

<!-- Describe how you tested your changes -->
<!-- Include any relevant details about your testing environment -->

**Tests added/modified:**

- (None)

**Manual testing performed:**

- (None)

## Related Issues

<!-- Link any related issues using GitHub keywords -->
<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->

---

<!-- Thank you for contributing to Fortress Rollback! -->
