use router_env::{counter_metric, gauge_metric, global_meter};

global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(KV_MISS, GLOBAL_METER); // No. of KV misses

// Metrics for KV
counter_metric!(KV_OPERATION_SUCCESSFUL, GLOBAL_METER);
counter_metric!(KV_OPERATION_FAILED, GLOBAL_METER);
counter_metric!(KV_PUSHED_TO_DRAINER, GLOBAL_METER);
counter_metric!(KV_FAILED_TO_PUSH_TO_DRAINER, GLOBAL_METER);
counter_metric!(KV_SOFT_KILL_ACTIVE_UPDATE, GLOBAL_METER);

// Metrics for In-memory cache
gauge_metric!(IN_MEMORY_CACHE_ENTRY_COUNT, GLOBAL_METER);
// The configured entry ceiling, recorded alongside the count above so that utilisation is
// computable from the metrics rather than from configuration. moka's other runtime figure,
// `weighted_size`, is not recorded: with no weigher configured it equals the entry count.
gauge_metric!(IN_MEMORY_CACHE_MAX_CAPACITY, GLOBAL_METER);
counter_metric!(IN_MEMORY_CACHE_HIT, GLOBAL_METER);
counter_metric!(IN_MEMORY_CACHE_MISS, GLOBAL_METER);
counter_metric!(IN_MEMORY_CACHE_EVICTION_COUNT, GLOBAL_METER);

// Metrics for in-memory cache population coalescing
counter_metric!(IN_MEMORY_CACHE_POPULATION_AVOIDED, GLOBAL_METER); // Miss served by another caller
counter_metric!(IN_MEMORY_CACHE_POPULATE_TIMEOUT, GLOBAL_METER); // Populate hit the timeout cap
counter_metric!(IN_MEMORY_CACHE_TYPE_MISMATCH, GLOBAL_METER); // Entry had an unexpected type
counter_metric!(IN_MEMORY_CACHE_INVARIANT_VIOLATION, GLOBAL_METER); // Unreachable compute outcome

// Metrics for cache invalidation
counter_metric!(CACHE_REDACTION_FAILURE_COUNT, GLOBAL_METER);
