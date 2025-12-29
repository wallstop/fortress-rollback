# GitHub Wiki Sync Best Practices

> **This document covers best practices for synchronizing documentation from a MkDocs-based `docs/` directory to GitHub Wiki format.**

## Overview

GitHub Wiki has different rendering capabilities than MkDocs Material. When syncing documentation, several transformations are needed to ensure content displays correctly.

---

## Wiki Page Naming Rules

### Critical: Avoid Special Characters

GitHub Wiki has special handling for certain characters in page names:

| Character | Issue | Solution |
|-----------|-------|----------|
| `--` (double hyphen) | May be URL-encoded differently, causing broken links | Use single hyphen or alternative separator |
| `+` | URL-encoded as `%2B`, may not resolve correctly | Spell out (e.g., `TLAplus` instead of `TLA+`) |
| Spaces | Converted to hyphens in URLs | Use hyphens explicitly |
| Special chars (`@`, `#`, `&`) | May break URL resolution | Avoid entirely |

### Naming Convention

```python
# ✅ GOOD: Simple, hyphenated names
WIKI_STRUCTURE = {
    "user-guide.md": "User-Guide",
    "api-contracts.md": "API-Contracts",
    "tlaplus-tooling.md": "TLAplus-Tooling",  # Spelled out, no +
}

# ❌ BAD: Special characters that cause issues
WIKI_STRUCTURE = {
    "tla-tooling.md": "TLA--Tooling",      # Double hyphen breaks links
    "c++-guide.md": "C++-Guide",           # Plus sign problematic
}
```

### Sidebar Link Consistency

**Critical:** Sidebar link names MUST exactly match the wiki page filenames (without `.md` extension).

```markdown
<!-- ✅ CORRECT: Link matches filename -->
- [[TLAplus-Tooling|TLA+ Tooling]]  <!-- Links to TLAplus-Tooling.md -->

<!-- ❌ BROKEN: Link doesn't match any file -->
- [[TLA--Tooling|TLA+ Tooling]]     <!-- TLA--Tooling.md doesn't exist -->
```

---

## MkDocs Feature Conversion

### Tabbed Content (`===`)

MkDocs Material's tabbed content syntax doesn't render on GitHub Wiki.

**Input (MkDocs):**

```markdown
=== "Cargo.toml"

    ```toml
    [dependencies]
    my_crate = "1.0"
    ```

=== "main.rs"

    ```rust
    fn main() {}
    ```
```

**Converted (GitHub Wiki):**

```markdown
### Cargo.toml

    ```toml
    [dependencies]
    my_crate = "1.0"
    ```

### main.rs

    ```rust
    fn main() {}
    ```
```

**Conversion regex:**

```python
content = re.sub(r'^=== "([^"]+)"$', r"### \1", content, flags=re.MULTILINE)
```

### Admonitions (`!!!`)

MkDocs admonitions need conversion to blockquotes.

**Input (MkDocs):**

```markdown
!!! warning "Be Careful"

    This is important content.
```

**Converted (GitHub Wiki):**

```markdown
> **Be Careful**
>
> This is important content.
```

**Conversion regex:**

```python
content = re.sub(
    r'^!!! (\w+)(?: "([^"]*)")?\s*$',
    lambda m: f"> **{m.group(2) or m.group(1).title()}**\n>",
    content,
    flags=re.MULTILINE,
)
```

### Material Icons

Remove Material/Octicons/FontAwesome icon shortcodes:

```python
content = re.sub(r":material-[a-z-]+:", "", content)
content = re.sub(r":octicons-[a-z-]+:", "", content)
content = re.sub(r":fontawesome-[a-z-]+:", "", content)
```

### Attribute Annotations

Remove `{ .class #id }` style annotations:

```python
# Only remove class/id selectors, preserve {variable} placeholders
content = re.sub(r"\{\s*[.#][^}]*\}", "", content)
```

### Grid Cards

MkDocs Material grid cards need complete removal (they're complex nested divs):

```python
def remove_grid_cards_divs(content: str) -> str:
    """Remove Material grid cards divs with proper nesting support."""
    # Track div depth to handle nested divs correctly
    # See sync-wiki.py for full implementation
```

---

## Asset Path Handling

### Relative Paths

Wiki pages are at the wiki root, so asset paths need adjustment:

**Input (from `docs/user-guide.md`):**

```markdown
![Logo](../assets/logo.svg)
```

**Converted:**

```markdown
![Logo](assets/logo.svg)
```

### HTML Image Tags

Same conversion for `<img>` tags:

```html
<!-- Input -->
<img src="../assets/logo.svg" alt="Logo">

<!-- Converted -->
<img src="assets/logo.svg" alt="Logo">
```

---

## External Link Handling

Links that escape the `docs/` directory should be converted to full GitHub URLs:

**Input (from `docs/index.md`):**

```markdown
[Changelog](../CHANGELOG.md)
```

**Converted:**

```markdown
[Changelog](https://github.com/owner/repo/blob/main/CHANGELOG.md)
```

---

## Debugging Wiki Sync Issues

### 1. Page Doesn't Exist (404 / Redirect to Home)

**Symptoms:**

- Clicking sidebar link shows Home page content
- URL in browser shows `/wiki/` instead of `/wiki/Page-Name`

**Diagnosis:**

1. Check if the file exists in the wiki directory
2. Verify sidebar link name matches filename exactly
3. Check for special characters in the filename

**Fix:**

```bash
# List generated wiki files
ls -la wiki/

# Verify sidebar links match
grep -o '\[\[[^]]*\]\]' wiki/_Sidebar.md | sort | uniq
```

### 2. Content Not Rendering

**Symptoms:**

- Raw MkDocs syntax visible (e.g., `=== "Tab"`)
- Icons showing as `:material-icon:`

**Diagnosis:**

- MkDocs feature stripping is incomplete

**Fix:**

- Add regex patterns to `strip_mkdocs_features()` for the unhandled syntax

### 3. Broken Internal Links

**Symptoms:**

- Links between wiki pages don't work
- 404 when clicking cross-references

**Diagnosis:**

- Link conversion isn't mapping to correct wiki page names

**Fix:**

- Verify `WIKI_STRUCTURE` mapping covers all source files
- Check `convert_links()` function handles the link format

---

## Testing Wiki Sync Locally

```bash
# Generate wiki files locally
python scripts/sync-wiki.py

# List generated files
ls -la wiki/

# Verify sidebar
cat wiki/_Sidebar.md

# Check specific page content
cat wiki/Page-Name.md

# Dry run (no changes)
python scripts/sync-wiki.py --dry-run
```

---

## CI/CD Workflow Considerations

### Workflow Triggers

Sync wiki when docs change:

```yaml
on:
  push:
    branches: [main]
    paths:
      - "docs/**"
      - "scripts/sync-wiki.py"
```

### Preventing Race Conditions

Use concurrency controls:

```yaml
concurrency:
  group: wiki-sync
  cancel-in-progress: false  # Don't cancel in-progress syncs
```

### Wiki Repository Access

The workflow needs write access to the wiki repository:

```yaml
permissions:
  contents: write

# Clone wiki repo
git clone "https://github.com/${{ github.repository }}.wiki.git" wiki
```

---

## Common Pitfalls

1. **Forgetting to update sidebar when adding pages** — Always update both `WIKI_STRUCTURE` and `generate_sidebar()` together

2. **Using double hyphens in page names** — GitHub Wiki may not resolve these correctly

3. **Not testing locally before pushing** — Always run `python scripts/sync-wiki.py` locally first

4. **Inconsistent naming between mapping and sidebar** — Keep them in sync!

5. **Forgetting about nested MkDocs features** — Grid cards, nested admonitions, etc. need special handling

---

## Quick Reference

### Files to Update When Adding Wiki Pages

1. `scripts/sync-wiki.py`:
   - `WIKI_STRUCTURE` dict — Add mapping from source file to wiki name
   - `generate_sidebar()` — Add link to sidebar navigation

2. `docs/` source file — Create the actual documentation

### Verification Checklist

- [ ] Page name uses only single hyphens and alphanumeric characters
- [ ] `WIKI_STRUCTURE` has mapping for source file
- [ ] Sidebar link matches wiki page filename exactly
- [ ] Local `python scripts/sync-wiki.py` succeeds
- [ ] Local `python scripts/check-wiki-consistency.py` passes
- [ ] Generated wiki files have expected content
- [ ] MkDocs-specific syntax is converted/stripped

---

## Wiki Consistency Validation

The `scripts/check-wiki-consistency.py` script validates wiki integrity:

### What It Validates

1. **Sidebar Link Validity** — All `[[Page|Text]]` links in `_Sidebar.md` point to existing `.md` files
2. **WIKI_STRUCTURE Completeness** — All `docs/*.md` files have mappings in `sync-wiki.py`
3. **Sidebar Completeness** — All wiki pages have corresponding sidebar entries

### Running the Validation

```bash
# Basic validation
python scripts/check-wiki-consistency.py

# Verbose output
python scripts/check-wiki-consistency.py --verbose
```

### CI/CD Integration

Wiki consistency is validated in:

- **Pre-commit hook** — `wiki-consistency` hook runs on wiki/docs changes
- **CI workflow** — `ci-docs.yml` has a `wiki-consistency` job
- **Wiki sync workflow** — `wiki-sync.yml` validates before syncing

### Common Errors and Fixes

| Error | Cause | Fix |
|-------|-------|-----|
| `Link [[Page\|Text]] points to non-existent page` | Sidebar references missing wiki file | Add file or fix sidebar link |
| `docs/file.md is not mapped in WIKI_STRUCTURE` | New docs file not in mapping | Add to `WIKI_STRUCTURE` dict |
| `Wiki page has no entry in _Sidebar.md` | Page generated but not in navigation | Add to `generate_sidebar()` |
| `WIKI_STRUCTURE maps 'file' but file doesn't exist` | Stale mapping | Remove from `WIKI_STRUCTURE` |

### Adding a New Wiki Page

1. Create source file: `docs/new-page.md`
2. Add to `WIKI_STRUCTURE` in `scripts/sync-wiki.py`:
   ```python
   "new-page.md": "New-Page",
   ```
3. Add sidebar entry in `generate_sidebar()`:
   ```python
   "- [[New-Page|New Page]]",
   ```
4. Run validation: `python scripts/check-wiki-consistency.py`
5. Sync locally: `python scripts/sync-wiki.py`

---

*See also: [scripts/sync-wiki.py](../../scripts/sync-wiki.py), [scripts/check-wiki-consistency.py](../../scripts/check-wiki-consistency.py)*
