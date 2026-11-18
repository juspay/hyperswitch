# Required-fields API contract companion

Opt-in Step 2 test harness. It does not add router features, a sync endpoint, a mutating daemon or an uploader. [H's evidence register](../../docs/required_fields_sync_contracts/server-v1/README.md) separates executed isolated checks from **pending server certification**.

The standalone crate uses the actual generated `superposition_sdk = 0.119.0`, clap and error-stack. Its registry dependency identities/checksums match H's Cargo.lock, checked by `check_lock.py`; it does not build H's release feature graph. A custom bounded local HTTP connector uses locked reqwest with redirects, proxies and underlying retries disabled. SDK mutations use one attempt, reads two. This transport is the test companion's transport, not certification of the existing provider's default HTTPS client.

## Local checks

Inspect commands before running. Set `CARGO_TARGET_DIR` to a newly allocated, registered disposable filesystem directory. Rust builds only this crate; no test below contacts any API server, Docker daemon or hosted environment. In-memory connectors run the actual generated SDK. Python tests use fake Docker metadata solely to test refusals and cleanup selection.

From H:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/required_fields_contracts/check_lock.py
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/required_fields_contracts -p 'test_*.py' -v
RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= cargo test --offline --locked --manifest-path scripts/required_fields_contracts/Cargo.toml --test isolated
RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= cargo check --offline --locked --manifest-path scripts/required_fields_contracts/Cargo.toml
RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= cargo clippy --offline --locked --manifest-path scripts/required_fields_contracts/Cargo.toml --all-targets -- -D warnings
cargo +nightly fmt --manifest-path scripts/required_fields_contracts/Cargo.toml -- --check
```

No production environment variables, credential files, shared seed or arbitrary operation JSON are accepted. `--help` is read-only; invoking the probe without explicit registration/report arguments fails before client construction.

## Real disposable server run — not executed in this step

Requires a local Unix Docker socket, already cached immutable Superposition-demo and Redis images, and exact server source/version provenance. Never use `docker compose up`, the existing dev workspace, root seeder, production credentials or a shared service. No pulls or daemon startup are performed by this harness. There is no force/reuse/live mode.

`fixture.py run` creates a fresh internally networked Redis/server pair with unique ownership labels, no host mounts, and one dynamic port published to literal `127.0.0.1`. Before the first API write and again before every request, it verifies image digest and ID, OCI source/version labels, exact container/network IDs, running state, ownership, network peers, mounts and exposed ports. Missing provenance refuses rather than trusting a tag. The probe then requires the unique workspace to be absent, creates it with a synthetic admin, verifies org/name/admin and checks empty initial data. Existing, denied or ambiguous workspace reads cannot lead to writes.

After building the binary in the registered build directory, this is the **command shape**, with placeholders requiring actual inspected values:

```text
python3 scripts/required_fields_contracts/fixture.py run
  --docker-host unix:///ABSOLUTE/LOCAL/docker.sock
  --server-image ghcr.io/juspay/superposition-demo@sha256:EXACT_DIGEST
  --server-version 0.115.5
  --source-revision EXACT_40_HEX_COMMIT
  --redis-image docker.io/library/redis@sha256:EXACT_DIGEST
  --probe ABSOLUTE_OWNED_BUILD_PATH/debug/required-fields-api-contracts
```

Only the three candidate versions in H's checked-in configuration are currently accepted as fixture candidates. They are not declared supported. Per-image bootstrap/API incompatibilities, absent OCI labels, implicit image volumes or startup requirements must be investigated as refusals; do not relax guards or alter the original image to obtain a pass.

This fixture profile accepts only the root API base path; prefixed deployments remain unsupported here. Attached validation/computation functions require additional exact-build synthetic fixtures; preserving the returned function fields on a default conflict is not execution proof for arbitrary functions.

The launcher always attempts cleanup of only its nonce-labeled containers (including a lost create response) and its empty owned network. It records cleanup outcome and retains the owned filesystem directory with registration/report for review. It performs no global reset/prune or API workspace deletion. Cleanup failure is an error with retained ownership information. Disposal of the whole owned server ends test requests, but does not prove production timeout cancellation or writer fencing.

## Cases, reports and limits

Nine implemented server phases cover fresh target identity, prerequisite schemas/default conflicts, complete pagination, lossy prefix reads, merge versus full replacement/removal, delete-missing, bulk rollback, versioned ordering, option-array hash collisions and active experiment interaction. A failed phase stops subsequent writes; later cases remain pending. The ordered-array phase reports collision as unsupported rather than sorting values.

The remaining capability IDs are explicit pending gates: authenticated full-scope reads, actual server action/body limits, real webhook/cache post-commit injection, real dropped responses, server/socket dispatch instrumentation and old-request exclusion. Their isolated negative models cannot fill those server gates. Build-specific instrumentation and approved fixture identities are needed before implementing/accepting the corresponding real failure profiles; no generic successful fault stub is supplied.

Local caps are one MiB per request/response, 16 actions in one bulk, 32 pages, 512 dispatches, 8 MiB per report and 16 KiB registration/response headers. Local caps are not measured server maxima. Proof, connect, request and operation timeouts are bounded. Reports contain exact client locks, requested server/image/source/container/workspace identity, actual wire exchanges and every declared case once. They omit bearer headers and contain synthetic data only. Missing metadata is refused even where the SDK defaults it. The report is local test evidence, not a signed deployment attestation.

Until the pending capabilities are certified, the real-run command returns nonzero even if every implemented server phase passes. `hosted_apply_enabled` is always false. This is deliberate incomplete certification, not a successful skip. See [case requirements](../../docs/required_fields_sync_contracts/server-v1/cases.json) and [candidate matrix](../../docs/required_fields_sync_contracts/server-v1/capabilities.json).
