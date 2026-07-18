#!/usr/bin/env python3
"""Reconcile a Cargo publish attempt with the authoritative crates.io state."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from enum import Enum


DEFAULT_REGISTRY_API = "https://crates.io/api/v1"
DEFAULT_ATTEMPTS = 8
DEFAULT_INITIAL_DELAY = 2.0
DEFAULT_MAX_DELAY = 30.0
DEFAULT_CARGO_TIMEOUT = 600
MAX_REGISTRY_RESPONSE_BYTES = 1_048_576


class PublishError(RuntimeError):
    """Publication cannot safely continue or be reconciled."""


class RegistryState(Enum):
    """Authoritative state of one crate version."""

    ABSENT = "absent"
    MATCHING = "matching"
    CONFLICT = "conflict"


@dataclass(frozen=True)
class RegistryResult:
    state: RegistryState
    published_checksum: str | None = None


def probe_registry(
    crate_name: str,
    version: str,
    expected_checksum: str,
    *,
    api_base: str = DEFAULT_REGISTRY_API,
) -> RegistryResult:
    """Classify a crate version using the registry checksum."""
    url = f"{api_base.rstrip('/')}/crates/{crate_name}/{version}"
    request = urllib.request.Request(
        url,
        headers={"User-Agent": f"fortress-rollback-release-workflow/{version}"},
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            raw = response.read(MAX_REGISTRY_RESPONSE_BYTES + 1)
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return RegistryResult(RegistryState.ABSENT)
        snippet = error.read(201).decode(errors="replace")[:200]
        raise PublishError(
            f"registry returned HTTP {error.code} for {crate_name} {version}: {snippet}"
        ) from error
    except (urllib.error.URLError, TimeoutError, OSError) as error:
        raise PublishError(
            f"registry request failed for {crate_name} {version}: {error}"
        ) from error
    if len(raw) > MAX_REGISTRY_RESPONSE_BYTES:
        raise PublishError(
            f"registry metadata for {crate_name} {version} exceeds "
            f"{MAX_REGISTRY_RESPONSE_BYTES} bytes"
        )
    try:
        document = json.loads(raw.decode("utf-8"))
        checksum = document["version"]["checksum"]
    except (UnicodeError, json.JSONDecodeError, KeyError, TypeError) as error:
        snippet = raw.decode(errors="replace")[:200]
        raise PublishError(
            f"registry returned malformed metadata for {crate_name} {version}: {snippet!r}"
        ) from error
    if not isinstance(checksum, str) or not checksum:
        raise PublishError(
            f"registry returned an empty checksum for {crate_name} {version}"
        )
    if checksum == expected_checksum:
        return RegistryResult(RegistryState.MATCHING, checksum)
    return RegistryResult(RegistryState.CONFLICT, checksum)


def _probe_or_retry(
    crate_name: str,
    version: str,
    checksum: str,
    *,
    api_base: str,
    attempts: int,
    initial_delay: float,
    max_delay: float,
) -> RegistryResult:
    """Poll long enough for the crates.io index/API to expose an upload."""
    if attempts < 1:
        raise PublishError("poll attempts must be at least one")
    delay = initial_delay
    last_error: PublishError | None = None
    for attempt in range(1, attempts + 1):
        try:
            result = probe_registry(
                crate_name, version, checksum, api_base=api_base
            )
            last_error = None
            if result.state is not RegistryState.ABSENT:
                return result
        except PublishError as error:
            last_error = error
        if attempt < attempts:
            time.sleep(delay)
            delay = min(max_delay, delay * 2)
    if last_error is not None:
        raise PublishError(
            f"registry never returned usable metadata after {attempts} attempts: "
            f"{last_error}"
        ) from last_error
    return RegistryResult(RegistryState.ABSENT)


def _initial_probe_with_retries(
    crate_name: str,
    version: str,
    checksum: str,
    *,
    api_base: str,
    attempts: int = 3,
) -> RegistryResult:
    """Retry transient lookup failures without delaying a confirmed absence."""
    last_error: PublishError | None = None
    for attempt in range(1, attempts + 1):
        try:
            return probe_registry(crate_name, version, checksum, api_base=api_base)
        except PublishError as error:
            last_error = error
            if attempt < attempts:
                time.sleep(float(2 ** (attempt - 1)))
    if last_error is None:
        raise PublishError("registry state probe failed without diagnostics")
    raise PublishError(
        f"registry state remained unavailable after {attempts} attempts: {last_error}"
    ) from last_error


def reconcile_publish(
    crate_name: str,
    version: str,
    checksum: str,
    *,
    api_base: str = DEFAULT_REGISTRY_API,
    attempts: int = DEFAULT_ATTEMPTS,
    initial_delay: float = DEFAULT_INITIAL_DELAY,
    max_delay: float = DEFAULT_MAX_DELAY,
) -> str:
    """Publish only when absent, then accept success only by registry checksum."""
    initial = _initial_probe_with_retries(
        crate_name, version, checksum, api_base=api_base
    )
    if initial.state is RegistryState.MATCHING:
        return "already-published"
    if initial.state is RegistryState.CONFLICT:
        raise PublishError(
            f"{crate_name} {version} exists with checksum "
            f"{initial.published_checksum}, expected {checksum}"
        )
    if not os.environ.get("CARGO_REGISTRY_TOKEN"):
        raise PublishError(
            "CRATES_IO_TOKEN/CARGO_REGISTRY_TOKEN is required because the target "
            "version is not published"
        )

    cargo_result: int | str
    try:
        publish = subprocess.run(
            ["cargo", "publish", "--locked", "--registry", "crates-io"],
            check=False,
            timeout=DEFAULT_CARGO_TIMEOUT,
        )
        cargo_result = publish.returncode
    except subprocess.TimeoutExpired:
        cargo_result = f"timed out after {DEFAULT_CARGO_TIMEOUT} seconds"
    except OSError as error:
        raise PublishError(f"could not start cargo publish: {error}") from error

    final = _probe_or_retry(
        crate_name,
        version,
        checksum,
        api_base=api_base,
        attempts=attempts,
        initial_delay=initial_delay,
        max_delay=max_delay,
    )
    if final.state is RegistryState.MATCHING:
        if cargo_result != 0:
            print(
                f"cargo publish {cargo_result}, but crates.io has the exact packaged "
                "checksum; treating the accepted upload as successful.",
                file=sys.stderr,
            )
        return "published"
    if final.state is RegistryState.CONFLICT:
        raise PublishError(
            f"crates.io exposed {crate_name} {version} with checksum "
            f"{final.published_checksum}, expected {checksum}"
        )
    raise PublishError(
        f"cargo publish result was {cargo_result}, but crates.io did not expose "
        f"{crate_name} {version} after {attempts} attempts"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crate", required=True, dest="crate_name")
    parser.add_argument("--version", required=True)
    parser.add_argument("--checksum", required=True)
    parser.add_argument("--attempts", type=int, default=DEFAULT_ATTEMPTS)
    parser.add_argument("--initial-delay", type=float, default=DEFAULT_INITIAL_DELAY)
    parser.add_argument("--max-delay", type=float, default=DEFAULT_MAX_DELAY)
    args = parser.parse_args()
    try:
        outcome = reconcile_publish(
            args.crate_name,
            args.version,
            args.checksum,
            attempts=args.attempts,
            initial_delay=args.initial_delay,
            max_delay=args.max_delay,
        )
    except PublishError as error:
        print(f"publish-state: error: {error}", file=sys.stderr)
        return 1
    print(f"publication_outcome={outcome}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
