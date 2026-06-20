#!/bin/bash
# Doc Comment Accuracy Check for Fortress Rollback
#
# Checks that doc comments make accurate claims for safety-sensitive APIs:
# - "downcast" docs must be backed by actual downcasting infrastructure
# - bounded decode docs must not point callers at unbounded decode helpers
# - `codec::decode_*` references in docs/allocation comments must name real helpers
# - Rustdoc range contracts and test names must match implementation clues
#
# Usage: ./scripts/ci/check-doc-claims.sh [options]
#   ./scripts/ci/check-doc-claims.sh            # Check all Rust files
#   ./scripts/ci/check-doc-claims.sh --verbose  # Show all files checked
#   ./scripts/ci/check-doc-claims.sh --help     # Show help
#
# Exit codes:
#   0 - No issues found
#   1 - Misleading doc comments detected

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Options
VERBOSE=false

print_usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --verbose  Show all files checked"
    echo "  --help     Show this help message"
    echo ""
    echo "Checks doc comments and test names for misleading semantic claims."
}

# Guard against reintroducing removed floor-gossip identifiers as CURRENT-production
# claims in docs/comments. The floor-round work (S55) removed two identifiers from
# production: `UdpProtocol::peer_pessimistic_floor` (replaced by `round_floor`) and the
# `Input::pessimistic_floor` wire field (replaced by `FloorReply::floors`). Any tracked
# `*.md`/`*.tla`/`*.cfg`/`*.rs` line that names one of them MUST carry a same-line
# historical qualifier (legacy/formerly/removed/pre-S55/old/S50-54-era); otherwise it is
# read as a false current-production claim and fails the check. POSIX ERE only (no
# \b/\s/\w): token boundaries use [^[:alnum:]_]; the qualifier match is case-insensitive.
check_removed_floor_identifiers() {
    local project_root="$1"

    # The two removed identifiers, as POSIX-ERE token patterns:
    #   - the removed peer_pessimistic_floor   (NOT followed by an identifier char, e.g. no "...floors")
    #   - Input.pessimistic_floor / Input::pessimistic_floor   (the old per-Input wire field;
    #     deliberately excludes the CURRENT `P2PSession::pessimistic_floors` plural and the
    #     `pessimistic_floor_relay_topology` / `send_floor_reply` helpers).
    local removed_pattern='(^|[^[:alnum:]_])(peer_pessimistic_floor|Input(\.|::)pessimistic_floor)([^[:alnum:]_]|$)'

    # Same-line historical qualifiers that mark a mention as legitimate narrative.
    # `replace` also covers `replaced`/`replaces` ("X (replaced by Y)" is a qualified
    # historical mention) and keeps this file's own descriptive comments in the clear
    # were `*.sh` ever brought into scope.
    local qualifier_pattern='(pre-s55|legacy|formerly|former|removed|remove|replace|no-longer|no longer|old|s5[0-4]|historic)'

    # Scan TRACKED docs/comments only. Gitignored working notes (e.g. the audit and
    # `progress/` session logs) legitimately narrate the full development history,
    # including the removed identifiers, so they are out of scope. Prefer `git ls-files`;
    # fall back to `find` only when not inside a git work tree (e.g. test fixtures).
    local file_list
    if git -C "$project_root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        file_list=$(git -C "$project_root" ls-files -- '*.md' '*.tla' '*.cfg' '*.rs' 2>/dev/null \
            | sed "s#^#$project_root/#" | sort)
    else
        file_list=$(find "$project_root" \
            \( -path "$project_root/target" -o -path "$project_root/.git" -o -path "$project_root/.tla-tools" \) -prune -o \
            \( -name '*.md' -o -name '*.tla' -o -name '*.cfg' -o -name '*.rs' \) -type f -print 2>/dev/null \
            | sort)
    fi

    local found=0
    local rel_path
    local file
    while IFS= read -r file; do
        [[ -z "$file" ]] && continue
        [[ -f "$file" ]] || continue
        rel_path="${file#"$project_root/"}"

        local hits
        hits=$(grep -nE "$removed_pattern" "$file" 2>/dev/null || true)
        [[ -z "$hits" ]] && continue

        local hit_line
        while IFS= read -r hit_line; do
            [[ -z "$hit_line" ]] && continue
            # Strip the leading "<lineno>:" to test the source text for a qualifier.
            local hit_text="${hit_line#*:}"
            if printf '%s' "$hit_text" | grep -qiE "$qualifier_pattern"; then
                continue
            fi
            if [[ "$found" -eq 0 ]]; then
                echo ""
                echo -e "${RED}ERROR${NC}: removed floor identifier(s) claimed as current production"
            fi
            found=$((found + 1))
            echo -e "  ${YELLOW}$rel_path:${hit_line}${NC}"
            echo -e "    ${BLUE}Removed:${NC} peer_pessimistic_floor -> UdpProtocol::round_floor; Input::pessimistic_floor -> FloorReply::floors"
            echo -e "    ${BLUE}Fix:${NC} use the current identifier, or qualify the mention as historical (legacy/formerly/removed/pre-S55)."
        done <<< "$hits"
    done <<< "$file_list"

    if [[ "$found" -gt 0 ]]; then
        return 1
    fi
    echo -e "${GREEN}OK${NC}: no removed floor identifiers claimed as current production."
    return 0
}

main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --verbose)
                VERBOSE=true
                shift
                ;;
            --help)
                print_usage
                exit 0
                ;;
            *)
                echo "Unknown argument: $1"
                print_usage
                exit 1
                ;;
        esac
    done

    echo "=========================================="
    echo " Doc Comment Accuracy Check"
    echo "=========================================="
    echo ""

    local codec_file="$PROJECT_ROOT/src/network/codec.rs"
    local codec_decode_helpers=""
    if [[ -f "$codec_file" ]]; then
        codec_decode_helpers=$(grep -nE '^[[:space:]]*(pub([[:space:]]*\([^)]*\))?[[:space:]]+)?fn[[:space:]]+decode[[:alnum:]_]*' "$codec_file" 2>/dev/null \
            | sed -E 's/.*fn[[:space:]]+(decode[[:alnum:]_]*).*/\1/' \
            | sort -u)
    fi

    # Patterns that indicate actual downcasting infrastructure.
    # Use POSIX ERE-safe token boundaries for portability across GNU/BSD grep.
    # If a file mentions downcasting in docs, it should contain at least one of these.
    local downcast_infra_patterns='(as_any|downcast_ref|downcast_mut|dyn Any|: Any|impl Any|Any \+|Any\+|\.downcast([^[:alnum:]_]|$))'

    local issues=0
    local files_with_downcast_claims=0
    local files_with_bounded_decode_claims=0
    local files_with_codec_decode_refs=0

    # Find all Rust source files (excluding target directories)
    while IFS= read -r file; do
        [[ -z "$file" ]] && continue

        local rel_path="${file#"$PROJECT_ROOT/"}"

        # Find doc comment lines mentioning "downcast" (case-insensitive)
        local doc_matches
        doc_matches=$(grep -niE '^[[:space:]]*///.*downcast|^[[:space:]]*//!.*downcast' "$file" 2>/dev/null || true)

        if [[ -z "$doc_matches" ]]; then
            if [[ "$VERBOSE" == "true" ]]; then
                echo -e "  ${GREEN}OK${NC}: $rel_path (no downcast doc claims)"
            fi
        else
            files_with_downcast_claims=$((files_with_downcast_claims + 1))

            if [[ "$VERBOSE" == "true" ]]; then
                echo -e "  ${YELLOW}Checking${NC}: $rel_path (has downcast doc claims)"
            fi

            # Check if the file has actual downcasting infrastructure
            local infra_matches
            infra_matches=$(grep -nE "$downcast_infra_patterns" "$file" 2>/dev/null || true)

            local has_infra
            has_infra=$(echo "$infra_matches" | grep -cE '^[0-9]+:' || true)
            has_infra=${has_infra:-0}

            if [[ "$has_infra" -eq 0 ]]; then
                issues=$((issues + 1))
                echo ""
                echo -e "${RED}ERROR${NC}: $rel_path"
                echo -e "  Doc comments mention \"downcast\" but no downcasting infrastructure found."
                echo -e "  ${YELLOW}Doc comment(s):${NC}"
                while IFS= read -r match_line; do
                    echo -e "    $match_line"
                done <<< "$doc_matches"
                echo -e "  ${BLUE}Expected one of:${NC} as_any, downcast_ref, downcast_mut, dyn Any, : Any"
                echo -e "  ${BLUE}Fix:${NC} Either add downcasting support or update the doc comment"
                echo -e "       to accurately describe the actual pattern used."
            else
                if [[ "$VERBOSE" == "true" ]]; then
                    echo -e "    ${GREEN}OK${NC}: downcasting infrastructure found ($has_infra occurrence(s))"
                    echo "$infra_matches" | head -3 | sed 's/^/      match: /'
                fi
            fi
        fi

        local bounded_decode_doc_matches=""
        local doc_block=""
        local doc_start=0
        local has_doc=false
        local line_no=0
        local source_line
        while IFS= read -r source_line || [[ -n "$source_line" ]]; do
            line_no=$((line_no + 1))

            if [[ "$source_line" =~ ^[[:space:]]*/// ]]; then
                if [[ "$has_doc" == "false" ]]; then
                    doc_start=$line_no
                    doc_block=""
                fi
                doc_block+="$source_line"$'\n'
                has_doc=true
                continue
            fi

            if [[ "$source_line" =~ ^[[:space:]]*#\[ ]]; then
                continue
            fi

            if [[ "$source_line" == *"fn decode_bounded"* && "$source_line" != *"fn decode_bounded_with_consumed"* ]]; then
                local lower_doc
                lower_doc=$(printf '%s' "$doc_block" | tr '[:upper:]' '[:lower:]')
                local has_unbounded_decode_ref=false
                if printf '%s' "$doc_block" | grep -Eq '`((crate::network::)?codec::)?decode`'; then
                    has_unbounded_decode_ref=true
                fi
                if [[ "$has_doc" == "true" &&
                    "$lower_doc" == *"consumed"* &&
                    "$lower_doc" == *"length"* &&
                    "$has_unbounded_decode_ref" == "true" &&
                    "$doc_block" != *"decode_bounded_with_consumed"* ]]; then
                    bounded_decode_doc_matches+="${doc_start}: decode_bounded docs direct consumed-length callers to unbounded decode"$'\n'
                fi
            fi

            doc_block=""
            doc_start=0
            has_doc=false
        done < "$file"

        if [[ -n "$bounded_decode_doc_matches" ]]; then
            files_with_bounded_decode_claims=$((files_with_bounded_decode_claims + 1))
            issues=$((issues + 1))
            echo ""
            echo -e "${RED}ERROR${NC}: $rel_path"
            echo -e "  decode_bounded docs mention consumed length but point at unbounded decode."
            echo -e "  ${YELLOW}Doc claim(s):${NC}"
            while IFS= read -r match_line; do
                echo -e "    $match_line"
            done <<< "$bounded_decode_doc_matches"
            echo -e "  ${BLUE}Fix:${NC} Reference decode_bounded_with_consumed for bounded consumed-length decoding."
        fi

        local codec_decode_ref_matches
        codec_decode_ref_matches=$(grep -nE '^[[:space:]]*(///|//!|// alloc-bound:).*codec::decode_[[:alnum:]_]+' "$file" 2>/dev/null || true)

        if [[ -n "$codec_decode_ref_matches" ]]; then
            files_with_codec_decode_refs=$((files_with_codec_decode_refs + 1))
            while IFS= read -r ref_line; do
                [[ -z "$ref_line" ]] && continue

                local ref_lineno="${ref_line%%:*}"
                local ref_text="${ref_line#*:}"
                local decode_refs
                decode_refs=$(echo "$ref_text" | grep -oE 'codec::decode_[[:alnum:]_]+' | sed 's/.*:://' || true)

                while IFS= read -r helper_name; do
                    [[ -z "$helper_name" ]] && continue

                    if ! grep -qx "$helper_name" <<< "$codec_decode_helpers"; then
                        issues=$((issues + 1))
                        echo ""
                        echo -e "${RED}ERROR${NC}: $rel_path:$ref_lineno"
                        echo -e "  Doc or alloc-bound comment references unknown codec decode helper \`$helper_name\`."
                        echo -e "  ${YELLOW}Comment:${NC} $ref_text"
                        echo -e "  ${BLUE}Fix:${NC} Reference an existing helper such as decode_bounded_with_consumed, or use plain prose if no helper exists."
                    fi
                done <<< "$decode_refs"
            done <<< "$codec_decode_ref_matches"
        elif [[ "$VERBOSE" == "true" ]]; then
            echo -e "  ${GREEN}OK${NC}: $rel_path (no codec::decode_* doc/comment refs)"
        fi

    done < <(find "$PROJECT_ROOT/src" "$PROJECT_ROOT/tests" "$PROJECT_ROOT/examples" "$PROJECT_ROOT/benches" \
        -name '*.rs' -print 2>/dev/null \
        | sort)

    echo ""

    if ! check_removed_floor_identifiers "$PROJECT_ROOT"; then
        issues=$((issues + 1))
    fi

    echo ""

    local semantic_claim_checker="$PROJECT_ROOT/scripts/hooks/check-rust-semantic-claims.py"
    if [[ -f "$semantic_claim_checker" ]]; then
        if ! python3 "$semantic_claim_checker"; then
            issues=$((issues + 1))
        fi
    else
        issues=$((issues + 1))
        echo ""
        echo -e "${RED}ERROR${NC}: missing semantic claim checker: $semantic_claim_checker"
        echo -e "  ${BLUE}Fix:${NC} Restore scripts/hooks/check-rust-semantic-claims.py."
    fi

    echo ""

    if [[ "$issues" -eq 0 ]]; then
        echo -e "${GREEN}SUCCESS: No misleading doc claims found.${NC}"
        if [[ "$files_with_downcast_claims" -gt 0 ]]; then
            echo -e "  ($files_with_downcast_claims file(s) with downcast references verified)"
        fi
        if [[ "$files_with_bounded_decode_claims" -gt 0 ]]; then
            echo -e "  ($files_with_bounded_decode_claims file(s) with bounded decode claims verified)"
        fi
        if [[ "$files_with_codec_decode_refs" -gt 0 ]]; then
            echo -e "  ($files_with_codec_decode_refs file(s) with codec::decode_* references verified)"
        fi
        exit 0
    fi

    echo -e "${RED}FAILED: $issues misleading doc claim(s) detected.${NC}"
    echo ""
    echo "Doc comments and allocation-bound comments should accurately describe safety-sensitive APIs."
    exit 1
}

main "$@"
