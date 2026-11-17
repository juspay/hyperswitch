# Step 3 checks and acceptance register

All commands were inspected for effects before execution. Cargo commands are offline; no credential fetch, network client, uploader, server, database, container or workspace mutation is part of these checks. Source edits are scoped local files; formatting targets only edited Rust files with `skip_children=true`.

## Disposable registration

The filesystem fixture `required-fields-step3-l4gbslgr` was registered before builds with owner `ef8eb441-10b2-4aba-a31a-dcf0d69c83dd` in `owner.json`. It contains Cargo output, an isolated manifest/lock, logs and a before-edit hash inventory of H/W/U. No application or Superposition target was registered. Only owned fixture resources may be cleaned up.

The pure-artifact check points `[lib].path` directly at the production `required_fields_renderer/artifacts.rs` and executes its committed tests. It neither copies nor substitutes implementations. Its isolated dependencies match H's locked registry versions/checksums: serde 1.0.229 (manifest minimum is 1.0.219), serde_json 1.0.140 with `preserve_order`, sha2 0.10.9, toml 0.8.22, error-stack 0.4.1, thiserror 1.0.69. The initial attempt pinned serde's manifest minimum and refused offline resolution; it was corrected to the actual lock. This isolated check is **not** compilation/execution of RequiredFields, router Settings or the full workspace.

## Executed checks

Results below are specific to this local checkout; missing compilation prerequisites do not count as passing tests.

| Command / inspection | Result |
|---|---|
| Initial `cargo check --offline --locked -p router --features release` | Before implementation: blocked by uncached `aws-sdk-kms v1.111.0`; no download attempted |
| `cargo test --offline --locked -p payment_methods --features v1,required_fields_sync,storage_impl/default,hyperswitch_interfaces/v1 --lib required_fields_renderer` | **27 pass**, repeated in a fresh process after final source/guard formatting, against the actual package, constructor and API types; includes the 17 artifact tests below, not 27 additional tests. Existing dependency future-compatibility warning for num-bigint-dig 0.8.4. This focused closure is not the router release closure |
| `cargo check --offline --locked -p payment_methods --no-default-features --features v1,payouts,required_fields_sync,storage_impl/default,hyperswitch_interfaces/v1 --lib` | Pass with dummy feature absent; retains payouts and the existing required dependency features |
| `cargo test --offline --locked --manifest-path <owned>/pure-contract/Cargo.toml` | 17 actual production artifact tests pass: canonical byte/hash vector, nested map order, ordered duplicates, strict nested TOML, refusal state, exclusions, countries, key ownership, build/SDK/policy identity, compound context serialization, sanitized errors |
| `cargo clippy --offline --locked --manifest-path <owned>/pure-contract/Cargo.toml --all-targets -- -D warnings` | Pass |
| Targeted `rustup run nightly rustfmt --edition 2021 --config skip_children=true <changed Rust files>` | Pass; does not traverse unrelated modules |
| `cargo metadata --offline --locked --format-version 1 --no-deps` | Pass; opt-in dependency/feature edges inspected, not build evidence |
| Python lock comparison to `HEAD:Cargo.lock` and isolated lock | Package identities/checksums unchanged; only payment_methods dependency edges to existing sha2 0.10.9/toml 0.8.22 added; isolated registry identities match H |
| Source audit of FieldType enum versus exhaustive extraction match | All 56 current variants classified; nine option-bearing variants. Source evidence only, not a future-variant compile-test pass |
| `rustc --edition=2021 crates/{payment_methods,router}/build.rs -o <owned>/<case>` with explicit `--cfg feature=...` cases | Both actual build scripts compile with feature off and with v1+sync; missing-v1, v2+sync and v1+v2+sync each refuse with the expected unsupported-build diagnostic. **10 expected outcomes pass**; running the v1 scripts in a cleared synthetic build environment confirms sorted embedded keys. This is a focused build-script check, not full v2 acceptance |

The initial minimal payment_methods command (`--features v1,required_fields_sync`) and its no-default-features variant fail because this existing package does not select a Redis backend. Adding only `-p redis_interface` with `redis_interface/redis-rs` exposes further existing feature requirements (`hyperswitch_interfaces/v1`, storage accounts_cache/olap/oltp). These are **command/feature-closure corrections**, not new feature regressions or missing-server evidence. The successful command above selects those existing dependency features explicitly; no unrelated feature conventions were changed to make it pass.

| Full router command (all `--offline --locked -p router`) | Result |
|---|---|
| `cargo test ... --features required_fields_sync --lib required_fields_sync` | Blocked in existing superposition_core 0.119.0 build script; router foundation tests did not run |
| `cargo check ... --features release` | Blocked by uncached aws-sdk-kms 1.111.0, matching the pre-change baseline |
| `cargo check ... --features release,required_fields_sync` | Same uncached aws-sdk-kms blocker |
| `cargo check ... --no-default-features --features v2,redis-rs` | Blocked in superposition_core build script; ordinary v2 acceptance pending |
| `cargo check ... --no-default-features --features v2,redis-rs,required_fields_sync` | Initial and final attempts both blocked in superposition_core before this package's guards. Their nonzero exits are **not** successful full-package feature-rejection evidence. Direct actual-build-script guard checks above pass separately |
| `cargo check ... --no-default-features --features v1,fred,required_fields_sync` | Blocked by uncached fred 9.0.3 |

The superposition_core build script invokes cbindgen, which runs nested Cargo metadata against that cached crate's own lock. It requests `icu_locale_core 2.4.0`, absent from the offline index (available candidates include 2.3.0/2.2.0/2.1.1). Its header output would be outside CARGO_TARGET_DIR under the Cargo cache; the observed metadata failure prevents that generation. No dependency source/lock edits or `DOCS_RS` bypass were used. This dependency acceptance remains pending.

## Acceptance boundaries

| Gate | Evidence / remaining requirement |
|---|---|
| T02 foundation | Actual constructor, all nine option-bearing variants, serialized equality distinction, prefill removal and source-enum tests pass. The exhaustive 56-variant match compiles; a deliberate future-variant mutation/negative compile probe was not run. Presentation is retained only as input evidence; no renderer exists yet. |
| T03 foundation | Pure map canonicalization/known digest/ordered duplicate tests and actual repeated constructor HashMap/HashSet normalization tests pass in two processes. EPS provenance mismatch and duplicate options refuse; non-EPS bank vectors retain order/duplicates. |
| T04 foundation | Feature/contract identity and actual effective bank changes, missing/empty distinction, unrelated-settings invariance tests pass. Compiled dummy feature keys match cfg. Required-empty-select refusal and dummy retirement policy await renderer/planner. |
| T25 foundation | No sync dispatcher or network path exists; reports only permit disabled apply. Source reuse of existing Settings precedence is explicit. Actual subprocess Settings precedence test requires the registered fixture and compiled router test binary. |
| T33 | Focused actual payment_methods v1 build/tests, no-dummy check and direct build-script guards pass. Rust 1.97.1 installed; required MSRV 1.94.1 is not installed and was not run. Full router v1/release, sync-v1, ordinary v2, negative sync/v2 and backend acceptance remain blocked as above. No-dummy router v1 is not a current supported closure. |
| Hosted acceptance | No target contract, journal, coordinator, restoration, rollout or V1–V4 evidence supplied by this step. Hosted apply stays disabled. |

## Reproduction inputs

Register a fresh owned filesystem directory before running. Set `CARGO_TARGET_DIR` to its `target` child, `CARGO_NET_OFFLINE=true`, and avoid inherited remote compiler-cache wrappers. Do not run the existing repository uploader or live test defaults. For full settings precedence, set `RF_FOUNDATION_FIXTURE_ROOT` to that registered root and use `cargo test --offline --locked -p router --features required_fields_sync --lib required_fields_inputs_settings_precedence -- --ignored`; its children clear the environment and read only owned synthetic configuration.

The isolated manifest uses an empty `[workspace]`, `[lib].path` pointing at the production artifact module, and the exact versions listed above. Seed its temporary lock from H's lock, allow Cargo to prune that **temporary** lock offline, then require `--locked` for subsequent tests/Clippy. Verify every resolved registry identity/checksum against H. It is intentionally a fallback for pure-contract evidence, not a replacement for workspace acceptance.

## Final review and cleanup

- Targeted nightly `rustfmt --check` passed for every changed Rust file. `git diff --check` and new-file `git diff --no-index --check /dev/null <file>` passed. Changed-document local links resolve.
- Before/after file hashes identify exactly the 22 H files listed in the [handoff](README.md). Every other pre-existing H file is unchanged, including original roadmap/plan, seed, required-field business source and prior harnesses. W/U file hashes are unchanged. H/W branches and U's detached pin, all three HEADs and Git indexes are preserved; nothing staged or committed.
- The registered fixture was removed after verifying its exact owner marker and completed builds. A follow-up filesystem check confirms both the owned root and its temporary pointer are absent. No repository, shared Cargo cache, container or unrelated resource was cleaned up.
- Full router settings-precedence/report tests, ordinary serving startup, MSRV 1.94.1, full release/backend/v2 acceptance and a future-FieldType mutation compile probe remain unexecuted or blocked as specified above. No hosted target or server certification was attempted.
