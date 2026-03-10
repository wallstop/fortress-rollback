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

### Exception Handling

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

The #1 cross-platform `sed` failure:

```bash
# WRONG on macOS:
sed -i 's/old/new/g' file.txt

# PORTABLE: use backup extension, then remove
sed -i.bak 's/old/new/g' file.txt && rm file.txt.bak

# ALTERNATIVE: temp file pattern
sed 's/old/new/g' file.txt > file.txt.tmp && mv file.txt.tmp file.txt
```

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

```bash
#!/bin/bash
set -euo pipefail        # Strict mode

# Check dependencies
command -v jq >/dev/null 2>&1 || { echo "Error: jq required" >&2; exit 1; }

# Always quote variables
rm "$file"

# Use $() not backticks
result=$(command)

# Tool availability with fallback
if command -v sd &>/dev/null; then
    sd 'pattern' 'replacement' file
else
    sed -i.bak -E 's|pattern|replacement|g' file && rm -f file.bak
fi
```

### Platform Detection

```bash
case "$(uname -s)" in
    Linux*)  OS=linux ;;
    Darwin*) OS=macos ;;
    MINGW*|CYGWIN*) OS=windows ;;
    *)       OS=unknown ;;
esac
```

### CI-Specific

```yaml
# GitHub Actions: use bash explicitly
- name: Run script
  shell: bash
  run: |
    set -euo pipefail
    ./scripts/my-script.sh
```
