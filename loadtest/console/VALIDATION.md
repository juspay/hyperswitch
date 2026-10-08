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
