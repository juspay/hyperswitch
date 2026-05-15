from __future__ import annotations

from pathlib import Path

from normalizers.common import Cassette


def normalize_connector(
    captures_dir: Path,
    quarantine_dir: Path,
    connector: str,
    cassettes: list[Cassette],
    stats: dict,
) -> None:
    """Explicit no-op normalizer for razorpay.

    Keep connector-specific cassette curation isolated in this file. Do not add
    behavior here unless it has been validated for razorpay only.
    """
    return None
