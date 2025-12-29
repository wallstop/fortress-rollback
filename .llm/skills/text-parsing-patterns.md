# Text Parsing and Regex Best Practices

> **A guide to robust text parsing, regex patterns, and handling structured content like Markdown and HTML in scripts.**

## Overview

When processing text files programmatically (especially Markdown, HTML, or configuration files), naive regex patterns often fail on edge cases. This guide covers common pitfalls and robust solutions.

---

## Regex Limitations with Nested Structures

### The Nested Tag Problem

**Problem:** Regex cannot properly handle arbitrarily nested structures. A simple non-greedy match will fail on nested tags:

```python
# ❌ WRONG: Non-greedy match fails on nested divs
pattern = r'<div class="grid"[^>]*>[\s\S]*?</div>'

# Input:
# <div class="grid">
#     <div>nested content</div>  ← regex stops here
#     more content
# </div>
```

**Solution:** Use a state-based parser that tracks nesting depth:

```python
# ✅ CORRECT: Track nesting depth
def remove_grid_divs(content: str) -> str:
    result = []
    i = 0

    while i < len(content):
        # Look for opening grid div
        grid_match = re.match(r'<div\s+class="grid"[^>]*>', content[i:], re.I)
        if grid_match:
            # Track nesting depth
            depth = 1
            j = i + grid_match.end()

            while j < len(content) and depth > 0:
                open_match = re.match(r'<div[^>]*>', content[j:], re.I)
                if open_match:
                    depth += 1
                    j += open_match.end()
                    continue

                close_match = re.match(r'</div\s*>', content[j:], re.I)
                if close_match:
                    depth -= 1
                    j += close_match.end()
                    continue

                j += 1

            i = j  # Skip the entire matched div
        else:
            result.append(content[i])
            i += 1

    return ''.join(result)
```

### When to Use a Proper Parser

**Use regex when:**

- Patterns are flat (no nesting)
- Exact format is known and controlled
- Performance is critical and patterns are simple

**Use a parser when:**

- Content has nested structures (HTML, XML, JSON)
- Format has edge cases (escaping, quotes, comments)
- Correctness is more important than simplicity
- Processing user-generated content

**Python libraries for structured parsing:**

| Format | Library | Use Case |
|--------|---------|----------|
| Markdown | `mistune`, `markdown-it-py` | Full CommonMark compliance |
| HTML | `beautifulsoup4`, `lxml` | HTML/XML parsing and modification |
| JSON | `json` (stdlib) | JSON data |
| YAML | `pyyaml`, `ruamel.yaml` | YAML with comments |
| TOML | `tomllib` (stdlib 3.11+) | TOML configuration |

---

## Markdown Link Patterns

### Standard Link Pattern Limitations

```python
# Basic pattern - works for simple cases
pattern = r'\[([^\]]*)\]\(([^)]+)\)'

# Matches: [text] + (url)
# Fails on:
#   - [text [nested]] + (url)     ← nested brackets
#   - [text] + (url(with)parens)  ← parentheses in URL
#   - [text] + (url "title")      ← link titles
```

**Robust approach:** Document limitations or use a Markdown parser:

```python
def extract_links(content: str) -> list[tuple[str, str]]:
    """Extract markdown links.

    Known limitations:
    - Nested brackets in link text won't match
    - Parentheses in URLs won't match
    - For full CommonMark compliance, use mistune or markdown-it-py
    """
    pattern = re.compile(r'\[([^\]]*)\]\(([^)]+)\)')
    return [(m.group(1), m.group(2)) for m in pattern.finditer(content)]
```

### URL with Parentheses

Wikipedia URLs commonly contain parentheses. Handle with balanced matching:

```python
def extract_links_with_parens(content: str) -> list[tuple[str, str]]:
    """Extract links, handling parentheses in URLs."""
    links = []
    i = 0

    while i < len(content):
        # Find link start
        if content[i] == '[':
            # Find matching ]
            j = i + 1
            bracket_depth = 1
            while j < len(content) and bracket_depth > 0:
                if content[j] == '[':
                    bracket_depth += 1
                elif content[j] == ']':
                    bracket_depth -= 1
                j += 1

            # Check for (
            if j < len(content) and content[j] == '(':
                # Find matching ) with balanced counting
                k = j + 1
                paren_depth = 1
                while k < len(content) and paren_depth > 0:
                    if content[k] == '(':
                        paren_depth += 1
                    elif content[k] == ')':
                        paren_depth -= 1
                    k += 1

                text = content[i+1:j-1]
                url = content[j+1:k-1]
                links.append((text, url))
                i = k
                continue

        i += 1

    return links
```

---

## Inline Code Detection

### Basic Pattern Limitations

```python
# ❌ WRONG: Misses edge cases
pattern = r'`[^`\n]+`'

# Fails on:
#   - ``         ← empty inline code
#   - ``code``   ← double backticks
#   - `code`     ← works, but what about `` `code` ``?
```

### Robust Inline Code Pattern

```python
def find_inline_code_ranges(content: str) -> list[tuple[int, int]]:
    """Find inline code spans with proper handling.

    Handles:
    - Standard: `code`
    - Empty: ``
    - Double backticks: ``code with ` inside``

    Limitation: Does not handle inline code with newlines
    (rare in practice, would need multi-line mode).
    """
    ranges = []

    # Handle double backticks first (can contain single `)
    for match in re.finditer(r'``[^`\n]*``', content):
        ranges.append((match.start(), match.end()))

    # Then single backticks, avoiding overlaps
    for match in re.finditer(r'`[^`\n]*`', content):
        start, end = match.start(), match.end()
        if not any(s <= start < e for s, e in ranges):
            ranges.append((start, end))

    return ranges
```

---

## Code Fence Detection

### State-Based Parser

Code fences require tracking state because:

1. Fences can use ``` or ~~~
2. Closing fence must match or exceed opening length
3. Different characters (` vs ~) can't close each other

```python
def find_code_fence_ranges(content: str) -> list[tuple[int, int]]:
    """Find fenced code blocks with proper nesting support."""
    ranges = []
    lines = content.split('\n')
    pos = 0

    fence_start = None
    fence_char = None
    fence_len = 0

    for line in lines:
        line_start = pos
        stripped = line.lstrip()

        if stripped.startswith('```') or stripped.startswith('~~~'):
            char = stripped[0]
            count = len(stripped) - len(stripped.lstrip(char))

            if fence_start is None:
                # Opening fence
                fence_start = line_start
                fence_char = char
                fence_len = count
            elif char == fence_char and count >= fence_len:
                # Matching closing fence
                ranges.append((fence_start, pos + len(line)))
                fence_start = None
                fence_char = None
                fence_len = 0

        pos += len(line) + 1

    # Handle unclosed fence at EOF
    if fence_start is not None:
        ranges.append((fence_start, len(content)))

    return ranges
```

---

## MkDocs Material to GitHub Wiki Conversion

### Grid Cards Content Extraction

MkDocs Material "grid cards" use a specific structure that must be **converted**, not removed,
when generating GitHub Wiki content:

```markdown
<!-- Source: MkDocs Material -->
<div class="grid cards" markdown>

-   :material-icon:{ .lg .middle } **Title**

    ---

    Description text.

    [:octicons-arrow-right-24: Link text](url)

</div>
```

**Common mistake:** Simply removing the div leaves empty sections. Always **convert to markdown**:

```markdown
<!-- Target: GitHub Wiki -->
- **Title** — Description text. [Link text](url)
```

### Parsing Grid Cards

```python
def _parse_grid_cards_content(div_content: str) -> str:
    """Parse grid cards list items and convert to markdown list.

    Each card has this structure:
        -   :icon:{ .attrs } **Title**
            ---
            Description paragraph.
            [:octicons-arrow-right-24: Link text](url)
    """
    lines = div_content.split('\n')
    cards: list[dict[str, str]] = []
    current_card: dict[str, str] | None = None

    for line in lines:
        stripped = line.strip()

        # Check for card start (list item with title)
        card_match = re.match(r'^-\s+.*\*\*([^*]+)\*\*', stripped)
        if card_match:
            if current_card:
                cards.append(current_card)
            current_card = {
                "title": card_match.group(1).strip(),
                "description": "",
                "link_text": "",
                "link_url": "",
            }
            continue

        if current_card:
            # Skip separator lines
            if stripped == "---":
                continue

            # Check for link line
            link_match = re.match(r'^\[:[\w-]+:\s*([^\]]+)\]\(([^)]+)\)', stripped)
            if link_match:
                current_card["link_text"] = link_match.group(1).strip()
                current_card["link_url"] = link_match.group(2).strip()
                continue

            # Regular content (description)
            if stripped and not stripped.startswith(("<", "<!--")):
                if current_card["description"]:
                    current_card["description"] += " " + stripped
                else:
                    current_card["description"] = stripped

    if current_card:
        cards.append(current_card)

    # Build markdown list
    output = []
    for card in cards:
        parts = [f"- **{card['title']}**"]
        if card["description"]:
            parts.append(f" — {card['description']}")
        if card["link_text"] and card["link_url"]:
            parts.append(f" [{card['link_text']}]({card['link_url']})")
        output.append("".join(parts))

    return "\n".join(output) + "\n"
```

### Validation: Detecting Empty Sections

After conversion, validate that no sections are left empty:

```python
def check_empty_sections(content: str, filename: str) -> list[Issue]:
    """Check for empty content sections (headers followed by only whitespace/comments).

    This catches issues like grid cards content being removed instead of converted,
    leaving empty sections.
    """
    issues = []
    lines = content.split("\n")

    for i, line in enumerate(lines, 1):
        # Check for section headers (##, ###, etc.)
        header_match = re.match(r"^(#{2,6})\s+(.+)$", line)
        if header_match:
            header_level = header_match.group(1)
            header_text = header_match.group(2).strip()

            # Look ahead to see if section has content
            # Use while loop with explicit index since we need precise control
            section_has_content = False
            # i is 1-indexed (line number), but lines[] is 0-indexed.
            # Conveniently, the 1-indexed line number of current line equals
            # the 0-indexed position of the NEXT line (i.e., lines[i] is next line).
            j = i  # 0-indexed position of next line after header

            while j < len(lines):
                next_line = lines[j]

                # Check if we've hit another header of same or higher level
                next_header_match = re.match(r"^(#{2,6})\s+", next_line)
                if next_header_match:
                    next_level = next_header_match.group(1)
                    # If same or higher level (fewer or equal #), section ends
                    if len(next_level) <= len(header_level):
                        break

                # Skip empty lines, horizontal rules, and HTML comments
                stripped = next_line.strip()
                if stripped and stripped != "---" and not stripped.startswith("<!--"):
                    section_has_content = True
                    break

                j += 1

            # If section has no content, report it
            if not section_has_content:
                issues.append(
                    Issue(
                        file=filename,
                        line=i,
                        severity="error",
                        message=f"Empty section: '{header_text}' has no content (possible conversion issue)",
                    )
                )

    return issues
```

---

## MkDocs and Static Site Considerations

### Asset Path Resolution

When using MkDocs with `docs_dir: docs`, relative paths resolve differently:

| Context | Base Directory | `../assets/logo.svg` resolves to |
|---------|---------------|----------------------------------|
| GitHub raw view | Repository root | `assets/logo.svg` (correct) |
| MkDocs build | `docs/` directory | `../assets/` (outside docs, broken) |

**Solutions:**

1. **Absolute GitHub URLs** (works everywhere):

```markdown
<img src="https://raw.githubusercontent.com/owner/repo/main/assets/logo.svg">
```

1. **Copy assets into docs/** (requires build step):

```yaml
# In CI/CD
- run: cp -r assets docs/assets
```

1. **Symlink** (may not work in all contexts):

```bash
# In docs/
ln -s ../assets assets
```

**Best practice:** Use absolute URLs for cross-context compatibility, or ensure build process copies assets.

---

## Safe Text Replacement

### Avoid Position Drift

When replacing multiple matches, process in reverse order to avoid position drift:

```python
# ❌ WRONG: Positions drift as you replace
for match in pattern.finditer(content):
    content = content[:match.start()] + replacement + content[match.end():]
    # Next match positions are now wrong!

# ✅ CORRECT: Process in reverse order
matches = list(pattern.finditer(content))
for match in reversed(matches):
    content = content[:match.start()] + replacement + content[match.end():]
```

### Check for Overlap Before Replacing

When tracking multiple range types (code blocks, inline code, links), check for overlaps:

```python
def in_protected_range(pos: int, ranges: list[tuple[int, int]]) -> bool:
    """Check if position falls within any protected range."""
    return any(start <= pos < end for start, end in ranges)

# Use before processing
for link_match in links:
    if in_protected_range(link_match.start, code_ranges):
        continue  # Skip links inside code blocks
    # Process link...
```

---

## Testing Text Processing

### Property-Based Testing for Parsers

```python
from hypothesis import given, strategies as st

@given(st.text())
def test_code_fence_detection_never_crashes(text):
    """Parser should handle any input without crashing."""
    ranges = find_code_fence_ranges(text)
    assert isinstance(ranges, list)
    for start, end in ranges:
        assert 0 <= start <= end <= len(text)

@given(st.text(alphabet='`abc\n '))
def test_inline_code_ranges_dont_overlap(text):
    """Detected ranges should never overlap."""
    ranges = find_inline_code_ranges(text)
    ranges.sort()
    for i in range(len(ranges) - 1):
        assert ranges[i][1] <= ranges[i+1][0], "Ranges overlap"
```

### Round-Trip Testing

```python
def test_link_conversion_roundtrip():
    """Converting links should preserve structure."""
    original = "[Guide]" + "(user-guide.md)"  # Note: split to avoid link checker
    content = "See the " + original + " for details."

    # Convert and verify link is still present
    converted = convert_links(content)
    assert "[Guide]" in converted
    assert "(" in converted and ")" in converted
```

---

## Enumerate Indexing Pitfall

### The 1-Indexed vs 0-Indexed Confusion

When using `enumerate(lines, 1)` for human-readable line numbers, be careful when using the index for subsequent list access:

```python
# ❌ CONFUSING: Comment claims j is 1-indexed but it's used as 0-indexed
for i, line in enumerate(lines, 1):
    if is_header(line):
        j = i  # "j is 1-indexed like line numbers" <- MISLEADING COMMENT
        while j < len(lines):
            next_line = lines[j]  # Actually works! But why?
```

**Why this code works (by coincidence):**

| `i` (1-indexed) | `line` | Want to access | `lines[i]` gives |
|-----------------|--------|----------------|------------------|
| 1 | `lines[0]` | Next line: `lines[1]` | ✓ `lines[1]` |
| 2 | `lines[1]` | Next line: `lines[2]` | ✓ `lines[2]` |
| N | `lines[N-1]` | Next line: `lines[N]` | ✓ `lines[N]` |

The 1-indexed line number of the current line **happens to equal** the 0-indexed position of the next line! This is a mathematical consequence (`i = (i-1) + 1`), not immediately obvious from the code.

### Clear Alternatives

**Option 1: Use 0-indexed enumerate, calculate line number for reporting:**

```python
# ✅ CLEAR: Standard 0-indexed loop, line_num only for error messages
for idx, line in enumerate(lines):
    line_num = idx + 1  # For error reporting only
    if is_header(line):
        # Start checking from next line (idx + 1)
        j = idx + 1
        while j < len(lines):
            next_line = lines[j]  # Obviously 0-indexed access
```

**Option 2: Keep 1-indexed but document the math explicitly:**

```python
# ✅ CLEAR: Explicitly convert when needed
for i, line in enumerate(lines, 1):
    if is_header(line):
        # i is 1-indexed (line number), but lines[] is 0-indexed.
        # Conveniently, the 1-indexed line number of current line equals
        # the 0-indexed position of the NEXT line (i.e., lines[i] is next line).
        j = i  # 0-indexed position of next line after header
        while j < len(lines):
            next_line = lines[j]
```

### Test for Index Correctness

When writing code that mixes indexing schemes, add explicit tests:

```python
def test_enumerate_indexing_edge_cases():
    """Verify line number to index conversion is correct."""
    # Single line - no next line to access
    lines = ["## Header"]
    # Should not crash, should not access out of bounds

    # Header on last line
    lines = ["Content", "## Header"]
    # j = 2, len(lines) = 2, while 2 < 2 is False - no loop entered (correct)

    # Header on first line
    lines = ["## Header", "Content"]
    # i = 1, j = 1, lines[1] = "Content" (correct - the next line)
```

---

## Summary Checklist

When writing text processing code:

- [ ] Document limitations of regex patterns
- [ ] Use proper parser for nested structures (HTML, XML)
- [ ] Handle edge cases (empty strings, unclosed tags)
- [ ] Process matches in reverse order when replacing
- [ ] Check for overlapping ranges before modification
- [ ] Test with property-based testing for robustness
- [ ] Consider cross-context path resolution (GitHub vs built site)
- [ ] Validate with real-world samples from the codebase

---

## Related Documentation

- [Markdown Link Validation](markdown-link-validation.md) — Link checking and path resolution
- [Property Testing](property-testing.md) — Automated edge case discovery
- [Defensive Programming](defensive-programming.md) — Error handling patterns

---

*License: MIT OR Apache-2.0*
