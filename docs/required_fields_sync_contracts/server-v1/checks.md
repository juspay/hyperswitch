# Step 2 checks and limits

2026-11-18. Commands were inspected for effects before execution. No hosted credentials, hosted requests, production secrets, uploader, Docker pull/start/run, API mutation or deployment occurred. The public source URLs linked in [README](README.md) were read; this is source inspection, not hosted application access or image certification.

## Starting state and disposable build target

H: `main`, `82fe5dfd3ac527fd4a1e03dc01fbb5be13e3198b`; W: `main`, `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39`; U detached at W's exact gitlink `1af3e5dabf9d5a17dd864ac7312151ebac4367c6`. No tracked/staged differences existed. Prior H design/register and H/W Step 1 additions were already untracked. Heads/branches/index hashes and all pre-existing untracked-file hashes were captured before edits.

The new disposable **filesystem build target**, not an API workspace, was registered before compilation:

- Owner: `required-fields-step2-57b5cc41038b449bb7dbdb8c7e8ff884`.
- Temporary root suffix: `required-fields-step2-5h9gixkf`.
- Marker: `fixture-target.json`, `kind=disposable-filesystem-build`, exact root/cleanup scope, `hosted_target=null`.
- Uses: Cargo target/binaries, compiler/test logs and synthetic in-memory tests only. No socket/server resources were created by the tests.

## Executed checks

`PROBE` means `scripts/required_fields_contracts/Cargo.toml`; `BUILD` means the registered root above. All Cargo commands set `CARGO_TARGET_DIR=BUILD/target`, `RUSTC_WRAPPER=` and `RUSTC_WORKSPACE_WRAPPER=`. No entire H workspace/all-features build or file-writing required-fields dump was run.

| Command / inspection | Result and effects |
|---|---|
| Git status/HEAD/branch/index/gitlink and scoped source/instruction reads | Captured current state; preserved prior files; no Git mutation. Existing SDK/client/settings/error/CLI conventions inspected. |
| `docker context ls --format '{{.Name}} {{.DockerEndpoint}}'` | Two local Unix socket entries only: default and OrbStack. Read-only local context metadata. |
| `docker --host unix:///Users/deepanshu.bansal/.orbstack/run/docker.sock image ls --digests ...` and `ps ...`, each with a subprocess deadline | Both failed: socket absent. No image list or running-server identity was obtained. No pull or start attempted. |
| Local socket existence checks; `podman system connection list ...`; `podman machine list --format '{{.Name}} {{.Running}}'` | `/var/run/docker.sock` and OrbStack socket absent; Podman connection/machine lists empty. Exact server images remain uninspected, not assumed absent from every possible cache. |
| Copy H Cargo.lock into the **new probe lock only**, then `cargo check --offline --manifest-path PROBE` | Passed; pruned/resolved a standalone subset without modifying H Cargo.lock or downloading packages. After adding Tokio process support, the identity checker detected two newly selected transitive versions; reseeding from H's lock fixed that drift. Final dependency graph matches H. |
| `PYTHONDONTWRITEBYTECODE=1 python3 scripts/required_fields_contracts/check_lock.py` | **Passed:** all 167 registry package name/version/source/checksum identities match H; exact SDK 0.119.0 checked in both locks. Lock hashes recorded in [provenance](provenance.json). |
| `cargo test --offline --locked --manifest-path PROBE --test isolated` | **18 passed; zero failed/ignored.** Actual generated SDK through an in-memory connector; no API socket/network or server certification. Includes CLI opt-in/ambient-target refusal. [Output](reports/rust-tests.txt). |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/required_fields_contracts -p 'test_*.py' -v` | **10 passed.** Fake Docker metadata tests strict target proof, ownership/network/mount/image refusal and cleanup only of owned resources, including lost-create-response lookup. No Docker calls in these tests. [Output](reports/python-tests.txt). |
| `cargo check --offline --locked --manifest-path PROBE` | **Passed**, including the opt-in binary. [Output](reports/cargo-check.txt). |
| `cargo clippy --offline --locked --manifest-path PROBE --all-targets -- -D warnings` | **Passed** after fixing two new-test style issues (header type alias and borrowed slice). [Output](reports/clippy.txt). |
| `cargo +nightly fmt --manifest-path PROBE -- --check` | **Passed.** Formatting only applied to this new standalone crate before the final checks. |

Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`; Python: `3.14.6`. Declared MSRV is H's 1.94.1, but that minimum compiler/release-feature graph was not executed. The companion's direct SDK transport uses a bounded local reqwest connector; the existing router/provider's default HTTPS client is outside this test result.

An initial config-metadata test failed (15 passed, one failed) because it assumed missing Last-Modified made the SDK reject. Source/actual execution show permissive defaulting. The corrected test preserves the actual SDK result and separately asserts harness refusal. Initial Clippy issues and transitive-lock drift were corrected, not suppressed. Final outputs are successful reruns, not relabeled failed runs.

## Inspected but not executed

- `fixture.py run` / actual probe API suite: unavailable local runtime and exact cached images/provenance. All 16 server capability IDs remain pending for all three candidate builds. No disposable API registration or image digest is invented.
- Real pagination/preservation/prerequisites/bulk rollback/exports/experiments: executable driver assertions exist, but no server result.
- Real authenticated full-scope identities, gateway/server boundary limits, webhook/cache failure, committed-response drop, socket dispatch/timeout instrumentation and old-request exclusion: missing exact-build fixture facilities. Their seven real profiles remain pending, not implemented as successful stubs. The isolated models certify only harness/SDK behavior.
- Actual journal crash/immutable-before-image checks, writer exclusion, restoration and V1–V4: dependent later-step/external gates, not supplied by a test companion or a local container deletion.
- Hosted read-only inspection, seeding, apply, deployments, uploader and remote Git actions: not authorized or attempted.

The retained [isolated report](reports/isolated.json) is labeled `isolated_locked_client`, with `server_certified=false` and `hosted_apply_enabled=false`. The [candidate matrix](capabilities.json) has only pending server results. Source/lock/artifact hashes are in [provenance](provenance.json); an edited artifact invalidates that evidence.

## Final review and cleanup

**Final verification passed:** all source/SDK/lock hashes and the isolated report binding match; the 16 case IDs agree across Rust, case requirements, the isolated report and all three pending candidate rows. JSON parsing, relative Markdown links/fences, `git diff --check` and explicit untracked-file `git diff --no-index --check /dev/null <file>` checks passed for all 27 scoped files. Final trailing blank lines in the new manifest/ignore file were removed. The late-commit test uses an explicit release signal after the observed timeout, avoiding a timing race; the final 18-test and Clippy reruns pass.

H/W/U heads, branches, indexes and tracked/staged contents match the pre-step snapshot. All pre-existing untracked contents are unchanged except the three intended H progress/question/checkpoint updates. W/U and all Step 1 artifacts, original plan, roadmap, seed and H Cargo.lock remain preserved. No staging, commit, push or branch/submodule update occurred.

**Cleanup executed:** final sanitized logs/report/artifact hashes were saved first. No matching build/test process remained. The exact owner, kind, root/cleanup scope, absent hosted target and absence of `.git`/root symlink were verified, then only the registered `required-fields-step2-5h9gixkf` filesystem root was removed. No API/container resources were created, so none were cleaned up. Original repositories and user caches were not cleanup targets.

Files for review:

- New `scripts/required_fields_contracts/`: Cargo.toml/Cargo.lock, `.gitignore`, README, `check_lock.py`, `fixture.py`, `test_fixture.py`, `src/{api,contract,lib,main,suite,transport}.rs`, and `tests/isolated.rs`.
- New `docs/required_fields_sync_contracts/server-v1/`: README, checks, cases/capabilities/provenance JSON and five report/log files.
- Updated H `docs/required_fields_sync_contracts/{README,questions}.md` and the implementation playbook's current checkpoint only. The original roadmap and technical plan were not changed.

Suggested commit: `test: add opt-in Superposition API contract harness`. This describes delivered work without claiming that unavailable servers were certified. Keep this H Step 2 review group separate from pre-existing H/W Step 1 additions. Stop for review; no later step began.
