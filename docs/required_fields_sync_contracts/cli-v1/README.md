# Step 7: offline rendering and provisional read-only planning

Local implementation, 18 November 2026. This is the early renderer/planner review
checkpoint, not target adoption. Read/projection registries remain empty. A production
`plan` invocation refuses before credentials or HTTP until independently certified
registration exists. No mutating commands, status placeholder, journal or lease were added.

## Source evidence and preserved work

H started on `feat/superposition-required-fields-foundations` at
`6a2e1869643b4008fe6e46cdd4c3b1873712f0c8`. Steps 4/5 were already committed;
Step 6's planner, tests and evidence were present as uncommitted work. They are reused,
not reimplemented. W remains at `939f45786aee1c3b14e3f0b134bb42cb2cf2dd39` with its
existing untracked harness. Its U gitlink/checkout remains
`1af3e5dabf9d5a17dd864ac7312151ebac4367c6`. No W/U changes were needed.

Read Step 7, plan §6.9/P6/T25/T33, repository contribution guidance and PR template.
No applicable AGENTS.md was found in the inspected H ancestors/scoped directories.
The initial file/index/revision inventory was captured before editing. Final preservation
and command results are in [evidence.json](evidence.json) and [checks.md](checks.md).

Current source boundaries:

- `configs/cli.rs` owns the argument types; `settings::CmdLineConf` remains a re-export
  of the original serving-only parser, including for the scheduler. The router wrapper
  adds only `required-fields render` and `required-fields plan` under the opt-in feature.
- `bin/router.rs` dispatches before rustls serving-provider installation, full Settings
  deserialization/validation, logging, decryption, AppState, DB/Redis and server startup.
  The Tokio runtime still exists for the async read-only command. The ordinary serving
  body and Result termination behavior are retained.
- `Settings::config_from_sources` is the existing source-precedence builder, extracted
  without changing normal behavior. The new explicit-file projection deserializes only
  `bank_config`, calls existing `RequiredFields::new`, then captures `EffectiveInputs`.
  The earlier `load_effective_inputs` helper still deserializes full Settings and is
  deliberately not the command path.
- `command.rs` handles bounded strict input, create-new private output and sanitized
  orchestration. Its backend has render, registration and inventory operations only.
  `command_runtime.rs` connects actual compiled rendering and `DirectAdminReader`.
- Direct reads inherit Step 5's permission binding, URL/header allowlist, HTTPS-only
  transport, no proxies/redirects, fixed operations, consistency proof and bounds.
  No cached serving read or mutation method is introduced.

## Implemented choices for review

These are local command-design choices, not business approvals or release attestations.

1. Both commands require `--input-profile` and a new `--output` file. Plan also requires
   `--target` and `--plan-config`. Top-level `-f` conflicts with a sync command. Plan
   mode defaults to disabled and accepts only `disabled`/`plan`. There are no endpoint,
   org, workspace, arbitrary header, token, force or mutating overrides.
2. A versioned profile carries explicit release identity, policy, catalog and exclusions.
   `seed` supplies `stripe_eps_banks` (null/absent means the compiled bank default) and
   never reads ambient Settings. A supplied list uses the existing typed bank configuration;
   an explicit empty list remains different from absent configuration and unknown banks refuse.
   `effective_settings` requires an explicit existing file plus the expected effective
   settings hash; relevant ambient overrides must match that hash. File references
   resolve relative to the owning profile/config. RequiredFields currently depends only
   on EPS/Stripe bank configuration; this is source evidence, not a permanent promise.
3. The current release artifact must be `offline`. A caller-written image claim is
   rejected. Actual compiled feature keys are included by the existing build boundary;
   revision/source-tree fields are caller-provided offline provenance, not authenticated
   release evidence. Image attestation remains later work.
4. Profile input is capped at 2 MiB, plan config at 128 KiB, token file at 4 KiB and output
   at 4 MiB, with existing JSON depth/node and semantic limits. The larger profile cap
   accommodates an explicit exclusion for every compiled rule; it changes no server
   request limit. Inputs reject duplicate JSON keys, unknown fields and leaf symlinks.
   Output uses create-new mode 0600, never truncation. A failed write can leave a partial
   new artifact; only exit 0 and a complete envelope indicate success.
5. Target identity and registration are checked before settings/rendering, credential
   access or HTTP. Plan reads a specifically named reader token file only after that
   validation. The existing certified authorization fingerprint still binds its scope.
   No serving credential, environment token or secret-provider fallback exists.
6. Success stdout contains hashes and counts, not input values. Renderer refusals can
   create a sanitized failure artifact and still exit nonzero. Argument errors use a
   fixed envelope (exit 2); execution refusals use sanitized code/phase/detail/counts
   (exit 1). Plan exports only the sanitized candidate report, not live rows/case bodies.
   It says provisional, SDK verification not executed, zero mutations/leases and hosted
   apply disabled. It does not claim a verified plan hash or accepted semantic impact.

## Specification discrepancies and assumptions

Plan §6.9's short plan example omitted the input/config paths; this slice makes both
mandatory so no ambient profile/target is selected. The proposed future command family
includes seed/status/mutations; Step 7 explicitly defers those. Existing Step 5/6 limits
are retained instead of increasing them to the plan's proposed 1,000-action/512-KiB
ceilings. Locked SDK/core remain 0.119.0, not P6's historical 0.115.5 reference. The
current MSRV is 1.94.1; installed stable 1.97.1 is separate evidence.

Source review found that the existing Settings CSV bank parser cannot represent an empty
set. Named seeds therefore construct `BankRedirectConfig`/`ConnectorBankNames`/`BanksVector`
directly, using the same bank enum parser, instead of round-tripping through CSV. This
retains the foundation's absent/empty/populated distinction. Effective-settings parsing
and source business data are unchanged; changing their CSV contract would be separate work.

Assumption: explicit local profile/config/credential paths are controlled by the operator.
Leaf symlink/type/size checks do not constitute a hostile-filesystem sandbox. Profile
files are immutable during invocation. The effective Settings parser reopens the checked
file using the shared loader; observed effective inputs are hash-bound. No claim is made
that the CLI authenticates a deployed image, verifies a business catalog, or proves a
reader's real permission scope without target certification.

## Review and remaining gates

Use the [local draft PR](review-draft.md), [walkthrough](walkthrough.md) and
[checks](checks.md). The all-excluded compiled profile is explicitly a synthetic plumbing
fixture; blocked full profiles remain blocked, and successful synthetic planning is not
real-server/browser evidence. Business-source, aliases/options/presentation and SDK
corrections remain separate proposals. Original plan/roadmap and committed seed stay intact.

After review, Step 8's local journal implementation is the next sequential slice, with
Q6 storage certification separate. Step 2 certification and Step 1 owner/candidate-browser
work can progress independently when requested with verified disposable dependencies.
Contracts, journal, coordination, restoration, release/feature evidence and V1–V4 remain
required before hosted apply. No next step was started.

Manual H commit: `feat: expose required-fields render and planning commands`.
The already-uncommitted Step 6 files belong to
`feat: plan required-fields changes with explicit ownership`; review shared module/test/
progress-file hunks separately. No new W/U commit is needed for Step 7.
