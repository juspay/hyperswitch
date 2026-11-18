# Step 13 checks

Commands were inspected before execution. All Cargo commands use `--offline --locked`, a
cleared/allowlisted environment, two jobs and OS-denied networking. The explicitly registered
Step 13 owner root contains only synthetic inputs/reports and command logs. Previously owned
Step 11 Cargo dependency caches and isolated/router targets are registered for reuse; no
ambient Cargo configuration or credentials are copied. Source/index/lock hashes are checked
separately from build output. No hosted access, uploader, container, cluster or application
startup/payment action is authorized or executed.

The focused suite compiles the actual router modules. Its synthetic success cases prove control
flow only; they do not certify HTTP servers, W/U browser artifacts, image provenance or external
writer exclusion. The binary harness verifies network denial before executing early commands
with deliberately invalid DB/Redis settings, synthetic command configuration and private outputs.
It checks the actual binary digest against its fixture registration and never supplies a live
credential. The fixture exporter and file checks require their owner marker before writes.

Executed command results, intermediate failures, final source hashes and preserved files are
recorded in [evidence.json](evidence.json). An ignored or unexecuted check is not a pass.

## Executed local checks

Commands below ran through the owned fixture runner with the isolation above. Full arguments,
durations and log hashes are in the evidence file. Tests that create fixture files use private
create-new outputs and require explicit ownership registration; do not run ignored tests against
an arbitrary directory or use `--include-ignored` for the entire suite.

| Command/check | Result |
|---|---|
| `cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib` | 153 passed, 0 failed, 14 ignored |
| Same suite, explicitly selected owned operation file-contract/export test | 1 passed; strict input/output, private permissions, redaction and fixture export |
| Same suite, explicitly selected owned offline command tests | 4 passed |
| Isolated feature-off CLI parser tests | 2 passed; ordinary serving/scheduler argument contracts |
| `cargo +stable clippy --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib --tests -- -D warnings` | Passed; existing unreachable `uuid::Uuid::now_v7` Clippy configuration warning remains |
| `cargo +stable build --offline --locked -p router --features required_fields_sync --bin router` | Passed; actual default-v1 feature closure, existing warnings retained |
| `cargo +stable clippy --offline --locked -p router --features required_fields_sync --lib --bin router` | Passed; final run has only existing email-import, renderer qualification and preflight conversion warnings |
| `cargo +stable check --offline --locked -p router --bin router` | Passed with the opt-in feature disabled; existing unrelated email-import warnings retained |
| `python3 docs/required_fields_sync_contracts/operations-v1/test_cli.py --root <registered-root> --router <registered-binary>` | 72 actual-binary cases passed; denials, argument/config security and early dispatch, not successful real-target execution |
| `python3 docs/required_fields_sync_contracts/cli-v1/test_cli.py --root <registered-child> --router <registered-binary>` | 31 actual-binary offline render/plan regression cases passed |
| Actual build-script compile guards, router/payment_methods sync without v1 and sync with v1+v2 | All 4 rejected as expected; this is not a full v2 build |
| `rustc +stable --edition=2021 …/startup_probe.rs --extern router=<built-router-rlib> --extern serde_json=<matching-rlib> …`, then owned probe | Compiled; 5 cases passed against the actual public router library startup function |
| `rustfmt +stable --check --edition 2021 --config skip_children=true <changed Rust files>` | Passed; nightly-only repository formatting options are unavailable on stable |

The startup probe is a test executable, not a new product command or mutating service. Its
library digest, Cargo feature fingerprint and exact link arguments are recorded separately.
It linked the reviewed library before the final report-delivery indexing cleanup; the startup
implementation was unchanged. The command harness uses the separately rebuilt final binary.
It exercises absent/matching/drifted/seed/missing profiles, effective-input hashing, nonsecret
output and independent rollout/sync/verification states. The matching profile explicitly
excludes the compiled sources for synthetic plumbing; it is not a business-profile approval.
It does not launch serving, AppState, payment submission or a platform workload.

The installed compiler was Rust 1.97.1 (`8bab26f4f`, `aarch64-apple-darwin`), selected explicitly
with `+stable`. The MSRV 1.94.1, release image and expanded feature/platform matrix were not run.
The final rebuild also retains the existing renderer qualification warning, macOS linker unwind
size warning and dependency future-compatibility notice; these are not new clean-build claims.

Two attempted router unit-test gates remain blocked, separately from that successful probe:

- `cargo +stable test --offline --locked -p router --features required_fields_sync --lib command_runtime::attestation_tests -- --include-ignored`
  failed before tests ran: existing `analytics_report.rs` test configuration references
  `external_services::email` while the email feature is disabled.
- The same command with `--features required_fields_sync,email` failed offline because
  `aws-sdk-sesv2 v1.123.0` is absent from the registered Cargo cache. No dependency was fetched
  and no unrelated email behavior was changed.

## Acceptance boundaries

| Gate | Local check | External acceptance still required |
|---|---|---|
| T25 early dispatch/security | Parser, strict configuration, file/redaction tests and actual binary denials; render/plan regressions | Registered exact-target read/write credentials, endpoint and permission certification |
| T26/T27 identity/recovery | Actual executable hashing, altered image/build/input/profile/source rejection; status, inverse plan, resume/restore and late-request exclusion faults | Platform workload/route membership, immutable image/config provenance, old/pre-feature readers and retained image access |
| T32 outcomes/reports | Existing engine outcomes preserved, no verification stub, lease counts, bounded telemetry and failed-output preservation | Approved report/metric ingestion and real incident recovery visibility |
| T33 build/startup | Supported opt-in build, feature-off parser, explicit rejected-build guards; startup facts do not execute sync | MSRV/release-image and broader TLS/Redis/dummy/v2/target matrix |
| T34 one-shot executor | Final event/file format and separate rollout/sync/verification states | Actual Job/collector/retention/alerts; source/Helm/platform dependencies remain unavailable |
| Hosted activation | Every production registry/provisioner remains refusing | All previous contract, journal, coordination, restoration and target V1–V4 gates |

No source-data/catalog approval, remote server certification, browser rerun, real per-pod probe,
backup startup, deployment/restore, platform collector delivery or external alert is claimed.
