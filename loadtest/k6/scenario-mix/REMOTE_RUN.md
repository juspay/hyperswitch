# Running the scenario-mix load test on a remote SSH machine

Self-contained runbook: everything the load test needs (target URL, ramp
shape, traffic mix) lives in `loadtest.env` in this directory. The two
merchant credential fields are placeholders — this repo
(`juspay/hyperswitch`) is public on GitHub, so real `api_key`/
`publishable_key` values are never committed. Fill those two in once on the
remote box (step 3 below); everything else needs no extra input.

## 0. Security note — read this first

`loadtest.env`'s `API_KEY`/`PUBLISHABLE_KEY` ship as `REPLACE_WITH_...`
placeholders on purpose — this repo is public, so real merchant credentials
must never be committed here. `run-remote-loadtest.sh` refuses to start if
either is still a placeholder. Once you fill in real values on the remote
box:

- Don't commit/push `loadtest.env` with real credentials back to this
  branch.
- Rotate the `api_key`/`publishable_key` once you're done testing, same as
  any credential that touched a shared box.
- Treat the remote box like you'd treat any secret holder — don't leave it
  open to others.

## 1. Provision the remote machine

k6 is CPU-bound (JS VU execution + TLS), not memory-bound. Guidance from
prior runs against this target: a 10-core laptop generating this scenario
mix started dropping iterations and hitting client-side connection errors
(`connection reset`, `broken pipe`, port exhaustion) around ~230 effective
rps (~650 HTTP req/s) — the client became the bottleneck before the server
did. For target RPS in the hundreds to `1200` (`loadtest.env`'s default),
use a compute-optimized box:

- `c6i.2xlarge` (8 vCPU / 16GB) as a baseline.
- `c6i.4xlarge` (16 vCPU) or larger for pushing toward `1200` rps.
- Default 20GB gp3 storage is plenty.
- Security group: inbound SSH (22) from your IP only — k6 only makes
  outbound calls to the target, no inbound needed.

## 2. Get this branch onto the remote machine

```bash
ssh -i <your-key.pem> <user>@<remote-host>
git clone <this-repo-url> hyperswitch   # or: git fetch && git checkout <this-branch>
cd hyperswitch
git checkout <this-branch>
cd loadtest/k6/scenario-mix
```

(`<user>` is `ec2-user` for Amazon Linux, `ubuntu` for Ubuntu AMIs.)

## 3. Configure credentials

Edit `loadtest.env` and replace the two placeholders with the merchant's
real credentials (everything else in the file already has working
defaults):

```bash
sed -i 's/REPLACE_WITH_MERCHANT_API_KEY/<real api_key>/; s/REPLACE_WITH_MERCHANT_PUBLISHABLE_KEY/<real publishable_key>/' loadtest.env
# or just edit the file directly
```

`run-remote-loadtest.sh` checks for and refuses to run against leftover
placeholders, so a stale credential fails fast with a clear message instead
of a confusing API error.

## 4. Run it — one command

```bash
./run-remote-loadtest.sh
```

This single command:

1. Installs `k6` and `envsubst` (via `dnf`/`apt-get`) if either is missing —
   no manual install step needed.
2. Reads `loadtest.env` and renders `config.template.json` into a
   timestamped `output/run_<timestamp>/config.json`.
3. Validates that config with `k6 inspect` before sending any traffic.
4. Starts the ramp **in the background** (`nohup`) and returns immediately —
   your SSH session isn't tied up for the ~15+ minutes a full ramp takes.
5. Prints the output directory, the k6 PID, and next-step commands.

Example output:

```
k6 started (pid 12345)
dashboard is live on this machine at: http://127.0.0.1:5665
  -> port-forward it to view from your own machine, see REMOTE_RUN.md 'View the dashboard'
live progress:   tail -f output/run_20260916_030000/run.log
failure count:   wc -l output/run_20260916_030000/failures.log
stop early:      kill $(cat output/run_20260916_030000/k6.pid)
```

## 5. View the dashboard from your own machine

The dashboard only listens on `127.0.0.1:5665` on the remote box (it has no
authentication, so it's never exposed to the internet). Reach it by
SSH-tunneling the port — run this **from your own machine**, in a separate
terminal from the one you used to start the test:

```bash
ssh -i <your-key.pem> -L 5665:localhost:5665 <user>@<remote-host>
```

Then open **http://localhost:5665** in your browser. It's live and
auto-updating for the whole run; closing the tunnel doesn't stop the test
(it keeps running on the remote box via `nohup`).

A static copy is also saved to `output/run_<timestamp>/timeseries.html` when
the run finishes — `scp` it back if you want an offline copy:

```bash
scp -i <your-key.pem> -r <user>@<remote-host>:~/hyperswitch/loadtest/k6/scenario-mix/output/run_<timestamp> ./
```

## 6. Verify failures / check whether it stopped on a threshold

- **Live failure count:** `wc -l output/run_<timestamp>/failures.log` — each
  line is one failed iteration as JSON (scenario, phase, HTTP status,
  response body, `X-Request-Id`). Grows as failures happen.
- **Failure reasons at a glance:**
  ```bash
  grep -oE '"reason\\":\\"[^"]*\\"' output/run_<timestamp>/failures.log | sort | uniq -c | sort -rn
  ```
- **Is it still running?** `kill -0 $(cat output/run_<timestamp>/k6.pid) && echo running || echo stopped`
- **Did a threshold abort it early?** Check the tail of
  `output/run_<timestamp>/run.log`:
  ```bash
  tail -50 output/run_<timestamp>/run.log
  ```
  A threshold-triggered stop prints:
  `level=error msg="thresholds on metrics '...' were crossed; at least one has abortOnFail enabled, stopping test prematurely"`
  — followed by the per-phase summary table showing `0 | 0` for every phase
  after the one that tripped it. `config.template.json` sets
  `scenario_failure_nomod_guest`/`scenario_failure_mod_cit_off` to abort if
  either scenario hits 100 total failures.
- **Did the client (not the server) become the bottleneck?** Look for
  `dropped_iterations` > 0 and a `WARNING: reports at the requested rate
  were NOT all achieved` line in the global summary row of `run.log` — that
  means the box couldn't generate the requested rate; move to a bigger
  instance rather than trusting the numbers at that phase.
- **Peak achieved throughput:** the `global` row's `peak_iteration_rate` in
  `run.log`, or per-phase `achieved tps` in the per-scenario tables.

## 7. Change the test and re-run

Edit `loadtest.env` (target RPS, ramp step/hold/idle, traffic weights,
credentials) and run `./run-remote-loadtest.sh` again — each run gets its
own timestamped `output/run_<timestamp>/` directory, so nothing overwrites a
previous run.

## 8. Clean up

```bash
kill $(cat output/run_<timestamp>/k6.pid)   # if still running
```

When you're done with the box entirely, terminate/stop the instance from
your cloud console.

## Reference

This wraps `scenario-mix.js` — see `README.md` in this directory for the
full scenario catalogue, config reference, and metrics produced, and
`EC2_RUNBOOK.md` for the manual (non-scripted) version of the same flow this
runbook automates.
