<!-- CATEGORY: Rust Language -->
<!-- WHEN: Parsing text, regex patterns, state machine parsers -->
# Text Parsing Patterns

## When to Use Regex vs Parser

**Use regex when:** Flat patterns, exact known format, simple performance needs.
**Use a parser when:** Nested structures, edge cases (escaping, quotes), user-generated content.

## Nested Structure Problem

Regex cannot handle arbitrary nesting. Use state-based depth tracking:

```python
def remove_nested_divs(content: str) -> str:
    depth = 1
    j = start_pos
    while j < len(content) and depth > 0:
        if re.match(r'<div[^>]*>', content[j:]): depth += 1
        elif re.match(r'</div\s*>', content[j:]): depth -= 1
        j += 1
```

## Inline Code Detection (CommonMark)

Closing delimiter must be **exactly** N backticks, not part of a longer sequence:

```python
# Find opening backticks, count them
# Search for closing with exact same count
# Verify: char before != '`' AND char after != '`'
```

Example: `` ``code ``` more text`` `` -- the triple backticks are skipped because they are not exactly 2.

## Code Fence Detection

Track state: opening fence char (`` ` `` or `~`), length, and match closing fence of same char with >= length.

## Common Regex Pitfalls

| Pitfall | Wrong | Right |
|---------|-------|-------|
| Character class stops early | `[^*]+` in `**([^*]+)**` | `(.+?)` non-greedy |
| Greedy matches too much | `.*` before capture | `.*?` non-greedy |
| Overly broad HTML detection | `startswith("<")` | `re.match(r'^<(!--|[a-zA-Z])')` |
| Single-line comment assumption | `startswith("<!--")` | Track multi-line state |
| Mixed regex modes (BRE/ERE) | `grep` then `grep -E` | Consistent `-E` everywhere |

## Safe Text Replacement

Process matches in reverse order to avoid position drift:

```python
matches = list(pattern.finditer(content))
for match in reversed(matches):
    content = content[:match.start()] + replacement + content[match.end():]
```

Check for overlapping ranges before modifying:

```python
def in_protected_range(pos, ranges):
    return any(start <= pos < end for start, end in ranges)
```

## Markdown Link Pattern

```python
pattern = r'\[([^\]]*)\]\(([^)]+)\)'
# Limitations: no nested brackets, no parens in URL, no titles
# For full CommonMark: use mistune or markdown-it-py
```

For Wikipedia URLs with parens, use balanced-counting parser instead of regex.

## Empty Section Detection

After content conversion, validate sections are not empty:
- Track multi-line HTML comment state
- Skip blank lines and `---`
- Report headers followed by only whitespace/comments

## Enumerate Indexing

When using `enumerate(lines, 1)` for line numbers, note that the 1-indexed line number of the current line equals the 0-indexed position of the next line (`i = (i-1) + 1`).

Clearer alternative:
```python
for idx, line in enumerate(lines):
    line_num = idx + 1  # for error reporting
    j = idx + 1         # obviously 0-indexed next line
```

## Testing Text Processing

### Property-Based Testing
```python
@given(st.text())
def test_parser_never_crashes(text):
    ranges = find_code_fence_ranges(text)
    for start, end in ranges:
        assert 0 <= start <= end <= len(text)
```

### Overlap Testing
```python
@given(st.text(alphabet='`abc\n '))
def test_ranges_dont_overlap(text):
    ranges = sorted(find_inline_code_ranges(text))
    for i in range(len(ranges) - 1):
        assert ranges[i][1] <= ranges[i+1][0]
```

## Parser Libraries

| Format | Python Library |
|--------|---------------|
| Markdown | `mistune`, `markdown-it-py` |
| HTML | `beautifulsoup4`, `lxml` |
| JSON | `json` (stdlib) |
| YAML | `pyyaml`, `ruamel.yaml` |
| TOML | `tomllib` (stdlib 3.11+) |

## Checklist

- [ ] Document limitations of regex patterns
- [ ] Use proper parser for nested structures
- [ ] Handle edge cases (empty strings, unclosed tags)
- [ ] Process matches in reverse order when replacing
- [ ] Check for overlapping ranges
- [ ] Test with property-based testing
- [ ] Validate with real-world samples
