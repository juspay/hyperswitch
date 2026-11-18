# Step 10 — inspected commands and executed checks

Run from H, 18 November 2026. Results in [evidence.json](evidence.json) are scoped to this
step and distinguish synthetic execution from unavailable exact-server certification.

## Isolation and side effects

Before editing/running checks, inspected the selected playbook/plan sections, applicable
repository instructions, revisions/indexes/worktrees, existing modules and build scripts,
companion dependency setup and actual locked SDK serializers. No package install, uploader,
source-data correction, hosted credential read or deployed target selection was authorized.

Registered owned root:
`/private/var/folders/3r/5h91zf1d2515z0phtg4mdsv40000gp/T/required-fields-step10-mj68gdwn`.
Owner UUID `4f95d2b2-c161-4e9a-b32b-a6e4b20d705a`, purpose
`Step 10 isolated apply checks`; canonical root equals cleanup scope. Baseline hashes and
copies of the starting sources preserve the already uncommitted Step 8/9 work for incremental
review. Cargo registry/git caches were copied into its private CARGO_HOME without configuration
or credentials; this also contains the existing cbindgen relative-output side effect.

The runner uses two jobs, no incremental/debug output, separate isolated/router target
directories, cleared Rust compiler wrappers and `sandbox-exec` with all networking denied.
Commands are offline/locked. Ordinary tests apply only synthetic values in memory, including
the actual SDK's intercepted requests. No DNS, socket, API server or payment request is used.

The journal regression gets a separately registered `journal-fixture` child with purpose
`Step 8 isolated journal checks`, its own UUID and equal canonical root/cleanup scope. Its
existing parent test creates only a new owned child, launches a reader with a cleared
environment, checks overwrite/tamper/missing-event behavior and removes that owned child.
Local fsync/create_new is not encryption, WORM or independent-role certification.

Only read-only Docker image listing/selected metadata inspection used the explicit previously
authorized local OrbStack socket. No container/workspace was created, seeded, contacted or
cleaned up. No environment variables from image/container inspection were requested.

## Commands/results

Each Cargo command below ran through the owned runner; full argv, result and log hashes are
retained in the machine record. Installed stable is Rust 1.97.1.

```sh
cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib
cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib mutation_wire
cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib journal::tests::fixture::independent_process_recovery_and_filesystem_tamper -- --exact --ignored --nocapture
cargo +stable clippy --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib --tests -- -D warnings
cargo +stable check --offline --locked -p router --features required_fields_sync --lib
cargo +stable clippy --offline --locked -p router --features required_fields_sync --lib
```

- Final ordinary suite: **126 passed, 0 failed, 8 ignored**. Fifteen new ordinary cases:
  twelve apply tests (many parameterized faults), two actual SDK wire/retry tests and one
  legacy journal-reader test. Earlier intermediate suite passes do not cover final additions.
- SDK selection: **2 passed**; whole suite also executes them. Three operation types each
  dispatch once for 400/409/429/500/503/512 and for middleware timeouts. Requests are compared
  to independently specified compiler actions, with explicit ordered-array/decimal assertions.
- Independent-process filesystem parent: **1 passed**, including its child reader. The eight
  default ignored cases remain visible; only this parent and its selected helper were
  separately executed in Step 10. Earlier exporter/browser/CLI/loopback evidence is historical.
- Strict isolated Clippy and actual router check/Clippy: **pass**. The final router Clippy
  compiles the final source; earlier router checks preceded the last outcome/cancellation
  hardening. Exact command results are in the machine record. Existing router analytics imports, renderer qualification, two Step 6 preflight conversion
  warnings and the dependency future-compatibility notice are distinguished from new code.
  No unrelated warning cleanup or `cargo fix` was performed.

Initial tests caught a test-event borrow/move error. Initial strict Clippy caught large
argument lists and then a redundant field name; typed outcome/observation records fixed
those. Capacity hardening exposed a coroutine capture/move error; copying the approval expiry
before the coroutine fixed it. Failed commands are retained with exit 101, not counted as
passing. Final source is tested after those corrections.

The engine reserves the journal reader's aggregate capacity before storage or mutation:
actual frozen intent sizes plus bounded transport/resolution/proof/report space and the
retained first-adoption reference. Oversized or overflowed histories refuse before dispatch.
This can lower the 32-request ceiling for large snapshots/baselines; it never creates batches.

Scoped nightly rustfmt check, incremental diff review, JSON and source-hash bindings, relative
Markdown links, H-lock dependency equality, preservation comparison and `git diff --check`
are recorded in the final machine record. The first preservation audit found nine Unicode U
icon paths missing from the starting hash inventory (quoted path handling); the corrected
NUL-delimited audit checks them against pinned HEAD, unchanged status/index and older mtimes.
It does not claim missing starting hashes existed. Default-rule gitleaks scans copies of the changed
files, with GITLEAKS environment settings cleared, no repository config/ignore/baseline and
allow-comments disabled. It does not attest the whole repository/history or resolve earlier
secret-scan incidents.

## Gates not executed / not certified

- **Exact Superposition server cases:** not run. Cached image/tag is not source/build provenance
  or a verified disposable fixture. Auth/permission scope, full pagination, rollback,
  ordered-array identity, experiments, limits, post-commit webhook/cache failures, real raw
  transport retries/timeouts and old accepted-request drain still need Step 2 certification.
- **Real infrastructure:** no storage/retention/access, Kubernetes, release publisher,
  external-writer barrier or rollout gate certification. Empty factories continue refusing.
- **Verification/restoration:** target V1–V4 and terminal release/recovery/restore unimplemented
  in this step. Missing verifier never reports success. No hosted apply/resume/restore wired.
- **Build matrix:** declared Rust 1.94.1 is not installed; no MSRV download, release image,
  cross-target, broader feature matrix or full-workspace test was executed. Actual default-v1
  opt-in library checks do not attest those gates.
- **Business/SDK:** no live browser candidate certification, source option/mapping correction,
  SDK/submodule update, committed seed regeneration or profile approval.

Only owned build/cache/scan children may be removed after final checks; owner marker, logs,
registration, baseline and starting-source copies remain for review. No Git index/history,
branch, remote PR or deployment action was performed. Step 11 awaits separate review/request.
