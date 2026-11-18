# Step 1 — independent SDK expectations

2026-11-18. **Local synthetic acceptance passes: 22 Node tests, 14 actual Rust resolver tests, 38 browser-component tests and five full-checkout tests.** No renderer, sync command, source-data correction or SDK behavior change. Owner review and target certification remain open.

Read [browser continuation checks](browser-checks.md) for current commands/results, [historical checks](checks.md) for the earlier offline dependency blocker, and [decisions](decisions.md) for proposed choices and missing evidence. Step 0 [baseline](../baseline.md) remains historical source evidence. H/W/U revisions are unchanged from that baseline:

| Repository | Branch / revision |
|---|---|
| H | `main` / `82fe5dfd3ac527fd4a1e03dc01fbb5be13e3198b` |
| W | `main` / `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39` |
| U | detached / `1af3e5dabf9d5a17dd864ac7312151ebac4367c6`, exactly W's gitlink |

H's existing untracked design documents/register were preserved; W/U started clean. No applicable AGENTS.md was found in the inspected H/W/U directories or ancestors. H's CONTRIBUTING, current justfile/feature rules, W scripts/configs/fixtures and U package instructions were inspected before selecting commands. Existing W hermetic fixtures, parser, hook, components, payload merge and bundled evaluator were reused; none was reimplemented as an oracle. I and external artifacts were outside this step.

## Independent corpus

[cases.json](cases.json) contains 14 resolver cases, seven mapping cases, six country cases, and IDs for 38 browser-component and five full-checkout tests. Inputs and expected values are synthetic and independently specified from plan §3.2–3.3/§6.3/§11.3, API wire shapes and pinned consumer source. No expected output is copied from a seed dump or renderer. Initial browser failures exposed a fixture wire-shape mistake and an incorrect full-form failure expectation; their source-backed corrections are recorded in [browser checks](browser-checks.md). `review.status` stays `pending-owner-review`, with no invented owners. These examples still need independent human/business review.

| Cases | Specified expectation / layer |
|---|---|
| R01/R02 | Default `[A,B]`, common `[A]`, explicit mandate `[A,B]`; omission changes result to `[A]`. Executed JS and Rust. |
| R03/R04 | Removal exposes baseline true; explicit country false wins. Executed JS and Rust. |
| R05–R08 | Supplied order, absent mandate, exact casing, scalar versus array matching. Executed JS and Rust. |
| R09 | Missing-default override key: JS adds it; current Rust omits it. Both executed separately. |
| R10–R14 | Ordered duplicates, empty/null options, later tenant, missing override reference. Execution describes permissive resolution; null options/references are separately refused by the case contract. |
| M-* | Four scoped Adyen online-banking aliases, UK non-alias, two Givex field IDs/read/write paths and independently written nested payloads. Actual casing and component payload tests pass. |
| C-* | Literal ALL/comma have no country choices; supplied four-code sample expands to exact names; AC/TA are absent from bundled fallback; literal order/duplicates and empty lists retained. Browser cases pass; sample is not an approved universal catalog. |
| B-* | Actual constructor, account/client parsers, casing/UI type normalization, eligibility order, missing provider/currency, Credit-only mismatch, hook inheritance/removal/kind change, connector/alias orders and per-connector exception, forced optional billing, prefill, missing metadata/config/connectors, ordered generic/bank choices, ACH/EFT BankType payloads, per-kind empty options, state allowlist/order/country change, ISO payloads, language/core-card filtering, locale/English/bundled/malformed assets. All 38 pass. |
| E-* | Actual SDK/demo checkout: Card/Debit, retained/fresh/restored snapshots, whole-form config-fetch failure, required-field validation, card plus dynamic billing confirm payload, explicit-versus-omitted mandate options. All five pass; confirm is intercepted and aborted. Other payment-method checkout combinations remain unexecuted. |

The Node counterexamples deliberately remove an equal-to-default mandate contribution and sort semantic options. Both produce different results from the independent oracle. A generated golden file could not legitimize either transformation.

## Bounded case/report contract

[cases.schema.json](cases.schema.json) and [report.schema.json](report.schema.json) describe v1 JSON. W `playwright-tests/required-fields-sync/contract.cjs` is the executable boundary and adds cross-field and byte bounds:

- Case pack ≤262,144 UTF-8 bytes, report ≤65,536 bytes; depth ≤16, individual arrays ≤256, object keys ≤512, strings ≤4096 characters, ≤128 combined unique case IDs, ≤64 contexts per resolver case, ≤128 artifact files. Unknown structural properties and unsafe keys refuse.
- Exact W/U revisions, Rust core version, source/lockfile/asset/harness SHA-256 identities. Relative artifact paths only; missing/drifted files refuse execution. U's actual evaluator digest remains `5db572c0540ecf067beb78537f69215d9d0cc538e28b8bdd44685fcef1f91a19`.
- Separate stages `js-resolver`, `rust-resolver`, `browser-components`, `checkout`. Reports must include exactly that stage's case IDs once, matching pack hash and provenance. Status is `passed`, `failed` or `not_run`; aggregate incomplete/failed cannot be promoted to passed. All-pass requires an executed artifact digest. JS requires its exact evaluator hash; browser runners bind every emitted file through a sealed manifest, verified before serving and after execution.
- Result detail ≤512 characters. No secrets, real payment/tenant data, live inventory, hosted target address or force flag. Separate executed [JS](reports/js-resolver.json), [Rust](reports/rust-resolver.json), [components](reports/browser-components.json) and [checkout](reports/checkout.json) reports record actual runtimes. See [current checks](browser-checks.md).

**Scope:** this is a local synthetic contract, usable as a versioned oracle/validator building block for later planner/verifier work. It is not a secure attestation protocol or an implementation of planner preflight. It does not authenticate a caller, reject duplicate raw JSON keys before parsing, prove a deployment/target, or bind an actual router response/run/authority. Those later interfaces must add exact response/config/version/target/run/case/artifact binding, trusted execution and independently reviewed coverage; missing/stale evidence must refuse. Local emitted manifests do not identify deployed assets. A local test report cannot enable hosted apply or certify target V3. Do not transplant its `passed` status into a post-apply success result.

## Source evidence and remaining gates

Relevant source locations are pinned by digest in the pack: U `ConfigurationService.res`, `SdkConfigParser.res`, `SuperpositionHelper.res`, `SuperpositionTypes.res`, `CommonUtils.res`, evaluator; W `DynamicFieldsUtils.res`, `PaymentUtils.res`, `PaymentMethodsRecord.res`, `ParentCardComponent.res`, `CardBusinessFields.res`, `DynamicFields.res`, `DynamicFieldInput.res`, dropdown components, `Utils.res`, `Country.res`, `States.json`, `S3Utils.res`. H's current cached 0.119.0 Rust core API was inspected and used directly instead of the plan's historical 0.115.5 symbol assumptions. Step 0's D1–D6 remain applicable; no fresh revision drift justified repeating unrelated deployment research.

| Plan gate | Evidence now | Still missing |
|---|---|---|
| T05 | Seven mapping examples; actual component payloads including ACH/EFT; actual card/dynamic-billing confirm | Owner approval, every payment-method full-checkout payload, live metadata and deployed readers |
| T06 | Country/State cases and fallback assets execute | Approved full ordered ALL catalog, AC/TA policy, future generator's unrestricted/missing/empty contract |
| T07 | Actual JS/Rust and browser hook inheritance; full-checkout explicit/omitted mandate options | Other method/payload combinations and target serving-path verification |
| T08 | Actual JS/Rust baseline/false/tenant and browser baseline-removal cases | Planner semantic impact and target effects |
| T09 | Actual JS/Rust matrix; browser render-kind change retaining inherited options | H partial-filter integration, broader construction/collection matrix; local malformed-ref refusal is not planner preflight |
| T29 | ReScript/webpack/typecheck; 38 component + five checkout cases; real card validation/confirm | Full checkout for every supported method, old/mobile/deployed artifacts |
| T35 | Browser construction/eligibility/mandate/normalization/Card/Debit/Bancontact cases | Exhaustive H enum and CIT comparison; full Bancontact checkout |
| T36 | Browser connector permutations, unique later field, optional billing, Givex aliases, unsafe metadata and injected per-connector evaluator exception | Business conflict approval and refusal in future planner |
| T37 | Browser options/prefill/state/language/core-card/ACH/EFT payloads; real card form validation | Payment-method owner review and other methods' full form validation |
| T38 | Browser failure/snapshot/asset cases; sealed full emitted manifests and exact report binding; Node drift/report refusals | Mobile/deployed assets, real sync/restoration/cache/backup lifetimes and authenticated target result binding |

Exact EPS settings/order, dummy exclusion/retirement and complete field-type replacement policy still need input-policy cases and review before accepting renderer goldens. The generic-kind replacement probe does not approve that policy. No successful stub fills any missing gate. Target V1, V2, V3 and V4 remain uncertified; hosted apply remains disabled.

After review, **Step 2** local server-harness work is independent. Step 3's pure-input/label work becomes eligible once this input/label contract is reviewed; do not imply that unresolved mapping/catalog choices permit Step 4 activation. No later step was begun.
