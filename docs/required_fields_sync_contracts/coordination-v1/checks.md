# Step 9 inspected commands and executed checks

See [evidence.json](evidence.json) for final exit codes, log hashes, source bindings and
preservation checks. Commands below were inspected for side effects before execution.

## Isolation

Owned fixture: `required-fields-step9-ulr8huij`, owner
`be9263eb-65ed-4005-93ba-b6a0b968a266`, purpose `Step 9 isolated coordination checks`.
The canonical root and equal cleanup scope are in its `owner.json`; the machine record has
the local log path. Separate copied Cargo sources/cache and isolated build targets avoid
the known superposition_core/cbindgen relative-output side effect in shared sources.
No Cargo credential/config files were copied. Builds are offline/locked except the single
documented offline lock-resolution command below. Two jobs, debug/incremental disabled,
and Rust compiler wrapper variables cleared. No unrelated application startup occurs.

Ordinary checks run under macOS `sandbox-exec` with `(deny network*)`. The opt-in fixture
allows loopback binding/inbound and **only loopback remote destinations** for outbound calls.
The exact final policy is recorded in the local `network-proof.json`; a loopback bind passed
and a connect to the reserved documentation address `192.0.2.1` was denied by the OS.

The first policy used a literal address unsupported by sandbox-exec and exited 65 before
running the fixture. A second combined allow rule proved too broad (the documentation-address
probe timed out instead of being denied). It was replaced with directional rules and verified;
the final transport test was rerun under the corrected policy. No hosted target was probed.
The adapter itself always restricts this test constructor to literal `127.0.0.1`, no proxy,
redirect or retry, independent of the OS policy.

The Python wire server starts only after the exact root marker, fresh owned child and synthetic
object UID/namespace/name are verified. The actual adapter must read the matching fixture
header/object before PUT. The test covers refusal before that read, resourceVersion conflict,
denied read, committed PUT with lost response, no retry, and unresolved state after process
restart. The parent kills/waits for its own children and removes only its allocated child.
It does not contact Kubernetes or certify it.

## Commands and results

Run from H using the registered fixture runner (full command arrays in the machine record):

```sh
cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib
cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib kubernetes_coordination::tests::owned_loopback_adapter_cas_conflict_unknown_outcome_and_restart -- --exact --ignored --nocapture
cargo +stable clippy --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib --tests -- -D warnings
cargo +stable check --offline --locked -p router --features required_fields_sync --lib
cargo +stable clippy --offline --locked -p router --features required_fields_sync --lib
```

The final ordinary suite passes **111 tests**, with eight opt-in cases ignored. Nineteen new
ordinary Step 9 cases cover authority, mixed readers/traffic, scaling, barrier failures,
owner/lease/CAS faults, independent heartbeat, resumed-after-expiry refusal, takeover proof,
sanitized reports, raw wire validation and actual accepted planner/journal checkpoint binding.
The separately selected loopback case passes; the other ignored exporter/journal/CLI helpers
were not executed in Step 9. Their prior evidence remains historical.

The initial locked test refused the companion lock change before compiling. One
`cargo +stable check --offline --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib`
resolved the existing `time 0.3.41` formatting/parsing feature edges offline. Only the companion
package→time and time→itoa edges changed; all 168 external dependency identities/checksums
still match H's lock. No version upgrade or root manifest/lock change occurred. Subsequent
commands use `--locked`. Earlier 108/109-test passes preceded the final integration and
lease-expiry/heartbeat additions; only the final count describes current ordinary coverage.

Actual package build/Clippy results and warnings are in `evidence.json`. The first router
check overlapped the final lease hardening; the subsequent router Clippy command compiles
the final source. Existing analytics imports, renderer qualification and Step 6 preflight
casts are recorded separately from any new warnings. No unrelated warning cleanup is included.

Initial Clippy refused a test Mutex guard spanning an await; it was scoped explicitly before
awaiting and the strict isolated check was rerun successfully. The router check also reported
a new unnecessary qualification in `coordination.rs`; that was fixed before final-source
router Clippy. These initial diagnostics are retained in the logs, not reported as clean passes.

Scoped nightly rustfmt, Python syntax compile without repository bytecode, JSON/source binding,
relative Markdown links, `git diff --check`, dependency equality and default-rule gitleaks
on copied changed files are recorded in the final machine record. The secret scan uses no
repository ignore/config, clears GITLEAKS environment variables and does not honor allow
comments. It does not claim a whole-history scan or replace prior incident review.

## Not executed / not certified

- Declared Rust 1.94.1 MSRV is not installed; checks use installed stable 1.97.1. Release/image,
  cross-target, broader v1/v2 feature matrix and full-workspace tests were not run here.
- No real Kubernetes API/RBAC/admission/clock-skew/retention test. The explicit local OrbStack
  image filters returned no cached control-plane/node images; no pull/bootstrap was attempted.
- No actual authenticated release publisher, deployment/Argo/Rollout/InfraSwitch gate,
  external-writer gateway or old-server-request exclusion drill. These adapters refuse.
- No Superposition API server, hosted storage/credentials, live apply, resume, restore or
  V1–V4 execution. Only local coordination fixture writes were authorized and performed.

After checks, only registered disposable build/cache/scan children are eligible for cleanup;
logs, owner marker and original preservation baseline remain for review. Step 8's independent
fixture and all repository indexes/history/branches remain untouched.
