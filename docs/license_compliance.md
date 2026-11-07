# Dependency license compliance

Every pull request is checked for the licenses of the dependencies it brings
into the tree. A pull request that pulls in a strong-copyleft or
source-available dependency fails the check and cannot be merged; weak-copyleft
and undeclared licenses are reported in a comment but do not block anything.

The check evaluates the **dependency delta only** — packages that the pull
request adds or version-bumps relative to the merge base. Dependencies that
already exist on the base branch are never re-evaluated, so tightening the
policy cannot retroactively break unrelated pull requests, and an author is
only ever told about dependencies they actually introduced.

| File | Role |
| --- | --- |
| [`.github/license-policy.toml`](../.github/license-policy.toml) | The policy: license tiers, the lockfiles to scan, and per-package exceptions |
| [`.github/scripts/license_check.py`](../.github/scripts/license_check.py) | The engine: lockfile diffing, license resolution and classification |
| [`.github/scripts/tests/test_license_check.py`](../.github/scripts/tests/test_license_check.py) | Unit tests, run as part of the workflow |
| [`.github/workflows/license-compliance.yml`](../.github/workflows/license-compliance.yml) | Runs the check on pull requests and in the merge queue |
| [`.github/workflows/license-compliance-comment.yml`](../.github/workflows/license-compliance-comment.yml) | Posts the report as a single sticky comment |

## The policy

| Tier | Licenses | Pull request behaviour |
| --- | --- | --- |
| A — Allow | MIT, Apache-2.0, the BSD family, ISC, Zlib, BSL-1.0, Unicode-3.0, … | Passes silently |
| B — Warn | MPL-2.0, CDDL, EPL, Ms-PL, OSL-3.0, Artistic-2.0, CC-BY-3.0/4.0, … | Passes, reported in the comment |
| C — Deny | GPL, AGPL, LGPL, SSPL-1.0, BUSL-1.1, Elastic-2.0, CC-BY-NC/SA/ND, … | **Fails, blocks the merge** |
| D — Unknown | No declared license, non-standard identifiers, `NOASSERTION` | Passes, reported in the comment |

The authoritative lists live in `.github/license-policy.toml`. Runtime
dependencies and dev/build dependencies are both evaluated and reported
separately, in their own column of the report.

## How a license is decided

**Licenses come from registry metadata, not from the source tree.** For each
package in the delta the checker asks crates.io or the npm registry what the
publisher declared. Nothing from the pull request is installed, built or
executed, and no language toolchain is needed, which is what keeps the check
down to a few seconds. Source-file license header scanning is deliberately out
of scope: it needs ScanCode-class tooling and has a long false-positive tail.

**SPDX expressions are evaluated, not string-matched.** This matters in both
directions:

| Declared license | Result | Why |
| --- | --- | --- |
| `MIT OR Apache-2.0` | Allow | Either license may be taken |
| `(MIT OR GPL-3.0-or-later)` | Allow | This is how `jszip` ships; taking it under MIT is fine |
| `MIT AND GPL-3.0-only` | **Deny** | Both sets of terms have to be honoured |
| `Apache-2.0 WITH LLVM-exception` | Allow | Listed in tier A as a whole |
| `GPL-2.0-or-later WITH Classpath-exception-2.0` | **Deny** | The exception is not listed, so it cannot soften the license |
| `GPL-2.0+`, `MIT/Apache-2.0` | Deny, Allow | Deprecated spellings are normalised |

`OR` resolves to its least restrictive operand and `AND` to its most
restrictive one. Where an identifier matches more than one tier, the most
restrictive tier wins.

**Runtime and dev/build scopes are derived from the manifests.** `Cargo.lock`
records no dependency kinds, so the checker walks the lockfile graph from the
`[dependencies]` of every workspace manifest; anything it does not reach is
dev/build. Two subtleties are handled explicitly: a workspace crate's lockfile
edges merge its normal, dev and build dependencies into one list, so the walk
does not follow them and uses the crate's manifest instead; and a workspace
crate that only ever appears as another crate's dev dependency — `test_utils`,
for instance — does not contribute its dependencies to the runtime set. For
npm, the lockfile's own `dev` flags are used, and an ecosystem can be pinned to
one scope with `scope = "dev/build"` when the whole tree is test tooling, as
the Cypress and Postman harnesses in this repository are.

## When the check reports something

**A denied license.** Drop the dependency or replace it with a permissively
licensed equivalent. If the classification is wrong — the registry metadata is
stale, or the package is dual-licensed in a way the declaration does not
capture — record an exception in `.github/license-policy.toml`:

```toml
[[exceptions]]
ecosystem = "cargo"
name = "some-crate"
versions = "1.*"
tier = "allow"
license = "ISC AND MIT AND OpenSSL"
reason = "Registry metadata carries no SPDX expression; LICENSE file reviewed."
```

An exception is matched before any registry lookup happens, so it also fixes
packages the registries know nothing about.

**A weak-copyleft license.** Nothing is blocked, but check what the license
asks for. MPL-2.0 and EPL-2.0, for example, require modifications to the
dependency's own files to be published; they do not reach into the code that
merely uses them.

**An unknown license.** The package declares no license, or one that is not an
SPDX identifier. Confirm it by hand from the package's repository and record it
as an exception with the verified value.

## Running the check locally

```bash
python .github/scripts/license_check.py --base-ref origin/main
```

The head revision defaults to the working tree, so this reports what the branch
you are on would introduce. `--head-ref` compares two committed revisions
instead, `--offline` skips the registry lookups, and `--json-out` and
`--markdown-out` write the machine-readable report and the comment body.

The unit tests need no network and no dependencies beyond the standard library:

```bash
python -m unittest discover --start-directory .github/scripts/tests
```

## Why two workflows

`license-compliance.yml` runs on `pull_request`, which for a fork gets a
read-only token and no secrets. It cannot post a comment, so it uploads the
report as an artifact. `license-compliance-comment.yml` runs on `workflow_run`,
where a write-capable token is available, and posts it.

The alternative — `pull_request_target` with a checkout of the pull request's
head — would run fork-authored code with a privileged token, which is the
pattern this repository already avoids in `pr-convention-checks.yml`. In the
commenting workflow the artifact is treated strictly as data: the comment body
is handed to the API as a single quoted argument rather than interpolated into
a command, and the pull request to comment on is resolved from the head commit
rather than read out of the report.

Because GitHub only runs `workflow_run` workflows from the default branch, the
sticky comment starts working once these files are on `main`.

## Porting this check to another repository

The check was written to be copied as-is to `juspay/prism`,
`juspay/hyperswitch-web`, `juspay/hyperswitch-control-center`,
`juspay/hyperswitch-suite`, `juspay/hyperswitch-helm`,
`juspay/decision-engine` and the SDK repositories.

1. Copy `.github/license-policy.toml`, `.github/scripts/license_check.py`,
   `.github/scripts/tests/test_license_check.py` and the two workflow files.
2. Edit the `[[ecosystems]]` entries in the policy to name that repository's
   lockfiles. Nothing else in the policy is repository-specific — keeping the
   tier lists identical across repositories is the point.
3. Mark **Check licenses of newly introduced dependencies** as a required
   status check in the repository's branch protection rules.

For a Cargo workspace, make sure the `manifests` patterns cover every workspace
member, otherwise that member's dependencies are classified as dev/build rather
than runtime.

## Adding support for another ecosystem

`COLLECTORS` in `license_check.py` maps an ecosystem name to a `Collector`. A
new one implements `collect(revision) -> {(ecosystem, name, version): Package}`
and sets `Package.source` so that `Package.registry` can tell where to look the
license up; `LicenseResolver` then needs a branch for that registry. The
delta computation, the policy evaluation and the reporting are all shared and
need no changes.

Gradle and Swift Package Manager are the obvious next candidates, for the
Android and iOS SDK repositories.
