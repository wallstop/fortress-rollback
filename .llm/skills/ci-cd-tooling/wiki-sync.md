<!-- CATEGORY: CI/CD & Tooling -->
<!-- WHEN: Wiki sync, documentation links, markdown anchor validation -->
# Documentation Sync Guide

## Wiki Files Are Generated -- Never Edit Directly

All `wiki/*.md` files come from `docs/` via `scripts/docs/sync-wiki.py`. Manual edits are overwritten.

```
docs/user-guide.md  --sync-wiki.py-->  wiki/User-Guide.md
docs/index.md       --sync-wiki.py-->  wiki/Home.md
```

Fix content in `docs/`, then re-sync.

## Link Format Rules

| Context             | Format                   | Example                                               |
| ------------------- | ------------------------ | ----------------------------------------------------- |
| `docs/` files       | Lowercase with `.md`     | `[Guide](user-guide.md)`                              |
| `wiki/` files       | PascalCase, no extension | `[Guide](User-Guide)`                                 |
| `wiki/_Sidebar.md`  | PascalCase, no extension | `[User Guide](User-Guide)`                            |
| External (non-docs) | Full GitHub URL          | `[Code](https://github.com/.../blob/main/src/lib.rs)` |

Use standard markdown `[Text](Page)` syntax in sidebar -- NOT wiki-link `[[Page|Text]]` syntax (has URL generation bugs).

## WIKI_STRUCTURE Mapping

```python
WIKI_STRUCTURE = {
    "index.md": "Home",
    "user-guide.md": "User-Guide",
    "architecture.md": "Architecture",
    "specs/formal-spec.md": "Formal-Specification",
}
```

When adding a new docs/ page:
1. Create `docs/new-page.md`
2. Add `"new-page.md": "New-Page"` to `WIKI_STRUCTURE`
3. Add sidebar entry in `generate_sidebar()`
4. Run `python3 scripts/docs/sync-wiki.py`
5. Run `python3 scripts/docs/check-wiki-consistency.py`

## Sync Pipeline

```
docs/*.md --> sync-wiki.py
  +-- Strip MkDocs frontmatter
  +-- Convert admonitions (!!! -> blockquotes)
  +-- Convert tabbed content (=== -> headings)
  +-- Remove Material icons (:material-*:, :octicons-*:)
  +-- Remove grid cards (<div class="grid cards">)
  +-- Convert links (*.md -> PascalCase extensionless)
  +-- Adjust asset paths (../assets/ -> assets/)
  +-- Convert external links (../ escaping docs -> GitHub URLs)
  +-- Add SYNC comment header
  +-- Generate _Sidebar.md
--> wiki/*.md

Pre-commit enforcement now runs this sequence for docs/wiki changes:

1. `sync-wiki` regenerates `wiki/*.md` from `docs/`
2. `wiki-consistency` validates links/sidebar/mapping integrity
3. `check-sync-headers` validates reciprocal `<!-- SYNC: ... -->` headers

This prevents committing stale or manually-edited wiki mirrors.
```

## MkDocs Conversion Patterns

### Admonitions

```python
content = re.sub(
    r'^!!! (\w+)(?: "([^"]*)")?\s*$',
    lambda m: f"> **{m.group(2) or m.group(1).title()}**\n>",
    content, flags=re.MULTILINE,
)
```

### Material Icons (include `\s*` to prevent malformed links)

```python
content = re.sub(r":material-[a-z0-9-]+:\s*", "", content)
content = re.sub(r":octicons-[a-z0-9-]+:\s*", "", content)
content = re.sub(r":fontawesome-[a-z0-9-]+:\s*", "", content)
```

### Tabbed Content

```python
content = re.sub(r'^=== "([^"]+)"$', r"### \1", content, flags=re.MULTILINE)
```

## Wiki Page Naming Rules

Avoid special characters: `--` (double hyphen), `+`, spaces, `@`, `#`, `&`.

```python
# GOOD
"tlaplus-tooling.md": "TLAplus-Tooling"
# BAD
"tla-tooling.md": "TLA--Tooling"   # Double hyphen breaks links
```

### Wiki-Link Display Text Characters (if using `[[Page|Text]]`)

`+` in display text corrupts URLs. Use standard markdown links instead:

```markdown
<!-- BROKEN: '+' corrupts URL -->
- [[TLAplus-Tooling-Research|TLA+ Tooling Research]]

<!-- CORRECT: standard markdown link -->
- [TLA Plus Tooling Research](TLAplus-Tooling-Research)
```

## Validation Tools

```bash
python3 scripts/docs/sync-wiki.py                 # Generate wiki
python3 scripts/docs/check-wiki-consistency.py    # Cross-validate pipeline
python3 scripts/docs/validate-wiki-output.py      # Check rendering issues
./scripts/docs/check-links.sh                     # Link validation
```

### What check-wiki-consistency.py Validates

1. Sidebar links point to existing wiki pages
2. All `docs/*.md` files are mapped in `WIKI_STRUCTURE`
3. All wiki pages have sidebar entries
4. No special characters in wiki-link display text

`sync-wiki.py` also validates that every `WIKI_STRUCTURE` page appears in
the generated sidebar and fails fast if any mapped page is missing.

`sync-wiki.py` now enforces deterministic writer normalization for generated
markdown: non-empty outputs end with exactly one LF newline (trailing
whitespace/newlines are normalized). This prevents churn with
`end-of-file-fixer` and keeps repeated sync runs idempotent.

Sidebar coverage validation runs before any wiki writes, so an invalid sidebar
template fails early without leaving partial regenerated output.

### Common Errors

| Error                            | Fix                          |
| -------------------------------- | ---------------------------- |
| Link points to non-existent page | Add file or fix sidebar link |
| `docs/file.md` not mapped        | Add to `WIKI_STRUCTURE`      |
| Wiki page has no sidebar entry   | Add to `generate_sidebar()`  |
| Stale WIKI_STRUCTURE mapping     | Remove entry                 |

## Markdown Link Validation

### Relative Path Resolution

Links resolve from the directory containing the markdown file:

| From                      | To Root     | Example                         |
| ------------------------- | ----------- | ------------------------------- |
| `docs/`                   | `../`       | `[README](../README.md)`        |
| `.github/`                | `../`       | `[Context](../.llm/context.md)` |
| `.llm/skills/<category>/` | `../../../` | `[README](../../../README.md)`  |

### Heading Anchor Generation Rules

1. Lowercase all letters
2. Spaces become hyphens
3. Remove `()`, `[]`, most punctuation
4. ` / ` becomes `--`
5. `~` removed

| Heading                              | Anchor                         |
| ------------------------------------ | ------------------------------ |
| `## Quick Start`                     | `#quick-start`                 |
| `## LAN / Local Network (~20ms RTT)` | `#lan--local-network-20ms-rtt` |
| `## Web / WASM Integration`          | `#web--wasm-integration`       |

### Pipe Escaping in Tables

Pipe `|` inside backticks in table cells MUST be escaped as `\|` -- the table parser runs before inline code:

```markdown
<!-- WRONG -->
| `[[Page|Text]]` | Wiki syntax |

<!-- CORRECT -->
| `[[Page\|Text]]` | Wiki syntax |
```

### Validation Commands

```bash
./scripts/docs/check-links.sh                                    # All links
lychee --config .lychee.toml docs/user-guide.md             # Single file
npx markdownlint 'docs/file.md' --config .markdownlint.json # Lint check
```

## Debugging

If sidebar link 404s: check file exists in `wiki/`, verify link name matches filename exactly, check for special characters.

If MkDocs syntax visible in wiki: add regex patterns to `strip_mkdocs_features()`.

If internal links broken: verify `WIKI_STRUCTURE` mapping, check `convert_links()`.
