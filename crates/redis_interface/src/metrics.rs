//! OpenTelemetry metrics and per-roundtrip event emission for Redis operations.
//!
//! `track_redis_call` wraps every raw Redis client future — i.e. every actual
//! network roundtrip. It records the OTel latency histogram (gated behind the
//! `metrics` feature) and, independently, emits one `ExternalServiceCall` event
//! per roundtrip reflecting that exact command's latency and outcome.
//!
//! Emitting at the roundtrip layer (rather than once per logical command
//! method) means multi-roundtrip methods — multitenancy-fallback retries,
//! `HSET` + `EXPIRE`, cluster parallel `GET`, `SCAN`/`HSCAN` cursor pages —
//! each produce one event per real roundtrip with correct wall-clock latency,
//! instead of a single event whose latency sums or hides the constituent calls.

use common_utils::external_service::{ExternalServiceCall, ExternalServiceEventEmitter};
#[cfg(feature = "metrics")]
use router_env::{global_meter, histogram_metric_f64};

#[cfg(feature = "metrics")]
global_meter!(GLOBAL_METER, "REDIS");

#[cfg(feature = "metrics")]
histogram_metric_f64!(REDIS_CALL_TIME, GLOBAL_METER);

/// The Redis operation being performed. Used both as the `operation` metric
/// label and as the `endpoint` field on the emitted `ExternalServiceCall`.
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
    StreamReadGroup,
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

/// Extracts whether a completed Redis roundtrip succeeded, so the emitted
/// `ExternalServiceCall` can carry `success`/`status_code`.
///
/// Most `track_redis_call` call sites pass a future whose output is a `Result`
/// (the raw `RedisResult`/`FredResult`, or an already-`change_context`ed
/// `CustomResult`), so the blanket impl below covers those.
pub(crate) trait RedisCallStatus {
    fn is_success(&self) -> bool;
}

impl<T, E> RedisCallStatus for Result<T, E> {
    fn is_success(&self) -> bool {
        self.is_ok()
    }
}

/// Fred's streaming `SCAN`/`HSCAN` helpers collect successful pages into a
/// `Vec<String>` after logging and dropping page-level errors. At this layer no
/// error signal remains, so completion is treated as success for event emission.
impl RedisCallStatus for Vec<String> {
    fn is_success(&self) -> bool {
        true
    }
}

/// Times a single Redis roundtrip, records its latency metric, and emits one
/// `ExternalServiceCall` event reflecting this exact roundtrip.
///
/// When `request_id` is absent (background work, drainer, scheduler) no event is
/// emitted: the correlator joins on `request_id` and cannot place request-less
/// rows.
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
        let success = output.is_success();
        event_emitter.emit_external_service_call(ExternalServiceCall {
            service_name: "redis".to_string(),
            endpoint: format!("{operation:?}"),
            method: "Redis".to_string(),
            request_id: request_id.to_string(),
            status_code: if success { 200 } else { 500 },
            success,
            latency_ms: time_elapsed.as_millis(),
            created_at_timestamp: common_utils::date_time::now_unix_timestamp_nanos(),
        });
    }

    output
}
