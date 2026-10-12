# Validation — 7 October 2026

The SSR build and Svelte checks pass. Automated checks cover password hashing,
session persistence/revocation, login limiting, origins, secret redaction, weighted
profiles, concurrent-start rejection, immutable job profiles, SSE/reconnect,
cancellation, crash recovery, paginated history and two authenticated Grafana panels.
Go recorder tests pass with the race detector. A PostgreSQL integration check
compared 1,004 payments in two read-only batches and verified persisted matches,
mismatches, missing IDs and the incomplete-recording override.

The mock end-to-end console workflow passes at 10 offered flows/sec. Against the
running local services, the ten-second 10 flows/sec test recorded all 205 API
attempts with zero gaps and no dropped iterations. Router returned HTTP 500 for
all 103 payment creations (`Something went wrong`); successful payment TPS was 0.
Customer and PM session creation returned HTTP 200. Recording completed even
though the HTTP failure threshold failed the load test.

## Short local mock benchmarks

Each enabled/disabled run lasts ten seconds at 2,000 offered flows/sec. These
tests use an isolated HTTP mock, not the live services. CPU is generator user +
system time; memory is peak RSS. The Node mock runs on the same workstation,
so this is recorder characterization rather than an EC2 capacity measurement.

| Recorded run | 50/30/20 mix | Six flows with three delayed polls |
|---|---:|---:|
| Successful flows/sec | 2,000.2 | 1,997.9 |
| API attempts/sec | 8,800.7 | 11,169.1 |
| Persisted events | 88,007 | 111,691 |
| Recording / response gaps | 0 / 0 | 0 / 0 |
| Dropped iterations, disabled / enabled | 0 / 0 | 33 / 27 |
| CPU seconds, disabled / enabled | 13.59 / 15.18 | 16.89 / 19.95 |
| CPU overhead | 11.7% | 18.1% |
| Peak RSS, disabled / enabled | 625 / 636 MiB | 761 / 767 MiB |
| Peak recorder queue | 569 events / 308 KiB | 805 events / 438 KiB |
| Maximum commit latency | 14.8 ms | 14.5 ms |
| Received / sent bytes, enabled | 24,268,489 / 35,780,035 | 31,568,142 / 38,889,333 |

The <5% CPU overhead target has **not** passed. Delayed polling also produced
dropped iterations in both variants; this workstation test does not establish
1,000 flows/sec capacity. Use the [EC2 acceptance procedure](DEPLOYMENT.md) to
measure the deployed generator and sustain 1,000 flows/sec for 30 minutes in
both merchant modes.

Docker/Compose and deployment assets are supplied. No Docker engine is available
on this host, so the Linux image has not been built here. No EC2 instance was
provisioned. Grafana authentication, isolation and multiple panels were validated
against a mock upstream; validate the existing Grafana deployment's embedding
and auth-proxy settings before use.

## Load controls and resilience view (2026-10-07)

- `npm run check`: zero errors/warnings; production build passed.
- Console tests: 19 passed, 1 optional PostgreSQL test skipped.
- Go recorder/admission tests passed with the race detector; existing storage
  tests passed (6). Tests include weighted fractional admission, delayed ramp
  ticks, drop accounting, pause/drain, frozen active time and resume.
- Mock integration at 10 flows/sec: exactly 60 successful flows in six active
  seconds despite a pause, zero dropped flows, zero response gaps and complete
  recording. Staged ramps from zero also completed and retained their snapshots.
- Verified rejected competing starts while paused, SSE, cancellation, shutdown,
  interrupted-job recovery, simulated faults, durable restoration and preserved
  k6 threshold failures with complete recordings.
- Browser checks covered Idle Start, Running Pause, Paused Resume, zero active
  flows while paused, Completed without load controls, live-run selection,
  historical replay, simulation confirmation and collapsed Diagnostics.
- Docker/Kubernetes/GCP adapters were tested with injected command responses;
  no real outages or cloud failovers were executed. Simulation validates the
  control lifecycle, not infrastructure resilience.

These are local functional checks. EC2 capacity, sustained 1,000/2,000 flows/sec
and the recorder-overhead target remain subject to the deployment benchmarks
above. Console control pacing uses a different executor from native CLI tests;
benchmark both on EC2 before drawing capacity conclusions.


## Profile, fixture and worker separation — 2026-10-08

- Svelte check: zero errors/warnings; production build succeeds.
- Console suite: 22 passing tests; PostgreSQL opt-in test run separately and passes.
- Protected routes: Scenarios, Fixtures, Performance and Results; profile versions,
  stale-save rejection, connection edits independent of profiles, fixture signature
  compatibility and visual ramp/hold serialization verified.
- HTTP integration: SSE disconnect, active-page reload, pause/drain/resume,
  immutable configuration, cancellation, graceful and forced SSR restart all
  verified. SSR restarts preserve the active generator. A worker crash marks the
  job and recording incomplete; the next worker restores recovery state.
- Real local services: fixture creation completed for one new merchant, with
  configured Superposition defaults and a ready compatible manifest.
- Real local load: 10 offered and successful flows/sec for 10 active seconds;
  100 completed flows, 440 response events, zero HTTP errors, zero dropped flows,
  zero recording gaps. Confirmation p99: 1,195.9 ms.
- Direct PostgreSQL reconciliation: 100 matches, zero mismatches, zero missing
  payments. Results UI connection check, background reconciliation and page
  reload restoration verified in an isolated development preview.
- Recorded run: `single:2026-10-07T18-52-42-792Z-single-10tps-e70647e8`.
- New fixture set: `35a4c019-3568-45d5-9fde-c5b75bf0158f`.

EC2 capacity, a sustained 1,000 TPS run and the final Linux container deployment
remain deployment-stage validation; these local results do not establish that
capacity. Existing 1,500-merchant fixtures were imported, but no 1,500-merchant
provisioning or high-rate load was launched during this pass.


## Interaction and reconciliation improvements — 2026-10-09

- Final Svelte diagnostics: zero errors and zero warnings. Production build succeeded. Console regression suite: 67 passed, zero failed, one opt-in live PostgreSQL check skipped.
- Browser verification in an isolated production preview: scenario/target atomic save, full-width drag, keyboard reorder and Undo, saved navigation context, settings save/discard protection, database shortcut, worker onboarding, outcome filters, discrepancy jump, payment drawer and linked request failures. Checked 390px and desktop layouts with synthetic data.
- Exact synthetic 10,000,000-payment index: all 686 discrepancies preserved; overview returned 79 bars / 8,868 bytes in 7ms. Data creation and all nine aggregation levels took 75.7s on this workstation. These are local single-run measurements, not a deployment SLA.
- Tests cover pending→compared bucket updates, first-observation ordering, partial edge buckets, filtering, pagination, detail lookup, streamed CSV escaping/completeness, atomic rollback/conflict rejection, archive/version preservation, environment presets and weight balancing.
- Reconciliation integration uses real SQLite recording/index files with an injected PostgreSQL client: 1,201 payments compared in batches, 1,199 matched, one missing and one status mismatch; unresolved transport failures remain separate. The live PostgreSQL check remains opt-in.
- No real cloud/local infrastructure outage was executed for this pass. Load-generation capacity and real database throughput are separate from the ten-million-row chart benchmark.


## Console visual redesign — 2026-10-09

- Replaced layered control overrides with shared colors, spacing and explicit primary, secondary, quiet, destructive, success, disabled and focus states. New icon component uses local SVG paths; no external font or icon service is required.
- Redesigned navigation, scenario library/editor, expandable timeline steps, fixture progress, metrics strip, run details, reconciliation controls, settings sidebar, confirmation dialogs and sign-in. Existing API contracts and saved configuration are preserved.
- Svelte diagnostics: zero errors/warnings. Production build passes. Console tests: 67 passed, zero failed, one opt-in PostgreSQL test skipped. The chart regression now checks future-time proportions independently of viewport width.
- Isolated production-preview browser checks: 390px and desktop layouts; row-body dragging, Alt+Arrow reordering, stage editing/save feedback, unsaved-dialog Continue editing, settings keyboard navigation/focus, inline service-check failure, graph inspection by keyboard, discrepancy jump and payment detail drawer. Synthetic data only; no load or infrastructure interruption was executed.


## Settings and menu follow-up — 2026-10-09

- Settings retains the same dimensions and position across all five tabs: 940 × 656 at a 1280 × 720 viewport, and 352 × 812 at 390 × 844. Long forms scroll inside the content panel; the sidebar/header/footer stay visible. Switching tabs resets content scroll.
- Timeline and scenario action menus share outside-click, Escape, selection and focus-leave dismissal. Verified only one menu stays open, disabled move actions remain disabled, and selecting Move down still reorders the stage.
- Neutral buttons, menu triggers/items, settings tabs, editable rows and table rows use shared pale-blue hover tokens; primary, destructive and success/failure variants retain their distinct states.
- Svelte diagnostics: zero errors/warnings. Production build passes. Browser checks use the isolated synthetic workspace without executing load or faults.

## Local outage, recording and reconciliation validation — 2026-10-10

- Exercised the real console at `127.0.0.1:4174`, before, during and after load, and before/after reconciliation. Used the existing `local-fault-sequence` and ready fixture `df749926-6b16-4713-8418-ed19911751e5`: 1 TPS hold 20s, vault outage 20s, ramp to 5 TPS over 5s, PostgreSQL outage 120s, then 5 TPS hold 60s. No cloud resources or database roles were created.
- Two initial full runs reproduced the recovery complaint. With the router's 180s master/replica DB pool acquisition wait and the client's 30s timeout, successful confirmations did not resume during the post-recovery hold. Even after raising the local router's descriptor limit to 4096, the next full run reproduced the stall. A controlled rerun with all four primary DB pool waits set to 5s resumed fresh traffic after DB recovery: 5.02 successful TPS and zero HTTP errors over the measured 180–230s interval. These tested timeout values were persisted in the running router checkout's `config/development.toml` (`/Users/mohammed.kashif/codes/hyperswitch`); the local router also requires its existing 16 MiB Rust worker stack setting.
- Final run: `single:2026-10-10T08-53-27-447Z-single-5tps-f32e9996`. PostgreSQL recovery was verified at 175.3s; 331 confirmations succeeded after verification. 225s of active load took approximately 236s including fault activation/recovery delays and final drain. The ramp followed vault recovery verification; the final hold followed DB recovery verification. The graph preserves the full remaining horizon for every metric, and HTTP error samples now use interval deltas instead of a cumulative rate. The injected outage still violates the unchanged `http_req_failed < 1%` threshold, so the run correctly shows failed checks.
- The final recording is complete: 1,378 events enqueued and persisted, zero pending writes and zero flows in flight at the end. Reconciliation inspected all 979 outcomes: 399 payment IDs (394 status matches, five status mismatches) plus 580 failed creates without returned IDs. Five known-ID confirmations returned HTTP 500 and remain independently filterable as confirmation failures. Database comparisons use 100-ID batches. Reconciliation took 67ms on this local run; this is not a capacity guarantee.
- Failed creates retain the exact response event, HTTP status or transport error, message, body, merchant and time in the console SQLite recording/index. Their category is **Creation unverified**, because a timeout or 5xx without a returned ID cannot prove that no application row was created. Confirmation failures preserve both the API failure and the later database status comparison. Verified both categories in the browser, including the linked timeout and HTTP 500 response drawers and CSV exports.
- Performance controls/status are grouped inside Load timeline during execution and after completion. Replay and diagnostics are collapsed, the single-flow selector is hidden, run labels omit scenario versions, fixture labels show the short hash/date, and services form a centered bottom dock. Results use one elapsed-time chart consistently across navigation, compact nonzero outcome counts, and a collapsed inspection table.
- Superposition exited with code 0 during a DB interruption; its local `on_failure` restart policy did not restart that clean exit. Restarted the existing process after PostgreSQL recovered and verified Router, PM, DB, Superposition and Card vault healthy. This remains a local service-supervision limitation for subsequent DB outage tests.
- A 1 TPS / 10s smoke passed (`612080c8-a049-4429-96ac-1d3a0906f8f7`). Final automated validation: console 79 passed, zero failed (the opt-in PostgreSQL test was run separately and passed); native-recorder/scenario suite 15 passed with zero skips; Go recorder tests passed; Svelte diagnostics zero errors/warnings; production build and `git diff --check` passed. Cancellation while a fault is active, recovery gating, failed-threshold/complete-recording separation, failed-attempt indexing and 100-ID reconciliation batches are covered.

These checks establish the exercised local 1–5 TPS workflow. They do not establish cloud readiness or high-rate production capacity. Screenshot evidence is saved in `/tmp/loadtest-ui-validation/`.

## Traffic semantics and concise timelines — 2026-10-10 follow-up

- Flows/second now uses native admission counters (`started_flows` and per-flow `flow_starts`), incremented when a VU receives a flow slot. It includes blocked and subsequently failed requests; dropped slots and response completions do not increment it. Successful TPS retains a separate success counter. Distributed aggregation preserves the same distinction. Earlier completed recordings reconstruct starts from each flow's first HTTP attempt in bounded time bins; they do not have the new exact admission timestamp.
- Smoke execution now uses the same native controller while response recording remains disabled. Its final Successful TPS uses completed successes over active duration, avoiding the final sampling-window spike. Verified the real local 1 TPS / 10s smoke `d7f0d328-a3bc-4d63-b138-a0396bd5b423`: ten admissions, ten successes, zero in flight, zero persisted responses, passed checks, final display 1 TPS. The native delayed-response regression verifies admissions are visible before any success and confirms a failed smoke threshold still returns exit 99.
- Removed graph legends, duplicate reconciliation total blocks, explanatory verdict paragraphs, the second database-settings CTA and the unrelated Results acceptance panel. Outcome counts live in the filter; exact failure responses, status comparisons and CSV export remain in the collapsed inspector. Performance uses thin traffic/target lines and smaller event markers; threshold failure is represented by the timeline status instead of the long global completion alert. Empty activity and single-flow breakdown sections are hidden.
- Reconciliation now overlays Hold, Ramp and actual outage phases. Outage shading starts at verified interruption and continues until verified recovery, including restoration delay. The subsequent Ramp/Hold starts at recovery verification. Labels remain visible for the five-second ramp at the exercised desktop viewport; exact phase boundaries are available on hover. Reload/navigation retains the elapsed-time representation.
- Reconciled the same full local outage run again through the simplified UI: 979 outcomes, unchanged 394 matches, five status mismatches and 580 creation-unverified attempts; five known-ID confirmation failures remain separately filterable. All five configured local services were reachable afterward.
- Final checks after this follow-up: console suite 84 passed, zero failed, one opt-in PostgreSQL skip; the real read-only PostgreSQL test was run separately and passed. Native/scenario suite 16 passed with zero skips, including the delayed-response/controller smoke regression. Go recorder tests passed. Svelte check reports zero errors/warnings; production build and diff whitespace check pass.
- Updated screenshots: `/tmp/loadtest-ui-validation/performance-clean.png`, `smoke-clean-final.png`, and `results-clean.png`.
