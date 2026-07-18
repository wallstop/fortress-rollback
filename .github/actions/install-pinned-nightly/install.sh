#!/usr/bin/env bash
# Install the repository's generic CI nightly with bounded network retries.

set -euo pipefail

pin_file_name="${PIN_FILE_NAME:-toolchain}"
case "$pin_file_name" in
  toolchain|miri-toolchain) ;;
  *)
    echo "::error::Unsupported pinned-nightly file '$pin_file_name'."
    exit 1
    ;;
esac

pin_file="${GITHUB_ACTION_PATH}/${pin_file_name}"
if [ ! -f "$pin_file" ]; then
  echo "::error::Pinned-nightly file is missing: $pin_file"
  exit 1
fi

toolchain=""
extra_line=""
{
  # Avoid Bash-4-only bulk line readers because macOS ships Bash 3.2. Accept a final
  # line without a newline, but reject every second logical line (even empty).
  if ! IFS= read -r toolchain && [ -z "$toolchain" ]; then
    echo "::error::Pinned-nightly file must contain exactly one line: $pin_file"
    exit 1
  fi
  if IFS= read -r extra_line || [ -n "$extra_line" ]; then
    echo "::error::Pinned-nightly file must contain exactly one line: $pin_file"
    exit 1
  fi
} < "$pin_file"
if [ -z "$toolchain" ]; then
  echo "::error::Pinned-nightly file must contain exactly one line: $pin_file"
  exit 1
fi
toolchain="${toolchain%$'\r'}"
if [[ ! "$toolchain" =~ ^nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
  echo "::error::Pinned nightly must be exactly nightly-YYYY-MM-DD; got '$toolchain'."
  exit 1
fi

install_args=(--profile minimal --no-self-update)

# Parse with scalar state rather than read -a. macOS ships Bash 3.2, where expanding
# an empty array under `set -u` fails even when the array was explicitly initialized.
append_requested_items() {
  local kind="$1"
  local remaining="$2"
  local item=""
  local has_more="false"

  while :; do
    case "$remaining" in
      *,*)
        item="${remaining%%,*}"
        remaining="${remaining#*,}"
        has_more="true"
        ;;
      *)
        item="$remaining"
        remaining=""
        has_more="false"
        ;;
    esac

    item="${item#"${item%%[![:space:]]*}"}"
    item="${item%"${item##*[![:space:]]}"}"
    if [ -n "$item" ]; then
      if [[ ! "$item" =~ ^[A-Za-z0-9._-]+$ ]]; then
        echo "::error::Invalid rustup $kind '$item'."
        exit 1
      fi
      install_args+=("--$kind" "$item")
    fi

    if [ "$has_more" != "true" ]; then
      break
    fi
  done
}

append_requested_items "component" "${REQUESTED_COMPONENTS:-}"
append_requested_items "target" "${REQUESTED_TARGETS:-}"

echo "=== Pinned nightly installation ==="
echo "toolchain=$toolchain"
echo "components=${REQUESTED_COMPONENTS:-<none>}"
echo "targets=${REQUESTED_TARGETS:-<none>}"
rustup --version

installed=false
for attempt in 1 2 3; do
  echo "Installing $toolchain (attempt $attempt/3)"
  if rustup toolchain install "$toolchain" "${install_args[@]}"; then
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

echo "=== Installed nightly diagnostics ==="
rustup run "$toolchain" rustc --version --verbose
rustup run "$toolchain" cargo --version --verbose
rustup component list --installed --toolchain "$toolchain"
rustup target list --installed --toolchain "$toolchain"
