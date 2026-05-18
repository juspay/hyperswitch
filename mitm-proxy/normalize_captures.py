#!/usr/bin/env python3
"""
Normalize cassettes after a recording run, scoped per connector.

Usage:
    python3 mitm-proxy/normalize_captures.py [captures_dir] [connector ...]

Design rules
------------
* Common normalization must be safe for every connector: counting and keeping
  server-UUID folders.
* Connector-specific cleanup lives in mitm-proxy/normalizers/<connector>.py.
  This prevents a heuristic needed for one connector from silently changing
  another connector's cassette suite.
* Nothing is deleted. Connector-specific curation moves files to the sibling
  captures_quarantine/ tree, preserving relative paths.
"""

from __future__ import annotations

import importlib
import sys
from pathlib import Path

from normalizers.common import count_connector, load_cassettes, selected_connectors


def _empty_stats() -> dict[str, int]:
    return {
        "orphan_duplicate_cassettes_quarantined": 0,
        "cassettes_kept": 0,
    }


def _add_stats(dst: dict[str, int], src: dict[str, int]) -> None:
    for key, value in src.items():
        dst[key] = dst.get(key, 0) + int(value)


def _run_connector_module(
    captures_dir: Path,
    quarantine_dir: Path,
    connector: str,
    connector_dir: Path,
    stats: dict[str, int],
) -> str:
    module_name = f"normalizers.{connector}"
    try:
        module = importlib.import_module(module_name)
    except ModuleNotFoundError as exc:
        if exc.name == module_name:
            raise RuntimeError(
                f"missing connector normalizer {module_name}; add "
                f"mitm-proxy/normalizers/{connector}.py before normalizing this connector"
            ) from exc
        raise

    normalize_connector = getattr(module, "normalize_connector", None)
    if normalize_connector is None:
        raise RuntimeError(f"{module_name} must define normalize_connector(...)")

    cassettes = load_cassettes(captures_dir, connector_dir, connector)
    normalize_connector(captures_dir, quarantine_dir, connector, cassettes, stats)
    return module.__name__


def normalize(
    captures_dir: Path, quarantine_dir: Path, connectors: set[str] | None = None
) -> dict[str, int]:
    stats = _empty_stats()

    if not captures_dir.exists():
        print(f"No captures directory at {captures_dir}; nothing to do.")
        return stats

    selected = selected_connectors(captures_dir, connectors)
    if not selected:
        wanted = ", ".join(sorted(connectors or [])) or "<all>"
        print(f"No matching connector capture directories for: {wanted}")
        return stats

    for connector, connector_dir in selected:
        print(f"Connector: {connector}")
        connector_stats = _empty_stats()
        _add_stats(connector_stats, count_connector(captures_dir, connector_dir))

        module_name = _run_connector_module(
            captures_dir, quarantine_dir, connector, connector_dir, connector_stats
        )
        print(f"  connector normalizer: {module_name}")

        _add_stats(stats, connector_stats)

    return stats


def main() -> int:
    here = Path(__file__).resolve().parent
    captures = Path(sys.argv[1]) if len(sys.argv) > 1 else here / "captures"
    connectors = set(sys.argv[2:]) or None
    quarantine = captures.parent / "captures_quarantine"

    print(f"Normalizing cassettes in {captures}")
    print(f"Quarantine destination : {quarantine}")
    if connectors:
        print(f"Connector filter       : {', '.join(sorted(connectors))}")
    print()

    stats = normalize(captures, quarantine, connectors)

    print("\n── summary ──")
    print(
        "  orphan duplicate cassettes quarantined: "
        f"{stats['orphan_duplicate_cassettes_quarantined']}"
    )
    print(f"  cassettes kept                         : {stats['cassettes_kept']}")
    if stats["orphan_duplicate_cassettes_quarantined"]:
        print(f"\nQuarantined items are at: {quarantine}")
        print("Restore by `mv` back into the captures tree if normalize misjudged.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
