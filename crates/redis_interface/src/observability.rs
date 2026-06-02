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
/// task-local; if absent (background work, un-rescoped spawn) the row is
/// emitted with `request_id = ""` — intentional, no warning.
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

    let request_id = router_env::request_context::try_get().unwrap_or_default();

    pool.event_emitter
        .emit_external_service_call(ExternalServiceCall {
            service_name: "redis".to_string(),
            endpoint: command.to_string(),
            method: command.to_string(),
            request_id,
            status_code: if result.is_ok() { 200 } else { 500 },
            success: result.is_ok(),
            latency_ms: start.elapsed().as_millis(),
            created_at_timestamp: OffsetDateTime::now_utc().unix_timestamp_nanos(),
        });

    result
}
