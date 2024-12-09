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
counter_metric!(IN_MEMORY_CACHE_HIT, GLOBAL_METER);
counter_metric!(IN_MEMORY_CACHE_MISS, GLOBAL_METER);
counter_metric!(IN_MEMORY_CACHE_EVICTION_COUNT, GLOBAL_METER);
