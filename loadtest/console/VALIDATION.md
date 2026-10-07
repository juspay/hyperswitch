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
