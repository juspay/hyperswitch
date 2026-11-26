# Load-test console

SvelteKit SSR and authenticated APIs expose the master control plane. It manages
local or distributed k6 workers, owns infrastructure faults and reconciliation,
and keeps jobs running independently of browser connections. Load workers call
Router and PM directly.

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

For PostgreSQL servers that require TLS, set the database values in `.env` and
start the application with the TLS launcher:

```bash
# PGSSLROOTCERT is optional when the database certificate uses a system-trusted CA.
export PGSSLROOTCERT=/absolute/path/to/database-ca.pem
npm run dev:tls
# Production build: npm run build && npm run start:tls
```

The launcher enforces certificate verification, validates the optional PEM path,
and reloads the detached worker so it inherits the TLS configuration. It refuses
to reload while a job, outage, or recovery is active. The same CA path can be
entered under Settings → Database; **Test connection** uses the identical TLS
configuration as reconciliation. `sslmode=disable` behavior remains available by
using the regular `npm run dev`/`npm start` commands with TLS unchecked.

Use the screens in this order:

1. **Scenarios** — create or edit a named scenario with Single/Tenancy, merchant count,
   weighted flows, and constant TPS or visual Ramp/Hold stages. Saving creates a
   new immutable version; existing runs keep their original version.
2. **Fixtures** — select a scenario and create/recover its infrastructure. Existing
   ready CLI fixtures are imported automatically. Only compatible ready sets can
   be selected for execution; merchant identity and provisioning-value changes
   require new fixtures.
3. **Performance** — select a saved scenario and ready fixture set. **Smoke test**
   runs a short, configurable verification without recording responses or adding
   a Results run. **Start load** launches the recorded scenario independently;
   it never runs smoke implicitly. “Open Performance” on Scenarios only selects
   the scenario; it does not launch k6.
4. **Results** — inspect recordings and Compare with PostgreSQL. Open Settings →
   Database, enter an existing database account, and Test connection first.

Flow weights must total 100%. TPS means offered payment flows/sec, not API
requests/sec; a flow may make several requests. The ramp preview uses active test
time, while live monitoring includes wall time and pauses. Settings → Load
generator controls preallocated and maximum VUs per TPS. Those limits are copied
into each run snapshot, so changing them does not alter a run already in progress.

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
arrival-rate executors. The console searches `K6_BINARY`, the source-tree build,
its container path and `PATH`. Build the custom binary after a fresh checkout:

```bash
npm run build:recorder
```

Cards and the live chart show successful TPS, confirmation p99, HTTP errors and
pending SQLite writes. Live p99 uses a bounded one-second window (8,192 observations);
final p99 uses the complete k6 summary. Chart samples use wall time so pauses and
faults remain visible. The latest 30 minutes are available for replay and flow
filtering; all samples remain in each run's telemetry.jsonl. Activity is persisted
in console.sqlite and exported beside the run. k6 output is under Diagnostics.
Results evaluates the criteria saved in that run's configuration, keeping
performance acceptance separate from infrastructure recovery verification.

### Environments and payment flows

Settings → Connections selects **Local**, **GCP**, **AWS**, or **Azure**. Service
connections, database/TLS credentials, and fault targets are saved separately per
environment. Switching back restores that environment’s saved connections. Saving
an environment or connection change is blocked while a job or fault recovery is active. Reconciliation of a provider-tagged run requires that provider to be selected. Existing
run snapshots remain unchanged; provider selection describes the system under test,
not the location of the load generator.

Local supports the vault launcher and process-compose PostgreSQL/Redis outages.
GCP supports Kubernetes vault outages and managed AlloyDB/Redis failovers. GCP
resource identities must be entered explicitly; the editor does not assume a
project or deployment. Fault target choices, interruption dialogs, activity, and
new Results summaries identify the provider. Changing providers does not translate
an outage into a failover: choose a supported action for the selected target.

AWS and Azure retain placeholder connections and resource fields; connection
checks, fixtures, load generation, and fault execution are unavailable. No fallback
adapter is invoked. Legacy `cloud` targets migrate to GCP. Unclassified custom
Kubernetes/Docker targets require provider assignment; ambiguous legacy connection
sets require an explicit environment selection. Only targets for the selected
provider participate in health and execution, including server-defined targets.

New scenarios use the payment-method service by default. The flow editor has no
integration selector or integration badges. Guest, CIT, pay-to-vault and SDK
checkout flows share this path. Metadata-update, MIT, and saved-card checkout are
not yet supported on this path and are unavailable for new traffic; an existing
scenario containing an enabled unsupported flow fails preflight explicitly.
Historical scenario versions and recorded responses are not rewritten. New fixture
signatures include provider and execution-model version, so prepare fixtures again
before running newly migrated scenarios.

## Scenario checks in Results

The run summary groups applicable original test cases under **Load & performance**
(TC-01, TC-02, TC-03, TC-09) and **Faults & recovery** (TC-04–TC-08, TC-10).
Each TC has a compact Expected / Observed / Verdict row; expand it to inspect the
measurements and limits. Steady holds are pooled into one check; repeated
failovers retain their individual evidence and any failed occurrence. Original
per-stage observations remain in the cached summary for inspection.
Normal holds map to steady-load performance. Calibration/burst require explicit
`load.stages[].intent` (`calibration` / `burst`) or `testCase` (`TC-01` / `TC-03`)
in the saved run configuration. A ramp alone is shown as observations, never
claimed as a burst hold. The saved fault-target type identifies the recovery TCs.
Application-reconnection, restart and merchant-coverage
claims stay Not measured until the required evidence exists. Card-vault outage
checks use measured recovery, throughput, latency, and reconciled data-loss
criteria; additional unattended/unaffected-flow evidence is not required. Explicitly selected
`load.testCases` without execution evidence stay Not run. The mapping is persisted
with the summary, without rewriting the saved scenario or raw recording.

The existing evaluator still measures each execution period. Normal stages compare
successful completions with the integrated offered load, including changing ramp
rates. Pauses and recovery gates use the sampled active/wall clock recorded in
`execution.jsonl`; older runs fall back to their uncompressed telemetry. Stage
boundaries are interpolated between samples, not inferred from the planned wall
duration. Latency p50/p95/p99 uses successful measured payment confirmations only.
Steady drift compares median latency in the last third with the first third of
the combined settled steady periods. The run verdict uses those same pooled
steady checks shown in the UI.

The latency table uses separate settled periods: baseline after startup settling,
verified outage after settling until restoration starts, and verified recovery
after settling until the next fault or run end. A complete flow must fit in a
settled period, so queued work crossing a transition is excluded. Adjacent clock
samples form continuous windows; pauses remain gaps. Percentiles use the actual
successful request samples, with no trimming or capping by latency value. Short
periods without eligible samples show unavailable values. All raw samples remain
recorded, and HTTP errors, completed-flow totals and recovery timing retain their
existing measurement windows.

**Settings → Results limits** defaults to 97% throughput, 0.1% normal-stage HTTP
errors, 160 ms hold/post-recovery p99, 500 ms ramp p99, 10% latency drift, 60 seconds
for recovery after restoration starts, 10 seconds of transition settling,
and zero acknowledged payments lost. Settings apply when the next run starts,
even for an existing scenario. The effective `resultsLimits` are saved with that
run; **Reset to defaults** only changes the Settings draft until it is saved.

Intentional fault windows record all failed requests, including vault errors
hidden from the Performance chart and timeouts returned after recovery. They do
not apply the normal-stage HTTP-error limit. Database availability can end the
observed outage before the provider finishes its operation. Recovery time starts
when restoration is requested, excluding the configured intentional downtime.
The data-loss check needs completed reconciliation; a missing comparison is not
a zero-loss result. Simulated recovery timings are recorded, not passed.

Completed results are cached atomically in `scenario-checks.json` with calculation
version and source fingerprint; reconciliation updates invalidate the cache.
Exact measurements are deferred until the recorder has drained to avoid scanning
a growing database during load. Missing evidence and unexecuted stages remain
explicit. Historical runs keep their saved throughput/error limits and record new
metrics without retroactively applying today's limits or declaring a new overall
pass. These Results checks do not change the existing k6 exit thresholds.

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
recovery checks container identity; Kubernetes restores the configured replica count
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
window. New installations include RPS, API latency and transaction-processing
latency panels from the bundled k6 dashboard. See [EC2 deployment](DEPLOYMENT.md)
for Grafana and ALB configuration.

Settings includes read-only health checks for Router and Payment Methods. The
Superposition check performs an authenticated read of one configured default and
supports either `x-superposition-access` or bearer-token authentication.
Superposition's refresh wait and progress-poll interval are configurable
independently. Changing run rate, duration, credentials, health
endpoints, database, Grafana or polling does not invalidate fixtures; changing
merchant mode/count, fixture Router, connector, organization/workspace/context,
or applied defaults does.

```bash
npm run check
npm run build
npm test
```

To exercise read-only reconciliation against an existing PostgreSQL database with
at least two payments, supply `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, and
`PGPASSWORD`, then run `CONSOLE_TEST_PG=1 node --test test/postgres.test.mjs`.
Run the native engine integration tests as described in [the engine README](../k6/scenario-mix/README.md).

### Local outages and restarts

In the scenario editor’s **Fault targets** section, add a Card vault or PostgreSQL / Redis target, use **Check target** (read-only), then **Save scenario** to save the scenario and shared targets together. During a running load, select it in **Create fault**. Outages last 1–300 seconds; restart immediately begins recovery. Recovery is verified, recorded on the run, and continues independently of the browser. Failed recovery blocks new loads and offers **Restore**.

- **Card vault:** configure the absolute path to your card-vault repository, service port `3001`. The worker verifies the listener's repository, binary and process start time before SIGTERM. Recovery runs `nix develop -c cargo r` from that repository, with logs in the data directory. Nix, lsof and ps must be available to the worker. Build the vault first so recovery fits the 120-second readiness deadline.
- **PostgreSQL / Redis:** configure the process-compose controller port, exact process name and service port (`5432` / `6379`; vault Redis may use `6378`). Use `process-compose -p 8090 process list` against your controller to find names. The console talks to the loopback controller API directly, so its process does not need the process-compose CLI in `PATH`. For an outage, another process must remain running to keep older controllers alive; the console checks this before stopping anything. It stops only that named process, then starts it again; it does not kill the controller or delete database files.
- **Current local setup:** start ext-services with its HTTP server enabled on a dedicated loopback controller port such as `8090` (router already owns `8080`): `nix run .#ext-services -- --no-server=false --port 8090`. Do not start a second database on the same data directory.

Local PostgreSQL outage/restart is **not an HA failover**. Existing server-configured HA adapters remain available through `LOADTEST_FAULT_TARGETS`. Local adapters operate on the worker host; an EC2 container cannot control host processes merely by sharing a port number. Use infrastructure adapters there. Shared target settings persist in `console.sqlite` (earlier `local-fault-targets.json` files are read for compatibility); changes are rejected during an active fault or pending recovery. Target controls use fixed argv, with no free-form shell command field.

GCP Kubernetes card-vault targets use a hard outage by default: capture deployment-owned pod identities and the replica count, scale to zero, then request zero-grace deletion of those captured pods. The hold timer starts only after no deployment-owned pods remain and each old pod UID is absent from its node’s kubelet runtime list. Preflight verifies `nodes/proxy` runtime-read access before interruption. Recovery restores the configured replica count (or the original count when unspecified) and waits for Ready and available replicas. The target’s Commands preview shows this sequence; it does not accept arbitrary shell commands. Existing explicit `outageMode: "graceful"` targets retain scale-only behavior. Runtime verification errors cause recovery rather than an assumed successful outage. No node pools or zones are stopped.

Fault timelines: edit a scenario, choose **Timeline · Hold, Ramp, Fault**, and add a Fault stage. Configure Card vault / PostgreSQL / Redis under the editor’s collapsed **Fault targets** section; each service has one target and one operation. A local service or cloud card vault is interrupted for the configured downtime and always restored. Cloud PostgreSQL and Redis use managed failover. Fault steps maintain the preceding TPS. At the planned boundary, the load keeps offering that TPS while its active stage clock waits for the infrastructure change to be verified. The configured duration starts only after activation is verified. The clock is held again when recovery starts and the next stage begins only after recovery is verified. Starting the saved scenario authorizes its scheduled faults. The control plane uses the minimum active time reported by all load workers, snapshots target identities, executes each fault once, records activation/recovery in run activity, and restores active faults on Stop. Load workers receive no fault target, cloud command or infrastructure credential. A local PostgreSQL interruption is not an HA failover. Process-compose targets require its controller server enabled. Missed fault windows are recorded as errors and are never injected late.

To test recovery without k6, save the target and click **Test interruption** (or **Test failover** for a managed database). Choose 1–300 seconds (default 5) for an interruption, enter the displayed confirmation, then start. **Restore now** ends the interruption early. The worker persists test status and activation/recovery events in `console.sqlite`; browser refresh does not stop recovery. Standalone tests require no active fixture/load job and block new generators until recovery finishes.

Results reconciliation shows exact payment bars: matched is green, missing/status mismatch is red, and not-yet-compared is grey. The overview groups by elapsed load time (first recorded observation), with a red marker for every bucket containing a discrepancy, including rare errors. Zoom in for one bar per payment, ordered by observation; hover shows its actual time and click opens statuses, request attempts and nearby faults. Count filters, **Next discrepancy**, a paginated payment list and streamed filtered CSV support large runs without sending all rows to the browser. Transport failures with no payment ID remain separately unresolved, never silently counted as matches. Earlier comparisons require **Compare again** to build the index.

The payment index and nine exact aggregation levels live in `reconciliation.sqlite`; batches update outcomes and aggregates in one transaction. Responses are capped at 800 bars (the UI normally requests at most 600). CSV uses bounded, backpressured pages. Run `npm run test:scale` for the synthetic ten-million-payment index/query benchmark. This measures the reconciliation view, not generator or database capacity.

The Performance graph spans the configured active duration before Start and throughout execution. Grey marks time without samples; dotted markers show planned ramps/holds, fault activation, and interruption recovery start. Colored markers show actual events, with verified recovery in green. Hover or use Inspect time for target TPS, measured values, and stage/event details. Pause time does not shift scheduled stages. Managed failover completion has no predicted timestamp.

Local vault recovery uses `nix develop -c cargo r` when the repository has a flake; otherwise it launches the existing `target/debug/locker` or `target/release/locker` executable from that repository. Preflight rejects targets with neither startup option. Retry recovery is asynchronous; the server verifies the listener/process identity and records launcher failures without holding the HTTP request open.

Reconciliation also checks recorded transport failures (HTTP status 0). Known merchant/payment pairs are looked up in PostgreSQL in 1,000-row batches and saved in `transport_comparisons` inside `reconciliation.sqlite`. Found means present, not necessarily succeeded. Failures without a usable payment ID remain unresolved; a timeout does not prove the server rejected the request. These attempt counts are separate from payment-status totals, and matching remains an exact status comparison.

### Fault environments

In the scenario editor, expand **Fault targets**. Each service has one target, with **Local**, **Cloud**, or **Custom** configuration:

- Local: card vault launcher or process-compose PostgreSQL/Redis.
- Cloud: managed AlloyDB PostgreSQL failover, managed Redis HA failover, or Kubernetes card vault.
- Custom: a Docker container or Kubernetes deployment; arbitrary shell commands are not accepted.

Cloud PostgreSQL defaults to project `hyperswitch-dev`, region `asia-south1`, cluster `dev-hyperswitch-alloydb`, instance `dev-hyperswitch-alloydb-primary`. Edit these before saving if needed. **Check target** only describes the resource. **Test failover** requires typed confirmation and performs real standby promotion. AlloyDB failover is submitted asynchronously and its operation ID is saved in the fault ledger. In a scenario timeline, choose PostgreSQL → failover; the configured duration begins after GCP accepts the request and does not control the actual outage length. The next stage waits for that exact operation to finish successfully, the original HA primary to report READY, and the Settings database connection to read `payment_intent`. Recovery polls every five seconds for up to ten minutes; a failed operation, missing operation ID, or verification deadline keeps recovery blocked instead of declaring success. Restart recovery resumes polling a saved operation without submitting another failover.

The control-plane host/container must have `gcloud` installed and a server-side identity authorized to describe the instance and invoke failover. Build `Dockerfile.cloud` on top of the stock console image to add `gcloud` and `kubectl`; the ordinary image remains available when infrastructure faults are disabled. Credentials stay on the control plane and are never mounted into load workers. Reload the control-plane worker service after updating its environment, only when no job or recovery is active.

Cloud card vault defaults to Kubernetes namespace `hyperswitch`, deployment `hyperswitch-card-vault-hyperswitch-card-vault`. Its interruption scales to zero, waits for the configured downtime, and scales back to 8 replicas by default. Recovery replicas can be changed on the target. Leave Context empty to use the worker's current kubeconfig context, or specify a context explicitly. Existing non-empty namespace/deployment values are preserved.

### Console interactions

- **Scenarios:** searchable library with Archive/Restore. A new scenario explicitly copies the selected one. One sticky **Save scenario** saves the scenario version and shared fault targets atomically; stale edits are rejected. Navigation offers Save, Discard or Continue editing. Target environments retain independent Local/Cloud/Custom presets.
- **Timeline:** drag a row or its handle across the full row width, use Alt+Up/Down or the move menu, and Undo moves. Stage times and inherited Hold TPS stay visible. Flow balancing is an explicit, undoable action.
- **Fixtures:** compatibility explains the changed provisioning fields. Ready sets can be selected for Performance; failed or interrupted sets can be repaired. Busy states link to the active job.
- **Performance:** one setup panel, a separate Smoke test dialog, explicit New run/Run again, worker allocation preview, disabled-action explanations, and live connection state. Selected scenario, fixture, run and graph filters survive section navigation.
- **Faults:** target checks report inline. Test confirmation, countdown, restore and outcome stay in one dialog; closing it does not stop recovery. Faults execute on the master.
- **Settings:** Connections, Database & TLS, Load generation, Workers and Grafana sections share save/discard protection. Imported certificates distinguish pending selection from the active saved CA. Worker onboarding includes live health; Grafana includes loading/retry and direct links.

After a code upgrade, reload the detached control-plane worker while **no job or recovery is active** (`npm run worker -- stop`, then `npm run worker -- start`) as well as the web server. Restarting only Vite preserves the previous worker process by design.

Server-side latency is payment-confirm p99 from the response `x-hs-latency` header (milliseconds). Router requests opt in with `x-hs-latency: true`, so this measures internal processing and excludes connector time. Missing or invalid headers remain unavailable; client-side latency is kept separately. New recordings persist the header value as `request_events.server_latency_ms`; historical runs without this measurement show no server latency.

### Target flow completion

Results evaluates run-wide **Flow completion** against a strict 100% target.
Successful flows finishing during drain count toward this check. The target is
at least the integrated load-plan budget and also includes additional offered
flows during recovery gates and recorded drops. Failed or missing flows and
scheduler drops cannot be hidden by shrinking the denominator. Incomplete or
cancelled recordings cannot pass this check.

Window throughput, TPS, and completion counts remain time-window measurements;
a flow crossing a stage boundary or finishing during drain can make these differ
slightly from the run total. They do not measure run-wide completion. Latency
continues to use the existing settled, successful-request sample windows.

### Connector load testing

Scenarios have an optional **Connectors** section. Dummy (`stripe_test`) remains the default. In real/sandbox mode, select supported card connectors and give each a whole-number share; the shares must total 100%. The mix applies independently to each payment-flow stream without increasing total offered TPS. Selection is deterministic and uses a smoothed 100-entry schedule; each complete cycle has the exact requested mix. Every distributed worker uses global iteration-based selection.

Create fixtures for that scenario, entering each connector's sandbox credentials and required account metadata. All selected accounts are provisioned for each merchant. Connector credentials are encrypted with AES-256-GCM under `.data/connector-secrets/`; job/fixture snapshots contain only opaque references. Keep that directory and its key private when moving the console. Connector credentials are never sent to load workers; workers receive merchant API credentials and connector account IDs as before. Partial provisioning can resume. Changing mix weights reuses fixtures; changing connector identities requires matching accounts.

Real-connector card confirmations include synthetic browser information required by providers such as Adyen. `payment.browser_info` overrides these defaults. Sandbox cards can differ between providers: Adyen uses the successful non-3DS card from the repository's connector tests, while other connectors use the configured card/pool. Set `payment.connector_cards[connector]` to override a provider's card explicitly, including for negative tests. These defaults do not change dummy-mode traffic.

Performance's **Connector throughput** metric shows confirmation responses identifying each connector. Results distinguish assigned flows from reported connector confirmations, successes/failures, and unreported confirmations. These are payment-level observations, not counts of provider HTTP attempts or fallback/retry calls. HS-only p99 uses `x-hs-latency`; full confirm p99 uses the complete HTTP request duration. Asynchronous/3DS completion is not automatically followed to settlement; existing payment success semantics still apply.

The generated catalog comes from `crates/euclid/src/enums.rs` (`RoutableConnectors`) and sandbox auth/card definitions in `crates/connector_configs/toml/sandbox.toml`. Run `npm run generate:connectors` after upstream changes and `npm run check:connectors` to check drift (Python 3.11+). Non-card connectors and unsupported credential shapes are excluded from selectable options. The deployed router must support the chosen integration; fixture creation and smoke testing verify that environment, rather than treating enum membership as proof. Build/update the native recorder with `npm run build:recorder`; older recorders fail fast for connector-attributed runs. No application image rebuild is needed.

Real-sandbox validation requires provider sandbox keys, required metadata, compatible test cards/currency in payment settings, and reachable router/PM services. The automated integration test uses a local mock router and two connector accounts per merchant; it does not contact Stripe or Adyen.

### Import existing fixtures

On **Fixtures**, choose **Import existing**, select a scenario, and enter one merchant
or upload/paste a JSON array. Each entry contains `merchant_id`, `profile_id`,
`api_key`, flow-dependent `publishable_key`, and a `connector_accounts` map from
connector name to merchant connector-account ID. An example entry is:

```json
[{"merchant_id":"merchant_123","profile_id":"pro_123","api_key":"…","publishable_key":"…","connector_accounts":{"stripe":"mca_123"}}]
```

**Import and verify** checks merchant credentials and profile ownership, then checks
that every selected connector account exists, is enabled, belongs to that profile,
and supports the required card configuration. Publishable-key inspection requires
the Router admin key in Connection settings. Failure details distinguish credential
verification from setup verification. Import never provisions or repairs remote
resources. Use a separate smoke test to check actual connector payment execution.
Changes to required setup or credentials invalidate readiness; use **Verify** again.
After changing destinations, explicitly reimport credentials for the new destination.

### Scheduled custom fault commands

Fault targets support **Built-in adapter** or **Custom commands · scheduled only**.
Custom commands require `LOADTEST_ALLOW_CUSTOM_COMMANDS=true` on the supervisor;
restart the console and its idle worker after enabling it. Configure the trigger,
optional restore (omit for self-recovering failovers), absolute working directory,
timeout, and read-only checks for healthy and fault-active states. Checks must exit
0 only when their expected state is observed. Schedule the target in scenario stages.
Commands execute through `/bin/sh` as the supervisor OS user, not in the browser or
on load workers. Stop/cancellation still performs required recovery. Command output
is discarded to avoid retaining secrets; failures identify the command stage and
exit status. Do not embed credentials in commands. See [deployment guidance](DEPLOYMENT.md).
