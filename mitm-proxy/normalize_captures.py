#!/usr/bin/env python3
"""
Quarantine obvious-noise cassettes after a recording run.

Run between `./start.sh` (capture) and `./start_replay.sh` (replay):

    ./start.sh
    # ... run cypress in capture mode ...
    python3 normalize_captures.py
    ./start_replay.sh

What this script does
---------------------
Walks `captures/<connector>/<test>/<rid>/*.json` and quarantines any <rid>
folder whose name isn't a Cypress-format ID ({8hex}-{NNN}).

These "server-UUID" folders are created when HS receives an inbound HTTP
request that doesn't carry Cypress's X-Request-ID header — for example,
during a 3DS browser dance the connector's ACS form POSTs back to HS, and
HS mints a fresh UUID for that incoming request and propagates it onto its
connector sync outbound. Cypress in replay mode bypasses the whole browser
dance (PROXY_MODE=replay), so it never issues a request whose ID would
look like a UUID. The resulting cassettes are never matched at replay
time and just clutter the captures dir.

Nothing is deleted. Quarantined items are moved to a sibling
`captures_quarantine/` directory, preserving their relative path. To
restore if normalize misjudged:

    mv mitm-proxy/captures_quarantine/<path> mitm-proxy/captures/<path>

Manual curation
---------------
This script intentionally does NOT try to detect "duplicate" cassettes or
"orphan" cassettes from cy.visit-induced beforeEach refire. Those cases
are subtle and easy to get wrong with heuristics (e.g. multiple `it()`
blocks within one context that share titles will legitimately share a
testIdHash and reuse rids — they look like duplicates but aren't).

When replay logs `[replay] MISS` or `LIVE`, inspect `captures/` for the
relevant `(test, rid)` folder and decide what to keep or quarantine by
hand. The mitm replay matcher serves cassettes FIFO within a key, so if
there are too many cassettes, quarantine the earlier (by `captured_at`)
or otherwise wrong one; if too few, restore from `captures_quarantine/`
or re-capture.

Safe to re-run; idempotent.
"""

from __future__ import annotations

import json
import re
import shutil
import sys
from pathlib import Path

CYPRESS_RID_RE = re.compile(r"^[0-9a-f]{8}-\d{3}$")


def _quarantine(src: Path, captures_dir: Path, quarantine_dir: Path) -> None:
    """Move a file or directory under captures_dir into the mirror location
    inside quarantine_dir, preserving the relative structure. If the
    destination already exists from a prior run, overwrite it."""
    rel = src.relative_to(captures_dir)
    dest = quarantine_dir / rel
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        if dest.is_dir():
            shutil.rmtree(dest)
        else:
            dest.unlink()
    shutil.move(str(src), str(dest))


def normalize(captures_dir: Path, quarantine_dir: Path) -> dict:
    stats = {
        "server_uuid_folders_quarantined": 0,
        "server_uuid_cassettes_quarantined": 0,
        "cassettes_kept": 0,
    }

    if not captures_dir.exists():
        print(f"No captures directory at {captures_dir}; nothing to do.")
        return stats

    for connector_dir in sorted(captures_dir.iterdir()):
        if not connector_dir.is_dir():
            continue
        for test_dir in sorted(connector_dir.iterdir()):
            if not test_dir.is_dir():
                continue
            for rid_dir in list(test_dir.iterdir()):
                if not rid_dir.is_dir():
                    continue
                rid = rid_dir.name
                if CYPRESS_RID_RE.match(rid):
                    stats["cassettes_kept"] += sum(1 for _ in rid_dir.glob("*.json"))
                    continue
                n = sum(1 for _ in rid_dir.glob("*.json"))
                rel = rid_dir.relative_to(captures_dir)
                print(f"  quarantine server-UUID folder: {rel}  ({n} cassettes)")
                _quarantine(rid_dir, captures_dir, quarantine_dir)
                stats["server_uuid_folders_quarantined"] += 1
                stats["server_uuid_cassettes_quarantined"] += n

    return stats


def main() -> int:
    here = Path(__file__).resolve().parent
    captures = Path(sys.argv[1]) if len(sys.argv) > 1 else here / "captures"
    quarantine = captures.parent / "captures_quarantine"

    print(f"Normalizing cassettes in {captures}")
    print(f"Quarantine destination : {quarantine}\n")
    stats = normalize(captures, quarantine)
    print("\n── summary ──")
    print(f"  server-UUID folders quarantined   : {stats['server_uuid_folders_quarantined']}")
    print(f"  server-UUID cassettes quarantined : {stats['server_uuid_cassettes_quarantined']}")
    print(f"  cassettes kept                    : {stats['cassettes_kept']}")
    if stats["server_uuid_folders_quarantined"]:
        print(f"\nQuarantined items are at: {quarantine}")
        print("Restore by `mv` back into the captures tree if normalize misjudged.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
