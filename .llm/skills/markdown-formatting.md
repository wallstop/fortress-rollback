# Markdown Formatting Standards

Guidelines for consistent markdown formatting to prevent lint errors.

## ATX Heading Rules

| Rule | Description | Fix |
|------|-------------|-----|
| MD018 | No space after `#` in heading | Add single space after `#` |
| MD019 | Multiple spaces after `#` in heading | Use exactly one space after `#` |

### Examples

```markdown
❌ WRONG:
#No space after hash
##  Two spaces after hash
###   Three spaces after hash

✅ CORRECT:
# Single space after hash
## Single space after hash
### Single space after hash
```

## Common Mistakes

| Mistake | Example | Fix |
|---------|---------|-----|
| Missing space | `#Title` | `# Title` |
| Extra spaces | `##  Title` | `## Title` |
| Trailing spaces | `## Title` | `## Title` |
| Inconsistent heading levels | Skip from `#` to `###` | Use sequential levels |

## How to Check

Run markdownlint on specific files or directories:

```bash
# Check all markdown files
npx markdownlint "**/*.md"

# Check specific file
npx markdownlint docs/index.md

# Check with config file
npx markdownlint -c .markdownlint.json "**/*.md"
```

## Auto-Fix Command

Many issues can be automatically fixed:

```bash
# Fix all markdown files
npx markdownlint --fix "**/*.md"

# Fix specific directory
npx markdownlint --fix "docs/**/*.md"

# Fix with config
npx markdownlint --fix -c .markdownlint.json "**/*.md"
```

## Pre-Commit Integration

The repository uses pre-commit hooks to catch markdown lint errors before commit. If you encounter lint failures:

1. Run `npx markdownlint --fix` on the affected files
2. Review changes to ensure correctness
3. Commit the fixed files

## Configuration

Markdown lint rules are configured in `.markdownlint.json` at the repository root. Check this file for:

- Enabled/disabled rules
- Line length limits
- Heading style preferences
- Other project-specific settings

## Link Syntax Rules

### Malformed Links to Avoid

Spaces inside link brackets cause rendering issues:

```markdown
❌ WRONG:
[ Text with leading space](url)
[Text with trailing space ](url)
[](url)  # Empty link text

✅ CORRECT:
[Text](url)
[Descriptive link text](url)
```

### Common Causes

| Issue | Cause | Fix |
|-------|-------|-----|
| `[ Text]` | Icon removal left trailing space | Update regex to include `\s*` |
| `[Text ]` | Copy-paste with trailing space | Trim link text |
| `[]` | Missing link text | Add descriptive text |

### Wiki Sync Considerations

When stripping MkDocs icons (`:octicons-*:`, `:material-*:`), ensure trailing whitespace is also removed:

```python
# ❌ Leaves space: [:octicons-arrow-right-24: Text] -> [ Text]
re.sub(r":octicons-[a-z0-9-]+:", "", content)

# ✅ Correct: [:octicons-arrow-right-24: Text] -> [Text]
re.sub(r":octicons-[a-z0-9-]+:\s*", "", content)
```

## Quick Reference

```bash
# Check before committing
npx markdownlint "**/*.md"

# Auto-fix most issues
npx markdownlint --fix "**/*.md"

# Check wiki link syntax
python3 scripts/check-wiki-consistency.py
```
