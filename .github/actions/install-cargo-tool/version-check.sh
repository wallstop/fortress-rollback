#!/usr/bin/env bash
# Shared version checks for the install-cargo-tool composite action.

resolve_cargo_home() {
  if [ -n "${CARGO_HOME:-}" ]; then
    printf '%s\n' "${CARGO_HOME//\\//}"
    return 0
  fi

  if [ -n "${HOME:-}" ]; then
    printf '%s\n' "${HOME//\\//}/.cargo"
    return 0
  fi

  echo "ERROR: neither CARGO_HOME nor HOME is set" >&2
  return 1
}

cargo_tool_cache_glob() {
  local tool_name="$1"
  local cargo_home_value="${2:-}"

  if [ -z "$tool_name" ]; then
    echo "ERROR: tool name is required" >&2
    return 1
  fi

  if [ -z "$cargo_home_value" ]; then
    cargo_home_value="$(resolve_cargo_home)"
  fi

  cargo_home_value="${cargo_home_value//\\//}"
  cargo_home_value="${cargo_home_value%/}"
  printf '%s/bin/%s*\n' "$cargo_home_value" "$tool_name"
}

primary_version_output_matches_required() {
  local output="$1"
  local tool_name="$2"
  local required="$3"
  [ -z "$required" ] && return 0

  awk -v tool="$tool_name" -v required="$required" '
    BEGIN {
      saw_nonempty = 0
    }
    function normalize(version) {
      sub(/^v/, "", version)
      sub(/:$/, "", version)
      sub(/,$/, "", version)
      sub(/;$/, "", version)
      return version
    }
    /^[[:space:]]*$/ {
      next
    }
    {
      saw_nonempty = 1
      first = $1
      version = normalize($2)
      exit ((first == tool || first == "cargo-" tool || first == tool ".exe") && version == required) ? 0 : 1
    }
    END {
      if (saw_nonempty == 0) {
        exit 1
      }
    }
  ' <<< "$output"
}

cargo_install_list_entry_matches() {
  local tool_name="$1"
  local required="$2"
  [ -z "$required" ] && return 0

  cargo install --list 2>/dev/null | awk -v tool="$tool_name" -v required="$required" '
    $1 == tool {
      version = $2
      sub(/^v/, "", version)
      sub(/:$/, "", version)
      if (version == required) {
        found = 1
      }
    }
    END {
      exit found ? 0 : 1
    }
  '
}

installed_version_matches() {
  local tool_name="$1"
  local output="$2"
  local required="$3"
  [ -z "$required" ] && return 0

  # If the target binary reports a version, trust only that target-specific
  # output. Falling back after a mismatch can hide a stale binary on PATH.
  if [ -n "$output" ]; then
    primary_version_output_matches_required "$output" "$tool_name" "$required"
    return
  fi

  # Some tools do not expose --version consistently. In that case, accept the
  # cargo-install database only when the requested package entry matches.
  cargo_install_list_entry_matches "$tool_name" "$required"
}
