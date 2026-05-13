#!/usr/bin/env python3
"""
Curate captured cassettes after a recording run.

Run between `./start.sh` (capture) and `./start_replay.sh` (replay):

    ./start.sh
    # ... run cypress in capture mode ...
    python3 normalize_captures.py
    ./start_replay.sh

Why this is needed
------------------
Two kinds of cassette pollute the captures dir after a real 3DS capture run.
Replay never looks them up (so they're harmless to mitm) but they make the
captures dir confusing to read, and they include the *wrong* duplicates that
break replay determinism when HS's final state doesn't match what we serve:

1. **Server-UUID folders.** During the 3DS browser dance, the ACS form posts
   back to HS without carrying Cypress's X-Request-ID. HS mints a UUID for
   that incoming request and propagates it to its connector outbound. Cypress
   in replay mode bypasses the whole browser dance, so it never issues a
   request whose ID would look like a UUID. The resulting cassettes are
   never matched.

2. **Per-(method, path) duplicates within a single Cypress request_id.**
   Cypress's beforeEach fires multiple times mid-test when cy.visit triggers
   external page navigations (a Cypress 14 quirk; not our test code). Each
   firing resets the step counter and replays earlier steps, including the
   confirm cy.request — so HS makes a second POST /v1/payment_intents under
   the same X-Request-ID, creating a fresh Stripe PI. HS's final state
   references the LATEST PI; the earlier cassette is what HS would have used
   if the test had run cleanly, but isn't what later cassettes (retrieve,
   capture) are tied to.

   Replay only fires one cy.request per logical step (no cy.visit, no
   beforeEach refire), so it consumes one cassette per (rid, method, path).
   We need that one to be the LATEST captured response, since downstream
   cassettes are keyed on the IDs the latest response returned.

What this script does
---------------------
Walks `captures/<connector>/<test>/<rid>/*.json` and:
  - Deletes any <rid> folder whose name isn't a Cypress-format ID
    ({8hex}-{NNN}).
  - Within each Cypress-format <rid> folder, groups cassettes by
    (method, path) and deletes all but the one with the latest
    `captured_at` timestamp.

Inputs are JSON; outputs are the same JSON files with the duplicates removed.
Safe to re-run; idempotent.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import sys
from collections import defaultdict
from pathlib import Path

CYPRESS_RID_RE = re.compile(r"^[0-9a-f]{8}-\d{3}$")


def normalize(captures_dir: Path) -> dict:
    stats = {
        "server_uuid_folders_deleted": 0,
        "server_uuid_cassettes_deleted": 0,
        "duplicate_cassettes_deleted": 0,
        "kept_cassettes": 0,
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
            for rid_dir in sorted(test_dir.iterdir()):
                if not rid_dir.is_dir():
                    continue
                rid = rid_dir.name

                # Server-UUID folder → never looked up in replay, delete it.
                if not CYPRESS_RID_RE.match(rid):
                    n = sum(1 for _ in rid_dir.glob("*.json"))
                    rel = rid_dir.relative_to(captures_dir)
                    print(f"  drop server-UUID folder: {rel}  ({n} cassettes)")
                    shutil.rmtree(rid_dir)
                    stats["server_uuid_folders_deleted"] += 1
                    stats["server_uuid_cassettes_deleted"] += n
                    continue

                # Cypress-format folder → dedupe by (method, path), keep latest.
                groups: dict[tuple[str, str], list[tuple[str, Path]]] = defaultdict(list)
                for f in sorted(rid_dir.glob("*.json")):
                    try:
                        rec = json.loads(f.read_text())
                    except (OSError, json.JSONDecodeError) as e:
                        print(f"  WARN: could not read {f.relative_to(captures_dir)}: {e}", file=sys.stderr)
                        continue
                    key = (rec["request"]["method"], rec["request"]["path"])
                    groups[key].append((rec.get("captured_at", ""), f))

                for (method, path), entries in groups.items():
                    if len(entries) <= 1:
                        stats["kept_cassettes"] += 1
                        continue
                    entries.sort(key=lambda e: e[0])  # ascending captured_at
                    keep_ts, keep_file = entries[-1]
                    for _, f in entries[:-1]:
                        rel = f.relative_to(captures_dir)
                        print(f"  drop duplicate: {rel}  (kept {keep_file.name} @ {keep_ts})")
                        f.unlink()
                        stats["duplicate_cassettes_deleted"] += 1
                    stats["kept_cassettes"] += 1

    return stats


def main() -> int:
    here = Path(__file__).resolve().parent
    captures = Path(sys.argv[1]) if len(sys.argv) > 1 else here / "captures"

    print(f"Normalizing cassettes in {captures}\n")
    stats = normalize(captures)
    print("\n── summary ──")
    print(f"  server-UUID folders deleted   : {stats['server_uuid_folders_deleted']}")
    print(f"  server-UUID cassettes deleted : {stats['server_uuid_cassettes_deleted']}")
    print(f"  duplicate cassettes deleted   : {stats['duplicate_cassettes_deleted']}")
    print(f"  cassettes kept                : {stats['kept_cassettes']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
