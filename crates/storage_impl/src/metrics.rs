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
// Total weight a cache holds, and the ceiling it is weighed against. Both carry a `limit_unit`
// attribute, since a cache bounded by entry count weighs 1 per entry while one bounded in
// megabytes weighs each entry's reported bytes. Recording the ceiling alongside the weight is
// what makes utilisation computable without hardcoding configuration into a dashboard.
gauge_metric!(IN_MEMORY_CACHE_WEIGHTED_SIZE, GLOBAL_METER);
gauge_metric!(IN_MEMORY_CACHE_MAX_CAPACITY, GLOBAL_METER);
counter_metric!(IN_MEMORY_CACHE_HIT, GLOBAL_METER);
counter_metric!(IN_MEMORY_CACHE_MISS, GLOBAL_METER);
counter_metric!(IN_MEMORY_CACHE_EVICTION_COUNT, GLOBAL_METER);

// Metrics for cache invalidation
counter_metric!(CACHE_REDACTION_FAILURE_COUNT, GLOBAL_METER);
