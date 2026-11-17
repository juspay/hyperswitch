# Step 1 — local browser continuation

2026-11-18. The user requested completion of the blocked local browser work. Locked public npm packages and the matching Playwright Chromium were downloaded into an owned disposable build copy. No hosted application target, production secret, uploader, deployment or payment submission was used. No later step was begun.

**Result:** 22 Node tests, 14 actual Rust resolver tests, 38 browser-component tests and five full-checkout tests pass. ReScript, component webpack, SDK/demo webpack, TypeScript, formatting and report validation pass. Independent owner review and the remaining gates in the [coverage matrix](README.md#source-evidence-and-remaining-gates) are still open. The [previous offline failures](checks.md) remain historical evidence, not current blockers.

## Source evidence and corrections

H/W/U revisions and branches remain those in [README](README.md); U is still exactly W's gitlink. Existing Step 0/1 files were inventoried before changes; no feature, renderer or source correction was introduced. Relevant repository instructions, playbook Step 1 and its referenced plan sections were read. Only affected source/harness evidence was refreshed; unrelated deployment research was not repeated.

The browser failures led to these source-backed corrections to the new tests:

| Finding | Evidence and disposition |
|---|---|
| Country asset fixture used decoded keys on the wire | W `src/Utilities/S3Utils.res:3–29` decodes top-level `country` and row `value` into `countries`/`countryName`. Corrected synthetic input, retaining independently specified decoded expectations. Initial component run: 31 passed, two failed; corrected run: 33 passed; expanded final run: 38 passed. |
| Config-fetch failure does not leave an ordinary V1 card form | W `src/PaymentElementRenderer.res:50–59` gates the whole form on `LoadError`. Initial checkout run: two passed, one failed waiting for readiness. Now assert HTTP 503, top-level error and no card iframe. This remains distinct from the component-level missing-config/no-dynamic-fields case. |
| Successful malformed asset object does not trigger fallback | The same S3 parser accepts `{}` as empty countries/null states. Added a negative browser case; no SDK repair. Future validation/fallback policy needs review. |
| Tracked archives contain dotenv inputs | The checkout build guard refused their presence before evaluating the normal webpack configuration. Preparation now excludes those two files from the disposable build copy without reading their values. Original files are untouched. Explicit synthetic inputs replace ambient configuration. |
| Full validation must finish before asserting no confirm | The actual demo reports `Please enter valid details` and re-enables submit after invalid card/dynamic-field submission. The test waits for both before asserting no request, then supplies the missing address and asserts the actual confirm's complete `payment_method_data`. This does not certify every envelope field or every payment method. |

Added B cases cover a per-connector evaluator exception, requiredness after contribution removal, inherited options across render-kind changes, EFT lowercase BankType payloads and malformed asset JSON. Only the explicit exception case injects a fault into the actual evaluator; healthy connectors execute the original function. Added E cases exercise real card validation/confirm and mandate option inheritance through actual checkout. All expected values remain test-authored; no renderer/seed generates the oracle.

## Owned resources and boundaries

| Owner ID | Registered disposable resource |
|---|---|
| `required-fields-step1-574ecbcf-8f8b-4e14-9ea9-c862a16eb959` | Temporary root ending `required-fields-step1-i48Ca7`: local W/U archives, fixture-owned npm cache/browser install, Rust probe/target, generated assets and fresh per-run loopback origins |
| `required-fields-step1-5dd984e6-50e7-4c04-9f39-650012d9a145` | Temporary root ending `required-fields-step1-ZKTnnN`: recheck of the updated preparation script, including empty npm configs and excluded dotenv files; no dependencies/services installed |

Both markers specify exact root/web/cleanup paths, `kind=disposable-filesystem-build`, `hosted_target=null`, `server_mutations=false`. Registrations are local filesystem targets, not Superposition workspaces. Test runners bind fresh `127.0.0.1` ports, accept only static GET/HEAD requests and seal/hash every emitted asset. They verify unchanged artifacts after tests and close servers in `finally`. Reports are written exclusively in fresh run directories. No existing service is reused.

The component layer mounts actual W/U components and helpers. The checkout layer builds the actual SDK loader/card frames and demo with pinned webpack entry points/templates/loaders; it uses existing hermetic fixtures. Custom configs avoid normal dev servers, proxies, live Playwright projects, `server.js`, dotenv and optional upload plugins. Child environments contain explicit synthetic inputs. The checkout build disables CSP and demo minification, so it is local behavioral evidence, not a production-build certification. Component CSS is text-loaded and does not certify styling.

Browser external requests are routed to synthetic fixtures or aborted; Node fetch is forbidden in checkout tests and service workers are blocked. Router identity is the reserved `https://required-fields-router.invalid`. Confirm POSTs are aborted before even the synthetic router; its confirm call log remains empty. Test card data is synthetic. No country asset was fetched from a production origin.

## Executed commands

`W` denotes the source repository; `COPY` the registered archive's `web` directory; `ROOT` its owned parent; `PACK` H's `sdk-v1/cases.json`. These are explanatory placeholders, not exported variables. All command scripts/configs and dependency lifecycle hooks were inspected before execution. See W's fixture README for setup commands using the returned paths.

| Command in the indicated directory | Result / side effects |
|---|---|
| Source W: `REQUIRED_FIELDS_CASE_PACK=PACK node playwright-tests/required-fields-sync/prepare-fixture.cjs` | Passed. Local archives at exact W/U revisions, copies additions, verifies source hashes, registers only the created root. Final preparation also creates empty npm user/global configs and excludes copied dotenv inputs. No Git/network mutation. |
| COPY, COPY/playwright-tests, COPY/Hyperswitch-React-Demo-App: `npm ci --ignore-scripts --no-audit --no-fund --update-notifier=false --registry=https://registry.npmjs.org --userconfig=ROOT/npm-user.npmrc --globalconfig=ROOT/npm-global.npmrc --cache=ROOT/npm-cache` | Passed: 814, seven and 574 packages respectively. All three lockfile download hosts inspected as public npm registry. No lifecycle, lockfile update or submodule update; existing dependency deprecation warnings retained. Writes/downloads confined to disposable setup. |
| COPY/playwright-tests: `PLAYWRIGHT_BROWSERS_PATH=ROOT/browsers node node_modules/@playwright/test/cli.js install chromium --only-shell` | Passed. Official browser CDN download of matching Chromium headless shell revision 1243 and ffmpeg 1011 into owned root. |
| COPY: `npm run re:build --ignore-scripts` | Passed with pre-existing deprecation/unused warnings. Compiles actual pinned W/U; only copied build outputs change. |
| COPY: `node playwright-tests/node_modules/typescript/bin/tsc -p playwright-tests/required-fields-sync/tsconfig.json --noEmit` | Passed. Initial pre-compile check failed because existing fixtures import generated `TestUtils.bs.js`; compile-first ordering resolves it. No fake declaration substituted. |
| COPY: `node node_modules/webpack-cli/bin/cli.js --config playwright-tests/required-fields-sync/webpack.config.cjs` | Passed. Actual component imports compiled into fixture assets. |
| COPY: `REQUIRED_FIELDS_CASE_PACK=PACK node playwright-tests/required-fields-sync/run-browser.cjs` | **38 passed; zero failed/skipped.** Fresh registered static origin, exact IDs and sealed emitted manifest. [Output](reports/browser-components-tests.txt), [report](reports/browser-components.json), [manifest](reports/component-artifacts.json). |
| COPY: `REQUIRED_FIELDS_CASE_PACK=PACK node playwright-tests/required-fields-sync/run-checkout.cjs` | **Five passed; zero failed/skipped.** Also executes actual SDK/demo webpack CLIs through the fixture wrappers, both builds pass. [Output](reports/checkout-tests.txt), [report](reports/checkout.json), [manifest](reports/checkout-artifacts.json). |
| COPY: `REQUIRED_FIELDS_CASE_PACK=PACK node --test playwright-tests/required-fields-sync/contract.test.cjs` | **22 passed; zero failed/skipped.** Final pack and actual JS bundle, including malformed/bounded/report counterexamples. [Output](reports/node-tests.txt). |
| COPY: `REQUIRED_FIELDS_CASE_PACK=PACK node playwright-tests/required-fields-sync/run-resolver.cjs` | **14 passed.** Separate actual JS [report](reports/js-resolver.json). |
| H, after copying final pack into owned Rust fixture: `DOCS_RS=1 RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= CARGO_TARGET_DIR=ROOT/rust-target cargo test --offline --locked --manifest-path ROOT/rust-probe/Cargo.toml --lib` | **14 passed; zero failed/ignored.** Actual core 0.119.0, final pack compiled by `include_str!`. [Output](reports/rust-tests.txt), [report](reports/rust-resolver.json). DOCS_RS suppresses only the inspected external C-header generation described in historical checks. No router/MSRV feature build claimed. |
| H: `cargo +nightly fmt --manifest-path docs/required_fields_sync_contracts/sdk-v1/rust-probe/Cargo.toml -- --check` | Passed. No source rewriting. |
| COPY: `node playwright-tests/node_modules/prettier/bin/prettier.cjs --check 'playwright-tests/required-fields-sync/*.{cjs,js,ts,json}' playwright-tests/e2e/required-fields-sync.spec.ts` | Passed after formatting only new test/harness files with W's existing Playwright configuration. A preliminary root-node_modules invocation failed because Prettier is installed under playwright-tests; the corrected path above is the executed check. Final typecheck and all test reports rerun after formatting/hash updates. |

Runtime: Node `v26.0.0`, Playwright `1.63.0`, Chromium `153.0.8010.12`, ReScript `12.3.1`, webpack `5.109.2`, Rust `1.97.1 (8bab26f4f 2026-07-14)`. No minimum Rust 1.94.1 or H release-feature acceptance is inferred. Schemas are parsed and their executable boundary is tested; a separate JSON Schema validator was not installed/run.

## Evidence identities and final checks

The current pack binds 72 source/lockfile/harness artifacts. Reports require exact stage case IDs and matching pack/provenance; none of the earlier failing reports was promoted to success. Final reports come from successful reruns. Sanitized outputs replace machine-specific temporary paths; emitted manifests retain only relative files and digests.

| Artifact | SHA-256 |
|---|---|
| Final case pack | `a7b7936786fffac90fcdea0669a739dc28e5ade87e7d5989742d54f31443f95e` |
| Component emitted manifest | `582ff5e81a609e3be468f1f5cec8751ea4ab218fbe6b9cc9f8333c5fa10ace0b` |
| SDK/demo emitted manifest | `87b8f5ea6148d5ed4d1a65f9e43e904a82e00db170f560aaecac2ec1a7f340b8` |
| Executed Rust test binary | `645d31dcc11a80fe83441260ddb7e64730968ffa0c342e992ab099b1666c4021` |

**Final checks passed:** all four reports match their exact case sets and the final pack; all 72 source identities match W/U; both emitted-manifest hashes match the executed browser reports. JSON parsing, Markdown relative-file links/fences, `git diff --check` and explicit `git diff --no-index --check /dev/null <file>` passed for all 45 scoped files, including untracked additions. Trailing blank log lines were removed without changing recorded results.

H/W/U heads, branches and Git index digests match the continuation's starting snapshot. Tracked/staged diffs remain empty; U is clean at W's exact gitlink. Original plan, roadmap, seed and Step 0 baseline/check records are unchanged. Pre-existing untracked files remain present. No staging, commit, push, branch switch or submodule update occurred.

**Cleanup executed:** both owners/markers, exact root/web/cleanup paths, absence of `.git`/dotenv inputs, and empty npm configs were verified. All three final-run loopback listeners were confirmed closed; no fixture/browser process remained in the process check. Sanitized reports/manifests/output were saved first, then only the two registered roots above were removed. Original working trees, unrelated processes and user caches were not cleanup targets.

Changed groups for review: H `sdk-v1/cases.json`, README/check/decision records, `reports/`, parent progress/questions and playbook checkpoint; W `playwright-tests/required-fields-sync/` fixture preparation, browser/checkout runners/configs/reporter/tests/docs and `playwright-tests/e2e/required-fields-sync.spec.ts`. Existing contract/evaluator files received scoped formatting; no production source changed. The JSON schemas and Rust probe source/lock were preserved.

## Remaining gates

No input is needed to unblock these local browser checks. Owner approval of aliases/catalogs/AC–TA/EPS/dummy/retirement/conflicts, exhaustive enum/CIT/partial-filter comparisons, additional payment-method full-checkout cases, supported old/mobile/deployed readers and live presentation metadata remain missing evidence. See [decisions](decisions.md) and [Q1/Q8/Q11](../questions.md). A missing target-specific artifact blocks its gate, not unrelated local development.

Target V1–V4, contracts, journal, coordination and restoration remain uncertified; hosted apply stays disabled. Next eligible work after review is independent Step 2, or Step 3 after review of Step 1's input/label contract. No renderer or later step was started. Suggested separate manual commits: H `test: define required-fields mapping expectations`; W `test: cover required-fields SDK contracts`.
