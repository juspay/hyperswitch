# Required-fields sync evidence and progress

Updated **2026-11-18** through **playbook Step 12 local work**. [Recovery evidence](recovery-v1/README.md) covers observation-first resume, three-way restoration, retained baseline/image bindings and the existing apply gates. Synthetic recovery does not certify server termination, writer exclusion, retention, deployed readers or V1–V4. All production registries remain empty; hosted apply/resume/restore remains disabled. This sanitized register contains no hosted credentials, adoption grants or environment authorization.

Execution order: [implementation playbook](../superposition_required_fields_sync_implementation_playbook.md). Technical specification: [implementation plan](../superposition_required_fields_sync_implementation_plan.md). Read the selected step and its plan sections before changing code.

## Evidence records

- [Step 0 source baseline and discrepancies](baseline.md): exact revisions, existing implementation, current source references and proposed follow-up amendments.
- [Q1–Q11 register](questions.md): missing decisions/evidence, proposed responsible components and dependent gates.
- [Commands, side effects and checks](checks.md): executed checks separated from inspected future commands.
- [Step 1 independent SDK corpus](sdk-v1/README.md), [current browser execution](sdk-v1/browser-checks.md), [historical checks](sdk-v1/checks.md) and [decision/correction register](sdk-v1/decisions.md): JS/Rust, component and full-checkout results remain separate.
- [Step 2 API contract harness](server-v1/README.md), [checks](server-v1/checks.md) and [candidate capabilities](server-v1/capabilities.json): actual locked-client isolated evidence is separate from pending server certification.
- [Step 3 pure foundations](foundation-v1/README.md) and [checks](foundation-v1/checks.md): historical input/build evidence.
- [Step 4 pure renderer](renderer-v2/README.md), [checks](renderer-v2/checks.md) and [profile results](renderer-v2/results.json): deterministic contributions, bounded refusals, actual pinned JS bridge and explicit offline profiles. Includes the Step 3 managed-key spelling correction; no transport.

- [Step 5 direct inventory](inventory-v1/README.md), [checks](inventory-v1/checks.md) and [results](inventory-v1/evidence.json): raw row preservation, bounded two-pass proof, actual locked-client request comparison and pending real-server scope/pagination certification.

- [Step 6 ownership and planner](planner-v1/README.md), [checks](planner-v1/checks.md) and [evidence](planner-v1/evidence.json): pure complete-row planning, exact approvals, separate JS/Rust/browser evidence boundaries and pending certification.

- [Step 7 offline/read-only commands](cli-v1/README.md), [checks](cli-v1/checks.md), [evidence](cli-v1/evidence.json), [local draft PR](cli-v1/review-draft.md) and [walkthrough](cli-v1/walkthrough.md): explicit profiles, shared Settings projection, redacted provisional reports and no mutating command wiring.

- [Step 8 recovery journal](journal-v1/README.md), [checks](journal-v1/checks.md) and [evidence](journal-v1/evidence.json): immutable preparation/baseline, exact intents, observations, unresolved records and independent readers; production storage certification remains pending.

- [Step 9 authority/coordination](coordination-v1/README.md), [checks](coordination-v1/checks.md) and [evidence](coordination-v1/evidence.json): shared apply/resume/restore contracts, journal-bound active records and owner-checked renewal; real platform and external-writer adapters remain refusing.

- [Step 10 disabled apply](apply-v1/README.md), [checks](apply-v1/checks.md) and [evidence](apply-v1/evidence.json): one-attempt frozen mutations, durable observations/resolutions, all outcome/exit mappings and closed safety/verification capabilities.

- [Step 11 serving/fallback verification](verification-v1/README.md), [checks](verification-v1/checks.md) and [evidence](verification-v1/evidence.json): full-state/export checks, complete per-pod SDK-route coverage, response-bound actual browsers, JSON/TOML startup and independent receipt reading.

- [Step 12 resume/restoration](recovery-v1/README.md), [checks](recovery-v1/checks.md) and [evidence](recovery-v1/evidence.json): observation and fresh replanning, durable recovery decisions, three-way preservation, additive residuals and independently read recovery evidence.

**Evidence vocabulary:** *source* means inspected code/configuration; *executed* identifies a command and its result; *historical* means an earlier investigation's evidence, not a rerun; *proposed* is a choice still needing the relevant review; *open* means evidence is missing. Source inference is labeled separately. Neither source review nor Cargo metadata certifies a server, browser, build or deployment.

## Step checklist

- [x] **0 — Baseline and progress register:** local documentation complete and checked; awaiting user review. This does not complete plan milestone P0.
- [ ] **1 — Independent SDK/mapping cases:** local implementation/checks complete, awaiting review; 22 Node, 14 actual Rust resolver, 38 browser-component and five full-checkout tests pass, with focused builds/typecheck. Remaining coverage/owner decisions listed in sdk-v1. Full reader/mapping acceptance depends on Q1/Q8/Q11.
- [ ] **2 — Server contract harness:** local opt-in companion/guards/case requirements implemented; 18 Rust and 10 Python tests, focused build/Clippy/format checks pass. Subsequent read-only OrbStack inspection established runtime availability and cached candidate digests; exact source provenance and authenticated/fault/limit/exclusion certification remain pending. See server-v1 and Q2/Q4.
- [ ] **3 — Pure inputs and canonical artifacts:** locally implemented; 27 focused actual-package tests pass, including the 17 artifact tests also run in isolation. Isolated Clippy and focused build-script guards pass. Router settings/release/MSRV and other feature gates remain separately recorded in foundation-v1; pending checks and review prevent blanket acceptance. Unapproved mappings remain refused.
- [ ] **4 — Renderer:** locally implemented and tested; full compiled profiles remain blocked on 1,625 missing bindings and three malformed-option occurrences. Owner approvals, supported-profile acceptance and dependent reader/server gates remain open; see renderer-v2.
- [ ] **5 — Read-only inventory:** locally implemented; 47 isolated tests and focused build/Clippy pass. Full raw rows, two-pass consistency and sealed completeness proof; no certified production target. Step 2 read/permission contracts and review remain required; see inventory-v1.
- [ ] **6 — Ownership/planner/preflight:** locally implemented; 73 isolated tests plus the opt-in exporter, actual pinned JS/Rust comparisons and focused checks. Exact-server ordering/identity and authenticated full-browser candidate receipts remain pending; see planner-v1. Review/approvals remain required.
- [ ] **7 — Offline/read-only CLI and early review:** locally implemented; parser/orchestration/actual binary evidence and missing matrix gates are separated in cli-v1. Production targets remain unregistered; human review and dependent certification remain required.
- [ ] **8 — Durable journal:** locally implemented with bounded immutable artifacts, read-back receipts and isolated recovery/fault checks. Review, Q6 store/retention/access approval and real certification remain open; see journal-v1.
- [ ] **9 — Authority/coordination/writer exclusion:** locally implemented with isolated faults and an owned loopback transport/restart test. Real Q3/Q4 authority/rollout/exclusion and Q6 retention/access certification remain open; see coordination-v1.
- [ ] **10 — Apply state machine:** locally implemented with synthetic faults, actual locked-SDK wire/retry checks and independent journal recovery. Exact-server/raw-transport, real safety/storage and terminal verification/recovery gates remain open; see apply-v1.
- [ ] **11 — V1–V4 verification:** locally implemented with bounded read transport, actual pinned browser package, independent report reader and provider startup/last-good checks; target/server/platform/old-reader certification remains open; see verification-v1.
- [ ] **12 — Resume/restoration:** locally implemented and tested; original evidence/baseline retained, before-state retry requires drained/revoked old executor, inverse planning shares approvals and V1–V4. Real server/platform/retention/recovery-image gates remain open; see recovery-v1.
- [ ] **13 — Mutating CLI/reporting:** needs Steps 7–12.
- [ ] **14 — Seed/CI:** offline subset can follow Step 7; browser/full CLI gates need Steps 11/13.
- [ ] **15 — Disabled deployment integration:** needs Steps 8–13 and actual selected Helm/platform source.
- [ ] **16 — Integrated failure drills/handoff:** needs Steps 10–15 and verified disposable resources.
- [ ] **17 — Target adoption packet:** needs Step 16 and target-specific external gates; preparation does not authorize apply.

No step after 12 was started. T01–T38 were not executed in Step 0. Step 1 supplies actual JS/Rust and local browser evidence for applicable T05–T09/T29/T35–T38 portions. Step 2 supplies isolated client/guard evidence, not server T10/T14/T16–T19/T22 certification. Step 3 supplies foundation evidence; Step 4 supplies the recorded pure portions of T01–T09 and focused build evidence, not T10 server certification; Step 5 supplies isolated T13/T14 and actual-client read-wire evidence, not exact-server T16 certification; Step 6 supplies the pure ownership/preflight/approval and bounded resolver portions of T07–T15/T22/T32/T35–T37; exact-server and candidate-specific browser portions remain pending. Step 7 adds T25 command isolation/redaction and the recorded T33 build-boundary checks, not target certification. Step 8 adds the local journal portions of T18/T23/T24; server outcomes, real storage and actual restoration remain unexecuted. Step 9 adds local T20/T25/T26 protocol evidence; real T21 enforcement and T34 platform integration were not executed. Step 10 adds local T12/T16–T21/T23/T32 engine/wire/recovery portions; no real server, writer-exclusion or V1–V4 certificate is implied. Each step's matrix distinguishes executed coverage from remaining gates.

## Target and acceptance status

| Gate | Current status |
|---|---|
| Step 0 local source/command/document review | Complete; results in [checks](checks.md) |
| Step 1 local contract/harness | Implemented; JS/Rust/components/checkout checks pass; [results](sdk-v1/browser-checks.md); owner review pending |
| Step 2 local API harness | Implemented; isolated checks pass; [results](server-v1/checks.md); every server candidate remains uncertified |
| Step 8 local recovery journal | Implemented; isolated faults/independent filesystem reader, with actual build results in [journal-v1](journal-v1/checks.md); Q6 certification pending |
| Step 9 local authority/coordination | Implemented; local protocol and loopback transport evidence in [coordination-v1](coordination-v1/checks.md); actual enforcement/rollout/retention certification pending |
| Step 11 local verification | Implemented; [checks and remaining gates](verification-v1/checks.md); every target V1–V4 remains uncertified |
| Step 10 local disabled apply | Implemented; [checks and remaining gates](apply-v1/checks.md); no production registration or mutating command |
| Plan P0 contract certification and decisions | Open; accountable owners unassigned |
| Feature implementation/build/test acceptance | Pure renderer, inventory, planner and offline/read-only CLI locally implemented; focused checks in renderer-v2/inventory-v1/planner-v1/cli-v1. Full supported profiles, real-server and bound-browser certification and broader release/MSRV/feature matrix remain open |
| Target V1 persisted state / V2 router-cache / V3 actual SDK / V4 fallback | All uncertified; local SDK/browser cases are executed, but are not target V3 proof |
| Disposable fixtures | Step 1 source/build copies/static origins and Steps 2–11 owned filesystem builds, Step 8 owned journal fixtures and Step 9 coordination and Step 11 read-only loopback wire fixtures only; no Superposition/API server or hosted workspace started. Registrations and cleanup recorded per step. |
| Hosted read-only target authorization | **None supplied** |
| Hosted apply/restore/deployment | **Disabled / not authorized**; no enabling changes made |

Do not turn absent targets, contracts, coordination, journal, restoration or V1–V4 evidence into a successful stub or force flag. Missing external evidence blocks its dependent acceptance gate; independent local work may continue only when its step is requested.

Future records should bind case IDs, exact source/artifact revisions, commands and results, fixture ownership/cleanup, outstanding gates and a reviewed owner decision. Keep real target inventories and recovery artifacts in approved restricted storage, referenced here only by opaque identifiers.

## Review handoff

Step 0 changed only H documentation. Step 1 adds H synthetic JSON/schemas, a standalone Rust probe and evidence, plus W resolver/component/checkout harness files. U/I, source business data, seed, original roadmap, Git index/history and branches remain preserved. The continuation authorized locked public npm/browser dependency downloads in an owned copy; all four test layers now run. No uploader, hosted application request, payment submission or deployment occurred.

Review H Step 1's `sdk-v1/` and register/checkpoint updates separately from W's new `playwright-tests/required-fields-sync/` and `e2e/required-fields-sync.spec.ts`. The Step 0 register and original design documents were already untracked; their eventual inclusion is the user's staging decision. If source/SDK/timing-key corrections are approved later, keep them in separate review groups.

Suggested manual commits: H `test: define required-fields mapping expectations`; W `test: cover required-fields SDK contracts`. No staging or commit was performed.

Step 2 adds only H `scripts/required_fields_contracts/`, `server-v1/` evidence and progress/checkpoint updates. Suggested separate H commit: `test: add opt-in Superposition API contract harness`. Preserve the earlier H/W Step 1 review groups; no staging/commit occurred.

Step 3 adds H pure contracts/tests, settings/report boundaries, opt-in feature/build wiring, two existing lock edges, v2 recipe exclusions and foundation evidence. Suggested H commit: `feat: define required-fields sync inputs and artifacts`. Keep earlier H/W review groups separate; no staging or commit occurred.

Step 4 adds only H renderer/contracts/tests, an opt-in offline example, and renderer evidence/checkpoint updates. Suggested H commit: `feat: render required-fields Superposition contributions`. Business-source corrections and SDK/presentation changes remain separate unimplemented proposals. W/U, seed, original plan/roadmap, dependency lock and indexes remain preserved.

Step 5 adds only H direct read-only inventory/types/raw parsing/tests, an isolated actual-source check crate and inventory evidence/checkpoint updates. Suggested H commit: `feat: inventory required-fields configuration without loss`. Historical handoff requested separate Step 4/5 commits; those are now committed at 2785cd427 and 6a2e18696, respectively. No index/history/branch changes or hosted activity occurred.

Step 6 adds only H pure ownership/planner/preflight/types/tests, the existing isolated check-crate imports, offline resolver probes and progress evidence. W/U, seed, original plan/roadmap, dependency locks/manifests and all indexes remain preserved. Suggested H commit: `feat: plan required-fields changes with explicit ownership`. No Step 6 staging/commit occurred.

Step 7 adds only H CLI/runtime orchestration, the shared Settings projection, focused fixtures/tests and local review documents. The isolated companion adds exact H-locked Clap; the root manifest/lock, W/U, seed, original plan/roadmap and Git indexes remain preserved. Suggested manual H commit: `feat: expose required-fields render and planning commands`. Step 6's uncommitted planner group remains separate; shared module/test/progress hunks require manual review. No staging/commit/push or remote PR occurred.

Subsequent explicit authorization committed/pushed Steps 6/7 together as `480e15c04a1524b7c297d1a8df761fcef55be4fa` and opened a draft PR. The earlier handoff paragraphs record their historical uncommitted state; Step 8 starts after that authorized action and does not edit the PR.

Step 8 adds only H journal/snapshot types, accepted-evidence retention, fault/recovery tests, companion imports and scoped evidence/progress updates. Existing storage adapters remain unchanged because their upload interfaces do not establish immutability/retention. Suggested H commit: `feat: journal required-fields plans and request intentions`. No Step 8 staging, commit, push or hosted access occurred.

Step 9 adds only H authority/coordination contracts, the narrow conditional Kubernetes Lease adapter, actual journal checkpoint integration, tests and scoped evidence. Lease and active-run state share one preprovisioned object as a proposed atomicity choice; real no-delete/retention, rollout and writer enforcement are not certified. Step 8 remains uncommitted; shared journal/module/test/progress hunks need manual separation. Suggested H commit: `feat: coordinate required-fields execution by target and release`. No staging/commit/push or hosted access occurred.

Historical Step 9 checkpoint: after review, **Step 10 local disabled state-machine work** is next only on a separate request; Q2/Q3/Q4/Q6 and target V1–V4 remain independent gates. Step 2/platform certification and Step 1 owner/candidate-browser work may resume with verified disposable dependencies. No later step has begun. Stop for review.


Historical Step 10 checkpoint: Step 10 extends the existing Step 8/9 journal/coordinator and imports the new apply modules in
the same isolated companion. Exact changed files and preserved baseline hashes are recorded
in [apply-v1/evidence.json](apply-v1/evidence.json). No manifest/lock, source data, SDK, seed,
original plan/roadmap or W/U/I content changed in this step. Step 8 and Step 9 remain
uncommitted; keep their review groups distinct from Step 10 when selecting shared hunks.
Suggested Step 10 H commit: `feat: apply required-fields plans with recoverable outcomes`.
After review, Step 11 local verifier work is eligible only on a separate request. No later
step, staging, commit, push, PR update or deployment was performed.


## Historical Step 11 review checkpoint

Local implementation and isolated checks are recorded in [verification-v1](verification-v1/README.md). Target V1–V4 are **pending**, not completed: no hosted reads, mutations, real pod probing, backup publisher or platform credential inspection occurred. The companion is an executable local package, not a deployed image certificate.

Step 8–10 changes already present at the start remain uncommitted and preserved. Step 11 adds H verifier/provider/HTTP/report modules and tests, opt-in dependencies, and W `playwright-tests/required-fields-verifier/`; U/I, SDK behavior, source business data, seed and original plan/roadmap remain unchanged. Suggested separate commits: H `feat: verify required-fields serving and fallback state`; W `test: package pinned required-fields browser verification`.

After review, Step 12 local resume/restore work is eligible only on a separate request. Independently, Step 2/platform certification and missing reader/case coverage can resume with explicit disposable targets/artifacts. No staging, commit, push, PR update or deployment was performed.


## Step 12 review checkpoint

[Recovery implementation and evidence](recovery-v1/README.md) cover local observation-first resume,
three-way inverse planning, immutable successor decisions and the same apply/verification gates.
Current unrelated content and additive prerequisites survive; unresolved requests need certified
revocation/drain evidence before any successor mutation. The original baseline and source journals
remain intact. Exact first-adoption undo requires its confirmed after-state; a bare later-run baseline
reference does not establish an intervening ownership chain.

Only H changed in this step; W/U/I, dependency locks, the seed and original plan/roadmap are unchanged.
Existing uncommitted Steps 8–11 were preserved. Suggested H commit:
`feat: resume and restore required-fields synchronization`.

After review, Step 13 local command/report wiring is next only on a separate request. Real server,
writer exclusion, retention/access, recovery-image and V1–V4 certification remain required before
hosted mutation. No staging, commit, push, deployment or hosted restore occurred.
