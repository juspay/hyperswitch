# Required-fields inventory local checks

This standalone crate imports the actual feature-gated router inventory files and Step 3
artifact types by path, plus the later actual planner, CLI argument/orchestration, journal, coordination and disabled apply
modules. It has no substitute production implementation, AppState, credentials, server target
or hosted connection. Its 168 registry dependency identities/checksums match H's `Cargo.lock`.
Step 14 adds a direct edge to the already-locked `toml_edit 0.22.26` in this crate and the opt-in router; no package version/checksum changes.

Inspect commands before running. Register a new disposable filesystem directory with an
owner/purpose marker and use it as `CARGO_TARGET_DIR`; verify ownership before cleanup.
Set `CARGO_NET_OFFLINE=true`, clear `RUSTC_WRAPPER`/`RUSTC_WORKSPACE_WRAPPER`, and use two
build jobs with incremental/debug output disabled. Then run from H:

```sh
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib
cargo check --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib
cargo clippy --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib --tests -- -D warnings
```

The original Step 5 suite had 18 foundation tests, 23 inventory tests, five adapter tests
and one request comparison covering seven actual SDK 0.119.0 builders. Later planner, CLI
and journal cases extend that suite; per-step evidence records the executed counts. Inventory
responses are independently written synthetic raw fixtures. The SDK connector captures requests in
memory; its placeholder responses are used only to exercise request serialization, never
as server evidence. The production HTTP adapter refuses synthetic certificates before
dispatch. Ordinary tests open no listener and use no DNS/network connection; the separately selected Step 9 test below starts only its verified owned loopback fixture.

These checks do not establish server pagination, complete permission scope, real transport,
or router feature/release compatibility. See [Step 5 evidence](../../docs/required_fields_sync_contracts/inventory-v1/README.md)
and its [executed checks](../../docs/required_fields_sync_contracts/inventory-v1/checks.md).

Step 8 imports the real journal/snapshot code and runs synthetic failure cases in memory.
The filesystem/independent-process test is opt-in and requires an exact canonical
`RF_JOURNAL_FIXTURE_ROOT` containing `owner.json` with owner UUID, purpose
`Step 8 isolated journal checks`, `root` and equal `cleanup_scope`. It allocates and removes
only a fresh owned child. Run the parent only; it selects its own ignored child helper:

```sh
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib journal::tests::fixture::independent_process_recovery_and_filesystem_tamper -- --exact --ignored
```

The fixture does not certify hosted durability, encryption, lifecycle or separate-role access.
The production registry is empty. See [Step 8 checks](../../docs/required_fields_sync_contracts/journal-v1/checks.md).

Step 9 imports actual authority, coordination and Kubernetes wire modules. The existing
H-locked `time 0.3.41` formatting/parsing features encode/validate Lease timestamps; the
companion lock adds two dependency edges without changing any external identity/checksum.
The opt-in transport test requires `RF_COORDINATION_FIXTURE_ROOT` with exact canonical
root/cleanup scope, owner UUID and purpose `Step 9 isolated coordination checks`. It starts
`coordination_fixture.py` on a newly allocated literal loopback port only after verifying
the marker and synthetic object. The adapter verifies the fixture header and UID before PUT.

```sh
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib kubernetes_coordination::tests::owned_loopback_adapter_cas_conflict_unknown_outcome_and_restart -- --exact --ignored
```

Use an OS network policy allowing only loopback inbound/bind and remote loopback outbound;
verify it before running. The test covers CAS conflict, denied reads, post-commit lost response,
no retry and unresolved state across restart, then stops its children and removes only its
owned child directory. This Python wire fixture is not Kubernetes and certifies no cluster,
RBAC, writer exclusion, rollout or durability control. Production execution/control factories
remain refusing. See [Step 9 evidence](../../docs/required_fields_sync_contracts/coordination-v1/README.md).


Step 10 imports the actual disabled apply/compiler/observation modules. Twelve apply tests
exercise the accepted planner, journal and execution session together; independently applied
synthetic payloads cover faults and all outcomes. Two in-memory tests invoke the locked SDK's
mutation serializers and one-attempt retry middleware. A separate legacy journal-format test
extends recovery coverage. These are local tests, not server or V1–V4 certification.

```sh
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib apply_tests
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib mutation_wire
```

They open no socket. The actual administration transport remains unavailable through its
closed registration factory. Journal event version 2 records resolutions/reports; new readers
retain v1 compatibility. See [Step 10 checks](../../docs/required_fields_sync_contracts/apply-v1/checks.md)
for the independent-process regression and pending exact-server/storage/control gates.


Step 11 imports the actual verification, raw GET, companion-report and pinned-provider modules. Optional router dependencies and this companion reuse H-locked provider/types 0.119.0; the reconciled companion lock matches H dependency identities/checksums. Ordinary tests are network-free.

```sh
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib plan::tests::verification_tests -- --ignored --nocapture
```

The second command requires an explicitly registered `RF_VERIFICATION_FIXTURE_ROOT` with owner, canonical root/cleanup scope, null hosted target and purpose `Step 11 isolated verification checks`. Provider tests create only owned children and remove them. The independent browser reader requires that root's `browser-execution.json` pointing to an actually executed W receipt/input/registration. Missing files fail. The HTTP ignored test also requires OS policy permitting only owned loopback; it records each literal listener target before its first read and cleans only its listeners/child. Other ignored tests retain their own registrations; this root does not authorize them.

See [Step 11 checks](../../docs/required_fields_sync_contracts/verification-v1/checks.md). Local reports cannot enable hosted apply; deployed pod, backup, image/mount and supported-reader certification remain pending.


Step 12 imports the actual recovery, restricted evidence decoder and three-way inverse builder.
The ordinary suite includes synthetic observation/replanning, preservation, exclusion, retention,
recreation, approval and apply/verification failures. No recovery request is replayed by this harness.

```sh
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib recovery_tests
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib plan::tests::apply_tests::recovery_tests::fixture::independent_recovery_after_executor_disappears -- --exact --ignored --nocapture
```

The second command requires `RF_RECOVERY_FIXTURE_ROOT` explicitly registered with an owner UUID,
canonical root equal to cleanup_scope, null hosted_target and purpose `Step 12 isolated recovery checks`.
Run under an OS network-denied policy. It allocates a private synthetic child directory, persists
fixture journal/coordinator/current-state evidence, drops the original executor and starts only a
bounded read-only child with a cleared environment. It removes only that verified owned child.
Missing registration fails; no hosted storage or real retention/access capability is supplied.
Snapshot v2 recovery references are mandatory and independently checked. See
[Step 12 evidence](../../docs/required_fields_sync_contracts/recovery-v1/README.md).


Step 13 imports the actual operation/attestation/shared-executor modules. Ordinary tests exercise
synthetic command-to-engine outcomes, identity refusals, read-only status/inverse planning,
resume/restore gates, executable hashing and final-report failure handling. The opt-in
`operation_file_contract_and_exported_binary_inputs` test requires `RF_OPERATION_FIXTURE_ROOT`
with purpose `Step 13 isolated command checks`, matching absolute root/cleanup scope and null
hosted target. It writes only a new `operation-inputs` child and refuses existing outputs.
[Step 13 checks](../../docs/required_fields_sync_contracts/operations-v1/checks.md) distinguish
actual router build/binary evidence from these synthetic adapters. No production certificate is
created, and no dependency version or lockfile change is required.


Step 14 imports the actual pure seed editor and early command adapter. Handwritten fixtures
check exact managed values, ordered options, manual TOML preservation, explicit retirement,
metadata/ownership refusals and deterministic check equality. The separate ignored
`seed_export_independent_semantic_inputs` test requires `RF_SEED_FIXTURE_ROOT` with owner UUID,
purpose `Step 14 isolated seed and CI checks`, canonical matching root/cleanup scope and null
hosted target. It creates only a new synthetic good/wrong input directory; the actual pinned JS
oracle must reject the wrong seed despite generator equality. See [Step 14 checks](../../docs/required_fields_sync_contracts/seed-v1/checks.md).
