#!/usr/bin/env python3

import csv
import json
import os
import sys
import time
from pathlib import Path

import requests

# === Config ===
SCRIPT_DIR = Path(__file__).resolve().parent
INPUT_CSV = SCRIPT_DIR / "local_data.csv"
OUTPUT_CSV = SCRIPT_DIR / "local_data_migration_result.csv"
FAILURE_CSV = SCRIPT_DIR / "failed_algorithms.csv"

API_URL = os.getenv(
    "HYPERSWITCH_RULE_MIGRATION_URL",
    "https://sandbox.hyperswitch.io/routing/rule/migrate",
)
API_KEY = "f%qjG@QW&rfh9VrQ6AzELe2kETH^fFhLKu5VNxC7K8P3KNA$H54n^jccGR^V*p4C"
PAGE_LIMIT = 1000
REQUEST_TIMEOUT_SECONDS = 120

HEADERS = {
    "Content-Type": "application/json",
    "Accept": "application/json",
    "api-key": API_KEY
}

def set_max_csv_field_size():
    # Some profiles can have very large all_algorithm_ids payloads.
    # Raise parser limit to avoid: _csv.Error: field larger than field limit (131072)
    field_limit = sys.maxsize
    while True:
        try:
            csv.field_size_limit(field_limit)
            return
        except OverflowError:
            field_limit = field_limit // 10

def extract_source_algorithm_id(success_item):
    # Different sandbox builds can return different field names for source algorithm ID.
    for key in ("euclid_algorithm_id", "algorithm_id", "routing_algorithm_id"):
        value = success_item.get(key)
        if isinstance(value, str) and value:
            return value
    return None

def extract_decision_engine_id(success_item):
    for key in ("decision_engine_routing_id", "decision_engine_algorithm_id"):
        value = success_item.get(key)
        if isinstance(value, str) and value:
            return value
    return None

def parse_algorithm_ids(all_algorithm_ids_raw):
    return json.loads(all_algorithm_ids_raw.replace('""', '"'))

def collect_resume_state(all_algos_by_profile):
    processed_profiles = set()
    restored_failed_rows = []

    if not OUTPUT_CSV.exists():
        return processed_profiles, restored_failed_rows

    with open(OUTPUT_CSV, newline='') as existing_output:
        reader = csv.DictReader(existing_output)
        for row in reader:
            profile_id = (row.get("profile_id") or "").strip()
            if not profile_id:
                continue

            processed_profiles.add(profile_id)
            not_migrated_value = (row.get("not_migrated_algorithm_ids") or "").strip()
            if not not_migrated_value:
                continue

            if not_migrated_value.startswith("ERROR:"):
                for algo_id in all_algos_by_profile.get(profile_id, []):
                    restored_failed_rows.append((profile_id, algo_id))
                continue

            try:
                not_migrated_ids = json.loads(not_migrated_value)
            except json.JSONDecodeError:
                continue

            if isinstance(not_migrated_ids, list):
                for algo_id in not_migrated_ids:
                    if isinstance(algo_id, str):
                        restored_failed_rows.append((profile_id, algo_id))

    return processed_profiles, restored_failed_rows

def main():
    set_max_csv_field_size()
    with open(INPUT_CSV, newline='') as infile:
        input_reader = csv.DictReader(infile)
        input_fieldnames = input_reader.fieldnames
        input_rows = list(input_reader)

    if not input_fieldnames:
        raise RuntimeError("Input CSV has no headers")

    all_algos_by_profile = {}
    for row in input_rows:
        profile_id = (row.get("profile_id") or "").strip()
        if not profile_id:
            continue
        try:
            all_algos_by_profile[profile_id] = parse_algorithm_ids(row["all_algorithm_ids"])
        except Exception:
            all_algos_by_profile[profile_id] = []

    processed_profiles, failed_rows = collect_resume_state(all_algos_by_profile)
    output_exists = OUTPUT_CSV.exists()
    write_header = (not output_exists) or OUTPUT_CSV.stat().st_size == 0
    output_mode = "a" if output_exists else "w"

    if processed_profiles:
        print(
            f"↻ Resume mode: found {len(processed_profiles)} already processed profiles in "
            f"{OUTPUT_CSV}. Continuing from next profile."
        )

    fieldnames = ["status"] + input_fieldnames + ["not_migrated_algorithm_ids"]
    api_call_counter = 0
    interrupted = False
    processed_now = 0

    with open(OUTPUT_CSV, output_mode, newline='') as outfile:
        writer = csv.DictWriter(outfile, fieldnames=fieldnames)
        if write_header:
            writer.writeheader()

        try:
            for row in input_rows:
                profile_id = row["profile_id"]
                if profile_id in processed_profiles:
                    continue

                processed_now += 1
                merchant_id = row["merchant_id"]
                all_algos = all_algos_by_profile.get(profile_id, [])

                try:
                    migrated_ids = set()
                    error_ids = set()
                    offset = 0
                    total_success_items = 0
                    total_error_items = 0
                    saw_mappable_success = False
                    saw_unmapped_success = False
                    decision_engine_ids = set()

                    while True:
                        params = {
                            "profile_id": profile_id,
                            "limit": PAGE_LIMIT,
                            "offset": offset,
                            "merchant_id": merchant_id
                        }

                        response = requests.post(
                            API_URL,
                            headers=HEADERS,
                            params=params,
                            timeout=REQUEST_TIMEOUT_SECONDS,
                            allow_redirects=False,
                        )
                        api_call_counter += 1
                        print(
                            f"Calling API for profile_id={profile_id} "
                            f"(offset={offset}, limit={PAGE_LIMIT}) → HTTP {response.status_code}"
                        )

                        if api_call_counter % 5 == 0:
                            time.sleep(5)

                        if 300 <= response.status_code < 400:
                            redirect_target = response.headers.get("Location", "<missing>")
                            raise RuntimeError(
                                "Redirect received from migration endpoint. "
                                f"HTTP {response.status_code} Location={redirect_target}"
                            )

                        if response.status_code != 200:
                            raise RuntimeError(f"HTTP {response.status_code}: {response.text}")

                        data = response.json()
                        success_items = data.get("success", [])
                        error_items = data.get("errors", [])
                        total_success_items += len(success_items)
                        total_error_items += len(error_items)

                        for item in success_items:
                            source_algorithm_id = extract_source_algorithm_id(item)
                            decision_engine_id = extract_decision_engine_id(item)
                            if decision_engine_id:
                                decision_engine_ids.add(decision_engine_id)
                            if source_algorithm_id:
                                migrated_ids.add(source_algorithm_id)
                                saw_mappable_success = True
                            elif decision_engine_id:
                                saw_unmapped_success = True

                        for item in error_items:
                            algorithm_id = item.get("algorithm_id")
                            if algorithm_id:
                                error_ids.add(algorithm_id)

                        processed_in_page = len(success_items) + len(error_items)
                        if processed_in_page < PAGE_LIMIT:
                            break

                        offset += PAGE_LIMIT

                    if saw_mappable_success:
                        not_migrated = [
                            algo for algo in all_algos if algo not in migrated_ids or algo in error_ids
                        ]
                    elif saw_unmapped_success:
                        # Fallback for API variants that return only decision-engine IDs in success entries.
                        # In this mode we can only trust explicit error IDs.
                        not_migrated = [algo for algo in all_algos if algo in error_ids]
                        print(
                            "⚠ Success payload does not include source algorithm IDs; "
                            "using errors-only reconciliation."
                        )
                    elif total_error_items > 0:
                        # No success entries, only explicit failures returned by the API.
                        not_migrated = [algo for algo in all_algos if algo in error_ids]
                    else:
                        # Ambiguous case: API returned neither success nor errors.
                        not_migrated = list(all_algos)
                        print(
                            "⚠ API returned empty success/errors for this profile; "
                            "marking all as not migrated."
                        )

                    for algo in not_migrated:
                        failed_rows.append((profile_id, algo))

                    row["not_migrated_algorithm_ids"] = json.dumps(not_migrated)
                    row["status"] = "✅" if not not_migrated else "❌"

                    print(
                        f"✔ Success items: {total_success_items} | "
                        f"❌ Error items: {total_error_items} | "
                        f"❌ Failed IDs: {len(not_migrated)} | "
                        f"✅ Decision-engine IDs: {len(decision_engine_ids)}"
                    )

                except Exception as e:
                    print(f"❌ Exception for profile_id={profile_id}: {str(e)}")
                    row["not_migrated_algorithm_ids"] = f"ERROR: {str(e)}"
                    row["status"] = "❌"
                    for algo in all_algos:
                        failed_rows.append((profile_id, algo))

                writer.writerow(row)
                outfile.flush()
                processed_profiles.add(profile_id)
        except KeyboardInterrupt:
            interrupted = True
            print("\n⚠ Interrupted by user. Progress is saved. Re-run to resume.")

    # === Write Failed Routing IDs to separate CSV ===
    failed_rows = sorted(set(failed_rows))
    with open(FAILURE_CSV, 'w', newline='') as failfile:
        fail_writer = csv.writer(failfile)
        fail_writer.writerow(["serial_id", "profile_id", "algorithm_id"])
        for idx, (profile_id, algo_id) in enumerate(failed_rows, start=1):
            fail_writer.writerow([idx, profile_id, algo_id])

    if interrupted:
        print(f"\n⚠ Migration interrupted after processing {processed_now} new profiles.")
    else:
        print(f"\n✅ Migration complete. Processed {processed_now} new profiles.")
    print(f"🔹 Full result → {OUTPUT_CSV}")
    print(f"🔻 Failures only → {FAILURE_CSV}")

if __name__ == "__main__":
    main()
