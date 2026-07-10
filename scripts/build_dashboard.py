#!/usr/bin/env python3
"""
build_dashboard.py — Generate a standalone HTML dashboard of feature and
cypress coverage trends from features.db.

Reads tag_snapshots (per-tag aggregates) and writes dashboard.html, a single
self-contained file with Chart.js charts. No server needed — just open it.

Usage:
  python3 scripts/build_dashboard.py
  open dashboard.html
"""

import os
import sys
import json
import argparse
import sqlite3
from datetime import date, timedelta
from string import Template


def tag_to_iso_week(tag):
    """Convert a tag like '2026.04.23.0' to an ISO year-week label like '2026-W17'."""
    parts = tag.split(".")
    try:
        d = date(int(parts[0]), int(parts[1]), int(parts[2]))
    except (ValueError, IndexError):
        return None
    year, week, _ = d.isocalendar()
    return f"{year}-W{week:02d}"

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DB_PATH = os.path.join(REPO_ROOT, "features.db")
OUT_HTML = os.path.join(REPO_ROOT, "dashboard.html")

# ---- Exclusions ----
EXCLUDED_CONNECTORS = {
    # Can't test in hosted environment
    "absa_sanlam",
    # Standard exclusions
    "blackhawknetwork", "boku", "breadpay", "celero", "chargebee", "digitalvirgo",
    "flexiti", "getnet", "gpayments", "hyperwallet",
    "imerchantsolutions",  # UCS only connector
    "tsys_transit",  # UCS only connector
    "fiservcommercehub",  # UCS only connector
    "revolv3",  # UCS only connector
    "juspaythreedesserver", "katapult", "mpgs", "payeezy", "payme",  # creds not available
    "paytm", "phonepe",
    "powertranz", "prophetpay", "santander", "sift", "silverflow", "square",
    "hyperpg", "tokenex", "trustpayments", "zen"
}

# Payment method types to exclude from Bucket 2
EXCLUDED_PM_TYPES_BUCKET2 = {"GooglePay", "ApplePay", "Paze", "SamsungPay"}  # Paze not fully implemented, SamsungPay can't automate

# Specific B2 (connector, pm, pmt, feature) combinations to exclude
EXCLUDED_B2_COMBINATIONS = {
    ("braintree", "Wallet", "Paypal", "Payment"),  # Can't automate in cypress
}

# Specific connector + flow combinations to exclude
EXCLUDED_FLOW_COMBINATIONS = {
    ("airwallex", "Order Create Flow"),   # Internal flow
    ("nordea",    "Order Create Flow"),   # Internal flow
    ("payme",     "Order Create Flow"),   # Internal flow
    ("payme",     "Refund"),              # Not possible to verify e2e cases
    ("razorpay",  "Order Create Flow"),   # Internal flow
    ("trustpay",  "Order Create Flow"),   # Internal flow
    ("amazonpay", "Refund"),              # Not possible to verify e2e cases
    ("bitpay",    "Refund"),              # Not possible to verify e2e cases
    ("coingate",  "Refund"),              # Not possible to verify e2e cases
    ("gigadat",   "Refund"),              # Not possible to verify e2e cases
    ("itaubank",  "Refund"),              # Not possible to verify e2e cases
    ("klarna",    "Refund"),              # Not possible to verify e2e cases
    ("loonio",    "Refund"),              # Not possible to verify e2e cases
    ("razorpay",  "Refund"),              # Not possible to verify e2e cases
    ("santander", "Refund"),              # Not possible to verify e2e cases
    ("stripe",    "Overcapture"),         # Creds not available
    ("truelayer", "Refund"),              # UCS only connector
    ("trustly",   "Refund"),              # UCS only connector
    ("adyen",     "Split Refunds"),       # Creds not available
    ("adyen",          "Dispute Accept"), # No connector config data
    ("adyen",          "Dispute Defend"), # No connector config data
    ("checkout",       "Dispute Accept"), # No connector config data
    ("checkout",       "Dispute Defend"), # No connector config data
    ("worldpayvantiv", "Dispute Accept"), # No connector config data
    ("adyen",          "Split Payments"),         # Creds not available
    ("xendit",         "Split Payments"),         # Creds not available
    ("xendit",         "Split Refunds"),          # Creds not available
    ("xendit",         "Settlement Split Call"),  # Creds not available
    ("worldpaymodular", "Refund"),                 # Not possible to verify e2e cases
    ("barclaycard", "Pre-Authentication Flow"),     # Internal flow
    ("cybersource", "Pre-Authentication Flow"),     # Internal flow
    ("nexixpay",    "Pre-Authentication Flow"),     # Internal flow
    ("nmi",         "Pre-Authentication Flow"),     # Internal flow
    ("nuvei",       "Pre-Authentication Flow"),     # Internal flow
    ("redsys",      "Pre-Authentication Flow"),     # Internal flow
    ("shift4",      "Pre-Authentication Flow"),     # Internal flow
    ("worldpayxml", "Pre-Authentication Flow"),     # Internal flow
}

# Features to exclude from Bucket 3
EXCLUDED_FEATURES_BUCKET3 = {
    "Split Transactions Enabled",    # v2 feature
    "Process Tracker Mapping",       # Not testable via Cypress
    "Payout Tracker Mapping",        # Not testable via Cypress
    "CVV Collection During Payment", # v2 feature
    "Dispute Polling Interval",      # Not possible in Cypress
    "FRM Routing Algorithm",         # Internal flow
    "Sub-Merchants",                 # Deprecated feature
}


def fetch_prod_coverage(until_tag=None):
    """
    Return prod-usage coverage breakdown (after exclusions) as a dict.
    When until_tag is given, coverage is determined from tag_snapshots at
    that specific tag (point-in-time snapshot) rather than issues.status
    (current state).
    """
    conn = sqlite3.connect(DB_PATH)

    # Coverage source: tag_snapshots at until_tag, or current issues.status
    if until_tag:
        snap = {
            (b, c, pm, pmt, f): (cov, blk)
            for b, c, pm, pmt, f, cov, blk in conn.execute(
                "SELECT bucket,connector,pm,pmt,feature,is_covered,is_blocked_by_bug FROM tag_snapshots WHERE tag=?",
                (until_tag,)
            ).fetchall()
        }

    rows = conn.execute("""
        SELECT bucket, connector, pm, pmt, feature, prod_used, cypress_status, blocked_by_bug
        FROM issues
    """).fetchall()
    conn.close()

    from collections import defaultdict
    buckets_by_prod = defaultdict(lambda: defaultdict(lambda: {'covered': 0, 'blocked': 0, 'total': 0}))

    for bucket, connector, pm, pmt, feature, prod_used, cypress_status, blocked_by_bug in rows:
        if connector and connector.lower() in EXCLUDED_CONNECTORS:
            continue
        if bucket == 2 and pmt in EXCLUDED_PM_TYPES_BUCKET2:
            continue
        if bucket == 2 and (connector and connector.lower(), pm, pmt, feature) in EXCLUDED_B2_COMBINATIONS:
            continue
        if bucket == 1 and (connector and connector.lower(), feature) in EXCLUDED_FLOW_COMBINATIONS:
            continue
        if bucket == 3 and feature in EXCLUDED_FEATURES_BUCKET3:
            continue

        prod = (prod_used or 'unknown').lower()
        if prod not in ('yes', 'no', 'unknown'):
            prod = 'unknown'

        if until_tag:
            key = (bucket, connector or '', pm or '', pmt or '', feature)
            covered, blocked = snap.get(key, (0, 0))
        else:
            covered = 1 if cypress_status == 'covered' else 0
            blocked = 1 if blocked_by_bug == 1 else 0

        buckets_by_prod[prod][f"b{bucket}"]['covered'] += covered
        buckets_by_prod[prod][f"b{bucket}"]['blocked'] += blocked
        buckets_by_prod[prod][f"b{bucket}"]['total'] += 1
        buckets_by_prod[prod]['all']['covered'] += covered
        buckets_by_prod[prod]['all']['blocked'] += blocked
        buckets_by_prod[prod]['all']['total'] += 1

    result = {}
    for prod in ('yes', 'no', 'unknown'):
        result[prod] = {}
        for k in ('b1', 'b2', 'b3', 'all'):
            d = buckets_by_prod[prod][k]
            c, blk, t = d['covered'], d['blocked'], d['total']
            auto = t - blk
            result[prod][k] = {
                'covered': c,
                'blocked': blk,
                'total': t,
                'automatable': auto,
                'not_covered': t - c - blk,
                'pct_achieved': round(100.0 * c / auto, 1) if auto else 0.0,
                'pct_potential': round(100.0 * (c + blk) / t, 1) if t else 0.0,
            }
    return result


def fetch_series(cutoff_tag=None, until_tag=None):
    """Return list of {tag, b1, b2, b3, b1_covered, b2_covered, b3_covered} dicts, oldest first.
    Filters out tags where extraction didn't work (total features == 0).
    Applies exclusions for specific connectors, flows, and features."""
    conn = sqlite3.connect(DB_PATH)
    
    # Build exclusion conditions for connectors
    connector_excludes = " OR ".join([f"LOWER(connector) = '{c}'" for c in EXCLUDED_CONNECTORS])
    pm_type_excludes = " OR ".join([f"pmt = '{pmt}'" for pmt in EXCLUDED_PM_TYPES_BUCKET2])
    b3_feature_excludes = " OR ".join([f"feature = '{f}'" for f in EXCLUDED_FEATURES_BUCKET3])
    
    # Exclude specific (connector, feature) flow combinations from B1
    flow_combo_excludes = " OR ".join(
        [f"(LOWER(connector) = '{c}' AND feature = '{f}')"
         for c, f in EXCLUDED_FLOW_COMBINATIONS]
    )

    # Exclude specific (connector, pm, pmt, feature) combinations from B2
    b2_combo_excludes = " OR ".join(
        [f"(LOWER(connector) = '{c}' AND pm = '{pm}' AND pmt = '{pmt}' AND feature = '{f}')"
         for c, pm, pmt, f in EXCLUDED_B2_COMBINATIONS]
    )

    sql = f"""
        SELECT
            tag,
            SUM(CASE WHEN bucket = 1 THEN 1 ELSE 0 END) AS b1,
            SUM(CASE WHEN bucket = 2 THEN 1 ELSE 0 END) AS b2,
            SUM(CASE WHEN bucket = 3 THEN 1 ELSE 0 END) AS b3,
            SUM(CASE WHEN bucket = 1 AND is_covered = 1 THEN 1 ELSE 0 END) AS b1_covered,
            SUM(CASE WHEN bucket = 2 AND is_covered = 1 THEN 1 ELSE 0 END) AS b2_covered,
            SUM(CASE WHEN bucket = 3 AND is_covered = 1 THEN 1 ELSE 0 END) AS b3_covered,
            SUM(CASE WHEN bucket = 1 AND is_blocked_by_bug = 1 THEN 1 ELSE 0 END) AS b1_blocked,
            SUM(CASE WHEN bucket = 2 AND is_blocked_by_bug = 1 THEN 1 ELSE 0 END) AS b2_blocked,
            SUM(CASE WHEN bucket = 3 AND is_blocked_by_bug = 1 THEN 1 ELSE 0 END) AS b3_blocked
        FROM tag_snapshots
        WHERE (bucket != 1 OR NOT ({connector_excludes}))
          AND (bucket != 1 OR NOT ({flow_combo_excludes}))
          AND (bucket != 2 OR (NOT ({connector_excludes}) AND NOT ({pm_type_excludes}) AND NOT ({b2_combo_excludes})))
          AND (bucket != 3 OR NOT ({b3_feature_excludes}))
          {{cutoff_where}}
        GROUP BY tag
        ORDER BY tag
    """
    cutoff_where = ""
    if cutoff_tag:
        cutoff_where += f"AND tag >= '{cutoff_tag}' "
    if until_tag:
        cutoff_where += f"AND tag <= '{until_tag}' "
    rows = conn.execute(sql.format(cutoff_where=cutoff_where)).fetchall()
    conn.close()

    series = []
    for tag, b1, b2, b3, b1c, b2c, b3c, b1blk, b2blk, b3blk in rows:
        total = (b1 or 0) + (b2 or 0) + (b3 or 0)
        if total == 0:
            continue
        series.append({
            "tag": tag,
            "b1": b1 or 0,
            "b2": b2 or 0,
            "b3": b3 or 0,
            "b1_covered": b1c or 0,
            "b2_covered": b2c or 0,
            "b3_covered": b3c or 0,
            "b1_blocked": b1blk or 0,
            "b2_blocked": b2blk or 0,
            "b3_blocked": b3blk or 0,
        })
    return series


def fetch_monthly_new(cutoff_tag=None, until_tag=None):
    """Aggregate new features and new cypress tests per month."""
    conn = sqlite3.connect(DB_PATH)
    conditions_feat, conditions_cyp, params_feat, params_cyp = [], [], [], []
    if cutoff_tag:
        conditions_feat.append("introduced_in_tag >= ?"); params_feat.append(cutoff_tag)
        conditions_cyp.append("covered_in_tag >= ?");    params_cyp.append(cutoff_tag)
    if until_tag:
        conditions_feat.append("introduced_in_tag <= ?"); params_feat.append(until_tag)
        conditions_cyp.append("covered_in_tag <= ?");    params_cyp.append(until_tag)
    w_feat = ("WHERE " + " AND ".join(conditions_feat)) if conditions_feat else ""
    w_cyp  = ("WHERE " + " AND ".join(conditions_cyp))  if conditions_cyp  else ""
    params = (cutoff_tag,) if cutoff_tag else ()
    features = conn.execute(f"""
        SELECT substr(introduced_in_tag, 1, 7) AS month,
               SUM(CASE WHEN bucket = 1 THEN 1 ELSE 0 END),
               SUM(CASE WHEN bucket = 2 THEN 1 ELSE 0 END),
               SUM(CASE WHEN bucket = 3 THEN 1 ELSE 0 END)
        FROM feature_introductions
        {w_feat}
        GROUP BY month ORDER BY month
    """, params_feat).fetchall()
    cypress = dict((r[0], r[1]) for r in conn.execute(f"""
        SELECT substr(covered_in_tag, 1, 7) AS month, COUNT(*)
        FROM cypress_test_introductions
        {w_cyp}
        GROUP BY month
    """, params_cyp).fetchall())
    conn.close()

    months = []
    for m, b1, b2, b3 in features:
        months.append({
            "month": m,
            "b1": b1 or 0,
            "b2": b2 or 0,
            "b3": b3 or 0,
            "cypress": cypress.get(m, 0),
        })
    return months


def fetch_weekly_new(cutoff_tag=None, until_tag=None):
    """Aggregate new features and new cypress tests per ISO week."""
    conn = sqlite3.connect(DB_PATH)
    conditions_feat, conditions_cyp, params_feat, params_cyp = [], [], [], []
    if cutoff_tag:
        conditions_feat.append("introduced_in_tag >= ?"); params_feat.append(cutoff_tag)
        conditions_cyp.append("covered_in_tag >= ?");    params_cyp.append(cutoff_tag)
    if until_tag:
        conditions_feat.append("introduced_in_tag <= ?"); params_feat.append(until_tag)
        conditions_cyp.append("covered_in_tag <= ?");    params_cyp.append(until_tag)
    w_feat = ("WHERE " + " AND ".join(conditions_feat)) if conditions_feat else ""
    w_cyp  = ("WHERE " + " AND ".join(conditions_cyp))  if conditions_cyp  else ""
    feat_rows = conn.execute(
        f"SELECT introduced_in_tag, bucket FROM feature_introductions {w_feat}", params_feat
    ).fetchall()
    cyp_rows = conn.execute(
        f"SELECT covered_in_tag FROM cypress_test_introductions {w_cyp}", params_cyp
    ).fetchall()
    conn.close()

    buckets = {}  # week -> {b1, b2, b3, cypress}
    def ensure(week):
        if week not in buckets:
            buckets[week] = {"week": week, "b1": 0, "b2": 0, "b3": 0, "cypress": 0}
        return buckets[week]

    for tag, bucket in feat_rows:
        w = tag_to_iso_week(tag)
        if w is None:
            continue
        row = ensure(w)
        row[f"b{bucket}"] += 1

    for (tag,) in cyp_rows:
        w = tag_to_iso_week(tag)
        if w is None:
            continue
        ensure(w)["cypress"] += 1

    return [buckets[w] for w in sorted(buckets)]


HTML_TEMPLATE = Template("""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Hyperswitch Feature &amp; Cypress Coverage</title>
<script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.js"></script>
<style>
  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    margin: 0; padding: 24px;
    background: #0f1115; color: #e6e6e6;
  }
  h1 { margin: 0 0 8px; font-size: 22px; }
  .sub { color: #888; margin-bottom: 24px; font-size: 13px; }
  .legend-key {
    background: #1a1d24; border: 1px solid #2a2e38; border-radius: 8px;
    padding: 12px 16px; margin-bottom: 20px; font-size: 12px; color: #aaa;
    display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 8px 24px;
  }
  .legend-key .row { display: flex; align-items: flex-start; gap: 8px; }
  .legend-key .swatch {
    width: 10px; height: 10px; border-radius: 2px; margin-top: 4px; flex-shrink: 0;
  }
  .legend-key strong { color: #e6e6e6; font-weight: 500; }
  .cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
           gap: 12px; margin-bottom: 24px; }
  .card {
    background: #1a1d24; border-radius: 8px; padding: 16px;
    border: 1px solid #2a2e38;
  }
  .card .label { font-size: 11px; color: #888; text-transform: uppercase;
                 letter-spacing: 0.5px; }
  .card .value { font-size: 26px; font-weight: 600; margin-top: 4px; }
  .card .delta { font-size: 11px; color: #8ac; margin-top: 4px; line-height: 1.5; }
  .chart-box {
    background: #1a1d24; border-radius: 8px; padding: 16px 20px;
    margin-bottom: 20px; border: 1px solid #2a2e38;
  }
  .chart-box h2 { margin: 0 0 6px; font-size: 15px; font-weight: 500; }
  .chart-box .hint { font-size: 12px; color: #888; margin-bottom: 12px; line-height: 1.5; }
  canvas { max-height: 360px; }
  footer { color: #666; font-size: 11px; margin-top: 32px; text-align: center; }
  .prod-table-wrap { overflow-x: auto; }
  .prod-table {
    width: 100%; border-collapse: collapse; font-size: 13px;
  }
  .prod-table th {
    background: #12151b; color: #888; font-weight: 500;
    padding: 8px 12px; text-align: center; border-bottom: 1px solid #2a2e38;
    font-size: 11px; text-transform: uppercase; letter-spacing: 0.4px;
  }
  .prod-table th.left { text-align: left; }
  .prod-table td {
    padding: 7px 12px; border-bottom: 1px solid #1e2229; text-align: center;
  }
  .prod-table td.left { text-align: left; color: #ccc; }
  .prod-table tr:last-child td { border-bottom: none; }
  .prod-table .section-header td {
    background: #12151b; color: #aaa; font-weight: 600;
    padding: 6px 12px; font-size: 12px;
  }
  .prod-table .pct-cell { font-weight: 600; }
  .prod-table .pct-high  { color: #6bda8f; }
  .prod-table .pct-mid   { color: #ffa56c; }
  .prod-table .pct-low   { color: #ff7070; }
  .prod-table .covered   { color: #6bda8f; }
  .prod-table .blocked   { color: #ffa56c; }
  .prod-table .notcov    { color: #ff7070; }
  .prod-table .achieved  { color: #6bda8f; font-weight: 700; }
  .prod-table .potential { color: #ffa56c; font-weight: 700; }
</style>
</head>
<body>

<h1>Hyperswitch Feature &amp; Cypress Coverage</h1>
<div class="sub">From <code>features.db</code> · $first_tag → $last_tag · $num_tags tags scanned</div>

<div class="legend-key">
  <div class="row"><span class="swatch" style="background:#5cc8ff"></span>
    <div><strong>Connector Flows</strong> — per-connector flow capabilities (Refund, Incremental Auth, 3DS, Dispute Defend, etc.). Detected from connector Rust code.</div>
  </div>
  <div class="row"><span class="swatch" style="background:#9d7cff"></span>
    <div><strong>Connector × Payment Method</strong> — supported (connector, payment method, payment method type) combinations. Detected from each connector's <code>SUPPORTED_PAYMENT_METHODS</code> static.</div>
  </div>
  <div class="row"><span class="swatch" style="background:#ffa56c"></span>
    <div><strong>Core / Schema</strong> — platform-wide features. Detected as new fields in <code>business_profile</code> / <code>merchant_account</code> Diesel structs.</div>
  </div>
  <div class="row"><span class="swatch" style="background:#6bda8f"></span>
    <div><strong>Cypress Tests</strong> — features with cypress coverage across all three categories (Connector Flows, Connector × PM, and Core/Schema).</div>
  </div>
</div>

<div class="cards">
  <div class="card">
    <div class="label">Total Features (latest tag)</div>
    <div class="value">$total_features</div>
    <div class="delta">
      Connector Flows: $b1_latest<br>
      Connector × PM: $b2_latest<br>
      Core / Schema: $b3_latest
    </div>
  </div>
  <div class="card">
    <div class="label">Coverage (Achieved)</div>
    <div class="value">$coverage_pct%</div>
    <div class="delta">$total_covered / $automatable_count automatable<br><span style="color:#ffa56c">Potential: $potential_pct% (with $total_blocked bug-blocked)</span></div>
  </div>
  <div class="card">
    <div class="label">Features Introduced (period)</div>
    <div class="value">$total_new_features</div>
    <div class="delta">
      Connector Flows: $total_new_b1<br>
      Connector × PM: $total_new_b2<br>
      Core / Schema: $total_new_b3
    </div>
  </div>
  <div class="card">
    <div class="label">Cypress Tests Introduced (period)</div>
    <div class="value">$total_new_cypress</div>
    <div class="delta">$cypress_ratio% ratio to new features — higher is better</div>
  </div>
  <div class="card">
    <div class="label">Latest Release (<strong>$latest_tag</strong>)</div>
    <div class="value">+$latest_new_cypress new cypress</div>
    <div class="delta">
      $latest_new_features new feature(s) added in this tag.<br>
      Coverage: $prev_pct% → $coverage_pct% ($delta_sign$delta_pct pp).<br>
      <span class="text-zinc-500">Last 7 tags:</span> +$week_new_cypress cypress · +$week_new_features features
    </div>
  </div>
</div>

<div class="chart-box">
  <h2>New Features vs. New Cypress Tests — Per Week</h2>
  <div class="hint">
    Stacked bars: new features landing each ISO week (by category).
    Green line: new cypress tests landing the same week.
    When the green line sits <strong>above</strong> the bars, cypress is keeping pace.
    <strong>Below</strong> the bars means tests are lagging feature additions.
  </div>
  <canvas id="weekly_chart"></canvas>
</div>

<div class="chart-box">
  <h2>New Features vs. New Cypress Tests — Per Month</h2>
  <div class="hint">Monthly roll-up of the same comparison — useful for smoothing out week-to-week noise.</div>
  <canvas id="monthly_chart"></canvas>
</div>

<div class="chart-box">
  <h2>Feature Introduction Over Time</h2>
  <div class="hint">Cumulative count of features known to the parser at each daily tag, stacked by category. Lines trending up = features getting added; flat = quiet period.</div>
  <canvas id="features_chart"></canvas>
</div>

<div class="chart-box">
  <h2>Cypress Test Introduction Over Time</h2>
  <div class="hint">Cumulative count of features with cypress coverage at each daily tag, across all three categories.</div>
  <canvas id="cypress_chart"></canvas>
</div>

<div class="chart-box">
  <h2>Cypress Coverage Ratio Over Time</h2>
  <div class="hint">% of all features that have cypress tests. Moving up = tests catching up to features. Moving down = features outpacing tests.</div>
  <canvas id="ratio_chart"></canvas>
</div>

<div class="chart-box">
  <h2>Prod Usage vs Cypress Coverage</h2>
  <div class="hint">
    Coverage of features grouped by prod usage status, after applying the exclusion list.
    <strong>prod = yes</strong> — feature observed in production traffic.
    <strong>prod = no</strong> — not yet seen in production.
    <strong>prod = unknown</strong> — prod data not collected.<br>
    <strong>Achieved %</strong> = Covered ÷ Automatable (excludes bug-blocked). 
    <strong>Potential %</strong> = (Covered + Bug Blocked) ÷ Total.
  </div>
  <div class="prod-table-wrap">
  <table class="prod-table">
    <thead>
      <tr>
        <th class="left">Prod Used</th>
        <th class="left"></th>
        <th>B1 — Connector Flows</th>
        <th>B2 — Connector × PM</th>
        <th>B3 — Core / Schema</th>
        <th>All Buckets (B1+B2+B3)</th>
      </tr>
    </thead>
    <tbody>
      $prod_table_rows
    </tbody>
  </table>
  </div>
</div>

<footer>Generated by scripts/build_dashboard.py · data lives in features.db</footer>

<script>
const series = $series_json;
const monthly = $monthly_json;
const weekly = $weekly_json;

const labels = series.map(s => s.tag);

Chart.defaults.color = '#aaa';
Chart.defaults.borderColor = '#2a2e38';
Chart.defaults.font.family = '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';

function line(ctx, datasets, opts) {
  opts = opts || {};
  return new Chart(ctx, {
    type: 'line',
    data: { labels: labels, datasets: datasets },
    options: {
      responsive: true,
      interaction: { mode: 'index', intersect: false },
      scales: {
        x: { ticks: { maxTicksLimit: 12, autoSkip: true } },
        y: Object.assign({ beginAtZero: true }, opts.y || {})
      },
      plugins: {
        legend: { labels: { boxWidth: 12 } },
        tooltip: { backgroundColor: '#1a1d24', borderColor: '#444', borderWidth: 1 }
      },
      elements: { point: { radius: 0, hoverRadius: 4 } }
    }
  });
}

line(document.getElementById('features_chart'), [
  { label: 'Connector Flows',         data: series.map(s => s.b1),
    borderColor: '#5cc8ff', backgroundColor: 'rgba(92,200,255,0.15)', fill: true, tension: 0.2 },
  { label: 'Connector × Payment Method', data: series.map(s => s.b2),
    borderColor: '#9d7cff', backgroundColor: 'rgba(157,124,255,0.15)', fill: true, tension: 0.2 },
  { label: 'Core / Schema',           data: series.map(s => s.b3),
    borderColor: '#ffa56c', backgroundColor: 'rgba(255,165,108,0.15)', fill: true, tension: 0.2 }
], { y: { stacked: true } });

line(document.getElementById('cypress_chart'), [
  { label: 'Connector Flows — with cypress',          data: series.map(s => s.b1_covered),
    borderColor: '#5cc8ff', backgroundColor: 'rgba(92,200,255,0.15)', fill: true, tension: 0.2 },
  { label: 'Connector × Payment Method — with cypress', data: series.map(s => s.b2_covered),
    borderColor: '#9d7cff', backgroundColor: 'rgba(157,124,255,0.15)', fill: true, tension: 0.2 },
  { label: 'Core / Schema — with cypress',            data: series.map(s => s.b3_covered),
    borderColor: '#ffa56c', backgroundColor: 'rgba(255,165,108,0.15)', fill: true, tension: 0.2 }
], { y: { stacked: true } });

const ratio = series.map(s => {
  const total = s.b1 + s.b2 + s.b3;
  return total === 0 ? 0 : Math.round(((s.b1_covered + s.b2_covered + s.b3_covered) / total) * 1000) / 10;
});
line(document.getElementById('ratio_chart'), [
  { label: 'Cypress coverage %', data: ratio,
    borderColor: '#6bda8f', backgroundColor: 'rgba(107,218,143,0.15)', fill: true, tension: 0.2 }
], { y: { min: 0, max: 100, ticks: { callback: v => v + '%' } } });

function periodChart(ctx, data, labelKey) {
  return new Chart(ctx, {
    type: 'bar',
    data: {
      labels: data.map(d => d[labelKey]),
      datasets: [
        { label: 'New Connector Flows',            data: data.map(d => d.b1),
          backgroundColor: '#5cc8ff', stack: 'features' },
        { label: 'New Connector × Payment Method', data: data.map(d => d.b2),
          backgroundColor: '#9d7cff', stack: 'features' },
        { label: 'New Core / Schema',              data: data.map(d => d.b3),
          backgroundColor: '#ffa56c', stack: 'features' },
        { label: 'New Cypress tests',              data: data.map(d => d.cypress),
          backgroundColor: '#6bda8f', stack: 'cypress', type: 'line',
          borderColor: '#6bda8f', tension: 0.2, fill: false, pointRadius: 3, borderWidth: 2 }
      ]
    },
    options: {
      responsive: true,
      interaction: { mode: 'index', intersect: false },
      scales: {
        x: { stacked: true, ticks: { maxTicksLimit: 20, autoSkip: true } },
        y: { stacked: true, beginAtZero: true, title: { display: true, text: 'Count' } }
      },
      plugins: {
        legend: { labels: { boxWidth: 12 } },
        tooltip: { backgroundColor: '#1a1d24', borderColor: '#444', borderWidth: 1 }
      }
    }
  });
}

periodChart(document.getElementById('weekly_chart'), weekly, 'week');
periodChart(document.getElementById('monthly_chart'), monthly, 'month');
</script>
</body>
</html>
""")


def main():
    parser = argparse.ArgumentParser(
        description="Build dashboard.html from features.db"
    )
    parser.add_argument(
        "--months", type=int, default=None,
        help="Limit view to the last N months (default: all data)",
    )
    parser.add_argument(
        "--until-tag", default=None, metavar="TAG",
        help="Generate snapshot dashboard up to (and including) this tag, e.g. 2026.05.05.0",
    )
    parser.add_argument(
        "--out", default=None, metavar="FILE",
        help="Output HTML filename (default: dashboard.html, or dashboard_<tag>.html with --until-tag)",
    )
    args = parser.parse_args()

    if not os.path.exists(DB_PATH):
        print(f"ERROR: {DB_PATH} not found. Run track_feature_history.py first.", file=sys.stderr)
        sys.exit(1)

    cutoff_tag = None
    if args.months:
        cutoff_date = date.today() - timedelta(days=args.months * 30)
        cutoff_tag = cutoff_date.strftime("%Y.%m.%d.0")

    until_tag = args.until_tag or None

    # Determine output file
    global OUT_HTML
    if args.out:
        OUT_HTML = os.path.join(REPO_ROOT, args.out)
    elif until_tag:
        OUT_HTML = os.path.join(REPO_ROOT, f"dashboard_{until_tag}.html")

    series  = fetch_series(cutoff_tag, until_tag)
    monthly = fetch_monthly_new(cutoff_tag, until_tag)
    weekly  = fetch_weekly_new(cutoff_tag, until_tag)
    prod_cov = fetch_prod_coverage(until_tag=until_tag)

    if not series:
        print("ERROR: No data in tag_snapshots for the selected window.", file=sys.stderr)
        sys.exit(1)

    latest = series[-1]
    earliest = series[0]
    total_features_latest = latest["b1"] + latest["b2"] + latest["b3"]
    total_covered = latest["b1_covered"] + latest["b2_covered"] + latest["b3_covered"]
    total_blocked = latest["b1_blocked"] + latest["b2_blocked"] + latest["b3_blocked"]
    coverage_pct = round(100.0 * total_covered / total_features_latest, 1) if total_features_latest else 0.0
    potential_pct = round(100.0 * (total_covered + total_blocked) / total_features_latest, 1) if total_features_latest else 0.0

    # "Introduced in period" totals — filtered to the cutoff window
    conn = sqlite3.connect(DB_PATH)
    cond_f, cond_c, pf, pc = [], [], [], []
    if cutoff_tag:
        cond_f.append("introduced_in_tag >= ?"); pf.append(cutoff_tag)
        cond_c.append("covered_in_tag >= ?");    pc.append(cutoff_tag)
    if until_tag:
        cond_f.append("introduced_in_tag <= ?"); pf.append(until_tag)
        cond_c.append("covered_in_tag <= ?");    pc.append(until_tag)
    w_feat  = ("WHERE " + " AND ".join(cond_f)) if cond_f else ""
    w_cyp   = ("WHERE " + " AND ".join(cond_c)) if cond_c else ""
    params  = tuple(pf)
    params_c = tuple(pc)
    new_totals = conn.execute(f"""
        SELECT
            SUM(CASE WHEN bucket = 1 THEN 1 ELSE 0 END),
            SUM(CASE WHEN bucket = 2 THEN 1 ELSE 0 END),
            SUM(CASE WHEN bucket = 3 THEN 1 ELSE 0 END)
        FROM feature_introductions
        {w_feat}
    """, params).fetchone()
    total_new_cypress = conn.execute(
        f"SELECT COUNT(*) FROM cypress_test_introductions {w_cyp}", params_c
    ).fetchone()[0]

    # Latest-tag delta + last-7-tags rollup. We include the rollup so PRs
    # whose cypress entries are split across consecutive tags (e.g. one PR
    # adds 3 entries on day N, another adds 1 entry on day N+1) don't
    # disappear into a single "+1" headline number.
    latest_tag = latest["tag"]
    recent_tags = [s["tag"] for s in series[-7:]]
    placeholders = ",".join("?" * len(recent_tags))

    latest_new_cypress = conn.execute(
        "SELECT COUNT(*) FROM cypress_test_introductions WHERE covered_in_tag = ?",
        (latest_tag,),
    ).fetchone()[0]
    latest_new_features = conn.execute(
        "SELECT COUNT(*) FROM feature_introductions WHERE introduced_in_tag = ?",
        (latest_tag,),
    ).fetchone()[0]
    # Restrict recent-tags window to within the until_tag boundary
    if until_tag:
        recent_tags = [t for t in recent_tags if t <= until_tag]
    placeholders = ",".join("?" * len(recent_tags))
    week_new_cypress = conn.execute(
        f"SELECT COUNT(*) FROM cypress_test_introductions WHERE covered_in_tag IN ({placeholders})",
        recent_tags,
    ).fetchone()[0]
    week_new_features = conn.execute(
        f"SELECT COUNT(*) FROM feature_introductions WHERE introduced_in_tag IN ({placeholders})",
        recent_tags,
    ).fetchone()[0]
    week_window = f"{recent_tags[0]} → {recent_tags[-1]}" if len(recent_tags) > 1 else recent_tags[0]
    conn.close()

    # Coverage % at latest vs previous tag (across all 3 buckets)
    if len(series) >= 2:
        prev = series[-2]
        prev_total = prev["b1"] + prev["b2"] + prev["b3"]
        prev_cov = prev["b1_covered"] + prev["b2_covered"] + prev["b3_covered"]
        prev_pct = round(100.0 * prev_cov / prev_total, 1) if prev_total else 0.0
    else:
        prev_pct = coverage_pct
    delta_pct = round(coverage_pct - prev_pct, 2)
    delta_sign = "+" if delta_pct >= 0 else ""

    total_new_b1 = new_totals[0] or 0
    total_new_b2 = new_totals[1] or 0
    total_new_b3 = new_totals[2] or 0
    total_new_features = total_new_b1 + total_new_b2 + total_new_b3
    new_b1_b2 = total_new_b1 + total_new_b2
    cypress_ratio = round(100.0 * total_new_cypress / new_b1_b2, 1) if new_b1_b2 else 0.0

    # Build prod usage table rows
    def pct_class(p):
        if p >= 80: return "pct-high"
        if p >= 50: return "pct-mid"
        return "pct-low"

    prod_labels = {
        'yes':     'prod = yes — observed in production',
        'no':      'prod = no — not yet seen in production',
        'unknown': 'prod = unknown — data not available',
    }
    prod_rows_html = []
    for prod in ('yes', 'no', 'unknown'):
        label = prod_labels[prod]
        d = prod_cov[prod]
        # Section header
        prod_rows_html.append(
            f'<tr class="section-header"><td colspan="6">{label}</td></tr>'
        )
        # Covered row
        cells = "".join(
            f'<td class="covered">{d[k]["covered"]}</td>'
            for k in ('b1','b2','b3','all')
        )
        prod_rows_html.append(f'<tr><td></td><td class="left">Covered</td>{cells}</tr>')
        # Bug Blocked row
        cells = "".join(
            f'<td class="blocked">{d[k]["blocked"]}</td>'
            for k in ('b1','b2','b3','all')
        )
        prod_rows_html.append(f'<tr><td></td><td class="left">Bug Blocked</td>{cells}</tr>')
        # Not covered row
        cells = "".join(
            f'<td class="notcov">{d[k]["not_covered"]}</td>'
            for k in ('b1','b2','b3','all')
        )
        prod_rows_html.append(f'<tr><td></td><td class="left">Not Covered</td>{cells}</tr>')
        # Total row
        cells = "".join(f'<td>{d[k]["total"]}</td>' for k in ('b1','b2','b3','all'))
        prod_rows_html.append(f'<tr><td></td><td class="left">Total</td>{cells}</tr>')
        # Achieved % row
        cells = "".join(
            f'<td class="pct-cell achieved {pct_class(d[k]["pct_achieved"])}">{d[k]["pct_achieved"]}%</td>'
            for k in ('b1','b2','b3','all')
        )
        prod_rows_html.append(f'<tr><td></td><td class="left">Achieved %</td>{cells}</tr>')
        # Potential % row
        cells = "".join(
            f'<td class="pct-cell potential {pct_class(d[k]["pct_potential"])}">{d[k]["pct_potential"]}%</td>'
            for k in ('b1','b2','b3','all')
        )
        prod_rows_html.append(f'<tr><td></td><td class="left">Potential %</td>{cells}</tr>')

    prod_table_rows = "\n      ".join(prod_rows_html)

    html = HTML_TEMPLATE.substitute(
        first_tag=earliest["tag"],
        last_tag=latest["tag"],
        num_tags=len(series),
        total_features=latest["b1"] + latest["b2"] + latest["b3"],
        b1_latest=latest["b1"],
        b2_latest=latest["b2"],
        b3_latest=latest["b3"],
        total_covered=total_covered,
        coverage_pct=coverage_pct,
        total_blocked=total_blocked,
        automatable_count=total_features_latest - total_blocked,
        potential_pct=potential_pct,
        total_new_features=total_new_features,
        total_new_b1=total_new_b1,
        total_new_b2=total_new_b2,
        total_new_b3=total_new_b3,
        total_new_cypress=total_new_cypress,
        cypress_ratio=cypress_ratio,
        latest_tag=latest_tag,
        latest_new_cypress=latest_new_cypress,
        latest_new_features=latest_new_features,
        week_new_cypress=week_new_cypress,
        week_new_features=week_new_features,
        prev_pct=prev_pct,
        delta_sign=delta_sign,
        delta_pct=delta_pct,
        series_json=json.dumps(series),
        monthly_json=json.dumps(monthly),
        weekly_json=json.dumps(weekly),
        prod_table_rows=prod_table_rows,
    )

    with open(OUT_HTML, "w", encoding="utf-8") as f:
        f.write(html)

    window_desc = f"last {args.months} months" if args.months else "all history"
    print(f"Wrote {OUT_HTML}  ({window_desc})")
    print(f"  Tags covered: {earliest['tag']} → {latest['tag']} ({len(series)})")
    print(f"  Open with:    open {OUT_HTML}")


if __name__ == "__main__":
    main()
