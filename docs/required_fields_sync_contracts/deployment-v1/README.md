# Step 15 — disabled deployment integration

Local definitions and render checks are implemented in I and the actual selected chart
repository. **No target is registered or enabled; platform acceptance remains pending.**
See [checks](checks.md) and [machine evidence](evidence.json). No Step 16 work occurred.

## Source evidence

| Repository | Starting revision / branch | Starting tree |
|---|---|---|
| H | `68c938724127fe507cdb5b2c365c6f15af1aec1d`, `feat/superposition-required-fields-foundations` | 59 pre-existing changed/untracked files, including Steps 13/14 |
| I | `92041e63f025dc838388a489a5511ba74f387590`, `main` | Clean |
| Helm | `d20ad9bec02603a1fd8d145d45bf74ae6be7a243`, `main` | Clean |
| W | `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39`, `main` | 30 pre-existing untracked verifier/SDK files |
| U | `1af3e5dabf9d5a17dd864ac7312151ebac4367c6`, detached | Clean |

Read Step 15, plan §8/P8 and T20/T21/T23/T26–T30/T34, H contributing guidance, I AGENTS
and its local Helm skill before editing. No applicable AGENTS was found in H/chart or
their inspected ancestors. I's `hyperswitch-ai-brain` example chart is absent; the real
selected source was used. Initial public-source access failed; the user subsequently
provided `/Users/deepanshu.bansal/hyperswitch-helm`. No restriction was bypassed or remote
source fetched. Helm and exact dependency archives were then available locally.

I selects 16 main/v2/custom/playground ApplicationSets with 36 active release-region
elements. Main v1 tags are `app-2026.10.12.0` (integ), `app-2026.10.07.0` (sandbox),
and `app-2026.09.30.0-hotfix2` (production). Sandbox and production are manual Argo syncs;
integ is automatic. Separate region elements do not prove separate databases. Current
scheduler toggles do not establish a passive region. Actual physical targets/readers remain
unknown; no endpoint, organization/workspace or credential value was promoted to a grant.

The actual chart uses `templates/router/_router-shared.tpl` for Deployment/Rollout and
`hyperswitch-router.image` for its main image. Both selected tags and HEAD were inspected;
HEAD's dependency history differs from the tags. Each isolated tag retained its own
dependency archives and Chart metadata. No app/server/dependency/chart version was changed.
The custom integ-v2 tag `cus-fe774b0b64-v2` is unavailable locally; its writer remains off.
Templated CUG `chartPath` was resolved from its element rather than reported as a literal path.

H already has the Step 8–13 journal, authority, recovery, gated CLI and release observations.
Its real target/storage/image/control/collector provisioners still refuse. Step 14 already
pins W/U, verifier source archive and case pack. Step 15 changes no Rust/runtime/SDK/business
source. I's Jenkins currently reports image tags and updates/dispatches chart selections;
no durable monotonic intent publisher or complete reader gate was found. InfraSwitch's
changed-version-header check does not satisfy that gate. Existing pipelines remain unchanged.

## Implemented choices

I adds an empty registry with explicitly pending candidate identities, no regional lock
scope, disabled nonwriter classes, a reviewed deployment contract, sanitized source/SDK pins
and read-only drift/refusal checks. The three main v1 ApplicationSets reference an explicitly
disabled overlay. Their selected tags predate these templates: values alone deploy no Job.

The actual chart adds a suspended PostSync Job/companion definition, immutable content-named
input ConfigMap, separate ServiceAccount, narrow named-Lease/read-discovery RBAC and deny-all
NetworkPolicy. The main router and executor share the exact OCI digest helper and immutable
profile mount. Generation, target/registration/reader membership, image/build/settings/policy/
catalog/input/bundle/plan and verifier/recovery artifacts are bound. Execution enablement or
non-disabled mode is an unconditional render error; there is no certification/force switch.

The Job has no blind retries, no cleanup hook/TTL, no created Lease/journal/PVC and no shared
writer credentials. The companion has no token/Secret/report/journal mount. Actual workload
identity injection, effective filesystem/RBAC/network access and admission remain untested.
Pod NetworkPolicy cannot prove per-container isolation. Scratch storage is not a journal.

The existing W runner is named in the companion definition, but its OCI packaging, provisioned
fixture/response/case exchange with H and report collection remain unavailable. Tests transport
synthetic JSON bytes; they do not claim H has accepted a semantic input profile, an image
attestation or a real reader compatibility proof. Every deployment certification stays pending.

## Dependent gates and assumptions

No live topology, authority winner, retention duration, credential privilege, registered server
contract, pod UID, writer barrier or SDK compatibility is assumed. The proposed namespace RBAC
is a definition, not access proof. Cross-region readers need explicit independently verified
access and compatibility. Infrastructure requirements are in I's `deployment-contract.json`.

- **Q2/Q3/Q4:** certify exact servers, full physical target/reader inventory, one durable current
  intent authority, complete writer exclusion and old executor/request drain. Manual Argo,
  delayed jobs, scaling, mixed readers and traffic drain require actual platform exercises.
- **Q6/Q7:** provision immutable encrypted journal/receipts/unresolved protection, first-adoption
  baseline and independent recovery access/image retention. Keep these outside Job cleanup.
- **Q1/V1–V4:** certify per-pod route/cache inputs, actual browser image/artifacts/response-bound
  reports, old snapshots/SDKs, backup atomic publication/mount and primary-unavailable startup.
- **P8 reporting:** publish immutable build/provenance/authority-issued intent from Jenkins,
  implement H's real provisioners/companion exchange, ingest durable reports and prove alerts.
  App rollout, sync, verification and delivery are separate outcomes. No guessed metrics or
  ineffective alert resources are installed. Unknown commits cannot trigger blind app rollback.
- **Release integration:** publish/review a versioned chart and select it explicitly later.
  Existing tag pins are unchanged. No hosted deployment or effective permission test was run.

All available independent local work for this step is recorded; these gaps block operational
acceptance and all hosted mutations. Step 16 adoption is **not eligible** until its required
contracts, approvals and platform gates pass. Next eligible work is review and completion of
these specific deployment/contract gates in separately authorized disposable infrastructure.

Suggested separate manual commits: I `feat: configure disabled required-fields deployment
integration`; Helm `feat: add suspended required-fields job definitions`; H `docs: record
required-fields deployment evidence and gates`. Preserve earlier H/W review groups. No index,
branch, commit, push, PR, deployment or submodule change was made.
