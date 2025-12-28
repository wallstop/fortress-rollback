# Markdown Link Validation

> **Critical:** Broken links in markdown files will fail CI. Always validate links before committing.

## Why Link Validation Matters

1. **CI enforces link validity** — The `check-links.sh` script runs in CI and fails on any broken link
2. **Documentation integrity** — Broken links frustrate users and make docs unusable
3. **Cross-references break silently** — File moves/renames can break links in other files

## Validation Command

**Run after ANY markdown file modification:**

```bash
./scripts/check-links.sh
```

This script:

- Scans all `.md` files in the repository
- Validates all internal links (file paths, anchors)
- Reports broken links with file location and line number

## Understanding Relative Paths

Relative links are resolved **from the directory containing the markdown file**, not from the repository root.

### Path Resolution Rules

| File Location | Link Target | Correct Link |
|---------------|-------------|--------------|
| `README.md` (root) | `docs/user-guide.md` | `[Guide]` + `(docs/user-guide.md)` |
| `docs/user-guide.md` | `README.md` (root) | `[README]` + `(../README.md)` |
| `.github/copilot-instructions.md` | `.llm/context.md` | `[Context]` + `(../.llm/context.md)` |
| `.llm/context.md` | `.llm/skills/defensive-programming.md` | `[Defensive]` + `(skills/defensive-programming.md)` |
| `.llm/skills/defensive-programming.md` | `.llm/context.md` | `[Context]` + `(../context.md)` |

### Key Insight: The `..` Prefix

When linking to files **outside your current directory**, you must use `../` to navigate up:

```text
From .github/copilot-instructions.md:

✅ CORRECT: [Context] + (../.llm/context.md)     ← Go up to root, then into .llm/
❌ WRONG:   [Context] + (.llm/context.md)        ← Looks for .github/.llm/context.md

✅ CORRECT: [README] + (../README.md)            ← Go up to root
❌ WRONG:   [README] + (README.md)               ← Looks for .github/README.md
```

## Common Mistakes and Fixes

### Mistake 1: Missing `../` from Subdirectories

```text
File: .github/CONTRIBUTING.md

❌ WRONG:
[See the docs] + (docs/architecture.md)
Resolves to: .github/docs/architecture.md (doesn't exist)

✅ CORRECT:
[See the docs] + (../docs/architecture.md)
Resolves to: docs/architecture.md
```

### Mistake 2: Including Current Directory in Path

```text
File: .llm/context.md

❌ WRONG:
[Defensive Programming] + (.llm/skills/defensive-programming.md)
Resolves to: .llm/.llm/skills/defensive-programming.md

✅ CORRECT:
[Defensive Programming] + (skills/defensive-programming.md)
Resolves to: .llm/skills/defensive-programming.md
```

### Mistake 3: Using Absolute Paths

```text
Absolute paths don't work reliably in Git-hosted markdown

❌ WRONG:
[Guide] + (/docs/guide.md)
GitHub renders this, but local tools may not

✅ CORRECT:
[Guide] + (../docs/guide.md)
Works everywhere
```

### Mistake 4: Broken Anchor Links

```text
❌ WRONG:
[Section] + (#section-that-doesnt-exist)

✅ CORRECT:
[Section] + (#actual-heading-text)
Anchors are lowercase, spaces become hyphens
```

---

## Heading Anchor Generation Rules

Markdown heading anchors are auto-generated with specific transformation rules. **MD051 (link-fragments) errors occur when your `#fragment` doesn't match the actual generated anchor.**

### Transformation Rules

1. **Lowercase** — All letters converted to lowercase
2. **Spaces → hyphens** — Spaces become single hyphens (`-`)
3. **Remove special chars** — Parentheses `()`, brackets `[]`, and most punctuation removed
4. **Slashes with spaces → double hyphens** — ` / ` becomes `--`
5. **Tildes removed** — `~` characters are stripped

### Examples

| Heading | Generated Anchor |
|---------|------------------|
| `## Quick Start` | `#quick-start` |
| `## LAN / Local Network (~20ms RTT)` | `#lan--local-network-20ms-rtt` |
| `## Web / WASM Integration` | `#web--wasm-integration` |
| `## P2P Session (peer-to-peer)` | `#p2p-session-peer-to-peer` |
| `## Advanced Configuration [Optional]` | `#advanced-configuration-optional` |

### Common MD051 Mistakes

```text
❌ WRONG: #lan-/-local-network   ← Slash not converted correctly
✅ RIGHT: #lan--local-network-20ms-rtt

❌ WRONG: #web-wasm-integration  ← Missing double hyphen for slash
✅ RIGHT: #web--wasm-integration

❌ WRONG: #quick-start-(guide)   ← Parens should be removed
✅ RIGHT: #quick-start-guide
```

### Verify Before Committing

**Always verify fragment links match actual headings:**

```bash
# Check for MD051 errors
npx markdownlint 'docs/user-guide.md' --config .markdownlint.json

# Search for a heading to see its exact text
grep -n "^##" docs/user-guide.md | grep -i "network"
```

## Directory Reference

Quick reference for common link patterns in this repository:

| From Directory | To Repo Root | Example Syntax |
|----------------|--------------|----------------|
| Root (`/`) | (already there) | `[File]` + `(docs/user-guide.md)` |
| `docs/` | `../` | `[README]` + `(../README.md)` |
| `.github/` | `../` | `[LLM Context]` + `(../.llm/context.md)` |
| `.llm/` | `../` | `[README]` + `(../README.md)` |
| `.llm/skills/` | `../../` | `[README]` + `(../../README.md)` |
| `src/` | `../` | `[Docs]` + `(../docs/user-guide.md)` |

---

## MkDocs Path Resolution

When using MkDocs (with `docs_dir: docs`), paths resolve differently than when viewing raw Markdown on GitHub:

### The Problem

```text
Repository structure:
/
├── assets/
│   └── logo.svg
├── docs/
│   └── index.md (contains: img src = "../assets/logo.svg")
└── mkdocs.yml (docs_dir: docs)
```

| Context | Path resolves to |
|---------|------------------|
| GitHub raw view | `assets/logo.svg` ✅ Works |
| MkDocs build | Outside `docs/` directory ❌ Broken |

### Solutions

**1. Use absolute GitHub URLs** (recommended for cross-context compatibility):

```markdown
<!-- Works in GitHub, MkDocs, anywhere -->
<img src="https://raw.githubusercontent.com/owner/repo/main/assets/logo.svg">
```

**2. Copy assets into docs/** (add to CI build):

```yaml
# In workflow
- run: cp -r assets docs/assets
```

Then use `assets/logo.svg` (relative to docs/).

**3. Store assets in docs/** (simplest):

Move shared assets to `docs/assets/` and update all references.

### When Paths Work

These patterns work in both GitHub and MkDocs contexts (paths relative to `docs/`):

```text
✅ Works: Links to other docs files
   [Guide] + (user-guide.md)

✅ Works: Relative links within docs
   [Spec] + (specs/formal-spec.md)

✅ Works: Anchor links
   [Section] + (#my-heading)
```

These may break in MkDocs (when assets are outside `docs/`):

```text
⚠️ May break: Parent directory assets
   <img> with src pointing to "../assets/logo.svg"

⚠️ May break: Root-level files
   [README] + (../README.md)
```

---

## Pre-Commit Checklist

Before committing markdown changes:

- [ ] Run `./scripts/check-links.sh` — no broken links
- [ ] Run `npx markdownlint '<file>' --config .markdownlint.json` — no lint errors
- [ ] Verify relative paths are correct for your file's location
- [ ] If you moved/renamed a file, search for links to it: `rg 'old-filename\.md'`

## Debugging Broken Links

When `check-links.sh` reports a broken link:

1. **Note the source file and line number**
2. **Check the path resolution:**

   ```bash
   # From the directory containing your markdown file:
   ls -la <the-path-from-your-link>
   ```

3. **Trace the path manually:**
   - Start from the markdown file's directory
   - Apply each `../` to go up one level
   - Follow the remaining path

4. **Use fd to find the target file:**

   ```bash
   fd 'target-filename.md'
   ```

## See Also

- [GitHub Actions Best Practices](github-actions-best-practices.md) — Workflow file validation
- [Text Parsing Patterns](text-parsing-patterns.md) — Regex limitations and robust parsing
- [Main Context](../context.md) — Mandatory pre-commit checks
