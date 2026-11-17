# Step 4 checks and acceptance gates

All commands were inspected before execution. Builds use `--offline --locked`, an owned `CARGO_TARGET_DIR`, `CARGO_NET_OFFLINE=true`, two build jobs, no incremental/debug data and no inherited `RUSTC_WRAPPER`. No uploader, secret loader, application client, container, hosted workspace or payment request was used.

The filesystem fixture `required-fields-step4-yahd_k5q` was registered before builds with owner `66879130-d2a9-4a16-9dec-8f0effd6d37b` and purpose `Step 4 offline synthetic renderer/build checks`. It contains build output, synthetic inputs, source inventories and logs only. Export/negative-probe helpers require its ownership-purpose marker; outputs use new files/directories. This is not a Superposition target registration.

## Executed checks

In these commands, `F` means exactly `v1,required_fields_sync,storage_impl/default,hyperswitch_interfaces/v1`. Default payment_methods features include dummy connectors and payouts. This is the supported focused package closure established in Step 3, not a router release build.

| Command | Result |
|---|---|
| `cargo test --offline --locked -p payment_methods --features "$F" --lib required_fields_renderer` | **49 pass, 1 opt-in export ignored**, including existing foundation tests; final run against actual package/constructor/API enums |
| Same test binary, `--exact configs::required_fields_renderer::render_tests::export_synthetic_renderer_bundle --ignored`, with registered `RF_RENDER_FIXTURE_ROOT` | Pass in fresh processes. Four bundles (inheritance, ordered duplicates/Unicode, empty contribution, compiled one-rule EPS projection) identical across **four** processes, including two from the final test binary |
| `node docs/required_fields_sync_contracts/renderer-v2/check-resolver.cjs "$W" "$OWNED" "$EXPORT_PID"` | **7 checks pass** through actual pinned U CacReader; W helper verifies source hashes. Explicit supplied context order, not server-order certification |
| `python3 docs/required_fields_sync_contracts/renderer-v2/check-exhaustiveness.py "$H" "$OWNED"` | Copied actual FieldType enum and extraction function compile; adding one future variant fails with **E0004**. Narrow negative compile probe, separate from workspace build evidence |
| `cargo build --offline --locked -p payment_methods --features "$F" --example required_fields_render_profile` | Pass; explicit offline full-profile runner |
| Example binary with `PROFILE.toml RELEASE.toml UNUSED_OUTPUT.json` | EPS AB, EPS A and absent-EPS profiles: expected **exit 2**, bounded blocked reports, distinct settings/retained-input hashes. Empty-string EPS: expected **exit 1**, no report; existing settings parser refusal. Reusing an output path: expected exit 1, prior bytes preserved |
| `cargo check --offline --locked -p payment_methods --no-default-features --features v1,payouts,required_fields_sync,storage_impl/default,hyperswitch_interfaces/v1 --lib` | Pass, actual no-dummy compilation |
| `cargo clippy --offline --locked -p payment_methods --features "$F" --lib --tests --example required_fields_render_profile` | Pass; no new-code warnings. Existing common_utils keymanager warning is outside this change |
| `cargo check --offline --locked -p payment_methods --features v1,storage_impl/default,hyperswitch_interfaces/v1 --lib` | Pass; sync feature disabled |

Initial focused compilation took about 13 minutes in a fresh target. The existing `num-bigint-dig 0.8.4` future-compatibility warning remains. No dependency or business-source changes were made to clear checks. A formatting-time delimiter error and new Clippy style diagnostics were corrected before final validation; no failed check is represented as passed.

All 56 FieldType variants are exercised. Exhaustive method/type/connector matches compile; the 16 methods and 126 v1 types are checked against an independent translation of the inspected U casing algorithm. ClassicReward is explicitly unsupported. This translation test is **source-contract evidence**, not execution of compiled CommonUtils. The seven CacReader checks are actual JS resolver execution. Step 1's 14 Rust resolver, 38 component and five checkout passes remain historical; they were not rerun or counted as new renderer/browser evidence.

## Profile reproduction

Copy [profile-pending.toml](profile-pending.toml) into a newly registered owned directory. Vary only its explicit profile ID and bank settings: `abn_amro,bank_austria`; `bank_austria`; an empty `[bank_config]` table; or `banks = ""` for the parser-refusal case. The renderer never reads a developer/production settings file or environment override.

Supply a separate bounded TOML identity:

```toml
[release]
application_revision = "<actual H revision>"
source_tree_hash = "sha256:<actual local source-list digest>"
renderer_implementation = "step4-renderer-v2-offline"
[release.artifact]
kind = "offline"
```

For executed profiles, the source digest is SHA-256 over compact canonical JSON of sorted `{path,sha256}` entries from Git-visible H `crates/`, Cargo manifests/lock, `.cargo/` and toolchain inputs, including local source edits. The report retains the supplied identity and actual package feature keys; this local caller-supplied digest is not release/image attestation. The retained rules/settings/policy/catalog/exclusion hashes bind effective inputs independently. Full inputs and logs are disposable; [results.json](results.json) retains sanitized identities/counts and the exact final commands.

## Gates still open

| Gate | Evidence and limitation |
|---|---|
| T01–T04 pure rendering | Exact independent conditions/keys/values; all variants; future-field negative probe; prefill exclusion; deterministic maps/EPS sets; ordered vectors; bank-profile differences; dummy inclusion/exclusion and explicit non-retirement pass. Full supported-profile approval is not satisfied |
| T05–T06 mappings/options | Scoped aliases, actual source scopes, lowercase bank values, country catalog/order/ALL, AC–TA refusal and malformed/empty/state cases pass synthetically. Owner decisions, full certified asset catalog and separate source/presentation fixes remain open |
| T07–T09 resolution | Exact rendered inheritance matches pinned JS, including default-equal overrides, baseline true after contribution removal, exact casing, and JS override-only keys. New Rust-reference/full-browser consumption of rendered bundles, live metadata, all reader contexts/connector orders and target projection remain separate gates |
| T10 | **Not certified**: exact-server ordered-array identity/export behavior requires Step 2's real disposable server contracts; local order preservation is not that evidence |
| Full supported profile | **Blocked**: no approved full mapping/catalog. Three malformed comma-country occurrences remain. No automatic exclusion, seed equality or synthetic success substitutes for this gate |
| T33 broader matrix | Focused PM builds only. MSRV 1.94.1 is not installed; installed stable is 1.97.1. Full router settings/release/v2/backend checks were not rerun here. Step 3's aws-sdk-kms/fred/cbindgen offline blockers are historical, not a fresh claim about current caches |
| Hosted apply | Disabled. Contracts, journal, writer coordination/exclusion, restoration and target V1–V4 are still required |

Final formatting/diff/link, redacted secret-scan, working-tree preservation and owned cleanup results are recorded in results.json. No Git index/history/branch operation is part of these checks.
