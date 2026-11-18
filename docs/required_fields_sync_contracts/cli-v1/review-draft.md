# Local draft PR — do not publish automatically

Suggested title: **feat: add offline required-fields rendering and provisional planning**

## Type of Change

- [x] New feature
- [x] Documentation

## Description

Required-field definitions can drift from manually maintained Superposition configuration.
This slice makes the compiled definitions reviewable through the existing router binary:
`required-fields render` produces a deterministic offline bundle from an explicit profile;
`required-fields plan` binds that desired state to complete inventory, exact ownership and
prerequisites, then emits a sanitized provisional candidate. An explicit field equal to a
default remains explicit, ordered options retain their order, and a shared row retains its
unowned data. Ambiguous mappings and missing metadata refuse. The planner's separate
acceptance boundary rejects unapproved semantic impact; this CLI exports candidates only.

The opt-in command dispatch runs before unrelated service initialization. Seed profiles
ignore ambient bank overrides; effective-settings profiles bind relevant inputs to an
expected hash using the existing Settings loader. Outputs are bounded, private and
create-new. Argument/config failures are redacted. Plans do not acquire leases, create
prerequisites, stamp descriptions or mutate Superposition.

The review scope includes the completed input/renderer/inventory/planner foundations and
this offline/read-only command slice. Existing serving invocation and the scheduler parser
are retained. Both production contract registries remain empty: a real plan refuses before
opening credentials or HTTP. No hosted target has been authorized and no mutating command
is available. Synthetic fixtures are not substitute contract or browser certification.

### Additional Changes

- [ ] This PR modifies the HTTP API contract
- [ ] This PR modifies the database schema
- [ ] This PR adds serving configuration/environment variables

New versioned CLI profile/plan artifacts are documented in [README.md](README.md).
The isolated check crate now imports the actual argument/command modules and exact
H-locked Clap dependency. Root lock and production dependency versions are unchanged.

## Motivation and Context

The implementation playbook deliberately places an offline/read-only review checkpoint
before journaling, authority/coordination, mutation, verification and restoration. Review
can resolve ownership/mapping/report contracts while those dependent gates remain closed.
Business-source and SDK corrections need separate approved changes. The committed seed
is unchanged; unsupported compiled profiles still produce failure, not an empty success.

## How did you test it?

See [checks.md](checks.md) and [evidence.json](evidence.json) for executed commands and
their final results. They distinguish actual source unit tests, synthetic orchestration,
actual compiled binary execution, build-script rejection, and unexecuted acceptance gates.
The actual-binary harness denies all network access at the OS level and uses only owned
synthetic artifacts, invalid/unavailable unrelated DB/Redis settings and intercepted local
command boundaries. Successful full-compiled fixture rendering explicitly excludes every
source for plumbing coverage; a separate unexcluded profile must refuse.

Historical Step 1 browser and Step 6 actual JS/Rust comparisons retain their original scope.
No real-server, new candidate browser, image/MSRV/release matrix or target V1–V4 acceptance
is claimed here. Exact contracts, journal, coordination, restoration and all verification
gates must pass before any hosted apply is enabled.

## Checklist

- [ ] Whole-workspace `cargo +nightly fmt --all` (not run; targeted formatting recorded)
- [ ] Full-workspace Clippy (not run; focused checks recorded)
- [x] Added focused tests and bounded synthetic fixtures
- [ ] Human review of renderer/planner slice
- [ ] Exact-server and candidate-specific browser certification

This is a local draft only. Git staging, commits, push and remote PR creation remain manual.
