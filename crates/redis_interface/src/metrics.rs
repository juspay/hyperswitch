//! OpenTelemetry metrics for Redis operations, gated behind the `metrics` feature.

#[cfg(feature = "metrics")]
use router_env::{global_meter, histogram_metric_f64};

#[cfg(feature = "metrics")]
global_meter!(GLOBAL_METER, "REDIS");

// The histogram's `_count` series carries the call count, so no separate counter is needed.
#[cfg(feature = "metrics")]
histogram_metric_f64!(REDIS_CALL_TIME, GLOBAL_METER);

/// The Redis operation being performed, used as the `operation` metric label.
#[derive(Debug)]
pub(crate) enum RedisOperation {
    SetKey,
    SetKeyWithoutModifyingTtl,
    SetKeyWithExpiry,
    SerializeAndSetKeyWithExpiry,
    SetMultipleKeysIfNotExist,
    SetKeyIfNotExistsWithExpiry,
    SetKeyIfNotExistsAndGetValue,
    GetKey,
    GetMultipleKeys,
    Exists,
    DeleteKey,
    SetExpiry,
    SetExpireAt,
    GetTtl,
    SetHashFields,
    SetHashFieldIfNotExist,
    IncrementFieldsInHash,
    GetHashField,
    GetHashFields,
    Hscan,
    Scan,
    Sadd,
    StreamAppendEntry,
    StreamDeleteEntries,
    StreamTrimEntries,
    StreamAcknowledgeEntries,
    StreamGetLength,
    StreamReadEntries,
    StreamReadWithOptions,
    AppendElementsToList,
    GetListElements,
    GetListLength,
    LpopListElements,
    ConsumerGroupCreate,
    ConsumerGroupDestroy,
    ConsumerGroupDeleteConsumer,
    ConsumerGroupSetLastId,
    ConsumerGroupSetMessageOwner,
    EvaluateRedisScript,
}

/// Times a Redis future and records its latency, tagged by operation.
#[cfg(feature = "metrics")]
#[inline]
pub(crate) async fn track_redis_call<Fut, U>(operation: RedisOperation, future: Fut) -> U
where
    Fut: std::future::Future<Output = U>,
{
    let start = std::time::Instant::now();
    let output = future.await;
    let time_elapsed = start.elapsed();

    router_env::logger::debug!(
        redis_operation = ?operation,
        execution_time = ?time_elapsed,
        "Redis operation executed"
    );

    let attributes = router_env::metric_attributes!(("operation", format!("{operation:?}")));
    REDIS_CALL_TIME.record(time_elapsed.as_secs_f64(), attributes);

    output
}

/// No-op pass-through when the `metrics` feature is disabled.
#[cfg(not(feature = "metrics"))]
#[inline]
pub(crate) async fn track_redis_call<Fut, U>(_operation: RedisOperation, future: Fut) -> U
where
    Fut: std::future::Future<Output = U>,
{
    future.await
}
