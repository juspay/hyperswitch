# Step 3 — pure input and artifact foundations

2026-11-18. Local implementation only; renderer, CLI dispatch, transport, target adoption and hosted apply are absent. See [checks](checks.md) for executed evidence and pending acceptance gates.

## Source evidence

- H: `main`, `82fe5dfd3ac527fd4a1e03dc01fbb5be13e3198b`. Initially no tracked changes; Step 0–2 documents/harnesses were already untracked. No applicable AGENTS.md found in H or ancestors. Read the contributing guidance, current Justfile/CI, Step 3 and plan §6.1–6.2/§6.9/P1 before editing.
- W: `main`, `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39`; its Step 1 harness remains untracked. U: detached `1af3e5dabf9d5a17dd864ac7312151ebac4367c6`, matching W's pinned gitlink. Neither repository is changed by this step; earlier browser results are historical evidence, not rerun here.
- Existing `payment_methods::configs::settings::{RequiredFields, BankRedirectConfig}` remain unchanged. `RequiredFields::new` in `payment_connector_required_fields.rs` takes bank configuration; its only current setting read is EPS → `stripe`. `EpsBankOptions` creates a vector from that HashSet. The exact source identity is `(bank_redirect, eps, stripe, non_mandate, payment_method_data.bank_redirect.eps.bank_name)`.
- `api_models::payment_methods::RequiredFieldInfo` includes a secret prefill value; the retained projection has no such member. `FieldType` has 56 current variants, nine with option payloads; its custom `PartialEq` ignores some payloads. Hashing uses full serialized field types, never that equality implementation.
- Router `Settings<SecuredSecret>::with_config_path` already selects the file via `RUN_ENV`/`CONFIG_DIR` and applies `ROUTER__` overrides after it, then constructs required fields from effective bank settings. The new loader calls that existing abstraction; it does not call validation, secret conversion, AppState or a provider.
- The current `router/v1` feature includes `common_default`, which includes dummy connectors and TLS. A claimed router v1/no-dummy build would be inaccurate without a separate feature refactor. No such refactor was made. Existing v2 Justfile recipes enumerate all features, so their exclusion lists must also omit this new v1-only feature.

## Implemented choices

- Pure `RenderInputs`, retained input/build identities, compound context keys, managed values/keys, desired-state/prerequisite/diagnostic contracts, plus router plan-binding and foundation-report boundaries. A compound context map serializes as deterministically ordered entry tuples, because JSON object keys cannot be structs.
- Strict, bounded TOML mapping/catalog/exclusion parsing: missing or unknown fields, unsupported versions, invalid references, unrecognized decisions and duplicate source scopes refuse. Mapping/exclusion source names additionally validate against the actual compiled Rust enum vocabulary; manifest construction repeats that validation. Approved entries require a review reference; unsettled entries remain `Refused` and cannot be retrieved as approved. This schema is not business approval.
- Version 1 canonical UTF-8 JSON sorts object keys even when another crate enables `serde_json/preserve_order`; it preserves array order, duplicates, case and Unicode. Hashes use domain-separated, versioned SHA-256 envelopes. Floating-point inputs refuse. Typed managed keys allow only `dynamic_fields.<path>._is_required` and `._options`.
- Only the effective Stripe EPS bank setting is projected: other bank configuration does not currently affect the constructor. Absent, explicit-empty and populated inputs have distinct settings identities. The retained rules include outer path, inner path, display-name evidence and complete field type, without prefill. Maps are sorted by source identity. Only the exact EPS options source is normalized, and it must match the effective bank set; stale values or duplicates refuse. Other `UserBankOptions` vectors remain ordered.
- Input manifests bind settings, full retained rules, mappings, catalog/SDK identity, exclusions and build identity. Cargo embeds actual normalized feature keys separately for router and payment_methods. Release identity explicitly distinguishes offline artifacts from image identities and includes a source-tree hash so a dirty tree cannot be represented by a Git SHA alone.
- Settings provenance explicitly says `effective_values_only`. Loading settings does not prove immutable file/ConfigMap revision, environment-origin history, image provenance or all-pod attestation. Those later CLI/release gates remain required; this step neither exports nor hashes the entire application settings/file containing secrets.
- `required_fields_sync` is opt-in in both packages, adds only optional SHA-256/TOML dependencies to payment_methods, and rejects missing-v1 or any-v2 combinations explicitly. Ordinary defaults are unchanged. Foundation reports can represent only disabled hosted apply.

## Assumptions and unresolved decisions

The current constructor's bank-input dependency and exact EPS provenance define normalization contract v1. A new constructor setting or transition to an ordered source requires updating this projection and its dependency/invariant tests together. Current full-rule hashing also binds actual compiled rules, not just the settings projection.

No reviewed `config/required_fields_sync_policy.toml` is invented. Synthetic parser tests demonstrate the schema; Step 1's [decision register](../sdk-v1/decisions.md) still controls aliases, country catalog/AC–TA, malformed comma countries, EPS/dummy scope, Card/Debit, connector order and descriptor replacement. Unapproved mappings cannot become implicit defaults. Empty-select usability and exact semantic mapping belong to Step 4; no renderer or source-data correction was added.

The existing published plan's client-version references remain historical (actual lock is 0.119.0); Step 3 adds no client dependency or server capability claim. OrbStack availability was established in the intervening read-only follow-up, but exact-server provenance/auth/fault/limit/exclusion certification is still pending. No container or hosted target was accessed for Step 3.

## Review handoff

Review group: new renderer contracts/tests, router input/report boundaries, feature/build wiring, lock edges, v2 recipe exclusion and this evidence/checkpoint update. Existing source business rules, RequiredFields definitions, original roadmap/plan, seed, W/U and prior harness files remain preserved. No staging, commits, branch changes, uploader or deployment.

Changed files in H:

- Root wiring: [Cargo.lock](../../../Cargo.lock), [justfile](../../../justfile).
- payment_methods wiring: [Cargo.toml](../../../crates/payment_methods/Cargo.toml), [build.rs](../../../crates/payment_methods/build.rs), [lib.rs](../../../crates/payment_methods/src/lib.rs), [configs.rs](../../../crates/payment_methods/src/configs.rs).
- Pure inputs: [required_fields_renderer.rs](../../../crates/payment_methods/src/configs/required_fields_renderer.rs), [artifacts.rs](../../../crates/payment_methods/src/configs/required_fields_renderer/artifacts.rs).
- Foundation tests: [artifacts_tests.rs](../../../crates/payment_methods/src/configs/required_fields_renderer/artifacts_tests.rs), [tests.rs](../../../crates/payment_methods/src/configs/required_fields_renderer/tests.rs).
- Router wiring: [Cargo.toml](../../../crates/router/Cargo.toml), [build.rs](../../../crates/router/build.rs), [lib.rs](../../../crates/router/src/lib.rs), [core.rs](../../../crates/router/src/core.rs).
- Settings/report boundary: [settings.rs](../../../crates/router/src/configs/settings.rs), [required_fields_sync.rs](../../../crates/router/src/core/required_fields_sync.rs), [tests.rs](../../../crates/router/src/core/required_fields_sync/tests.rs).
- Evidence: [this file](README.md), [checks](checks.md), [progress](../README.md), [questions](../questions.md), [playbook checkpoint](../../superposition_required_fields_sync_implementation_playbook.md).

Suggested H commit: `feat: define required-fields sync inputs and artifacts`.

Next eligible implementation after review is **Step 4** pure rendering, with unapproved mappings remaining refused until their decisions are resolved. Router settings/release/MSRV gates stay pending for their dependent acceptance; they do not prevent unrelated pure renderer work. Step 2 certification can resume independently with its verified disposable dependencies; Step 5 additionally requires its relevant read contracts. No later step was begun.
