#!/usr/bin/env python3
"""Fail if cassette files still contain raw values from creds.json.

Use this before committing captures, and in CI if creds.json is available:

    python3 mitm-proxy/check_cassettes_redacted.py mitm-proxy/captures
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from secret_redaction import (  # noqa: PLC2701 - internal scan helpers
    _REDACTED_PREFIX,
    _is_card_secret_key,
    _is_request_sensitive_key,
    _known_test_pans,
    _replacement_pairs,
    creds_path,
)


def _placeholder(value: object) -> bool:
    return isinstance(value, str) and value.startswith(_REDACTED_PREFIX)


def _find_unredacted_card_values(obj: object, path: str = "") -> list[str]:
    failures: list[str] = []
    if isinstance(obj, dict):
        for key, value in obj.items():
            child = f"{path}.{key}" if path else str(key)
            path_has_card_context = any("card" in part.lower() for part in child.split("."))
            normalized = "".join(ch for ch in str(key).lower() if ch.isalnum())
            if (
                _is_card_secret_key(str(key)) or (normalized == "number" and path_has_card_context)
            ) and value not in (None, "") and not _placeholder(value):
                failures.append(child)
            else:
                failures.extend(_find_unredacted_card_values(value, child))
    elif isinstance(obj, list):
        for idx, value in enumerate(obj):
            failures.extend(_find_unredacted_card_values(value, f"{path}.{idx}" if path else str(idx)))
    return failures


def _find_unredacted_request_values(obj: object, path: str = "request") -> list[str]:
    failures: list[str] = []
    if isinstance(obj, dict):
        for key, value in obj.items():
            child = f"{path}.{key}"
            if _is_request_sensitive_key(str(key)) and value not in (None, "") and not _placeholder(value):
                failures.append(child)
            else:
                failures.extend(_find_unredacted_request_values(value, child))
    elif isinstance(obj, list):
        for idx, value in enumerate(obj):
            failures.extend(_find_unredacted_request_values(value, f"{path}.{idx}"))
    return failures


def _placeholder_connectors(placeholder: str) -> set[str]:
    label = placeholder.removeprefix("{{MITM_SECRET:").removesuffix("}}")
    for prefix in ("urlquote:", "urlplus:", "base64:", "pair:", "basic64:", "basic:"):
        if label.startswith(prefix):
            label = label[len(prefix):]
            break
    connectors = set()
    for part in label.split("+"):
        connector = part.split(".", 1)[0].strip()
        if connector:
            connectors.add(connector)
    return connectors


def _record_connectors(record: object, default_connector: str) -> set[str]:
    connectors = {default_connector} if default_connector and default_connector != "captures" else set()
    if not isinstance(record, dict):
        return connectors
    top = str(record.get("connector") or "").strip()
    if top:
        connectors.add(top)
    request = record.get("request") or {}
    if isinstance(request, dict):
        headers = {str(k).lower(): v for k, v in (request.get("headers") or {}).items()}
        header_connector = str(headers.get("x-connector") or "").strip()
        if header_connector:
            connectors.add(header_connector)
    return connectors


def main() -> int:
    captures = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent / "captures"
    if not captures.exists():
        print(f"No captures directory at {captures}; nothing to check.")
        return 0

    pairs = _replacement_pairs()
    if not pairs:
        print(f"No credential values loaded from {creds_path()}; redaction check skipped.")
        return 0

    failures: list[tuple[Path, str]] = []
    for fpath in sorted(captures.glob("**/*.json")):
        text = fpath.read_text(errors="replace")
        try:
            record = json.loads(text)
        except Exception:
            record = None
        relevant_connectors = _record_connectors(record, captures.name)
        for actual, placeholder, substring_ok in pairs:
            if relevant_connectors and _placeholder_connectors(placeholder).isdisjoint(relevant_connectors):
                continue
            if len(actual) < 8 and not substring_ok:
                continue
            if actual in text:
                failures.append((fpath, f"raw credential; expected placeholder {placeholder}"))
                break
        for pan in _known_test_pans():
            if pan in text:
                failures.append((fpath, "unredacted Cypress test card number"))
                break

        if record is None:
            continue
        for path in _find_unredacted_card_values(record):
            failures.append((fpath, f"unredacted card field {path}"))
            break
        request = record.get("request") if isinstance(record, dict) else None
        if isinstance(request, dict):
            for path in _find_unredacted_request_values(request):
                failures.append((fpath, f"unredacted request field {path}"))
                break
        headers = (((record.get("response") or {}).get("headers") or {}) if isinstance(record, dict) else {})
        if isinstance(headers, dict):
            for key, value in headers.items():
                if str(key).lower() in {"set-cookie", "cookie"} and value and not _placeholder(value):
                    failures.append((fpath, f"unredacted response header {key}"))
                    break

    if failures:
        print("Unredacted cassette values found:")
        for fpath, reason in failures[:50]:
            print(f"  {fpath}: {reason}")
        if len(failures) > 50:
            print(f"  ... {len(failures) - 50} more")
        print("Run: python3 mitm-proxy/normalize_captures.py mitm-proxy/captures [connector ...]")
        return 1

    print(f"OK: no raw credential/cardholder values found under {captures}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
