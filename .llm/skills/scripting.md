<!-- CATEGORY: CI/CD & Tooling -->
<!-- WHEN: Writing build scripts, Python helpers, shell portability -->
# Scripting Guide (Python & Shell)

## Python Script Rules

### Required Practices

- **Remove unused imports** -- linters flag as errors
- **Prefix unused variables** with `_` (e.g., `_link_text = match.group(1)`)
- **Type hints** on all function signatures
- **`pathlib.Path`** for all path operations (not `os.path`)
- **f-strings** for formatting (not `%` or `.format()`)
- **Meaningful exit codes** via `sys.exit(main())`
- **Errors to stderr**: `print("ERROR: ...", file=sys.stderr)`
- **No `shell=True`** in subprocess calls

### Error Reporting in Lint Scripts

Lint hooks must use `{path}:{line_number}: {message}` format so editors
hyperlink to the correct location. When a violation spans multiple lines
(e.g., attribute on line A, target on line B), the `path:line` prefix
must point to the **violation site** (where the fix is needed), not the
trigger/detection line. Mention the trigger line in the message body:

```python
# Multi-line: prefix → async fn line, body → attribute line
f"{path}:{fn_line}: #[track_caller] (line {attr_line}) on async fn ..."
```

Never prefix issue lines with leading whitespace (e.g., `"  {issue}"`) in
`main()` summary output -- leading spaces break the `path:line:` prefix
that editors rely on for hyperlinking.

For file-level errors where no specific line number exists (e.g., cannot
read file), use a synthetic line number `:0:` to maintain the format:

```python
# File-level error: use :0: as synthetic line number
f"{path}:0: cannot read file: {exc}"
```

### Exception Handling

#### read\_text() Exceptions

`path.read_text(encoding="utf-8")` can raise both `OSError` (missing/locked file)
and `UnicodeDecodeError` (non-UTF-8 bytes). Always catch both:

```python
# WRONG                              # CORRECT
except OSError as e:                  except (OSError, UnicodeDecodeError) as e:
    ...                                   ...
```

Alternative: `errors="replace"` for best-effort reading (grep-like hooks).

#### Read Error Propagation

Hooks that cannot read a file must **fail**, not silently pass. Return the error
in the issues list so `main()` sees it:

```python
except (OSError, UnicodeDecodeError) as exc:
    msg = f"{path}:0: cannot read file: {exc}"
    print(msg, file=sys.stderr)
    return [msg]  # NOT return [] -- that silently passes
```

#### Parse Error Line Numbers

Extract the real line number from parse exceptions instead of hard-coding `:1:`:

```python
line = getattr(e, "lineno", 1) or 1  # fallback to 1
print(f"{path}:{line}: TOML error: {e}", file=sys.stderr)
```

Line number attributes: `tomllib.TOMLDecodeError.lineno`,
`json.JSONDecodeError.lineno`, `yaml.YAMLError.problem_mark.line` (0-based).

#### General Pattern

```python
# WRONG: silent swallowing
try:
    content = file.read_text()
except OSError:
    pass

# CORRECT: comment explains why
try:
    content = file.read_text()
except OSError:
    pass  # File read errors are non-fatal; treat link as valid
```

### Regex Patterns for f-string Detection

When writing regex to detect f-string patterns in Python source code (e.g., in
lint hooks), handle **both quote styles** and the `r` prefix (the only prefix
that combines with `f`):

```python
# WRONG: only matches double-quoted f-strings
re.search(r'f"\{(\w+)\}: cannot read', line)

# CORRECT: both quotes + optional r prefix (rf/fr)
re.search(r'''r?fr?["']\{(\w+)\}:\s+cannot\s+read''', line)
```

The `check-hook-output-format.py` pre-commit hook enforces these patterns.

### Subprocess Best Practices

```python
# For linters: let output flow to terminal (no capture needed)
result = subprocess.run(["actionlint"], check=False)
return result.returncode

# Validate tool existence, then run without redundant handlers
def run_tool(tool_name: str, args: list[str]) -> int:
    tool_path = shutil.which(tool_name)
    if tool_path is None:
        print(f"Warning: {tool_name} not found, skipping", file=sys.stderr)
        return 0
    result = subprocess.run([tool_path, *args], check=False)
    return result.returncode
```

Do NOT catch `FileNotFoundError` after `shutil.which()` already validated existence.

### Script Template

```python
#!/usr/bin/env python3
"""Brief description. Works on Windows, macOS, and Linux."""
import sys
from pathlib import Path

def get_project_root() -> Path:
    return Path(__file__).parent.parent.resolve()

def main() -> int:
    project_root = get_project_root()
    if not (project_root / "Cargo.toml").exists():
        print("ERROR: Must run from project root", file=sys.stderr)
        return 1
    # Logic here...
    return 0

if __name__ == "__main__":
    sys.exit(main())
```

### Test Naming

| Function | Test Class |
|----------|------------|
| `convert_admonitions` | `TestConvertAdmonitions` |
| `path_to_wiki_name` | `TestPathToWikiName` |

Methods: `test_empty_input_returns_empty_string`, `test_unclosed_div_is_handled_gracefully`

### Common Linter Codes

| Code | Issue | Fix |
|------|-------|-----|
| F401 | Unused import | Remove it |
| F841 | Unused variable | Prefix with `_` |
| E722 | Bare `except:` | Specify exception type |

---

## Shell Script Portability

### sed -i (Critical)

The #1 cross-platform `sed` failure. Portable: `sed -i.bak 's/.../g' f && rm f.bak`

### Portable Patterns Quick Reference

| Task | Non-Portable | Portable |
|------|-------------|----------|
| In-place sed | `sed -i 's/.../g' f` | `sed -i.bak 's/.../g' f && rm f.bak` |
| Newlines | `echo -e "a\nb"` | `printf "a\nb\n"` |
| Pattern match | `[[ $x == p* ]]` | `case "$x" in p*) ... ;; esac` |
| Source script | `source file` | `. file` |
| Process sub | `diff <(a) <(b)` | Use temp files |
| grep file filter | `grep -rl 'pat' --include='*.rs'` | `find . -name '*.rs' -exec grep -l 'pat' {} +` |
| Perl regex | `grep -oP 'fn \K\w+'` | `grep -o 'fn [a-zA-Z_]*' \| sed 's/^fn //'` |
| Timeout | `timeout 300 cmd` | Wrapper with `timeout`/`gtimeout` fallback |
| Canonical path | `readlink -f path` | `realpath path` |
| Binary path | `/bin/sed 's/.../g'` | `sed 's/.../g'` (rely on PATH) |

### Backtick Escaping

| Context | Backtick Handling |
|---------|-------------------|
| Single quotes `'...'` | Literal, no escaping needed |
| Double quotes `"..."` | Must escape: `\`cmd\`` |
| Heredoc `<< 'EOF'` | Literal, no escaping needed |
| Heredoc `<< EOF` | Executes -- avoid or escape |

### GNU grep Extensions (Avoid)

`--include`, `--exclude`, `-P` (Perl regex) are GNU-only. Use `find` + `grep` and `sed` instead.

### Best Practices

- `set -euo pipefail` at the top of every script
- `command -v tool >/dev/null 2>&1 || { echo "Error" >&2; exit 1; }` for deps
- Always quote variables: `rm "$file"`
- Use `$()` not backticks
- Platform detection: `case "$(uname -s)" in Linux*) ... ;; Darwin*) ... ;; esac`
- GitHub Actions: always set `shell: bash` and `set -euo pipefail`

---

## Dockerfile Best Practices

### Quick Reference

| Task | Wrong | Right |
|------|-------|-------|
| pip install | `pip install pkg` | `pip install --no-cache-dir pkg` |
| Silent detection | `command -v tool >&2` | `command -v tool >/dev/null 2>&1` |
| Multi-tool install | `pip install a b c` | Install individually with fallback |
| Layer cleanup | Separate `RUN rm ...` | Clean up in the same `RUN` layer |

### Key Rules

- `pip install --no-cache-dir` always (no wheel cache in image layers)
- `command -v tool >/dev/null 2>&1` for silent detection (not `>&2`)
- Install optional tools individually with `|| echo "failed"` fallback
- Guard `.bashrc` aliases/`eval` with `command -v` when tools are optional
- Clean up caches in the **same** `RUN` layer (otherwise bytes persist)

```dockerfile
# CORRECT: single layer, cleanup at the end
RUN apt-get update \
 && apt-get install -y --no-install-recommends curl \
 && rm -rf /var/lib/apt/lists/*
```

```bash
# Guard optional tool aliases
command -v eza >/dev/null 2>&1 && alias ls="eza"
command -v zoxide >/dev/null 2>&1 && eval "$(zoxide init bash)"
```

The pre-commit hook `check-dockerfile` enforces unguarded `eval "$("`
detection. Mandatory apt-installed tools do not need guards.
