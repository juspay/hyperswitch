# Step 11 — local serving and fallback verification

2026-11-18. Local implementation is ready for review. **Every target V1–V4 certification remains pending; hosted apply is disabled.** No hosted target, credentials, real pod, backup publisher or deployed companion was accessed. “Post-apply” below is a phase exercised with supplied synthetic responses; no server apply was performed.

## Starting state and scope

H `feat/superposition-required-fields-foundations` at `480e15c04a1524b7c297d1a8df761fcef55be4fa`; W `main` at `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39`; U detached at W's exact gitlink `1af3e5dabf9d5a17dd864ac7312151ebac4367c6`; I `main` at `92041e63f025dc838388a489a5511ba74f387590`. H already contained uncommitted Steps 8–10. W already contained untracked Step 1 harness files. Baselines include their bytes and Git index hashes. These changes are preserved, not reimplemented.

Read Step 11, plan §6.8/§6.9/P5 and the relevant T07–T10/T28–T30/T34–T38 checks; inspected repository instructions, contributor/build conventions, Step 1/6/10 contracts, serving/authentication paths and actual locked provider sources. No applicable AGENTS.md was found. Original plan/roadmap, business data, SDK behavior and committed seed were not edited.

## Implementation and evidence by level

| Level | Implemented / local evidence | Required external evidence still missing |
|---|---|---|
| **V1 persisted** | Sealed CompleteInventory bound to the apply inventory/target/read contract; raw-row changes invalidate the binding. Latest and explicitly versioned exports must match the observed version head, all defaults, ordered contexts/override references/values and dimensions. Tests also corrupt both exports and the head together, independently of direct rows. | Exact-server read/permission/cache/order contracts, real export bodies and verified writer exclusion. No disposable Superposition server was certified or contacted. |
| **V2 per-pod cache** | Complete declared roster, reader/build/input identities, both existing authenticated SDK-config routes on every pod, direct-address capability, complete body fingerprints and final roster/release recheck. Missing pods, replacement/scaling, stale bodies, missing metadata and expired observations refuse. Bounded read transport uses no proxy, redirect, mutation retry, secret query, cache inference or arbitrary URL forwarding. | Registered platform discovery/traffic gate, authenticated direct-pod tunnels, synthetic provisioned payment/profile inputs and actual every-pod content convergence. A UID/version response header or load-balanced sample is insufficient. |
| **V3 browser** | W's pinned package executes actual constructors, connector aggregation, components, complete loader/frames/checkout, validation and aborted confirm payloads. Step 1 JS, component and checkout reports remain separate. Three exact response/input phases and retained/fresh sessions are bound to request/run/target/plan/reader/cases/assets/runtime hashes. H independently parses and rejects stale/missing/failed/rebound reports. Local receipts refuse CertifiedServer scope. | Deployed companion image and runtime/mount attestation, all candidate matrices, supported old/mobile readers, real assets and actual post-commit per-pod responses. New bound checkout driver currently supports Card/Debit, US, normal payment and billing line1 only. Other matrices refuse or remain pending; the broad Step 1 corpus is not arbitrary-response coverage. |
| **V4 backup/startup** | Target/reader/version/publication-time/content binding; actual pinned FileDataSource JSON/TOML parsing and fresh non-serving LocalResolutionProvider with an absent primary. Incorrect content/order, corrupt/missing backup and available primary refuse. Running-provider last-good memory is tested separately after both files disappear. | Backup image/source, atomic publication, target selection, timestamps/checksums/mount/access provenance, actual startup on every supported reader image and retention/recovery-role evidence. A local parser test is not backup-publication certification. |

`ServingVerifier` uses the Step 10 ApplyVerifier binding/receipt interface. It cannot issue a production success without a private reviewed registration. Both verification and read factories remain closed. The companion contains no writer client or credential fetcher. No mutating CLI wiring, endpoint, daemon, lease acquisition, automatic batch fallback or Step 12 recovery transition was added.

Reports expose hashes and static statuses; raw config and credentials do not enter public outcomes. The restricted report reader reuses journal SecretGuard and duplicate-key rejection. Requests/JSON/body sizes, case/pod counts, aggregate input size, process/test lifetimes and overall verification time are bounded. Missing evidence returns verification failure; it cannot become a no-diff success.

## Source findings and material discrepancies

1. **Locked library version:** provider/core/types/SDK are **0.119.0**, not the plan's older 0.115.x references. Cached `.crate` SHA-256 values match H Cargo.lock; unpacked members of all four packages matched their archives. No library source patch was made. New opt-in direct dependencies reuse these versions. The isolated dependency graph was reconciled to H's lock, rather than accepting newer transitive resolution.
2. **Actual backup format:** FileDataSource uses core JsonFormat/TomlFormat, which consume markup with `default-configs` and schemas. Raw `/config` JSON is a different shape. The first raw-JSON startup fixture failed and was retained as failed evidence; independently authored JSON/TOML markup fixtures now pass against the actual provider. No successful fake parser substituted for it.
3. **Known export omission:** `DimensionInfo.value_compute_function_name` is omitted when None in types 0.119.0. V1 accepts only this proven absent/null equivalence; missing schemas/positions/other metadata still refuse. Full direct-row metadata remains bound separately. Description is not part of the resolution export.
4. **No initialization shortcut:** canaries construct a fresh Manual-refresh provider directly; they do not start AppState, DB/Redis, global OpenFeature or the existing SuperpositionClient writer. Initialization fallback and refresh-time retained memory are separate pinned behaviors.
5. **Exact export wire method:** the actual locked-client serialization test caught that `get_config` is `POST /config` with `{}`, optionally `?version=…`. The read adapter now matches that method/body; both router SDK-config paths remain GET. No arbitrary POST/body forwarding or mutating operation is exposed. Initial incorrect GET evidence is retained as failed, not certified.
6. **JSON feature boundary:** Step 11 input/provenance/derived-pack hashes sort object members explicitly; they do not depend on the isolated harness’s `serde_json/preserve_order` feature. Arrays and byte-exact response/request hashes are unchanged. Router/SDK JSON behavior was not modified.
7. **Actual SDK scope:** W/U still omit some planner partition dimensions from the browser constructor and have reviewed first-connector/alias/Card-Debit behavior. This step does not manufacture those dimensions, change SDK logic, approve business choices or claim old readers are covered.

## Assumptions and proposed operational choices

- The future registry must bind read-only transport, complete reader discovery and writer exclusion to Step 9 authority. Caller-supplied JSON/boolean claims cannot register a target.
- The plan's cache 120 seconds / backup 40 minutes remain **proposals**, not measured target freshness guarantees. The local contract takes an absolute deadline and refuses expiry; a 15-second poll is not proof. No operational deadline was approved here.
- Same-Job companion packaging, immutable image, resource limits, read-only input/output mounts, no service-account/writer-secret exposure and filesystem/process isolation require platform review. Local environment whitelisting and OS network denial are evidence only for the isolated process. Pod NetworkPolicy cannot isolate containers from each other.
- Published backup receipt integrity and direct-pod identity must come from trusted platform adapters. No such adapter is registered. Unavailable targets/readers remain pending independently of completed local work.

## Review groups and next work

Exact files, commands and hashes are in [checks](checks.md) and [evidence](evidence.json). H adds `verification*.rs`, shares the existing pure context-order helper, imports modules in the opt-in boundary, adds provider/types dependencies and focused companion/locked-client checks. W adds only `playwright-tests/required-fields-verifier/`; its preexisting Step 1 files and U remain unchanged. No I change.

Suggested separate manual commits:

- H: `feat: verify required-fields serving and fallback state`
- W: `test: package pinned required-fields browser verification`

Keep preexisting Steps 8–10 and W Step 1 work distinguishable when reviewing shared files. After review, **Step 12 local resume/restore** is eligible only on a separate request. Independently obtain missing Step 2/server, platform, business-case and supported-reader evidence with explicitly authorized disposable targets. No next step, staging, commit, push, remote PR or deployment was performed.
