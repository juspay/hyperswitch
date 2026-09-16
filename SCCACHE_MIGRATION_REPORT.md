# sccache CI caching migration — investigation report

Branch: `ci/sccache-migration` · PR: [juspay/hyperswitch#14253](https://github.com/juspay/hyperswitch/pull/14253)

## Trigger

[mozilla/sccache#2438](https://github.com/mozilla/sccache/pull/2438) (merged, in sccache 0.11.0+) fixes sccache
misinterpreting a leading `rustc` argument that `clippy-driver` prepends, which previously caused
"multiple input files" errors and made clippy invocations fail to cache. This prompted revisiting whether
sccache could now replace/augment `Swatinem/rust-cache` on this repo's `check-msrv`, `test`, and `check-v2`
CI jobs.

## Prior work discovered mid-investigation

Two pieces of prior work on this exact question were found on the remote that predated this session and were
not initially visible from local `main` (which was stale):

- **[juspay/hyperswitch#14195](https://github.com/juspay/hyperswitch/issues/14195)** (issue) — root-caused
  that `test`/`check-v2`'s existing `RUSTC_WRAPPER: sccache` (no backend configured) has a **structurally
  zero cross-run hit rate**: `hyperswitch-runners` are ephemeral ARC pods, a new one per job, so sccache's
  local-disk fallback cache is destroyed with the pod every time. Also flagged the repo's shared GitHub
  Actions cache budget was at 9.56/10GB.
- **[juspay/hyperswitch#14197](https://github.com/juspay/hyperswitch/pull/14197)** (draft PR,
  `ci/sccache-gha-backend`) — an earlier experiment layering sccache's GHA cache backend on top of
  `Swatinem/rust-cache` for `check-msrv` only. Measured ~7% faster warm, but 17.6% hit rate, write errors,
  and active cache-eviction thrash against the already-full 10GB budget. Concluded **"not recommended."**
  Also claimed `cargo clippy`'s `RUSTC_WORKSPACE_WRAPPER=clippy-driver` mechanism was fundamentally
  uncacheable by sccache, citing [sccache#539](https://github.com/mozilla/sccache/issues/539),
  [sccache#423](https://github.com/mozilla/sccache/issues/423), and a rejected
  [sccache#728](https://github.com/mozilla/sccache/pull/728) — **this claim turned out to be wrong**, see
  below.

## `check-msrv` (GitHub-hosted `ubuntu-latest`)

Re-ran the GHA-backend experiment independently (full replacement of rust-cache rather than layering), then
tried a different design. All numbers are cold-vs-warm on **identical code** (empty commits, same job, same
runner type).

| Design | Run | Rust cache hit rate | Total job time |
|---|---|---|---|
| `rust-cache` (baseline, unrelated PRs) | — | n/a | ~13m28s avg |
| sccache, GHA backend (`SCCACHE_GHA_ENABLED`) | cold | 0% | 10m14s |
| sccache, GHA backend | populate (write) | 0% | 16m25s |
| sccache, GHA backend | **warm** | 73.18% | **10m54s — slower than the 0%-hit cold run** |
| sccache, local disk + bulk `actions/cache` | populate (write) | 0% | 15m30s |
| sccache, local disk + bulk `actions/cache` | **warm** | **97.65%** | **7m52s** |

**GHA-backend root cause:** ~1193 individual Rust cache lookups (3522 total incl. C/C++, Assembler) over the
network per run. Confirmed via `gh cache list`, cross-referencing `createdAt`/`lastAccessedAt` timestamps
across two different ephemeral VMs, that objects written during the populate run were genuinely read back
during the warm run — the mechanism worked correctly, it just didn't pay for itself. This single experiment
created **2821 separate cache entries (~2GB)** in the repo's shared cache store — since deleted as cleanup.

**Winning design:** sccache in its default local-disk mode (zero network calls per compilation unit),
wrapped in a single `actions/cache` restore-at-start / save-at-end — same I/O shape as `rust-cache` (one
download, one upload), but sccache's cache stays content-addressed per compilation unit instead of one
whole-tree blob keyed on `Cargo.lock`. Net **~41% faster** than the rust-cache baseline, and the hit rate
itself came out higher too (97.65% vs. 73.18% on identical code) — plausibly because local disk reads can't
time out the way per-object network lookups apparently were.

**Productionized as:**
- `CI-pr.yml`: restore-only, no save step — matches the old `save-if: false` (PRs never write the cache).
- `CI-push.yml`: restores, then on `push` events only (not `merge_group`) deletes the existing entry under
  the fixed key `sccache-local-check-msrv` and saves a fresh one (`actions/cache` keys are immutable, so
  reusing one key requires deleting the stale entry first — mirrors what `Swatinem/rust-cache` does
  internally). Needs `actions: write` scoped to that job for the delete call; that job never runs on fork
  PRs.
- All cache steps `continue-on-error: true` — a caching hiccup should never fail the actual compilation
  check.
- **Known first-merge behavior:** every PR (including this one) shows `Cache not found for input keys:
  sccache-local-check-msrv` until this PR merges and the first post-merge push populates the key for the
  first time. Confirmed expected, not a regression.

## `test` / `check-v2` (self-hosted `hyperswitch-runners`, S3)

Extended the same "bulk transfer, zero per-object network calls" idea to the self-hosted jobs, using
`aws s3 sync` for the bulk restore/save instead of `actions/cache` (S3 bucket/region/prefix are configured
at the runner-pod level, outside this repo, authenticated via the pod's IAM instance role — no AWS
credentials added to the workflow). sccache's own native S3 backend is deliberately avoided (it would
reintroduce the same per-object network problem found above) by capturing `SCCACHE_BUCKET`/`SCCACHE_REGION`/
`SCCACHE_S3_KEY_PREFIX` under different variable names, then blanking the originals so sccache falls back to
local disk during compilation.

| Job | Populate (cold) | Warm | Baseline (current `main`, unrelated PRs) |
|---|---|---|---|
| `test` | 47m56s | 37m08s | ~36–39m (3 samples) |
| `check-v2` | 59m30s | 41m46s | ~40–42m (3 samples) |

Rust cache hit rates on the warm runs: **98.11%** (`test`, 1456/1484) and **97.79%** (`check-v2`,
2436/2491) — both confirm `cargo clippy`'s workspace-crate compiles genuinely **are** cacheable by sccache,
contradicting #14197's claim. [sccache#2438](https://github.com/mozilla/sccache/pull/2438) does appear to
cover this case; the two upstream issues #14197 cited are still open on GitHub but that's inconclusive on
its own (old issues often stay open after an unrelated partial fix lands).

**But wall-clock time didn't clearly improve over baseline**, despite the excellent hit rates. Per-step
breakdown makes the reason visible:

| Job | Step | Populate | Warm |
|---|---|---|---|
| `test` | clippy (redis-rs) | 24m21s | **13m47s** |
| `test` | clippy (fred) | 11m19s | 10m54s |
| `test` | cargo check | 10m25s | 10m09s |
| `check-v2` | clippy_v2 (redis-rs) | 24m01s | **12m04s** |
| `check-v2` | clippy_v2 (fred) | 9m52s | 9m04s |
| `check-v2` | cargo check (redis-rs) | 14m34s | **9m14s** |
| `check-v2` | cargo check (fred) | 9m34s | 9m02s |

Only the *first* compile step of each job showed a large speedup; every subsequent step barely moved.

**The decisive control:** [juspay/hyperswitch#14252](https://github.com/juspay/hyperswitch/pull/14252),
job [`check-v2`](https://github.com/juspay/hyperswitch/actions/runs/35083364298/job/104752359377) — an
unrelated PR running the **unmodified current `main` setup** (`RUSTC_WRAPPER: sccache`, no backend
configured at all, the one #14195 called structurally broken) — took **41m52s**. Statistically identical to
our warm run's **41m46s**.

**Conclusion:** `RUSTC_WRAPPER: sccache` still caches *within* a single job even with zero cross-run
persistence — the pod stays alive for the whole run, so the second clippy step reuses what the first one
just compiled, the `cargo check` steps reuse what clippy compiled, etc. That within-run reuse already
happens today, on unmodified `main`, with no backend at all. Cross-run persistence (what the S3 mechanism
adds) only helps the *first* step of a job, since it's the only one with nothing to reuse within-run — and
that gain gets diluted into statistical noise once averaged across the other 2–3 steps. This is the inverse
of `check-msrv`, which is a *single*-step job with no within-run reuse possible, so 100% of its benefit had
to come from cross-run persistence — explaining why it won cleanly and `test`/`check-v2` did not.

**Current recommendation:** do not productionize the S3 bulk-sync design for `test`/`check-v2` — no
demonstrated benefit over the current state, for real added complexity and security surface (S3 write access
via IAM role on a runner pool that also executes fork PR code). Next: try the same bulk-transfer idea using
`actions/cache` (GHA backend) instead of S3 on `hyperswitch-runners`, both to sanity-check the "transport
doesn't matter, within-run reuse is the dominant factor" hypothesis with a second data point, and because it
would sidestep the fork-PR-gets-IAM-credentials concern entirely (GitHub restricts the cache-write token on
fork PRs automatically; no AWS credential ever needs to be ambient on the pod).

## Cleanup performed

- Deleted 2821 leftover cache entries (~2GB) created during the GHA-backend `check-msrv` experiment.
- Added `::add-mask::` for the S3 bucket/region/prefix values in job logs — not secrets (IAM-role auth), but
  `test`/`check-v2` also run on fork PRs, so the values were otherwise visible to any external contributor
  in a public repo's logs via the runner's automatic per-step env dump.

## Reference links

- [sccache#2438](https://github.com/mozilla/sccache/pull/2438) — clippy-driver argument-parsing fix (the trigger for this whole investigation)
- [sccache#539](https://github.com/mozilla/sccache/issues/539), [sccache#423](https://github.com/mozilla/sccache/issues/423), [sccache#728](https://github.com/mozilla/sccache/pull/728) — cited by #14197 as reasons clippy can't be cached; contradicted by this investigation's empirical results
- [juspay/hyperswitch#14195](https://github.com/juspay/hyperswitch/issues/14195) — root cause diagnosis for `test`/`check-v2`
- [juspay/hyperswitch#14196](https://github.com/juspay/hyperswitch/pull/14196) — rust-cache restoration for check/clippy/MSRV jobs
- [juspay/hyperswitch#14197](https://github.com/juspay/hyperswitch/pull/14197) — prior GHA-backend experiment on `check-msrv` (draft, "not recommended")
- [juspay/hyperswitch#14252](https://github.com/juspay/hyperswitch/pull/14252) — control comparison (unmodified `main`, decisive for the within-run-reuse conclusion)
- [juspay/hyperswitch#14253](https://github.com/juspay/hyperswitch/pull/14253) — this PR
