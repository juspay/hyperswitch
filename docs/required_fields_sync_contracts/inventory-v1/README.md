# Step 5 — bounded direct inventory

Local implementation, **18 November 2026**, awaiting review. Real-server completeness is
**not certified**. Every candidate in [Step 2](../server-v1/capabilities.json) remains pending;
the production target registry is empty and returns `CertificationPending` before I/O.
There is no caller-supplied certificate/force flag. Hosted apply remains disabled.

## Source evidence and preserved work

H is on `feat/superposition-required-fields-foundations` at
`299ae5d8d0b48e427fbc88dea0661d16fb8c2030`. Step 4 renderer, profile runner and evidence were
already uncommitted when Step 5 began; those changes were inspected and preserved, not
reimplemented. W remains at `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39` with existing Step 1
untracked tests; U remains detached at its pinned
`1af3e5dabf9d5a17dd864ac7312151ebac4367c6`. No AGENTS.md applied in the inspected H paths;
`docs/CONTRIBUTING.md`, Step 5 and plan §6.4–6.6/P3/T13/T14/T16 were read before edits.
Only affected evidence was refreshed. Original plan/roadmap, seed, source business data,
W/U and Git indexes are preserved; see [results](evidence.json).

Material source refreshes from **locked SDK 0.119.0** (not the roadmap's 0.115.5):

| Source | Observed contract and implementation consequence |
|---|---|
| Generated `operation/list_contexts.rs`, `list_default_configs.rs`, `list_dimensions.rs` | GET `/context`, `/default-config`, `/dimension`; explicitly counted one-based `count/page/all=false`. No prefix, dimension or other filter inputs are exposed |
| `operation/list_experiment.rs`, `list_experiment_groups.rs` | Singular client `list_experiment`; both list routes use POST (`/experiments/list`, `/experiment-groups/list`) with `{}` when unfiltered. Experiment groups/buckets can carry references and must also be retained; a missing route refuses rather than falling back to an incomplete inventory |
| `operation/get_workspace.rs`, `list_versions.rs` | Workspace route contains the workspace name and uses `x-org-id` without `x-workspace`. Versions use `count=1&page=1`; descending/latest ordering and empty boundaries still need server certification |
| `types/_dimension_type.rs` and generated serde code | Wire union is `{"REGULAR":{}}`, `{"LOCAL_COHORT":"..."}` or `{"REMOTE_COHORT":"..."}`. H's `dimension_type_to_value` serving converter flattens regular dimensions to a string. The new reader validates the wire object; serving/SDK behavior is unchanged |
| SDK context/default/dimension/workspace/experiment/group response types; H `crates/external_services/src/superposition.rs:306–500,717–807,973–1038` | Existing serving projections/caches are not complete raw administrative rows. Preserve raw response bytes alongside typed projections instead of round-tripping generated SDK structs; do not use serving cache/file fallback |
| `superposition_core-0.119.0/build.rs` | cbindgen writes a relative `../../target/include/superposition_core.h`, outside `CARGO_TARGET_DIR`. Router checking uses an owned copy of dependency caches, without ambient Cargo config/credentials; no dependency patch or `DOCS_RS` bypass |

Actual source paths/digests and command outcomes are in [results](evidence.json). The seven
request shapes are compared to execution of the actual locked client's builders in an
in-memory connector. This verifies request construction only, not server semantics.

## Implemented behavior

- Dedicated `DirectAdminReader`: fixed seven read operations, explicit authorization header,
  registered endpoint/org/workspace and an independently reviewed credential fingerprint,
  no ambient secret/settings loading. A different authorization header cannot inherit a
  principal's certified permission scope. HTTPS only, no
  proxy, redirects, decompression, retry, arbitrary URL/body forwarding or mutation API.
  The two POST operations above are read-only lists. Inventory acquires no lock.
- Complete contexts, defaults, dimensions, all experiment history, groups and workspace
  settings, with server-returned IDs. Full rows, schemas, function names, audit metadata,
  unknown fields and page envelopes are retained. No validation/computation function runs.
- Each collection follows its own counted-page and empty-boundary contract. Missing pages,
  duplicates, contradictory totals/boundaries, partial/filtered/error responses, invalid
  JSON/required fields/known enum shapes and exhausted bounds refuse the whole inventory.
  Consistent totals alone cannot prove permission scope; a reviewed independent permission
  receipt is a mandatory sealed target prerequisite.
- Two full passes, with latest version observations before/between/after. Compare full
  rows and envelopes, not just counts/IDs/versions. Version/workspace mismatch, unchanged
  version with changed metadata or ordered arrays, duplicate conditions and observed
  override-ID collisions refuse. No partial/resumed result escapes after a failed read.
- Duplicate JSON keys are rejected before map insertion, including escaped spellings.
  Raw numeric lexemes survive without floating-point reserialization. Canonical comparison
  sorts object keys and normalizes string escapes only; arrays, duplicates and row order
  remain significant. Numeric `1` and `1.0` conservatively differ. Local hashes bind target,
  generation, contract/client/server/principal/permission evidence and full observations;
  they are never used to guess server IDs.
- Unsupported object predicates are retained as `QuarantinedUnsupportedPredicate`.
  Malformed top-level conditions refuse; no owned `ContextKey` is inferred. Step 6 must
  refuse unsupported desired collisions/overlaps. Historical experiment references remain
  visible even if their original context is gone; inventory does not infer deletion rights.
- `CompleteInventory` has no public unchecked constructor or deserializer; consumers receive
  immutable views and a completeness proof only after all checks. Every proof explicitly
  remains provisional without writer exclusion. Synthetic proof scope exists only in tests
  and cannot dispatch HTTP through the production adapter.

## Local choices and assumptions requiring review

These are implementation bounds, **not certified server limits**: default page size 100,
32 pages/resource, 5,000 rows/resource, 30,000 total observed items, 2 MiB/response,
32 MiB aggregate body bytes, 400 requests, 10 seconds/request and 60 seconds/attempt.
Aggregate budgets include both passes and all three version observations. JSON limits
are depth 32 and 100,000 nodes/body. Validated hard ceilings prevent unbounded overrides.
One attempt makes no automatic retries; a caller can start a new complete read after a
refusal. This deliberately tightens the plan's proposed caps/retry allowance.

The plan prefers `all=true` when its completeness/size contract is certified. None is
certified here; this implementation selects bounded counted traversal per resource and
keeps it disabled pending that contract. It does not guess an `all` convention or fall
back automatically between pagination modes.

Stable counted row order, full row disclosure/all experiment history, latest-version order,
empty boundaries and non-filtered permission scope are **required future contract facts**,
not facts inferred from the SDK or synthetic responses. Unknown/new wire variants refuse
pending explicit support. An older server without groups or the required full fields is
unsupported until a separately reviewed contract proves a complete alternative. No 404
fallback is implemented. HTTPS-only transport is a conservative local restriction; it does
not certify the checked-in plaintext service URLs or a deployed ingress. The credential
fingerprint is private, domain separated and included only inside the inventory digest;
credential rotation requires a fresh reviewed binding/permission receipt. No raw token
or its fingerprint is emitted in the completeness report. A future real registration
must obtain that binding from approved restricted permission evidence; real credential
fingerprints must not be committed as source entries. This step registers no real target.

Business mappings/catalog and Step 4 full-profile blockers are unchanged. Source/SDK
corrections remain separate review groups. This step does not enable owned-row planning,
apply, restoration, deployment, or target V1–V4 acceptance.

## Handoff

Review [checks](checks.md) and [sanitized results](evidence.json). Suggested H commit:
`feat: inventory required-fields configuration without loss`.

After review, **Step 6's local ownership/preflight/planner work** is next in playbook order;
its acceptance still needs Step 4 approvals and Step 2 certified ordering/projection/read
scope. Step 2 certification can resume independently only with explicitly registered
disposable targets and its required facilities. No Step 6 work was started.
