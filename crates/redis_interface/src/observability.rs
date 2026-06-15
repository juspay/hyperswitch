//! Per-call event emission for Redis commands.
//!
//! `observe` wraps each public async command method on `RedisConnectionPool`
//! to record latency, success, and the current request ID, then dispatch the
//! row through the pool's `event_emitter`. The `observed!` macro at the crate
//! root is sugar for the common `observe(self, "CMD", { body }).await` shape.
//!
//! Backend-agnostic: `crate::RedisConnectionPool` is whichever backend
//! (`redis-rs` or `fred`) is active for this build. Both backends carry the
//! same `event_emitter` field shape.

use std::{future::Future, time::Instant};

use common_utils::external_service::ExternalServiceCall;
use time::OffsetDateTime;

use crate::RedisConnectionPool;

/// Time the given future, then emit one `ExternalServiceCall` event reflecting
/// the outcome. `command` is the Redis command name (e.g. `"SET"`, `"HGET"`).
///
/// `request_id` is read from the `router_env::request_context::REQUEST_ID`
/// task-local. If absent (background work — drainer, scheduler, un-rescoped
/// spawn), no event is emitted: the correlator joins events to API requests on
/// `request_id` and has no way to place request-less rows, so emitting them
/// would only buffer dead data. Revisit if a direct background-bucket
/// consumption path is built (see
/// `crates/analytics/docs/redis_instrumentation_plan.md`).
#[inline]
pub async fn observe<F, R, E>(
    pool: &RedisConnectionPool,
    command: &'static str,
    fut: F,
) -> Result<R, E>
where
    F: Future<Output = Result<R, E>>,
{
    let start = Instant::now();
    let result = fut.await;

    // Skip the request-id lookup and event construction entirely when the
    // emitter is a no-op (emission disabled) — Redis is the hottest call path.
    if pool.event_emitter.is_enabled() {
        if let Some(request_id) = router_env::request_context::try_get() {
            pool.event_emitter.emit_external_service_call(build_event(
                command,
                request_id,
                result.is_ok(),
                start.elapsed().as_millis(),
                OffsetDateTime::now_utc().unix_timestamp_nanos(),
            ));
        }
    }

    result
}

fn build_event(
    command: &'static str,
    request_id: String,
    success: bool,
    latency_ms: u128,
    created_at_timestamp: i128,
) -> ExternalServiceCall {
    ExternalServiceCall {
        service_name: "redis".to_string(),
        endpoint: command.to_string(),
        method: "Redis".to_string(),
        request_id,
        status_code: if success { 200 } else { 500 },
        success,
        latency_ms,
        created_at_timestamp,
    }
}
