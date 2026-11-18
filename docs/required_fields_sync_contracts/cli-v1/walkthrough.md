# Synthetic local review walkthrough

This walkthrough does not run an uploader or start/contact a service. The CLI's production
registries are empty. Successful planning below is a separately identified in-memory unit
fixture, not a registered real server. The binary harness requires macOS `sandbox-exec`
and verifies network denial before invoking any CLI case; on another platform supply an
independently reviewed equivalent isolation mechanism rather than dropping that check.

## Prepare the disposable files

The executed run used the owned root recorded in [checks.md](checks.md). To reproduce,
register a fresh directory before writing fixtures. Do not reuse a directory from another
task or infer permission from its name:

The executed run's build caches were cleaned after validation; its final binary remains
under that fixture's `review-binary/router` for review. The following build commands are
for a fresh reproduction, not a claim that its old `router-target` directory still exists.

```python
import json, pathlib, tempfile, uuid
root = pathlib.Path(tempfile.mkdtemp(prefix="required-fields-step7-")).resolve()
owner = {
    "owner": str(uuid.uuid4()),
    "purpose": "Step 7 offline CLI checks",
    "root": str(root),
    "cleanup_scope": str(root),
    "network": "offline; synthetic filesystem/in-memory fixtures only",
}
(root / "owner.json").write_text(json.dumps(owner, indent=2) + "\n")
print(root)
```

Set `RF_CLI_FIXTURE_ROOT` to that exact result and `CARGO_TARGET_DIR` to its `router-target`
child. Use a new `CARGO_HOME` child containing only copies of already-cached
`registry/{src,index,cache}` and `git` dependencies, without ambient config/credentials.
Do not download missing dependencies or reuse a hosted settings file as part of this
walkthrough. The owned cache isolates core's known cbindgen relative-output side effect.
Set jobs to 2 (or lower), incremental to 0, dev/test debug to 0 and clear compiler wrappers.
Use offline/locked commands from H. No service availability is needed.

## Build, export and check

These commands write only owned build/fixture artifacts. The exporter and command tests
verify the marker above and create new files. Reusing exporter filenames is an error.

```sh
cargo build --offline --locked -p router --features required_fields_sync --bin router
cargo test --offline --locked -p payment_methods --features v1,required_fields_sync,storage_impl/default,hyperswitch_interfaces/v1 --lib export_router_cli_profiles -- --ignored
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib command_tests -- --ignored
python3 docs/required_fields_sync_contracts/cli-v1/test_cli.py --root "$RF_CLI_FIXTURE_ROOT" --router "$CARGO_TARGET_DIR/debug/router"
```

The first command is the real existing router binary. The second exports:

- `seed-profile.json`: explicit EPS/Stripe bank input and synthetic policy/catalog, with
  every compiled source explicitly excluded for this plumbing test only;
- `effective-profile.json`: the same relevant input through the real Settings projection,
  bound to the expected hash; `unavailable-services.toml` has invalid unrelated DB settings,
  unavailable Redis and a synthetic redaction sentinel;
- `blocked-profile.json`: the same compiled source without the exclusion list, which
  must fail for unresolved mappings/options.

The third command exercises successful provisional planning through the actual command
and planner with an in-memory complete inventory and private synthetic registration. It
does not install a production registry entry or construct an HTTP client. Unit fixture
`plan.json` supplies strictly typed synthetic configuration for the binary's refusal tests.

The final command invokes render/plan with explicit profile/output/config/target arguments,
denies network access and checks stdout, stderr and artifacts. The result is
`binary-cli/report.json`. Read that file together with [evidence.json](evidence.json).
Do not label a command passed unless it actually completed successfully in the new run.

After the harness passes, these are the corresponding direct command shapes. The first
creates a separate local bundle; the second must exit 1 with `unsupported_server_contract`
and create no plan artifact. Both retain the network-denial boundary:

```sh
sandbox-exec -p '(version 1)(allow default)(deny network*)' "$CARGO_TARGET_DIR/debug/router" required-fields render --input-profile "$RF_CLI_FIXTURE_ROOT/seed-profile.json" --output "$RF_CLI_FIXTURE_ROOT/manual-render.json"
sandbox-exec -p '(version 1)(allow default)(deny network*)' "$CARGO_TARGET_DIR/debug/router" required-fields plan --input-profile "$RF_CLI_FIXTURE_ROOT/seed-profile.json" --plan-config "$RF_CLI_FIXTURE_ROOT/binary-cli/plan.json" --target synthetic-inventory --output "$RF_CLI_FIXTURE_ROOT/manual-plan.json"
```

## Expected review observations

Named seed output must remain identical under an invalid ambient bank override. Effective
settings must succeed with the same relevant hash despite invalid unrelated service
settings, and refuse a changed relevant bank hash or absent explicit file. Output collision
must preserve the old bytes. Input symlinks, unknown fields, image claims, mutation commands
and target/token/header/force overrides must refuse. Renderer failure must stay nonzero.
Absent, explicitly empty and populated seed bank configurations must retain distinct bundle
hashes, and an unknown bank name must refuse. Empty effective-settings CSV remains rejected
by the existing Settings contract; the walkthrough does not change that parser.

A binary `plan` for the synthetic target must return `unsupported_server_contract` before
reading the deliberately absent credential. Omitted mode returns `disabled`; target
mismatch returns `target_mismatch`. It must never become an empty successful plan.
The in-memory successful plan reports `planned_provisional`, no verified plan hash,
SDK `not_executed`, and zero mutation/lease calls. It exposes no raw live row or sentinel.

Only after verifying the ownership marker's exact root and cleanup scope may the owned
build caches/fixtures be removed. No repository file, another task's root or shared Cargo
cache belongs to this cleanup. The executed run's cleanup status is in evidence.json.

Stop at the review checkpoint. Do not run seed/apply/status/resume/restore, contact a hosted
target, or proceed to Step 8 from this walkthrough.
