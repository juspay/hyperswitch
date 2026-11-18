# Step 12 checks and acceptance gates

Commands were inspected before execution. Cargo/test runs use OS-denied networking, explicit synthetic fixtures and owned build/cache directories. No uploader, live endpoint, Docker/cluster operation, credential fetch, application/payment submission, Git staging/history mutation or remote action ran.

## Isolation and preservation

The explicitly registered root is recorded in [evidence.json](evidence.json). Its owner marker contains a UUID, purpose `Step 12 isolated recovery checks`, matching canonical root/cleanup scope and null hosted target. Existing Step 11 owned Cargo caches/targets were separately registered for reuse; no ambient Cargo configuration/credentials were copied. Tests use `--offline --locked`, two build jobs, separate isolated/router targets and cleared wrappers. The inherited inspected build wrapper contains relative build-script outputs in the owned target. No dependency identity, manifest dependency, lockfile or submodule revision was changed.

The independent reader fixture allocates only a child of that root, writes synthetic journal/current-state/coordinator artifacts with create-new private files, drops the original fixture executor and starts a bounded read-only child with a cleared environment. It re-observes the lost create through production recovery code. It supplies no writer credential and opens no socket. The test verifies child ownership and cleans only its allocated child. Build caches and command logs remain for review. Local files and in-memory stores do not certify WORM, access separation, cloud encryption or retention enforcement.

Starting HEAD/branch/index and file hashes were captured before edits. Final checks compare H/W/U/I with that baseline, excluding credential dotenv content from reads. Prior uncommitted work remains. The original plan/roadmap, W/U/I and committed seed are outside this change.

## Executed checks

Exact argv, exits, durations, log hashes, changed-file hashes and preservation results are recorded in [evidence.json](evidence.json). This document reports final passing runs separately from intermediate failures.

```sh
cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib
cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib plan::tests::apply_tests::recovery_tests::fixture::independent_recovery_after_executor_disappears -- --exact --ignored --nocapture
cargo +stable clippy --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib --tests -- -D warnings
cargo +stable check --offline --locked -p router --features required_fields_sync --lib
cargo +stable clippy --offline --locked -p router --features required_fields_sync --lib
```

- Full isolated suite: **146 passed, 0 failed, 13 ignored**, including **15 ordinary recovery tests**. Parameterized cases cover lost response/create, intent-before-dispatch crash, divergence, unrelated edits, unchanged/deviating owned attributes, current IDs/grants, target generation, additive residuals, recreation/experiments/prerequisites, exact retirement review, already-restored no-change planning, unknown restore commits, post-commit errors, missing verification and ordinary-preparation bypass refusal.
- Recovery authorization faults cover missing revoke/drain/head/current-state binding, stale generation, wrong snapshot/baseline/image, insufficient retention, expired lease, failed immutable write/read-back and missing/corrupt source artifacts. No recovery mutation occurs on these failures. Model-positive verification is explicitly synthetic; it cannot certify any level.
- Explicit independent process fixture: **1 parent test passed**, including its separately selected child. The 13 default ignored tests remain visible; unrelated storage/CLI/transport/provider/browser fixtures were not implicitly counted as rerun.
- Strict isolated Clippy, actual router build/Clippy, formatting, diff/document checks and scoped secret scan: see final machine results. Existing warnings remain distinguished from new code.

Intermediate failures were resolved without weakening production boundaries: the test helper initially tried to deserialize sealed DesiredState/InputManifest and access a private BuildIdentity field; it now reconstructs synthetic fixture values and calls the existing build constructor. Strict Clippy flagged the shared planner's additional sealed recreation argument; a narrowly documented argument-count allowance preserves the single planner path. No sealed desired/input/build type was made deserializable to accommodate the fixture. Intermediate logs remain retained.

## Acceptance matrix

| Gate | Local evidence | Still required |
|---|---|---|
| T18–T19 request recovery | Exact saved wire comparison, observation-first classification, no replay after confirmed commit, no before-state retry without exclusion, one-send successor | Certified server request drain/termination, late-commit and post-commit failure drills |
| T20–T21 coordination | Owner/lease/intent, every-reader binding, immutable predecessor exclusion checked through dispatch, current-state revalidation | Real target-wide external-writer enforcement, workload/route/source attestations |
| T22/T24 inverse safety | Current prerequisites/experiments/order gates; three-way values/absence; new IDs; preservation; divergent edits and unsafe deletion refuse | Exact target projection/wire/order and arbitrary candidate browser acceptance |
| T23 durable evidence | Immutable decision read-back, original journal/baseline retained, missing/corrupt artifacts refuse, independent process reads after original disappears | Protected remote retention, encryption/decryption/access, complete reads and incident recovery access |
| T27 pre-feature rollback | Recovery executor can differ from current serving readers only under explicit snapshot/baseline/reader intent; baseline preserved in successor | Real retained image and supported pre-feature/old-reader compatibility, availability/access/retention drill |
| V1–V4 | Existing verifier boundary is mandatory; missing verification remains unverified | All real target/server/pod/browser/backup gates listed in Step 11 |

No MSRV 1.94.1, release image, cross-target, broader-feature/workspace build, actual browser rerun, hosted restore or application rollback is claimed. No production registry is enabled. Step 13 was not begun.
