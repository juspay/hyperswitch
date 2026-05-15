"""Credential redaction/hydration helpers for MITM cassettes.

Captured connector requests can contain connector credentials in headers, URLs,
form bodies, or JSON payloads. Cassettes are intended to be committed to the
repo for CI replay, so any value derived from ``creds.json`` must be stored as a
placeholder and materialized again only at replay time.

The placeholder is intentionally stable and path-based, e.g.
``{{MITM_SECRET:paypal.connector_account_details.api_key}}``. It contains the
credential field name but never the credential value.
"""

from __future__ import annotations

import base64
import copy
import json
import os
import re
from functools import lru_cache
from pathlib import Path
from typing import Any
from urllib.parse import quote, quote_plus

_PLACEHOLDER_PREFIX = "{{MITM_SECRET:"
_PLACEHOLDER_RE = re.compile(r"\{\{MITM_SECRET:[^{}]+\}\}")
_REDACTED_PREFIX = "{{MITM_REDACTED:"
# Keep redaction intentionally narrow. The goal is to remove connector
# credentials, not every metadata string that happens to appear in creds.json
# (for example Google Pay gateway names or merchant display names). Connector
# auth payloads live under connector_account_details; outside that subtree we
# redact only clear secret-looking leaf keys.
_SENSITIVE_LEAF_KEYS = {
    "api_key",
    "api_secret",
    "key1",
    "key2",
    "key3",
    "secret",
    "client_id",
    "client_secret",
    "private_key",
    "certificate",
    "token",
    "access_token",
    "refresh_token",
    "password",
    "passwd",
    "merchant_key",
    "merchant_secret",
    "auth_key",
}
_SENSITIVE_KEY_PREFIXES = (
    "api_key",
    "api_secret",
    "client_secret",
    "private_key",
    "password",
)
_EXCLUDED_KEYS = {"auth_type"}
_COMMON_NON_SECRETS = {
    "usd", "eur", "gbp", "inr", "aud", "cad", "jpy", "sgd", "nzd",
    "true", "false", "null", "none", "test", "sandbox", "manual",
}
_MIN_SUBSTRING_SECRET_LEN = 8
_MIN_EXACT_SECRET_LEN = 4

# Request payloads are retained for debugging only; replay matching uses the
# recorded connector/method/path/request id and returns the recorded response.
# Keep committed cassettes free of cardholder data / transient connector cookies.
_REQUEST_SENSITIVE_KEYS = {
    "cardnumber",
    "card_number",
    "cardno",
    "card_no",
    "pan",
    "securitycode",
    "security_code",
    "cvv",
    "cvc",
    "cardcvv",
    "card_cvv",
    "cvv2",
    "cardverificationvalue",
    "cardholdername",
    "card_holder_name",
    "cardholder_name",
    "firstname",
    "first_name",
    "lastname",
    "last_name",
    "email",
    "phone",
    "phonenumber",
    "phone_number",
    "addressline1",
    "address_line1",
    "addressline2",
    "address_line2",
    "line1",
    "line2",
    "city",
    "state",
    "zip",
    "zipcode",
    "zip_code",
    "postalcode",
    "postal_code",
}
_REQUEST_SENSITIVE_KEY_PARTS = ("cardnumber", "securitycode")
_REQUEST_SENSITIVE_RE = re.compile(
    r'(?i)((?:cardNumber|card_number|securityCode|security_code|cvv|cvc|email|firstName|lastName)'
    r'(?:"\s*:\s*"|=|>))([^"&<]+)'
)
_COOKIE_HEADER_RE = re.compile(r"(?i)^(set-cookie|cookie)$")
_PAN_IN_REQUEST_RE = re.compile(r"\b\d{13,19}\b")
_CARD_NUMBER_CONFIG_RE = re.compile(r"\bcard_number\s*:\s*['\"](\d{13,19})['\"]")


def _repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def _default_creds_path() -> Path:
    # mitm-proxy/secret_redaction.py -> repo root / creds.json
    return _repo_root() / "creds.json"


def creds_path() -> Path:
    return Path(os.environ.get("CREDS_FILE") or _default_creds_path())


@lru_cache(maxsize=1)
def _load_creds() -> dict[str, Any]:
    path = creds_path()
    if not path.exists():
        return {}
    try:
        with path.open() as f:
            data = json.load(f)
        return data if isinstance(data, dict) else {}
    except Exception as exc:  # noqa: BLE001 - mitm addon should stay usable
        print(f"[secrets] WARN unable to load creds file {path}: {exc}")
        return {}


def _path_label(path: tuple[str, ...]) -> str:
    return ".".join(path)


def _placeholder(label: str) -> str:
    return f"{_PLACEHOLDER_PREFIX}{label}}}}}"


def _is_secret_leaf(path: tuple[str, ...], value: str) -> bool:
    if not value:
        return False
    key = (path[-1] if path else "").lower()
    if key in _EXCLUDED_KEYS:
        return False
    lowered = value.strip().lower()
    if lowered in _COMMON_NON_SECRETS:
        return False

    # Connector auth is always considered sensitive, including nested maps such
    # as cashtocode.connector_account_details.auth_key_map.EUR.password_classic.
    if "connector_account_details" in {part.lower() for part in path}:
        return len(value) >= _MIN_EXACT_SECRET_LEN

    # Outside connector_account_details, only redact explicit credential leaf
    # keys. Do not match broad substrings like "merchant" or "tokenization" in
    # metadata paths; those created false placeholders in connector names and
    # Google Pay metadata.
    if key in _SENSITIVE_LEAF_KEYS or any(key.startswith(prefix) for prefix in _SENSITIVE_KEY_PREFIXES):
        return len(value) >= _MIN_EXACT_SECRET_LEN

    return False


def _walk_secret_leaves(obj: Any, path: tuple[str, ...] = ()) -> list[tuple[tuple[str, ...], str]]:
    leaves: list[tuple[tuple[str, ...], str]] = []
    if isinstance(obj, dict):
        for key, value in obj.items():
            leaves.extend(_walk_secret_leaves(value, (*path, str(key))))
    elif isinstance(obj, list):
        for idx, value in enumerate(obj):
            leaves.extend(_walk_secret_leaves(value, (*path, str(idx))))
    elif isinstance(obj, str) and _is_secret_leaf(path, obj):
        leaves.append((path, obj))
    return leaves


def _variant_map_from_creds(creds: dict[str, Any]) -> dict[str, str]:
    """Return placeholder -> actual value map for raw and derived secrets."""
    out: dict[str, str] = {}
    by_connector: dict[str, list[tuple[tuple[str, ...], str]]] = {}

    for connector, cfg in creds.items():
        leaves = _walk_secret_leaves(cfg, (str(connector),))
        by_connector[str(connector)] = leaves
        for path, value in leaves:
            label = _path_label(path)
            out[_placeholder(label)] = value
            # Common encodings seen in URL-encoded bodies or connector headers.
            out[_placeholder(f"urlquote:{label}")] = quote(value, safe="")
            out[_placeholder(f"urlplus:{label}")] = quote_plus(value)
            out[_placeholder(f"base64:{label}")] = base64.b64encode(value.encode()).decode()

    # HTTP Basic auth frequently stores only base64(username:password), so the
    # raw credential values are not visible in the cassette. Generate ordered
    # pairs per connector; connector credential sets are small and this keeps
    # replay deterministic without connector-specific auth knowledge.
    for connector, leaves in by_connector.items():
        for left_path, left_value in leaves:
            for right_path, right_value in leaves:
                if left_path == right_path:
                    continue
                pair = f"{left_value}:{right_value}"
                pair_label = f"{_path_label(left_path)}+{_path_label(right_path)}"
                out[_placeholder(f"pair:{pair_label}")] = pair
                out[_placeholder(f"basic64:{pair_label}")] = base64.b64encode(pair.encode()).decode()
                out[_placeholder(f"basic:{pair_label}")] = "Basic " + base64.b64encode(pair.encode()).decode()

    # Remove no-op/empty variants and prefer longer concrete values when
    # redacting substrings so nested values do not partially redact first.
    return {ph: actual for ph, actual in out.items() if actual}


@lru_cache(maxsize=1)
def _replacement_pairs() -> tuple[tuple[str, str, bool], ...]:
    """Return (actual, placeholder, substring_ok) replacement triples."""
    pairs = [
        (actual, placeholder, len(actual) >= _MIN_SUBSTRING_SECRET_LEN)
        for placeholder, actual in _variant_map_from_creds(_load_creds()).items()
        if len(actual) >= _MIN_EXACT_SECRET_LEN
    ]
    # Longest actual first prevents partial replacement of overlapping secrets.
    pairs.sort(key=lambda item: len(item[0]), reverse=True)
    return tuple(pairs)


@lru_cache(maxsize=1)
def _hydration_pairs() -> tuple[tuple[str, str], ...]:
    return tuple(_variant_map_from_creds(_load_creds()).items())


@lru_cache(maxsize=1)
def _known_test_pans() -> tuple[str, ...]:
    """Return Cypress test PANs that may appear in request/echo payloads."""
    root = _repo_root() / "cypress-tests" / "cypress" / "e2e" / "configs"
    pans: set[str] = set()
    if root.exists():
        for path in root.glob("**/*.js"):
            try:
                text = path.read_text(errors="ignore")
            except Exception:
                continue
            pans.update(_CARD_NUMBER_CONFIG_RE.findall(text))
    return tuple(sorted(pans, key=len, reverse=True))


def _redact_known_test_pans(value: str) -> tuple[str, int]:
    out = value
    changed = 0
    for pan in _known_test_pans():
        if pan in out:
            n = out.count(pan)
            out = out.replace(pan, _redacted("card.pan"))
            changed += n
    return out, changed


def _redact_string(value: str, pairs: list[tuple[str, str, bool]]) -> tuple[str, int]:
    changed = 0
    out = value
    for actual, placeholder, substring_ok in pairs:
        if out == actual:
            out = placeholder
            changed += 1
            continue
        if substring_ok and actual in out:
            n = out.count(actual)
            out = out.replace(actual, placeholder)
            changed += n
    return out, changed


def _hydrate_string(value: str, pairs: list[tuple[str, str]]) -> tuple[str, int]:
    changed = 0
    out = value
    for placeholder, actual in pairs:
        if placeholder in out:
            n = out.count(placeholder)
            out = out.replace(placeholder, actual)
            changed += n
    return out, changed


def _transform(obj: Any, string_fn) -> tuple[Any, int]:
    if isinstance(obj, dict):
        total = 0
        transformed: dict[Any, Any] = {}
        for key, value in obj.items():
            new_value, changed = _transform(value, string_fn)
            transformed[key] = new_value
            total += changed
        return transformed, total
    if isinstance(obj, list):
        total = 0
        transformed_list = []
        for value in obj:
            new_value, changed = _transform(value, string_fn)
            transformed_list.append(new_value)
            total += changed
        return transformed_list, total
    if isinstance(obj, str):
        return string_fn(obj)
    return obj, 0


def redact_obj(obj: Any) -> tuple[Any, int]:
    pairs = _replacement_pairs()
    if not pairs:
        return obj, 0
    return _transform(obj, lambda value: _redact_string(value, pairs))


def hydrate_obj(obj: Any) -> tuple[Any, int]:
    pairs = _hydration_pairs()
    if not pairs:
        return obj, 0
    return _transform(obj, lambda value: _hydrate_string(value, pairs))


def _redacted(label: str) -> str:
    safe = re.sub(r"[^A-Za-z0-9_.-]+", "_", label).strip("_") or "value"
    return f"{_REDACTED_PREFIX}{safe}}}}}"


def _is_card_secret_key(key: str) -> bool:
    normalized = re.sub(r"[^a-z0-9]", "", key.lower())
    return normalized in {
        "cardnumber", "cardno", "pan", "securitycode", "cvv", "cvc", "cardcvv", "cvv2",
        "cardverificationvalue",
    }


def _is_request_sensitive_key(key: str) -> bool:
    normalized = re.sub(r"[^a-z0-9]", "", key.lower())
    return (
        key.lower() in _REQUEST_SENSITIVE_KEYS
        or normalized in _REQUEST_SENSITIVE_KEYS
        or any(part in normalized for part in _REQUEST_SENSITIVE_KEY_PARTS)
    )


def _redact_request_string(value: str) -> tuple[str, int]:
    changed = 0

    def repl(match: re.Match) -> str:
        nonlocal changed
        changed += 1
        prefix = match.group(1)
        key = re.split(r'[:=>]', prefix, maxsplit=1)[0].strip('"')
        return prefix + _redacted(f"request.{key}")

    out = _REQUEST_SENSITIVE_RE.sub(repl, value)
    out, pan_count = _PAN_IN_REQUEST_RE.subn(_redacted("request.card.pan"), out)
    return out, changed + pan_count


def _redact_request_obj(obj: Any, path: tuple[str, ...] = ()) -> tuple[Any, int]:
    if isinstance(obj, dict):
        total = 0
        transformed: dict[Any, Any] = {}
        for key, value in obj.items():
            skey = str(key)
            path_has_card_context = any("card" in part.lower() for part in path)
            normalized_key = re.sub(r"[^a-z0-9]", "", skey.lower())
            if (
                _is_request_sensitive_key(skey)
                or (normalized_key == "number" and path_has_card_context)
            ) and value not in (None, ""):
                transformed[key] = _redacted("request." + ".".join((*path, skey)))
                total += 1
            else:
                transformed[key], changed = _redact_request_obj(value, (*path, skey))
                total += changed
        return transformed, total
    if isinstance(obj, list):
        total = 0
        transformed_list = []
        for idx, value in enumerate(obj):
            new_value, changed = _redact_request_obj(value, (*path, str(idx)))
            transformed_list.append(new_value)
            total += changed
        return transformed_list, total
    if isinstance(obj, str):
        return _redact_request_string(obj)
    return obj, 0


def _redact_card_echo_obj(obj: Any, path: tuple[str, ...] = ()) -> tuple[Any, int]:
    if isinstance(obj, dict):
        total = 0
        transformed: dict[Any, Any] = {}
        for key, value in obj.items():
            skey = str(key)
            path_has_card_context = any("card" in part.lower() for part in path)
            normalized_key = re.sub(r"[^a-z0-9]", "", skey.lower())
            if (
                _is_card_secret_key(skey)
                or (normalized_key == "number" and path_has_card_context)
            ) and value not in (None, "") and not (isinstance(value, str) and value.startswith(_REDACTED_PREFIX)):
                transformed[key] = _redacted(".".join((*path, skey)))
                total += 1
            else:
                transformed[key], changed = _redact_card_echo_obj(value, (*path, skey))
                total += changed
        return transformed, total
    if isinstance(obj, list):
        total = 0
        transformed_list = []
        for idx, value in enumerate(obj):
            new_value, changed = _redact_card_echo_obj(value, (*path, str(idx)))
            transformed_list.append(new_value)
            total += changed
        return transformed_list, total
    return obj, 0


def _redact_transient_response_headers(record: dict[str, Any]) -> int:
    headers = ((record.get("response") or {}).get("headers") or {})
    if not isinstance(headers, dict):
        return 0
    changed = 0
    for key, value in list(headers.items()):
        if _COOKIE_HEADER_RE.match(str(key)) and value:
            headers[key] = _redacted(f"response.headers.{key}")
            changed += 1
    return changed


def redact_record(record: dict[str, Any]) -> int:
    redacted, credential_count = redact_obj(record)
    if isinstance(redacted, dict):
        record.clear()
        record.update(redacted)

    pan_redacted, pan_count = _transform(record, _redact_known_test_pans)
    if isinstance(pan_redacted, dict):
        record.clear()
        record.update(pan_redacted)

    card_echo_redacted, card_echo_count = _redact_card_echo_obj(record)
    if isinstance(card_echo_redacted, dict):
        record.clear()
        record.update(card_echo_redacted)

    request_count = 0
    request = record.get("request")
    if isinstance(request, dict):
        redacted_request, request_count = _redact_request_obj(request, ("request",))
        record["request"] = redacted_request

    header_count = _redact_transient_response_headers(record)
    count = credential_count + pan_count + card_echo_count + request_count + header_count
    if count:
        meta = record.setdefault("redaction", {})
        if isinstance(meta, dict):
            meta["version"] = 2
            meta["placeholder_count"] = credential_count
            meta["test_pan_placeholder_count"] = pan_count
            meta["card_echo_placeholder_count"] = card_echo_count
            meta["request_placeholder_count"] = request_count
            meta["response_header_placeholder_count"] = header_count
    return count


def hydrate_record(record: dict[str, Any]) -> tuple[dict[str, Any], int]:
    # Return a copy so replay queues/sticky GET cache can safely retain the
    # placeholder form if needed.
    hydrated, count = hydrate_obj(copy.deepcopy(record))
    return hydrated, count


def redact_file(path: Path) -> int:
    with path.open() as f:
        record = json.load(f)
    count = redact_record(record)
    if count:
        with path.open("w") as f:
            json.dump(record, f, indent=2)
            f.write("\n")
    return count


def has_unresolved_placeholders(obj: Any) -> bool:
    if isinstance(obj, dict):
        return any(has_unresolved_placeholders(v) for v in obj.values())
    if isinstance(obj, list):
        return any(has_unresolved_placeholders(v) for v in obj)
    if isinstance(obj, str):
        return bool(_PLACEHOLDER_RE.search(obj))
    return False
