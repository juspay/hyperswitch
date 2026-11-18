# Step 7 checks and acceptance

Commands were inspected for side effects before running. Validation uses offline/locked
Cargo with registered owned Cargo home/target directories, two jobs and debug/incremental
output disabled. Only source/index/archive/Git dependency caches were copied; ambient
Cargo config/credentials were not. This isolates the existing core cbindgen build-script
write outside `CARGO_TARGET_DIR`. The root dependency manifest/lock remain unchanged.

Fixture `required-fields-step7-isqz22kf`, owner
`e5bda0db-4a7c-42ea-a643-1e3581ffc767`, was registered for
`Step 7 offline CLI checks` before writes. Its marker binds root and cleanup scope.
Exporters and command tests refuse to run without this marker. Tests create owned new
files/directories and never clean another fixture. No Superposition server/workspace,
container, uploader, hosted target, credentials, mutation or lease is used.

After all checks finished, the owned Cargo home, both target directories and scan copy
were removed after marker verification. The final executable (about 1.49 GB) is retained
at the fixture's `review-binary/router`, together with synthetic profiles/reports and logs.
Its digest matches the 31-case report and its dynamic libraries do not reference the
removed caches. Shared caches, repositories and other fixtures were not cleaned.

## Executed commands

Final results, durations, hashes and preservation checks are recorded in
[evidence.json](evidence.json). Do not infer an unlisted build from another successful one.

Executed focused tests: **75 pass / 5 opt-in ignored** in the ordinary isolated suite,
**4 pass** when the registered command tests are selected, **2 pass** with the parser
feature disabled, **49 pass / 2 opt-in ignored** in the actual payment-method renderer
suite, **1 pass** for the compiled CLI-profile exporter, and **31 actual-binary cases
pass** under verified OS network denial. Four actual build-script invalid-feature
combinations fail with the expected guard message. The unexcluded compiled CLI profile
still refuses **1,625 missing bindings and three malformed-option occurrences**.

The opt-in default-v1 router binary built successfully. Its final source warnings are
the three preserved earlier warnings (two analytics imports and the renderer qualification).
The macOS linker also reports a large unwind-section warning; `num-bigint-dig` retains
its existing future-compatibility notice. Three new command-adapter qualification warnings
were removed before the final binary build. Timings include dependency compilation; some
also include waiting on another command's build-directory lock, not just compiler time.

The first 28-case binary run passed before the bank-input review. Three additional cases
now cover distinct absent/empty/populated hashes and unknown-bank refusal; the final
31-case run passed against the rebuilt binary. Source review found the CSV empty-set
limitation, so named seeds use existing typed bank configuration instead. No Settings
parser or source-data correction was made. Full-router linting also prompted checked
file-cap conversions and boxing the extracted serving future; their focused tests/build
were rerun. The two conversion warnings in pre-existing Step 6 `preflight.rs` remain
separate from this step, alongside the earlier imports/qualification warnings.

| Check | Scope |
|---|---|
| `cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib` | Actual foundation/inventory/planner/parser sources. Opt-in filesystem exporters/tests stay ignored in the ordinary run. |
| Same with `command_tests -- --ignored` and `RF_CLI_FIXTURE_ROOT` | Synthetic successful/failed plan orchestration; zero mutation/lease surface; redaction, disabled/unregistered/mismatched targets, input bounds, duplicate JSON, output preservation. These backend results are not actual compiled-render or server evidence. |
| Same with `--no-default-features cli::tests` | Real argument types with the feature absent; original serving/scheduler flags retained, new family rejected. |
| `cargo clippy --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib --tests -- -D warnings` | Initial process-ID fixture call violated replay convention; replaced with create-new directory allocation and rerun. No lint suppression was added for that finding. |
| `cargo build --offline --locked -p router --features required_fields_sync --bin router` | Actual default-v1 opt-in binary, not the isolated crate. |
| `cargo check --offline --locked -p router --bin router` | Pass for ordinary default-v1 serving build with the opt-in feature absent; the two earlier analytics warnings remain. |
| `cargo clippy --offline --locked -p router --features required_fields_sync --lib --bin router` | Actual library/runtime/binary adapter lint check; final result and any preserved warnings are recorded in evidence.json. |
| `cargo test --offline --locked -p payment_methods --features v1,required_fields_sync,storage_impl/default,hyperswitch_interfaces/v1 --lib required_fields_renderer` | Existing supported focused closure, retaining package defaults. |
| Same closure with `export_router_cli_profiles -- --ignored`, registered fixture environment | Actual compiled RequiredFields/settings/catalog profiles; every source explicitly excluded in successful plumbing cases, separately unexcluded refusal profile. Does not approve any business profile. |
| `python3 docs/required_fields_sync_contracts/cli-v1/test_cli.py --root <owned> --router <owned actual binary>` | OS-enforced network denial checked before invocation; clean environment; unavailable DB/Redis and malformed unrelated DB settings, seed pollution, effective hash drift, explicit paths, closed registration, private output, redaction and rejected flags/commands. |
| `rustc --edition=2021 --cfg 'feature="required_fields_sync"' … crates/{router,payment_methods}/build.rs -o <owned>` | Actual build-script compile guards: missing-v1 and v1+v2 must fail with the documented message. Not ordinary-v2 or full rejected Cargo graph acceptance. |

The companion adds exact H-locked Clap 4.5.38 to compile the real argument definitions.
Its lock was regenerated from H's lock by one initial offline check without `--locked`;
all subsequent validation is locked. All 168 external tuples/checksums match H. This is
not a dependency upgrade. Whole-workspace formatting/testing is not
claimed; targeted formatting, Python syntax, diff/link/JSON checks and scoped redacted
default-rule secret scanning are separate final checks.

## Gates

| Gate | Executable local evidence | Remaining acceptance |
|---|---|---|
| T25 dispatch/security | Parser/orchestration and actual-binary checks recorded separately; no backend write/lease method; explicit profiles and closed production registry | Authorized exact-target complete-read permission and endpoint contracts; real provisional plan requires certified registration, catalog and grants |
| T25 service independence | Actual compiled fixtures plus network-denied command run, when marked passed in evidence | No ordinary payment server startup/DB integration is claimed |
| T33 feature boundary | Feature-off parser, actual supported binary build and exact build-script guard results | MSRV 1.94.1, release/image attestation, dummy/no-dummy, TLS/Redis variants and ordinary v2 matrix are not inferred |
| Renderer/planner regression | Focused real-source suites; Step 6 pure rules retained | Full compiled profile approval, exact server identity/ordering and new candidate-specific authenticated browser receipts remain open |
| Hosted apply | No mutating command or placeholder success | All contracts, journal, coordination, restoration and V1–V4 still required |

Historical Step 1 browser and Step 6 actual JS/Rust resolver comparisons are not rerun by
these CLI checks. They do not become new candidate browser receipts. Missing external
dependencies block those acceptance gates, not the local command implementation.

In particular, no production CLI run reached reader-token loading, HTTP inventory or a
successful real-target plan: empty certified registries intentionally make that impossible.
Those positive-path checks require the exact disposable server and permission contract;
the synthetic backend cannot close them. Ordinary serving was checked through parser,
source-preservation and compilation evidence, not by starting a payment service.
