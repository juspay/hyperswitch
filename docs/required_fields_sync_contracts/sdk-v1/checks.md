# Step 1 commands, results and limits

**Historical record — 2026-11-17.** The offline dependency blocker below was resolved on 2026-11-18. Read [browser continuation checks](browser-checks.md) for current results and cleanup. Files under `reports/` now contain the latest rerun against the expanded pack; hashes and counts stated below describe the earlier execution, not those replacement reports.

2026-11-17. Commands were inspected before execution. H/W/U heads, branches, status and index hashes were captured first; Step 0's untracked H documents were preserved. No remote fetch, secret retrieval, hosted target, uploader, deployment or payment submission was authorized or attempted.

`H` and `W` below mean the repositories in [README](README.md); `PACK=H/docs/required_fields_sync_contracts/sdk-v1/cases.json`. Placeholders describe executed commands; they are not shell variables implicitly set in the user's environment.

## Owned disposable resources

| Owner ID | Resource | Use |
|---|---|---|
| `required-fields-step1-0474eca9-9274-407d-bc58-037033d08982` | Temporary root ending `required-fields-step1-jigzorti`; marker `fixture-target.json` | Local W/U archives, offline npm attempts, copied Rust probe/cases and isolated Cargo target |
| `required-fields-step1-a638a531-2e0d-4847-bfdd-2de285ca9c74` | Temporary root ending `required-fields-step1-inWc6i`; marker `fixture-target.json` | Executed preparation script, verified source/harness copy and final Node suite |

Both registrations specify their exact root/web/cleanup scope, `kind=disposable-filesystem-build`, `hosted_target=null`, `server_mutations=false`. No server/port/workspace was started or reused; no browser launched. Only these owned roots are eligible for cleanup. npm's ordinary cache/log directories and other working trees are not cleanup targets. Cleanup outcome is recorded below.

## Executed checks

| Command / operation | Result and side effects |
|---|---|
| `git status --short`, `rev-parse HEAD`, branch, `ls-tree HEAD shared-code`, index SHA-256; scoped `rg`/`sed`/`cat`/Python reads | H/W/U baseline captured; U exactly matches W's gitlink. No applicable AGENTS.md found. Existing implementation and command side effects inspected. No Git mutation. |
| Local W/U `git archive HEAD`, extraction into owned root | Source copies only; no checkout, network, submodule lifecycle, stash or reset. |
| `npm ci --offline --ignore-scripts --no-audit --no-fund --update-notifier=false` in the first fixture's W root | **Failed `ENOTCACHED`: `wsl-utils-0.1.0.tgz` unavailable.** Partial node_modules/npm logs may be written; no download/lifecycle ran. Source checkout/lockfile unaffected. |
| Same offline npm command in first fixture's `web/playwright-tests/` | **Failed `ENOTCACHED`: `prettier-3.9.9.tgz` unavailable.** No usable pinned Playwright/TypeScript installation. These are the first observed missing packages, not an exhaustive list. |
| `REQUIRED_FIELDS_CASE_PACK=PACK node playwright-tests/required-fields-sync/prepare-fixture.cjs` from W | **Passed.** Creates second registered archive after revision/hash verification; explicitly copies untracked test additions and verifies them. No installation/server/network. Final test additions were copied to the same owned root and reverified by the evaluator checks. |
| `REQUIRED_FIELDS_CASE_PACK=PACK node --test playwright-tests/required-fields-sync/contract.test.cjs` in W and final registered copy | **22 passed; 0 failed/skipped.** Actual pinned JS bundle for 14 cases plus eight negative/boundary tests; fresh VM with one-second execution bounds. [Sanitized output](reports/node-tests.txt). |
| `REQUIRED_FIELDS_CASE_PACK=PACK node playwright-tests/required-fields-sync/run-resolver.cjs` in final copy | **14 passed**, exact pack/artifact-bound [JS report](reports/js-resolver.json). No fetch/browser. Node `v26.0.0`. |
| `cargo generate-lockfile --offline --manifest-path rust-probe/Cargo.toml` in first fixture | **Passed**, 292 packages resolved from local cache. Copied standalone lock into H's probe; H workspace Cargo.lock unchanged. |
| `DOCS_RS=1 RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= CARGO_TARGET_DIR=<owned-root>/rust-target cargo test --offline --locked --manifest-path rust-probe/Cargo.toml --lib` in first fixture | **14 passed; 0 failed/ignored.** Actual 0.119.0 `Config`, `eval_config`, `MergeStrategy::MERGE`; independent Rust expectations including JS divergence. [Output](reports/rust-tests.txt), [Rust report](reports/rust-resolver.json). Rerun after final pack/provenance updates because cases use `include_str!`. |
| `cargo +nightly fmt --manifest-path docs/required_fields_sync_contracts/sdk-v1/rust-probe/Cargo.toml -- --check` in H | **Passed** after correcting new probe line wrapping; check does not rewrite source. |
| `node --check <file>` for every new W `.js`, `.cjs`, `.ts` (12 files including e2e) | **Passed.** Syntax only; no imports/dependency execution and **not a TypeScript typecheck**. |

Rust ran with `rustc 1.97.1 (8bab26f4f 2026-07-14)`, not an executed 1.94.1 minimum-toolchain check. The probe declares H's current minimum 1.94.1. Its independent transitive lock is not H's release feature graph. Lock SHA-256: `96ef0e32b7973fcfb7f765fd3716df32989adc66cd22abec117b43b33850b458`; executed binary digest is in the Rust report.

The inspected `superposition_core` build.rs normally generates a C header outside its package directory. Its existing `DOCS_RS` early return suppresses that unrelated generation in this isolated test. It does not replace the evaluator, skip Rust compilation/tests or fabricate results. Clearing compiler wrappers avoids ambient wrapper/cache commands. No broad H test, including the known writing required-fields dump, was run.

Authoring checks initially caught one invalid case ID and rustfmt wrapping; both were corrected and rerun. These were new-fixture issues, not application baseline failures. Source review corrected the hermetic whole-response override API before execution; no checkout pass is claimed.

## Inspected but unexecuted checks

| Command or gate | Missing evidence / side-effect boundary |
|---|---|
| Pinned `tsc -p playwright-tests/required-fields-sync/tsconfig.json --noEmit` | Missing dependencies. Scoped config includes component/checkout specs; no typecheck result. |
| W `npm run re:build` in registered copy | No pinned ReScript installation. Reviewed script compiles/writes only in the copy; not executed. |
| Local webpack CLI with `required-fields-sync/webpack.config.cjs` | Needs compiled W/U and webpack dependencies. Custom config avoids ordinary production/dev plugins, env loading and proxies. No emitted component bundle. |
| `node playwright-tests/required-fields-sync/run-browser.cjs` | Needs bundle and pinned Playwright/browser. Runner registers a fresh loopback origin, denies external traffic, aborts fixture confirm and refuses incomplete reports. None of 33 component tests executed; no successful stub report. |
| `test:hermetic -- e2e/required-fields-sync.spec.ts` | Three tests authored, not collected/run. Default config can auto-build, invoke `npx` and start dev servers/proxies. Needs demo dependencies, explicitly registered disposable origins, `PW_REUSE_SERVERS=0`, service-worker blocking and complete served artifact/report binding before certification. No arbitrary server reuse. |
| Root npm install/start, default/live Playwright, submodule updater | Not selected: lifecycle advances hooks/submodule; default tests use live targets. |
| H router/v1/v2 feature builds, minimum MSRV, server contracts, mobile/old/deployed SDKs, V1–V4 | Outside this resolver probe or missing authorized targets/artifacts. No acceptance inferred from local tests. |

See [gate matrix](README.md#source-evidence-and-remaining-gates) for T05–T09/T29/T35–T38 gaps and [decisions](decisions.md) for Q1/Q8/Q11. Schemas document JSON; the executed Node boundary enforces shape/bounds/cross-field identity. A separate standards-compliant JSON Schema validator was not installed or run.

## Final document, integrity and cleanup checks

Final checks **passed**: JSON parsing; exact case/artifact/report hash validation through W's Node boundary; 33 B/three E declared IDs match authored tests; all 12 JS/CJS/TS syntax checks; rustfmt; Markdown relative-file links and balanced fences; `git diff --check` and explicit `git diff --no-index --check /dev/null <file>` for all 32 scoped files (including untracked files).

H/W/U HEAD, branch and Git index SHA-256 match the pre-step snapshot; tracked/staged diffs remain empty and U stays clean at the exact gitlink. Step 1 additions and the pre-existing H docs are still untracked. Protected roadmap, seed, W package-lock and U evaluator SHA-256 values match the starting snapshot. Original implementation plan and business/SDK source were not edited.

Final case-pack SHA-256: `ca4d136c47692914392985cc3aec4580271e9c66e2b34beba046bacc2b9e9a63`. Reports contain the matching pack/provenance and separate executed artifacts; the latest Rust binary SHA-256 is `1c427ca773da022402c18b18fda7aeff55a2066ed36cfd030d500c4197357030`.

**Cleanup executed:** both registered temporary roots above were removed only after verifying their exact owner IDs, kind, root/web/cleanup paths, absence of `.git`, and no hosted target. Sanitized reports/output were retained first. No server was started, so none needed termination. Original repositories and npm cache/log directories were not removed or modified by cleanup.

These checks do not certify browser semantics or complete the outstanding plan gates. No later step was started. Suggested separate manual commits: H `test: define required-fields mapping expectations`; W `test: cover required-fields SDK contracts`. Review before staging; this session staged/committed nothing.
