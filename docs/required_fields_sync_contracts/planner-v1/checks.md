# Step 6 checks and acceptance gates

Commands and build-script side effects were inspected before execution. This step used
synthetic in-memory inventory and one registered disposable filesystem root, never an
API/workspace/server. No uploader, secrets, AppState, hosted reads, writes or leases ran.

Fixture `required-fields-step6-2kaln9rq`, owner
`82fc53ca-dd61-4e52-b376-de92efc49682`, was registered for
`Step 6 offline synthetic planner checks` before writes. Its ownership marker binds the
exact root and cleanup scope. Exporters require that marker and create-new files; only
owned paths may be removed. Final preservation/cleanup is recorded in [evidence.json](evidence.json).

## Commands and executed results

Cargo ran offline with owned target directories, two jobs, disabled incremental/debug
output and cleared compiler wrappers. No dependency/lock update was needed. The router
and core probe additionally use an owned Cargo home copied from source/index/archive/Git
caches only, without ambient config/credentials: `superposition_core`'s cbindgen build
script otherwise writes outside `CARGO_TARGET_DIR`. Stable Rust/Cargo 1.97.1 was used;
declared MSRV 1.94.1 is not installed and was not tested.

| Command/check | Executed result |
|---|---|
| `cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib` | Final **73 pass, 0 fail, 1 opt-in exporter ignored**: 47 earlier foundation/inventory/locked-client checks plus 26 planner checks. Synthetic receipt fixtures are explicitly not real execution evidence. |
| Same command with `plan::tests::export_synthetic_planner_cases -- --exact --ignored`, `RF_STEP6_FIXTURE_ROOT=<registered root>` | **1 pass** separately; five synthetic profiles exported with bounded requests and separate JS/Rust expectations. Final re-export/request-identity comparison is in evidence.json. |
| `cargo check --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib` | Pass, including the production empty-registry boundary. |
| `cargo clippy --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib --tests -- -D warnings` | Final pass. Earlier three findings (two argument-count lints and a redundant `Ok`/`?`) were fixed, not suppressed. |
| `cargo test --offline --locked --manifest-path <owned Step 1 rust-probe copy>/Cargo.toml` | **14 existing actual-core cases + 1 new actual-core probe pass**. `core-check.rs` is copied to its `tests/planner.rs`; the original probe and lock remain unchanged. It records actual core results for all new partitions and asserts three identity properties. |
| `node docs/required_fields_sync_contracts/planner-v1/check-resolvers.cjs <registered root> <W>` | **396 actual pinned JS + 396 actual Rust resolver observations agree with their distinct planner reference hashes** across five profiles. Independent literal assertions include baseline true, missing defaults, option order and explicit `[A,B]` mandate options inheriting `[A]` only after removal. No full browser or trusted receipt is claimed. |
| `cargo check --offline --locked -p router --features required_fields_sync --lib` | Initial run exposed two new type-inference errors under the full rkyv dependency graph; fixed with explicit `BTreeSet` collection and `&str` comparison. Final **pass in 2m 18s** with three pre-existing warnings (two analytics imports and the earlier renderer qualification), plus an existing num-bigint-dig future-compatibility notice. Timings are in evidence.json. |
| Companion registry dependency tuple/checksum comparison against H `Cargo.lock` | **162 exact matches; zero mismatches**. Root/companion locks and manifests are unchanged. The separate core probe retains Step 1's lock; it does not replace H's dependency graph. |
| Targeted `rustfmt +nightly --edition 2021 --config skip_children=true --check …`, `node --check …`, `git diff --check`, new-file whitespace/link/JSON review, scoped redacted Gitleaks, baseline/index comparison | Final recorded results in evidence.json. Whole-workspace formatting and unrelated tests are not claimed. |

An initial isolated check found a `Digest` ordering-bound error; the local receipt set
and browser dedup key representation were corrected. Successful later checks supersede
that failure. The first full router check also found three newly introduced unnecessary
qualifications, which were removed. Pre-existing analytics imports and the earlier
`compiled_feature_keys` qualification are preserved. No check is replaced by a stub,
`DOCS_RS` bypass or force flag.

## Coverage matrix

| Gate | Local evidence | Still required |
|---|---|---|
| T07–T09 inheritance and resolver scope | Explicit default-equal values, common/mandate counterexample, baseline true after removal, country/tenant/collection partitions, all declared connector permutations, missing/default-only Rust–JS divergence; actual pinned resolver comparisons | Contract-bound full browser constructor, visibility, prefill/validation/payload and deployment reader cases for the new projected configurations |
| T10 ordered identity | Before/after and cross-row collision refusal; actual core 0.119 hash confirms array reorder collision, separator collision, and duplicate-count distinction | Exact server image/source and ID/export collision behavior; no source/core result certifies server storage |
| T11 ownership | No marker/broad-read ownership, stale/duplicate/target-mismatched grants, shared/fully managed/unrelated/excluded classifications, feature absence refusal, durable create-intent identity and future manual collision | Real reviewed adoption/retirement/feature handoffs, journal confirmation of new IDs |
| T12 preservation and second-run invariant | Complete shared replacements and six independent nested unrelated-value variations; exact descriptions/unknown metadata; large-number/raw lexeme preservation; second projected inventory needs no action after new exact grant | Actual apply/restore and server-maintained audit/write behavior, later coordinated drift policy |
| T13–T14 identity/discovery | Step 5's complete read/permission/pagination/consistency suite rerun; planner accepts only its sealed complete type; unsupported predicates, returned IDs, version exports, missing override references and identity collisions refuse | Exact-server full scope, stable discovery and concurrent/fault contracts |
| T15 prerequisites | Valid catalog cannot replace missing live presentation; exact managed-default creation; typed enum extension; unknown metadata/schema/functions, empty/unusable/malformed options refuse; empty arrays cannot hide unsupported item schemas | Owner-approved live promotion, catalog/options/assets, broader supported constraints/functions if needed |
| T22 experiments | Tests cover retained context/override references, active overlapping variants and missing group/bucket members. The global prerequisite-conflict guard was inspected in source; no separate runtime case for that guard was executed | Actual server experiment rejection/transaction behavior |
| T32 approval/reporting | Deterministic target/input/action/case hashes; exact approval binding/expiry; eight stage/phase receipts; wrong pack/artifact/request/provenance/status/IDs refuse; browser effects separated from resolver counts; raw data omitted from reports | Authenticated execution/approval integration, CLI exit codes and runtime mutation/uncertainty outcomes belong to later steps |
| T35–T37 web/aggregation | Closed descriptor vocabulary; aliases sharing one active confirm path refuse; both connector orders and billing-force reference inputs generated; option case/order/duplicates and malformed choices tested | New actual browser/component/checkout adapter execution, supported constructor/scenario/asset contracts and owner approval. Historical Step 1 browser passes do not fill this gap |

The production read and projection registries are empty. The bound-case receipt adapter
is intentionally unregistered; no successful synthetic receipt escapes the test module.
Exact-server contracts, full supported renderer profiles, business/source corrections,
release/MSRV/feature matrix, journal, coordination, restoration and target V1–V4 remain
open. This is a reviewable local planner implementation, not enablement for hosted apply.
