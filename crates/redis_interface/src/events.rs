//! External service call event emission for Redis commands.

use std::sync::Arc;

use common_utils::external_service::{ExternalServiceCall, ExternalServiceEventEmitter};

/// Service name attached to every Redis external service call event.
const REDIS_SERVICE_NAME: &str = "redis";

/// Builds and emits an [`ExternalServiceCall`] event for a completed Redis command.
///
/// Events are emitted only when a `request_id` is present (i.e. the command ran
/// within a request context) and the emitter is enabled; background flows
/// (drainer, scheduler polling, startup) skip emission silently. `endpoint` is
/// evaluated lazily so hot paths don't pay for key formatting when emission is
/// skipped.
pub(crate) fn emit_redis_external_service_call(
    event_emitter: &Arc<dyn ExternalServiceEventEmitter>,
    request_id: Option<&str>,
    method: &str,
    endpoint: impl FnOnce() -> String,
    success: bool,
    start_time: std::time::Instant,
) {
    if !event_emitter.is_enabled() {
        return;
    }
    let Some(request_id) = request_id else {
        return;
    };

    let latency_ms = start_time.elapsed().as_millis();
    let created_at_timestamp = time::OffsetDateTime::now_utc().unix_timestamp_nanos();

    event_emitter.emit_external_service_call(ExternalServiceCall {
        service_name: REDIS_SERVICE_NAME.to_string(),
        endpoint: endpoint(),
        method: method.to_string(),
        request_id: request_id.to_string(),
        status_code: 0,
        success,
        latency_ms,
        created_at_timestamp,
    });
}
