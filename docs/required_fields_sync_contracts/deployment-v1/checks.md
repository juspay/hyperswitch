# Step 15 checks and acceptance

## Inspected commands and fixture scope

Before execution, commands were inspected for effects. Git status/rev-parse/show/archive/diff
are local reads; no fetch/reset/checkout/stash/submodule update was used. `git apply` ran only
inside registered disposable tag copies, after `--check`, for the two edited shared templates.
New templates and disabled defaults were copied into those copies; tag/dependency metadata
was preserved. No replacement chart or empty dependency stub was used.

`helm template`, `helm lint --strict`, `helm dependency list`, `helm-docs --dry-run`, `yq`,
Python checks and local `gitleaks dir` have no deployment action. Tests use Python stdlib and
existing local tools/dependencies. No install/upgrade, dependency update, image pull, Argo sync,
Terraform, kubectl, uploader or hosted-storage command was run. Existing I cache-cleanup recipes
were inspected and not executed. H/W Rust/browser suites are historical evidence, not rerun
for this infrastructure-only change.

Registered fixture: `required-fields-step15-rjj4t1z9`, owner
`15cb6def-68f9-48bf-8a10-5e6bdae5adce`; exact root/cleanup scope are in the private marker
and [evidence](evidence.json). It contains only local source copies, synthetic values/renders
and logs. No Superposition/Kubernetes server or service was started. Tests and renders ran
under macOS `sandbox-exec` with `(deny network*)`. Files were written only in that registered
root or explicitly changed source/document paths. No running fixture needs termination;
private evidence and copies are retained for review.

## Executed results

| Command/check | Result | Evidence boundary |
|---|---|---|
| `python3 required-fields-sync/test_check.py -v` in I | 5 tests pass | Disabled registry, regional-authority refusal, false certification/enablement refusal, barrier coverage, strict JSON/size/path/source drift |
| `python3 required-fields-sync/check.py --hyperswitch <H>` | Pass; `deployable:false`, zero targets, all platform gates pending | Local source hashes and exact verifier archive/case pin checks only |
| `python3 tests/required_fields_sync/test_render.py -v` in Helm | 7 tests pass on each of HEAD and three patched selected v1 tags: 28 test executions | Actual full-chart renders; no cluster or H semantic/profile execution |
| `helm lint --strict <chart>` | Pass on all four chart inputs | Available exact local dependency bundles; no server certification |
| Full default render before/after with fixed synthetic subchart random defaults | Byte-identical | Ordinary serving output unchanged when required-fields defaults remain off |
| Initial raw default render comparison | Not equal: four generated Secret data entries differ | Existing Postgres passwords/Kafka cluster ID randomization; values were not printed or copied to Git. Rechecked with explicit synthetic deterministic defaults, not by ignoring arbitrary differences |
| `helm-docs --dry-run --document-dependency-values --chart-search-root <chart>` | Generated docs reviewed and applied | Baseline generator output had no drift. Normalizing only values-file line links leaves 58 diff lines: link, new disabled section and digest field; remaining churn refreshes moved source references |

Final diff/preservation, syntax/document checks, scoped secret scan and any additional results
are recorded in `evidence.json`. No unexecuted platform check is included as a pass.

The render suite checks default absence; exact serving/executor image and immutable mounts for
both Deployment and Rollout; CLI argv; one suspended PostSync Job; no retries, TTL, hook cleanup
or recovery resource creation; no companion credentials/ambient token; named-Lease-only write
permission; deny-all network; wrong authority/v2/custom/OLAP/InfraSwitch refusal; missing image,
plan, byte hashes, reader membership, verifier artifact or false certificates; and distinct
new-intent names without deleting the previous Job.

The suspended template and hard refusal are deliberate disabled behavior, **not a successful
stub executor**. The test profile is synthetic transport data and does not certify effective
settings, H strict semantic validation, browser construction or any target verification level.

## Acceptance matrix

| Gate | Local Step 15 evidence | Remaining acceptance |
|---|---|---|
| T20 coordination/executor failures | Named Lease get/update only, no delete/create, no retry hook, distinct intent names | Real renewal/loss/old executor and successor Job behavior; unresolved durable protection |
| T21 external writers | All writer classes explicitly required; enablement always refuses | Actual UI/API/router/seeder/admin/barrier exclusion and old request drain |
| T23 durable recovery storage | No Job-owned journal/PVC/Lease/TTL; independent-store contract | Real encryption/access/immutability/read-back/delete-denial/retention and independent recovery reads |
| T26 deployment authority | Exact declared image/config/intent binding; Deployment/Rollout renders; shared-reader membership mismatch refuses | Authenticated effective inputs, current monotonic intent, all traffic routes/regions/pods, scaling, stale jobs, held gates |
| T27 pre-feature rollback | Retained recovery image/input obligation; cleanup cannot delete journal resources it does not own | Retention, newer recovery intent, old executor/request exclusion and actual rollback drill |
| T28 router cache | Expected per-pod V2 gate remains pending | Authenticated per-pod SDK-config routes, actual caches/exports/last-good behavior |
| T29 browser verification | Exact source/case pin drift checks; credential-free companion definition | Certified OCI browser package, response/input exchange, real response-bound browser/old SDK runs; intercepted confirm |
| T30 backup startup | Exact publication/mount/target/freshness/startup contract | Backup publisher source/image and actual atomic publication/fallback canaries; memory refresh separate |
| T34 PostSync/platform | Actual chart templates render; manual/automatic selections inspected; zero retry/cleanup; fixed report/metric requirements | Real manual/automatic Argo runs, Job/companion IO/credentials, report ingestion/alerts and equivalent InfraSwitch controls |

Full live values rendering would require tfstate/platform inputs and remains unexecuted.
The custom integ-v2 source tag is missing locally; all custom/v2/InfraSwitch writers stay disabled.
No chart release/version selection or remote CI run occurred. Helm lint is not Kubernetes
admission/schema/effective-policy validation; no kubeconform or disposable cluster test is claimed.
