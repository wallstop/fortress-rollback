# Shell Script Portability

> **A guide to writing portable shell scripts that work across Linux, macOS, and CI environments.**

## Overview

Shell scripts must work consistently across different platforms. Common issues include differences between GNU and BSD utilities (particularly `sed`), non-portable bash features, and output formatting quirks.

---

## `sed -i` Portability (Critical)

The `-i` (in-place edit) flag is the #1 source of cross-platform `sed` failures.

### The Problem

| Platform | `sed -i` Syntax | Backup Extension |
|----------|-----------------|------------------|
| **GNU (Linux)** | `sed -i 's/old/new/' file` | Optional — empty string means no backup |
| **BSD (macOS)** | `sed -i '' 's/old/new/' file` | **Required** — empty string `''` means no backup |

```bash
# ❌ FAILS on macOS — BSD sed requires backup extension argument
sed -i 's/old/new/g' file.txt
# Error: "sed: 1: extra characters at the end of command"

# ❌ FAILS on Linux — GNU sed interprets '' as the pattern
sed -i '' 's/old/new/g' file.txt
# Creates backup file "file.txt''" or fails
```

### Portable Solutions

**Option 1: Use a backup extension (simplest)**

```bash
# ✅ Works everywhere — creates .bak backup, then remove it
sed -i.bak 's/old/new/g' file.txt && rm file.txt.bak
```

**Option 2: Detect the platform**

```bash
# ✅ Platform detection helper
sed_inplace() {
    if sed --version 2>/dev/null | grep -q GNU; then
        sed -i "$@"
    else
        sed -i '' "$@"
    fi
}

# Usage
sed_inplace 's/old/new/g' file.txt
```

**Option 3: Use `sponge` from moreutils**

```bash
# ✅ Avoids sed -i entirely — requires moreutils
sed 's/old/new/g' file.txt | sponge file.txt
```

**Option 4: Temporary file pattern**

```bash
# ✅ Most portable — no special tools needed
sed 's/old/new/g' file.txt > file.txt.tmp && mv file.txt.tmp file.txt
```

---

## Backtick Escaping in Output

### The Problem

Backticks in shell have special meaning (command substitution). When outputting code examples containing backticks, escaping gets confusing.

```bash
# ❌ CONFUSING: What does this actually output?
echo "Use \`grep\` to search"
# Output varies by context and escaping levels

# ❌ WRONG: Escaping backticks in heredocs (unnecessary)
cat << 'EOF'
Use \`grep\` to search    # Outputs literal backslash!
EOF
```

### Best Practices

**Rule 1: Single-quoted strings don't need escaping**

```bash
# ✅ Single quotes preserve everything literally
echo 'Use `grep` to search files'
# Output: Use `grep` to search files
```

**Rule 2: Double quotes require escaping**

```bash
# ✅ Escape backticks in double quotes
echo "Use \`grep\` to search"
# Output: Use `grep` to search
```

**Rule 3: Quoted heredocs don't need escaping**

```bash
# ✅ Quoting EOF prevents ALL expansion — no escaping needed
cat << 'EOF'
Use `grep` to search
Variables like $HOME are literal
EOF
```

**Rule 4: Unquoted heredocs behave like double quotes**

```bash
# ✅ Unquoted EOF — variables expand, backticks execute
cat << EOF
Home is: $HOME
# Backticks would execute here — avoid or escape
EOF
```

**Summary Table**

| Context | Backtick Handling | Example |
|---------|-------------------|---------|
| Single quotes `'...'` | Literal, no escaping | `'Use cmd'` (backticks preserved) |
| Double quotes `"..."` | Must escape with `\` | `"Use \`cmd\`"` |
| Heredoc `<< 'EOF'` | Literal, no escaping | Just type backticks directly |
| Heredoc `<< EOF` | Executes — avoid or escape | Use quoted heredoc instead |

---

## Other Common Portability Issues

### `echo` Options

```bash
# ❌ NON-PORTABLE: -e flag not supported everywhere
echo -e "line1\nline2"    # Works on GNU, may print "-e" on BSD

# ✅ PORTABLE: Use printf instead
printf "line1\nline2\n"

# ✅ PORTABLE: Use $'...' for escape sequences
echo $'line1\nline2'
```

### Arrays

```bash
# ❌ NON-PORTABLE: Arrays are bashism
files=(*.txt)              # Fails in sh, dash, ash

# ✅ PORTABLE (if using bash): Declare bash explicitly
#!/bin/bash
files=(*.txt)

# ✅ PORTABLE (pure POSIX): Use positional parameters
set -- *.txt
for f in "$@"; do echo "$f"; done
```

### Test Brackets: `[[` vs `[`

```bash
# ❌ NON-PORTABLE: [[ is bashism
[[ $var == pattern* ]]    # Fails in sh, dash

# ✅ PORTABLE: Use [ with quotes
[ "$var" = "exact" ]      # POSIX compatible

# ✅ If you need pattern matching: Use case
case "$var" in
    pattern*) echo "matches" ;;
esac
```

### Process Substitution

```bash
# ❌ NON-PORTABLE: <() is bashism
diff <(cmd1) <(cmd2)      # Fails in sh, dash

# ✅ PORTABLE: Use temporary files
cmd1 > /tmp/out1.$$
cmd2 > /tmp/out2.$$
diff /tmp/out1.$$ /tmp/out2.$$
rm /tmp/out1.$$ /tmp/out2.$$
```

### `local` Variables

```bash
# ⚠️ MOSTLY PORTABLE: local works in bash, dash, ash, zsh
# But NOT in strict POSIX sh

local var="value"         # Use only if shebang is #!/bin/bash

# ✅ STRICT POSIX: Use subshell for scope isolation
(
    var="value"
    # var is local to subshell
)
```

### `source` vs `.`

```bash
# ❌ NON-PORTABLE: source is bashism
source ./script.sh

# ✅ PORTABLE: Use dot
. ./script.sh
```

---

## Best Practices

### 1. Use Explicit Shebang

```bash
#!/bin/bash              # For bash features (arrays, [[, etc.)
#!/bin/sh                # For POSIX-only scripts
#!/usr/bin/env bash      # For portability across $PATH locations
```

**CI Note:** Most CI environments have bash available. Use `#!/bin/bash` when you need bash features.

### 2. Enable Strict Mode

```bash
#!/bin/bash
set -euo pipefail        # Exit on error, undefined vars, pipe failures

# Or individually:
set -e                   # Exit immediately on error
set -u                   # Treat unset variables as errors
set -o pipefail          # Pipe fails if any command fails
```

### 3. Check for Dependencies

```bash
# ✅ Check before using
command -v jq >/dev/null 2>&1 || {
    echo "Error: jq is required but not installed" >&2
    exit 1
}

# ✅ Or with a helper function
require_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "Error: $1 is required but not installed" >&2
        exit 1
    }
}

require_cmd jq
require_cmd curl
```

### 4. Quote Variables

```bash
# ❌ DANGEROUS: Word splitting and globbing
rm $file                  # Breaks on spaces, expands globs
for f in $files; do       # Word splitting

# ✅ SAFE: Always quote
rm "$file"
for f in "${files[@]}"; do   # Bash array iteration
```

### 5. Use `$()` Not Backticks for Command Substitution

```bash
# ❌ AVOID: Backticks are harder to nest and read
result=`command`
nested=`echo \`inner\``

# ✅ PREFER: $() is cleaner and nestable
result=$(command)
nested=$(echo $(inner))
```

---

## Quick Reference

### Portable Patterns

| Task | ❌ Non-Portable | ✅ Portable |
|------|-----------------|-------------|
| In-place sed | `sed -i 's/.../g' file.txt` | `sed -i.bak 's/.../g' file.txt && rm file.txt.bak` |
| Newlines in echo | `echo -e "a\nb"` | `printf "a\nb\n"` |
| Pattern matching | `[[ $x == p* ]]` | `case "$x" in p*) ... ;; esac` |
| String comparison | `[[ $a == $b ]]` | `[ "$a" = "$b" ]` |
| Source script | `source file` | `. file` |
| Process substitution | `diff <(a) <(b)` | Use temp files |
| Arrays | `arr=(a b c)` | Use `#!/bin/bash` or positional params |
| Local variables | `local x=1` | Use `#!/bin/bash` or subshell |

### Platform Detection

```bash
# Detect OS
case "$(uname -s)" in
    Linux*)  OS=linux ;;
    Darwin*) OS=macos ;;
    MINGW*|CYGWIN*) OS=windows ;;
    *)       OS=unknown ;;
esac

# Detect GNU vs BSD coreutils
if sed --version 2>/dev/null | grep -q GNU; then
    SED_TYPE=gnu
else
    SED_TYPE=bsd
fi
```

### CI-Specific Notes

```yaml
# GitHub Actions: bash is available, use it explicitly
- name: Run script
  shell: bash           # Don't rely on default shell
  run: |
    set -euo pipefail
    ./scripts/my-script.sh
```

---

## Summary

1. **Always use `sed -i.bak` pattern** for portable in-place editing
2. **Use single quotes or quoted heredocs** to avoid backtick confusion
3. **Use `printf`** instead of `echo -e`
4. **Use explicit `#!/bin/bash`** when using bash features
5. **Quote all variables** to prevent word splitting
6. **Check for dependencies** before using external tools
7. **Set strict mode** (`set -euo pipefail`) at script start

---

*See also: [github-actions-best-practices.md](github-actions-best-practices.md) for CI-specific shell scripting guidance.*
