//! OpenTelemetry metrics for Redis operations, gated behind the `metrics` feature.

#[cfg(feature = "metrics")]
use router_env::{global_meter, histogram_metric_f64};

#[cfg(feature = "metrics")]
global_meter!(GLOBAL_METER, "REDIS");

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
#[inline]
pub(crate) async fn track_redis_call<Fut, U>(operation: RedisOperation, future: Fut) -> U
where
    Fut: std::future::Future<Output = U>,
{
    #[cfg(feature = "metrics")]
    let _breakdown_timer = router_env::pms_confirm_breakdown::start(match operation {
        RedisOperation::GetKey
        | RedisOperation::GetMultipleKeys
        | RedisOperation::Exists
        | RedisOperation::GetTtl
        | RedisOperation::GetHashField
        | RedisOperation::GetHashFields
        | RedisOperation::Hscan
        | RedisOperation::Scan
        | RedisOperation::StreamGetLength
        | RedisOperation::StreamReadEntries
        | RedisOperation::StreamReadWithOptions
        | RedisOperation::GetListElements
        | RedisOperation::GetListLength => router_env::pms_confirm_breakdown::Operation::RedisRead,
        RedisOperation::EvaluateRedisScript => {
            router_env::pms_confirm_breakdown::Operation::RedisOther
        }
        _ => router_env::pms_confirm_breakdown::Operation::RedisWrite,
    });
    let start = std::time::Instant::now();
    let output = future.await;
    let time_elapsed = start.elapsed();

    tracing::debug!(
        redis_operation = ?operation,
        execution_time = ?time_elapsed,
        "Redis operation executed"
    );

    #[cfg(feature = "metrics")]
    {
        let attributes = router_env::metric_attributes!(("operation", format!("{operation:?}")));
        REDIS_CALL_TIME.record(time_elapsed.as_secs_f64(), attributes);
    }

    output
}
