# Step 14 — offline seed and independent CI checks

18 November 2026. Local implementation; **complete seed adoption and target certification are
pending**. Nothing uploads, starts serving, acquires a lease or enables hosted apply. The existing
committed seed and uploader are unchanged. [Executed checks](checks.md) and
[machine-readable evidence](evidence.json) distinguish passing local tests from missing gates.

**Assumptions:** local generation is not a live ownership grant or a deployed release claim.
Business decisions remain unresolved rather than inferred from the existing seed.

**Implementation choices for review:** a separate strict local-grant policy binds the input
profile and features; the already-locked TOML editor preserves formatting; a hashed archive pins
W's existing uncommitted verifier files. CI separates explicit dependency/image provisioning
from isolated execution. None of these choices supplies missing business/platform approval.

## Implementation and review boundary

The existing opt-in router binary now accepts:

```sh
router required-fields seed --input-profile config/required_fields_sync/local-profile.json \
  --seed-policy config/required_fields_sync/local-seed-policy.json \
  --template config/superposition_seed.toml --check

router required-fields seed --input-profile <reviewed-profile.json> \
  --seed-policy <reviewed-local-grants.json> --template config/superposition_seed.toml \
  --output <new-seed.toml>
```

`--check` and `--output` are mutually exclusive and one is required. Check mode never writes;
drift returns sanitized `seed_drift`, exit 2. Generation requires a new private output file;
it cannot overwrite the template. The new `just required-fields-seed-check` and
`just required-fields-seed-generate` recipes run Cargo offline/locked. Existing
`just superposition-seed` remains the separate mutating uploader and is not a generation step.

The command reuses the compiled `RequiredFields` renderer before application initialization.
It requires an explicit seed constructor profile with a bank vector, a bound profile hash and
the exact compiled feature sets. Effective/ambient Settings selection is refused. Existing
renderer policy, catalog, exclusions, strict parsing, size bounds and redaction are reused.

Local ownership grants name exact conditions and managed attributes. Missing grants, duplicate
condition identities, malformed managed values, unsupported schemas/functions, unapproved
scope and undeclared retirement refuse. Only `_is_required` / `_dropdown_options`, declared
missing managed defaults and approved append-only dimension enum extensions can change.
Existing presentation/default corrections are not generated. The planner's prerequisite and
descriptor validation is reused; missing dimensions/metadata refuse. Reports bind profile,
policy, bundle, template, output and preserved-content hashes without dumping values.

TOML editing retains existing row order, comments/decoration where practical and equal values
verbatim. Ordered options are never sorted or deduplicated. A second parsed-content comparison
masks only explicitly owned attributes and added prerequisites; all unrelated values, country,
tenant/baseline rows and Deja defaults must remain equal. Explicit default-equal options survive.

## Explicit local profile and initial migration

**Source evidence:** `local-profile.json` lists the 27 Stripe EPS banks from the development
bank HashSet, normalized only because that source collection is unordered. Its offline release
hash covers the renderer/source files listed in [local-profile-source.json](local-profile-source.json).
It is not release-image provenance. The policy pins the actual default-v1 router/payment-method
feature closure, including dummy connectors. Source business data is unchanged.

**Unresolved choices:** full binding/field catalogs, unrestricted-country/AC–TA decisions,
aliases, unsupported SDK aggregation cases, precise local grants and the initial migration
review still need owner approval. Both ownership and unrestricted-country decisions explicitly
refuse; there are no automatic exclusions. The actual binary reports 1,625 missing compiled
bindings and three malformed-option occurrences. See [business decisions](../sdk-v1/decisions.md).

**Initial managed seed diff:** none. A synthetic one-source fixture demonstrates only the
independently expected false-to-true requiredness change, leaving its manual data untouched.
That fixture grants no authority over the committed seed. Approving a full seed requires a
separate reviewed mapping/catalog/grants change, then generation to a new file, an explicit
managed diff and independent semantic review. Source/SDK corrections must remain separate.

## CI and pinned verifier

`.github/workflows/CI-pr.yml` calls the new reusable offline workflow. It checks actual renderer,
planner, CLI, contract-unit and Rust-resolver code, supported builds and an independent JS
oracle. The oracle deliberately rejects `[B,A]` when the literal expectation is `[A,B]`, even
though regeneration/check equality succeeds. Full-seed acceptance is an explicit failing
opt-in while business approval is absent; routine synthetic CI is not that acceptance gate.
After profile/migration approval, replace the expected-refusal fixture and promote complete
seed `--check` into the required PR path. That dependent acceptance work is still pending.

The browser job pins W `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39` and U
`1af3e5dabf9d5a17dd864ac7312151ebac4367c6`, Node 26.0.0, all three npm lockfiles,
the H case pack and the verifier archive/file hashes in
[`verifier.lock.json`](../../../scripts/required_fields_ci/verifier.lock.json).
`verifier-source.tar` contains the exact 30 already-existing W Step 1/11 test files. Those
files are still uncommitted in W; the content-addressed archive makes their identity reviewable
without inventing a W revision or updating U. Archive extraction refuses links, traversal,
duplicates, missing files and changed bytes. A source copy excludes dotenv/npm credential files.

Provisioning is separate from execution: npm lifecycle scripts are disabled; no remote-tracking
submodule updater or default live Playwright project runs. Actual ReScript/TypeScript/webpack,
resolver, component and checkout tests execute with non-loopback networking denied and a clean
child environment. Confirm is intercepted. The actual browser results remain distinct from JS
resolver and Rust results, and bind only the synthetic supported case/response/artifact scope.

The manual server-contract workflow requires explicit public image digests/source revisions.
Its separate provisioning phase validates references, pulls only those selected immutable
artifacts using an empty Docker configuration and a fixed local Unix socket, then invokes the
existing ownership-verifying disposable harness. The harness itself does not pull images. No
floating version, Docker login, hosted endpoint or uploader is accepted. Unavailable artifacts
or missing exact source/ownership/capability evidence fail; unit mocks are not certificates.
No server workflow, image pull or uploader was executed locally in this step.

## Refreshed discrepancies and remaining gates

- Current lock uses client **0.119.0**, not the plan's historical 0.115.5. MSRV is **1.94.1**,
  not historical 1.85. No package identity/checksum changed; only direct edges to already-locked
  `toml_edit 0.22.26` were added in H and the isolated check crate.
- Compose remains **0.113.0**, development compose **0.115.5**, and five Cypress services
  **0.102.0**. CI guards these candidates separately; no server version was changed.
- The unchanged seed has **8,889 lines, 1,704 defaults, 23 dimensions and 912 overrides**,
  with no duplicate exact context conditions. Manual `flow_name` and `merchant_connector_id`
  both declare dimension position 21. This observation is not server-validity evidence; retain
  both and review their contract separately before upload, rather than silently renumbering.
- T01–T09/T31 have local synthetic evidence. T10 and uploader/Deja API regression still require
  verified disposable exact-server fixtures. Static uploader checks are not runtime proof.
- T29/T35–T38 cover the pinned synthetic browser scope; deployed/mobile/old artifacts and actual
  target V1–V4 remain pending. T33's complete MSRV/TLS/Redis/v2/release/platform matrix is not
  implied by supported local v1 builds. New Linux CI execution itself remains unexecuted.

After review, Step 15's disabled deployment integration is next only on a new request. Pending
business/server/platform acceptance can be resolved independently within explicit authorization.
No later step was begun.

Suggested H Step 14 commit: `ci: check generated required-fields configuration and SDK semantics`.
Keep the pre-existing Step 13 command/attestation changes separate where practical. W's existing
test packages can be committed separately as `test: add pinned required-fields browser verifier`;
this step made no W/U/I edits. Business-source, catalog approval and SDK fixes need their own
reviewed commits. Nothing was staged or committed.
