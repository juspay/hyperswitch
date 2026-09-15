#!/usr/bin/env python3
"""Recovery-time report for a failover drill (see run-failover-loadtest.sh).

Reads scenario-mix.js's --console-output failure log (one JSON failure
record per line, wrapped in k6's own logfmt `time=... level=error
msg="..."` line format), and reports the first and last 5xx failure
timestamps observed — the gap between them is a proxy for how long the
application was actually failing requests during a triggered failover.

Safe to run at any point during a still-running test (shows what's been
observed so far) or after it finishes.
"""
import json
import re
import sys
from datetime import datetime

MSG_RE = re.compile(r'msg="(.*)"\s*$')


def parse_line(line):
    m = MSG_RE.search(line.strip())
    if not m:
        return None
    try:
        # k6's logfmt value is a Go-quoted string; its escaping (\", \\, \n,
        # \t, ...) is a superset-compatible subset of JSON string escaping,
        # so wrapping it back in quotes and running it through json.loads
        # undoes it in one step, then a second json.loads parses the
        # failure record itself.
        unescaped = json.loads('"' + m.group(1) + '"')
        return json.loads(unescaped)
    except (ValueError, json.JSONDecodeError):
        return None


def parse_ts(t):
    return datetime.strptime(t, "%Y-%m-%dT%H:%M:%S.%fZ")


def main():
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} <failures.log>", file=sys.stderr)
        sys.exit(1)

    path = sys.argv[1]
    records = []
    try:
        with open(path) as f:
            for line in f:
                rec = parse_line(line)
                if rec is not None:
                    records.append(rec)
    except FileNotFoundError:
        print(f"error: {path} not found (has the run started yet?)", file=sys.stderr)
        sys.exit(1)

    print(f"total failure records: {len(records)}")

    def summarize(label, subset):
        if not subset:
            print(f"{label}: none observed")
            return
        subset = sorted(subset, key=lambda r: r["time"])
        first, last = subset[0], subset[-1]
        delta = (parse_ts(last["time"]) - parse_ts(first["time"])).total_seconds()
        print(f"{label}: {len(subset)} records")
        print(f"  first: {first['time']}  status={first.get('status')}"
              f"  reason={first.get('reason')}  request_id={first.get('headers', {}).get('X-Request-Id')}")
        print(f"  last:  {last['time']}  status={last.get('status')}"
              f"  reason={last.get('reason')}  request_id={last.get('headers', {}).get('X-Request-Id')}")
        print(f"  window (first -> last): {delta:.1f}s")

    fivexx = [r for r in records if isinstance(r.get("status"), int) and 500 <= r["status"] < 600]
    transport = [r for r in records if not r.get("status")]  # status 0/absent = connection-level failure, not an HTTP response

    print()
    print("=== 5xx failures (what you asked for — recovery window) ===")
    summarize("5xx", fivexx)

    print()
    print("=== transport-level failures (connection reset/refused/timeout — no HTTP response at all) ===")
    print("(a real failover sometimes shows up here instead of/as well as 5xx, e.g. LB draining a target)")
    summarize("transport errors", transport)


if __name__ == "__main__":
    main()
