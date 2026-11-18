# Step 9 — target authority and execution coordination

18 November 2026. **Local implementation; real control certification pending.** No production
target, authority, Kubernetes connection or writer barrier is registered. Their constructors
refuse. No apply command, Superposition mutation, deployment or hosted inspection was added.
The [checks](checks.md) and [machine record](evidence.json) separate executed local evidence
from unexecuted platform gates. The original plan and roadmap remain unchanged.

## Source evidence and existing work

Read playbook Step 9 and plan §7.1–7.2, §8.2–8.3, P4, T20/T21/T25/T26/T34 before editing.
H started on `feat/superposition-required-fields-foundations` at
`480e15c04a1524b7c297d1a8df761fcef55be4fa`, with Step 8 journal work already uncommitted.
The existing renderer, inventory, planner, CLI and journal were reused. Only the journal's
in-memory preparation identity and a sealed checkpoint bridge were added for coordination;
its persisted format was not changed. Two fixture helpers now have crate visibility for the
actual planner/journal/coordination integration test.

W remains at `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39` (main, earlier untracked SDK cases);
U remains detached at pinned `1af3e5dabf9d5a17dd864ac7312151ebac4367c6`;
I remains at `92041e63f025dc838388a489a5511ba74f387590` (main).
Applicable instructions, revisions and working trees were inspected first. No applicable H/W/U
AGENTS file was found; I's chart workflow was inspected, but no chart was created or edited.

Refreshed only relevant source references:

- H `crates/scheduler/src/db/queue.rs::acquire_pt_lock` does SETNX followed by a separate
  expiry; failure cleanup and `release_pt_lock` delete without owner comparison. This is
  **not** a suitable renewable target coordinator. Existing scheduler behavior is unchanged.
- H `crates/external_services/src/superposition.rs::set_config_value` still calls
  `create_context`. A sync lease does not exclude this ordinary writer.
- I `argo-production/apps/prod/hyperswitch-app.yaml` selects external
  `juspay/hyperswitch-helm`, `charts/hyperswitch-app`, and references `rollouts-controller`
  management of traffic resources. Selected chart templates were not present in the inspected
  local source. A Deployment-only implementation would be an unsupported assumption.
- Compiled feature keys are `REQUIRED_FIELDS_SYNC` and `V1`, not lower-case Cargo feature
  names. Intent validation and journal binding use the actual keys.
- Kubernetes uses `resourceVersion` for conditional updates; stale versions conflict.
  Lease fields describe holder, renewal and duration, rather than enforcing an external
  Superposition transaction fence. These are public API references, **not cluster evidence**:
  [API concepts](https://kubernetes.io/docs/reference/using-api/api-concepts/),
  [Lease v1](https://kubernetes.io/docs/reference/kubernetes-api/coordination/lease-v1/),
  [leases](https://kubernetes.io/docs/concepts/architecture/leases/).

## Implemented contracts

`authority.rs` binds a physical backing instance/org/workspace to one coordinator cluster,
namespace, object and UID. The lock scope excludes regional aliases, release names and
instance generation; changing those cannot create another lock. State still checks the exact
registered generation and refuses an unimplemented migration. Registry validation rejects
duplicate scopes/coordinators and ambiguous reader registrations.

Release intents bind issuer, authority, application UID/revision, increasing generation,
immutable executor image, build/features, effective inputs, bundle, policy, catalog, settings
and the complete reader roster. Republishing identical content is allowed; changing content
at the same generation or moving backward is refused. Rolling back to an older image requires
a newer intent. No real authenticated intent publisher is implemented or implied by these
pure validation rules.

The shared `ExecutionSession` interface accepts Apply, Resume and Restore. It checks the
current intent, atomically claims lease/active record/high-water stamp, obtains positive
control evidence, and rechecks all gates during renewal and before future dispatch. A
separately polled heartbeat renews every 10 seconds; lease duration is 60 seconds, control
calls are bounded to 5 seconds and dispatch requires at least 15 seconds remaining. Backward
clock steps, expiry, owner changes, failed/unknown CAS, cancellation during a write, stale
intent, changed population or lost enforcement stop dispatch. Fresh control evidence after
a suspension cannot resurrect an expired lease. No lease duration is treated as a server
request-drain timeout.

The bounded rollout contract covers all registered regions and readers, exact Deployment or
Rollout UID, controller/traffic resource versions, promotion/readiness, no old replicas or
connections, exact pod image/build/inputs, all traffic endpoints and explicit compatibility
with both before and after semantics. Scaling or any changed membership invalidates the
held view. A real adapter must enforce the deployment/intent gate while writes are possible;
periodic healthy-pod sampling cannot supply this guarantee.

Positive barrier evidence must cover UI, API, ordinary router, seeder, automation and
administrative paths for the physical target. A privileged edit cancels the run. Neither a
lease nor a last-moment re-read substitutes for exclusion. `UnavailableSafetyControls`
refuses; JSON and test fixtures cannot register a certified production target.

The sealed journal checkpoint binds actual prepared-plan/input/image identities and the
durable exact request intent. A CAS reserves its request hash in the active record before
future dispatch. Release clears only lease ownership and retains the unresolved record,
request, first-adoption reference and external barrier. A successor needs independent journal
recovery plus positive old-executor revocation/termination, accepted-request drain/cannot-commit
proof and direct-state observation. The exclusion receipt must bind the recovered journal
head **after** the provider has frozen the old executor and drained its accepted requests;
if it differs from the reader's head, acquisition refuses and recovery must be repeated.
Lease expiry or a vanished Job is insufficient. Predecessors are retained, with bounded
capacity that refuses overflow rather than deleting history.

## Proposed choices requiring review

**Co-location:** lease fields and the non-expiring active-run annotation share one preprovisioned
Kubernetes Lease object. One resourceVersion CAS claims both, avoiding a two-object partial
claim. This implements the plan's two logical records in one physical object. Release never
deletes it, TTL is not used for the active record, ownerReferences/deletion and coordinated
leader-election fields are refused. Production use requires approved object retention,
no-delete RBAC/admission, independent recovery access and verified persistence. This is a
proposal for Q3/Q4/Q6, not approval of a deployment or a change to the roadmap.

**Bounds:** at most 64 registered targets, 32 reader groups, 16 traffic resources per group,
256 pods per workload, eight predecessor records, 64 KiB contract/state and 1 MiB wire response.
These are conservative refusal limits; larger topology needs review, not silent truncation.
Clock skew bounds and operational barrier duration also require platform approval.

**Narrow adapter:** `kubernetes_coordination.rs` implements GET and conditional PUT of exactly
one preprovisioned UID, preserving unowned raw metadata and ordered values. No list, create,
delete, kubeconfig, ambient credentials, retry or redirect is used. Only the test-only
constructor is usable today: literal loopback address, explicit port, no proxy, owned fixture
header and successful identity-checked GET before any PUT. The real factory refuses until
endpoint/CA/principal, exact server build and Q3/Q4/Q6 controls are certified.

The future Step 10 caller must poll the heartbeat and call the fresh dispatch check immediately
before one bounded send, including after a pause. No reusable dispatch permit or network send
exists in Step 9. Resolution of requests, terminal V1–V4 completion and barrier release belong
to later steps; no automatic clear or successful completion shortcut was added here.

## Acceptance and gaps

| Gate | Actual evidence | Remaining dependency |
|---|---|---|
| T20 coordination faults | Isolated owner/CAS/expiry/suspension/cancellation/renewal and recovery tests; owned loopback lost-ack and restart | Exact Kubernetes version, RBAC/admission, clock and persistence failure drills |
| T21 external writer | Missing/incomplete barrier and privileged-edit observations refuse | Actual competing UI/router/seeder/API/admin writes prevented by certified enforcement; not executed |
| T25 command/security subset | Closed factories, no ambient discovery, scope/redaction and no force field; existing CLI tests still pass | Actual principal/transport enforcement; no new CLI wiring in Step 9 |
| T26 rollout/authority | Independent synthetic mixed-version, stale generation, rollback, region/co-reader and scaling cases | Real selected chart/controllers/routes, immutable per-pod attestations and authenticated issuer |
| T34 platform integration | Contract only; no platform execution | Actual Argo/InfraSwitch, Job lifecycle, verifier credentials and deployment gates |
| Journal/restore/V1–V4 | Actual in-process planner → journal receipt → coordinator binding | Q6 real storage; later request resolution, restoration and all target verification stages |

An explicit local OrbStack socket was inspected read-only for cached kube-apiserver, etcd,
kind-node and minikube images; those filters returned none. No cluster was started or selected,
no kubeconfig read, no image pulled and no hosted target contacted. This limited inspection
does not assert that no Kubernetes installation exists. The local Python server is a wire
fault fixture, **not Kubernetes**, and cannot certify server persistence, fencing or access.

Q2/Q3/Q4/Q5/Q6/Q9 and reader/business approvals remain open. Hosted apply/restore stays disabled.
After review, Step 10 **local disabled state-machine work** is next only on a separate request;
dependent real-control gates do not pass by implementing that work.

## Review groups

Step 9 files: `authority.rs`, `coordination.rs`, `kubernetes_coordination.rs`,
`journal_coordination.rs`, their three new test files, actual-source module imports,
companion fixture/dependency edges, and this evidence/progress update. See `evidence.json`
for the exact file list. Root Cargo manifest/lock, serving code, original plan/roadmap,
business inputs, seed and W/U/I were preserved.

Step 8 remains uncommitted. Keep its journal/snapshot/plan group separate from this coordination
group; shared module/journal/test/progress hunks need manual selection. Suggested H commits:

- Step 8: `feat: journal required-fields plans and request intentions`
- Step 9: `feat: coordinate required-fields execution by target and release`

No staging, commit, push, PR update or deployment was performed. Stop for review.
