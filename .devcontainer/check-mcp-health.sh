#!/usr/bin/env bash
# Report project MCP server health without displaying credential values.
#
# The report uses the actual process environment that clients receive, checks it
# against the filtered runtime file, probes each configured endpoint with an MCP
# initialize request, and starts Vision through its wrapper for a stdio handshake.
#
# Exit codes:
#   0 - every configured server responded
#   1 - a configured server failed, or the runtime environment is unsafe
#   2 - no MCP credentials are configured

set -uo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
workspace_dir="$(dirname "${script_dir}")"
timeout_seconds=20
offline=0
prefix="[devcontainer:mcp]"

readonly ZAI_WEB_SEARCH_URL="https://api.z.ai/api/mcp/web_search_prime/mcp"
readonly ZAI_WEB_READER_URL="https://api.z.ai/api/mcp/web_reader/mcp"
readonly ZAI_ZREAD_URL="https://api.z.ai/api/mcp/zread/mcp"
readonly GITHUB_MCP_URL="https://api.githubcopilot.com/mcp/"
readonly INITIALIZE_REQUEST='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"fortress-mcp-health","version":"1"}}}'

usage() {
    cat <<'HELP'
Usage: bash .devcontainer/check-mcp-health.sh [options]

Options:
  --workspace DIR    Workspace root containing .devcontainer/.mcp.env
  --timeout SECONDS  Per-probe time limit (default: 20)
  --offline          Validate configuration only; skip network and stdio probes
  -h, --help         Show this help

Exit codes: 0 healthy, 1 failure, 2 credentials not configured.
HELP
}

emit() {
    printf '%s %s\n' "${prefix}" "$*"
}

fail() {
    printf '%s ERROR: %s\n' "${prefix}" "$*" >&2
    exit 1
}

is_positive_integer() {
    case "$1" in
        ''|*[!0-9]*) return 1 ;;
        *) [ "$1" -gt 0 ] ;;
    esac
}

run_with_timeout() {
    local limit="$1"
    shift

    if command -v timeout >/dev/null 2>&1; then
        timeout "${limit}" "$@"
    elif command -v gtimeout >/dev/null 2>&1; then
        gtimeout "${limit}" "$@"
    else
        "$@"
    fi
}

# Keep diagnostics short and ASCII-safe for a terminal table.
first_line() {
    local text="$1"
    printf '%s' "${text}" | tr '\r\n' '  ' | cut -c1-140
}

report_row() {
    local name="$1" transport="$2" state="$3" detail="${4:-}"
    printf '%-21s %-7s %-10s %s\n' "${name}" "${transport}" "${state}" "${detail}"
}

# Never echo a value that appears in the probe output.
safe_detail() {
    local text="$1" token="$2"
    case "${text}" in
        *"${token}"*) printf 'response redacted; it contained the credential' ;;
        *) first_line "${text}" ;;
    esac
}

finish() {
    local failures="$1"
    if [ "${failures}" -gt 0 ]; then
        emit "STATUS: ${failures} configured server(s) failed."
        exit 1
    fi
    if [ "${offline}" = "1" ]; then
        emit "STATUS: configuration looks complete; probes were skipped."
        exit 0
    fi
    emit "STATUS: every configured MCP server responded."
    exit 0
}

report_unconfigured() {
    emit "Project MCP servers are not configured; the runtime environment has no credentials."
    emit "Copy .env.local.example to .env.local, fill the keys you have, and rebuild the devcontainer."
    emit "A container restart is not enough: Docker reads the environment file at creation time."
    exit 2
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --workspace)
            [ "$#" -ge 2 ] || fail "--workspace requires a directory."
            workspace_dir="$2"
            shift 2
            ;;
        --timeout)
            [ "$#" -ge 2 ] || fail "--timeout requires a value."
            timeout_seconds="$2"
            shift 2
            ;;
        --offline)
            offline=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            fail "Unknown argument: $1 (run with --help)."
            ;;
    esac
done

is_positive_integer "${timeout_seconds}" \
    || fail "--timeout must be a positive integer."
[ -d "${workspace_dir}" ] || fail "Workspace directory does not exist."
[ "$(uname -s)" = "Linux" ] \
    || fail "MCP health checks require the Linux devcontainer."

runtime_env="${workspace_dir}/.devcontainer/.mcp.env"
github_token="${GITHUB_PERSONAL_ACCESS_TOKEN:-}"
zai_token="${ZAI_API_KEY:-}"

# Process credentials use the same restricted syntax as the generated env file.
# Reject malformed values before placing them in a curl config or wrapper env.
for key in GITHUB_PERSONAL_ACCESS_TOKEN ZAI_API_KEY; do
    value="${!key:-}"
    case "${value}" in
        *[[:space:]\"\']*)
            fail "${key} in the process environment contains quotes or whitespace; update .env.local and rebuild."
            ;;
    esac
done

if [ ! -e "${runtime_env}" ]; then
    if [ -n "${github_token}" ] || [ -n "${zai_token}" ]; then
        fail "Process credentials exist but .mcp.env is missing; rebuild required (a restart is not enough)."
    fi
    report_unconfigured
fi
if [ -L "${runtime_env}" ] || [ ! -f "${runtime_env}" ]; then
    emit "ERROR: ${runtime_env} must be a regular file."
    exit 1
fi
runtime_mode="$(stat -c '%a' "${runtime_env}")" \
    || fail "Could not read ${runtime_env} permissions."
if [ "${runtime_mode}" != "600" ]; then
    fail "Unsafe runtime environment: ${runtime_env} mode is ${runtime_mode}; expected 600. Rebuild required."
fi

# Returns the credential on stdout, or nothing when the key is absent or blank.
# Diagnostics go to stderr so command substitution captures only the value.
read_runtime_key() {
    local key="$1" definition_count value
    definition_count="$({ grep -Ec "^${key}=" "${runtime_env}" || true; })"
    if [ "${definition_count}" = "0" ]; then
        return 1
    fi
    if [ "${definition_count}" != "1" ]; then
        printf '%s ERROR: %s appears more than once in .env.local; rebuild after fixing it.\n' \
            "${prefix}" "${key}" >&2
        return 2
    fi
    value="$(sed -n "s/^${key}=//p" "${runtime_env}")"
    value="${value%$'\r'}"
    case "${value}" in
        '') return 1 ;;
        *[[:space:]\"\']*)
            printf '%s ERROR: %s is not Docker env-file syntax; rebuild after fixing .env.local.\n' \
                "${prefix}" "${key}" >&2
            return 2
            ;;
    esac
    printf '%s' "${value}"
}

github_file_token="$(read_runtime_key GITHUB_PERSONAL_ACCESS_TOKEN)"
github_status=$?
zai_file_token="$(read_runtime_key ZAI_API_KEY)"
zai_status=$?

if [ "${github_status}" = "2" ] || [ "${zai_status}" = "2" ]; then
    exit 1
fi
if [ "${github_file_token}" != "${github_token}" ] \
    || [ "${zai_file_token}" != "${zai_token}" ]; then
    fail ".mcp.env does not match the actual process environment; rebuild required (a restart is not enough)."
fi
if [ -z "${github_token}" ] && [ -z "${zai_token}" ]; then
    report_unconfigured
fi

emit "Runtime environment: actual process environment (matches ${runtime_env}, mode ${runtime_mode})"
emit "GITHUB_PERSONAL_ACCESS_TOKEN: $( [ -n "${github_token}" ] && printf 'set' || printf 'missing' )"
emit "ZAI_API_KEY: $( [ -n "${zai_token}" ] && printf 'set' || printf 'missing' )"
if [ "${offline}" = "1" ]; then
    emit "Offline mode: endpoint and stdio probes are skipped."
fi
printf '\n'
printf '%-21s %-7s %-10s %s\n' "SERVER" "TRANSPORT" "STATE" "DETAIL"
failures=0
if [ "${offline}" != "1" ] && ! command -v jq >/dev/null 2>&1; then
    fail "jq is required to validate MCP responses; rebuild the devcontainer."
fi

# Return "ok", "error", or "invalid" plus a non-secret detail. Accept one JSON
# response or SSE data records, classify JSON-RPC errors before successes, and
# require the matching initialize result to contain protocolVersion, capabilities,
# and serverInfo.
classify_initialize_response() {
    local body="$1" payloads

    if printf '%s' "${body}" | jq -e 'type == "object"' >/dev/null 2>&1; then
        payloads="${body}"
    else
        payloads="$(printf '%s\n' "${body}" | sed -n 's/^data:[[:space:]]*//p')"
    fi
    if [ -z "${payloads}" ]; then
        printf 'invalid\tno JSON-RPC response\n'
        return 0
    fi

    printf '%s\n' "${payloads}" | jq -r -s '
        (map(select(
            type == "object" and .jsonrpc == "2.0" and .id == 1
            and (.error? != null)
        )) | first) as $error
        | if $error != null then
            ["error", (
                if ($error.error | type) == "object"
                    and ($error.error.code | type) == "number"
                then $error.error.code | tostring
                else "unknown"
                end
            )] | @tsv
        else
            (map(select(
                type == "object" and .jsonrpc == "2.0" and .id == 1
                and (.result | type) == "object"
                and (.result.protocolVersion | type) == "string"
                and (.result.protocolVersion | length) > 0
                and (.result.capabilities | type) == "object"
                and (.result.serverInfo | type) == "object"
                and (.result.serverInfo.name | type) == "string"
                and (.result.serverInfo.name | length) > 0
                and (.result.serverInfo.version | type) == "string"
                and (.result.serverInfo.version | length) > 0
            )) | first) as $success
            | if $success != null then
                ["ok", "valid initialize result"] | @tsv
            else
                ["invalid", "no valid initialize result fields"] | @tsv
            end
        end
    ' 2>/dev/null || printf 'invalid\tinvalid JSON-RPC response\n'
}

# Probe one HTTP MCP endpoint with the credential a client would send. The
# token travels through a curl config file on stdin so it never reaches argv.
probe_http() {
    local name="$1" url="$2" token="$3"
    local error_file response http_status body curl_status state detail
    local classification response_kind response_detail

    if [ "${offline}" = "1" ]; then
        report_row "${name}" http "NOT PROBED" "offline mode"
        return 0
    fi

    error_file="$(mktemp)"
    if [ -z "${error_file}" ]; then
        report_row "${name}" http "FAIL" "could not create a temporary file"
        failures=$((failures + 1))
        return 0
    fi

    response="$(
        curl --silent --show-error --max-time "${timeout_seconds}" \
            --request POST \
            --header 'Content-Type: application/json' \
            --header 'Accept: application/json, text/event-stream' \
            --data "${INITIALIZE_REQUEST}" \
            --write-out '\n%{http_code}' \
            --config - 2>"${error_file}" <<EOF
url = "${url}"
header = "Authorization: Bearer ${token}"
EOF
    )"
    curl_status=$?
    if [ "${curl_status}" -ne 0 ]; then
        detail="$(safe_detail "$(head -n 1 "${error_file}")" "${token}")"
        rm -f -- "${error_file}"
        report_row "${name}" http "NETWORK" "${detail:-curl exit ${curl_status}}"
        failures=$((failures + 1))
        return 0
    fi
    rm -f -- "${error_file}"

    http_status="${response##*$'\n'}"
    body="${response%$'\n'*}"

    state="FAIL"
    detail=""
    case "${http_status}" in
        401|403)
            state="AUTH"
            detail="HTTP ${http_status}: the credential was rejected"
            ;;
        400)
            if printf '%s' "${body}" | grep -Eqi 'badly formatted|authorization'; then
                state="AUTH"
                detail="HTTP 400: the credential is malformed"
            else
                detail="HTTP 400: $(safe_detail "${body}" "${token}")"
            fi
            ;;
        200|201|202)
            classification="$(classify_initialize_response "${body}")"
            response_kind="${classification%%$'\t'*}"
            response_detail="${classification#*$'\t'}"
            case "${response_kind}" in
                ok)
                    state="OK"
                    detail="initialize accepted"
                    ;;
                error)
                    case "${response_detail}" in
                        401|403|1001)
                            state="AUTH"
                            detail="the endpoint rejected the credential"
                            ;;
                        *)
                            detail="MCP initialize returned JSON-RPC error ${response_detail}"
                            ;;
                    esac
                    ;;
                *)
                    detail="$(safe_detail "${body}" "${token}")"
                    if [ -z "${detail}" ]; then
                        detail="${response_detail}"
                    fi
                    ;;
            esac
            ;;
        [0-9][0-9][0-9])
            detail="HTTP ${http_status}: $(safe_detail "${body}" "${token}")"
            ;;
        *)
            state="NETWORK"
            detail="no HTTP status returned"
            ;;
    esac

    if [ "${state}" != "OK" ]; then
        failures=$((failures + 1))
    fi
    report_row "${name}" http "${state}" "${detail}"
}

# Start the Vision server through the shipped wrapper and wait for one MCP
# initialize response. This confirms the wrapper, package, and key plumbing
# without spending API quota.
probe_vision() {
    local token="$1" error_file response detail vision_status
    local classification response_kind response_detail

    if [ "${offline}" = "1" ]; then
        report_row "zai-vision" stdio "NOT PROBED" "offline mode"
        return 0
    fi
    if ! command -v zai-mcp-vision >/dev/null 2>&1; then
        report_row "zai-vision" stdio "FAIL" "zai-mcp-vision is missing; rebuild the devcontainer"
        failures=$((failures + 1))
        return 0
    fi
    if ! command -v zai-mcp-server >/dev/null 2>&1; then
        report_row "zai-vision" stdio "FAIL" "zai-mcp-server is missing; rebuild the devcontainer"
        failures=$((failures + 1))
        return 0
    fi

    error_file="$(mktemp)"
    if [ -z "${error_file}" ]; then
        report_row "zai-vision" stdio "FAIL" "could not create a temporary file"
        failures=$((failures + 1))
        return 0
    fi

    response="$(
        printf '%s\n' "${INITIALIZE_REQUEST}" \
            | ZAI_API_KEY="${token}" \
                run_with_timeout "${timeout_seconds}" zai-mcp-vision \
                2>"${error_file}" \
            | head -n 1
    )"
    vision_status=$?
    classification="$(classify_initialize_response "${response}")"
    response_kind="${classification%%$'\t'*}"
    response_detail="${classification#*$'\t'}"
    if [ "${response_kind}" = "ok" ]; then
        report_row "zai-vision" stdio "OK" "initialize accepted"
        rm -f -- "${error_file}"
        return 0
    fi

    if [ "${response_kind}" = "error" ]; then
        detail="MCP initialize returned JSON-RPC error ${response_detail}"
    else
        detail="$(safe_detail "$(head -n 1 "${error_file}")" "${token}")"
        if [ "${vision_status}" -ne 0 ]; then
            detail="stdio probe failed or timed out (status ${vision_status})${detail:+: ${detail}}"
        fi
    fi
    rm -f -- "${error_file}"
    report_row "zai-vision" stdio "FAIL" "${detail:-no MCP initialize response}"
    failures=$((failures + 1))
}

if [ -n "${github_token}" ]; then
    probe_http "github-mcp-server" "${GITHUB_MCP_URL}" "${github_token}"
else
    report_row "github-mcp-server" http "DISABLED" "GITHUB_PERSONAL_ACCESS_TOKEN is not set"
fi

if [ -n "${zai_token}" ]; then
    probe_http "zai-web-search" "${ZAI_WEB_SEARCH_URL}" "${zai_token}"
    probe_http "zai-web-reader" "${ZAI_WEB_READER_URL}" "${zai_token}"
    probe_http "zai-zread" "${ZAI_ZREAD_URL}" "${zai_token}"
    probe_vision "${zai_token}"
else
    report_row "zai-web-search" http "DISABLED" "ZAI_API_KEY is not set"
    report_row "zai-web-reader" http "DISABLED" "ZAI_API_KEY is not set"
    report_row "zai-zread" http "DISABLED" "ZAI_API_KEY is not set"
    report_row "zai-vision" stdio "DISABLED" "ZAI_API_KEY is not set"
fi

printf '\n'
if [ "${failures}" -gt 0 ]; then
    emit "Hint: AUTH rows usually mean the key is wrong, expired, or lacks plan access; NETWORK rows point at the host uplink or proxy."
fi
finish "${failures}"
