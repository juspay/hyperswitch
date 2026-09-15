# Failover/recovery drill on a remote SSH machine

A variant of `REMOTE_RUN.md`'s ramp runbook for a different question: **not**
"what's the max RPS", but "if I trigger a failover mid-run, how long does
the application take to recover?" Read `REMOTE_RUN.md` first for
provisioning, getting this branch onto the box, and the SSH port-forward —
this doc only covers what's different.

## How this differs from the ramp runbook

| | `run-remote-loadtest.sh` (ramp) | `run-failover-loadtest.sh` (this) |
| --- | --- | --- |
| Load shape | Staircase, 30→1200 rps | Flat, steady rate for a long fixed duration |
| Stops on failure? | Yes — `abortOnFail` thresholds | **No** — no thresholds at all, keeps running through the failure window on purpose |
| Question answered | Max sustainable RPS | Recovery time from a triggered failover |
| Config | `loadtest.env` / `config.template.json` | `failover.env` / `config.failover.template.json` |
| Output | Per-phase summary table | `recovery_report.txt` — first/last 5xx timestamps + the gap between them |

## 1–2. Provision + get the branch

Same as `REMOTE_RUN.md` steps 1–2.

## 3. Configure credentials

Edit `failover.env` (not `loadtest.env`) and fill in the same two
placeholders — `API_KEY` / `PUBLISHABLE_KEY`. Everything else already has
working defaults: `FLAT_RPS=100` held for `DURATION_SECONDS=3600` (1 hour —
generously longer than any failover drill should take; stop it early once
you've captured what you need, see step 6).

`run-failover-loadtest.sh` refuses to start against leftover placeholders,
same as the ramp script.

## 4. Run it — one command

```bash
./run-failover-loadtest.sh
```

Same shape as the ramp script (installs `k6`/`envsubst` if missing, renders
the config, validates it with `k6 inspect`, starts in the background with
`nohup`, returns immediately) plus one addition: it also installs `python3`
if missing, and starts an independent watcher process alongside k6 that
waits for the run to end and then writes `recovery_report.txt` — whether the
run finishes naturally at `DURATION_SECONDS` or you kill it early.

Example output:

```
k6 started (pid 12345)
dashboard is live on this machine at: http://127.0.0.1:5665
  -> port-forward it to view from your own machine, see REMOTE_RUN.md 'View the dashboard'
live progress:      tail -f output/run_20260916_040000/run.log
live failure count: wc -l output/run_20260916_040000/failures.log
live recovery check (safe to run any time, before or after the run ends):
                     python3 analyze_recovery.py output/run_20260916_040000/failures.log
auto-generated once the run ends (naturally or via kill): output/run_20260916_040000/recovery_report.txt
stop early:          kill $(cat output/run_20260916_040000/k6.pid)
```

## 5. Trigger the failover

Once traffic is flowing steadily (check the dashboard — port-forward per
`REMOTE_RUN.md` step 5), trigger whatever failover you're testing. The load
keeps running unmodified through it: no threshold will abort the test, no
matter how high the failure rate gets.

## 6. Read the recovery report

Once you've seen the application recover (dashboard error rate back to
~0%, or you've just waited long enough), stop the run:

```bash
kill $(cat output/run_<timestamp>/k6.pid)
```

The watcher process notices within ~5s and writes
`output/run_<timestamp>/recovery_report.txt`. Or check it live at any point
without stopping anything:

```bash
python3 analyze_recovery.py output/run_<timestamp>/failures.log
```

Example output:

```
total failure records: 426

=== 5xx failures (what you asked for — recovery window) ===
5xx: 312 records
  first: 2026-09-16T04:12:03.114Z  status=500  reason=payment_create_500  request_id=...
  last:  2026-09-16T04:14:47.902Z  status=500  reason=payment_confirm_500_failed  request_id=...
  window (first -> last): 164.8s

=== transport-level failures (connection reset/refused/timeout — no HTTP response at all) ===
(a real failover sometimes shows up here instead of/as well as 5xx, e.g. LB draining a target)
transport errors: 114 records
  first: 2026-09-16T04:12:01.558Z  status=0  reason=payment_create_invalid  request_id=None
  last:  2026-09-16T04:12:09.220Z  status=0  reason=customer_create_invalid  request_id=None
  window (first -> last): 7.7s
```

**Reading it:**

- The **5xx window** (first → last 5xx timestamp) is the primary number —
  how long the application was actively returning error responses. Treat it
  as the recovery time.
- The **transport-error window** is a secondary signal: some failover
  mechanisms (an LB draining a target, a connection-level drop) surface as
  connection resets/timeouts rather than a clean 5xx body, especially in the
  first moment or two of the failover before things settle into steady
  5xx-until-recovered. If transport errors start before and/or extend past
  the 5xx window, the *real* outage window is closer to the union of both —
  i.e. from the earliest of either "first" timestamp to the latest of either
  "last" timestamp.
- `request_id` on each entry is the same `X-Request-Id` the router returns —
  use it to cross-reference server-side logs for the exact moment things
  broke/recovered.
- If neither section reports anything, no failures were observed at all in
  that window — either the failover didn't affect this traffic path, or it
  happened outside the time you were running the drill.

## 7. Re-run

Edit `failover.env` (rate, duration, traffic mix, credentials) and run
`./run-failover-loadtest.sh` again — each run gets its own timestamped
`output/run_<timestamp>/` directory.

## 8. Clean up

Same as `REMOTE_RUN.md` step 8.
