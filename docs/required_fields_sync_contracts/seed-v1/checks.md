# Step 14 checks and acceptance boundaries

Commands were inspected before execution. Local Cargo checks use `+stable --offline --locked`,
an allowlisted environment, two jobs and OS-denied networking. The disposable Step 14 root is
explicitly registered with owner, purpose, equal canonical root/cleanup scope and no hosted
target. Previously owned Step 11 Cargo caches are registered for reuse. W/U source copies and
copied locked dependency/browser caches are separate from all working trees; no npm install,
lifecycle script, submodule update or download was executed locally. Browser execution permits
only loopback and probes non-loopback denial before testing. These are filesystem/browser
fixtures, not a registered Superposition workspace.

The starting H tree already contained 31 changed/untracked files from Step 13 and the original
plan/roadmap. W already contained 30 untracked verifier files. Step 14 changes are compared to
those starting bytes as well as HEAD. Index/head/branch hashes, W/U/I tree states, unchanged
source business data, original documents, seed, uploader and compose/Cypress files are checked
separately. [evidence.json](evidence.json) lists exact commands/results/log hashes and Step 14
file groups; private raw logs and synthetic outputs remain in the owned fixture root.

## Executed evidence

| Check | Result and scope |
|---|---|
| Actual router-module isolated suite | 162 passed, 15 explicitly ignored; final focused seed selection passed 10 tests with one exporter ignored |
| Isolated `cargo clippy … --lib --tests -- -D warnings` | Passed; existing unreachable `uuid::Uuid::now_v7` Clippy configuration warning remains |
| Isolated feature-off CLI tests (`cli::tests`) | Two passed; no sync command in the ordinary parser |
| Actual payment_methods package renderer tests | 49 passed, 2 explicitly ignored; independent sources, labels, options, aliases and input invariants |
| Opt-in router `cargo build --offline --locked -p router --features required_fields_sync --bin router` | Passed; actual default-v1 binary, existing warnings retained |
| Ordinary router `cargo check --offline --locked -p router --bin router` | Passed with the opt-in feature disabled |
| Payment methods `cargo check … --no-default-features --features v1,payouts,required_fields_sync,storage_impl/default,hyperswitch_interfaces/v1 --lib` | Passed; supported no-dummy build, not the complete T33 matrix |
| `test_seed_cli.py` against that binary | 12 cases passed with complete network denial and invalid DB/Redis settings; zero writes to Superposition, zero leases, unchanged template/tree |
| Actual build.rs guards for router/payment_methods without v1 and with v1+v2 | Four expected refusals; not a full ordinary-v2 build |
| Actual Rust resolver crate `cargo test … sdk-v1/rust-probe/Cargo.toml --lib` | 14 passed; Rust semantics remain distinct from JS |
| Contract companion `cargo test … required_fields_contracts/Cargo.toml --test isolated` | 18 passed; synthetic/wire evidence, no server certificate |
| Existing contract `check_lock.py` / Python fixture tests | Lock matched SDK 0.119.0; 10 passed; no API writes |
| New `test_contracts.py` | Five passed: archive tamper/traversal refusal, explicit profile, unchanged server matrix and source-level Deja preservation |
| Owned `seed_export_independent_semantic_inputs -- --ignored` | Passed; actual editor exports synthetic good/wrong seeds with stable regeneration |
| `seed_semantics.cjs <owned-web> <owned-seed-inputs>` | Two expectations passed against actual pinned U: good seed passes, wrong ordered seed fails despite generator agreement |
| Pinned W/U `run_browser.py` | Seven stages passed: 27 Node tests, actual ReScript build, two TS checks, webpack, synthetic input creation and actual verifier |
| Actual browser stages inside that verifier | 38 component + 5 checkout + 7 response-bound cases passed; confirm intercepted, no payment submission |
| New workflow `actionlint` (including ShellCheck) | Passed; remote workflow execution is not claimed |
| `just --dry-run required-fields-seed-check …`, `bash -n` on offline runner and uploader | Passed; syntax/source evidence only, uploader not invoked |
| Changed Rust `rustfmt +nightly --check --config skip_children=true …` | Passed after formatting the new test imports with the installed nightly |

The actual router build/CLI and payment-method build matrix have their own command records in
the evidence file; an isolated module pass does not stand in for a full binary. The CLI harness
uses independently authored expected context/requiredness and deliberately invalid DB/Redis
settings under complete network denial. It checks deterministic explicit inputs, ambient bank
pollution, exclusive create-new private output, no overwrite, drift, unchanged files in check
mode, denied profiles/scope/features and the full-profile approval refusal. Its synthetic
single-source exclusions are test plumbing, never adopted by the checked-in local profile.

The complete explicit local profile returned exit 2, `mapping_unresolved`, with **1,625
MissingBinding** and **three MalformedOptions** diagnostics. This expected refusal passed the
denial test; it did **not** pass complete-seed acceptance. No initial managed seed diff was
created. Existing renderer qualification/email imports, linker unwind-size and dependency
future-compatibility warnings were retained rather than corrected incidentally.

## Intermediate failures and source findings

- The first isolated compile used a digest as a sorted map key, but `Digest` intentionally has
  no `Ord`. Exact canonical condition bytes now identify local rows; digest hashes remain report
  evidence. A later full-router inference error in a string lookup was removed by using the
  typed managed key directly. These failures are retained, not reported as passing attempts.
- A new malformed-function test initially named a nonexistent generic property. It now uses
  the actual `value_validation_function_name` contract. No production behavior was changed to
  satisfy an invented symbol.
- The first macOS browser sandbox string was rejected before execution; the corrected policy
  uses supported `localhost:*` rules plus a non-loopback denial probe. The subsequent complete
  browser run passed.
- Initial new-workflow lint rejected `runner.temp` in job-level env. Paths are now selected in
  steps; network namespaces drop back to the runner UID before writing private artifacts.
  Full `CI-pr.yml` lint also reports existing custom-runner labels/action-input metadata warnings
  outside the added reusable job. They are not changed or hidden by suppressing new-file lint.
- Stable rustfmt cannot apply the repository's nightly import grouping. Installed nightly was
  verified and used for the final changed-file format check. MSRV 1.94.1 is not installed.
- Two exploratory feature-off filters selected zero cases and are not test passes. The actual
  `cli::tests` module then ran both ordinary-parser checks successfully; CI uses that path.

## Gates still pending

| Gate | Why it is not passed |
|---|---|
| Complete local seed / initial managed migration diff | Full source bindings, field/country catalogs, exact grants and review are absent. The explicit local policy refuses. No automatic exclusions or hidden data corrections |
| T10 ordered-array server identity / all real API capabilities | Exact image/source/auth/permission/fault evidence is not certified; isolated tests do not replace it |
| Uploader/Deja runtime regression | No verified disposable API target was provisioned for this step. The uploader was read and syntax-checked only |
| Linux CI execution and cold npm/Rust provisioning | New workflows were linted; no remote CI job was started locally |
| Full T33 release/MSRV/TLS/Redis/v2 matrix | Only recorded supported local commands and explicit unsupported-source guards ran; no matrix-wide claim |
| Deployed/mobile/old SDK, real V1–V4 and hosted safety | Local response/browser fixtures and retained snapshots are synthetic; platform/storage/authority/caches/backups and actual reader provenance remain separate gates |

No hosted target, Docker server fixture, uploader, payment, deployment, staging, commit or push
ran. Missing certification remains pending/refusing; no force flag, successful stub, version
upgrade or Step 15 deployment integration was added.
