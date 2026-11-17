# Step 5 checks and acceptance gates

Commands and build-script side effects were inspected before execution. No uploader,
AppState/settings secret load, hosted credential, API/server/container invocation, live
inventory, or mutation was used. All request tests use synthetic data in memory; the
production reader explicitly rejects their test-only certificate before HTTP dispatch.

The disposable filesystem fixture `required-fields-step5-6p0zanjv`, owner
`15df1abc-877b-49b5-ae1a-3e2564b1c67d`, was registered before writes for
`Step 5 offline read-only inventory checks`. It owns source/index baselines, build outputs,
logs and an isolated dependency cache copy. It is not a Superposition workspace. Only this
verified owned root is eligible for cleanup. Final preservation/cleanup outcomes are
recorded in [results](evidence.json).

The sanitized JSON is named `evidence.json` because H's existing `result*` ignore rule
would hide a new `results.json` from ordinary review/staging. No ignore rule or earlier
step's artifacts were changed.

## Executed local checks

All Cargo commands use `CARGO_NET_OFFLINE=true`, owned target directories, two jobs,
disabled incremental/debug output and cleared Rust compiler wrappers. Installed stable
is 1.97.1; the declared 1.94.1 MSRV was not installed/tested. The companion manifest below
imports the actual production source files; it neither replaces router dependencies nor
claims to compile Settings/AppState or a router release.

| Command/check | Result |
|---|---|
| Initial `cargo test --offline --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib` | 41 pass. Only the newly created companion lock was pruned from H's lock; root lock unchanged |
| Final `cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib` | **47 pass, 0 fail/ignored**: 18 existing artifact tests, 23 inventory tests, five adapter tests and one actual SDK comparison covering seven operations |
| `cargo check --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib` | Pass, including the production `cfg(not(test))` certificate boundary |
| `cargo clippy --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib --tests -- -D warnings` | Pass |
| Compare companion registry `(name, version, source, checksum)` tuples against H `Cargo.lock` | **162 match; zero mismatches** |
| `cargo metadata --offline --locked --format-version 1 --manifest-path <cached superposition_core-0.119.0>/Cargo.toml` | Pass. The old cbindgen metadata/cache blocker is not reproduced in the current cache |
| `cargo check --offline --locked -p router --features required_fields_sync --lib` | **Pass** in the actual router/default-v1 feature closure, including final recheck after credential binding; three pre-existing warnings only |
| Targeted nightly rustfmt, whitespace/new-file diff review, documentation/JSON/link review, redacted changed-file secret scan and H/W/U/index preservation | Final outcomes in evidence.json |

Router build isolation: `superposition_core` runs cbindgen and writes outside
`CARGO_TARGET_DIR`. Its actual build was run with an owned `CARGO_HOME` containing copies
of only registry source/index/archive and Git dependency caches, without ambient config or
credential files. An initial incomplete clone failed on missing `fastrand` source in that
owned copy; the original local archive/source existed. A subsequent copy encountered
existing read-only Git cache files/symlinks. Recreating only the owned Git-cache copy fixed
the setup; this is not a missing external dependency or a successful router build. No
shared cache, dependency source, `DOCS_RS` bypass or lockfile was used to hide a build failure.

## Coverage and limits

| Gate | Local evidence | Still required |
|---|---|---|
| T13 identity/conditions | Returned IDs; duplicate IDs for every collection; map-key permutations; duplicate/escaped JSON members; malformed conditions; retained/quarantined unsupported predicates; same override ID with different ordered content refuses | Exact-server identity/hash contract, semantic ownership/overlap refusal in Step 6. Quarantine is never ownership or permission to mutate |
| T14 inventory | Multiple/empty pages with resource-specific boundaries; dropped/repeated pages; changing/missing totals; full counts with duplicate IDs; explicit filters/partial flags; 401/403/206/redirect/error bodies; wrong media/encoding/range/length/age; JSON depth/node and byte/item/page/request/time caps | Real server pagination, direct full rows, permission-scoped reads, concurrent mutation and fault injection for each exact server/client pair |
| T14 consistency/preservation | Two complete passes; three version heads; workspace identity/version checks; same counts/version with changed rows/envelopes/ordered arrays refuses; unknown large numbers, schema/audit/function/metadata preserved; target/principal/receipt-bound proof | Prove all authorized reads disclose all rows/fields. Two stable scoped responses can still omit data, so sealed full-scope evidence is required before network I/O |
| T16 read wire surface | Seven actual SDK 0.119.0 requests agree with the direct reader for methods/base path/count/page/unfiltered body/org/workspace headers; retry disabled; synthetic HTTP dispatch blocked; an authorization header differing from the independently reviewed credential binding refuses | Auth/transport, response shapes, stable order/latest versions/empty conventions and compatibility with each candidate's exact image/source. No real request was executed |
| Step 5 build boundary | Actual source imported by isolated library/test checks; opt-in router module wiring unchanged | Actual sync-enabled default-v1 router check passes; MSRV/release/other-feature matrix is separate |
| Hosted apply/restore/V1–V4 | Disabled; no mutating methods or locks in this adapter | All contract, journal, coordination, restoration and V1–V4 evidence |

No real-server T13/T14/T16 certification is claimed. Step 2's historical "Docker socket
unavailable" text is not a current blocker: later OrbStack availability was already
recorded. This step did not inspect or start it again. Exact provenance, registered
disposable target/principal permissions and fault/limit/coordination facilities remain
pending. No hosted read target was named/authorized in this step.

The complete router build passed in 11m 23s and the final check after the credential-binding
edit passed in 2m 03s. Existing warnings are two unused imports in `analytics_report.rs`,
the pre-existing `renderer::compiled_feature_keys()` qualification in the Step 3 boundary,
and `num-bigint-dig 0.8.4` future compatibility. They were not corrected incidentally.
Final checks: formatting/whitespace and 41 local documentation links pass; redacted
Gitleaks finds zero findings in the scoped 20-file set (including the unchanged adjacent
foundation test). H/W/U baseline file and index/revision checks pass. Owned cleanup is
recorded separately in evidence.json. None of these checks certifies a server.
