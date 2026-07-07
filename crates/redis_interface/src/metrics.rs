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
use time::OffsetDateTime;

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
    Publish,
}

/// Extracts whether a completed Redis roundtrip succeeded, so the emitted
/// `ExternalServiceCall` can carry `success`/`status_code`.
///
/// Most `track_redis_call` call sites pass a future whose output is a `Result`
/// (the raw `RedisResult`/`FredResult`, or an already-`change_context`ed
/// `CustomResult`), so the blanket impl below covers those.
pub(crate) trait RedisCallOutcome {
    fn succeeded(&self) -> bool;
}

impl<T, E> RedisCallOutcome for Result<T, E> {
    fn succeeded(&self) -> bool {
        self.is_ok()
    }
}

/// Fred's streaming `SCAN`/`HSCAN` helpers collect successful pages into a
/// `Vec<String>` after logging and dropping page-level errors. At this layer no
/// error signal remains, so completion is treated as success for event emission.
impl RedisCallOutcome for Vec<String> {
    fn succeeded(&self) -> bool {
        true
    }
}

/// Records the OTel latency metric and emits one `ExternalServiceCall` event
/// for a completed Redis roundtrip.
///
/// `request_id` is read from the `router_env::request_context` task-local. When
/// absent (background work — drainer, scheduler, un-rescoped spawn) no event is
/// emitted: the correlator joins on `request_id` and cannot place request-less
/// rows. The `is_enabled()` guard skips the request-id lookup and event
/// construction entirely when emission is disabled — Redis is the hottest call
/// path.
#[inline]
fn record_roundtrip(
    event_emitter: &dyn ExternalServiceEventEmitter,
    operation: &RedisOperation,
    success: bool,
    time_elapsed: std::time::Duration,
) {
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

    if event_emitter.is_enabled() {
        if let Some(request_id) = router_env::request_context::try_get() {
            event_emitter.emit_external_service_call(ExternalServiceCall {
                service_name: "redis".to_string(),
                endpoint: format!("{operation:?}"),
                method: "Redis".to_string(),
                request_id,
                status_code: if success { 200 } else { 500 },
                success,
                latency_ms: time_elapsed.as_millis(),
                created_at_timestamp: OffsetDateTime::now_utc().unix_timestamp_nanos(),
            });
        }
    }
}

/// Times a single Redis roundtrip, records its latency metric, and emits one
/// `ExternalServiceCall` event reflecting this exact roundtrip. `success` is
/// derived from the future's `Result` output.
#[inline]
pub(crate) async fn track_redis_call<Fut, U>(
    event_emitter: &dyn ExternalServiceEventEmitter,
    operation: RedisOperation,
    future: Fut,
) -> U
where
    Fut: std::future::Future<Output = U>,
    U: RedisCallOutcome,
{
    let start = std::time::Instant::now();
    let output = future.await;
    let time_elapsed = start.elapsed();

    record_roundtrip(event_emitter, &operation, output.succeeded(), time_elapsed);

    output
}
