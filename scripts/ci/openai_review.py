#!/usr/bin/env python3
"""Review one pull request with the OpenAI Responses API and post one COMMENT review.

Paid by the OpenAI API credits on the ChatGPT Pro plan: the key must belong to the API
organization linked to that plan. The review never approves and never requests changes, so it
cannot block a merge. It reviews each head commit once.

Environment: OPENAI_API_KEY, GITHUB_TOKEN, GITHUB_REPOSITORY.
Usage:
    openai_review.py --pr NUMBER [--model MODEL] [--effort EFFORT] [--max-findings N] [--dry-run]
"""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import os
import pathlib
import re
import sys
import urllib.error
import urllib.request
from typing import Any, Mapping, Sequence

GITHUB_API = "https://api.github.com"
OPENAI_API = "https://api.openai.com/v1/responses"
DEFAULT_MODEL = "gpt-6-luna"
DEFAULT_MAX_FINDINGS = 5
DEFAULT_MAX_DIFF_CHARS = 180000
GUIDE_MAX_CHARS = 24000
MARKER_PREFIX = "<!-- openai-review:"
RECORD_PREFIX = "<!-- openai-review-run: "
RECORD_SUFFIX = " -->"
DISCLOSURE = os.environ.get("REVIEW_DISCLOSURE", "DISCLOSURE: LLM-GENERATED TEXT")
SKIPPED_PATTERNS = (
    "*.meta",
    "*.lock",
    "*.lock.json",
    "package-lock.json",
    "uv.lock",
    "*.asset",
    "*.unity",
    "*.prefab",
    "*.png",
    "*.jpg",
    "*.gif",
    "*.webp",
    "*.wav",
    "*.ogg",
    "*.mp3",
    "*.ttf",
    "*.otf",
    "*.svg",
    "*.import",
    "*.tscn.import",
    "*.min.js",
    "*.dll",
    "*.so",
    "*.pck",
    "*.wasm",
)
GUIDE_CANDIDATES = (
    "REVIEW.md",
    ".github/REVIEW.md",
    "AGENTS.md",
    ".llm/context.md",
    "CLAUDE.md",
)
HUNK_HEADER = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@")
SEVERITIES = ("critical", "major", "minor")

FINDINGS_SCHEMA: dict[str, Any] = {
    "type": "object",
    "additionalProperties": False,
    "required": ["summary", "findings"],
    "properties": {
        "summary": {"type": "string"},
        "findings": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["path", "line", "severity", "title", "failure", "fix"],
                "properties": {
                    "path": {"type": "string"},
                    "line": {"type": "integer"},
                    "severity": {"type": "string", "enum": list(SEVERITIES)},
                    "title": {"type": "string"},
                    "failure": {"type": "string"},
                    "fix": {"type": "string"},
                },
            },
        },
    },
}

INSTRUCTIONS = """You review one pull request for defects. Report only problems that would make the
code wrong: a crash, wrong output, data loss, a race, a security hole, a broken contract, a test that
cannot fail, or a change that breaks a rule in the repository guide. Do not report style, naming,
formatting, or taste. Do not praise. Each finding names the file and a line that the diff adds or
changes, the failure as a concrete input and its wrong result, and the smallest fix. Report at most
{max_findings} findings, most severe first. When you find nothing, return an empty list; that is a
good outcome. Write short, plain sentences. The summary is two sentences at most."""


def http_json(
    method: str,
    url: str,
    headers: Mapping[str, str],
    body: Any = None,
    raw: bool = False,
) -> Any:
    data = json.dumps(body).encode("utf-8") if body is not None else None
    request = urllib.request.Request(  # noqa: S310
        url, data=data, method=method, headers=dict(headers)
    )
    if data is not None:
        request.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(request, timeout=300) as response:  # noqa: S310
        payload = response.read().decode("utf-8")
    return payload if raw else (json.loads(payload) if payload else None)


def github_headers(
    token: str, accept: str = "application/vnd.github+json"
) -> dict[str, str]:
    return {
        "Authorization": f"Bearer {token}",
        "Accept": accept,
        "X-GitHub-Api-Version": "2022-11-28",
    }


GIT_ESCAPES = {
    "n": 10,
    "t": 9,
    "\\": 92,
    '"': 34,
    "a": 7,
    "b": 8,
    "f": 12,
    "v": 11,
    "r": 13,
}


def unquote_git_path(text: str) -> str:
    """Git quotes a name with unusual bytes as "a/caf\\303\\251"; give back the real name."""
    if not (2 <= len(text) and text[0] == '"' and text[-1] == '"'):
        return text
    body = text[1:-1]
    raw = bytearray()
    index = 0
    while index < len(body):
        character = body[index]
        if character == "\\" and index + 1 < len(body):
            following = body[index + 1]
            octal = body[index + 1 : index + 4]
            if len(octal) == 3 and all(digit in "01234567" for digit in octal):
                raw.append(int(octal, 8))
                index += 4
                continue
            raw.append(GIT_ESCAPES.get(following, ord(following)))
            index += 2
            continue
        raw.extend(character.encode("utf-8"))
        index += 1
    return raw.decode("utf-8", errors="replace")


def side_path(text: str) -> str:
    """The path in a '--- a/x' or '+++ b/x' line, or "" for /dev/null."""
    name = unquote_git_path(text.rstrip("\n").rstrip("\t").strip())
    if name == "/dev/null":
        return ""
    return name[2:] if name[:2] in ("a/", "b/") else name


def block_path(lines: Sequence[str]) -> str:
    """A file's path from its '+++' line, its '---' line for a deletion, or its header."""
    old = ""
    for line in lines:
        if line.startswith("+++ "):
            new = side_path(line[4:])
            if new:
                return new
        elif line.startswith("--- "):
            old = side_path(line[4:])
    if old:
        return old
    match = (
        re.match(r"diff --git a/(.+?) b/(.+)$", lines[0].rstrip("\n"))
        if lines
        else None
    )
    return match.group(2) if match else ""


def split_diff(diff: str) -> list[tuple[str, str]]:
    blocks: list[list[str]] = []
    for line in diff.splitlines(keepends=True):
        if line.startswith("diff --git ") or not blocks:
            blocks.append([line])
        else:
            blocks[-1].append(line)
    return [
        (block_path(block), "".join(block))
        for block in blocks
        if block and block[0].startswith("diff --git ")
    ]


PATCH_NOISE = re.compile(
    r"^(?:@@ [^@]* @@|index [0-9a-f]+\.\.[0-9a-f]+.*)", re.MULTILINE
)


def patch_fingerprint(text: str) -> str:
    """A file's change without its line numbers or blob ids, so a rebase leaves it unchanged."""
    return hashlib.sha256(
        PATCH_NOISE.sub("", text).encode("utf-8", errors="replace")
    ).hexdigest()


def changed_since(diff: str, previous: str) -> tuple[str, list[str]]:
    """The files whose change differs from the last reviewed diff, and the names of the rest."""
    if not previous.strip():
        return diff, []
    before = {path: patch_fingerprint(text) for path, text in split_diff(previous)}
    kept: list[str] = []
    unchanged: list[str] = []
    for path, text in split_diff(diff):
        if before.get(path) == patch_fingerprint(text):
            unchanged.append(path)
        else:
            kept.append(text)
    return "".join(kept), unchanged


def is_skipped(path: str) -> bool:
    name = path.rsplit("/", 1)[-1]
    return any(
        fnmatch.fnmatch(name, pattern) or fnmatch.fnmatch(path, pattern)
        for pattern in SKIPPED_PATTERNS
    )


def trim_diff(diff: str, max_chars: int) -> tuple[str, list[str]]:
    kept: list[str] = []
    dropped: list[str] = []
    used = 0
    for path, text in split_diff(diff):
        if is_skipped(path):
            dropped.append(path)
            continue
        if used + len(text) > max_chars:
            dropped.append(path)
            continue
        kept.append(text)
        used += len(text)
    return "".join(kept), dropped


def commentable_lines(diff: str) -> dict[str, set[int]]:
    lines: dict[str, set[int]] = {}
    for path, text in split_diff(diff):
        new_line = 0
        targets = lines.setdefault(path, set())
        for line in text.splitlines():
            header = HUNK_HEADER.match(line)
            if header:
                new_line = int(header.group(1))
                continue
            if new_line == 0:
                continue
            if line.startswith("+"):
                targets.add(new_line)
                new_line += 1
            elif line.startswith(" "):
                targets.add(new_line)
                new_line += 1
    return lines


def read_guide(root: pathlib.Path) -> str:
    parts: list[str] = []
    room = GUIDE_MAX_CHARS
    for candidate in GUIDE_CANDIDATES:
        path = root / candidate
        if room <= 0 or not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")[:room]
        parts.append(f"Repository guide ({candidate}):\n{text}")
        room -= len(text)
    return "\n\n".join(parts) if parts else "The repository has no guide file."


def build_request(
    model: str,
    max_findings: int,
    guide: str,
    pull: Mapping[str, Any],
    diff: str,
    dropped: Sequence[str],
    effort: str = "",
    unchanged: Sequence[str] = (),
    cache_key: str = "",
) -> dict[str, Any]:
    dropped_note = (
        f"\nFiles left out of this diff: {', '.join(dropped[:50])}" if dropped else ""
    )
    unchanged_note = (
        "\nThese files are also in the pull request, unchanged since your last review of it; they are not "
        f"shown again: {', '.join(unchanged[:50])}"
        if unchanged
        else ""
    )
    prompt = (
        f"{guide}\n\nPull request #{pull.get('number')}: {pull.get('title')}\n\n"
        f"Description:\n{(pull.get('body') or '(none)')[:6000]}\n{dropped_note}{unchanged_note}\n\nDiff:\n{diff}"
    )
    request: dict[str, Any] = {
        "model": model,
        "instructions": INSTRUCTIONS.format(max_findings=max_findings),
        "input": prompt,
        "text": {
            "format": {
                "type": "json_schema",
                "name": "review",
                "schema": FINDINGS_SCHEMA,
                "strict": True,
            }
        },
    }
    if effort:
        request["reasoning"] = {"effort": effort}
    if cache_key:
        request["prompt_cache_key"] = cache_key
    return request


def output_text(response: Mapping[str, Any]) -> str:
    direct = response.get("output_text")
    if isinstance(direct, str):
        return direct
    parts: list[str] = []
    for item in response.get("output", []) or []:
        for content in item.get("content", []) or []:
            if content.get("type") in ("output_text", "text"):
                parts.append(content.get("text", ""))
    return "".join(parts)


def usable_answer(response: Mapping[str, Any]) -> dict[str, Any] | None:
    """The parsed findings, or None when the model refused, stopped early, or sent no JSON."""
    if response.get("status", "completed") != "completed":
        return None
    for item in response.get("output", []) or []:
        for content in item.get("content", []) or []:
            if content.get("type") == "refusal":
                return None
    text = output_text(response)
    if not text.strip():
        return None
    try:
        parsed = json.loads(text)
    except json.JSONDecodeError:
        return None
    return parsed if isinstance(parsed, dict) and "findings" in parsed else None


def review_record(
    model: str,
    effort: str,
    usage: Mapping[str, Any],
    findings: int,
    inline: int,
    diff_chars: int,
    dropped: int,
    unchanged: int = 0,
) -> str:
    """A hidden line with the review's settings and tokens, for agent_metrics.py. The rendered review
    does not name the model, so the name cannot sway a reaction to a comment."""
    details = usage.get("output_tokens_details") or {}
    input_details = usage.get("input_tokens_details") or {}
    record = {
        "v": 1,
        "model": model,
        "effort": effort or "default",
        "input_tokens": usage.get("input_tokens"),
        "output_tokens": usage.get("output_tokens"),
        "reasoning_tokens": details.get("reasoning_tokens")
        if isinstance(details, Mapping)
        else None,
        "findings": findings,
        "inline": inline,
        "diff_chars": diff_chars,
        "dropped_files": dropped,
        "cached_tokens": input_details.get("cached_tokens")
        if isinstance(input_details, Mapping)
        else None,
        "unchanged_files": unchanged,
    }
    text = json.dumps(record, separators=(",", ":")).replace(">", "\\u003e")
    return f"{RECORD_PREFIX}{text}{RECORD_SUFFIX}"


def read_review_record(body: str) -> dict[str, Any] | None:
    start = body.rfind(RECORD_PREFIX)
    end = body.find(RECORD_SUFFIX, start + len(RECORD_PREFIX)) if 0 <= start else -1
    if end < 0:
        return None
    try:
        record = json.loads(body[start + len(RECORD_PREFIX) : end])
    except json.JSONDecodeError:
        return None
    return record if isinstance(record, dict) else None


def build_review(
    result: Mapping[str, Any],
    lines: Mapping[str, set[int]],
    head_sha: str,
    model: str,
    max_findings: int,
    usage: Mapping[str, Any],
    effort: str = "",
    diff_chars: int = 0,
    dropped: int = 0,
    unchanged: int = 0,
) -> dict[str, Any]:
    findings = sorted(
        result.get("findings", []),
        key=lambda item: SEVERITIES.index(item.get("severity", "minor")),
    )
    comments: list[dict[str, Any]] = []
    loose: list[str] = []
    for finding in findings[:max_findings]:
        text = (
            f"**{finding['severity']}: {finding['title']}**\n\n{finding['failure']}\n\n"
            f"Fix: {finding['fix']}"
        )
        path, line = finding.get("path", ""), int(finding.get("line", 0))
        if line in lines.get(path, set()):
            comments.append({"path": path, "line": line, "side": "RIGHT", "body": text})
        else:
            loose.append(f"- `{path}:{line}` {text}")
    count = len(comments) + len(loose)
    tokens = (
        f"{usage.get('input_tokens', '?')} in, {usage.get('output_tokens', '?')} out"
    )
    body_parts = [DISCLOSURE] if DISCLOSURE else []
    body_parts += [
        f"OpenAI review: {count} finding{'s' if count != 1 else ''}.",
        str(result.get("summary", "")).strip(),
    ]
    if loose:
        body_parts += ["Findings outside the changed lines:", "\n".join(loose)]
    body_parts.append(
        f"<sub>{tokens} tokens. Comments only; this review never blocks a merge.</sub>"
    )
    body_parts.append(
        review_record(
            model, effort, usage, count, len(comments), diff_chars, dropped, unchanged
        )
    )
    body_parts.append(f"{MARKER_PREFIX}{head_sha} -->")
    return {
        "commit_id": head_sha,
        "event": "COMMENT",
        "body": "\n\n".join(body_parts),
        "comments": comments,
    }


REVIEWER = os.environ.get("OPENAI_REVIEW_WRITER", "github-actions[bot]")


def last_reviewed_sha(reviews: Sequence[Mapping[str, Any]], head_sha: str) -> str:
    """The newest commit this workflow reviewed before the head, or "" for a first review."""
    found = ""
    for review in reviews:
        if (review.get("user") or {}).get("login") != REVIEWER:
            continue
        match = re.search(
            re.escape(MARKER_PREFIX) + r"([0-9a-f]{40}) -->", review.get("body") or ""
        )
        if match and match.group(1) != head_sha:
            found = match.group(1)
    return found


def already_reviewed(reviews: Sequence[Mapping[str, Any]], head_sha: str) -> bool:
    """Done only when this workflow's own review of this exact commit carries the marker."""
    marker = f"{MARKER_PREFIX}{head_sha} -->"
    return any(
        marker in (review.get("body") or "")
        and (review.get("user") or {}).get("login") == REVIEWER
        and review.get("commit_id") == head_sha
        for review in reviews
    )


def body_only(
    review: Mapping[str, Any],
    model: str,
    effort: str,
    usage: Mapping[str, Any],
    findings: int,
    diff_chars: int,
    dropped: int,
    unchanged: int = 0,
) -> dict[str, Any]:
    """The same review with its inline comments moved into the body, and a record that says so."""
    inline = "\n".join(
        f"- `{c['path']}:{c['line']}` {c['body']}" for c in review["comments"]
    )
    body = str(review["body"])
    start = body.find(RECORD_PREFIX)
    end = body.find(RECORD_SUFFIX, start) + len(RECORD_SUFFIX) if 0 <= start else -1
    record = review_record(
        model, effort, usage, findings, 0, diff_chars, dropped, unchanged
    )
    if 0 <= start and len(RECORD_SUFFIX) <= end:
        body = body[:start] + record + body[end:]
    body = body.replace(MARKER_PREFIX, inline + "\n\n" + MARKER_PREFIX, 1)
    return {**review, "comments": [], "body": body}


def main(argv: Sequence[str]) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--pr", type=int, required=True)
    parser.add_argument(
        "--model", default=os.environ.get("OPENAI_REVIEW_MODEL") or DEFAULT_MODEL
    )
    parser.add_argument(
        "--effort",
        default=os.environ.get("OPENAI_REVIEW_EFFORT", ""),
        help="reasoning effort (for example low, medium, high); blank keeps the model default",
    )
    parser.add_argument("--max-findings", type=int, default=DEFAULT_MAX_FINDINGS)
    parser.add_argument("--max-diff-chars", type=int, default=DEFAULT_MAX_DIFF_CHARS)
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument("--dry-run", action="store_true")
    arguments = parser.parse_args(argv)
    repository = os.environ.get("GITHUB_REPOSITORY", "")
    github_token = os.environ.get("GITHUB_TOKEN", "")
    openai_key = os.environ.get("OPENAI_API_KEY", "")
    if not openai_key.strip():
        print(
            "::error::OPENAI_API_KEY is not set. Add it under Settings, Secrets and variables, Actions.",
            file=sys.stderr,
        )
        return 1
    if not repository or not github_token:
        print("GITHUB_REPOSITORY and GITHUB_TOKEN are required.", file=sys.stderr)
        return 1
    base = f"{GITHUB_API}/repos/{repository}/pulls/{arguments.pr}"
    try:
        pull = http_json("GET", base, github_headers(github_token))
        head_sha = pull["head"]["sha"]
        reviews: list[Any] = []
        page = 1
        while True:
            batch = http_json(
                "GET",
                f"{base}/reviews?per_page=100&page={page}",
                github_headers(github_token),
            )
            if not batch:
                break
            reviews.extend(batch)
            page += 1
        if already_reviewed(reviews, head_sha):
            print(f"Commit {head_sha[:10]} already has an OpenAI review.")
            return 0
        base_sha = pull["base"]["sha"]
        compare = f"{GITHUB_API}/repos/{repository}/compare/{base_sha}...{head_sha}"
        diff = http_json(
            "GET",
            compare,
            github_headers(github_token, "application/vnd.github.diff"),
            raw=True,
        )
        latest = http_json("GET", base, github_headers(github_token))
        if latest["head"]["sha"] != head_sha:
            print(
                f"The pull request moved past {head_sha[:10]}; the run for the new commit reviews it."
            )
            return 0
    except (urllib.error.URLError, TimeoutError, KeyError) as error:
        print(
            f"Skipped: GitHub did not give the diff ({error}). Large pull requests are skipped."
        )
        return 0
    previous_sha = last_reviewed_sha(reviews, head_sha)
    previous = ""
    if previous_sha:
        try:
            before = (
                f"{GITHUB_API}/repos/{repository}/compare/{base_sha}...{previous_sha}"
            )
            previous = http_json(
                "GET",
                before,
                github_headers(github_token, "application/vnd.github.diff"),
                raw=True,
            )
        except (urllib.error.URLError, TimeoutError):
            previous = ""
    changed, unchanged = changed_since(diff, previous)
    unchanged = [path for path in unchanged if not is_skipped(path)]
    trimmed, dropped = trim_diff(changed, arguments.max_diff_chars)
    if not trimmed.strip():
        reason = (
            "since the last review"
            if previous
            else "after skipping generated and binary files"
        )
        print(f"No reviewable change {reason}.")
        return 0
    request = build_request(
        arguments.model,
        arguments.max_findings,
        read_guide(arguments.root),
        pull,
        trimmed,
        dropped,
        arguments.effort,
        unchanged,
        f"openai-review:{repository}",
    )
    try:
        response = http_json(
            "POST", OPENAI_API, {"Authorization": f"Bearer {openai_key}"}, request
        )
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")[:600]
        print(f"Skipped: the OpenAI API answered {error.code}: {detail}")
        return 0
    except (urllib.error.URLError, TimeoutError) as error:
        print(f"Skipped: the OpenAI API did not answer ({error}).")
        return 0
    result = usable_answer(response)
    if result is None:
        print(
            f"Skipped: the model gave no usable review (status {response.get('status')}). No marker posted."
        )
        return 0
    review = build_review(
        result,
        commentable_lines(trimmed),
        head_sha,
        arguments.model,
        arguments.max_findings,
        response.get("usage") or {},
        arguments.effort,
        len(trimmed),
        len(dropped),
        len(unchanged),
    )
    if arguments.dry_run:
        print(json.dumps(review, indent=2))
        return 0
    try:
        http_json("POST", f"{base}/reviews", github_headers(github_token), review)
    except urllib.error.HTTPError as error:
        print(
            f"Inline review refused ({error.code}); posting the findings in the review body.",
            file=sys.stderr,
        )
        findings = len(result.get("findings") or [])
        review = body_only(
            review,
            arguments.model,
            arguments.effort,
            response.get("usage") or {},
            findings,
            len(trimmed),
            len(dropped),
            len(unchanged),
        )
        http_json("POST", f"{base}/reviews", github_headers(github_token), review)
    print(
        f"Posted a review with {len(review['comments'])} inline comments on {head_sha[:10]}."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
