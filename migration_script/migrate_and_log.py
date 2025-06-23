import csv
import requests
import json
import time
from pathlib import Path

# === Config ===
SCRIPT_DIR = Path(__file__).resolve().parent
INPUT_CSV = SCRIPT_DIR / "local_data.csv"
OUTPUT_CSV = SCRIPT_DIR / "local_data_migration_result.csv"
FAILURE_CSV = SCRIPT_DIR / "failed_algorithms.csv"

# API_URL = "https://integ-api.hyperswitch.io/routing/rule/migrate"
# API_KEY = "BbprfuJw3fA5Ar5EGigEPsQ!ervJ&ywR!TV!Y*$H^obR$d@5Bx^AA@#QFh3LsrK9"
API_URL = "http://localhost:8080/routing/rule/migrate"
API_KEY = "test_admin"

HEADERS = {
    "Content-Type": "application/json",
    "Accept": "application/json",
    "api-key": API_KEY
}

failed_rows = []

with open(INPUT_CSV, newline='') as infile, open(OUTPUT_CSV, 'w', newline='') as outfile:
    reader = csv.DictReader(infile)
    fieldnames = ["status"] + reader.fieldnames + ["not_migrated_algorithm_ids"]
    writer = csv.DictWriter(outfile, fieldnames=fieldnames)
    writer.writeheader()

    counter = 0
    for row in reader:
        counter+=1
        profile_id = row["profile_id"]
        merchant_id = row["merchant_id"]

        try:
            all_algos = json.loads(row["all_algorithm_ids"].replace('""', '"'))

            params = {
                "profile_id": profile_id,
                "limit": 1000,
                "offset": 0,
                "merchant_id": merchant_id
            }

            response = requests.post(API_URL, headers=HEADERS, params=params)
            print(f"Calling API for profile_id={profile_id} → HTTP {response.status_code}")

            if response.status_code != 200:
                print(f"❌ Error response: {response.text}")
                row["not_migrated_algorithm_ids"] = f"ERROR: HTTP {response.status_code}"
                row["status"] = "❌"
                writer.writerow(row)
                for algo_id in all_algos:
                    failed_rows.append((profile_id, algo_id))
                continue

            data = response.json()

            migrated_ids = {
                item["decision_engine_algorithm_id"]
                for item in data.get("success", [])
            }

            error_ids = {
                item["algorithm_id"]
                for item in data.get("errors", [])
            }

            not_migrated = [
                algo for algo in all_algos
                if algo not in migrated_ids or algo in error_ids
            ]

            for algo in not_migrated:
                failed_rows.append((profile_id, algo))

            row["not_migrated_algorithm_ids"] = json.dumps(not_migrated)
            row["status"] = "✅" if not not_migrated else "❌"

            print(f"✔ Migrated: {list(migrated_ids)} | ❌ Failed: {list(not_migrated)}")

            if counter%5==0:
                time.sleep(5)
        except Exception as e:
            print(f"❌ Exception for profile_id={profile_id}: {str(e)}")
            row["not_migrated_algorithm_ids"] = f"ERROR: {str(e)}"
            row["status"] = "❌"
            for algo in all_algos:
                failed_rows.append((profile_id, algo))

        writer.writerow(row)

# === Write Failed Routing IDs to separate CSV ===
failed_rows.sort()
with open(FAILURE_CSV, 'w', newline='') as failfile:
    fail_writer = csv.writer(failfile)
    fail_writer.writerow(["serial_id", "profile_id", "algorithm_id"])
    for idx, (profile_id, algo_id) in enumerate(failed_rows, start=1):
        fail_writer.writerow([idx, profile_id, algo_id])

print(f"\n✅ Migration complete.")
print(f"🔹 Full result → {OUTPUT_CSV}")
print(f"🔻 Failures only → {FAILURE_CSV}")
