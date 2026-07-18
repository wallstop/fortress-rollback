#!/usr/bin/env bash
# Install the repository's exact stable release compiler with bounded retries.

set -euo pipefail

readonly U64_MAX="18446744073709551615"
pin_file="${GITHUB_ACTION_PATH}/toolchain"

if [ ! -f "$pin_file" ]; then
  echo "::error::Pinned release-toolchain file is missing: $pin_file"
  exit 1
fi

toolchain=""
extra_line=""
{
  # Avoid Bash-4-only bulk line readers because macOS ships Bash 3.2. Accept a final
  # line without a newline, but reject every second logical line (even empty).
  if ! IFS= read -r toolchain && [ -z "$toolchain" ]; then
    echo "::error::Pinned release-toolchain file must contain exactly one line: $pin_file"
    exit 1
  fi
  if IFS= read -r extra_line || [ -n "$extra_line" ]; then
    echo "::error::Pinned release-toolchain file must contain exactly one line: $pin_file"
    exit 1
  fi
} < "$pin_file"
toolchain="${toolchain%$'\r'}"

is_u64_component() {
  local value="$1"

  case "$value" in
    0) return 0 ;;
    ''|0*|*[!0-9]*) return 1 ;;
  esac

  if [ "${#value}" -lt "${#U64_MAX}" ]; then
    return 0
  fi
  if [ "${#value}" -gt "${#U64_MAX}" ]; then
    return 1
  fi
  LC_ALL=C
  # Equal-width decimal strings preserve numeric order; shell arithmetic cannot
  # represent the upper half of u64.
  # shellcheck disable=SC2071
  if [[ "$value" > "$U64_MAX" ]]; then
    return 1
  fi
  return 0
}

if [[ ! "$toolchain" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  echo "::error::Pinned release toolchain must be exactly stable X.Y.Z; got '$toolchain'."
  exit 1
fi
major="${BASH_REMATCH[1]}"
minor="${BASH_REMATCH[2]}"
patch="${BASH_REMATCH[3]}"
if ! is_u64_component "$major" || \
   ! is_u64_component "$minor" || \
   ! is_u64_component "$patch"; then
  echo "::error::Pinned release toolchain components must be canonical u64 values; got '$toolchain'."
  exit 1
fi

echo "=== Pinned release-toolchain installation ==="
echo "toolchain=$toolchain"
rustup --version

installed=false
for attempt in 1 2 3; do
  echo "Installing $toolchain (attempt $attempt/3)"
  if rustup toolchain install "$toolchain" --profile minimal --no-self-update; then
    installed=true
    break
  fi
  if [ "$attempt" -lt 3 ]; then
    echo "::warning::rustup installation attempt $attempt failed; retrying in 10 seconds."
    sleep 10
  fi
done

if [ "$installed" != "true" ]; then
  echo "::error::Failed to install $toolchain after 3 attempts."
  rustup show || true
  exit 1
fi

echo "RUSTUP_TOOLCHAIN=$toolchain" >> "$GITHUB_ENV"
echo "toolchain=$toolchain" >> "$GITHUB_OUTPUT"

echo "=== Installed release-toolchain diagnostics ==="
rustup run "$toolchain" rustc --version --verbose
rustup run "$toolchain" cargo --version --verbose
