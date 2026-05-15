from __future__ import annotations

from pathlib import Path

from normalizers.common import Cassette, quarantine_orphan_duplicate_cassettes


def normalize_connector(
    captures_dir: Path,
    quarantine_dir: Path,
    connector: str,
    cassettes: list[Cassette],
    stats: dict,
) -> None:
    """PayPal-specific curation.

    PayPal 3DS/off-session paths can produce cy.visit duplicate create-order
    cassettes. Keep the branch whose order id is referenced by subsequent
    retrieve/capture cassettes and quarantine the orphan duplicate.
    """
    quarantined = quarantine_orphan_duplicate_cassettes(
        captures_dir, quarantine_dir, connector, cassettes
    )
    stats["orphan_duplicate_cassettes_quarantined"] += quarantined
    stats["cassettes_kept"] -= quarantined
