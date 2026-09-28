#!/usr/bin/env python3
"""Order-insensitive check for [mandates.supported_payment_methods].

A missing key silently downgrades setup_future_usage from off_session to
on_session. Connector order is ignored so real membership drift is not
hidden by reordering.

Key sets must match across the six config files. Connector sets must match
across the three deployment files (sandbox, production, integration_test).
development.toml, docker_compose.toml, and config.example.toml are allowed
to list a different connector subset for a key that exists in every file.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SECTION = "mandates.supported_payment_methods"

KEY_SET_FILES = (
    "config/docker_compose.toml",
    "config/config.example.toml",
    "config/development.toml",
    "config/deployments/sandbox.toml",
    "config/deployments/production.toml",
    "config/deployments/integration_test.toml",
)

CONNECTOR_SET_FILES = (
    "config/deployments/sandbox.toml",
    "config/deployments/production.toml",
    "config/deployments/integration_test.toml",
)

KEY_RE = re.compile(
    r"^([A-Za-z0-9_]+)\.([A-Za-z0-9_]+)(?:\.connector_list)?"
    r"\s*=\s*(?:\{\s*connector_list\s*=\s*)?\"([^\"]*)\""
)


def parse_section(path: Path) -> dict[str, set[str]]:
    text = path.read_text()
    match = re.search(rf"^\[{re.escape(SECTION)}\]\s*$", text, re.M)
    if match is None:
        raise SystemExit(f"{path}: missing [{SECTION}]")
    start = match.end()
    nxt = re.search(r"^\[", text[start:], re.M)
    body = text[start : start + nxt.start()] if nxt else text[start:]
    keys: dict[str, set[str]] = {}
    for raw in body.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        parsed = KEY_RE.match(line)
        if parsed is None:
            raise SystemExit(f"{path}: unparsed mandate line: {line}")
        key = f"{parsed.group(1)}.{parsed.group(2)}"
        connectors = {part.strip() for part in parsed.group(3).split(",") if part.strip()}
        if key in keys:
            raise SystemExit(f"{path}: duplicate key {key}")
        if not connectors:
            raise SystemExit(f"{path}: {key} has an empty connector list")
        keys[key] = connectors
    return keys


def main() -> int:
    parsed = {rel: parse_section(REPO_ROOT / rel) for rel in KEY_SET_FILES}
    failed = False

    union = set().union(*(data.keys() for data in parsed.values()))
    for rel, data in parsed.items():
        missing = sorted(union - data.keys())
        if missing:
            failed = True
            print(f"{rel} is missing {len(missing)} key(s): {', '.join(missing)}")

    reference = CONNECTOR_SET_FILES[0]
    reference_sets = parsed[reference]
    for rel in CONNECTOR_SET_FILES[1:]:
        for key in sorted(set(reference_sets) | set(parsed[rel])):
            left = reference_sets.get(key, set())
            right = parsed[rel].get(key, set())
            if left == right:
                continue
            failed = True
            only_left = sorted(left - right)
            only_right = sorted(right - left)
            print(f"{key}: {reference} vs {rel}")
            if only_left:
                print(f"  only in {reference}: {', '.join(only_left)}")
            if only_right:
                print(f"  only in {rel}: {', '.join(only_right)}")

    if failed:
        print(
            "\n[mandates.supported_payment_methods] drifted. "
            "Compare connector lists as sets; order does not matter."
        )
        return 1

    print(
        f"[{SECTION}] key sets match across {len(KEY_SET_FILES)} files; "
        f"connector sets match across {len(CONNECTOR_SET_FILES)} deployment files."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
