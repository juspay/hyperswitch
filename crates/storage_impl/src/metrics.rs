use router_env::{counter_metric, gauge_metric, global_meter, histogram_metric_f64};

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

// Metrics for cache invalidation
counter_metric!(CACHE_REDACTION_FAILURE_COUNT, GLOBAL_METER);

// Metrics for database
histogram_metric_f64!(DATABASE_CONNECTION_ACQUIRE_DURATION, GLOBAL_METER);

pub async fn record_db_connection_acquire_duration<Fut, T, E>(
    future: Fut,
    db_pool: crate::database::pool_metrics::DbPool,
    tenant_id: &str,
) -> Result<T, E>
where
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let start = std::time::Instant::now();
    let result = future.await;
    let duration = start.elapsed();
    let outcome = if result.is_ok() { "success" } else { "error" };
    let tenant_id = tenant_id.to_owned();

    DATABASE_CONNECTION_ACQUIRE_DURATION.record(
        duration.as_secs_f64(),
        router_env::metric_attributes!(
            ("pool", db_pool.as_str()),
            ("outcome", outcome),
            ("tenant_id", tenant_id)
        ),
    );

    result
}
