# Step 4 — pure required-fields renderer

Local implementation, 2026-11-18. Full supported-profile acceptance remains **blocked on mapping/catalog review and malformed source data**. Hosted apply stays disabled. See [checks and gates](checks.md) and [sanitized profile results](results.json).

## Source evidence

H starts this step on `feat/superposition-required-fields-foundations` at `299ae5d8d0b48e427fbc88dea0661d16fb8c2030`. Its tracked tree was clean; the original implementation plan and roadmap were already untracked. No applicable AGENTS.md was found in H or its ancestors. The playbook's Step 4, plan §6.3/P2/T01–T10, current contributing/build conventions and existing Step 1/3 code were read before changes.

W remains `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39`; U remains pinned at `1af3e5dabf9d5a17dd864ac7312151ebac4367c6`. Their existing harness/source work is preserved. Step 1 browser results are historical evidence, not a new browser execution for rendered bundles.

Material discrepancies refreshed against these sources:

- U `sdk-utils/utils/SuperpositionHelper.res` consumes `._dropdown_options`. Step 3's `ManagedKey` accepted `._options`. The validator/tests now use the actual key and explicitly refuse the old spelling. Desired/bundle/build renderer schema advances to **2**; canonical encoding and input contract schema stay at 1. Common conditions omit `mandate_type` entirely.
- Step 1's two Givex examples use a synthetic connector named `givex`, which is not a compiled `Connector`. The actual constructor scope is **Adyen/GiftCard/Givex/non_mandate**. Tests retain the corpus unchanged and separately assert actual outer `gift_card.number/cvc` versus inner `gift_card.givex.number/cvc` paths.
- `PaymentMethodType::ClassicReward` displays/parses as `classic_reward`, but serializes as `classic`. Source identity validation now uses the compiled FromStr/Display contract. Rendering this unresolved SDK label refuses; no enum or SDK correction is inferred. All other 125 v1 types and 16 methods match the inspected snake-to-Pascal source contract.
- The existing bank-settings deserializer rejects `banks = ""`. The offline example preserves that behavior. Programmatic `Some(empty set)` remains distinguishable from an absent bank entry and is tested separately; this is not a settings-parser fix.

## Implemented choices

`render(RenderInputs)` uses the existing captured `RequiredFields` and effective bank inputs, explicit policy, field catalog, exclusions and build identity. It performs no I/O. Exhaustive enum matches require review when dimensions or FieldType grow. Output contains only true requiredness and explicit ordered options, plus prerequisites, retained inputs and scope evidence. Missing source options inherit; explicit default-equal values, duplicates, case and Unicode remain intact. Only the established EPS HashSet provenance normalizes order.

Bindings are exact source scopes. Alias approvals also bind the expected inner path; catalog approvals declare compatible source types, render kind, write/read paths and any inherited defaults. Old catalogs with no declared source types cannot authorize a field. Duplicate bindings/output producers, overlapping write paths, incompatible common/specific descriptors, unfillable required selects, unsupported options and malformed countries refuse the whole bundle. Errors retain totals/counts with at most 256 deterministic samples, including a representative of each refusal category; no partial desired output is returned.

Country ALL expands only an approved ordered catalog. Nonempty State allowlists conservatively refuse because no country/asset-scoped acceptance contract has been approved; missing/empty State options remain distinct and supported. Exclusions are explicit exact scopes. Empty groups, absent features and dummy exclusions never grant retirement authority. Cross-connector eligibility/ordering and live presentation prerequisites still belong to later whole-plan preflight/verification.

The opt-in [offline example](../../../crates/payment_methods/examples/required_fields_render_profile.rs) accepts explicit profile and release-identity TOML files, constructs the full `RequiredFields::new(bank_config)`, and creates a new report file. It never initializes Settings secrets, AppState or a client. Standalone offline build identity records actual payment_methods features and an empty router feature set; it does not attest a compiled router or release image. This example is not Step 7's router sync command.

## Decisions and blocked profiles

Synthetic `approved` values in unit tests authorize only those test cases. The supplied [pending profile](profile-pending.toml) intentionally contains no approved bindings, catalog fields or automatic exclusions. Full EPS AB, EPS A and absent-EPS runs each report **1,625 missing bindings and three malformed-option occurrences**, exit 2 and no desired bundle. Empty-string EPS input exits 1 before render. Independent one-rule compiled-EPS projections prove option/hash variation; they are not a successful full profile.

The [Step 1 decision register](../sdk-v1/decisions.md) remains proposed: exact issuer/Givex bindings, lowercase bank-type activation, country catalog/AC–TA assets, EPS/dummy scope, Card/Debit coverage and connector aggregation need owner review. The existing comma-country source and Getnet Credit-only card-network assignment are untouched. Any approved source, presentation or SDK correction must be a separate review/commit with its own legacy/browser acceptance. No seed migration is included.

## Review handoff

Changed H groups: payment_methods renderer/contracts/tests and example wiring; this evidence folder; progress/questions/playbook checkpoint. Original plan/roadmap, source business rules, seed, dependency lock, W/U and Git indexes are preserved. No staging, commit, push, hosted access or later step.

Suggested H commit: `feat: render required-fields Superposition contributions`.

After review, **Step 5 local inventory work** is next in playbook order; certified inventory still requires Step 2's relevant real read contracts. Step 4's full-profile approval gate remains open, and Step 2 certification can resume independently only with its verified disposable dependencies. Stop for review.
