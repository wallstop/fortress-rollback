# Python Script Guidelines

> **Guidelines for Python scripts used in tooling, CI/CD, and pre-commit hooks.**
> This project is primarily Rust, but uses Python for cross-platform scripting.

## Code Quality Standards

Python scripts in this project must pass linting checks. Follow these rules to prevent common issues.

### Unused Imports

**Remove all unused imports.** Linters flag these as errors.

```python
# ❌ BAD: os imported but never used
import os
import sys
from pathlib import Path

def main():
    return Path.cwd()

# ✅ GOOD: Only import what you use
import sys
from pathlib import Path

def main():
    return Path.cwd()
```

**Tip:** Use `pathlib.Path` instead of `os.path` for cross-platform path handling. This often eliminates the need to import `os` at all.

### Unused Variables

**Prefix intentionally unused variables with underscore (`_`)** to signal to linters and readers that the value is deliberately ignored.

```python
# ❌ BAD: link_text assigned but never used — linter warning
for match in pattern.finditer(content):
    link_text = match.group(1)  # Warning: unused variable
    link_target = match.group(2)
    process(link_target)

# ✅ GOOD: Underscore prefix indicates intentional non-use
for match in pattern.finditer(content):
    _link_text = match.group(1)  # Captured but unused; kept for debugging
    link_target = match.group(2)
    process(link_target)

# ✅ ALSO GOOD: Use _ for completely discarded values
for match in pattern.finditer(content):
    _ = match.group(1)  # Explicitly discarded
    link_target = match.group(2)
    process(link_target)

# ✅ BEST: Don't capture if you don't need it
for match in pattern.finditer(content):
    link_target = match.group(2)  # Only capture what's needed
    process(link_target)
```

### Empty Exception Handlers

**Never use bare `except: pass` or `except SomeError: pass` without explanation.** Add a comment explaining why the exception is being silently ignored.

```python
# ❌ BAD: Silent exception swallowing — what happens on error?
try:
    content = file.read_text()
    anchors = extract_anchors(content)
except (OSError, UnicodeDecodeError):
    pass

# ✅ GOOD: Comment explains the intentional behavior
try:
    content = file.read_text()
    anchors = extract_anchors(content)
except (OSError, UnicodeDecodeError):
    pass  # File read errors are non-fatal; treat link as valid

# ✅ BETTER: Handle the error explicitly when possible
try:
    content = file.read_text()
    anchors = extract_anchors(content)
except (OSError, UnicodeDecodeError) as e:
    # Log but continue; file read errors shouldn't block validation
    if verbose:
        print(f"Warning: Could not read {file}: {e}", file=sys.stderr)
    return True, ""  # Treat as valid
```

**When is `pass` acceptable?**

- Graceful degradation where the error is truly ignorable
- Optional feature detection (e.g., trying to import an optional module)
- Cases where recovery is handled elsewhere

**Always document WHY** the exception is being ignored.

### Type Hints

Use type hints for function signatures. They improve readability and catch bugs early.

```python
# ❌ BAD: No type information
def check_link(file, target, root, verbose):
    pass

# ✅ GOOD: Clear type hints
def check_link(
    file: Path,
    target: str,
    root: Path,
    verbose: bool = False,
) -> tuple[bool, str]:
    """Check if a link target is valid.

    Returns:
        Tuple of (is_valid, error_message).
    """
    pass
```

### Named Tuples for Structured Returns

Use `NamedTuple` for functions returning multiple values:

```python
# ❌ BAD: Magic tuple positions
def validate() -> tuple[int, int, int]:
    return errors, warnings, checked  # Which is which?

# ✅ GOOD: Self-documenting return type
from typing import NamedTuple

class ValidationResult(NamedTuple):
    """Result of validation run."""
    errors: int
    warnings: int
    checked: int

def validate() -> ValidationResult:
    return ValidationResult(errors=0, warnings=2, checked=100)
```

### Path Handling

Use `pathlib.Path` for all path operations. It's cross-platform and more readable.

```python
# ❌ BAD: os.path is verbose and error-prone
import os
script_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.dirname(script_dir)
file_path = os.path.join(project_root, "src", "lib.rs")

# ✅ GOOD: pathlib is cleaner and cross-platform
from pathlib import Path
script_dir = Path(__file__).parent.resolve()
project_root = script_dir.parent
file_path = project_root / "src" / "lib.rs"
```

### Error Handling Patterns

#### Fail Fast at Entry Points

Validate early and provide clear error messages:

```python
def main() -> int:
    """Main entry point."""
    project_root = get_project_root()

    if not (project_root / "Cargo.toml").exists():
        print("ERROR: Must run from project root", file=sys.stderr)
        return 1

    # Continue with validated state...
```

#### Use sys.exit() or Return Codes

Scripts should return meaningful exit codes:

```python
def main() -> int:
    """Main entry point. Returns 0 on success, non-zero on failure."""
    try:
        result = run_checks()
        return 0 if result.errors == 0 else 1
    except KeyboardInterrupt:
        print("\nInterrupted", file=sys.stderr)
        return 130  # Standard interrupted exit code

if __name__ == "__main__":
    sys.exit(main())
```

### Cross-Platform Considerations

Scripts must work on Windows, macOS, and Linux:

```python
# ❌ BAD: Unix-specific path separator
path = "src/lib.rs"

# ✅ GOOD: Use pathlib for cross-platform paths
path = Path("src") / "lib.rs"

# ❌ BAD: Unix-specific shell command
subprocess.run("ls -la", shell=True)

# ✅ GOOD: Use list form with cross-platform commands
subprocess.run(["python", "-m", "pip", "list"], check=True)
```

### Subprocess Best Practices

#### Output Handling for Linters and Tools

When running linters or tools where output should flow to the user's terminal:

```python
# ❌ BAD: check=False without capturing output — errors are silent
result = subprocess.run(cmd, check=False)
if result.returncode != 0:
    # What went wrong? No output captured!
    return result.returncode

# ✅ GOOD: Let output flow naturally to terminal (for linters)
# No capture needed — actionlint/clippy/etc. print their own output
result = subprocess.run(cmd, check=False)
return result.returncode

# ✅ ALSO GOOD: Capture when you need to process/filter output
result = subprocess.run(cmd, capture_output=True, text=True)
if result.returncode != 0:
    print(f"Error:\n{result.stderr}", file=sys.stderr)
```

#### When to Capture Output

| Scenario | Capture? | Why |
|----------|----------|-----|
| Running a linter (actionlint, clippy) | No | Output flows to terminal naturally |
| Parsing command output | Yes | Need to process the result |
| Filtering/transforming output | Yes | Need to modify before display |
| Checking for specific patterns | Yes | Need to search output text |

#### Avoid Redundant Exception Handlers

Don't catch exceptions that can't happen due to prior validation:

```python
# ❌ BAD: Redundant FileNotFoundError after shutil.which() check
actionlint = shutil.which("actionlint")
if actionlint is None:
    print("actionlint not found")
    return 0

# ... later ...
try:
    result = subprocess.run([actionlint, ...])
except FileNotFoundError:  # Can't happen! We checked above
    print("actionlint not found")  # Unreachable
    return 0

# ✅ GOOD: Trust the prior validation
actionlint = shutil.which("actionlint")
if actionlint is None:
    print("actionlint not found", file=sys.stderr)
    return 0

# shutil.which() verified it exists — no FileNotFoundError possible
result = subprocess.run([actionlint, ...])
return result.returncode
```

**Why this matters:**

1. Redundant handlers mask real errors — returning success (0) when something actually failed
2. Dead code confuses readers about what can actually happen
3. Swallowing OSError hides genuine system problems (permissions, I/O errors)

#### Safe Subprocess Pattern

```python
# ✅ RECOMMENDED: Validate existence, then run without redundant handling
def run_tool(tool_name: str, args: list[str]) -> int:
    """Run an external tool, returning its exit code.

    Returns 0 (skip) if tool is not installed.
    """
    tool_path = shutil.which(tool_name)
    if tool_path is None:
        print(f"Warning: {tool_name} not found, skipping", file=sys.stderr)
        return 0

    # Tool exists — run it. Any exception here is a real error.
    result = subprocess.run([tool_path, *args], check=False)
    return result.returncode
```

#### Command Construction

```python
# ❌ BAD: shell=True is a security risk and platform-dependent
result = subprocess.run("cargo fmt --check", shell=True)

# ✅ GOOD: Use list form, capture output, check errors
result = subprocess.run(
    ["cargo", "fmt", "--check"],
    capture_output=True,
    text=True,
)
if result.returncode != 0:
    print(f"Format check failed:\n{result.stderr}")
```

### String Formatting

Use f-strings for string formatting:

```python
# ❌ BAD: Old-style formatting
print("Checking %s..." % filename)
print("Found {} errors".format(count))

# ✅ GOOD: f-strings are clearer
print(f"Checking {filename}...")
print(f"Found {count} errors")
```

---

## Pre-commit Integration

### Running Linters

Before committing Python changes, run:

```bash
# Check for issues (if ruff is installed)
ruff check scripts/

# Or use the built-in Python linter
python -m py_compile scripts/*.py
```

### Common Linter Codes

| Code | Issue | Fix |
|------|-------|-----|
| F401 | Unused import | Remove the import |
| F841 | Unused variable | Prefix with `_` or remove |
| E722 | Bare `except:` | Specify exception type |
| E501 | Line too long | Break line or refactor |
| W503 | Binary operator at line start | Move to end of previous line |

---

## Script Template

Use this template for new scripts:

```python
#!/usr/bin/env python3
"""
Brief description of what the script does.

Longer description if needed, explaining:
- What the script validates/processes
- When it should be run (pre-commit, CI, manually)
- Any prerequisites or dependencies

Works on Windows, macOS, and Linux.
"""

import sys
from pathlib import Path
from typing import NamedTuple


class Result(NamedTuple):
    """Result of the operation."""
    success: bool
    message: str


def get_project_root() -> Path:
    """Get the project root directory."""
    return Path(__file__).parent.parent.resolve()


def main() -> int:
    """Main entry point. Returns 0 on success, non-zero on failure."""
    project_root = get_project_root()

    if not (project_root / "Cargo.toml").exists():
        print("ERROR: Must run from project root", file=sys.stderr)
        return 1

    # Your logic here...

    return 0


if __name__ == "__main__":
    sys.exit(main())
```

---

## Summary Checklist

Before committing Python scripts:

- [ ] No unused imports
- [ ] No unused variables (prefix with `_` if intentionally unused)
- [ ] All `except: pass` clauses have explanatory comments
- [ ] No redundant exception handlers after prior validation (e.g., FileNotFoundError after shutil.which)
- [ ] Type hints on function signatures
- [ ] `pathlib.Path` for path operations
- [ ] f-strings for string formatting
- [ ] Meaningful exit codes
- [ ] Errors/warnings printed to `sys.stderr`, not stdout
- [ ] Works cross-platform (no shell=True, no hardcoded paths)
- [ ] Subprocess output captured only when needed (let linter output flow naturally)

---

*These guidelines apply to all Python scripts in `scripts/`, CI workflows, and tooling.*
