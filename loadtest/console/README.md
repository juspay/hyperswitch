# Load-test console

SvelteKit SSR and authenticated APIs manage a single k6 generator. The browser
talks only to the console; the generator calls Router and PM directly.

```bash
cd loadtest/console
npm ci
cp .env.example .env
# Generate a password hash without putting the password in shell history:
read -s "console_password?Admin password: "
printf '%s' "$console_password" | npm run --silent password
unset console_password
# Paste the hash into .env, then configure service credentials.
npm run dev
```

Open http://127.0.0.1:4174. For a local production build:
`npm run build && node --env-file=.env start.mjs`.
The example uses local PostgreSQL on `127.0.0.1:5432`. Configure a database user
with SELECT access to `payment_intent` and its password. Saved Settings override
the initial environment values, so update the saved connection when switching
between local services and a container deployment.

The first start imports the existing ignored `loadtest.config.json` into `.data`;
subsequent settings edits affect the console copy. Set `LOADTEST_CONFIG_FILE` to
import a different configuration and `LOADTEST_DATA_DIR` for durable storage.
Existing artifact directories remain readable. Settings never return stored
passwords, API keys or Grafana tokens. Leaving a secret input blank keeps it.

Use the screens in this order:

1. **Scenarios** — create or edit a named scenario with Single/Tenancy, merchant count,
   weighted flows, and constant TPS or visual Ramp/Hold stages. Saving creates a
   new immutable version; existing runs keep their original version.
2. **Fixtures** — select a scenario and create/recover its infrastructure. Existing
   ready CLI fixtures are imported automatically. Only compatible ready sets can
   be selected for execution; service or provisioning changes require new fixtures.
3. **Performance** — select a saved scenario and ready fixture set, then Start.
   “Open Performance” on Scenarios only selects the scenario; it does not launch k6.
4. **Results** — inspect recordings and Compare with PostgreSQL. Open Settings →
   Database, enter an existing database account, and Test connection first.

Flow weights must total 100%. TPS means offered payment flows/sec, not API
requests/sec; a flow may make several requests. The ramp preview uses active test
time, while live monitoring includes wall time and pauses.

The background worker starts automatically and owns execution. Browser refresh,
closing a tab, logging out, hot reload, and SSR process restart do not cancel a
run. Reopening Performance attaches to the active run. Settings edits do not
change an already captured run or fixture configuration.

```bash
npm run worker -- status
# Explicitly stop the worker and drain any active job before changing worker code:
npm run worker -- stop
npm run worker -- start
```

Worker code is loaded when the worker starts; frontend hot reload does not reload
an executing worker. Container shutdown stops both services, allows recording to
drain for 60 seconds, and recovers interrupted jobs as incomplete after a crash.
The container entrypoint supervises both services; `npm start` starts SSR alone,
so stopping that process intentionally leaves the worker running.

Performance uses five load states:

| State | Available load controls |
|---|---|
| Idle | Start |
| Running | Pause, Stop |
| Pausing | None; active flows are draining |
| Paused | Resume, Stop |
| Completed | None; choose New run to start another |

Pause closes admission to new flows. Already admitted flows finish before Paused.
The configured load clock freezes during pause and drain; resume does not catch
up missed arrivals. Waiting VUs remain allocated. The console's Go admission
controller is used with k6's externally-controlled executor, because k6 1.0.0
cannot pause a running arrival-rate executor. CLI runs retain their native
arrival-rate executors. Rebuild the custom binary after pulling this change:

```bash
../k6/scenario-mix/recorder/build.sh
```

Cards and the live chart show successful TPS, confirmation p99, HTTP errors and
pending SQLite writes. Live p99 uses a bounded one-second window (8,192 observations);
final p99 uses the complete k6 summary. Chart samples use wall time so pauses and
faults remain visible. The latest 30 minutes are available for replay and flow
filtering; all samples remain in each run's telemetry.jsonl. Activity is persisted
in console.sqlite and exported beside the run. k6 output is under Diagnostics.
Results evaluates the criteria saved in that run's configuration, keeping
performance acceptance separate from infrastructure recovery verification.

Faults default to **Simulation**, which changes no infrastructure. Configure a
server-held JSON file through LOADTEST_FAULT_TARGETS to enable real targets:

```json
[
  {"id":"router-test","label":"Test Router","adapter":"docker","container":"test-router"},
  {"id":"pm-test","label":"Test PM","adapter":"kubernetes","context":"test-cluster","namespace":"loadtest","deployment":"pm"},
  {"id":"cache-test","label":"Test cache","adapter":"gcp-redis","project":"test-project","region":"us-central1","instance":"test-cache"},
  {"id":"db-test","label":"Test database","adapter":"gcp-alloydb","project":"test-project","region":"us-central1","cluster":"test-cluster","instance":"test-primary"}
]
```

Docker/Kubernetes support outage and restart; GCP adapters support HA failover.
The server needs the corresponding CLI and a scoped infrastructure identity.
The supplied container does not include docker, kubectl or gcloud; install the
needed adapter CLI in your deployment image. An explicit target/action phrase
confirms each operation. Only one fault can run, and a fault requires a running
load. Pause is disabled until recovery completes. Outage duration is bounded to
1–300 seconds; run termination restores an active outage early.

Before mutation, the server flushes a recovery ledger to disk. Shutdown and
restart retry pending recovery; unresolved recovery blocks new loads. Docker
recovery checks container identity; Kubernetes restores the saved replica count
and verifies the rollout. HA failover verifies a healthy primary and does not
switch back to the original primary. Verification is readiness-based, not an
application end-to-end recovery guarantee. Real fault adapters have command-level
tests; only simulation is exercised automatically.

Results offers paginated response history and PostgreSQL reconciliation without
CSV input. Comparison uses the merchant/payment pair, excludes baseline payments,
and requires complete recording unless explicitly overridden. It compares the
latest client status to a single read-only, repeatable-read database snapshot; later asynchronous state
changes can therefore appear as mismatches. Full comparisons live in
`reconciliation.sqlite`; `reconciliation.json` retains counts and the first 200
discrepancies. Failed creates with no returned payment ID remain in response
history and cannot be reconciled by ID.

Grafana panel URLs must use the configured server and `/grafana/d-solo/...`.
The iframe uses the authenticated console origin and the selected run's time
window. See [EC2 deployment](DEPLOYMENT.md) for Grafana and ALB configuration.

```bash
npm run check
npm run build
npm test
```

To exercise read-only reconciliation against an existing PostgreSQL database with
at least two payments, supply `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, and
`PGPASSWORD`, then run `CONSOLE_TEST_PG=1 node --test test/postgres.test.mjs`.
See [validation results](VALIDATION.md) for measurements and remaining EC2 checks.

### Local outages and restarts

In the scenario editor’s **Fault targets** section, add a Card vault or PostgreSQL / Redis target, use **Check target** (read-only), then **Save faults**. During a running load, select it in **Create fault**. Outages last 1–300 seconds; restart immediately begins recovery. Recovery is verified, recorded on the run, and continues independently of the browser. Failed recovery blocks new loads and offers **Restore**.

- **Card vault:** repository `/Users/mohammed.kashif/codes/hyperswitch-card-vault`, service port `3001`. The worker verifies the listener's repository, binary and process start time before SIGTERM. Recovery runs `nix develop -c cargo r` from that repository, with logs in the data directory. Nix, lsof and ps must be available to the worker. Build the vault first so recovery fits the 120-second readiness deadline.
- **PostgreSQL / Redis:** configure the process-compose binary, controller port, exact process name and service port (`5432` / `6379`; vault Redis may use `6378`). Use `process-compose process list` against your controller to find names. For an outage, another process must remain running to keep older process-compose controllers alive; the console checks this before stopping anything. The console stops only that named process, then starts it again; it does not kill the controller or delete database files.
- **Current local setup:** process-compose is running with `--no-server`, which disables remote control. Restart your service launcher with its HTTP server enabled, on a dedicated loopback controller port such as `8090` (router already owns `8080`). The standalone equivalent is `process-compose -p 8090 -f /path/to/process-compose.yaml up`. This must use your existing service configuration. Do not start a second database on the same data directory. The installed binary currently lives at `/nix/store/pbgm7naha6x0x4m9g34h93x9xhln45kc-process-compose-1.64.1/bin/process-compose`; configure that path if it is absent from the worker PATH.

Local PostgreSQL outage/restart is **not an HA failover**. Existing server-configured HA adapters remain available through `LOADTEST_FAULT_TARGETS`. Local adapters operate on the worker host; an EC2 container cannot control host processes merely by sharing a port number. Use infrastructure adapters there. Local target settings persist in `local-fault-targets.json`; changes are rejected during an active fault or pending recovery. Target controls use fixed argv, with no free-form shell command field.

Fault timelines: edit a scenario, choose **Timeline · Hold, Ramp, Fault**, and add steps with **+ Fault**. Configure Card vault / PostgreSQL / Redis under the editor’s collapsed **Fault targets** section; each service has one target. Fault steps maintain the preceding TPS. Starting the saved scenario authorizes its scheduled faults. The worker uses active load time, snapshots target identities, records activation/recovery in run activity, and restores active faults on Stop. Local PostgreSQL outages/restarts are not HA failovers. Process-compose targets require its controller server enabled. Missed fault windows are recorded as errors and are never injected late.

To test recovery without k6, save the target and click **Test outage**. Choose 1–300 seconds (default 5), enter the displayed confirmation, then start. **Restore now** ends the outage early. The worker persists test status and activation/recovery events in `console.sqlite`; browser refresh does not stop recovery. Standalone tests require no active fixture/load job and block new generators until recovery finishes.

Results reconciliation shows cumulative matched (green), not found (red), and status mismatch (amber) counts against elapsed comparison time. Progress updates after each 1,000-payment batch; small runs can finish in one batch. The saved summary includes counts, percentages, comparison time, and up to 200 discrepancies with merchant/payment IDs. Timelines are retained in `reconciliation.json` and capped at 600 samples; earlier comparisons require a new comparison to obtain a graph.

The Performance graph spans the configured active duration before Start and throughout execution. Grey marks time without samples; dotted markers show planned ramps/holds, fault activation, and outage recovery start. Colored markers show actual events, with verified recovery in green. Hover or use Inspect time for target TPS, measured values, and stage/event details. Pause time does not shift scheduled stages. Restart recovery has no predicted timestamp.

Local vault recovery uses `nix develop -c cargo r` when the repository has a flake; otherwise it launches the existing `target/debug/locker` or `target/release/locker` executable from that repository. Preflight rejects targets with neither startup option. Retry recovery is asynchronous; the server verifies the listener/process identity and records launcher failures without holding the HTTP request open.

Reconciliation also checks recorded transport failures (HTTP status 0). Known merchant/payment pairs are looked up in PostgreSQL in 1,000-row batches and saved in `transport_comparisons` inside `reconciliation.sqlite`. Found means present, not necessarily succeeded. Failures without a usable payment ID remain unresolved; a timeout does not prove the server rejected the request. These attempt counts are separate from payment-status totals, and matching remains an exact status comparison.

### Fault environments

In the scenario editor, expand **Fault targets**. Each service has one target, with **Local**, **Cloud**, or **Custom** configuration:

- Local: card vault launcher or process-compose PostgreSQL/Redis.
- Cloud: managed AlloyDB PostgreSQL failover, managed Redis HA failover, or Kubernetes card vault.
- Custom: a Docker container or Kubernetes deployment; arbitrary shell commands are not accepted.

Cloud PostgreSQL defaults to project `hyperswitch-dev`, region `asia-south1`, cluster `dev-hyperswitch-alloydb`, instance `dev-hyperswitch-alloydb-primary`. Edit these before saving if needed. **Check target** only describes the resource. **Test failover** requires typed confirmation and performs real standby promotion. In a scenario timeline, choose PostgreSQL → failover; stage duration controls when the next stage begins, not managed failover downtime.

The worker host/container must have `gcloud` installed and a server-side identity authorized to describe the instance and invoke failover. The stock image does not bundle the Google Cloud CLI; install it in a derived image or on the worker host. Credentials stay on the server. Docker/Kubernetes targets similarly require their CLIs and server-side access. Reload the worker after updating its environment, only when no job or recovery is active.

Cloud card vault defaults to Kubernetes namespace `hyperswitch`, deployment `hyperswitch-card-vault-hyperswitch-card-vault`. Outage scales to zero and restores the original replica count after downtime; restart uses `kubectl rollout restart deployment`. Leave Context empty to use the worker's current kubeconfig context, or specify a context explicitly. Existing non-empty namespace/deployment values are preserved.
