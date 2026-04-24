#!/usr/bin/env python3
"""
sync_prod_usage.py — Query ClickHouse and mark which features are used in production.

Adds/updates prod_usage, prod_last_seen_at, prod_checked_at columns in features.db.

Status values:
  used      - seen in production within the lookback window
  not_used  - not seen in production within the lookback window
  unknown   - cannot be determined from ClickHouse (e.g. transformer-level features)

Usage:
  export CH_URL=http://localhost:8123
  export CH_USER=default
  export CH_PASSWORD=''
  export CH_DATABASE=default
  python3 scripts/sync_prod_usage.py [--days 90]
"""

import sqlite3
import os
import sys
import json
import argparse
import urllib.request
import urllib.parse
from datetime import datetime, timezone

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DB_PATH = os.path.join(REPO_ROOT, "features.db")

CH_URL = os.environ.get("CH_URL", "http://localhost:8123")
CH_USER = os.environ.get("CH_USER", "default")
CH_PASSWORD = os.environ.get("CH_PASSWORD", "")
CH_DATABASE = os.environ.get("CH_DATABASE", "default")


def ch_query(sql):
    """Execute a ClickHouse query via HTTP interface, return list of row dicts."""
    params = urllib.parse.urlencode({
        "query": sql,
        "user": CH_USER,
        "password": CH_PASSWORD,
        "database": CH_DATABASE,
        "default_format": "JSONEachRow",
    })
    url = f"{CH_URL}/?{params}"
    try:
        with urllib.request.urlopen(url, timeout=60) as resp:
            lines = resp.read().decode().strip().splitlines()
            return [json.loads(line) for line in lines if line.strip()]
    except urllib.error.HTTPError as e:
        body = e.read().decode()[:300]
        print(f"  [CH HTTP ERROR {e.code}] {body}", file=sys.stderr)
        return []
    except Exception as e:
        print(f"  [CH ERROR] {e}", file=sys.stderr)
        return []


def add_prod_columns(conn):
    """Add prod tracking columns to issues table if they don't exist yet."""
    for col, defn in [
        ("prod_usage",        "TEXT DEFAULT 'unknown'"),
        ("prod_last_seen_at", "TEXT"),
        ("prod_checked_at",   "TEXT"),
    ]:
        try:
            conn.execute(f"ALTER TABLE issues ADD COLUMN {col} {defn}")
        except sqlite3.OperationalError:
            pass  # column already exists
    conn.commit()


# ---------------------------------------------------------------------------
# Bucket 1 — connector_events.flow mapping
# Flow values are Rust type names from router_flow_types/ (last :: segment)
# ---------------------------------------------------------------------------

# Features that can be detected by looking at connector_events.flow
FEATURE_TO_FLOWS = {
    "Incremental Authorization":   ["IncrementalAuthorization"],
    "Extended Authorization":      ["ExtendAuthorization"],
    "Preprocessing Flow":          ["PreProcessing"],
    "Post-Authentication Flow":    ["CompleteAuthorize"],
    "Order Create Flow":           ["CreateOrder"],
    "Settlement Split Call":       ["SettlementSplitCreate"],
    "QR Code Generation Flow":     ["GenerateQr"],
    "Push Notification Flow":      ["PushNotification"],
    "Balance Check Flow":          ["GiftCardBalanceCheck"],
    "Dispute Accept":              ["Accept"],
    "Dispute Defend":              ["Defend", "Evidence"],
    "Refund":                      ["Execute"],
    "Connector Customer Creation": ["CreateConnectorCustomer"],
    "Authorize Session Token":     ["AuthorizeSessionToken"],
    "Revenue Recovery":            ["Execute"],
    # 3DS authentication flows land in the authentications table, not connector_events
    "Pre-Authentication Flow":     ["PreAuthentication"],
    "Authentication Flow":         ["Authentication"],
}

# These are embedded inside the Authorize payload — not detectable from connector_events
UNMAPPABLE_B1 = {
    "Partial Authorization", "Split Payments", "Split Refunds",
    "Billing Descriptor", "L2/L3 Data Processing", "Surcharge",
    "Installments", "Network Transaction ID", "Step Up Authentication",
    "Overcapture", "Handle Response Without Body", "Auth Token For Token Creation",
    "Connector Request Reference ID", "Payment Recurrence Operation",
    "Connector Intent Metadata", "Connector Testing Data", "API Webhook Config",
    "Skip Tokenization Before Mandate", "Connector Customer ID Generation",
    "Partner Merchant Identifier", "Payment (Decrypt Flow)",
}


def sync_bucket1(conn, days):
    print("  Syncing Bucket 1 via connector_events...", file=sys.stderr)

    rows = ch_query(f"""
        SELECT
            lower(connector_name) AS connector,
            flow,
            toString(max(created_at)) AS last_seen
        FROM connector_events
        WHERE created_at > now() - INTERVAL {days} DAY
        GROUP BY connector_name, flow
    """)

    # (connector_lower, flow) → last_seen timestamp string
    seen = {(r["connector"], r["flow"]): r["last_seen"] for r in rows}

    now = datetime.now(timezone.utc).isoformat()
    issues = conn.execute(
        "SELECT id, connector, feature FROM issues WHERE bucket = 1"
    ).fetchall()

    updates = []
    for issue_id, connector, feature in issues:
        if feature in UNMAPPABLE_B1:
            updates.append(("unknown", None, now, issue_id))
            continue

        flows = FEATURE_TO_FLOWS.get(feature)
        if not flows:
            updates.append(("unknown", None, now, issue_id))
            continue

        last_seen = None
        for flow in flows:
            val = seen.get((connector.lower(), flow))
            if val:
                last_seen = val
                break

        usage = "used" if last_seen else "not_used"
        updates.append((usage, last_seen, now, issue_id))

    conn.executemany(
        "UPDATE issues SET prod_usage=?, prod_last_seen_at=?, prod_checked_at=? WHERE id=?",
        updates,
    )
    conn.commit()

    used    = sum(1 for u, *_ in updates if u == "used")
    unknown = sum(1 for u, *_ in updates if u == "unknown")
    print(f"  Bucket 1: {used} used / {len(updates)-used-unknown} not_used / {unknown} unknown", file=sys.stderr)


# ---------------------------------------------------------------------------
# Bucket 2 — payment_attempts grouped by (connector, pm, pmt)
# ---------------------------------------------------------------------------

def sync_bucket2(conn, days):
    print("  Syncing Bucket 2 via payment_attempts...", file=sys.stderr)

    # All (connector, pm, pmt) combinations that had at least one payment
    payment_rows = ch_query(f"""
        SELECT
            lower(connector)              AS connector,
            payment_method                AS pm,
            payment_method_type           AS pmt,
            toString(max(created_at))     AS last_seen
        FROM payment_attempts
        WHERE sign_flag = 1
          AND created_at > now() - INTERVAL {days} DAY
        GROUP BY connector, payment_method, payment_method_type
    """)
    payments_seen = {
        (r["connector"], r["pm"], r["pmt"]): r["last_seen"]
        for r in payment_rows
    }

    # (connector, pm, pmt) combos that had a mandate payment
    mandate_rows = ch_query(f"""
        SELECT
            lower(connector)              AS connector,
            payment_method                AS pm,
            payment_method_type           AS pmt,
            toString(max(created_at))     AS last_seen
        FROM payment_attempts
        WHERE sign_flag = 1
          AND mandate_id IS NOT NULL
          AND mandate_id != ''
          AND created_at > now() - INTERVAL {days} DAY
        GROUP BY connector, payment_method, payment_method_type
    """)
    mandates_seen = {
        (r["connector"], r["pm"], r["pmt"]): r["last_seen"]
        for r in mandate_rows
    }

    now = datetime.now(timezone.utc).isoformat()
    issues = conn.execute(
        "SELECT id, connector, pm, pmt, feature FROM issues WHERE bucket = 2"
    ).fetchall()

    updates = []
    for issue_id, connector, pm, pmt, feature in issues:
        key = (connector.lower(), pm, pmt)
        if feature == "Mandate":
            last_seen = mandates_seen.get(key)
        else:
            last_seen = payments_seen.get(key)

        usage = "used" if last_seen else "not_used"
        updates.append((usage, last_seen, now, issue_id))

    conn.executemany(
        "UPDATE issues SET prod_usage=?, prod_last_seen_at=?, prod_checked_at=? WHERE id=?",
        updates,
    )
    conn.commit()

    used = sum(1 for u, *_ in updates if u == "used")
    print(f"  Bucket 2: {used} used / {len(updates)-used} not_used", file=sys.stderr)


# ---------------------------------------------------------------------------
# Bucket 3 — per-feature queries across multiple tables
# Note: api_flow values match Hyperswitch internal handler names.
#       Verify against your production api_events if values drift.
# ---------------------------------------------------------------------------

B3_QUERIES = {
    "Routing Algorithm": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM routing_events
        WHERE created_at > now() - INTERVAL {days} DAY
    """,
    "Dynamic Routing": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM routing_events
        WHERE routing_engine = 'dynamic'
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "External 3DS Authentication": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM authentications
        WHERE sign_flag = 1
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Dispute Management": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM dispute
        WHERE sign_flag = 1
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "FRM (Fraud Risk Management)": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM fraud_check
        WHERE sign_flag = 1
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Payout Type": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM payout
        WHERE sign_flag = 1
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Payout Auto Fulfill": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM payout
        WHERE sign_flag = 1 AND auto_fulfill = 1
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Payout Recurring": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM payout
        WHERE sign_flag = 1 AND recurring = 1
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Save Card Flow": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM payment_intents
        WHERE sign_flag = 1
          AND setup_future_usage IS NOT NULL
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Off Session Payments": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM payment_intents
        WHERE sign_flag = 1 AND off_session = 1
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Mandate Management": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM payment_attempts
        WHERE sign_flag = 1
          AND mandate_id IS NOT NULL AND mandate_id != ''
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Multiple Capture": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM payment_attempts
        WHERE sign_flag = 1
          AND multiple_capture_count > 1
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Surcharge": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM payment_attempts
        WHERE sign_flag = 1
          AND surcharge_amount IS NOT NULL AND surcharge_amount > 0
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Webhook Details": """
        SELECT count() AS cnt, toString(max(created_at)) AS last_seen
        FROM outgoing_webhook_events
        WHERE created_at > now() - INTERVAL {days} DAY
    """,
    "Void/Cancel Payment": """
        SELECT count() AS cnt
        FROM api_events
        WHERE api_flow = 'PaymentsCancel'
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Payment Sync": """
        SELECT count() AS cnt
        FROM api_events
        WHERE api_flow = 'PaymentsRetrieve'
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Customer Management": """
        SELECT count() AS cnt
        FROM api_events
        WHERE api_flow LIKE 'Customer%'
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "SDK Client Token Generation": """
        SELECT count() AS cnt
        FROM api_events
        WHERE api_flow = 'PaymentsSessionToken'
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Payment Method Operations": """
        SELECT count() AS cnt
        FROM api_events
        WHERE api_flow LIKE 'PaymentMethod%'
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Eligibility Check": """
        SELECT count() AS cnt
        FROM api_events
        WHERE api_flow = 'PaymentsEligibility'
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Health Check": """
        SELECT count() AS cnt
        FROM api_events
        WHERE api_flow = 'Health'
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Auto Retries": """
        SELECT count() AS cnt
        FROM api_events
        WHERE api_flow = 'PaymentsStart'
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Manual Retry": """
        SELECT count() AS cnt
        FROM api_events
        WHERE api_flow = 'PaymentsRetry'
          AND created_at > now() - INTERVAL {days} DAY
    """,
    "Relay Operations": """
        SELECT count() AS cnt
        FROM api_events
        WHERE api_flow LIKE 'Relay%'
          AND created_at > now() - INTERVAL {days} DAY
    """,
}


def sync_bucket3(conn, days):
    print("  Syncing Bucket 3 (per-feature queries)...", file=sys.stderr)
    now = datetime.now(timezone.utc).isoformat()

    issues = conn.execute(
        "SELECT id, feature FROM issues WHERE bucket = 3"
    ).fetchall()

    updates = []
    for issue_id, feature in issues:
        if feature not in B3_QUERIES:
            updates.append(("unknown", None, now, issue_id))
            continue

        sql = B3_QUERIES[feature].format(days=days)
        rows = ch_query(sql)

        if not rows:
            updates.append(("not_used", None, now, issue_id))
            continue

        cnt = int(rows[0].get("cnt", 0))
        last_seen = rows[0].get("last_seen") if cnt > 0 else None
        usage = "used" if cnt > 0 else "not_used"
        updates.append((usage, last_seen, now, issue_id))

    conn.executemany(
        "UPDATE issues SET prod_usage=?, prod_last_seen_at=?, prod_checked_at=? WHERE id=?",
        updates,
    )
    conn.commit()

    used    = sum(1 for u, *_ in updates if u == "used")
    unknown = sum(1 for u, *_ in updates if u == "unknown")
    print(f"  Bucket 3: {used} used / {len(updates)-used-unknown} not_used / {unknown} unknown", file=sys.stderr)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Sync production usage from ClickHouse into features.db"
    )
    parser.add_argument(
        "--days", type=int, default=90,
        help="Lookback window in days (default: 90)",
    )
    args = parser.parse_args()

    if not os.path.exists(DB_PATH):
        print(f"ERROR: {DB_PATH} not found. Run extract_features.py first.", file=sys.stderr)
        sys.exit(1)

    print(f"ClickHouse: {CH_URL} / database={CH_DATABASE}", file=sys.stderr)
    print(f"Lookback:   {args.days} days", file=sys.stderr)
    print(f"DB:         {DB_PATH}", file=sys.stderr)
    print("", file=sys.stderr)

    conn = sqlite3.connect(DB_PATH)
    add_prod_columns(conn)

    sync_bucket1(conn, args.days)
    sync_bucket2(conn, args.days)
    sync_bucket3(conn, args.days)

    conn.close()
    print(f"\nDone. Query example:", file=sys.stderr)
    print(f"  sqlite3 features.db \"SELECT bucket,prod_usage,COUNT(*) FROM issues GROUP BY bucket,prod_usage ORDER BY bucket,prod_usage;\"", file=sys.stderr)


if __name__ == "__main__":
    main()
