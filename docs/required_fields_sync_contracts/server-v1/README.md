# Step 2 — API contracts and capability evidence

2026-11-18. **Opt-in local harness implemented; 18 isolated Rust tests and 10 Python safety tests pass. Exact-server certification is pending.** No server/API workspace was started, contacted or written. No mock outcome certifies a server. Hosted apply remains disabled.

Execution followed playbook Step 2 and plan §6.6/Q2/Q4, with T10/T14/T16–T19/T22 and §6.7's outcome/retry requirements read before editing. H remains `main` at `82fe5dfd3ac527fd4a1e03dc01fbb5be13e3198b`. Existing Step 0/1 files were inventoried and reused; no API contract harness existed in the searched sources. No applicable AGENTS.md was found. H's contributing/formatting guidance and feature separation were inspected. W/U working states and prior evidence were preserved; no revision change required repeating their browser checks.

## Implemented scope

- [Test companion](../../../scripts/required_fields_contracts/README.md): standalone Rust crate using actual generated SDK **0.119.0**, existing clap/error-stack conventions, and an offline lock subset matching every registry package in H. No production workspace/dependency/feature changes.
- [Case requirements](cases.json): bounded request/report/case contract and explicit T-test mapping. Independent synthetic assertions are in Rust tests and the real-server suite, not generated from observed responses or a seed.
- [Candidate capability matrix](capabilities.json): every candidate build/capability remains pending; image digests and actual fixture identities are null, not fabricated.
- [Checks](checks.md), [executed isolated report](reports/isolated.json) and [source identities](provenance.json): exact commands, results and artifacts. This is local evidence, not signed deployment attestation.

The real-server driver implements fresh workspace proof, prerequisite dimensions/defaults, complete two-pass pagination, prefix stripping, create-on-existing merge, full replacement/removal, delete-missing, one-bulk rollback, exported/versioned ordering, ordered-array collisions and active experiment references/overlap. It stops further writes after a failed phase. A collision is explicitly unsupported, never normalized away.

The launcher requires immutable, already cached demo/Redis images and OCI source/version provenance. It creates only a fresh nonce-owned internal network and containers, publishes only one literal loopback port, refuses host mounts/privilege/extra peers, and revalidates identities before every API request. The workspace name is unique and tied to the run; it must return 404 before creation, then match its synthetic admin/org/name and empty initial inventory. Cleanup removes only the owned containers/network, including a lost create response, and retains registration/report/cleanup evidence. Default Compose/dev workspaces and the seeder are never used.

## Evidence versus pending gates

| Gate | Executed local evidence | Remaining server/dependent evidence |
|---|---|---|
| T10 ordered-array identity | Actual SDK serialization retains order/duplicates; independent values differ | Exact server override IDs, exported collision behavior and context order; driver authored, no server execution |
| T14 complete inventory | Count/page/version/full-row/byte guards; duplicate/missing/partial/changing pages and same-count edits refuse | Real pagination/export consistency and independently authorized full-read scope; scoped reads with plausible totals cannot prove completeness |
| T16 API transport and preservation | Actual SDK method/path/org/workspace/body serialization; schema and unrelated nested values preserved; explicit registration/target refusal tests | Per-image bootstrap, authentication, API response compatibility, real merge/replacement/deletion/prerequisites/version behavior and limits |
| T17 transactions/prerequisites | Single-dispatch bulk; local action/body cap refuses before dispatch; outcome table distinguishes prior commits and proven rollback | Actual DB rollback/version boundary, prerequisite-only commits and exact server rejection limits; production planner/journal integration belongs to later steps |
| T18 post-commit/lost response | Real SDK processes scripted 512/500/drop; simulated commit remains unknown without observation; no automatic mutation replay | Real webhook/cache-publication failure injection, dropped committed response and persisted observation. Journal/crash durability is not implemented by this companion (Steps 8/10) |
| T19 retries/timeouts | Actual SDK bounded read retry and single write dispatch; delayed simulated server task commits after caller timeout | Actual socket/server attempt count, request lifetime, completion/termination proof and exclusion before any recovery retry; existing provider/default HTTPS transport not certified |
| T22 experiments | Actual create/ramp SDK request serialization | Real active experiment context protection, overlap policy and unrelated-row preservation; driver authored, unexecuted |
| Q4/CAS/fencing | Ownership/isolated-target guard unit tests | Dedicated authenticated scopes, external-writer exclusion, server/platform fencing and old-request revocation remain unproven; no support inferred |

Seven real capability profiles are explicitly pending: auth scope, configured server limits, post-commit webhook, cache publication, dropped response, server/socket timeout instrumentation and old-request exclusion. These require exact-build instrumentation/identities and are not implemented as green substitutes. The normal disposable suite can collect partial real evidence later, but its command returns nonzero while those gates remain unresolved.

## Refreshed source evidence and discrepancies

**Source:** the locked client package checksum and source identity are recorded in [provenance](provenance.json). The plan's 0.115.5 client assumptions are historical; the harness pins 0.119.0 and all 167 registry dependency identities/checksums to H. Published SDK VCS identity remains separate from server image/source identity.

**Executed/source:** the generated client sends **`x-workspace`** and `x-org-id`, matching H's current seeder convention. It serializes full replacement as PATCH `/context/overrides` and bulk as an `operations` envelope. `update_dimension` currently exposes optional `position`; the fixture intentionally does not send position on update. This refresh avoids silently copying older generated symbols.

**Executed:** the SDK can accept a config response lacking Last-Modified by defaulting metadata. The test companion separately requires actual version/Last-Modified evidence and forwards the server's headers unchanged. An initial assertion assuming SDK rejection failed; the corrected independent contract test asserts permissive SDK behavior plus harness refusal. No SDK behavior was changed.

**Source:** current SDK response/error types include workspace-lock information/conflicts. A read of the plan-linked [S115 service types](https://raw.githubusercontent.com/juspay/superposition/8b8f992127e883deba3048af03aa2389e423cc62/crates/service_utils/src/service/types.rs) also exposes `WorkspaceWritePermit` locking code. Historical line references/absence assumptions therefore cannot establish current fencing guarantees. No inspected generated mutation input supplies an expected version/fencing token. Lock scope, lifetime and coverage remain unexecuted Q2/Q4 contracts, not certified support. [Workspace wire types](https://raw.githubusercontent.com/juspay/superposition/8b8f992127e883deba3048af03aa2389e423cc62/crates/superposition_types/src/api/workspace.rs) were inspected as public source only; they do not establish any installed image.

**Proposed fixture choices:** conservative local caps (16 actions/1 MiB), two complete inventory passes, synthetic demo auth and strict OCI labels/internal-network ownership. These are harness boundaries, not approved production limits/permissions. Missing labels, image-specific startup requirements or wire incompatibility must be investigated; no tag substitution, compatibility header fallback or server upgrade occurs automatically.

## Handoff

Missing dependencies are explicit: a usable local container runtime; exact cached server/Redis digests and source/version provenance; authenticated disposable identities; actual fault/limit/exclusion facilities. No hosted credentials or target are requested. Candidate 0.102.0/0.113.0/0.115.5 tags are configuration leads only. See the [matrix](capabilities.json).

Step 2's local work is ready for review; its server acceptance gates remain pending. Next eligible implementation is **Step 3** after reviewing Step 1's input/label contract; Step 2 certification can resume when its exact disposable dependencies exist. Neither was begun. Suggested H commit: `test: add opt-in Superposition API contract harness`. No staging or commit performed.
