# Step 6: pure ownership, prerequisites and semantic planning

Local implementation dated 2026-11-18. Exact-server and bound full-browser acceptance
remain **pending**. No target, mutation, lease, uploader, container, secret provider or
AppState was used. This record does not authorize those operations.

Read [checks](checks.md) and [machine-readable evidence](evidence.json) together. The
implementation follows playbook Step 6, plan §6.3–6.7, §7.4 and P3. The original plan
and roadmap are preserved. Step 7 has not started.

## Existing work and source evidence

H began at `6a2e1869643b4008fe6e46cdd4c3b1873712f0c8` on
`feat/superposition-required-fields-foundations`. Steps 4 and 5 are already committed
(`2785cd4279152cd296bdd3e1284cd3c697a58013` and the current HEAD). Their renderer,
strict artifacts, direct reader, raw JSON preservation and sealed `CompleteInventory`
are reused. Only the original plan/roadmap were initially untracked in H.

W remains `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39` on main, with its pre-existing
untracked Step 1 harness. U remains detached at W's pinned
`1af3e5dabf9d5a17dd864ac7312151ebac4367c6`. No W/U files or submodule pointers changed.
No applicable AGENTS.md was found in the inspected ancestors/subtrees; H's
`docs/CONTRIBUTING.md` was read. Final preservation evidence is in evidence.json.

Relevant current symbols, replacing stale roadmap references:

| Source | Evidence and consequence |
|---|---|
| H `payment_methods::configs::required_fields_renderer::{render, RenderedBundle, BuildIdentity}` | The adapter re-renders the retained effective inputs, policy, catalog, exclusions and build identity, then compares the entire bundle hash. Public bundle fields cannot inject an unrelated desired map. No source business-data or renderer behavior change is needed. |
| H `required_fields_sync::{inventory, types, raw_json}` | The planner takes only `CompleteInventory`; no JSON deserializer or public completeness constructor exists. Full raw rows, version export and proof bindings are retained. |
| Locked `superposition_core` 0.119.0 `src/helpers.rs:39–99` | Position weights use powers of two. The custom override hash sorts arrays and joins strings without escaping separators. The executed core probe confirms both reordered-array and comma-boundary collisions. This is **core evidence, not server certification**. |
| U `SuperpositionHelper.res:15–54,81–176,253–339` | Billing force, first included confirm-path selection, prefill and descriptor parsing are distinct operations. The reference selection model is not a React/browser renderer. |
| W Step 1 `contract.cjs`, `evaluator.cjs`, `report.schema.json` | Actual pinned CacReader is reused in a fresh bounded VM per case. Step 1 reports alone do not bind a planner request or authenticate execution. The new wrapper binds both; its production trust adapter remains unregistered. |

Earlier Step 1 browser results are historical evidence for that fixed corpus. They are
not reused as successful reports for these projected configurations. The five current
synthetic profiles compare **396 JS and 396 Rust resolver observations**, separately,
and include independent literal assertions for the common/mandate counterexample.

## Implemented boundaries

`ownership.rs` classifies fully managed/shared rows, unowned adoption candidates,
explicit exclusions, unsupported conditions and unrelated rows. Grants bind the
target/workspace/instance generation, owner/revision, compiled feature availability,
exact condition, observed server ID, complete row hash, delegated attributes and review.
Descriptions never confer ownership. A create grant cannot adopt a future manual row.
New creates carry deterministic ownership-intent digests and have **no guessed server
ID**. Intent persistence/confirmation belongs to the later journal step.

The planner constructs `unrelated current values + desired delegated values`. Removing
the last delegated attribute preserves a shared row; it never emits an empty
replacement. Deleting a fully managed row requires explicit delegation and exact impact
approval. An empty desired set for an active owner additionally needs the exact reviewed
retirement manifest. Exclusions and a changed feature scope refuse automatic retirement.
Descriptions and unknown raw child values are preserved, including large numeric
lexemes and ordered duplicates. Before/after images are **logical restricted artifacts**,
not HTTP bodies: old observed audit fields/override IDs are retained for review, not a
prediction that server-maintained fields remain unchanged after a future write.

`preflight.rs` validates live dimensions, workspace flags, regular dimension positions,
typed vocabulary, default schemas, exact reviewed presentation, paths, render kinds and
option values. Missing defaults permit only declared exact creations. Presentation must
exist in live effective data even if a seed/catalog is valid. Unknown functions are
refused and never invoked. Additive enum actions retain existing member order and all
other row/schema properties; dimensions and positions are never created/changed.

`impact.rs` checks the observed version export against complete rows/defaults and
returned override references. It compares ordered overrides across **both** observed and
projected stores through the sealed identity contract. It enumerates the complete
supported equality partition (observed literals, missing, fresh other; both booleans),
all declared co-eligible connector permutations and both billing-force states. Required
and option changes, shadowed edits, order conflicts and managed-attribute counts remain
separate. Missing defaults deliberately yield separate JS and Rust reference hashes.

`plan.rs` rejects retained experiment context/override references, active overlaps and
unresolved group/bucket membership. Global prerequisite actions refuse any active
experiment. Bound candidate reports expose only hashes/counters and static remedies;
arbitrary row values, descriptions and SDK payloads stay in restricted artifacts.

`examine_cases` validates all eight before/after stage receipts and returns sanitized
browser-effect counts plus an evidence hash for review. `accept` requires an exact,
unexpired approval for every action or connector-order conflict. The final hash binds
target/instance, full inventory/reader permissions, release intent/generation, bundle,
grants, prerequisites, server/SDK contract, exact case request, impact, action images,
runner receipts and approval. Even a no-action plan needs every SDK stage. Inconsistent
browser results for identical before/after inputs refuse. Every result remains
provisional, with hosted apply disabled.

Assumptions are confined to synthetic fixtures and the proposed closed projection profile below. No deployed ordering, permission scope, reader eligibility or browser outcome is inferred from those assumptions; registration and execution evidence are required.

## Proposed supported profile and limits

These restrictions are implementation choices for review, not new business approvals:

- Only regular string/boolean equality predicates are modeled. Numeric predicates,
  cohorts, dependent dimensions, attached functions, unknown ordering, export gaps and
  overlapping unresolved timestamp ties refuse. Positions are unique and bounded 0–30;
  observed weights must agree. Exact deployed ordering still needs certification.
- All four managed dimensions need nonempty string enums and a reviewed typed vocabulary.
  Schema support is deliberately closed: strings/booleans with enum, string arrays with
  item enum/minItems/maxItems/uniqueItems, plus title/description. Unsupported constraints
  refuse even when the current array is empty. No schema/function is weakened.
- Presentation uses a closed list from the pinned U parser and exact reviewed values.
  Optional fields need not be invented. `_extended_dropdown_options` may be retained
  exactly but is never treated as this browser's choices. Intentional no-prefill/null
  contracts and additional schema/function profiles need explicit support before use.
- Simultaneously included aliases within one connector sharing a confirm path refuse;
  their field-enumeration ordering is not certified. Cross-connector ordering is enumerated
  and conflicts require approval. Excluding a source conservatively blocks ownership of
  its connector until a reviewed handoff profile exists.
- Limits: 512 observed/projected contexts; 100 total actions; 128 resolver partitions;
  at most four connectors per declared set and 24 total orders; 4,096 evaluations;
  128 browser matrix entries; 256 KiB request, 64 KiB per receipt, 128 KiB manifests and
  1 MiB aggregate action images. Exceeding any bound refuses, without sampling or batching.
- No automatic additive policy is enabled. Existing seed/default values are never
  repaired here. Capitalized BankType, Nordea comma options, country assets/AC–TA,
  aliases and full-profile binding approvals remain separate owner decisions.

## Case and report contract

The restricted request `required-fields-planner-cases-v1` carries bound inputs,
Step 1 pack identity, partition IDs/contexts, managed/competing fields, complete
before/after serving projections and browser matrix entries. A matrix entry binds its
context, ordered connectors, billing force and reviewed scenario IDs. Local
`projection-N` override references and `pending:` context labels are case-artifact
references only; they never become server identities or write requests.

Each strict wrapper contains `schema_version`, `request_hash`, `phase`, the unchanged
Step 1 `report` shape, `reference_hashes` and `browser_observations`. Required stages are
`js-resolver`, `rust-resolver`, `browser-components` and `checkout`, each before and after.
Provenance, pack, runtime/artifact, case IDs, all statuses and request identity must match
the sealed contract. Resolver observations must match their respective reference hashes.
Browser observations bind each matrix hash, selected-descriptor **reference projection**,
included fields, visible fields, validation and intercepted-confirm payload hashes.
The reference selection model does not produce actual browser outcomes; trusted executor
receipts must supply them. Browser component and full-checkout effects are reported separately.

`ProjectionContract` has private fields, no deserializer/public constructor, and an empty
production registry. Its exact-build identity function and authenticated receipt set can
only be registered by a future reviewed adapter. Synthetic unit fixtures exercise that
boundary under `SyntheticFixture`; they cannot certify a target or issue production trust.
The offline resolver comparison deliberately issues **no trusted receipt**. Report `passed`
strings, caller booleans, CLI JSON and the static Step 1 report cannot bypass these gates.

## Remaining acceptance gates

Exact image/source/permission/pagination/ordering/transaction/identity certification from
Step 2 is still missing. OrbStack availability was already recorded; this step neither
rechecks it nor treats it as certification. No hosted target is named or authorized.

The contract-bound executor for the new browser matrix, authenticated receipt registration,
actual component/checkout execution on these candidates, deployment reader coverage and
business/profile approvals remain pending. Full compiled renderer profiles still have the
previously recorded 1,625 missing bindings and three malformed-option occurrences; this
step does not waive them. Journal, coordination, restoration, release/MSRV/feature matrix
and target V1–V4 remain open.

After review, the next implementation slice is **Step 7's offline/read-only CLI**, keeping
uncertified target/plan gates closed. Step 2 certification and Step 1 owner/case work can
resume independently with their explicitly registered fixtures. Neither begins here.

Suggested H commit: `feat: plan required-fields changes with explicit ownership`.
No separate W/U commit is needed for this step. No staging or commit was performed.
