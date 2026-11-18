# Step 10 — disabled apply engine

18 November 2026. **Local implementation complete; real mutation certification pending.**
No usable production mutation target, journal, coordinator, external-writer control or
serving verifier is registered. Factories refuse; no apply CLI/HTTP route was added.
[Executed checks](checks.md) and the [machine record](evidence.json) distinguish local
fault tests from missing server and platform evidence. The original plan/roadmap are unchanged.

## Source evidence and preserved work

Read playbook Step 10 and plan §6.6–6.7, §6.9, P5 and T12/T16–T21/T23/T32 before editing.
H started on `feat/superposition-required-fields-foundations` at
`480e15c04a1524b7c297d1a8df761fcef55be4fa`, with Steps 8 and 9 already uncommitted.
The existing accepted planner, complete inventory, journal and execution session are reused.
No new Settings/AppState startup, serving reader or deployment mechanism was needed.
Applicable repository instructions, revisions and working trees were inspected first.

W remains `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39` (main);
U remains pinned/detached at `1af3e5dabf9d5a17dd864ac7312151ebac4367c6`;
I remains `92041e63f025dc838388a489a5511ba74f387590` (main).
All four indexes and captured unrelated file contents are compared to this step's starting
baseline, including W/U's pre-existing work. The initial hash inventory omitted nine Unicode
icon filenames in U; the final NUL-delimited audit verifies those against pinned HEAD, their
older mtimes and unchanged worktree status. Their absent starting hashes are not invented. This is not a claim that their working trees were clean.

Relevant current sources differ from older plan references:

- H locks `superposition_sdk` **0.119.0**, not the original 0.115.5 reference. Its actual
  `update_dimension` serializer uses **PATCH**; `bulk_operation` sends an **object with an
  `operations` array**. `ContextIdentifier::Id` serializes as `{"id":...}`. Dimension
  `position` is optional in this SDK; omitting it preserves the existing position. Tests
  invoke the actual locked builders and retry middleware through an in-memory connector.
- SDK document numbers reserialize `1.2300` as `1.23`. The direct adapter therefore sends
  already journaled raw bytes through the existing reqwest dependency. It is compared to
  actual SDK wire semantics but does not run a second serializer after journaling. This
  preserves unrelated decimal lexemes and ordered/duplicate array entries. Direct transport
  certification remains pending; SDK-only tests cannot certify that transport or the server.
- The explicit local OrbStack socket is available. Read-only image metadata shows cached
  `ghcr.io/juspay/superposition-demo:0.102.0`, digest
  `sha256:bd14b25e3fd4d85d977ee714afaaed991b8ec512bac3db1f38f7ac7d95297f8e`.
  OCI source/revision labels are absent. The selected `version` label says `25.10`; it is
  **not proof of the Superposition application version**. No image was pulled or started.
  Historical runtime-unavailable notes do not describe this current inspection. Exact build
  provenance, authenticated disposable registration and server fault controls are still absent.

## Implemented behavior

`PreparedApply` compiles and validates the entire accepted plan before dispatch. It requires
matching target/read/server evidence, complete two-pass discovery and a version, explicit
certified audit-field declarations, no unreadable attributes, matching journal/release/approval
bindings and bounded snapshots. Unsupported descriptions, paths, sizes, action ordering,
ambiguous row identity or malformed last actions refuse the whole preparation.

Dimension enum extensions and managed-default creation are separate requests. All context
PUT/REPLACE/DELETE operations form exactly one bounded bulk. Existing server-returned IDs,
complete overrides, descriptions, unrelated metadata and ordered values are retained. There
is no batching, reset, automatic replay, rollback, marker context or seed change.

The engine polls the existing renewable session throughout execution. Before each send it
checks authority/lease/barrier and approval expiry, obtains a fresh complete inventory,
compares it to the previous observed state, durably records exact intent, reserves that hash
in the active record and checks dispatch authority again. A sync lease or re-read does not
substitute for certified external-writer exclusion. Cancellation/drop stops local dispatch;
the durable pending intent survives. Approval and authority are rechecked after verification
and final report persistence as well.

The direct transport has no ambient credential discovery, proxy, redirect, decompression or
automatic retry. It accepts only the registered endpoint/identity, three compiled method/path
forms and frozen content-type/body bytes; authentication headers remain outside artifacts.
Each mutation is dispatched once with a deadline. Responses have bounded headers/body and
must have an acceptable JSON envelope/version before another request can proceed. A failed
complete read is retried at most three times, restarting discovery from the beginning.

Transport observations precede direct full-state resolution. Expected before/after comparison
covers all contexts/defaults/dimensions, workspace metadata, experiments/groups, server IDs
and unowned values. Only explicitly certified audit fields and narrowly defined generated
fields can differ. New default function fields must be absent/null; existing function values
are preserved. A rollback status is usable only under the selected certified contract,
with a valid response and unchanged full state/version. Timeout/5xx plus a before image does
not prove rollback or that an old request can no longer commit. Divergence or missing reads
leave the request unknown. A known post-commit error stops subsequent dispatch too.

Resolution is persisted and read back before its typed receipt can clear the coordinator's
in-flight request hash. If an exact direct observation proves commitment but its resolution
append fails, the output reflects known progress (`PartialApplied`/`AppliedUnverified`) with
a journal error; the durable pending claim stays blocked and recovery must re-observe it.
A failure before that observation remains `CommitUnknown`. The durable active run, original baseline, pending-create ownership
and external holds remain. All production success verification is unavailable: successful
V1–V4 receipts in tests are explicitly synthetic and cannot register a real target.

| Outcome | Exit | Required fact |
|---|---:|---|
| NoChangeVerified | 0 | Zero SP writes; all required verification succeeds |
| NoChangeUnverified | 6 | Zero SP writes; verification unavailable/fails |
| Refused | 2 / 3 | Before dispatch; validation / target-authority-conflict failure |
| RejectedNoChange | 7 | Certified definite rejection, unchanged full state, no prior/unknown commit |
| AppliedVerified | 0 | Every planned request observed committed and verification succeeds |
| PartialApplied | 4 | Known commits, incomplete plan, no unknown dispatched request |
| CommitUnknown | 5 | Any unresolved dispatched request; takes precedence over partial progress |
| AppliedUnverified | 6 | All planned writes observed committed; verification/authority/report persistence fails |

Reports expose bound digests, outcome, exit, reason and committed/rejected/unknown request
hashes, not raw rows, credentials, tenant IDs, descriptions or response bodies. Private before/
after images stay in guarded journal artifacts. No `success_with_skips` or force option exists.

## Proposed choices and compatibility limits

- **Transport:** narrow raw reqwest dispatch with locked-SDK wire comparison is proposed for
  exact byte preservation. Actual authentication/base-path/principal/timeout/retry behavior
  must pass the Step 2 disposable-server contract before registration.
- **Bounds:** 512 KiB per body, 32 journaled requests total (at most 31 prerequisites when a
  context bulk exists), 128 events, 32 MiB prepared snapshot and bounded retained direct-state
  images. The planner's existing 100 total actions and smaller certified caps still apply;
  the mutation contract's 1,000-context ceiling does not override planner limits. Aggregate
  recovery capacity is reserved before writes and can further lower the request ceiling for
  large snapshots/baselines. No chunking.
- **Time:** mutation wait is at most **10 seconds**, below Step 9's 15-second dispatch lease
  margin (60-second lease, 10-second heartbeat). This deliberately tightens the plan's proposed
  30-second operation wait until a reviewed lease/deadline contract supports it. Full-read
  attempts are at most 5 seconds; verification is bounded to 40 minutes. A timeout never
  proves server cancellation. Exact supported server timing remains uncertified.
- **Journal:** append event schema is now **2**, adding durable resolution/report events;
  snapshots/prepared records/markers remain version 1. The new reader accepts legacy v1
  intent/observation events and refuses new event kinds mislabeled v1. Older readers refuse
  v2 events. A retained recovery image with this reader must be certified before activation.
- **Terminal lifecycle:** this step deliberately leaves active records/holds in place, even
  after synthetic verified outcomes. A repeated invocation sends zero SP mutations and
  remains blocked until later certified terminal verification/recovery handling. Step 11/12
  will implement those handoffs; there is no optimistic automatic clear/release here.

## Acceptance gates and next work

| Gate | Executed local evidence | Missing evidence |
|---|---|---|
| T12 preservation/second run | Independent payload application preserves shared metadata/order; no-change zero writes; unresolved second acquisition blocked | Certified server preservation and terminal no-change rerun after later handoff |
| T16/T19 wire/retries | Actual SDK serializers and one-attempt error/timeout tests; closed raw adapter | Exact build/auth/base-path/body/action limit and raw transport certification |
| T17 transaction/partial | One-bulk rollback model; separate prerequisites; whole-plan refusal | Actual database transaction and prerequisite fault injection |
| T18 unknown/crash | Lost response, post-commit 512, timeout before/after, dropped executor, durable pending evidence | Actual webhook/cache publication failure and accepted old-request exclusion |
| T20/T21 coordination | Actual session interface, missing barrier/stale authority/expiry refuse; final authority checks | Real writer prevention, platform rollout gates, clock/lease/drain controls |
| T23 journal | Faults before/after intent/transport/resolution/final report; independent filesystem reader; legacy compatibility | Certified store retention, encryption/access, recovery image and independent principal |
| T32 reporting/verification | All eight outcomes/exits, unknown precedence, missing/wrong/expired/timed-out proof never success | Target V1–V4, approvals and target-specific semantic/browser evidence |

Step 2 exact-server, Step 9 real controls, Step 8 real storage, restoration and V1–V4 remain
open. No certified disposable Superposition target exists, so server tests were **not run**.
No mock, successful status or image label is substituted for those gates. After review,
**Step 11 local verifier work** is next on a separate request; it has not begun.

New files: `apply.rs`, `apply_contract.rs`, `apply_observation.rs`, `mutation.rs`,
`apply_tests.rs`, companion `mutation_wire.rs`, and this evidence directory. Small extensions
in journal/coordination/checkpoint code, module imports, journal tests and progress documents
are enumerated in `evidence.json`. No manifest, lock, source business data, SDK, committed
seed, original plan/roadmap or W/U/I content changed during this step.

Suggested H Step 10 commit: `feat: apply required-fields plans with recoverable outcomes`.
Steps 8 and 9 remain separate uncommitted review groups; manually select shared journal,
module/test/progress hunks if committing them separately. No staging, commit, push, remote PR
update or deployment was performed. Stop for review.
