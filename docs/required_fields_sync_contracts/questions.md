# Open questions and dependent gates

2026-11-18, updated through Step 11 local verification work. All items remain **open**. Component responsibilities below are **proposed from plan §13**, not assigned people or approvals. **Named accountable owner: unassigned for Q1–Q11.** No hosted target is authorized. Only owned filesystem builds, Step 1/11 local browser origins, Step 11 provider files/read-only loopback targets and the Step 9 owned loopback wire fixture were registered; no Superposition/workspace target was started.

| ID | Current evidence / outstanding decision | Proposed responsible component and required evidence | Gate blocked / independent work available |
|---|---|---|---|
| Q1 | W/U source/evaluator identity unchanged; 14 independent resolver cases execute against actual JS and Rust 0.119.0, with divergence asserted. 38 component and five full-checkout tests pass, including intercepted actual card/billing confirm. H provider filtering changed (D3). Step 4 feeds rendered synthetic bundles to the actual pinned JS reader; new full-browser/Rust-resolver execution is not claimed. See [renderer evidence](renderer-v2/README.md). | SDK + payment-method owners: exact deployed web/mobile artifacts and remaining method/enum/CIT coverage; approve aliases/catalog/AC–TA and conflict semantics. See [Step 1](sdk-v1/README.md). | Affected mapping acceptance and target V3/adoption; successful local browser execution does not close these gates. |
| Q2 | Client **0.119.0** is unchanged. Step 2's original 167-dependency lock match is historical; the expanded Step 11 companion also matches H, with exact counts in its evidence. Step 2 opt-in harness plus 18 Rust/10 Python isolated tests pass. The subsequent read-only OrbStack check established runtime availability and cached candidate image digests, but candidates remain uncertified; source/auth/fault/limit/exclusion evidence is absent. SDK findings are in [Step 2](server-v1/README.md). | Superposition owner: exact source/image provenance, registered local fixtures and real auth/fault/limit/exclusion facilities; certify T10/T14/T16–T19/T22. Mock SDK dispatch/unknown-outcome tests cannot close server gates. | Certified adapters and hosted apply remain blocked. Step 3 foundations supply no server certification. |
| Q3 | Matching checked-in endpoints are historical topology clues, not target identity/reader/region proof. No authority or monotonic intent issuer chosen. | Platform/release owners: backing instance/workspace/generation, every reader/region/release, rollout sources, exact inputs and one authority per target. | Target registration, real coordination/rollout and adoption. Pure authority/registry schemas can proceed at their steps. |
| Q4 | No effective identity/access, protected transport, complete-read scope, external-writer barrier or old-request exclusion demonstrated. | Platform/security + Superposition owners: prove complete read/write scopes and exclusion covering UI/router/seeder/API/admin paths, lease loss and old executor/request revocation. | **All hosted mutations including restoration.** Interfaces and local negative tests can proceed. |
| Q5 | External main/incubator chart references unchanged; selected templates not present in inspected I tree. Old I timing keys differ from H (D2). | Helm/platform owners: actual selected chart source, rendered settings, Job lifecycle/RBAC, Argo automatic/manual and all-traffic rollout evidence; separately review timing-key migration. | Deployment template/integration acceptance. Independent disabled registry/values design can proceed when requested. |
| Q6 | Storage delegates through `cloud_services` (D6). Step 8 adds immutable journal receipts and isolated recovery tests; no production store, protected retention or independent-role certification. | Platform/security owners: approved store/retention/encryption, immutable creation and read-back, least privilege, protected unresolved runs/baseline and independent recovery role. | First mutation using the store; real journal deployment certification. Local journal types/fault cases can proceed. |
| Q7 | Backup image/tag/schedule remain checked-in configuration only; implementation, publication, target identity and deployed startup fallback unverified. Step 11 actual local JSON/TOML startup and separate retained-memory checks pass. | Backup/platform owners: source/digest, exact content identity/format, atomic publication/mounts/permissions and primary-unavailable startup drill for every reader. | V4, `AppliedVerified`, recovery-ready adoption. Local V4 interfaces/failure cases can proceed. |
| Q8 | Presentation binding semantics known from pinned W/U; no live descriptor/default/schema/function inventory or promotion owner. | SDK/configuration owners: reviewed descriptor contract, aliases/render/options/assets, default/schema/function ownership and actual promotion evidence. | New-field activation/live preflight/adoption. Synthetic missing/incompatible-metadata refusals can proceed. |
| Q9 | MSRV/features/dependencies changed (D1/D6). Step 11 actual default-v1 opt-in router check/Clippy and standalone probes passed; broader workspace feature compilation and release/image/effective-config attestation did not run. I timing mismatch unresolved (D2). | Rust/build + platform owners: supported toolchains/feature closure, ordinary v1/v2 and sync-enabled matrix, exact image/input digests and immutable per-pod attestation. | Feature/build finalization and release-bound automatic apply. Local build-manifest design/checks can proceed at the requested step. |
| Q10 | InfraSwitch/Euler executor and all-traffic rollout not certified; historical health-header checks are insufficient. | InfraSwitch/deployment owners: same-image one-shot API, authority/credentials/settings, intended digest and complete rollout/recovery evidence. | Euler writers only; Argo/local work is independent. |
| Q11 | No ownership grants, approved dummy/retirement/exclusion policy or source-data corrections. New compiled Realex/Wero inputs (D4); malformed country/bank/path choices unresolved. Step 4 full profiles refuse 1,625 unbound sources and three malformed-option occurrences; the Givex synthetic/actual scope and ClassicReward wire/display discrepancies are explicit. | Payment-method/configuration/release owners: independent business cases, accepted aliases/catalog/source corrections, exact attribute grants and explicit exclusion/retirement handoffs. | Adoption, affected mappings and destructive/ambiguous semantic changes. Candidate reporting and focused proposals can proceed without implicit ownership. |

No open question prevented completion of documentation-only Step 0. Step 1's local tests do not close target-specific gates without their specified evidence. Q8/Q11 mapping and business choices are detailed in [Step 1 decisions](sdk-v1/decisions.md); all remain proposals. Business/source or SDK behavior corrections require separate review groups; none was made here.

## Step 5 evidence refresh — 18 November 2026

- **Q2/Q4 source and executed evidence:** locked SDK 0.119.0 seven-operation read-wire comparison and 47 isolated tests pass. Full permission scope/all experiment history, stable counted ordering/latest versions and exact-server compatibility remain uncertified. An empty sealed target registry refuses network reads; synthetic test certificates cannot dispatch HTTP. [Inventory evidence](inventory-v1/README.md) records experiment-group discovery and the object-form dimension wire union. No deployed scope is inferred from totals or checked-in URLs.
- **Q9 executed evidence:** actual inventory modules compile/test and pass Clippy in the isolated companion; all 162 dependency identities/checksums match H. Current offline core metadata passes, refreshing the old cache blocker. The cbindgen relative-output side effect is isolated in an owned cache copy for the router check; the actual default-v1 sync-enabled router check passes with pre-existing warnings. Remaining matrix gates are in [checks/results](inventory-v1/checks.md). This is not release/MSRV attestation.
- **Unchanged blockers:** Q1/Q8/Q11 business/profile approvals, Q3/Q6/Q7 target/journal/fallback evidence and all apply/restore/V1–V4 gates remain open. Step 5 neither invokes functions nor changes source data, SDK/serving behavior, seed or prior certification records. Step 6 local work is next only after review and an explicit request.

## Step 6 evidence refresh — 18 November 2026

- **Q1/Q8/Q11 source and executed evidence:** pure grant/preservation/preflight/planning checks and 396 actual JS plus 396 actual Rust resolver observations pass. Catalog/seed data cannot replace missing live metadata. Eight before/after SDK stage receipts and exact semantic approvals are required; the new full-browser matrix executor and authenticated receipt adapter remain unregistered. Historical Step 1 browser results are not candidate receipts. Business/catalog/source corrections and full-profile approvals remain open.
- **Q2/Q4:** actual core 0.119.0 reproduces ordered-array/separator identity collisions, and the planner refuses conflicting observed/projected identities. This is not exact-server certification. Permission, pagination, ordering, limits and experiments still require the Step 2 disposable server contract. Both production registries remain empty; there is no hosted authorization.
- **Q9:** the existing actual-source companion retains all 162 H-locked registry identities. Current checks, full router build results and preserved limitations are in [Step 6 checks/evidence](planner-v1/checks.md). No MSRV/release/other-feature or deployment attestation is inferred.
- **Q3/Q6/Q7 and all mutation gates:** still open. No authority service, lease, journal write, backup change, mutation adapter, serving read, restoration or V1–V4 check was implemented/executed in Step 6. Pending-create digests are pure intended identities, not durable receipts.

At the Step 6 checkpoint, Step 7 was next and had not started. The following refresh records only its newly executed evidence.


## Step 7 evidence refresh — 18 November 2026

- **Q9 source and executed evidence:** opt-in router render/plan dispatch now precedes unrelated initialization. A shared Settings projection captures only relevant inputs; named seeds ignore ambient bank overrides, effective profiles require a matching settings hash. Parser, synthetic orchestration, actual binary and rejected-build checks are distinguished in [Step 7 checks/evidence](cli-v1/checks.md). Exact H-locked Clap was added only to the isolated companion. Release/image/MSRV and the remaining feature matrix are not inferred.
- **Q2/Q4:** production read/projection registries are still empty. Target/config validation precedes credential reads and transport; no endpoint/header/token overrides or write/lease methods exist. Successful synthetic planning cannot certify server contracts or a real identity's permission scope.
- **Q1/Q8/Q11:** explicitly all-excluded compiled profiles test command plumbing only. Full business profiles remain refused; existing source/SDK corrections and ownership/catalog approvals stay separate. No new candidate browser or resolver certification is claimed.
- **Q3/Q6/Q7 and mutation gates:** no journal, authority, coordination, mutating command, restoration or V1–V4 enablement was added. Local draft PR and synthetic walkthrough are available for the human checkpoint. No hosted target was contacted.

After review, Step 8's local journal work is next on an explicit request, with Q6 storage certification still open. No later step has begun.

## Step 8 evidence refresh — 18 November 2026

- **Q6 local implementation/executed checks:** snapshot/hash binding, conditional-create/read-back protocol, append-only events, exact frozen requests, held first-adoption baseline, pending ownership and unresolved-run schema now have isolated fault and independent-process filesystem tests. See [journal-v1](journal-v1/README.md) and its [checks](journal-v1/checks.md).
- **Q6 source finding/decision:** current `FileStorageInterface` upload/delete/read and delegated `S3Storage` do not expose immutable creation/version/retention guarantees. They were inspected, not instantiated or repurposed. New production journal registry remains empty. Local `create_new`/fsync is not provider/WORM/encryption/access certification; the fixture owner can bypass it, which the tests explicitly demonstrate.
- **Q6 proposed, not approved:** retention at least max(90 days, rollback window), indefinite holds for baseline/active/unresolved artifacts, restricted encryption/decryption and access logs, separate recovery role and retained recovery image. Missing real-store authorization/contract blocks journal deployment and first mutation, not completed local tests. Input credential classification and exact adapter/provider fault/access/lifecycle evidence remain required.
- **Q2/Q3/Q4/Q7 and mutation gates:** exact request bytes are retained, but action-to-wire serialization, server outcomes, authority, active-record coordination, writer exclusion, V1–V4 and three-way restoration remain pending. No HTTP dispatch, lease or mutating command was implemented. Synthetic HTTP statuses are not actual post-commit server tests.

After review, Step 9 local coordination work is next on a separate request. No Step 9 work has begun. Hosted targets remain unauthorized; no stage/commit/push was performed for Step 8.


## Step 9 evidence refresh — 18 November 2026

- **Q3 source/implementation:** physical-target registry and monotonic release-intent contracts now bind complete reader/region and immutable image/input identities. Synthetic Deployment/Rollout, old-traffic, scaling, stale-job and rollback cases execute. No authenticated issuer, selected chart adapter or actual reader-discovery/gate certification exists.
- **Q4 executed/local versus pending:** owner-checked CAS/renew/release, suspension/lease-loss/unknown-outcome cases and an owned loopback request/restart fixture pass. Missing external enforcement always refuses. No real UI/API/router/seeder/admin exclusion or old-server-request drain was exercised; a lease is not a transaction fence.
- **Q3/Q4/Q6 proposal:** lease plus durable active-run annotation are co-located in one preprovisioned Kubernetes Lease for an atomic claim. No ownerReferences, TTL clear or delete method exists. Actual no-delete RBAC/admission, durable retention, independent recovery principal, clock skew and complete writer enforcement must be approved/certified before registration. Bounded predecessor retention refuses overflow. See [Step 9 choices and gaps](coordination-v1/README.md).
- **Q5/Q10 source:** I still selects an external Helm chart and references rollouts-controller traffic management. Selected templates were not present locally. Neither Deployment-only assumptions nor healthy-pod sampling were used as successful platform evidence; actual Argo/InfraSwitch adapters remain unavailable.
- **Q6/Q9 integration:** sealed checkpoints bind the actual accepted plan, retained journal and exact durable intent before request reservation. The isolated companion adds only existing H-locked time feature edges; current focused checks and final-source build evidence are in [Step 9 checks](coordination-v1/checks.md). Step 8 historical evidence is preserved.
- **Unchanged gates:** Q1/Q2/Q7/Q8/Q11, real storage certification, request resolution, restoration and all target V1–V4 remain open. No apply/resume/restore network dispatch, deployment, hosted credential lookup or live inspection occurred. Step 10 has not begun.


## Step 10 evidence refresh — 18 November 2026

- **Q2 source/executed:** actual locked SDK 0.119.0 confirms PATCH dimension, object-form bulk and server ID union; in-memory retry tests dispatch each operation once on errors/timeouts. Raw byte-preserving reqwest dispatch is a documented proposal, not server certification. Cached local 0.102.0-tagged image digest is available, but source/revision provenance and authenticated disposable fault/limit/exclusion controls are absent. Historical no-socket blockers are stale for this inspection. No image was started or server test certified.
- **Q3/Q4 implementation:** the disabled engine uses the existing session/heartbeat/authority/barrier interface before every single send. Direct reads never substitute for external-writer exclusion. Expired approvals, missing controls, stale authority, cancellation and unresolved old requests refuse progression; no real control adapter was enabled.
- **Q6 implementation/compatibility:** exact intents, transport observations, direct-state resolutions and reports are persisted before state transitions. Event version 2 adds resolution/report records; the current reader accepts legacy v1 events and rejects new kinds mislabeled v1. Original baseline and pending ownership are retained. Real provider retention/access and a retained recovery image that understands v2 still require certification. Active records/holds stay in place until later terminal verification/recovery work.
- **Q9 executed versus pending:** actual-source synthetic faults, locked-SDK serialization/retries, independent filesystem reader, focused builds and their exact results are in [Step 10 checks](apply-v1/checks.md). No release/MSRV/other-feature attestation is inferred. No dependency version or manifest/lock changed this step.
- **Q1/Q7/Q8/Q11 and verification:** a sealed verifier receipt is required; missing/wrong/timed-out verification produces exit 6 after confirmed writes, while any unresolved dispatch takes precedence as exit 5. Synthetic positive receipts are explicitly test-only. Target V1–V4, restoration, business/profile approvals and source/SDK corrections remain separate open gates.

Step 10 local work stops for review. Step 11 is next on a separate request; no hosted apply,
credentials, deployment, staging or commit was performed. See [choices/caps/outcomes](apply-v1/README.md).


## Step 11 evidence refresh — 18 November 2026

- **Q1/Q8 source and execution:** unchanged pinned W/U now run as the bounded local companion, with the Step 1 corpus plus exact-response preflight/post-apply-phase cases and independent H receipt rejection tests. The new full-checkout driver is explicitly Card/Debit/US/normal billing line1 only; unsupported Step 6 matrices and old/mobile/deployed artifacts remain pending. Post-apply phase used synthetic provided responses, not an actual server mutation.
- **Q2/Q4:** read-only raw read transport, locked-client export serialization and owned loopback malformed-response checks add local evidence. They do not certify a server, permissions, per-pod addressing, complete traffic roster or exclusion. Both SDK routes are required for every registered pod; header/load-balanced samples are insufficient.
- **Q7 material format finding:** provider 0.119.0 FileDataSource reads JSON/TOML backup markup (`default-configs` values plus schemas), not raw API Config JSON. Actual new-provider startup and running last-good memory were tested separately; backup source/image, target/publication/mount proofs and old-reader canaries remain open.
- **Q5/Q9 platform gaps:** local child environment excludes credential variables and confirm is intercepted. This does not prove filesystem/process/mount isolation in a deployment. No companion image digest or per-container controls are certified; proposed 120-second cache / 40-minute backup deadlines remain unmeasured for real targets. No business-source/SDK behavior correction or broader feature/MSRV attestation was made.

[Step 11 checks and exact remaining gates](verification-v1/checks.md) are the current record. No open question is closed by a synthetic success.


## Step 12 evidence refresh — 18 November 2026

- **Q2/Q4 implementation, not certification:** observation-first recovery distinguishes before/after/divergence from durable requests and direct complete rows. Before-state cannot authorize a retry; exact old-executor revocation, accepted-request drain, journal head and current inventory must be bound by the same certified controls. Exclusion is rechecked throughout the session. Real server termination and external-writer drills remain open.
- **Q3/Q8 material authority finding:** serving-reader equality was too narrow for a retained recovery executor. Explicit recovery intent now binds source run/prepared artifact/baseline and every current reader build; all current rollout/compatibility gates remain. No pre-feature binary is assumed to expose a command. Actual supported old-reader artifacts and full candidate browser coverage remain open.
- **Q6 implementation/format:** successor recovery decisions are immutable and read-back checked, original requests and baseline remain untouched, and recovery snapshots use schema 2 with a mandatory referenced decision. Legacy schema 1 remains readable. Missing reference/storage/access/retention evidence refuses; actual storage/image retention, decryption and recovery access remain uncertified. First-adoption undo needs the adoption run's confirmed after-state; a later baseline reference alone is not a multi-run ownership/history proof.
- **Q9 checks:** production modules pass isolated fault/recovery tests; an explicitly owned separate read-only process reconstructs lost-create evidence after the original fixture executor disappears. Actual router/format/lint results are in [Step 12 checks](recovery-v1/checks.md). No SDK/business-source/seed change or hosted restore ran.

No open certification question is closed by a synthetic success. Step 13 is next only after review and a separate request.


## Step 13 evidence refresh — 18 November 2026

- **Q3/Q4/Q6:** one shared executor connects existing safety/journal/recovery gates. Production
  adapter provisioning still has no certified implementation; CLI requests cannot supply a
  capability, endpoint override or force flag. Synthetic fixtures do not close these questions.
- **Q9:** startup observes actual loaded inputs and compiled features; optional desired-state
  observations require an explicit matching effective profile. Executable hashes and immutable
  profile/image bindings are checked separately from user-provided release claims. Actual image
  provenance and the wider release/MSRV matrix remain pending.
- **Q5/Q7/Q10:** final reports contain bounded metric fields and separate rollout/sync/verification
  status. Real one-shot collector/alert delivery, pod identity attachment and stale-backup alerts
  remain platform dependencies; stdout/private file output is not a delivery certificate.

[Step 13 checks](operations-v1/checks.md) record executed local evidence. Original source/SDK
business decisions and every target V1–V4 gate remain open. No hosted target was enabled.


## Step 14 evidence refresh — 18 November 2026

- **Q1/Q8/Q11:** the seed editor preserves ordered values and requires exact local grants. The
  explicit development bank profile has no automatic exclusions; full mapping/field catalog,
  unrestricted country/AC–TA and initial migration approval remain pending. No business/SDK
  corrections are included.
- **Q2/Q9:** CI pins SDK 0.119.0 and separately checks the existing 0.102.0/0.113.0/0.115.5
  server candidates. Offline unit evidence is not server certification. Exact cached image/source
  fixtures and uploader/Deja API regression remain pending; no uploader ran.
- **Q1/Q5/Q9:** the exact W/U/verifier archive was exercised with isolated real browser tests,
  clean child inputs and intercepted confirm. That proves only its synthetic local scope. Linux
  workflow execution, deployed/old/mobile artifact coverage, release/MSRV/feature matrix and
  real target V1–V4 remain open.

[Step 14 checks](seed-v1/checks.md) distinguish implementation, independent semantic evidence and
remaining approval/platform gates. Nothing in this record enables hosted mutation.


## Step 15 evidence update

- **Source/access resolved locally:** the user supplied the selected `hyperswitch-helm` checkout.
  I's 16 ApplicationSet files / 36 release-region selections and actual Deployment/Rollout/image
  helpers are recorded in [deployment-v1](deployment-v1/README.md). Main selected v1 tags are
  present; one custom integ-v2 tag is missing and stays non-authoritative. No hosted access was
  authorized or used. Missing chart source is no longer the main v1 local render blocker.
- **Q3/Q4:** empty registry, exact declared digest/input/intent bindings, disabled values wiring,
  permanently suspended PostSync Job, separate companion mounts, named-Lease RBAC and static
  checks are implemented. They do not establish physical targets, complete readers/current
  intent, held rollout gates or external-writer exclusion. Manual/automatic Argo behavior,
  scaling and effective admission/workload-identity/credential isolation remain untested.
- **Q6/Q7:** journal, baseline/unresolved protection, independent recovery image and backup
  identity/startup obligations are defined; actual storage, retention/access and publisher
  contracts remain pending. Job scratch is not durable evidence.
- **Q1/V1–V4/P8:** exact W/U source/archive/case pins are checked; companion OCI packaging and
  the H response/case/report exchange, actual target browsers/per-pod caches, Jenkins immutable
  intent publisher, collector/alerts and fallback canaries remain missing. No successful stub,
  guessed alert metric or credential-bearing companion was substituted.
- **Q10:** inspected InfraSwitch dispatch/version-header logic does not satisfy equivalent
  execution/authority/rollout controls; it remains disabled. No pipeline or live state changed.
