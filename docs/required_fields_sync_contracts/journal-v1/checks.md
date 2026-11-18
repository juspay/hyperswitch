# Step 8 checks and acceptance boundaries

Commands were inspected for side effects before execution. Cargo uses `--offline --locked`,
two jobs, no incremental/debug output, cleared compiler wrappers and owned target directories.
Registry/git sources were copied to an owned `CARGO_HOME` without credentials/config so the
existing Superposition build script cannot modify the shared source cache. The OS sandbox
denies network; a loopback bind probe failed with PermissionError before checks ran.

The disposable filesystem root is identified by owner
`4e1acf42-6636-4916-b9c3-df41131c47dc`, purpose `Step 8 isolated journal checks`, exact canonical
root and cleanup scope in `owner.json`. Rust filesystem tests require
`RF_JOURNAL_FIXTURE_ROOT`, verify that marker, create a fresh child using create-new semantics,
and remove only that owned child. They do not touch the existing file-storage workspace path.
Memory stores are synthetic per-test fixtures. No hosted store, SDK storage client, API server,
credential loader, DB, Redis, uploader or deployment is invoked.

## Executed

See [evidence.json](evidence.json) for final exit codes, durations and source/log digests.

| Command | Result/scope |
|---|---|
| `cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib` | 92 passed, seven opt-in tests ignored. Includes 16 journal unit/fault tests and one parameterized actual accepted-planner snapshot test, with synthetic evidence only. |
| Same command with `journal::tests::fixture::independent_process_recovery_and_filesystem_tamper -- --exact --ignored --nocapture` | One parent fixture passes and invokes a passing child recovery test in a separate process, after original writer/store handles are dropped. Child receives only the owned child identifier, not a prepared receipt, locator or final report. |
| `cargo +stable clippy --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib --tests -- -D warnings` | Isolated actual-source lint check; final result in evidence. |
| `cargo +stable check --offline --locked -p router --features required_fields_sync --lib` | Actual opt-in default-v1 library build passed with three pre-existing warnings. This first build overlapped final journal hardening; the separate final-source Clippy build below closes compilation for the final sources. |
| `cargo +stable clippy --offline --locked -p router --features required_fields_sync --lib` | Passed for final default-v1 opt-in library sources. Five pre-existing warnings: two analytics imports, renderer qualification and two Step 6 preflight casts; no new journal warnings. |
| `rustfmt +nightly --edition 2021 --config skip_children=true --check <changed Rust files>` | Scoped formatting; no workspace rewrite. |
| `git diff --check`, scoped document links/JSON/source binding and default-rule redacted secret scan | Final results in evidence. No staging or commit. |

The initial test run found a nested Serde tag collision in observation events (86 pass,
two fail, seven ignored). Event observation payloads now use an explicit nested field;
the subsequent complete suite passed. This was a local format bug, not server evidence.
Final tests also cover newly added read-back failures, missing markers, aggregate limits,
and both shared-row replacement and pending-create capture. All final results refer to
the recorded source hashes, not an earlier passing revision.

## What the checks establish

| Gate | Local evidence | Still pending |
|---|---|---|
| T18 journal crash points | Before intent leaves no pending dispatch; after intent/before dispatch retains exact bytes and unknown work. Lost storage acknowledgement for intent/outcome is found by independent recovery. Poisoned handle refuses retry. | Actual server dropped response after DB commit, webhook 512 and cache-publication fault. Tests only record synthetic transport statuses/outcomes. No apply machine or server mutation was exercised. |
| T23 create/read-back | Read denial, corrupt/truncated bytes, wrong immutable version, absent hold, missing baseline/prepared/unresolved marker, conflicting creates, duplicate/out-of-order intent, bounded slots and aggregate refusal. | Exact provider consistency and atomic create semantics, transport faults and approved access policy. |
| T23 immutability/access | Fixture adapter's second create fails without overwrite; OS-owner tamper is detected, deletion below the retained head refuses, and an independent process recovers after handle loss. Capability validation rejects each missing guarantee. | Real WORM/delete denial, encrypted data/key grants, logs, separate recovery principal, power-loss durability, lifecycle expiry and hold protection. No claim that a local filesystem supplies these guarantees. |
| T24 retained recovery evidence | Exact first-adoption baseline, full raw shared rows and metadata, before/after images, requests/order, server IDs, pending-create intent, accepted approvals/cases and direct observations survive. Wrong target/instance/baseline and hash mismatches refuse. | Three-way restore, old-request exclusion, priority-safe recreation and managed-field conflict verification are Step 12 work; no restore was run. |
| Security/build | Closed production storage registry, no mutation/lease/HTTP path, credential refusal and sanitized reports; actual source tests and supported build recorded separately. | Q6 input classification/retention/access approvals; declared MSRV 1.94.1 and release/image/dummy/TLS/Redis/v2 feature matrix are not inferred from stable checks. |

The reader scans a bounded immutable prefix without relying on a storage listing. The
minimum-head anchor can detect removal of the last acknowledged event. Without a retained
anchor, a provider able to delete/rewrite a suffix could conceal history: certified immutable
storage and Step 9's persistent unresolved record remain mandatory. An expired lease, a
before-state observation or HTTP success never clears unknown work in this implementation.

## Preservation and scope

H starts at `480e15c04a1524b7c297d1a8df761fcef55be4fa`. W remains main at
`939f45786aee1c3b14e3f0b134bb42cb2cf2dd39`; U remains detached at
`1af3e5dabf9d5a17dd864ac7312151ebac4367c6`. Pre-existing W harness files remain untracked.
Original H plan/roadmap files remain untracked and unchanged. Previous Steps 6/7 were
committed/pushed and a draft PR opened under the prior explicit request; this Step 8
does not edit that PR or repeat those actions. No index, branch, history, seed, business
source, SDK, dependency manifest/lock or existing storage adapter is changed.

Final preservation compares Git-visible file hashes and index bytes against the initial
H/W/U baseline. Cleanup and any retained local logs are listed in evidence; shared caches
and other fixtures are outside this step's cleanup authority. Stop for review after Step 8.

Final cleanup verified the owner marker and removed only this step's copied Cargo home,
two build targets and scan copy. Test-created journal children were already removed. The
34.7 MB synthetic recovery test executable, baseline, logs and small evidence files are
retained under the registered fixture; its hash is recorded in evidence. It is not an
attested release recovery image. The original plan/roadmap, all 580 W and 745 U baseline
files, H files outside the eight listed existing-file edits, and all three indexes/revisions/
branches passed preservation checks. No later step or remote action was performed.
