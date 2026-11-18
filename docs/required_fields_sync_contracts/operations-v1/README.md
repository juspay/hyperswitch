# Step 13 — command execution and release observations

Scope: H only, local command implementation; **hosted apply/resume/restore remain disabled**.
Starting H revision: `68c938724127fe507cdb5b2c365c6f15af1aec1d`, branch
`feat/superposition-required-fields-foundations`. Existing original plan/roadmap files remain
untracked and unchanged. No remote refresh, branch change, staging, commit or deployment is part
of this step. The previously observed remote formatter commit is not silently merged into this
checkout. Exact checks and preservation evidence are in [checks.md](checks.md) and [evidence.json](evidence.json).

## Source evidence and implemented scope

Step 7 already dispatches before serving configuration validation, secret conversion, AppState,
DB/Redis/queues and providers. Steps 8–12 already implement journal, coordinator, apply, V1–V4
and recovery interfaces. This step reuses those implementations. It adds strict `status`, `apply`,
`resume`, and mutually exclusive `restore --plan` / `restore --plan-hash` argument shapes; `seed`
remains Step 14. Ordinary router and scheduler argument contracts remain distinct.

`operation.rs` parses bounded duplicate-free configuration, enforces disabled/plan/apply mode,
target/instance and intent/run binding, reserves a new private output before dispatch and emits
sanitized final reports. Configuration cannot supply endpoint/header/token overrides or construct
certification capabilities. Status needs no profile and can inspect recovery evidence while mode
is disabled. Applying restore and resume require explicit apply mode, a newer recovery intent and
an exact accepted plan hash; `--plan` cannot be combined with `--plan-hash`.

`operation_executor.rs` connects sealed rendered inputs, complete inventory, whole-plan/browser
acceptance, authenticated current intent, owner-checked coordination, immutable journal,
retained recovery image and the existing apply verifier. All three mutating operations use the
same engine. Renewal covers planning and execution with one loop. Recovery reads/observes the
original evidence, obtains old executor/request exclusion, replans against current data, records
the recovery authorization and preserves the baseline. Status has a separate narrow
`JournalReader` function. Inverse planning uses a narrow read-only function: no coordinator/control/image/verifier/writer capability is required, and it neither acquires a lease nor appends journal events. Its requested intent is provisional until execution reauthenticates current authority.

The actual production CLI refuses at its closed certification/provisioning boundary. There is
**no usable production storage/safety/image/collector provisioner** to instantiate the shared
executor. Local synthetic tests exercise that shared executor; they do not demonstrate a usable
real-target CLI or certify any server/platform. Adding a registry entry alone must not enable
execution: exact adapters, independently verified image/config provenance, collector delivery,
retention and all previous gates remain required. No placeholder reports success.

## Release observations and identity

With the opt-in build, normal startup emits a nonsecret observation through the existing logger,
using the already-loaded `Settings.required_fields` and bank inputs. It contains hashes and
actual Cargo feature closures, never field values, prefill, tokens, endpoints or file paths.
The optional strict `[required_fields_sync]` block defaults to `mode = "disabled"` and no
`attestation_profile`. Serving never runs synchronization in any mode.

An optional profile must describe effective settings with the exact observed settings hash.
Its policy/catalog/exclusions render against the pod's actual loaded inputs. Missing/refused
profiles explicitly leave desired/bundle observations unavailable; they do not break serving
or claim verification. Image provenance, rollout, synchronization and verification statuses are
separate. A log collector must attach authenticated pod identity; the process does not assert
that a hostname or version string proves workload membership or image digest.

The executor additionally binds a bounded hash of its actual executable and exact compiled
features to a sealed platform-issued image/profile capability. It checks source/build, settings,
policy/catalog, input manifest, release and accepted bundle/plan against the authenticated intent.
The real image/provenance capability registry is empty. The synthetic identity fixture is not a
release attestation, OCI provenance receipt or old-reader compatibility certificate.

## Reports and metrics

Apply outcomes preserve the existing contract: `NoChangeVerified`/`AppliedVerified` → 0;
validation refusal → 2; authority/conflict refusal → 3; `PartialApplied` → 4;
`CommitUnknown` → 5; no-change/applied-unverified → 6; `RejectedNoChange` → 7.
Status returns 5 for durable pending requests and 3 while recovery protection remains; it does
not declare a run synchronized from an empty diff. Restore planning returns a provisional plan,
not an accepted apply or a V1–V4 receipt. Refusals after lease acquisition report that acquisition;
they do not falsely claim zero coordination writes.

Reports include bound run/intent/image/input/bundle/plan hashes, actual request outcomes/status
and durations, action counts and recovery/verification references. Only enum/static labels enter
metric dimensions; run IDs, hashes and tenant/connector detail are excluded. Each verification
level remains pending unless the existing all-level receipt and verified outcome are present.
Synthetic execution reports explicitly carry `synthetic_fixture` scope.

The final private file is create-new, synced, and cannot replace an existing file or symlink.
Delivery failure preserves actual mutation facts and any exit 3–7; an otherwise successful exit
becomes 6. The engine's restricted journal reports remain the durable execution evidence.
The final JSON event is suitable for a one-shot collector, but stdout/file persistence is **not**
proof of platform ingestion, retention, failed-Job alerts or stale-backup alert delivery. T34/P8
collector integration remains pending and does not follow from this step.

## Choices and discrepancies

- Proposed operational JSON is passed by `--command-config`; there are no ambient writer defaults.
  Mounted credential/endpoint construction belongs to certified adapter provisioning, which is
  unavailable. A config file, success boolean or force switch cannot replace it.
- The source has `configs/cli.rs` and `command_runtime.rs`, rather than a command enum inside
  `settings.rs`. The shared Settings loader/secret transformer and early router dispatch are reused.
- The old command error exit 1 is refined to the plan's validation/authority exits 2/3. Serving-only
  Clap behavior remains unchanged. No source business data, SDK, seed or original roadmap is changed.
- Desired-state observations are optional facts, not self-authenticated image attestations.
  Platform pod/image/immutable-config checks remain indispensable and unavailable here.
- Local output protection is not a remote retention/access certificate. Production provisioning
  and collector integration are pending; there is no hosted execution path to bypass them.

After review, Step 14 local seed/CI work is next only on a new request. Exact server, image,
platform, storage, candidate-browser and V1–V4 certification can proceed independently with
explicitly authorized disposable targets/artifacts. No later step was begun.

Suggested H commit: `feat: wire required-fields execution and release attestations`.
