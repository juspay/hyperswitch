# Step 11 checks and acceptance gates

All commands were inspected for side effects before execution. No uploader, hosted credential, live payment submission, live configuration write, Docker/cluster operation, Git staging/history/branch change or remote PR action ran. Existing Steps 8–10 and W Step 1 work were preserved.

## Isolation

Owned root: `/private/var/folders/3r/5h91zf1d2515z0phtg4mdsv40000gp/T/required-fields-step11-3_ea8njm`; owner `7cc8ac85-1ccb-420e-beed-b62c332ce188`; purpose `Step 11 isolated verification checks`; canonical root equals cleanup scope, hosted target null and server mutations false. Baseline, starting-source copies and command logs remain there. The browser child has its own marker, pinned archive copies and `parent_step: 11`; the existing Step 1 fixture helper's owner-name namespace is retained only for compatibility.

W/U were copied from exact local Git objects without hooks, fetch, branch or submodule updates. Dotenv/npm configuration files were excluded before archive content reads. Node dependencies were installed only in that disposable copy using locked `npm ci --ignore-scripts`, empty user/global configs and an owned cache. Initial OS-network-denied/offline installs failed ENOTCACHED. Subsequent public npm registry downloads and the pinned Playwright Chromium-headless-shell download passed; these were development-tool downloads, not hosted application access. No normal postinstall/start/live Playwright command ran.

Cargo uses an owned copy of registry/git caches without configuration or credentials, separate target directories, two jobs, no incremental/debug output and cleared compiler wrappers. This contains the known cbindgen relative-output side effect. Rust tests/builds deny all networking at the OS boundary, except the explicitly selected read-only HTTP test, whose policy allows only loopback. Browser executions likewise allow only owned loopback origins. Provider fixtures are registered children containing synthetic files; HTTP listeners are recorded before a read and closed afterward. Cleanup targets only verified owned children.

## Executed commands and results

Exact argv, exit status, timing and log hashes are in [evidence.json](evidence.json); failed intermediate runs are retained separately.

```sh
cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib
cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib plan::tests::verification_tests -- --ignored --nocapture
cargo +stable test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib verification_http::tests::bounded_registered_reads -- --ignored --nocapture
cargo +stable test --offline --locked --manifest-path docs/required_fields_sync_contracts/sdk-v1/rust-probe/Cargo.toml --lib
cargo +stable clippy --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib --tests -- -D warnings
cargo +stable check --offline --locked -p router --features required_fields_sync --lib
cargo +stable clippy --offline --locked -p router --features required_fields_sync --lib
```

- Ordinary companion suite: **131 passed, 0 failed, 11 ignored**. New ordinary coverage includes all-level orchestration, 22 parameterized missing/stale/binding/roster/backup faults, export-to-row metadata/ordered-value checks, exact registered read request shapes and actual locked SDK export serialization. These are synthetic/serialization tests, not server certification.
- Explicit provider and independent browser-receipt selection: **2 passed**. Actual JSON/TOML fallback parser/new startup, wrong/corrupt/missing input, available-primary refusal, retained memory after primary/backup disappearance, and independent reading of the final W receipt with binding/skip/promotion refusals.
- Owned loopback transport selection: **1 passed**, nine response shapes. `POST /config` is the locked read operation with fixed `{}`; redirects, partial status, wrong media/encoding, oversized/truncated content, duplicate JSON and server error refuse. Both router SDK routes are GET. No Superposition state exists in this fixture.
- Independent actual Rust resolver: **14 passed**. JS/Rust missing-default divergence remains intentional; browser execution is not inferred from Rust.
- Strict isolated Clippy and actual opt-in router build/Clippy are recorded in the machine record. Existing analytics imports, renderer qualification, Step 6 conversion warnings, the isolated `.clippy.toml` unreachable UUID-method warning and dependency future-compatibility notice remain distinguished from new code. No unrelated `cargo fix` ran.

The two explicit selections require `RF_VERIFICATION_FIXTURE_ROOT`. The report reader also requires its `browser-execution.json` with owned input/registration/report paths. Missing evidence fails. The HTTP selection additionally needs the loopback-only OS policy. The eleven default ignored cases remain visible; only these three Step 11 ignored cases were separately executed here. Prior storage/exporter/coordination/CLI ignored tests were not silently counted as rerun.

In the pinned disposable W copy:

```sh
npm ci --ignore-scripts --no-audit --no-fund --update-notifier=false --registry=https://registry.npmjs.org --userconfig=<owned-empty> --globalconfig=<owned-empty> --cache=<owned-cache>
node playwright-tests/node_modules/@playwright/test/cli.js install chromium --only-shell
npm run re:build --ignore-scripts
node node_modules/webpack-cli/bin/cli.js --config playwright-tests/required-fields-sync/webpack.config.cjs
node --test playwright-tests/required-fields-sync/contract.test.cjs playwright-tests/required-fields-verifier/contract.test.cjs
node playwright-tests/node_modules/typescript/bin/tsc -p playwright-tests/required-fields-verifier/tsconfig.json --noEmit
node playwright-tests/required-fields-verifier/run.cjs
```

- Locked installation ran separately in root SDK, playwright-tests and demo directories. ReScript, component webpack, actual SDK/demo webpack and scoped TypeScript checks passed.
- Node contract/guard tests: **27 passed** (22 existing plus five new bounded-envelope/credential-environment tests).
- Final companion: **38 Step 1 component cases, 5 Step 1 complete-checkout cases, 14 actual JS resolver cases, 7 additional response-bound cases passed**. It reuses the original case/report contract; component, JS and complete checkout remain separate nested reports. Assets include emitted SDK/demo/component files, companion source, pinned provenance, Chromium runtime files and the Node executable hash. Final integrity checks precede publication of the passing receipt.
- New cases execute each exact before/after/post-apply-phase response through actual constructor/aggregation/rendering and full checkout validation plus intercepted confirm; retained snapshot stays old while reload receives the new response. Inputs are synthetic. All confirmation requests are aborted before the fixture router too. No payment is submitted.
- The actual child sentinel test proves credential environment variables and NODE_OPTIONS/HOME are not inherited. It does not certify deployed filesystem/process isolation or container mounts. The final receipt explicitly marks platform credential isolation pending.

The byte-exact final report and registration remain in the owned evidence directory. Their digests and explicitly labeled sanitized indexes, plus the unchanged synthetic input, are recorded as [browser-report.json](browser-report.json), [browser-input.json](browser-input.json) and [browser-artifacts.json](browser-artifacts.json). The indexes are deliberately not accepted as executable verifier receipts; the independent H check uses the original report. They contain synthetic data and hashes, no writer credential or live payment data. Seven initial generic-api-key scan findings were SHA-256 entries under API/authentication file names. Their values were checked against the originals; typed file-hash indexes avoid misleading key/value presentation without changing the actual runtime report contract or adding scanner exclusions.

## Failures found and corrected

- Offline npm packages were unavailable; recorded ENOTCACHED attempts preceded isolated public locked installation.
- Initial local build driver used the old ReScript executable path. A subsequently named `rescript.json` command log was interpreted as an ancestor ReScript config; logs were renamed. Pinned source/build configuration was not changed.
- Final hash review found the isolated JSON insertion-order feature could mask router hash differences. Step 11 now sorts object members explicitly for input/provenance/derived-pack hashes, preserving arrays and raw response/request bytes. A separate owned build without `serde_json/preserve_order` checks the actual companion receipt; its feature graph and unchanged H dependency identities are recorded.
- Initial standalone Cargo resolution selected newer transitives. The graph was reconciled from H Cargo.lock, then locked checks reran; all retained registry package identity/checksum pairs match H. Preliminary mismatched-graph results are not evidence for the final build.
- Compile checks caught Digest ordering/access and reference-comparison mistakes; focused checks caught raw backup JSON versus the actual markup format. Clippy caught raw process/clock fixture naming, indexing and a numeric cast; bounded owned directory slots and checked access/conversion fixed these without changing shared helpers or unrelated warnings.
- Actual SDK serialization caught an incorrect GET assumption for `/config`; adapter, fixture registration and request tests now use the locked POST read contract. No production request was dispatched before this correction.
- One Step 1 component `page.goto` timed out before assertions during the combined run; the runner refused and published no final report. Later complete executions with retries still disabled passed. The failed log is retained; this is not a deployment latency/reliability certificate.

## Pending / not executed

- **V1/V2 target certification:** exact server/image/permission/cache contracts; actual full inventory/export comparison under exclusion; authenticated synthetic payment/profile provisioning; all traffic-eligible pod/co-reader discovery, direct endpoint identity and real content convergence. Source/library and local HTTP tests do not certify these.
- **V3 operational/candidate coverage:** real post-commit responses, Step 6 arbitrary candidate matrix conversion, every supported method/profile/prefill/asset combination, old/mobile/deployed readers, Linux companion image and runtime/mount/credential-access certification. The current bound full-checkout driver is explicitly limited; no successful placeholder or Rust approximation fills missing cases.
- **V4 operational evidence:** selected backup implementation/image, target/version/content/publication provenance, atomicity/mount/access, real publication deadline, old-reader startup and protected retention. No backup artifact was fetched from a hosted store.
- **Build/platform:** declared Rust 1.94.1 is not installed; no MSRV download, release image, cross-target or broader feature/workspace suite ran. The actual default-v1 opt-in library build is the supported local check executed here.
- **Earlier/later gates:** server contracts, real storage/exclusion controls, business/source approvals, Step 12 restore/rollback and subsequent mutating CLI/deployment/adoption remain open. No seed, source option/path correction or SDK behavior change was bundled.

Formatting, dependency/archive/source bindings, scoped default-rule secret scanning, document links, incremental diff review and worktree/index preservation are recorded in the final machine record. A successful source scan does not attest prior commits/history or resolve an earlier secret incident. Stop for review; Step 12 is not begun.
