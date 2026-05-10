#!/usr/bin/env bash
# Shared version checks for the install-cargo-tool composite action.

primary_version_output_matches_required() {
  local output="$1"
  local tool_name="$2"
  local required="$3"
  [ -z "$required" ] && return 0

  awk -v tool="$tool_name" -v required="$required" '
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
      first = $1
      version = normalize($2)
      exit ((first == tool || first == "cargo-" tool || first == tool ".exe") && version == required) ? 0 : 1
    }
    END {
      if (NR == 0) {
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
