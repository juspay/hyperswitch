//! OpenTelemetry metrics for Redis operations, gated behind the `metrics` feature.

use common_utils::external_service::{ExternalServiceCall, ExternalServiceEventEmitter};
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
    Publish,
}

pub(crate) trait RedisCallStatus {
    fn is_success(&self) -> bool;
}

impl<T, E> RedisCallStatus for Result<T, E> {
    fn is_success(&self) -> bool {
        self.is_ok()
    }
}

/// Times a Redis future, records its latency, and emits an external-service event
/// when request context is available.
#[inline]
pub(crate) async fn track_redis_call<Fut, U>(
    request_id: Option<&str>,
    event_emitter: &dyn ExternalServiceEventEmitter,
    operation: RedisOperation,
    future: Fut,
) -> U
where
    Fut: std::future::Future<Output = U>,
    U: RedisCallStatus,
{
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

    if let Some(request_id) = request_id {
        event_emitter.emit_external_service_call(ExternalServiceCall {
            service_name: "redis".to_string(),
            endpoint: "redis_command".to_string(),
            method: format!("{operation:?}"),
            request_id: request_id.to_string(),
            status_code: 0,
            success: output.is_success(),
            latency_ms: time_elapsed.as_millis(),
            created_at_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| i128::try_from(duration.as_nanos()).unwrap_or(i128::MAX))
                .unwrap_or_default(),
        });
    }

    output
}
