# Superposition required-fields automation — implementation playbook

Working guide for Deepanshu. Complete one step, review its changes and evidence, then move to the next. Each step includes a copyable implementation prompt and a suggested message for a manual commit.

**Current checkpoint — 18 November 2026:** **Step 5** bounded direct read-only inventory and its lossless completeness boundary are locally implemented. See [inventory evidence/checks](required_fields_sync_contracts/inventory-v1/README.md) for 47 isolated tests, actual SDK 0.119.0 request comparison, build evidence and pending exact-server certification. The certified target registry is empty; network reads refuse before I/O. Step 4 full-profile mapping/catalog/source-data approvals remain open. Hosted apply remains disabled pending contracts, journal, coordination, restoration and target V1–V4. After review, Step 6 local ownership/preflight/planner work is next; its acceptance requires Step 2 certified ordering/projection/read scope and Step 4 approvals. Step 2 certification may resume independently with verified disposable dependencies. No later step was begun; do not continue automatically.

**Historical checkpoint at playbook creation:** design and SDK investigation were documented, with Step 0 next. Hyperswitch HEAD was `82fe5dfd3ac527fd4a1e03dc01fbb5be13e3198b`; only the implementation plan and historical roadmap were untracked before this playbook was created. The proposed renderer/sync symbols were not found in the searched router/payment_methods source. Recheck current implementation before every step; this historical checkpoint is not permission to overwrite later work.

The technical specification is [superposition_required_fields_sync_implementation_plan.md](superposition_required_fields_sync_implementation_plan.md), called **the plan** below. This playbook turns its P0–P9 milestones into smaller execution steps. It does not replace its mapping, ownership, failure, security or deployment contracts. The [historical roadmap](superposition_required_fields_sync_roadmap.md) remains investigation background. If current code conflicts with the plan, record evidence and propose a specific amendment before implementing the affected behavior.

The format follows the residuals implementation playbook. Its business rules, database setup, branch, private-file policy and time budget do not apply here. Creating this file performs no feature implementation and grants no environment access or deployment approval.

## 1. Decisions to carry into every step

| Area | Implementation direction |
|---|---|
| Managed surface | Connector/method/type equality contexts, optionally mandate type; only approved `dynamic_fields.*._is_required` and `._dropdown_options` contributions within granted ownership. |
| Rule source | Existing compiled `RequiredFields`, effective relevant settings, build features, explicit mappings/catalogs and exclusions. Reuse existing construction and settings loading. |
| Rendering | Pure, deterministic, explicit inputs; canonical object ordering; preserve ordered option values and duplicates. Normalize only documented unordered sources such as EPS bank sets. |
| Executor | One `router required-fields …` command family in the release image. Hosted execution is a deployment Job bound to exact image/settings and one authoritative release per actual Superposition target. |
| Ownership | Full inventory and explicit adoption grants; preserve unrelated keys and existing descriptions. A structural match or provenance marker does not confer ownership. |
| Prerequisites | Reviewed enum extension and exact managed-default creation only. Existing schemas/default values and manually authored presentation metadata need explicit compatibility checks. A valid seed does not prove live metadata exists. |
| Validation | Known invalid items refuse the whole plan before mutation. No skip-and-success behavior or generic force bypass. |
| Atomicity | One bounded context bulk transaction; prerequisites commit separately. Refuse oversize rather than introducing hidden batches. |
| Coordination | Target-wide lease, durable unresolved-run record and enforced protection against external writers; a sync-only lock is insufficient. |
| Recovery | Persist complete before images and exact request intentions before any mutation. Unknown response outcomes require observation before retry or restoration. |
| Verification | V1 persisted state, V2 router/cache, V3 actual SDK construction/resolution/render/payload, V4 backup/fallback. An empty diff proves only the persisted projection. |
| Entry points | Offline render/seed, read-only plan/status, and gated apply/resume/restore. Ordinary serving does not synchronize automatically. |
| Initial scope | v1 writer, opt-in and disabled by default; v2/custom/playground/OLAP/Euler releases are not automatic writers. Determine real readers and sharing per target. |
| Delivery | First useful slice is an offline renderer and read-only planner. All journal, concurrency, restore and verification controls remain mandatory before hosted mutation. |

### SDK contracts already investigated

Carry these source-backed findings into independent tests, then revalidate supported deployed artifacts:

- Browser field binding comes from presentation confirm/intent paths. Givex alias families share payload paths but differ in layout/order; the four Adyen online-banking bindings need exact scoped aliases.
- Explicit options equal to global defaults must remain explicit when another matching context can supply a different value.
- U's JavaScript evaluator adds override keys missing from defaults; the inspected Rust resolver omits them. Test each expected outcome instead of asserting universal parity.
- Browser aggregation keeps the first included descriptor per confirm write path. Test co-eligible connector orders and duplicate aliases.
- Ordinary web card collection uses Card/Debit; credit-only differences require explicit coverage or a separately reviewed SDK change.
- Country `ALL` needs an approved ordered expansion; `DK,FI,NO,SE` as one option is invalid for the inspected country consumer. Generic BankType values must match the lowercase API contract. Separate source-data corrections from generator work.
- Empty options mean different things for Country/Dropdown/BankNamesSelect and State. State can fall back to all states on a wholly invalid allowlist.
- Browser snapshots can outlive router cache refresh. Runtime country/state assets are additional versioned verification inputs; seed AC/TA versus bundled fallback coverage remains a catalog decision.

The plan's 14 executed JS probes are historical evaluator evidence. They do not mean the feature tests, full browser flow, mobile readers or live deployments have passed.

## 2. Repositories and working rules

| Label | Local location | Role |
|---|---|---|
| H | `/Users/deepanshu.bansal/hyperswitch` | Renderer, planner, command, execution/recovery and CI. |
| W | `/Users/deepanshu.bansal/Desktop/hyperswitch-web` | Actual browser flow, independent SDK fixtures and verifier packaging. |
| U | W `shared-code/` | Pinned `hyperswitch-sdk-utils` submodule; resolver and descriptor helpers. |
| I | `/Users/deepanshu.bansal/Desktop/hyperswitch-infra` | Target registry, values, build/deployment integration and alerts when that step is explicitly requested. |
| External | Selected Helm chart, exact Superposition server builds, mobile SDKs and backup implementation | Obtain actual sources/artifacts; do not invent paths or treat missing access as evidence. |

H's original audit used `f237029…`, W `939f457…`, U `1af3e5d…` and I `92041e6…`. These are evidence baselines, not instructions to reset checkouts or assume deployment versions. Read applicable `AGENTS.md` and other required repository guidance before editing each repository. Preserve existing work, including changes in submodules; inspect a pinned dependency in a separate checkout if necessary.

### Paste these session instructions first

```text
We are implementing Superposition required-fields automation.
Use /Users/deepanshu.bansal/hyperswitch/docs/superposition_required_fields_sync_implementation_playbook.md
for execution order and the adjacent implementation_plan.md for the technical specification.
Read the named step and its referenced plan sections before editing. Implement only that step.

Inspect applicable repository instructions, branch/revision and working-tree state first.
Preserve unrelated changes and reuse existing Rust/settings/CLI/error conventions.
Refresh stale source references and document material discrepancies; do not silently follow outdated symbols.
Keep assumptions, proposed choices, source evidence and executed checks distinct.

Make scoped local changes and add/run the named step's focused tests and build checks.
Inspect commands for side effects first. Use synthetic data and explicitly registered disposable targets.
Do not fetch production secrets or contact hosted environments from a coding prompt alone.
Hosted read-only inspection needs an explicitly named authorized target; reuse existing authorization within its scope.
Do not seed a hosted workspace, change live Superposition state or deploy from these coding prompts.
Contract/integration tests may write only to their verified disposable fixture target and clean up only owned resources.
A missing external dependency blocks the dependent acceptance gate, not unrelated local work.
Do not replace missing contract/coordination/verification evidence with a successful stub or force flag.

Keep hosted apply disabled until all required contracts, journal, coordination, restoration and V1–V4 checks pass.
Do not add a sync HTTP endpoint, separate mutating daemon, destructive reset or automatic batch fallback.
Do not change source business data or SDK behavior as an incidental renderer fix.

Leave Git staging, commits, pushes, remote PRs, merges and deployments to me.
Do not reset, stash, discard work, update a submodule to remote HEAD, or change branches automatically.
Suggest separate commit messages/file groups for changes spanning repositories or unrelated corrections.
After the step: explain changes and why, list files, commands/results, acceptance gates and remaining blockers,
identify the next eligible step, suggest a Conventional Commit message, then stop for my review.
Do not begin the next step automatically or mark an unexecuted check as passed.
```

These are instructions for future implementation sessions when pasted with a step. They do not request feature work during creation of this guide.

**Manual commit checkpoint:** review `git status --short`, the local diff and new-file contents; stage explicit intended paths yourself; inspect the staged diff and commit manually. In a multi-repository step, review and commit each repository separately. Keep secrets, live inventories, arbitrary tenant data, temporary outputs and unrelated work out of commits. Unlike the residuals guide, this file is under H `docs/`; do not assume it is gitignored or private.

**Early review checkpoint:** after Step 7, prepare a local draft PR description for the renderer/read-only planner. Deepanshu can manually push/open it for review while operational prerequisites are being resolved. This is a reviewable slice, not an enabled synchronization feature.

## 3. Order, dependencies and checkpoints

Step numbers here differ from P0–P9 in the plan. T01–T38 and Q1–Q11 always refer to the plan's tests and open questions.

| Step | Deliverable | Plan milestone | Prerequisites |
|---|---|---|---|
| 0 | Refreshed baseline and progress/evidence register | P0 | Existing docs/current code |
| 1 | Independent SDK/mapping cases and contract harness | P0 | 0 |
| 2 | Server API contract harness and certification record | P0 | 0; exact disposable server builds for certification |
| 3 | Pure inputs, canonical artifacts and feature boundaries | P1 | 0; input/label contract from 1 |
| 4 | Deterministic renderer and approved mapping policy | P2 | 1, 3; business decisions for affected mappings |
| 5 | Complete read-only inventory adapter | P3 | 2's relevant read contracts; 3 |
| 6 | Ownership, preflight, planner and semantic impact | P3 | 1, 4, 5; certified ordering/projection from 2 |
| 7 | Offline/read-only command slice and early review | P6 subset | 4–6 |
| 8 | Durable journal and recoverable before images | P4 subset | 3, 6; Q6 for real storage certification |
| 9 | Release authority, coordination and writer exclusion | P4 subset | 6, 8; Q3/Q4 for actual platform integration |
| 10 | Apply state machine and exact mutation outcomes | P5 subset | 2, 6, 8, 9 |
| 11 | V1–V4 verification and SDK companion | P5 subset | 1, 2, 6, 10; artifact/backup evidence for external gates |
| 12 | Resume, three-way restore and pre-feature rollback | P5 subset | 8–11 |
| 13 | Mutating command wiring, attestations and reports | P6 completion | 7–12 |
| 14 | Seed generation and CI integration | P7 | 1, 4, 6, 7; 11 for browser CI; 13 for full command CI |
| 15 | Disabled infrastructure executor and target policy | P8 | 8–13; actual Helm/platform source |
| 16 | Integrated failure drills and implementation handoff | P5/P8 acceptance | 10–15 |
| 17 | Target adoption packet and operator handoff | P9 | 16 and target-specific external gates |

Steps 1 and 2 can progress independently. Step 3/4 work may proceed while live/server certification is pending if its dependent schemas/mappings are settled; do not mark Step 2 complete from mocks. Step 14's offline seed work can follow Step 7, while its full CI acceptance waits for Step 13. Source-data corrections are separate conditional PRs attached to Step 1/4 decisions, not hidden extra changes.

**Intentional sequencing adjustment:** expose only offline/read-only parts of P6 at Step 7 so the first slice is usable. Write-command wiring stays in Step 13. Verification and restoration are split for review, but both remain first-release requirements; internal apply code from Step 10 is not permission for hosted execution.

## 4. Steps and copyable implementation prompts

### Step 0 — refresh the evidence and record the starting point

**Goal:** start from current code without rerunning the entire historical investigation or overwriting existing work.

**Needs:** plan §3–4, §12–14; current H/W/U/I states and applicable instructions. No environment access is needed.

**Work:**

- Inspect the relevant changes since the recorded revisions: constructor/types/settings, SDK config route/provider, feature/dependency versions, W/U consumer path, deployment inputs. Use a separate inspection checkout if an old revision is needed; preserve current branches.
- Record current revisions, existing implementation, unresolved questions, named owners if known and proposed local test commands. Do not invent team approvals.
- Create a sanitized `docs/required_fields_sync_contracts/` evidence/progress area (proposed) with a step checklist and Q1–Q11 register. Record this playbook's checkpoint as historical once implementation advances.
- Inspect existing test commands before running anything. Identify the file-writing required-fields dump, W's remote-tracking postinstall and default live Playwright project so they are not run accidentally.

**Checks / done:** differences affecting the plan are identified; selected commands and their side effects are understood; no app/infra/seed changes; no Git history/index changes. Baseline failures are recorded only if actually observed.

**Suggested commit:** `docs: record required-fields sync implementation baseline`

```text
Complete Step 0 of docs/superposition_required_fields_sync_implementation_playbook.md in H.
Read the plan and relevant repository instructions. Refresh only evidence affected by current revisions.
Preserve all working trees and identify any already implemented portions before planning new files.
Create the sanitized contract/progress register with current revisions, dependent blockers and inspected check commands.
Do not implement the feature, run the uploader, access live environments, or change the original roadmap.
Check the document/diff. Report findings, changed files and next eligible steps; suggest a commit message.
Do not stage or commit. Stop for my review.
```

### Step 1 — make mapping and SDK expectations executable

**Goal:** create an independent oracle before generating desired-state fixtures.

**Needs:** Step 0; plan §3.2–3.3, §6.3, §11.3, Q1/Q8/Q11; W/U source at explicit revisions.

**Work:**

- Add synthetic cases and independently reviewed expected JSON/payloads under the established test conventions. Use H's proposed contract area for cross-repository manifest/decisions and W's proposed `playwright-tests/e2e/required-fields-sync.spec.ts` or a convention-compatible equivalent for browser tests.
- Turn the 14 bundled-JS probes into repeatable evaluator tests. Include `[A,B]` default/common/mandate overlap, baseline requiredness after removal, exact casing, absent dimensions/defaults, ordered duplicates and malformed override references.
- Exercise actual context construction, parser and field aggregation: Card/Debit, mandate classification, absent provider/currency dimensions, collection flags, connector permutations, duplicate Givex payload paths and prefill/visibility.
- Cover exact online-banking/Givex bindings, lowercase BankType payloads, ALL expansion, invalid comma country, per-render-type empty lists and country/state asset fallback. Intercept confirm; do not submit payments.
- Define a bounded case/report format usable by planner preflight and post-apply verification. Pin W/U/bundle/assets and record source-supported versus executed behavior.
- Record mapping/catalog decisions requiring owner review. If a Rust source or SDK correction is needed, prepare a separately scoped proposal/PR; this step's tests must not silently fix business behavior to pass.

**Checks / done:** relevant T05–T09, T29, T35–T38 executed with independent expectations where tooling allows; wrong semantics fail even if generator output would match a seed. Missing browser/mobile/deployed-artifact evidence remains pending. No npm lifecycle advances U or invokes live tests.

**Suggested commits:** H `test: define required-fields mapping expectations`; W `test: cover required-fields SDK contracts`.

```text
Complete Step 1 of H docs/superposition_required_fields_sync_implementation_playbook.md.
Create independently specified synthetic mapping/resolver/browser cases and a bounded case/report contract in H/W.
Use the actual pinned W/U functions; distinguish JS resolver results from full browser results and Rust behavior.
Cover the listed inheritance, aliases, connector orders, Card/Debit, options, prefill and payload cases.
Inspect dependency setup and use isolated/hermetic tests with intercepted confirm; preserve the pinned submodule.
Record unresolved business choices and separate source/SDK corrections instead of changing behavior incidentally.
Run the focused checks you can perform; list missing evidence precisely. Suggest commits per repository.
Do not stage or commit, contact hosted targets or begin the renderer. Stop for my review.
```

### Step 2 — certify the server operations in disposable workspaces

**Goal:** establish actual transport and server guarantees before building destructive planning/apply logic.

**Needs:** Step 0; plan §6.6, Q2/Q4; exact server image/source identity and local disposable target registration. H's locked client is a separate identity from the server version.

**Work:**

- Add an opt-in contract harness using fresh fixture-owned workspaces, synthetic data and target checks. Bound requests, record versions/digests and clean up only resources owned by the harness.
- Certify complete list/pagination/auth behavior; prefix stripping; create-on-existing merge; full replacement/removal; delete-missing; dimensions/defaults; experiments; config versions and actual action/body limits.
- Exercise one-bulk rollback, errors after commit, dropped responses, SDK dispatch count/timeouts and whether an old timed-out request can commit later. Test ordered-array hash collisions and exported context ordering.
- Record supported capabilities and refusals by exact server/client pair. Checked-in 0.102.0 production and 0.115.5 local descriptions are leads to recheck, not live certification.
- Keep unknown CAS/fencing or writer permissions explicitly unsupported. Mocks may develop the harness but cannot certify a server.

**Checks / done:** T10/T14/T16–T19/T22 evidence for each certified disposable build; clear unsupported matrix for missing builds. Nothing is uploaded to a shared/hosted workspace. No global seed/reset/cleanup operation.

**Suggested commit:** `test: certify required-fields Superposition API contracts`

```text
Complete Step 2 of docs/superposition_required_fields_sync_implementation_playbook.md.
Implement the opt-in API contract harness and capability evidence using the locked client and exact server builds.
Before any test writes, prove the target is the explicit disposable fixture workspace with synthetic data and scoped cleanup.
Test pagination, preservation-sensitive writes, prerequisites, transaction boundaries, post-commit errors,
ordered-array identity, experiments, limits, retries and unknown outcomes described in the step.
If a required image/environment is unavailable, implement isolated harness work and record certification as pending.
Do not use hosted credentials or claim mock results certify the server. Report commands/results and suggest a commit.
Do not stage or commit. Stop for my review.
```

### Step 3 — define pure inputs, canonical artifacts and build boundaries

**Goal:** make rules reproducible and release-bound before adding transport.

**Needs:** Step 0 and Step 1's input/label contract; plan §6.1–6.2, §6.9, P1. Unsettled field choices remain explicit refused values.

**Work:**

- Add renderer types next to H `crates/payment_methods/src/configs/settings.rs`, with a proposed `configs/required_fields_renderer.rs` module and exports through the current module structure.
- Define `RenderInputs`, `ContextKey`, `ManagedValue`, `DesiredState`, input/build identities and strict mapping/catalog/exclusion schema. Define router-side plan/report boundaries without pulling transport into payment_methods.
- Reuse settings selection/environment precedence while projecting only relevant nonsecret inputs. Do not initialize AppState, DB, queues or the serving provider for rendering.
- Version canonical encoding/hashes. Sort maps and proven source sets; preserve ordered vectors in both output and retained input hashes. Exclude volatile timestamps and secret settings.
- Add an opt-in `required_fields_sync` build boundary with explicit v1 support and unsupported writer/v2 handling; preserve ordinary serving builds.

**Checks / done:** T02–T04/T25/T33 appropriate to this foundation; same inputs hash identically across shuffled maps/sets, changed relevant inputs invalidate identity, no external reads. Record actual feature/build commands and unavailable build prerequisites.

**Suggested commit:** `feat: define required-fields sync inputs and artifacts`

```text
Complete Step 3 of docs/superposition_required_fields_sync_implementation_playbook.md.
Implement pure input/output types, strict policy/catalog parsing, canonical hashes and the opt-in build boundary.
Reuse current RequiredFields and Settings abstractions; capture relevant effective inputs without secrets or AppState startup.
Preserve ordered lists and normalize only proven unordered source collections, including retained input hashing.
Add/run the foundation tests and supported build checks; report missing feature/dependency evidence honestly.
Do not add live reads, mutation adapters or source-data corrections. Suggest the commit and stop for review without staging/committing.
```

### Step 4 — implement the deterministic renderer

**Goal:** turn compiled rules and explicit inputs into exact connector contributions.

**Needs:** Steps 1 and 3; approved mapping choices for affected inputs; plan §6.3 and P2. Server access is unnecessary.

**Work:**

- Render `RequiredFields::new(effective_bank_config)` using exhaustive method/type/connector and `FieldType` handling. Preserve common versus mandate/non-mandate conditions and exact scalar types.
- Emit true for present fields; absence removes this owner's contribution, not an invented false. Preserve explicit options even when equal to defaults. Keep missing, empty and unrestricted intent distinct.
- Implement exact scoped aliases and versioned ordered country expansion from the input catalog. Refuse unapproved mappings, invalid comma countries, unsupported options and duplicate output keys before insertion.
- Track outer map keys for diagnostics/legacy compatibility, while metadata contracts determine SDK bindings. Handle empty groups, feature absence and dummy exclusions without implying retirement permission.
- Return useful diagnostics and a canonical bundle for explicit local profiles. Any necessary business-data correction is a separate reviewed change with legacy/SDK acceptance; do not silently skip a bad rule to produce a successful full bundle.

**Checks / done:** T01–T10 where applicable; deterministic cross-process output, explicit bank-profile variation, all variants, reviewed aliases, duplicates and inheritance. A full supported-profile render has no unresolved mapping diagnostics; a blocked profile reports why and is not marked complete. Seed equality is supplementary evidence only.

**Suggested commit:** `feat: render required-fields Superposition contributions`

```text
Complete Step 4 of docs/superposition_required_fields_sync_implementation_playbook.md.
Implement the pure renderer using existing compiled RequiredFields plus explicit settings, policy, catalog and exclusions.
Follow the reviewed mapping/SDK fixtures; preserve ordered options and explicit default-equal values.
Reject duplicate bindings, unsupported choices and malformed data rather than normalizing or skipping them.
Add/run independent unit/invariant tests and render explicit input profiles without network access.
Keep business-source corrections separate and report any profile blocked on approval.
Do not change the committed seed yet. Report changes/checks and suggest a commit; do not stage/commit. Stop for review.
```

### Step 5 — implement complete, read-only inventory

**Goal:** make partial or lossy reads impossible to mistake for absence.

**Needs:** Step 3 types; Step 2's relevant read contracts; plan §6.4–6.6 and P3. Implement test adapters while certification is pending, but do not claim a live inventory is complete.

**Work:**

- Add router sync inventory/types and a dedicated administration read adapter using the actual SDK/API contracts. Do not use the serving cache or file fallback for inventory.
- Read unfiltered full contexts, defaults, dimensions, workspace settings and experiment references. Follow every resource's pagination contract, validate totals/IDs/boundaries, and bound bytes/items/time.
- Preserve complete raw rows and unknown metadata alongside typed projections; avoid SDK serialization that drops fields needed for restoration.
- Require a completeness proof before constructing `CompleteInventory`. Permission-scoped reads, missing pages, inconsistent repeated reads, malformed responses and exhausted limits are errors, not empty results.
- Use returned server IDs and typed conditions. Never recreate a guessed server hash as client identity. Read-only plans remain provisional until apply revalidates under exclusion.

**Checks / done:** T13/T14/T16; dropped/duplicate pages, changed totals, filters, insufficient permissions, malformed conditions, unsupported predicates, repeated IDs and unknown metadata. No mutating calls or locks are made by inventory code.

**Suggested commit:** `feat: inventory required-fields configuration without loss`

```text
Complete Step 5 of docs/superposition_required_fields_sync_implementation_playbook.md.
Implement bounded direct unfiltered inventory with complete raw row preservation and a validated CompleteInventory type.
Use certified pagination/permission contracts and server-returned IDs; reject partial/inconsistent/filtered data.
Cover context/default/dimension/workspace/experiment discovery and malformed response cases with focused tests.
Keep cached serving reads and mutation methods out of this adapter. Report pending real-server certification accurately.
Do not contact hosted environments unless separately named and authorized. Suggest a commit, leave the index untouched, and stop for review.
```

### Step 6 — implement ownership, preflight and semantic planning

**Goal:** produce a reviewable, nonmutating plan that preserves manual content and exposes real user impact.

**Needs:** Steps 1, 4 and 5; certified server ordering/projection from Step 2; plan §6.3–6.7, §7.4 and P3.

**Work:**

- Add pure ownership/planner/impact modules in the proposed router sync module. Classify granted fully managed/shared rows, unowned candidates, excluded resources and unrelated contexts.
- Require target-bound adoption grants; future matching manual rows are not automatically adopted. For pending creates, define ownership intents that later survive a lost response. Existing descriptions are preserved; adoption needs no marker write.
- Construct complete replacement payloads from unrelated keys plus desired owned attributes. Delete only an approved fully managed obsolete row; never send an empty replacement. Exclusion/feature absence is not retirement authority. Refuse an empty desired set for an active owner; retirement requires an explicitly reviewed migration manifest.
- Check live/projected dimension vocabulary, compatible schemas/functions and presentation paths/kinds/options. Distinguish missing managed defaults eligible for declared creation from missing manual presentation prerequisites. Reject experiment references and unexplained overlaps.
- Analyze before/after inheritance and effective requiredness/options, including country/tenant rules, explicit default-equal values, connector permutations and duplicate confirm paths. Use Step 1's SDK case interface; unknown projection or exhausted analysis bounds refuse.
- Include complete inventory, target/instance, release intent, bundle, grants, SDK/server contracts and exact semantic approvals in the plan hash. Reject known failures before any write; no generic force or skipped desired item.

**Checks / done:** T07–T15/T22/T32/T35–T37; a valid seed with missing live metadata refuses; shared attributes/descriptions remain byte/value-equivalent where required; identical input gives an empty action list; broad reads cannot imply ownership. Test the server option-order collision behavior certified in Step 2.

**Suggested commit:** `feat: plan required-fields changes with explicit ownership`

```text
Complete Step 6 of docs/superposition_required_fields_sync_implementation_playbook.md.
Implement ownership grants, complete-row preservation, prerequisite checks and the pure semantic planner.
Evaluate inherited and browser-effective changes using the certified projection/case contract, including connector ordering.
Reject incomplete discovery, unsupported mappings/metadata/functions, experiments, ambiguous identity and unapproved impact.
Test unchanged planning, shared-row preservation, default inheritance and missing live metadata despite a valid seed.
Produce sanitized actionable reports with bound hashes; do not dispatch any writes or acquire a lease.
Report changes/checks and remaining gates, suggest a commit, and stop without staging or committing.
```

### Step 7 — expose the offline and read-only command slice

**Goal:** make the first working slice usable and ready for early review.

**Needs:** Steps 4–6; plan §6.9/P6; current `CmdLineConf` and router initialization conventions.

**Work:**

- Add only `render` and read-only `plan` dispatch in the existing router binary; add read-only `status` once its journal reader exists. Preserve ordinary invocation and choose strict command/settings validation.
- Dispatch before unrelated application validation, secret decryption, AppState, DB/Redis/queues and serving-provider initialization. Reuse the needed settings projection instead of introducing a second config loader.
- Offline render must have no transport construction. Plan uses a read-only target identity, no lease, no prerequisite writes and no description stamping. Mark plans provisional.
- Enforce target allowlisting, endpoint/header handling, credential redaction and relevant request bounds. Unsupported builds and unresolved mapping/catalog inputs fail clearly.
- Write a synthetic local walkthrough and a local draft PR title/body covering completed scope, tests and remaining steps. Do not add a placeholder apply command that reports success.

**Checks / done:** T25/T33 plus renderer/planner tests; commands work without payment-service infrastructure; plan dispatch count contains zero mutations; ambient bank overrides cannot corrupt a named seed profile. No hosted access is implied by this CLI's existence.

**Suggested commit:** `feat: expose required-fields render and planning commands`

```text
Complete Step 7 of docs/superposition_required_fields_sync_implementation_playbook.md.
Wire the existing router binary's offline render and read-only plan commands before unrelated application initialization.
Keep serving behavior intact and enforce target/config validation, explicit profiles, redaction and zero-write planning.
Test the CLI with synthetic inputs/disposable fixtures, including unavailable DB/Redis and rejected builds.
Prepare a local draft PR description and walkthrough for the renderer/planner review checkpoint.
Do not implement mutating command wiring, stage, commit, push or open a remote PR. Suggest the manual commit and stop for review.
```

**Human checkpoint:** review the renderer/planner slice and manually commit/push/open a draft when ready. Continue contract/platform work while it is reviewed. A useful dry run does not establish mutation safety or environment adoption.

### Step 8 — persist recovery information before requests

**Goal:** make recovery possible when the executor dies or never receives a response.

**Needs:** Steps 3 and 6; plan §7.3/P4; Q6 storage decision for real adapter certification. Local fake storage is useful for unit tests, not durability evidence.

**Work:**

- Implement `RunJournal` and immutable prepared snapshots, append-only request intents/observations and durable receipts. Verify checksums through read-back before authorizing dispatch.
- Store target/instance/release/build/input identity; complete before inventory and metadata; exact after images/request bytes; ordering, grants/approvals/contracts; pending ownership and the first-adoption baseline.
- Define the unresolved-run record used by Step 9. Retain enough evidence to reconstruct a request even if the Job, stdout and final report disappear.
- Implement/test chosen storage semantics explicitly; ordinary existing upload support is not proof of immutability or retention. Separate restricted snapshots from sanitized reports; redact credentials.
- Document retention at least through the approved rollback window, with the plan's proposed 90-day floor subject to owner approval. Protect baseline, active and unresolved artifacts from expiry; define independent recovery access.

**Checks / done:** T18/T23/T24 journal cases; failure before/after intent, corrupt/read-back-denied objects, overwrite/delete attempts and independent reader recovery. Storage certification remains pending until verified on an explicitly scoped test store. No Superposition apply is enabled.

**Suggested commit:** `feat: journal required-fields plans and request intentions`

```text
Complete Step 8 of docs/superposition_required_fields_sync_implementation_playbook.md.
Implement immutable before images, exact request-intent journaling, durable receipts and recovery metadata.
Reuse existing storage infrastructure only where its actual semantics satisfy the contract; test missing guarantees explicitly.
Keep secrets out of artifacts/reports and retain pending-create ownership plus the first-adoption baseline.
Add/run fault-injection and independent recovery-reader checks using isolated storage; report external retention/access gaps.
Do not enable apply or use hosted storage without separate scoped authorization. Suggest a commit and stop without staging/committing.
```

### Step 9 — enforce release authority and exclusive execution

**Goal:** coordinate every mutating entry point and prevent stale plans or competing writers from overwriting shared state.

**Needs:** Steps 6 and 8; plan §7.1–7.2/§8.2–8.3/P4; Q3/Q4 for real coordinator/writer-barrier certification.

**Work:**

- Implement target registry/release-intent contracts, one coordinator per backing instance/workspace, owner-checked lease acquire/renew/release and the durable active-run record.
- Check image/input identities and monotonically increasing intent generations, including rollback to an older image. Model all relevant readers/regions; release-name suffixes and one healthy pod are insufficient authority.
- Require a positive external-writer barrier or certified conditional-write strategy. A successful lease test does not protect against UI/API/router/seeder writes. Missing enforcement refuses apply.
- On lease/authority loss, stop dispatch, retain unknown work and require old-executor/request exclusion before takeover. Lease expiry alone cannot authorize a successor.
- Define rollout evidence for actual Rollout/Deployment resources, traffic endpoints, scaling and compatible co-readers. Keep platform-specific adapters narrow and bound by target identity.

**Checks / done:** T20/T21/T25/T26/T34; suspended executor, renewal failure, mismatched owner release, stale plans, delayed jobs, cross-region sharing and an external edit. Real enforcement/rollout proof needs its actual platform; no always-success fake in a production path.

**Suggested commit:** `feat: coordinate required-fields execution by target and release`

```text
Complete Step 9 of docs/superposition_required_fields_sync_implementation_playbook.md.
Implement target/release authority, renewable owner-checked coordination and durable unresolved-run protection.
Require certified external-writer exclusion; do not equate a sync lease or pre-write re-read with conflict prevention.
Handle stale intents, mixed traffic versions, scaling, lease loss and old requests that may still commit.
Test supported adapters in explicit disposable infrastructure; keep unavailable real controls disabled/refusing.
Use the same interfaces for future apply, resume and restore. Report actual evidence/gaps and suggest a commit.
Do not deploy or stage/commit. Stop for review.
```

### Step 10 — implement the bounded apply state machine

**Goal:** execute a validated plan with honest outcomes at real transaction boundaries.

**Needs:** Steps 2, 6, 8 and 9; plan §6.6–6.7/P5. This is internal, disabled engine work; hosted activation waits for Steps 11–17.

**Work:**

- Add the administration mutation adapter and apply state machine. Validate everything before the first write; persist snapshot and request intent before every prerequisite or context dispatch.
- Check current authority/lease/barrier before each dispatch. Observe each prerequisite result, then send one bounded complete-row-preserving context bulk. Reject excessive size/action counts before mutation; no implicit batching.
- Set explicit deadlines and one mutation dispatch attempt; certify actual SDK behavior. Bounded read retries must restart inconsistent inventories.
- Implement all plan outcomes: `NoChangeVerified`, `NoChangeUnverified`, `Refused`, `RejectedNoChange`, `AppliedVerified`, `PartialApplied`, `CommitUnknown`, `AppliedUnverified`, with the plan's exact exit mapping and precedence.
- Treat lost responses, post-commit errors and cancellation after dispatch as unknown until observed. Earlier prerequisite commits survive context rollback. Missing verification must never produce a verified outcome.
- Preserve exact request bytes after journaling. Stop on lease loss, journal failure, divergence or newer intent; maintain durable unresolved evidence rather than automatically replaying.

**Checks / done:** T12/T16–T21/T23/T32; whole-plan refusal, second-run zero mutations including markers, prerequisite-only progress, one-bulk rollback, post-commit 512, lost response, executor crash and journal failure. Actual server tests use Step 2's isolated target. All production mutation dispatch paths require real coordinator/journal capabilities; no public/hosted activation yet.

**Suggested commit:** `feat: apply required-fields plans with recoverable outcomes`

```text
Complete Step 10 of docs/superposition_required_fields_sync_implementation_playbook.md.
Implement the disabled apply engine and certified mutation adapter using existing planner, journal and coordinator interfaces.
Journal before every request, dispatch prerequisites separately and contexts in one bounded transaction, and disable automatic mutation retries.
Implement every documented outcome/exit mapping, including unknown commits and failed verification after successful writes.
Exercise fake fault injection and certified disposable-server cases; missing verification cannot report success.
Do not expose hosted apply or bypass missing safety adapters. Report results and remaining gates, suggest a commit, and stop without staging/committing.
```

### Step 11 — verify the full serving path

**Goal:** distinguish persisted success from the configuration users actually receive.

**Needs:** Steps 1, 2, 6 and 10; plan §6.8/P5; exact SDK/assets and backup evidence for dependent external checks.

**Work:**

- Implement V1 direct full-row/default/dimension and exported/versioned-config comparison, including preserved data and server-cache discrepancies.
- Implement V2 existing authenticated SDK-config checks for every declared relevant traffic-eligible router pod. Compare content fingerprints, since a response/version header or load-balanced sample alone does not prove cache convergence.
- Package Step 1's actual W/U harness as a pinned read-only companion, proposed under W `playwright-tests/required-fields-verifier/`. Bind cases/results to SDK/submodule/asset and response hashes. Run it on projected before/after state during preflight and actual responses afterward.
- Keep browser execution offline with provided assets/stubbed API responses and intercepted confirm. Do not mount writer credentials or blanket service-account tokens into the companion. Pod-scoped NetworkPolicy is not per-container isolation; test actual credential boundaries.
- Implement V4 backup target/content verification and a non-serving startup canary with primary unavailable. Running-router last-good memory and startup file fallback are separate cases.
- Exercise fresh and retained browser snapshots, country/state assets, supported reader versions and actual dimensions/eligible connector orders. Report missing cases/artifacts and deadlines instead of substituting Rust resolution or empty diffs for browser results.

**Checks / done:** T07–T10/T28–T30/T34–T38; stale cache, absent metadata, missing/mismatched verifier result, browser option/payload errors, stale or wrong-target backup, startup failure and retained sessions. Configure deadlines from measured/certified behavior; 15-second polling is not a freshness guarantee. Unavailable V3/V4 proof prevents verified completion.

**Suggested commits:** H `feat: verify required-fields serving and fallback state`; W `test: package pinned required-fields browser verification`.

```text
Complete Step 11 of H docs/superposition_required_fields_sync_implementation_playbook.md.
Implement V1–V4 and package the actual pinned W/U browser verifier using the Step 1 case/report contract.
Bind reports to exact response, input and artifact hashes; test projected preflight and actual post-apply responses.
Verify per-pod caches, actual browser construction/aggregation/rendering, old snapshots and backup startup separately.
Use synthetic provisioned inputs and intercepted confirm with no live payment submission or companion writer credentials.
Missing artifacts/platform access must remain failed or pending verification, never successful stubs.
Run available isolated checks, report each level's evidence and gaps, suggest per-repository commits, and stop without staging/committing.
```

### Step 12 — implement resume, restore and pre-feature rollback

**Goal:** recover without replaying uncertain requests or overwriting newer manual content.

**Needs:** Steps 8–11; plan §6.7/§10.2/§10.4/P5 and baseline retention/recovery identity contracts.

**Work:**

- Implement resume as observation and replanning: classify requests as before-state, expected after-state or divergence. Before-state permits retry only after proving the old executor/request cannot still commit.
- Record confirmed after-state without reissuing; preserve unknown state and block takeover if evidence is insufficient. Replan remaining actions against a fresh complete inventory and current release intent/approvals.
- Build a three-way inverse plan from recorded before/after and current state. Restore owned contributions, preserve current unrelated attributes/descriptions, and refuse divergent owned edits or wrong target/instance/snapshot.
- Account for context recreation changing IDs/order. Additive enum/default prerequisites normally remain; restoring contexts is not full workspace rollback.
- Retain a recovery-capable image and first-adoption baseline so a rollback to an application predating the command remains recoverable. Use a new intent generation and current reader compatibility checks.
- Use the same authorization, coordinator/barrier, journal, impact and V1–V4 gates for applying restore/resume as for apply. No reset mode.

**Checks / done:** T18–T24/T27; original executor gone, lost create response/pending ownership, current unrelated edits, divergent owned edits, expired lease, wrong snapshot target, additive residuals and rollback to a pre-feature app. Restoration itself is journaled before writes.

**Suggested commit:** `feat: resume and restore required-fields synchronization`

```text
Complete Step 12 of docs/superposition_required_fields_sync_implementation_playbook.md.
Implement observation-first resume and three-way restoration using durable before/after/request evidence.
Never retry an unresolved request until the old executor/request cannot commit later; replan against current state.
Preserve current unrelated data, refuse divergent owned edits and retain additive prerequisite changes explicitly.
Support a retained recovery image and first-adoption baseline for pre-feature application rollback.
Test recovery failures and enforce the same gates as apply. Do not run hosted restore or add reset/force shortcuts.
Report results/gaps and suggest a commit; do not stage or commit. Stop for review.
```

### Step 13 — finish command wiring, attestations and operational reports

**Goal:** provide one executor for automation and operators, with no weaker manual path.

**Needs:** Steps 7–12; plan §6.9/§7.5/P6; actual build-feature and image provenance contracts.

**Work:**

- Extend the existing command family with `status`, `apply`, `resume`, and `restore` planning/application modes. Keep `seed` for Step 14. Hosted mutation requires explicit command, opt-in mode and completed target gates; defaults remain disabled.
- Bind execution to the registered target, instance generation, immutable settings/policy, image/build/features and intent. Reject arbitrary endpoint overrides, unapproved redirects, unknown fields and unsupported v2 writer combinations.
- Emit nonsecret per-pod build/input/desired-state attestations for rollout checking; report facts that the platform can verify rather than trusting the executable's version string alone.
- Expose exact outcome/exit mappings from plan §6.7, structured restricted reports and bounded-cardinality metrics. Retain raw errors internally with request phase/status; sanitize public summaries and secret-bearing values.
- Distinguish app rollout, sync and verification status. Unknown commits, partial application, verification failure and stale backups must reach the platform's collector even after the one-shot Job exits.

**Checks / done:** T25–T27/T32–T34; offline/plan/status no writes; every mutating command invokes identical safety gates; disabled/default configuration refuses hosted writes; secrets are absent from logs/reports. Normal router serving and supported feature variants remain intact.

**Suggested commit:** `feat: wire required-fields execution and release attestations`

```text
Complete Step 13 of docs/superposition_required_fields_sync_implementation_playbook.md.
Finish router command wiring for status/apply/resume/restore with all implemented safety and verification gates.
Keep defaults disabled, validate actual target/build/input/intent identity and preserve ordinary router startup.
Add nonsecret release attestations, exact exit/report outcomes and bounded metrics suitable for a one-shot executor.
Test every entry point, including restore planning versus applying and denied/unsupported configurations.
Do not add an HTTP sync route or enable any hosted target. Report changes/checks and suggest a commit; stop without staging/committing.
```

### Step 14 — generate local seed content and integrate CI

**Goal:** prevent Rust/seed drift without making the generator its own semantic oracle.

**Needs:** Steps 1, 4, 6 and 7 for offline work; Steps 11/13 for full browser/command CI; plan §3.3/P7.

**Work:**

- Implement offline `seed` and `seed --check` using the named local bank/feature/catalog/exclusion profile. Reject ambient input pollution and preserve unrelated parsed seed content and formatting where practical.
- Generate only the approved managed subset and catalog-declared prerequisites. Existing descriptor/default changes or business-data fixes need separate review; initial migration diff is explicit.
- Preserve country/baseline/tenant content and the seeder's independent Deja behavior. Do not invoke the uploader as a generation/check side effect.
- Wire focused renderer/planner checks, independent SDK fixtures, supported feature builds and opt-in server contracts into existing CI conventions. Verify current compose/workflow images rather than changing versions just to pass.
- Pin W/U and the verifier artifact; avoid remote-tracking lifecycle scripts/default live Playwright project. Generated seed equality and independently expected effective outcomes must both pass.

**Checks / done:** T01–T10/T29/T31/T33/T35–T38; check mode leaves tracked files untouched, intentional ordered vectors remain ordered, unrelated seed content is preserved, a semantically wrong fixture can fail despite generator/seed agreement. Uploader regression runs only in an explicit disposable target after inspecting effects.

**Suggested commit:** `ci: check generated required-fields configuration and SDK semantics`

```text
Complete Step 14 of docs/superposition_required_fields_sync_implementation_playbook.md.
Implement offline seed generation/checking for the explicit local profile and wire independent semantic/build checks into CI.
Preserve manual seed content and existing Deja behavior; review initial managed changes without hidden data corrections.
Keep generation separate from upload, pin W/U verification inputs and use hermetic browser tests.
Test determinism, unrelated-content preservation and check-mode cleanliness; run uploader tests only in the registered disposable target.
Report any source/catalog approval blocking a complete seed and suggest separated commits where needed.
Do not stage/commit, change hosted state or silently update server versions. Stop for review.
```

### Step 15 — add disabled deployment integration

**Goal:** prepare the actual deployment executor and its permissions, without activating a target.

**Needs:** Steps 8–13; plan §8/P8; I instructions and selected external Helm/platform sources. Missing chart access blocks template acceptance; do not invent a template path or claim a values-only change deploys the Job.

**Work:**

- Add a reviewed target registry and disabled values wiring in I; identify actual backing instance/workspace, one writer, compatible readers, coordinator, secret references, journal location and certified contract IDs.
- In the real selected chart, add the same-router-image post-rollout Job and pinned read-only SDK companion. Pass identical immutable relevant settings/policy inputs; test both automatic and manual Argo sync paths.
- Check all traffic-serving destinations, rollout generations, scaling and co-readers. Jenkins publishes image/build/intent evidence; it does not run a competing reconciler. InfraSwitch remains off until equivalent executor/rollout evidence exists.
- Prepare least-privilege identities, per-container credential mounts, network/storage access and metrics/alerts. Definitions are not effective permission evidence; record tests needed for the actual cluster.
- Define Job cleanup without deleting recovery data, independent recovery image retention, backup content identity and fallback canary integration.
- Keep v2, custom/playground, OLAP and unverified regional releases disabled as writers. Shared regions use one target authority/coordinator, not independent regional locks.

**Checks / done:** T20/T21/T23/T26–T30/T34 as local render/static or isolated platform tests permit. Render the actual chart and check Job lifecycle/settings/permissions. No deployment is performed; access, barrier and rollout evidence remain pending until exercised in an authorized test environment.

**Suggested commits:** I `feat: configure disabled required-fields deployment jobs`; chart repository `feat: add required-fields sync job templates` (adjust to actual changes).

```text
Complete Step 15 of H docs/superposition_required_fields_sync_implementation_playbook.md.
Inspect I and the actual selected Helm/platform source, then add disabled target-registry, Job/companion and deployment wiring.
Bind exact release image/settings/intent, one target authority, writer barriers, journal retention and verification/reporting.
Account for shared regions/readers and manual Argo syncs; keep all unverified writers and InfraSwitch paths disabled.
Render/check only locally or in an explicitly authorized disposable test setup. Do not deploy, retrieve production secrets or enable apply.
If external source/access is missing, finish independent definitions and record blocked acceptance precisely.
Report changes and checks per repository; suggest manual commits and stop without staging/committing or remote PR actions.
```

### Step 16 — run integrated failure drills and prepare the handoff

**Goal:** establish implementation readiness with real failure evidence before adoption.

**Needs:** Steps 10–15; explicitly identified isolated test environments; plan §10–11/§14. Existing test permission does not extend to a hosted shared workspace.

**Work:**

- Run the supported feature/build matrix and T01–T38 checks appropriate to available components. Use explicit disposable server/storage/coordinator resources; maintain owned cleanup and artifact records.
- Exercise first adoption, repeat zero-write apply, required metadata absent despite a valid seed, shared-row preservation, experiment rejection, incomplete reads and semantic-approval refusal.
- Kill/drop responses around each journal/prerequisite/context boundary; demonstrate unknown-outcome observation, external-writer exclusion, lease loss, competing/stale release refusal and no blind SDK retries.
- Exercise mixed-version/region/readers, old browser snapshots, per-pod caches, backup publication/fallback and pre-feature application rollback using the retained recovery executor.
- Review all mutation paths and security/reporting boundaries. Fix scoped failures, distinguish existing baseline issues and keep any unrelated correction in a separate review group.
- Prepare a sanitized runbook and local PR handoff with architecture, approved contracts, exact checks/results, known limitations, disablement and recovery procedures. Keep full live/test secrets and restricted inventories out of Git.

**Checks / done:** relevant plan §14.1 gates have evidence; missing platform/SDK evidence stays pending. A compiled binary and passing mock suite are not complete transaction/rollout certification. Record actual commands/artifact hashes and show an independent executor recovering after the original Job disappears.

**Suggested commit:** `test: verify required-fields synchronization and recovery`

```text
Complete Step 16 of docs/superposition_required_fields_sync_implementation_playbook.md.
Run the integrated test/failure matrix on explicitly verified isolated resources and record evidence per acceptance gate.
Exercise lost responses, partial prerequisites, lease/external-writer races, shared-row restore, mixed readers and fallback.
Test the actual server and browser contracts where available; do not turn missing external checks into passed results.
Make focused fixes, review changed files for secrets/unrelated changes and prepare the implementation runbook/local PR handoff.
Do not access production, deploy, enable hosted apply, stage, commit or open remote PRs.
Suggest fix-specific/manual commit groups and stop for review.
```

### Step 17 — prepare one target's adoption packet

**Goal:** make initial environment adoption a concrete operator-reviewed action with recovery ready.

**Needs:** Step 16, plan §9/§14.2–14.3/P9 and the selected target's Q gates. First candidate is one confirmed independent integ target; its name alone does not establish identity or authorization.

**Work:**

- Identify the exact target/instance/workspace, authoritative release/intent/image/settings, all readers/regions, permitted identity, complete inventory and actual server/SDK/asset contracts. Use only already authorized read-only access; otherwise prepare the packet structure and pending evidence list.
- Review structural candidates, explicit ownership grants, dummy exclusions, semantic changes and active experiment overlaps. Review every initial removal/options/casing/catalog change; historical seed counts are not adoption forecasts.
- Arrange approved presentation promotion through the responsible configuration workflow, then verify actual live prerequisites. Preparing this packet does not itself promote metadata.
- Produce the exact read-only plan, hashes, narrowly scoped approval manifest, durable baseline/journal readiness, rollback intent, retained recovery image and recovery access evidence.
- Prepare version-correct operator commands from the implemented CLI help, including first apply, verification, second run, disablement, unknown-outcome diagnosis and restore planning. Flag commands as future actions; never fill unknown target/credential values with plausible defaults.
- Include escalation owners and independent application/sync/V1–V4 status. Do not authorize automatic rollback over an unknown commit.

**Checks / done:** complete target-specific packet and clearly marked pending gates; no hosted writes from this step. Initial application must be followed by V1–V4 evidence, a zero-write second run and recovery drills before promotion. Sandbox and each production target require their own evidence or an explicitly shared-target authority.

**Suggested commit:** `docs: prepare required-fields target adoption and recovery runbook` (commit sanitized policy/runbook only; restricted inventory/approvals belong in approved storage).

```text
Complete Step 17 of docs/superposition_required_fields_sync_implementation_playbook.md for the target I identify.
Prepare a concrete adoption packet using existing explicit read-only authorization and the implemented command contracts.
Verify target authority, complete inventory, ownership/impact review, live metadata, exact reader artifacts and recovery readiness.
If no target/access is supplied, prepare the sanitized packet template and exact missing evidence without contacting a hosted service.
Provide reviewed commands and pass/fail gates for first apply, V1–V4, second run, disablement and recovery.
Do not deploy, promote metadata, apply/restore configuration or enable automation from this preparation prompt.
Separate restricted artifacts from commit-ready documentation, suggest manual commits, and stop for operator review.
```

**Operator execution checkpoint:** Deepanshu/the authorized release owner performs the reviewed adoption using the normal release process, or gives a separate explicit instruction naming the target, operation and approved packet. The coding prompts above do not supply that authorization. Record actual results before marking adoption complete. Resolve any `CommitUnknown`, `PartialApplied` or verification failure before another release or restoration proceeds. Promote integ → sandbox → production target by target; never infer one region's success covers another.

## 5. External prerequisites and what can continue meanwhile

A missing answer should stop the dependent behavior, not trigger a fresh investigation of everything or a guessed production guarantee. Keep the plan's Q register updated with evidence, owner and blocked milestone.

| Question | Needed evidence/decision | Work that can continue | Gate that stays closed |
|---|---|---|---|
| Q1: supported SDKs and mapping acceptance | Exact deployed web/mobile/U and asset identities; independently accepted bindings/options/card/connector-order behavior | Local pinned SDK fixtures; pure types; approved renderer subsets and diagnostics | Affected mapping acceptance and target V3/adoption |
| Q2: actual server contract | Exact image/client certification, API semantics/limits, ordering/hashing, experiments and unknown-request handling | Harness, pure renderer, fake-adapter tests | Certified adapter use and hosted apply |
| Q3: target sharing and authority | Backing instance/workspace, every relevant release/region/reader and monotonic intent issuer | Registry schema and authority unit tests | Real target registration/rollout gate |
| Q4: access and writer exclusion | Effective complete-read/write rights, protected target path, enforced competing-writer protection and old-request exclusion | Negative tests and adapter interfaces | All hosted mutations, including restore |
| Q5: actual Helm/platform source | Selected chart templates, Job lifecycle, settings projection and rollout/traffic checks | Independent values/registry/report definitions | Deployment integration acceptance |
| Q6: durable recovery store | Immutable read-back receipts, retention/encryption/access and independent recovery role | Journal types/fault tests | First mutation using that store |
| Q7: backup implementation | Actual target/content identity, publication semantics and startup fallback evidence | V4 interfaces and local failure cases | V4 verified result and recovery-ready adoption |
| Q8: live presentation promotion | Reviewed field-kind descriptor/default contracts, owners and actual workspace content | Seed fixtures and missing-metadata refusal tests | New-field activation/live preflight |
| Q9: build/config attestation | Actual supported feature closure, image digests and immutable effective inputs | Build manifest schema and local feature checks | Release-bound automatic apply |
| Q10: InfraSwitch/Euler | Same-image one-shot executor, target/credential path and full rollout evidence | Argo/local implementation | Euler writers only |
| Q11: ownership and business migration | Initial grants, dummy/retirement policy and explicitly accepted source corrections | Candidate inventory and scoped impact reports | Adoption/destructive or ambiguous changes |

A signed-off mapping is not permission to change live configuration. A passing local API harness is not proof of production access. Existing session authorization may be reused within its exact scope; elapsed time, repository configuration and a copied prompt do not supply missing authorization.

### Keep business-data corrections reviewable

If the Nordea comma value, a Rust issuer path, bank-type defaults, country catalog or SDK aggregation/card behavior needs correction:

1. Show the specific before/after data and source/consumer evidence.
2. Identify affected legacy, SDK and deployed-reader behavior; write independent expected cases.
3. Make the correction in a separate explicitly requested change with an appropriate owner/review record.
4. Re-render/replan and review the new migration diff. Do not hide the correction in generated seed changes or relax a schema/enum to legitimize it.

While unresolved, fail the affected whole plan or use an explicitly reviewed exclusion with no implied deletion/handoff. Do not silently skip the bad item and report a complete apply.

## 6. Check and reporting discipline

The prompts request their named tests during future implementation. During creation of this guide, only documentation inspection/validation is performed; no feature tests or environment operations are implied.

Use the test commands selected against current manifests in Step 0. Inspect setup and effects first, run narrow tests/builds, and expand to the declared feature/integration matrix at the appropriate gate. The old required-field dump writes files and uses empty bank settings; it is not a substitute for the explicit renderer harness. W's default test command is live in the inspected version; a command named test is not automatically isolated.

| Test coverage from the plan | Primary steps |
|---|---|
| T01–T06: mapping, options, input determinism | 1, 3–4, 14 |
| T07–T10: inheritance, SDK difference, server order/hash | 1–2, 4, 6, 11 |
| T11–T15: ownership, preservation, identity, discovery, live metadata | 5–6, 10, 14, 16 |
| T16–T19: API, transaction, lost response and retries | 2, 8, 10, 12, 16 |
| T20–T24: coordination, external writers, experiments, journal and restore | 2, 6, 8–12, 15–16 |
| T25–T27: command/security, rollout/intent and pre-feature rollback | 3, 7, 9, 12–13, 15–16 |
| T28–T30: router, actual SDK and fallback | 1, 11, 15–17 |
| T31–T34: seed/CI, impact/reports, build matrix and deployment integration | 3, 6–7, 10, 13–16 |
| T35–T38: actual browser dimensions, aggregation, typed options and lifetime | 1, 6, 11, 14, 16–17 |

### Required handoff after every step

```text
Step:
Repository/revision(s) and scope:
What changed and why:
Changed files, grouped by repository:
Commands actually run and results:
Independent expected cases exercised:
Checks not run, why, and which acceptance gate they block:
Source-backed findings versus remaining assumptions:
External prerequisites/owner/evidence needed:
Acceptance status (implemented / checked / externally certified are separate):
Recovery or compatibility implications:
Suggested manual commit message(s) and file group(s):
Next eligible step:
```

Update the progress/evidence register with links to reports, relevant case IDs and actual revisions. Keep restricted snapshot locations as opaque references in public reports. Do not mark a gate passed from an earlier version's results without showing that the relevant inputs/contracts are unchanged.

**No automatic continuation:** once a step's local work is reviewable, stop with this handoff. If a prerequisite prevents dependent work, finish the independent portion, report the precise gap and identify an eligible independent step; do not implement a later step without being asked.

## 7. Completion and activation checklist

### Offline and read-only milestone

- [ ] Current source/build evidence and P0 decisions are recorded; original documents and unrelated work are preserved.
- [ ] Independent SDK/mapping fixtures exist; source-only, resolver-only and browser execution evidence are distinguished.
- [ ] Explicit effective input profiles and deterministic canonical bundles work; ordered lists and hashes are stable.
- [ ] Renderer diagnostics block unsupported mappings/data; source corrections are separately reviewed.
- [ ] Complete inventory, explicit ownership, preservation and semantic preflight work with no target mutations.
- [ ] The router's offline/read-only commands work without unrelated application initialization.
- [ ] A local renderer/planner PR handoff is ready for manual review and commit/push.

### Implementation ready for adoption review

- [ ] Exact supported server/client operations, bounds and failure behavior are certified on disposable builds.
- [ ] Durable snapshots/request intents precede every mutation, including prerequisites and restoration.
- [ ] Authority, lease, unresolved-run protection and enforced external-writer handling cover all mutating entry points.
- [ ] One-bulk context boundaries and every documented outcome/exit code are tested; no skipped-success or blind retry path exists.
- [ ] V1–V4 and the actual pinned SDK verifier work; missing evidence cannot produce a verified result.
- [ ] Observation-first resume, three-way restore and pre-feature rollback are demonstrated using an independent recovery executor.
- [ ] CLI/build/seed/CI checks and preserved ordinary v1/v2 behavior have evidence.
- [ ] Disabled deployment definitions use the actual chart and image/settings identities; monitoring survives Job termination.
- [ ] Mixed-version, cache/fallback, unknown-commit and external-writer drills have recorded results and limitations.
- [ ] Sanitized runbook, changed-file review and manual commit handoffs are complete.

### Each environment before/after activation

- [ ] Real target identity, sharing/readers, one authority and exact current release/configuration are confirmed.
- [ ] Permissions, protected transport, full discovery, writer barrier and all-traffic rollout checks are demonstrated.
- [ ] Live metadata/default/dimension prerequisites, ownership grants and exact semantic changes are reviewed.
- [ ] Recovery artifacts/retention/identity, first baseline, backup and retained recovery image are ready before first write.
- [ ] An authorized operator reviews the concrete adoption packet before performing the selected environment action.
- [ ] First application has separate app/sync/V1–V4 results; the second identical run makes zero Superposition mutations.
- [ ] Recovery drills pass and no unresolved unknown/partial/unverified run remains.
- [ ] Sandbox and production have their own target evidence; shared targets have one authority covering all regions/readers.
- [ ] Apply/additive automation is enabled only through an explicit policy/release decision; new-field metadata remains an onboarding prerequisite.

Completion means the applicable gates have evidence. Writing the code, generating a matching seed, reaching one healthy pod or passing 14 evaluator probes alone does not meet the adoption or production gates.
