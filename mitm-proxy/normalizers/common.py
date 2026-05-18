from __future__ import annotations

import json
import shutil
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass
class Cassette:
    path: Path
    connector: str
    test: str
    request_id: str
    method: str
    request_path: str
    response_id: str | None


def quarantine(src: Path, captures_dir: Path, quarantine_dir: Path) -> None:
    """Move src under quarantine_dir, preserving captures_dir-relative layout."""
    rel = src.relative_to(captures_dir)
    dest = quarantine_dir / rel
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        if dest.is_dir():
            shutil.rmtree(dest)
        else:
            dest.unlink()
    shutil.move(str(src), str(dest))


def response_id(body: Any) -> str | None:
    if isinstance(body, dict):
        value = body.get("id")
        return value if isinstance(value, str) and value else None
    return None


def selected_connectors(
    captures_dir: Path, connectors: set[str] | None
) -> list[tuple[str, Path]]:
    selected: list[tuple[str, Path]] = []
    for connector_dir in sorted(captures_dir.iterdir()):
        if not connector_dir.is_dir():
            continue
        connector = connector_dir.name
        if connectors and connector not in connectors:
            continue
        selected.append((connector, connector_dir))
    return selected


def load_cassettes(captures_dir: Path, connector_dir: Path, connector: str) -> list[Cassette]:
    out: list[Cassette] = []
    for fpath in sorted(connector_dir.glob("**/*.json")):
        try:
            with fpath.open() as f:
                record = json.load(f)
            out.append(
                Cassette(
                    path=fpath,
                    connector=record.get("connector", connector),
                    test=record.get("test", ""),
                    request_id=record.get("request_id", ""),
                    method=record.get("request", {}).get("method", ""),
                    request_path=record.get("request", {}).get("path", ""),
                    response_id=response_id(record.get("response", {}).get("body")),
                )
            )
        except Exception as exc:  # noqa: BLE001 - normalize is best-effort
            rel = fpath.relative_to(captures_dir)
            print(f"  skip unreadable cassette {rel}: {exc}")
    return out


def count_connector(captures_dir: Path, connector_dir: Path) -> dict[str, int]:
    # Structure is connector/{spec}/{ctx1}/{ctx2}/.../NNN.json at variable depth
    return {"cassettes_kept": sum(1 for _ in connector_dir.glob("**/*.json"))}


def quarantine_orphan_duplicate_cassettes(
    captures_dir: Path,
    quarantine_dir: Path,
    connector: str,
    cassettes: list[Cassette],
) -> int:
    """Quarantine clear cy.visit duplicate orphans for one connector.

    If several cassettes share (connector, test, request_id, method, path) and
    one response id is referenced by a later request path in the same test while
    a sibling response id is not, the unreferenced sibling is an orphan.
    """
    groups: dict[tuple[str, str, str, str, str], list[Cassette]] = defaultdict(list)
    by_test: dict[tuple[str, str], list[Cassette]] = defaultdict(list)
    for cassette in cassettes:
        groups[
            (
                cassette.connector,
                cassette.test,
                cassette.request_id,
                cassette.method,
                cassette.request_path,
            )
        ].append(cassette)
        by_test[(cassette.connector, cassette.test)].append(cassette)

    quarantined = 0
    for (_conn, test, _rid, _method, _path), dupes in groups.items():
        if len(dupes) < 2:
            continue

        referenced: set[str] = set()
        candidate_ids = {d.response_id for d in dupes if d.response_id}
        if not candidate_ids:
            continue

        dupe_paths = {d.path for d in dupes}
        other_paths = [
            c.request_path
            for c in by_test[(connector, test)]
            if c.path not in dupe_paths
        ]
        for candidate in candidate_ids:
            if any(candidate in request_path for request_path in other_paths):
                referenced.add(candidate)

        if not referenced:
            continue

        for cassette in dupes:
            if cassette.response_id and cassette.response_id not in referenced and cassette.path.exists():
                rel = cassette.path.relative_to(captures_dir)
                print(f"  [{connector}] quarantine orphan duplicate: {rel}  (id={cassette.response_id})")
                quarantine(cassette.path, captures_dir, quarantine_dir)
                quarantined += 1

    return quarantined
