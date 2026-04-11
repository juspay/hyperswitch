#!/usr/bin/env python3
"""
Script to add BINs from bin.csv to Hyperswitch blocklist via the REST API.

Usage:
    python3 add_bins_to_blocklist.py \
        --api-base-url http://localhost:8080 \
        --api-key <merchant_api_key> \
        --csv-file bin.csv \
        --concurrency 10 \
        --progress-file blocklist_progress.json

Behaviour:
  - 6-digit BINs  → POST /blocklist  {"type": "card_bin", "data": "..."}
  - 8-digit BINs  → POST /blocklist  {"type": "extended_card_bin", "data": "..."}
  - 7-digit BINs  → skipped with warning (API only accepts 6 or 8 digits)
  - Already-blocked BINs (HTTP 400/409/412 duplicate semantics) → logged as skipped
  - Resumable: only added/skipped BINs are persisted in --progress-file
"""

import argparse
import csv
import json
import logging
import os
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed

import requests

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)],
)
logger = logging.getLogger(__name__)

HTTP_400 = 400
HTTP_409 = 409
HTTP_412 = 412
HTTP_200 = 200
_THREAD_LOCAL = threading.local()


def parse_args():
    parser = argparse.ArgumentParser(
        description="Add BINs from CSV to Hyperswitch blocklist"
    )
    parser.add_argument(
        "--api-base-url",
        required=True,
        help="Base URL of the Hyperswitch server (e.g. http://localhost:8080)",
    )
    parser.add_argument(
        "--api-key",
        required=True,
        help="Merchant API key for authentication",
    )
    parser.add_argument(
        "--csv-file",
        default="bin.csv",
        help="Path to the CSV file containing BINs (default: bin.csv)",
    )
    parser.add_argument(
        "--concurrency",
        type=int,
        default=10,
        help="Number of concurrent API calls (default: 10)",
    )
    parser.add_argument(
        "--progress-file",
        default="blocklist_progress.json",
        help="File to track progress for resume capability (default: blocklist_progress.json)",
    )
    return parser.parse_args()


def read_bins_from_csv(csv_path: str) -> list[dict]:
    bins = []
    seen = set()
    with open(csv_path, "r", encoding="utf-8") as f:
        sample = f.read(4096)
        f.seek(0)
        delimiter = ";"
        try:
            dialect = csv.Sniffer().sniff(sample, delimiters=";,")
            delimiter = dialect.delimiter
        except csv.Error:
            pass

        reader = csv.reader(f, delimiter=delimiter)
        header = next(reader, None)
        if header is None:
            logger.error("CSV file is empty")
            return bins

        normalized_header = [h.strip().lower() for h in header]
        if normalized_header:
            normalized_header[0] = normalized_header[0].lstrip("\ufeff")

        bin_index = 1
        if "bin" in normalized_header:
            bin_index = normalized_header.index("bin")

        for row in reader:
            if len(row) <= bin_index:
                continue
            bin_value = row[bin_index].strip().strip(",")
            if not bin_value or not bin_value.isdigit():
                continue
            if bin_value in seen:
                continue
            seen.add(bin_value)
            bin_len = len(bin_value)
            if bin_len == 6:
                bins.append({"bin": bin_value, "type": "card_bin"})
            elif bin_len == 8:
                bins.append({"bin": bin_value, "type": "extended_card_bin"})
            elif bin_len == 7:
                logger.warning("Skipping 7-digit BIN (unsupported): %s", bin_value)
            else:
                logger.warning(
                    "Skipping BIN with unexpected length %d: %s",
                    bin_len,
                    bin_value,
                )
    return bins


def load_progress(progress_file: str) -> set:
    if not os.path.exists(progress_file):
        return set()
    try:
        with open(progress_file, "r") as f:
            data = json.load(f)
            return set(data.get("processed", []))
    except (json.JSONDecodeError, IOError):
        logger.warning("Could not read progress file, starting fresh")
        return set()


def save_progress(progress_file: str, processed: set):
    tmp = progress_file + ".tmp"
    with open(tmp, "w") as f:
        json.dump({"processed": sorted(processed)}, f)
    os.replace(tmp, progress_file)


def _get_thread_local_session() -> requests.Session:
    session = getattr(_THREAD_LOCAL, "session", None)
    if session is None:
        session = requests.Session()
        _THREAD_LOCAL.session = session
    return session


def _is_duplicate_response(resp: requests.Response) -> bool:
    if resp.status_code in (HTTP_409, HTTP_412):
        return True

    if resp.status_code != HTTP_400:
        return False

    try:
        body = resp.json()
    except ValueError:
        return False

    error_obj = body.get("error", body) if isinstance(body, dict) else {}
    code = str(error_obj.get("code", "")).upper()
    message = str(error_obj.get("message", "")).lower()

    duplicate_codes = {"IR_16", "IR_38", "HE_01"}
    duplicate_markers = ("already blocked", "already exists", "duplicate")
    return code in duplicate_codes or any(marker in message for marker in duplicate_markers)


def add_bin_to_blocklist(
    base_url: str,
    api_key: str,
    bin_entry: dict,
) -> dict:
    bin_value = bin_entry["bin"]
    bin_type = bin_entry["type"]
    url = f"{base_url}/blocklist"
    payload = {"type": bin_type, "data": bin_value}
    headers = {
        "Content-Type": "application/json",
        "api_key": api_key,
    }
    try:
        session = _get_thread_local_session()
        resp = session.post(url, json=payload, headers=headers, timeout=30)
        if resp.status_code == HTTP_200:
            return {"bin": bin_value, "status": "added"}
        if _is_duplicate_response(resp):
            return {"bin": bin_value, "status": "already_exists"}
        return {
            "bin": bin_value,
            "status": "error",
            "code": resp.status_code,
            "body": resp.text[:200],
        }
    except requests.RequestException as e:
        return {"bin": bin_value, "status": "error", "code": 0, "body": str(e)[:200]}


def main():
    args = parse_args()
    base_url = args.api_base_url.rstrip("/")
    api_key = args.api_key
    csv_path = args.csv_file
    concurrency = args.concurrency
    progress_file = args.progress_file

    if not os.path.exists(csv_path):
        logger.error("CSV file not found: %s", csv_path)
        sys.exit(1)
    if concurrency <= 0:
        logger.error("--concurrency must be a positive integer")
        sys.exit(1)

    logger.info("Reading BINs from %s ...", csv_path)
    bins = read_bins_from_csv(csv_path)
    logger.info(
        "Found %d unique processable BINs (6-digit: card_bin, 8-digit: extended_card_bin)",
        len(bins),
    )

    if not bins:
        logger.info("No BINs to process. Exiting.")
        sys.exit(0)

    processed = load_progress(progress_file)
    remaining = [b for b in bins if b["bin"] not in processed]
    logger.info(
        "Already processed: %d | Remaining: %d", len(processed), len(remaining)
    )

    if not remaining:
        logger.info("All BINs already processed. Nothing to do.")
        sys.exit(0)

    stats = {"added": 0, "already_exists": 0, "error": 0}
    total = len(remaining)
    last_save = time.time()

    with ThreadPoolExecutor(max_workers=concurrency) as executor:
        futures = {
            executor.submit(
                add_bin_to_blocklist, base_url, api_key, bin_entry
            ): bin_entry
            for bin_entry in remaining
        }
        for i, future in enumerate(as_completed(futures), 1):
            result = future.result()
            status = result["status"]
            stats[status] += 1

            if status in ("added", "already_exists"):
                processed.add(result["bin"])

            if status == "error":
                logger.error(
                    "Error for BIN %s: HTTP %s - %s",
                    result["bin"],
                    result.get("code"),
                    result.get("body", ""),
                )

            if i % 100 == 0 or i == total:
                logger.info(
                    "Progress: %d/%d | added=%d already_exists=%d error=%d",
                    i,
                    total,
                    stats["added"],
                    stats["already_exists"],
                    stats["error"],
                )

            now = time.time()
            if now - last_save >= 5 or i == total:
                save_progress(progress_file, processed)
                last_save = now

    save_progress(progress_file, processed)

    logger.info("=" * 60)
    logger.info("FINAL SUMMARY")
    logger.info("  Total processed this run : %d", total)
    logger.info("  Added                    : %d", stats["added"])
    logger.info("  Already existed (skipped): %d", stats["already_exists"])
    logger.info("  Errors                   : %d", stats["error"])
    logger.info("  Cumulative processed     : %d", len(processed))
    logger.info("  Progress file            : %s", progress_file)
    logger.info("=" * 60)

    if stats["error"] > 0:
        logger.warning(
            "There were %d errors. Re-run the script to retry failed BINs.",
            stats["error"],
        )


if __name__ == "__main__":
    main()
