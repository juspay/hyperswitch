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


def fetch_series(cutoff_tag=None):
    """Return list of {tag, b1, b2, b3, b1_covered, b2_covered} dicts, oldest first.
    Filters out tags where extraction didn't work (total features == 0)."""
    conn = sqlite3.connect(DB_PATH)
    sql = """
        SELECT
            tag,
            SUM(CASE WHEN bucket = 1 THEN 1 ELSE 0 END) AS b1,
            SUM(CASE WHEN bucket = 2 THEN 1 ELSE 0 END) AS b2,
            SUM(CASE WHEN bucket = 3 THEN 1 ELSE 0 END) AS b3,
            SUM(CASE WHEN bucket = 1 AND is_covered = 1 THEN 1 ELSE 0 END) AS b1_covered,
            SUM(CASE WHEN bucket = 2 AND is_covered = 1 THEN 1 ELSE 0 END) AS b2_covered
        FROM tag_snapshots
        {where}
        GROUP BY tag
        ORDER BY tag
    """
    where = "WHERE tag >= ?" if cutoff_tag else ""
    params = (cutoff_tag,) if cutoff_tag else ()
    rows = conn.execute(sql.format(where=where), params).fetchall()
    conn.close()

    series = []
    for tag, b1, b2, b3, b1c, b2c in rows:
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
        })
    return series


def fetch_monthly_new(cutoff_tag=None):
    """Aggregate new features and new cypress tests per month."""
    conn = sqlite3.connect(DB_PATH)
    w_feat = "WHERE introduced_in_tag >= ?" if cutoff_tag else ""
    w_cyp = "WHERE covered_in_tag >= ?" if cutoff_tag else ""
    params = (cutoff_tag,) if cutoff_tag else ()
    features = conn.execute(f"""
        SELECT substr(introduced_in_tag, 1, 7) AS month,
               SUM(CASE WHEN bucket = 1 THEN 1 ELSE 0 END),
               SUM(CASE WHEN bucket = 2 THEN 1 ELSE 0 END),
               SUM(CASE WHEN bucket = 3 THEN 1 ELSE 0 END)
        FROM feature_introductions
        {w_feat}
        GROUP BY month ORDER BY month
    """, params).fetchall()
    cypress = dict((r[0], r[1]) for r in conn.execute(f"""
        SELECT substr(covered_in_tag, 1, 7) AS month, COUNT(*)
        FROM cypress_test_introductions
        {w_cyp}
        GROUP BY month
    """, params).fetchall())
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


def fetch_weekly_new(cutoff_tag=None):
    """Aggregate new features and new cypress tests per ISO week."""
    conn = sqlite3.connect(DB_PATH)
    w_feat = "WHERE introduced_in_tag >= ?" if cutoff_tag else ""
    w_cyp = "WHERE covered_in_tag >= ?" if cutoff_tag else ""
    params = (cutoff_tag,) if cutoff_tag else ()
    feat_rows = conn.execute(
        f"SELECT introduced_in_tag, bucket FROM feature_introductions {w_feat}", params
    ).fetchall()
    cyp_rows = conn.execute(
        f"SELECT covered_in_tag FROM cypress_test_introductions {w_cyp}", params
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
    <div><strong>Cypress Tests</strong> — features with cypress coverage (Connector Flows + Connector × PM only; Core/Schema cypress coverage is not tracked).</div>
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
    <div class="label">Features With Cypress Tests</div>
    <div class="value">$total_covered</div>
    <div class="delta">$coverage_pct% of Connector Flows + Connector×PM features</div>
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
    <div class="label">Latest Tag Contribution</div>
    <div class="value">+$latest_new_cypress cypress</div>
    <div class="delta">
      Tag: <strong>$latest_tag</strong><br>
      Features added: $latest_new_features<br>
      Coverage: $prev_pct% → $coverage_pct% ($delta_sign$delta_pct pp)
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
  <div class="hint">Cumulative count of features that have cypress coverage at each daily tag. Only Connector Flows and Connector × PM are tracked (Core/Schema cypress coverage not detected dynamically).</div>
  <canvas id="cypress_chart"></canvas>
</div>

<div class="chart-box">
  <h2>Cypress Coverage Ratio Over Time</h2>
  <div class="hint">% of Connector Flow + Connector × PM features that have cypress tests. Moving up = tests catching up to features. Moving down = features outpacing tests.</div>
  <canvas id="ratio_chart"></canvas>
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
  { label: 'Connector Flows — with cypress',         data: series.map(s => s.b1_covered),
    borderColor: '#5cc8ff', backgroundColor: 'rgba(92,200,255,0.15)', fill: true, tension: 0.2 },
  { label: 'Connector × Payment Method — with cypress', data: series.map(s => s.b2_covered),
    borderColor: '#9d7cff', backgroundColor: 'rgba(157,124,255,0.15)', fill: true, tension: 0.2 }
], { y: { stacked: true } });

const ratio = series.map(s => {
  const total = s.b1 + s.b2;
  return total === 0 ? 0 : Math.round(((s.b1_covered + s.b2_covered) / total) * 1000) / 10;
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
    args = parser.parse_args()

    if not os.path.exists(DB_PATH):
        print(f"ERROR: {DB_PATH} not found. Run track_feature_history.py first.", file=sys.stderr)
        sys.exit(1)

    cutoff_tag = None
    if args.months:
        cutoff_date = date.today() - timedelta(days=args.months * 30)
        cutoff_tag = cutoff_date.strftime("%Y.%m.%d.0")

    series = fetch_series(cutoff_tag)
    monthly = fetch_monthly_new(cutoff_tag)
    weekly = fetch_weekly_new(cutoff_tag)

    if not series:
        print("ERROR: No data in tag_snapshots for the selected window.", file=sys.stderr)
        sys.exit(1)

    latest = series[-1]
    earliest = series[0]
    total_b1_b2 = latest["b1"] + latest["b2"]
    total_covered = latest["b1_covered"] + latest["b2_covered"]
    coverage_pct = round(100.0 * total_covered / total_b1_b2, 1) if total_b1_b2 else 0.0

    # "Introduced in period" totals — filtered to the cutoff window
    conn = sqlite3.connect(DB_PATH)
    w_feat = "WHERE introduced_in_tag >= ?" if cutoff_tag else ""
    w_cyp = "WHERE covered_in_tag >= ?" if cutoff_tag else ""
    params = (cutoff_tag,) if cutoff_tag else ()
    new_totals = conn.execute(f"""
        SELECT
            SUM(CASE WHEN bucket = 1 THEN 1 ELSE 0 END),
            SUM(CASE WHEN bucket = 2 THEN 1 ELSE 0 END),
            SUM(CASE WHEN bucket = 3 THEN 1 ELSE 0 END)
        FROM feature_introductions
        {w_feat}
    """, params).fetchone()
    total_new_cypress = conn.execute(
        f"SELECT COUNT(*) FROM cypress_test_introductions {w_cyp}", params
    ).fetchone()[0]

    # Latest-tag delta: what did the most recent tag add to cypress coverage?
    latest_tag = latest["tag"]
    latest_new_cypress = conn.execute(
        "SELECT COUNT(*) FROM cypress_test_introductions WHERE covered_in_tag = ?",
        (latest_tag,),
    ).fetchone()[0]
    latest_new_features = conn.execute(
        "SELECT COUNT(*) FROM feature_introductions WHERE introduced_in_tag = ?",
        (latest_tag,),
    ).fetchone()[0]
    conn.close()

    # Coverage % at latest vs previous tag (% of Connector Flows + Connector×PM)
    if len(series) >= 2:
        prev = series[-2]
        prev_total = prev["b1"] + prev["b2"]
        prev_cov = prev["b1_covered"] + prev["b2_covered"]
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
        total_new_features=total_new_features,
        total_new_b1=total_new_b1,
        total_new_b2=total_new_b2,
        total_new_b3=total_new_b3,
        total_new_cypress=total_new_cypress,
        cypress_ratio=cypress_ratio,
        latest_tag=latest_tag,
        latest_new_cypress=latest_new_cypress,
        latest_new_features=latest_new_features,
        prev_pct=prev_pct,
        delta_sign=delta_sign,
        delta_pct=delta_pct,
        series_json=json.dumps(series),
        monthly_json=json.dumps(monthly),
        weekly_json=json.dumps(weekly),
    )

    with open(OUT_HTML, "w", encoding="utf-8") as f:
        f.write(html)

    window_desc = f"last {args.months} months" if args.months else "all history"
    print(f"Wrote {OUT_HTML}  ({window_desc})")
    print(f"  Tags covered: {earliest['tag']} → {latest['tag']} ({len(series)})")
    print(f"  Open with:    open {OUT_HTML}")


if __name__ == "__main__":
    main()
