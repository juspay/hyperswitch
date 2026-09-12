# Request-local card-info reuse

The modular smoke trace contained one card-info lookup during PM session confirmation
and two during router payment confirmation. The first optimization targets the
router's repeated same-BIN lookup, reducing that flow from three to two successful
lookups across its two HTTP requests. Nonmodular confirmation benefits from the
same change, targeting two lookups down to one.

Each freshly constructed tenant SessionState owns a card-info cache; clones share
its entries, including the spawned confirmation enrichment task. Both confirmation
enrichment passes use this cache. Modular-PM's BIN enrichment and tokenization
lookup paths also use it. Values are raw CardInfo rows or successful missing results,
not request-dependent derived metadata. Distinct BINs use distinct entries;
concurrent same-key callers share one initialization. Errors and cancelled
initializations remain retryable. A new HTTP request starts with a fresh cache.

Co-badge network selection, expiration, holder data, token information, and the
existing enrichment precedence are computed as before. Missing optional metadata
is not treated as proof that the lookup has not happened. No customer or attempt
writes are changed, and no PAN/CVC is stored in the cache or trace attributes.

The cache does not cross the router/modular-PM HTTP boundary. Eliminating the
remaining lookup across that boundary requires a trusted, versioned internal
metadata contract and explicit freshness semantics. It is a separate optimization.

Regression coverage includes concurrent same-key calls, successful missing rows,
sparse rows, separate BIN keys, transient failures, and independent requests.
After rebuilding and deploying both images, fresh smoke traces verified one
card-info lookup for nonmodular confirmation and two across modular PM session
confirmation plus router confirmation, down from two and three respectively.

Validation: the shared cache implementation's four focused tests pass, covering
coalesced concurrent lookups, sparse/missing values, snapshot cloning, independent
requests, transient failures, and cancellation. They were run from a small harness
including the production `services/request_cache.rs` source directly, avoiding a
full-router test executable. Production integration is checked under both v1 and
v2 feature sets. The full router test suite was not run.

Deployed validation on 12 September 2026 also passed 4,800 modular payments at
40 RPS, 600 modular payments at 5 RPS, and 600 nonmodular payments at 5 RPS,
with zero payment failures. These runs establish successful behavior, not a
matched before/after latency improvement attributable solely to this cache.

Port validation: this change was applied cleanly to origin/main at
`2b210c02bc` in the `hs-pool-guard` worktree. The four focused cache tests
were rerun against that worktree source. The v1/v2 compilation and deployed
load validations above used the original development branch before this port;
a full router compilation on the new main-based branch has not been rerun.
